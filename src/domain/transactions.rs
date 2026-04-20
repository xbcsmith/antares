// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Transaction operations - pure domain functions for NPC commerce and services
//!
//! This module provides the pure domain-layer functions that enforce all business
//! rules for NPC interactions. All functions operate exclusively on domain types
//! (no Bevy, no I/O) and return `Result<T, TransactionError>` for explicit error
//! handling.
//!
//! # Operations
//!
//! - `buy_item` - Party purchases an item from a merchant
//! - `sell_item` - Party sells an item to a merchant
//! - `consume_service` - Party pays for a priest/innkeeper service
//! - `drop_item` - Character drops an item onto the game world map
//! - `pickup_item` - Character picks up a dropped item from the game world map
//! - `equip_item` - Character equips an item from inventory into an equipment slot
//! - `unequip_item` - Character unequips an item from a slot back to inventory
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.3 (Character/Party) and the
//! inventory system implementation plan for complete specifications.

use crate::domain::character::{Character, InventorySlot};
use crate::domain::character::{EquipmentSlot, Party};
use crate::domain::classes::ClassDatabase;
use crate::domain::inventory::ServiceCatalog;
use crate::domain::items::equipment_validation::{
    calculate_armor_class, can_equip_item, EquipError,
};
use crate::domain::items::ItemDatabase;
use crate::domain::races::RaceDatabase;
use crate::domain::types::{CharacterId, ItemId, MapId, Position};
use crate::domain::world::npc::NpcDefinition;
use crate::domain::world::npc_runtime::NpcRuntimeState;
use crate::domain::world::{DroppedItem, World};
use thiserror::Error;

// ===== TransactionError =====

/// Errors that can occur during NPC commerce or service transactions
///
/// Every variant carries enough context for the caller to construct a
/// human-readable error message or to distinguish error cases in tests.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::TransactionError;
///
/// let err = TransactionError::InsufficientGold { have: 10, need: 50 };
/// assert!(err.to_string().contains("10"));
/// assert!(err.to_string().contains("50"));
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    /// Party does not have enough gold to complete the transaction
    #[error("Insufficient gold: have {have}, need {need}")]
    InsufficientGold { have: u32, need: u32 },

    /// Party does not have enough gems to complete the transaction
    #[error("Insufficient gems: have {have}, need {need}")]
    InsufficientGems { have: u32, need: u32 },

    /// The target character's inventory is full and cannot accept another item
    #[error("Inventory full for character {character_id}")]
    InventoryFull { character_id: CharacterId },

    /// The requested item is not listed in the NPC's stock at all
    #[error("Item {item_id} is not in NPC stock")]
    ItemNotInStock { item_id: ItemId },

    /// The item is listed in NPC stock but has a quantity of zero
    #[error("Item {item_id} is out of stock")]
    OutOfStock { item_id: ItemId },

    /// The item is not present in the character's inventory
    #[error("Item {item_id} not found in inventory of character {character_id}")]
    ItemNotInInventory {
        item_id: ItemId,
        character_id: CharacterId,
    },

    /// No NPC with the given ID was found in the runtime store
    #[error("NPC not found: {npc_id}")]
    NpcNotFound { npc_id: String },

    /// No service with the given ID exists in the service catalog
    #[error("Service not found: {service_id}")]
    ServiceNotFound { service_id: String },

    /// No character with the given ID was found
    #[error("Character not found: {character_id}")]
    CharacterNotFound { character_id: CharacterId },

    /// A quantity argument was zero or otherwise invalid
    #[error("Invalid quantity")]
    InvalidQuantity,

    /// The map with the given ID was not found in the world
    #[error("Map {map_id} not found in world")]
    MapNotFound { map_id: MapId },
}

// ===== ServiceOutcome =====

/// Result returned by a successful `consume_service` call
///
/// Provides a summary of what was deducted and which characters were affected,
/// so the caller can display feedback or write to a log.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::ServiceOutcome;
///
/// let outcome = ServiceOutcome {
///     service_id: "heal_all".to_string(),
///     gold_paid: 50,
///     gems_paid: 0,
///     characters_affected: vec![0, 1],
/// };
///
/// assert_eq!(outcome.gold_paid, 50);
/// assert_eq!(outcome.characters_affected.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOutcome {
    /// The service that was consumed
    pub service_id: String,
    /// Gold deducted from the party
    pub gold_paid: u32,
    /// Gems deducted from the party
    pub gems_paid: u32,
    /// IDs of characters whose state was modified by the service
    pub characters_affected: Vec<CharacterId>,
}

// ===== buy_item =====

/// Purchase an item from a merchant NPC
///
/// Enforces all purchase preconditions in the following order:
/// 1. Item exists in the `ItemDatabase`
/// 2. Item is listed in the NPC's current runtime stock
/// 3. Stock quantity is greater than zero
/// 4. Party has enough gold (price = effective_price × sell_rate, rounded)
/// 5. Character's inventory has a free slot
///
/// On success, gold is deducted from the party, stock is decremented, and the
/// item (with its `max_charges`) is added to the character's inventory.
///
/// # Arguments
///
/// * `party` - The active party (gold is deducted from here)
/// * `character` - The character receiving the item
/// * `character_id` - Roster index of `character` (used in error messages)
/// * `npc_runtime` - Mutable runtime state of the merchant NPC
/// * `npc_def` - Static NPC definition (provides economy settings)
/// * `item_id` - The item the player wants to buy
/// * `item_db` - Item database for looking up item metadata
///
/// # Returns
///
/// Returns `Ok(InventorySlot)` – the slot that was added to the inventory.
///
/// # Errors
///
/// Returns `TransactionError::ItemNotInStock` if the item is not in the NPC's
/// stock or if the item does not exist in the item database.
/// Returns `TransactionError::OutOfStock` if quantity is zero.
/// Returns `TransactionError::InsufficientGold` if party cannot afford the item.
/// Returns `TransactionError::InventoryFull` if the character has no free slot.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::{buy_item, TransactionError};
/// use antares::domain::character::{Character, Alignment, Party, Sex};
/// use antares::domain::world::npc::NpcDefinition;
/// use antares::domain::world::npc_runtime::NpcRuntimeState;
/// use antares::domain::inventory::{MerchantStock, StockEntry};
/// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
///
/// // Build a minimal item database
/// let mut item_db = ItemDatabase::new();
/// let club = Item {
///     id: 1,
///     name: "Club".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 3, 0),
///         bonus: 0,
///         hands_required: 1,
///         classification: WeaponClassification::Simple,
///     }),
///     base_cost: 10,
///     sell_cost: 5,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
/// };
/// item_db.add_item(club).unwrap();
///
/// // Build NPC runtime with stock
/// let mut npc_runtime = NpcRuntimeState::new("merchant_tom".to_string());
/// npc_runtime.stock = Some(MerchantStock {
///     entries: vec![StockEntry::new(1, 3)],
///     restock_template: None,
/// });
///
/// let npc_def = NpcDefinition::merchant("merchant_tom", "Tom", "tom.png");
///
/// let mut party = Party::new();
/// party.gold = 100;
///
/// let mut character = Character::new(
///     "Adventurer".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// let slot = buy_item(&mut party, &mut character, 0, &mut npc_runtime, &npc_def, 1, &item_db)
///     .expect("purchase must succeed");
///
/// assert_eq!(slot.item_id, 1);
/// assert_eq!(party.gold, 90); // 100 - 10
/// ```
pub fn buy_item(
    party: &mut Party,
    character: &mut Character,
    character_id: CharacterId,
    npc_runtime: &mut NpcRuntimeState,
    npc_def: &NpcDefinition,
    item_id: ItemId,
    item_db: &ItemDatabase,
) -> Result<InventorySlot, TransactionError> {
    // Step 1: Look up item in the database
    let item = item_db
        .get_item(item_id)
        .ok_or(TransactionError::ItemNotInStock { item_id })?;

    // Step 2: Look up entry in NPC runtime stock
    let stock = npc_runtime
        .stock
        .as_ref()
        .and_then(|s| s.get_entry(item_id))
        .ok_or(TransactionError::ItemNotInStock { item_id })?;

    // Step 3: Check quantity
    if stock.quantity == 0 {
        return Err(TransactionError::OutOfStock { item_id });
    }

    // Step 4: Compute price applying sell_rate
    let base_price = npc_runtime
        .stock
        .as_ref()
        .map(|s| s.effective_price(item_id, item.base_cost))
        .unwrap_or(item.base_cost);

    let price = if let Some(economy) = &npc_def.economy {
        // sell_rate is the multiplier the merchant charges the player when selling
        let raw = base_price as f64 * economy.sell_rate as f64;
        raw.round() as u32
    } else {
        base_price
    };

    // Step 5: Check party gold
    if party.gold < price {
        return Err(TransactionError::InsufficientGold {
            have: party.gold,
            need: price,
        });
    }

    // Step 6: Check inventory space
    if character.inventory.is_full() {
        return Err(TransactionError::InventoryFull { character_id });
    }

    // Step 7: Deduct gold
    party.gold = party.gold.saturating_sub(price);

    // Step 8: Decrement NPC stock
    if let Some(stock) = npc_runtime.stock.as_mut() {
        stock.decrement(item_id);
    }

    // Step 9 & 10: Build slot and add to inventory
    // max_charges is u16 in the item definition; InventorySlot stores charges as u8,
    // so we clamp to u8::MAX (255) which covers all practical charge values.
    let charges = item.max_charges.min(u8::MAX as u16) as u8;
    character
        .inventory
        .add_item(item_id, charges)
        .expect("inventory has space – checked above");

    // Return the slot that was added
    let slot = InventorySlot { item_id, charges };
    Ok(slot)
}

// ===== sell_item =====

/// Sell an item from a character's inventory to a merchant NPC
///
/// Enforces all sell preconditions in the following order:
/// 1. Item is present in the character's inventory
/// 2. Item exists in the `ItemDatabase`
/// 3. Computes sell price (uses `item.sell_cost` if non-zero, otherwise
///    `item.base_cost / 2`), then applies `npc_def.economy.buy_rate` (default
///    0.5), rounded down, minimum 1 gold.
/// 4. Removes the item from the inventory
/// 5. Adds sell price to party gold (clamped to `u16::MAX`)
/// 6. Optionally adds item back to NPC stock
///
/// # Arguments
///
/// * `party` - The active party (gold is added here)
/// * `character` - The character selling the item
/// * `character_id` - Roster index of `character` (used in error messages)
/// * `npc_runtime` - Mutable runtime state of the merchant NPC
/// * `npc_def` - Static NPC definition (provides economy settings)
/// * `item_id` - The item the player wants to sell
/// * `item_db` - Item database for looking up item metadata
///
/// # Returns
///
/// Returns `Ok(u32)` – the gold amount added to the party.
///
/// # Errors
///
/// Returns `TransactionError::ItemNotInInventory` if the item is not in the
/// character's inventory.
/// Returns `TransactionError::ItemNotInStock` if the item does not exist in
/// the item database.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::{sell_item, TransactionError};
/// use antares::domain::character::{Character, Party, Sex, Alignment};
/// use antares::domain::world::npc::NpcDefinition;
/// use antares::domain::world::npc_runtime::NpcRuntimeState;
/// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
///
/// let mut item_db = ItemDatabase::new();
/// let sword = Item {
///     id: 2,
///     name: "Sword".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 8, 0),
///         bonus: 0,
///         hands_required: 1,
///         classification: WeaponClassification::MartialMelee,
///     }),
///     base_cost: 20,
///     sell_cost: 10,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
/// };
/// item_db.add_item(sword).unwrap();
///
/// let mut npc_runtime = NpcRuntimeState::new("merchant_tom".to_string());
/// let npc_def = NpcDefinition::merchant("merchant_tom", "Tom", "tom.png");
///
/// let mut party = Party::new();
/// party.gold = 0;
///
/// let mut character = Character::new(
///     "Adventurer".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.inventory.add_item(2, 0).unwrap();
///
/// let gold_received = sell_item(&mut party, &mut character, 0, &mut npc_runtime, &npc_def, 2, &item_db)
///     .expect("sell must succeed");
///
/// assert!(gold_received >= 1);
/// assert_eq!(character.inventory.items.len(), 0);
/// ```
pub fn sell_item(
    party: &mut Party,
    character: &mut Character,
    character_id: CharacterId,
    npc_runtime: &mut NpcRuntimeState,
    npc_def: &NpcDefinition,
    item_id: ItemId,
    item_db: &ItemDatabase,
) -> Result<u32, TransactionError> {
    // Step 1: Find slot index in character inventory
    let slot_index = character
        .inventory
        .items
        .iter()
        .position(|s| s.item_id == item_id)
        .ok_or(TransactionError::ItemNotInInventory {
            item_id,
            character_id,
        })?;

    // Step 2: Look up item in database
    let item = item_db
        .get_item(item_id)
        .ok_or(TransactionError::ItemNotInStock { item_id })?;

    // Step 3: Compute sell price
    // Base: use sell_cost if non-zero, otherwise base_cost / 2
    let base_sell = if item.sell_cost > 0 {
        item.sell_cost
    } else {
        item.base_cost / 2
    };

    // Apply buy_rate (what the merchant pays; default 0.5 if no economy settings)
    let sell_price = if let Some(economy) = &npc_def.economy {
        let raw = base_sell as f64 * economy.buy_rate as f64;
        (raw.floor() as u32).max(1)
    } else {
        // Default: merchant buys at 50% of base sell
        let raw = base_sell as f64 * 0.5_f64;
        (raw.floor() as u32).max(1)
    };

    // Step 4: Remove slot from inventory
    character.inventory.remove_item(slot_index);

    // Step 5: Add gold to party (clamped to u32::MAX via saturating_add)
    party.gold = party.gold.saturating_add(sell_price);

    // Step 6: Optionally add item back to NPC stock (only if NPC has existing entry)
    if let Some(stock) = npc_runtime.stock.as_mut() {
        if let Some(entry) = stock.get_entry_mut(item_id) {
            entry.quantity = entry.quantity.saturating_add(1);
        }
    }

    Ok(sell_price)
}

// ===== consume_service =====

/// Consume a paid service from a priest or innkeeper NPC
///
/// Enforces all service preconditions in the following order:
/// 1. Service exists in `service_catalog`
/// 2. Party has enough gold
/// 3. Party has enough gems
/// 4. Gold and gems are deducted
/// 5. Service effect is applied to each character in `targets`
/// 6. `service_id` is recorded in `npc_runtime.services_consumed`
///
/// # Service Effects
///
/// | `service_id`     | Effect applied to each target character        |
/// |------------------|------------------------------------------------|
/// | `"heal_all"` / `"heal"` | `hp.current = hp.base`                |
/// | `"restore_sp"`   | `sp.current = sp.base`                        |
/// | `"cure_poison"`  | removes `Condition::POISONED`                 |
/// | `"cure_disease"` | removes `Condition::DISEASED`                 |
/// | `"cure_all"`     | clears all conditions                          |
/// | `"resurrect"`    | clears all conditions, sets `hp.current = 1`  |
/// | `"rest"`         | restores HP, SP, and clears all conditions    |
/// | other            | no-op (character unaffected; no error)        |
///
/// # Arguments
///
/// * `party` - The active party (gold and gems are deducted here)
/// * `targets` - Mutable references to characters the service will affect
/// * `npc_runtime` - Mutable runtime state of the NPC (records consumed service)
/// * `service_catalog` - Catalog defining available services and their costs
/// * `service_id` - Which service to consume
///
/// # Returns
///
/// Returns `Ok(ServiceOutcome)` with payment and affected character details.
///
/// # Errors
///
/// Returns `TransactionError::ServiceNotFound` if `service_id` is not in the catalog.
/// Returns `TransactionError::InsufficientGold` if party cannot pay the gold cost.
/// Returns `TransactionError::InsufficientGems` if party cannot pay the gem cost.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::consume_service;
/// use antares::domain::character::{Character, Party, Sex, Alignment};
/// use antares::domain::world::npc_runtime::NpcRuntimeState;
/// use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
///
/// let mut party = Party::new();
/// party.gold = 100;
///
/// let mut character = Character::new(
///     "Adventurer".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.hp.base = 30;
/// character.hp.current = 10;
///
/// let mut npc_runtime = NpcRuntimeState::new("high_priest".to_string());
///
/// let mut catalog = ServiceCatalog::new();
/// catalog.services.push(ServiceEntry::new("heal_all", 50, "Full heal"));
///
/// let outcome = consume_service(
///     &mut party,
///     &mut vec![&mut character],
///     &mut npc_runtime,
///     &catalog,
///     "heal_all",
/// )
/// .expect("service must succeed");
///
/// assert_eq!(outcome.gold_paid, 50);
/// assert_eq!(party.gold, 50);
/// assert_eq!(character.hp.current, 30);
/// ```
pub fn consume_service(
    party: &mut Party,
    targets: &mut Vec<&mut Character>,
    npc_runtime: &mut NpcRuntimeState,
    service_catalog: &ServiceCatalog,
    service_id: &str,
) -> Result<ServiceOutcome, TransactionError> {
    // Step 1: Look up service entry
    let entry = service_catalog
        .get_service(service_id)
        .ok_or_else(|| TransactionError::ServiceNotFound {
            service_id: service_id.to_string(),
        })?
        .clone();

    // Step 2: Check gold
    if party.gold < entry.cost {
        return Err(TransactionError::InsufficientGold {
            have: party.gold,
            need: entry.cost,
        });
    }

    // Step 3: Check gems
    if party.gems < entry.gem_cost {
        return Err(TransactionError::InsufficientGems {
            have: party.gems,
            need: entry.gem_cost,
        });
    }

    // Step 4: Deduct gold
    party.gold = party.gold.saturating_sub(entry.cost);

    // Step 5: Deduct gems
    party.gems = party.gems.saturating_sub(entry.gem_cost);

    // Step 6: Apply service effect to each target
    let mut characters_affected = Vec::new();
    for character in targets.iter_mut() {
        apply_service_effect(character, service_id);
        characters_affected.push(character_roster_id(character));
    }

    // Step 7: Record consumed service
    npc_runtime.services_consumed.push(service_id.to_string());

    Ok(ServiceOutcome {
        service_id: service_id.to_string(),
        gold_paid: entry.cost,
        gems_paid: entry.gem_cost,
        characters_affected,
    })
}

// ===== Helpers =====

/// Applies a service effect to a single character based on the service ID
///
/// Unrecognised service IDs are silently ignored (no error, no effect).
fn apply_service_effect(character: &mut Character, service_id: &str) {
    use crate::domain::character::Condition;

    match service_id {
        "heal_all" | "heal" => {
            character.hp.current = character.hp.base;
        }
        "restore_sp" => {
            character.sp.current = character.sp.base;
        }
        "cure_poison" => {
            character.conditions.remove(Condition::POISONED);
        }
        "cure_disease" => {
            character.conditions.remove(Condition::DISEASED);
        }
        "cure_all" => {
            character.conditions.clear();
            character.active_conditions.clear();
        }
        "resurrect" => {
            character.conditions.clear();
            character.active_conditions.clear();
            character.hp.current = 1;
        }
        "rest" => {
            character.hp.current = character.hp.base;
            character.sp.current = character.sp.base;
            character.conditions.clear();
            character.active_conditions.clear();
        }
        // Unrecognised service ID: no-op
        _ => {}
    }
}

/// Returns a stable CharacterId for a Character.
///
/// The `Character` struct does not carry its own roster index, so we use
/// a deterministic sentinel (0) when building `ServiceOutcome`. In practice
/// callers that need precise character IDs should track them externally and
/// construct `characters_affected` from the returned outcome if needed.
///
/// This helper is intentionally private and only used to satisfy the
/// `ServiceOutcome::characters_affected` field contract.
fn character_roster_id(_character: &Character) -> CharacterId {
    // Characters do not embed their own roster index; callers that require
    // precise per-character IDs should map results from the slice index.
    0
}

// ===== drop_item =====

/// Drop an item from a character's inventory onto the game world map.
///
/// Removes the item at `slot_index` from `character.inventory`, creates a
/// [`DroppedItem`] record at `position` on the given `map_id`, appends it to
/// the map's `dropped_items` list, and returns the created record.
///
/// # Arguments
///
/// * `character`     – The character dropping the item.
/// * `character_id`  – Roster/party index of `character` (used in error messages).
/// * `slot_index`    – Zero-based index of the inventory slot to drop.
/// * `world`         – Mutable reference to the game world (map lookup target).
/// * `map_id`        – ID of the map on which the item is being dropped.
/// * `position`      – Tile coordinate where the item lands.
///
/// # Returns
///
/// Returns `Ok(DroppedItem)` – the record that was added to the map.
///
/// # Errors
///
/// Returns [`TransactionError::ItemNotInInventory`] if `slot_index` is out of
/// bounds for `character.inventory`.
/// Returns [`TransactionError::MapNotFound`] if no map with `map_id` exists in
/// `world`.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::drop_item;
/// use antares::domain::character::{Character, Alignment, Sex};
/// use antares::domain::world::{World, Map};
/// use antares::domain::types::Position;
///
/// let mut world = World::new();
/// let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 20, 20);
/// world.add_map(map);
/// world.set_current_map(1);
///
/// let mut character = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// character.inventory.add_item(5, 3).unwrap();
///
/// let pos = Position::new(10, 10);
/// let dropped = drop_item(&mut character, 0, 0, &mut world, 1, pos)
///     .expect("drop must succeed");
///
/// assert_eq!(dropped.item_id, 5);
/// assert_eq!(dropped.charges, 3);
/// assert!(character.inventory.items.is_empty());
/// assert_eq!(world.get_map(1).unwrap().dropped_items.len(), 1);
/// ```
pub fn drop_item(
    character: &mut Character,
    character_id: CharacterId,
    slot_index: usize,
    world: &mut World,
    map_id: MapId,
    position: Position,
) -> Result<DroppedItem, TransactionError> {
    // Step 1: Bounds-check slot index without yet removing the item.
    // We must confirm both the slot and the map are valid before mutating
    // either, so that a MapNotFound error does not cause item loss.
    let item_id_check = character
        .inventory
        .items
        .get(slot_index)
        .map(|s| s.item_id)
        .ok_or(TransactionError::ItemNotInInventory {
            item_id: 0,
            character_id,
        })?;

    // Step 2: Verify the target map exists before touching the inventory.
    if world.get_map(map_id).is_none() {
        return Err(TransactionError::MapNotFound { map_id });
    }

    // Step 3: Remove the item — safe because bounds were verified in step 1.
    // Use `remove_item` which returns `Option`; treat `None` as a logic error.
    let slot = character
        .inventory
        .remove_item(slot_index)
        .expect("slot must exist after bounds check");

    debug_assert_eq!(
        slot.item_id, item_id_check,
        "item_id changed between bounds check and removal"
    );

    // Step 4: Build and persist the dropped-item record.
    let dropped = DroppedItem {
        item_id: slot.item_id,
        charges: slot.charges,
        position,
        map_id,
    };
    // SAFETY: map existence was confirmed in step 2; no interleaving mutations.
    world
        .get_map_mut(map_id)
        .expect("map must exist after pre-check")
        .add_dropped_item(dropped.clone());

    Ok(dropped)
}

// ===== pickup_item =====

/// Pick up a dropped item from the game world map into a character's inventory.
///
/// Verifies that `character.inventory` has space, removes the first
/// [`DroppedItem`] at `position` with matching `item_id` from the map's
/// `dropped_items` list, and adds it to the character's inventory.
///
/// # Arguments
///
/// * `character`    – The character picking up the item.
/// * `character_id` – Roster/party index of `character` (used in error messages).
/// * `world`        – Mutable reference to the game world (map lookup target).
/// * `map_id`       – ID of the map from which the item is being picked up.
/// * `position`     – Tile coordinate where the item lies.
/// * `item_id`      – Logical item identifier to pick up.
///
/// # Returns
///
/// Returns `Ok(InventorySlot)` – the slot that was added to the character's
/// inventory, carrying the same `item_id` and `charges` as the dropped record.
///
/// # Errors
///
/// Returns [`TransactionError::InventoryFull`] if `character.inventory` has no
/// free slot.
/// Returns [`TransactionError::MapNotFound`] if no map with `map_id` exists in
/// `world`.
/// Returns [`TransactionError::ItemNotInInventory`] if no dropped item matching
/// both `position` and `item_id` is found on the map.
///
/// # Examples
///
/// ```
/// use antares::domain::transactions::{drop_item, pickup_item};
/// use antares::domain::character::{Character, Alignment, Sex};
/// use antares::domain::world::{World, Map};
/// use antares::domain::types::Position;
///
/// let mut world = World::new();
/// let map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 20, 20);
/// world.add_map(map);
/// world.set_current_map(1);
///
/// let pos = Position::new(5, 5);
///
/// let mut dropper = Character::new(
///     "Dropper".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// dropper.inventory.add_item(7, 2).unwrap();
/// drop_item(&mut dropper, 0, 0, &mut world, 1, pos).unwrap();
///
/// let mut picker = Character::new(
///     "Picker".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
///
/// let slot = pickup_item(&mut picker, 1, &mut world, 1, pos, 7)
///     .expect("pickup must succeed");
///
/// assert_eq!(slot.item_id, 7);
/// assert_eq!(slot.charges, 2);
/// assert!(world.get_map(1).unwrap().dropped_items.is_empty());
/// ```
pub fn pickup_item(
    character: &mut Character,
    character_id: CharacterId,
    world: &mut World,
    map_id: MapId,
    position: Position,
    item_id: ItemId,
) -> Result<InventorySlot, TransactionError> {
    // Step 1: Verify inventory has space before mutating the world.
    if character.inventory.is_full() {
        return Err(TransactionError::InventoryFull { character_id });
    }

    // Step 2: Look up the target map.
    let map = world
        .get_map_mut(map_id)
        .ok_or(TransactionError::MapNotFound { map_id })?;

    // Step 3: Remove the dropped-item record from the map (FIFO for stacked items).
    let dropped =
        map.remove_dropped_item(position, item_id)
            .ok_or(TransactionError::ItemNotInInventory {
                item_id,
                character_id,
            })?;

    // Step 4: Add item to the character's inventory.
    // is_full was already checked, so add_item should not fail; if it somehow
    // does (e.g. MAX_ITEMS changed between check and add), the dropped item
    // record has already been removed — we re-insert it to avoid item loss.
    if let Err(_inventory_err) = character
        .inventory
        .add_item(dropped.item_id, dropped.charges)
    {
        // Rollback: re-insert the dropped item so it is not lost.
        if let Some(m) = world.get_map_mut(map_id) {
            m.add_dropped_item(dropped.clone());
        }
        return Err(TransactionError::InventoryFull { character_id });
    }

    Ok(InventorySlot {
        item_id: dropped.item_id,
        charges: dropped.charges,
    })
}

// ===== equip_item =====

/// Equip an item from a character's inventory into the appropriate equipment slot.
///
/// The target slot is determined automatically from the item's type and
/// classification.  If the slot already contains an item, the old item is
/// moved back to inventory (swap behaviour).  All mutations are atomic: if
/// any step fails the character state is left unchanged.
///
/// # Arguments
///
/// * `character`            – The character equipping the item.
/// * `inventory_slot_index` – Index into `character.inventory.items`.
/// * `item_db`              – Item database for look-up and AC recalculation.
/// * `classes`              – Class database for proficiency validation.
/// * `races`                – Race database for race-restriction validation.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// * [`EquipError::ItemNotFound`]        – `inventory_slot_index` is out of bounds.
/// * [`EquipError::ClassRestriction`]    – Character class lacks required proficiency.
/// * [`EquipError::RaceRestriction`]     – Character race has an incompatible item tag.
/// * [`EquipError::AlignmentRestriction`]– Character alignment cannot use the item.
/// * [`EquipError::NoSlotAvailable`]     – Item type is not equippable.
pub fn equip_item(
    character: &mut Character,
    inventory_slot_index: usize,
    item_db: &ItemDatabase,
    classes: &ClassDatabase,
    races: &RaceDatabase,
) -> Result<(), EquipError> {
    // Step 1: Bounds-check the inventory slot index.
    let item_id = character
        .inventory
        .items
        .get(inventory_slot_index)
        .map(|s| s.item_id)
        .ok_or(EquipError::ItemNotFound(0))?;

    // Step 2: Validate proficiency, race, alignment, and slot availability.
    can_equip_item(character, item_id, item_db, classes, races)?;

    // Step 3: Look up the item (guaranteed to exist — can_equip_item verified it).
    let item = item_db
        .get_item(item_id)
        .expect("item must exist in db after can_equip_item succeeded");

    // Step 4: Determine the target equipment slot.
    let target_slot =
        EquipmentSlot::for_item(item, &character.equipment).ok_or(EquipError::NoSlotAvailable)?;

    // Step 5: Record any item currently occupying the target slot (for swap).
    let displaced_id = target_slot.get(&character.equipment);

    // Step 6: Remove the item from inventory.
    let removed_slot = character
        .inventory
        .remove_item(inventory_slot_index)
        .expect("slot must exist after bounds-check in step 1");

    debug_assert_eq!(
        removed_slot.item_id, item_id,
        "item_id changed between bounds check and removal"
    );

    // Step 7: Place the item in the equipment slot.
    target_slot.set(&mut character.equipment, Some(item_id));

    // Step 8: Move the displaced item (if any) back to inventory.
    // Because a slot was freed in step 6, add_item should always succeed.
    if let Some(old_id) = displaced_id {
        if character.inventory.add_item(old_id, 0).is_err() {
            // Rollback: undo steps 7 and 6.
            target_slot.set(&mut character.equipment, displaced_id);
            character.inventory.items.insert(
                inventory_slot_index.min(character.inventory.items.len()),
                removed_slot,
            );
            return Err(EquipError::NoSlotAvailable);
        }
    }

    // Step 9: Recalculate AC when an armour item was equipped.
    use crate::domain::items::types::ItemType;
    if matches!(&item.item_type, ItemType::Armor(_)) {
        character.ac.current = calculate_armor_class(&character.equipment, item_db);
    }

    Ok(())
}

// ===== unequip_item =====

/// Unequip an item from a specific equipment slot and return it to inventory.
///
/// If the slot is already empty, returns `Ok(())` without modifying any state.
/// If the unequipped item is armour, `character.ac.current` is recalculated.
///
/// # Arguments
///
/// * `character` – The character unequipping the item.
/// * `slot`      – Which equipment slot to clear.
/// * `item_db`   – Item database used for AC recalculation.
///
/// # Returns
///
/// `Ok(())` on success or when the slot was already empty.
///
/// # Errors
///
/// * [`TransactionError::InventoryFull`] – No inventory space for the unequipped item.
pub fn unequip_item(
    character: &mut Character,
    slot: EquipmentSlot,
    item_db: &ItemDatabase,
) -> Result<(), TransactionError> {
    // Step 1: Nothing to do when the slot is already empty.
    let item_id = match slot.get(&character.equipment) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Step 2: Ensure inventory has space before mutating anything.
    if character.inventory.is_full() {
        return Err(TransactionError::InventoryFull { character_id: 0 });
    }

    // Step 3: Clear the equipment slot.
    slot.set(&mut character.equipment, None);

    // Step 4: Add the item to inventory.
    // Cannot fail: we verified !is_full() in step 2 and just cleared a slot.
    character
        .inventory
        .add_item(item_id, 0)
        .expect("inventory must accept item after is_full check");

    // Step 5: Recalculate AC if an armour item was removed.
    use crate::domain::items::types::ItemType;
    if let Some(item) = item_db.get_item(item_id) {
        if matches!(&item.item_type, ItemType::Armor(_)) {
            character.ac.current = calculate_armor_class(&character.equipment, item_db);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Condition, EquipmentSlot, Inventory, Sex};
    use crate::domain::classes::{ClassDatabase, ClassDefinition};
    use crate::domain::inventory::{MerchantStock, ServiceCatalog, ServiceEntry, StockEntry};
    use crate::domain::items::{
        ArmorClassification, ArmorData, ConsumableData, ConsumableEffect, EquipError, Item,
        ItemType, WeaponClassification, WeaponData,
    };
    use crate::domain::races::{RaceDatabase, RaceDefinition};
    use crate::domain::types::DiceRoll;
    use crate::domain::world::npc::NpcDefinition;
    use crate::domain::world::npc_runtime::NpcRuntimeState;
    use crate::domain::world::Map;

    // ===== Helpers =====

    fn make_item(id: ItemId, base_cost: u32, sell_cost: u32, max_charges: u16) -> Item {
        Item {
            id,
            name: format!("Item {id}"),
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
            max_charges,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    fn item_db_with(items: Vec<Item>) -> ItemDatabase {
        let mut db = ItemDatabase::new();
        for item in items {
            db.add_item(item).unwrap();
        }
        db
    }

    fn merchant_with_stock(npc_id: &str, entries: Vec<StockEntry>) -> NpcRuntimeState {
        let mut state = NpcRuntimeState::new(npc_id.to_string());
        state.stock = Some(MerchantStock {
            entries,
            restock_template: None,
        });
        state
    }

    fn basic_npc_def(id: &str) -> NpcDefinition {
        NpcDefinition::merchant(id, id, "portrait.png")
    }

    fn party_with_gold(gold: u32) -> Party {
        let mut p = Party::new();
        p.gold = gold;
        p
    }

    fn make_character() -> Character {
        Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    fn character_with_items(item_ids: Vec<ItemId>) -> Character {
        let mut c = make_character();
        for id in item_ids {
            c.inventory.add_item(id, 0).unwrap();
        }
        c
    }

    // ===== equip_item / unequip_item helpers =====

    /// Build a ClassDatabase containing a single class with the given proficiencies.
    /// The class id and the character's class_id must match (default: "knight").
    fn make_class_db_with_profs(class_id: &str, proficiencies: &[&str]) -> ClassDatabase {
        let mut db = ClassDatabase::new();
        db.add_class(ClassDefinition {
            id: class_id.to_string(),
            name: class_id.to_string(),
            description: String::new(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: proficiencies.iter().map(|s| s.to_string()).collect(),
        })
        .unwrap();
        db
    }

    /// Build a RaceDatabase containing a basic human race (no restrictions).
    fn make_race_db_human() -> RaceDatabase {
        let mut db = RaceDatabase::new();
        db.add_race(RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        ))
        .unwrap();
        db
    }

    /// Create a MartialMelee weapon item.
    fn make_weapon_item_tx(id: ItemId) -> Item {
        Item {
            id,
            name: format!("Sword {id}"),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 15,
            sell_cost: 7,
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

    /// Create an armor item with the given AC bonus and classification.
    fn make_armor_item_tx(id: ItemId, ac_bonus: u8, classification: ArmorClassification) -> Item {
        Item {
            id,
            name: format!("Armor {id}"),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus,
                weight: 10,
                classification,
            }),
            base_cost: 50,
            sell_cost: 25,
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

    /// Create a consumable item (not equippable).
    fn make_consumable_item_tx(id: ItemId) -> Item {
        Item {
            id,
            name: format!("Potion {id}"),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(10),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 1,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    // ===== buy_item tests =====

    #[test]
    fn test_buy_item_success() {
        let item_db = item_db_with(vec![make_item(1, 10, 5, 0)]);
        let mut npc_runtime = merchant_with_stock("seller", vec![StockEntry::new(1, 3)]);
        let npc_def = basic_npc_def("seller");

        let mut party = party_with_gold(100);
        let mut character = make_character();

        let slot = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            1,
            &item_db,
        )
        .expect("buy must succeed");

        // Gold decreased
        assert_eq!(party.gold, 90);
        // Item added to inventory
        assert_eq!(character.inventory.items.len(), 1);
        assert_eq!(character.inventory.items[0].item_id, 1);
        // Stock decremented
        assert_eq!(
            npc_runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            2
        );
        // Returned slot is correct
        assert_eq!(slot.item_id, 1);
    }

    #[test]
    fn test_buy_item_insufficient_gold() {
        let item_db = item_db_with(vec![make_item(1, 100, 50, 0)]);
        let mut npc_runtime = merchant_with_stock("seller", vec![StockEntry::new(1, 5)]);
        let npc_def = basic_npc_def("seller");

        let mut party = party_with_gold(5); // too poor
        let mut character = make_character();

        let result = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            1,
            &item_db,
        );

        assert!(matches!(
            result,
            Err(TransactionError::InsufficientGold { have: 5, need: 100 })
        ));
        // Party gold unchanged
        assert_eq!(party.gold, 5);
        // Inventory unchanged
        assert!(character.inventory.items.is_empty());
        // Stock unchanged
        assert_eq!(
            npc_runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            5
        );
    }

    #[test]
    fn test_buy_item_inventory_full() {
        let item_db = item_db_with(vec![make_item(1, 10, 5, 0)]);
        let mut npc_runtime = merchant_with_stock("seller", vec![StockEntry::new(1, 5)]);
        let npc_def = basic_npc_def("seller");

        let mut party = party_with_gold(500);

        // Fill inventory to MAX_ITEMS
        let mut character = make_character();
        for i in 0..(Inventory::MAX_ITEMS as ItemId) {
            character.inventory.add_item(i + 10, 0).unwrap();
        }
        assert!(character.inventory.is_full());

        let result = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            1,
            &item_db,
        );

        assert!(matches!(
            result,
            Err(TransactionError::InventoryFull { character_id: 0 })
        ));
        // Party gold unchanged
        assert_eq!(party.gold, 500);
        // Stock unchanged
        assert_eq!(
            npc_runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            5
        );
    }

    #[test]
    fn test_buy_item_out_of_stock() {
        let item_db = item_db_with(vec![make_item(1, 10, 5, 0)]);
        // quantity = 0 → sold out
        let mut npc_runtime = merchant_with_stock("seller", vec![StockEntry::new(1, 0)]);
        let npc_def = basic_npc_def("seller");

        let mut party = party_with_gold(500);
        let mut character = make_character();

        let result = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            1,
            &item_db,
        );

        assert!(matches!(
            result,
            Err(TransactionError::OutOfStock { item_id: 1 })
        ));
        assert_eq!(party.gold, 500);
        assert!(character.inventory.items.is_empty());
    }

    #[test]
    fn test_buy_item_not_in_stock() {
        let item_db = item_db_with(vec![make_item(1, 10, 5, 0)]);
        // NPC stock has item 2, but we try to buy item 99
        let mut npc_runtime = merchant_with_stock("seller", vec![StockEntry::new(2, 5)]);
        let npc_def = basic_npc_def("seller");

        let mut party = party_with_gold(500);
        let mut character = make_character();

        let result = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            99,
            &item_db,
        );

        assert!(matches!(
            result,
            Err(TransactionError::ItemNotInStock { item_id: 99 })
        ));
    }

    #[test]
    fn test_buy_item_charges_set_from_item_max_charges() {
        let item_db = item_db_with(vec![make_item(5, 20, 10, 3)]);
        let mut npc_runtime = merchant_with_stock("magic_shop", vec![StockEntry::new(5, 2)]);
        let npc_def = basic_npc_def("magic_shop");

        let mut party = party_with_gold(100);
        let mut character = make_character();

        let slot = buy_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            5,
            &item_db,
        )
        .unwrap();

        assert_eq!(slot.charges, 3);
        assert_eq!(character.inventory.items[0].charges, 3);
    }

    // ===== sell_item tests =====

    #[test]
    fn test_sell_item_success() {
        let item_db = item_db_with(vec![make_item(2, 20, 10, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![StockEntry::new(2, 1)]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new(); // gold starts at 0
        let mut character = character_with_items(vec![2]);

        let gold = sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            2,
            &item_db,
        )
        .expect("sell must succeed");

        // Item removed from inventory
        assert!(character.inventory.items.is_empty());
        // Gold added to party
        assert!(party.gold > 0);
        // Returned value matches party increase
        assert_eq!(party.gold, gold);
    }

    #[test]
    fn test_sell_item_not_in_inventory() {
        let item_db = item_db_with(vec![make_item(2, 20, 10, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = make_character(); // empty inventory

        let result = sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            2,
            &item_db,
        );

        assert!(matches!(
            result,
            Err(TransactionError::ItemNotInInventory {
                item_id: 2,
                character_id: 0
            })
        ));
    }

    #[test]
    fn test_sell_item_minimum_price() {
        // sell_cost = 0, base_cost = 1 → minimum 1 gold
        let item_db = item_db_with(vec![make_item(3, 1, 0, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = character_with_items(vec![3]);

        let gold = sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            3,
            &item_db,
        )
        .expect("sell must succeed");

        assert!(gold >= 1, "minimum sell price must be at least 1 gold");
    }

    #[test]
    fn test_sell_item_uses_sell_cost_when_nonzero() {
        // sell_cost = 15; buy_rate default 0.5 → floor(15 * 0.5) = 7
        let item_db = item_db_with(vec![make_item(4, 30, 15, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = character_with_items(vec![4]);

        let gold = sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            4,
            &item_db,
        )
        .unwrap();

        // floor(15 * 0.5) = 7
        assert_eq!(gold, 7);
    }

    #[test]
    fn test_sell_item_uses_base_cost_when_sell_cost_zero() {
        // sell_cost = 0, base_cost = 20; buy_rate default 0.5 → floor(10 * 0.5) = 5
        let item_db = item_db_with(vec![make_item(6, 20, 0, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = character_with_items(vec![6]);

        let gold = sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            6,
            &item_db,
        )
        .unwrap();

        // floor((20/2) * 0.5) = floor(5) = 5
        assert_eq!(gold, 5);
    }

    #[test]
    fn test_sell_item_increments_npc_stock_if_entry_exists() {
        let item_db = item_db_with(vec![make_item(7, 10, 5, 0)]);
        let mut npc_runtime = merchant_with_stock("buyer", vec![StockEntry::new(7, 2)]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = character_with_items(vec![7]);

        sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            7,
            &item_db,
        )
        .unwrap();

        // Stock for item 7 should have increased by 1
        assert_eq!(
            npc_runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(7)
                .unwrap()
                .quantity,
            3
        );
    }

    #[test]
    fn test_sell_item_does_not_add_stock_if_no_existing_entry() {
        let item_db = item_db_with(vec![make_item(8, 10, 5, 0)]);
        // NPC has no entry for item 8
        let mut npc_runtime = merchant_with_stock("buyer", vec![StockEntry::new(1, 5)]);
        let npc_def = basic_npc_def("buyer");

        let mut party = Party::new();
        let mut character = character_with_items(vec![8]);

        sell_item(
            &mut party,
            &mut character,
            0,
            &mut npc_runtime,
            &npc_def,
            8,
            &item_db,
        )
        .unwrap();

        // NPC stock still has no entry for item 8
        assert!(npc_runtime.stock.as_ref().unwrap().get_entry(8).is_none());
    }

    // ===== consume_service tests =====

    fn catalog_with(services: Vec<(&str, u32, u32)>) -> ServiceCatalog {
        let mut catalog = ServiceCatalog::new();
        for (id, cost, gem_cost) in services {
            let entry = if gem_cost > 0 {
                ServiceEntry::with_gem_cost(id, cost, gem_cost, "")
            } else {
                ServiceEntry::new(id, cost, "")
            };
            catalog.services.push(entry);
        }
        catalog
    }

    fn character_with_reduced_hp(base: u16, current: u16) -> Character {
        let mut c = make_character();
        c.hp.base = base;
        c.hp.current = current;
        c
    }

    #[test]
    fn test_consume_service_heal_all_success() {
        let catalog = catalog_with(vec![("heal_all", 50, 0)]);
        let mut party = party_with_gold(100);

        let mut character = character_with_reduced_hp(30, 10);
        let mut npc_runtime = NpcRuntimeState::new("priest_1".to_string());

        let outcome = consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "heal_all",
        )
        .expect("service must succeed");

        // Gold deducted
        assert_eq!(party.gold, 50);
        // HP restored
        assert_eq!(character.hp.current, 30);
        // Outcome details
        assert_eq!(outcome.gold_paid, 50);
        assert_eq!(outcome.gems_paid, 0);
        assert_eq!(outcome.service_id, "heal_all");
        // Service recorded
        assert_eq!(npc_runtime.services_consumed, vec!["heal_all"]);
    }

    #[test]
    fn test_consume_service_insufficient_gold() {
        let catalog = catalog_with(vec![("heal_all", 100, 0)]);
        let mut party = party_with_gold(10); // too poor

        let mut character = character_with_reduced_hp(30, 5);
        let mut npc_runtime = NpcRuntimeState::new("priest_1".to_string());

        let result = consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "heal_all",
        );

        assert!(matches!(
            result,
            Err(TransactionError::InsufficientGold {
                have: 10,
                need: 100,
            })
        ));
        // Party gold unchanged
        assert_eq!(party.gold, 10);
        // Character HP unchanged
        assert_eq!(character.hp.current, 5);
    }

    #[test]
    fn test_consume_service_not_found() {
        let catalog = ServiceCatalog::new(); // empty
        let mut party = party_with_gold(100);
        let mut npc_runtime = NpcRuntimeState::new("priest".to_string());

        let result = consume_service(
            &mut party,
            &mut vec![],
            &mut npc_runtime,
            &catalog,
            "nonexistent_service",
        );

        assert!(matches!(
            result,
            Err(TransactionError::ServiceNotFound { .. })
        ));
    }

    #[test]
    fn test_consume_service_resurrect() {
        let catalog = catalog_with(vec![("resurrect", 200, 0)]);
        let mut party = party_with_gold(500);

        let mut character = make_character();
        character.hp.base = 20;
        character.hp.current = 0;
        // Mark character as dead
        character.conditions.add(Condition::DEAD);
        assert!(character.conditions.has(Condition::DEAD));

        let mut npc_runtime = NpcRuntimeState::new("high_priest".to_string());

        let outcome = consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "resurrect",
        )
        .expect("resurrect must succeed");

        // Conditions cleared
        assert!(!character.conditions.has(Condition::DEAD));
        assert!(character.conditions.is_fine());
        // HP set to 1
        assert_eq!(character.hp.current, 1);
        assert_eq!(outcome.gold_paid, 200);
    }

    #[test]
    fn test_consume_service_restore_sp() {
        let catalog = catalog_with(vec![("restore_sp", 30, 0)]);
        let mut party = party_with_gold(100);

        let mut character = make_character();
        character.sp.base = 50;
        character.sp.current = 10;

        let mut npc_runtime = NpcRuntimeState::new("sage".to_string());

        consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "restore_sp",
        )
        .expect("service must succeed");

        assert_eq!(character.sp.current, 50);
        assert_eq!(party.gold, 70);
    }

    #[test]
    fn test_consume_service_cure_poison() {
        let catalog = catalog_with(vec![("cure_poison", 20, 0)]);
        let mut party = party_with_gold(100);

        let mut character = make_character();
        character.conditions.add(Condition::POISONED);
        assert!(character.conditions.has(Condition::POISONED));

        let mut npc_runtime = NpcRuntimeState::new("apothecary".to_string());

        consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "cure_poison",
        )
        .expect("service must succeed");

        assert!(!character.conditions.has(Condition::POISONED));
    }

    #[test]
    fn test_consume_service_insufficient_gems() {
        let catalog = catalog_with(vec![("resurrect", 0, 5)]);
        let mut party = party_with_gold(500);
        party.gems = 2; // not enough gems

        let mut character = make_character();
        let mut npc_runtime = NpcRuntimeState::new("temple".to_string());

        let result = consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "resurrect",
        );

        assert!(matches!(
            result,
            Err(TransactionError::InsufficientGems { have: 2, need: 5 })
        ));
        assert_eq!(party.gold, 500);
        assert_eq!(party.gems, 2);
    }

    #[test]
    fn test_consume_service_rest() {
        let catalog = catalog_with(vec![("rest", 10, 0)]);
        let mut party = party_with_gold(100);

        let mut character = make_character();
        character.hp.base = 40;
        character.hp.current = 10;
        character.sp.base = 20;
        character.sp.current = 5;
        character.conditions.add(Condition::BLINDED);

        let mut npc_runtime = NpcRuntimeState::new("innkeeper".to_string());

        consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "rest",
        )
        .expect("service must succeed");

        assert_eq!(character.hp.current, 40);
        assert_eq!(character.sp.current, 20);
        assert!(!character.conditions.has(Condition::BLINDED));
        assert_eq!(party.gold, 90);
    }

    #[test]
    fn test_consume_service_unknown_id_no_op() {
        // An unrecognised service_id that IS in the catalog should not error,
        // and the character should be unaffected.
        let catalog = catalog_with(vec![("mystery_service", 1, 0)]);
        let mut party = party_with_gold(100);

        let mut character = make_character();
        character.hp.base = 20;
        character.hp.current = 5;

        let mut npc_runtime = NpcRuntimeState::new("shaman".to_string());

        let outcome = consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "mystery_service",
        )
        .expect("service must succeed (no error for unknown effect)");

        // Character HP unchanged (no-op effect)
        assert_eq!(character.hp.current, 5);
        // Gold still deducted
        assert_eq!(party.gold, 99);
        assert_eq!(outcome.gold_paid, 1);
    }

    #[test]
    fn test_consume_service_multiple_targets() {
        let catalog = catalog_with(vec![("heal_all", 50, 0)]);
        let mut party = party_with_gold(200);

        let mut c1 = character_with_reduced_hp(30, 10);
        let mut c2 = character_with_reduced_hp(20, 5);

        let mut npc_runtime = NpcRuntimeState::new("healer".to_string());

        consume_service(
            &mut party,
            &mut vec![&mut c1, &mut c2],
            &mut npc_runtime,
            &catalog,
            "heal_all",
        )
        .expect("service must succeed");

        // Both characters healed
        assert_eq!(c1.hp.current, 30);
        assert_eq!(c2.hp.current, 20);
        // Gold deducted once
        assert_eq!(party.gold, 150);
    }

    #[test]
    fn test_consume_service_records_service_id() {
        let catalog = catalog_with(vec![("heal_all", 10, 0), ("restore_sp", 10, 0)]);
        let mut party = party_with_gold(100);

        let mut npc_runtime = NpcRuntimeState::new("priest".to_string());
        let mut character = make_character();

        consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "heal_all",
        )
        .unwrap();

        consume_service(
            &mut party,
            &mut vec![&mut character],
            &mut npc_runtime,
            &catalog,
            "restore_sp",
        )
        .unwrap();

        assert_eq!(
            npc_runtime.services_consumed,
            vec!["heal_all", "restore_sp"]
        );
    }

    // ===== TransactionError display tests =====

    // ===== drop_item tests =====

    fn make_world_with_map(map_id: MapId) -> World {
        let mut world = World::new();
        let map = Map::new(
            map_id,
            format!("Map {map_id}"),
            "Test map".to_string(),
            20,
            20,
        );
        world.add_map(map);
        world.set_current_map(map_id);
        world
    }

    fn character_with_single_item(item_id: ItemId, charges: u8) -> Character {
        let mut c = make_character();
        c.inventory.add_item(item_id, charges).unwrap();
        c
    }

    /// `drop_item` appends a `DroppedItem` to the correct map.
    #[test]
    fn test_drop_item_records_in_world() {
        let mut world = make_world_with_map(1);
        let mut character = character_with_single_item(5, 3);
        let pos = Position::new(10, 10);

        let dropped =
            drop_item(&mut character, 0, 0, &mut world, 1, pos).expect("drop must succeed");

        let map = world.get_map(1).unwrap();
        assert_eq!(map.dropped_items.len(), 1);
        assert_eq!(map.dropped_items[0].item_id, 5);
        assert_eq!(map.dropped_items[0].charges, 3);
        assert_eq!(map.dropped_items[0].position, pos);
        assert_eq!(map.dropped_items[0].map_id, 1);
        assert_eq!(dropped.item_id, 5);
        assert_eq!(dropped.charges, 3);
    }

    /// `drop_item` removes the item from the character's inventory.
    #[test]
    fn test_drop_item_removes_from_inventory() {
        let mut world = make_world_with_map(1);
        let mut character = character_with_single_item(7, 0);

        drop_item(&mut character, 0, 0, &mut world, 1, Position::new(5, 5))
            .expect("drop must succeed");

        assert!(character.inventory.items.is_empty());
    }

    /// `drop_item` returns `ItemNotInInventory` when slot index is out of bounds.
    #[test]
    fn test_drop_item_out_of_bounds_slot_returns_error() {
        let mut world = make_world_with_map(1);
        let mut character = make_character(); // empty inventory

        let result = drop_item(&mut character, 0, 99, &mut world, 1, Position::new(0, 0));

        assert!(matches!(
            result,
            Err(TransactionError::ItemNotInInventory { .. })
        ));
    }

    /// `drop_item` returns `MapNotFound` when the map ID does not exist.
    #[test]
    fn test_drop_item_map_not_found_returns_error() {
        let mut world = make_world_with_map(1);
        let mut character = character_with_single_item(3, 0);

        let result = drop_item(&mut character, 0, 0, &mut world, 999, Position::new(0, 0));

        assert!(matches!(
            result,
            Err(TransactionError::MapNotFound { map_id: 999 })
        ));
        // Item should still be in inventory since drop failed
        assert_eq!(character.inventory.items.len(), 1);
    }

    // ===== pickup_item tests =====

    /// `pickup_item` adds the item to the character's inventory.
    #[test]
    fn test_pickup_item_adds_to_inventory() {
        let mut world = make_world_with_map(2);
        let mut dropper = character_with_single_item(9, 5);
        let pos = Position::new(3, 7);
        drop_item(&mut dropper, 0, 0, &mut world, 2, pos).unwrap();

        let mut picker = make_character();
        let slot = pickup_item(&mut picker, 1, &mut world, 2, pos, 9).expect("pickup must succeed");

        assert_eq!(slot.item_id, 9);
        assert_eq!(slot.charges, 5);
        assert_eq!(picker.inventory.items.len(), 1);
        assert_eq!(picker.inventory.items[0].item_id, 9);
        assert_eq!(picker.inventory.items[0].charges, 5);
    }

    /// `pickup_item` removes the `DroppedItem` entry from the map.
    #[test]
    fn test_pickup_item_removes_from_map() {
        let mut world = make_world_with_map(3);
        let mut dropper = character_with_single_item(4, 1);
        let pos = Position::new(1, 1);
        drop_item(&mut dropper, 0, 0, &mut world, 3, pos).unwrap();

        let mut picker = make_character();
        pickup_item(&mut picker, 0, &mut world, 3, pos, 4).unwrap();

        assert!(world.get_map(3).unwrap().dropped_items.is_empty());
    }

    /// `pickup_item` returns `InventoryFull` when the character cannot accept more items.
    #[test]
    fn test_pickup_item_inventory_full_returns_error() {
        let mut world = make_world_with_map(1);

        // Drop the item first using a temporary dropper
        let mut dropper = character_with_single_item(2, 0);
        let pos = Position::new(5, 5);
        drop_item(&mut dropper, 0, 0, &mut world, 1, pos).unwrap();

        // Fill picker inventory to capacity
        let mut picker = make_character();
        for i in 0..crate::domain::character::Inventory::MAX_ITEMS {
            picker
                .inventory
                .add_item((i % 256) as u8, 0)
                .expect("should have space");
        }

        let result = pickup_item(&mut picker, 0, &mut world, 1, pos, 2);

        assert!(matches!(
            result,
            Err(TransactionError::InventoryFull { .. })
        ));
        // Item must still be on the map (was not consumed)
        assert_eq!(world.get_map(1).unwrap().dropped_items.len(), 1);
    }

    /// `pickup_item` returns `ItemNotInInventory` when no matching dropped item exists.
    #[test]
    fn test_pickup_item_missing_returns_error() {
        let mut world = make_world_with_map(1);
        let mut picker = make_character();

        let result = pickup_item(&mut picker, 0, &mut world, 1, Position::new(5, 5), 42);

        assert!(matches!(
            result,
            Err(TransactionError::ItemNotInInventory { item_id: 42, .. })
        ));
    }

    /// `pickup_item` returns `MapNotFound` when the map ID does not exist.
    #[test]
    fn test_pickup_item_map_not_found_returns_error() {
        let mut world = World::new(); // empty world, no maps
        let mut picker = make_character();

        let result = pickup_item(&mut picker, 0, &mut world, 99, Position::new(0, 0), 1);

        assert!(matches!(
            result,
            Err(TransactionError::MapNotFound { map_id: 99 })
        ));
    }

    /// `TransactionError::MapNotFound` displays the map ID.
    #[test]
    fn test_transaction_error_map_not_found_display() {
        let err = TransactionError::MapNotFound { map_id: 7 };
        assert!(err.to_string().contains("7"));
    }

    #[test]
    fn test_transaction_error_insufficient_gold_display() {
        let err = TransactionError::InsufficientGold { have: 5, need: 100 };
        let msg = err.to_string();
        assert!(msg.contains("5"));
        assert!(msg.contains("100"));
    }

    #[test]
    fn test_transaction_error_item_not_in_stock_display() {
        let err = TransactionError::ItemNotInStock { item_id: 7 };
        assert!(err.to_string().contains("7"));
    }

    // ===== equip_item Tests =====

    #[test]
    fn test_equip_item_weapon_moves_from_inventory_to_slot() {
        // Arrange: sword in inventory slot 0
        let item_db = item_db_with(vec![make_weapon_item_tx(1)]);
        let mut character = character_with_items(vec![1]);
        let classes = make_class_db_with_profs("knight", &["simple_weapon", "martial_melee"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(character.equipment.weapon, Some(1));
        assert!(
            character.inventory.items.is_empty(),
            "inventory should be empty after equipping"
        );
    }

    #[test]
    fn test_equip_item_swaps_old_weapon_back_to_inventory() {
        // Arrange: sword 1 already equipped, sword 2 in inventory slot 0
        let item_db = item_db_with(vec![make_weapon_item_tx(1), make_weapon_item_tx(2)]);
        let mut character = make_character();
        character.equipment.weapon = Some(1);
        character.inventory.add_item(2, 0).unwrap();
        let classes = make_class_db_with_profs("knight", &["simple_weapon", "martial_melee"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            character.equipment.weapon,
            Some(2),
            "new sword must be equipped"
        );
        assert_eq!(
            character.inventory.items.len(),
            1,
            "displaced sword must have returned to inventory"
        );
        assert_eq!(
            character.inventory.items[0].item_id, 1,
            "original sword ID must be in inventory"
        );
    }

    #[test]
    fn test_equip_item_armor_updates_ac() {
        // Arrange: +5 chain mail (Medium) in inventory; AC starts at AC_DEFAULT (10)
        let item_db = item_db_with(vec![make_armor_item_tx(21, 5, ArmorClassification::Medium)]);
        let mut character = character_with_items(vec![21]);
        let classes = make_class_db_with_profs("knight", &["medium_armor"]);
        let races = make_race_db_human();

        assert_eq!(character.ac.current, 10, "AC must start at AC_DEFAULT (10)");

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            character.ac.current, 15,
            "AC must be AC_DEFAULT (10) + ac_bonus (5) = 15"
        );
    }

    #[test]
    fn test_equip_item_helmet_routes_to_helmet_slot() {
        // Arrange: Iron Helmet (Helmet classification) in inventory
        let item_db = item_db_with(vec![make_armor_item_tx(25, 1, ArmorClassification::Helmet)]);
        let mut character = character_with_items(vec![25]);
        let classes = make_class_db_with_profs("knight", &["light_armor"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            character.equipment.helmet,
            Some(25),
            "Helmet item must route to the helmet slot"
        );
        assert!(
            character.equipment.armor.is_none(),
            "body armor slot must remain unchanged"
        );
    }

    #[test]
    fn test_equip_item_boots_routes_to_boots_slot() {
        // Arrange: Leather Boots (Boots classification) in inventory
        let item_db = item_db_with(vec![make_armor_item_tx(26, 1, ArmorClassification::Boots)]);
        let mut character = character_with_items(vec![26]);
        let classes = make_class_db_with_profs("knight", &["light_armor"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            character.equipment.boots,
            Some(26),
            "Boots item must route to the boots slot"
        );
        assert!(
            character.equipment.armor.is_none(),
            "body armor slot must remain unchanged"
        );
    }

    #[test]
    fn test_equip_item_invalid_class_returns_error() {
        // Arrange: Plate armor (Heavy — needs "heavy_armor"); class only has "simple_weapon"
        let item_db = item_db_with(vec![make_armor_item_tx(22, 6, ArmorClassification::Heavy)]);
        let mut character = character_with_items(vec![22]);
        let classes = make_class_db_with_profs("knight", &["simple_weapon"]); // no heavy_armor
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(
            matches!(result, Err(EquipError::ClassRestriction)),
            "expected ClassRestriction, got {:?}",
            result
        );
        // Inventory must be unchanged — no mutations on failure
        assert_eq!(
            character.inventory.items.len(),
            1,
            "inventory must be unchanged after failed equip"
        );
    }

    #[test]
    fn test_equip_item_out_of_bounds_returns_error() {
        // Arrange: empty inventory; attempt to equip slot index 5
        let item_db = item_db_with(vec![]);
        let mut character = make_character(); // empty inventory
        let classes = make_class_db_with_profs("knight", &["martial_melee"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 5, &item_db, &classes, &races);

        // Assert
        assert!(
            matches!(result, Err(EquipError::ItemNotFound(_))),
            "expected ItemNotFound for out-of-bounds index, got {:?}",
            result
        );
    }

    #[test]
    fn test_equip_item_non_equipable_item_returns_error() {
        // Arrange: consumable in inventory (consumables have no equipment slot)
        let item_db = item_db_with(vec![make_consumable_item_tx(50)]);
        let mut character = character_with_items(vec![50]);
        let classes = make_class_db_with_profs("knight", &["simple_weapon", "light_armor"]);
        let races = make_race_db_human();

        // Act
        let result = equip_item(&mut character, 0, &item_db, &classes, &races);

        // Assert
        assert!(
            matches!(result, Err(EquipError::NoSlotAvailable)),
            "expected NoSlotAvailable for consumable, got {:?}",
            result
        );
        assert_eq!(
            character.inventory.items.len(),
            1,
            "inventory must be unchanged after failed equip"
        );
    }

    // ===== unequip_item Tests =====

    #[test]
    fn test_unequip_item_moves_to_inventory() {
        // Arrange: sword equipped in weapon slot, empty inventory
        let item_db = item_db_with(vec![make_weapon_item_tx(1)]);
        let mut character = make_character();
        character.equipment.weapon = Some(1);

        // Act
        let result = unequip_item(&mut character, EquipmentSlot::Weapon, &item_db);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert!(
            character.equipment.weapon.is_none(),
            "weapon slot must be cleared after unequip"
        );
        assert_eq!(
            character.inventory.items.len(),
            1,
            "item must move to inventory"
        );
        assert_eq!(
            character.inventory.items[0].item_id, 1,
            "unequipped item ID must appear in inventory"
        );
    }

    #[test]
    fn test_unequip_item_reduces_ac() {
        // Arrange: leather armor +4 equipped; set AC to reflect equipped state
        let item_db = item_db_with(vec![make_armor_item_tx(20, 4, ArmorClassification::Light)]);
        let mut character = make_character();
        character.equipment.armor = Some(20);

        use crate::domain::items::calculate_armor_class;
        character.ac.current = calculate_armor_class(&character.equipment, &item_db);
        assert_eq!(character.ac.current, 14, "AC should be 14 before unequip");

        // Act
        let result = unequip_item(&mut character, EquipmentSlot::Armor, &item_db);

        // Assert
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        assert_eq!(
            character.ac.current, 10,
            "AC must return to AC_DEFAULT (10) after unequipping armor"
        );
    }

    #[test]
    fn test_unequip_item_empty_slot_is_noop() {
        // Arrange: weapon slot is empty
        let item_db = item_db_with(vec![]);
        let mut character = make_character();
        let initial_len = character.inventory.items.len();

        // Act
        let result = unequip_item(&mut character, EquipmentSlot::Weapon, &item_db);

        // Assert: Ok(()) and no state change
        assert!(result.is_ok(), "unequip of empty slot must return Ok");
        assert_eq!(
            character.inventory.items.len(),
            initial_len,
            "inventory must not change when slot was already empty"
        );
    }

    #[test]
    fn test_unequip_item_inventory_full_returns_error() {
        // Arrange: armor equipped AND inventory completely full
        let item_db = item_db_with(vec![make_armor_item_tx(20, 4, ArmorClassification::Light)]);
        let mut character = make_character();
        character.equipment.armor = Some(20);
        character.ac.current = 14; // 10 + 4

        // Fill inventory to capacity with dummy item IDs
        for i in 0..crate::domain::character::Inventory::MAX_ITEMS {
            character.inventory.add_item(i as ItemId + 100, 0).unwrap();
        }
        assert!(
            character.inventory.is_full(),
            "inventory must be full before test"
        );

        // Act
        let result = unequip_item(&mut character, EquipmentSlot::Armor, &item_db);

        // Assert
        assert!(
            matches!(result, Err(TransactionError::InventoryFull { .. })),
            "expected InventoryFull, got {:?}",
            result
        );
        // Armor slot must still be occupied — no partial mutation
        assert_eq!(
            character.equipment.armor,
            Some(20),
            "armor slot must be unchanged after failed unequip"
        );
    }
}
