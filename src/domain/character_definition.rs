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

use crate::domain::character::{Alignment, Sex, Stats};
use crate::domain::classes::ClassId;
use crate::domain::types::{ItemId, RaceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

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

// ===== Base Stats =====

/// Base statistics for a character definition
///
/// Represents the starting stat values before any race or class modifiers.
/// Values typically range from 3-18 for standard characters.
///
/// # Examples
///
/// ```
/// use antares::domain::character_definition::BaseStats;
///
/// let stats = BaseStats {
///     might: 14,
///     intellect: 10,
///     personality: 12,
///     endurance: 13,
///     speed: 11,
///     accuracy: 15,
///     luck: 10,
/// };
///
/// assert_eq!(stats.might, 14);
/// ```
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

impl BaseStats {
    /// Creates a new BaseStats with the specified values
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::BaseStats;
    ///
    /// let stats = BaseStats::new(14, 10, 12, 13, 11, 15, 10);
    /// assert_eq!(stats.might, 14);
    /// assert_eq!(stats.accuracy, 15);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character_definition::BaseStats;
    ///
    /// let base = BaseStats::new(14, 10, 12, 13, 11, 15, 10);
    /// let stats = base.to_stats();
    /// assert_eq!(stats.might.base, 14);
    /// assert_eq!(stats.might.current, 14);
    /// ```
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
/// use antares::domain::character_definition::{CharacterDefinition, BaseStats, StartingEquipment};
/// use antares::domain::character::{Sex, Alignment};
///
/// let knight = CharacterDefinition {
///     id: "pregen_human_knight".to_string(),
///     name: "Sir Galahad".to_string(),
///     race_id: "human".to_string(),
///     class_id: "knight".to_string(),
///     sex: Sex::Male,
///     alignment: Alignment::Good,
///     base_stats: BaseStats::new(16, 8, 10, 14, 12, 14, 10),
///     portrait_id: 1,
///     starting_gold: 100,
///     starting_gems: 0,
///     starting_food: 10,
///     starting_items: vec![],
///     starting_equipment: StartingEquipment::default(),
///     description: "A noble knight seeking glory.".to_string(),
///     is_premade: true,
/// };
///
/// assert_eq!(knight.name, "Sir Galahad");
/// assert!(knight.is_premade);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Base statistics before race/class modifiers
    pub base_stats: BaseStats,

    /// Portrait/avatar identifier
    #[serde(default)]
    pub portrait_id: u8,

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
}

/// Default starting food value (10 units)
fn default_starting_food() -> u8 {
    10
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
            base_stats: BaseStats::default(),
            portrait_id: 0,
            starting_gold: 0,
            starting_gems: 0,
            starting_food: 10,
            starting_items: Vec::new(),
            starting_equipment: StartingEquipment::new(),
            description: String::new(),
            is_premade: false,
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

        Ok(())
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
    ///         portrait_id: 1,
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

    // ===== BaseStats Tests =====

    #[test]
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
    fn test_base_stats_to_stats() {
        let base = BaseStats::new(14, 10, 12, 13, 11, 15, 10);
        let stats = base.to_stats();

        assert_eq!(stats.might.base, 14);
        assert_eq!(stats.might.current, 14);
        assert_eq!(stats.intellect.base, 10);
        assert_eq!(stats.accuracy.base, 15);
    }

    #[test]
    fn test_base_stats_serialization() {
        let stats = BaseStats::new(16, 8, 10, 14, 12, 14, 10);
        let serialized = ron::to_string(&stats).unwrap();
        let deserialized: BaseStats = ron::from_str(&serialized).unwrap();
        assert_eq!(stats, deserialized);
    }

    // ===== CharacterDefinition Tests =====

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
        def.base_stats = BaseStats::new(8, 16, 12, 10, 14, 10, 12);
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
                portrait_id: 1,
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
                portrait_id: 2,
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
        assert_eq!(knight.base_stats.might, 16);
        assert_eq!(knight.starting_gold, 100);
        assert_eq!(knight.starting_items.len(), 2);
        assert!(knight.starting_equipment.weapon.is_some());

        let sorcerer = db.get_character("pregen_sorcerer").unwrap();
        assert_eq!(sorcerer.name, "Merlin");
        assert_eq!(sorcerer.base_stats.intellect, 16);
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
}
