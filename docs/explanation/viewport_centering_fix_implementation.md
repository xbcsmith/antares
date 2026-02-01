# Viewport Centering Fix Implementation Plan

## Overview

Fix the party viewpoint centering issue where doors and other map objects appear
offset by half a tile during navigation. The camera correctly centers within
tiles, but map objects are positioned at tile corners, creating visual
misalignment.

## Current State Analysis

### Existing Infrastructure

The Antares game engine uses Bevy for 3D rendering with a first-person camera
system and tile-based map rendering:

- **Camera system**: [camera.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/camera.rs)
  - Lines 232-236: `update_first_person_camera` positions camera at
    `party_pos.x + 0.5` and `party_pos.y + 0.5`
- **Map rendering**: [map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs)
  - `spawn_map` function handles tile, wall, door, terrain, NPC, and event
    marker spawning
- **Procedural meshes**: [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs)
  - `spawn_tree`, `spawn_portal`, `spawn_sign` functions for environmental objects

### Identified Issues

1. **Camera-object position mismatch**: Camera adds +0.5 offset to center in
   tiles, but map objects position at integer coordinates (tile corners)
2. **Affected rendering locations** in `map.rs`:
   - Line 473: Water terrain
   - Line 513: Mountain terrain
   - Line 531: Forest grass floor
   - Line 553: Grass terrain
   - Line 565: Ground/other floor
   - Line 625: Normal walls
   - Line 668: Doors
   - Line 714: Torches
   - Line 775: NPC sprites
3. **Affected locations** in `procedural_meshes.rs`:
   - Line 183: `spawn_tree` parent transform
   - Line 293: `spawn_portal` parent transform
   - Line 429: `spawn_sign` parent transform

## Implementation Phases

### Phase 1: Core Map Rendering Centering

#### 1.1 Foundation Work

Define `TILE_CENTER_OFFSET` constant in [map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs).

Add after the imports section (around line 75):

```rust
/// Offset to center map objects within their tile (matches camera centering)
const TILE_CENTER_OFFSET: f32 = 0.5;
```

#### 1.2 Add Foundation Functionality

Update all terrain and wall transform spawns in the `spawn_map` function to add `TILE_CENTER_OFFSET` to x and z coordinates.

**Comprehensive Transformation Table**:

| Line | Context | Current Transform | Target Transform | Notes |
|------|---------|-------------------|------------------|-------|
| 473 | Water terrain | `Transform::from_xyz(x as f32, -0.1, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, -0.1, y as f32 + TILE_CENTER_OFFSET)` | Y unchanged at -0.1 |
| 513 | Mountain terrain | `Transform::from_xyz(x as f32, y_pos, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, y_pos, y as f32 + TILE_CENTER_OFFSET)` | Y from metadata |
| 531 | Forest grass floor | `Transform::from_xyz(x as f32, 0.0, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, 0.0, y as f32 + TILE_CENTER_OFFSET)` | Y=0 ground level |
| 553 | Grass terrain | `Transform::from_xyz(x as f32, 0.0, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, 0.0, y as f32 + TILE_CENTER_OFFSET)` | Y=0 ground level |
| 565 | Ground/other floor | `Transform::from_xyz(x as f32, 0.0, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, 0.0, y as f32 + TILE_CENTER_OFFSET)` | Y=0 ground level |
| 625 | Normal walls | `Transform::from_xyz(x as f32, y_pos, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, y_pos, y as f32 + TILE_CENTER_OFFSET)` | Y from metadata |
| 668 | Doors | `Transform::from_xyz(x as f32, y_pos, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, y_pos, y as f32 + TILE_CENTER_OFFSET)` | Y from metadata |
| 714 | Torches | `Transform::from_xyz(x as f32, y_pos, y as f32)` | `Transform::from_xyz(x as f32 + TILE_CENTER_OFFSET, y_pos, y as f32 + TILE_CENTER_OFFSET)` | Y from metadata |

**Step-by-step instructions**:

1. Locate line 473 (Water terrain spawn) - search for `world::TerrainType::Water`
2. Locate line 513 (Mountain terrain spawn) - search for `world::TerrainType::Mountain`
3. Locate line 531 (Forest grass floor) - search for `world::TerrainType::Forest`
4. Locate line 553 (Grass terrain) - search for `world::TerrainType::Grass`
5. Locate line 565 (Ground/other floor) - search for `_ =>` in terrain match
6. Locate line 625 (Normal walls) - search for `world::WallType::Normal`
7. Locate line 668 (Doors) - search for `world::WallType::Door`
8. Locate line 714 (Torches) - search for `world::WallType::Torch`

For each location, replace `x as f32` with `x as f32 + TILE_CENTER_OFFSET` and `y as f32` with `y as f32 + TILE_CENTER_OFFSET`.

#### 1.3 Integrate Foundation Work

Update NPC sprite positioning in the `spawn_map` function.

**NPC Transform Update**:

| Line | Context | Current Transform | Target Transform | Notes |
|------|---------|-------------------|------------------|-------|
| 775 | NPC sprites | `Vec3::new(x, 0.9, y)` | `Vec3::new(x + TILE_CENTER_OFFSET, 0.9, y + TILE_CENTER_OFFSET)` | Y=0.9 sprite height |

**Step-by-step instructions**:

1. Locate line 775 in `spawn_map` function
2. Search for `Vec3::new(x, 0.9, y)` in the NPC spawning section
3. Replace with `Vec3::new(x + TILE_CENTER_OFFSET, 0.9, y + TILE_CENTER_OFFSET)`

#### 1.4 Testing Requirements

Add unit test to verify constant and centered position calculations.

**Test Implementation**:

Add to the `tests` module at the end of [map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs) (after line 1039):

```rust
#[test]
fn test_tile_positions_are_centered() {
    // Verify the TILE_CENTER_OFFSET constant is 0.5
    assert_eq!(TILE_CENTER_OFFSET, 0.5);
    
    // Verify centered position calculation
    let tile_x = 5;
    let tile_y = 10;
    let centered_x = tile_x as f32 + TILE_CENTER_OFFSET;
    let centered_z = tile_y as f32 + TILE_CENTER_OFFSET;
    
    assert_eq!(centered_x, 5.5);
    assert_eq!(centered_z, 10.5);
}
```

**Verification Commands**:

```bash
# Run specific test
cargo test map::tests::test_tile_positions_are_centered

# Run all map tests
cargo test map::tests
```

**Expected Output**:
```
running 1 test
test map::tests::test_tile_positions_are_centered ... ok
```

#### 1.5 Deliverables

- [ ] [map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs) updated with `TILE_CENTER_OFFSET` constant
- [ ] 8 terrain/wall transforms updated (lines 473, 513, 531, 553, 565, 625, 668, 714)
- [ ] 1 NPC transform updated (line 775)
- [ ] Unit test `test_tile_positions_are_centered` added and passing

#### 1.6 Success Criteria

**Automated Validation**:

```bash
# Verify compilation succeeds
cargo build
# Expected: Build completes without errors

# Run unit tests
cargo test map::tests::test_tile_positions_are_centered
# Expected: test map::tests::test_tile_positions_are_centered ... ok

# Run all map tests
cargo test map::tests
# Expected: All tests pass

# Verify no warnings
cargo clippy -- -D warnings
# Expected: No warnings or errors
```

**Manual Validation**:

1. Run game: `cargo run`
2. Navigate party to a door at any position
3. Verify door appears centered in viewport (not offset to tile corner)
4. Navigate to an NPC
5. Verify NPC sprite appears centered in tile
6. Navigate to different terrain types (water, mountain, grass)
7. Verify all terrain appears centered

---

### Phase 2: Procedural Mesh Centering

#### 2.1 Foundation Work

Define `TILE_CENTER_OFFSET` constant in [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs).

Add after the color constants section (around line 100):

```rust
/// Offset to center procedural meshes within their tile
const TILE_CENTER_OFFSET: f32 = 0.5;
```

#### 2.2 Add Foundation Functionality

Update parent transforms in `spawn_tree`, `spawn_portal`, and `spawn_sign` functions.

**Procedural Mesh Transformation Table**:

| Line | Function | Context | Current Transform | Target Transform | Notes |
|------|----------|---------|-------------------|------------------|-------|
| 183 | `spawn_tree` | Parent entity | `Transform::from_xyz(position.x as f32, 0.0, position.y as f32)` | `Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, 0.0, position.y as f32 + TILE_CENTER_OFFSET)` | Y=0 ground level |
| 293 | `spawn_portal` | Parent entity | `Transform::from_xyz(position.x as f32, PORTAL_Y_POSITION, position.y as f32)` | `Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, PORTAL_Y_POSITION, position.y as f32 + TILE_CENTER_OFFSET)` | Y from constant |
| 429 | `spawn_sign` | Parent entity | `Transform::from_xyz(position.x as f32, 0.0, position.y as f32)` | `Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, 0.0, position.y as f32 + TILE_CENTER_OFFSET)` | Y=0 ground level |

**Step-by-step instructions**:

1. Open [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs)

2. Locate `spawn_tree` function (around line 143):
   - Find line 183: `Transform::from_xyz(position.x as f32, 0.0, position.y as f32)`
   - Replace with: `Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, 0.0, position.y as f32 + TILE_CENTER_OFFSET)`

3. Locate `spawn_portal` function (around line 242):
   - Find line 293: `let transform = Transform::from_xyz(position.x as f32, PORTAL_Y_POSITION, position.y as f32)`
   - Replace with: `let transform = Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, PORTAL_Y_POSITION, position.y as f32 + TILE_CENTER_OFFSET)`

4. Locate `spawn_sign` function (around line 384):
   - Find line 429: `let transform = Transform::from_xyz(position.x as f32, 0.0, position.y as f32)`
   - Replace with: `let transform = Transform::from_xyz(position.x as f32 + TILE_CENTER_OFFSET, 0.0, position.y as f32 + TILE_CENTER_OFFSET)`

#### 2.3 Integrate Foundation Work

No integration work required - procedural meshes are self-contained functions.

#### 2.4 Testing Requirements

Add unit test to verify constant and centered position calculations for procedural meshes.

**Test Implementation**:

Add to the `tests` module at the end of [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs) (after line 635):

```rust
#[test]
fn test_procedural_mesh_centering_offset() {
    assert_eq!(TILE_CENTER_OFFSET, 0.5);
    
    // Verify offset produces centered coordinates
    let pos = types::Position { x: 3, y: 7 };
    let centered_x = pos.x as f32 + TILE_CENTER_OFFSET;
    let centered_z = pos.y as f32 + TILE_CENTER_OFFSET;
    
    assert_eq!(centered_x, 3.5);
    assert_eq!(centered_z, 7.5);
}
```

**Verification Commands**:

```bash
# Run specific test
cargo test procedural_meshes::tests::test_procedural_mesh_centering_offset

# Run all procedural mesh tests
cargo test procedural_meshes::tests
```

**Expected Output**:
```
running 1 test
test procedural_meshes::tests::test_procedural_mesh_centering_offset ... ok
```

#### 2.5 Deliverables

- [ ] [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs) updated with `TILE_CENTER_OFFSET` constant
- [ ] `spawn_tree` parent transform updated (line 183)
- [ ] `spawn_portal` parent transform updated (line 293)
- [ ] `spawn_sign` parent transform updated (line 429)
- [ ] Unit test `test_procedural_mesh_centering_offset` added and passing

#### 2.6 Success Criteria

**Automated Validation**:

```bash
# Verify compilation succeeds
cargo build
# Expected: Build completes without errors

# Run unit tests
cargo test procedural_meshes::tests::test_procedural_mesh_centering_offset
# Expected: test procedural_meshes::tests::test_procedural_mesh_centering_offset ... ok

# Run all procedural mesh tests
cargo test procedural_meshes::tests
# Expected: All tests pass

# Verify no warnings
cargo clippy -- -D warnings
# Expected: No warnings or errors

# Run all tests
cargo test
# Expected: All tests pass including new centering tests
```

**Manual Validation**:

1. Run game: `cargo run`
2. Navigate to a map with signs
3. Verify signs appear centered in their tiles
4. Navigate to a portal/teleport marker
5. Verify portal appears centered in tile
6. Navigate to a forest tile with trees
7. Verify trees appear centered in tiles
8. Compare with camera position - all objects should align with camera center

## Files to Modify

| File | Changes | Deliverables |
|------|---------|--------------|
| [map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs) | Add `TILE_CENTER_OFFSET` constant, update 9 transforms (8 terrain/wall + 1 NPC), add 1 unit test | Phase 1.5: 4 deliverables |
| [procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs) | Add `TILE_CENTER_OFFSET` constant, update 3 transforms (tree, portal, sign), add 1 unit test | Phase 2.5: 5 deliverables |

## Summary

**Total Changes**:
- 2 files modified
- 2 constants added
- 12 transforms updated
- 2 unit tests added
- 9 deliverables tracked

**Validation**:
- Automated: `cargo test`, `cargo build`, `cargo clippy`
- Manual: In-game visual verification of centering for doors, NPCs, signs, portals, trees
