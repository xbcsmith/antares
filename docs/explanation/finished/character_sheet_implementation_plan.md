# Character Sheet Implementation Plan

## Overview

This plan adds three user-facing features to the existing character sheet
infrastructure: (1) configurable number-key selection (1–6) to open and switch
characters, (2) HUD portrait click to open the sheet for the clicked character
(including from within Combat mode), and (3) a full-length portrait image from
`<campaign>/assets/portraits/full/` inside the single-character view.
A fifth polishing phase adds the currently missing Resistances and basic
character-info sections to complete the stat display.

The core `CharacterSheetState`, `CharacterSheetPlugin`, `GameMode::CharacterSheet`,
and the `P`-key toggle are already implemented and fully tested. All work below
builds on top of that foundation without breaking it.

**Design Decisions (locked):**

| # | Question | Decision |
|---|---|---|
| 1 | Number-key selection configurable or hardcoded? | **Configurable** — new `ControlsConfig` fields + `GameAction::SelectCharacter(usize)` |
| 2 | Full-length portrait path? | **Separate sub-directory** — `<campaign_root>/assets/portraits/full/` |
| 3 | Portrait click allowed in Combat? | **Yes** — read-only sheet opens and returns to Combat on close |

## Current State Analysis

### Existing Infrastructure

| File | What Is Already Done |
|---|---|
| `src/application/character_sheet_state.rs` | `CharacterSheetState`, `CharacterSheetView`, `focus_next/prev`, `toggle_view`, full test suite |
| `src/application/mod.rs` — `GameMode::CharacterSheet` | Variant registered; `enter_character_sheet()` opens at `focused_index = 0` |
| `src/game/systems/character_sheet_ui.rs` | `CharacterSheetPlugin` with input (Esc / Tab / Arrow / O) and egui render (Single + PartyOverview) |
| `src/game/systems/hud.rs` — `CharacterPortrait` | Head portrait rendered in HUD card; asset loading via `PortraitAssets`; **no `Button`/`Interaction` component** |
| `src/game/systems/input/frame_input.rs` — `FrameInputIntent` | All toggles including `character_sheet_toggle`; **no `character_select` field** |
| `src/game/systems/input/global_toggles.rs` | Routes `character_sheet_toggle` to `enter_character_sheet()`; **no number-key routing** |
| `src/game/systems/input/keymap.rs` — `GameAction` | `CharacterSheet` action; `P` default key; **no `SelectCharacter` variant** |
| `src/sdk/game_config.rs` — `ControlsConfig` | `character_sheet: Vec<String>` field; **no character-select-1 through 6 fields** |

### Identified Issues

1. Opening the sheet via `P` always focuses `party_index = 0`; there is no way
   to open it directly to a specific character.
2. Number-key selection (1–6) does not exist at all — no config fields, no
   `GameAction` variants, no input decoding.
3. `CharacterPortrait` Bevy UI nodes have no `Button`/`Interaction` components,
   so portrait clicks are silently ignored.
4. Portrait click is not handled in `Combat` mode.
5. The single-character view contains only text panels; there is no portrait
   image rendering inside the sheet.
6. `<campaign>/assets/portraits/full/` is never scanned or loaded.
7. `Resistances` (magic, fire, cold, electricity, acid, fear, poison, psychic)
   are not displayed on the sheet.
8. Basic character identity fields (Sex, Alignment, Age, personal Gold/Gems)
   are absent from the sheet.

---

## Implementation Phases

### Phase 1: Configurable Number-Key Character Selection (1–6)

#### 1.1 New `GameAction::SelectCharacter(usize)` Variant

In [`src/game/systems/input/keymap.rs`](../../src/game/systems/input/keymap.rs):

- Add variant `SelectCharacter(usize)` to `GameAction` where the inner value
  is the **0-based party index** (i.e. the key labelled `1` selects index `0`).
- `GameAction` must remain `Clone`, `PartialEq`, `Debug`.  Because `usize` is
  `Copy + PartialEq`, these derives continue to work unchanged.
- In `KeyMap::from_controls_config`, register all six select bindings:
  ```
  insert_action_bindings(&mut bindings, &config.character_select_1, GameAction::SelectCharacter(0));
  insert_action_bindings(&mut bindings, &config.character_select_2, GameAction::SelectCharacter(1));
  // … through SelectCharacter(5)
  ```
- `is_action_just_pressed` already works by equality comparison; the data
  variant compares both tag and inner value correctly.

#### 1.2 New `ControlsConfig` Fields in `src/sdk/game_config.rs`

Add six fields to `ControlsConfig`, each with a `#[serde(default)]` attribute
and a private default function:

| Field | Default keys | Notes |
|---|---|---|
| `character_select_1: Vec<String>` | `["1"]` | Selects party slot 1 (index 0) |
| `character_select_2: Vec<String>` | `["2"]` | Selects party slot 2 (index 1) |
| `character_select_3: Vec<String>` | `["3"]` | Selects party slot 3 (index 2) |
| `character_select_4: Vec<String>` | `["4"]` | Selects party slot 4 (index 3) |
| `character_select_5: Vec<String>` | `["5"]` | Selects party slot 5 (index 4) |
| `character_select_6: Vec<String>` | `["6"]` | Selects party slot 6 (index 5) |

- Add validation in `ControlsConfig::validate` that none of the six lists is
  empty (consistent with the existing empty-key-list check for other actions).
- Add the six fields to `impl Default for ControlsConfig`.
- Update `data/test_campaign/config.ron` with all six keys so validation passes
  (or confirm `#[serde(default)]` handles the missing-field case — it does, but
  the tutorial campaign's `config.ron` must also be updated for SDK validation).

#### 1.3 Extend `FrameInputIntent`

In [`src/game/systems/input/frame_input.rs`](../../src/game/systems/input/frame_input.rs):

- Add field `pub character_select: Option<usize>` (0-based index).
- Default: `None`.
- In `decode_frame_input`, iterate `0..6` and find the first index `i` for
  which `key_map.is_action_just_pressed(GameAction::SelectCharacter(i),
  keyboard_input)` is true; assign `character_select: result`.

#### 1.4 Route `character_select` in `handle_global_mode_toggles`

In [`src/game/systems/input/global_toggles.rs`](../../src/game/systems/input/global_toggles.rs):

- After the `character_sheet_toggle` block, add a `character_select` block.
- Allowed source modes: `Exploration`, `Automap`, `Inventory`, `SpellBook`,
  `GameLog` (same set already allowed for `character_sheet_toggle`).
- **Blocked** in `Combat`, `Dialogue`, `Training`, `MerchantInventory`
  (number-key input conflicts with combat UI; use portrait click for in-combat
  access — see Phase 2).
- When in an allowed mode, call `GameState::enter_character_sheet_at(index)`.
- When already in `CharacterSheet`, update `cs.focused_index` in-place (no
  mode transition; just an index update).

#### 1.5 Add `GameState::enter_character_sheet_at`

In [`src/application/mod.rs`](../../src/application/mod.rs) `impl GameState`:

- Add `pub fn enter_character_sheet_at(&mut self, index: usize)`.
- If already in `CharacterSheet` mode, set `cs.focused_index = index.min(party.members.len().saturating_sub(1))` and return early.
- Otherwise, clone `self.mode` as `prev`, create `CharacterSheetState::new(prev)`,
  set `focused_index = index.min(...)`, then assign to `self.mode`.
- Clamping keeps out-of-range keys safe when the party has fewer than six
  members.

#### 1.6 Configurable Key Switching inside `character_sheet_input_system`

In [`src/game/systems/character_sheet_ui.rs`](../../src/game/systems/character_sheet_ui.rs)
`character_sheet_input_system`:

- Accept `key_map: Option<Res<KeyMap>>` in addition to the existing `keyboard`
  resource (pattern already used by other input systems).
- After the arrow-key block, if a `KeyMap` is available, iterate
  `GameAction::SelectCharacter(0..6)` using `is_action_just_pressed`; on match,
  clamp the index to `party_size` and set `cs.focused_index`.
- This handles switching while the sheet is already open; `global_toggles`
  handles opening from outside.

#### 1.7 Update `data/test_campaign/config.ron`

Add the six new keys to the `controls` section of
`data/test_campaign/config.ron` so that `cargo nextest run` does not trip the
`character_select_*` validation check:

```
character_select_1: ["1"],
character_select_2: ["2"],
character_select_3: ["3"],
character_select_4: ["4"],
character_select_5: ["5"],
character_select_6: ["6"],
```

Also update `campaigns/tutorial/config.ron` with the same keys (this is the
live campaign config, not a test fixture; it must stay in sync).

#### 1.8 Testing Requirements

- `test_game_action_select_character_variants_exist` (sanity: 0–5 are distinct)
- `test_controls_config_character_select_defaults`
- `test_controls_validation_empty_character_select_key_fails`
- `test_frame_input_intent_default_character_select_is_none`
- `test_decode_frame_input_character_select_1_fires_on_digit1`
- `test_decode_frame_input_character_select_6_fires_on_digit6`
- `test_decode_frame_input_custom_character_select_key`
- `test_handle_global_mode_toggles_character_select_opens_sheet_at_index`
- `test_handle_global_mode_toggles_character_select_ignored_in_combat`
- `test_handle_global_mode_toggles_character_select_switches_index_when_in_sheet`
- `test_enter_character_sheet_at_sets_focused_index`
- `test_enter_character_sheet_at_clamps_to_party_size`
- `test_enter_character_sheet_at_when_already_open_updates_index`
- `test_character_sheet_input_configured_digit_key_switches_focused_index`

#### 1.9 Deliverables

- [ ] `GameAction::SelectCharacter(usize)` variant in `keymap.rs`
- [ ] Six `character_select_1`–`character_select_6` fields in `ControlsConfig`
- [ ] `ControlsConfig::validate` rejects empty select-key lists
- [ ] `character_select: Option<usize>` field in `FrameInputIntent`
- [ ] `decode_frame_input` resolves configured select keys
- [ ] `GameState::enter_character_sheet_at(index: usize)`
- [ ] `handle_global_mode_toggles` routes `character_select`
- [ ] `character_sheet_input_system` uses `KeyMap` for in-sheet switching
- [ ] `data/test_campaign/config.ron` updated with six select keys
- [ ] `campaigns/tutorial/config.ron` updated with six select keys
- [ ] All tests listed above pass

#### 1.10 Success Criteria

Pressing `1` in Exploration opens the sheet showing party member 0. Pressing
`3` while the sheet is open switches to member 2. Keys can be rebound via
`config.ron`. Keys beyond party size are silently clamped. All existing tests
still pass.

---

### Phase 2: HUD Portrait Click → Open Character Sheet (Including Combat)

#### 2.1 Add Interactivity to `CharacterPortrait` Nodes

In [`src/game/systems/hud.rs`](../../src/game/systems/hud.rs)
`setup_hud` — the `CharacterPortrait` spawn block:

- Add `Button` and `Interaction(Interaction::None)` components to the portrait
  `Node` entity alongside the existing `CharacterPortrait`, `ImageNode`, and
  `BackgroundColor`.
- No visual change is required; `Button` opts the node into Bevy's interaction
  pipeline, providing cursor-style changes on hover as a bonus.

#### 2.2 New `handle_portrait_click_system`

Add a new system `handle_portrait_click_system` to
[`src/game/systems/hud.rs`](../../src/game/systems/hud.rs):

- System signature:
  - `Query<(&CharacterPortrait, Ref<Interaction>)>`
  - `Option<Res<ButtonInput<MouseButton>>>`
  - `ResMut<GlobalState>`
- Compute `mouse_just_pressed` via `mouse_input::mouse_just_pressed`.
- For each portrait entity, call `mouse_input::is_activated` with the portrait's
  `Interaction` ref.
- On activation, check the current mode.  **Allowed modes:**
  `Exploration`, `Automap`, `Inventory`, `SpellBook`, `GameLog`, and
  **`Combat(_)`**.
  **Blocked modes:** `Dialogue`, `Training`, `MerchantInventory`,
  `ContainerInventory`, `TempleService` (UI conflict).
- Call `global_state.0.enter_character_sheet_at(portrait.party_index)`.
- Because `enter_character_sheet_at` stores the previous mode (including
  `Combat(_)`) inside `CharacterSheetState::previous_mode`, Esc will correctly
  return the player to combat after viewing the sheet.
- Register the system in `HudPlugin::build` in the `Update` schedule **without**
  the `not_in_combat` run condition so it fires during combat frames.

#### 2.3 Testing Requirements

- `test_handle_portrait_click_opens_sheet_in_exploration`
- `test_handle_portrait_click_opens_sheet_in_combat`
- `test_handle_portrait_click_ignored_in_dialogue`
- `test_handle_portrait_click_ignored_in_training`
- `test_handle_portrait_click_selects_correct_party_index`
- `test_handle_portrait_click_when_already_in_sheet_updates_index`
- `test_close_sheet_from_combat_returns_to_combat` (verify `previous_mode` round-trip)

#### 2.4 Deliverables

- [ ] `Button` + `Interaction` added to `CharacterPortrait` spawn in `setup_hud`
- [ ] `handle_portrait_click_system` implemented and registered (no `not_in_combat` guard)
- [ ] Tests pass, including the new `combat` cases

#### 2.5 Success Criteria

Clicking any HUD portrait during exploration, automap, or combat opens the
character sheet focused on that party member. Closing the sheet with Esc returns
to the exact mode that was active before (including back to the active combat
turn). Clicking in dialogue or training does nothing.

---

### Phase 3: Full-Length Portrait Asset Loading

#### 3.1 Asset Path Convention

Full-length (head-to-feet) portraits live in a dedicated sub-directory:

```
<campaign_root>/assets/portraits/full/<portrait_id>.png
```

The key is the file stem lowercased with spaces replaced by `_`, matching the
existing head-portrait convention. A character with `portrait_id = "aldric"`
resolves to `assets/portraits/full/aldric.png`. The head-portrait directory
(`assets/portraits/`) is unchanged. When no full portrait file exists for a
character, the sheet renders a deterministic color placeholder via the existing
`get_portrait_color` helper.

#### 3.2 Add `FullPortraitAssets` Resource

In [`src/game/systems/hud.rs`](../../src/game/systems/hud.rs):

- Add `pub struct FullPortraitAssets` mirroring `PortraitAssets`:
  - `pub handles_by_name: HashMap<String, Handle<Image>>`
  - `pub fallback: Handle<Image>`
  - `pub loaded_for_campaign: Option<String>`
- Derive `Resource` and `Default`.
- Register via `app.init_resource::<FullPortraitAssets>()` in `HudPlugin::build`.

#### 3.3 Add `ensure_full_portraits_loaded` System

In [`src/game/systems/hud.rs`](../../src/game/systems/hud.rs):

- Duplicate the logic of `ensure_portraits_loaded` but:
  - Target `<campaign_root>/assets/portraits/full/`
  - Write to `ResMut<FullPortraitAssets>` instead of `ResMut<PortraitAssets>`
- Early-return without error when the `full/` sub-directory does not exist.
- Register in `HudPlugin::build` with the same scheduling constraints as
  `ensure_portraits_loaded`.

#### 3.4 Test Campaign Fixture

In [`data/test_campaign`](../../data/test_campaign):

- Create the stub directory `data/test_campaign/assets/portraits/full/`
  (empty is sufficient; the loader must not panic on an empty directory).

#### 3.5 Testing Requirements

- `test_full_portrait_assets_default_is_empty`
- `test_ensure_full_portraits_loaded_graceful_on_missing_directory`
- `test_ensure_full_portraits_loaded_graceful_on_empty_directory`
- `test_ensure_full_portraits_loaded_indexes_png_file` (temp-dir fixture with one file)
- `test_ensure_full_portraits_loaded_skips_non_image_files`

#### 3.6 Deliverables

- [ ] `FullPortraitAssets` resource declared and registered
- [ ] `ensure_full_portraits_loaded` system declared and registered
- [ ] `data/test_campaign/assets/portraits/full/` directory created
- [ ] Tests pass

#### 3.7 Success Criteria

`FullPortraitAssets` is populated at runtime from `assets/portraits/full/`.
An absent or empty directory produces no errors. Handles are keyed by lowercased
file stem. Head portraits in `assets/portraits/` are unaffected.

---

### Phase 4: Full-Length Portrait Rendering in the Character Sheet

#### 4.1 Pass `FullPortraitAssets` into `character_sheet_ui_system`

In [`src/game/systems/character_sheet_ui.rs`](../../src/game/systems/character_sheet_ui.rs):

- Add `full_portraits: Option<Res<FullPortraitAssets>>` to the system
  parameter list.
- Resolve the focused character's portrait key (lowercased `portrait_id`; fall
  back to lowercased `name` when `portrait_id` is empty — same logic as the HUD
  head-portrait lookup).
- If a handle is found and loaded, call `EguiContexts::add_image(handle)` to
  obtain an `egui::TextureId`.  This call is idempotent; egui caches by
  `AssetId`.
- Pass `Option<egui::TextureId>` and `portrait_key: &str` into
  `render_single_view`.

#### 4.2 Redesign `render_single_view` Layout

Update `render_single_view` in
[`src/game/systems/character_sheet_ui.rs`](../../src/game/systems/character_sheet_ui.rs)
to a **left-portrait + right-stats** layout:

- **Left column** (~180 px wide, `ui.allocate_ui(egui::vec2(180.0, 0.0), ...)`):
  - If a `TextureId` is available: render
    `egui::Image::from_texture((texture_id, egui::vec2(170.0, 280.0)))`.
  - If no portrait: use `ui.painter().rect_filled(rect, 4.0,
    get_portrait_color(portrait_key))` for the same 170×280 area, then overlay
    the character's initials (first letter of first and last name tokens) using
    `ui.painter().text(...)` in white.
  - Below the portrait area: character name in bold (`TITLE_COLOR`), race /
    class / level on the next line, then Sex and Alignment.

- **Right column** (remaining width): unchanged from the current two-sub-column
  layout (Core Stats | Conditions left, Combat | XP | Equipment | Proficiencies
  right).

`CharacterSheetView::PartyOverview` is unaffected.

#### 4.3 Update Hint Bar

Add `[1-6] Select` to the hint bar to document Phase 1's new navigation:

```
[Esc] Close  [Tab/→] Next  [Shift+Tab/←] Prev  [1-6] Select  [O] Toggle View
```

#### 4.4 Testing Requirements

- `test_render_single_view_placeholder_when_no_full_portrait`
- `test_render_single_view_hint_bar_contains_1_6_select`
- `test_character_sheet_ui_system_accepts_full_portrait_assets_resource`

#### 4.5 Deliverables

- [ ] `character_sheet_ui_system` accepts and uses `FullPortraitAssets`
- [ ] `render_single_view` left column renders portrait or colored-placeholder
- [ ] Hint bar updated with `[1-6] Select`
- [ ] Tests pass

#### 4.6 Success Criteria

When a full-length portrait PNG exists in `assets/portraits/full/` for a
character, it displays in the left column at runtime. When absent, a colored
placeholder with the character's initials is shown. No crash or panic in either
case.

---

### Phase 5: Resistances and Character Info Section

#### 5.1 Add `Resistances` Section

In `render_single_view`
([`src/game/systems/character_sheet_ui.rs`](../../src/game/systems/character_sheet_ui.rs)):

- Add a **Resistances** section below Equipment in the right column.
- Display all eight `Resistances` fields from `Character.resistances`:
  magic, fire, cold, electricity, acid, fear, poison, psychic.
- Zero values rendered in `STAT_EMPTY_COLOR` (grey).
- Non-zero values rendered in `STAT_MODIFIED_COLOR` (amber) for visual
  prominence.
- Reuse `render_u8_row` or add a dedicated `render_resistance_row` helper if
  the zero/non-zero coloring logic needs to differ from the stat-row helper.

#### 5.2 Expand Character Identity Block in Left Column

In the left column, replace the simple name/race/class/level label added in
Phase 4 with a fuller **About** block:

| Field | Source |
|---|---|
| Sex | `character.sex` — display variant name |
| Alignment | `character.alignment` |
| Age | `character.age` years, `character.age_days` days |
| Gold | `character.gold` |
| Gems | `character.gems` |

#### 5.3 Testing Requirements

- `test_render_resistances_zero_uses_empty_color`
- `test_render_resistances_nonzero_uses_modified_color`
- `test_render_about_section_displays_sex_alignment_age`

#### 5.4 Deliverables

- [ ] `Resistances` section rendered in `render_single_view` right column
- [ ] Expanded identity block (Sex, Alignment, Age, Gold, Gems) in left column
- [ ] Tests pass
- [ ] `docs/explanation/implementations.md` updated

#### 5.5 Success Criteria

All eight resistance values appear on the character sheet. Non-zero resistances
stand out in amber. The character identity block gives the player complete
information without opening a separate screen.

---

## Architecture Compliance Checklist

- [ ] All new `.rs` files carry SPDX copyright header
- [ ] All new public items have `///` doc comments with runnable examples
- [ ] Type aliases (`CharacterId`, etc.) used where applicable — no raw `u32`/`usize` IDs
- [ ] Constants used (`PARTY_MAX_SIZE`, `get_portrait_color`) — no magic numbers
- [ ] `AttributePair` pattern respected (read `.base` and `.current`, never mutate stats here)
- [ ] No test references `campaigns/tutorial` — all fixtures use `data/test_campaign`
- [ ] `data/test_campaign/config.ron` includes all six `character_select_*` keys
- [ ] `cargo fmt --all` → zero output
- [ ] `cargo check --all-targets --all-features` → zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → zero warnings
- [ ] `cargo nextest run --all-features` → all pass
