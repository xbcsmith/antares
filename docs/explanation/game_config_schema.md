# Game Configuration Schema

**Category**: Explanation  
**Audience**: Campaign authors, game designers, developers

This document explains the game configuration system (`config.ron`) used by Antares campaigns to customize graphics, audio, controls, and camera settings.

---

## Overview

Each campaign can provide a `config.ron` file to customize the game experience without modifying code. Configuration uses RON (Rusty Object Notation) format and validates all values on load.

### Purpose

- **Customization**: Tailor game settings for specific campaign styles (action, tactical, exploration)
- **Defaults**: System provides sensible defaults if config.ron is missing
- **Validation**: All values are validated on load to prevent invalid configurations
- **Consistency**: Campaign authors can rely on stable configuration schema

### Location

Configuration files are located in campaign directories:

```text
campaigns/
  <campaign-name>/
    campaign.ron       # Campaign metadata
    config.ron         # Game configuration (this file)
    data/              # Game data (items, spells, etc.)
    assets/            # Campaign assets
```

**Example**: `campaigns/tutorial/config.ron`

---

## Schema Structure

The root configuration type is `GameConfig`, which contains four sub-configurations:

```ron
GameConfig(
    graphics: GraphicsConfig(...),
    audio: AudioConfig(...),
    controls: ControlsConfig(...),
    camera: CameraConfig(...),
)
```

---

## GraphicsConfig

Controls rendering settings, window properties, and visual quality.

### Fields

| Field | Type | Range/Values | Default | Description |
|-------|------|--------------|---------|-------------|
| `resolution` | `(u32, u32)` | Both > 0 | `(1280, 720)` | Window width and height in pixels |
| `fullscreen` | `bool` | — | `false` | Start in fullscreen mode |
| `vsync` | `bool` | — | `true` | Enable vertical sync (prevents tearing) |
| `msaa_samples` | `u32` | 0 or power of 2 | `4` | Multisample anti-aliasing samples |
| `shadow_quality` | `ShadowQuality` | See below | `Medium` | Shadow rendering quality level |

### ShadowQuality Enum

```ron
Low     // Minimal shadows, best performance
Medium  // Balanced quality (recommended)
High    // Detailed shadows
Ultra   // Maximum quality, highest cost
```

### Example

```ron
graphics: GraphicsConfig(
    resolution: (1920, 1080),
    fullscreen: true,
    vsync: true,
    msaa_samples: 4,
    shadow_quality: High,
),
```

### Validation Rules

- Resolution width and height must both be greater than 0
- MSAA samples must be 0 (disabled) or a power of 2 (2, 4, 8, 16, etc.)
- Recommended MSAA values: 0 (off), 4 (balanced), 8 (high quality)

---

## AudioConfig

Controls volume levels for different audio channels and global audio enable.

### Fields

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `master_volume` | `f32` | 0.0–1.0 | `0.8` | Master volume (affects all channels) |
| `music_volume` | `f32` | 0.0–1.0 | `0.6` | Background music volume |
| `sfx_volume` | `f32` | 0.0–1.0 | `1.0` | Sound effects volume (combat, UI) |
| `ambient_volume` | `f32` | 0.0–1.0 | `0.5` | Ambient/environmental sound volume |
| `enable_audio` | `bool` | — | `true` | Enable/disable audio system |

### Volume Calculation

Final playback volume for each channel is calculated as:

```text
effective_volume = channel_volume * master_volume
```

If `enable_audio` is `false`, all audio is silenced regardless of volume settings.

### Example

```ron
audio: AudioConfig(
    master_volume: 0.8,
    music_volume: 0.6,
    sfx_volume: 1.0,
    ambient_volume: 0.5,
    enable_audio: true,
),
```

### Validation Rules

- All volume values must be in range 0.0–1.0 (inclusive)
- Values outside range will cause validation error on load

---

## ControlsConfig

Defines key bindings for game actions. Each action can have multiple keys.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `move_forward` | `Vec<String>` | `["W", "ArrowUp"]` | Keys for moving forward |
| `move_back` | `Vec<String>` | `["S", "ArrowDown"]` | Keys for moving backward |
| `turn_left` | `Vec<String>` | `["A", "ArrowLeft"]` | Keys for turning left (90°) |
| `turn_right` | `Vec<String>` | `["D", "ArrowRight"]` | Keys for turning right (90°) |
| `interact` | `Vec<String>` | `["Space", "E"]` | Keys for interaction (NPCs, doors) |
| `menu` | `Vec<String>` | `["Escape"]` | Keys for opening menu |
| `movement_cooldown` | `f32` | `0.2` | Cooldown in seconds between moves |

### Key Names

Key strings use Bevy's `KeyCode` naming convention:

- **Letters**: `"A"`, `"B"`, `"C"`, ..., `"Z"`
- **Numbers**: `"0"`, `"1"`, `"2"`, ..., `"9"`
- **Arrows**: `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`
- **Special**: `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`
- **Function**: `"F1"`, `"F2"`, ..., `"F12"`
- **Modifiers**: `"ControlLeft"`, `"ShiftLeft"`, `"AltLeft"`

See Bevy documentation for complete list.

### Example

```ron
controls: ControlsConfig(
    move_forward: ["W", "ArrowUp"],
    move_back: ["S", "ArrowDown"],
    turn_left: ["A", "ArrowLeft"],
    turn_right: ["D", "ArrowRight"],
    interact: ["Space", "E"],
    menu: ["Escape"],
    movement_cooldown: 0.2,
),
```

### Validation Rules

- `movement_cooldown` must be non-negative (>= 0.0)
- Recommended cooldown: 0.1–0.3 seconds for grid-based movement

---

## CameraConfig

Controls camera behavior, field of view, clipping planes, and associated lighting.

### Fields

| Field | Type | Range | Default | Description |
|-------|------|-------|---------|-------------|
| `mode` | `CameraMode` | See below | `FirstPerson` | Camera perspective mode |
| `eye_height` | `f32` | > 0 | `0.6` | Eye height in world units (1 unit = 10 ft) |
| `fov` | `f32` | 30.0–150.0 | `70.0` | Field of view in degrees (horizontal) |
| `near_clip` | `f32` | > 0 | `0.1` | Near clipping plane distance |
| `far_clip` | `f32` | > near_clip | `1000.0` | Far clipping plane distance |
| `smooth_rotation` | `bool` | — | `false` | Enable smooth camera rotation |
| `rotation_speed` | `f32` | 1.0–1000.0 | `180.0` | Rotation speed (deg/sec, if smooth) |
| `light_height` | `f32` | any | `5.0` | Light height above ground (world units) |
| `light_intensity` | `f32` | > 0 | `2000000.0` | Light intensity in lumens |
| `light_range` | `f32` | > 0 | `60.0` | Light falloff range (world units) |
| `shadows_enabled` | `bool` | — | `true` | Enable shadow rendering |

### CameraMode Enum

```ron
FirstPerson  // First-person view (immersive, standard RPG)
Tactical     // Top-down angled view (tactical combat focus)
Isometric    // Isometric/diagonal view (classic CRPG style)
```

### World Units

Antares uses **1 world unit = 10 feet**.

- `eye_height: 0.6` = 6 feet (human eye level)
- `light_height: 5.0` = 50 feet above player
- `light_range: 60.0` = 600 feet radius

### Example

```ron
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
```

### Validation Rules

- `eye_height` must be positive
- `fov` must be in range 30.0–150.0 degrees
- `near_clip` must be positive
- `far_clip` must be greater than `near_clip`
- `rotation_speed` must be positive

---

## Campaign Style Examples

Different campaign types benefit from different configuration profiles.

### Tactical RPG

**Focus**: Turn-based combat, battlefield overview, responsive controls

```ron
GameConfig(
    graphics: GraphicsConfig(
        resolution: (1920, 1080),
        shadow_quality: Medium,
        // ... other defaults
    ),
    camera: CameraConfig(
        mode: Tactical,
        fov: 90.0,
        eye_height: 1.0,
        light_height: 10.0,
        smooth_rotation: false,
        // ... other defaults
    ),
    controls: ControlsConfig(
        movement_cooldown: 0.1,
        // ... other defaults
    ),
    // ... audio defaults
)
```

**Rationale**:
- Tactical camera provides battlefield overview
- Higher FOV (90°) shows more units
- Lower cooldown (0.1s) for responsive tactical movement

---

### Action RPG

**Focus**: Fast-paced exploration and combat, fluid controls

```ron
GameConfig(
    graphics: GraphicsConfig(
        resolution: (1920, 1080),
        shadow_quality: High,
        msaa_samples: 8,
        // ... other defaults
    ),
    camera: CameraConfig(
        mode: FirstPerson,
        fov: 90.0,
        smooth_rotation: true,
        rotation_speed: 360.0,
        // ... other defaults
    ),
    controls: ControlsConfig(
        movement_cooldown: 0.05,
        // ... other defaults
    ),
    audio: AudioConfig(
        music_volume: 0.5,
        sfx_volume: 1.0,
        // ... other defaults
    ),
)
```

**Rationale**:
- First-person for immersion
- Higher FOV (90°) for awareness
- Fast smooth rotation (360°/s)
- Very low cooldown (0.05s) for fluid movement
- Emphasized SFX for combat feedback

---

### Exploration RPG

**Focus**: Puzzles, story, atmosphere, classic CRPG feel

```ron
GameConfig(
    graphics: GraphicsConfig(
        resolution: (1280, 720),
        shadow_quality: Medium,
        // ... other defaults
    ),
    camera: CameraConfig(
        mode: Isometric,
        fov: 80.0,
        eye_height: 0.8,
        // ... other defaults
    ),
    controls: ControlsConfig(
        movement_cooldown: 0.2,
        // ... other defaults
    ),
    audio: AudioConfig(
        music_volume: 0.4,
        ambient_volume: 0.8,
        // ... other defaults
    ),
)
```

**Rationale**:
- Isometric camera for classic CRPG aesthetic
- Standard FOV (80°) for environment visibility
- Standard cooldown (0.2s) for deliberate movement
- Lower music, higher ambient for atmospheric exploration

---

### Horror RPG

**Focus**: Tension, atmosphere, limited visibility

```ron
GameConfig(
    graphics: GraphicsConfig(
        shadow_quality: High,
        // ... other defaults
    ),
    camera: CameraConfig(
        mode: FirstPerson,
        fov: 70.0,
        light_intensity: 500000.0,
        light_range: 30.0,
        shadows_enabled: true,
        // ... other defaults
    ),
    audio: AudioConfig(
        master_volume: 0.9,
        music_volume: 0.3,
        ambient_volume: 0.9,
        sfx_volume: 0.8,
        // ... other defaults
    ),
)
```

**Rationale**:
- First-person for immersion and limited view
- Low light intensity/range for dark, tense atmosphere
- High shadow quality for atmospheric lighting
- Low music, high ambient for environmental storytelling
- Reduced SFX to avoid desensitization

---

## Loading Configuration

Configuration is loaded automatically when a campaign is launched.

### Default Behavior

If `campaigns/<name>/config.ron` is missing:

1. System prints warning to console
2. All default values are used (see above)
3. Game proceeds normally

### Custom Configuration

To customize, create `config.ron` in your campaign directory:

```bash
campaigns/
  my_campaign/
    campaign.ron
    config.ron    # ← Create this file
```

### Validation

On load, the system:

1. Parses RON file
2. Validates all fields (ranges, types)
3. Returns error if validation fails
4. Prints descriptive error message

**Example validation error**:

```text
Error: ValidationError("fov must be in range 30.0-150.0, got 200.0")
```

---

## Usage from Code

Campaign configuration is loaded in the campaign builder and distributed to game systems via Bevy plugins.

### Loading

```rust
use antares::sdk::game_config::GameConfig;
use std::path::Path;

// Load from file or use defaults
let config_path = Path::new("campaigns/tutorial/config.ron");
let config = GameConfig::load_or_default(config_path)?;
```

### Plugin Registration

```rust
// Extract subsystem configs
let graphics_config = config.graphics.clone();
let audio_config = config.audio.clone();
let camera_config = config.camera.clone();
let controls_config = config.controls.clone();

// Register plugins
app.add_plugins(GraphicsPlugin { config: graphics_config })
    .add_plugins(AudioPlugin { config: audio_config })
    .add_plugins(CameraPlugin { config: camera_config })
    .add_plugins(ControlsPlugin { config: controls_config });
```

### Runtime Access

Systems access configuration via Bevy resources:

```rust
fn audio_playback_system(
    audio_settings: Res<AudioSettings>,
    // ... other resources
) {
    // Get effective volume
    let volume = audio_settings.effective_music_volume();
    // Use for playback
}
```

---

## Best Practices

### 1. Start with Template

Copy `campaigns/config.template.ron` to your campaign directory and customize:

```bash
cp campaigns/config.template.ron campaigns/my_campaign/config.ron
```

### 2. Comment Your Changes

Add comments explaining *why* you chose specific values:

```ron
camera: CameraConfig(
    // Lower FOV for claustrophobic dungeon feel
    fov: 60.0,
    // Dim lighting creates tension
    light_intensity: 500000.0,
    // ... other fields
),
```

### 3. Test Configuration

Always test your config.ron loads correctly:

```bash
cargo run --bin antares -- campaigns/my_campaign
```

Check console for validation errors.

### 4. Version Control

Commit `config.ron` to version control with your campaign:

```bash
git add campaigns/my_campaign/config.ron
git commit -m "Add campaign configuration"
```

### 5. Profile Validation

Test edge cases and boundary values:

- Minimum/maximum FOV
- Zero volumes vs. audio disabled
- Different shadow qualities
- Various MSAA samples

---

## Future Enhancements

Planned additions to configuration system:

- **Keybinding remapping**: Runtime key rebinding via settings menu
- **Quality presets**: Low/Medium/High/Ultra profiles
- **Hardware detection**: Auto-detect optimal settings
- **Per-map overrides**: Override camera/lighting per map
- **Audio profiles**: Separate volume profiles (combat/exploration)
- **Accessibility**: Color blind modes, UI scaling, font size

---

## Related Documentation

- **Tutorial**: `docs/tutorials/creating_a_campaign.md` (creating campaigns)
- **Reference**: `docs/reference/architecture.md` (system architecture)
- **How-To**: `docs/how-to/customize_campaign_settings.md` (configuration guide)
- **Template**: `campaigns/config.template.ron` (annotated template)
- **Example**: `campaigns/tutorial/config.ron` (working example)

---

## Summary

The game configuration system provides:

- **Flexibility**: Customize graphics, audio, controls, camera per campaign
- **Defaults**: Sensible defaults if config missing
- **Validation**: Strict validation prevents invalid configurations
- **Documentation**: Comprehensive schema and examples
- **Templates**: Ready-to-use templates for common campaign styles

Campaign authors can create unique experiences by tailoring configuration to their design goals without modifying engine code.
