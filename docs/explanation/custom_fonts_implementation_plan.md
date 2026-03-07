# Custom Fonts Implementation Plan

## Overview

This plan adds campaign-scoped custom font support for dialogue UI and game menu
UI. The implementation extends `config.ron` with optional font path settings,
loads campaign-local `.ttf` assets from `./campaigns/<campaign name>/fonts/`,
and applies those fonts at runtime only when configured. If a campaign omits a
custom font entry, the existing default Bevy font behavior remains unchanged.

## Current State Analysis

### Existing Infrastructure

- [`GameConfig`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs#L119)
  in [`src/sdk/game_config.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs)
  owns the `config.ron` schema and validation path used by campaign configs.
- [`CampaignLoader::load_campaign`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/campaign_loader.rs#L400)
  in [`src/sdk/campaign_loader.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/campaign_loader.rs)
  already loads campaign `config.ron` into `campaign.game_config`.
- [`spawn_dialogue_bubble`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs#L52)
  in [`src/game/systems/dialogue_visuals.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs)
  creates dialogue `TextFont` components using default font handles.
- [`menu_setup`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/menu.rs#L140)
  and menu spawn helpers in
  [`src/game/systems/menu.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/menu.rs)
  create menu `TextFont` components using default font handles.
- [`ConfigEditorState`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs#L37)
  in
  [`sdk/campaign_builder/src/config_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs)
  already edits `GameConfig`, but it has no font-specific section or fields.
- [`campaigns/config.template.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/config.template.ron)
  and
  [`data/test_campaign/config.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/data/test_campaign/config.ron)
  provide the canonical config examples and test fixture.

### Identified Issues

- `GameConfig` has no font configuration block, so campaign authors cannot
  declare custom font asset paths.
- Dialogue and menu UI systems do not resolve campaign-specific font handles;
  they rely on `TextFont { ..default() }`.
- There is no shared runtime resource or helper that maps configured font paths
  to loaded `Handle<Font>` values.
- Campaign Builder cannot author, validate, or persist font settings.
- There are no tests for optional custom font fallback, invalid path handling,
  or campaign-local font asset resolution.

## Implementation Phases

### Phase 1: Config Schema and Data Model

#### 1.1 Foundation Work

- Extend [`GameConfig`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs#L119)
  with a new font configuration block, for example `fonts: FontConfig`.
- Add a new public `FontConfig` struct in
  [`src/sdk/game_config.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs)
  with two optional fields:
  `dialogue_font: Option<String>` and `game_menu_font: Option<String>`.
- Use `#[serde(default)]` on the new block and fields so existing `config.ron`
  files continue to deserialize without requiring font entries.
- Keep the config model strictly optional. An omitted field means "use the
  current default engine font."

#### 1.2 Add Foundation Functionality

- Add `Default` and `validate()` behavior for `FontConfig` in
  [`src/sdk/game_config.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs).
- Validation should enforce:
  `None` is valid, configured paths are relative paths, configured paths point
  under the campaign `fonts/` directory, and configured filenames end in
  `.ttf`.
- Decide and document one canonical stored path format in `config.ron`:
  either `fonts/<font-name>.ttf` or `<font-name>.ttf`. The runtime should not
  accept arbitrary absolute paths.

#### 1.3 Integrate Foundation Work

- Update [`campaigns/config.template.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/config.template.ron)
  to include the new `fonts:` section with commented examples.
- Update
  [`data/test_campaign/config.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/data/test_campaign/config.ron)
  to include either explicit `None` coverage or a representative configured
  value if fixture assets are added.
- Update doc comments and examples in
  [`src/sdk/game_config.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs)
  so `GameConfig::load_or_default` examples remain accurate.

#### 1.4 Testing Requirements

- Add unit tests in
  [`src/sdk/game_config.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs)
  for:
  missing `fonts` block, partial `fonts` block, valid `.ttf` paths, invalid
  extensions, invalid absolute paths, and invalid parent-directory traversal.
- Add RON round-trip tests confirming that omitted font fields deserialize to
  default fallback behavior.

#### 1.5 Deliverables

- [ ] `FontConfig` added to `GameConfig`
- [ ] Backward-compatible RON deserialization for missing font fields
- [ ] `config.template.ron` documents custom font usage
- [ ] `data/test_campaign/config.ron` covers the new schema

#### 1.6 Success Criteria

- `config.ron` can declare optional dialogue and menu font paths.
- Existing campaigns without font settings still load unchanged.
- Invalid font path formats fail validation with explicit errors.

### Phase 2: Runtime Font Resolution and Fallback

#### 2.1 Foundation Work

- Add a game-layer font resource, such as `CampaignFontHandles`, under
  [`src/game/resources/`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/resources)
  to store resolved optional `Handle<Font>` values for dialogue and menu text.
- Add a single helper responsible for converting configured campaign font paths
  into Bevy asset paths rooted at the active campaign directory.

#### 2.2 Add Foundation Functionality

- During campaign startup or campaign load completion, read
  `campaign.game_config.fonts` and attempt to load the configured font handles
  via `AssetServer`.
- Resolve paths relative to the active campaign root so a config entry maps to
  `./campaigns/<campaign name>/fonts/<font-name>.ttf` at runtime.
- Treat missing config values as `None` and missing assets as a logged warning
  plus fallback to default font behavior, not a fatal startup error, unless the
  project explicitly prefers strict validation.

#### 2.3 Integrate Foundation Work

- Thread the resolved font resource into the same Bevy app path that already
  consumes `GlobalState` and campaign assets.
- Keep resolution logic centralized; dialogue and menu systems should consume
  pre-resolved handles instead of reconstructing asset paths inline.

#### 2.4 Testing Requirements

- Add runtime-focused tests for the path-resolution helper:
  configured relative path, missing config value, invalid traversal attempt, and
  fallback behavior when the asset cannot be resolved.
- If Bevy asset-server tests are practical in this repo, add one integration
  test proving a campaign-local `.ttf` path is turned into a non-default
  `Handle<Font>`.

#### 2.5 Deliverables

- [ ] Shared runtime font resource exists
- [ ] Campaign config paths resolve through one helper
- [ ] Missing or invalid configured assets fall back safely

#### 2.6 Success Criteria

- Active campaigns expose optional dialogue and menu font handles at runtime.
- Font resolution is deterministic and campaign-scoped.
- Fallback behavior is explicit, logged, and non-breaking.

### Phase 3: Dialogue and Menu UI Integration

#### 3.1 Feature Work

- Update [`spawn_dialogue_bubble`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs#L52)
  in
  [`src/game/systems/dialogue_visuals.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs)
  to apply the configured dialogue font handle to speaker and content text when
  present.
- Update menu spawn helpers in
  [`src/game/systems/menu.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/menu.rs)
  to apply the configured game menu font handle to title text, button text, and
  save/load text where appropriate.

#### 3.2 Integrate Feature

- Standardize font selection behind a small helper or constructor so every
  `TextFont` instantiation follows the same rule:
  configured custom handle first, engine default handle otherwise.
- Decide whether save/load screens, settings screens, and dialogue choice text
  all count as "game menu" text. Document the rule and apply it consistently.

#### 3.3 Configuration Updates

- Ensure runtime systems read from `GlobalState` or the new font resource only;
  they should not parse config files directly.
- Add comments to
  [`campaigns/config.template.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/config.template.ron)
  that state the fallback rule clearly:
  no configured font means default font.

#### 3.4 Testing Requirements

- Add UI-oriented tests in
  [`src/game/systems/dialogue_visuals.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs)
  and
  [`src/game/systems/menu.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/menu.rs)
  asserting that configured handles are attached to spawned `TextFont`
  components when present.
- Add counterpart tests asserting default font behavior remains in effect when no
  custom font is configured.

#### 3.5 Deliverables

- [ ] Dialogue UI uses configured dialogue font when present
- [ ] Menu UI uses configured game menu font when present
- [ ] All affected text systems preserve default fallback behavior

#### 3.6 Success Criteria

- Campaign authors can visibly override dialogue text without affecting menu
  text.
- Campaign authors can visibly override menu text without affecting dialogue
  text.
- Default font behavior remains unchanged for campaigns that omit font config.

### Phase 4: SDK Authoring Support and Documentation

#### 4.1 Feature Work

- Extend
  [`sdk/campaign_builder/src/config_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs#L37)
  with a new fonts section and edit fields for `dialogue_font` and
  `game_menu_font`.
- Follow `sdk/AGENTS.md` egui ID rules for any new controls:
  every loop uses `push_id`, every `ScrollArea` has `id_salt`, every
  `ComboBox` uses `from_id_salt`, and layout-driving state changes call
  `request_repaint()`.

#### 4.2 Integrate Feature

- Add field-level validation messaging in Campaign Builder for path format and
  `.ttf` requirements.
- Decide whether the editor should offer free-text input only or also enumerate
  files found under `<campaign>/fonts/`; if file enumeration is added, keep it
  campaign-local and non-destructive.

#### 4.3 Configuration Updates

- Update
  [`campaigns/config.template.ron`](/Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/config.template.ron)
  and any relevant campaign README guidance to show the expected folder layout:
  `campaigns/<campaign>/fonts/<font-name>.ttf`.
- Update
  [`docs/explanation/implementations.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/implementations.md)
  after implementation is complete, per repo rules.

#### 4.4 Testing Requirements

- Add Campaign Builder tests covering load, edit, save, and reload behavior for
  the new font fields.
- If the editor auto-discovers files, add tests for empty `fonts/` directories
  and for preserving manually entered values.

#### 4.5 Deliverables

- [ ] Campaign Builder can edit and save both custom font fields
- [ ] Template docs explain font folder layout and fallback behavior
- [ ] Implementation summary requirements are captured for completion

#### 4.6 Success Criteria

- Campaign authors can configure custom fonts without hand-editing RON unless
  they choose to.
- Documentation and editor behavior match runtime behavior exactly.

## Recommended Implementation Order

1. Phase 1: Config Schema and Data Model
2. Phase 2: Runtime Font Resolution and Fallback
3. Phase 3: Dialogue and Menu UI Integration
4. Phase 4: SDK Authoring Support and Documentation

## Open Questions

1. What exact `config.ron` value should be stored for each font path:
   `fonts/<font>.ttf` or just `<font>.ttf`?
2. Should an invalid configured font asset fail campaign loading, or warn and
   fall back to the default font?
3. Does "game menu font" apply only to pause/menu screens, or to every non-dialogue
   UI text surface built in [`src/game/systems/menu.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/menu.rs)?
