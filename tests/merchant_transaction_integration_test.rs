// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests: Merchant Transaction Flow (Phase 6)
//!
//! End-to-end tests covering the complete merchant buy/sell flow from game
//! state construction through transaction execution through save/load round-trip
//! verification.
//!
//! # Test Coverage
//!
//! - `test_merchant_buy_flow_end_to_end` – full buy path with save/load verification
//! - `test_merchant_sell_flow_end_to_end` – full sell path: item removed, gold added
//! - `test_merchant_buy_respects_inventory_limit` – `InventoryFull` when backpack full
//! - `test_merchant_stock_depletes_after_buy` – `OutOfStock` after last unit purchased
//! - `test_merchant_stock_persists_depletion_after_save_load` – depletion survives round-trip

use antares::application::save_game::SaveGameManager;
use antares::application::GameState;
use antares::domain::character::{Alignment, Character, Inventory, Sex};
use antares::domain::inventory::{MerchantStock, StockEntry};
use antares::domain::items::{Item, ItemDatabase, ItemType, WeaponClassification, WeaponData};
use antares::domain::transactions::{buy_item, sell_item, TransactionError};
use antares::domain::types::{DiceRoll, ItemId};
use antares::domain::world::npc::NpcDefinition;
use antares::domain::world::npc_runtime::NpcRuntimeState;
use tempfile::TempDir;

// ============================================================================
// Shared Test Helpers
// ============================================================================

/// Build an `Item` suitable for use in tests.
///
/// Produces a weapon with predictable `base_cost` and `sell_cost` so that
/// price calculations in assertions are straightforward.
fn make_test_item(id: ItemId, base_cost: u32, sell_cost: u32) -> Item {
    Item {
        id,
        name: format!("TestItem {id}"),
        item_type: ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 6, 0),
            bonus: 0,
            hands_required: 1,
            classification: WeaponClassification::Simple,
        }),
        base_cost,
        sell_cost,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    }
}

/// Construct an `ItemDatabase` pre-loaded with the provided items.
fn item_db_with(items: Vec<Item>) -> ItemDatabase {
    let mut db = ItemDatabase::new();
    for item in items {
        db.add_item(item).unwrap();
    }
    db
}

/// Build an `NpcRuntimeState` for a merchant with the given stock entries.
fn merchant_runtime(npc_id: &str, entries: Vec<StockEntry>) -> NpcRuntimeState {
    let mut state = NpcRuntimeState::new(npc_id.to_string());
    state.stock = Some(MerchantStock {
        entries,
        restock_template: None,
    });
    state
}

/// Create a standard test `Character`.
fn make_character(name: &str) -> Character {
    Character::new(
        name.to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    )
}

// ============================================================================
// 6.1  Merchant Buy Flow – End-to-End
// ============================================================================

/// Full buy path: create `GameState`, set gold, buy item, verify state, then
/// save and reload and confirm the loaded state matches.
#[test]
fn test_merchant_buy_flow_end_to_end() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let item_id: ItemId = 1;
    let base_cost: u32 = 10;

    let item_db = item_db_with(vec![make_test_item(item_id, base_cost, 5)]);

    let mut npc_runtime = merchant_runtime("merchant_bob", vec![StockEntry::new(item_id, 5)]);
    let npc_def = NpcDefinition::merchant("merchant_bob", "Bob", "bob.png");

    // Start with a freshly initialised game state
    let mut game = GameState::new();
    game.party.gold = 100;

    let mut character = make_character("BuyHero");

    // ── Act: purchase ────────────────────────────────────────────────────────
    let slot = buy_item(
        &mut game.party,
        &mut character,
        0,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    )
    .expect("buy_item must succeed when party has sufficient gold and inventory has space");

    // ── Assert: post-purchase state ──────────────────────────────────────────
    assert_eq!(
        slot.item_id, item_id,
        "returned slot must reference the purchased item"
    );
    assert!(
        game.party.gold < 100,
        "gold must be deducted after purchase (was 100, now {})",
        game.party.gold
    );
    assert_eq!(game.party.gold, 90, "gold should be exactly 90 (100 - 10)");

    assert_eq!(
        character.inventory.items.len(),
        1,
        "character inventory must contain exactly one item after purchase"
    );
    assert_eq!(
        character.inventory.items[0].item_id, item_id,
        "the item in inventory must be the one that was purchased"
    );

    // Stock must be decremented (5 → 4)
    let remaining = npc_runtime
        .stock
        .as_ref()
        .unwrap()
        .get_entry(item_id)
        .unwrap()
        .quantity;
    assert_eq!(
        remaining, 4,
        "stock must decrement from 5 to 4 after one purchase"
    );

    // ── Act: save then load ──────────────────────────────────────────────────
    // Persist the NPC runtime into game state before saving so the round-trip
    // covers the full data path.
    game.npc_runtime.insert(npc_runtime.clone());
    game.party
        .add_member(character)
        .expect("character must be addable to the party");

    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager.save("buy_flow_test", &game).unwrap();

    let loaded = manager.load("buy_flow_test").unwrap();

    // ── Assert: loaded state matches ─────────────────────────────────────────
    assert_eq!(
        loaded.party.gold, 90,
        "gold must be 90 after round-trip through save/load"
    );
    assert_eq!(
        loaded.party.members.len(),
        1,
        "party must still have one member after round-trip"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items.len(),
        1,
        "character inventory must survive the round-trip"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items[0].item_id, item_id,
        "item ID must survive the save/load round-trip"
    );

    let loaded_runtime = loaded
        .npc_runtime
        .get(&"merchant_bob".to_string())
        .expect("merchant_bob runtime must be present after load");

    assert_eq!(
        loaded_runtime
            .stock
            .as_ref()
            .unwrap()
            .get_entry(item_id)
            .unwrap()
            .quantity,
        4,
        "decremented stock quantity must survive the save/load round-trip"
    );
}

// ============================================================================
// 6.1  Merchant Sell Flow – End-to-End
// ============================================================================

/// Full sell path: add item to character inventory manually, call `sell_item`,
/// verify item is removed and party gold increased.
#[test]
fn test_merchant_sell_flow_end_to_end() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let item_id: ItemId = 2;
    let base_cost: u32 = 20;
    let sell_cost: u32 = 10;

    let item_db = item_db_with(vec![make_test_item(item_id, base_cost, sell_cost)]);

    // Merchant has no pre-existing stock for this item so the back-to-stock
    // path in sell_item is not triggered (entry must already exist for that).
    let mut npc_runtime = NpcRuntimeState::new("merchant_carol".to_string());
    let npc_def = NpcDefinition::merchant("merchant_carol", "Carol", "carol.png");

    let mut game = GameState::new();
    game.party.gold = 0;

    let mut character = make_character("Seller");
    // Manually place the item in inventory (simulates having obtained it previously)
    character
        .inventory
        .add_item(item_id, 0)
        .expect("adding item to empty inventory must succeed");

    assert_eq!(
        character.inventory.items.len(),
        1,
        "precondition: character must start with 1 item"
    );

    // ── Act ──────────────────────────────────────────────────────────────────
    let gold_received = sell_item(
        &mut game.party,
        &mut character,
        0,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    )
    .expect("sell_item must succeed when item is in inventory");

    // ── Assert ───────────────────────────────────────────────────────────────
    assert!(
        gold_received >= 1,
        "sell price must be at least 1 gold (got {})",
        gold_received
    );
    assert_eq!(
        game.party.gold, gold_received,
        "party gold must equal the gold received from the sale"
    );
    assert!(
        game.party.gold > 0,
        "party gold must increase after selling an item worth {} base cost",
        base_cost
    );
    assert_eq!(
        character.inventory.items.len(),
        0,
        "item must be removed from inventory after selling"
    );
}

// ============================================================================
// 6.1  Buy Respects Inventory Limit
// ============================================================================

/// Fill a character's inventory to `Inventory::MAX_ITEMS`, then attempt a
/// purchase.  The transaction must return `TransactionError::InventoryFull`
/// and the party gold must remain unchanged.
#[test]
fn test_merchant_buy_respects_inventory_limit() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let item_id: ItemId = 3;
    let base_cost: u32 = 15;

    let item_db = item_db_with(vec![make_test_item(item_id, base_cost, 7)]);
    let mut npc_runtime = merchant_runtime("merchant_dan", vec![StockEntry::new(item_id, 10)]);
    let npc_def = NpcDefinition::merchant("merchant_dan", "Dan", "dan.png");

    let mut game = GameState::new();
    game.party.gold = 1_000; // More than enough gold

    let mut character = make_character("FullPack");

    // Fill inventory to the maximum using item IDs starting at 100 (well clear
    // of our test item IDs to avoid any accidental collision).
    for i in 0..Inventory::MAX_ITEMS {
        let filler_id: ItemId = (100 + i) as ItemId;
        character
            .inventory
            .add_item(filler_id, 0)
            .expect("filling inventory must not fail before MAX_ITEMS is reached");
    }

    assert!(
        character.inventory.is_full(),
        "precondition: inventory must be full before attempting buy"
    );

    let gold_before = game.party.gold;

    // ── Act ──────────────────────────────────────────────────────────────────
    let result = buy_item(
        &mut game.party,
        &mut character,
        0,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    );

    // ── Assert ───────────────────────────────────────────────────────────────
    assert!(
        result.is_err(),
        "buy_item must fail when character inventory is full"
    );
    assert!(
        matches!(result, Err(TransactionError::InventoryFull { .. })),
        "error must be InventoryFull, got: {:?}",
        result
    );
    assert_eq!(
        game.party.gold, gold_before,
        "party gold must be unchanged when the purchase fails due to full inventory"
    );
    assert_eq!(
        character.inventory.items.len(),
        Inventory::MAX_ITEMS,
        "inventory must still contain exactly MAX_ITEMS items after the failed buy"
    );

    // Stock must NOT have been decremented
    let remaining = npc_runtime
        .stock
        .as_ref()
        .unwrap()
        .get_entry(item_id)
        .unwrap()
        .quantity;
    assert_eq!(remaining, 10, "stock must not decrement when buy fails");
}

// ============================================================================
// 6.1  Stock Depletes After Buy
// ============================================================================

/// Buy all units of a single item, then attempt to buy one more.  The second
/// purchase must return `TransactionError::OutOfStock`.
#[test]
fn test_merchant_stock_depletes_after_buy() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let item_id: ItemId = 4;
    let base_cost: u32 = 8;
    let initial_quantity: u8 = 1; // Only one in stock so a single buy drains it

    let item_db = item_db_with(vec![make_test_item(item_id, base_cost, 4)]);
    let mut npc_runtime = merchant_runtime(
        "merchant_eve",
        vec![StockEntry::new(item_id, initial_quantity)],
    );
    let npc_def = NpcDefinition::merchant("merchant_eve", "Eve", "eve.png");

    let mut game = GameState::new();
    game.party.gold = 500;

    let mut character = make_character("Shopper");

    // ── Act: first buy (should succeed) ──────────────────────────────────────
    let first_buy = buy_item(
        &mut game.party,
        &mut character,
        0,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    );
    assert!(
        first_buy.is_ok(),
        "first buy must succeed when stock > 0: {:?}",
        first_buy
    );

    // Verify stock is now zero
    let qty = npc_runtime
        .stock
        .as_ref()
        .unwrap()
        .get_entry(item_id)
        .unwrap()
        .quantity;
    assert_eq!(qty, 0, "stock must be zero after purchasing the last unit");

    // ── Act: second buy on depleted stock ────────────────────────────────────
    let mut character2 = make_character("Shopper2");

    let second_buy = buy_item(
        &mut game.party,
        &mut character2,
        1,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    );

    // ── Assert ───────────────────────────────────────────────────────────────
    assert!(
        second_buy.is_err(),
        "second buy must fail when stock is exhausted"
    );
    assert!(
        matches!(second_buy, Err(TransactionError::OutOfStock { .. })),
        "error must be OutOfStock, got: {:?}",
        second_buy
    );
    assert_eq!(
        character2.inventory.items.len(),
        0,
        "second character's inventory must be empty after the failed buy"
    );
}

// ============================================================================
// 6.1  Stock Depletion Persists After Save/Load
// ============================================================================

/// Buy one item from a merchant, save the game, reload it, and verify that the
/// NPC's runtime stock quantity is still decremented in the loaded state.
#[test]
fn test_merchant_stock_persists_depletion_after_save_load() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let item_id: ItemId = 5;
    let base_cost: u32 = 25;
    let initial_quantity: u8 = 5;

    let item_db = item_db_with(vec![make_test_item(item_id, base_cost, 12)]);
    let mut npc_runtime = merchant_runtime(
        "merchant_frank",
        vec![StockEntry::new(item_id, initial_quantity)],
    );
    let npc_def = NpcDefinition::merchant("merchant_frank", "Frank", "frank.png");

    let mut game = GameState::new();
    game.party.gold = 500;

    let mut character = make_character("StockChecker");

    // ── Act: buy one item ────────────────────────────────────────────────────
    buy_item(
        &mut game.party,
        &mut character,
        0,
        &mut npc_runtime,
        &npc_def,
        item_id,
        &item_db,
    )
    .expect("buy must succeed");

    // Verify pre-save state: quantity decremented to 4
    let qty_before_save = npc_runtime
        .stock
        .as_ref()
        .unwrap()
        .get_entry(item_id)
        .unwrap()
        .quantity;
    assert_eq!(qty_before_save, 4, "stock must be 4 after one purchase");

    // ── Act: persist NPC runtime into GameState, then save + load ────────────
    game.npc_runtime.insert(npc_runtime);
    game.party
        .add_member(character)
        .expect("party must accept the character");

    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager.save("stock_persist_test", &game).unwrap();

    let loaded = manager.load("stock_persist_test").unwrap();

    // ── Assert: stock depletion present in loaded state ──────────────────────
    let loaded_runtime = loaded
        .npc_runtime
        .get(&"merchant_frank".to_string())
        .expect("merchant_frank runtime must be present in loaded state");

    assert!(
        loaded_runtime.stock.is_some(),
        "merchant stock must survive the save/load round-trip"
    );

    let qty_after_load = loaded_runtime
        .stock
        .as_ref()
        .unwrap()
        .get_entry(item_id)
        .unwrap()
        .quantity;

    assert_eq!(
        qty_after_load, 4,
        "stock depletion (5 → 4) must persist across the save/load round-trip; \
         got {} instead of 4",
        qty_after_load
    );

    // Sanity: the item is still in the party member's inventory after reload
    assert_eq!(
        loaded.party.members.len(),
        1,
        "party must still have one member after load"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items.len(),
        1,
        "character inventory must survive save/load with purchased item present"
    );
    assert_eq!(
        loaded.party.members[0].inventory.items[0].item_id, item_id,
        "item ID must be intact after save/load"
    );
}
