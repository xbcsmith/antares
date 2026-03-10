// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character system - Stats, inventory, equipment, and party management
//!
//! This module contains all character-related data structures including
//! character attributes, inventory management, equipment, and party composition.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.3 for complete specifications.
//! See `docs/reference/stat_ranges.md` for detailed stat range documentation.

use crate::domain::classes::{ClassDatabase, ClassId, SpellSchool as ClassSpellSchool};
use crate::domain::types::{CharacterId, InnkeeperId, ItemId, MapId, RaceId, SpellId};
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur in character operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CharacterError {
    #[error("Inventory is full (max {0} items)")]
    InventoryFull(usize),

    #[error("Party is full (max {0} members)")]
    PartyFull(usize),

    #[error("Roster is full (max {0} characters)")]
    RosterFull(usize),

    #[error("Item not found at index {0}")]
    ItemNotFound(usize),

    #[error("Character not found at index {0}")]
    CharacterNotFound(usize),
}

// ===== Stat Range Constants =====
//
// These constants define the valid ranges for character statistics.
// See docs/reference/stat_ranges.md for detailed documentation.

/// Minimum value for primary attributes (Might, Intellect, etc.)
pub const ATTRIBUTE_MIN: u8 = 3;

/// Maximum value for primary attributes (Might, Intellect, etc.)
pub const ATTRIBUTE_MAX: u8 = 255;

/// Default starting value for primary attributes
pub const ATTRIBUTE_DEFAULT: u8 = 10;

/// Minimum value for HP/SP (AttributePair16)
pub const HP_SP_MIN: u16 = 0;

/// Maximum value for HP/SP (AttributePair16)
pub const HP_SP_MAX: u16 = 9999;

/// Minimum Armor Class value
pub const AC_MIN: u8 = 0;

/// Maximum Armor Class value (practical limit)
pub const AC_MAX: u8 = 30;

/// Default Armor Class for unarmored character
pub const AC_DEFAULT: u8 = 10;

/// Minimum character level
pub const LEVEL_MIN: u32 = 1;

/// Maximum character level
pub const LEVEL_MAX: u32 = 200;

/// Minimum spell level
pub const SPELL_LEVEL_MIN: u8 = 1;

/// Maximum spell level
pub const SPELL_LEVEL_MAX: u8 = 7;

/// Minimum character age
pub const AGE_MIN: u16 = 18;

/// Maximum character age before death from old age
pub const AGE_MAX: u16 = 200;

/// Minimum food units
pub const FOOD_MIN: u8 = 0;

/// Maximum food units per character
pub const FOOD_MAX: u8 = 40;

/// Default starting food units
pub const FOOD_DEFAULT: u8 = 10;

/// Minimum resistance value (0%)
pub const RESISTANCE_MIN: u8 = 0;

/// Maximum resistance value (100%)
pub const RESISTANCE_MAX: u8 = 100;

/// Maximum party size
pub const PARTY_MAX_SIZE: usize = 6;

/// Maximum roster size (characters stored at inns)
pub const ROSTER_MAX_SIZE: usize = 18;

/// Maximum inventory slots per character
pub const INVENTORY_MAX_SLOTS: usize = 6;

/// Maximum equipment slots per character
pub const EQUIPMENT_MAX_SLOTS: usize = 6;

/// Minimum attribute modifier value (for effects/conditions)
pub const ATTRIBUTE_MODIFIER_MIN: i16 = -255;

/// Maximum attribute modifier value (for effects/conditions)
pub const ATTRIBUTE_MODIFIER_MAX: i16 = 255;

// ===== Core Pattern: AttributePair =====

/// Core pattern: base value + current temporary value for buffs/debuffs
///
/// When saving, save base. When loading, restore current = base.
///
/// # Examples
///
/// ```
/// use antares::domain::character::AttributePair;
///
/// let mut attr = AttributePair::new(10);
/// assert_eq!(attr.current, 10);
///
/// attr.modify(5); // Buff
/// assert_eq!(attr.current, 15);
///
/// attr.reset();
/// assert_eq!(attr.current, 10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AttributePair {
    /// Permanent base value
    pub base: u8,
    /// Current value (includes temporary buffs/debuffs)
    pub current: u8,
}

impl AttributePair {
    /// Creates a new AttributePair with the same base and current value
    pub fn new(value: u8) -> Self {
        Self {
            base: value,
            current: value,
        }
    }

    /// Reset temporary value to base value
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::AttributePair;
    ///
    /// let mut attr = AttributePair::new(10);
    /// attr.current = 15;
    /// attr.reset();
    /// assert_eq!(attr.current, 10);
    /// ```
    pub fn reset(&mut self) {
        self.current = self.base;
    }

    /// Apply a temporary modifier (positive or negative)
    ///
    /// Uses saturating arithmetic to prevent overflow/underflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::AttributePair;
    ///
    /// let mut attr = AttributePair::new(10);
    /// attr.modify(5);
    /// assert_eq!(attr.current, 15);
    /// attr.modify(-8);
    /// assert_eq!(attr.current, 7);
    /// ```
    pub fn modify(&mut self, amount: i16) {
        self.current = self.current.saturating_add_signed(amount as i8);
    }
}

impl std::fmt::Display for AttributePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.base == self.current {
            write!(f, "{}", self.base)
        } else {
            write!(f, "{}/{}", self.current, self.base)
        }
    }
}

// Deserialization: accept either a simple number or an object with base/current.
// This maintains backward compatibility for data files that use raw numbers for attributes.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum AttributePairDef {
    Full { base: u8, current: u8 },
    Simple(u8),
}

impl<'de> serde::Deserialize<'de> for AttributePair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = AttributePairDef::deserialize(deserializer)?;
        match helper {
            AttributePairDef::Full { base, current } => Ok(AttributePair { base, current }),
            AttributePairDef::Simple(v) => Ok(AttributePair::new(v)),
        }
    }
}

/// AttributePair for 16-bit values (HP, SP)
///
/// # Examples
///
/// ```
/// use antares::domain::character::AttributePair16;
///
/// let mut hp = AttributePair16::new(50);
/// hp.modify(-20);
/// assert_eq!(hp.current, 30);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AttributePair16 {
    /// Permanent base value
    pub base: u16,
    /// Current value (includes temporary buffs/debuffs)
    pub current: u16,
}

impl AttributePair16 {
    /// Creates a new AttributePair16 with the same base and current value
    pub fn new(value: u16) -> Self {
        Self {
            base: value,
            current: value,
        }
    }

    /// Reset temporary value to base value
    pub fn reset(&mut self) {
        self.current = self.base;
    }

    /// Apply a temporary modifier (positive or negative)
    pub fn modify(&mut self, amount: i32) {
        self.current = self.current.saturating_add_signed(amount as i16);
    }
}

impl std::fmt::Display for AttributePair16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.base == self.current {
            write!(f, "{}", self.base)
        } else {
            write!(f, "{}/{}", self.current, self.base)
        }
    }
}

// Deserialization: accept either a raw u16 (interpreted as base == current) or the struct form.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum AttributePair16Def {
    Full { base: u16, current: u16 },
    Simple(u16),
}

impl<'de> serde::Deserialize<'de> for AttributePair16 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = AttributePair16Def::deserialize(deserializer)?;
        match helper {
            AttributePair16Def::Full { base, current } => Ok(AttributePair16 { base, current }),
            AttributePair16Def::Simple(v) => Ok(AttributePair16::new(v)),
        }
    }
}

// ===== Primary Character Attributes =====

/// Primary character attributes
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Stats, AttributePair};
///
/// let stats = Stats::new(15, 10, 12, 14, 11, 13, 8);
/// assert_eq!(stats.might.base, 15);
/// assert_eq!(stats.intellect.base, 10);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    /// Physical strength, melee damage
    pub might: AttributePair,
    /// Magical power, spell effectiveness
    pub intellect: AttributePair,
    /// Charisma, social interactions
    pub personality: AttributePair,
    /// Constitution, HP calculation
    pub endurance: AttributePair,
    /// Initiative, dodging, turn order
    pub speed: AttributePair,
    /// Hit chance, ranged attacks
    pub accuracy: AttributePair,
    /// Critical hits, random events, loot
    pub luck: AttributePair,
}

impl Stats {
    /// Creates a new Stats struct with the given base values
    pub fn new(
        might: u8,
        intellect: u8,
        personality: u8,
        endurance: u8,
        speed: u8,
        accuracy: u8,
        luck: u8,
    ) -> Self {
        Self {
            might: AttributePair::new(might),
            intellect: AttributePair::new(intellect),
            personality: AttributePair::new(personality),
            endurance: AttributePair::new(endurance),
            speed: AttributePair::new(speed),
            accuracy: AttributePair::new(accuracy),
            luck: AttributePair::new(luck),
        }
    }

    /// Resets all stats to their base values
    pub fn reset_all(&mut self) {
        self.might.reset();
        self.intellect.reset();
        self.personality.reset();
        self.endurance.reset();
        self.speed.reset();
        self.accuracy.reset();
        self.luck.reset();
    }
}

// ===== Resistances =====

/// Resistances to various damage types and effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resistances {
    /// Generic magic resistance
    pub magic: AttributePair,
    /// Fire damage reduction
    pub fire: AttributePair,
    /// Cold damage reduction
    pub cold: AttributePair,
    /// Lightning damage reduction
    pub electricity: AttributePair,
    /// Acid damage reduction
    pub acid: AttributePair,
    /// Fear effect resistance
    pub fear: AttributePair,
    /// Poison resistance
    pub poison: AttributePair,
    /// Mental attack resistance
    pub psychic: AttributePair,
}

impl Resistances {
    /// Creates a new Resistances struct with all resistances at zero
    pub fn new() -> Self {
        Self {
            magic: AttributePair::new(0),
            fire: AttributePair::new(0),
            cold: AttributePair::new(0),
            electricity: AttributePair::new(0),
            acid: AttributePair::new(0),
            fear: AttributePair::new(0),
            poison: AttributePair::new(0),
            psychic: AttributePair::new(0),
        }
    }

    /// Resets all resistances to their base values
    pub fn reset_all(&mut self) {
        self.magic.reset();
        self.fire.reset();
        self.cold.reset();
        self.electricity.reset();
        self.acid.reset();
        self.fear.reset();
        self.poison.reset();
        self.psychic.reset();
    }
}

impl Default for Resistances {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Enums =====

/// Character sex
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
    Other,
}

/// Character alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Good,
    Neutral,
    Evil,
}

/// Character location tracking for party and roster management
///
/// This enum tracks where a character currently resides in the game world.
/// Characters can be in the active party, stored at an inn, or available
/// as recruitable characters on specific maps.
///
/// # Examples
///
/// ```
/// use antares::domain::character::CharacterLocation;
///
/// // Character in active party
/// let loc = CharacterLocation::InParty;
///
/// // Character stored at an inn by innkeeper NPC ID (string)
/// let loc = CharacterLocation::AtInn("tutorial_innkeeper_town".to_string());
///
/// // Character available for recruitment on map 5
/// let loc = CharacterLocation::OnMap(5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterLocation {
    /// Character is in the active party
    InParty,

    /// Character is stored at a specific inn (references an innkeeper NPC)
    AtInn(InnkeeperId),

    /// Character is available on a specific map (for recruitment encounters)
    OnMap(MapId),
}

// ===== Condition Flags =====

/// Character conditions (can have multiple via bitflags)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
///
/// let mut condition = Condition::new();
/// assert!(condition.is_fine());
///
/// condition.add(Condition::POISONED);
/// assert!(condition.has(Condition::POISONED));
/// assert!(!condition.is_fine());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition(u8);

impl Condition {
    pub const FINE: u8 = 0;
    pub const ASLEEP: u8 = 1;
    pub const BLINDED: u8 = 2;
    pub const SILENCED: u8 = 4;
    pub const DISEASED: u8 = 8;
    pub const POISONED: u8 = 16;
    pub const PARALYZED: u8 = 32;
    pub const UNCONSCIOUS: u8 = 64;
    pub const DEAD: u8 = 128;
    pub const STONE: u8 = 160; // Dead + special
    pub const ERADICATED: u8 = 255; // Permanent death

    /// Creates a new Condition with FINE status
    pub fn new() -> Self {
        Self(Self::FINE)
    }

    /// Adds a condition flag
    pub fn add(&mut self, flag: u8) {
        self.0 |= flag;
    }

    /// Removes a condition flag
    pub fn remove(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    /// Checks if a specific condition is present
    pub fn has(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    /// Returns true if the character is fine (no conditions)
    pub fn is_fine(&self) -> bool {
        self.0 == Self::FINE
    }

    /// Returns true if the character has bad conditions (paralyzed or worse)
    pub fn is_bad(&self) -> bool {
        self.0 >= Self::PARALYZED
    }

    /// Returns true if the character is dead or worse
    pub fn is_fatal(&self) -> bool {
        self.0 >= Self::DEAD
    }

    /// Returns true if the character is unconscious
    pub fn is_unconscious(&self) -> bool {
        self.has(Self::UNCONSCIOUS)
    }

    /// Returns true if the character is silenced
    pub fn is_silenced(&self) -> bool {
        self.has(Self::SILENCED)
    }

    /// Clears all conditions (sets to FINE)
    pub fn clear(&mut self) {
        self.0 = Self::FINE;
    }
}

impl Default for Condition {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Inventory =====

/// Inventory slot with item ID and charges
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySlot {
    pub item_id: ItemId,
    /// Charges remaining for magical items (0 = useless)
    pub charges: u8,
}

/// Container for items (backpack)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Inventory;
///
/// let mut inventory = Inventory::new();
/// assert!(inventory.has_space());
/// assert!(!inventory.is_full());
/// ```
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventorySlot>,
}

impl Inventory {
    /// Maximum number of items in backpack
    pub const MAX_ITEMS: usize = 64;

    /// Creates a new empty inventory
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Returns true if the inventory is full
    pub fn is_full(&self) -> bool {
        self.items.len() >= Self::MAX_ITEMS
    }

    /// Returns true if there is space for more items
    pub fn has_space(&self) -> bool {
        self.items.len() < Self::MAX_ITEMS
    }

    /// Adds an item to the inventory
    ///
    /// Returns `Ok(())` if successful, `Err(())` if inventory is full
    pub fn add_item(&mut self, item_id: ItemId, charges: u8) -> Result<(), CharacterError> {
        if self.is_full() {
            return Err(CharacterError::InventoryFull(Self::MAX_ITEMS));
        }
        self.items.push(InventorySlot { item_id, charges });
        Ok(())
    }

    /// Removes an item from the inventory by index
    pub fn remove_item(&mut self, index: usize) -> Option<InventorySlot> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Equipment =====

/// Equipped items in specific slots
///
/// # Examples
///
/// ```
/// use antares::domain::character::Equipment;
///
/// let mut equipment = Equipment::new();
/// assert_eq!(equipment.equipped_count(), 0);
/// ```
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    pub weapon: Option<ItemId>,
    pub armor: Option<ItemId>,
    pub shield: Option<ItemId>,
    pub helmet: Option<ItemId>,
    pub boots: Option<ItemId>,
    pub accessory1: Option<ItemId>,
    pub accessory2: Option<ItemId>,
}

impl Equipment {
    /// Maximum number of equipped items
    pub const MAX_EQUIPPED: usize = 6;

    /// Creates a new Equipment with all slots empty
    pub fn new() -> Self {
        Self {
            weapon: None,
            armor: None,
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        }
    }

    /// Count currently equipped items
    pub fn equipped_count(&self) -> usize {
        [
            &self.weapon,
            &self.armor,
            &self.shield,
            &self.helmet,
            &self.boots,
            &self.accessory1,
        ]
        .iter()
        .filter(|slot| slot.is_some())
        .count()
    }

    /// Returns `true` if the given item is currently occupying any equipment slot.
    ///
    /// Checks all seven slots: weapon, armor, shield, helmet, boots, accessory1,
    /// and accessory2. This is used by the merchant sell guard to prevent the
    /// player from selling a cursed item that is currently equipped.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::Equipment;
    ///
    /// let mut eq = Equipment::new();
    /// eq.weapon = Some(42);
    ///
    /// assert!(eq.is_item_equipped(42));
    /// assert!(!eq.is_item_equipped(99));
    /// ```
    pub fn is_item_equipped(&self, item_id: crate::domain::types::ItemId) -> bool {
        [
            self.weapon,
            self.armor,
            self.shield,
            self.helmet,
            self.boots,
            self.accessory1,
            self.accessory2,
        ]
        .contains(&Some(item_id))
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

// ===== SpellBook =====

/// Character's known spells organized by school and level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpellBook {
    /// Cleric spells by level (1-7)
    pub cleric_spells: [Vec<SpellId>; 7],
    /// Sorcerer spells by level (1-7)
    pub sorcerer_spells: [Vec<SpellId>; 7],
}

impl SpellBook {
    /// Creates a new empty SpellBook
    pub fn new() -> Self {
        Self {
            cleric_spells: Default::default(),
            sorcerer_spells: Default::default(),
        }
    }

    /// Returns the appropriate spell list for the character's class using class_id
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier string
    ///
    /// # Returns
    ///
    /// Returns the appropriate spell list based on the class ID.
    /// Cleric and Paladin use cleric spells, Sorcerer and Archer use sorcerer spells.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::SpellBook;
    ///
    /// let spellbook = SpellBook::new();
    /// let cleric_spells = spellbook.get_spell_list("cleric");
    /// let sorcerer_spells = spellbook.get_spell_list("sorcerer");
    /// ```
    pub fn get_spell_list(&self, class_id: &str) -> &[Vec<SpellId>; 7] {
        match class_id {
            "cleric" | "paladin" => &self.cleric_spells,
            "sorcerer" | "archer" => &self.sorcerer_spells,
            _ => &self.sorcerer_spells, // Default to empty
        }
    }

    /// Returns the mutable spell list for the character's class using class_id
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier string
    ///
    /// # Returns
    ///
    /// Returns the appropriate mutable spell list based on the class ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::SpellBook;
    ///
    /// let mut spellbook = SpellBook::new();
    /// let cleric_spells = spellbook.get_spell_list_mut("cleric");
    /// ```
    pub fn get_spell_list_mut(&mut self, class_id: &str) -> &mut [Vec<SpellId>; 7] {
        match class_id {
            "cleric" | "paladin" => &mut self.cleric_spells,
            "sorcerer" | "archer" => &mut self.sorcerer_spells,
            _ => &mut self.sorcerer_spells,
        }
    }

    /// Returns the appropriate spell list for a class using ClassDatabase
    ///
    /// This is the data-driven version that looks up class definitions from the database.
    /// Use this when working with campaign-specific or modded classes.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class ID to look up
    /// * `class_db` - Reference to the class database
    ///
    /// # Returns
    ///
    /// Returns the appropriate spell list based on the class's spell school.
    /// Returns the sorcerer spell list as default if the class is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::SpellBook;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
    /// let spellbook = SpellBook::new();
    ///
    /// // Cleric gets cleric spells
    /// let cleric_spells = spellbook.get_spell_list_by_id("cleric", &db);
    /// // Sorcerer gets sorcerer spells
    /// let sorcerer_spells = spellbook.get_spell_list_by_id("sorcerer", &db);
    /// ```
    pub fn get_spell_list_by_id(
        &self,
        class_id: &str,
        class_db: &ClassDatabase,
    ) -> &[Vec<SpellId>; 7] {
        let Some(class_def) = class_db.get_class(class_id) else {
            return &self.sorcerer_spells; // Default fallback
        };

        match &class_def.spell_school {
            Some(ClassSpellSchool::Cleric) => &self.cleric_spells,
            Some(ClassSpellSchool::Sorcerer) => &self.sorcerer_spells,
            None => &self.sorcerer_spells, // Non-casters default to empty sorcerer list
        }
    }

    /// Returns the mutable spell list for a class using ClassDatabase
    ///
    /// This is the data-driven version that looks up class definitions from the database.
    /// Use this when working with campaign-specific or modded classes.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class ID to look up
    /// * `class_db` - Reference to the class database
    ///
    /// # Returns
    ///
    /// Returns the appropriate mutable spell list based on the class's spell school.
    /// Returns the sorcerer spell list as default if the class is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::SpellBook;
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
    /// let mut spellbook = SpellBook::new();
    ///
    /// // Add a spell to cleric's list
    /// let cleric_spells = spellbook.get_spell_list_mut_by_id("cleric", &db);
    /// cleric_spells[0].push(0x0101); // Add level 1 spell
    /// ```
    pub fn get_spell_list_mut_by_id(
        &mut self,
        class_id: &str,
        class_db: &ClassDatabase,
    ) -> &mut [Vec<SpellId>; 7] {
        // Need to look up class first, then match
        // Since we can't hold a reference across the mutable borrow,
        // we determine the school type first
        let uses_cleric_school = class_db
            .get_class(class_id)
            .and_then(|c| c.spell_school.as_ref())
            .map(|s| matches!(s, ClassSpellSchool::Cleric))
            .unwrap_or(false);

        if uses_cleric_school {
            &mut self.cleric_spells
        } else {
            &mut self.sorcerer_spells
        }
    }
}

impl Default for SpellBook {
    fn default() -> Self {
        Self::new()
    }
}

// ===== QuestFlags =====

/// Per-character quest and event tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestFlags {
    /// Indexed flags for game events
    pub flags: Vec<bool>,
}

impl QuestFlags {
    /// Creates a new QuestFlags with no flags set
    pub fn new() -> Self {
        Self { flags: Vec::new() }
    }

    /// Sets a flag by index
    pub fn set_flag(&mut self, index: usize) {
        if index >= self.flags.len() {
            self.flags.resize(index + 1, false);
        }
        self.flags[index] = true;
    }

    /// Gets a flag by index
    pub fn get_flag(&self, index: usize) -> bool {
        self.flags.get(index).copied().unwrap_or(false)
    }
}

impl Default for QuestFlags {
    fn default() -> Self {
        Self::new()
    }
}

// ===== TimedStatBoost =====

/// A reversible timed attribute boost applied by a consumable item.
///
/// When `minutes_remaining` reaches zero the boost is reversed by subtracting
/// `amount` from `stats.<attribute>.current`.
///
/// # Examples
///
/// ```
/// use antares::domain::character::TimedStatBoost;
/// use antares::domain::items::types::AttributeType;
///
/// let boost = TimedStatBoost {
///     attribute: AttributeType::Might,
///     amount: 5,
///     minutes_remaining: 30,
/// };
/// assert_eq!(boost.minutes_remaining, 30);
/// assert_eq!(boost.amount, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedStatBoost {
    /// Which attribute this boost modifies.
    pub attribute: crate::domain::items::types::AttributeType,
    /// Signed delta applied to `current` (positive = boost, negative = penalty).
    pub amount: i8,
    /// Minutes remaining before the boost expires and is reversed.
    pub minutes_remaining: u16,
}

// ===== Character =====

/// Represents a single character (party member or roster character)
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let hero = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// assert_eq!(hero.name, "Sir Lancelot");
/// assert_eq!(hero.race_id, "human");
/// assert_eq!(hero.class_id, "knight");
/// assert_eq!(hero.level, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    /// Data-driven race identifier (e.g., "human", "elf")
    /// Used for lookups in RaceDatabase.
    pub race_id: RaceId,
    /// Data-driven class identifier (e.g., "knight", "sorcerer")
    /// Used for lookups in ClassDatabase.
    pub class_id: ClassId,
    pub sex: Sex,
    /// Current alignment
    pub alignment: Alignment,
    /// Starting alignment (for tracking changes)
    pub alignment_initial: Alignment,
    pub level: u32,
    pub experience: u64,
    /// Age in years (starts at 18)
    pub age: u16,
    /// Day counter for aging
    pub age_days: u32,
    pub stats: Stats,
    /// Hit points (current/base)
    pub hp: AttributePair16,
    /// Spell points (current/base)
    pub sp: AttributePair16,
    /// Armor class (higher is better, 0-30 range)
    pub ac: AttributePair,
    /// Max spell level castable
    pub spell_level: AttributePair,
    /// Backpack items (max 6)
    pub inventory: Inventory,
    /// Equipped items (max 6)
    pub equipment: Equipment,
    /// Known spells
    pub spells: SpellBook,
    /// Active status conditions (bitflags)
    pub conditions: Condition,
    /// Active data-driven conditions
    pub active_conditions: Vec<crate::domain::conditions::ActiveCondition>,
    /// Active timed attribute boosts from consumable items.
    /// Each entry is reversed when `minutes_remaining` reaches zero.
    #[serde(default)]
    pub timed_stat_boosts: Vec<TimedStatBoost>,
    /// Damage resistances
    pub resistances: Resistances,
    /// Per-character quest/event tracking
    pub quest_flags: QuestFlags,
    /// Portrait/avatar ID (filename stem / unique string)
    pub portrait_id: String,
    /// Special quest attribute
    pub worthiness: u8,
    /// Individual gold (0-max)
    pub gold: u32,
    /// Individual gems (0-max)
    pub gems: u32,
    /// Legacy food counter — deprecated in Phase 2.
    ///
    /// Food is now represented as `ConsumableEffect::IsFood` inventory items
    /// (e.g. "Food Ration", item id 53).  The rest system reads food from
    /// character inventories via [`crate::domain::resources::count_food_in_party`].
    ///
    /// This field is kept for save-game backward compatibility but is **no
    /// longer read or written by any game logic**.  Do not rely on it.
    #[deprecated(
        since = "0.2.0",
        note = "Use ConsumableEffect::IsFood inventory items instead; \
                see food_system_implementation_plan.md Phase 2"
    )]
    pub food: u8,
}

impl Character {
    /// Creates a new character with default starting values
    ///
    /// Uses data-driven race_id and class_id for all lookups.
    ///
    /// # Arguments
    ///
    /// * `name` - Character's display name
    /// * `race_id` - Race identifier (e.g., "human", "elf")
    /// * `class_id` - Class identifier (e.g., "knight", "sorcerer")
    /// * `sex` - Sex enum value
    /// * `alignment` - Alignment enum value
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Character, Sex, Alignment};
    ///
    /// let hero = Character::new(
    ///     "Sir Lancelot".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// assert_eq!(hero.race_id, "human");
    /// assert_eq!(hero.class_id, "knight");
    /// ```
    pub fn new(
        name: String,
        race_id: RaceId,
        class_id: ClassId,
        sex: Sex,
        alignment: Alignment,
    ) -> Self {
        #[allow(deprecated)]
        Self {
            name,
            race_id,
            class_id,
            sex,
            alignment,
            alignment_initial: alignment,
            level: 1,
            experience: 0,
            age: 18,
            age_days: 0,
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: AttributePair16::new(10),
            sp: AttributePair16::new(0),
            ac: AttributePair::new(0),
            spell_level: AttributePair::new(0),
            inventory: Inventory::new(),
            equipment: Equipment::new(),
            spells: SpellBook::new(),
            conditions: Condition::new(),
            active_conditions: Vec::new(),
            timed_stat_boosts: Vec::new(),
            resistances: Resistances::new(),
            quest_flags: QuestFlags::new(),
            portrait_id: String::new(),
            worthiness: 0,
            gold: 0,
            gems: 0,
            food: 0,
        }
    }

    /// Returns true if the character is alive (not dead, stoned, or eradicated)
    pub fn is_alive(&self) -> bool {
        !self.conditions.is_fatal()
    }

    /// Returns true if the character can act in combat
    pub fn can_act(&self) -> bool {
        self.is_alive() && !self.conditions.is_bad()
    }

    /// Adds a condition to the character
    pub fn add_condition(&mut self, condition: crate::domain::conditions::ActiveCondition) {
        // Check if condition already exists, if so, refresh/overwrite it
        if let Some(existing) = self
            .active_conditions
            .iter_mut()
            .find(|c| c.condition_id == condition.condition_id)
        {
            existing.duration = condition.duration;
        } else {
            self.active_conditions.push(condition);
        }
    }

    /// Removes a condition by ID
    pub fn remove_condition(&mut self, condition_id: &str) {
        self.active_conditions
            .retain(|c| c.condition_id != condition_id);
    }

    /// Updates conditions based on round tick
    pub fn tick_conditions_round(&mut self) {
        self.active_conditions.retain_mut(|c| !c.tick_round());
    }

    /// Updates conditions based on minute tick
    pub fn tick_conditions_minute(&mut self) {
        self.active_conditions.retain_mut(|c| !c.tick_minute());
    }

    /// Applies a timed attribute boost and records it for later reversal.
    ///
    /// Calls [`crate::domain::items::types::normalize_duration`] on
    /// `duration_minutes` before storing, so `Some(0)` behaves identically to
    /// `None` — no boost is applied and no entry is stored.
    ///
    /// # Arguments
    ///
    /// * `attr` — the attribute to boost
    /// * `amount` — signed delta (positive = increase, negative = decrease)
    /// * `duration_minutes` — `Some(n)` for a timed boost; `None` or `Some(0)`
    ///   means permanent (no entry is stored and no current value is changed)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Character, Sex, Alignment};
    /// use antares::domain::items::types::AttributeType;
    ///
    /// let mut hero = Character::new(
    ///     "Hero".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// let base_might = hero.stats.might.current;
    /// hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(30));
    /// assert_eq!(hero.stats.might.current, base_might + 5);
    /// assert_eq!(hero.timed_stat_boosts.len(), 1);
    /// assert_eq!(hero.timed_stat_boosts[0].minutes_remaining, 30);
    /// ```
    pub fn apply_timed_stat_boost(
        &mut self,
        attr: crate::domain::items::types::AttributeType,
        amount: i8,
        duration_minutes: Option<u16>,
    ) {
        use crate::domain::items::types::normalize_duration;
        let Some(minutes) = normalize_duration(duration_minutes) else {
            return;
        };
        self.apply_attribute_delta(attr, amount as i16);
        self.timed_stat_boosts.push(TimedStatBoost {
            attribute: attr,
            amount,
            minutes_remaining: minutes,
        });
    }

    /// Ticks all timed stat boosts by one minute.
    ///
    /// Boosts whose `minutes_remaining` reaches zero are expired and reversed:
    /// the original `amount` is subtracted from the corresponding `current`
    /// attribute value.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Character, Sex, Alignment};
    /// use antares::domain::items::types::AttributeType;
    ///
    /// let mut hero = Character::new(
    ///     "Hero".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// let base_might = hero.stats.might.current;
    /// hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(2));
    ///
    /// // After 1 minute: boost still active
    /// hero.tick_timed_stat_boosts_minute();
    /// assert_eq!(hero.stats.might.current, base_might + 5);
    /// assert_eq!(hero.timed_stat_boosts[0].minutes_remaining, 1);
    ///
    /// // After 2nd minute: boost expires and is reversed
    /// hero.tick_timed_stat_boosts_minute();
    /// assert_eq!(hero.stats.might.current, base_might);
    /// assert!(hero.timed_stat_boosts.is_empty());
    /// ```
    pub fn tick_timed_stat_boosts_minute(&mut self) {
        let mut expired: Vec<TimedStatBoost> = Vec::new();
        self.timed_stat_boosts.retain_mut(|boost| {
            boost.minutes_remaining = boost.minutes_remaining.saturating_sub(1);
            if boost.minutes_remaining == 0 {
                expired.push(boost.clone());
                false
            } else {
                true
            }
        });
        for boost in expired {
            self.apply_attribute_delta(boost.attribute, -(boost.amount as i16));
        }
    }

    /// Applies a signed delta to the `current` value of the named attribute.
    ///
    /// This is the single authoritative mapping from [`crate::domain::items::types::AttributeType`]
    /// to a [`Character`] stats field.  It is used by both
    /// [`Character::apply_timed_stat_boost`] (to apply the initial boost) and
    /// [`Character::tick_timed_stat_boosts_minute`] (to reverse expired boosts).
    ///
    /// Modification is performed via [`crate::domain::character::AttributePair::modify`],
    /// which saturates at the `u8` type boundary — a delta that would push
    /// `current` below 0 clamps to 0; a delta that would exceed 255 clamps to
    /// 255.
    ///
    /// # Arguments
    ///
    /// * `attr`  — which attribute to modify (maps to the corresponding
    ///   `self.stats.<field>` member).
    /// * `delta` — signed amount to add to `current` (positive = increase,
    ///   negative = decrease / reversal).
    ///
    /// # Returns
    ///
    /// `()` — the character is mutated in place.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Character, Sex, Alignment};
    /// use antares::domain::items::types::AttributeType;
    ///
    /// let mut hero = Character::new(
    ///     "Hero".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// let base = hero.stats.might.current;
    ///
    /// // Apply a +5 boost to Might.
    /// hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(10));
    /// assert_eq!(hero.stats.might.current, base + 5);
    ///
    /// // Tick 10 minutes — the reversal delta of -5 is applied via apply_attribute_delta.
    /// for _ in 0..10 {
    ///     hero.tick_timed_stat_boosts_minute();
    /// }
    /// assert_eq!(hero.stats.might.current, base);
    /// ```
    pub(crate) fn apply_attribute_delta(
        &mut self,
        attr: crate::domain::items::types::AttributeType,
        delta: i16,
    ) {
        use crate::domain::items::types::AttributeType;
        match attr {
            AttributeType::Might => self.stats.might.modify(delta),
            AttributeType::Intellect => self.stats.intellect.modify(delta),
            AttributeType::Personality => self.stats.personality.modify(delta),
            AttributeType::Endurance => self.stats.endurance.modify(delta),
            AttributeType::Speed => self.stats.speed.modify(delta),
            AttributeType::Accuracy => self.stats.accuracy.modify(delta),
            AttributeType::Luck => self.stats.luck.modify(delta),
        }
    }

    /// Calculates the total modifier from active conditions for a given attribute
    pub fn get_condition_modifier(
        &self,
        attribute: &str,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> i16 {
        let mut total_modifier = 0i16;

        for active in &self.active_conditions {
            // Find the definition
            if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
                for effect in &def.effects {
                    if let crate::domain::conditions::ConditionEffect::AttributeModifier {
                        attribute: attr,
                        value,
                    } = effect
                    {
                        if attr == attribute {
                            let modified = (*value as f32 * active.magnitude).round() as i16;
                            total_modifier = total_modifier.saturating_add(modified);
                        }
                    }
                }
            }
        }

        total_modifier
    }

    /// Returns true if character has a specific status effect from conditions
    pub fn has_status_effect(
        &self,
        status: &str,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> bool {
        for active in &self.active_conditions {
            if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
                for effect in &def.effects {
                    if let crate::domain::conditions::ConditionEffect::StatusEffect(s) = effect {
                        if s == status {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

// ===== Party =====

/// Active party (max 6 characters)
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment};
///
/// let mut party = Party::new();
/// let hero = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// assert!(party.add_member(hero).is_ok());
/// assert_eq!(party.members.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    /// 0-6 active characters
    pub members: Vec<Character>,
    /// Party gold (can be pooled)
    pub gold: u32,
    /// Party gems (can be pooled)
    pub gems: u32,
    /// Legacy party food pool — deprecated in Phase 2.
    ///
    /// Food is now tracked as `ConsumableEffect::IsFood` items in each
    /// character's inventory.  This field is retained for save-game
    /// backward compatibility only and is **not read by any rest logic**.
    #[deprecated(
        since = "0.2.0",
        note = "Use ConsumableEffect::IsFood inventory items instead; \
                see food_system_implementation_plan.md Phase 2"
    )]
    pub food: u32,
    /// Combat: which positions can attack
    pub position_index: [bool; 6],
    /// Available light units for dark areas
    pub light_units: u8,
}

impl Party {
    /// Maximum party size
    pub const MAX_MEMBERS: usize = 6;

    /// Creates a new empty party
    pub fn new() -> Self {
        #[allow(deprecated)]
        Self {
            members: Vec::new(),
            gold: 0,
            gems: 0,
            food: 0,
            position_index: [true, true, true, false, false, false],
            light_units: 0,
        }
    }

    /// Adds a member to the party
    ///
    /// Returns `Ok(())` if successful, `Err(())` if party is full
    pub fn add_member(&mut self, character: Character) -> Result<(), CharacterError> {
        if self.members.len() >= Self::MAX_MEMBERS {
            return Err(CharacterError::PartyFull(Self::MAX_MEMBERS));
        }
        self.members.push(character);
        Ok(())
    }

    /// Removes a member from the party by index
    pub fn remove_member(&mut self, index: usize) -> Option<Character> {
        if index < self.members.len() {
            Some(self.members.remove(index))
        } else {
            None
        }
    }

    /// Returns true if the party is empty
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns true if the party is full
    pub fn is_full(&self) -> bool {
        self.members.len() >= Self::MAX_MEMBERS
    }

    /// Returns the number of members in the party
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Returns the number of living members
    pub fn living_count(&self) -> usize {
        self.members.iter().filter(|c| c.is_alive()).count()
    }
}

impl Default for Party {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Roster =====

/// Character roster (character pool)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roster {
    /// Up to 18 characters total
    pub characters: Vec<Character>,
    /// Where each character is located (party, inn, or map)
    pub character_locations: Vec<CharacterLocation>,
}

impl Roster {
    /// Maximum roster size
    pub const MAX_CHARACTERS: usize = 18;

    /// Creates a new empty roster
    pub fn new() -> Self {
        Self {
            characters: Vec::new(),
            character_locations: Vec::new(),
        }
    }

    /// Adds a character to the roster
    pub fn add_character(
        &mut self,
        character: Character,
        location: CharacterLocation,
    ) -> Result<(), CharacterError> {
        if self.characters.len() >= Self::MAX_CHARACTERS {
            return Err(CharacterError::RosterFull(Self::MAX_CHARACTERS));
        }
        self.characters.push(character);
        self.character_locations.push(location);
        Ok(())
    }

    /// Finds a character in the roster by character ID
    ///
    /// Returns the roster index if found.
    ///
    /// # Arguments
    ///
    /// * `id` - Character ID to search for
    ///
    /// # Returns
    ///
    /// Returns `Some(index)` if character found, `None` otherwise
    pub fn find_character_by_id(&self, id: CharacterId) -> Option<usize> {
        // Character ID is the roster index in the current implementation
        if id < self.characters.len() {
            Some(id)
        } else {
            None
        }
    }

    /// Gets a reference to a character by roster index
    ///
    /// # Arguments
    ///
    /// * `index` - Roster index of the character
    ///
    /// # Returns
    ///
    /// Returns `Some(&Character)` if index valid, `None` otherwise
    pub fn get_character(&self, index: usize) -> Option<&Character> {
        self.characters.get(index)
    }

    /// Gets a mutable reference to a character by roster index
    ///
    /// # Arguments
    ///
    /// * `index` - Roster index of the character
    ///
    /// # Returns
    ///
    /// Returns `Some(&mut Character)` if index valid, `None` otherwise
    pub fn get_character_mut(&mut self, index: usize) -> Option<&mut Character> {
        self.characters.get_mut(index)
    }

    /// Updates the location of a character in the roster
    ///
    /// # Arguments
    ///
    /// * `index` - Roster index of the character
    /// * `location` - New location for the character
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful, `Err(CharacterError::CharacterNotFound)` if index invalid
    pub fn update_location(
        &mut self,
        index: usize,
        location: CharacterLocation,
    ) -> Result<(), CharacterError> {
        if index >= self.character_locations.len() {
            return Err(CharacterError::CharacterNotFound(index));
        }
        self.character_locations[index] = location;
        Ok(())
    }

    /// Gets all characters currently at a specific inn
    ///
    /// Returns a vector of tuples containing (roster_index, character_reference)
    ///
    /// # Arguments
    ///
    /// * `innkeeper_id` - The innkeeper NPC ID (string) to search for
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Roster, Character, CharacterLocation, Sex, Alignment};
    /// use antares::domain::character::Stats;
    ///
    /// let mut roster = Roster::new();
    /// let char1 = Character::new("Hero".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
    /// roster.add_character(char1, CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())).unwrap();
    ///
    /// let at_inn_1 = roster.characters_at_inn("tutorial_innkeeper_town");
    /// assert_eq!(at_inn_1.len(), 1);
    /// ```
    pub fn characters_at_inn(&self, innkeeper_id: &str) -> Vec<(usize, &Character)> {
        self.character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if let CharacterLocation::AtInn(ref id) = loc {
                    if id == innkeeper_id {
                        return self.characters.get(idx).map(|c| (idx, c));
                    }
                }
                None
            })
            .collect()
    }

    /// Gets all characters currently in the active party
    ///
    /// Returns a vector of tuples containing (roster_index, character_reference)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Roster, Character, CharacterLocation, Sex, Alignment};
    /// use antares::domain::character::Stats;
    ///
    /// let mut roster = Roster::new();
    /// let char1 = Character::new("Hero".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
    /// roster.add_character(char1, CharacterLocation::InParty).unwrap();
    ///
    /// let in_party = roster.characters_in_party();
    /// assert_eq!(in_party.len(), 1);
    /// ```
    pub fn characters_in_party(&self) -> Vec<(usize, &Character)> {
        self.character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if *loc == CharacterLocation::InParty {
                    self.characters.get(idx).map(|c| (idx, c))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for Roster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::classes::ClassDatabase;
    use crate::domain::items::types::AttributeType;

    fn make_hero() -> Character {
        Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    // ===== Phase 2: TimedStatBoost tests =====

    #[test]
    fn test_timed_stat_boosts_defaults_empty_on_new_character() {
        let hero = make_hero();
        assert!(
            hero.timed_stat_boosts.is_empty(),
            "new Character must have no timed stat boosts"
        );
    }

    #[test]
    fn test_apply_timed_stat_boost_modifies_current_not_base() {
        let mut hero = make_hero();
        let base_before = hero.stats.might.base;
        let current_before = hero.stats.might.current;

        hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(30));

        assert_eq!(
            hero.stats.might.base, base_before,
            "base must not change after timed boost"
        );
        assert_eq!(
            hero.stats.might.current,
            current_before + 5,
            "current must increase by the boost amount"
        );
        assert_eq!(hero.timed_stat_boosts.len(), 1);
        assert_eq!(hero.timed_stat_boosts[0].minutes_remaining, 30);
    }

    #[test]
    fn test_apply_timed_stat_boost_none_duration_is_noop() {
        let mut hero = make_hero();
        let current_before = hero.stats.might.current;

        hero.apply_timed_stat_boost(AttributeType::Might, 5, None);

        assert_eq!(
            hero.stats.might.current, current_before,
            "None duration must not change current value"
        );
        assert!(
            hero.timed_stat_boosts.is_empty(),
            "None duration must not store a boost entry"
        );
    }

    #[test]
    fn test_apply_timed_stat_boost_zero_duration_is_noop() {
        let mut hero = make_hero();
        let current_before = hero.stats.might.current;

        hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(0));

        assert_eq!(
            hero.stats.might.current, current_before,
            "Some(0) duration must not change current value"
        );
        assert!(
            hero.timed_stat_boosts.is_empty(),
            "Some(0) duration must not store a boost entry"
        );
    }

    #[test]
    fn test_tick_timed_stat_boosts_decrements_counter() {
        let mut hero = make_hero();
        let current_before = hero.stats.might.current;

        hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(10));

        hero.tick_timed_stat_boosts_minute();

        assert_eq!(
            hero.timed_stat_boosts[0].minutes_remaining, 9,
            "minutes_remaining must decrement by 1"
        );
        assert_eq!(
            hero.stats.might.current,
            current_before + 5,
            "stat must remain boosted while counter > 0"
        );
    }

    #[test]
    fn test_tick_timed_stat_boosts_reverses_on_expiry() {
        let mut hero = make_hero();
        let current_before = hero.stats.might.current;

        hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(3));
        assert_eq!(hero.stats.might.current, current_before + 5);

        // Tick 3 times — boost expires on the 3rd tick
        for _ in 0..3 {
            hero.tick_timed_stat_boosts_minute();
        }

        assert_eq!(
            hero.stats.might.current, current_before,
            "stat must be restored after boost expires"
        );
        assert!(
            hero.timed_stat_boosts.is_empty(),
            "expired boost must be removed from the list"
        );
    }

    #[test]
    fn test_tick_timed_stat_boosts_multiple_boosts_independent() {
        let mut hero = make_hero();
        let base_might = hero.stats.might.current;
        let base_luck = hero.stats.luck.current;

        // Boost Might for 2 minutes, Luck for 4 minutes
        hero.apply_timed_stat_boost(AttributeType::Might, 3, Some(2));
        hero.apply_timed_stat_boost(AttributeType::Luck, 7, Some(4));

        assert_eq!(hero.stats.might.current, base_might + 3);
        assert_eq!(hero.stats.luck.current, base_luck + 7);

        // Tick 2 minutes — Might boost expires; Luck boost still active
        hero.tick_timed_stat_boosts_minute();
        hero.tick_timed_stat_boosts_minute();

        assert_eq!(
            hero.stats.might.current, base_might,
            "Might boost must have reversed after 2 ticks"
        );
        assert_eq!(
            hero.stats.luck.current,
            base_luck + 7,
            "Luck boost must still be active after 2 ticks"
        );
        assert_eq!(
            hero.timed_stat_boosts.len(),
            1,
            "only the Luck boost must remain"
        );

        // Tick 2 more minutes — Luck boost expires
        hero.tick_timed_stat_boosts_minute();
        hero.tick_timed_stat_boosts_minute();

        assert_eq!(
            hero.stats.luck.current, base_luck,
            "Luck boost must have reversed after 4 total ticks"
        );
        assert!(hero.timed_stat_boosts.is_empty());
    }

    #[test]
    fn test_timed_stat_boost_serde_default_deserializes() {
        // Serialise a character to RON, then strip the timed_stat_boosts field
        // to simulate a save file created before Phase 2 (the field did not
        // exist yet).  #[serde(default)] must cause it to deserialise as an
        // empty Vec rather than returning an error.
        let hero = make_hero();
        let serialized = ron::to_string(&hero).expect("serialization must succeed");

        // ron::to_string emits "timed_stat_boosts: []," (with a space and a
        // trailing comma inside the struct).  Strip both forms defensively.
        let stripped = serialized
            .replace("timed_stat_boosts: [],", "")
            .replace("timed_stat_boosts:[],", "")
            .replace("timed_stat_boosts: []", "")
            .replace("timed_stat_boosts:[]", "");

        // Confirm the field is actually gone before attempting deserialization.
        assert!(
            !stripped.contains("timed_stat_boosts"),
            "field must have been removed from the RON string; got: {stripped}"
        );

        let deserialized: Character = ron::from_str(&stripped)
            .expect("deserialization must succeed without timed_stat_boosts");
        assert!(
            deserialized.timed_stat_boosts.is_empty(),
            "missing timed_stat_boosts field must default to empty Vec"
        );
    }

    #[test]
    fn test_attribute_pair_new() {
        let attr = AttributePair::new(15);
        assert_eq!(attr.base, 15);
        assert_eq!(attr.current, 15);
    }

    #[test]
    fn test_character_location_at_inn_ron_serialization() {
        // Verify that CharacterLocation::AtInn with a string InnkeeperId
        // round-trips correctly through RON serialization/deserialization.
        let original = CharacterLocation::AtInn("tutorial_innkeeper_town".to_string());

        let ron_str = ron::ser::to_string_pretty(&original, Default::default())
            .expect("Failed to serialize CharacterLocation::AtInn to RON");

        let parsed: CharacterLocation = ron::de::from_str(&ron_str)
            .expect("Failed to deserialize CharacterLocation::AtInn from RON");

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_characters_at_inn_string_id() {
        // Verify that roster.characters_at_inn filters by innkeeper string ID
        let mut roster = Roster::new();

        let char1 = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let char2 = Character::new(
            "Mira".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );

        roster
            .add_character(
                char1,
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn("other_inn".to_string()))
            .unwrap();

        let at_inn = roster.characters_at_inn("tutorial_innkeeper_town");
        assert_eq!(at_inn.len(), 1);
        assert_eq!(at_inn[0].1.name, "Hero");
    }

    #[test]
    fn test_attribute_pair_modify() {
        let mut attr = AttributePair::new(10);
        attr.modify(5);
        assert_eq!(attr.current, 15);
        assert_eq!(attr.base, 10);
    }

    #[test]
    fn test_attribute_pair_reset() {
        let mut attr = AttributePair::new(10);
        attr.modify(5);
        attr.reset();
        assert_eq!(attr.current, attr.base);
    }

    #[test]
    fn test_inventory_max_items() {
        let mut inventory = Inventory::new();
        assert!(!inventory.is_full());

        // Fill inventory to max
        for i in 0..Inventory::MAX_ITEMS {
            assert!(inventory.add_item(i as ItemId, 1).is_ok());
        }

        assert!(inventory.is_full());
        assert!(inventory.add_item(99, 0).is_err());
    }

    #[test]
    fn test_equipment_count() {
        let mut equipment = Equipment::new();
        assert_eq!(equipment.equipped_count(), 0);

        equipment.weapon = Some(1);
        assert_eq!(equipment.equipped_count(), 1);

        equipment.armor = Some(2);
        assert_eq!(equipment.equipped_count(), 2);
    }

    #[test]
    fn test_condition_flags() {
        let mut condition = Condition::new();
        assert!(condition.is_fine());

        condition.add(Condition::POISONED);
        assert!(condition.has(Condition::POISONED));
        assert!(!condition.is_fine());
    }

    #[test]
    fn test_party_max_members() {
        let mut party = Party::new();
        assert!(party.is_empty());
        assert!(!party.is_full());

        // Fill party to max
        for i in 0..Party::MAX_MEMBERS {
            let character = Character::new(
                format!("Hero {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            assert!(party.add_member(character).is_ok());
        }

        assert!(party.is_full());
        assert_eq!(party.size(), Party::MAX_MEMBERS);

        // Try to add one more
        let extra = Character::new(
            "Extra".to_string(),
            "elf".to_string(),
            "archer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        assert!(party.add_member(extra).is_err());
    }

    #[test]
    fn test_character_creation() {
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        assert_eq!(hero.name, "Test Hero");
        assert_eq!(hero.race_id, "human");
        assert_eq!(hero.class_id, "knight");
        assert_eq!(hero.level, 1);
        assert_eq!(hero.experience, 0);
        assert!(hero.is_alive());
        assert!(hero.can_act());
    }

    // ===== Character ID Tests =====

    #[test]
    fn test_character_with_various_race_ids() {
        let race_ids = ["human", "elf", "dwarf", "gnome", "half_elf", "half_orc"];

        for race_id in race_ids {
            let character = Character::new(
                "Test".to_string(),
                race_id.to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            assert_eq!(character.race_id, race_id);
        }
    }

    #[test]
    fn test_character_with_various_class_ids() {
        let class_ids = [
            "knight", "paladin", "archer", "cleric", "sorcerer", "robber",
        ];

        for class_id in class_ids {
            let character = Character::new(
                "Test".to_string(),
                "human".to_string(),
                class_id.to_string(),
                Sex::Male,
                Alignment::Good,
            );
            assert_eq!(character.class_id, class_id);
        }
    }

    #[test]
    fn test_character_all_race_class_combinations() {
        // Test a sampling of race/class combinations
        let combos = [
            ("elf", "archer"),
            ("dwarf", "cleric"),
            ("gnome", "sorcerer"),
            ("half_orc", "robber"),
            ("human", "paladin"),
        ];

        for (race_id, class_id) in combos {
            let character = Character::new(
                "Test".to_string(),
                race_id.to_string(),
                class_id.to_string(),
                Sex::Male,
                Alignment::Neutral,
            );
            assert_eq!(character.race_id, race_id);
            assert_eq!(character.class_id, class_id);
        }
    }

    #[test]
    fn test_character_default_values() {
        let hero = Character::new(
            "Test Hero".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );

        // Check default starting values
        assert_eq!(hero.level, 1);
        assert_eq!(hero.experience, 0);
        assert_eq!(hero.age, 18);
        // Phase 2: food is now tracked as IsFood inventory items, not Character.food.
        // Character::new() sets food=0 (deprecated field).
        #[allow(deprecated)]
        {
            assert_eq!(hero.food, 0);
        }
        assert_eq!(hero.gold, 0);
        assert_eq!(hero.gems, 0);
        assert!(hero.is_alive());
        assert!(hero.can_act());
    }

    // ===== Serialization Tests =====

    #[test]
    fn test_character_serialization_with_ids() {
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        // Serialize to RON
        let serialized = ron::to_string(&hero).expect("Failed to serialize character");

        // Should contain ID fields
        assert!(serialized.contains("race_id"));
        assert!(serialized.contains("class_id"));
        assert!(serialized.contains("\"human\""));
        assert!(serialized.contains("\"knight\""));
    }

    #[test]
    fn test_character_deserialization_with_ids() {
        let hero = Character::new(
            "Test Hero".to_string(),
            "elf".to_string(),
            "archer".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );

        // Serialize and deserialize
        let serialized = ron::to_string(&hero).expect("Failed to serialize");
        let deserialized: Character = ron::from_str(&serialized).expect("Failed to deserialize");

        // Verify all fields match
        assert_eq!(deserialized.name, hero.name);
        assert_eq!(deserialized.race_id, hero.race_id);
        assert_eq!(deserialized.class_id, hero.class_id);
        assert_eq!(deserialized.sex, hero.sex);
        assert_eq!(deserialized.alignment, hero.alignment);
    }

    #[test]
    fn test_character_serialization_roundtrip() {
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        // Verify serialization round-trip preserves the IDs
        let serialized = ron::to_string(&hero).expect("Failed to serialize");
        let deserialized: Character = ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(deserialized.race_id, "human");
        assert_eq!(deserialized.class_id, "knight");
    }

    #[test]
    fn test_character_location_ron_serialization() {
        use super::CharacterLocation;

        let location = CharacterLocation::AtInn("test_innkeeper".to_string());

        // Serialize to RON and ensure the innkeeper ID appears in the output
        let ron_string =
            ron::to_string(&location).expect("Failed to serialize CharacterLocation to RON");
        assert!(ron_string.contains("test_innkeeper"));

        // Deserialize from RON and verify round-trip equality
        let deserialized: CharacterLocation =
            ron::from_str(&ron_string).expect("Failed to deserialize CharacterLocation from RON");
        assert_eq!(location, deserialized);
    }

    // ===== SpellBook Tests =====

    #[test]
    fn test_spellbook_get_spell_list_cleric() {
        let mut spellbook = SpellBook::new();

        // Add a spell to cleric list
        spellbook.cleric_spells[0].push(0x0101);

        // Cleric should get cleric spells
        let spell_list = spellbook.get_spell_list("cleric");
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_sorcerer() {
        let mut spellbook = SpellBook::new();

        // Add a spell to sorcerer list
        spellbook.sorcerer_spells[0].push(0x0201);

        // Sorcerer should get sorcerer spells
        let spell_list = spellbook.get_spell_list("sorcerer");
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0201);
    }

    #[test]
    fn test_spellbook_get_spell_list_paladin() {
        let mut spellbook = SpellBook::new();

        // Add a spell to cleric list
        spellbook.cleric_spells[0].push(0x0101);

        // Paladin (hybrid with cleric spells) should get cleric spells
        let spell_list = spellbook.get_spell_list("paladin");
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_knight() {
        let spellbook = SpellBook::new();

        // Knight (non-caster) defaults to sorcerer list (which is empty)
        let spell_list = spellbook.get_spell_list("knight");
        assert!(spell_list[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_unknown_class() {
        let spellbook = SpellBook::new();

        // Unknown class defaults to sorcerer list
        let spell_list = spellbook.get_spell_list("unknown");
        assert!(spell_list[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_cleric() {
        let mut spellbook = SpellBook::new();

        // Add a spell through the mutable accessor
        {
            let spell_list = spellbook.get_spell_list_mut("cleric");
            spell_list[0].push(0x0101);
        }

        // Verify it was added to cleric spells
        assert_eq!(spellbook.cleric_spells[0].len(), 1);
        assert_eq!(spellbook.cleric_spells[0][0], 0x0101);
        // Sorcerer list should be unaffected
        assert!(spellbook.sorcerer_spells[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_sorcerer() {
        let mut spellbook = SpellBook::new();

        // Add a spell through the mutable accessor
        {
            let spell_list = spellbook.get_spell_list_mut("sorcerer");
            spell_list[0].push(0x0201);
        }

        // Verify it was added to sorcerer spells
        assert_eq!(spellbook.sorcerer_spells[0].len(), 1);
        assert_eq!(spellbook.sorcerer_spells[0][0], 0x0201);
        // Cleric list should be unaffected
        assert!(spellbook.cleric_spells[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_paladin() {
        let mut spellbook = SpellBook::new();

        // Paladin should modify cleric spell list
        {
            let spell_list = spellbook.get_spell_list_mut("paladin");
            spell_list[0].push(0x0101);
        }

        // Verify it was added to cleric spells
        assert_eq!(spellbook.cleric_spells[0].len(), 1);
        assert_eq!(spellbook.cleric_spells[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_knight() {
        let mut spellbook = SpellBook::new();

        // Knight (non-caster) defaults to sorcerer list
        {
            let spell_list = spellbook.get_spell_list_mut("knight");
            spell_list[0].push(0x0201);
        }

        // Verify it was added to sorcerer spells (default)
        assert_eq!(spellbook.sorcerer_spells[0].len(), 1);
    }

    #[test]
    fn test_spellbook_id_and_db_methods_match() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add spells to both lists
        spellbook.cleric_spells[0].push(0x0101);
        spellbook.sorcerer_spells[0].push(0x0201);

        // Test that ID and db methods return the same lists
        let test_cases = ["cleric", "sorcerer", "paladin", "knight"];

        for class_id in test_cases {
            let id_list = spellbook.get_spell_list(class_id);
            let db_list = spellbook.get_spell_list_by_id(class_id, &db);
            assert_eq!(
                id_list as *const _, db_list as *const _,
                "Spell lists should be the same reference for {}",
                class_id
            );
        }
    }

    #[test]
    fn test_spellbook_multiple_spell_levels() {
        let mut spellbook = SpellBook::new();

        // Add spells to multiple levels
        {
            let spell_list = spellbook.get_spell_list_mut("cleric");
            spell_list[0].push(0x0101); // Level 1
            spell_list[2].push(0x0301); // Level 3
            spell_list[6].push(0x0701); // Level 7
        }

        // Verify all were added correctly
        let spell_list = spellbook.get_spell_list("cleric");
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[1].len(), 0);
        assert_eq!(spell_list[2].len(), 1);
        assert_eq!(spell_list[3].len(), 0);
        assert_eq!(spell_list[4].len(), 0);
        assert_eq!(spell_list[5].len(), 0);
        assert_eq!(spell_list[6].len(), 1);
    }

    // ===== SpellBook Database Tests =====

    #[test]
    fn test_spellbook_get_spell_list_by_id_cleric() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add a spell to cleric list
        spellbook.cleric_spells[0].push(0x0101);

        // Cleric should get cleric spells
        let spell_list = spellbook.get_spell_list_by_id("cleric", &db);
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_by_id_sorcerer() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add a spell to sorcerer list
        spellbook.sorcerer_spells[0].push(0x0201);

        // Sorcerer should get sorcerer spells
        let spell_list = spellbook.get_spell_list_by_id("sorcerer", &db);
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0201);
    }

    #[test]
    fn test_spellbook_get_spell_list_by_id_paladin() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add a spell to cleric list
        spellbook.cleric_spells[0].push(0x0101);

        // Paladin (hybrid with cleric spells) should get cleric spells
        let spell_list = spellbook.get_spell_list_by_id("paladin", &db);
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_by_id_knight() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spellbook = SpellBook::new();

        // Knight (non-caster) defaults to sorcerer list (which is empty)
        let spell_list = spellbook.get_spell_list_by_id("knight", &db);
        assert!(spell_list[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_by_id_unknown_class() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spellbook = SpellBook::new();

        // Unknown class defaults to sorcerer list
        let spell_list = spellbook.get_spell_list_by_id("unknown", &db);
        assert!(spell_list[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_by_id_cleric() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add a spell through the mutable accessor
        {
            let spell_list = spellbook.get_spell_list_mut_by_id("cleric", &db);
            spell_list[0].push(0x0101);
        }

        // Verify it was added to cleric spells
        assert_eq!(spellbook.cleric_spells[0].len(), 1);
        assert_eq!(spellbook.cleric_spells[0][0], 0x0101);
        // Sorcerer list should be unaffected
        assert!(spellbook.sorcerer_spells[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_by_id_sorcerer() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add a spell through the mutable accessor
        {
            let spell_list = spellbook.get_spell_list_mut_by_id("sorcerer", &db);
            spell_list[0].push(0x0201);
        }

        // Verify it was added to sorcerer spells
        assert_eq!(spellbook.sorcerer_spells[0].len(), 1);
        assert_eq!(spellbook.sorcerer_spells[0][0], 0x0201);
        // Cleric list should be unaffected
        assert!(spellbook.cleric_spells[0].is_empty());
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_by_id_paladin() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Paladin should modify cleric spell list
        {
            let spell_list = spellbook.get_spell_list_mut_by_id("paladin", &db);
            spell_list[0].push(0x0101);
        }

        // Verify it was added to cleric spells
        assert_eq!(spellbook.cleric_spells[0].len(), 1);
        assert_eq!(spellbook.cleric_spells[0][0], 0x0101);
    }

    #[test]
    fn test_spellbook_get_spell_list_mut_by_id_knight() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Knight (non-caster) defaults to sorcerer list
        {
            let spell_list = spellbook.get_spell_list_mut_by_id("knight", &db);
            spell_list[0].push(0x0201);
        }

        // Verify it was added to sorcerer spells (default)
        assert_eq!(spellbook.sorcerer_spells[0].len(), 1);
    }

    #[test]
    fn test_spellbook_db_multiple_spell_levels() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut spellbook = SpellBook::new();

        // Add spells to multiple levels using data-driven accessor
        {
            let spell_list = spellbook.get_spell_list_mut_by_id("cleric", &db);
            spell_list[0].push(0x0101); // Level 1
            spell_list[2].push(0x0301); // Level 3
            spell_list[6].push(0x0701); // Level 7
        }

        // Verify all were added correctly
        let spell_list = spellbook.get_spell_list_by_id("cleric", &db);
        assert_eq!(spell_list[0].len(), 1);
        assert_eq!(spell_list[1].len(), 0);
        assert_eq!(spell_list[2].len(), 1);
        assert_eq!(spell_list[3].len(), 0);
        assert_eq!(spell_list[4].len(), 0);
        assert_eq!(spell_list[5].len(), 0);
        assert_eq!(spell_list[6].len(), 1);
    }
}

#[cfg(test)]
mod ecs_tests {
    use super::*;
    use bevy::prelude::World;

    /// Verify that `Inventory` derives `Component` and can be inserted into
    /// a Bevy `World` and queried back.
    ///
    /// This test confirms that the `#[derive(Component)]` attribute on
    /// `Inventory` is functional and that the struct remains usable as
    /// both a domain type and an ECS component.
    #[test]
    fn test_inventory_component_derive() {
        let mut world = World::new();
        let inventory = Inventory::new();
        let entity = world.spawn(inventory).id();

        let stored = world
            .get::<Inventory>(entity)
            .expect("Inventory component should be present on entity");

        assert!(stored.items.is_empty());
        assert!(!stored.is_full());
        assert!(stored.has_space());
    }

    /// Verify that `Inventory` with items can round-trip through the ECS
    /// component system without data loss.
    #[test]
    fn test_inventory_component_with_items() {
        let mut world = World::new();
        let mut inventory = Inventory::new();
        inventory.add_item(42, 3).expect("add_item should succeed");
        inventory.add_item(7, 0).expect("add_item should succeed");

        let entity = world.spawn(inventory).id();

        let stored = world
            .get::<Inventory>(entity)
            .expect("Inventory component should be present");

        assert_eq!(stored.items.len(), 2);
        assert_eq!(stored.items[0].item_id, 42);
        assert_eq!(stored.items[0].charges, 3);
        assert_eq!(stored.items[1].item_id, 7);
        assert_eq!(stored.items[1].charges, 0);
    }

    /// Verify that `InventorySlot` derives `Component` and can be inserted
    /// into a Bevy `World` and queried back with matching field values.
    ///
    /// `InventorySlot` is `Copy`, so it can be cheaply duplicated; this test
    /// confirms the component derive does not interfere with that.
    #[test]
    fn test_inventory_slot_component_derive() {
        let mut world = World::new();
        let slot = InventorySlot {
            item_id: 1,
            charges: 3,
        };
        let entity = world.spawn(slot).id();

        let stored = world
            .get::<InventorySlot>(entity)
            .expect("InventorySlot component should be present on entity");

        assert_eq!(stored.item_id, 1);
        assert_eq!(stored.charges, 3);
    }

    /// Verify that `InventorySlot` with zero charges can be stored and
    /// retrieved as an ECS component.
    #[test]
    fn test_inventory_slot_component_zero_charges() {
        let mut world = World::new();
        let slot = InventorySlot {
            item_id: 99,
            charges: 0,
        };
        let entity = world.spawn(slot).id();

        let stored = world
            .get::<InventorySlot>(entity)
            .expect("InventorySlot component should be present");

        assert_eq!(stored.item_id, 99);
        assert_eq!(stored.charges, 0);
    }

    /// Verify that `Equipment` derives `Component` and can be inserted into
    /// a Bevy `World` and queried back.
    ///
    /// This test confirms that the `#[derive(Component)]` attribute on
    /// `Equipment` is functional and that default (empty) equipment is
    /// preserved when stored as a component.
    #[test]
    fn test_equipment_component_derive() {
        let mut world = World::new();
        let equipment = Equipment::new();
        let entity = world.spawn(equipment).id();

        let stored = world
            .get::<Equipment>(entity)
            .expect("Equipment component should be present on entity");

        assert_eq!(stored.equipped_count(), 0);
        assert!(stored.weapon.is_none());
        assert!(stored.armor.is_none());
        assert!(stored.shield.is_none());
        assert!(stored.helmet.is_none());
        assert!(stored.boots.is_none());
        assert!(stored.accessory1.is_none());
        assert!(stored.accessory2.is_none());
    }

    /// Verify that `Equipment` with populated slots round-trips through the
    /// ECS component system without data loss.
    #[test]
    fn test_equipment_component_with_slots() {
        let mut world = World::new();
        let mut equipment = Equipment::new();
        equipment.weapon = Some(10);
        equipment.armor = Some(20);
        equipment.helmet = Some(30);

        let entity = world.spawn(equipment).id();

        let stored = world
            .get::<Equipment>(entity)
            .expect("Equipment component should be present");

        assert_eq!(stored.equipped_count(), 3);
        assert_eq!(stored.weapon, Some(10));
        assert_eq!(stored.armor, Some(20));
        assert_eq!(stored.helmet, Some(30));
        assert!(stored.shield.is_none());
        assert!(stored.boots.is_none());
    }
}
