# Dropped Item World Persistence Implementation Plan

## Overview

When a character drops an item via the inventory UI, the item is currently
removed from the character's inventory and discarded. There is no mechanism to
place it in the game world at the position where it was dropped, nor to
represent it as a pickable entity on the map. This plan adds full dropped-item
world persistence: items placed on the map survive save/load, are rendered with
a visual marker, and can be picked up by the player.

The implementation is divided into four phases: (1) domain-layer data model,
(2) transaction logic and event wiring, (3) game-engine visual representation,
and (4) save/load validation and full test coverage.

## Current State Analysis

### Existing Infrastructure

- `Map` struct in [src/domain/world/types.rs](../../src/domain/world/types.rs)
  has an `events: HashMap<Position, MapEvent>` field and
  `npc_placements: Vec<NpcPlacement>`, but no `dropped_items` field.
- `EventResult` in [src/domain/world/events.rs](../../src/domain/world/events.rs)
  has `Encounter`, `Treasure`, `Teleported`, `Trap`, `Sign`, `NpcDialogue`,
  `RecruitableCharacter`, `EnterInn`, `EnterMerchant`, `Furniture`, and
  `EnterContainer` variants. No `PickupItem` variant exists.
- The drop operation lives in `inventory_action_system` in
  [src/game/systems/inventory_ui.rs](../../src/game/systems/inventory_ui.rs):
  it calls `inventory.remove_item(slot_index)` and logs the fact but discards
  the item entirely. No `drop_item()` domain function exists in
  [src/domain/transactions.rs](../../src/domain/transactions.rs).
- `SaveGame` in [src/application/save_game.rs](../../src/application/save_game.rs)
  serializes the entire `GameState` (which owns `World` → `maps`) via RON. Any
  field added to `Map` with `#[serde(default)]` will automatically round-trip
  through saves with no additional wiring.
- `ItemId = u8`, `MapId = u16`, `Position` are defined in
  [src/domain/types.rs](../../src/domain/types.rs).

### Identified Issues

1. `DropItemAction` discards items permanently with no world record.
2. `Map` has no storage for items that exist on the ground.
3. No pickup interaction: `trigger_event` cannot produce a pickup result.
4. No visual representation: dropped items are invisible.
5. Game engine does not spawn/despawn world entities for dropped items on
   map load/unload.

---

## Implementation Phases

### Phase 1: Domain Data Model

Add the `DroppedItem` struct and extend `Map` to store dropped items per map.
This is a pure domain change with no Bevy dependencies.

#### 1.1 Add `DroppedItem` Struct

Create a new module `src/domain/world/dropped_items.rs` with:

```rust
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

- `DroppedItem` struct with fields:
  - `item_id: ItemId`
  - `charges: u8`
  - `position: Position`
  - `map_id: MapId`
- `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`
- Full `///` doc comment with a runnable doctest.

Re-export `DroppedItem` from `src/domain/world/mod.rs` alongside the other
public world types.

#### 1.2 Extend `Map` With `dropped_items`

In [src/domain/world/types.rs](../../src/domain/world/types.rs), add a field
to the `Map` struct after `npc_placements`:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub dropped_items: Vec<DroppedItem>,
```

Update `Map::new()` to initialize `dropped_items: Vec::new()`.

Add helper methods to `Map`:

- `add_dropped_item(&mut self, item: DroppedItem)` – appends to
  `dropped_items`.
- `remove_dropped_item(&mut self, position: Position, item_id: ItemId) -> Option<DroppedItem>` –
  finds the first matching entry, removes it, and returns it.
- `dropped_items_at(&self, position: Position) -> Vec<&DroppedItem>` – returns
  all items at a given tile (multiple items can stack on one tile).

#### 1.3 Testing Requirements

Tests in `dropped_items.rs` and `types.rs`:

- `test_add_dropped_item_appends_entry`
- `test_remove_dropped_item_returns_correct_entry`
- `test_remove_dropped_item_missing_returns_none`
- `test_dropped_items_at_position_returns_all`
- `test_dropped_items_field_default_is_empty` (serde round-trip via RON)

#### 1.4 Deliverables

- [ ] `src/domain/world/dropped_items.rs` with `DroppedItem` struct
- [ ] `dropped_items` field added to `Map` struct
- [ ] `Map::add_dropped_item`, `Map::remove_dropped_item`, `Map::dropped_items_at` helpers
- [ ] `DroppedItem` re-exported from `src/domain/world/mod.rs`
- [ ] All domain tests pass

#### 1.5 Success Criteria

`cargo check --all-targets --all-features` and
`cargo nextest run --all-features` pass with zero errors/warnings. A RON
round-trip test confirms `dropped_items` serializes and deserializes correctly
through `SaveGame`.

---

### Phase 2: Transaction Logic and Event Wiring

Add `drop_item()` and `pickup_item()` to the transactions domain layer, update
the `DropItemAction` handler, add a `PickupItem` event result, and wire pickup
interaction into `trigger_event`.

#### 2.1 Add `drop_item()` to `src/domain/transactions.rs`

Add a new `TransactionError` variant:

```rust
#[error("Map {map_id} not found in world")]
MapNotFound { map_id: MapId },
```

Implement:

```rust
pub fn drop_item(
    character: &mut Character,
    character_id: CharacterId,
    slot_index: usize,
    world: &mut World,
    map_id: MapId,
    position: Position,
) -> Result<DroppedItem, TransactionError>
```

Logic:
1. Remove item from `character.inventory` at `slot_index`.
2. If slot_index is out-of-bounds return `TransactionError::ItemNotInInventory`.
3. Look up the map by `map_id` in `world`; return `MapNotFound` if absent.
4. Construct a `DroppedItem { item_id, charges, position, map_id }`.
5. Call `map.add_dropped_item(dropped_item.clone())`.
6. Return `Ok(dropped_item)`.

#### 2.2 Add `pickup_item()` to `src/domain/transactions.rs`

Implement:

```rust
pub fn pickup_item(
    character: &mut Character,
    character_id: CharacterId,
    world: &mut World,
    map_id: MapId,
    position: Position,
    item_id: ItemId,
) -> Result<InventorySlot, TransactionError>
```

Logic:
1. Verify `character.inventory` is not full; return `InventoryFull` if so.
2. Look up the map by `map_id`; return `MapNotFound` if absent.
3. Call `map.remove_dropped_item(position, item_id)`; return
   `ItemNotInInventory` if `None`.
4. Call `character.inventory.add_item(item_id, charges)`.
5. Return `Ok(InventorySlot { item_id, charges })`.

#### 2.3 Add `PickupItem` to `EventResult`

In [src/domain/world/events.rs](../../src/domain/world/events.rs), add:

```rust
/// Player may pick up a dropped item at this position
PickupItem {
    /// The item available for pickup
    item_id: ItemId,
    /// Remaining charges on the item
    charges: u8,
    /// World position of the item
    position: Position,
},
```

#### 2.4 Extend `trigger_event` for Dropped Items

In [src/domain/world/events.rs](../../src/domain/world/events.rs), add a check
at the start of `trigger_event` — before the event HashMap lookup — that
inspects `map.dropped_items_at(position)`. If any dropped items are present and
no other event is at that position, return:

```rust
EventResult::PickupItem {
    item_id: dropped.item_id,
    charges: dropped.charges,
    position,
}
```

When multiple items are stacked, return the first one (FIFO). The pickup action
will re-trigger interaction to surface the next item.

#### 2.5 Update `DropItemAction` Handler in `inventory_ui.rs`

In `inventory_action_system`, replace the bare `remove_item` call with a call
to `drop_item()`. The handler needs access to:

- The current `GlobalState` (party + world).
- The current map ID and party position from `GlobalState`.

Change the drop block to:

```rust
let map_id = global_state.0.world.current_map;
let position = global_state.0.world.party_position;
if let Err(e) = drop_item(&mut character, character_id, slot_index, &mut world, map_id, position) {
    warn!("drop_item failed: {e}");
}
```

Emit a `DroppedItemWorldEvent { item_id, position, map_id }` Bevy event so the
rendering system can spawn a visual marker (see Phase 3).

#### 2.6 Add Interaction Handler for `PickupItem` Result

In the existing input/interaction system (or `src/game/systems/interaction.rs`),
handle `EventResult::PickupItem` by calling `pickup_item()` and emitting a
`PickedUpItemWorldEvent { item_id, position, map_id }` Bevy event for the
visual system to despawn the marker.

#### 2.7 Testing Requirements

All tests use `data/test_campaign` — no references to `campaigns/tutorial`.

- `test_drop_item_records_in_world` – drop item, verify `world.maps[map_id].dropped_items` has the entry.
- `test_drop_item_removes_from_inventory` – confirm character inventory is shorter.
- `test_drop_item_out_of_bounds_slot_returns_error`
- `test_pickup_item_adds_to_inventory` – pickup item, verify inventory gained entry.
- `test_pickup_item_removes_from_map` – confirm `dropped_items` is empty after pickup.
- `test_pickup_item_inventory_full_returns_error`
- `test_pickup_item_missing_returns_error`
- `test_trigger_event_returns_pickup_when_item_present` – unit test of `trigger_event`.

#### 2.8 Deliverables

- [ ] `drop_item()` in `src/domain/transactions.rs`
- [ ] `pickup_item()` in `src/domain/transactions.rs`
- [ ] `MapNotFound` variant in `TransactionError`
- [ ] `PickupItem` variant in `EventResult`
- [ ] `trigger_event` updated to surface dropped items
- [ ] `DropItemAction` handler in `inventory_ui.rs` updated to call `drop_item()`
- [ ] `DroppedItemWorldEvent` and `PickedUpItemWorldEvent` Bevy events declared
- [ ] All domain + integration tests pass

#### 2.9 Success Criteria

All quality gates pass. Dropping an item in the running game no longer
silently discards it — the world state contains the dropped item entry.
`trigger_event` returns `PickupItem` when the party steps on a tile containing
a dropped item.

---

### Phase 3: Visual Representation in the Game Engine

Spawn and despawn mesh/sprite markers for dropped items so that the player can
see items on the ground.

#### 3.1 Define Visual Marker Component and Bundle

In `src/game/systems/` (likely a new `dropped_item_visuals.rs`):

- `DroppedItemMarker` component carrying `item_id: ItemId`, `position: Position`,
  `map_id: MapId`.
- A spawn helper `spawn_dropped_item_marker(commands, meshes, materials, item)`
  that creates a small flat quad mesh (or reuses an existing sprite) at the
  world-space position of the tile, slightly elevated above the floor plane
  (e.g. `y = 0.05`). A glowing dot or a small bag icon are acceptable stand-ins
  until art assets are ready.

#### 3.2 Spawning on Map Load

In the map-load system (wherever NPC and encounter markers are currently
spawned), add a pass after NPC spawning:

```rust
for dropped in &map.dropped_items {
    if dropped.map_id == current_map_id {
        spawn_dropped_item_marker(&mut commands, &mut meshes, &mut materials, dropped);
    }
}
```

#### 3.3 Spawn on Drop

Listen for `DroppedItemWorldEvent` (emitted in Phase 2.5) and immediately
spawn a marker entity at the drop position without requiring a map reload.

#### 3.4 Despawn on Pickup

Listen for `PickedUpItemWorldEvent` (emitted in Phase 2.6). Query for a
`DroppedItemMarker` whose `item_id` and `position` match, then call
`commands.entity(entity).despawn()`.

#### 3.5 Cleanup on Map Unload

When the current map changes, despawn all entities with a `DroppedItemMarker`
component that belong to the previous map, preventing stale markers from
persisting across teleports.

#### 3.6 Testing Requirements

Integration-level tests (Bevy `App` tests):

- `test_spawn_marker_on_map_load` – load a map with a pre-populated
  `dropped_items` list; assert a `DroppedItemMarker` entity exists at the
  correct tile position.
- `test_spawn_marker_on_drop_event` – emit `DroppedItemWorldEvent`; assert
  marker is spawned.
- `test_despawn_marker_on_pickup_event` – emit `PickedUpItemWorldEvent`; assert
  marker is gone.
- `test_marker_cleanup_on_map_unload` – change current map; assert no stale
  markers remain.

#### 3.7 Deliverables

- [ ] `src/game/systems/dropped_item_visuals.rs` with marker component and
  spawn/despawn helpers
- [ ] Map-load system updated to spawn markers for `map.dropped_items`
- [ ] `DroppedItemWorldEvent` handler spawns marker on drop
- [ ] `PickedUpItemWorldEvent` handler despawns marker on pickup
- [ ] Map-unload cleanup system
- [ ] All visual integration tests pass

#### 3.8 Success Criteria

Running the game, dropping an item shows a visible marker on the floor at the
party's position. Walking onto the marker and pressing the interaction key picks
it up and removes the marker from the screen.

---

### Phase 4: Save/Load Validation and End-to-End Testing

Confirm that `dropped_items` survives a full save/load round-trip and that
integration tests cover the entire flow end-to-end.

#### 4.1 Save Game RON Round-Trip

Because `Map` already derives `Serialize`/`Deserialize` and `dropped_items`
uses `#[serde(default)]`, no new wiring is required in
[src/application/save_game.rs](../../src/application/save_game.rs). Verify by:

- Dropping an item in a test scenario.
- Serializing `GameState` to RON via `SaveGameManager::save`.
- Deserializing via `SaveGameManager::load`.
- Asserting the loaded world's map still contains the `DroppedItem` entry.

#### 4.2 Add `data/test_campaign` Fixture Data

Add two items to `data/test_campaign/data/items.ron` (if not already present)
that can be used as fixtures in integration tests: a weapon and a consumable.
Do **not** reference `campaigns/tutorial` in any test.

#### 4.3 End-to-End Integration Test

In `tests/dropped_item_integration_test.rs`:

- `test_dropped_item_round_trip_save_load` – full flow: drop item, save game,
  load game, verify dropped item present in world, interact to pick it up,
  verify inventory updated and world cleared.
- `test_multiple_items_stacked_on_same_tile` – drop two items on the same tile
  and confirm both persist and both can be picked up in sequence.
- `test_dropped_item_scoped_to_map` – drop item on map 1, teleport to map 2,
  teleport back to map 1, confirm item still present.

#### 4.4 Testing Requirements

All tests use `data/test_campaign`, not `campaigns/tutorial`.
`cargo nextest run --all-features` must show `test result: ok. X passed; 0 failed`.

#### 4.5 Deliverables

- [ ] `tests/dropped_item_integration_test.rs` with all three integration tests
- [ ] `data/test_campaign/data/items.ron` contains required test fixtures
- [ ] Save/load round-trip confirmed in automated test
- [ ] `docs/explanation/implementations.md` updated

#### 4.6 Success Criteria

All four quality gates pass:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Dropped items survive a full save/load cycle with no data loss. Items stack
correctly on a single tile and are individually retrievable.

---

## Sequence Summary

| Phase | Core Outputs | Quality Gate |
|-------|-------------|--------------|
| 1 | `DroppedItem` struct, `Map.dropped_items`, helper methods | `cargo check` + unit tests |
| 2 | `drop_item()`, `pickup_item()`, `PickupItem` event, `trigger_event` update, `DropItemAction` wired | Full test suite |
| 3 | Visual marker spawn/despawn on drop, pickup, map load/unload | Bevy integration tests |
| 4 | Save/load round-trip, end-to-end integration tests, `implementations.md` updated | Full test suite |

## Architecture Compliance Checklist

- [ ] `DroppedItem` uses `ItemId`, `MapId`, `Position` type aliases (no raw `u32`/`u16`)
- [ ] New `.rs` files include SPDX header
- [ ] All public structs/functions have `///` doc comments with doctests
- [ ] `dropped_items` field uses `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- [ ] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [ ] New fixture data added to `data/test_campaign/data/`
- [ ] RON format used for all data files
- [ ] `docs/explanation/implementations.md` updated after completion
