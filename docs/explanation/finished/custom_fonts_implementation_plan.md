# Custom Fonts Implementation Plan

## Overview

This plan adds campaign-scoped custom font support for dialogue UI and game menu
UI. The implementation extends `config.ron` with optional font path settings,
loads campaign-local `.ttf` assets from `<campaign_root>/fonts/`, and applies
those fonts at runtime only when configured. If a campaign omits a custom font
entry the existing default Bevy font behavior remains unchanged.

**Resolved design decisions (formerly Open Questions):**

| # | Decision |
|---|---|
| 1 | **Path format stored in `config.ron`**: `fonts/<name>.ttf` (relative to campaign root; the `fonts/` prefix is required and validated). |
| 2 | **Invalid configured font asset**: warn via `tracing::warn!` and fall back to the default font. Campaign loading is never aborted for a missing font file. |
| 3 | **`game_menu_font` scope**: ALL text spawned by `spawn_main_menu`, `spawn_save_load_menu`, and `spawn_settings_menu` in `src/game/systems/menu.rs` — titles, button labels, save slot text, settings labels. |

---

## Current State Analysis

### Existing Infrastructure

| Item | Location | Notes |
|---|---|---|
| `GameConfig` | `src/sdk/game_config.rs` L136 | Root config struct; has no `fonts` field |
| `GameConfig::validate` | `src/sdk/game_config.rs` L230 | Calls sub-config `validate()` methods; must be extended |
| `CampaignLoader::load_campaign` | `src/sdk/campaign_loader.rs` L400 | Loads `config.ron` into `campaign.game_config` |
| `spawn_dialogue_bubble` | `src/game/systems/dialogue_visuals.rs` L52 | Bevy system; spawns speaker and content `TextFont { ..default() }` |
| `spawn_main_menu` | `src/game/systems/menu.rs` L217 | Helper fn; spawns title and button `TextFont { ..default() }` |
| `spawn_save_load_menu` | `src/game/systems/menu.rs` L364 | Helper fn; spawns all save/load text `TextFont { ..default() }` |
| `spawn_settings_menu` | `src/game/systems/menu.rs` L676 | Helper fn; spawns all settings text `TextFont { ..default() }` |
| `menu_setup` | `src/game/systems/menu.rs` L144 | Bevy system; dispatches to the three spawn helpers above |
| `text_style` | `src/game/systems/ui_helpers.rs` L74 | Returns `(TextFont { ..default() }, TextColor)` — no font handle support |
| `HudPlugin::build` | `src/game/systems/hud.rs` L423 | Registers `PortraitAssets`, `FullPortraitAssets`, `ensure_portraits_loaded`, `ensure_full_portraits_loaded` — the exact pattern to follow for fonts |
| `ConfigEditorState` | `sdk/campaign_builder/src/config_editor.rs` L53 | SDK editor for `GameConfig`; no fonts section |
| `config.template.ron` | `campaigns/config.template.ron` | Canonical config template; no `fonts:` section |
| `data/test_campaign/config.ron` | `data/test_campaign/config.ron` | Test fixture config; no `fonts:` section |
| `campaigns/tutorial/config.ron` | `campaigns/tutorial/config.ron` | Live campaign config; no `fonts:` section |

### Identified Issues

1. `GameConfig` has no `FontConfig` block; campaign authors cannot declare custom fonts.
2. No `CampaignFontHandles` Bevy resource exists to hold resolved `Handle<Font>` values.
3. `text_style` in `ui_helpers.rs` always produces `TextFont { ..default() }` with no way to
   inject a custom font handle.
4. `spawn_dialogue_bubble` and the three menu spawn helpers use `TextFont { ..default() }`
   unconditionally; they have no access to campaign font handles.
5. `ConfigEditorState` in the Campaign Builder cannot author, validate, or persist font settings.
6. No tests exist for font config validation, handle resolution, or UI fallback.
7. `campaigns/tutorial/fonts/` directory does not exist; the loader would find no files.
8. `data/test_campaign/fonts/` directory does not exist; loading tests need it.

---

## Implementation Phases

### Phase 1: Config Schema and Data Model

**Files modified in this phase:**
- `src/sdk/game_config.rs`
- `campaigns/config.template.ron`
- `data/test_campaign/config.ron`
- `campaigns/tutorial/config.ron`

#### 1.1 Foundation Work

Add a `FontConfig` struct and a `fonts` field to `GameConfig` in
`src/sdk/game_config.rs`.

**`FontConfig` struct** — add after `LevelingConfig` (around L1165).

Fields (both default to `None`, both carry `#[serde(default)]`):

| Field | Type | RON default | Meaning |
|---|---|---|---|
| `dialogue_font` | `Option<String>` | omitted → `None` | Path to dialogue font relative to campaign root |
| `game_menu_font` | `Option<String>` | omitted → `None` | Path to game menu font relative to campaign root |

- Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`, `Default`.
- Add `#[serde(default)] pub fonts: FontConfig` to `GameConfig` (alongside the
  existing fields at L136–L163).
- `game_config.rs` already has the SPDX header; no new file is created.

#### 1.2 Add Foundation Functionality

Implement `FontConfig::validate(&self) -> Result<(), ConfigError>` with these
rules applied independently to each of `dialogue_font` and `game_menu_font`:

| Condition | Outcome |
|---|---|
| `None` | `Ok(())` — always valid |
| Non-`None` starts with `/` or contains `:\` | `Err(ConfigError::ValidationError("… must be a relative path …"))` |
| Non-`None` contains `..` | `Err(ConfigError::ValidationError("… must not contain '..' …"))` |
| Non-`None` does not start with `fonts/` | `Err(ConfigError::ValidationError("… must start with 'fonts/' …"))` |
| Non-`None` does not end with `.ttf` | `Err(ConfigError::ValidationError("… must end with '.ttf' …"))` |

Call `self.fonts.validate()?` inside `GameConfig::validate` (L230) so the error
propagates through the existing validation chain.

#### 1.3 Integrate Foundation Work

**`campaigns/config.template.ron`** — add a commented `fonts:` section after the
`game_log:` block:

```
// ========================================================================
// Custom Fonts Configuration (optional)
// ========================================================================
// fonts: FontConfig(
//     // Path to a .ttf font file relative to the campaign root.
//     // Must start with "fonts/" and end with ".ttf".
//     // Omit or set to None to use the engine default font.
//     // Omitting a font field uses the Bevy engine default font.
//     dialogue_font: Some("fonts/my_dialogue_font.ttf"),
//     game_menu_font: Some("fonts/my_menu_font.ttf"),
// ),
```

**`data/test_campaign/config.ron`** — add `fonts: FontConfig(),` (both fields
omitted; serde defaults both to `None`). Place it after the `leveling:` block.

**`campaigns/tutorial/config.ron`** — add the same `fonts: FontConfig(),` entry.

Update the doc-comment example in `GameConfig::load_or_default` (L193) to
remain accurate after the new field is added.

#### 1.4 Testing Requirements

Add the following `#[test]` functions inside the existing `mod tests` block in
`src/sdk/game_config.rs`:

| Test function name | What it asserts |
|---|---|
| `test_font_config_default_both_none` | `FontConfig::default()` has both fields `None` |
| `test_font_config_validate_none_fields_ok` | `FontConfig::default().validate()` returns `Ok(())` |
| `test_font_config_validate_valid_ttf_path_ok` | `dialogue_font: Some("fonts/foo.ttf")` passes validation |
| `test_font_config_validate_invalid_extension_fails` | `.otf` extension returns `Err` |
| `test_font_config_validate_absolute_path_fails` | Path starting with `/` returns `Err` |
| `test_font_config_validate_traversal_path_fails` | Path containing `../` returns `Err` |
| `test_font_config_validate_missing_fonts_prefix_fails` | `"myfile.ttf"` (no `fonts/` prefix) returns `Err` |
| `test_font_config_ron_roundtrip_both_none` | RON `FontConfig()` round-trips to `FontConfig::default()` |
| `test_font_config_ron_roundtrip_dialogue_only` | RON with only `dialogue_font` set deserializes correctly |
| `test_font_config_ron_roundtrip_both_set` | RON with both fields set deserializes correctly |
| `test_game_config_fonts_field_default` | `GameConfig::default().fonts == FontConfig::default()` |
| `test_game_config_fonts_defaults_when_missing_from_ron` | A RON string without `fonts:` deserializes successfully with `fonts == FontConfig::default()` |
| `test_game_config_validate_propagates_font_error` | A `GameConfig` with an invalid font path returns `Err` from `validate()` |

#### 1.5 Deliverables

- [ ] `FontConfig` struct with `dialogue_font: Option<String>` and `game_menu_font: Option<String>` added to `src/sdk/game_config.rs`
- [ ] `FontConfig::validate()` enforces relative path, no traversal, `fonts/` prefix, `.ttf` extension
- [ ] `GameConfig::validate()` calls `self.fonts.validate()?`
- [ ] `GameConfig` has `#[serde(default)] pub fonts: FontConfig`
- [ ] `campaigns/config.template.ron` documents the `fonts:` section with commented examples
- [ ] `data/test_campaign/config.ron` includes `fonts: FontConfig()`
- [ ] `campaigns/tutorial/config.ron` includes `fonts: FontConfig()`
- [ ] All 13 tests listed in §1.4 pass

#### 1.6 Success Criteria

- `config.ron` can declare optional `dialogue_font` and `game_menu_font` paths.
- Existing campaign configs that omit `fonts:` load and validate without error.
- Invalid font path formats (absolute, traversal, wrong extension, missing prefix)
  each produce a distinct, descriptive `ConfigError::ValidationError`.

---

### Phase 2: `CampaignFontHandles` Resource and Loading System

**Files modified in this phase:**
- `src/game/resources/font_handles.rs` *(new file)*
- `src/game/resources/mod.rs`
- `src/game/systems/hud.rs`

#### 2.1 Foundation Work

**Create `src/game/resources/font_handles.rs`** (new file).

First two lines must be:
```
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

Define the resource struct:

```
pub struct CampaignFontHandles {
    pub dialogue_font: Option<Handle<Font>>,
    pub game_menu_font: Option<Handle<Font>>,
    pub loaded_for_campaign: Option<String>,
}
```

- Derive `Resource`, `Default`.
- All three fields default to `None` via `#[derive(Default)]`.
- `Handle<Font>` is `bevy::prelude::Handle<bevy::prelude::Font>` loaded via
  `AssetServer::load::<Font>(path)`.
- Add `///` doc comments to the struct and each field.

**Update `src/game/resources/mod.rs`:**
- Add `pub mod font_handles;` with the existing `pub mod` declarations.
- Add `pub use font_handles::CampaignFontHandles;` in the re-export block.

#### 2.2 Add Foundation Functionality

Add `fn ensure_campaign_fonts_loaded` to `src/game/systems/hud.rs`, positioned
after `ensure_full_portraits_loaded`.

System signature:
```
fn ensure_campaign_fonts_loaded(
    global_state: Res<GlobalState>,
    asset_server: Option<Res<AssetServer>>,
    mut font_handles: ResMut<CampaignFontHandles>,
)
```

**Execution logic — in this exact order:**

1. Early-return if `global_state.0.campaign` is `None`.
2. Early-return (with `debug!` log) if `asset_server` resource is absent.
3. Early-return if `font_handles.loaded_for_campaign.as_deref() == Some(campaign.id.as_str())`.
4. For **each** of `dialogue_font` and `game_menu_font`:
   - Read the configured path from `campaign.game_config.fonts.<field>` (`Option<String>`).
   - If `None`: leave `font_handles.<field>` as `None`. Do not log.
   - If `Some(rel_path)` (e.g. `"fonts/my_font.ttf"`):
     - Call `let handle: Handle<Font> = asset_server.load(rel_path.clone());`
       (the asset server is rooted at `campaign.root_path`).
     - If `handle == Handle::default()`: log
       `warn!("ensure_campaign_fonts_loaded: failed to load '{}' for campaign '{}'; using default font", rel_path, campaign.id)`
       and leave `font_handles.<field>` as `None`.
     - Otherwise: assign `handle` to `font_handles.<field>`.
5. Set `font_handles.loaded_for_campaign = Some(campaign.id.clone())`.
6. Log `debug!("ensure_campaign_fonts_loaded: loaded fonts for campaign '{}'", campaign.id)`.

**Register in `HudPlugin::build` (`src/game/systems/hud.rs` L423):**

- Add `.init_resource::<CampaignFontHandles>()` alongside `.init_resource::<FullPortraitAssets>()`.
- Add `ensure_campaign_fonts_loaded` to the `.run_if(not_in_combat)` `Update` set
  alongside `ensure_portraits_loaded` and `ensure_full_portraits_loaded`.

Add to imports in `hud.rs`:
```
use crate::game::resources::CampaignFontHandles;
```

#### 2.3 Configuration Updates

No RON file changes in this phase. The resource reads from
`campaign.game_config.fonts` (populated in Phase 1).

#### 2.4 Testing Requirements

Add a `mod campaign_font_tests` block to `src/game/systems/hud.rs` (following
the `mod full_portrait_tests` pattern):

| Test function name | What it asserts |
|---|---|
| `test_campaign_font_handles_default_both_none` | `CampaignFontHandles::default()` has `dialogue_font: None`, `game_menu_font: None` |
| `test_campaign_font_handles_loaded_for_campaign_default_none` | `CampaignFontHandles::default().loaded_for_campaign` is `None` |
| `test_ensure_campaign_fonts_loaded_skips_when_no_campaign` | System does not modify handles when `GlobalState` has no campaign |
| `test_ensure_campaign_fonts_loaded_skips_when_no_asset_server` | System does not modify handles when `AssetServer` resource is absent |
| `test_ensure_campaign_fonts_loaded_skips_when_already_loaded` | System returns without changes when `loaded_for_campaign` already equals the active campaign id |
| `test_ensure_campaign_fonts_loaded_none_config_leaves_handles_none` | When both config font fields are `None`, both handle fields remain `None` after the system runs |
| `test_ensure_campaign_fonts_loaded_sets_loaded_for_campaign` | After the system runs (even with no configured fonts), `loaded_for_campaign` equals the campaign id |

#### 2.5 Deliverables

- [ ] `src/game/resources/font_handles.rs` created with `CampaignFontHandles` (SPDX header, `Resource`, `Default`, doc comments)
- [ ] `src/game/resources/mod.rs` exports `CampaignFontHandles`
- [ ] `ensure_campaign_fonts_loaded` system implemented in `src/game/systems/hud.rs`
- [ ] `CampaignFontHandles` registered via `.init_resource` in `HudPlugin::build`
- [ ] `ensure_campaign_fonts_loaded` registered in the `not_in_combat` `Update` set in `HudPlugin::build`
- [ ] All 7 tests listed in §2.4 pass

#### 2.6 Success Criteria

- `CampaignFontHandles` is populated at runtime from `campaign.game_config.fonts`.
- A campaign with no configured fonts produces two `None` handles — no errors logged.
- A campaign with an unreachable font path logs a `warn!` and produces a `None` handle — no crash.
- The resource is campaign-scoped: switching campaigns causes a reload.

---

### Phase 3: Font-Aware Text Style Helper and UI Integration

**Files modified in this phase:**
- `src/game/systems/ui_helpers.rs`
- `src/game/systems/dialogue_visuals.rs`
- `src/game/systems/menu.rs`
- `campaigns/config.template.ron`

#### 3.1 Feature Work — `ui_helpers.rs`

Add a new public function immediately after `text_style` (around L82) in
`src/game/systems/ui_helpers.rs`:

```
pub fn text_style_with_font(
    font: Option<Handle<Font>>,
    font_size: f32,
    color: Color,
) -> (TextFont, TextColor)
```

Behavior:
- `Some(handle)` → `(TextFont { font: handle, font_size, ..default() }, TextColor(color))`.
- `None` → `(TextFont { font_size, ..default() }, TextColor(color))` — identical
  to `text_style`.

Add `///` doc comments and a `# Examples` section with a runnable `no_run` snippet.

#### 3.2 Feature Work — `dialogue_visuals.rs`

Modify `spawn_dialogue_bubble` in `src/game/systems/dialogue_visuals.rs` (L52):

1. Add `font_handles: Option<Res<CampaignFontHandles>>` as a new system parameter.
2. At the top of the spawn block (before any `.spawn()` call), extract:
   ```
   let dialogue_font: Option<Handle<Font>> =
       font_handles.as_ref().and_then(|fh| fh.dialogue_font.clone());
   ```
3. Replace both `TextFont { ..default() }` instances:
   - Speaker name text (uses `DIALOGUE_SPEAKER_FONT_SIZE`) →
     `text_style_with_font(dialogue_font.clone(), DIALOGUE_SPEAKER_FONT_SIZE, color).0`
   - Content text (uses `DIALOGUE_CONTENT_FONT_SIZE`) →
     `text_style_with_font(dialogue_font.clone(), DIALOGUE_CONTENT_FONT_SIZE, color).0`

   Because `TextFont` and `TextColor` are separate components, use `.0` to
   extract just the `TextFont`, keeping the existing `TextColor(…)` component
   spawn unchanged.

4. Add imports:
   ```
   use crate::game::resources::CampaignFontHandles;
   use crate::game::systems::ui_helpers::text_style_with_font;
   ```

#### 3.3 Feature Work — `menu.rs`

**Step A — `menu_setup` system (L144):**

Add `font_handles: Res<CampaignFontHandles>` as a new system parameter.
Extract the menu font once at the top of the function:
```
let menu_font: Option<Handle<Font>> = font_handles.game_menu_font.clone();
```

**Step B — update the three spawn helper signatures:**

| Helper | New parameter to add |
|---|---|
| `spawn_main_menu` | `font: Option<Handle<Font>>` |
| `spawn_save_load_menu` | `font: Option<Handle<Font>>` |
| `spawn_settings_menu` | `font: Option<Handle<Font>>` |

Update each call-site in `menu_setup` to pass `menu_font.clone()`.

**Step C — replace every `TextFont { ..default() }` and `text_style(…)` call**
inside all three spawn helpers with `text_style_with_font(font.clone(), size, color)`.

Call sites to update by function:

`spawn_main_menu` (L274 title text, L343 button labels).

`spawn_save_load_menu` (L406 title, L435 "no saves" text, L475 filename via
`text_style`, L493 date, L515 party, L537 location, L580/L605/L630/L655
Save/Load/Delete/Back button labels).

`spawn_settings_menu` (L719 title, L742 "Audio Settings" header, L761/L805/L849/L893
volume labels via `text_style`, L941 "Graphics Settings" header,
L969/L994/L1023/L1048 graphics toggle buttons via `text_style`,
L1067 "Controls" header, L1086/L1103/L1120/L1140 controls key labels,
L1178/L1203/L1228 Apply/Reset/Back button labels).

Add imports to `menu.rs`:
```
use crate::game::resources::CampaignFontHandles;
use crate::game::systems::ui_helpers::text_style_with_font;
```

#### 3.4 Configuration Updates

Add one clarifying line to the commented `fonts:` section in
`campaigns/config.template.ron` (already added in Phase 1):
```
// Omitting a font field (or setting it to None) uses the Bevy engine default font.
```

No other RON changes. All runtime font selection reads from `CampaignFontHandles`.

#### 3.5 Testing Requirements

**In `src/game/systems/ui_helpers.rs`** (`mod tests`):

| Test function name | What it asserts |
|---|---|
| `test_text_style_with_font_none_uses_default` | `text_style_with_font(None, 16.0, Color::WHITE)` returns `TextFont` with default handle and `font_size == 16.0` |
| `test_text_style_with_font_some_sets_handle` | `text_style_with_font(Some(handle), 16.0, Color::WHITE).0.font == handle` |

**In `src/game/systems/dialogue_visuals.rs`** (`mod tests`):

| Test function name | What it asserts |
|---|---|
| `test_spawn_dialogue_bubble_uses_default_font_when_not_configured` | When `CampaignFontHandles` is absent or both fields are `None`, spawned `TextFont` components use the default font handle |

**In `src/game/systems/menu.rs`** (`mod tests`):

| Test function name | What it asserts |
|---|---|
| `test_menu_spawn_uses_default_font_when_not_configured` | Spawning main menu with `menu_font = None` produces entities whose `TextFont` components use the default font handle |

#### 3.6 Deliverables

- [ ] `text_style_with_font(font, font_size, color) -> (TextFont, TextColor)` added to `src/game/systems/ui_helpers.rs` with doc comment and `no_run` example
- [ ] `spawn_dialogue_bubble` accepts `Option<Res<CampaignFontHandles>>` and applies `dialogue_font` to speaker and content text
- [ ] `menu_setup` accepts `Res<CampaignFontHandles>` and extracts `game_menu_font`
- [ ] `spawn_main_menu`, `spawn_save_load_menu`, and `spawn_settings_menu` accept `font: Option<Handle<Font>>` and apply it to every `TextFont` spawn via `text_style_with_font`
- [ ] All 4 tests listed in §3.5 pass

#### 3.7 Success Criteria

- Setting `dialogue_font: Some("fonts/myface.ttf")` in `config.ron` causes all
  dialogue speaker and content text to render in that font at runtime.
- Setting `game_menu_font: Some("fonts/mymenu.ttf")` causes all main menu,
  save/load menu, and settings menu text to render in that font.
- Campaigns that omit both fields render identically to the pre-implementation baseline.

---

### Phase 4: SDK Authoring Support, Directory Structure, and Documentation

**Files modified in this phase:**
- `sdk/campaign_builder/src/config_editor.rs`
- `campaigns/tutorial/fonts/.gitkeep` *(new file — empty)*
- `data/test_campaign/fonts/.gitkeep` *(new file — empty)*
- `docs/how-to/add_custom_fonts.md` *(new file)*
- `docs/explanation/implementations.md`

#### 4.1 Feature Work — `ConfigEditorState`

**New fields** — add to `pub struct ConfigEditorState`
(`sdk/campaign_builder/src/config_editor.rs` L53) alongside the existing buffer
fields:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `pub fonts_expanded: bool` | `bool` | `false` | Collapsed/expanded state of the Fonts section |
| `pub dialogue_font_buffer: String` | `String` | `""` | Text-edit buffer for `dialogue_font` path |
| `pub game_menu_font_buffer: String` | `String` | `""` | Text-edit buffer for `game_menu_font` path |

Update `impl Default for ConfigEditorState` (L119) to initialize all three fields
to their default values shown above.

#### 4.2 Add Foundation Functionality — `show_fonts_section`

Add a private method to `impl ConfigEditorState`:

```
fn show_fonts_section(&mut self, ui: &mut egui::Ui)
```

Layout (follow the pattern of `show_leveling_section` at L985):

1. Collapsible header toggled by `self.fonts_expanded`. Call
   `ui.ctx().request_repaint()` when the expansion state changes.
2. Wrap the expanded body in `ui.push_id("fonts_section", |ui| { … })` for a
   stable egui ID scope.
3. When expanded, render two rows:
   - Row 1: label `"Dialogue Font"` + `ui.text_edit_singleline(&mut self.dialogue_font_buffer)`.
   - Row 2: label `"Game Menu Font"` + `ui.text_edit_singleline(&mut self.game_menu_font_buffer)`.
4. After each row, if the buffer is non-empty, construct a temporary `FontConfig`
   with only that field set and call `validate()`. If it returns `Err`, display
   the error message in red (`egui::Color32::RED`) below the input.

Call `self.show_fonts_section(ui)` from `show` (L193), after the
`show_leveling_section` call.

#### 4.3 Integrate Feature — Buffer Sync Methods

**`update_edit_buffers` (L1128)** — add at the end of the method body:
```
self.dialogue_font_buffer =
    self.game_config.fonts.dialogue_font.clone().unwrap_or_default();
self.game_menu_font_buffer =
    self.game_config.fonts.game_menu_font.clone().unwrap_or_default();
```

**`update_config_from_buffers` (L1157)** — add at the end of the method body:
```
self.game_config.fonts.dialogue_font =
    if self.dialogue_font_buffer.is_empty() { None }
    else { Some(self.dialogue_font_buffer.clone()) };
self.game_config.fonts.game_menu_font =
    if self.game_menu_font_buffer.is_empty() { None }
    else { Some(self.game_menu_font_buffer.clone()) };
```

**`save_config` (L1098)** — no change required. `save_config` already calls
`self.game_config.validate()`, which now calls `self.fonts.validate()?`
(added in Phase 1). Font validation is automatically included.

#### 4.4 Directory Structure

**Create `campaigns/tutorial/fonts/.gitkeep`** (empty file).

This establishes the expected campaign font directory so the
`ensure_campaign_fonts_loaded` system does not log a spurious warning on startup
when no font is configured.

**Create `data/test_campaign/fonts/.gitkeep`** (empty file).

Per AGENTS.md Implementation Rule 5, all test fixtures live under
`data/test_campaign/`. Font-loading tests must point at
`data/test_campaign/fonts/`, never `campaigns/tutorial/fonts/`.

#### 4.5 Documentation

**Create `docs/how-to/add_custom_fonts.md`** with these sections:

1. **Overview** — what `dialogue_font` and `game_menu_font` control; that both
   are optional and non-configuring preserves default Bevy font behavior.
2. **Directory layout** — place font files under `<campaign_root>/fonts/`; show
   an example directory tree.
3. **`config.ron` entry format** — exact RON syntax for each field; note that
   omitting a field equals `None`.
4. **Validation rules** — relative path only; no `..`; must start with `fonts/`;
   must end with `.ttf`.
5. **Fallback behavior** — what the engine does when a configured file is missing
   at runtime (logs `warn!`, uses default font, no crash).
6. **Authoring with the Campaign Builder** — how to use the Fonts section in the
   Config Editor tab; save and reload to persist.
7. **Testing in-game** — load the campaign, open the menu or trigger dialogue,
   confirm the font appears; troubleshooting checklist (filename casing, missing
   `fonts/` prefix, wrong extension).
8. **Acceptance checklist** — one checkbox per step, matching the format used in
   `docs/how-to/portrait_export_checklist.md`.

**Update `docs/explanation/implementations.md`** — prepend a new entry titled
"Custom Fonts — All Phases (Complete)" after all implementation work is
complete, following the standard entry format used throughout that file.

#### 4.6 Testing Requirements

Add the following `#[test]` functions inside `mod tests` in
`sdk/campaign_builder/src/config_editor.rs`:

| Test function name | What it asserts |
|---|---|
| `test_config_editor_fonts_section_defaults` | `ConfigEditorState::default()` has `fonts_expanded: false` and both buffers are `""` |
| `test_update_edit_buffers_populates_font_fields` | After setting `game_config.fonts.dialogue_font = Some("fonts/x.ttf")` and calling `update_edit_buffers`, `dialogue_font_buffer == "fonts/x.ttf"` |
| `test_update_edit_buffers_populates_font_fields_none_values` | When both config font fields are `None`, both buffers are `""` after `update_edit_buffers` |
| `test_update_config_from_buffers_sets_fonts` | After setting `dialogue_font_buffer = "fonts/x.ttf"` and calling `update_config_from_buffers`, `game_config.fonts.dialogue_font == Some("fonts/x.ttf")` |
| `test_update_config_from_buffers_empty_font_buffer_sets_none` | Empty buffer string produces `None` in the config field |
| `test_config_editor_font_round_trip` | Set buffer → `update_config_from_buffers` → `update_edit_buffers` → buffer equals original value |
| `test_config_editor_save_rejects_invalid_font_path` | Calling `save_config` on a `ConfigEditorState` with an invalid font path (e.g. absolute path `/usr/share/fonts/x.ttf`) returns `Err(ConfigEditorError::ValidationFailed(_))` |

#### 4.7 Deliverables

- [ ] `fonts_expanded: bool`, `dialogue_font_buffer: String`, `game_menu_font_buffer: String` added to `ConfigEditorState` and `impl Default`
- [ ] `show_fonts_section` method implemented and called from `show()`
- [ ] `update_edit_buffers` populates both font buffers from `game_config.fonts`
- [ ] `update_config_from_buffers` writes both font buffers back to `game_config.fonts`
- [ ] `campaigns/tutorial/fonts/.gitkeep` created
- [ ] `data/test_campaign/fonts/.gitkeep` created
- [ ] `docs/how-to/add_custom_fonts.md` created with all 8 sections
- [ ] `docs/explanation/implementations.md` updated
- [ ] All 7 tests listed in §4.6 pass

#### 4.8 Success Criteria

- A campaign author can open the Campaign Builder Config Editor, type
  `fonts/myfont.ttf` in the Dialogue Font field, save, and reload — the value
  persists in `config.ron` and loads at game runtime.
- Typing an invalid path shows an inline red validation error and `save_config`
  returns `Err`.
- Documentation and editor behavior match runtime behavior exactly.

---

## Architecture Compliance Checklist

- [ ] All new `.rs` files carry the SPDX copyright + license header as first two lines
- [ ] All new public items have `///` doc comments with runnable examples where applicable
- [ ] `Handle<Font>` (not `Handle<Image>`) used for font assets throughout
- [ ] `CampaignFontHandles` follows the `PortraitAssets` / `FullPortraitAssets` resource and loading-system pattern exactly
- [ ] No test references `campaigns/tutorial/fonts/` — all font fixture data uses `data/test_campaign/fonts/`
- [ ] `data/test_campaign/config.ron` includes `fonts: FontConfig()`
- [ ] `campaigns/tutorial/config.ron` includes `fonts: FontConfig()`
- [ ] `campaigns/tutorial/fonts/.gitkeep` exists
- [ ] `data/test_campaign/fonts/.gitkeep` exists
- [ ] `cargo fmt --all` → zero output
- [ ] `cargo check --all-targets --all-features` → zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → zero warnings
- [ ] `cargo nextest run --all-features` → all pass
