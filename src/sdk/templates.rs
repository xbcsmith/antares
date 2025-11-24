// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Content templates for quick creation of game content
//!
//! This module provides pre-configured templates for creating common game
//! content types like weapons, armor, maps, and other entities. These templates
//! serve as starting points that can be customized for specific needs.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_implementation_plan.md` Phase 3.5 for specifications.
//!
//! # Examples
//!
//! ```
//! use antares::sdk::templates::{basic_weapon, basic_armor, town_map, dungeon_map};
//! use antares::domain::types::DiceRoll;
//!
//! // Create a basic weapon template
//! let sword = basic_weapon(1, "Iron Sword", DiceRoll::new(1, 8, 0));
//! assert_eq!(sword.name, "Iron Sword");
//!
//! // Create a basic armor template
//! let leather = basic_armor(2, "Leather Armor", 2);
//! assert_eq!(leather.name, "Leather Armor");
//! ```

use crate::domain::items::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorData, Bonus, BonusAttribute,
    ConsumableData, ConsumableEffect, Disablement, Item, ItemType, QuestData, WeaponData,
};
use crate::domain::types::{DiceRoll, ItemId, MapId};
use crate::domain::world::{Map, TerrainType, Tile, WallType};

// ===== Weapon Templates =====

/// Creates a basic weapon template
///
/// # Arguments
///
/// * `id` - Item ID
/// * `name` - Weapon name
/// * `damage` - Damage dice roll (e.g., 1d8+0)
///
/// # Returns
///
/// Returns an `Item` configured as a basic weapon with no special properties.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::basic_weapon;
/// use antares::domain::types::DiceRoll;
///
/// let sword = basic_weapon(1, "Longsword", DiceRoll::new(1, 8, 2));
/// assert_eq!(sword.name, "Longsword");
/// assert!(sword.is_weapon());
/// ```
pub fn basic_weapon(id: ItemId, name: &str, damage: DiceRoll) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage,
            bonus: 0,
            hands_required: 1,
        }),
        base_cost: 100,
        sell_cost: 50,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a two-handed weapon template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::two_handed_weapon;
/// use antares::domain::types::DiceRoll;
///
/// let greatsword = two_handed_weapon(2, "Greatsword", DiceRoll::new(2, 6, 0));
/// assert_eq!(greatsword.name, "Greatsword");
/// ```
pub fn two_handed_weapon(id: ItemId, name: &str, damage: DiceRoll) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage,
            bonus: 0,
            hands_required: 2,
        }),
        base_cost: 200,
        sell_cost: 100,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a magical weapon template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::magical_weapon;
/// use antares::domain::types::DiceRoll;
///
/// let flame_sword = magical_weapon(3, "Flame Sword", DiceRoll::new(1, 8, 0), 2);
/// assert_eq!(flame_sword.name, "Flame Sword");
/// assert!(flame_sword.is_magical());
/// ```
pub fn magical_weapon(id: ItemId, name: &str, damage: DiceRoll, bonus: i8) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage,
            bonus,
            hands_required: 1,
        }),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement::ALL,
        constant_bonus: Some(Bonus {
            attribute: BonusAttribute::Might,
            value: 1,
        }),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Armor Templates =====

/// Creates a basic armor template
///
/// # Arguments
///
/// * `id` - Item ID
/// * `name` - Armor name
/// * `ac_bonus` - Armor class bonus
///
/// # Returns
///
/// Returns an `Item` configured as basic armor.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::basic_armor;
///
/// let chainmail = basic_armor(10, "Chainmail", 4);
/// assert_eq!(chainmail.name, "Chainmail");
/// assert!(chainmail.is_armor());
/// ```
pub fn basic_armor(id: ItemId, name: &str, ac_bonus: u8) -> Item {
    let weight = ac_bonus * 5;
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Armor(ArmorData { ac_bonus, weight }),
        base_cost: (ac_bonus as u32) * 50,
        sell_cost: (ac_bonus as u32) * 25,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a shield template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::shield;
///
/// let wooden_shield = shield(11, "Wooden Shield", 1);
/// assert_eq!(wooden_shield.name, "Wooden Shield");
/// ```
pub fn shield(id: ItemId, name: &str, ac_bonus: u8) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus,
            weight: 10,
        }),
        base_cost: 50,
        sell_cost: 25,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates magical armor template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::magical_armor;
///
/// let enchanted_mail = magical_armor(12, "Enchanted Mail", 5, 2);
/// assert_eq!(enchanted_mail.name, "Enchanted Mail");
/// assert!(enchanted_mail.is_magical());
/// ```
pub fn magical_armor(id: ItemId, name: &str, ac_bonus: u8, magic_bonus: i8) -> Item {
    let weight = ac_bonus * 5;
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Armor(ArmorData { ac_bonus, weight }),
        base_cost: (ac_bonus as u32) * 100 + (magic_bonus as u32) * 200,
        sell_cost: (ac_bonus as u32) * 50 + (magic_bonus as u32) * 100,
        disablements: Disablement::ALL,
        constant_bonus: Some(Bonus {
            attribute: BonusAttribute::ArmorClass,
            value: magic_bonus,
        }),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Accessory Templates =====

/// Creates a basic ring template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::basic_ring;
/// use antares::domain::items::{Bonus, BonusAttribute};
///
/// let ring = basic_ring(20, "Ring of Strength", Bonus {
///     attribute: BonusAttribute::Might,
///     value: 2,
/// });
/// assert_eq!(ring.name, "Ring of Strength");
/// ```
pub fn basic_ring(id: ItemId, name: &str, bonus: Bonus) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Accessory(AccessoryData {
            slot: AccessorySlot::Ring,
        }),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement::ALL,
        constant_bonus: Some(bonus),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a basic amulet template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::basic_amulet;
/// use antares::domain::items::{Bonus, BonusAttribute};
///
/// let amulet = basic_amulet(21, "Amulet of Protection", Bonus {
///     attribute: BonusAttribute::ArmorClass,
///     value: 3,
/// });
/// assert_eq!(amulet.name, "Amulet of Protection");
/// ```
pub fn basic_amulet(id: ItemId, name: &str, bonus: Bonus) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Accessory(AccessoryData {
            slot: AccessorySlot::Amulet,
        }),
        base_cost: 750,
        sell_cost: 375,
        disablements: Disablement::ALL,
        constant_bonus: Some(bonus),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Consumable Templates =====

/// Creates a healing potion template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::healing_potion;
///
/// let potion = healing_potion(30, "Healing Potion", 20);
/// assert_eq!(potion.name, "Healing Potion");
/// ```
pub fn healing_potion(id: ItemId, name: &str, healing_amount: u16) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::HealHp(healing_amount),
            is_combat_usable: true,
        }),
        base_cost: (healing_amount as u32) * 5,
        sell_cost: (healing_amount as u32) * 2,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a spell point restoration potion template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::sp_potion;
///
/// let potion = sp_potion(31, "Mana Potion", 10);
/// assert_eq!(potion.name, "Mana Potion");
/// ```
pub fn sp_potion(id: ItemId, name: &str, sp_amount: u16) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::RestoreSp(sp_amount),
            is_combat_usable: true,
        }),
        base_cost: (sp_amount as u32) * 10,
        sell_cost: (sp_amount as u32) * 5,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Ammo Templates =====

/// Creates an arrow bundle template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::arrow_bundle;
///
/// let arrows = arrow_bundle(40, 20);
/// assert!(arrows.name.contains("Arrow"));
/// ```
pub fn arrow_bundle(id: ItemId, count: u16) -> Item {
    Item {
        id,
        name: format!("Arrows ({})", count),
        item_type: ItemType::Ammo(AmmoData {
            ammo_type: AmmoType::Arrow,
            quantity: count,
        }),
        base_cost: (count as u32) * 2,
        sell_cost: count as u32,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

/// Creates a bolt bundle template (for crossbows)
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::bolt_bundle;
///
/// let bolts = bolt_bundle(41, 15);
/// assert!(bolts.name.contains("Bolt"));
/// ```
pub fn bolt_bundle(id: ItemId, count: u16) -> Item {
    Item {
        id,
        name: format!("Bolts ({})", count),
        item_type: ItemType::Ammo(AmmoData {
            ammo_type: AmmoType::Bolt,
            quantity: count,
        }),
        base_cost: (count as u32) * 3,
        sell_cost: (count as u32) * 2,
        disablements: Disablement::ALL,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Quest Item Templates =====

/// Creates a quest item template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::quest_item;
///
/// let key = quest_item(50, "Ancient Key", "brothers_quest");
/// assert_eq!(key.name, "Ancient Key");
/// ```
pub fn quest_item(id: ItemId, name: &str, quest_id: &str) -> Item {
    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Quest(QuestData {
            quest_id: quest_id.to_string(),
            is_key_item: true,
        }),
        base_cost: 0,
        sell_cost: 0,
        disablements: Disablement::NONE,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    }
}

// ===== Map Templates =====

/// Creates a town map template
///
/// # Arguments
///
/// * `id` - Map ID
/// * `name` - Town name
/// * `width` - Map width
/// * `height` - Map height
///
/// # Returns
///
/// Returns a `Map` configured as a basic town with grass terrain.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::town_map;
///
/// let town = town_map(1, "Starting Village", 20, 20);
/// assert_eq!(town.id, 1);
/// assert_eq!(town.width, 20);
/// assert_eq!(town.height, 20);
/// ```
pub fn town_map(id: MapId, _name: &str, width: u32, height: u32) -> Map {
    // Create 2D tile grid filled with grass
    let tiles = (0..(width * height))
        .map(|_| Tile::new(TerrainType::Grass, WallType::None))
        .collect();

    Map {
        id,
        width,
        height,
        tiles,
        events: std::collections::HashMap::new(),
        npcs: Vec::new(),
    }
}

/// Creates a dungeon map template
///
/// # Arguments
///
/// * `id` - Map ID
/// * `name` - Dungeon name
/// * `width` - Map width
/// * `height` - Map height
///
/// # Returns
///
/// Returns a `Map` configured as a basic dungeon with stone floor.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::dungeon_map;
///
/// let dungeon = dungeon_map(2, "Dark Cavern", 30, 30);
/// assert_eq!(dungeon.id, 2);
/// assert_eq!(dungeon.width, 30);
/// assert_eq!(dungeon.height, 30);
/// ```
pub fn dungeon_map(id: MapId, _name: &str, width: u32, height: u32) -> Map {
    // Create 2D tile grid filled with stone floor
    let tiles = (0..(width * height))
        .map(|_| Tile::new(TerrainType::Stone, WallType::None))
        .collect();

    Map {
        id,
        width,
        height,
        tiles,
        events: std::collections::HashMap::new(),
        npcs: Vec::new(),
    }
}

/// Creates a forest map template
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::forest_map;
///
/// let forest = forest_map(3, "Darkwood Forest", 40, 40);
/// assert_eq!(forest.id, 3);
/// assert_eq!(forest.width, 40);
/// assert_eq!(forest.height, 40);
/// ```
pub fn forest_map(id: MapId, _name: &str, width: u32, height: u32) -> Map {
    // Create 2D tile grid filled with forest terrain
    let tiles = (0..(width * height))
        .map(|_| Tile::new(TerrainType::Forest, WallType::None))
        .collect();

    Map {
        id,
        width,
        height,
        tiles,
        events: std::collections::HashMap::new(),
        npcs: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_weapon() {
        let sword = basic_weapon(1, "Sword", DiceRoll::new(1, 8, 0));
        assert_eq!(sword.id, 1);
        assert_eq!(sword.name, "Sword");
        assert!(sword.is_weapon());
        assert!(!sword.is_cursed);
    }

    #[test]
    fn test_two_handed_weapon() {
        let weapon = two_handed_weapon(2, "Greatsword", DiceRoll::new(2, 6, 0));
        assert_eq!(weapon.name, "Greatsword");
        assert!(weapon.is_weapon());
        if let ItemType::Weapon(data) = weapon.item_type {
            assert_eq!(data.hands_required, 2);
        } else {
            panic!("Expected weapon type");
        }
    }

    #[test]
    fn test_magical_weapon() {
        let sword = magical_weapon(3, "Flame Sword", DiceRoll::new(1, 8, 0), 2);
        assert_eq!(sword.name, "Flame Sword");
        assert!(sword.is_magical());
        assert!(sword.constant_bonus.is_some());
    }

    #[test]
    fn test_basic_armor() {
        let armor = basic_armor(10, "Chainmail", 4);
        assert_eq!(armor.id, 10);
        assert_eq!(armor.name, "Chainmail");
        assert!(armor.is_armor());
    }

    #[test]
    fn test_shield() {
        let shield_item = shield(11, "Wooden Shield", 1);
        assert_eq!(shield_item.name, "Wooden Shield");
        assert!(shield_item.is_armor());
    }

    #[test]
    fn test_magical_armor() {
        let armor = magical_armor(12, "Enchanted Mail", 5, 2);
        assert_eq!(armor.name, "Enchanted Mail");
        assert!(armor.is_magical());
        assert!(armor.constant_bonus.is_some());
    }

    #[test]
    fn test_basic_ring() {
        let bonus = Bonus {
            attribute: BonusAttribute::Might,
            value: 2,
        };
        let ring = basic_ring(20, "Ring of Strength", bonus);
        assert_eq!(ring.name, "Ring of Strength");
        assert_eq!(ring.constant_bonus, Some(bonus));
        assert!(ring.is_accessory());
    }

    #[test]
    fn test_basic_amulet() {
        let bonus = Bonus {
            attribute: BonusAttribute::ArmorClass,
            value: 3,
        };
        let amulet = basic_amulet(21, "Amulet of Protection", bonus);
        assert_eq!(amulet.name, "Amulet of Protection");
        assert_eq!(amulet.constant_bonus, Some(bonus));
        assert!(amulet.is_accessory());
    }

    #[test]
    fn test_healing_potion() {
        let potion = healing_potion(30, "Healing Potion", 20);
        assert_eq!(potion.name, "Healing Potion");
        assert!(potion.is_consumable());
        if let ItemType::Consumable(data) = potion.item_type {
            assert!(matches!(data.effect, ConsumableEffect::HealHp(20)));
        } else {
            panic!("Expected consumable type");
        }
    }

    #[test]
    fn test_sp_potion() {
        let potion = sp_potion(31, "Mana Potion", 10);
        assert_eq!(potion.name, "Mana Potion");
        assert!(potion.is_consumable());
        if let ItemType::Consumable(data) = potion.item_type {
            assert!(matches!(data.effect, ConsumableEffect::RestoreSp(10)));
        } else {
            panic!("Expected consumable type");
        }
    }

    #[test]
    fn test_arrow_bundle() {
        let arrows = arrow_bundle(40, 20);
        assert!(arrows.name.contains("Arrow"));
        assert!(arrows.is_ammo());
        if let ItemType::Ammo(data) = arrows.item_type {
            assert_eq!(data.ammo_type, AmmoType::Arrow);
            assert_eq!(data.quantity, 20);
        } else {
            panic!("Expected ammo type");
        }
    }

    #[test]
    fn test_bolt_bundle() {
        let bolts = bolt_bundle(41, 15);
        assert!(bolts.name.contains("Bolt"));
        assert!(bolts.is_ammo());
        if let ItemType::Ammo(data) = bolts.item_type {
            assert_eq!(data.ammo_type, AmmoType::Bolt);
            assert_eq!(data.quantity, 15);
        } else {
            panic!("Expected ammo type");
        }
    }

    #[test]
    fn test_quest_item() {
        let key = quest_item(50, "Ancient Key", "brothers_quest");
        assert_eq!(key.name, "Ancient Key");
        assert!(key.is_quest_item());
        assert_eq!(key.base_cost, 0);
    }

    #[test]
    fn test_town_map() {
        let town = town_map(1, "Village", 20, 20);
        assert_eq!(town.id, 1);
        assert_eq!(town.width, 20);
        assert_eq!(town.height, 20);
        assert_eq!(town.tiles.len(), 400);
    }

    #[test]
    fn test_dungeon_map() {
        let dungeon = dungeon_map(2, "Cave", 30, 30);
        assert_eq!(dungeon.id, 2);
        assert_eq!(dungeon.width, 30);
        assert_eq!(dungeon.height, 30);
        assert_eq!(dungeon.tiles.len(), 900);
    }

    #[test]
    fn test_forest_map() {
        let forest = forest_map(3, "Forest", 40, 40);
        assert_eq!(forest.id, 3);
        assert_eq!(forest.width, 40);
        assert_eq!(forest.height, 40);
        assert_eq!(forest.tiles.len(), 1600);
    }
    #[test]
    fn test_weapon_costs() {
        let sword = basic_weapon(1, "Sword", DiceRoll::new(1, 8, 0));
        assert!(sword.base_cost > 0);
        assert!(sword.sell_cost > 0);
        assert!(sword.sell_cost < sword.base_cost);
    }

    #[test]
    fn test_armor_scaling() {
        let light = basic_armor(1, "Light", 2);
        let heavy = basic_armor(2, "Heavy", 6);
        // Higher AC bonus should mean higher cost
        assert!(heavy.base_cost > light.base_cost);
    }

    #[test]
    fn test_cursed_items_not_created() {
        let sword = basic_weapon(1, "Sword", DiceRoll::new(1, 8, 0));
        assert!(!sword.is_cursed);

        let armor = basic_armor(2, "Armor", 5);
        assert!(!armor.is_cursed);
    }
}
