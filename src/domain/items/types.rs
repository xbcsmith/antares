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
use crate::domain::proficiency::{ProficiencyDatabase, ProficiencyId};
use crate::domain::types::{DiceRoll, ItemId, SpellId};
use crate::domain::visual::item_mesh::ItemMeshDescriptorOverride;
use serde::{Deserialize, Serialize};
use std::fmt;

// ===== Item Type Enum =====

/// Main item type discriminator
///
/// # Examples
///
/// ```
/// use antares::domain::items::{ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
///
/// let weapon_type = ItemType::Weapon(WeaponData {
///     damage: DiceRoll::new(1, 8, 0),
///     bonus: 1,
///     hands_required: 1,
///     classification: WeaponClassification::MartialMelee,
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
///
/// let helmet = ArmorClassification::Helmet;
/// assert_ne!(helmet, ArmorClassification::Light);
///
/// let boots = ArmorClassification::Boots;
/// assert_ne!(boots, ArmorClassification::Light);
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
    /// Helmet / headgear — maps to equipment.helmet
    Helmet,
    /// Boots / footwear — maps to equipment.boots
    Boots,
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
/// use antares::domain::items::{WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
///
/// // Club: 1d3 damage, no bonus, 1-handed, simple weapon
/// let club = WeaponData {
///     damage: DiceRoll::new(1, 3, 0),
///     bonus: 0,
///     hands_required: 1,
///     classification: WeaponClassification::Simple,
/// };
///
/// // Two-handed sword: 2d6 damage, +2 bonus, 2-handed, martial melee
/// let greatsword = WeaponData {
///     damage: DiceRoll::new(2, 6, 0),
///     bonus: 2,
///     hands_required: 2,
///     classification: WeaponClassification::MartialMelee,
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
    /// Weapon classification determines required proficiency
    #[serde(default)]
    pub classification: WeaponClassification,
}

// ===== Armor Data =====

/// Armor-specific data
///
/// # Examples
///
/// ```
/// use antares::domain::items::{ArmorData, ArmorClassification};
///
/// // Leather armor: +2 AC, light armor
/// let leather = ArmorData {
///     ac_bonus: 2,
///     weight: 15,
///     classification: ArmorClassification::Light,
/// };
///
/// // Plate mail: +8 AC, heavy armor
/// let plate = ArmorData {
///     ac_bonus: 8,
///     weight: 50,
///     classification: ArmorClassification::Heavy,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Armor class bonus (higher is better)
    pub ac_bonus: u8,
    /// Weight in pounds (affects movement)
    pub weight: u8,
    /// Armor classification determines required proficiency
    #[serde(default)]
    pub classification: ArmorClassification,
}

// ===== Accessory Data =====

/// Accessory-specific data (rings, amulets, belts)
///
/// # Examples
///
/// ```
/// use antares::domain::items::{AccessoryData, AccessorySlot, MagicItemClassification};
///
/// // Mundane ring (no magic classification)
/// let ring = AccessoryData {
///     slot: AccessorySlot::Ring,
///     classification: None,
/// };
///
/// // Arcane wand (requires arcane_item proficiency)
/// let wand = AccessoryData {
///     slot: AccessorySlot::Ring,
///     classification: Some(MagicItemClassification::Arcane),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessoryData {
    /// Which accessory slot this occupies
    pub slot: AccessorySlot,
    /// Magic item classification (None for mundane accessories)
    #[serde(default)]
    pub classification: Option<MagicItemClassification>,
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
///     duration_minutes: None,
/// };
///
/// use antares::domain::items::types::ResistanceType;
///
/// let fire_resist_potion = ConsumableData {
///     effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
///     is_combat_usable: false,
///     duration_minutes: Some(60),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effect when consumed
    pub effect: ConsumableEffect,
    /// Whether usable during combat
    pub is_combat_usable: bool,
    /// Optional duration in in-game minutes.
    ///
    /// - `None` — effect is permanent (legacy behavior; backward compatible).
    /// - `Some(0)` — normalized to `None` at application time; treat as permanent.
    /// - `Some(n)` — effect expires after `n` in-game minutes.
    ///
    /// Only meaningful for `BoostAttribute` and `BoostResistance` effects.
    /// `HealHp`, `RestoreSp`, and `CureCondition` are instant and ignore this field.
    #[serde(default)]
    pub duration_minutes: Option<u16>,
}

/// Normalizes a raw `duration_minutes` value.
///
/// `Some(0)` is treated as permanent (`None`) so that editor inputs of `0`
/// and omitted RON fields both produce identical runtime semantics.
///
/// # Examples
///
/// ```
/// use antares::domain::items::types::normalize_duration;
/// assert_eq!(normalize_duration(Some(0)), None);
/// assert_eq!(normalize_duration(Some(60)), Some(60));
/// assert_eq!(normalize_duration(None), None);
/// ```
pub fn normalize_duration(raw: Option<u16>) -> Option<u16> {
    match raw {
        Some(0) | None => None,
        other => other,
    }
}

/// Effects from consuming items
///
/// # Examples
///
/// ```
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect, AttributeType, ResistanceType};
///
/// let heal_potion = ConsumableData {
///     effect: ConsumableEffect::HealHp(20),
///     is_combat_usable: true,
///     duration_minutes: None,
/// };
///
/// let food_ration = ConsumableData {
///     effect: ConsumableEffect::IsFood(1),
///     is_combat_usable: false,
///     duration_minutes: None,
/// };
///
/// assert_eq!(food_ration.effect, ConsumableEffect::IsFood(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsumableEffect {
    HealHp(u16),
    RestoreSp(u16),
    CureCondition(u8), // Condition flags to clear
    BoostAttribute(AttributeType, i8),
    /// Temporarily boost a resistance by `i8` points (positive = resist more,
    /// negative = resist less).  Maps onto `character::Resistances` fields.
    BoostResistance(ResistanceType, i8),
    /// A food item providing the given number of rations (almost always 1).
    ///
    /// When consumed during rest, each unit satisfies one character's food
    /// requirement for that rest period.  The rest system searches character
    /// inventories for items with this effect instead of decrementing the
    /// legacy numeric `Character.food` counter.
    ///
    /// # Value
    ///
    /// The inner `u8` is the **ration count** supplied by one unit of this item.
    /// Standard "Food Ration" items carry a value of `1`.  A "Trail Ration Pack"
    /// could carry `3` to satisfy three characters with a single item.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::types::ConsumableEffect;
    ///
    /// let ration = ConsumableEffect::IsFood(1);
    /// let trail_pack = ConsumableEffect::IsFood(3);
    ///
    /// // Pattern match to extract ration count
    /// if let ConsumableEffect::IsFood(count) = ration {
    ///     assert_eq!(count, 1);
    /// }
    /// if let ConsumableEffect::IsFood(count) = trail_pack {
    ///     assert_eq!(count, 3);
    /// }
    /// ```
    IsFood(u8),
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

impl AttributeType {
    /// Returns a human-readable display name for UI labels.
    pub fn display_name(self) -> &'static str {
        match self {
            AttributeType::Might => "Might",
            AttributeType::Intellect => "Intellect",
            AttributeType::Personality => "Personality",
            AttributeType::Endurance => "Endurance",
            AttributeType::Speed => "Speed",
            AttributeType::Accuracy => "Accuracy",
            AttributeType::Luck => "Luck",
        }
    }
}

/// Resistance types that can be boosted by a consumable item.
///
/// Variants match the fields of `character::Resistances` and follow the
/// same naming convention used by `MonsterResistances`.
///
/// # Examples
///
/// ```
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect, ResistanceType};
///
/// // Potion of Fire Resistance — grants +25 fire resistance
/// let potion = ConsumableData {
///     effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
///     is_combat_usable: true,
///     duration_minutes: None,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResistanceType {
    /// Physical / weapon damage resistance
    Physical,
    /// Fire damage resistance
    Fire,
    /// Cold damage resistance
    Cold,
    /// Electricity / lightning damage resistance
    Electricity,
    /// Energy / magic damage resistance (maps to `Resistances::magic`)
    Energy,
    /// Paralysis effect resistance (maps to `Resistances::psychic` as closest analogue)
    Paralysis,
    /// Fear effect resistance
    Fear,
    /// Sleep effect resistance (maps to `Resistances::psychic` as closest analogue)
    Sleep,
}

impl ResistanceType {
    /// Returns a human-readable display name for UI labels.
    pub fn display_name(self) -> &'static str {
        match self {
            ResistanceType::Physical => "Physical",
            ResistanceType::Fire => "Fire",
            ResistanceType::Cold => "Cold",
            ResistanceType::Electricity => "Electricity",
            ResistanceType::Energy => "Energy",
            ResistanceType::Paralysis => "Paralysis",
            ResistanceType::Fear => "Fear",
            ResistanceType::Sleep => "Sleep",
        }
    }

    /// All variants in display order (used by UI ComboBox).
    pub fn all() -> [ResistanceType; 8] {
        [
            ResistanceType::Physical,
            ResistanceType::Fire,
            ResistanceType::Cold,
            ResistanceType::Electricity,
            ResistanceType::Energy,
            ResistanceType::Paralysis,
            ResistanceType::Fear,
            ResistanceType::Sleep,
        ]
    }
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

// ===== Complete Item Definition =====

/// Complete item definition
///
/// This is the main item struct that gets serialized from data files.
///
/// # Examples
///
/// ```
/// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification, AlignmentRestriction};
/// use antares::domain::types::DiceRoll;
///
/// let club = Item {
///     id: 1,
///     name: "Club".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 3, 0),
///         bonus: 0,
///         hands_required: 1,
///         classification: WeaponClassification::Simple,
///     }),
///     base_cost: 1,
///     sell_cost: 0,
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
///
/// // Holy Sword - good alignment only
/// let holy_sword = Item {
///     id: 2,
///     name: "Holy Sword".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 8, 0),
///         bonus: 2,
///         hands_required: 1,
///         classification: WeaponClassification::MartialMelee,
///     }),
///     base_cost: 500,
///     sell_cost: 250,
///     alignment_restriction: Some(AlignmentRestriction::GoodOnly),
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
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

    /// Alignment restriction for this item (separate from proficiency)
    ///
    /// Some items can only be used by characters of specific alignments.
    /// - `None` - Any alignment can use
    /// - `Some(GoodOnly)` - Only good-aligned characters
    /// - `Some(EvilOnly)` - Only evil-aligned characters
    #[serde(default)]
    pub alignment_restriction: Option<AlignmentRestriction>,
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

    /// Optional campaign-author override for the procedural 3-D world mesh.
    ///
    /// When `Some`, the fields present in the override are applied on top of the
    /// values automatically derived by [`ItemMeshDescriptor::from_item`].  Fields
    /// set to `None` inside the override struct keep their auto-derived values.
    ///
    /// Use `#[serde(default)]` ensures all existing RON item files remain valid
    /// without modification — the field simply deserialises as `None` when absent.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::Item;
    /// use antares::domain::visual::item_mesh::ItemMeshDescriptorOverride;
    ///
    /// // In a RON item file you can optionally add:
    /// // mesh_descriptor_override: Some((
    /// //     primary_color: Some((0.0, 0.8, 0.0, 1.0)),
    /// // ))
    /// ```
    #[serde(default)]
    pub mesh_descriptor_override: Option<ItemMeshDescriptorOverride>,

    /// ID of the pre-authored mesh RON entry (e.g. 9003 = ItemMeshShortSword).
    ///
    /// When `Some`, `spawn_dropped_item_system` looks up the full `CreatureDefinition`
    /// from `GameDataResource` and uses its `scale` and mesh data directly.
    /// When `None`, the system falls back to the procedural `ItemMeshDescriptor::from_item` path.
    #[serde(default)]
    pub mesh_id: Option<u32>,
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

    /// Get the proficiency ID required to use this item
    ///
    /// Derives the required proficiency from the item's classification.
    /// Items without a classification requirement (consumables, ammo, quest items,
    /// universal accessories) return `None`.
    ///
    /// # Returns
    ///
    /// - `Some(ProficiencyId)` - The proficiency required to use this item
    /// - `None` - No proficiency required (anyone can use)
    ///
    /// # Examples
    ///
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let sword = Item {
    ///     id: 1,
    ///     name: "Long Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 15,
    ///     sell_cost: 7,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    /// };
    ///
    /// assert!(sword.is_weapon());
    /// assert!(!sword.is_armor());
    ///
    /// // Holy weapon with alignment restriction
    /// let holy_avenger = Item {
    ///     id: 2,
    ///     name: "Holy Avenger".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 3,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 1000,
    ///     sell_cost: 500,
    ///     alignment_restriction: Some(AlignmentRestriction::GoodOnly),
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    /// };
    ///
    /// // Access item properties
    /// assert_eq!(sword.name, "Long Sword");
    /// assert_eq!(sword.base_cost, 15);
    pub fn required_proficiency(&self) -> Option<ProficiencyId> {
        match &self.item_type {
            ItemType::Weapon(data) => Some(ProficiencyDatabase::proficiency_for_weapon(
                data.classification,
            )),
            ItemType::Armor(data) => Some(ProficiencyDatabase::proficiency_for_armor(
                data.classification,
            )),
            ItemType::Accessory(data) => {
                // Accessories only require proficiency if they have a magic classification
                data.classification
                    .and_then(ProficiencyDatabase::proficiency_for_magic_item)
            }
            // Consumables, ammo, and quest items have no proficiency requirements
            ItemType::Consumable(_) | ItemType::Ammo(_) | ItemType::Quest(_) => None,
        }
    }

    /// Check if a character can use this item based on alignment
    ///
    /// This checks only the alignment restriction, not proficiency.
    /// Use in combination with proficiency checks for complete validation.
    ///
    /// # Arguments
    ///
    /// * `alignment` - The character's alignment
    ///
    /// # Returns
    ///
    /// `true` if the character's alignment allows them to use this item
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification, AlignmentRestriction};
    /// use antares::domain::character::Alignment;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let holy_sword = Item {
    ///     id: 1,
    ///     name: "Holy Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 2,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 500,
    ///     sell_cost: 250,
    ///     alignment_restriction: Some(AlignmentRestriction::GoodOnly),
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    ///     mesh_descriptor_override: None,
    /// };
    ///
    /// assert!(holy_sword.can_use_alignment(Alignment::Good));
    /// assert!(!holy_sword.can_use_alignment(Alignment::Evil));
    /// assert!(!holy_sword.can_use_alignment(Alignment::Neutral));
    /// ```
    pub fn can_use_alignment(&self, alignment: Alignment) -> bool {
        match self.alignment_restriction {
            None => true, // No restriction, any alignment can use
            Some(AlignmentRestriction::GoodOnly) => alignment == Alignment::Good,
            Some(AlignmentRestriction::EvilOnly) => alignment == Alignment::Evil,
        }
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
    fn test_item_type_checks() {
        let weapon = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 10,
            sell_cost: 5,
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
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,
            alignment_restriction: None,
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
            mesh_descriptor_override: None,
            mesh_id: None,
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
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 10,
            sell_cost: 5,
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
        };

        assert_eq!(item.to_string(), "Basic Sword");
    }

    #[test]
    #[allow(deprecated)]
    fn test_cursed_item_display() {
        let cursed = Item {
            id: 2,
            name: "Cursed Mace".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Blunt,
            }),
            base_cost: 100,
            sell_cost: 50,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: true,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(cursed.to_string(), "Cursed Mace (Cursed)");
    }

    // ===== Phase 3: Item::required_proficiency Tests =====

    #[test]
    fn test_weapon_required_proficiency_simple() {
        let club = Item {
            id: 1,
            name: "Club".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 3, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
            }),
            base_cost: 1,
            sell_cost: 0,

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
        };

        assert_eq!(
            club.required_proficiency(),
            Some("simple_weapon".to_string())
        );
    }

    #[test]
    fn test_weapon_required_proficiency_martial_melee() {
        let longsword = Item {
            id: 1,
            name: "Long Sword".to_string(),
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
        };

        assert_eq!(
            longsword.required_proficiency(),
            Some("martial_melee".to_string())
        );
    }

    #[test]
    fn test_weapon_required_proficiency_martial_ranged() {
        let longbow = Item {
            id: 1,
            name: "Long Bow".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 2,
                classification: WeaponClassification::MartialRanged,
            }),
            base_cost: 25,
            sell_cost: 12,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["large_weapon".to_string(), "two_handed".to_string()],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(
            longbow.required_proficiency(),
            Some("martial_ranged".to_string())
        );
    }

    #[test]
    fn test_weapon_required_proficiency_blunt() {
        let mace = Item {
            id: 1,
            name: "Mace".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Blunt,
            }),
            base_cost: 8,
            sell_cost: 4,

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
        };

        assert_eq!(
            mace.required_proficiency(),
            Some("blunt_weapon".to_string())
        );
    }

    #[test]
    fn test_armor_required_proficiency_light() {
        let leather = Item {
            id: 20,
            name: "Leather Armor".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 2,
                weight: 15,
                classification: ArmorClassification::Light,
            }),
            base_cost: 5,
            sell_cost: 2,

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
        };

        assert_eq!(
            leather.required_proficiency(),
            Some("light_armor".to_string())
        );
    }

    #[test]
    fn test_armor_required_proficiency_heavy() {
        let platemail = Item {
            id: 22,
            name: "Plate Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 8,
                weight: 50,
                classification: ArmorClassification::Heavy,
            }),
            base_cost: 600,
            sell_cost: 300,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["heavy_armor".to_string()],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(
            platemail.required_proficiency(),
            Some("heavy_armor".to_string())
        );
    }

    #[test]
    fn test_armor_required_proficiency_shield() {
        let shield = Item {
            id: 23,
            name: "Wooden Shield".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 8,
                classification: ArmorClassification::Shield,
            }),
            base_cost: 10,
            sell_cost: 5,

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
        };

        assert_eq!(shield.required_proficiency(), Some("shield".to_string()));
    }

    // ===== Phase 5: Item::required_proficiency — Helmet and Boots =====

    #[test]
    fn test_armor_required_proficiency_helmet() {
        // Helmet classification must map to the same proficiency as Light armor ("light_armor").
        // This validates that ArmorClassification::Helmet is handled by required_proficiency()
        // and that callers do not need a special-case branch for headgear.
        let iron_helmet = Item {
            id: 25,
            name: "Iron Helmet".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 5,
                classification: ArmorClassification::Helmet,
            }),
            base_cost: 40,
            sell_cost: 20,

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
        };

        assert_eq!(
            iron_helmet.required_proficiency(),
            Some("light_armor".to_string()),
            "Helmet classification must require light_armor proficiency"
        );
    }

    #[test]
    fn test_armor_required_proficiency_boots() {
        // Boots classification must map to the same proficiency as Light armor ("light_armor").
        // This validates that ArmorClassification::Boots is handled by required_proficiency()
        // and that callers do not need a special-case branch for footwear.
        let leather_boots = Item {
            id: 26,
            name: "Leather Boots".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 2,
                classification: ArmorClassification::Boots,
            }),
            base_cost: 20,
            sell_cost: 10,

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
        };

        assert_eq!(
            leather_boots.required_proficiency(),
            Some("light_armor".to_string()),
            "Boots classification must require light_armor proficiency"
        );
    }

    #[test]
    fn test_accessory_required_proficiency_arcane() {
        let wand = Item {
            id: 43,
            name: "Arcane Wand".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: Some(MagicItemClassification::Arcane),
            }),
            base_cost: 1000,
            sell_cost: 500,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 20,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(wand.required_proficiency(), Some("arcane_item".to_string()));
    }

    #[test]
    fn test_accessory_required_proficiency_divine() {
        #[allow(deprecated)]
        let symbol = Item {
            id: 44,
            name: "Holy Symbol".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Amulet,
                classification: Some(MagicItemClassification::Divine),
            }),
            base_cost: 800,
            sell_cost: 400,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 15,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(
            symbol.required_proficiency(),
            Some("divine_item".to_string())
        );
    }

    #[test]
    fn test_accessory_required_proficiency_universal() {
        let ring = Item {
            id: 40,
            name: "Ring of Protection".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: Some(MagicItemClassification::Universal),
            }),
            base_cost: 100,
            sell_cost: 50,

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
        };

        // Universal magic items have no proficiency requirement
        assert_eq!(ring.required_proficiency(), None);
    }

    #[test]
    fn test_accessory_required_proficiency_mundane() {
        #[allow(deprecated)]
        let ring = Item {
            id: 40,
            name: "Plain Ring".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: None,
            }),
            base_cost: 10,
            sell_cost: 5,

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
        };

        // Mundane accessories have no proficiency requirement
        assert_eq!(ring.required_proficiency(), None);
    }

    #[test]
    fn test_consumable_no_proficiency() {
        let potion = Item {
            id: 50,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
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
        };

        assert_eq!(potion.required_proficiency(), None);
    }

    #[test]
    fn test_ammo_no_proficiency() {
        #[allow(deprecated)]
        let arrows = Item {
            id: 60,
            name: "Arrows".to_string(),
            item_type: ItemType::Ammo(AmmoData {
                ammo_type: AmmoType::Arrow,
                quantity: 20,
            }),
            base_cost: 5,
            sell_cost: 2,

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
        };

        assert_eq!(arrows.required_proficiency(), None);
    }

    #[test]
    fn test_quest_item_no_proficiency() {
        let quest_item = Item {
            id: 100,
            name: "Ruby Whistle".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "brothers_quest".to_string(),
                is_key_item: true,
            }),
            base_cost: 500,
            sell_cost: 250,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 200,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert_eq!(quest_item.required_proficiency(), None);
    }

    // ===== Phase 3: Item::can_use_alignment Tests =====

    #[test]
    fn test_alignment_restriction_none() {
        let item = Item {
            id: 1,
            name: "Normal Sword".to_string(),
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
        };

        assert!(item.can_use_alignment(Alignment::Good));
        assert!(item.can_use_alignment(Alignment::Evil));
        assert!(item.can_use_alignment(Alignment::Neutral));
    }

    #[test]
    fn test_alignment_restriction_good_only() {
        let holy_sword = Item {
            id: 1,
            name: "Holy Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 2,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,

            alignment_restriction: Some(AlignmentRestriction::GoodOnly),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert!(holy_sword.can_use_alignment(Alignment::Good));
        assert!(!holy_sword.can_use_alignment(Alignment::Evil));
        assert!(!holy_sword.can_use_alignment(Alignment::Neutral));
    }

    #[test]
    fn test_alignment_restriction_evil_only() {
        let dark_blade = Item {
            id: 1,
            name: "Dark Blade".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 2,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,

            alignment_restriction: Some(AlignmentRestriction::EvilOnly),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };

        assert!(!dark_blade.can_use_alignment(Alignment::Good));
        assert!(dark_blade.can_use_alignment(Alignment::Evil));
        assert!(!dark_blade.can_use_alignment(Alignment::Neutral));
    }

    // ===== IsFood / ConsumableEffect Tests =====

    #[test]
    fn test_is_food_effect_equality() {
        let ration = ConsumableEffect::IsFood(1);
        let trail_pack = ConsumableEffect::IsFood(3);

        assert_eq!(ration, ConsumableEffect::IsFood(1));
        assert_ne!(ration, trail_pack);
        assert_ne!(ration, ConsumableEffect::HealHp(1));
    }

    #[test]
    fn test_is_food_ration_count_extracted() {
        let effect = ConsumableEffect::IsFood(1);
        if let ConsumableEffect::IsFood(count) = effect {
            assert_eq!(count, 1u8);
        } else {
            panic!("Expected IsFood variant");
        }
    }

    #[test]
    fn test_is_food_trail_pack_ration_count() {
        let effect = ConsumableEffect::IsFood(3);
        if let ConsumableEffect::IsFood(count) = effect {
            assert_eq!(count, 3u8);
        } else {
            panic!("Expected IsFood variant");
        }
    }

    #[test]
    fn test_is_food_serializes_correctly() {
        let effect = ConsumableEffect::IsFood(1);
        let serialized = ron::to_string(&effect).expect("serialization must succeed");
        // RON serialises a tuple variant as VariantName(value)
        assert!(
            serialized.contains("IsFood"),
            "serialized form must contain 'IsFood', got: {serialized}"
        );
        assert!(
            serialized.contains('1'),
            "serialized form must contain the ration count, got: {serialized}"
        );
    }

    #[test]
    fn test_is_food_deserializes_correctly() {
        let ron_str = "IsFood(1)";
        let effect: ConsumableEffect =
            ron::from_str(ron_str).expect("deserialization must succeed");
        assert_eq!(effect, ConsumableEffect::IsFood(1));
    }

    #[test]
    fn test_is_food_roundtrip_serde() {
        let original = ConsumableEffect::IsFood(2);
        let serialized = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: ConsumableEffect =
            ron::from_str(&serialized).expect("deserialization must succeed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_consumable_data_with_is_food_roundtrip() {
        let data = ConsumableData {
            effect: ConsumableEffect::IsFood(1),
            is_combat_usable: false,
            duration_minutes: None,
        };
        let serialized = ron::to_string(&data).expect("serialization must succeed");
        let deserialized: ConsumableData =
            ron::from_str(&serialized).expect("deserialization must succeed");
        assert_eq!(deserialized.effect, ConsumableEffect::IsFood(1));
        assert!(!deserialized.is_combat_usable);
        assert_eq!(deserialized.duration_minutes, None);
    }

    #[test]
    fn test_food_ration_item_loads_from_ron_string() {
        let ron_data = r#"
[
    (
        id: 53,
        name: "Food Ration",
        item_type: Consumable((
            effect: IsFood(1),
            is_combat_usable: false,
        )),
        base_cost: 2,
        sell_cost: 1,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 1,
        is_cursed: false,
        icon_path: None,
        tags: [],
    ),
]
"#;
        use crate::domain::items::database::ItemDatabase;
        let db = ItemDatabase::load_from_string(ron_data)
            .expect("ItemDatabase must load Food Ration without error");
        let item = db
            .get_item(53)
            .expect("Food Ration must be findable by id 53");
        assert_eq!(item.name, "Food Ration");
        assert!(item.is_consumable());
        if let ItemType::Consumable(ref data) = item.item_type {
            assert_eq!(data.effect, ConsumableEffect::IsFood(1));
            assert!(!data.is_combat_usable);
        } else {
            panic!("Food Ration must be a Consumable item type");
        }
    }

    // ===== Phase 1: duration_minutes / normalize_duration tests =====

    #[test]
    fn test_consumable_data_duration_defaults_none_in_ron() {
        // RON without duration_minutes field must deserialize with duration_minutes: None
        // due to #[serde(default)].
        let ron_str = r#"(effect: HealHp(20), is_combat_usable: true)"#;
        let data: ConsumableData = ron::from_str(ron_str).expect("deserialization must succeed");
        assert_eq!(
            data.duration_minutes, None,
            "omitted duration_minutes must default to None"
        );
        assert_eq!(data.effect, ConsumableEffect::HealHp(20));
        assert!(data.is_combat_usable);
    }

    #[test]
    fn test_consumable_data_duration_some_round_trips() {
        let original = ConsumableData {
            effect: ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
            is_combat_usable: false,
            duration_minutes: Some(60),
        };
        let serialized = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: ConsumableData =
            ron::from_str(&serialized).expect("deserialization must succeed");
        assert_eq!(
            deserialized.duration_minutes,
            Some(60),
            "Some(60) must survive a RON round-trip"
        );
        assert_eq!(deserialized.effect, original.effect);
    }

    #[test]
    fn test_normalize_duration_zero_becomes_none() {
        assert_eq!(
            normalize_duration(Some(0)),
            None,
            "Some(0) must be normalized to None"
        );
    }

    #[test]
    fn test_normalize_duration_none_stays_none() {
        assert_eq!(
            normalize_duration(None),
            None,
            "None must stay None after normalization"
        );
    }

    #[test]
    fn test_normalize_duration_positive_unchanged() {
        assert_eq!(
            normalize_duration(Some(30)),
            Some(30),
            "positive duration must pass through unchanged"
        );
        assert_eq!(normalize_duration(Some(1)), Some(1));
        assert_eq!(normalize_duration(Some(u16::MAX)), Some(u16::MAX));
    }

    #[test]
    fn test_consumable_data_struct_literal_compiles_with_new_field() {
        let data = ConsumableData {
            effect: ConsumableEffect::HealHp(10),
            is_combat_usable: true,
            duration_minutes: None,
        };
        assert_eq!(data.duration_minutes, None);
        assert_eq!(data.effect, ConsumableEffect::HealHp(10));
        assert!(data.is_combat_usable);
    }

    #[test]
    fn test_food_ration_not_combat_usable() {
        let data = ConsumableData {
            effect: ConsumableEffect::IsFood(1),
            is_combat_usable: false,
            duration_minutes: None,
        };
        assert!(
            !data.is_combat_usable,
            "Food Rations must not be usable during combat"
        );
    }

    #[test]
    fn test_is_food_no_required_proficiency() {
        let food_item = Item {
            id: 53,
            name: "Food Ration".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::IsFood(1),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 2,
            sell_cost: 1,
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
        };
        assert_eq!(
            food_item.required_proficiency(),
            None,
            "Food items must not require any proficiency"
        );
    }
}
