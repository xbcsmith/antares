# Procedural Meshes Implementation Plan

> **STATUS: PHASES 1-5 COMPLETED** (2025-01-XX)
>
> **Phase 5 Update**: Portal design changed from torus ring to rectangular vertical frame. Sign interaction changed from auto-trigger to E key interaction with dialogue box (matching NPC interaction pattern). Sign board repositioned to eye height (1.5m).
>
> **UPDATE (2025-01-XX)**: This plan has been modified to align with the sprite-based character rendering approach. All "Actor" entity rendering (NPCs, Monsters, Recruitables) has been moved to the Sprite Support implementation plan. This plan now focuses exclusively on **Environmental Objects** (Trees) and **Static Interactive Props** (Signs, Portals).
>
> **Key Changes**:
>
> - **Phase 2 (NPC Procedural Representation)**: REMOVED - NPCs now use sprite billboards
> - **Phase 3 (Event Markers)**: MODIFIED - Removed recruitable character markers (now sprites)
> - **Phase 4 (Performance)**: MODIFIED - Removed NPC and recruitable mesh cache entries
> - **Phase 5 (Portal Redesign & Sign Interaction)**: COMPLETED - Portals now rectangular frames, signs require E key
>
> **Rationale**: Establishes "2.5D" aesthetic where the game world is 3D (procedural geometry) and characters are 2D (billboard sprites) using native Bevy PBR rendering.

## Overview

Replace remaining placeholder visuals (simple cuboids and flat planes) with procedural 3D meshes defined in pure Rust using Bevy primitives. This affects **Environmental Objects** (Trees) and **Static Interactive Props** (Event Markers: Portals/Teleports, Signs), enhancing visual fidelity without adding external asset dependencies.

**Scope**: Game Engine rendering only. Campaign Builder preview rendering remains unchanged (uses simplified geometry for performance).

**Goal**: Improve visual immersion by replacing generic cuboids with composite procedural meshes that convey object type at a glance.

**Note**: Character rendering (NPCs, Monsters, Recruitables) is handled by the Sprite Support implementation plan using billboard sprites with native Bevy PBR.

## Current State Analysis

### Existing Infrastructure

**Map Rendering System** (`src/game/systems/map.rs`):

- **Forest Tiles** (lines 471-501): Currently spawn single cuboid mesh using `TileVisualMetadata` for dimensions
- **Event Markers** (lines 802-849): Spawn flat `Plane3d` markers with color coding:
  - `SIGN_MARKER_COLOR`: Brown/tan (`Color::srgb(0.59, 0.44, 0.27)`)
  - `TELEPORT_MARKER_COLOR`: Purple (`Color::srgb(0.53, 0.29, 0.87)`)
  - Constants defined at lines 18-22
- **Mesh Caching** (lines 12-14, 318-326): `HashMap<MeshDimensions, Handle<Mesh>>` caches cuboid terrain meshes
- **Material System**: StandardMaterial with base_color, perceptual_roughness, emissive properties

**Domain Model** (`src/domain/world/types.rs`):

- `TerrainType::Forest` (line 52): Forest terrain type
- `MapEvent` enum (lines 414-500): Sign, Teleport, RecruitableCharacter, Treasure, etc.
- `TileVisualMetadata` (lines 67-104): Optional height, width_x, width_z, color_tint, scale, y_offset, rotation_y

**Current Visual Appearance**:

- Trees: Green cuboid blocks (no trunk/foliage distinction)
- Event markers: Flat colored squares on ground (no 3D depth)

### Identified Issues

1. **Lack of Visual Distinction**: All objects use generic cuboids/planes

   - Trees indistinguishable from other green blocks
   - Event markers blend into floor geometry

2. **Immersion Breaking**: Simple geometry reduces game atmosphere

   - No organic shapes (trees)
   - No architectural detail (signs, portals)
   - Flat marker planes cause z-fighting with floor

3. **No Mesh Reuse for Procedural Objects**: Terrain uses caching, but no strategy for procedural meshes

   - Could cause performance issues with many trees
   - Memory overhead from duplicate meshes

4. **Limited Visual Metadata Application**: `TileVisualMetadata` only applies to terrain cuboids
   - Trees can't have separate trunk/foliage customization
   - Event markers can't use height/rotation metadata

## Implementation Phases

### Phase 1: Core Procedural Mesh Infrastructure

#### 1.1 Create Procedural Meshes Module

**File**: `src/game/systems/procedural_meshes.rs`

**Actions**:

1. Create new file with SPDX header:

   ```
   // SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
   // SPDX-License-Identifier: Apache-2.0
   ```

2. Add module documentation:

   ```
   //! Procedural mesh generation for environmental objects and static event markers
   //!
   //! This module provides pure Rust functions to spawn composite 3D meshes using
   //! Bevy primitives (Cylinder, Sphere, Torus, Cuboid). No external assets required.
   //!
   //! Character rendering (NPCs, Monsters, Recruitables) uses the sprite system.
   ```

3. Import dependencies:

   ```
   use bevy::prelude::*;
   use crate::domain::types::{MapId, Position};
   use super::{MapEntity, TileCoord};
   ```

4. Define dimension constants (explicit values for AI clarity):

   ```
   // Tree dimensions (world units, 1 unit ≈ 10 feet)
   const TREE_TRUNK_RADIUS: f32 = 0.15;
   const TREE_TRUNK_HEIGHT: f32 = 2.0;
   const TREE_FOLIAGE_RADIUS: f32 = 0.6;
   const TREE_FOLIAGE_Y_OFFSET: f32 = 2.0;

   // Event marker dimensions
   const PORTAL_TORUS_MAJOR_RADIUS: f32 = 0.4;
   const PORTAL_TORUS_MINOR_RADIUS: f32 = 0.05;
   const PORTAL_Y_POSITION: f32 = 0.5;
   const PORTAL_ROTATION_SPEED: f32 = 1.0; // radians/sec

   const SIGN_POST_RADIUS: f32 = 0.05;
   const SIGN_POST_HEIGHT: f32 = 1.5;
   const SIGN_BOARD_WIDTH: f32 = 0.6;
   const SIGN_BOARD_HEIGHT: f32 = 0.3;
   const SIGN_BOARD_DEPTH: f32 = 0.05;
   const SIGN_BOARD_Y_OFFSET: f32 = 1.3;
   ```

5. Define color constants:

   ```
   const TREE_TRUNK_COLOR: Color = Color::srgb(0.4, 0.25, 0.15); // Brown
   const TREE_FOLIAGE_COLOR: Color = Color::srgb(0.2, 0.6, 0.2); // Green

   const PORTAL_COLOR: Color = Color::srgb(0.53, 0.29, 0.87); // Purple (from TELEPORT_MARKER_COLOR)
   const SIGN_POST_COLOR: Color = Color::srgb(0.4, 0.3, 0.2); // Dark brown
   const SIGN_BOARD_COLOR: Color = Color::srgb(0.59, 0.44, 0.27); // Tan (from SIGN_MARKER_COLOR)
   ```

**File**: `src/game/systems/mod.rs`

**Actions**:

1. Add module declaration after line 12 (after `pub mod map;`):
   ```
   pub mod procedural_meshes;
   ```

#### 1.2 Implement Tree Procedural Generation

**File**: `src/game/systems/procedural_meshes.rs`

**Function Signature**:

````rust
/// Spawns a procedural tree mesh with trunk and foliage
///
/// Creates two child entities:
/// - Trunk: Brown cylinder (0.15 radius, 2.0 height)
/// - Foliage: Green sphere (0.6 radius) positioned at trunk top
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position (x, y) in world coordinates
/// * `map_id` - Map identifier for cleanup
///
/// # Returns
///
/// Entity ID of the parent tree entity
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes;
/// use antares::domain::types::{MapId, Position};
///
/// // Inside a Bevy system:
/// let tree_entity = procedural_meshes::spawn_tree(
///     &mut commands,
///     &mut materials,
///     &mut meshes,
///     Position::new(5, 10),
///     MapId(1),
/// );
/// ```
pub fn spawn_tree(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Position,
    map_id: MapId,
) -> Entity
````

**Implementation Steps**:

1. Create trunk mesh: `Cylinder { radius: TREE_TRUNK_RADIUS, half_height: TREE_TRUNK_HEIGHT / 2.0 }`
2. Create trunk material: `StandardMaterial { base_color: TREE_TRUNK_COLOR, perceptual_roughness: 0.9, .. }`
3. Create foliage mesh: `Sphere { radius: TREE_FOLIAGE_RADIUS }`
4. Create foliage material: `StandardMaterial { base_color: TREE_FOLIAGE_COLOR, perceptual_roughness: 0.8, .. }`
5. Spawn parent entity at `(position.x as f32, 0.0, position.y as f32)` with `MapEntity(map_id)` and `TileCoord(position)`
6. Spawn trunk child at `Transform::from_xyz(0.0, TREE_TRUNK_HEIGHT / 2.0, 0.0)` (center at half-height)
7. Spawn foliage child at `Transform::from_xyz(0.0, TREE_FOLIAGE_Y_OFFSET, 0.0)`
8. Return parent entity ID

#### 1.3 Integrate Tree Spawning into Map Rendering

**File**: `src/game/systems/map.rs`

**Location**: Inside `spawn_map` function, Forest terrain handling (around lines 471-501)

**Changes**:

1. Remove existing Forest cuboid spawn code (lines 477-501)
2. Replace with:

   ```rust
   world::TerrainType::Forest => {
       // Render grass floor first
       commands.spawn((
           Mesh3d(floor_mesh.clone()),
           MeshMaterial3d(grass_material.clone()),
           Transform::from_xyz(x as f32, 0.0, y as f32),
           GlobalTransform::default(),
           Visibility::default(),
           MapEntity(map.id),
           TileCoord(pos),
       ));

       // Spawn procedural tree
       crate::game::systems::procedural_meshes::spawn_tree(
           &mut commands,
           &mut materials,
           &mut meshes,
           pos,
           map.id,
       );
   }
   ```

**Note**: Visual metadata (height, color_tint, rotation_y) will be ignored for procedural trees in this phase. Future enhancement could apply metadata to foliage/trunk separately.

#### 1.4 Testing Requirements

**Unit Tests** (`src/game/systems/procedural_meshes.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use crate::domain::types::{MapId, Position};

    #[test]
    fn test_tree_constants_valid() {
        assert!(TREE_TRUNK_RADIUS > 0.0);
        assert!(TREE_TRUNK_HEIGHT > 0.0);
        assert!(TREE_FOLIAGE_RADIUS > 0.0);
        assert!(TREE_FOLIAGE_Y_OFFSET > 0.0);
    }

    #[test]
    fn test_spawn_tree_creates_parent_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let tree_entity = app.world_mut().run_system_once(|
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        | {
            spawn_tree(
                &mut commands,
                &mut materials,
                &mut meshes,
                Position::new(5, 10),
                MapId(1),
            )
        });

        assert!(app.world().get_entity(tree_entity).is_ok());
    }

    #[test]
    fn test_spawn_tree_has_map_entity_component() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let tree_entity = app.world_mut().run_system_once(|
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        | {
            spawn_tree(
                &mut commands,
                &mut materials,
                &mut meshes,
                Position::new(3, 7),
                MapId(2),
            )
        });

        app.update();

        let map_entity = app.world().get::<MapEntity>(tree_entity).unwrap();
        assert_eq!(map_entity.0, MapId(2));
    }

    #[test]
    fn test_spawn_tree_has_tile_coord_component() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let tree_entity = app.world_mut().run_system_once(|
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        | {
            spawn_tree(
                &mut commands,
                &mut materials,
                &mut meshes,
                Position::new(8, 12),
                MapId(1),
            )
        });

        app.update();

        let tile_coord = app.world().get::<TileCoord>(tree_entity).unwrap();
        assert_eq!(tile_coord.0, Position::new(8, 12));
    }
}
```

**Quality Gates**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Manual Verification**:

1. Run game: `cargo run --bin antares`
2. Load tutorial map (or any map with Forest tiles)
3. Verify trees visible with brown trunk + green foliage
4. Verify trees positioned correctly on Forest tiles

#### 1.5 Deliverables

- [ ] `src/game/systems/procedural_meshes.rs` created with SPDX header
- [ ] Module registered in `src/game/systems/mod.rs`
- [ ] All dimension constants defined (tree and event markers)
- [ ] All color constants defined (tree and event markers)
- [ ] `spawn_tree()` function implemented with full doc comments
- [ ] Forest terrain updated to call `spawn_tree()` instead of cuboid spawn
- [ ] 4 unit tests passing (`test_tree_constants_valid`, `test_spawn_tree_creates_parent_entity`, `test_spawn_tree_has_map_entity_component`, `test_spawn_tree_has_tile_coord_component`)
- [ ] All quality gates passing (fmt, check, clippy, nextest)
- [ ] Manual verification completed (trees visible in-game)

#### 1.6 Success Criteria

- Module compiles without errors
- All 4 unit tests pass
- `cargo clippy` reports zero warnings
- Trees render in-game with distinct trunk and foliage
- Trees despawn correctly when map changes (MapEntity cleanup)
- No performance regression (framerate remains stable)

---

### Phase 2: Static Event Marker Procedural Meshes

**Note**: This phase implements only **static** event markers (Signs, Portals). Character-related markers (Recruitables) are handled by the Sprite Support plan.

#### 2.1 Implement Portal/Teleport Procedural Generation

**File**: `src/game/systems/procedural_meshes.rs`

**Function Signature**:

```rust
/// Spawns a procedural portal/teleport mesh (torus ring)
///
/// Creates a glowing purple torus positioned above ground.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
///
/// # Returns
///
/// Entity ID of the portal entity
pub fn spawn_portal(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Position,
    event_name: String,
    map_id: MapId,
) -> Entity
```

**Implementation Steps**:

1. Create torus mesh: `Torus { minor_radius: PORTAL_TORUS_MINOR_RADIUS, major_radius: PORTAL_TORUS_MAJOR_RADIUS }`
2. Create material: `StandardMaterial { base_color: PORTAL_COLOR, emissive: LinearRgba::from(PORTAL_COLOR) * 0.5, unlit: false, .. }`
3. Spawn entity at `(position.x as f32, PORTAL_Y_POSITION, position.y as f32)` rotated 90° on X-axis (ring stands vertical)
4. Add components: `MapEntity(map_id)`, `TileCoord(position)`, `Name::new(format!("PortalMarker_{}", event_name))`
5. Return entity ID

#### 2.2 Implement Sign Procedural Generation

**File**: `src/game/systems/procedural_meshes.rs`

**Function Signature**:

```rust
/// Spawns a procedural sign mesh (post + board)
///
/// Creates two child entities:
/// - Post: Brown cylinder (0.05 radius, 1.5 height)
/// - Board: Tan cuboid (0.6w × 0.3h × 0.05d) positioned at post top
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
///
/// # Returns
///
/// Entity ID of the parent sign entity
pub fn spawn_sign(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Position,
    event_name: String,
    map_id: MapId,
) -> Entity
```

**Implementation Steps**:

1. Create post mesh: `Cylinder { radius: SIGN_POST_RADIUS, half_height: SIGN_POST_HEIGHT / 2.0 }`
2. Create post material: `StandardMaterial { base_color: SIGN_POST_COLOR, perceptual_roughness: 0.9, .. }`
3. Create board mesh: `Cuboid::new(SIGN_BOARD_WIDTH, SIGN_BOARD_HEIGHT, SIGN_BOARD_DEPTH)`
4. Create board material: `StandardMaterial { base_color: SIGN_BOARD_COLOR, perceptual_roughness: 0.7, .. }`
5. Spawn parent entity at `(position.x as f32, 0.0, position.y as f32)`
6. Spawn post child at `Transform::from_xyz(0.0, SIGN_POST_HEIGHT / 2.0, 0.0)`
7. Spawn board child at `Transform::from_xyz(0.0, SIGN_BOARD_Y_OFFSET, 0.0)`
8. Add components: `MapEntity(map_id)`, `TileCoord(position)`, `Name::new(format!("SignMarker_{}", event_name))`
9. Return parent entity ID

#### 2.3 Integrate Event Marker Spawning

**File**: `src/game/systems/map.rs`

**Location**: Inside `spawn_map` function, event marker spawning loop (lines 802-849)

**Changes**:

1. Replace existing event marker spawn loop with:

   ```rust
   // Spawn procedural event markers for signs and teleports
   // Note: Recruitables handled by sprite system (see sprite_support_implementation_plan.md)
   for (position, event) in map.events.iter() {
       match event {
           world::MapEvent::Sign { name, .. } => {
               crate::game::systems::procedural_meshes::spawn_sign(
                   &mut commands,
                   &mut materials,
                   &mut meshes,
                   *position,
                   name.clone(),
                   map.id,
               );
           }
           world::MapEvent::Teleport { name, .. } => {
               crate::game::systems::procedural_meshes::spawn_portal(
                   &mut commands,
                   &mut materials,
                   &mut meshes,
                   *position,
                   name.clone(),
                   map.id,
               );
           }
           // RecruitableCharacter rendering handled by sprite system
           // Other events (Encounter, Trap, Treasure, NpcDialogue, InnEntry) have no visual markers
           _ => {}
       }
   }
   ```

2. Remove old constants (lines 18-22):
   - Delete `SIGN_MARKER_COLOR` (now in procedural_meshes.rs as `SIGN_BOARD_COLOR`)
   - Delete `TELEPORT_MARKER_COLOR` (now `PORTAL_COLOR`)
   - Delete `RECRUITABLE_CHARACTER_MARKER_COLOR` (no longer used - sprites handle this)
   - Delete `EVENT_MARKER_SIZE` (no longer needed)
   - Delete `EVENT_MARKER_Y_OFFSET` (replaced by individual Y positions)

#### 2.4 Testing Requirements

**Unit Tests** (`src/game/systems/procedural_meshes.rs`):

```rust
#[test]
fn test_portal_constants_valid() {
    assert!(PORTAL_TORUS_MAJOR_RADIUS > 0.0);
    assert!(PORTAL_TORUS_MINOR_RADIUS > 0.0);
    assert!(PORTAL_Y_POSITION > 0.0);
}

#[test]
fn test_sign_constants_valid() {
    assert!(SIGN_POST_RADIUS > 0.0);
    assert!(SIGN_POST_HEIGHT > 0.0);
    assert!(SIGN_BOARD_WIDTH > 0.0);
    assert!(SIGN_BOARD_HEIGHT > 0.0);
    assert!(SIGN_BOARD_DEPTH > 0.0);
}

#[test]
fn test_spawn_portal_creates_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let portal = app.world_mut().run_system_once(|
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
    | {
        spawn_portal(
            &mut commands,
            &mut materials,
            &mut meshes,
            Position::new(10, 10),
            "Portal1".to_string(),
            MapId(1),
        )
    });

    assert!(app.world().get_entity(portal).is_ok());
}

#[test]
fn test_spawn_sign_creates_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let sign = app.world_mut().run_system_once(|
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
    | {
        spawn_sign(
            &mut commands,
            &mut materials,
            &mut meshes,
            Position::new(5, 5),
            "Welcome".to_string(),
            MapId(2),
        )
    });

    assert!(app.world().get_entity(sign).is_ok());
}

#[test]
fn test_spawn_portal_has_name_component() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let portal = app.world_mut().run_system_once(|
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
    | {
        spawn_portal(
            &mut commands,
            &mut materials,
            &mut meshes,
            Position::new(3, 7),
            "TestPortal".to_string(),
            MapId(1),
        )
    });

    app.update();

    let name = app.world().get::<Name>(portal).unwrap();
    assert_eq!(name.as_str(), "PortalMarker_TestPortal");
}
```

**Quality Gates**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Manual Verification**:

1. Run game: `cargo run --bin antares`
2. Find map with events (tutorial map has signs, teleports)
3. Verify portals render as purple glowing torus rings
4. Verify signs render as post + board
5. Verify all markers positioned correctly on event tiles

#### 2.5 Deliverables

- [ ] `spawn_portal()` function implemented with full doc comments
- [ ] `spawn_sign()` function implemented with full doc comments
- [ ] Event marker spawn loop updated in `spawn_map` (only Signs and Teleports)
- [ ] Old color constants removed from `map.rs` (lines 18-22)
- [ ] 5 new unit tests passing (2 constant validation + 3 entity creation)
- [ ] All quality gates passing (fmt, check, clippy, nextest)
- [ ] Manual verification completed (portals and signs visible)

#### 2.6 Success Criteria

- Portals render as vertical torus rings with purple glow
- Signs render with post and board structure
- All markers positioned correctly at event tiles
- Old flat plane markers completely replaced
- No clippy warnings
- All 9 tests pass (4 tree + 5 event markers)

---

### Phase 3: Performance Optimization and Polish

#### 3.1 Add Mesh Caching for Procedural Meshes

**File**: `src/game/systems/procedural_meshes.rs`

**Rationale**: Multiple trees use identical meshes. Caching prevents duplicate mesh asset creation.

**Implementation**:

1. Add cache struct at module level:

   ```rust
   /// Cache for procedural mesh handles to avoid duplicate asset creation
   pub struct ProceduralMeshCache {
       tree_trunk: Option<Handle<Mesh>>,
       tree_foliage: Option<Handle<Mesh>>,
       portal_torus: Option<Handle<Mesh>>,
       sign_post: Option<Handle<Mesh>>,
       sign_board: Option<Handle<Mesh>>,
   }

   impl Default for ProceduralMeshCache {
       fn default() -> Self {
           Self {
               tree_trunk: None,
               tree_foliage: None,
               portal_torus: None,
               sign_post: None,
               sign_board: None,
           }
       }
   }
   ```

2. Update all `spawn_*` functions to accept `cache: &mut Local<ProceduralMeshCache>` parameter

3. Modify mesh creation to check cache first:

   ```rust
   let trunk_mesh = cache.tree_trunk.clone().unwrap_or_else(|| {
       let handle = meshes.add(Cylinder {
           radius: TREE_TRUNK_RADIUS,
           half_height: TREE_TRUNK_HEIGHT / 2.0,
       });
       cache.tree_trunk = Some(handle.clone());
       handle
   });
   ```

4. Update `spawn_map` to create and pass `Local<ProceduralMeshCache>`

**Note**: This is optional optimization. If implementation time is limited, defer to future enhancement.

#### 3.2 Add Visual Metadata Support (Future Enhancement)

**File**: `src/game/systems/procedural_meshes.rs`

**Changes**:

1. Add optional `visual_metadata: Option<&TileVisualMetadata>` parameter to `spawn_tree()`
2. Apply metadata to foliage:
   - `color_tint` → multiply foliage base color
   - `scale` → scale foliage sphere radius
   - `rotation_y` → rotate entire tree parent entity
3. Update `spawn_map` to pass `Some(&tile.visual)` for Forest tiles

**Note**: Out of scope for initial implementation. Document as future enhancement.

#### 3.3 Testing Requirements

**Performance Test** (manual):

1. Load large map with 100+ forest tiles
2. Measure FPS before/after procedural meshes
3. Verify no significant framerate drop (< 5% regression acceptable)
4. Monitor memory usage (should not grow excessively)

**Quality Gates**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

#### 3.4 Deliverables

- [ ] Performance testing completed (FPS measurements documented)
- [ ] Mesh caching implemented (if time permits)
- [ ] Visual metadata support documented as future enhancement
- [ ] All quality gates passing
- [ ] No performance regressions identified

#### 3.5 Success Criteria

- Performance impact < 5% FPS reduction
- Memory usage stable (no leaks)
- All optimizations compile and pass tests
- Documentation updated with enhancement notes

---

### Phase 4: Documentation and Verification

#### 4.1 Update Implementation Documentation

**File**: `docs/explanation/implementations.md`

**Section**: Add "Procedural Meshes Rendering" section

**Content**:

````markdown
### Procedural Meshes Rendering

**Completion Date**: [DATE]

**Implementation**: Phases 1-3 of procedural_meshes_implementation_plan.md

**Changes Made**:

1. **New Module**: `src/game/systems/procedural_meshes.rs`

   - Tree generation: Cylinder trunk + Sphere foliage
   - Portal generation: Torus ring with purple glow
   - Sign generation: Cylinder post + Cuboid board

2. **Map Rendering Updates**: `src/game/systems/map.rs`

   - Forest tiles: Replaced cuboid with `spawn_tree()`
   - Event markers: Replaced flat planes with 3D procedural meshes (Signs, Portals only)
   - Removed old event marker color constants (lines 18-22)

3. **Constants Defined**:

   - Dimension constants: Tree, Portal, Sign sizes
   - Color constants: Trunk, foliage, portal, sign colors
   - Positioning constants: Y offsets, rotation speeds

4. **Testing**:
   - 9 unit tests added (all passing)
   - Manual verification completed for all object types
   - Performance testing: < 3% FPS impact on large maps

**Object Rendering Details**:

- **Trees**: Brown cylinder trunk (0.15 radius, 2.0 height) + green sphere foliage (0.6 radius)
- **Portals**: Purple glowing torus (0.4 major radius, 0.05 minor radius) at y=0.5
- **Signs**: Dark brown post (0.05 radius, 1.5 height) + tan board (0.6×0.3×0.05)

**Character Rendering**: NPCs, Monsters, and Recruitables now use sprite billboards (see sprite_support_implementation_plan.md)

**Files Modified**:

- `src/game/systems/procedural_meshes.rs` (new file)
- `src/game/systems/mod.rs` (added module declaration)
- `src/game/systems/map.rs` (integrated procedural spawning, removed old code)

**Quality Verification**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```
````

**Results**: All checks passed, 9/9 tests passing, zero warnings.

**Future Enhancements**:

- Mesh caching for improved performance
- Visual metadata support (color_tint, scale, rotation for trees)
- Animated portals (rotating torus)
- Text rendering on sign boards

````

#### 4.2 Add Code Documentation

**All `spawn_*` functions** must have:

1. Doc comment with description
2. `# Arguments` section listing all parameters
3. `# Returns` section describing return value
4. `# Examples` section with usage example
5. Implementation comments for complex logic

**Module-level documentation** (`src/game/systems/procedural_meshes.rs`):

```rust
//! Procedural mesh generation for environmental objects and static event markers
//!
//! This module provides pure Rust functions to spawn composite 3D meshes using
//! Bevy primitives (Cylinder, Sphere, Torus, Cuboid). No external assets required.
//!
//! # Supported Objects
//!
//! - **Trees**: Cylinder trunk + Sphere foliage (for Forest terrain)
//! - **Portals**: Torus ring with glow (for Teleport events)
//! - **Signs**: Cylinder post + Cuboid board (for Sign events)
//!
//! # Character Rendering
//!
//! NPCs, Monsters, and Recruitable Characters use sprite billboards with native
//! Bevy PBR rendering. See sprite_support_implementation_plan.md.
//!
//! # Usage
//!
//! All spawn functions are called from `src/game/systems/map.rs` during map
//! rendering. They create parent entities with child mesh components, properly
//! tagged with `MapEntity` and `TileCoord` for cleanup.
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::procedural_meshes;
//!
//! // Inside a Bevy system with Commands, materials, meshes:
//! let tree = procedural_meshes::spawn_tree(
//!     &mut commands,
//!     &mut materials,
//!     &mut meshes,
//!     Position::new(5, 10),
//!     MapId(1),
//! );
//! ```
````

#### 4.3 Campaign Builder Documentation

**File**: `docs/explanation/implementations.md` (append to Procedural Meshes section)

**Content**:

```markdown
**Campaign Builder Impact**:

No changes required. Campaign Builder map preview continues to use simplified
rendering (cuboids) for performance. Procedural meshes only affect game engine
rendering (`cargo run --bin antares`).

If future Campaign Builder preview enhancement is desired, procedural mesh
functions can be reused by passing Campaign Builder's `Commands`/`Assets`.
```

#### 4.4 Verification Checklist

**Code Quality**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo nextest run --all-features` passes with all 9+ tests
- [ ] All public functions have doc comments with examples
- [ ] Module has comprehensive module-level documentation
- [ ] SPDX headers present in all new files

**Functionality**:

- [ ] Trees render with trunk and foliage
- [ ] Portals render as torus rings
- [ ] Signs render with post and board
- [ ] All objects positioned correctly on tiles
- [ ] All objects despawn correctly on map change

**Testing**:

- [ ] 9 unit tests passing (4 tree + 5 event markers)
- [ ] Manual verification completed for all object types
- [ ] Performance testing shows < 5% FPS impact
- [ ] No memory leaks or asset duplication

**Documentation**:

- [ ] Implementation section added to `implementations.md`
- [ ] All spawn functions have complete doc comments
- [ ] Module-level documentation comprehensive
- [ ] Future enhancements documented
- [ ] Campaign Builder impact documented

#### 4.5 Deliverables

- [ ] All Phase 1-3 code changes implemented and tested
- [ ] `docs/explanation/implementations.md` updated with Procedural Meshes section
- [ ] All code documentation complete (module + function doc comments)
- [ ] All verification checklist items completed
- [ ] Quality gates passing with zero errors/warnings
- [ ] Manual verification screenshots/videos (optional)

#### 4.6 Success Criteria

- All code quality checks pass
- All 9+ tests pass
- All functionality verified manually
- Documentation complete and accurate
- No regressions introduced
- Feature ready for production use

---

## Implementation Notes

### Design Decisions

1. **Composite Entities (Parent/Child)**: Trees and Signs use parent entity with child mesh entities

   - **Rationale**: Simplifies positioning (parent at tile coords, children offset relative)
   - **Benefit**: Easy to add more components later (animations, accessories)

2. **No External Assets**: Pure procedural generation using Bevy primitives

   - **Rationale**: Maintains zero-dependency visual system
   - **Benefit**: No asset pipeline, loading, or version management

3. **Color Constants**: Hardcoded colors instead of configuration files

   - **Rationale**: Visual tuning expected to be rare after initial implementation
   - **Benefit**: Simpler code, no I/O overhead
   - **Future**: Can move to config file if artists need runtime tweaking

4. **No Mesh Caching (Initial)**: Deferred to Phase 3 optimization

   - **Rationale**: Premature optimization; verify performance impact first
   - **Benefit**: Simpler initial implementation
   - **Future**: Add if profiling shows mesh creation bottleneck

5. **Visual Metadata Ignored (Initial)**: TileVisualMetadata not applied to trees

   - **Rationale**: Requires complex foliage/trunk separate customization
   - **Benefit**: Reduces scope, allows faster delivery
   - **Future**: Phase 3.2 enhancement adds metadata support

6. **Event Marker Selection**: Only Sign and Teleport get 3D markers
   - **Rationale**: Recruitables are characters, use sprite system for consistency
   - **Benefit**: Unified character rendering approach (all use sprites)

### Technical Considerations

**Entity Hierarchy**:

```
Parent Entity (Transform at tile coords)
├─ MapEntity(map_id)
├─ TileCoord(position)
├─ Name(optional)
└─ Children:
   ├─ Trunk/Post Mesh (relative transform)
   └─ Foliage/Board Mesh (relative transform)
```

**Mesh Primitive Usage**:

- `Cylinder`: Trees (trunk), Signs (post)
- `Sphere`: Trees (foliage)
- `Torus`: Portals
- `Cuboid`: Signs (board)

**Material Properties**:

- `base_color`: RGB color (from constants)
- `perceptual_roughness`: 0.5-0.9 (wood=0.9, metal=0.3)
- `emissive`: Portal markers glow (0.5 multiplier)
- `unlit`: Always false (use lighting system)

**Performance Characteristics**:

- Forest tile: 2 entities (trunk + foliage) vs. 1 cuboid (2× entity count)
- Event markers: 1-2 entities vs. 1 plane (up to 2× entity count)
- Expected impact: 1.5-2× total entities, < 5% FPS impact (modern GPUs handle easily)

### Future Enhancements (Out of Scope)

1. **Mesh Caching System**: `Local<ProceduralMeshCache>` to reuse identical meshes
2. **Visual Metadata Integration**: Apply `TileVisualMetadata` to procedural meshes
3. **Animation System**: Rotating portals, swaying trees
4. **Texture Mapping**: Add simple procedural textures (wood grain, stone)
5. **LOD System**: Simpler meshes at distance (performance optimization)
6. **Text Rendering**: Display sign text on board using 3D text or billboards
7. **Particle Effects**: Portal sparkles, environmental effects

---

## Appendix: Testing Strategy

### Unit Test Coverage

**Per-Function Tests** (9 total):

1. `test_tree_constants_valid` - Validates all tree constants > 0
2. `test_spawn_tree_creates_parent_entity` - Tree entity exists
3. `test_spawn_tree_has_map_entity_component` - MapEntity component attached
4. `test_spawn_tree_has_tile_coord_component` - TileCoord component attached
5. `test_portal_constants_valid` - Validates portal constants > 0
6. `test_sign_constants_valid` - Validates sign constants > 0
7. `test_spawn_portal_creates_entity` - Portal entity exists
8. `test_spawn_sign_creates_entity` - Sign entity exists
9. `test_spawn_portal_has_name_component` - Name component formatted correctly

### Integration Test Coverage

**Map Rendering Integration** (manual):

1. Load tutorial map → verify all trees, event markers render
2. Teleport between maps → verify old entities despawn, new entities spawn
3. Walk around map → verify no z-fighting, clipping, or visual artifacts
4. Change time of day (if applicable) → verify materials respond to lighting

### Performance Test Coverage

**Benchmarks** (manual):

1. **Small Map** (10×10, 5 trees): Baseline FPS
2. **Medium Map** (50×50, 50 trees): FPS delta
3. **Large Map** (100×100, 200 trees): FPS delta
4. **Memory Usage**: Measure before/after across all maps

**Acceptance Criteria**:

- Small map: > 120 FPS (no performance concern)
- Medium map: > 60 FPS (acceptable for gameplay)
- Large map: > 30 FPS (minimum acceptable, rare edge case)
- Memory delta: < 100 MB increase

---

## References

**Related Files**:

- `src/game/systems/map.rs`: Map rendering system
- `src/game/systems/mod.rs`: System module registration
- `src/domain/world/types.rs`: Domain model (TerrainType, MapEvent, TileVisualMetadata)
- `docs/explanation/implementations.md`: Implementation documentation
- `docs/explanation/sprite_support_implementation_plan.md`: Character sprite rendering
- `AGENTS.md`: Development rules and quality gates

**Bevy Documentation**:

- [Bevy Primitives](https://docs.rs/bevy/latest/bevy/prelude/primitives/): Mesh primitive types
- [Bevy StandardMaterial](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html): Material properties
- [Bevy Entity Hierarchy](https://docs.rs/bevy/latest/bevy/hierarchy/): Parent/child relationships

**Architecture References**:

- `docs/reference/architecture.md` Section 4.2: World System (Map, Tile, MapEvent)
- `docs/reference/architecture.md` Section 7.3: Rendering Strategy
