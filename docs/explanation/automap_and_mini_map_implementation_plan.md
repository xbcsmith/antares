# Automap and Mini Map Implementation Plan

## Overview

Antares already stores `visited: bool` on every `Tile` and marks tiles visited
during movement. This plan extends that foundation in five sequential phases:
widen fog-of-war coverage so nearby tiles are revealed automatically, build the
mini map widget merged with compass and clock into a single top-right panel,
add the full-screen automap overlay (M key, configurable), build a color-coded
POI marker layer with an on-screen legend, and finish with config file and
Campaign Builder SDK integration.

---

## Current State Analysis

### Existing Infrastructure

- `Tile.visited` in
  [src/domain/world/types.rs](../../src/domain/world/types.rs) — per-tile visit
  flag; `mark_visited()` is already called on each tile the party steps onto in
  [src/domain/world/movement.rs](../../src/domain/world/movement.rs).
- `CompassRoot` / `CompassText` in
  [src/game/systems/hud.rs](../../src/game/systems/hud.rs) — absolute-positioned
  at `top: 20px, right: 20px`; this is the anchor for the new combined
  top-right panel.
- Time System plan (`docs/explanation/time_system_implementation_plan.md`)
  targets a `ClockRoot` slot directly below `CompassRoot`; the combined panel
  layout must reserve that space.
- `Map` struct (`src/domain/world/types.rs`) — `id`, `width`, `height`,
  `tiles`, `events`, `npc_placements`; all required automap data is already
  present.
- `MapEvent` variants (`Encounter`, `Treasure`, `Teleport`, `Trap`, `Sign`,
  `NpcDialogue`) and `NpcPlacement` — sources for POI markers.
- `GameMode` enum (`src/application/mod.rs`) — extensible; adding `Automap` is
  a one-liner.
- `GameAction` + `KeyMap` + `ControlsConfig`
  (`src/game/systems/input.rs`, `src/sdk/game_config.rs`) — fully config-driven
  key bindings; `parse_key_code("M")` already resolves correctly.
- `GlobalState` resource — exposes current map, party position, world, and game
  mode to all Bevy systems.
- Save/load already serializes `Map.tiles`, so visited state persists across
  sessions with no additional work.

### Identified Issues

- `mark_visited()` is only called on the exact tile the party steps onto.
  Adjacent tiles within sight are **not** revealed, so the automap will only
  show the literal path walked rather than a sensible fog-of-war radius.
- The starting tile of a map is not marked visited on initial load; the
  player's spawn position appears dark until they move away and back.
- `CompassRoot` is a lone absolute-positioned node. Merging compass + clock +
  mini map into a single column requires refactoring the compass spawn in
  `setup_hud`.
- No `GameMode::Automap`, `GameAction::Automap`, or `automap` key binding exist.
- `ControlsConfig` has no `automap` field.
- `GraphicsConfig` has no `show_minimap` toggle.
- SDK Config Editor has no Automap key-binding or mini map toggle.

---

## Implementation Phases

### Phase 1: Fog-of-War Foundation

Ensures all tile visibility events are captured correctly before any rendering
is built on top.

#### 1.1 Widen Visibility Radius on Movement

Add a helper `mark_visible_area(world: &mut World, center: Position, radius: u32)`
in `src/domain/world/movement.rs` (or a new
`src/domain/world/visibility.rs`). The function iterates the Chebyshev square
of tiles within `radius` of `center` and calls `mark_visited()` on each
in-bounds tile. Extract the default as a module-level constant:

```rust
pub const VISIBILITY_RADIUS: u32 = 1;
```

Replace the single `tile.mark_visited()` call in `move_party` with a call to
`mark_visible_area(world, new_pos, VISIBILITY_RADIUS)`.

#### 1.2 Mark Starting Area on Map Load

In `src/game/systems/map.rs`, inside the system that handles `MapChangeMessage`
/ `handle_map_change`, after the world's current map is set, call
`mark_visible_area` on the party's starting position so the initial area is
immediately revealed.

#### 1.3 Verify Save/Load Round-Trip

Confirm `Tile.visited` is already serialized/deserialized through the existing
RON save path. Add a unit test `test_visited_persists_after_save_load` in
`src/domain/world/movement.rs` (or `tests/`) that marks a tile, serializes
world state, deserializes, and asserts `visited == true`.

#### 1.4 Testing Requirements

- `test_mark_visible_area_marks_radius` — 5×5 map, call at center with
  `radius = 1`, assert all 9 surrounding tiles are visited.
- `test_mark_visible_area_clamps_to_bounds` — call at a corner tile, assert no
  out-of-bounds panic and only valid tiles are marked.
- `test_visited_persists_after_save_load` — serialize/deserialize map, assert
  tile visited state is preserved.
- `test_starting_tile_marked_on_map_load` — simulate map-change event, assert
  the party's start tile is visited.

#### 1.5 Deliverables

- [ ] `mark_visible_area(world, pos, radius)` helper
- [ ] `VISIBILITY_RADIUS` constant
- [ ] Starting-area mark wired in `src/game/systems/map.rs`
- [ ] All phase-1 tests passing

#### 1.6 Success Criteria

After any movement event all tiles within `VISIBILITY_RADIUS` of the player are
marked visited. Save/load round-trip preserves visited state. No regressions in
existing movement tests.

---

### Phase 2: Top-Right Panel Consolidation and Mini Map Widget

Replaces the standalone `CompassRoot` with a vertical column panel stacking
mini map → compass → clock (clock placeholder slots into the Time System plan).

#### 2.1 New Top-Right Panel Container

In `src/game/systems/hud.rs`, replace the bare `CompassRoot` spawn with a
parent `TopRightPanel` node:

```rust
Node {
    position_type: PositionType::Absolute,
    top: Val::Px(20.0),
    right: Val::Px(20.0),
    flex_direction: FlexDirection::Column,
    align_items: AlignItems::Center,
    row_gap: Val::Px(4.0),
    ..default()
}
```

Add marker components:

```rust
#[derive(Component)] pub struct TopRightPanel;
#[derive(Component)] pub struct MiniMapRoot;
#[derive(Component)] pub struct MiniMapCanvas;   // ImageNode driven by dynamic image
```

Add constants adjacent to the existing compass constants:

```rust
pub const MINI_MAP_SIZE_PX: f32 = 80.0;
pub const MINI_MAP_VIEWPORT_RADIUS: u32 = 6;     // tiles visible in each direction
pub const MINI_MAP_TILE_PX: f32 = 6.0;           // pixels per tile
pub const MINI_MAP_BG_COLOR: Color = Color::srgba(0.05, 0.05, 0.05, 0.9);
pub const MINI_MAP_VISITED_FLOOR: [u8; 4] = [100, 100, 100, 255];
pub const MINI_MAP_WALL: [u8; 4] = [60, 60, 60, 255];
pub const MINI_MAP_PLAYER: [u8; 4] = [255, 255, 255, 255];
pub const MINI_MAP_UNVISITED: [u8; 4] = [0, 0, 0, 0];   // transparent
```

#### 2.2 `MiniMapImage` Resource and `update_mini_map` System

Add a `MiniMapImage` resource holding a `Handle<Image>` for a dynamically-
rewritten RGBA8 `Image` asset (`MINI_MAP_SIZE_PX × MINI_MAP_SIZE_PX` pixels).
The `update_mini_map` system runs every frame guarded by `not_in_combat`:

1. Get party position and current map from `GlobalState`.
2. Compute viewport: tiles in
   `[party_x − MINI_MAP_VIEWPORT_RADIUS, party_x + MINI_MAP_VIEWPORT_RADIUS]`
   × same for y.
3. For each pixel in the image, map to a tile coordinate.
4. Out-of-bounds → `MINI_MAP_UNVISITED` (transparent).
5. `tile.visited == false` → `MINI_MAP_UNVISITED` (fog of war).
6. Visited wall → `MINI_MAP_WALL`.
7. Visited walkable floor → `MINI_MAP_VISITED_FLOOR`.
8. Party position tile → `MINI_MAP_PLAYER`.
9. Write to the `Image` asset via `images.get_mut(handle)`.

The `MiniMapCanvas` node holds `ImageNode` referencing this handle.

#### 2.3 NPC Dots on Mini Map

After terrain rendering in `update_mini_map`, iterate `npc_placements` on the
current map. For each NPC whose tile is within the viewport and has
`tile.visited == true`, write a 2×2 pixel dot using `MINI_MAP_NPC_COLOR`
(`[0, 200, 100, 255]`). Full POI layer comes in Phase 4.

#### 2.4 Compass Reparenting and Clock Placeholder

Move the compass spawn entirely inside the `TopRightPanel` column — the
`CompassRoot` marker and `update_compass` system are unchanged. Spawn a
`ClockRoot` node (`display: Display::None`, `width: Px(COMPASS_SIZE)`) as the
bottom child of `TopRightPanel` so the Time System plan's Phase 3 can activate
it without layout changes.

#### 2.5 Testing Requirements

- `test_mini_map_image_dimensions` — assert image width and height equal
  `MINI_MAP_SIZE_PX as u32`.
- `test_mini_map_player_pixel_is_white` — party at (5,5) on a 13×13 map, run
  `update_mini_map`, assert center pixel is `MINI_MAP_PLAYER`.
- `test_mini_map_unvisited_is_transparent` — unvisited tile in viewport → pixel
  alpha == 0.
- `test_mini_map_visited_wall_color` — visited wall tile → pixel matches
  `MINI_MAP_WALL`.

#### 2.6 Deliverables

- [ ] `TopRightPanel`, `MiniMapRoot`, `MiniMapCanvas` marker components
- [ ] `MiniMapImage` resource and startup initialization
- [ ] `update_mini_map` system registered in `HudPlugin` (.run_if(not_in_combat))
- [ ] `CompassRoot` reparented inside `TopRightPanel`
- [ ] `ClockRoot` placeholder slot reserved inside `TopRightPanel`
- [ ] All phase-2 tests passing

#### 2.7 Success Criteria

Mini map appears above the compass in the top-right corner, shows explored
tiles centered on the player, scrolls as the player moves, and renders a white
dot for the player position. Unvisited tiles are transparent (fog of war).

---

### Phase 3: Full-Screen Automap Overlay

Activates on the M key, renders the entire map with fog-of-war coloring.

#### 3.1 `GameMode::Automap` and New Input Action

Add `Automap` to the `GameMode` enum in `src/application/mod.rs`:

```rust
/// Full-screen automap overlay is open
Automap,
```

Add `Automap` to `GameAction` in `src/game/systems/input.rs`. Add
`automap: Vec<String>` to `ControlsConfig` in `src/sdk/game_config.rs`:

```rust
#[serde(default = "default_automap_keys")]
pub automap: Vec<String>,

fn default_automap_keys() -> Vec<String> { vec!["M".to_string()] }
```

Wire `automap` key parsing in `KeyMap::from_controls_config`. In
`handle_input`, when in `Exploration` mode and the `Automap` action fires, set
`game_state.mode = GameMode::Automap`. When in `Automap` mode and either
`Automap` or `Menu` (Escape) fires, set `game_state.mode = GameMode::Exploration`.

#### 3.2 Automap Overlay Bevy Systems

Add `AutomapPlugin` (or extend `HudPlugin`) with these systems:

- `setup_automap` (Startup): spawn a full-screen overlay node
  (`width: Percent(100)`, `height: Percent(100)`, `position_type: Absolute`,
  `display: Display::None`, `ZIndex(10)`) with `AutomapRoot`. Children: map
  canvas `AutomapCanvas`, right-side legend column `AutomapLegend`, and a
  bottom-left hint text ("M / Esc — close map").
- `update_automap_visibility` (Update): set `AutomapRoot` display to
  `Display::Flex` when `mode == GameMode::Automap`, `Display::None` otherwise.
- `update_automap_image` (Update, `.run_if(in_automap_mode)`): produces a full
  map `AutomapImage` resource. Scale: `floor(overlay_px / max(map_w, map_h))`
  pixels per tile, clamped 4–16 px per tile.

Add marker components:

```rust
#[derive(Component)] pub struct AutomapRoot;
#[derive(Component)] pub struct AutomapCanvas;
#[derive(Component)] pub struct AutomapLegend;
```

#### 3.3 Automap Color Coding

| Tile state | Color (RGBA) |
|---|---|
| Unvisited | `[0, 0, 0, 255]` — black (fog) |
| Visited floor / ground | `[120, 120, 120, 255]` — gray |
| Visited wall | `[70, 50, 50, 255]` — dark red-gray |
| Visited door | `[180, 140, 80, 255]` — tan |
| Visited water | `[60, 80, 160, 255]` — blue |
| Visited forest / grass | `[50, 120, 50, 255]` — dark green |
| Player position | `[255, 255, 255, 255]` — white |

#### 3.4 Testing Requirements

- `test_gamemode_automap_toggle` — M from Exploration → mode becomes Automap;
  M again → Exploration.
- `test_gamemode_automap_escape_closes` — Escape from Automap → Exploration.
- `test_automap_image_unvisited_is_black` — unvisited tile → black pixel.
- `test_automap_image_visited_floor_is_gray` — visited floor → correct gray.
- `test_controls_config_default_automap_key` — default config has `"M"` in
  `automap`.

#### 3.5 Deliverables

- [ ] `GameMode::Automap` variant
- [ ] `GameAction::Automap` + key parsing in `KeyMap`
- [ ] `automap: Vec<String>` in `ControlsConfig` with `serde(default)`
- [ ] `AutomapPlugin` with setup, visibility toggle, and image-update systems
- [ ] Full fog-of-war automap rendering with terrain color coding
- [ ] M / Escape toggle wired in `handle_input`
- [ ] All phase-3 tests passing

#### 3.6 Success Criteria

Pressing M opens a full-screen automap; pressing M or Escape closes it.
Unvisited tiles are black. Visited tiles are color-coded by terrain and wall
type. Player position is clearly visible as a white marker.

---

### Phase 4: POI Markers and Legend

Adds semantically-meaningful colored dots on both mini map and automap and a
legend on the automap overlay.

#### 4.1 `PointOfInterest` Enum and Collection Helper

Add to `src/domain/world/types.rs` (or a new `src/domain/world/poi.rs`):

```rust
/// A notable location on a map, shown as a colored dot on maps
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointOfInterest {
    QuestObjective { quest_id: u32 },
    Merchant,
    Sign,
    Teleport,
    Encounter,
    Treasure,
}
```

Add `collect_map_pois(map: &Map, active_quests: &[Quest]) -> Vec<(Position, PointOfInterest)>`:
iterates `map.events` and `map.npc_placements`, returning only entries whose
map position has `tile.visited == true`.

Quest-objective positions are sourced from the active quest list in
`GlobalState`; any quest-objective tile on the current map that has been
visited is included.

#### 4.2 POI Color Constants

Add to `src/game/systems/hud.rs`:

```rust
pub const POI_QUEST_COLOR: [u8; 4]    = [255, 220,   0, 255]; // yellow
pub const POI_MERCHANT_COLOR: [u8; 4] = [  0, 200, 100, 255]; // green
pub const POI_SIGN_COLOR: [u8; 4]     = [180, 180, 255, 255]; // light blue
pub const POI_TELEPORT_COLOR: [u8; 4] = [200, 100, 255, 255]; // purple
pub const POI_ENCOUNTER_COLOR: [u8; 4]= [220,  50,  50, 255]; // red
pub const POI_TREASURE_COLOR: [u8; 4] = [255, 180,   0, 255]; // gold
```

#### 4.3 Integrate POIs into Mini Map and Automap

After terrain rendering in both `update_mini_map` and `update_automap_image`,
call `collect_map_pois` and overlay colored pixel dots at each POI position
that falls inside the current viewport (mini map) or the full canvas
(automap). Dot size: 2×2 pixels on the mini map, 3×3 pixels on the automap.

#### 4.4 Legend Panel

The `AutomapLegend` column node is populated during `setup_automap` with one
static entry per `PointOfInterest` variant plus the player-position white
square. Each entry is a horizontal row: a `20×20px` colored `BackgroundColor`
square followed by a label `Text` node. No per-frame updates are required —
the legend is static.

Legend entries (top to bottom):

| Color | Label |
|---|---|
| White | You are here |
| Yellow | Quest objective |
| Green | Merchant |
| Light blue | Sign / notice |
| Purple | Teleport |
| Red | Monster encounter |
| Gold | Treasure |

#### 4.5 Testing Requirements

- `test_collect_map_pois_only_visited` — map with merchant NPC on visited tile
  and encounter event on unvisited tile; assert only the merchant is returned.
- `test_collect_map_pois_encounter` — `MapEvent::Encounter` at visited tile →
  `PointOfInterest::Encounter` in result.
- `test_collect_map_pois_treasure` — `MapEvent::Treasure` at visited tile →
  `PointOfInterest::Treasure`.
- `test_mini_map_poi_dot_rendered` — visited NPC tile → pixel matches
  `POI_MERCHANT_COLOR` at the expected coordinate.

#### 4.6 Deliverables

- [ ] `PointOfInterest` enum + `collect_map_pois` helper
- [ ] POI color constants in `src/game/systems/hud.rs`
- [ ] POI overlay integrated into `update_mini_map`
- [ ] POI overlay integrated into `update_automap_image`
- [ ] Legend panel spawned in `setup_automap`
- [ ] All phase-4 tests passing

#### 4.7 Success Criteria

Both mini map and automap show color-coded dots for merchants, quest
objectives, signs, teleports, encounters, and treasure — only on visited tiles.
The automap legend panel explains each symbol.

---

### Phase 5: Config, Save/Load Verification, and SDK Integration

#### 5.1 `ControlsConfig` and `GraphicsConfig` Additions

In `src/sdk/game_config.rs`:

- `ControlsConfig`: `automap: Vec<String>` with
  `#[serde(default = "default_automap_keys")]` (done in Phase 3; verify here).
- `GraphicsConfig`: add `show_minimap: bool` with
  `#[serde(default = "default_show_minimap")]` and
  `fn default_show_minimap() -> bool { true }`.

In `update_mini_map`, query the `GameConfig` resource: if
`config.graphics.show_minimap == false`, set `MiniMapRoot` display to
`Display::None` and return early.

#### 5.2 Config Template and Campaign Config Updates

- Update `campaigns/config.template.ron`: document `automap` under controls
  and `show_minimap` under graphics with comments.
- Update `data/test_campaign/config.ron`: add `automap: ["M"]` and
  `show_minimap: true`.
- Update `campaigns/tutorial/config.ron`: same additions.

Both fields use `#[serde(default)]` so old config files load without error.

#### 5.3 Save/Load Verification

`Tile.visited` is already part of the serialized `Map`. Run the existing
campaign integration tests (`tests/campaign_integration_test.rs`) to confirm
automap state round-trips through `SaveGame`. If they do not cover visited
state, add `test_automap_state_round_trips_save` in `tests/`.

#### 5.4 SDK Config Editor

In `sdk/campaign_builder/src/config_editor.rs`:

- Add `controls_automap_buffer: String` to `ConfigEditorState`.
- Add a **Show Mini Map** checkbox in the Graphics section wired to
  `game_config.graphics.show_minimap`.
- Add an **Automap Key** binding field in the Controls section, using the
  identical capture/clear/validate pattern as the existing key-binding fields.
- Update `update_edit_buffers`, `update_config_from_buffers`, and
  `validate_controls_section` to cover the new fields.

#### 5.5 Testing Requirements

- `test_controls_config_serde_automap_default` — deserializing an old config
  without `automap` key → default `["M"]` applied via serde default.
- `test_graphics_config_serde_show_minimap_default` — deserializing config
  without `show_minimap` → default `true`.
- `test_mini_map_hidden_when_show_minimap_false` — set `show_minimap = false`,
  assert `MiniMapRoot` node has `display == Display::None`.

#### 5.6 Deliverables

- [ ] `show_minimap: bool` in `GraphicsConfig` with `serde(default)`
- [ ] `automap` key confirmed/verified in `ControlsConfig`
- [ ] `campaigns/config.template.ron` updated
- [ ] `data/test_campaign/config.ron` updated
- [ ] `campaigns/tutorial/config.ron` updated
- [ ] SDK Config Editor: mini map toggle + automap key binding field
- [ ] All phase-5 tests passing

#### 5.7 Success Criteria

Automap key is configurable per campaign. Mini map can be toggled off via
config. Old config files without the new fields load without error, defaulting
to sensible values. The Campaign Builder exposes both settings.

---

## Architectural Notes

- **No new data files required.** All automap data (`visited` flags, map
  dimensions, events, NPC placements) is already part of the existing `Map`
  struct and serialization.
- **Dynamic image pattern.** `MiniMapImage` and `AutomapImage` each hold a
  single `Handle<Image>` for a CPU-side RGBA8 pixel buffer that is rewritten
  every frame (mini map) or on open (automap). This avoids spawning hundreds of
  individual `Node` entities per tile and integrates cleanly with Bevy's
  `Assets<Image>` API.
- **`TopRightPanel` compatibility.** The Time System plan's `ClockRoot` must be
  spawned as the third child of `TopRightPanel` (below the compass). This plan
  reserves that slot; the Time System plan should simply activate the clock node
  rather than spawning a new absolute-positioned root.
- **`GameMode::Automap` is non-persistent.** It is never saved to the save
  file; on load the mode always restores to `Exploration`.
