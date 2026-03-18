# Doors as Furniture Implementation Plan

## Overview

Doors are currently rendered as plain cuboid boxes via `WallType::Door` — flat brown blocks with no geometry detail and no wood textures. This plan converts doors into procedural mesh furniture objects, giving them proper 3D geometry (frame, planks, studs, hinges) and leveraging the existing `FurnitureMaterial` PBR system for wood/metal/stone appearances. The door panel becomes a `FurnitureType::Door` while the surrounding frame uses the existing `StructureType::DoorFrame` and `DoorFrameConfig`. Door interaction logic (open/close/locked) is preserved but migrated from `WallType::Door` tile mutation to `MapEvent::Furniture` with `FurnitureFlags`.

## Current State Analysis

### Existing Infrastructure

- **`WallType::Door`** in [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L28-L29) — an enum variant on `Tile.wall_type`
- **Door rendering** in [map.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs#L970-L1012) — spawns a cuboid with `door_rgb = (0.4, 0.2, 0.1)`, no textures, no detail geometry
- **Door interaction** in [input.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/input.rs#L612-L622) — detect `WallType::Door` on tile ahead, mutate `wall_type` to `None`, fire `DoorOpenedEvent`
- **`DoorOpenedEvent`** in [map.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs#L113) — message that triggers map visual refresh
- **Movement system** in [movement.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/movement.rs#L32-L34) — `MovementError::DoorLocked` variant exists but is not yet used
- **Furniture system** — full pipeline: `FurnitureType` enum → `spawn_furniture()` dispatch → individual `spawn_*()` functions → `furniture_rendering.rs` adds `FurnitureEntity` + `Interactable` components
- **`DoorFrameConfig`** in [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1547-L1566) — door frame structure config (width: 1.0, height: 2.5, frame_thickness: 0.15) already defined but not wired to a spawn function
- **`FurnitureFlags`** in [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1185-L1218) — has `locked`, `blocking`, `lit` booleans
- **`InteractionType`** in [furniture.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/furniture.rs#L137-L146) — `OpenChest`, `SitOnChair`, `LightTorch`, `ReadBookshelf`
- **RON map data** — ~37 tiles across 10+ map files use `wall_type: Door`

### Identified Issues

1. **No 3D door geometry** — doors render as flat half-tile cuboid blocks identical to walls, just brown
2. **No texture/PBR** — no wood grain, no metallic bands, no visual depth
3. **Door state is tile mutation** — opening a door mutates `tile.wall_type` from `Door` to `None`, permanently destroying information; no way to re-close
4. **No door frame** — `DoorFrameConfig` and `StructureType::DoorFrame` exist in the domain but have no spawn function or rendering code
5. **No locked-door gameplay** — `MovementError::DoorLocked` exists but is never returned; `FurnitureFlags.locked` exists but is not checked for doors
6. **Doors not in furniture pipeline** — cannot benefit from material system, interaction system, or editor support

## Implementation Phases

### Phase 1: Domain Types and Door Procedural Mesh

Add `Door` to the furniture type system and build procedural mesh geometry for a proper 3D door.

#### 1.1 Foundation Work

- Add `FurnitureType::Door` variant to the [FurnitureType](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1236-L1253) enum
- Update `FurnitureType::all()`, `name()`, `icon()` (🚪), `category()` (new `FurnitureCategory::Passage`) and `default_presets()` methods
- Add `FurnitureCategory::Passage` variant to [FurnitureCategory](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1375-L1411) enum with `name()` and `all()` updates
- Add `InteractionType::OpenDoor` variant to [InteractionType](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/furniture.rs#L137-L146) with `name()` returning `"Open Door"`

#### 1.2 Procedural Mesh: `DoorConfig` and `spawn_door()`

Create a `DoorConfig` struct and `spawn_door()` function in [procedural_meshes.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs):

- **`DoorConfig`** fields: `width: f32` (default 0.9), `height: f32` (default 2.3), `thickness: f32` (default 0.08), `plank_count: u8` (default 5), `has_studs: bool` (default true), `has_hinges: bool` (default true), `color_override: Option<Color>`
- **`spawn_door()`** geometry — parent entity with children:
  - **Door panel**: vertical rectangle (`width × height × thickness`) — the main door face made of `plank_count` visible plank strips for wood texture effect
  - **Cross braces**: 2 thin horizontal cuboids at 1/3 and 2/3 height, slightly proud of the panel surface for depth
  - **Iron studs** (optional): small sphere or cylinder primitives arranged in a grid pattern on the face for studded appearance
  - **Hinges** (optional): 2 thin cuboids at left edge, top-third and bottom-third
  - **Handle**: small cylinder on the right side at ~mid-height
- Apply `FurnitureMaterial` PBR properties (metallic, roughness) based on material type
- Support rotation via `rotation_y` parameter
- Register cache fields in `ProceduralMeshCache` for door panel, studs, hinges

#### 1.3 Integrate into Furniture Pipeline

- Add `FurnitureType::Door` match arm to [spawn_furniture()](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs#L3666-L3778) dispatch function
- Add `FurnitureType::Door` match arm to [spawn_furniture_with_rendering()](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/furniture_rendering.rs#L52-L211) for material application and component insertion
- Wire `InteractionType::OpenDoor` in `get_interaction_type()` and `get_interaction_distance()` (distance: 1.5)

#### 1.4 Testing Requirements

- Unit test: `FurnitureType::Door` is in `all()`, `name()` returns `"Door"`, `icon()` returns `"🚪"`, `category()` returns `Passage`
- Unit test: `DoorConfig::default()` has expected values
- Unit test: `InteractionType::OpenDoor.name()` returns `"Open Door"`
- Unit test: `FurnitureCategory::Passage` is in `all()` and `name()` returns `"Passage"`
- Unit test: `spawn_furniture()` dispatch handles `FurnitureType::Door` without panic (Bevy headless App)
- Run existing furniture editor tests in `sdk/campaign_builder/tests/furniture_editor_tests.rs` to verify no regressions

#### 1.5 Deliverables

- [ ] `FurnitureType::Door` variant with all trait methods
- [ ] `FurnitureCategory::Passage` variant
- [ ] `InteractionType::OpenDoor` variant
- [ ] `DoorConfig` struct with `Default` implementation
- [ ] `spawn_door()` procedural mesh function
- [ ] `spawn_furniture()` dispatch integration
- [ ] `spawn_furniture_with_rendering()` integration
- [ ] `ProceduralMeshCache` fields for door components
- [ ] All tests passing

#### 1.6 Success Criteria

- Spawning a `MapEvent::Furniture { furniture_type: Door, material: Wood, .. }` renders a detailed 3D door with planks, braces, studs, and handle instead of a flat brown cuboid
- Door respects `FurnitureMaterial` for Wood/Stone/Metal/Gold PBR properties
- All quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo nextest run`

---

### Phase 2: Door Frame Procedural Mesh

Build the companion door frame using the existing `DoorFrameConfig` and `StructureType::DoorFrame`.

#### 2.1 Feature Work

Create `spawn_door_frame()` function in [procedural_meshes.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs):

- Uses existing `DoorFrameConfig` (width: 1.0, height: 2.5, frame_thickness: 0.15)
- **Geometry**: two vertical posts (left/right) + one horizontal lintel (top) — forming an inverted U
- **Material**: stone color (`STRUCTURE_STONE_COLOR`) to match other architectural structures
- **Cache**: store frame post and lintel meshes in `ProceduralMeshCache.structure_door_frame`

#### 2.2 Composite Spawn: Door + Frame

- Create `spawn_door_with_frame()` helper that spawns both the door panel (`spawn_door()`) and the frame (`spawn_door_frame()`) as sibling entities centered on the same tile
- The frame should be slightly larger than the door opening, framing it visually
- Wire this composite into the map rendering path so doors placed via `MapEvent::Furniture` get both frame and panel

#### 2.3 Configuration Updates

- No config file changes — `DoorFrameConfig` already exists with sensible defaults
- Ensure `StructureType::DoorFrame` constants (`DOOR_FRAME_THICKNESS`, `DOOR_FRAME_BORDER`) are used from existing definitions

#### 2.4 Testing Requirements

- Unit test: `spawn_door_frame()` produces entities with correct component set (Bevy headless App)
- Unit test: `DoorFrameConfig::default()` values match architecture expectations
- Verify existing structure type tests still pass

#### 2.5 Deliverables

- [ ] `spawn_door_frame()` procedural mesh function
- [ ] `spawn_door_with_frame()` composite helper
- [ ] `ProceduralMeshCache` wiring for door frame meshes
- [ ] Tests passing

#### 2.6 Success Criteria

- Door furniture events render with a visible stone/wood frame surrounding the door panel
- Frame geometry is distinct from the door panel (different material, slightly larger)

---

### Phase 3: Door Interaction and State Management

Replace the current tile mutation approach with proper furniture-based door interaction.

#### 3.1 Feature Work: Door Open/Close System

- **New `DoorState` component** in `src/game/components/furniture.rs`:
  - Fields: `is_open: bool`, `is_locked: bool`, `key_item_id: Option<ItemId>`
  - Attached to door entities during `spawn_furniture_with_rendering()` when `FurnitureType::Door`
- **Door visual update system** — when `DoorState.is_open` changes:
  - Rotate door panel entity 90° around its hinge edge (left side pivot) via `Transform` mutation
  - Update `FurnitureEntity.blocking` to `false` when open, `true` when closed
- Modify `DoorOpenedEvent` to carry the door `Entity` rather than just `Position`, enabling targeted visual updates

#### 3.2 Integrate with Input System

- In [input.rs handle_input()](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/input.rs#L612-L622), add a new code path for furniture-based doors:
  - Query for `FurnitureEntity` + `DoorState` entities at the target position
  - If found and `!is_locked`: toggle `DoorState.is_open`, fire `DoorOpenedEvent`
  - If found and `is_locked`: check party inventory for matching `key_item_id`, show "Door is locked" message if no key
- Keep the existing `WallType::Door` path as a fallback during the migration period (Phase 4 removes it)

#### 3.3 Blocking Integration

- Tiles with door furniture events should use `Tile.blocked` synced with `DoorState.is_open`:
  - Closed door → tile blocked
  - Open door → tile unblocked
- Wire `MovementError::DoorLocked` to actually fire when attempting to move through a locked door

#### 3.4 Testing Requirements

- Unit test: `DoorState` component creation, default values
- Unit test: door open toggle changes `blocking` state
- Unit test: locked door denies interaction without key
- Integration test: interact key opens a furniture door (Bevy headless App with `handle_input` system)
- Verify existing door interaction tests in `input.rs` still pass for legacy `WallType::Door`

#### 3.5 Deliverables

- [ ] `DoorState` component
- [ ] Door visual rotation on open/close
- [ ] Input system furniture-door interaction path
- [ ] Locked door key check
- [ ] `MovementError::DoorLocked` wired to locked furniture doors
- [ ] Tests passing

#### 3.6 Success Criteria

- Pressing interact on a furniture door toggles it open (rotates 90°) and unblocks the tile
- Locked doors show "Door is locked" and remain closed unless the party has the correct key item
- Legacy `WallType::Door` tiles still work through the old code path

---

### Phase 4: Map Data Migration and Campaign Builder

Migrate existing `WallType::Door` tiles to `MapEvent::Furniture` door events and update the campaign builder editor.

#### 4.1 Feature Work: Migration Script

- Write a one-time Rust binary or Python script in `tools/` that:
  - Scans all `.ron` map files in `data/` and `campaigns/`
  - For each tile with `wall_type: Door`: adds a corresponding `MapEvent::Furniture` entry at that position with `furniture_type: Door`, `material: Wood`, `flags: { blocking: true, .. }`, and appropriate `rotation_y`
  - Changes the tile's `wall_type` from `Door` to `None` and sets `blocked: false` (the furniture event now controls blocking)
  - Preserves all other tile properties
- ~37 door tiles across 10+ map files need migration

#### 4.2 Campaign Builder UI

- In [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs), update the `EventType::Furniture` editor panel:
  - `FurnitureType::Door` should appear in the furniture type dropdown
  - Show door-specific fields: locked checkbox, key item selector
  - Preview icon: 🚪
- Add `FurnitureCategory::Passage` to any category filter UI

#### 4.3 Remove Legacy Door Rendering

- Remove the `WallType::Door` match arm from [map.rs spawn_map()](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs#L970-L1012) — door rendering now comes entirely from furniture events
- Remove or deprecate `WallType::Door` from the enum (or leave as unused for backward compatibility with un-migrated maps)
- Remove the legacy `WallType::Door` interaction path from `input.rs`
- Clean up `door_rgb` color constant from `map.rs`

#### 4.4 Testing Requirements

- Run migration script on `data/test_campaign` maps and verify output parses correctly with `cargo check`
- Verify all existing door interaction tests pass after migration
- Verify campaign builder tests in `sdk/campaign_builder/tests/furniture_editor_tests.rs` pass with new `Door` variant
- Manual testing: load tutorial campaign and verify all doors render as 3D models and can be opened

#### 4.5 Deliverables

- [ ] Migration script (RON door tile → furniture event converter)
- [ ] Campaign builder UI updated for `FurnitureType::Door`
- [ ] Legacy `WallType::Door` rendering removed from `map.rs`
- [ ] Legacy door interaction path removed from `input.rs`
- [ ] All `.ron` map files migrated
- [ ] Tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 4.6 Success Criteria

- No `.ron` map file contains `wall_type: Door` — all doors are `MapEvent::Furniture` events
- Campaign builder can create, edit, and preview door furniture events
- All quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo nextest run`
- Game loads and renders doors as proper 3D models with wood/metal textures
