# Implementations

## Phase 1: Fog-of-War Foundation (Complete)

### Overview

Phase 1 establishes the visited-tile foundation required for both the mini map
and the full-screen automap. The goal of this phase was to ensure visibility is
recorded correctly in domain state before any new rendering/UI work is layered
on top.

### Problem Statement

The existing movement flow only marked the single destination tile as visited.
That left several gaps relative to the automap plan:

- The immediate area around the party was not revealed during movement.
- The party's starting area could remain unrevealed until the first move.
- Fog-of-war persistence needed explicit round-trip verification through save/load.
- No dedicated tests existed for radius-based visibility behavior.

### Files Changed

| File                                  | Change                                                                                                                                 |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/world/movement.rs`        | Added `VISIBILITY_RADIUS`, introduced `mark_visible_area`, replaced single-tile visit marking in `move_party`, and added phase-1 tests |
| `src/domain/world/mod.rs`             | Re-exported `mark_visible_area` and `VISIBILITY_RADIUS`                                                                                |
| `src/game/systems/map.rs`             | Wired starting-area reveal into `map_change_handler` and added map-load visibility test                                                |
| `src/bin/antares.rs`                  | Marked the starting area visible during initial campaign boot                                                                          |
| `docs/explanation/implementations.md` | Added this implementation summary                                                                                                      |

---

### 1.1 — Visibility Radius in Movement (`src/domain/world/movement.rs`)

Added a module-level constant:

```text
src/domain/world/movement.rs#L1-1
pub const VISIBILITY_RADIUS: u32 = 1;
```

and introduced:

```text
src/domain/world/movement.rs#L1-1
pub fn mark_visible_area(world: &mut World, center: Position, radius: u32)
```

The helper iterates the Chebyshev square around `center` and marks each
in-bounds tile as visited. Out-of-bounds coordinates are ignored rather than
causing panics.

`move_party` now reveals the full visible area after a successful move instead
of marking only the single destination tile.

### 1.2 — Starting Area Reveal on Map Load (`src/game/systems/map.rs`)

The map transition path now reveals the area around the arrival position inside
`map_change_handler` immediately after `current_map` and `party_position` are
updated. This ensures teleports, portals, and other map transitions expose the
starting neighborhood as soon as the party arrives.

### 1.3 — Starting Area Reveal on Initial Campaign Boot (`src/bin/antares.rs`)

Initial game startup now mirrors map-transition behavior by calling
`mark_visible_area` after the campaign's starting position is applied. This
ensures a brand-new game begins with the intended starting area already marked
visited, instead of waiting for the first movement action.

### 1.4 — Save/Load Verification

Phase 1 confirmed that `Tile.visited` already participates in the existing RON
serialization path. A dedicated regression test now serializes a save containing
visited tiles, deserializes it, and verifies the visited state survives the
round-trip unchanged.

### 1.5 — Test Coverage Added

The following tests were added to satisfy the phase requirements:

- `test_mark_visible_area_marks_radius`
- `test_mark_visible_area_clamps_to_bounds`
- `test_visited_persists_after_save_load`
- `test_starting_tile_marked_on_map_load`

The existing movement test was also strengthened so successful movement now
verifies the destination area is revealed, not just the party position update.

### Deliverables Completed

- [x] `mark_visible_area(world, pos, radius)` helper
- [x] `VISIBILITY_RADIUS` constant
- [x] Starting-area mark wired in `src/game/systems/map.rs`
- [x] Initial campaign starting area marked during boot
- [x] All phase-1 tests implemented

### Outcome

After this phase, exploration visibility behaves as the automap plan requires:

- Movement reveals all tiles within `VISIBILITY_RADIUS` of the party.
- Starting positions on both initial load and map transitions are revealed immediately.
- Visited state persists across save/load.
- The behavior is covered by targeted regression tests for radius, bounds, map load,
  and serialization persistence.

## Phase 2: Top-Right Panel Consolidation and Mini Map Widget (Complete)

### Overview

Phase 2 adds the first visible automap feature to the runtime HUD: a dynamic
mini map rendered into a writable image and displayed above the compass in a
new consolidated top-right panel. This phase also reserves the final layout
slot for the future clock widget so later time-system work can be activated
without another HUD restructuring pass.

### Problem Statement

Before this phase, the HUD had separate top-right widgets and no mini map
rendering path at all. That created several gaps relative to the plan:

- There was no parent panel for stacking top-right HUD widgets vertically.
- The compass existed as a standalone root instead of part of a reusable panel.
- No dynamic `Image` resource existed for rendering an explored-tile mini map.
- No viewport-based rendering system existed for party position, walls, floors,
  or NPC markers.
- No placeholder slot existed for the future clock widget layout.

### Files Changed

| File                                  | Change                                                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/hud.rs`             | Added mini map constants, marker components, image resource, startup initialization, render system, and tests |
| `docs/explanation/implementations.md` | Added this implementation summary                                                                             |

---

### 2.1 — Consolidated Top-Right HUD Panel (`src/game/systems/hud.rs`)

Added the new marker components required by the plan:

- `TopRightPanel`
- `MiniMapRoot`
- `MiniMapCanvas`

The HUD setup path now spawns a single absolute-positioned top-right column
container. Inside that panel, the widget order is:

1. mini map
2. compass
3. clock placeholder

This replaces the previous standalone top-right compass/clock anchoring model
with a single layout container that can grow in later phases.

### 2.2 — Mini Map Constants and Dynamic Image Resource

Added the phase-defined constants:

- `MINI_MAP_SIZE_PX`
- `MINI_MAP_VIEWPORT_RADIUS`
- `MINI_MAP_TILE_PX`
- `MINI_MAP_BG_COLOR`
- `MINI_MAP_VISITED_FLOOR`
- `MINI_MAP_WALL`
- `MINI_MAP_PLAYER`
- `MINI_MAP_UNVISITED`
- `MINI_MAP_NPC_COLOR`

Also added the `MiniMapImage` resource, which stores the writable `Handle<Image>`
used by the HUD mini map canvas.

At startup, `initialize_mini_map_image` creates an `RGBA8` image asset sized to
`MINI_MAP_SIZE_PX × MINI_MAP_SIZE_PX`, initializes it transparent, stores it in
`Assets<Image>`, and inserts the `MiniMapImage` resource for later updates.

### 2.3 — `update_mini_map` Rendering System

Added `update_mini_map` and registered it in `HudPlugin` under the existing
`not_in_combat` exploration-only guard.

The system:

1. reads the current map and party position from `GlobalState`
2. computes the square viewport centered on the party
3. rewrites the mini map image every frame
4. renders transparent pixels for out-of-bounds and unvisited tiles
5. renders visited blocked tiles with `MINI_MAP_WALL`
6. renders visited walkable tiles with `MINI_MAP_VISITED_FLOOR`
7. renders the player tile with `MINI_MAP_PLAYER`
8. overlays discovered NPCs as 2×2 dots using `MINI_MAP_NPC_COLOR`

To support this, the implementation also added helper functions for image size,
viewport diameter, tile pixel scaling, pixel addressing, tile fills, and NPC
dot fills.

### 2.4 — Compass Reparenting and Clock Placeholder

The compass is now spawned as a child of `TopRightPanel`, preserving the existing
`CompassRoot` marker and `update_compass` behavior while moving it into the new
column layout.

The clock slot is now also spawned under the same panel as `ClockRoot`, but with
`display: Display::None` so the reserved layout position exists without changing
runtime presentation yet. This satisfies the phase requirement to reserve the
slot for upcoming time-system work.

### 2.5 — Test Coverage Added

Added the required mini map tests:

- `test_mini_map_image_dimensions`
- `test_mini_map_player_pixel_is_white`
- `test_mini_map_unvisited_is_transparent`
- `test_mini_map_visited_wall_color`

The existing clock startup test also continued to validate that the clock root
still exists after the panel refactor.

### Deliverables Completed

- [x] `TopRightPanel`, `MiniMapRoot`, `MiniMapCanvas` marker components
- [x] `MiniMapImage` resource and startup initialization
- [x] `update_mini_map` system registered in `HudPlugin` with exploration-only gating
- [x] `CompassRoot` reparented inside `TopRightPanel`
- [x] `ClockRoot` placeholder slot reserved inside `TopRightPanel`
- [x] All phase-2 tests implemented

### Outcome

After this phase, the top-right HUD layout matches the automap plan foundation:

- The mini map appears above the compass in a consolidated panel.
- The image scrolls with party movement because rendering is centered on the player.
- Explored floors and walls render distinctly.
- Unvisited tiles remain transparent for fog-of-war behavior.
- The player renders as a white marker.
- Discovered NPCs render as green mini map dots.
- The future clock slot is already reserved in the final panel structure.

## Phase 3: Full-Screen Automap Overlay (Complete)

### Overview

Phase 3 adds a full-screen automap overlay that opens from exploration with the
automap key and renders the entire current map with fog-of-war and terrain-aware
color coding. This phase builds directly on the visited-tile foundation from
Phase 1 and the dynamic image rendering path established in Phase 2.

### Problem Statement

After Phase 2, the project had a functioning mini map but still lacked the
larger full-map exploration view described in the implementation plan. Several
pieces were still missing:

- `GameMode` had no dedicated `Automap` variant.
- Input handling had no automap action or toggle behavior.
- `ControlsConfig` had no configurable automap key list.
- The HUD had no full-screen automap overlay UI.
- No rendering path existed for color-coding the entire map by visited state,
  terrain, and wall type.

### Files Changed

| File                                  | Change                                                                                               |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `src/application/mod.rs`              | Added `GameMode::Automap`                                                                            |
| `src/game/systems/input.rs`           | Added `GameAction::Automap`, controls parsing, automap toggle behavior, and input integration tests  |
| `src/sdk/game_config.rs`              | Added `controls.automap`, default key handling, validation updates, serde coverage, and config tests |
| `src/game/systems/hud.rs`             | Added automap overlay components, dynamic image resource, setup/visibility/render systems, and tests |
| `docs/explanation/implementations.md` | Added this implementation summary                                                                    |

---

### 3.1 — `GameMode::Automap` and Input Toggle Flow

Added the new application-layer mode:

- `GameMode::Automap`

Input handling now supports:

- opening automap from `GameMode::Exploration` via the automap action
- closing automap via the automap action again
- closing automap via the menu action (`Escape`) without opening the normal menu

This preserves the expected full-screen overlay behavior from the plan:

- `M` from Exploration → Automap
- `M` from Automap → Exploration
- `Escape` from Automap → Exploration

### 3.2 — `ControlsConfig` Automap Binding

Added a new controls field in `ControlsConfig`:

- `automap: Vec<String>`

with serde default support and the default binding:

- `["M"]`

The controls pipeline now parses `automap` bindings into `KeyMap`, and config
validation now rejects an empty automap key list just like inventory and rest
bindings.

Additional config coverage was added for:

- default automap key presence
- validation of empty/non-empty automap lists
- RON round-trip persistence
- serde defaulting when the field is omitted

### 3.3 — Automap Overlay UI and Dynamic Image

Added full-screen automap overlay infrastructure in `src/game/systems/hud.rs`:

- `AutomapRoot`
- `AutomapCanvas`
- `AutomapLegend`
- `AutomapImage`

Added the required systems:

- `setup_automap`
- `update_automap_visibility`
- `update_automap_image`

The overlay is spawned at startup as a full-screen hidden UI layer with:

- centered map canvas
- right-side legend column
- bottom-left hint text: `"M / Esc — close map"`

Visibility is driven entirely by `GameMode::Automap`.

### 3.4 — Automap Color Coding

Implemented full-map rendering with fog-of-war and terrain/wall coloring:

- Unvisited → black
- Visited floor / generic ground → gray
- Visited wall / torch wall → dark red-gray
- Visited door → tan
- Visited water → blue
- Visited grass / forest → dark green
- Player tile → white

The rendering pass scales the image by map size using the planned approach:
pixels-per-tile is derived from map dimensions and clamped between 4 and 16.

### 3.5 — Test Coverage Added

Added the required phase tests:

- `test_gamemode_automap_toggle`
- `test_gamemode_automap_escape_closes`
- `test_automap_image_unvisited_is_black`
- `test_automap_image_visited_floor_is_gray`
- `test_controls_config_default_automap_key`

Also added supporting config tests to cover automap serialization/defaulting and
validation behavior.

### Deliverables Completed

- [x] `GameMode::Automap` variant
- [x] `GameAction::Automap` + key parsing in `KeyMap`
- [x] `automap: Vec<String>` in `ControlsConfig` with `serde(default)`
- [x] Automap overlay setup, visibility toggle, and image update systems
- [x] Full fog-of-war automap rendering with terrain color coding
- [x] M / Escape toggle wired in input handling
- [x] All phase-3 tests implemented

### Outcome

After this phase, the game supports a full-screen automap workflow that matches
the implementation plan:

- Pressing `M` from exploration opens the automap.
- Pressing `M` again closes it.
- Pressing `Escape` while automap is open closes it back to exploration.
- Unvisited tiles render as black fog.
- Visited tiles render with terrain/wall-aware colors.
- The party position is clearly visible as a white marker.

## Phase 4: POI Markers and Legend (Complete)

### Overview

Phase 4 adds semantic points-of-interest to the mini map and automap so the
player can distinguish meaningful discovered locations from basic explored
terrain. This phase also upgrades the automap side panel into a real legend
that explains the POI symbol colors directly in the overlay.

### Problem Statement

After Phase 3, both map views could render explored terrain and the player
position, but they still lacked semantic world markers. The remaining gaps were:

- no `PointOfInterest` representation in the world layer
- no helper for collecting discovered POIs from map content
- no dedicated POI color palette shared by mini map and automap
- no POI overlay rendering on either map surface
- no legend entries explaining POI symbols on the automap overlay

### Files Changed

| File                                  | Change                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------- |
| `src/domain/world/types.rs`           | Added `PointOfInterest`, `Map::collect_map_pois`, and POI tests        |
| `src/domain/world/mod.rs`             | Re-exported `PointOfInterest`                                          |
| `src/game/systems/hud.rs`             | Added POI colors, legend entries, mini map / automap POI overlay logic |
| `docs/explanation/implementations.md` | Added this implementation summary                                      |

---

### 4.1 — `PointOfInterest` and Collection Helper

Added `PointOfInterest` to the world layer with the planned semantic variants:

- `QuestObjective { quest_id }`
- `Merchant`
- `Sign`
- `Teleport`
- `Encounter`
- `Treasure`

Also added `Map::collect_map_pois(...)`, which returns discovered POIs only for
visited tiles. The helper currently collects POIs from:

- merchant-like NPC placements
- `MapEvent::Encounter`
- `MapEvent::Treasure`
- `MapEvent::Teleport`
- `MapEvent::Sign`

This keeps POI filtering in the world/domain layer instead of duplicating it in
HUD rendering code.

### 4.2 — POI Colors

Added the phase POI color constants to `src/game/systems/hud.rs`:

- `POI_QUEST_COLOR`
- `POI_MERCHANT_COLOR`
- `POI_SIGN_COLOR`
- `POI_TELEPORT_COLOR`
- `POI_ENCOUNTER_COLOR`
- `POI_TREASURE_COLOR`

Also added a shared `poi_color(...)` helper so mini map and automap render from
the same source of truth.

### 4.3 — Mini Map and Automap POI Overlay

After base terrain rendering, both map systems now overlay POI dots:

- mini map uses `fill_mini_map_poi_dot(...)` with 2×2 markers
- automap uses `fill_automap_poi_dot(...)` with 3×3 markers

The mini map only renders POIs that fall within the current player-centered
viewport. The automap renders POIs across the whole full-map canvas. Both obey
the discovered/visited-tile rule through `collect_map_pois(...)`.

### 4.4 — Automap Legend Panel

The `AutomapLegend` content is now populated with one static row per POI type
plus the player marker. Each row contains:

- a `20×20` colored square
- a text label

Legend entries now include:

- White — You are here
- Yellow — Quest objective
- Green — Merchant
- Light blue — Sign / notice
- Purple — Teleport
- Red — Monster encounter
- Gold — Treasure

The previously-existing terrain explanation lines remain below these symbol rows.

### 4.5 — Test Coverage Added

Added the required phase tests:

- `test_collect_map_pois_only_visited`
- `test_collect_map_pois_encounter`
- `test_collect_map_pois_treasure`
- `test_mini_map_poi_dot_rendered`

These verify that:

- unvisited POIs are suppressed
- encounter and treasure events map to the correct POI types
- visited merchant POIs render with the expected mini map color

### Deliverables Completed

- [x] `PointOfInterest` enum + POI collection helper
- [x] POI color constants in `src/game/systems/hud.rs`
- [x] POI overlay integrated into `update_mini_map`
- [x] POI overlay integrated into `update_automap_image`
- [x] Legend panel expanded with static POI entries
- [x] All phase-4 tests implemented

### Outcome

After this phase, both map views now show discovered semantic markers instead of
only terrain:

- merchants appear as green markers
- signs appear as light-blue markers
- teleports appear as purple markers
- encounters appear as red markers
- treasure appears as gold markers
- the automap legend explains every symbol directly in the overlay

This completes the first POI visualization pass and establishes the structure
for richer quest-objective integration in later phases.
