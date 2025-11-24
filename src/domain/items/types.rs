// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item system types - Weapons, armor, and item definitions
//!
//! This module defines all item-related data structures as specified in
//! architecture.md Section 4.5.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.5 for complete specifications.

use crate::domain::types::{DiceRoll, ItemId, SpellId};
use serde::{Deserialize, Serialize};
use std::fmt;

// ===== Item Type Enum =====

/// Main item type discriminator
///
/// # Examples
///
/// ```
/// use antares::domain::items::{ItemType, WeaponData};
/// use antares::domain::types::DiceRoll;
///
/// let weapon_type = ItemType::Weapon(WeaponData {
///     damage: DiceRoll::new(1, 8, 0),
///     bonus: 1,
///     hands_required: 1,
/// });
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemType {
    Weapon(WeaponData),
    Armor(ArmorData),
    Accessory(AccessoryData),
    Consumable(ConsumableData),
    Ammo(AmmoData),
    Quest(QuestData),
}

// ===== Weapon Data =====

/// Weapon-specific data
///
/// # Examples
///
/// ```
/// use antares::domain::items::WeaponData;
/// use antares::domain::types::DiceRoll;
///
/// // Club: 1d3 damage, no bonus, 1-handed
/// let club = WeaponData {
///     damage: DiceRoll::new(1, 3, 0),
///     bonus: 0,
///     hands_required: 1,
/// };
///
/// // Two-handed sword: 2d6 damage, +2 bonus, 2-handed
/// let greatsword = WeaponData {
///     damage: DiceRoll::new(2, 6, 0),
///     bonus: 2,
///     hands_required: 2,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponData {
    /// Base weapon damage (e.g., 1d8 for longsword)
    pub damage: DiceRoll,
    /// Bonus to-hit and damage (e.g., +1 for a +1 sword)
    pub bonus: i8,
    /// Number of hands required (1 or 2)
    pub hands_required: u8,
}

// ===== Armor Data =====

/// Armor-specific data
///
/// # Examples
///
/// ```
/// use antares::domain::items::ArmorData;
///
/// // Leather armor: +2 AC, light
/// let leather = ArmorData {
///     ac_bonus: 2,
///     weight: 15,
/// };
///
/// // Plate mail: +8 AC, heavy
/// let plate = ArmorData {
///     ac_bonus: 8,
///     weight: 50,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Armor class bonus (higher is better)
    pub ac_bonus: u8,
    /// Weight in pounds (affects movement)
    pub weight: u8,
}

// ===== Accessory Data =====

/// Accessory-specific data (rings, amulets, belts)
///
/// # Examples
///
/// ```
/// use antares::domain::items::{AccessoryData, AccessorySlot};
///
/// let ring = AccessoryData {
///     slot: AccessorySlot::Ring,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessoryData {
    /// Which accessory slot this occupies
    pub slot: AccessorySlot,
}

/// Accessory equipment slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessorySlot {
    Ring,
    Amulet,
    Belt,
    Cloak,
}

// ===== Consumable Data =====

/// Consumable item data (potions, scrolls)
///
/// # Examples
///
/// ```
/// use antares::domain::items::{ConsumableData, ConsumableEffect};
///
/// let healing_potion = ConsumableData {
///     effect: ConsumableEffect::HealHp(20),
///     is_combat_usable: true,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effect when consumed
    pub effect: ConsumableEffect,
    /// Whether usable during combat
    pub is_combat_usable: bool,
}

/// Effects from consuming items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsumableEffect {
    HealHp(u16),
    RestoreSp(u16),
    CureCondition(u8), // Condition flags to clear
    BoostAttribute(AttributeType, i8),
}

/// Attribute types that can be boosted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeType {
    Might,
    Intellect,
    Personality,
    Endurance,
    Speed,
    Accuracy,
    Luck,
}

// ===== Ammo Data =====

/// Ammunition data (arrows, bolts)
///
/// # Examples
///
/// ```
/// use antares::domain::items::{AmmoData, AmmoType};
///
/// let arrows = AmmoData {
///     ammo_type: AmmoType::Arrow,
///     quantity: 20,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmmoData {
    /// Type of ammunition
    pub ammo_type: AmmoType,
    /// Number of shots in this bundle
    pub quantity: u16,
}

/// Types of ammunition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmmoType {
    Arrow,
    Bolt,
    Stone,
}

// ===== Quest Data =====

/// Quest item data
///
/// # Examples
///
/// ```
/// use antares::domain::items::QuestData;
///
/// let ruby_whistle = QuestData {
///     quest_id: "brothers_quest".to_string(),
///     is_key_item: true,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestData {
    /// Quest identifier this item belongs to
    pub quest_id: String,
    /// Whether this item is required and cannot be sold/dropped
    pub is_key_item: bool,
}

// ===== Bonus System =====

/// Attribute bonus (for magical items)
///
/// # Examples
///
/// ```
/// use antares::domain::items::{Bonus, BonusAttribute};
///
/// // +5 Fire Resistance
/// let fire_resist = Bonus {
///     attribute: BonusAttribute::ResistFire,
///     value: 5,
/// };
///
/// // +3 Might
/// let might_boost = Bonus {
///     attribute: BonusAttribute::Might,
///     value: 3,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bonus {
    /// Which attribute is affected
    pub attribute: BonusAttribute,
    /// Bonus value (can be negative for curses)
    pub value: i8,
}

/// Attributes that can receive bonuses from items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BonusAttribute {
    // Primary Stats
    Might,
    Intellect,
    Personality,
    Endurance,
    Speed,
    Accuracy,
    Luck,
    // Resistances
    ResistFire,
    ResistCold,
    ResistElectricity,
    ResistAcid,
    ResistPoison,
    ResistMagic,
    // Other
    ArmorClass,
}

// ===== Disablement Flags =====

/// Class/alignment restrictions for items (bitfield)
///
/// Bit layout (MM1 style):
/// - Bit 0: Knight
/// - Bit 1: Paladin
/// - Bit 2: Archer
/// - Bit 3: Cleric
/// - Bit 4: Sorcerer
/// - Bit 5: Robber (Thief)
/// - Bit 6: Good alignment only
/// - Bit 7: Evil alignment only
///
/// # Examples
///
/// ```
/// use antares::domain::items::Disablement;
///
/// // All classes can use (0xFF)
/// let universal = Disablement(0xFF);
///
/// // Knight, Paladin, Archer, Robber (0b00101011 = 0x2B)
/// let martial = Disablement(0x2B);
///
/// // Cleric/Paladin only, Good alignment (0b01001010 = 0x4A)
/// let holy = Disablement(0x4A);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disablement(pub u8);

impl Disablement {
    /// All classes can use
    pub const ALL: Self = Self(0xFF);

    /// No classes can use (quest items)
    pub const NONE: Self = Self(0x00);

    // Class flags
    pub const KNIGHT: u8 = 0b0000_0001;
    pub const PALADIN: u8 = 0b0000_0010;
    pub const ARCHER: u8 = 0b0000_0100;
    pub const CLERIC: u8 = 0b0000_1000;
    pub const SORCERER: u8 = 0b0001_0000;
    pub const ROBBER: u8 = 0b0010_0000;
    pub const GOOD: u8 = 0b0100_0000;
    pub const EVIL: u8 = 0b1000_0000;

    /// Check if a specific class can use this item
    pub fn can_use_class(&self, class_bit: u8) -> bool {
        (self.0 & class_bit) != 0
    }

    /// Check if good alignment can use
    pub fn good_only(&self) -> bool {
        (self.0 & Self::GOOD) != 0
    }

    /// Check if evil alignment can use
    pub fn evil_only(&self) -> bool {
        (self.0 & Self::EVIL) != 0
    }
}

// ===== Complete Item Definition =====

/// Complete item definition
///
/// This is the main item struct that gets serialized from data files.
///
/// # Examples
///
/// ```
/// use antares::domain::items::{Item, ItemType, WeaponData, Disablement};
/// use antares::domain::types::DiceRoll;
///
/// let club = Item {
///     id: 1,
///     name: "Club".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 3, 0),
///         bonus: 0,
///         hands_required: 1,
///     }),
///     base_cost: 1,
///     sell_cost: 0,
///     disablements: Disablement::ALL,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// Unique item identifier
    pub id: ItemId,
    /// Display name
    pub name: String,
    /// Item type and type-specific data
    pub item_type: ItemType,
    /// Base purchase cost in gold
    pub base_cost: u32,
    /// Sell value in gold
    pub sell_cost: u32,
    /// Class/alignment restrictions
    pub disablements: Disablement,
    /// Permanent bonus while equipped/carried
    pub constant_bonus: Option<Bonus>,
    /// Temporary bonus when used (consumes charges)
    pub temporary_bonus: Option<Bonus>,
    /// Spell effect when used (encoded: high byte = school, low byte = spell)
    pub spell_effect: Option<SpellId>,
    /// Maximum charges for magical effects (0 = non-magical)
    pub max_charges: u16,
    /// Whether the item is cursed (cannot unequip)
    pub is_cursed: bool,
    /// Path to item icon asset (optional)
    pub icon_path: Option<String>,
}

impl Item {
    /// Check if this is a weapon
    pub fn is_weapon(&self) -> bool {
        matches!(self.item_type, ItemType::Weapon(_))
    }

    /// Check if this is armor
    pub fn is_armor(&self) -> bool {
        matches!(self.item_type, ItemType::Armor(_))
    }

    /// Check if this is an accessory
    pub fn is_accessory(&self) -> bool {
        matches!(self.item_type, ItemType::Accessory(_))
    }

    /// Check if this is consumable
    pub fn is_consumable(&self) -> bool {
        matches!(self.item_type, ItemType::Consumable(_))
    }

    /// Check if this is ammunition
    pub fn is_ammo(&self) -> bool {
        matches!(self.item_type, ItemType::Ammo(_))
    }

    /// Check if this is a quest item
    pub fn is_quest_item(&self) -> bool {
        matches!(self.item_type, ItemType::Quest(_))
    }

    /// Check if this item has magical effects
    pub fn is_magical(&self) -> bool {
        self.max_charges > 0
            || self.constant_bonus.is_some()
            || self.temporary_bonus.is_some()
            || self.spell_effect.is_some()
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.is_cursed {
            write!(f, " (Cursed)")?;
        }
        if self.is_magical() {
            write!(f, " *")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disablement_all_classes() {
        let dis = Disablement::ALL;
        assert!(dis.can_use_class(Disablement::KNIGHT));
        assert!(dis.can_use_class(Disablement::SORCERER));
        assert!(dis.can_use_class(Disablement::ROBBER));
    }

    #[test]
    fn test_disablement_knight_only() {
        let dis = Disablement(Disablement::KNIGHT);
        assert!(dis.can_use_class(Disablement::KNIGHT));
        assert!(!dis.can_use_class(Disablement::SORCERER));
    }

    #[test]
    fn test_disablement_good_alignment() {
        let dis = Disablement(Disablement::PALADIN | Disablement::GOOD);
        assert!(dis.good_only());
        assert!(!dis.evil_only());
    }

    #[test]
    fn test_item_type_checks() {
        let weapon = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 10,
            sell_cost: 5,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        assert!(weapon.is_weapon());
        assert!(!weapon.is_armor());
        assert!(!weapon.is_magical());
    }

    #[test]
    fn test_magical_item_detection() {
        let magical_sword = Item {
            id: 2,
            name: "Flaming Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 3,
                hands_required: 1,
            }),
            base_cost: 500,
            sell_cost: 250,
            disablements: Disablement::ALL,
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::ResistFire,
                value: 20,
            }),
            temporary_bonus: None,
            spell_effect: Some(0x0104),
            max_charges: 30,
            is_cursed: false,
            icon_path: None,
        };

        assert!(magical_sword.is_magical());
        assert!(magical_sword.is_weapon());
    }

    #[test]
    fn test_item_display() {
        let item = Item {
            id: 1,
            name: "Basic Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 10,
            sell_cost: 5,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        assert_eq!(item.to_string(), "Basic Sword");
    }

    #[test]
    fn test_cursed_item_display() {
        let cursed = Item {
            id: 2,
            name: "Cursed Mace".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
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
            is_cursed: true,
            icon_path: None,
        };

        assert_eq!(cursed.to_string(), "Cursed Mace (Cursed)");
    }
}
