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

## Risk Analysis

**Risk**: GameConfig import may require path adjustments in SDK
**Mitigation**: Use fully qualified path `antares::sdk::game_config::GameConfig`

**Risk**: Key binding editor complexity for ControlsConfig
**Mitigation**: Start with simple text list, enhance later if needed

---

## Related Documentation

- [game_config_schema.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/game_config_schema.md) - Schema reference
- [spells_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/spells_editor.rs) - Pattern reference
