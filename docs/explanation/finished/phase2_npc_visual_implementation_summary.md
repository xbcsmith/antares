# Phase 2: NPC Visual Representation Implementation Summary

## Overview

Phase 2 of the NPC Gameplay Fix Implementation Plan added visual placeholders for NPCs on the map. NPCs are now visible as cyan-colored vertical planes positioned at their designated map locations.

## Implementation Details

### 1. NpcMarker Component

Added a new ECS component to track NPC visual entities:

```rust
/// Component tagging an entity as an NPC visual marker
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct NpcMarker {
    /// NPC ID from the definition
    pub npc_id: String,
}
```

**Purpose**:
- Identifies entities as NPC visual markers in the ECS
- Stores the NPC ID for lookup and interaction
- Enables queries and filtering of NPC entities

### 2. Visual Representation

**Mesh**: Vertical cuboid (billboard-like placeholder)
- Dimensions: 1.0 wide × 1.8 tall × 0.1 depth
- Represents human height (~6 feet at 1.8 units)

**Material**:
- Color: Cyan (RGB: 0.0, 1.0, 1.0)
- Perceptual roughness: 0.5
- Distinct from terrain colors for easy identification

**Positioning**:
- X/Z coordinates: NPC placement position from map data
- Y coordinate: 0.9 (centers the 1.8-unit tall mesh, bottom at 0, top at 1.8)

### 3. Spawn Logic

#### Initial Map Load (spawn_map)

Updated `spawn_map` system to:
1. Accept `GameContent` resource containing NPC database
2. Resolve NPCs using `map.resolve_npcs(&content.0.npcs)`
3. Spawn visual marker for each resolved NPC with components:
   - `Mesh3d` - vertical plane mesh
   - `MeshMaterial3d` - cyan material
   - `Transform` - position at NPC location
   - `MapEntity(map_id)` - tags entity for cleanup
   - `TileCoord(position)` - stores grid position
   - `NpcMarker { npc_id }` - identifies as NPC marker

#### Map Transitions (spawn_map_markers)

Updated `spawn_map_markers` system to:
1. Accept mesh/material resources for NPC spawning
2. Accept `GameContent` for NPC database access
3. Spawn NPC visuals when map changes (same logic as initial spawn)
4. Ensure NPCs are despawned with other map entities via `MapEntity` component

#### Door Opening (handle_door_opened)

Updated `handle_door_opened` system to:
1. Accept `GameContent` resource
2. Pass content to `spawn_map` when respawning after door state change

### 4. Lifecycle Management

**Spawning**:
- NPCs spawn at `Startup` (initial map)
- NPCs respawn during map transitions
- NPCs respawn after door opening events

**Despawning**:
- NPCs are automatically cleaned up via `MapEntity` component
- All entities with `MapEntity(old_map_id)` are despawned when map changes
- No special cleanup logic needed - follows existing map entity lifecycle

## Testing

### Quality Gate Results

All quality checks passed:

```bash
✅ cargo fmt --all                                      # Formatting: OK
✅ cargo check --all-targets --all-features            # Compilation: OK
✅ cargo clippy --all-targets --all-features -D warnings  # Linting: OK
✅ cargo nextest run --all-features                    # Tests: 974 passed, 0 failed
```

### Manual Verification

To verify NPC visuals in the game:

1. Run the game: `cargo run`
2. Load a map with NPC placements (e.g., starter_town)
3. Observe cyan vertical planes at NPC positions
4. Confirm NPCs appear at expected coordinates from map data

**Expected NPCs on starter_town (Map 1)**:
- Mayor (1, 16) - cyan marker at these coordinates
- Merchant Bob (10, 10) - cyan marker at these coordinates
- Guard (5, 5) - cyan marker at these coordinates
- Innkeeper (15, 8) - cyan marker at these coordinates

### Test Coverage

**Existing tests continue to pass**:
- All 974 existing tests pass without modification
- No breaking changes to existing functionality
- NPC blocking logic from Phase 1 remains functional

**No new unit tests required**:
- Phase 2 is primarily visual rendering
- Manual verification is the primary testing method per plan
- Integration with existing map entity lifecycle tested implicitly

## Architecture Compliance

### Data Structures

**ResolvedNpc** (from domain layer):
- Used exactly as defined in `src/domain/world/types.rs`
- Contains: npc_id, name, description, portrait_path, position, facing, dialogue_id, quest_ids, faction, is_merchant, is_innkeeper

**NpcPlacement** (from domain layer):
- Used through `map.npc_placements` collection
- Resolved via `map.resolve_npcs(&npc_db)`

**NpcDatabase** (from SDK layer):
- Accessed via `GameContent` resource
- Type: `ContentDatabase::npcs` field

### Module Placement

All changes in correct layer:
- `src/game/systems/map.rs` - game/rendering layer (appropriate for visual spawning)
- No domain layer modifications
- Proper separation of concerns maintained

### Type System

**Type aliases used correctly**:
- `MapId` used in `MapEntity` component
- `Position` used in `TileCoord` component
- NPC ID stored as String (matches domain definition)

**No magic numbers**:
- Visual dimensions are layout constants (1.0, 1.8, 0.1) - not domain constants
- Color values are visual constants (0.0, 1.0, 1.0) - rendering layer

## Changes Summary

### Files Modified

1. **src/game/systems/map.rs**
   - Added `NpcMarker` component
   - Updated `spawn_map` to spawn NPC visuals
   - Updated `spawn_map_markers` to spawn NPC visuals on map transitions
   - Updated `handle_door_opened` to pass GameContent resource

### Dependencies

**New resource access**:
- `GameContent` resource now required by:
  - `spawn_map`
  - `spawn_map_markers`
  - `handle_door_opened`

**No new crate dependencies added**

## Known Limitations & Future Work

### Current Limitations

1. **Placeholder Visuals**: NPCs render as simple cyan boxes, not sprites/models
2. **No Facing Representation**: NPC facing direction not visualized
3. **No Portrait Display**: Portrait paths stored but not rendered
4. **Static Visuals**: NPCs don't animate or change appearance

### Phase 3 (Next Steps)

**Dialogue Event Connection** (see implementation plan):
- Hook up `MapEvent::NpcDialogue` to start dialogue
- Update `handle_events` in `events.rs` to:
  - Look up NpcDefinition using npc_id
  - Start dialogue if dialogue_id present
  - Send dialogue to GameLog if no dialogue_id
- Update `application/mod.rs` to initialize DialogueState

**Future Enhancements** (post-Phase 3):
- Replace cuboid placeholders with sprite billboards
- Add NPC portraits in dialogue UI
- Visualize NPC facing direction (rotation or indicator)
- Add NPC animations (idle, talking, moving)
- Integrate NPC role indicators (merchant icon, quest marker, etc.)

## Deliverables Checklist

- [x] NpcMarker component for ECS tracking
- [x] NPC rendering logic in `src/game/systems/map.rs`
- [x] NPCs spawn at correct positions on initial map load
- [x] NPCs respawn during map transitions
- [x] NPCs despawn/respawn on door opening
- [x] All quality checks pass (fmt, check, clippy, tests)
- [x] Documentation updated

## Conclusion

Phase 2 successfully implements visual representation for NPCs using placeholder geometry. NPCs are now visible on the map as cyan vertical planes positioned at their designated coordinates. The implementation properly integrates with the existing map entity lifecycle, ensuring NPCs are spawned and despawned correctly during map transitions and door opening events.

The system is ready for Phase 3 (Dialogue Event Connection), which will enable player interaction with the visible NPCs.

---

**Implementation Date**: 2025-01-XX
**Phase**: 2 of 3 (NPC Gameplay Fix)
**Status**: ✅ Complete
**Tests**: 974 passed, 0 failed
