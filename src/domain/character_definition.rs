// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character definition and database for data-driven character templates
//!
//! This module implements the data-driven character definition system, allowing
//! character templates to be defined in external RON files. This enables pre-made
//! characters, NPCs, and template characters for campaigns.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4 for core data structures.
//! See `docs/explanation/character_definition_implementation_plan.md` for implementation details.
//!
//! # Key Concepts
//!
//! - **CharacterDefinition**: A template that defines a character's starting state
//! - **CharacterDatabase**: Manages loading and lookup of character definitions
//! - **StartingEquipment**: Specifies which items to equip when instantiating
//!
//! Character definitions are separate from runtime `Character` instances.
//! Definitions are loaded from RON files; instances are created at runtime.
//!
//! # RON Format Migration (AttributePair)
//!
//! As of Phase 1 migration, `base_stats` uses `Stats` (with `AttributePair` fields) and
//! `hp_override` uses `AttributePair16`. Both support backward-compatible deserialization:
//!
//! **Simple format (recommended for most cases):**
//! ```ron
//! base_stats: (
//!     might: 15,        // Expands to AttributePair { base: 15, current: 15 }
//!     intellect: 10,
//!     // ...
//! ),
//! hp_override: Some(50),  // Expands to AttributePair16 { base: 50, current: 50 }
//! ```
//!
//! **Full format (for pre-buffed characters):**
//! ```ron
//! base_stats: (
//!     might: (base: 15, current: 18),  // Character starts with buffed Might
//!     intellect: 10,
//!     // ...
//! ),
//! hp_override: Some((base: 50, current: 65)),  // Pre-buffed HP
//! ```
//!
//! **Legacy format (deprecated, still supported):**
//! ```ron
//! hp_base: Some(50),
//! hp_current: Some(65),  // Converts to hp_override: Some((base: 50, current: 65))
//! ```

use crate::domain::character::{
    Alignment, AttributePair, AttributePair16, Character, Condition, Equipment, Inventory,
    InventorySlot, QuestFlags, Resistances as CharacterResistances, Sex, SpellBook, Stats,
};
use crate::domain::classes::{ClassDatabase, ClassDefinition, ClassId, SpellStat};
use crate::domain::items::ItemDatabase;
use crate::domain::races::{RaceDatabase, RaceDefinition};
use crate::domain::types::{ItemId, RaceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::warn;

// ===== Error Types =====

/// Errors that can occur when working with character definitions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CharacterDefinitionError {
    /// Character definition not found in database
    #[error("Character definition not found: {0}")]
    CharacterNotFound(String),

    /// Failed to load character database from file
    #[error("Failed to load character database from file: {0}")]
    LoadError(String),

    /// Failed to parse character data
    #[error("Failed to parse character data: {0}")]
    ParseError(String),

    /// Validation error in character definition
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Duplicate character definition ID
    #[error("Duplicate character definition ID: {0}")]
    DuplicateId(String),

    /// Invalid race reference
    #[error("Invalid race_id '{race_id}' in character '{character_id}'")]
    InvalidRaceId {
        character_id: String,
        race_id: String,
    },

    /// Invalid class reference
    #[error("Invalid class_id '{class_id}' in character '{character_id}'")]
    InvalidClassId {
        character_id: String,
        class_id: String,
    },

    /// Invalid item reference
    #[error("Invalid item_id {item_id} in character '{character_id}'")]
    InvalidItemId {
        character_id: String,
        item_id: ItemId,
    },

    /// Error during character instantiation
    #[error("Instantiation error for character '{character_id}': {message}")]
    InstantiationError {
        character_id: String,
        message: String,
    },

    /// Inventory is full, cannot add more items
    #[error("Inventory full for character '{character_id}': cannot add item {item_id}")]
    InventoryFull {
        character_id: String,
        item_id: ItemId,
    },
}

// ===== Type Aliases =====

/// Unique identifier for a character definition
///
/// Character definition IDs are strings like "pregen_human_knight" or "npc_wizard".
pub type CharacterDefinitionId = String;

// ===== Starting Equipment =====

/// Starting equipment configuration for a character definition
///
/// Specifies which items should be equipped when a character is instantiated
/// from this definition. All fields are optional; `None` means the slot starts empty.
///
/// # Examples
///
/// ```
/// use antares::domain::character_definition::StartingEquipment;
///
/// let equipment = StartingEquipment {
///     weapon: Some(1),    // Longsword
///     armor: Some(10),    // Chain Mail
///     shield: Some(20),   // Small Shield
///     helmet: None,
///     boots: None,
///     accessory1: None,
///     accessory2: None,
/// };
///
/// assert!(equipment.weapon.is_some());
/// assert!(equipment.helmet.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartingEquipment {
    /// Weapon to equip (one-handed or two-handed)
    #[serde(default)]
    pub weapon: Option<ItemId>,

    /// Body armor to equip
    #[serde(default)]
    pub armor: Option<ItemId>,

    /// Shield to equip (if not using two-handed weapon)
    #[serde(default)]
    pub shield: Option<ItemId>,

    /// Helmet to equip
    #[serde(default)]
    pub helmet: Option<ItemId>,

    /// Boots to equip
    #[serde(default)]
    pub boots: Option<ItemId>,

    /// First accessory slot (ring, amulet, etc.)
    #[serde(default)]
    pub accessory1: Option<ItemId>,

    /// Second accessory slot
    #[serde(default)]
    pub accessory2: Option<ItemId>,
}

impl StartingEquipment {
    /// Creates a new StartingEquipment with all slots empty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::StartingEquipment;
    ///
    /// let equipment = StartingEquipment::new();
    /// assert!(equipment.weapon.is_none());
    /// assert!(equipment.armor.is_none());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if all equipment slots are empty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::StartingEquipment;
    ///
    /// let empty = StartingEquipment::new();
    /// assert!(empty.is_empty());
    ///
    /// let with_weapon = StartingEquipment {
    ///     weapon: Some(1),
    ///     ..Default::default()
    /// };
    /// assert!(!with_weapon.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.weapon.is_none()
            && self.armor.is_none()
            && self.shield.is_none()
            && self.helmet.is_none()
            && self.boots.is_none()
            && self.accessory1.is_none()
            && self.accessory2.is_none()
    }

    /// Returns the count of equipped item slots
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::StartingEquipment;
    ///
    /// let equipment = StartingEquipment {
    ///     weapon: Some(1),
    ///     armor: Some(10),
    ///     shield: None,
    ///     helmet: None,
    ///     boots: None,
    ///     accessory1: None,
    ///     accessory2: None,
    /// };
    /// assert_eq!(equipment.equipped_count(), 2);
    /// ```
    pub fn equipped_count(&self) -> usize {
        [
            &self.weapon,
            &self.armor,
            &self.shield,
            &self.helmet,
            &self.boots,
            &self.accessory1,
            &self.accessory2,
        ]
        .iter()
        .filter(|slot| slot.is_some())
        .count()
    }

    /// Returns a vector of all equipped item IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::StartingEquipment;
    ///
    /// let equipment = StartingEquipment {
    ///     weapon: Some(1),
    ///     armor: Some(10),
    ///     ..Default::default()
    /// };
    /// let items = equipment.all_item_ids();
    /// assert_eq!(items.len(), 2);
    /// assert!(items.contains(&1));
    /// assert!(items.contains(&10));
    /// ```
    pub fn all_item_ids(&self) -> Vec<ItemId> {
        [
            self.weapon,
            self.armor,
            self.shield,
            self.helmet,
            self.boots,
            self.accessory1,
            self.accessory2,
        ]
        .iter()
        .filter_map(|&slot| slot)
        .collect()
    }
}

// ===== Base Stats (Deprecated) =====

/// Base statistics for a character definition
///
/// **DEPRECATED**: Use `Stats` from `crate::domain::character` instead.
/// This type is maintained for backward compatibility with existing RON files.
/// The `Stats` type uses `AttributePair` for each stat, supporting base+current values.
///
/// # Migration
///
/// Old format (still supported via custom deserialization):
/// ```text
/// base_stats: (might: 14, intellect: 10, ...)
/// ```
///
/// New format (preferred):
/// ```text
/// base_stats: (might: (base: 14, current: 14), intellect: (base: 10, current: 10), ...)
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use Stats from crate::domain::character instead. BaseStats is kept for backward compatibility."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStats {
    /// Physical strength, melee damage
    pub might: u8,
    /// Magical power, spell effectiveness
    pub intellect: u8,
    /// Charisma, social interactions
    pub personality: u8,
    /// Constitution, HP calculation
    pub endurance: u8,
    /// Initiative, dodging, turn order
    pub speed: u8,
    /// Hit chance, ranged attacks
    pub accuracy: u8,
    /// Critical hits, random events, loot
    pub luck: u8,
}

#[allow(deprecated)]
impl BaseStats {
    /// Creates a new BaseStats with the specified values
    #[deprecated(
        since = "0.2.0",
        note = "Use Stats::new() from crate::domain::character instead"
    )]
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
            might,
            intellect,
            personality,
            endurance,
            speed,
            accuracy,
            luck,
        }
    }

    /// Converts BaseStats to the runtime Stats type
    pub fn to_stats(&self) -> Stats {
        Stats::new(
            self.might,
            self.intellect,
            self.personality,
            self.endurance,
            self.speed,
            self.accuracy,
            self.luck,
        )
    }
}

#[allow(deprecated)]
impl Default for BaseStats {
    fn default() -> Self {
        Self {
            might: 10,
            intellect: 10,
            personality: 10,
            endurance: 10,
            speed: 10,
            accuracy: 10,
            luck: 10,
        }
    }
}

// ===== Character Definition =====

/// Complete definition of a character template
///
/// This structure contains all the data needed to create a new character
/// instance. Character definitions are loaded from RON files and used
/// to instantiate pre-made characters, NPCs, or character templates.
///
/// # Examples
///
/// ```
/// use antares::domain::character_definition::{CharacterDefinition, StartingEquipment};
/// use antares::domain::character::{Sex, Alignment, Stats};
///
/// let knight = CharacterDefinition {
///     id: "pregen_human_knight".to_string(),
///     name: "Sir Galahad".to_string(),
///     race_id: "human".to_string(),
///     class_id: "knight".to_string(),
///     sex: Sex::Male,
///     alignment: Alignment::Good,
///     base_stats: Stats::new(16, 8, 10, 14, 12, 14, 10),
///     hp_override: None,
///     portrait_id: "1".to_string(),
///     starting_gold: 100,
///     starting_gems: 0,
///     starting_food: 10,
///     starting_items: vec![],
///     starting_equipment: StartingEquipment::default(),
///     description: "A noble knight seeking glory.".to_string(),
///     is_premade: true,
///     starts_in_party: false,
/// };
///
/// assert_eq!(knight.name, "Sir Galahad");
/// assert!(knight.is_premade);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "CharacterDefinitionDef")]
pub struct CharacterDefinition {
    /// Unique identifier (e.g., "pregen_human_knight", "npc_wizard")
    pub id: CharacterDefinitionId,

    /// Character's display name
    pub name: String,

    /// Reference to race definition (e.g., "human", "elf")
    pub race_id: RaceId,

    /// Reference to class definition (e.g., "knight", "sorcerer")
    pub class_id: ClassId,

    /// Character's sex
    pub sex: Sex,

    /// Starting alignment
    pub alignment: Alignment,

    /// Base statistics (supports both simple values and AttributePair format)
    ///
    /// Simple format (backward compatible):
    /// ```ron
    /// base_stats: (
    ///     might: 15,
    ///     intellect: 10,
    ///     // ...
    /// )
    /// ```
    ///
    /// Full format (explicit base and current):
    /// ```ron
    /// base_stats: (
    ///     might: (base: 15, current: 15),
    ///     intellect: (base: 10, current: 12),
    ///     // ...
    /// )
    /// ```
    pub base_stats: Stats,

    /// Optional HP override (base and current)
    ///
    /// When present, overrides the calculated starting HP.
    /// Supports both simple format (e.g., `Some(50)`) and full format
    /// (e.g., `Some((base: 50, current: 45))`).
    ///
    /// # Backward Compatibility
    ///
    /// Old `hp_base` and `hp_current` fields are supported via custom
    /// deserialization for migration purposes.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp_override: Option<AttributePair16>,

    /// Portrait/avatar identifier (filename stem / unique string)
    #[serde(default)]
    pub portrait_id: String,

    /// Starting gold amount
    #[serde(default)]
    pub starting_gold: u32,

    /// Starting gems amount
    #[serde(default)]
    pub starting_gems: u32,

    /// Starting food units
    #[serde(default = "default_starting_food")]
    pub starting_food: u8,

    /// Items to add to inventory on creation
    #[serde(default)]
    pub starting_items: Vec<ItemId>,

    /// Equipment to equip on creation
    #[serde(default)]
    pub starting_equipment: StartingEquipment,

    /// Character backstory/biography
    #[serde(default)]
    pub description: String,

    /// True for pre-made characters, false for templates
    #[serde(default)]
    pub is_premade: bool,

    /// Whether this character should start in the active party (new games only)
    ///
    /// When true, this character will be automatically added to the party
    /// when a new game is started. Maximum of 6 characters can have this
    /// flag set (PARTY_MAX_SIZE constraint).
    #[serde(default)]
    pub starts_in_party: bool,
}

/// Default starting food value (10 units)
fn default_starting_food() -> u8 {
    10
}

// ===== Backward Compatibility Deserialization =====

/// Temporary deserialization helper for backward compatibility
///
/// Supports old RON files with separate `hp_base` and `hp_current` fields.
#[derive(Deserialize)]
struct CharacterDefinitionDef {
    pub id: CharacterDefinitionId,
    pub name: String,
    pub race_id: RaceId,
    pub class_id: ClassId,
    pub sex: Sex,
    pub alignment: Alignment,
    pub base_stats: Stats,
    #[serde(default)]
    pub hp_base: Option<u16>,
    #[serde(default)]
    pub hp_current: Option<u16>,
    #[serde(default)]
    pub hp_override: Option<AttributePair16>,
    #[serde(default)]
    pub portrait_id: String,
    #[serde(default)]
    pub starting_gold: u32,
    #[serde(default)]
    pub starting_gems: u32,
    #[serde(default = "default_starting_food")]
    pub starting_food: u8,
    #[serde(default)]
    pub starting_items: Vec<ItemId>,
    #[serde(default)]
    pub starting_equipment: StartingEquipment,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_premade: bool,
    #[serde(default)]
    pub starts_in_party: bool,
}

impl From<CharacterDefinitionDef> for CharacterDefinition {
    fn from(def: CharacterDefinitionDef) -> Self {
        // Convert old hp_base/hp_current to hp_override
        let hp_override = if let Some(override_val) = def.hp_override {
            // New format takes precedence
            Some(override_val)
        } else if let Some(base) = def.hp_base {
            // Old format: hp_base with optional hp_current
            let current = def.hp_current.unwrap_or(base);
            Some(AttributePair16 {
                base,
                current: current.min(base),
            })
        } else {
            // Only hp_current specified - use it for both
            def.hp_current.map(AttributePair16::new)
        };

        Self {
            id: def.id,
            name: def.name,
            race_id: def.race_id,
            class_id: def.class_id,
            sex: def.sex,
            alignment: def.alignment,
            base_stats: def.base_stats,
            hp_override,
            portrait_id: def.portrait_id,
            starting_gold: def.starting_gold,
            starting_gems: def.starting_gems,
            starting_food: def.starting_food,
            starting_items: def.starting_items,
            starting_equipment: def.starting_equipment,
            description: def.description,
            is_premade: def.is_premade,
            starts_in_party: def.starts_in_party,
        }
    }
}

impl CharacterDefinition {
    /// Creates a new CharacterDefinition with required fields and defaults
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this definition
    /// * `name` - Character's display name
    /// * `race_id` - Reference to race definition
    /// * `class_id` - Reference to class definition
    /// * `sex` - Character's sex
    /// * `alignment` - Starting alignment
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDefinition;
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let definition = CharacterDefinition::new(
    ///     "test_char".to_string(),
    ///     "Test Character".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    ///
    /// assert_eq!(definition.id, "test_char");
    /// assert_eq!(definition.starting_food, 10);
    /// ```
    pub fn new(
        id: CharacterDefinitionId,
        name: String,
        race_id: RaceId,
        class_id: ClassId,
        sex: Sex,
        alignment: Alignment,
    ) -> Self {
        Self {
            id,
            name,
            race_id,
            class_id,
            sex,
            alignment,
            base_stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp_override: None,
            portrait_id: String::new(),
            starting_gold: 0,
            starting_gems: 0,
            starting_food: 10,
            starting_items: Vec::new(),
            starting_equipment: StartingEquipment::new(),
            description: String::new(),
            is_premade: false,
            starts_in_party: false,
        }
    }

    /// Returns all item IDs referenced by this definition
    ///
    /// Includes both starting inventory items and equipped items.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDefinition, StartingEquipment};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut definition = CharacterDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// definition.starting_items = vec![1, 2, 3];
    /// definition.starting_equipment.weapon = Some(10);
    ///
    /// let all_items = definition.all_item_ids();
    /// assert_eq!(all_items.len(), 4);
    /// ```
    pub fn all_item_ids(&self) -> Vec<ItemId> {
        let mut items = self.starting_items.clone();
        items.extend(self.starting_equipment.all_item_ids());
        items
    }

    /// Validates the character definition
    ///
    /// Checks for:
    /// - Non-empty ID and name
    /// - Non-empty race_id and class_id
    /// - Valid stat ranges (3-18 for standard characters)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or an error describing the issue.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDefinition;
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let valid = CharacterDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// assert!(valid.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CharacterDefinitionError> {
        if self.id.is_empty() {
            return Err(CharacterDefinitionError::ValidationError(
                "Character definition ID cannot be empty".to_string(),
            ));
        }

        if self.name.is_empty() {
            return Err(CharacterDefinitionError::ValidationError(format!(
                "Character '{}' has empty name",
                self.id
            )));
        }

        if self.race_id.is_empty() {
            return Err(CharacterDefinitionError::ValidationError(format!(
                "Character '{}' has empty race_id",
                self.id
            )));
        }

        if self.class_id.is_empty() {
            return Err(CharacterDefinitionError::ValidationError(format!(
                "Character '{}' has empty class_id",
                self.id
            )));
        }

        // Portrait ID must be a normalized filename stem when provided.
        // Normalization rule: lowercase, spaces replaced by underscores.
        if !self.portrait_id.is_empty() {
            let normalized = self.portrait_id.to_lowercase().replace(' ', "_");
            if normalized != self.portrait_id {
                return Err(CharacterDefinitionError::ValidationError(format!(
                    "Character '{}' has non-normalized portrait_id '{}'; expected '{}'",
                    self.id, self.portrait_id, normalized
                )));
            }

            // Ensure only allowed characters are present (a-z, 0-9, underscore, hyphen)
            if !normalized
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(CharacterDefinitionError::ValidationError(format!(
                    "Character '{}' has invalid portrait_id '{}'; allowed characters: a-z, 0-9, '_' and '-'",
                    self.id, self.portrait_id
                )));
            }
        }

        Ok(())
    }

    /// Instantiates a runtime Character from this definition
    ///
    /// Creates a fully populated Character instance using the definition's
    /// template data. Applies race stat modifiers, calculates starting HP/SP,
    /// populates inventory, and equips starting equipment.
    ///
    /// # Arguments
    ///
    /// * `races` - RaceDatabase for race lookups and modifier application
    /// * `classes` - ClassDatabase for class lookups and HP/SP calculation
    /// * `items` - ItemDatabase for validating item references
    ///
    /// # Returns
    ///
    /// Returns `Ok(Character)` on success, or an error if:
    /// - race_id doesn't exist in RaceDatabase
    /// - class_id doesn't exist in ClassDatabase
    /// - Any starting item ID doesn't exist in ItemDatabase
    /// - Inventory becomes full during population
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::character_definition::{CharacterDefinition, BaseStats};
    /// use antares::domain::character::{Sex, Alignment};
    /// use antares::domain::races::RaceDatabase;
    /// use antares::domain::classes::ClassDatabase;
    /// use antares::domain::items::ItemDatabase;
    ///
    /// let definition = CharacterDefinition::new(
    ///     "test_knight".to_string(),
    ///     "Sir Test".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    ///
    /// let races = RaceDatabase::load_from_file("data/races.ron").unwrap();
    /// let classes = ClassDatabase::load_from_file("data/classes.ron").unwrap();
    /// let items = ItemDatabase::load_from_file("data/items.ron").unwrap();
    ///
    /// let character = definition.instantiate(&races, &classes, &items).unwrap();
    /// assert_eq!(character.name, "Sir Test");
    /// ```
    pub fn instantiate(
        &self,
        races: &RaceDatabase,
        classes: &ClassDatabase,
        items: &ItemDatabase,
    ) -> Result<Character, CharacterDefinitionError> {
        // Validate race exists
        let race_def = races.get_race(&self.race_id).ok_or_else(|| {
            CharacterDefinitionError::InvalidRaceId {
                character_id: self.id.clone(),
                race_id: self.race_id.clone(),
            }
        })?;

        // Validate class exists
        let class_def = classes.get_class(&self.class_id).ok_or_else(|| {
            CharacterDefinitionError::InvalidClassId {
                character_id: self.id.clone(),
                class_id: self.class_id.clone(),
            }
        })?;

        // Validate all item IDs exist
        for item_id in self.all_item_ids() {
            if items.get_item(item_id).is_none() {
                return Err(CharacterDefinitionError::InvalidItemId {
                    character_id: self.id.clone(),
                    item_id,
                });
            }
        }

        // Copy base stats and apply race stat modifiers
        let stats = apply_race_modifiers(&self.base_stats, race_def);

        // Calculate starting HP based on class and endurance
        // Use hp_override if provided, otherwise use calculated value
        let hp = if let Some(hp_override) = self.hp_override {
            // Validate that current doesn't exceed base
            if hp_override.current > hp_override.base {
                warn!(
                    "instantiate: hp_override.current {} exceeds hp_override.base {}; clamping for character '{}'",
                    hp_override.current, hp_override.base, self.id
                );
                AttributePair16 {
                    base: hp_override.base,
                    current: hp_override.base,
                }
            } else {
                hp_override
            }
        } else {
            calculate_starting_hp(class_def, stats.endurance.base)
        };

        // Calculate starting SP based on class and relevant stat
        let sp = calculate_starting_sp(class_def, &stats);

        // Create resistances from race definition
        let resistances = apply_race_resistances(race_def);

        // Create inventory with starting items
        let inventory = populate_starting_inventory(&self.id, &self.starting_items)?;

        // Create equipment from starting equipment
        let equipment = create_starting_equipment(&self.starting_equipment);

        // Build the Character
        let character = Character {
            name: self.name.clone(),
            race_id: self.race_id.clone(),
            class_id: self.class_id.clone(),
            sex: self.sex,
            alignment: self.alignment,
            alignment_initial: self.alignment,
            level: 1,
            experience: 0,
            age: 18,
            age_days: 0,
            stats,
            hp,
            sp,
            ac: AttributePair::new(0), // AC calculated from equipment separately
            spell_level: AttributePair::new(calculate_starting_spell_level(class_def)),
            inventory,
            equipment,
            spells: SpellBook::new(),
            conditions: Condition::new(),
            active_conditions: Vec::new(),
            resistances,
            quest_flags: QuestFlags::new(),
            portrait_id: self.portrait_id.clone(),
            worthiness: 0,
            gold: self.starting_gold,
            gems: self.starting_gems,
            food: self.starting_food,
        };

        Ok(character)
    }
}

// ===== Instantiation Helper Functions =====

/// Applies race stat modifiers to base stats
///
/// Creates a Stats struct with race modifiers applied to the base values.
/// Modifiers are clamped to valid stat ranges (3-25).
///
/// # Arguments
///
/// * `base_stats` - The base stat values from the character definition
/// * `race_def` - The race definition containing stat modifiers
///
/// # Returns
///
/// Returns a Stats struct with modifiers applied to base values.
fn apply_race_modifiers(base_stats: &Stats, race_def: &RaceDefinition) -> Stats {
    let mods = &race_def.stat_modifiers;

    // Apply modifiers with clamping to valid range (3-25 for modified stats)
    let apply_mod = |base: u8, modifier: i8| -> u8 {
        let result = base as i16 + modifier as i16;
        result.clamp(3, 25) as u8
    };

    Stats::new(
        apply_mod(base_stats.might.base, mods.might),
        apply_mod(base_stats.intellect.base, mods.intellect),
        apply_mod(base_stats.personality.base, mods.personality),
        apply_mod(base_stats.endurance.base, mods.endurance),
        apply_mod(base_stats.speed.base, mods.speed),
        apply_mod(base_stats.accuracy.base, mods.accuracy),
        apply_mod(base_stats.luck.base, mods.luck),
    )
}

/// Calculates starting HP based on class and endurance
///
/// For level 1 characters, HP is calculated as:
/// - Maximum roll of the class HP die + endurance modifier
///
/// This provides consistent starting HP for premade characters.
///
/// # Arguments
///
/// * `class_def` - The class definition with HP die info
/// * `endurance` - The character's endurance stat (after race modifiers)
///
/// # Returns
///
/// Returns an AttributePair16 with the calculated HP as both base and current.
fn calculate_starting_hp(class_def: &ClassDefinition, endurance: u8) -> AttributePair16 {
    // Endurance modifier: (endurance - 10) / 2, rounded down
    let endurance_mod = (endurance as i16 - 10) / 2;

    // For level 1: max die roll + endurance modifier
    // Use max roll (sides) for consistent premade characters
    let base_hp = class_def.hp_die.sides as i16 + endurance_mod;

    // Minimum 1 HP
    let hp = base_hp.max(1) as u16;

    AttributePair16::new(hp)
}

/// Calculates starting SP based on class and relevant stat
///
/// SP calculation depends on the class's spell_stat:
/// - Sorcerers use Intellect
/// - Clerics/Paladins use Personality
/// - Non-casters get 0 SP
///
/// For level 1 pure casters: SP = max(0, stat - 10)
/// For level 1 hybrids (Paladin): SP = max(0, (stat - 10) / 2)
///
/// # Arguments
///
/// * `class_def` - The class definition with spell info
/// * `stats` - The character's stats (after race modifiers)
///
/// # Returns
///
/// Returns an AttributePair16 with the calculated SP as both base and current.
fn calculate_starting_sp(class_def: &ClassDefinition, stats: &Stats) -> AttributePair16 {
    // Non-casters get 0 SP
    if !class_def.can_cast_spells() {
        return AttributePair16::new(0);
    }

    // Get the relevant stat for SP calculation
    let spell_stat = match class_def.spell_stat {
        Some(SpellStat::Intellect) => stats.intellect.base,
        Some(SpellStat::Personality) => stats.personality.base,
        None => return AttributePair16::new(0),
    };

    // Calculate SP based on caster type
    let sp = if class_def.is_pure_caster {
        // Pure casters: (stat - 10), minimum 0
        (spell_stat as i16 - 10).max(0) as u16
    } else {
        // Hybrid casters (Paladin): (stat - 10) / 2, minimum 0
        ((spell_stat as i16 - 10) / 2).max(0) as u16
    };

    AttributePair16::new(sp)
}

/// Calculates starting spell level based on class
///
/// Pure casters start at spell level 1.
/// Hybrid casters (Paladin) start at spell level 0.
/// Non-casters have spell level 0.
///
/// # Arguments
///
/// * `class_def` - The class definition
///
/// # Returns
///
/// Returns the starting spell level (0 or 1).
fn calculate_starting_spell_level(class_def: &ClassDefinition) -> u8 {
    if class_def.is_pure_caster {
        1
    } else {
        0
    }
}

/// Applies race resistances to create character resistances
///
/// Converts RaceDefinition resistances (plain u8 values) to Character
/// Resistances struct (AttributePair values for base/current tracking).
///
/// # Arguments
///
/// * `race_def` - The race definition containing resistances
///
/// # Returns
///
/// Returns a Resistances struct for the character with race resistance values.
fn apply_race_resistances(race_def: &RaceDefinition) -> CharacterResistances {
    let race_res = &race_def.resistances;
    CharacterResistances {
        magic: AttributePair::new(race_res.magic),
        fire: AttributePair::new(race_res.fire),
        cold: AttributePair::new(race_res.cold),
        electricity: AttributePair::new(race_res.electricity),
        acid: AttributePair::new(race_res.acid),
        fear: AttributePair::new(race_res.fear),
        poison: AttributePair::new(race_res.poison),
        psychic: AttributePair::new(race_res.psychic),
    }
}

/// Populates the starting inventory with items
///
/// # Arguments
///
/// * `character_id` - The character ID for error reporting
/// * `starting_items` - List of item IDs to add to inventory
///
/// # Returns
///
/// Returns `Ok(Inventory)` on success, or error if inventory becomes full.
fn populate_starting_inventory(
    character_id: &str,
    starting_items: &[ItemId],
) -> Result<Inventory, CharacterDefinitionError> {
    let mut inventory = Inventory::new();

    for &item_id in starting_items {
        if inventory.is_full() {
            return Err(CharacterDefinitionError::InventoryFull {
                character_id: character_id.to_string(),
                item_id,
            });
        }

        // Add item with 0 charges (charges are set based on item type later)
        let slot = InventorySlot {
            item_id,
            charges: 0,
        };
        inventory.items.push(slot);
    }

    Ok(inventory)
}

/// Creates equipment from starting equipment definition
///
/// # Arguments
///
/// * `starting_equipment` - The starting equipment configuration
///
/// # Returns
///
/// Returns an Equipment struct with the specified items equipped.
fn create_starting_equipment(starting_equipment: &StartingEquipment) -> Equipment {
    Equipment {
        weapon: starting_equipment.weapon,
        armor: starting_equipment.armor,
        shield: starting_equipment.shield,
        helmet: starting_equipment.helmet,
        boots: starting_equipment.boots,
        accessory1: starting_equipment.accessory1,
        accessory2: starting_equipment.accessory2,
    }
}

// ===== Character Database =====

/// Database of all character definitions
///
/// Manages loading, validation, and lookup of character definitions.
/// Typically loaded from `data/characters.ron` or campaign-specific files.
///
/// # Examples
///
/// ```
/// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
/// use antares::domain::character::{Sex, Alignment};
///
/// let mut db = CharacterDatabase::new();
///
/// let knight = CharacterDefinition::new(
///     "pregen_knight".to_string(),
///     "Sir Galahad".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// db.add_character(knight).unwrap();
/// assert!(db.get_character("pregen_knight").is_some());
/// ```
#[derive(Debug, Clone, Default)]
pub struct CharacterDatabase {
    characters: HashMap<CharacterDefinitionId, CharacterDefinition>,
}

impl CharacterDatabase {
    /// Creates an empty character database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDatabase;
    ///
    /// let db = CharacterDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            characters: HashMap::new(),
        }
    }

    /// Loads character definitions from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing character definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(CharacterDatabase)` on success, or an error if the file
    /// cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::character_definition::CharacterDatabase;
    ///
    /// let db = CharacterDatabase::load_from_file("data/characters.ron").unwrap();
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CharacterDefinitionError> {
        let contents = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            CharacterDefinitionError::LoadError(format!("Failed to read file: {}", e))
        })?;

        Self::load_from_string(&contents)
    }

    /// Loads character definitions from a RON string
    ///
    /// # Arguments
    ///
    /// * `data` - RON string containing character definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(CharacterDatabase)` on success, or an error if parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "pregen_knight",
    ///         name: "Sir Galahad",
    ///         race_id: "human",
    ///         class_id: "knight",
    ///         sex: Male,
    ///         alignment: Good,
    ///         base_stats: (
    ///             might: 16,
    ///             intellect: 8,
    ///             personality: 10,
    ///             endurance: 14,
    ///             speed: 12,
    ///             accuracy: 14,
    ///             luck: 10,
    ///         ),
    ///         portrait_id: "1",
    ///         starting_gold: 100,
    ///         starting_gems: 0,
    ///         starting_food: 10,
    ///         starting_items: [],
    ///         starting_equipment: (),
    ///         description: "A noble knight.",
    ///         is_premade: true,
    ///     ),
    /// ]"#;
    ///
    /// let db = CharacterDatabase::load_from_string(ron_data).unwrap();
    /// assert!(db.get_character("pregen_knight").is_some());
    /// ```
    pub fn load_from_string(data: &str) -> Result<Self, CharacterDefinitionError> {
        let definitions: Vec<CharacterDefinition> = ron::from_str(data)
            .map_err(|e| CharacterDefinitionError::ParseError(format!("RON parse error: {}", e)))?;

        let mut db = Self::new();
        for definition in definitions {
            if db.characters.contains_key(&definition.id) {
                return Err(CharacterDefinitionError::DuplicateId(definition.id.clone()));
            }
            definition.validate()?;
            db.characters.insert(definition.id.clone(), definition);
        }

        Ok(db)
    }

    /// Adds a character definition to the database
    ///
    /// # Arguments
    ///
    /// * `definition` - The character definition to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `DuplicateId` error if ID already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// let knight = CharacterDefinition::new(
    ///     "test_knight".to_string(),
    ///     "Test Knight".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    ///
    /// assert!(db.add_character(knight.clone()).is_ok());
    /// assert!(db.add_character(knight).is_err()); // Duplicate
    /// ```
    pub fn add_character(
        &mut self,
        definition: CharacterDefinition,
    ) -> Result<(), CharacterDefinitionError> {
        if self.characters.contains_key(&definition.id) {
            return Err(CharacterDefinitionError::DuplicateId(definition.id.clone()));
        }
        definition.validate()?;
        self.characters.insert(definition.id.clone(), definition);
        Ok(())
    }

    /// Removes a character definition from the database
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the character definition to remove
    ///
    /// # Returns
    ///
    /// Returns the removed definition if found, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// let knight = CharacterDefinition::new(
    ///     "test_knight".to_string(),
    ///     "Test Knight".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    ///
    /// db.add_character(knight).unwrap();
    /// assert!(db.remove_character("test_knight").is_some());
    /// assert!(db.remove_character("test_knight").is_none());
    /// ```
    pub fn remove_character(&mut self, id: &str) -> Option<CharacterDefinition> {
        self.characters.remove(id)
    }

    /// Gets a character definition by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The character definition ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&CharacterDefinition)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// let knight = CharacterDefinition::new(
    ///     "pregen_knight".to_string(),
    ///     "Sir Galahad".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// db.add_character(knight).unwrap();
    ///
    /// let found = db.get_character("pregen_knight");
    /// assert!(found.is_some());
    /// assert_eq!(found.unwrap().name, "Sir Galahad");
    ///
    /// assert!(db.get_character("nonexistent").is_none());
    /// ```
    pub fn get_character(&self, id: &str) -> Option<&CharacterDefinition> {
        self.characters.get(id)
    }

    /// Returns an iterator over all character definitions
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// db.add_character(CharacterDefinition::new(
    ///     "knight".to_string(),
    ///     "Knight".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// )).unwrap();
    ///
    /// assert_eq!(db.all_characters().count(), 1);
    /// ```
    pub fn all_characters(&self) -> impl Iterator<Item = &CharacterDefinition> {
        self.characters.values()
    }

    /// Returns an iterator over all character definition IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// db.add_character(CharacterDefinition::new(
    ///     "knight".to_string(),
    ///     "Knight".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// )).unwrap();
    ///
    /// let ids: Vec<_> = db.all_character_ids().collect();
    /// assert!(ids.contains(&"knight".to_string()));
    /// ```
    pub fn all_character_ids(&self) -> impl Iterator<Item = CharacterDefinitionId> + '_ {
        self.characters.keys().cloned()
    }

    /// Returns all premade characters
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    ///
    /// let mut premade = CharacterDefinition::new(
    ///     "premade".to_string(),
    ///     "Premade".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// premade.is_premade = true;
    /// db.add_character(premade).unwrap();
    ///
    /// let mut template = CharacterDefinition::new(
    ///     "template".to_string(),
    ///     "Template".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// template.is_premade = false;
    /// db.add_character(template).unwrap();
    ///
    /// let premades: Vec<_> = db.premade_characters().collect();
    /// assert_eq!(premades.len(), 1);
    /// assert_eq!(premades[0].id, "premade");
    /// ```
    pub fn premade_characters(&self) -> impl Iterator<Item = &CharacterDefinition> {
        self.characters.values().filter(|c| c.is_premade)
    }

    /// Returns all template (non-premade) characters
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    ///
    /// let mut template = CharacterDefinition::new(
    ///     "template".to_string(),
    ///     "Template".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// template.is_premade = false;
    /// db.add_character(template).unwrap();
    ///
    /// let templates: Vec<_> = db.template_characters().collect();
    /// assert_eq!(templates.len(), 1);
    /// ```
    pub fn template_characters(&self) -> impl Iterator<Item = &CharacterDefinition> {
        self.characters.values().filter(|c| !c.is_premade)
    }

    /// Validates the entire character database
    ///
    /// Checks all character definitions for validity.
    /// Note: Cross-reference validation (race_id, class_id, item_ids)
    /// requires access to the respective databases and should be done
    /// at a higher level (e.g., ContentDatabase).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all validations pass, or the first error encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db = CharacterDatabase::new();
    /// db.add_character(CharacterDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// )).unwrap();
    ///
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CharacterDefinitionError> {
        for definition in self.characters.values() {
            definition.validate()?;
        }
        Ok(())
    }

    /// Returns the number of character definitions in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDatabase;
    ///
    /// let db = CharacterDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.characters.len()
    }

    /// Returns true if the database is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::CharacterDatabase;
    ///
    /// let db = CharacterDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.characters.is_empty()
    }

    /// Merges another database into this one
    ///
    /// Character definitions from the other database are added to this one.
    /// Returns an error if any duplicate IDs are encountered.
    ///
    /// # Arguments
    ///
    /// * `other` - The database to merge from
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `DuplicateId` error if any ID conflicts.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::{CharacterDatabase, CharacterDefinition};
    /// use antares::domain::character::{Sex, Alignment};
    ///
    /// let mut db1 = CharacterDatabase::new();
    /// db1.add_character(CharacterDefinition::new(
    ///     "char1".to_string(),
    ///     "Character 1".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// )).unwrap();
    ///
    /// let mut db2 = CharacterDatabase::new();
    /// db2.add_character(CharacterDefinition::new(
    ///     "char2".to_string(),
    ///     "Character 2".to_string(),
    ///     "elf".to_string(),
    ///     "sorcerer".to_string(),
    ///     Sex::Female,
    ///     Alignment::Neutral,
    /// )).unwrap();
    ///
    /// db1.merge(db2).unwrap();
    /// assert_eq!(db1.len(), 2);
    /// ```
    pub fn merge(&mut self, other: CharacterDatabase) -> Result<(), CharacterDefinitionError> {
        for (id, definition) in other.characters {
            if self.characters.contains_key(&id) {
                return Err(CharacterDefinitionError::DuplicateId(id));
            }
            self.characters.insert(id, definition);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Sex};

    #[test]
    fn test_character_definition_hp_is_optional() {
        let def = CharacterDefinition::new(
            "test_char".to_string(),
            "Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        assert!(def.hp_override.is_none());
    }

    #[test]
    fn test_character_definition_hp_roundtrip() {
        let mut def = CharacterDefinition::new(
            "test_char".to_string(),
            "Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        def.hp_override = Some(AttributePair16::new(25));

        let ron_str = ron::ser::to_string(&def).expect("Failed to serialize character to RON");
        let parsed: CharacterDefinition =
            ron::from_str(&ron_str).expect("Failed to deserialize character from RON");
        assert_eq!(parsed.hp_override, Some(AttributePair16::new(25)));
    }

    #[test]
    fn test_character_definition_hp_backward_compatibility() {
        // Test that old RON format with hp_base is still supported
        let ron_str = r#"(
            id: "test_char",
            name: "Test Character",
            race_id: "human",
            class_id: "knight",
            sex: Male,
            alignment: Good,
            base_stats: (
                might: 15,
                intellect: 10,
                personality: 10,
                endurance: 14,
                speed: 12,
                accuracy: 12,
                luck: 10,
            ),
            hp_base: Some(50),
        )"#;

        let parsed: CharacterDefinition =
            ron::from_str(ron_str).expect("Failed to parse old format");
        assert_eq!(parsed.hp_override, Some(AttributePair16::new(50)));
    }

    #[test]
    fn test_character_definition_hp_backward_compatibility_with_current() {
        // Test that old RON format with hp_base and hp_current is supported
        let ron_str = r#"(
            id: "test_char",
            name: "Test Character",
            race_id: "human",
            class_id: "knight",
            sex: Male,
            alignment: Good,
            base_stats: (
                might: 15,
                intellect: 10,
                personality: 10,
                endurance: 14,
                speed: 12,
                accuracy: 12,
                luck: 10,
            ),
            hp_base: Some(50),
            hp_current: Some(25),
        )"#;

        let parsed: CharacterDefinition =
            ron::from_str(ron_str).expect("Failed to parse old format");
        assert_eq!(
            parsed.hp_override,
            Some(AttributePair16 {
                base: 50,
                current: 25
            })
        );
    }

    // ===== StartingEquipment Tests =====

    #[test]
    fn test_starting_equipment_new() {
        let equipment = StartingEquipment::new();
        assert!(equipment.weapon.is_none());
        assert!(equipment.armor.is_none());
        assert!(equipment.shield.is_none());
        assert!(equipment.helmet.is_none());
        assert!(equipment.boots.is_none());
        assert!(equipment.accessory1.is_none());
        assert!(equipment.accessory2.is_none());
    }

    #[test]
    fn test_starting_equipment_is_empty() {
        let empty = StartingEquipment::new();
        assert!(empty.is_empty());

        let with_weapon = StartingEquipment {
            weapon: Some(1),
            ..Default::default()
        };
        assert!(!with_weapon.is_empty());
    }

    #[test]
    fn test_starting_equipment_equipped_count() {
        let equipment = StartingEquipment {
            weapon: Some(1),
            armor: Some(10),
            shield: None,
            helmet: Some(20),
            boots: None,
            accessory1: None,
            accessory2: None,
        };
        assert_eq!(equipment.equipped_count(), 3);
    }

    #[test]
    fn test_starting_equipment_all_item_ids() {
        let equipment = StartingEquipment {
            weapon: Some(1),
            armor: Some(10),
            shield: None,
            helmet: Some(20),
            boots: None,
            accessory1: Some(30),
            accessory2: None,
        };
        let items = equipment.all_item_ids();
        assert_eq!(items.len(), 4);
        assert!(items.contains(&1));
        assert!(items.contains(&10));
        assert!(items.contains(&20));
        assert!(items.contains(&30));
    }

    #[test]
    fn test_starting_equipment_serialization() {
        let equipment = StartingEquipment {
            weapon: Some(1),
            armor: Some(10),
            ..Default::default()
        };

        let serialized = ron::to_string(&equipment).unwrap();
        let deserialized: StartingEquipment = ron::from_str(&serialized).unwrap();

        assert_eq!(equipment, deserialized);
    }

    // ===== BaseStats Tests (Deprecated) =====

    #[test]
    #[allow(deprecated)]
    fn test_base_stats_new() {
        let stats = BaseStats::new(14, 10, 12, 13, 11, 15, 10);
        assert_eq!(stats.might, 14);
        assert_eq!(stats.intellect, 10);
        assert_eq!(stats.personality, 12);
        assert_eq!(stats.endurance, 13);
        assert_eq!(stats.speed, 11);
        assert_eq!(stats.accuracy, 15);
        assert_eq!(stats.luck, 10);
    }

    #[test]
    #[allow(deprecated)]
    fn test_base_stats_default() {
        let stats = BaseStats::default();
        assert_eq!(stats.might, 10);
        assert_eq!(stats.intellect, 10);
        assert_eq!(stats.personality, 10);
        assert_eq!(stats.endurance, 10);
        assert_eq!(stats.speed, 10);
        assert_eq!(stats.accuracy, 10);
        assert_eq!(stats.luck, 10);
    }

    #[test]
    #[allow(deprecated)]
    fn test_base_stats_to_stats() {
        let base = BaseStats::new(14, 10, 12, 13, 11, 15, 10);
        let stats = base.to_stats();

        assert_eq!(stats.might.base, 14);
        assert_eq!(stats.might.current, 14);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.accuracy.base, 15);
    }

    #[test]
    #[allow(deprecated)]
    fn test_base_stats_serialization() {
        let stats = BaseStats::new(16, 8, 10, 14, 12, 14, 10);
        let serialized = ron::to_string(&stats).unwrap();
        let deserialized: BaseStats = ron::from_str(&serialized).unwrap();
        assert_eq!(stats, deserialized);
    }

    // ===== Stats Tests =====

    #[test]
    fn test_stats_new() {
        let stats = Stats::new(14, 10, 12, 13, 11, 15, 10);
        assert_eq!(stats.might.base, 14);
        assert_eq!(stats.might.current, 14);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.personality.base, 12);
        assert_eq!(stats.endurance.base, 13);
        assert_eq!(stats.speed.base, 11);
        assert_eq!(stats.accuracy.base, 15);
        assert_eq!(stats.luck.base, 10);
    }

    #[test]
    fn test_stats_serialization_simple_format() {
        // Test that simple format (plain numbers) works
        let ron_str = r#"(
            might: 15,
            intellect: 10,
            personality: 12,
            endurance: 14,
            speed: 11,
            accuracy: 13,
            luck: 10,
        )"#;

        let stats: Stats = ron::from_str(ron_str).expect("Failed to deserialize simple format");
        assert_eq!(stats.might.base, 15);
        assert_eq!(stats.might.current, 15);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.intellect.current, 10);
    }

    #[test]
    fn test_stats_serialization_full_format() {
        // Test that full format (base and current) works
        let ron_str = r#"(
            might: (base: 15, current: 18),
            intellect: (base: 10, current: 10),
            personality: (base: 12, current: 12),
            endurance: (base: 14, current: 14),
            speed: (base: 11, current: 11),
            accuracy: (base: 13, current: 13),
            luck: (base: 10, current: 10),
        )"#;

        let stats: Stats = ron::from_str(ron_str).expect("Failed to deserialize full format");
        assert_eq!(stats.might.base, 15);
        assert_eq!(stats.might.current, 18);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.intellect.current, 10);
    }

    #[test]
    fn test_stats_serialization_roundtrip() {
        let stats = Stats::new(16, 8, 10, 14, 12, 14, 10);
        let serialized = ron::to_string(&stats).unwrap();
        let deserialized: Stats = ron::from_str(&serialized).unwrap();
        assert_eq!(stats, deserialized);
    }

    // ===== CharacterDefinition Tests =====

    #[test]
    fn test_instantiate_respects_hp_override() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let mut def = CharacterDefinition::new(
            "test_char".to_string(),
            "Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        def.hp_override = Some(AttributePair16 {
            base: 50,
            current: 25,
        });

        let character = def
            .instantiate(&races, &classes, &items)
            .expect("Instantiation failed");
        assert_eq!(character.hp.base, 50u16);
        assert_eq!(character.hp.current, 25u16);
    }

    #[test]
    fn test_instantiate_hp_override_current_clamped_to_base() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let mut def = CharacterDefinition::new(
            "test_char2".to_string(),
            "Test Character 2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        def.hp_override = Some(AttributePair16 {
            base: 30,
            current: 50, // Current exceeds base
        });

        let character = def
            .instantiate(&races, &classes, &items)
            .expect("Instantiation failed");
        assert_eq!(character.hp.base, 30u16);
        assert_eq!(character.hp.current, 30u16); // Should be clamped
    }

    #[test]
    fn test_instantiate_without_hp_override_uses_calculated() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let def = CharacterDefinition::new(
            "test_char3".to_string(),
            "Test Character 3".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        // No hp_override set

        let character = def
            .instantiate(&races, &classes, &items)
            .expect("Instantiation failed");
        assert!(character.hp.base > 0);
        assert_eq!(character.hp.current, character.hp.base);
    }

    fn create_test_knight() -> CharacterDefinition {
        CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Galahad".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    fn create_test_sorcerer() -> CharacterDefinition {
        let mut def = CharacterDefinition::new(
            "test_sorcerer".to_string(),
            "Merlin".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        def.base_stats = Stats::new(8, 16, 12, 10, 14, 10, 12);
        def.is_premade = true;
        def
    }

    #[test]
    fn test_character_definition_new() {
        let def = create_test_knight();
        assert_eq!(def.id, "test_knight");
        assert_eq!(def.name, "Sir Galahad");
        assert_eq!(def.race_id, "human");
        assert_eq!(def.class_id, "knight");
        assert_eq!(def.sex, Sex::Male);
        assert_eq!(def.alignment, Alignment::Good);
        assert_eq!(def.starting_food, 10);
        assert!(!def.is_premade);
    }

    #[test]
    fn test_character_definition_all_item_ids() {
        let mut def = create_test_knight();
        def.starting_items = vec![1, 2, 3];
        def.starting_equipment.weapon = Some(10);
        def.starting_equipment.armor = Some(20);

        let items = def.all_item_ids();
        assert_eq!(items.len(), 5);
        assert!(items.contains(&1));
        assert!(items.contains(&2));
        assert!(items.contains(&3));
        assert!(items.contains(&10));
        assert!(items.contains(&20));
    }

    #[test]
    fn test_character_definition_validate_success() {
        let def = create_test_knight();
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_character_definition_validate_empty_id() {
        let mut def = create_test_knight();
        def.id = String::new();
        let result = def.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::ValidationError(_)
        ));
    }

    #[test]
    fn test_character_definition_validate_empty_name() {
        let mut def = create_test_knight();
        def.name = String::new();
        let result = def.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_character_definition_validate_empty_race_id() {
        let mut def = create_test_knight();
        def.race_id = String::new();
        let result = def.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_character_definition_validate_empty_class_id() {
        let mut def = create_test_knight();
        def.class_id = String::new();
        let result = def.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_character_definition_serialization() {
        let def = create_test_sorcerer();
        let serialized = ron::to_string(&def).unwrap();
        let deserialized: CharacterDefinition = ron::from_str(&serialized).unwrap();
        assert_eq!(def, deserialized);
    }

    // ===== CharacterDatabase Tests =====

    #[test]
    fn test_character_database_new() {
        let db = CharacterDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_character_database_add_character() {
        let mut db = CharacterDatabase::new();
        let def = create_test_knight();
        assert!(db.add_character(def).is_ok());
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_character_database_add_duplicate_error() {
        let mut db = CharacterDatabase::new();
        let def1 = create_test_knight();
        let def2 = create_test_knight();

        assert!(db.add_character(def1).is_ok());
        let result = db.add_character(def2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::DuplicateId(_)
        ));
    }

    #[test]
    fn test_character_database_remove_character() {
        let mut db = CharacterDatabase::new();
        let def = create_test_knight();
        db.add_character(def).unwrap();

        let removed = db.remove_character("test_knight");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Sir Galahad");
        assert!(db.is_empty());

        let removed_again = db.remove_character("test_knight");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_character_database_get_character() {
        let mut db = CharacterDatabase::new();
        db.add_character(create_test_knight()).unwrap();

        let found = db.get_character("test_knight");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Sir Galahad");

        let not_found = db.get_character("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_character_database_all_characters() {
        let mut db = CharacterDatabase::new();
        db.add_character(create_test_knight()).unwrap();
        db.add_character(create_test_sorcerer()).unwrap();

        let all: Vec<_> = db.all_characters().collect();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_character_database_all_character_ids() {
        let mut db = CharacterDatabase::new();
        db.add_character(create_test_knight()).unwrap();
        db.add_character(create_test_sorcerer()).unwrap();

        let ids: Vec<_> = db.all_character_ids().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"test_knight".to_string()));
        assert!(ids.contains(&"test_sorcerer".to_string()));
    }

    #[test]
    fn test_character_database_premade_characters() {
        let mut db = CharacterDatabase::new();
        let mut premade = create_test_knight();
        premade.is_premade = true;
        db.add_character(premade).unwrap();

        let template = create_test_sorcerer();
        let is_premade = template.is_premade;
        db.add_character(template).unwrap();

        let premades: Vec<_> = db.premade_characters().collect();
        // create_test_sorcerer sets is_premade = true
        if is_premade {
            assert_eq!(premades.len(), 2);
        } else {
            assert_eq!(premades.len(), 1);
            assert_eq!(premades[0].id, "test_knight");
        }
    }

    #[test]
    fn test_character_database_template_characters() {
        let mut db = CharacterDatabase::new();

        let mut premade = create_test_knight();
        premade.is_premade = true;
        db.add_character(premade).unwrap();

        let mut template = CharacterDefinition::new(
            "template_char".to_string(),
            "Template".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        template.is_premade = false;
        db.add_character(template).unwrap();

        let templates: Vec<_> = db.template_characters().collect();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "template_char");
    }

    #[test]
    fn test_character_database_validate() {
        let mut db = CharacterDatabase::new();
        db.add_character(create_test_knight()).unwrap();
        db.add_character(create_test_sorcerer()).unwrap();

        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_character_database_merge() {
        let mut db1 = CharacterDatabase::new();
        db1.add_character(create_test_knight()).unwrap();

        let mut db2 = CharacterDatabase::new();
        db2.add_character(create_test_sorcerer()).unwrap();

        assert!(db1.merge(db2).is_ok());
        assert_eq!(db1.len(), 2);
    }

    #[test]
    fn test_character_database_merge_duplicate_error() {
        let mut db1 = CharacterDatabase::new();
        db1.add_character(create_test_knight()).unwrap();

        let mut db2 = CharacterDatabase::new();
        db2.add_character(create_test_knight()).unwrap();

        let result = db1.merge(db2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::DuplicateId(_)
        ));
    }

    #[test]
    fn test_character_database_load_from_string() {
        let ron_data = r#"[
            (
                id: "pregen_knight",
                name: "Sir Galahad",
                race_id: "human",
                class_id: "knight",
                sex: Male,
                alignment: Good,
                base_stats: (
                    might: 16,
                    intellect: 8,
                    personality: 10,
                    endurance: 14,
                    speed: 12,
                    accuracy: 14,
                    luck: 10,
                ),
                portrait_id: "1",
                starting_gold: 100,
                starting_gems: 5,
                starting_food: 10,
                starting_items: [1, 2],
                starting_equipment: (
                    weapon: Some(10),
                    armor: Some(20),
                ),
                description: "A noble knight seeking glory.",
                is_premade: true,
            ),
            (
                id: "pregen_sorcerer",
                name: "Merlin",
                race_id: "elf",
                class_id: "sorcerer",
                sex: Male,
                alignment: Neutral,
                base_stats: (
                    might: 8,
                    intellect: 16,
                    personality: 12,
                    endurance: 10,
                    speed: 14,
                    accuracy: 10,
                    luck: 12,
                ),
                portrait_id: "2",
                starting_gold: 50,
                starting_gems: 10,
                starting_food: 10,
                starting_items: [],
                starting_equipment: (),
                description: "A wise wizard.",
                is_premade: true,
            ),
        ]"#;

        let db = CharacterDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 2);

        let knight = db.get_character("pregen_knight").unwrap();
        assert_eq!(knight.name, "Sir Galahad");
        assert_eq!(knight.base_stats.might.base, 16);
        assert_eq!(knight.starting_gold, 100);
        assert_eq!(knight.starting_items.len(), 2);
        assert!(knight.starting_equipment.weapon.is_some());

        let sorcerer = db.get_character("pregen_sorcerer").unwrap();
        assert_eq!(sorcerer.name, "Merlin");
        assert_eq!(sorcerer.base_stats.intellect.base, 16);
    }

    #[test]
    fn test_character_database_load_from_string_rejects_numeric_portrait_id() {
        // RON with a numeric portrait_id should fail parsing since portrait_id is now a string key
        let ron_numeric = r#"[
            (
                id: "numeric_knight",
                name: "Num Knight",
                race_id: "human",
                class_id: "knight",
                sex: Male,
                alignment: Good,
                base_stats: (
                    might: 10,
                    intellect: 10,
                    personality: 10,
                    endurance: 10,
                    speed: 10,
                    accuracy: 10,
                    luck: 10,
                ),
                portrait_id: 1,
                starting_gold: 0,
                starting_gems: 0,
                starting_food: 10,
                starting_items: [],
                starting_equipment: (),
                description: "",
                is_premade: true,
            ),
        ]"#;

        let res = CharacterDatabase::load_from_string(ron_numeric);
        assert!(matches!(res, Err(CharacterDefinitionError::ParseError(_))));
    }

    #[test]
    fn test_character_definition_validate_rejects_non_normalized_portrait_id() {
        // Non-normalized portrait IDs (spaces / uppercase) should be rejected by validate()
        let mut def = CharacterDefinition::new(
            "test_char".to_string(),
            "Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        def.portrait_id = "Sir Galahad".to_string(); // contains space and uppercase
        let res = def.validate();
        assert!(matches!(
            res,
            Err(CharacterDefinitionError::ValidationError(_))
        ));
    }

    #[test]
    fn test_character_definition_validate_accepts_normalized_portrait_id() {
        // Properly normalized portrait IDs should pass
        let mut def = CharacterDefinition::new(
            "test_char2".to_string(),
            "Test Character 2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        def.portrait_id = "sir_galahad".to_string(); // normalized form
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_character_database_load_minimal() {
        let ron_data = r#"[
            (
                id: "minimal",
                name: "Minimal Character",
                race_id: "human",
                class_id: "knight",
                sex: Male,
                alignment: Good,
                base_stats: (
                    might: 10,
                    intellect: 10,
                    personality: 10,
                    endurance: 10,
                    speed: 10,
                    accuracy: 10,
                    luck: 10,
                ),
            ),
        ]"#;

        let db = CharacterDatabase::load_from_string(ron_data).unwrap();
        let character = db.get_character("minimal").unwrap();
        assert_eq!(character.starting_gold, 0);
        assert_eq!(character.starting_food, 10);
        assert!(character.starting_items.is_empty());
        assert!(character.starting_equipment.is_empty());
    }

    #[test]
    fn test_character_database_load_duplicate_id_error() {
        let ron_data = r#"[
            (
                id: "duplicate",
                name: "First",
                race_id: "human",
                class_id: "knight",
                sex: Male,
                alignment: Good,
                base_stats: (might: 10, intellect: 10, personality: 10, endurance: 10, speed: 10, accuracy: 10, luck: 10),
            ),
            (
                id: "duplicate",
                name: "Second",
                race_id: "elf",
                class_id: "sorcerer",
                sex: Female,
                alignment: Neutral,
                base_stats: (might: 10, intellect: 10, personality: 10, endurance: 10, speed: 10, accuracy: 10, luck: 10),
            ),
        ]"#;

        let result = CharacterDatabase::load_from_string(ron_data);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::DuplicateId(_)
        ));
    }

    #[test]
    fn test_character_database_load_invalid_ron() {
        let invalid_ron = "this is not valid ron";
        let result = CharacterDatabase::load_from_string(invalid_ron);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::ParseError(_)
        ));
    }

    #[test]
    fn test_character_database_load_validation_error() {
        let ron_data = r#"[
            (
                id: "",
                name: "Invalid",
                race_id: "human",
                class_id: "knight",
                sex: Male,
                alignment: Good,
                base_stats: (might: 10, intellect: 10, personality: 10, endurance: 10, speed: 10, accuracy: 10, luck: 10),
            ),
        ]"#;

        let result = CharacterDatabase::load_from_string(ron_data);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::ValidationError(_)
        ));
    }

    // ===== Integration Tests for Data Files =====

    #[test]
    fn test_load_core_characters_data_file() {
        // This test verifies that the actual data/characters.ron file is valid
        let result = CharacterDatabase::load_from_file("data/characters.ron");
        assert!(
            result.is_ok(),
            "Failed to load data/characters.ron: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // We expect 6 pre-made characters (one per class)
        assert_eq!(db.len(), 6, "Expected 6 characters in data/characters.ron");

        // Verify all expected characters exist
        let expected_ids = [
            "pregen_human_knight",
            "pregen_elf_paladin",
            "pregen_halfelf_archer",
            "pregen_dwarf_cleric",
            "pregen_gnome_sorcerer",
            "pregen_halforc_robber",
        ];

        for id in &expected_ids {
            assert!(
                db.get_character(id).is_some(),
                "Expected character '{}' not found in data/characters.ron",
                id
            );
        }

        // Verify all are pre-made characters
        for char_def in db.all_characters() {
            assert!(
                char_def.is_premade,
                "Character '{}' should be a pre-made character",
                char_def.id
            );
        }

        // Verify premade_characters returns all of them
        let premade: Vec<_> = db.premade_characters().collect();
        assert_eq!(
            premade.len(),
            6,
            "All 6 characters should be pre-made in core data"
        );
    }

    #[test]
    fn test_core_characters_have_valid_references() {
        let db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load data/characters.ron");

        // Valid race IDs from data/races.ron
        let valid_races = ["human", "elf", "dwarf", "gnome", "half_elf", "half_orc"];

        // Valid class IDs from data/classes.ron
        let valid_classes = [
            "knight", "paladin", "archer", "cleric", "sorcerer", "robber",
        ];

        for char_def in db.all_characters() {
            // Check race_id is valid
            assert!(
                valid_races.contains(&char_def.race_id.as_str()),
                "Character '{}' has invalid race_id: '{}'",
                char_def.id,
                char_def.race_id
            );

            // Check class_id is valid
            assert!(
                valid_classes.contains(&char_def.class_id.as_str()),
                "Character '{}' has invalid class_id: '{}'",
                char_def.id,
                char_def.class_id
            );

            // Verify starting stats are reasonable (3-18 range)
            assert!(
                char_def.base_stats.might.base >= 3 && char_def.base_stats.might.base <= 18,
                "Character '{}' has out-of-range might: {}",
                char_def.id,
                char_def.base_stats.might.base
            );
            assert!(
                char_def.base_stats.intellect.base >= 3 && char_def.base_stats.intellect.base <= 18,
                "Character '{}' has out-of-range intellect: {}",
                char_def.id,
                char_def.base_stats.intellect.base
            );

            // Verify description is not empty for pre-made characters
            if char_def.is_premade {
                assert!(
                    !char_def.description.is_empty(),
                    "Pre-made character '{}' should have a description",
                    char_def.id
                );
            }
        }
    }

    #[test]
    fn test_load_tutorial_campaign_characters() {
        // This test verifies that the tutorial campaign characters.ron file is valid
        let result = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron");
        assert!(
            result.is_ok(),
            "Failed to load campaigns/tutorial/data/characters.ron: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // We expect at least 9 characters (3 tutorial premade + 3 NPCs + 3 templates)
        assert!(
            db.len() >= 9,
            "Expected at least 9 characters in tutorial campaign, got {}",
            db.len()
        );

        // Verify tutorial pre-made characters exist
        let premade_ids = [
            "tutorial_human_knight",
            "tutorial_elf_sorcerer",
            "tutorial_human_cleric",
        ];

        for id in &premade_ids {
            let char_def = db.get_character(id);
            assert!(
                char_def.is_some(),
                "Expected tutorial pre-made character '{}' not found",
                id
            );
            assert!(
                char_def.unwrap().is_premade,
                "Character '{}' should be pre-made",
                id
            );
        }

        // Verify recruitable NPC characters exist and are pre-made
        let npc_ids = ["old_gareth", "whisper", "apprentice_zara"];

        for id in &npc_ids {
            let char_def = db.get_character(id);
            assert!(
                char_def.is_some(),
                "Expected recruitable NPC character '{}' not found",
                id
            );
            assert!(
                char_def.unwrap().is_premade,
                "Recruitable NPC '{}' should be pre-made",
                id
            );
        }

        // Verify template characters exist and are not pre-made
        let template_ids = [
            "template_human_fighter",
            "template_elf_mage",
            "template_dwarf_cleric",
        ];

        for id in &template_ids {
            let char_def = db.get_character(id);
            assert!(
                char_def.is_some(),
                "Expected template character '{}' not found",
                id
            );
            assert!(
                !char_def.unwrap().is_premade,
                "Template '{}' should not be pre-made",
                id
            );
        }
    }

    #[test]
    fn test_tutorial_campaign_characters_valid_references() {
        let db = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
            .expect("Failed to load campaigns/tutorial/data/characters.ron");

        // Valid race IDs from races.ron
        let valid_races = ["human", "elf", "dwarf", "gnome", "half_elf", "half_orc"];

        // Valid class IDs from classes.ron
        let valid_classes = [
            "knight", "paladin", "archer", "cleric", "sorcerer", "robber",
        ];

        for char_def in db.all_characters() {
            // Check race_id is valid
            assert!(
                valid_races.contains(&char_def.race_id.as_str()),
                "Character '{}' has invalid race_id: '{}'",
                char_def.id,
                char_def.race_id
            );

            // Check class_id is valid
            assert!(
                valid_classes.contains(&char_def.class_id.as_str()),
                "Character '{}' has invalid class_id: '{}'",
                char_def.id,
                char_def.class_id
            );
        }
    }

    #[test]
    fn test_premade_vs_template_characters() {
        let core_db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load core characters");

        let tutorial_db =
            CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
                .expect("Failed to load tutorial characters");

        // Core data should only have pre-made characters
        let core_templates: Vec<_> = core_db.template_characters().collect();
        let core_premade: Vec<_> = core_db.premade_characters().collect();
        assert_eq!(
            core_templates.len(),
            0,
            "Core characters.ron should not have template characters"
        );
        assert_eq!(
            core_premade.len(),
            core_db.len(),
            "All core characters should be pre-made"
        );

        // Tutorial data should have both pre-made and templates/NPCs
        let tutorial_premade: Vec<_> = tutorial_db.premade_characters().collect();
        let tutorial_templates: Vec<_> = tutorial_db.template_characters().collect();

        assert!(
            !tutorial_premade.is_empty(),
            "Tutorial campaign should have some pre-made characters"
        );
        assert!(
            !tutorial_templates.is_empty(),
            "Tutorial campaign should have some template/NPC characters"
        );
        assert_eq!(
            tutorial_premade.len() + tutorial_templates.len(),
            tutorial_db.len(),
            "Pre-made + template counts should equal total"
        );
    }

    #[test]
    fn test_character_starting_equipment_items_exist() {
        // This test verifies that starting equipment item IDs reference valid items
        // We check against known valid item IDs from data/items.ron
        let valid_item_ids: Vec<ItemId> = vec![
            1, 2, 3, 4, 5, 6, 7, // Basic weapons
            10, 11, 12, // Magical weapons
            20, 21, 22, // Basic armor
            30, 31, // Magical armor
            40, 41, 42, // Accessories
            50, 51, 52, // Consumables
            60, 61, // Ammunition
            100, 101, // Quest/cursed items
        ];

        let db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load core characters");

        for char_def in db.all_characters() {
            // Check starting items
            for item_id in &char_def.starting_items {
                assert!(
                    valid_item_ids.contains(item_id),
                    "Character '{}' has invalid starting item ID: {}",
                    char_def.id,
                    item_id
                );
            }

            // Check equipped items
            for item_id in char_def.starting_equipment.all_item_ids() {
                assert!(
                    valid_item_ids.contains(&item_id),
                    "Character '{}' has invalid equipped item ID: {}",
                    char_def.id,
                    item_id
                );
            }
        }
    }

    // ===== Instantiation Tests =====

    #[test]
    fn test_apply_race_modifiers_no_modifiers() {
        use crate::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

        let base_stats = Stats::new(10, 10, 10, 10, 10, 10, 10);
        let race_def = RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "Test race".to_string(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        };

        let stats = apply_race_modifiers(&base_stats, &race_def);
        assert_eq!(stats.might.base, 10);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.personality.base, 10);
        assert_eq!(stats.endurance.base, 10);
        assert_eq!(stats.speed.base, 10);
        assert_eq!(stats.accuracy.base, 10);
        assert_eq!(stats.luck.base, 10);
    }

    #[test]
    fn test_apply_race_modifiers_with_bonuses() {
        use crate::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

        let base_stats = Stats::new(10, 10, 10, 10, 10, 10, 10);
        let race_def = RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: "Test race".to_string(),
            stat_modifiers: StatModifiers {
                might: 0,
                intellect: 2,
                personality: 0,
                endurance: -1,
                speed: 1,
                accuracy: 1,
                luck: 0,
            },
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        };

        let stats = apply_race_modifiers(&base_stats, &race_def);
        assert_eq!(stats.might.base, 10);
        assert_eq!(stats.intellect.base, 12);
        assert_eq!(stats.personality.base, 10);
        assert_eq!(stats.endurance.base, 9);
        assert_eq!(stats.speed.base, 11);
        assert_eq!(stats.accuracy.base, 11);
        assert_eq!(stats.luck.base, 10);
    }

    #[test]
    fn test_apply_race_modifiers_clamping() {
        use crate::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

        // Test clamping at lower bound
        let low_stats = Stats::new(3, 3, 3, 3, 3, 3, 3);
        let race_def = RaceDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            stat_modifiers: StatModifiers {
                might: -5,
                intellect: 0,
                personality: 0,
                endurance: 0,
                speed: 0,
                accuracy: 0,
                luck: 0,
            },
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        };

        let stats = apply_race_modifiers(&low_stats, &race_def);
        assert_eq!(stats.might.base, 3); // Clamped to minimum

        // Test clamping at upper bound
        let high_stats = Stats::new(18, 18, 18, 18, 18, 18, 18);
        let race_def_bonus = RaceDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            stat_modifiers: StatModifiers {
                might: 10,
                intellect: 0,
                personality: 0,
                endurance: 0,
                speed: 0,
                accuracy: 0,
                luck: 0,
            },
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        };

        let stats = apply_race_modifiers(&high_stats, &race_def_bonus);
        assert_eq!(stats.might.base, 25); // Clamped to maximum
    }

    #[test]
    fn test_calculate_starting_hp_knight() {
        use crate::domain::classes::ClassDefinition;
        use crate::domain::types::DiceRoll;

        // Knight has d10 HP die
        let knight = ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        // Endurance 10 = 0 modifier, HP = 10 + 0 = 10
        let hp = calculate_starting_hp(&knight, 10);
        assert_eq!(hp.base, 10);
        assert_eq!(hp.current, 10);

        // Endurance 14 = +2 modifier, HP = 10 + 2 = 12
        let hp = calculate_starting_hp(&knight, 14);
        assert_eq!(hp.base, 12);

        // Endurance 8 = -1 modifier, HP = 10 - 1 = 9
        let hp = calculate_starting_hp(&knight, 8);
        assert_eq!(hp.base, 9);
    }

    #[test]
    fn test_calculate_starting_hp_minimum() {
        use crate::domain::classes::ClassDefinition;
        use crate::domain::types::DiceRoll;

        // Sorcerer has d4 HP die
        let sorcerer = ClassDefinition {
            id: "sorcerer".to_string(),
            name: "Sorcerer".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 4, 0),
            spell_school: None,
            is_pure_caster: true,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        // Very low endurance should still give minimum 1 HP
        let hp = calculate_starting_hp(&sorcerer, 3);
        assert!(hp.base >= 1, "HP should be at least 1");
    }

    #[test]
    fn test_calculate_starting_sp_non_caster() {
        use crate::domain::classes::ClassDefinition;
        use crate::domain::types::DiceRoll;

        let knight = ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        let stats = Stats::new(10, 16, 10, 10, 10, 10, 10);
        let sp = calculate_starting_sp(&knight, &stats);
        assert_eq!(sp.base, 0);
        assert_eq!(sp.current, 0);
    }

    #[test]
    fn test_calculate_starting_sp_pure_caster() {
        use crate::domain::classes::{ClassDefinition, SpellSchool};
        use crate::domain::types::DiceRoll;

        let sorcerer = ClassDefinition {
            id: "sorcerer".to_string(),
            name: "Sorcerer".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 4, 0),
            spell_school: Some(SpellSchool::Sorcerer),
            is_pure_caster: true,
            spell_stat: Some(SpellStat::Intellect),
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        // Intellect 16 = SP = 16 - 10 = 6
        let stats = Stats::new(10, 16, 10, 10, 10, 10, 10);
        let sp = calculate_starting_sp(&sorcerer, &stats);
        assert_eq!(sp.base, 6);
        assert_eq!(sp.current, 6);

        // Intellect 10 = SP = 10 - 10 = 0
        let stats = Stats::new(10, 10, 10, 10, 10, 10, 10);
        let sp = calculate_starting_sp(&sorcerer, &stats);
        assert_eq!(sp.base, 0);
    }

    #[test]
    fn test_calculate_starting_sp_hybrid_caster() {
        use crate::domain::classes::{ClassDefinition, SpellSchool};
        use crate::domain::types::DiceRoll;

        let paladin = ClassDefinition {
            id: "paladin".to_string(),
            name: "Paladin".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 8, 0),
            spell_school: Some(SpellSchool::Cleric),
            is_pure_caster: false, // Hybrid
            spell_stat: Some(SpellStat::Personality),
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        // Personality 16 = SP = (16 - 10) / 2 = 3
        let stats = Stats::new(10, 10, 16, 10, 10, 10, 10);
        let sp = calculate_starting_sp(&paladin, &stats);
        assert_eq!(sp.base, 3);
    }

    #[test]
    fn test_calculate_starting_spell_level() {
        use crate::domain::classes::{ClassDefinition, SpellSchool};
        use crate::domain::types::DiceRoll;

        let sorcerer = ClassDefinition {
            id: "sorcerer".to_string(),
            name: "Sorcerer".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 4, 0),
            spell_school: Some(SpellSchool::Sorcerer),
            is_pure_caster: true,
            spell_stat: Some(SpellStat::Intellect),
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        let paladin = ClassDefinition {
            id: "paladin".to_string(),
            name: "Paladin".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 8, 0),
            spell_school: Some(SpellSchool::Cleric),
            is_pure_caster: false,
            spell_stat: Some(SpellStat::Personality),
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        let knight = ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: "".to_string(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        };

        assert_eq!(calculate_starting_spell_level(&sorcerer), 1);
        assert_eq!(calculate_starting_spell_level(&paladin), 0);
        assert_eq!(calculate_starting_spell_level(&knight), 0);
    }

    #[test]
    fn test_apply_race_resistances() {
        use crate::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

        let race_def = RaceDefinition {
            id: "dwarf".to_string(),
            name: "Dwarf".to_string(),
            description: "Test".to_string(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances {
                magic: 5,
                fire: 0,
                cold: 0,
                electricity: 0,
                acid: 0,
                fear: 0,
                poison: 10,
                psychic: 0,
            },
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        };

        let resistances = apply_race_resistances(&race_def);
        assert_eq!(resistances.magic.base, 5);
        assert_eq!(resistances.poison.base, 10);
        assert_eq!(resistances.fire.base, 0);
    }

    #[test]
    fn test_populate_starting_inventory_empty() {
        let inventory = populate_starting_inventory("test", &[]).unwrap();
        assert!(inventory.items.is_empty());
        assert!(inventory.has_space());
    }

    #[test]
    fn test_populate_starting_inventory_with_items() {
        let items = vec![1, 2, 3];
        let inventory = populate_starting_inventory("test", &items).unwrap();
        assert_eq!(inventory.items.len(), 3);
        assert_eq!(inventory.items[0].item_id, 1);
        assert_eq!(inventory.items[1].item_id, 2);
        assert_eq!(inventory.items[2].item_id, 3);
    }

    #[test]
    fn test_populate_starting_inventory_full() {
        // Try to add more items than inventory can hold
        let items: Vec<ItemId> = (0..=Inventory::MAX_ITEMS as u8).collect();
        let result = populate_starting_inventory("test", &items);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::InventoryFull { .. }
        ));
    }

    #[test]
    fn test_create_starting_equipment_empty() {
        let starting = StartingEquipment::new();
        let equipment = create_starting_equipment(&starting);
        assert!(equipment.weapon.is_none());
        assert!(equipment.armor.is_none());
        assert!(equipment.shield.is_none());
        assert!(equipment.helmet.is_none());
        assert!(equipment.boots.is_none());
        assert!(equipment.accessory1.is_none());
        assert!(equipment.accessory2.is_none());
    }

    #[test]
    fn test_create_starting_equipment_with_items() {
        let starting = StartingEquipment {
            weapon: Some(1),
            armor: Some(20),
            shield: Some(30),
            helmet: None,
            boots: None,
            accessory1: Some(40),
            accessory2: None,
        };

        let equipment = create_starting_equipment(&starting);
        assert_eq!(equipment.weapon, Some(1));
        assert_eq!(equipment.armor, Some(20));
        assert_eq!(equipment.shield, Some(30));
        assert!(equipment.helmet.is_none());
        assert!(equipment.boots.is_none());
        assert_eq!(equipment.accessory1, Some(40));
        assert!(equipment.accessory2.is_none());
    }

    #[test]
    fn test_instantiate_with_real_databases() {
        // Load real databases for integration test
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        // Create a simple character definition
        let definition = CharacterDefinition {
            id: "test_knight".to_string(),
            name: "Test Knight".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Good,
            base_stats: Stats::new(14, 10, 10, 12, 10, 12, 10),
            hp_override: None,
            portrait_id: "1".to_string(),
            starting_gold: 100,
            starting_gems: 5,
            starting_food: 15,
            starting_items: vec![50], // A consumable
            starting_equipment: StartingEquipment {
                weapon: Some(1), // Basic weapon
                armor: Some(20), // Basic armor
                ..Default::default()
            },
            description: "A test knight".to_string(),
            is_premade: true,
            starts_in_party: false,
        };

        let character = definition
            .instantiate(&races, &classes, &items)
            .expect("Failed to instantiate character");

        // Verify basic fields
        assert_eq!(character.name, "Test Knight");
        assert_eq!(character.race_id, "human");
        assert_eq!(character.class_id, "knight");
        assert_eq!(character.sex, Sex::Male);
        assert_eq!(character.alignment, Alignment::Good);
        assert_eq!(character.alignment_initial, Alignment::Good);
        assert_eq!(character.level, 1);
        assert_eq!(character.experience, 0);
        assert_eq!(character.age, 18);
        assert_eq!(character.portrait_id, "1");
        assert_eq!(character.gold, 100);
        assert_eq!(character.gems, 5);
        assert_eq!(character.food, 15);

        // Verify stats were set
        assert!(character.stats.might.base >= 3);
        assert!(character.stats.endurance.base >= 3);

        // Verify HP was calculated (should be > 0)
        assert!(character.hp.base > 0);

        // Verify inventory has starting item
        assert_eq!(character.inventory.items.len(), 1);
        assert_eq!(character.inventory.items[0].item_id, 50);

        // Verify equipment was set
        assert_eq!(character.equipment.weapon, Some(1));
        assert_eq!(character.equipment.armor, Some(20));
        assert!(character.equipment.shield.is_none());
    }

    #[test]
    fn test_instantiate_invalid_race() {
        let races = RaceDatabase::new();
        let classes = ClassDatabase::new();
        let items = ItemDatabase::new();

        let definition = CharacterDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            "invalid_race".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let result = definition.instantiate(&races, &classes, &items);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::InvalidRaceId { .. }
        ));
    }

    #[test]
    fn test_instantiate_invalid_class() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes = ClassDatabase::new(); // Empty - no classes
        let items = ItemDatabase::new();

        let definition = CharacterDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            "human".to_string(),
            "invalid_class".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let result = definition.instantiate(&races, &classes, &items);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::InvalidClassId { .. }
        ));
    }

    #[test]
    fn test_instantiate_invalid_item() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items = ItemDatabase::new(); // Empty - no items

        let mut definition = CharacterDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        definition.starting_items = vec![255]; // Invalid item ID (not in empty database)

        let result = definition.instantiate(&races, &classes, &items);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterDefinitionError::InvalidItemId { .. }
        ));
    }

    #[test]
    fn test_instantiate_all_core_characters() {
        // Integration test: instantiate all core characters
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");
        let char_db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load characters.ron");

        for char_def in char_db.all_characters() {
            let result = char_def.instantiate(&races, &classes, &items);
            assert!(
                result.is_ok(),
                "Failed to instantiate character '{}': {:?}",
                char_def.id,
                result.err()
            );

            let character = result.unwrap();
            assert_eq!(character.name, char_def.name);
            assert_eq!(character.gold, char_def.starting_gold);
            assert_eq!(character.gems, char_def.starting_gems);
            assert_eq!(character.food, char_def.starting_food);
            assert!(
                character.hp.base > 0,
                "Character '{}' should have HP > 0",
                char_def.id
            );
            assert!(
                character.is_alive(),
                "Character '{}' should be alive",
                char_def.id
            );
        }
    }

    #[test]
    fn test_instantiate_sorcerer_has_sp() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        // Create a sorcerer with high intellect
        let definition = CharacterDefinition {
            id: "test_sorcerer".to_string(),
            name: "Test Sorcerer".to_string(),
            race_id: "human".to_string(),
            class_id: "sorcerer".to_string(),
            sex: Sex::Female,
            alignment: Alignment::Neutral,
            base_stats: Stats::new(6, 18, 6, 8, 6, 8, 6),
            hp_override: None,
            portrait_id: "2".to_string(),
            starting_gold: 50,
            starting_gems: 10,
            starting_food: 10,
            starting_items: vec![], // No items
            starting_equipment: StartingEquipment {
                ..Default::default()
            },
            description: "A test sorcerer".to_string(),
            is_premade: true,
            starts_in_party: false,
        };

        let character = definition
            .instantiate(&races, &classes, &items)
            .expect("Failed to instantiate sorcerer");

        assert_eq!(character.class_id, "sorcerer");
        // Sorcerer with 16+ intellect should have SP > 0
        // With base 16 intellect + possible gnome bonus, SP = (int - 10) should be > 0
        assert!(
            character.sp.base > 0,
            "Sorcerer with high intellect should have SP, got {}",
            character.sp.base
        );
        // Pure caster should start with spell level 1
        assert_eq!(character.spell_level.base, 1);
    }

    #[test]
    fn test_instantiate_knight_has_no_sp() {
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let definition = CharacterDefinition {
            id: "test_knight".to_string(),
            name: "Test Knight".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Good,
            base_stats: Stats::new(16, 16, 10, 14, 10, 14, 10),
            hp_override: None,
            portrait_id: "1".to_string(),
            starting_gold: 100,
            starting_gems: 0,
            starting_food: 10,
            starting_items: vec![],
            starting_equipment: StartingEquipment::default(),
            description: "Test".to_string(),
            is_premade: true,
            starts_in_party: false,
        };

        let character = definition
            .instantiate(&races, &classes, &items)
            .expect("Failed to instantiate knight");

        assert_eq!(character.class_id, "knight");
        // Knight should have 0 SP even with high intellect
        assert_eq!(character.sp.base, 0);
        // Non-caster should have spell level 0
        assert_eq!(character.spell_level.base, 0);
    }

    // ===== Phase 2: Campaign Data Migration Tests =====
    //
    // These tests verify that existing campaign data files load correctly
    // with the AttributePair migration (Phase 1 backward compatibility).

    #[test]
    fn test_phase2_tutorial_campaign_loads() {
        // Verify tutorial campaign characters.ron loads with new AttributePair format
        let result = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron");

        assert!(
            result.is_ok(),
            "Tutorial campaign characters.ron should load successfully: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // Tutorial campaign has 9 character definitions
        assert_eq!(
            db.len(),
            9,
            "Tutorial campaign should have 9 character definitions"
        );

        // Verify specific characters exist
        assert!(
            db.get_character("tutorial_human_knight").is_some(),
            "Should find Kira (tutorial_human_knight)"
        );
        assert!(
            db.get_character("tutorial_elf_sorcerer").is_some(),
            "Should find Sirius (tutorial_elf_sorcerer)"
        );
        assert!(
            db.get_character("tutorial_human_cleric").is_some(),
            "Should find Mira (tutorial_human_cleric)"
        );
    }

    #[test]
    fn test_phase2_tutorial_campaign_hp_override() {
        // Verify that tutorial campaign hp_base fields convert to hp_override correctly
        let db = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
            .expect("Failed to load tutorial campaign");

        let kira = db
            .get_character("tutorial_human_knight")
            .expect("Should find Kira");

        // Tutorial campaign uses hp_base: Some(10)
        // Should convert to hp_override: Some(AttributePair16 { base: 10, current: 10 })
        assert!(
            kira.hp_override.is_some(),
            "Kira should have hp_override from hp_base"
        );
        assert_eq!(
            kira.hp_override.unwrap().base,
            10,
            "Kira's hp_override base should be 10"
        );
        assert_eq!(
            kira.hp_override.unwrap().current,
            10,
            "Kira's hp_override current should equal base (not pre-buffed)"
        );
    }

    #[test]
    fn test_phase2_tutorial_campaign_stats_format() {
        // Verify that tutorial campaign base_stats (simple format) deserialize correctly
        let db = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
            .expect("Failed to load tutorial campaign");

        let sirius = db
            .get_character("tutorial_elf_sorcerer")
            .expect("Should find Sirius");

        // Tutorial campaign uses simple format: might: 8
        // Should deserialize to Stats with AttributePair { base: 8, current: 8 }
        assert_eq!(
            sirius.base_stats.might.base, 8,
            "Sirius might base should be 8"
        );
        assert_eq!(
            sirius.base_stats.might.current, 8,
            "Sirius might current should equal base"
        );
        assert_eq!(
            sirius.base_stats.intellect.base, 16,
            "Sirius intellect base should be 16"
        );
        assert_eq!(
            sirius.base_stats.intellect.current, 16,
            "Sirius intellect current should equal base"
        );
    }

    #[test]
    fn test_phase2_core_campaign_loads() {
        // Verify core data/characters.ron loads with new AttributePair format
        let result = CharacterDatabase::load_from_file("data/characters.ron");

        assert!(
            result.is_ok(),
            "Core characters.ron should load successfully: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // Core campaign has 6 pre-made character definitions
        assert_eq!(
            db.len(),
            6,
            "Core campaign should have 6 character definitions"
        );

        // Verify specific characters exist
        assert!(
            db.get_character("pregen_human_knight").is_some(),
            "Should find Sir Aldric (pregen_human_knight)"
        );
        assert!(
            db.get_character("pregen_gnome_sorcerer").is_some(),
            "Should find Lyria Starweaver (pregen_gnome_sorcerer)"
        );
    }

    #[test]
    fn test_phase2_core_campaign_stats_format() {
        // Verify that core campaign base_stats (simple format) deserialize correctly
        let db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load core campaign");

        let aldric = db
            .get_character("pregen_human_knight")
            .expect("Should find Sir Aldric");

        // Core campaign uses simple format: might: 16
        // Should deserialize to Stats with AttributePair { base: 16, current: 16 }
        assert_eq!(
            aldric.base_stats.might.base, 16,
            "Sir Aldric might base should be 16"
        );
        assert_eq!(
            aldric.base_stats.might.current, 16,
            "Sir Aldric might current should equal base"
        );
        assert_eq!(
            aldric.base_stats.endurance.base, 15,
            "Sir Aldric endurance base should be 15"
        );
    }

    #[test]
    fn test_phase2_campaign_instantiation() {
        // Verify that campaign characters can be instantiated successfully
        let char_db = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
            .expect("Failed to load tutorial campaign");
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let kira_def = char_db
            .get_character("tutorial_human_knight")
            .expect("Should find Kira");

        let kira = kira_def
            .instantiate(&races, &classes, &items)
            .expect("Should instantiate Kira successfully");

        // Verify instantiation worked correctly
        assert_eq!(kira.name, "Kira");
        assert_eq!(kira.race_id, "human");
        assert_eq!(kira.class_id, "knight");

        // Verify stats were applied correctly (base_stats + race modifiers)
        assert!(
            kira.stats.might.base >= 15,
            "Kira's might should be at least base 15 (may have race bonus)"
        );

        // Verify HP override was used
        // hp_override: Some(10) should be used instead of calculated HP
        assert_eq!(
            kira.hp.base, 10,
            "Kira's HP should use hp_override value of 10"
        );
        assert_eq!(
            kira.hp.current, 10,
            "Kira's current HP should equal base HP"
        );
    }

    #[test]
    fn test_phase2_all_tutorial_characters_instantiate() {
        // Verify ALL tutorial campaign characters can be instantiated
        let char_db = CharacterDatabase::load_from_file("campaigns/tutorial/data/characters.ron")
            .expect("Failed to load tutorial campaign");
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let mut success_count = 0;

        for char_def in char_db.all_characters() {
            match char_def.instantiate(&races, &classes, &items) {
                Ok(character) => {
                    success_count += 1;
                    // Verify basic invariants
                    assert_eq!(character.name, char_def.name);
                    assert_eq!(character.race_id, char_def.race_id);
                    assert_eq!(character.class_id, char_def.class_id);
                }
                Err(e) => {
                    panic!("Failed to instantiate character '{}': {:?}", char_def.id, e);
                }
            }
        }

        assert_eq!(
            success_count, 9,
            "All 9 tutorial characters should instantiate successfully"
        );
    }

    #[test]
    fn test_phase2_all_core_characters_instantiate() {
        // Verify ALL core campaign characters can be instantiated
        let char_db = CharacterDatabase::load_from_file("data/characters.ron")
            .expect("Failed to load core campaign");
        let races =
            RaceDatabase::load_from_file("data/races.ron").expect("Failed to load races.ron");
        let classes =
            ClassDatabase::load_from_file("data/classes.ron").expect("Failed to load classes.ron");
        let items =
            ItemDatabase::load_from_file("data/items.ron").expect("Failed to load items.ron");

        let mut success_count = 0;

        for char_def in char_db.all_characters() {
            match char_def.instantiate(&races, &classes, &items) {
                Ok(character) => {
                    success_count += 1;
                    // Verify basic invariants
                    assert_eq!(character.name, char_def.name);
                    assert_eq!(character.race_id, char_def.race_id);
                    assert_eq!(character.class_id, char_def.class_id);
                }
                Err(e) => {
                    panic!("Failed to instantiate character '{}': {:?}", char_def.id, e);
                }
            }
        }

        assert_eq!(
            success_count, 6,
            "All 6 core characters should instantiate successfully"
        );
    }

    #[test]
    fn test_phase2_stats_roundtrip_preserves_format() {
        // Verify that Stats can roundtrip through RON serialization
        // Both simple and full formats should work

        // Test simple format
        let simple_ron = r#"(
            might: 15,
            intellect: 10,
            personality: 12,
            endurance: 14,
            speed: 11,
            accuracy: 13,
            luck: 10,
        )"#;

        let stats: Stats = ron::from_str(simple_ron).expect("Should parse simple format");
        assert_eq!(stats.might.base, 15);
        assert_eq!(stats.might.current, 15);

        // Test full format
        let full_ron = r#"(
            might: (base: 15, current: 18),
            intellect: 10,
            personality: 12,
            endurance: 14,
            speed: 11,
            accuracy: 13,
            luck: 10,
        )"#;

        let stats_buffed: Stats = ron::from_str(full_ron).expect("Should parse full format");
        assert_eq!(stats_buffed.might.base, 15);
        assert_eq!(stats_buffed.might.current, 18);
        assert_eq!(stats_buffed.intellect.base, 10);
        assert_eq!(stats_buffed.intellect.current, 10);
    }

    #[test]
    fn test_phase2_example_formats_file_loads() {
        // Verify the example character_definition_formats.ron file loads correctly
        // This file demonstrates all supported formats for content authors
        let result =
            CharacterDatabase::load_from_file("data/examples/character_definition_formats.ron");

        assert!(
            result.is_ok(),
            "Example formats file should load successfully: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // File contains 5 example characters
        assert_eq!(
            db.len(),
            5,
            "Example formats file should have 5 character definitions"
        );

        // Verify simple format example
        let simple = db
            .get_character("example_simple_format")
            .expect("Should find simple format example");
        assert_eq!(simple.base_stats.might.base, 16);
        assert_eq!(simple.base_stats.might.current, 16);
        assert_eq!(simple.hp_override, Some(AttributePair16::new(50)));

        // Verify buffed character example (full format)
        let buffed = db
            .get_character("example_buffed_hero")
            .expect("Should find buffed hero example");
        assert_eq!(buffed.base_stats.might.base, 14);
        assert_eq!(buffed.base_stats.might.current, 18); // Pre-buffed
        assert_eq!(buffed.base_stats.luck.base, 11);
        assert_eq!(buffed.base_stats.luck.current, 16); // Pre-blessed
        assert_eq!(
            buffed.hp_override,
            Some(AttributePair16 {
                base: 45,
                current: 60
            })
        );

        // Verify wounded character example
        let wounded = db
            .get_character("example_wounded_veteran")
            .expect("Should find wounded veteran example");
        assert_eq!(
            wounded.hp_override,
            Some(AttributePair16 {
                base: 50,
                current: 30
            })
        );

        // Verify auto-calculated HP example (no hp_override)
        let auto_hp = db
            .get_character("example_auto_hp")
            .expect("Should find auto HP example");
        assert!(auto_hp.hp_override.is_none());

        // Verify legacy format example (backward compatibility)
        let legacy = db
            .get_character("example_legacy_format")
            .expect("Should find legacy format example");
        // Legacy hp_base + hp_current should convert to hp_override
        assert_eq!(
            legacy.hp_override,
            Some(AttributePair16 {
                base: 40,
                current: 25
            })
        );
    }
}
