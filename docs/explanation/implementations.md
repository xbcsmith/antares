# Implementations

## Phase 3: Dialogue Mouse Support (Complete)

### Overview

Phase 3 extends the shared mouse activation model into the dialogue system so
players can advance dialogue and select branching choices entirely with the
mouse. The goal of this phase was to bring dialogue interaction into parity
with the existing keyboard paths while preserving the same dialogue semantics
and state transitions.

### Problem Statement

Before this phase, dialogue interaction still depended on keyboard input for
two core actions:

- advancing dialogue text required `Space` or `E`
- selecting dialogue choices required arrow keys, digit keys, `Enter`, or `Space`

That left the dialogue UI visually present on screen but not fully operable by
mouse, which conflicted with the game-wide mouse support plan.

### Files Changed

| File                                   | Change                                                          |
| -------------------------------------- | --------------------------------------------------------------- |
| `src/game/systems/dialogue.rs`         | Added mouse-driven advance support for the dialogue panel       |
| `src/game/systems/dialogue_visuals.rs` | Wired the dialogue panel root for Bevy-UI click detection       |
| `src/game/systems/dialogue_choices.rs` | Added clickable choice buttons and mouse-driven choice dispatch |
| `docs/explanation/implementations.md`  | Added this implementation summary                               |

---

### 3.1 — Dialogue Advance on Click

The dialogue panel uses Bevy UI, so the implementation follows the Bevy-UI path
from the plan rather than switching to egui.

The dialogue text panel root was upgraded to participate in interaction by
adding `Button` and `Interaction::None` to the panel entity created by
`spawn_dialogue_bubble`. The dialogue input system now reads optional mouse
input and the panel interaction state, then uses the shared
`mouse_input::is_activated(...)` helper to emit `AdvanceDialogue` when the
player clicks the dialogue panel.

This preserves the existing keyboard behavior while adding a parallel mouse
route with identical semantics.

### 3.2 — Clickable Dialogue Choices

Dialogue choice rows were upgraded from passive layout/text nodes into
interactive Bevy UI buttons.

Each spawned choice row now carries:

- `Button`
- `Interaction::None`
- `ChoiceButton { choice_index }`

The choice input system now has a second mouse query alongside the existing
keyboard path. When a choice button is activated through the shared mouse
helper, it immediately emits `SelectDialogueChoice { choice_index }` and resets
`ChoiceSelectionState` just like the existing digit-key immediate confirm path.

Hovering alone remains non-destructive and does not dispatch a choice.

### 3.3 — Interaction Model

The dialogue mouse interaction model is intentionally aligned with earlier
phases:

- clicking the dialogue panel advances the conversation exactly like pressing
  `Space`
- clicking a choice selects it immediately exactly like pressing its digit key
- hovered state by itself never causes dialogue progression or choice
  selection

This keeps the canonical activation rule consistent across combat, menus, and
dialogue.

### 3.4 — Test Coverage Added

Added the required Phase 3 mouse interaction tests:

- `test_mouse_click_advances_dialogue`
- `test_mouse_click_choice_dispatches_select`
- `test_mouse_hover_choice_does_not_select`
- `test_mouse_click_choice_resets_choice_state`

These tests verify that:

- dialogue panel clicks emit `AdvanceDialogue`
- clicking a choice dispatches the correct `SelectDialogueChoice`
- hovered choice state alone does not emit a selection
- click selection resets `ChoiceSelectionState` to its idle defaults

### 3.5 — Deliverables Completed

- [x] Dialogue text panel wired for advance-on-click using the Bevy-UI path
- [x] `ChoiceButton` marker component added to the dialogue choice system
- [x] Choice nodes spawned with `Button`, `Interaction::None`, and choice marker data
- [x] `choice_input_system` extended with mouse activation handling
- [x] Phase 3 tests implemented
- [x] Quality gates run

### 3.6 — Outcome

After this phase, the dialogue system is fully operable by mouse:

- clicking the dialogue panel advances dialogue
- clicking a choice immediately selects it
- hovering alone never triggers any action

This completes the dialogue portion of the game-wide mouse input plan and keeps
the shared Bevy-UI activation model consistent across all implemented phases.

## Phase 2: Menu Mouse Support (Complete)

### Overview

Phase 2 extends the shared mouse activation model into the menu system so both
menu buttons and settings sliders are fully operable by mouse. The goal of this
phase was to remove the remaining keyboard-only gaps in the in-game menu,
bringing button activation and audio setting adjustment into parity with the
planned game-wide mouse input model.

### Problem Statement

Before this phase, the menu system still had two major mouse-input gaps:

- `menu_button_interaction` only reacted to direct `Interaction::Pressed`
  transitions and did not use the shared hovered-click fallback introduced in
  Phase 1.
- Settings sliders were effectively keyboard-only because their values were
  changed through keyboard navigation logic rather than direct mouse click/drag
  interaction on the slider widgets.

That meant the menu appeared mouse-driven visually, but some interactions were
either fragile or incomplete from an actual usability perspective.

### Files Changed

| File                                  | Change                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------- |
| `src/game/systems/menu.rs`            | Added hovered-click menu button activation and mouse-driven slider logic  |
| `src/game/components/menu.rs`         | Added slider-track marker data needed to identify settings slider widgets |
| `docs/explanation/implementations.md` | Added this implementation summary                                         |

---

### 2.1 — Shared Activation in `menu_button_interaction`

Updated `menu_button_interaction` to use the shared Phase 1 mouse helpers
instead of checking only for `Interaction::Pressed`.

The system now reads:

- `Ref<Interaction>` so it can detect whether the interaction changed this frame
- optional mouse button input so the left-click state is computed once per frame

This means a menu button activates when either:

- it enters `Interaction::Pressed` this frame, or
- the left mouse button is just pressed while the button is hovered

This matches the same canonical Bevy UI activation rule already established for
combat, so menu buttons and combat buttons now behave consistently.

### 2.2 — Mouse-Driven Settings Sliders

Implemented direct mouse interaction for settings sliders by introducing slider
track marker data and a dedicated slider mouse handler in the menu systems.

The chosen implementation remains in Bevy UI rather than switching the settings
screen to egui. This keeps the menu architecture consistent with the existing
menu panel and button hierarchy while adding the missing interaction behavior.

The slider track path now supports:

- click-to-set based on cursor position along the track width
- drag-to-adjust while the left mouse button remains held
- per-slider routing back into the matching audio config field

This gives the settings menu the expected slider behavior without requiring any
keyboard input.

### 2.3 — Slider Interaction Model

The slider mouse system computes a normalized horizontal cursor position within
the slider track bounds and maps that to the slider's `0.0..=1.0` value range.

Behavior is intentionally simple and deterministic:

- click near the left edge sets a low value
- click near the center sets an approximately 50% value
- click near the right edge sets a high value
- dragging continuously updates the value as the cursor moves

Hover alone never mutates slider state. The slider only changes when activation
or drag conditions are satisfied.

### 2.4 — Test Coverage Added

Added the required coverage for both menu buttons and sliders:

**Menu button tests**

- `test_mouse_click_resume_button`
- `test_mouse_hovered_click_save_button`
- `test_mouse_hover_does_not_dispatch_menu`

**Slider tests**

- `test_slider_mouse_click_sets_value`
- `test_slider_drag_updates_value`

These tests verify that:

- mouse button activation matches the expected menu action semantics
- hovered state alone is non-destructive
- slider values update from pointer position
- dragging changes values continuously rather than only on initial click

### 2.5 — Deliverables Completed

- [x] `menu_button_interaction` updated with hovered-click fallback using Phase 1 helpers
- [x] Slider track marker component added and slider widgets upgraded for click/drag
- [x] `handle_slider_mouse` registered in `MenuPlugin`
- [x] Tests from Phase 2 implemented
- [x] Quality gates run

### 2.6 — Outcome

After this phase, the menu system supports mouse interaction across both
navigation buttons and settings sliders:

- menu buttons activate reliably through the shared canonical activation helper
- audio sliders can be adjusted by click and drag
- hover alone never dispatches an action

This completes the menu portion of the game-wide mouse input plan and prepares
the same interaction model for dialogue and inventory phases.

## Phase 1: Shared Mouse Activation Utility (Complete)

### Overview

Phase 1 establishes a single canonical mouse-activation model for Bevy UI
buttons used by runtime game systems. The goal of this phase was to extract the
existing combat-specific dual-path click handling into a shared helper so later
mouse-input phases can reuse one implementation rather than duplicating
interaction logic.

### Problem Statement

Before this phase, combat embedded the same mouse activation pattern inline in
multiple places. That created several gaps relative to the mouse-input plan:

- `Interaction::Pressed` handling and hovered-click fallback were duplicated.
- The left-mouse `just_pressed` query pattern was repeated at each call site.
- Combat contained multiple copies of logic that should become the canonical
  game-wide Bevy UI activation rule.
- Later menu and dialogue mouse phases would have needed to copy combat logic
  again instead of depending on a shared utility.

### Files Changed

| File                                  | Change                                                                                                  |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/game/systems/mouse_input.rs`     | Added shared `is_activated` / `mouse_just_pressed` helpers, full doc comments, doctests, and unit tests |
| `src/game/systems/mod.rs`             | Registered the new `mouse_input` systems module                                                         |
| `src/game/systems/combat.rs`          | Replaced inline mouse-activation duplication with shared helper calls                                   |
| `docs/explanation/implementations.md` | Added this implementation summary                                                                       |

---

### 1.1 — Shared Mouse Activation Helpers (`src/game/systems/mouse_input.rs`)

Added a new `src/game/systems/mouse_input.rs` module as the single source of
truth for Bevy UI mouse activation semantics.

The module provides two small inline helpers:

- `is_activated(interaction, interaction_ref, mouse_just_pressed)` returns
  `true` when a button was newly pressed this frame or when the left mouse was
  just pressed while the widget is hovered.
- `mouse_just_pressed(mouse_buttons)` wraps the optional Bevy mouse-button
  resource lookup so callers do not repeat the same `Option` plumbing.

Both functions are documented with `///` comments and runnable doctests so the
activation contract is explicit and testable in one place.

### 1.2 — Combat Mechanical Refactor (`src/game/systems/combat.rs`)

Refactored combat to consume the shared helpers instead of open-coding the same
logic.

The following combat paths now use `mouse_input::is_activated` and
`mouse_input::mouse_just_pressed`:

- blocked-input logging when it is not the player's turn
- action-button activation in `combat_input_system`
- enemy-card activation in `select_target`

This was intentionally a mechanical refactor: the existing combat semantics were
preserved so mouse activation still behaves identically for both direct
`Interaction::Pressed` transitions and hovered left-click fallback.

### 1.3 — Test Coverage Added

Added the required unit tests for the shared helper behavior:

- `test_is_activated_pressed_changed`
- `test_is_activated_pressed_unchanged`
- `test_is_activated_hovered_with_mouse_press`
- `test_is_activated_hovered_without_mouse_press`
- `test_is_activated_none`
- `test_mouse_just_pressed_none_resource`

Existing combat mouse tests continue to validate that the refactor preserved the
runtime behavior expected by combat action buttons and target selection.

### 1.4 — Deliverables Completed

- [x] `src/game/systems/mouse_input.rs` created with SPDX header,
      `is_activated`, `mouse_just_pressed`, and unit tests
- [x] `src/game/systems/mod.rs` declares the new `mouse_input` module
- [x] `combat_input_system` and `select_target` refactored to use helpers
- [x] Combat mouse behavior preserved through the existing combat tests

### 1.5 — Outcome

After this phase, Antares has a reusable mouse activation utility that defines
the canonical Bevy UI click model for future mouse-input work.

This reduces duplication, keeps combat behavior unchanged, and gives later
phases (menu buttons, dialogue choices, and other Bevy UI interactions) a
single helper to call instead of re-implementing pressed-versus-hovered-click
logic independently.

## Phase 1: Fog-of-War Foundation (Complete)

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

## Phase 5: Config, Save/Load Verification, and SDK Integration (Complete)

### Overview

Phase 5 finalizes the automap/mini-map feature set by wiring the remaining
configuration, persistence, and SDK editor pieces together. The focus of this
phase was to make the feature campaign-configurable, preserve discovered map
state through save/load, and expose the new settings in the Campaign Builder.

### Problem Statement

After Phase 4, the runtime map features existed, but the surrounding project
integration was still incomplete:

- `GraphicsConfig` had no `show_minimap` toggle.
- Campaign config templates and shipped campaign configs did not document or
  provide the new automap / mini-map settings.
- Save/load round-trip verification for discovered automap state still needed a
  dedicated regression test.
- The Campaign Builder config editor did not expose mini-map visibility or the
  automap key binding.

### Files Changed

| File                                        | Change                                                                                      |
| ------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `src/sdk/game_config.rs`                    | Added `show_minimap`, serde defaults, config loading coverage, and validation/default tests |
| `src/game/systems/hud.rs`                   | Honored `graphics.show_minimap` and added visibility test                                   |
| `campaigns/config.template.ron`             | Documented `show_minimap` and `automap` settings                                            |
| `data/test_campaign/config.ron`             | Added `show_minimap: true` and `automap: ["M"]`                                             |
| `campaigns/tutorial/config.ron`             | Added `show_minimap: true` and `automap: ["M"]`                                             |
| `sdk/campaign_builder/src/config_editor.rs` | Added Show Mini Map checkbox and Automap key binding editor support                         |
| `tests/campaign_integration_test.rs`        | Added save/load regression test for discovered automap state                                |
| `docs/explanation/implementations.md`       | Added this implementation summary                                                           |

---

### 5.1 — `GraphicsConfig.show_minimap` and Config Defaults

Added the new graphics toggle:

- `show_minimap: bool`

with serde default behavior and a default value of `true`.

This means older config files that omit the field continue to load cleanly while
new campaigns can explicitly disable the exploration mini map.

`ControlsConfig.automap` from Phase 3 was also verified as part of this phase,
including default behavior and config round-trip coverage.

### 5.2 — Runtime Mini Map Visibility Control

`update_mini_map` now checks `global_state.0.config.graphics.show_minimap`.

Behavior is now:

- if `show_minimap == true`, `MiniMapRoot` remains visible and the mini map is rendered
- if `show_minimap == false`, `MiniMapRoot` is set to `Display::None` and the system exits early

This keeps the feature purely config-driven at runtime without affecting automap.

### 5.3 — Campaign Config Template and Config Files

Updated:

- `campaigns/config.template.ron`
- `data/test_campaign/config.ron`
- `campaigns/tutorial/config.ron`

to include:

- `graphics.show_minimap: true`
- `controls.automap: ["M"]`

The template comments now document both settings so campaign authors can
discover and customize them easily.

### 5.4 — Save/Load Verification

Added `test_automap_state_round_trips_save` in
`tests/campaign_integration_test.rs`.

This test:

1. creates a map
2. marks a tile as visited
3. saves the game
4. loads the game
5. verifies the visited tile is still marked visited

That provides explicit regression coverage for discovered automap/fog-of-war
state persistence through the standard save pipeline.

### 5.5 — SDK Config Editor Integration

### 5.6 — HUD Regression Fixes for Mini Map, Automap, and Clock

A follow-up HUD regression fix corrected several runtime presentation problems in
`src/game/systems/hud.rs`:

- the clock widget existed in the HUD tree but was spawned with `display: Display::None`,
  so neither the time nor date was visible at runtime
- the mini map and full automap could appear blank at runtime even though their
  backing dynamic images were being updated
- the party marker behavior could appear incorrect because the player indicator
  rendering and the HUD image binding path were not both being refreshed reliably
- NPC and POI overlays needed to stay tied to discovered tiles so newly explored
  merchants and other notable map features appear only after exploration reveals them

The fix now:

- makes `ClockRoot` visible by default so the datetime renders beneath the compass
- preserves discovered terrain colors on both map views, then overlays the party marker
  afterward
- renders directional player markers for both the mini map and automap so the
  indicator remains centered within the current tile while still showing facing
- rebinds the HUD mini map and automap canvas nodes to their dynamic image handles
  during update, ensuring the UI keeps displaying the current writable map textures
- keeps POI overlays gated to visited tiles so merchants and other discovered map
  features only appear once their tiles have actually been explored
- adds debug logging around map painting so future regressions can distinguish
  between fog-of-war state problems and UI image binding problems quickly

Additional regression coverage was added in `src/game/systems/hud.rs` for:

- visible clock root startup behavior
- directional mini map player marker rendering
- directional automap player marker rendering

### 5.5 — SDK Config Editor Integration

Updated `sdk/campaign_builder/src/config_editor.rs` to expose the new settings.

#### Added editor state

- `controls_automap_buffer: String`

#### Added graphics UI

- **Show Mini Map** checkbox bound to `game_config.graphics.show_minimap`

#### Added controls UI

- **Automap** key binding field using the same capture / clear / validate
  workflow as the existing inventory and rest bindings

#### Updated editor plumbing

- `update_edit_buffers`
- `update_config_from_buffers`
- key capture routing
- validation logic
- test coverage for automap buffer and validation behavior

### 5.6 — Test Coverage Added

Added and verified the phase-required tests:

- `test_controls_config_automap_defaults_when_missing_from_ron`
- `test_graphics_config_serde_show_minimap_default`
- `test_mini_map_hidden_when_show_minimap_false`
- `test_automap_state_round_trips_save`

Also added supporting Campaign Builder tests for the new automap editor field.

### Deliverables Completed

- [x] `show_minimap: bool` in `GraphicsConfig` with serde default
- [x] `automap` key confirmed in `ControlsConfig`
- [x] `campaigns/config.template.ron` updated
- [x] `data/test_campaign/config.ron` updated
- [x] `campaigns/tutorial/config.ron` updated
- [x] SDK Config Editor: mini map toggle + automap key binding field
- [x] All phase-5 tests implemented

### Outcome

After this phase, the automap/mini-map feature set is fully integrated:

- the automap key is configurable per campaign
- the mini map can be disabled through config
- older config files continue to load safely with sensible defaults
- discovered map state survives save/load
- the Campaign Builder exposes both new settings for authors

This completes the planned automap and mini-map implementation sequence.
