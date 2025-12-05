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

use crate::domain::character::Alignment;
use crate::domain::classes::ClassDatabase;
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

// ===== Classification Enums =====

/// Weapon classification determines proficiency requirement
///
/// Each weapon belongs to exactly one classification, which maps to a
/// proficiency requirement. Classes and races grant proficiencies that
/// allow use of items with matching classifications.
///
/// # Examples
///
/// ```
/// use antares::domain::items::WeaponClassification;
///
/// let classification = WeaponClassification::MartialMelee;
/// assert_ne!(classification, WeaponClassification::Simple);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WeaponClassification {
    /// Basic weapons anyone can use: clubs, daggers, staffs
    #[default]
    Simple,
    /// Advanced melee weapons: swords, axes, maces (fighters, paladins)
    MartialMelee,
    /// Ranged weapons: bows, crossbows (archers, rangers)
    MartialRanged,
    /// Weapons without edge: maces, hammers, staffs (clerics)
    Blunt,
    /// Unarmed combat: fists, martial arts (monks)
    Unarmed,
}

/// Armor classification determines proficiency requirement
///
/// Each armor piece belongs to exactly one classification, which maps to a
/// proficiency requirement.
///
/// # Examples
///
/// ```
/// use antares::domain::items::ArmorClassification;
///
/// let classification = ArmorClassification::Heavy;
/// assert_ne!(classification, ArmorClassification::Light);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ArmorClassification {
    /// Light armor: leather, padded
    #[default]
    Light,
    /// Medium armor: chain mail, scale
    Medium,
    /// Heavy armor: plate mail, full plate
    Heavy,
    /// All shield types
    Shield,
}

/// Magic item classification for arcane vs divine items
///
/// Determines which classes can use magical items like wands, scrolls,
/// and holy symbols.
///
/// # Examples
///
/// ```
/// use antares::domain::items::MagicItemClassification;
///
/// let classification = MagicItemClassification::Arcane;
/// assert_ne!(classification, MagicItemClassification::Divine);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MagicItemClassification {
    /// Arcane items: wands, arcane scrolls (sorcerers)
    Arcane,
    /// Divine items: holy symbols, divine scrolls (clerics)
    Divine,
    /// Universal items: potions, rings (anyone)
    #[default]
    Universal,
}

/// Alignment restriction for items (separate from proficiency)
///
/// Some items can only be used by characters of specific alignments.
/// This is checked separately from proficiency requirements.
///
/// # Examples
///
/// ```
/// use antares::domain::items::AlignmentRestriction;
///
/// let restriction = AlignmentRestriction::GoodOnly;
/// assert_ne!(restriction, AlignmentRestriction::EvilOnly);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlignmentRestriction {
    /// Only good-aligned characters can use this item
    GoodOnly,
    /// Only evil-aligned characters can use this item
    EvilOnly,
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

    // Alignment flags - these are fixed concepts and remain as constants
    pub const GOOD: u8 = 0b0100_0000;
    pub const EVIL: u8 = 0b1000_0000;

    /// Check if a specific class can use this item using a raw bit value
    ///
    /// For data-driven class lookups, prefer `can_use_class_id()` instead.
    ///
    /// # Class Bit Mapping
    ///
    /// The standard class bit positions are:
    /// - Bit 0 (0b0000_0001): Knight
    /// - Bit 1 (0b0000_0010): Paladin
    /// - Bit 2 (0b0000_0100): Archer
    /// - Bit 3 (0b0000_1000): Cleric
    /// - Bit 4 (0b0001_0000): Sorcerer
    /// - Bit 5 (0b0010_0000): Robber
    ///
    /// Custom classes should use `ClassDefinition.disablement_bit_index` to
    /// determine their bit position, then calculate the mask as `1 << bit_index`.
    ///
    /// # Arguments
    ///
    /// * `class_bit` - The class bit mask (e.g., `0b0000_0001` for knight)
    ///
    /// # Returns
    ///
    /// Returns `true` if the class bit is set in the disablement mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    ///
    /// // Knight-only item
    /// let knight_item = Disablement(0b0000_0001);
    /// assert!(knight_item.can_use_class(0b0000_0001)); // Knight can use
    /// assert!(!knight_item.can_use_class(0b0001_0000)); // Sorcerer cannot
    ///
    /// // All classes item
    /// let universal = Disablement::ALL;
    /// assert!(universal.can_use_class(0b0000_0001)); // Knight can use
    /// assert!(universal.can_use_class(0b0001_0000)); // Sorcerer can use
    /// ```
    pub fn can_use_class(&self, class_bit: u8) -> bool {
        (self.0 & class_bit) != 0
    }

    /// Check if a class can use this item using data-driven lookup
    ///
    /// This is the preferred method for checking class restrictions as it
    /// supports dynamically loaded classes from the ClassDatabase.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier (e.g., "knight", "sorcerer")
    /// * `class_db` - Reference to the ClassDatabase containing class definitions
    ///
    /// # Returns
    ///
    /// Returns `true` if the class can use this item, `false` otherwise.
    /// Returns `false` if the class_id is not found in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "knight",
    ///         name: "Knight",
    ///         description: "A brave warrior",
    ///         hp_die: (count: 1, sides: 10, bonus: 0),
    ///         spell_school: None,
    ///         is_pure_caster: false,
    ///         spell_stat: None,
    ///         disablement_bit: 0,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    ///     (
    ///         id: "sorcerer",
    ///         name: "Sorcerer",
    ///         description: "A master of arcane magic",
    ///         hp_die: (count: 1, sides: 4, bonus: 0),
    ///         spell_school: Some(Sorcerer),
    ///         is_pure_caster: true,
    ///         spell_stat: Some(Intellect),
    ///         disablement_bit: 4,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    /// ]"#;
    ///
    /// let class_db = ClassDatabase::load_from_string(ron_data).unwrap();
    ///
    /// // Knight-only item (bit 0 set)
    /// let knight_item = Disablement(0b0000_0001);
    /// assert!(knight_item.can_use_class_id("knight", &class_db));
    /// assert!(!knight_item.can_use_class_id("sorcerer", &class_db));
    ///
    /// // All classes item
    /// let universal = Disablement::ALL;
    /// assert!(universal.can_use_class_id("knight", &class_db));
    /// assert!(universal.can_use_class_id("sorcerer", &class_db));
    /// ```
    pub fn can_use_class_id(&self, class_id: &str, class_db: &ClassDatabase) -> bool {
        match class_db.get_class(class_id) {
            Some(class_def) => {
                let mask = class_def.disablement_mask();
                (self.0 & mask) != 0
            }
            None => false, // Unknown class cannot use item
        }
    }

    /// Check if a specific alignment can use this item
    ///
    /// # Arguments
    ///
    /// * `alignment` - The character's alignment
    ///
    /// # Returns
    ///
    /// Returns `true` if the alignment restriction allows use, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    /// use antares::domain::character::Alignment;
    ///
    /// // Good-only item
    /// let holy_sword = Disablement(0b0100_0001); // Knight + Good
    /// assert!(holy_sword.can_use_alignment(Alignment::Good));
    /// assert!(!holy_sword.can_use_alignment(Alignment::Evil));
    /// assert!(!holy_sword.can_use_alignment(Alignment::Neutral));
    ///
    /// // Evil-only item
    /// let dark_blade = Disablement(0b1000_0001); // Knight + Evil
    /// assert!(!dark_blade.can_use_alignment(Alignment::Good));
    /// assert!(dark_blade.can_use_alignment(Alignment::Evil));
    /// assert!(!dark_blade.can_use_alignment(Alignment::Neutral));
    ///
    /// // No alignment restriction
    /// let normal_sword = Disablement(0b0000_0001); // Knight only, any alignment
    /// assert!(normal_sword.can_use_alignment(Alignment::Good));
    /// assert!(normal_sword.can_use_alignment(Alignment::Evil));
    /// assert!(normal_sword.can_use_alignment(Alignment::Neutral));
    /// ```
    pub fn can_use_alignment(&self, alignment: Alignment) -> bool {
        let good_required = self.good_only();
        let evil_required = self.evil_only();

        // If neither restriction is set, any alignment can use
        if !good_required && !evil_required {
            return true;
        }

        // Check specific alignment requirements
        match alignment {
            Alignment::Good => good_required,
            Alignment::Evil => evil_required,
            Alignment::Neutral => {
                // Neutral can only use items with no alignment restriction
                // If we reach here, an alignment restriction exists
                false
            }
        }
    }

    /// Check if a character with given class and alignment can use this item
    ///
    /// This is the comprehensive check combining class and alignment restrictions.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier (e.g., "knight", "sorcerer")
    /// * `alignment` - The character's alignment
    /// * `class_db` - Reference to the ClassDatabase
    ///
    /// # Returns
    ///
    /// Returns `true` if both class and alignment restrictions are satisfied.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    /// use antares::domain::character::Alignment;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "knight",
    ///         name: "Knight",
    ///         description: "A brave warrior",
    ///         hp_die: (count: 1, sides: 10, bonus: 0),
    ///         spell_school: None,
    ///         is_pure_caster: false,
    ///         spell_stat: None,
    ///         disablement_bit: 0,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    ///     (
    ///         id: "sorcerer",
    ///         name: "Sorcerer",
    ///         description: "A master of arcane magic",
    ///         hp_die: (count: 1, sides: 4, bonus: 0),
    ///         spell_school: Some(Sorcerer),
    ///         is_pure_caster: true,
    ///         spell_stat: Some(Intellect),
    ///         disablement_bit: 4,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    /// ]"#;
    ///
    /// let class_db = ClassDatabase::load_from_string(ron_data).unwrap();
    ///
    /// // Holy sword: Knight only, Good alignment required
    /// let holy_sword = Disablement(0b0100_0001);
    /// assert!(holy_sword.can_use("knight", Alignment::Good, &class_db));
    /// assert!(!holy_sword.can_use("knight", Alignment::Evil, &class_db));
    /// assert!(!holy_sword.can_use("sorcerer", Alignment::Good, &class_db));
    /// ```
    pub fn can_use(&self, class_id: &str, alignment: Alignment, class_db: &ClassDatabase) -> bool {
        self.can_use_class_id(class_id, class_db) && self.can_use_alignment(alignment)
    }

    /// Check if good alignment can use
    pub fn good_only(&self) -> bool {
        (self.0 & Self::GOOD) != 0
    }

    /// Check if evil alignment can use
    pub fn evil_only(&self) -> bool {
        (self.0 & Self::EVIL) != 0
    }

    /// Create a `Disablement` from a bit index (0..=7), producing the corresponding mask.
    ///
    /// This is useful when you have a bit position and need the bitmask.
    pub const fn from_index(index: u8) -> Self {
        // Constrain to 0..=7 by masking the index
        Self(1u8 << (index & 0x07))
    }

    /// Return the raw mask for a given bit index (0..=7).
    pub const fn mask_from_index(index: u8) -> u8 {
        1u8 << (index & 0x07)
    }

    /// If this Disablement represents a single bit, return its index.
    /// Returns `None` for 0 or multi-bit masks.
    pub fn to_index(&self) -> Option<u8> {
        let n = self.0;
        if n == 0 {
            return None;
        }
        // If multiple bits set, it's not a single index
        if n & (n - 1) != 0 {
            return None;
        }
        // trailing_zeros gives position of single bit
        Some(n.trailing_zeros() as u8)
    }

    /// Build a disablement mask from an iterator of class IDs
    ///
    /// This is useful when constructing disablement masks programmatically
    /// from a list of classes that CAN use an item.
    ///
    /// # Arguments
    ///
    /// * `class_ids` - Iterator of class IDs that should be able to use the item
    /// * `class_db` - Reference to the ClassDatabase
    ///
    /// # Returns
    ///
    /// A `Disablement` with the appropriate bits set for the given classes.
    /// Unknown class IDs are ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "knight",
    ///         name: "Knight",
    ///         description: "A brave warrior",
    ///         hp_die: (count: 1, sides: 10, bonus: 0),
    ///         spell_school: None,
    ///         is_pure_caster: false,
    ///         spell_stat: None,
    ///         disablement_bit: 0,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    ///     (
    ///         id: "paladin",
    ///         name: "Paladin",
    ///         description: "A holy warrior",
    ///         hp_die: (count: 1, sides: 8, bonus: 0),
    ///         spell_school: Some(Cleric),
    ///         is_pure_caster: false,
    ///         spell_stat: Some(Personality),
    ///         disablement_bit: 1,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    /// ]"#;
    ///
    /// let class_db = ClassDatabase::load_from_string(ron_data).unwrap();
    ///
    /// let classes = vec!["knight", "paladin"];
    /// let dis = Disablement::from_class_ids(classes.iter().map(|s| *s), &class_db);
    /// assert!(dis.can_use_class_id("knight", &class_db));
    /// assert!(dis.can_use_class_id("paladin", &class_db));
    /// ```
    pub fn from_class_ids<'a, I>(class_ids: I, class_db: &ClassDatabase) -> Self
    where
        I: Iterator<Item = &'a str>,
    {
        let mut mask = 0u8;
        for class_id in class_ids {
            if let Some(class_def) = class_db.get_class(class_id) {
                mask |= class_def.disablement_mask();
            }
        }
        Self(mask)
    }

    /// Get a list of class IDs that can use this item
    ///
    /// # Arguments
    ///
    /// * `class_db` - Reference to the ClassDatabase
    ///
    /// # Returns
    ///
    /// A vector of class IDs that have their disablement bit set in this mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Disablement;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "knight",
    ///         name: "Knight",
    ///         description: "A brave warrior",
    ///         hp_die: (count: 1, sides: 10, bonus: 0),
    ///         spell_school: None,
    ///         is_pure_caster: false,
    ///         spell_stat: None,
    ///         disablement_bit: 0,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    ///     (
    ///         id: "sorcerer",
    ///         name: "Sorcerer",
    ///         description: "A master of arcane magic",
    ///         hp_die: (count: 1, sides: 4, bonus: 0),
    ///         spell_school: Some(Sorcerer),
    ///         is_pure_caster: true,
    ///         spell_stat: Some(Intellect),
    ///         disablement_bit: 4,
    ///         special_abilities: [],
    ///         starting_weapon_id: None,
    ///         starting_armor_id: None,
    ///         starting_items: [],
    ///     ),
    /// ]"#;
    ///
    /// let class_db = ClassDatabase::load_from_string(ron_data).unwrap();
    ///
    /// // Knight-only item
    /// let knight_item = Disablement(0b0000_0001);
    /// let allowed = knight_item.allowed_class_ids(&class_db);
    /// assert!(allowed.contains(&"knight".to_string()));
    /// assert!(!allowed.contains(&"sorcerer".to_string()));
    /// ```
    pub fn allowed_class_ids(&self, class_db: &ClassDatabase) -> Vec<String> {
        class_db
            .all_classes()
            .filter(|class_def| {
                let mask = class_def.disablement_mask();
                (self.0 & mask) != 0
            })
            .map(|class_def| class_def.id.clone())
            .collect()
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
///     icon_path: None,
///     tags: vec![],
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
    #[serde(default)]
    pub icon_path: Option<String>,
    /// Arbitrary tags for fine-grained restrictions (e.g., "large_weapon", "two_handed")
    ///
    /// Standard tags by convention (not enforced):
    /// - `large_weapon` - Too big for small races (Halfling, Gnome)
    /// - `two_handed` - Requires both hands
    /// - `heavy_armor` - Encumbering armor
    /// - `elven_crafted` - Made by elves
    /// - `dwarven_crafted` - Made by dwarves
    /// - `requires_strength` - Needs high strength
    #[serde(default)]
    pub tags: Vec<String>,
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

    // Helper to create a test ClassDatabase with standard classes
    fn create_test_class_db() -> ClassDatabase {
        let ron_data = r#"[
            (
                id: "knight",
                name: "Knight",
                description: "A brave warrior",
                hp_die: (count: 1, sides: 10, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 0,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
            (
                id: "paladin",
                name: "Paladin",
                description: "A holy warrior",
                hp_die: (count: 1, sides: 8, bonus: 0),
                spell_school: Some(Cleric),
                is_pure_caster: false,
                spell_stat: Some(Personality),
                disablement_bit: 1,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
            (
                id: "archer",
                name: "Archer",
                description: "A skilled marksman",
                hp_die: (count: 1, sides: 8, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 2,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
            (
                id: "cleric",
                name: "Cleric",
                description: "A divine spellcaster",
                hp_die: (count: 1, sides: 6, bonus: 0),
                spell_school: Some(Cleric),
                is_pure_caster: true,
                spell_stat: Some(Personality),
                disablement_bit: 3,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
            (
                id: "sorcerer",
                name: "Sorcerer",
                description: "A master of arcane magic",
                hp_die: (count: 1, sides: 4, bonus: 0),
                spell_school: Some(Sorcerer),
                is_pure_caster: true,
                spell_stat: Some(Intellect),
                disablement_bit: 4,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
            (
                id: "robber",
                name: "Robber",
                description: "A sneaky thief",
                hp_die: (count: 1, sides: 6, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 5,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
        ]"#;
        ClassDatabase::load_from_string(ron_data).unwrap()
    }

    #[test]
    fn test_disablement_from_index_and_mask() {
        assert_eq!(Disablement::from_index(0).0, 1_u8);
        assert_eq!(Disablement::from_index(4).0, 1_u8 << 4);

        assert_eq!(Disablement::mask_from_index(0), 0b0000_0001);
        assert_eq!(Disablement::mask_from_index(3), 0b0000_1000);
    }

    #[test]
    fn disablement_to_index_examples() {
        assert_eq!(Disablement(0b0000_1000).to_index(), Some(3));
        assert_eq!(Disablement(0).to_index(), None);
        assert_eq!(Disablement(0b0000_0110).to_index(), None);
        assert_eq!(Disablement(0b1111_1111).to_index(), None);
    }

    // Class bit constants for testing (standard bit positions)
    const BIT_KNIGHT: u8 = 0b0000_0001;
    const BIT_PALADIN: u8 = 0b0000_0010;
    const BIT_ARCHER: u8 = 0b0000_0100;
    const BIT_CLERIC: u8 = 0b0000_1000;
    const BIT_SORCERER: u8 = 0b0001_0000;
    const BIT_ROBBER: u8 = 0b0010_0000;

    #[test]
    fn can_use_class_and_alignment() {
        let d = Disablement::from_index(0); // mask 0b0000_0001
        assert!(d.can_use_class(BIT_KNIGHT));
        assert!(!d.can_use_class(BIT_ARCHER));
        let d = Disablement::ALL;
        assert!(d.can_use_class(BIT_KNIGHT));
        // presence tested for compile/coverage only
        assert!(d.good_only());
        assert!(d.evil_only());
    }

    #[test]
    fn test_disablement_all_classes() {
        let dis = Disablement::ALL;
        assert!(dis.can_use_class(BIT_KNIGHT));
        assert!(dis.can_use_class(BIT_SORCERER));
        assert!(dis.can_use_class(BIT_ROBBER));
    }

    #[test]
    fn test_disablement_knight_only() {
        let dis = Disablement(BIT_KNIGHT);
        assert!(dis.can_use_class(BIT_KNIGHT));
        assert!(!dis.can_use_class(BIT_SORCERER));
    }

    #[test]
    fn test_disablement_good_alignment() {
        let dis = Disablement(BIT_PALADIN | Disablement::GOOD);
        assert!(dis.good_only());
        assert!(!dis.evil_only());
    }

    // ===== New Data-Driven Tests =====

    #[test]
    fn test_can_use_class_id_single_class() {
        let class_db = create_test_class_db();

        // Knight-only item (bit 0)
        let knight_item = Disablement(0b0000_0001);
        assert!(knight_item.can_use_class_id("knight", &class_db));
        assert!(!knight_item.can_use_class_id("paladin", &class_db));
        assert!(!knight_item.can_use_class_id("sorcerer", &class_db));
    }

    #[test]
    fn test_can_use_class_id_multiple_classes() {
        let class_db = create_test_class_db();

        // Knight and Paladin only (bits 0 and 1)
        let martial_item = Disablement(0b0000_0011);
        assert!(martial_item.can_use_class_id("knight", &class_db));
        assert!(martial_item.can_use_class_id("paladin", &class_db));
        assert!(!martial_item.can_use_class_id("sorcerer", &class_db));
        assert!(!martial_item.can_use_class_id("cleric", &class_db));
    }

    #[test]
    fn test_can_use_class_id_all_classes() {
        let class_db = create_test_class_db();

        let universal = Disablement::ALL;
        assert!(universal.can_use_class_id("knight", &class_db));
        assert!(universal.can_use_class_id("paladin", &class_db));
        assert!(universal.can_use_class_id("archer", &class_db));
        assert!(universal.can_use_class_id("cleric", &class_db));
        assert!(universal.can_use_class_id("sorcerer", &class_db));
        assert!(universal.can_use_class_id("robber", &class_db));
    }

    #[test]
    fn test_can_use_class_id_none() {
        let class_db = create_test_class_db();

        let quest_item = Disablement::NONE;
        assert!(!quest_item.can_use_class_id("knight", &class_db));
        assert!(!quest_item.can_use_class_id("sorcerer", &class_db));
    }

    #[test]
    fn test_can_use_class_id_unknown_class() {
        let class_db = create_test_class_db();

        let item = Disablement::ALL;
        // Unknown class should return false
        assert!(!item.can_use_class_id("unknown_class", &class_db));
        assert!(!item.can_use_class_id("", &class_db));
    }

    #[test]
    fn test_can_use_alignment_any() {
        // No alignment restriction
        let normal_item = Disablement(0b0000_0001);
        assert!(normal_item.can_use_alignment(Alignment::Good));
        assert!(normal_item.can_use_alignment(Alignment::Evil));
        assert!(normal_item.can_use_alignment(Alignment::Neutral));
    }

    #[test]
    fn test_can_use_alignment_good_only() {
        // Good alignment required
        let holy_item = Disablement(0b0100_0001);
        assert!(holy_item.can_use_alignment(Alignment::Good));
        assert!(!holy_item.can_use_alignment(Alignment::Evil));
        assert!(!holy_item.can_use_alignment(Alignment::Neutral));
    }

    #[test]
    fn test_can_use_alignment_evil_only() {
        // Evil alignment required
        let dark_item = Disablement(0b1000_0001);
        assert!(!dark_item.can_use_alignment(Alignment::Good));
        assert!(dark_item.can_use_alignment(Alignment::Evil));
        assert!(!dark_item.can_use_alignment(Alignment::Neutral));
    }

    #[test]
    fn test_can_use_combined() {
        let class_db = create_test_class_db();

        // Holy sword: Knight only, Good alignment
        let holy_sword = Disablement(0b0100_0001);
        assert!(holy_sword.can_use("knight", Alignment::Good, &class_db));
        assert!(!holy_sword.can_use("knight", Alignment::Evil, &class_db));
        assert!(!holy_sword.can_use("knight", Alignment::Neutral, &class_db));
        assert!(!holy_sword.can_use("sorcerer", Alignment::Good, &class_db));

        // Dark dagger: Robber only, Evil alignment
        let dark_dagger = Disablement(0b1010_0000);
        assert!(dark_dagger.can_use("robber", Alignment::Evil, &class_db));
        assert!(!dark_dagger.can_use("robber", Alignment::Good, &class_db));
        assert!(!dark_dagger.can_use("knight", Alignment::Evil, &class_db));
    }

    #[test]
    fn test_from_class_ids() {
        let class_db = create_test_class_db();

        // Create disablement from class list
        let classes = ["knight", "paladin", "archer"];
        let dis = Disablement::from_class_ids(classes.iter().copied(), &class_db);

        assert!(dis.can_use_class_id("knight", &class_db));
        assert!(dis.can_use_class_id("paladin", &class_db));
        assert!(dis.can_use_class_id("archer", &class_db));
        assert!(!dis.can_use_class_id("cleric", &class_db));
        assert!(!dis.can_use_class_id("sorcerer", &class_db));
        assert!(!dis.can_use_class_id("robber", &class_db));
    }

    #[test]
    fn test_from_class_ids_empty() {
        let class_db = create_test_class_db();

        let classes: [&str; 0] = [];
        let dis = Disablement::from_class_ids(classes.iter().copied(), &class_db);

        assert_eq!(dis.0, 0);
        assert!(!dis.can_use_class_id("knight", &class_db));
    }

    #[test]
    fn test_from_class_ids_with_unknown() {
        let class_db = create_test_class_db();

        // Include an unknown class - should be ignored
        let classes = ["knight", "unknown_class", "paladin"];
        let dis = Disablement::from_class_ids(classes.iter().copied(), &class_db);

        assert!(dis.can_use_class_id("knight", &class_db));
        assert!(dis.can_use_class_id("paladin", &class_db));
        // Should only have bits 0 and 1 set
        assert_eq!(dis.0 & 0b0011_1111, 0b0000_0011);
    }

    #[test]
    fn test_allowed_class_ids() {
        let class_db = create_test_class_db();

        // Knight and Sorcerer only
        let item = Disablement(0b0001_0001);
        let allowed = item.allowed_class_ids(&class_db);

        assert_eq!(allowed.len(), 2);
        assert!(allowed.contains(&"knight".to_string()));
        assert!(allowed.contains(&"sorcerer".to_string()));
    }

    #[test]
    fn test_allowed_class_ids_all() {
        let class_db = create_test_class_db();

        let universal = Disablement::ALL;
        let allowed = universal.allowed_class_ids(&class_db);

        // Should include all 6 classes
        assert_eq!(allowed.len(), 6);
    }

    #[test]
    fn test_allowed_class_ids_none() {
        let class_db = create_test_class_db();

        let quest_item = Disablement::NONE;
        let allowed = quest_item.allowed_class_ids(&class_db);

        assert!(allowed.is_empty());
    }

    #[test]
    fn test_dynamic_class_lookup() {
        // Verify that the data-driven approach works correctly
        let class_db = create_test_class_db();

        // Check each class using raw bit values
        let knight_item = Disablement(BIT_KNIGHT);
        assert!(knight_item.can_use_class_id("knight", &class_db));

        let paladin_item = Disablement(BIT_PALADIN);
        assert!(paladin_item.can_use_class_id("paladin", &class_db));

        let archer_item = Disablement(BIT_ARCHER);
        assert!(archer_item.can_use_class_id("archer", &class_db));

        let cleric_item = Disablement(BIT_CLERIC);
        assert!(cleric_item.can_use_class_id("cleric", &class_db));

        let sorcerer_item = Disablement(BIT_SORCERER);
        assert!(sorcerer_item.can_use_class_id("sorcerer", &class_db));

        let robber_item = Disablement(BIT_ROBBER);
        assert!(robber_item.can_use_class_id("robber", &class_db));
    }

    #[test]
    fn test_bit_index_produces_correct_mask() {
        // Verify that ClassDefinition.disablement_bit_index produces the correct mask
        let class_db = create_test_class_db();

        // Knight: bit 0 -> mask 0b0000_0001
        let knight = class_db.get_class("knight").unwrap();
        assert_eq!(knight.disablement_mask(), BIT_KNIGHT);

        // Paladin: bit 1 -> mask 0b0000_0010
        let paladin = class_db.get_class("paladin").unwrap();
        assert_eq!(paladin.disablement_mask(), BIT_PALADIN);

        // Archer: bit 2 -> mask 0b0000_0100
        let archer = class_db.get_class("archer").unwrap();
        assert_eq!(archer.disablement_mask(), BIT_ARCHER);

        // Cleric: bit 3 -> mask 0b0000_1000
        let cleric = class_db.get_class("cleric").unwrap();
        assert_eq!(cleric.disablement_mask(), BIT_CLERIC);

        // Sorcerer: bit 4 -> mask 0b0001_0000
        let sorcerer = class_db.get_class("sorcerer").unwrap();
        assert_eq!(sorcerer.disablement_mask(), BIT_SORCERER);

        // Robber: bit 5 -> mask 0b0010_0000
        let robber = class_db.get_class("robber").unwrap();
        assert_eq!(robber.disablement_mask(), BIT_ROBBER);
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
            tags: vec![],
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
            tags: vec![],
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
            tags: vec![],
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
            tags: vec![],
        };

        assert_eq!(cursed.to_string(), "Cursed Mace (Cursed)");
    }
}
