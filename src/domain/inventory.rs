// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared inventory ownership primitives
//!
//! This module defines the shared ownership model used by both Characters and NPCs.
//! It provides merchant stock management, service catalogs, and economy settings
//! that compose on top of the core inventory system.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.3 and the inventory system
//! implementation plan for complete specifications.
//!
//! # Design
//!
//! - `InventoryOwner` identifies who owns a given inventory (character or NPC)
//! - `MerchantStock` tracks mutable runtime item quantities for merchants
//! - `ServiceCatalog` lists services offered by priests or innkeepers
//! - `NpcEconomySettings` controls per-NPC buy/sell price multipliers

use crate::domain::types::{CharacterId, ItemId};
use crate::domain::world::npc::NpcId;
use serde::{Deserialize, Serialize};

// ===== InventoryOwner =====

/// Identifies the owner of an inventory
///
/// Used when a transaction needs to specify which entity's inventory is
/// affected (e.g., delivering a purchased item to a specific character,
/// or removing sold stock from a specific NPC).
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::InventoryOwner;
///
/// let owner = InventoryOwner::Character(0);
/// let npc_owner = InventoryOwner::Npc("merchant_tom".to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryOwner {
    /// A party member identified by their roster index
    Character(CharacterId),
    /// An NPC identified by their string ID
    Npc(NpcId),
}

// ===== StockEntry =====

/// A single item entry in a merchant's stock
///
/// Tracks how many of a particular item the merchant currently has available,
/// along with an optional price override. When `quantity` reaches 0 the item
/// is sold out and cannot be purchased until the merchant restocks.
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::StockEntry;
///
/// let entry = StockEntry {
///     item_id: 5,
///     quantity: 3,
///     override_price: Some(150),
/// };
///
/// assert_eq!(entry.item_id, 5);
/// assert_eq!(entry.quantity, 3);
/// assert_eq!(entry.override_price, Some(150));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockEntry {
    /// The item being sold (references `ItemDatabase`)
    pub item_id: ItemId,

    /// Current available quantity; 0 means sold out
    pub quantity: u8,

    /// Optional price override in gold.
    ///
    /// When `Some`, this price is used instead of the item's `base_cost`.
    /// When `None`, the item's `base_cost` multiplied by the NPC's `sell_rate` is used.
    pub override_price: Option<u32>,
}

impl StockEntry {
    /// Creates a new stock entry with the given item, quantity, and no price override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::StockEntry;
    ///
    /// let entry = StockEntry::new(1, 5);
    /// assert_eq!(entry.item_id, 1);
    /// assert_eq!(entry.quantity, 5);
    /// assert!(entry.override_price.is_none());
    /// ```
    pub fn new(item_id: ItemId, quantity: u8) -> Self {
        Self {
            item_id,
            quantity,
            override_price: None,
        }
    }

    /// Creates a new stock entry with an explicit price override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::StockEntry;
    ///
    /// let entry = StockEntry::with_price(2, 10, 200);
    /// assert_eq!(entry.override_price, Some(200));
    /// ```
    pub fn with_price(item_id: ItemId, quantity: u8, price: u32) -> Self {
        Self {
            item_id,
            quantity,
            override_price: Some(price),
        }
    }

    /// Returns true if this entry has remaining stock
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::StockEntry;
    ///
    /// let in_stock = StockEntry::new(1, 3);
    /// let sold_out = StockEntry::new(2, 0);
    ///
    /// assert!(in_stock.is_available());
    /// assert!(!sold_out.is_available());
    /// ```
    pub fn is_available(&self) -> bool {
        self.quantity > 0
    }
}

// ===== MerchantStock =====

/// Runtime merchant inventory tracking current stock quantities
///
/// This is the mutable runtime state for a merchant's shop inventory.
/// It is initialised from a stock template at game start and mutated as
/// items are bought and sold.
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::{MerchantStock, StockEntry};
///
/// let mut stock = MerchantStock {
///     entries: vec![
///         StockEntry::new(1, 5),
///         StockEntry::new(2, 1),
///     ],
///     restock_template: Some("blacksmith_basic".to_string()),
/// };
///
/// assert!(stock.decrement(1));
/// assert_eq!(stock.get_entry(1).unwrap().quantity, 4);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantStock {
    /// Current mutable runtime stock entries
    pub entries: Vec<StockEntry>,

    /// Optional template ID used to restock this merchant's inventory.
    ///
    /// References a `MerchantStockTemplate` in the campaign data files.
    /// When `None` the merchant's stock is never automatically replenished.
    pub restock_template: Option<String>,
}

impl MerchantStock {
    /// Creates an empty merchant stock with no items
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::MerchantStock;
    ///
    /// let stock = MerchantStock::new();
    /// assert!(stock.entries.is_empty());
    /// assert!(stock.restock_template.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            restock_template: None,
        }
    }

    /// Returns an immutable reference to the stock entry for the given item, if present
    ///
    /// # Arguments
    ///
    /// * `item_id` - The item to look up
    ///
    /// # Returns
    ///
    /// `Some(&StockEntry)` if the item is listed in stock, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{MerchantStock, StockEntry};
    ///
    /// let mut stock = MerchantStock::new();
    /// stock.entries.push(StockEntry::new(3, 2));
    ///
    /// assert!(stock.get_entry(3).is_some());
    /// assert!(stock.get_entry(99).is_none());
    /// ```
    pub fn get_entry(&self, item_id: ItemId) -> Option<&StockEntry> {
        self.entries.iter().find(|e| e.item_id == item_id)
    }

    /// Returns a mutable reference to the stock entry for the given item, if present
    ///
    /// # Arguments
    ///
    /// * `item_id` - The item to look up
    ///
    /// # Returns
    ///
    /// `Some(&mut StockEntry)` if the item is listed in stock, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{MerchantStock, StockEntry};
    ///
    /// let mut stock = MerchantStock::new();
    /// stock.entries.push(StockEntry::new(4, 5));
    ///
    /// if let Some(entry) = stock.get_entry_mut(4) {
    ///     entry.quantity = 10;
    /// }
    /// assert_eq!(stock.get_entry(4).unwrap().quantity, 10);
    /// ```
    pub fn get_entry_mut(&mut self, item_id: ItemId) -> Option<&mut StockEntry> {
        self.entries.iter_mut().find(|e| e.item_id == item_id)
    }

    /// Decrements the quantity of the given item by one
    ///
    /// Returns `true` if the item was in stock and its quantity was decremented.
    /// Returns `false` if the item is not listed in stock or is already sold out
    /// (quantity == 0).
    ///
    /// # Arguments
    ///
    /// * `item_id` - The item to decrement
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{MerchantStock, StockEntry};
    ///
    /// let mut stock = MerchantStock::new();
    /// stock.entries.push(StockEntry::new(1, 2));
    ///
    /// assert!(stock.decrement(1));
    /// assert_eq!(stock.get_entry(1).unwrap().quantity, 1);
    ///
    /// assert!(stock.decrement(1));
    /// assert_eq!(stock.get_entry(1).unwrap().quantity, 0);
    ///
    /// // Already at 0 - returns false
    /// assert!(!stock.decrement(1));
    /// ```
    pub fn decrement(&mut self, item_id: ItemId) -> bool {
        match self.get_entry_mut(item_id) {
            Some(entry) if entry.quantity > 0 => {
                entry.quantity -= 1;
                true
            }
            _ => false,
        }
    }

    /// Returns the effective sell price for an item, respecting any override
    ///
    /// If the stock entry has an `override_price`, that value is returned
    /// directly. Otherwise `base_cost` (from the `ItemDatabase`) is used.
    ///
    /// # Arguments
    ///
    /// * `item_id`   - The item whose price should be determined
    /// * `base_cost` - The item's base cost from the `ItemDatabase`
    ///
    /// # Returns
    ///
    /// The price in gold that the merchant charges for this item.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{MerchantStock, StockEntry};
    ///
    /// let mut stock = MerchantStock::new();
    /// stock.entries.push(StockEntry::with_price(1, 3, 999));
    /// stock.entries.push(StockEntry::new(2, 3));
    ///
    /// // Override takes precedence
    /// assert_eq!(stock.effective_price(1, 100), 999);
    ///
    /// // No override: uses base_cost
    /// assert_eq!(stock.effective_price(2, 100), 100);
    ///
    /// // Item not in stock: falls back to base_cost
    /// assert_eq!(stock.effective_price(99, 50), 50);
    /// ```
    pub fn effective_price(&self, item_id: ItemId, base_cost: u32) -> u32 {
        match self.get_entry(item_id) {
            Some(entry) => entry.override_price.unwrap_or(base_cost),
            None => base_cost,
        }
    }
}

impl Default for MerchantStock {
    fn default() -> Self {
        Self::new()
    }
}

// ===== ServiceEntry =====

/// A single service offered by a priest or innkeeper NPC
///
/// Services are non-item transactions that consume party gold (and optionally
/// gems) in exchange for a beneficial effect such as healing, condition curing,
/// or resting.
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::ServiceEntry;
///
/// let heal = ServiceEntry {
///     service_id: "heal_all".to_string(),
///     cost: 50,
///     gem_cost: 0,
///     description: "Restore all HP for the entire party".to_string(),
/// };
///
/// assert_eq!(heal.service_id, "heal_all");
/// assert_eq!(heal.cost, 50);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceEntry {
    /// Unique identifier for this service (e.g., `"heal_all"`, `"cure_poison"`, `"rest"`)
    pub service_id: String,

    /// Gold cost to purchase this service
    pub cost: u32,

    /// Gem cost to purchase this service (0 if not required)
    pub gem_cost: u32,

    /// Human-readable description shown to the player
    pub description: String,
}

impl ServiceEntry {
    /// Creates a new service entry with no gem cost
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::ServiceEntry;
    ///
    /// let service = ServiceEntry::new("rest", 10, "Rest and recover HP overnight");
    /// assert_eq!(service.service_id, "rest");
    /// assert_eq!(service.cost, 10);
    /// assert_eq!(service.gem_cost, 0);
    /// ```
    pub fn new(service_id: impl Into<String>, cost: u32, description: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            cost,
            gem_cost: 0,
            description: description.into(),
        }
    }

    /// Creates a new service entry with both a gold and gem cost
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::ServiceEntry;
    ///
    /// let service = ServiceEntry::with_gem_cost("resurrect", 500, 1, "Resurrect a dead character");
    /// assert_eq!(service.gem_cost, 1);
    /// ```
    pub fn with_gem_cost(
        service_id: impl Into<String>,
        cost: u32,
        gem_cost: u32,
        description: impl Into<String>,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            cost,
            gem_cost,
            description: description.into(),
        }
    }
}

// ===== ServiceCatalog =====

/// A collection of services offered by a single NPC
///
/// Used by priests and innkeepers to define what paid services they provide.
/// The catalog is stored as part of the static `NpcDefinition` and is therefore
/// read-only at runtime (services do not have mutable quantity state).
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
///
/// let mut catalog = ServiceCatalog { services: vec![] };
/// catalog.services.push(ServiceEntry::new("heal_all", 50, "Heal the party"));
/// catalog.services.push(ServiceEntry::new("cure_poison", 25, "Cure poison"));
///
/// assert!(catalog.has_service("heal_all"));
/// assert!(!catalog.has_service("resurrect"));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCatalog {
    /// All services offered by this NPC
    pub services: Vec<ServiceEntry>,
}

impl ServiceCatalog {
    /// Creates an empty service catalog
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::ServiceCatalog;
    ///
    /// let catalog = ServiceCatalog::new();
    /// assert!(catalog.services.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// Returns a reference to the service with the given ID, if present
    ///
    /// # Arguments
    ///
    /// * `service_id` - The service ID to look up
    ///
    /// # Returns
    ///
    /// `Some(&ServiceEntry)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
    ///
    /// let mut catalog = ServiceCatalog::new();
    /// catalog.services.push(ServiceEntry::new("rest", 10, "Rest overnight"));
    ///
    /// assert!(catalog.get_service("rest").is_some());
    /// assert!(catalog.get_service("nonexistent").is_none());
    /// ```
    pub fn get_service(&self, service_id: &str) -> Option<&ServiceEntry> {
        self.services.iter().find(|s| s.service_id == service_id)
    }

    /// Returns true if this catalog contains a service with the given ID
    ///
    /// # Arguments
    ///
    /// * `service_id` - The service ID to check
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
    ///
    /// let mut catalog = ServiceCatalog::new();
    /// catalog.services.push(ServiceEntry::new("heal_all", 50, "Heal party"));
    ///
    /// assert!(catalog.has_service("heal_all"));
    /// assert!(!catalog.has_service("cure_poison"));
    /// ```
    pub fn has_service(&self, service_id: &str) -> bool {
        self.services.iter().any(|s| s.service_id == service_id)
    }
}

impl Default for ServiceCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ===== NpcEconomySettings =====

/// Per-NPC price multiplier settings for buying and selling
///
/// Controls the exchange rates applied when the party trades with this NPC.
/// The defaults model a standard merchant who sells at full price and buys
/// back at half price.
///
/// # Examples
///
/// ```
/// use antares::domain::inventory::NpcEconomySettings;
///
/// let economy = NpcEconomySettings::default();
/// assert_eq!(economy.buy_rate, 0.5);
/// assert_eq!(economy.sell_rate, 1.0);
/// assert!(economy.max_buy_value.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcEconomySettings {
    /// Multiplier applied to an item's `base_cost` when the NPC buys from the player.
    ///
    /// A value of `0.5` means the NPC pays half the item's base cost.
    /// Must be in the range `[0.0, 1.0]` in practice.
    pub buy_rate: f32,

    /// Multiplier applied to an item's `base_cost` when the NPC sells to the player.
    ///
    /// A value of `1.0` means the NPC charges full base cost.
    /// Values above `1.0` represent a surcharge (e.g., `1.5` = 50% markup).
    pub sell_rate: f32,

    /// Optional cap on the maximum gold the NPC will pay for any single item.
    ///
    /// When `Some(n)`, the NPC will never pay more than `n` gold regardless of
    /// `buy_rate`. When `None` there is no cap.
    pub max_buy_value: Option<u32>,
}

impl NpcEconomySettings {
    /// Creates economy settings with custom rates and no buy cap
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::NpcEconomySettings;
    ///
    /// let economy = NpcEconomySettings::new(0.4, 1.2);
    /// assert_eq!(economy.buy_rate, 0.4);
    /// assert_eq!(economy.sell_rate, 1.2);
    /// assert!(economy.max_buy_value.is_none());
    /// ```
    pub fn new(buy_rate: f32, sell_rate: f32) -> Self {
        Self {
            buy_rate,
            sell_rate,
            max_buy_value: None,
        }
    }

    /// Calculates the gold the NPC will pay when buying an item from the player
    ///
    /// The result is always at least 1 gold (items are never free to sell).
    ///
    /// # Arguments
    ///
    /// * `base_cost` - The item's base cost from the `ItemDatabase`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::NpcEconomySettings;
    ///
    /// let economy = NpcEconomySettings::new(0.5, 1.0);
    /// assert_eq!(economy.npc_buy_price(100), 50);
    /// assert_eq!(economy.npc_buy_price(0), 1);
    /// ```
    pub fn npc_buy_price(&self, base_cost: u32) -> u32 {
        let price = ((base_cost as f32) * self.buy_rate).round() as u32;
        let price = price.max(1);
        match self.max_buy_value {
            Some(cap) => price.min(cap),
            None => price,
        }
    }

    /// Calculates the gold the player must pay to buy an item from the NPC
    ///
    /// The result is always at least 1 gold.
    ///
    /// # Arguments
    ///
    /// * `base_cost` - The item's base cost from the `ItemDatabase`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::inventory::NpcEconomySettings;
    ///
    /// let economy = NpcEconomySettings::new(0.5, 1.5);
    /// assert_eq!(economy.npc_sell_price(100), 150);
    /// assert_eq!(economy.npc_sell_price(0), 1);
    /// ```
    pub fn npc_sell_price(&self, base_cost: u32) -> u32 {
        let price = ((base_cost as f32) * self.sell_rate).round() as u32;
        price.max(1)
    }
}

impl Default for NpcEconomySettings {
    /// Returns the standard merchant economy: buys at 50%, sells at 100%, no cap
    fn default() -> Self {
        Self {
            buy_rate: 0.5,
            sell_rate: 1.0,
            max_buy_value: None,
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ----- MerchantStock tests -----

    #[test]
    fn test_merchant_stock_decrement_success() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(1, 3));

        let result = stock.decrement(1);

        assert!(result, "decrement should return true when item is in stock");
        assert_eq!(
            stock.get_entry(1).unwrap().quantity,
            2,
            "quantity should decrease by 1"
        );
    }

    #[test]
    fn test_merchant_stock_decrement_out_of_stock() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(1, 0));

        let result = stock.decrement(1);

        assert!(
            !result,
            "decrement should return false when quantity is already 0"
        );
        assert_eq!(
            stock.get_entry(1).unwrap().quantity,
            0,
            "quantity should remain 0"
        );
    }

    #[test]
    fn test_merchant_stock_decrement_nonexistent_item() {
        let mut stock = MerchantStock::new();

        let result = stock.decrement(99);

        assert!(
            !result,
            "decrement should return false when item is not in stock"
        );
    }

    #[test]
    fn test_merchant_stock_effective_price_uses_override() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::with_price(5, 2, 999));

        let price = stock.effective_price(5, 100);

        assert_eq!(
            price, 999,
            "override price should take precedence over base_cost"
        );
    }

    #[test]
    fn test_merchant_stock_effective_price_uses_base_cost() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(5, 2)); // no override

        let price = stock.effective_price(5, 100);

        assert_eq!(
            price, 100,
            "base_cost should be used when there is no override"
        );
    }

    #[test]
    fn test_merchant_stock_decrement_reduces_to_zero() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(2, 1));

        assert!(stock.decrement(2));
        assert_eq!(stock.get_entry(2).unwrap().quantity, 0);

        // Second decrement must fail
        assert!(!stock.decrement(2));
    }

    #[test]
    fn test_merchant_stock_get_entry_present() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(7, 4));

        assert!(stock.get_entry(7).is_some());
    }

    #[test]
    fn test_merchant_stock_get_entry_absent() {
        let stock = MerchantStock::new();
        assert!(stock.get_entry(7).is_none());
    }

    #[test]
    fn test_merchant_stock_get_entry_mut() {
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(3, 5));

        if let Some(entry) = stock.get_entry_mut(3) {
            entry.quantity = 99;
        }

        assert_eq!(stock.get_entry(3).unwrap().quantity, 99);
    }

    // ----- ServiceCatalog tests -----

    #[test]
    fn test_service_catalog_get_service_found() {
        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal the entire party"));

        let service = catalog.get_service("heal_all");

        assert!(
            service.is_some(),
            "get_service should return Some for an existing service"
        );
        assert_eq!(service.unwrap().cost, 50);
    }

    #[test]
    fn test_service_catalog_get_service_not_found() {
        let catalog = ServiceCatalog::new();

        let service = catalog.get_service("resurrect");

        assert!(
            service.is_none(),
            "get_service should return None for a non-existent service"
        );
    }

    #[test]
    fn test_service_catalog_has_service() {
        let mut catalog = ServiceCatalog::new();
        catalog.services.push(ServiceEntry::new(
            "cure_poison",
            25,
            "Cure poison from one character",
        ));

        assert!(
            catalog.has_service("cure_poison"),
            "has_service should return true for an existing service"
        );
        assert!(
            !catalog.has_service("nonexistent"),
            "has_service should return false for a missing service"
        );
    }

    #[test]
    fn test_service_catalog_empty_has_no_services() {
        let catalog = ServiceCatalog::new();
        assert!(!catalog.has_service("any_service"));
    }

    // ----- NpcEconomySettings tests -----

    #[test]
    fn test_npc_economy_settings_default() {
        let economy = NpcEconomySettings::default();
        assert_eq!(economy.buy_rate, 0.5);
        assert_eq!(economy.sell_rate, 1.0);
        assert!(economy.max_buy_value.is_none());
    }

    #[test]
    fn test_npc_economy_settings_npc_buy_price() {
        let economy = NpcEconomySettings::new(0.5, 1.0);
        assert_eq!(economy.npc_buy_price(100), 50);
    }

    #[test]
    fn test_npc_economy_settings_npc_buy_price_minimum_one() {
        let economy = NpcEconomySettings::new(0.5, 1.0);
        assert_eq!(economy.npc_buy_price(0), 1);
    }

    #[test]
    fn test_npc_economy_settings_npc_buy_price_with_cap() {
        let mut economy = NpcEconomySettings::new(0.5, 1.0);
        economy.max_buy_value = Some(30);

        // 0.5 * 200 = 100, but cap is 30
        assert_eq!(economy.npc_buy_price(200), 30);
    }

    #[test]
    fn test_npc_economy_settings_npc_sell_price() {
        let economy = NpcEconomySettings::new(0.5, 1.5);
        assert_eq!(economy.npc_sell_price(100), 150);
    }

    #[test]
    fn test_npc_economy_settings_npc_sell_price_minimum_one() {
        let economy = NpcEconomySettings::new(0.5, 1.0);
        assert_eq!(economy.npc_sell_price(0), 1);
    }

    // ----- InventoryOwner tests -----

    #[test]
    fn test_inventory_owner_character_variant() {
        let owner = InventoryOwner::Character(2);
        assert_eq!(owner, InventoryOwner::Character(2));
    }

    #[test]
    fn test_inventory_owner_npc_variant() {
        let owner = InventoryOwner::Npc("merchant_tom".to_string());
        assert_eq!(owner, InventoryOwner::Npc("merchant_tom".to_string()));
    }

    #[test]
    fn test_inventory_owner_serialization_roundtrip() {
        let owner = InventoryOwner::Npc("blacksmith".to_string());
        let serialized = ron::to_string(&owner).expect("Failed to serialize");
        let deserialized: InventoryOwner =
            ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(owner, deserialized);
    }

    // ----- StockEntry tests -----

    #[test]
    fn test_stock_entry_new() {
        let entry = StockEntry::new(7, 3);
        assert_eq!(entry.item_id, 7);
        assert_eq!(entry.quantity, 3);
        assert!(entry.override_price.is_none());
    }

    #[test]
    fn test_stock_entry_with_price() {
        let entry = StockEntry::with_price(7, 3, 250);
        assert_eq!(entry.override_price, Some(250));
    }

    #[test]
    fn test_stock_entry_is_available() {
        let available = StockEntry::new(1, 1);
        let sold_out = StockEntry::new(2, 0);

        assert!(available.is_available());
        assert!(!sold_out.is_available());
    }

    #[test]
    fn test_stock_entry_serialization_roundtrip() {
        let entry = StockEntry::with_price(10, 5, 100);
        let serialized = ron::to_string(&entry).expect("Failed to serialize");
        let deserialized: StockEntry = ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(entry, deserialized);
    }

    // ----- ServiceEntry tests -----

    #[test]
    fn test_service_entry_new() {
        let service = ServiceEntry::new("rest", 10, "Rest overnight");
        assert_eq!(service.service_id, "rest");
        assert_eq!(service.cost, 10);
        assert_eq!(service.gem_cost, 0);
        assert_eq!(service.description, "Rest overnight");
    }

    #[test]
    fn test_service_entry_with_gem_cost() {
        let service =
            ServiceEntry::with_gem_cost("resurrect", 500, 1, "Resurrect a dead character");
        assert_eq!(service.gem_cost, 1);
        assert_eq!(service.cost, 500);
    }

    #[test]
    fn test_service_entry_serialization_roundtrip() {
        let service = ServiceEntry::with_gem_cost("resurrect", 1000, 2, "Resurrect a character");
        let serialized = ron::to_string(&service).expect("Failed to serialize");
        let deserialized: ServiceEntry = ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(service, deserialized);
    }

    // ----- MerchantStock default -----

    #[test]
    fn test_merchant_stock_default() {
        let stock = MerchantStock::default();
        assert!(stock.entries.is_empty());
        assert!(stock.restock_template.is_none());
    }

    // ----- ServiceCatalog serialization -----

    #[test]
    fn test_service_catalog_serialization_roundtrip() {
        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal all party members"));
        catalog.services.push(ServiceEntry::with_gem_cost(
            "resurrect",
            500,
            1,
            "Resurrect a dead character",
        ));

        let serialized = ron::to_string(&catalog).expect("Failed to serialize");
        let deserialized: ServiceCatalog =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(catalog, deserialized);
    }
}
