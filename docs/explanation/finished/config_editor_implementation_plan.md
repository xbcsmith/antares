# Config Editor Implementation Plan

## Overview

Add a Config tab to the Campaign Builder SDK that allows campaign authors to visually edit `config.ron` files. The editor will follow the existing editor patterns (e.g., `SpellsEditorState`, `ItemsEditorState`) and leverage the existing `GameConfig` struct from `src/sdk/game_config.rs`.

## Current State Analysis

### Existing Infrastructure

- **GameConfig Structure**: [`src/sdk/game_config.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/game_config.rs) defines `GameConfig` with four sub-configs:

  - `GraphicsConfig`: resolution, fullscreen, vsync, msaa_samples, shadow_quality
  - `AudioConfig`: master_volume, music_volume, sfx_volume, ambient_volume, enable_audio
  - `ControlsConfig`: move_forward, move_back, turn_left, turn_right, interact, menu, movement_cooldown
  - `CameraConfig`: mode, eye_height, fov, near_clip, far_clip, smooth_rotation, rotation_speed, light_height, light_intensity, light_range, shadows_enabled

- **Editor Pattern**: All editors follow a consistent pattern:

  - `*EditorState` struct with `mode`, `search_query`, `edit_buffer`, etc.
  - `new()` and `default()` constructors
  - `show()` method accepting `&mut egui::Ui` plus data references
  - Toolbar integration via `EditorToolbar`, save/load/reload actions

- **Tab System**: [`main.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/main.rs#L232-L246) defines `EditorTab` enum and dispatches to editor `show()` methods

### Identified Issues

1. **No Config Editor**: Campaign builders cannot visually edit `config.ron`
2. **Fixed config.ron Location**: Config is always at `<campaign>/config.ron`, not configurable via metadata

---

## Implementation Phases

### Phase 1: Core ConfigEditor Implementation

#### 1.1 Create config_editor.rs

**File:** [NEW] [`config_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs)

Create new editor module with:

- `ConfigEditorState` struct holding:
  - `game_config: GameConfig` (edit buffer)
  - `has_loaded: bool` (track if config was loaded)
  - Section-specific UI state (collapsed/expanded sections)
- `Default` and `new()` implementations
- `show()` method with signature matching other editors
- Section editors for each sub-config:
  - `show_graphics_section()`
  - `show_audio_section()`
  - `show_controls_section()`
  - `show_camera_section()`
- `save_config()` and `load_config()` methods

#### 1.2 Add EditorTab::Config Variant

**File:** [MODIFY] [`main.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/main.rs)

- Add `Config` variant to `EditorTab` enum (around line 232)
- Add `"Config"` case to `EditorTab::name()` method
- Add `Config` to tab list array (around line 2740)
- Add dispatch case in tab content match (around line 2787)

#### 1.3 Add ConfigEditorState to CampaignBuilderApp

**File:** [MODIFY] [`main.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/main.rs)

- Import `config_editor::ConfigEditorState`
- Add `config_editor_state: ConfigEditorState` field to `CampaignBuilderApp`
- Add `game_config: GameConfig` field to hold loaded config
- Initialize in `Default` impl

#### 1.4 Testing Requirements

- Unit tests for `ConfigEditorState::new()` and `default()`
- Tests for validation error display
- Tests for save/load round-trip

#### 1.5 Deliverables

- [ ] `sdk/campaign_builder/src/config_editor.rs` created
- [ ] `EditorTab::Config` variant added
- [ ] ConfigEditorState integrated into CampaignBuilderApp
- [ ] Config tab appears in UI

#### 1.6 Success Criteria

- ✅ Config tab visible in Campaign Builder tab bar
- ✅ Config editor displays all four sections (Graphics, Audio, Controls, Camera)
- ✅ `cargo check` and `cargo clippy` pass without errors
- ✅ All existing tests continue to pass

---

### Phase 2: UI Implementation

#### 2.1 Graphics Section UI

- Resolution width/height with `DragValue` widgets
- Fullscreen checkbox
- VSync checkbox
- MSAA samples dropdown (0, 2, 4, 8, 16)
- Shadow quality dropdown (Low, Medium, High, Ultra)

#### 2.2 Audio Section UI

- Master/Music/SFX/Ambient volume sliders (0.0-1.0)
- Enable audio checkbox

#### 2.3 Controls Section UI

- Key binding list editors for each action
- Movement cooldown `DragValue`

#### 2.4 Camera Section UI

- Camera mode dropdown (FirstPerson, Tactical, Isometric)
- Eye height, FOV, near/far clip `DragValue` widgets
- Smooth rotation checkbox
- Rotation speed, light height/intensity/range `DragValue` widgets
- Shadows enabled checkbox

#### 2.5 Deliverables

- [ ] All four config sections implemented with egui widgets
- [ ] Values display correctly when loading existing config
- [ ] Modified values saved correctly

#### 2.6 Success Criteria

- ✅ All config fields editable via UI
- ✅ Validation errors shown inline
- ✅ Changes can be saved to `config.ron`

---

## Verification Plan

### Automated Tests

**Existing tests to verify:**

```bash
cargo test --package antares --lib sdk::game_config::tests
```

**New tests to add in `config_editor.rs`:**

```bash
cargo test --package campaign_builder config_editor::tests
```

**Full test suite:**

```bash
cargo test --workspace
```

**Lint check:**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Format check:**

```bash
cargo fmt --check
```

### Manual Verification

1. Start Campaign Builder: `cargo run --package campaign_builder`
2. Open an existing campaign (e.g., `campaigns/tutorial`)
3. Click the "Config" tab in the tab bar
4. Verify all four sections (Graphics, Audio, Controls, Camera) are visible
5. Modify a value (e.g., change resolution to 1920x1080)
6. Click Save and verify `config.ron` is updated
7. Click Reload and verify values revert to file contents

---

### Phase 3: Interactive Key Capture and Auto-Population

#### 3.1 Key Capture Foundation

**Problem**: Current implementation requires users to TYPE key names, which is error-prone and unintuitive. Key binding fields should capture actual key presses.

**File:** [MODIFY] [`config_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs)

Add interactive key capture system:

- Implement `handle_key_capture()` method that processes `egui::Event::Key` events
- Track focus state for each key binding field
- Add "Press a key..." placeholder text when field is focused and empty
- Convert egui key codes to human-readable key names (e.g., `Key::W` → "W", `Key::ArrowUp` → "Up Arrow")
- Handle special keys: Escape cancels capture, Backspace clears binding
- Update `capturing_key_for` and `last_captured_key` fields (already added in Phase 2)

#### 3.2 UI Integration for Key Capture

**File:** [MODIFY] [`config_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs)

Enhance key binding UI in `show_controls_section()`:

- Add "Capture" button next to each key binding field
- When button clicked, set `capturing_key_for` to action name
- Show visual indicator when capturing (e.g., "🎮 Press a key..." label)
- Display captured key immediately in the text field
- Allow manual text editing as fallback
- Add "Clear" button to remove all bindings for an action

Visual feedback states:

- Normal: Text field with current bindings
- Capturing: Blue highlight with "Press a key..." placeholder
- Just Captured: Green flash/feedback showing new key added

#### 3.3 Auto-Population on Load

**Problem**: Key binding text fields don't show current config values when opening the Config tab.

**File:** [MODIFY] [`config_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/config_editor.rs)

Fix initialization flow:

- Ensure `update_edit_buffers()` is called when config tab first shown (not just on Load button)
- Add `needs_initial_load` boolean flag to `ConfigEditorState`
- In `show()` method, check if `campaign_dir` changed or `needs_initial_load` is true
- Auto-load config on first display if campaign is open
- Populate all key binding text fields from loaded `game_config.controls`

#### 3.4 Key Name Conversion Utilities

**File:** [MODIFY] [`config_editor.rs`](file:///Users/bsmith/go/src/config_editor.rs)

Add helper functions:

- `egui_key_to_string()` - Convert egui::Key to display name (e.g., Key::W → "W")
- `string_to_egui_key()` - Parse text input back to egui::Key for validation
- `format_key_list()` - Format Vec<String> as comma-separated display text
- `parse_key_list()` - Parse comma-separated text back to Vec<String>

Key name mappings to handle:

- Letters: A-Z
- Numbers: 0-9
- Special: Space, Enter, Escape, Tab, etc.
- Arrows: ArrowUp → "Up Arrow", ArrowDown → "Down Arrow", etc.
- Modifiers: Shift, Ctrl, Alt, Super (platform-aware)

#### 3.5 Testing Requirements

**File:** [NEW/MODIFY] Tests in `config_editor.rs`

Add tests for:

- `test_egui_key_to_string()` - Verify key conversion accuracy
- `test_key_capture_state_transitions()` - Test capturing_key_for state changes
- `test_auto_population_on_first_load()` - Verify text fields populate on init
- `test_escape_cancels_capture()` - Ensure Escape doesn't bind, just cancels
- `test_backspace_clears_binding()` - Verify clearing behavior
- `test_manual_text_edit_still_works()` - Ensure fallback text editing works
- `test_multiple_keys_per_action()` - Verify comma-separated multi-bind support

#### 3.6 Deliverables

- [ ] Interactive key capture system implemented
- [ ] "Capture" buttons next to each key binding field
- [ ] Visual feedback for capture state (blue highlight, "Press a key...")
- [ ] Auto-population of text fields when config loads
- [ ] Key name conversion utilities (egui::Key ↔ String)
- [ ] Special key handling (Escape cancels, Backspace clears)
- [ ] Manual text editing still available as fallback
- [ ] Clear button to remove bindings
- [ ] 7+ new tests for key capture functionality
- [ ] Documentation updated with key capture usage

#### 3.7 Success Criteria

- ✅ Key binding fields auto-populate with current config values on tab open
- ✅ Clicking "Capture" button enables key capture mode
- ✅ Pressing any key adds it to the binding (displayed as human-readable name)
- ✅ Escape key cancels capture without binding
- ✅ Backspace clears the current binding
- ✅ Multiple keys can be bound to one action (comma-separated)
- ✅ Manual text editing still works if user prefers typing
- ✅ Visual feedback clearly shows capture state
- ✅ All egui::Key variants correctly converted to readable names
- ✅ Config saves and loads key bindings correctly
- ✅ Zero regression in Phase 1 & 2 functionality

---

## Risk Analysis

**Risk**: GameConfig import may require path adjustments in SDK
**Mitigation**: Use fully qualified path `antares::sdk::game_config::GameConfig`

**Risk**: Key binding editor complexity for ControlsConfig
**Mitigation**: Start with simple text list (Phase 1-2), enhance with interactive capture (Phase 3)

**Risk**: egui event handling may not capture all key types correctly
**Mitigation**: Test extensively with different key types, provide manual text editing fallback

**Risk**: Auto-population may conflict with unsaved changes tracking
**Mitigation**: Set `needs_initial_load` flag carefully, don't trigger on every render

---

## Related Documentation

- [game_config_schema.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/game_config_schema.md) - Schema reference
- [spells_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/spells_editor.rs) - Pattern reference
- [egui input handling](https://docs.rs/egui/latest/egui/struct.Context.html#method.input) - Event capture reference
