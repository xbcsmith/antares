# Sky System Implementation Plan

## Overview

The game currently renders a solid grey background regardless of whether the
party is indoors or outdoors, and there is no concept of a sky, weather, day
or night visuals, suns, stars, or clouds. This plan introduces a data-driven
sky system that ties visual sky rendering to the existing `GameTime` /
`TimeOfDay` clock, map-level `is_outdoor` flags, and per-map `SkyConfig` data
stored in the map RON files. The SDK Map Editor is updated to expose all sky
settings to campaign authors. The work is split into five independent phases
that can be implemented and shipped incrementally without breaking existing
campaigns.

---

## Current State Analysis

### Existing Infrastructure

| Component                  | Location                                 | Notes                                               |
| -------------------------- | ---------------------------------------- | --------------------------------------------------- |
| `Map` struct               | `src/domain/world/types.rs`              | No outdoor flag, no sky data                        |
| `MapMetadata.is_outdoor`   | `sdk/campaign_builder/src/map_editor.rs` | SDK-only; not persisted to Map RON                  |
| `TimeOfDay` enum           | `src/domain/types.rs`                    | Dawn / Morning / Afternoon / Dusk / Evening / Night |
| `GameTime`                 | `src/domain/types.rs`                    | Drives HUD clock; `time_of_day()` already available |
| `TimeOfDayPlugin`          | `src/game/systems/time.rs`               | Updates `AmbientLight` brightness per `TimeOfDay`   |
| `update_ambient_light`     | `src/game/systems/time.rs`               | Reads `GlobalState.0.time_of_day()`                 |
| Bevy `ClearColor`          | global resource                          | Set once at startup; not updated per map            |
| `classify_map_environment` | `sdk/campaign_builder/src/map_editor.rs` | Heuristic only; not stored anywhere                 |
| Map RON files              | `campaigns/tutorial/data/maps/*.ron`     | No sky fields; existing files must remain valid     |

### Identified Issues

1. **Grey sky everywhere.** `ClearColor` is never changed, so the background is
   always Bevy's default grey whether the party stands on an open plains map or
   inside a dungeon.
2. **`is_outdoor` only exists in the SDK.** The `Map` domain struct has no
   indoor/outdoor flag; the SDK's `MapMetadata.is_outdoor` is never written to
   the RON file and is never read by the game engine.
3. **No sky data in the RON format.** There is no place to author day/night sky
   colors, sun count, star density, or cloud settings at a per-map level.
4. **No celestial body rendering.** No suns, stars, or clouds exist in the
   scene graph.
5. **Time system not wired to sky.** `TimeOfDayPlugin` adjusts ambient
   brightness but does not affect sky color, celestial body visibility, or
   cloud behavior.

---

## Implementation Phases

---

### Phase 1: Domain Foundation

**Goal:** Add `is_outdoor` and `SkyConfig` to the `Map` domain struct and RON
format with full backward compatibility. No rendering changes yet.

#### 1.1 Add `SkyConfig` Struct to `src/domain/world/types.rs`

Add a new public struct immediately before the `Map` struct definition. All
fields must be `#[serde(default)]` so that existing RON files that omit the
block continue to deserialize without error.

Field definitions:

| Field                 | Type       | Default                   | Description                                                    |
| --------------------- | ---------- | ------------------------- | -------------------------------------------------------------- |
| `day_sky_color`       | `[f32; 4]` | `[0.53, 0.81, 0.98, 1.0]` | RGBA sky color during day periods (Morning, Afternoon)         |
| `dusk_dawn_sky_color` | `[f32; 4]` | `[0.98, 0.60, 0.20, 1.0]` | RGBA sky color during Dawn and Dusk                            |
| `night_sky_color`     | `[f32; 4]` | `[0.02, 0.02, 0.08, 1.0]` | RGBA sky color during Evening and Night                        |
| `sun_count`           | `u8`       | `1`                       | Number of suns rendered (0 = overcast / no sun glow)           |
| `sun_color`           | `[f32; 4]` | `[1.0, 0.95, 0.80, 1.0]`  | RGBA color of each sun disc                                    |
| `sun_size`            | `f32`      | `1.0`                     | Relative size multiplier for sun discs                         |
| `star_count`          | `u32`      | `2000`                    | Total stars in the night sky                                   |
| `star_density`        | `f32`      | `0.5`                     | 0.0–1.0 density distribution (0 = sparse, 1 = dense Milky Way) |
| `cloud_coverage`      | `f32`      | `0.3`                     | 0.0–1.0 fraction of sky covered by clouds                      |
| `cloud_color`         | `[f32; 4]` | `[0.9, 0.9, 0.9, 0.8]`    | RGBA color of cloud layer                                      |
| `cloud_density`       | `f32`      | `0.5`                     | 0.0–1.0 cloud opacity / thickness                              |
| `cloud_speed`         | `f32`      | `1.0`                     | Relative speed multiplier for cloud animation                  |

All four `[f32; 4]` color fields represent RGBA in linear color space, matching
the pattern used by `TileVisualMetadata::color_tint` throughout the codebase.

#### 1.2 Add `is_outdoor` and `sky` Fields to the `Map` Struct

In `src/domain/world/types.rs`, inside `pub struct Map { … }`, add:

```antares/src/domain/world/types.rs#L2578-2582
/// Whether the map is outdoors (affects sky rendering and light behavior).
#[serde(default)]
pub is_outdoor: bool,

/// Per-map sky configuration. Only used when `is_outdoor` is true.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub sky: Option<SkyConfig>,
```

The `skip_serializing_if` annotation means existing map files that omit the
field round-trip without adding noise.

Add a `fn default_is_outdoor() -> bool { false }` private function and wire it
to `#[serde(default = "default_is_outdoor")]` if the default derivation is
insufficient (it is not — Rust defaults `bool` to `false`).

Update `Map::new(…)` to initialize both fields to their defaults.

#### 1.3 Export `SkyConfig` from `src/domain/world/mod.rs`

Add `SkyConfig` to the existing `pub use types::{ … }` line in
[`src/domain/world/mod.rs`](../../src/domain/world/mod.rs).

#### 1.4 RON Format

The sky block in a map RON file is optional. When present it looks like:

```antares/data/test_campaign/data/maps/map_1.ron#L1-25
(
    id: 1,
    width: 20,
    height: 20,
    name: "Town Square",
    description: "A sunny town square",
    is_outdoor: true,
    sky: Some((
        day_sky_color: (0.53, 0.81, 0.98, 1.0),
        dusk_dawn_sky_color: (0.98, 0.60, 0.20, 1.0),
        night_sky_color: (0.02, 0.02, 0.08, 1.0),
        sun_count: 1,
        sun_color: (1.0, 0.95, 0.80, 1.0),
        sun_size: 1.0,
        star_count: 2000,
        star_density: 0.5,
        cloud_coverage: 0.3,
        cloud_color: (0.9, 0.9, 0.9, 0.8),
        cloud_density: 0.5,
        cloud_speed: 1.0,
    )),
    tiles: [ … ],
)
```

When `sky` is absent the `Option` deserializes to `None` and the game engine
falls back to defaults.

#### 1.5 Update `data/test_campaign` Fixture

Add `is_outdoor: true` and a minimal `sky: Some(…)` block to at least one map
in `data/test_campaign/data/maps/` so the integration tests exercise the new
fields. All other fixture maps that omit the fields must continue to load
without error.

#### 1.6 Testing Requirements

- `test_sky_config_default_values` — assert all `SkyConfig::default()` fields
  match the documented defaults.
- `test_map_with_sky_config_ron_roundtrip` — serialize a `Map` with `sky:
Some(…)`, deserialize it, assert fields are equal.
- `test_map_without_sky_config_backward_compat` — deserialize an existing map
  RON string that lacks `is_outdoor` and `sky`; assert no error and `sky ==
None`, `is_outdoor == false`.
- `test_sky_config_partial_fields_deserialize` — omit `cloud_speed` from the
  RON; assert the field takes its default value.

All tests live inside the `mod tests` block at the bottom of
`src/domain/world/types.rs`.

#### 1.7 Deliverables

- [ ] `SkyConfig` struct with all fields, defaults, and `/// doc` comments
- [ ] `is_outdoor: bool` field on `Map`
- [ ] `sky: Option<SkyConfig>` field on `Map`
- [ ] `SkyConfig` exported from `src/domain/world/mod.rs`
- [ ] `Map::new(…)` initialises both new fields
- [ ] `data/test_campaign/data/maps/` has at least one map with sky block
- [ ] 4 unit tests passing

#### 1.8 Success Criteria

- `cargo check` passes with zero errors.
- `cargo clippy … -- -D warnings` passes with zero warnings.
- All existing map RON files in `campaigns/tutorial/` load without error (no
  RON parse failures from missing fields).
- All 4 new unit tests pass.

---

### Phase 2: Sky Background Rendering Engine

**Goal:** Update `ClearColor` every frame to reflect the current map's sky
color and the current `TimeOfDay`, replacing the static grey background with a
live sky tint. Indoor maps keep a dark ambient color.

#### 2.1 Create `src/game/systems/sky.rs`

This new module provides `SkyPlugin` and the core sky update system. Register
it in [`src/bin/antares.rs`](../../src/bin/antares.rs) after `TimeOfDayPlugin`
so that time advances before the sky color is computed in the same frame.

```antares/src/game/systems/sky.rs#L1-6
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sky background color system.
//! Updates Bevy's `ClearColor` each frame based on the current map's
//! `SkyConfig` and the current `TimeOfDay`.
```

#### 2.2 Sky Color Interpolation Logic

Introduce a pure function `sky_color_for_time(config: &SkyConfig, tod:
TimeOfDay) -> [f32; 4]` that selects and blends colors:

| `TimeOfDay` | Color used                                          |
| ----------- | --------------------------------------------------- |
| `Night`     | `night_sky_color`                                   |
| `Evening`   | lerp(`night_sky_color`, `dusk_dawn_sky_color`, 0.3) |
| `Dawn`      | `dusk_dawn_sky_color`                               |
| `Morning`   | lerp(`dusk_dawn_sky_color`, `day_sky_color`, 0.7)   |
| `Afternoon` | `day_sky_color`                                     |
| `Dusk`      | `dusk_dawn_sky_color`                               |

This is a pure function with no Bevy dependencies, making it straightforwardly
unit-testable without a full Bevy world.

#### 2.3 `update_sky_background` System

The Bevy system reads `GlobalState`, gets the current `Map`, checks
`is_outdoor`, and mutates `ClearColor`:

- If `is_outdoor == false` → set `ClearColor` to `INDOOR_SKY_COLOR` (a
  constant, dark brown/grey matching a cave ceiling).
- If `is_outdoor == true` and `sky == None` → use `SkyConfig::default()`.
- If `is_outdoor == true` and `sky == Some(cfg)` → call
  `sky_color_for_time(&cfg, tod)`.

Run this system inside `SkyPlugin::build` as part of `Update`, ordered
**after** `apply_time_advance` and **before** `update_ambient_light`.

#### 2.4 Constants

```text
INDOOR_SKY_COLOR: [f32; 4] = [0.05, 0.04, 0.03, 1.0]  // near-black warm grey
DEFAULT_OUTDOOR_DAY_SKY_COLOR: [f32; 4] = [0.53, 0.81, 0.98, 1.0]
DEFAULT_OUTDOOR_NIGHT_SKY_COLOR: [f32; 4] = [0.02, 0.02, 0.08, 1.0]
DEFAULT_OUTDOOR_DUSK_DAWN_SKY_COLOR: [f32; 4] = [0.98, 0.60, 0.20, 1.0]
```

#### 2.5 Register in `src/bin/antares.rs`

Add `app.add_plugins(antares::game::systems::sky::SkyPlugin)` after
`TimeOfDayPlugin` in [`src/bin/antares.rs`](../../src/bin/antares.rs).

#### 2.6 Export from `src/game/systems/mod.rs`

Add `pub mod sky;` to
[`src/game/systems/mod.rs`](../../src/game/systems/mod.rs).

#### 2.7 Testing Requirements

- `test_sky_color_for_time_night` — assert `sky_color_for_time` returns
  `night_sky_color` for `TimeOfDay::Night`.
- `test_sky_color_for_time_afternoon` — assert returns `day_sky_color` for
  `TimeOfDay::Afternoon`.
- `test_sky_color_for_time_dusk` — assert returns `dusk_dawn_sky_color` for
  `TimeOfDay::Dusk`.
- `test_sky_color_for_time_evening_is_blend` — assert the evening result is
  strictly between night and dusk/dawn colors.
- `test_sky_color_for_time_morning_is_blend` — similar for morning.
- `test_sky_color_all_periods_produce_valid_rgba` — iterate all six
  `TimeOfDay` variants with a default config; assert each component in
  `[0.0, 1.0]`.

#### 2.8 Deliverables

- [ ] `src/game/systems/sky.rs` with `SkyPlugin`, `update_sky_background`,
      `sky_color_for_time`, `INDOOR_SKY_COLOR` constant, and doc comments.
- [ ] `pub mod sky` added to `src/game/systems/mod.rs`
- [ ] `SkyPlugin` registered in `src/bin/antares.rs`
- [ ] 6 unit tests passing

#### 2.9 Success Criteria

- Running the game on an outdoor map during Morning shows a light-blue
  background.
- Running the game on an outdoor map during Night shows a near-black background.
- Running the game on an indoor/dungeon map shows the dark indoor color
  regardless of time.
- All quality gates pass.

---

### Phase 3: SDK Map Editor Integration

**Goal:** Expose all `is_outdoor` and `SkyConfig` fields in the SDK Map
Editor's Metadata panel so campaign authors can author sky settings visually
and save them to map RON files. Authoring tools are unblocked here — before
the celestial-body and cloud rendering is built — so content work can proceed
in parallel with Phases 4 and 5.

#### 3.1 Extend `MapMetadata` in `sdk/campaign_builder/src/map_editor.rs`

The existing `MapMetadata.is_outdoor` field is already present but was never
wired to the Map RON. Add a `sky_config` field to `MapMetadata`:

```text
pub sky_config: Option<SkyConfig>,
```

Default: `None` (sky section collapsed/hidden until the user enables it).

#### 3.2 Wire `MapMetadata` ↔ `Map` During Load and Save

In `MapEditorState::new(map: Map)` (load path):

- Set `metadata.is_outdoor = map.is_outdoor`
- Set `metadata.sky_config = map.sky.clone()`

In `MapEditorState::save_to_ron` / `save_map` (save path):

- Write `map.is_outdoor = metadata.is_outdoor`
- Write `map.sky = metadata.sky_config.clone()`

Previously, `is_outdoor` was stored only in `MapMetadata` and never saved to
the Map RON — this phase fixes that gap.

#### 3.3 `show_metadata_editor` UI Additions

In `MapsEditorState::show_metadata_editor`, immediately after the existing
**"Outdoor Map"** checkbox:

1. **Outdoor Map** checkbox (already exists; now also writes to `map.is_outdoor`).

2. **Sky Settings** collapsible section (only shown when `is_outdoor` is
   true):

   - **Enable Custom Sky** checkbox — toggles `sky_config` between `None`
     and `Some(SkyConfig::default())`.

   When `sky_config == Some(cfg)`:

   - **Day Sky Color** — `egui::color_picker::color_edit_button_rgba`
     for `cfg.day_sky_color`.
   - **Dusk/Dawn Sky Color** — color picker for `cfg.dusk_dawn_sky_color`.
   - **Night Sky Color** — color picker for `cfg.night_sky_color`.
   - **Sun Count** — `egui::DragValue::new(…).range(0..=8)` for
     `cfg.sun_count`.
   - **Sun Color** — color picker for `cfg.sun_color`.
   - **Sun Size** — `egui::Slider::new(…, 0.1..=5.0)` for `cfg.sun_size`.
   - **Stars** sub-section:
     - **Star Count** — `egui::DragValue::new(…).range(0..=10000)` for
       `cfg.star_count`.
     - **Star Density** — `egui::Slider::new(…, 0.0..=1.0)` for
       `cfg.star_density`.
   - **Clouds** sub-section:
     - **Cloud Coverage** — `egui::Slider::new(…, 0.0..=1.0)` for
       `cfg.cloud_coverage`.
     - **Cloud Color** — color picker for `cfg.cloud_color`.
     - **Cloud Density** — `egui::Slider::new(…, 0.0..=1.0)` for
       `cfg.cloud_density`.
     - **Cloud Speed** — `egui::Slider::new(…, 0.0..=5.0)` for
       `cfg.cloud_speed`.

   Each changed widget sets `editor.has_changes = true`.

#### 3.4 egui Layout Rules (Implementation Rule 6 Compliance)

The sky settings section lives inside the existing `show_metadata_editor`
vertical group. It does **not** use a multi-column layout, so Implementation
Rule 6 (`allocate_ui` requirements) does not apply here. Individual color
pickers and sliders are standard single-column egui controls.

#### 3.5 `classify_map_environment` Update

Replace the heuristic in `classify_map_environment` with an authoritative
check:

```text
if map.is_outdoor { ("Outdoor", …) } else { ("Indoor", …) }
```

Fall back to the existing wall-ratio heuristic only when `is_outdoor` is
`false` and `allow_random_encounters` is `true` (to distinguish dungeon from
town — the heuristic label becomes "Dungeon").

#### 3.6 Testing Requirements

- `test_map_metadata_is_outdoor_writes_to_map_ron` — save a map with
  `is_outdoor = true`; reload the RON; assert `map.is_outdoor == true`.
- `test_map_metadata_sky_config_round_trip` — set `sky_config = Some(…)`;
  save; reload; assert all sky fields equal.
- `test_map_metadata_sky_config_none_not_written` — `sky_config = None` →
  saved RON omits the `sky` key entirely.
- `test_classify_map_environment_uses_is_outdoor_flag` — `is_outdoor = true`
  → label "Outdoor" regardless of wall ratio.
- `test_metadata_sky_section_only_shown_when_outdoor` — verify `sky_config`
  remains `None` when `is_outdoor` is unchecked.

All five tests go into `mod tests` in
`sdk/campaign_builder/src/map_editor.rs`.

#### 3.7 Deliverables

- [ ] `sky_config: Option<SkyConfig>` field on `MapMetadata`.
- [ ] Load path: `MapEditorState::new` reads `map.is_outdoor` and `map.sky`.
- [ ] Save path: `save_map` writes `map.is_outdoor` and `map.sky`.
- [ ] Sky settings UI section in `show_metadata_editor` (collapsed when
      `is_outdoor = false`).
- [ ] `classify_map_environment` uses `map.is_outdoor` authoritatively.
- [ ] 5 unit tests passing.

#### 3.8 Success Criteria

- Opening an existing map in the SDK does not crash or clear the sky config.
- Setting `is_outdoor = true` and configuring sky settings, saving, and
  reopening the campaign preserves all sky values exactly.
- A map with `is_outdoor = false` has no `sky` key in the saved RON.
- All quality gates pass.

---

### Phase 4: Celestial Bodies — Suns and Stars

**Goal:** Spawn billboard sun entities for outdoor day maps and a star-field
entity for outdoor night maps. Visibility toggles automatically with
`TimeOfDay`.

#### 4.1 `SkyBodyPlugin` and `SkyBodyState` Bevy Resource

Add a `SkyBodyState` resource to track spawned entity IDs for suns and stars,
enabling despawn/respawn when the map changes.

Introduce `SkyBodyPlugin` in `src/game/systems/sky.rs` (same file as Phase 2)
or split into `sky_bodies.rs` if the file exceeds ~400 lines.

#### 4.2 Sun Entities

On map load, read `sky.sun_count` (defaulting to 1). For each sun:

- Spawn a `PbrBundle` (or a flat `Mesh2dBundle` in screen space) using a
  procedural disc mesh scaled by `sun_size`.
- Position suns in the upper hemisphere of the scene using fixed azimuth
  offsets: for `sun_count = 1` → center-left of sky; for `sun_count = 2` →
  left and right symmetric; for `sun_count = N > 2` → evenly distributed on
  a 120° arc.
- Apply `sun_color` as the base color with high emissive factor so the disc
  glows regardless of ambient light.
- Tag with a `SunMarker` component so they can be queried and despawned.

#### 4.3 Star Field Entity

On map load, read `sky.star_count` and `sky.star_density`. Spawn a single
entity with a `Mesh` containing `star_count` vertices scattered across a
hemisphere using a seeded RNG (seed derived from the map ID for determinism).
Each vertex represents one star rendered as a `PointList` primitive or a
screen-space `Mesh2dBundle` overlay.

- Tag with `StarFieldMarker`.
- Set vertex colors to white with varying alpha driven by `star_density`.

#### 4.4 Visibility Toggle System

Add `update_sky_body_visibility` system:

- `is_outdoor == false` → hide all `SunMarker` and `StarFieldMarker` entities.
- `is_outdoor == true` and `TimeOfDay::is_day()` → show suns, hide stars.
- `is_outdoor == true` and `TimeOfDay::is_dark()` (Evening or Night) → hide
  suns, show stars.
- Dawn and Dusk → show both at reduced opacity (lerp visibility).

Use Bevy's `Visibility` component to toggle, not entity despawn (to avoid
re-spawning every frame).

#### 4.5 Map Change Detection

When `GlobalState`'s current map ID changes, despawn all `SunMarker` /
`StarFieldMarker` entities and re-spawn with the new map's sky config. Use a
`Local<MapId>` system parameter to track the previous map ID.

#### 4.6 Testing Requirements

- `test_sun_positions_one_sun` — assert a single sun uses the center-left
  azimuth.
- `test_sun_positions_two_suns` — assert two suns are symmetric.
- `test_sun_count_zero_spawns_nothing` — `sun_count = 0` → no `SunMarker`
  entities.
- `test_star_count_zero_spawns_empty_field` — `star_count = 0` → empty mesh.
- `test_sky_body_visibility_night_shows_stars` — verify visibility logic pure
  function.
- `test_sky_body_visibility_afternoon_shows_suns` — similar.

#### 4.7 Deliverables

- [ ] `SunMarker` and `StarFieldMarker` Bevy components in
      `src/game/components/sky.rs` (new file) exported from
      `src/game/components/mod.rs`.
- [ ] `spawn_sky_bodies(map: &Map)` helper function.
- [ ] `despawn_sky_bodies` helper function.
- [ ] `update_sky_body_visibility` system.
- [ ] `SkyBodyPlugin` registered in `src/bin/antares.rs`.
- [ ] 6 unit tests passing.

#### 4.8 Success Criteria

- Outdoor map during Afternoon shows 1 (or N) glowing sun disc(s) in the sky.
- Outdoor map during Night shows a star field.
- Outdoor map at Dawn shows both, fading.
- Indoor map shows neither suns nor stars.
- All quality gates pass.

---

### Phase 5: Cloud Layer

**Goal:** Add a procedural cloud layer above the scene for outdoor maps,
controlled by `cloud_coverage`, `cloud_color`, `cloud_density`, and
`cloud_speed` from `SkyConfig`.

#### 5.1 Cloud Mesh Generation

Create a `CloudLayer` entity per map load:

- Generate a flat mesh plane at altitude `y = MAP_CLOUD_HEIGHT` (a new
  constant, e.g. `40.0` units).
- Tile the plane with alpha-blended cloud quads procedurally distributed via
  a seeded random (map ID seed) according to `cloud_coverage`.
- Each cloud quad uses a white noise texture (generate procedurally at
  startup using Bevy's `Image` API) tinted by `cloud_color`.
- Opacity = `cloud_density * cloud_coverage`.
- When `cloud_coverage < 0.05` → skip cloud entity entirely.

#### 5.2 Cloud Animation

Add `animate_clouds` system that translates the `CloudLayer` entity's
`Transform.translation.x` by `cloud_speed * delta_seconds` each frame, and
wraps the position when it exceeds half the mesh width. This gives a slow
east-to-west drift.

#### 5.3 Cloud Visibility

- `is_outdoor == false` → hide or despawn cloud entity.
- `cloud_coverage == 0.0` → no cloud entity spawned.
- Clouds are visible during all outdoor `TimeOfDay` periods (darker at night
  because ambient light is lower).

#### 5.4 `CloudLayerMarker` Component

Add to `src/game/components/sky.rs` alongside `SunMarker` and
`StarFieldMarker`.

#### 5.5 Testing Requirements

- `test_cloud_coverage_zero_skips_spawn` — `cloud_coverage = 0.0` → no
  `CloudLayerMarker` entity.
- `test_cloud_density_affects_opacity` — `cloud_density = 0.0` → alpha = 0.
- `test_animate_clouds_wraps_position` — cloud animation wraps at mesh width
  boundary.
- `test_cloud_color_applied_to_material` — verify tint color matches config.

#### 5.6 Deliverables

- [ ] `CloudLayerMarker` component in `src/game/components/sky.rs`.
- [ ] `spawn_cloud_layer(map: &Map, meshes, materials)` helper.
- [ ] `despawn_cloud_layer` helper.
- [ ] `animate_clouds` Bevy system.
- [ ] `MAP_CLOUD_HEIGHT: f32` constant.
- [ ] Cloud spawning wired into `SkyBodyPlugin` map-change detection.
- [ ] 4 unit tests passing.

#### 5.7 Success Criteria

- Outdoor map with `cloud_coverage = 0.8` shows a drifting cloud layer.
- Outdoor map with `cloud_coverage = 0.0` shows no clouds.
- Indoor map shows no clouds.
- All quality gates pass.

---

## Cross-Cutting Concerns

### Backward Compatibility

All new fields on `Map` use `#[serde(default)]` or
`#[serde(default, skip_serializing_if = "Option::is_none")]`. All existing map
RON files (`campaigns/tutorial/data/maps/*.ron`) must continue to load without
modification. This is verified by the existing campaign loader integration
tests which run against `data/test_campaign`.

### Test Data

All test fixtures live under `data/test_campaign/data/maps/`. No test may
reference `campaigns/tutorial`. At least one map in the test campaign gets a
`sky` block added in Phase 1 to exercise the new fields.

### Architecture Compliance

| Rule                                                       | How satisfied                           |
| ---------------------------------------------------------- | --------------------------------------- |
| `SkyConfig` fields use explicit types, no `usize`          | ✅ `u8`, `u32`, `f32`, `[f32; 4]`       |
| RON format for game data                                   | ✅ sky config lives in map `.ron` files |
| `SkyConfig` exported from `domain::world`                  | ✅ Phase 1 export                       |
| Doc comments on all public items                           | ✅ required by Implementation Rule 4    |
| SPDX header on all `.rs` files                             | ✅ `sky.rs` and `sky.rs` components     |
| No multi-column egui layouts without `allocate_ui`         | ✅ Sky UI is single-column              |
| Test data in `data/test_campaign` not `campaigns/tutorial` | ✅ Implementation Rule 5                |

### File Change Summary

| File                                     | Change                                                                                                          |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `src/domain/world/types.rs`              | Add `SkyConfig` struct; add `is_outdoor` and `sky` to `Map`                                                     |
| `src/domain/world/mod.rs`                | Export `SkyConfig`                                                                                              |
| `src/game/systems/sky.rs`                | New: `SkyPlugin`, `update_sky_background`, `sky_color_for_time`, `update_sky_body_visibility`, `animate_clouds` |
| `src/game/systems/mod.rs`                | Add `pub mod sky`                                                                                               |
| `src/game/components/sky.rs`             | New: `SunMarker`, `StarFieldMarker`, `CloudLayerMarker`                                                         |
| `src/game/components/mod.rs`             | Add `pub mod sky`                                                                                               |
| `src/bin/antares.rs`                     | Register `SkyPlugin` and `SkyBodyPlugin`                                                                        |
| `sdk/campaign_builder/src/map_editor.rs` | Add `sky_config` to `MapMetadata`; wire load/save; add sky UI; fix `classify_map_environment`                   |
| `data/test_campaign/data/maps/map_*.ron` | Add `is_outdoor` and `sky` to at least one map                                                                  |

---

## Implementation Order

Implement phases 1 through 5 in numerical order. Each phase depends on the
previous one: the data model (Phase 1) must exist before anything else;
the background sky color (Phase 2) must be live before SDK controls
(Phase 3) are worth testing end-to-end; celestial bodies (Phase 4) require
both the background and the authoring data to be in place; and clouds
(Phase 5) are the highest-complexity rendering work and come last.
