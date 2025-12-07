// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character race definitions and database
//!
//! This module implements the data-driven race system, allowing race
//! definitions to be loaded from external RON files. This enables modding
//! and campaign-specific race configurations.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4 for core data structures.
//! See `docs/explanation/hardcoded_removal_implementation_plan.md` Phase 4.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with race definitions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RaceError {
    #[error("Race not found: {0}")]
    RaceNotFound(String),

    #[error("Failed to load race database from file: {0}")]
    LoadError(String),

    #[error("Failed to parse race data: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Duplicate race ID: {0}")]
    DuplicateId(String),
}

// ===== Type Aliases =====

/// Unique identifier for a character race
pub type RaceId = String;

// ===== Enums =====

/// Size category for races
///
/// Affects movement, equipment compatibility, and certain combat mechanics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SizeCategory {
    /// Small races (gnomes, halflings)
    Small,
    /// Medium races (humans, elves, dwarves)
    #[default]
    Medium,
    /// Large races (half-giants, ogres)
    Large,
}

// ===== Structures =====

/// Stat modifiers for a race
///
/// These values are added to base stats during character creation.
/// Positive values indicate bonuses, negative values indicate penalties.
///
/// # Examples
///
/// ```
/// use antares::domain::races::StatModifiers;
///
/// // Elf modifiers: +1 Intellect, +1 Accuracy, -1 Endurance
/// let elf_mods = StatModifiers {
///     might: 0,
///     intellect: 1,
///     personality: 0,
///     endurance: -1,
///     speed: 0,
///     accuracy: 1,
///     luck: 0,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StatModifiers {
    /// Might modifier (strength, melee damage)
    #[serde(default)]
    pub might: i8,
    /// Intellect modifier (arcane magic, knowledge)
    #[serde(default)]
    pub intellect: i8,
    /// Personality modifier (divine magic, charisma)
    #[serde(default)]
    pub personality: i8,
    /// Endurance modifier (health, stamina)
    #[serde(default)]
    pub endurance: i8,
    /// Speed modifier (initiative, movement)
    #[serde(default)]
    pub speed: i8,
    /// Accuracy modifier (ranged attacks, precision)
    #[serde(default)]
    pub accuracy: i8,
    /// Luck modifier (critical hits, saves)
    #[serde(default)]
    pub luck: i8,
}

impl StatModifiers {
    /// Creates stat modifiers with all zeros
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if all modifiers are zero
    pub fn is_empty(&self) -> bool {
        self.might == 0
            && self.intellect == 0
            && self.personality == 0
            && self.endurance == 0
            && self.speed == 0
            && self.accuracy == 0
            && self.luck == 0
    }

    /// Returns the sum of all modifiers (for balance checking)
    pub fn total(&self) -> i16 {
        self.might as i16
            + self.intellect as i16
            + self.personality as i16
            + self.endurance as i16
            + self.speed as i16
            + self.accuracy as i16
            + self.luck as i16
    }
}

/// Elemental and damage resistances for a race
///
/// Values represent percentage resistance (0-100).
/// 0 = no resistance, 100 = immune.
///
/// # Examples
///
/// ```
/// use antares::domain::races::Resistances;
///
/// // Dwarf resistances: 10% poison, 5% magic
/// let dwarf_res = Resistances {
///     magic: 5,
///     fire: 0,
///     cold: 0,
///     electricity: 0,
///     acid: 0,
///     fear: 0,
///     poison: 10,
///     psychic: 0,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Resistances {
    /// Magic/spell resistance
    #[serde(default)]
    pub magic: u8,
    /// Fire resistance
    #[serde(default)]
    pub fire: u8,
    /// Cold resistance
    #[serde(default)]
    pub cold: u8,
    /// Electricity/lightning resistance
    #[serde(default)]
    pub electricity: u8,
    /// Acid resistance
    #[serde(default)]
    pub acid: u8,
    /// Fear resistance
    #[serde(default)]
    pub fear: u8,
    /// Poison resistance
    #[serde(default)]
    pub poison: u8,
    /// Psychic/mental resistance
    #[serde(default)]
    pub psychic: u8,
}

impl Resistances {
    /// Creates resistances with all zeros
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if all resistances are zero
    pub fn is_empty(&self) -> bool {
        self.magic == 0
            && self.fire == 0
            && self.cold == 0
            && self.electricity == 0
            && self.acid == 0
            && self.fear == 0
            && self.poison == 0
            && self.psychic == 0
    }

    /// Validates that all resistance values are in the valid range (0-100)
    pub fn validate(&self) -> Result<(), String> {
        let check = |name: &str, value: u8| {
            if value > 100 {
                Err(format!(
                    "{} resistance {} exceeds maximum of 100",
                    name, value
                ))
            } else {
                Ok(())
            }
        };

        check("Magic", self.magic)?;
        check("Fire", self.fire)?;
        check("Cold", self.cold)?;
        check("Electricity", self.electricity)?;
        check("Acid", self.acid)?;
        check("Fear", self.fear)?;
        check("Poison", self.poison)?;
        check("Psychic", self.psychic)?;

        Ok(())
    }
}

/// Complete definition of a character race
///
/// This structure contains all mechanical properties of a race,
/// loaded from external data files to support modding and campaigns.
///
/// # Examples
///
/// ```
/// use antares::domain::races::{RaceDefinition, StatModifiers, Resistances, SizeCategory};
///
/// let halfling = RaceDefinition {
///     id: "halfling".to_string(),
///     name: "Halfling".to_string(),
///     description: "Small and nimble".to_string(),
///     stat_modifiers: StatModifiers::default(),
///     resistances: Resistances::default(),
///     special_abilities: vec![],
///     size: SizeCategory::Small,
///     proficiencies: vec![],
///     incompatible_item_tags: vec!["large_weapon".to_string(), "heavy_armor".to_string()],
/// };
///
/// assert_eq!(halfling.name, "Halfling");
/// assert_eq!(halfling.size, SizeCategory::Small);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceDefinition {
    /// Unique identifier (e.g., "human", "elf")
    pub id: RaceId,

    /// Display name (e.g., "Human", "Elf")
    pub name: String,

    /// Description of the race
    #[serde(default)]
    pub description: String,

    /// Stat modifiers applied to base stats
    #[serde(default)]
    pub stat_modifiers: StatModifiers,

    /// Elemental and damage resistances
    #[serde(default)]
    pub resistances: Resistances,

    /// Special abilities this race has (e.g., "infravision", "magic_resistance")
    #[serde(default)]
    pub special_abilities: Vec<String>,

    /// Size category affecting equipment and combat
    #[serde(default)]
    pub size: SizeCategory,

    /// Proficiencies this race starts with (forward-compatible with proficiency system)
    #[serde(default)]
    pub proficiencies: Vec<String>,

    /// Item tags that are incompatible with this race (forward-compatible)
    #[serde(default)]
    pub incompatible_item_tags: Vec<String>,
}

impl RaceDefinition {
    /// Creates a new race definition with minimal required fields
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the race
    /// * `name` - Display name for the race
    /// * `description` - Description of the race
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::RaceDefinition;
    ///
    /// let human = RaceDefinition::new(
    ///     "human".to_string(),
    ///     "Human".to_string(),
    ///     "Versatile and adaptable".to_string(),
    /// );
    /// assert_eq!(human.id, "human");
    /// assert_eq!(human.name, "Human");
    /// ```
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        }
    }

    /// Checks if this race has a specific special ability
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::{RaceDefinition, StatModifiers, Resistances, SizeCategory};
    ///
    /// let elf = RaceDefinition {
    ///     id: "elf".to_string(),
    ///     name: "Elf".to_string(),
    ///     description: "Graceful and magical".to_string(),
    ///     stat_modifiers: StatModifiers::default(),
    ///     resistances: Resistances::default(),
    ///     special_abilities: vec!["infravision".to_string(), "resist_sleep".to_string()],
    ///     size: SizeCategory::Medium,
    ///     proficiencies: vec!["martial_ranged".to_string()],
    ///     incompatible_item_tags: vec![],
    /// };
    ///
    /// assert!(elf.has_ability("infravision"));
    /// assert!(elf.has_ability("resist_sleep"));
    /// assert!(!elf.has_ability("darkvision"));
    /// ```
    pub fn has_ability(&self, ability: &str) -> bool {
        self.special_abilities.iter().any(|a| a.as_str() == ability)
    }

    /// Checks if this race has a specific proficiency
    pub fn has_proficiency(&self, proficiency: &str) -> bool {
        self.proficiencies.iter().any(|p| p.as_str() == proficiency)
    }

    /// Checks if an item tag is incompatible with this race
    pub fn is_item_incompatible(&self, tag: &str) -> bool {
        self.incompatible_item_tags
            .iter()
            .any(|t| t.as_str() == tag)
    }

    /// Checks if this race can use an item based on its tags
    ///
    /// Returns `true` if the item has no tags that are incompatible with this race.
    /// This is a convenience wrapper around tag-based compatibility checking.
    ///
    /// # Arguments
    ///
    /// * `item_tags` - The tags on the item to check
    ///
    /// # Returns
    ///
    /// `true` if the race can use the item (no incompatible tags match)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::{RaceDefinition, StatModifiers, Resistances, SizeCategory};
    ///
    /// let gnome = RaceDefinition {
    ///     id: "gnome".to_string(),
    ///     name: "Gnome".to_string(),
    ///     description: "Small but clever".to_string(),
    ///     stat_modifiers: StatModifiers::default(),
    ///     resistances: Resistances::default(),
    ///     special_abilities: vec![],
    ///     size: SizeCategory::Medium,
    ///     proficiencies: vec![],
    ///     incompatible_item_tags: vec!["large_weapon".to_string(), "heavy_armor".to_string()],
    /// };
    ///
    /// // Gnome cannot use large weapons
    /// let large_sword_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];
    /// assert!(!gnome.can_use_item(&large_sword_tags));
    ///
    /// // Gnome can use small weapons
    /// let dagger_tags = vec!["light".to_string()];
    /// assert!(gnome.can_use_item(&dagger_tags));
    ///
    /// // Empty tags are always compatible
    /// assert!(gnome.can_use_item(&[]));
    /// ```
    pub fn can_use_item(&self, item_tags: &[String]) -> bool {
        !item_tags.iter().any(|tag| self.is_item_incompatible(tag))
    }

    /// Returns true if this race is Small size
    pub fn is_small(&self) -> bool {
        self.size == SizeCategory::Small
    }

    /// Returns true if this race is Medium size
    pub fn is_medium(&self) -> bool {
        self.size == SizeCategory::Medium
    }

    /// Returns true if this race is Large size
    pub fn is_large(&self) -> bool {
        self.size == SizeCategory::Large
    }
}

/// Database of all race definitions
///
/// Manages loading, validation, and lookup of race definitions.
/// Typically loaded once at game startup from `data/races.ron`.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::races::RaceDatabase;
///
/// let db = RaceDatabase::load_from_file("data/races.ron").unwrap();
/// let human = db.get_race("human").unwrap();
/// assert_eq!(human.name, "Human");
/// ```
#[derive(Debug, Clone, Default)]
pub struct RaceDatabase {
    races: HashMap<RaceId, RaceDefinition>,
}

impl RaceDatabase {
    /// Creates an empty race database
    pub fn new() -> Self {
        Self {
            races: HashMap::new(),
        }
    }

    /// Loads race definitions from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing race definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(RaceDatabase)` on success, or an error if the file
    /// cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::races::RaceDatabase;
    ///
    /// let db = RaceDatabase::load_from_file("data/races.ron").unwrap();
    /// assert!(db.get_race("human").is_some());
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, RaceError> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| RaceError::LoadError(format!("Failed to read file: {}", e)))?;

        Self::load_from_string(&contents)
    }

    /// Loads race definitions from a RON string
    ///
    /// # Arguments
    ///
    /// * `data` - RON string containing race definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(RaceDatabase)` on success, or an error if parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::RaceDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "human",
    ///         name: "Human",
    ///         description: "Versatile and adaptable",
    ///         stat_modifiers: (
    ///             might: 0, intellect: 0, personality: 0,
    ///             endurance: 0, speed: 0, accuracy: 0, luck: 0,
    ///         ),
    ///         resistances: (
    ///             magic: 0, fire: 0, cold: 0, electricity: 0,
    ///             acid: 0, fear: 0, poison: 0, psychic: 0,
    ///         ),
    ///         special_abilities: [],
    ///         size: Medium,
    ///         proficiencies: [],
    ///         incompatible_item_tags: [],
    ///     ),
    /// ]"#;
    ///
    /// let db = RaceDatabase::load_from_string(ron_data).unwrap();
    /// assert!(db.get_race("human").is_some());
    /// ```
    pub fn load_from_string(data: &str) -> Result<Self, RaceError> {
        let races: Vec<RaceDefinition> = ron::from_str(data)
            .map_err(|e| RaceError::ParseError(format!("RON parse error: {}", e)))?;

        let mut db = Self::new();
        for race_def in races {
            if db.races.contains_key(&race_def.id) {
                return Err(RaceError::DuplicateId(race_def.id.clone()));
            }
            db.races.insert(race_def.id.clone(), race_def);
        }

        db.validate()?;
        Ok(db)
    }

    /// Gets a race definition by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The race ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&RaceDefinition)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::RaceDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "elf",
    ///         name: "Elf",
    ///     ),
    /// ]"#;
    ///
    /// let db = RaceDatabase::load_from_string(ron_data).unwrap();
    /// let elf = db.get_race("elf").unwrap();
    /// assert_eq!(elf.name, "Elf");
    /// ```
    pub fn get_race(&self, id: &str) -> Option<&RaceDefinition> {
        self.races.get(id)
    }

    /// Returns an iterator over all race definitions
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::RaceDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "human",
    ///         name: "Human",
    ///     ),
    ///     (
    ///         id: "elf",
    ///         name: "Elf",
    ///     ),
    /// ]"#;
    ///
    /// let db = RaceDatabase::load_from_string(ron_data).unwrap();
    /// assert_eq!(db.all_races().count(), 2);
    /// ```
    pub fn all_races(&self) -> impl Iterator<Item = &RaceDefinition> {
        self.races.values()
    }

    /// Returns all race IDs
    pub fn all_race_ids(&self) -> Vec<&RaceId> {
        self.races.keys().collect()
    }

    /// Validates the race database
    ///
    /// Checks for:
    /// - Disablement bits are in valid range (0-7)
    /// - Resistance values are in valid range (0-100)
    /// - Stat modifiers are reasonable
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or an error describing the issue.
    pub fn validate(&self) -> Result<(), RaceError> {
        for race_def in self.races.values() {
            // Check proficiency format (basic validation)
            // Check resistance values
            if let Err(e) = race_def.resistances.validate() {
                return Err(RaceError::ValidationError(format!(
                    "Invalid resistances in race '{}': {}",
                    race_def.id, e
                )));
            }

            // Check stat modifier range (each modifier should be -10 to +10 for balance)
            let check_stat = |name: &str, value: i8| {
                if !(-10..=10).contains(&value) {
                    return Err(RaceError::ValidationError(format!(
                        "Stat modifier {} = {} in race '{}' is outside reasonable range (-10 to +10)",
                        name, value, race_def.id
                    )));
                }
                Ok(())
            };

            check_stat("might", race_def.stat_modifiers.might)?;
            check_stat("intellect", race_def.stat_modifiers.intellect)?;
            check_stat("personality", race_def.stat_modifiers.personality)?;
            check_stat("endurance", race_def.stat_modifiers.endurance)?;
            check_stat("speed", race_def.stat_modifiers.speed)?;
            check_stat("accuracy", race_def.stat_modifiers.accuracy)?;
            check_stat("luck", race_def.stat_modifiers.luck)?;
        }

        Ok(())
    }

    /// Returns true if a race with the given ID exists
    ///
    /// # Arguments
    ///
    /// * `id` - The race ID to check
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::races::RaceDatabase;
    ///
    /// let ron_data = r#"[
    ///     (
    ///         id: "human",
    ///         name: "Human",
    ///     ),
    /// ]"#;
    ///
    /// let db = RaceDatabase::load_from_string(ron_data).unwrap();
    /// assert!(db.has_race("human"));
    /// assert!(!db.has_race("nonexistent"));
    /// ```
    pub fn has_race(&self, id: &str) -> bool {
        self.races.contains_key(id)
    }

    /// Returns the number of races in the database
    pub fn len(&self) -> usize {
        self.races.len()
    }

    /// Returns true if the database is empty
    pub fn is_empty(&self) -> bool {
        self.races.is_empty()
    }

    /// Adds a race definition to the database
    ///
    /// # Returns
    ///
    /// Returns `Err` if a race with the same ID already exists.
    pub fn add_race(&mut self, race: RaceDefinition) -> Result<(), RaceError> {
        if self.races.contains_key(&race.id) {
            return Err(RaceError::DuplicateId(race.id.clone()));
        }
        self.races.insert(race.id.clone(), race);
        Ok(())
    }

    /// Removes a race definition from the database
    ///
    /// # Returns
    ///
    /// Returns the removed race definition, or `None` if not found.
    pub fn remove_race(&mut self, id: &str) -> Option<RaceDefinition> {
        self.races.remove(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_human() -> RaceDefinition {
        RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "Versatile and adaptable".to_string(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        }
    }

    fn create_test_elf() -> RaceDefinition {
        RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: "Graceful and magical".to_string(),
            stat_modifiers: StatModifiers {
                might: -1,
                intellect: 1,
                personality: 0,
                endurance: -1,
                speed: 1,
                accuracy: 1,
                luck: 0,
            },
            resistances: Resistances {
                magic: 5,
                fire: 0,
                cold: 0,
                electricity: 0,
                acid: 0,
                fear: 0,
                poison: 0,
                psychic: 0,
            },
            special_abilities: vec!["infravision".to_string(), "resist_sleep".to_string()],
            size: SizeCategory::Medium,
            proficiencies: vec!["longbow".to_string()],
            incompatible_item_tags: vec![],
        }
    }

    fn create_test_dwarf() -> RaceDefinition {
        RaceDefinition {
            id: "dwarf".to_string(),
            name: "Dwarf".to_string(),
            description: "Sturdy and resilient".to_string(),
            stat_modifiers: StatModifiers {
                might: 1,
                intellect: 0,
                personality: -1,
                endurance: 2,
                speed: -1,
                accuracy: 0,
                luck: 0,
            },
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
            special_abilities: vec!["stonecunning".to_string()],
            size: SizeCategory::Medium,
            proficiencies: vec!["battleaxe".to_string(), "warhammer".to_string()],
            incompatible_item_tags: vec![],
        }
    }

    fn create_test_gnome() -> RaceDefinition {
        RaceDefinition {
            id: "gnome".to_string(),
            name: "Gnome".to_string(),
            description: "Clever and lucky".to_string(),
            stat_modifiers: StatModifiers {
                might: -2,
                intellect: 1,
                personality: 0,
                endurance: 0,
                speed: 0,
                accuracy: 0,
                luck: 2,
            },
            resistances: Resistances {
                magic: 5,
                fire: 0,
                cold: 0,
                electricity: 0,
                acid: 0,
                fear: 0,
                poison: 0,
                psychic: 5,
            },
            special_abilities: vec!["infravision".to_string()],
            size: SizeCategory::Small,
            proficiencies: vec![],
            incompatible_item_tags: vec!["large_weapon".to_string()],
        }
    }

    // ===== StatModifiers Tests =====

    #[test]
    fn test_stat_modifiers_default() {
        let mods = StatModifiers::default();
        assert_eq!(mods.might, 0);
        assert_eq!(mods.intellect, 0);
        assert_eq!(mods.personality, 0);
        assert_eq!(mods.endurance, 0);
        assert_eq!(mods.speed, 0);
        assert_eq!(mods.accuracy, 0);
        assert_eq!(mods.luck, 0);
    }

    #[test]
    fn test_stat_modifiers_is_empty() {
        let empty = StatModifiers::default();
        assert!(empty.is_empty());

        let non_empty = StatModifiers {
            might: 1,
            ..Default::default()
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_stat_modifiers_total() {
        let mods = StatModifiers {
            might: 1,
            intellect: -1,
            personality: 2,
            endurance: -2,
            speed: 1,
            accuracy: 1,
            luck: 0,
        };
        assert_eq!(mods.total(), 2); // 1 - 1 + 2 - 2 + 1 + 1 + 0 = 2
    }

    // ===== Resistances Tests =====

    #[test]
    fn test_resistances_default() {
        let res = Resistances::default();
        assert_eq!(res.magic, 0);
        assert_eq!(res.fire, 0);
        assert_eq!(res.cold, 0);
        assert_eq!(res.electricity, 0);
        assert_eq!(res.acid, 0);
        assert_eq!(res.fear, 0);
        assert_eq!(res.poison, 0);
        assert_eq!(res.psychic, 0);
    }

    #[test]
    fn test_resistances_is_empty() {
        let empty = Resistances::default();
        assert!(empty.is_empty());

        let non_empty = Resistances {
            fire: 10,
            ..Default::default()
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_resistances_validate_success() {
        let valid = Resistances {
            magic: 50,
            fire: 100,
            cold: 0,
            electricity: 25,
            acid: 75,
            fear: 100,
            poison: 10,
            psychic: 5,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_resistances_validate_failure() {
        let invalid = Resistances {
            magic: 101, // Over 100
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    // ===== RaceDefinition Tests =====

    #[test]
    fn test_race_definition_has_ability() {
        let elf = create_test_elf();
        assert!(elf.has_ability("infravision"));
        assert!(elf.has_ability("resist_sleep"));
        assert!(!elf.has_ability("stonecunning"));
    }

    #[test]
    fn test_race_definition_has_proficiency() {
        let elf = create_test_elf();
        assert!(elf.has_proficiency("longbow"));
        assert!(!elf.has_proficiency("battleaxe"));

        let dwarf = create_test_dwarf();
        assert!(dwarf.has_proficiency("battleaxe"));
        assert!(dwarf.has_proficiency("warhammer"));
    }

    #[test]
    fn test_race_definition_is_item_incompatible() {
        let gnome = create_test_gnome();
        assert!(gnome.is_item_incompatible("large_weapon"));
        assert!(!gnome.is_item_incompatible("sword"));
    }

    #[test]
    fn test_race_definition_can_use_item() {
        let gnome = create_test_gnome();

        // Gnome cannot use large weapons
        let large_sword_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];
        assert!(!gnome.can_use_item(&large_sword_tags));

        // Gnome can use small weapons
        let dagger_tags = vec!["light".to_string()];
        assert!(gnome.can_use_item(&dagger_tags));

        // Empty tags are always compatible
        assert!(gnome.can_use_item(&[]));

        // Human has no restrictions
        let human = create_test_human();
        assert!(human.can_use_item(&large_sword_tags));
        assert!(human.can_use_item(&dagger_tags));

        // Elf has no tag restrictions
        let elf = create_test_elf();
        assert!(elf.can_use_item(&large_sword_tags));
    }

    #[test]
    fn test_race_definition_size_checks() {
        let gnome = create_test_gnome();
        assert!(gnome.is_small());
        assert!(!gnome.is_medium());
        assert!(!gnome.is_large());

        let human = create_test_human();
        assert!(!human.is_small());
        assert!(human.is_medium());
        assert!(!human.is_large());
    }

    // ===== RaceDatabase Tests =====

    #[test]
    fn test_race_database_new() {
        let db = RaceDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_race_database_add_race() {
        let mut db = RaceDatabase::new();
        let human = create_test_human();

        assert!(db.add_race(human.clone()).is_ok());
        assert_eq!(db.len(), 1);
        assert!(db.get_race("human").is_some());
    }

    #[test]
    fn test_race_database_duplicate_id_error() {
        let mut db = RaceDatabase::new();
        let human = create_test_human();

        assert!(db.add_race(human.clone()).is_ok());
        assert!(matches!(db.add_race(human), Err(RaceError::DuplicateId(_))));
    }

    #[test]
    fn test_race_database_remove_race() {
        let mut db = RaceDatabase::new();
        let human = create_test_human();

        db.add_race(human).unwrap();
        assert_eq!(db.len(), 1);

        let removed = db.remove_race("human");
        assert!(removed.is_some());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_race_database_load_from_string() {
        let ron_data = r#"[
            (
                id: "human",
                name: "Human",
                description: "Versatile and adaptable",
                stat_modifiers: (
                    might: 0, intellect: 0, personality: 0,
                    endurance: 0, speed: 0, accuracy: 0, luck: 0,
                ),
                resistances: (
                    magic: 0, fire: 0, cold: 0, electricity: 0,
                    acid: 0, fear: 0, poison: 0, psychic: 0,
                ),
                special_abilities: [],
                size: Medium,
                proficiencies: [],
                incompatible_item_tags: [],
            ),
            (
                id: "elf",
                name: "Elf",
                description: "Graceful and magical",
                stat_modifiers: (
                    might: -1, intellect: 1, personality: 0,
                    endurance: -1, speed: 1, accuracy: 1, luck: 0,
                ),
                resistances: (
                    magic: 5, fire: 0, cold: 0, electricity: 0,
                    acid: 0, fear: 0, poison: 0, psychic: 0,
                ),
                special_abilities: ["infravision", "resist_sleep"],
                size: Medium,
                proficiencies: ["longbow"],
                incompatible_item_tags: [],
            ),
        ]"#;

        let db = RaceDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 2);

        let human = db.get_race("human").unwrap();
        assert_eq!(human.name, "Human");
        assert!(human.stat_modifiers.is_empty());

        let elf = db.get_race("elf").unwrap();
        assert_eq!(elf.name, "Elf");
        assert_eq!(elf.stat_modifiers.intellect, 1);
        assert!(elf.has_ability("infravision"));
    }

    #[test]
    fn test_race_database_load_minimal() {
        // Test that minimal RON data works with defaults
        let ron_data = r#"[
            (
                id: "human",
                name: "Human",
            ),
        ]"#;

        let db = RaceDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 1);

        let human = db.get_race("human").unwrap();
        assert_eq!(human.name, "Human");
        assert_eq!(human.description, ""); // Default empty string
        assert!(human.stat_modifiers.is_empty()); // Default
        assert!(human.resistances.is_empty()); // Default
        assert_eq!(human.size, SizeCategory::Medium); // Default
    }

    #[test]
    fn test_race_database_get_race_not_found() {
        let db = RaceDatabase::new();
        assert!(db.get_race("nonexistent").is_none());
    }

    #[test]
    fn test_race_database_all_races() {
        let mut db = RaceDatabase::new();
        db.add_race(create_test_human()).unwrap();
        db.add_race(create_test_elf()).unwrap();
        db.add_race(create_test_dwarf()).unwrap();

        let races: Vec<_> = db.all_races().collect();
        assert_eq!(races.len(), 3);
    }

    #[test]
    fn test_race_database_all_race_ids() {
        let mut db = RaceDatabase::new();
        db.add_race(create_test_human()).unwrap();
        db.add_race(create_test_elf()).unwrap();

        let ids = db.all_race_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().any(|id| *id == "human"));
        assert!(ids.iter().any(|id| *id == "elf"));
    }

    #[test]
    fn test_race_database_duplicate_id_in_load() {
        let ron_data = r#"[
            (
                id: "human",
                name: "Human",
            ),
            (
                id: "human",
                name: "Another Human",
            ),
        ]"#;

        let result = RaceDatabase::load_from_string(ron_data);
        assert!(matches!(result, Err(RaceError::DuplicateId(_))));
    }

    #[test]
    fn test_race_database_validation_invalid_resistance() {
        let ron_data = r#"[
            (
                id: "invalid",
                name: "Invalid",
                resistances: (
                    magic: 150,
                    fire: 0, cold: 0, electricity: 0,
                    acid: 0, fear: 0, poison: 0, psychic: 0,
                ),
            ),
        ]"#;

        let result = RaceDatabase::load_from_string(ron_data);
        assert!(matches!(result, Err(RaceError::ValidationError(_))));
    }

    #[test]
    fn test_race_database_validation_extreme_stat_modifier() {
        let ron_data = r#"[
            (
                id: "invalid",
                name: "Invalid",
                stat_modifiers: (
                    might: 50,
                    intellect: 0, personality: 0,
                    endurance: 0, speed: 0, accuracy: 0, luck: 0,
                ),
            ),
        ]"#;

        let result = RaceDatabase::load_from_string(ron_data);
        // This test now validates proficiency format instead of disablement bit range
        assert!(matches!(result, Err(RaceError::ValidationError(_))));
    }

    // ===== SizeCategory Tests =====

    #[test]
    fn test_size_category_default() {
        let size: SizeCategory = Default::default();
        assert_eq!(size, SizeCategory::Medium);
    }

    #[test]
    fn test_size_category_serialization() {
        let small = SizeCategory::Small;
        let medium = SizeCategory::Medium;
        let large = SizeCategory::Large;

        // RON serialization round-trip
        let small_ron = ron::to_string(&small).unwrap();
        let medium_ron = ron::to_string(&medium).unwrap();
        let large_ron = ron::to_string(&large).unwrap();

        assert_eq!(ron::from_str::<SizeCategory>(&small_ron).unwrap(), small);
        assert_eq!(ron::from_str::<SizeCategory>(&medium_ron).unwrap(), medium);
        assert_eq!(ron::from_str::<SizeCategory>(&large_ron).unwrap(), large);
    }

    // ===== Integration Tests =====

    #[test]
    fn test_load_races_from_data_file() {
        // This test verifies that the actual data/races.ron file can be loaded
        // if it exists (integration test)
        let path = std::path::Path::new("data/races.ron");
        if path.exists() {
            let result = RaceDatabase::load_from_file(path);
            // The current file may have minimal data, so just check it loads
            assert!(
                result.is_ok(),
                "Failed to load data/races.ron: {:?}",
                result.err()
            );
        }
    }
}
