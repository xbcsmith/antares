# Per-Campaign Game Configuration Implementation Plan

## Overview

Implement a comprehensive per-campaign game configuration system that allows campaign authors to define graphics, audio, keymap, and camera settings. When Antares loads a campaign, it reads the campaign's `config.ron` file and configures all engine systems accordingly. This approach eliminates the need for in-game settings UI and allows different campaigns to have distinct gameplay experiences (e.g., tactical vs first-person, action vs exploration).

## Current State Analysis

### Existing Infrastructure

**Campaign Loading System (`src/sdk/campaign_loader.rs`):**

- `Campaign` struct loads metadata from `campaigns/<name>/campaign.ron`
- `CampaignConfig` contains gameplay settings (starting_map, starting_position, max_party_size, etc.)
- `CampaignData` specifies data file paths (items, spells, monsters, etc.)
- `CampaignAssets` defines asset directories (tilesets, music, sounds, etc.)
- No game engine configuration (graphics, audio, controls, camera)

**Camera System (`src/game/systems/camera.rs`):**

- Hardcoded first-person camera at eye_height = 0.6 units
- Hardcoded point light at y=5.0 with intensity 2,000,000 lumens
- No configuration options
- `setup_camera()` runs at Startup, `update_camera()` tracks party position

**Input System (`src/game/systems/input.rs`):**

- Hardcoded keybindings: W/ArrowUp = forward, S/ArrowDown = back, A/ArrowLeft = turn left, D/ArrowRight = turn right, Space/E = interact
- Movement cooldown hardcoded to 0.2 seconds
- No rebindable controls
- `handle_input()` system reads from `ButtonInput<KeyCode>`

**Bevy Window/Graphics (`src/bin/antares.rs`):**

- Uses `DefaultPlugins` with no customization
- No window size, fullscreen, vsync, or MSAA configuration
- Resolution/graphics settings inherited from Bevy defaults

**Audio System:**

- Currently not implemented in game engine
- Campaign has `assets.music` and `assets.sounds` paths defined but unused

### Identified Issues

1. **Hardcoded Camera Settings**: Eye height, light position, intensity cannot be adjusted per-campaign
2. **Fixed Keybindings**: Controls cannot be customized or rebound
3. **No Graphics Configuration**: Window size, fullscreen mode, quality settings not configurable
4. **No Audio Configuration**: Volume levels, audio enable/disable not available
5. **No Per-Campaign Customization**: All campaigns forced to use identical engine settings
6. **No Configuration File Structure**: No `config.ron` file or schema defined

## Implementation Phases

### Phase 1: Core Configuration Infrastructure

**Goal:** Create `GameConfig` data structures and extend campaign loading to read `config.ron` files.

#### 1.1 Define Configuration Structures

**File:** `src/sdk/game_config.rs` (new file)

Create the following structures with serde derives:

```rust
pub struct GameConfig {
    pub graphics: GraphicsConfig,
    pub audio: AudioConfig,
    pub controls: ControlsConfig,
    pub camera: CameraConfig,
}

pub struct GraphicsConfig {
    pub resolution: (u32, u32),          // Default: (1280, 720)
    pub fullscreen: bool,                 // Default: false
    pub vsync: bool,                      // Default: true
    pub msaa_samples: u32,                // Default: 4 (4x MSAA)
    pub shadow_quality: ShadowQuality,    // Default: Medium
}

pub enum ShadowQuality { Low, Medium, High, Ultra }

pub struct AudioConfig {
    pub master_volume: f32,               // Default: 0.8 (0.0-1.0)
    pub music_volume: f32,                // Default: 0.6
    pub sfx_volume: f32,                  // Default: 1.0
    pub ambient_volume: f32,              // Default: 0.5
    pub enable_audio: bool,               // Default: true
}

pub struct ControlsConfig {
    pub move_forward: Vec<String>,        // Default: ["W", "ArrowUp"]
    pub move_back: Vec<String>,           // Default: ["S", "ArrowDown"]
    pub turn_left: Vec<String>,           // Default: ["A", "ArrowLeft"]
    pub turn_right: Vec<String>,          // Default: ["D", "ArrowRight"]
    pub interact: Vec<String>,            // Default: ["Space", "E"]
    pub menu: Vec<String>,                // Default: ["Escape"]
    pub movement_cooldown: f32,           // Default: 0.2 seconds
}

pub struct CameraConfig {
    pub mode: CameraMode,                 // Default: FirstPerson
    pub eye_height: f32,                  // Default: 0.6 (6 feet)
    pub fov: f32,                         // Default: 70.0 degrees
    pub near_clip: f32,                   // Default: 0.1
    pub far_clip: f32,                    // Default: 1000.0
    pub smooth_rotation: bool,            // Default: false
    pub rotation_speed: f32,              // Default: 180.0 deg/sec (if smooth)
    pub light_height: f32,                // Default: 5.0
    pub light_intensity: f32,             // Default: 2_000_000.0 lumens
    pub light_range: f32,                 // Default: 60.0
    pub shadows_enabled: bool,            // Default: true
}

pub enum CameraMode { FirstPerson, Tactical, Isometric }
```

**Default Implementation:**

- Implement `Default` trait for all structs matching current hardcoded values
- Add validation methods: `GameConfig::validate() -> Result<(), ConfigError>`
- Add helper: `GameConfig::load_or_default(path: &Path) -> Result<Self, ConfigError>`

#### 1.2 Extend Campaign Structure

**File:** `src/sdk/campaign_loader.rs`

Add to `Campaign` struct:

```rust
pub struct Campaign {
    // ... existing fields ...

    /// Game engine configuration
    #[serde(skip)]
    pub game_config: GameConfig,
}
```

Update `Campaign::load()` method:

1. After loading `campaign.ron`, attempt to load `config.ron` from same directory
2. If `config.ron` exists, parse it into `GameConfig`
3. If missing, use `GameConfig::default()` with warning log
4. Store in `campaign.game_config` field

**Pseudo-code:**

```
let campaign_path = path.join("campaign.ron");
let config_path = path.join("config.ron");

let mut campaign = load_campaign_metadata(campaign_path)?;

campaign.game_config = if config_path.exists() {
    GameConfig::load_or_default(&config_path)?
} else {
    warn!("No config.ron found for campaign {}, using defaults", campaign.id);
    GameConfig::default()
};
```

#### 1.3 Module Registration

**File:** `src/sdk/mod.rs`

Add:

```rust
pub mod game_config;
```

Export types:

```rust
pub use game_config::{GameConfig, GraphicsConfig, AudioConfig, ControlsConfig, CameraConfig, CameraMode, ShadowQuality};
```

#### 1.4 Testing Requirements

**Unit Tests (`src/sdk/game_config.rs`):**

- `test_game_config_default_values()` - verify all defaults match current hardcoded values
- `test_graphics_config_validation()` - validate resolution > 0, msaa is power of 2
- `test_audio_config_validation()` - validate volumes in 0.0-1.0 range
- `test_camera_config_validation()` - validate eye_height > 0, fov in 30-120 range
- `test_load_valid_config_file()` - parse example config.ron successfully
- `test_load_invalid_config_returns_error()` - handle malformed RON
- `test_load_missing_config_uses_defaults()` - fallback to defaults

**Integration Tests (`tests/game_config_integration.rs`):**

- `test_campaign_loads_config_ron()` - load tutorial campaign with config.ron
- `test_campaign_without_config_uses_defaults()` - campaign without config.ron works

#### 1.5 Deliverables

- [ ] `src/sdk/game_config.rs` created with all structs and Default impls
- [ ] `Campaign` struct extended with `game_config` field
- [ ] `Campaign::load()` updated to read config.ron
- [ ] Module exports added to `src/sdk/mod.rs`
- [ ] Unit tests written and passing (minimum 7 tests)
- [ ] Integration tests written and passing (minimum 2 tests)
- [ ] Documentation comments added to all public structs/fields

#### 1.6 Success Criteria

- ✅ `cargo check --all-targets --all-features` passes
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- ✅ `cargo nextest run --all-features` all tests pass
- ✅ Campaign loads with and without config.ron without errors
- ✅ Default GameConfig values match current hardcoded behavior exactly

---

### Phase 2: Camera System Integration

**Goal:** Make camera system read configuration from `GameConfig` instead of hardcoded values.

#### 2.1 Pass Configuration to Camera Plugin

**File:** `src/game/systems/camera.rs`

Add `CameraConfig` as a Bevy resource:

```rust
pub struct CameraPlugin {
    pub config: CameraConfig,
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .add_systems(Startup, setup_camera)
            .add_systems(Update, update_camera);
    }
}
```

**File:** `src/bin/antares.rs`

Update camera plugin registration:

```rust
.add_plugins(CameraPlugin {
    config: campaign.game_config.camera.clone(),
})
```

#### 2.2 Update Camera Setup System

**File:** `src/game/systems/camera.rs`

Modify `setup_camera()`:

1. Accept `Res<CameraConfig>` parameter
2. Replace hardcoded `0.6` with `config.eye_height`
3. Replace hardcoded light values with `config.light_height`, `config.light_intensity`, `config.light_range`, `config.shadows_enabled`
4. Apply `config.fov`, `config.near_clip`, `config.far_clip` to camera projection

Modify `update_camera()`:

1. Accept `Res<CameraConfig>` parameter
2. Replace hardcoded `0.6` eye_height with `config.eye_height`
3. If `config.smooth_rotation`, use `transform.rotation.lerp()` with `config.rotation_speed`

#### 2.3 Support Multiple Camera Modes

Add camera mode handling in `setup_camera()`:

- `CameraMode::FirstPerson` - current behavior (eye_height at party position)
- `CameraMode::Tactical` - elevated camera (height=3.0, offset_back=2.0, tilt=-35°)
- `CameraMode::Isometric` - high camera (height=5.0, offset_back=3.0, tilt=-60°)

Use match statement on `config.mode` to spawn different camera transforms.

#### 2.4 Testing Requirements

**Unit Tests (camera.rs tests module):**

- `test_camera_config_resource_inserted()` - verify resource is inserted
- `test_camera_uses_config_eye_height()` - camera positioned at config.eye_height
- `test_light_uses_config_values()` - light matches config intensity/range/height
- `test_first_person_camera_mode()` - FirstPerson mode positions camera correctly
- `test_tactical_camera_mode()` - Tactical mode creates elevated angled camera
- `test_smooth_rotation_when_enabled()` - rotation lerps when smooth_rotation=true

#### 2.5 Deliverables

- [ ] `CameraPlugin` accepts `CameraConfig` parameter
- [ ] `setup_camera()` reads all values from config resource
- [ ] `update_camera()` uses config.eye_height and config.smooth_rotation
- [ ] Camera mode switching implemented (FirstPerson/Tactical/Isometric)
- [ ] `src/bin/antares.rs` passes campaign.game_config.camera to plugin
- [ ] Unit tests added and passing (minimum 6 tests)

#### 2.6 Success Criteria

- ✅ Camera respects all CameraConfig values
- ✅ Default config produces identical behavior to current hardcoded camera
- ✅ Changing config.eye_height in config.ron changes camera height in game
- ✅ All three camera modes (FirstPerson/Tactical/Isometric) functional
- ✅ All quality gates pass (fmt, check, clippy, tests)

---

### Phase 3: Input System Integration

**Goal:** Make input system use configurable keybindings from `ControlsConfig`.

#### 3.1 Create Keymap Translation System

**File:** `src/game/systems/input.rs`

Add keymap resource:

```rust
#[derive(Resource, Clone)]
pub struct KeyMap {
    move_forward: Vec<KeyCode>,
    move_back: Vec<KeyCode>,
    turn_left: Vec<KeyCode>,
    turn_right: Vec<KeyCode>,
    interact: Vec<KeyCode>,
    menu: Vec<KeyCode>,
    movement_cooldown: f32,
}

impl KeyMap {
    pub fn from_config(config: &ControlsConfig) -> Result<Self, InputError> {
        // Convert string keybindings to KeyCode enums
        // "W" -> KeyCode::KeyW, "ArrowUp" -> KeyCode::ArrowUp, etc.
    }

    fn is_pressed(&self, input: &ButtonInput<KeyCode>, action: &[KeyCode]) -> bool {
        action.iter().any(|key| input.pressed(*key))
    }

    fn is_just_pressed(&self, input: &ButtonInput<KeyCode>, action: &[KeyCode]) -> bool {
        action.iter().any(|key| input.just_pressed(*key))
    }
}
```

Update `InputPlugin`:

```rust
pub struct InputPlugin {
    pub config: ControlsConfig,
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        let keymap = KeyMap::from_config(&self.config)
            .expect("Invalid keybindings in config");
        app.insert_resource(keymap)
            .add_systems(Update, handle_input);
    }
}
```

#### 3.2 Update Input Handling

**File:** `src/game/systems/input.rs`

Modify `handle_input()`:

1. Accept `Res<KeyMap>` parameter
2. Replace all hardcoded `KeyCode::*` checks with `keymap.is_pressed()` / `keymap.is_just_pressed()`
3. Replace hardcoded `0.2` cooldown with `keymap.movement_cooldown`

Example:

```rust
// Old: keyboard_input.just_pressed(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::KeyE)
// New: keymap.is_just_pressed(&keyboard_input, &keymap.interact)

// Old: if current_time - *last_move_time < 0.2
// New: if current_time - *last_move_time < keymap.movement_cooldown
```

#### 3.3 Integrate with Campaign Loading

**File:** `src/bin/antares.rs`

Update input plugin registration:

```rust
.add_plugins(antares::game::systems::input::InputPlugin {
    config: campaign.game_config.controls.clone(),
})
```

#### 3.4 Testing Requirements

**Unit Tests (input.rs tests module):**

- `test_keymap_from_config_valid()` - convert valid config to KeyMap
- `test_keymap_from_config_invalid_key()` - error on unknown key string
- `test_keymap_is_pressed_single_key()` - detects single key press
- `test_keymap_is_pressed_multiple_keys()` - detects any of multiple keys
- `test_movement_cooldown_respects_config()` - cooldown uses config value
- `test_multiple_bindings_same_action()` - W and ArrowUp both trigger forward

#### 3.5 Deliverables

- [ ] `KeyMap` resource created with string-to-KeyCode conversion
- [ ] `InputPlugin` accepts `ControlsConfig` parameter
- [ ] `handle_input()` uses `KeyMap` instead of hardcoded keys
- [ ] Movement cooldown configurable via config
- [ ] `src/bin/antares.rs` passes campaign.game_config.controls to plugin
- [ ] Unit tests added and passing (minimum 6 tests)

#### 3.6 Success Criteria

- ✅ All input actions use configurable keybindings
- ✅ Default config produces identical input behavior to current system
- ✅ Changing keybindings in config.ron changes controls in game
- ✅ Invalid key strings in config.ron produce clear error messages
- ✅ All quality gates pass (fmt, check, clippy, tests)

---

### Phase 4: Graphics Configuration

**Goal:** Configure Bevy window and graphics settings from `GraphicsConfig`.

#### 4.1 Configure Bevy Window Plugin

**File:** `src/bin/antares.rs`

Replace `DefaultPlugins` with configured plugins:

```rust
let window_plugin = WindowPlugin {
    primary_window: Some(Window {
        resolution: (
            campaign.game_config.graphics.resolution.0 as f32,
            campaign.game_config.graphics.resolution.1 as f32,
        ).into(),
        title: format!("Antares - {}", campaign.name),
        mode: if campaign.game_config.graphics.fullscreen {
            bevy::window::WindowMode::Fullscreen
        } else {
            bevy::window::WindowMode::Windowed
        },
        present_mode: if campaign.game_config.graphics.vsync {
            bevy::window::PresentMode::AutoVsync
        } else {
            bevy::window::PresentMode::AutoNoVsync
        },
        ..default()
    }),
    ..default()
};

App::new()
    .add_plugins(DefaultPlugins.set(window_plugin))
    // ... rest of plugins
```

#### 4.2 Configure MSAA

Add MSAA resource configuration:

```rust
let msaa = match campaign.game_config.graphics.msaa_samples {
    0 | 1 => Msaa::Off,
    2 => Msaa::Sample2,
    4 => Msaa::Sample4,
    8 => Msaa::Sample8,
    _ => {
        warn!("Invalid MSAA samples {}, using 4x", campaign.game_config.graphics.msaa_samples);
        Msaa::Sample4
    }
};

App::new()
    .add_plugins(/* ... */)
    .insert_resource(msaa)
```

#### 4.3 Configure Shadow Quality

Map `ShadowQuality` enum to Bevy shadow settings:

- Low: 512x512 shadow maps, 1 cascade
- Medium: 1024x1024, 2 cascades
- High: 2048x2048, 3 cascades
- Ultra: 4096x4096, 4 cascades

Insert as resource or configure in camera light spawning.

#### 4.4 Testing Requirements

**Integration Tests:**

- `test_window_resolution_from_config()` - verify window created with config resolution
- `test_fullscreen_mode_from_config()` - verify fullscreen mode applied
- `test_vsync_from_config()` - verify vsync setting applied
- `test_msaa_from_config()` - verify MSAA level set correctly

#### 4.5 Deliverables

- [ ] WindowPlugin configured with GraphicsConfig values
- [ ] MSAA resource inserted based on config
- [ ] Shadow quality configuration implemented
- [ ] Window title includes campaign name
- [ ] Integration tests added and passing (minimum 4 tests)

#### 4.6 Success Criteria

- ✅ Window opens with correct resolution from config
- ✅ Fullscreen mode works when config.fullscreen = true
- ✅ VSync enabled/disabled based on config
- ✅ MSAA quality matches config setting
- ✅ All quality gates pass

---

### Phase 5: Audio System Foundation

**Goal:** Implement basic audio volume configuration (preparation for future audio implementation).

#### 5.1 Create Audio Configuration Resource

**File:** `src/game/systems/audio.rs` (new file)

```rust
#[derive(Resource, Clone)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub ambient_volume: f32,
    pub enabled: bool,
}

pub struct AudioPlugin {
    pub config: AudioConfig,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let settings = AudioSettings {
            master_volume: self.config.master_volume,
            music_volume: self.config.music_volume,
            sfx_volume: self.config.sfx_volume,
            ambient_volume: self.config.ambient_volume,
            enabled: self.config.enable_audio,
        };

        app.insert_resource(settings);
        // Future: add_systems for audio playback
    }
}
```

#### 5.2 Register Audio Plugin

**File:** `src/bin/antares.rs`

```rust
.add_plugins(antares::game::systems::audio::AudioPlugin {
    config: campaign.game_config.audio.clone(),
})
```

**File:** `src/game/systems/mod.rs`

```rust
pub mod audio;
```

#### 5.3 Documentation

Add comments explaining:

- AudioSettings resource available for future audio systems
- Volumes stored but not yet applied (no audio playback implemented)
- When audio playback added, use `AudioSettings` resource to scale volumes

#### 5.4 Testing Requirements

**Unit Tests:**

- `test_audio_settings_from_config()` - verify resource created with config values
- `test_audio_disabled_when_config_false()` - enabled flag set correctly

#### 5.5 Deliverables

- [ ] `src/game/systems/audio.rs` created with AudioPlugin
- [ ] AudioSettings resource inserted with config values
- [ ] Module registered in src/game/systems/mod.rs
- [ ] AudioPlugin added to src/bin/antares.rs
- [ ] Documentation added explaining future usage

#### 5.6 Success Criteria

- ✅ AudioSettings resource available in Bevy app
- ✅ Audio configuration loaded from config.ron
- ✅ Foundation ready for future audio implementation
- ✅ All quality gates pass

---

### Phase 6: Tutorial Campaign Configuration

**Goal:** Create example `config.ron` for tutorial campaign with documented defaults.

#### 6.1 Create Tutorial Config File

**File:** `campaigns/tutorial/config.ron`

```ron
GameConfig(
    graphics: GraphicsConfig(
        resolution: (1280, 720),
        fullscreen: false,
        vsync: true,
        msaa_samples: 4,
        shadow_quality: Medium,
    ),
    audio: AudioConfig(
        master_volume: 0.8,
        music_volume: 0.6,
        sfx_volume: 1.0,
        ambient_volume: 0.5,
        enable_audio: true,
    ),
    controls: ControlsConfig(
        move_forward: ["W", "ArrowUp"],
        move_back: ["S", "ArrowDown"],
        turn_left: ["A", "ArrowLeft"],
        turn_right: ["D", "ArrowRight"],
        interact: ["Space", "E"],
        menu: ["Escape"],
        movement_cooldown: 0.2,
    ),
    camera: CameraConfig(
        mode: FirstPerson,
        eye_height: 0.6,
        fov: 70.0,
        near_clip: 0.1,
        far_clip: 1000.0,
        smooth_rotation: false,
        rotation_speed: 180.0,
        light_height: 5.0,
        light_intensity: 2000000.0,
        light_range: 60.0,
        shadows_enabled: true,
    ),
)
```

#### 6.2 Create Configuration Template

**File:** `campaigns/config.template.ron`

Copy of tutorial config with extensive comments explaining each field:

```ron
// Antares Campaign Game Configuration Template
// Copy this file to your campaign directory as config.ron
//
// All values shown are defaults. You can omit any section to use defaults.

GameConfig(
    graphics: GraphicsConfig(
        // Window resolution (width, height)
        resolution: (1280, 720),

        // Start in fullscreen mode
        fullscreen: false,

        // Enable vertical sync (prevents screen tearing)
        vsync: true,

        // Multisample anti-aliasing (0=off, 2, 4, 8)
        msaa_samples: 4,

        // Shadow quality (Low, Medium, High, Ultra)
        shadow_quality: Medium,
    ),

    // ... (rest with detailed comments)
)
```

#### 6.3 Documentation

**File:** `docs/explanation/game_config_schema.md` (new file)

Create comprehensive documentation:

- Purpose of config.ron
- Location (campaigns/<name>/config.ron)
- Full schema with types and ranges
- Examples for different campaign styles:
  - Tactical RPG (Tactical camera mode, lower movement cooldown)
  - Action RPG (FirstPerson camera, higher FOV, faster rotation)
  - Exploration RPG (Isometric camera, wider FOV, ambient focus)

#### 6.4 Testing Requirements

**Integration Tests:**

- `test_tutorial_campaign_loads_config()` - tutorial campaign loads config.ron
- `test_config_template_is_valid_ron()` - template parses successfully
- `test_campaign_defaults_match_template()` - Default::default() matches template

#### 6.5 Deliverables

- [ ] `campaigns/tutorial/config.ron` created with all defaults
- [ ] `campaigns/config.template.ron` created with extensive comments
- [ ] `docs/explanation/game_config_schema.md` documentation written
- [ ] Example configs for different campaign styles documented
- [ ] Integration tests verify config loads correctly

#### 6.6 Success Criteria

- ✅ Tutorial campaign launches with config.ron
- ✅ Config template is valid and parseable RON
- ✅ Documentation clear and comprehensive
- ✅ Campaign authors can copy template and customize easily
- ✅ All quality gates pass

---

## Overall Success Criteria

### Functional Requirements

- ✅ Campaign loads config.ron if present, uses defaults if missing
- ✅ Graphics settings (resolution, fullscreen, vsync, MSAA) applied correctly
- ✅ Camera settings (mode, eye_height, FOV, lighting) configurable per-campaign
- ✅ Input keybindings rebindable via config
- ✅ Audio volume settings stored (ready for future audio system)
- ✅ Default config produces identical behavior to current hardcoded system

### Quality Requirements

- ✅ Zero clippy warnings
- ✅ All tests passing (target: 30+ new tests across all phases)
- ✅ Code formatted with cargo fmt
- ✅ Documentation complete for all public APIs
- ✅ AGENTS.md rules followed (SPDX headers, tests, no domain changes)

### Backward Compatibility

- ✅ Campaigns without config.ron continue to work (use defaults)
- ✅ Existing tutorial campaign unaffected until config.ron added
- ✅ No breaking changes to Campaign struct public API

### Documentation

- ✅ `docs/explanation/implementations.md` updated with completion summary
- ✅ `docs/explanation/game_config_schema.md` created
- ✅ Config template with comments available
- ✅ Examples for different campaign styles documented

## Design Decisions

The following decisions have been made and incorporated into the implementation plan:

1. **Config Validation Level**: Invalid config values (e.g., negative volumes, zero resolution) will produce warnings and fall back to defaults with detailed warning logs. This is more forgiving for campaign authors while preventing broken campaigns. Validation errors logged at WARN level will clearly indicate the issue and the default value being used.

2. **Config Hot Reload**: Config changes will NOT be detectable at runtime - game restart required. Hot reload is deferred to a future enhancement. This simplifies Phase 1-6 implementation and avoids complexity around partial state updates.

3. **Per-User Overrides**: Individual users will NOT be able to override campaign config. Campaign config is absolute for that campaign. Per-user overrides (e.g., ~/.config/antares/user_overrides.ron) can be added in the future if requested, but are out of scope for this implementation.

4. **Config Validation Strictness**: Config will be validated both at runtime AND in the campaign_builder tool. Runtime validation provides safety during gameplay with fallback to defaults. Campaign_builder validation provides early feedback to campaign authors during development. Phase 6 will add config validation to campaign_builder's validation panel.

5. **String-to-KeyCode Mapping**: User-friendly key names will be supported ("W", "Up", "Space", "ArrowUp", etc.) rather than requiring Bevy KeyCode variant names. This is more intuitive for campaign authors. The KeyMap::from_config() function will handle translation from friendly names to KeyCode enums with clear error messages for invalid key strings.

## Dependencies and Risks

### External Dependencies

- Bevy 0.17 WindowPlugin API (stable)
- Bevy KeyCode enum (may change in Bevy updates)
- RON serialization format (stable)

### Risks and Mitigations

**Risk**: Bevy KeyCode enum changes in future versions

- **Mitigation**: Use string-based config, translate to KeyCode at runtime, centralize mapping in one function

**Risk**: Invalid config.ron breaks campaign loading

- **Mitigation**: Fallback to defaults on parse error, log detailed error messages, provide config validation tool

**Risk**: Performance impact from config reads

- **Mitigation**: Config loaded once at campaign startup, stored in Bevy resources, zero runtime overhead

**Risk**: Config conflicts with hardcoded systems

- **Mitigation**: Phase 1-5 systematically remove all hardcoded values, replace with config reads

## Timeline Estimate

- **Phase 1** (Core Infrastructure): 4-6 hours
- **Phase 2** (Camera Integration): 3-4 hours
- **Phase 3** (Input Integration): 3-4 hours
- **Phase 4** (Graphics Configuration): 2-3 hours
- **Phase 5** (Audio Foundation): 1-2 hours
- **Phase 6** (Documentation & Examples): 2-3 hours

**Total**: 15-22 hours

**Recommended Approach**: Implement phases 1-3 first (core + camera + input), validate with user testing, then proceed to phases 4-6.
