# NPC Gameplay Fix Implementation Plan

## Overview

This plan resolves the issue where NPCs are neither visible nor interactable in gameplay. It provides a phased approach to implementing NPC sprites (using placeholders), connecting dialogue events, and ensuring NPCs block movement as required.

## Current State Analysis

### Existing Infrastructure

- `MapEvent::NpcDialogue` exists but has a `TODO` in its handler.
- `Npc` struct exists in map data but lacks visual metadata.
- `MapRenderingPlugin` renders tiles and event markers but completely ignores NPC data in `Map.npcs`.

### Identified Issues

- [events.rs:L105-111](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/events.rs#L105-L111) only logs a message and does not trigger dialogue.
- [map.rs:spawn_map](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs#L242-L503) lacks logic to iterate over and spawn NPC entities.
- NPCs are currently embedded in maps, making it difficult to reference global dialogue trees.

## Implementation Phases

### Phase 1: NPC Externalization & Blocking

**Goal**: Implement the core domain changes from the NPC Externalization plan and ensure NPC tiles block movement.

#### 1.1 Foundation Work

- Complete **Phases 1-2** of [npc_externalization_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/npc_externalization_implementation_plan.md).
- Update `NpcDefinition` to include required fields for the engine.

#### 1.2 Add Blocking Logic

- Modify `Map::is_blocked` in [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs) to check if an NPC occupies the target position.
- Ensure `NpcPlacement` or the `Map.npcs` vector is consulted during movement checks.

#### 1.4 Testing Requirements

- Unit test `Map::is_blocked` with and without NPCs.
- Verify RON serialization of the new `NpcPlacement` model.

#### 1.5 Deliverables

- [x] Updated `src/domain/world/types.rs` with NPC-aware blocking.
- [x] `src/domain/world/npc.rs` with `NpcDefinition` and `NpcPlacement`.
- [x] Migrated `tutorial` campaign map files (Phase 2 of externalization plan).

**Status**: ✅ COMPLETED - See `docs/explanation/phase1_npc_blocking_implementation_summary.md`

---

### Phase 2: NPC Visual Representation (Placeholders)

**Goal**: Make NPCs visible on the map using immediate colored placeholders.

#### 2.1 Feature Work

- Update `spawn_map` in [map.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs) to iterate over NPCs.
- Spawn a vertical plane or cuboid (placeholder) at each NPC tile position.
- Use a distinct color (e.g., Cyan `(0.0, 1.0, 1.0)`) and tag with `MapEntity` and a new `NpcMarker` component.

#### 2.2 Integrate Feature

- Update `spawn_map_markers` to handle NPC markers alongside tile markers.
- Ensure NPCs are despawned/respawned during map transitions.

#### 2.4 Testing Requirements

- Manual verification: Run game and confirm cyan markers appear at NPC coordinates (e.g., 1,16 on Map 1).

#### 2.5 Deliverables

- [x] NPC rendering logic in `src/game/systems/map.rs`.
- [x] `NpcMarker` component for ECS tracking.

**Status**: ✅ COMPLETED - See `docs/explanation/phase2_npc_visual_implementation_summary.md`

---

### Phase 3: Dialogue Event Connection

**Goal**: Trigger actual dialogue when interacting with an NPC.

#### 3.1 Feature Work

- Update `handle_events` in [events.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/events.rs).
- Look up `NpcDefinition` in `GameContent` database using the `npc_id`.
- If `dialogue_id` is present, send a `StartDialogue` message.
- If no `dialogue_id`, send the legacy `dialogue` string to the `GameLog`.

#### 3.2 Integrate Feature

- Update `application/mod.rs` to correctly initialize `DialogueState` with the resolved `dialogue_id` and root node.

#### 3.4 Testing Requirements

- Integration test: Stepping near an NPC (or triggering its event) opens the dialogue UI.
- Verify fallback to log for NPCs without dialogue trees.

#### 3.5 Deliverables

- [ ] Updated `events.rs` system.
- [ ] Updated `application/mod.rs` event handling.

---

## Verification Plan

### Automated Tests

- `cargo nextest run --all-features`
- Specific tests for `Map::is_blocked` and `database::NpcDatabase`.

### Manual Verification

- **Visual**: Launch game, confirm Village Elder is visible as a cyan marker in the Town Square.
- **Interaction**: Press "Talk" or move towards NPC; verify dialogue UI or log entry appears.
- **Blocking**: Attempt to walk into the Village Elder's tile; verify movement is blocked.
