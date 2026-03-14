// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Save/Load Validation and End-to-End Tests for Dropped Item Persistence
//!
//! Confirms that `DroppedItem` entries survive a full `SaveGameManager::save` /
//! `SaveGameManager::load` round-trip, that multiple items can stack on the same
//! tile and be recovered individually (FIFO order), and that dropped items are
//! strictly scoped to their originating map.
//!
//! # Test Coverage
//!
//! - `test_dropped_item_round_trip_save_load` — drop → save → load → trigger → pickup
//! - `test_multiple_items_stacked_on_same_tile` — two items on one tile, FIFO retrieval
//! - `test_dropped_item_scoped_to_map` — item on map 1 invisible from map 2 after load
//!
//! # Fixture Note
//!
//! All tests use items defined in `data/test_campaign/data/items.ron`:
//! - Item 3  = Short Sword  (Weapon, max_charges = 0)
//! - Item 50 = Healing Potion (Consumable, max_charges = 1)
//!
//! No test references `campaigns/tutorial`.

use antares::application::save_game::SaveGameManager;
use antares::application::GameState;
use antares::domain::character::{Alignment, Character, Sex};
use antares::domain::transactions::{drop_item, pickup_item};
use antares::domain::types::{GameTime, ItemId, MapId, Position};
use antares::domain::world::{trigger_event, EventResult, Map};
use tempfile::TempDir;

// ============================================================================
// Constants — sourced from data/test_campaign/data/items.ron
// ============================================================================

/// Short Sword — a weapon with `max_charges = 0`.
const WEAPON_ITEM_ID: ItemId = 3;

/// Short Sword has no charges (non-magical).
const WEAPON_CHARGES: u8 = 0;

/// Healing Potion — a consumable with `max_charges = 1`.
const CONSUMABLE_ITEM_ID: ItemId = 50;

/// Healing Potion has one charge.
const CONSUMABLE_CHARGES: u8 = 1;

/// Primary map used by most tests.
const MAP_ONE: MapId = 1;

/// Secondary map used by `test_dropped_item_scoped_to_map`.
const MAP_TWO: MapId = 2;

// ============================================================================
// Shared helpers
// ============================================================================

/// Creates a minimal `Character` suitable for use as a party member in tests.
fn make_character(name: &str) -> Character {
    Character::new(
        name.to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    )
}

/// Builds a `GameState` with one 20×20 map (id = `map_id`) set as current.
fn game_state_with_map(map_id: MapId) -> GameState {
    let mut state = GameState::new();
    let map = Map::new(map_id, format!("Map {map_id}"), String::new(), 20, 20);
    state.world.add_map(map);
    state.world.set_current_map(map_id);
    state
}

// ============================================================================
// Test 1 — Full save/load round-trip
// ============================================================================

/// Drop one item, save, load, verify item survives, then pick it up.
///
/// Specifically verifies:
/// 1. `drop_item` removes the item from the character's inventory.
/// 2. The `DroppedItem` entry is written to `Map::dropped_items`.
/// 3. `SaveGameManager::save` serialises the entry to RON.
/// 4. `SaveGameManager::load` deserialises it faithfully (all fields intact).
/// 5. `trigger_event` returns `EventResult::PickupItem` at the drop tile.
/// 6. `pickup_item` returns the item to the character's inventory.
/// 7. `Map::dropped_items` is empty after the pickup.
#[test]
fn test_dropped_item_round_trip_save_load() {
    // ── Arrange ──────────────────────────────────────────────────────────────
    let drop_pos = Position::new(5, 5);
    let mut state = game_state_with_map(MAP_ONE);
    state.world.set_party_position(drop_pos);

    let mut character = make_character("Theodric");
    character
        .inventory
        .add_item(WEAPON_ITEM_ID, WEAPON_CHARGES)
        .expect("inventory must accept weapon");
    state
        .party
        .add_member(character)
        .expect("party must accept member");

    // ── Act: drop the weapon ─────────────────────────────────────────────────
    // NLL field-split borrow: `state.party` and `state.world` are distinct
    // fields of `GameState`, so Rust permits simultaneous mutable borrows.
    {
        let party_ref = &mut state.party.members[0];
        let world_ref = &mut state.world;
        drop_item(party_ref, 0, 0, world_ref, MAP_ONE, drop_pos).expect("drop_item must succeed");
    }

    // Character inventory must now be empty.
    assert!(
        state.party.members[0].inventory.items.is_empty(),
        "inventory must be empty after drop"
    );

    // Map must carry exactly one dropped item with the expected field values.
    {
        let map = state.world.get_map(MAP_ONE).expect("map must exist");
        assert_eq!(map.dropped_items.len(), 1, "map must have one dropped item");
        let entry = &map.dropped_items[0];
        assert_eq!(
            entry.item_id, WEAPON_ITEM_ID,
            "item_id mismatch before save"
        );
        assert_eq!(
            entry.charges, WEAPON_CHARGES,
            "charges mismatch before save"
        );
        assert_eq!(entry.position, drop_pos, "position mismatch before save");
        assert_eq!(entry.map_id, MAP_ONE, "map_id mismatch before save");
    }

    // ── Act: save ────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager
        .save("round_trip", &state)
        .expect("save must succeed");

    // ── Act: load ────────────────────────────────────────────────────────────
    let mut loaded = manager.load("round_trip").expect("load must succeed");
    loaded.world.set_current_map(MAP_ONE);

    // ── Assert: dropped item survived the round-trip ─────────────────────────
    {
        let map = loaded
            .world
            .get_map(MAP_ONE)
            .expect("map 1 must exist after load");
        assert_eq!(
            map.dropped_items.len(),
            1,
            "dropped item must survive the RON save/load round-trip"
        );
        let entry = &map.dropped_items[0];
        assert_eq!(
            entry.item_id, WEAPON_ITEM_ID,
            "item_id must be intact after load"
        );
        assert_eq!(
            entry.charges, WEAPON_CHARGES,
            "charges must be intact after load"
        );
        assert_eq!(
            entry.position, drop_pos,
            "position must be intact after load"
        );
        assert_eq!(entry.map_id, MAP_ONE, "map_id must be intact after load");
    }

    // ── Assert: trigger_event surfaces the PickupItem result ─────────────────
    let game_time = GameTime::new(1, 12, 0);
    let event_result = trigger_event(&mut loaded.world, drop_pos, &game_time)
        .expect("trigger_event must not error");

    match event_result {
        EventResult::PickupItem {
            item_id,
            charges,
            position,
        } => {
            assert_eq!(item_id, WEAPON_ITEM_ID, "PickupItem must carry weapon id");
            assert_eq!(
                charges, WEAPON_CHARGES,
                "PickupItem must carry weapon charges"
            );
            assert_eq!(position, drop_pos, "PickupItem must carry drop position");
        }
        other => panic!("Expected PickupItem, got {other:?}"),
    }

    // ── Act: pick up the item ────────────────────────────────────────────────
    {
        let party_ref = &mut loaded.party.members[0];
        let world_ref = &mut loaded.world;
        let slot = pickup_item(party_ref, 0, world_ref, MAP_ONE, drop_pos, WEAPON_ITEM_ID)
            .expect("pickup_item must succeed");
        assert_eq!(
            slot.item_id, WEAPON_ITEM_ID,
            "returned slot must carry weapon id"
        );
        assert_eq!(
            slot.charges, WEAPON_CHARGES,
            "returned slot must carry weapon charges"
        );
    }

    // ── Assert: inventory updated and world cleared ──────────────────────────
    assert_eq!(
        loaded.party.members[0].inventory.items.len(),
        1,
        "inventory must have exactly one item after pickup"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items[0].item_id, WEAPON_ITEM_ID,
        "inventory item must be the weapon"
    );
    assert!(
        loaded
            .world
            .get_map(MAP_ONE)
            .unwrap()
            .dropped_items
            .is_empty(),
        "dropped_items must be empty after pickup"
    );
}

// ============================================================================
// Test 2 — Multiple items stacked on the same tile
// ============================================================================

/// Drops two items on the same tile, saves, loads, and verifies both can be
/// picked up individually in FIFO order.
///
/// Specifically verifies:
/// 1. Multiple `DroppedItem` entries can share one tile (stacking).
/// 2. Both entries survive a save/load round-trip.
/// 3. `trigger_event` surfaces the first-inserted item (FIFO).
/// 4. After the first pickup, `trigger_event` surfaces the second item.
/// 5. After the second pickup, `Map::dropped_items` is empty.
/// 6. Both items end up in the character's inventory.
#[test]
fn test_multiple_items_stacked_on_same_tile() {
    // ── Arrange ──────────────────────────────────────────────────────────────
    let drop_pos = Position::new(8, 8);
    let mut state = game_state_with_map(MAP_ONE);
    state.world.set_party_position(drop_pos);

    let mut character = make_character("Aldric");
    // Load the character with a weapon (slot 0) and a potion (slot 1).
    character
        .inventory
        .add_item(WEAPON_ITEM_ID, WEAPON_CHARGES)
        .expect("inventory must accept weapon");
    character
        .inventory
        .add_item(CONSUMABLE_ITEM_ID, CONSUMABLE_CHARGES)
        .expect("inventory must accept potion");
    state
        .party
        .add_member(character)
        .expect("party must accept character");

    // ── Act: drop weapon first (slot 0), then potion (slot 0 after shift) ────
    {
        let party_ref = &mut state.party.members[0];
        let world_ref = &mut state.world;
        // Drop weapon — it occupies slot 0; after removal potion shifts to slot 0.
        drop_item(party_ref, 0, 0, world_ref, MAP_ONE, drop_pos).expect("drop weapon must succeed");
    }
    {
        let party_ref = &mut state.party.members[0];
        let world_ref = &mut state.world;
        // Drop potion — now at slot 0 after weapon was removed.
        drop_item(party_ref, 0, 0, world_ref, MAP_ONE, drop_pos).expect("drop potion must succeed");
    }

    // Both items must now be on the map at the same position.
    {
        let map = state.world.get_map(MAP_ONE).unwrap();
        assert_eq!(
            map.dropped_items.len(),
            2,
            "map must have two dropped items"
        );
        let at_tile = map.dropped_items_at(drop_pos);
        assert_eq!(at_tile.len(), 2, "both items must be at the drop tile");
        assert_eq!(
            at_tile[0].item_id, WEAPON_ITEM_ID,
            "first entry (FIFO) must be weapon"
        );
        assert_eq!(
            at_tile[1].item_id, CONSUMABLE_ITEM_ID,
            "second entry must be potion"
        );
    }
    // Character inventory must be empty (both items dropped).
    assert!(
        state.party.members[0].inventory.items.is_empty(),
        "inventory must be empty after dropping both items"
    );

    // ── Act: save and load ───────────────────────────────────────────────────
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager.save("stacked", &state).expect("save must succeed");
    let mut loaded = manager.load("stacked").expect("load must succeed");
    loaded.world.set_current_map(MAP_ONE);

    // ── Assert: both items present after load ────────────────────────────────
    {
        let map = loaded
            .world
            .get_map(MAP_ONE)
            .expect("map must exist after load");
        assert_eq!(
            map.dropped_items.len(),
            2,
            "both dropped items must survive the save/load round-trip"
        );
        let at_tile = map.dropped_items_at(drop_pos);
        assert_eq!(
            at_tile.len(),
            2,
            "both items must be at the same tile after load"
        );
    }

    // ── Act: trigger_event → pick up first item (FIFO = weapon) ─────────────
    let game_time = GameTime::new(1, 12, 0);

    let first_result = trigger_event(&mut loaded.world, drop_pos, &game_time)
        .expect("trigger_event must not error (first item)");
    let first_id = match first_result {
        EventResult::PickupItem { item_id, .. } => {
            assert_eq!(
                item_id, WEAPON_ITEM_ID,
                "first pickup must be weapon (FIFO)"
            );
            item_id
        }
        other => panic!("Expected PickupItem for first item, got {other:?}"),
    };

    {
        let party_ref = &mut loaded.party.members[0];
        let world_ref = &mut loaded.world;
        pickup_item(party_ref, 0, world_ref, MAP_ONE, drop_pos, first_id)
            .expect("first pickup must succeed");
    }

    assert_eq!(
        loaded.world.get_map(MAP_ONE).unwrap().dropped_items.len(),
        1,
        "one item must remain on the map after first pickup"
    );

    // ── Act: trigger_event → pick up second item (potion) ───────────────────
    let second_result = trigger_event(&mut loaded.world, drop_pos, &game_time)
        .expect("trigger_event must not error (second item)");
    let second_id = match second_result {
        EventResult::PickupItem { item_id, .. } => {
            assert_eq!(item_id, CONSUMABLE_ITEM_ID, "second pickup must be potion");
            item_id
        }
        other => panic!("Expected PickupItem for second item, got {other:?}"),
    };

    {
        let party_ref = &mut loaded.party.members[0];
        let world_ref = &mut loaded.world;
        pickup_item(party_ref, 0, world_ref, MAP_ONE, drop_pos, second_id)
            .expect("second pickup must succeed");
    }

    // ── Assert: world cleared; both items in inventory ───────────────────────
    assert!(
        loaded
            .world
            .get_map(MAP_ONE)
            .unwrap()
            .dropped_items
            .is_empty(),
        "dropped_items must be empty after picking up both items"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items.len(),
        2,
        "inventory must contain both recovered items"
    );
    let inv = &loaded.party.members[0].inventory.items;
    assert!(
        inv.iter().any(|s| s.item_id == WEAPON_ITEM_ID),
        "weapon must be in inventory"
    );
    assert!(
        inv.iter().any(|s| s.item_id == CONSUMABLE_ITEM_ID),
        "potion must be in inventory"
    );
}

// ============================================================================
// Test 3 — Dropped items are scoped to their originating map
// ============================================================================

/// Drop an item on map 1, save, load, verify it is present on map 1 and absent
/// on map 2.  Then switch back to map 1 and confirm the item is still reachable
/// via `trigger_event`.
///
/// Specifically verifies:
/// 1. `DroppedItem` entries are attached to the owning map, not a global store.
/// 2. Items on map 1 are invisible to `trigger_event` when map 2 is active.
/// 3. Items on map 1 survive a save/load round-trip without bleeding to map 2.
/// 4. Returning to map 1 (simulated by setting `current_map`) exposes the item
///    again through `trigger_event`.
#[test]
fn test_dropped_item_scoped_to_map() {
    // ── Arrange ──────────────────────────────────────────────────────────────
    let drop_pos = Position::new(3, 7);
    let mut state = GameState::new();

    // Populate the world with two maps.
    state.world.add_map(Map::new(
        MAP_ONE,
        "Map One".to_string(),
        String::new(),
        20,
        20,
    ));
    state.world.add_map(Map::new(
        MAP_TWO,
        "Map Two".to_string(),
        String::new(),
        20,
        20,
    ));
    state.world.set_current_map(MAP_ONE);
    state.world.set_party_position(drop_pos);

    let mut character = make_character("Seraphina");
    character
        .inventory
        .add_item(WEAPON_ITEM_ID, WEAPON_CHARGES)
        .expect("inventory must accept weapon");
    state
        .party
        .add_member(character)
        .expect("party must accept character");

    // ── Act: drop weapon on map 1 ────────────────────────────────────────────
    {
        let party_ref = &mut state.party.members[0];
        let world_ref = &mut state.world;
        drop_item(party_ref, 0, 0, world_ref, MAP_ONE, drop_pos)
            .expect("drop on map 1 must succeed");
    }

    // Pre-save assertions: item on map 1, nothing on map 2.
    assert_eq!(
        state.world.get_map(MAP_ONE).unwrap().dropped_items.len(),
        1,
        "map 1 must have one dropped item before save"
    );
    assert!(
        state
            .world
            .get_map(MAP_TWO)
            .unwrap()
            .dropped_items
            .is_empty(),
        "map 2 must have no dropped items before save"
    );

    // ── Act: save ────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager
        .save("scoped_map", &state)
        .expect("save must succeed");

    // ── Act: load ────────────────────────────────────────────────────────────
    let mut loaded = manager.load("scoped_map").expect("load must succeed");

    // ── Assert: item on map 1, absent on map 2, after load ───────────────────
    {
        let map1 = loaded
            .world
            .get_map(MAP_ONE)
            .expect("map 1 must exist after load");
        assert_eq!(
            map1.dropped_items.len(),
            1,
            "item must still be on map 1 after save/load"
        );
        let entry = &map1.dropped_items[0];
        assert_eq!(entry.item_id, WEAPON_ITEM_ID, "item_id must be intact");
        assert_eq!(entry.position, drop_pos, "position must be intact");
        assert_eq!(entry.map_id, MAP_ONE, "map_id must point to map 1");
    }
    {
        let map2 = loaded
            .world
            .get_map(MAP_TWO)
            .expect("map 2 must exist after load");
        assert!(
            map2.dropped_items.is_empty(),
            "map 2 must have no dropped items after save/load"
        );
    }

    // ── Assert: trigger_event on map 1 at drop_pos returns PickupItem ────────
    loaded.world.set_current_map(MAP_ONE);
    let game_time = GameTime::new(1, 12, 0);
    let result_map1 = trigger_event(&mut loaded.world, drop_pos, &game_time)
        .expect("trigger_event must not error on map 1");

    assert!(
        matches!(
            result_map1,
            EventResult::PickupItem { item_id, .. } if item_id == WEAPON_ITEM_ID
        ),
        "trigger_event on map 1 must return PickupItem for the dropped weapon; \
         got {result_map1:?}"
    );

    // ── Assert: trigger_event on map 2 at the same position returns None ─────
    // (the item is scoped to map 1 — map 2 has no dropped items at this tile)
    loaded.world.set_current_map(MAP_TWO);
    let result_map2 = trigger_event(&mut loaded.world, drop_pos, &game_time)
        .expect("trigger_event must not error on map 2");

    assert!(
        matches!(result_map2, EventResult::None),
        "trigger_event on map 2 must return None — item is scoped to map 1; \
         got {result_map2:?}"
    );
}
