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

use crate::domain::types::{ItemId, SpellId, TownId};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Character race
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Gnome,
    HalfOrc,
}

/// Character class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Knight,
    Paladin,
    Archer,
    Cleric,
    Sorcerer,
    Robber,
}

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventorySlot>,
}

impl Inventory {
    /// Maximum number of items in backpack
    pub const MAX_ITEMS: usize = 6;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

// ===== SpellBook =====

/// Character's known spells organized by school and level
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Returns the appropriate spell list for the character's class
    pub fn get_spell_list(&self, class: Class) -> &[Vec<SpellId>; 7] {
        match class {
            Class::Cleric | Class::Paladin => &self.cleric_spells,
            Class::Sorcerer | Class::Archer => &self.sorcerer_spells,
            _ => &self.sorcerer_spells, // Default to empty
        }
    }

    /// Returns the mutable spell list for the character's class
    pub fn get_spell_list_mut(&mut self, class: Class) -> &mut [Vec<SpellId>; 7] {
        match class {
            Class::Cleric | Class::Paladin => &mut self.cleric_spells,
            Class::Sorcerer | Class::Archer => &mut self.sorcerer_spells,
            _ => &mut self.sorcerer_spells,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ===== Character =====

/// Represents a single character (party member or roster character)
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
///
/// let hero = Character::new(
///     "Sir Lancelot".to_string(),
///     Race::Human,
///     Class::Knight,
///     Sex::Male,
///     Alignment::Good,
/// );
/// assert_eq!(hero.name, "Sir Lancelot");
/// assert_eq!(hero.level, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub race: Race,
    pub class: Class,
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
    /// Damage resistances
    pub resistances: Resistances,
    /// Per-character quest/event tracking
    pub quest_flags: QuestFlags,
    /// Portrait/avatar ID
    pub portrait_id: u8,
    /// Special quest attribute
    pub worthiness: u8,
    /// Individual gold (0-max)
    pub gold: u32,
    /// Individual gems (0-max)
    pub gems: u32,
    /// Individual food units (max 40, starts at 10)
    pub food: u8,
}

impl Character {
    /// Creates a new character with default starting values
    pub fn new(name: String, race: Race, class: Class, sex: Sex, alignment: Alignment) -> Self {
        Self {
            name,
            race,
            class,
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
            resistances: Resistances::new(),
            quest_flags: QuestFlags::new(),
            portrait_id: 0,
            worthiness: 0,
            gold: 0,
            gems: 0,
            food: 10,
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
/// use antares::domain::character::{Party, Character, Race, Class, Sex, Alignment};
///
/// let mut party = Party::new();
/// let hero = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
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
    /// Party food supply (deprecated - use character food)
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
    /// Where inactive characters are stored
    pub character_locations: Vec<Option<TownId>>,
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
        location: Option<TownId>,
    ) -> Result<(), CharacterError> {
        if self.characters.len() >= Self::MAX_CHARACTERS {
            return Err(CharacterError::RosterFull(Self::MAX_CHARACTERS));
        }
        self.characters.push(character);
        self.character_locations.push(location);
        Ok(())
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

    #[test]
    fn test_attribute_pair_new() {
        let attr = AttributePair::new(15);
        assert_eq!(attr.base, 15);
        assert_eq!(attr.current, 15);
    }

    #[test]
    fn test_attribute_pair_modify() {
        let mut attr = AttributePair::new(10);
        attr.modify(5);
        assert_eq!(attr.current, 15);
        attr.modify(-3);
        assert_eq!(attr.current, 12);
    }

    #[test]
    fn test_attribute_pair_reset() {
        let mut attr = AttributePair::new(10);
        attr.modify(5);
        assert_eq!(attr.current, 15);
        attr.reset();
        assert_eq!(attr.current, 10);
    }

    #[test]
    fn test_inventory_max_items() {
        let mut inventory = Inventory::new();
        assert!(inventory.has_space());
        assert!(!inventory.is_full());

        // Fill to max
        for i in 0..Inventory::MAX_ITEMS {
            assert!(inventory.add_item(i as ItemId, 1).is_ok());
        }

        assert!(inventory.is_full());
        assert!(!inventory.has_space());
        assert!(inventory.add_item(99, 1).is_err());
    }

    #[test]
    fn test_equipment_count() {
        let mut equipment = Equipment::new();
        assert_eq!(equipment.equipped_count(), 0);

        equipment.weapon = Some(1);
        assert_eq!(equipment.equipped_count(), 1);

        equipment.armor = Some(2);
        equipment.shield = Some(3);
        assert_eq!(equipment.equipped_count(), 3);
    }

    #[test]
    fn test_condition_flags() {
        let mut condition = Condition::new();
        assert!(condition.is_fine());
        assert!(!condition.is_bad());

        condition.add(Condition::POISONED);
        assert!(condition.has(Condition::POISONED));
        assert!(!condition.is_fine());
    }

    #[test]
    fn test_party_max_members() {
        let mut party = Party::new();
        assert!(party.is_empty());

        // Add characters up to max
        for i in 0..Party::MAX_MEMBERS {
            let character = Character::new(
                format!("Hero {}", i),
                Race::Human,
                Class::Knight,
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
            Race::Elf,
            Class::Archer,
            Sex::Female,
            Alignment::Good,
        );
        assert!(party.add_member(extra).is_err());
    }

    #[test]
    fn test_character_creation() {
        let hero = Character::new(
            "Test Hero".to_string(),
            Race::Human,
            Class::Knight,
            Sex::Male,
            Alignment::Good,
        );

        assert_eq!(hero.name, "Test Hero");
        assert_eq!(hero.level, 1);
        assert_eq!(hero.experience, 0);
        assert!(hero.is_alive());
        assert!(hero.can_act());
    }
}
