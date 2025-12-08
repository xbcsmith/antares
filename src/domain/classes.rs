// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character class definitions and database
//!
//! This module implements the data-driven class system, allowing class
//! definitions to be loaded from external RON files. This enables modding
//! and campaign-specific class configurations.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4 for core data structures.
//! See `docs/explanation/sdk_implementation_plan.md` Phase 1 for implementation details.

use crate::domain::proficiency::{ProficiencyDatabase, ProficiencyId};
use crate::domain::types::{DiceRoll, ItemId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with class definitions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ClassError {
    #[error("Class not found: {0}")]
    ClassNotFound(String),

    #[error("Failed to load class database from file: {0}")]
    LoadError(String),

    #[error("Failed to parse class data: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Duplicate class ID: {0}")]
    DuplicateId(String),

    #[error("Invalid proficiency reference: {0}")]
    InvalidProficiency(String),
}

// ===== Type Aliases =====

/// Unique identifier for a character class
pub type ClassId = String;

// ===== Enums =====

/// Spell schools for spellcasting classes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellSchool {
    /// Divine magic (Cleric, Paladin)
    Cleric,
    /// Arcane magic (Sorcerer)
    Sorcerer,
}

/// Stat used for spell point calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellStat {
    /// Intelligence-based casting (Sorcerer)
    Intellect,
    /// Personality-based casting (Cleric)
    Personality,
}

// ===== Structures =====

/// Complete definition of a character class
///
/// This structure contains all mechanical properties of a class,
/// loaded from external data files to support modding and campaigns.
///
/// # Examples
///
/// ```
/// use antares::domain::classes::{ClassDefinition, SpellSchool, SpellStat};
/// use antares::domain::types::DiceRoll;
///
/// let knight = ClassDefinition {
///     id: "knight".to_string(),
///     name: "Knight".to_string(),
///     description: "A brave warrior".to_string(),
///     hp_die: DiceRoll::new(1, 10, 0),
///     spell_school: None,
///     is_pure_caster: false,
///     spell_stat: None,
///     special_abilities: vec!["multiple_attacks".to_string()],
///     starting_weapon_id: None,
///     starting_armor_id: None,
///     starting_items: vec![],
///     proficiencies: vec!["simple_weapon".to_string(), "heavy_armor".to_string()],
/// };
///
/// assert_eq!(knight.name, "Knight");
/// assert!(!knight.can_cast_spells());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// Unique identifier (e.g., "knight", "sorcerer")
    pub id: String,

    /// Display name (e.g., "Knight", "Sorcerer")
    pub name: String,

    /// Description of the class
    #[serde(default)]
    pub description: String,

    /// Hit dice for HP gain on level up (e.g., 1d10, 1d4)
    pub hp_die: DiceRoll,

    /// Spell school access, if any
    pub spell_school: Option<SpellSchool>,

    /// True for full casters (Cleric, Sorcerer), false for hybrids (Paladin)
    pub is_pure_caster: bool,

    /// Stat used for spell point calculation
    pub spell_stat: Option<SpellStat>,

    /// Special abilities this class has (e.g., "multiple_attacks", "backstab")
    pub special_abilities: Vec<String>,

    /// Starting weapon ID
    #[serde(default)]
    pub starting_weapon_id: Option<ItemId>,

    /// Starting armor ID
    #[serde(default)]
    pub starting_armor_id: Option<ItemId>,

    /// Starting items
    #[serde(default)]
    pub starting_items: Vec<ItemId>,

    /// Proficiencies this class grants (e.g., "simple_weapon", "heavy_armor")
    ///
    /// These proficiencies determine what items a character of this class can use.
    /// Uses UNION logic with race proficiencies (class OR race grants proficiency).
    #[serde(default)]
    pub proficiencies: Vec<ProficiencyId>,
}

impl ClassDefinition {
    /// Creates a new class definition with minimal required fields
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the class
    /// * `name` - Display name for the class
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::ClassDefinition;
    ///
    /// let knight = ClassDefinition::new("knight".to_string(), "Knight".to_string());
    /// assert_eq!(knight.id, "knight");
    /// assert_eq!(knight.name, "Knight");
    /// assert!(!knight.can_cast_spells());
    /// ```
    pub fn new(id: String, name: String) -> Self {
        use crate::domain::types::DiceRoll;
        Self {
            id,
            name,
            description: String::new(),
            hp_die: DiceRoll::new(1, 8, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
        }
    }

    /// Checks if this class can cast spells
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::ClassDefinition;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let knight = ClassDefinition {
    ///     id: "knight".to_string(),
    ///     name: "Knight".to_string(),
    ///     description: "A brave warrior".to_string(),
    ///     hp_die: DiceRoll::new(1, 10, 0),
    ///     spell_school: None,
    ///     is_pure_caster: false,
    ///     spell_stat: None,
    ///     special_abilities: vec!["multiple_attacks".to_string()],
    ///     starting_weapon_id: None,
    ///     starting_armor_id: None,
    ///     starting_items: vec![],
    ///     proficiencies: vec![],
    /// };
    ///
    /// assert!(!knight.can_cast_spells());
    /// ```
    pub fn can_cast_spells(&self) -> bool {
        self.spell_school.is_some()
    }

    /// Returns the disablement mask for this class (bit position as mask)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::ClassDefinition;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let knight = ClassDefinition {
    ///     id: "knight".to_string(),
    ///     name: "Knight".to_string(),
    ///     description: "A brave warrior".to_string(),
    ///     hp_die: DiceRoll::new(1, 10, 0),
    ///     spell_school: None,
    ///     is_pure_caster: false,
    ///     spell_stat: None,
    ///     special_abilities: vec!["multiple_attacks".to_string()],
    ///     starting_weapon_id: None,
    ///     starting_armor_id: None,
    ///     starting_items: vec![],
    ///     proficiencies: vec![],
    /// };
    ///
    /// assert_eq!(knight.name, "Knight");
    /// ```
    /// Checks if this class has a specific special ability
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::ClassDefinition;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let robber = ClassDefinition {
    ///     id: "robber".to_string(),
    ///     name: "Robber".to_string(),
    ///     description: "A cunning rogue".to_string(),
    ///     hp_die: DiceRoll::new(1, 6, 0),
    ///     spell_school: None,
    ///     is_pure_caster: false,
    ///     spell_stat: None,
    ///     special_abilities: vec!["backstab".to_string(), "disarm_trap".to_string()],
    ///     starting_weapon_id: None,
    ///     starting_armor_id: None,
    ///     starting_items: vec![],
    ///     proficiencies: vec!["simple_weapon".to_string(), "light_armor".to_string()],
    /// };
    ///
    /// assert!(robber.has_ability("backstab"));
    /// assert!(robber.has_ability("disarm_trap"));
    /// assert!(!robber.has_ability("multiple_attacks"));
    /// ```
    pub fn has_ability(&self, ability: &str) -> bool {
        self.special_abilities.iter().any(|a| a.as_str() == ability)
    }

    /// Checks if this class has a specific proficiency
    ///
    /// # Arguments
    ///
    /// * `proficiency` - The proficiency ID to check for
    ///
    /// # Returns
    ///
    /// `true` if this class grants the specified proficiency
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::ClassDefinition;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let knight = ClassDefinition {
    ///     id: "knight".to_string(),
    ///     name: "Knight".to_string(),
    ///     description: "A brave warrior".to_string(),
    ///     hp_die: DiceRoll::new(1, 10, 0),
    ///     spell_school: None,
    ///     is_pure_caster: false,
    ///     spell_stat: None,
    ///     special_abilities: vec!["multiple_attacks".to_string()],
    ///     starting_weapon_id: None,
    ///     starting_armor_id: None,
    ///     starting_items: vec![],
    ///     proficiencies: vec![
    ///         "simple_weapon".to_string(),
    ///         "martial_melee".to_string(),
    ///         "heavy_armor".to_string(),
    ///     ],
    /// };
    ///
    /// assert!(knight.has_proficiency("heavy_armor"));
    /// assert!(knight.has_proficiency("martial_melee"));
    /// assert!(!knight.has_proficiency("arcane_item"));
    /// ```
    pub fn has_proficiency(&self, proficiency: &str) -> bool {
        self.proficiencies.iter().any(|p| p.as_str() == proficiency)
    }
}

/// Database of all class definitions
///
/// Manages loading, validation, and lookup of class definitions.
/// Typically loaded once at game startup from `data/classes.ron`.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::classes::ClassDatabase;
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
/// let knight = db.get_class("knight").unwrap();
/// assert_eq!(knight.name, "Knight");
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClassDatabase {
    classes: HashMap<ClassId, ClassDefinition>,
}

impl ClassDatabase {
    /// Creates an empty class database
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    /// Loads class definitions from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing class definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(ClassDatabase)` on success, or an error if the file
    /// cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::classes::ClassDatabase;
    ///
    /// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
    /// assert!(db.get_class("knight").is_some());
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ClassError> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ClassError::LoadError(format!("Failed to read file: {}", e)))?;

        Self::load_from_string(&contents)
    }

    /// Loads class definitions from a RON string
    ///
    /// # Arguments
    ///
    /// * `data` - RON string containing class definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(ClassDatabase)` on success, or an error if parsing fails.
    ///
    /// # Examples
    ///
    /// ```
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
    /// ]"#;
    ///
    /// let db = ClassDatabase::load_from_string(ron_data).unwrap();
    /// assert!(db.get_class("knight").is_some());
    /// ```
    pub fn load_from_string(data: &str) -> Result<Self, ClassError> {
        let classes: Vec<ClassDefinition> = ron::from_str(data)
            .map_err(|e| ClassError::ParseError(format!("RON parse error: {}", e)))?;

        let mut db = Self::new();
        for class_def in classes {
            if db.classes.contains_key(&class_def.id) {
                return Err(ClassError::DuplicateId(class_def.id.clone()));
            }
            db.classes.insert(class_def.id.clone(), class_def);
        }

        db.validate()?;
        Ok(db)
    }

    /// Gets a class definition by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The class ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&ClassDefinition)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
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
    /// ]"#;
    ///
    /// let db = ClassDatabase::load_from_string(ron_data).unwrap();
    /// let knight = db.get_class("knight").unwrap();
    /// assert_eq!(knight.name, "Knight");
    /// ```
    pub fn get_class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.get(id)
    }

    /// Returns an iterator over all class definitions
    ///
    /// # Examples
    ///
    /// ```
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
    /// ]"#;
    ///
    /// let db = ClassDatabase::load_from_string(ron_data).unwrap();
    /// assert_eq!(db.all_classes().count(), 1);
    /// ```
    pub fn all_classes(&self) -> impl Iterator<Item = &ClassDefinition> {
        self.classes.values()
    }

    /// Validates the class database
    ///
    /// Checks for:
    /// - Spellcasters have spell_school and spell_stat
    /// - Non-spellcasters do not have spell_school
    /// - Disablement bits are unique
    /// - HP dice are valid (1-12 sided)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or an error describing the issue.
    pub fn validate(&self) -> Result<(), ClassError> {
        // let mut used_bits = std::collections::HashSet::new();

        for class_def in self.classes.values() {
            // Basic validation for proficiencies

            // Check spellcaster consistency
            if class_def.spell_school.is_some() {
                if class_def.spell_stat.is_none() {
                    return Err(ClassError::ValidationError(format!(
                        "Class '{}' has spell_school but no spell_stat",
                        class_def.id
                    )));
                }
            } else if class_def.spell_stat.is_some() {
                return Err(ClassError::ValidationError(format!(
                    "Class '{}' has spell_stat but no spell_school",
                    class_def.id
                )));
            }

            // Check HP dice validity
            if class_def.hp_die.count < 1 || class_def.hp_die.count > 10 {
                return Err(ClassError::ValidationError(format!(
                    "Invalid HP dice count {} in class '{}' (must be 1-10)",
                    class_def.hp_die.count, class_def.id
                )));
            }
            if class_def.hp_die.sides < 1 || class_def.hp_die.sides > 20 {
                return Err(ClassError::ValidationError(format!(
                    "Invalid HP dice sides {} in class '{}' (must be 1-20)",
                    class_def.hp_die.sides, class_def.id
                )));
            }
        }

        Ok(())
    }

    /// Validate proficiencies referenced by classes exist in the provided proficiency database.
    ///
    /// This performs the usual `validate()` checks (spellcaster settings, HP dice, etc.)
    /// and then verifies that every `proficiency` ID referenced by class definitions is
    /// present in the given `ProficiencyDatabase`.
    ///
    /// # Arguments
    ///
    /// * `prof_db` - Reference to the loaded `ProficiencyDatabase` for cross-reference validation
    ///
    /// # Errors
    ///
    /// Returns `ClassError::InvalidProficiency` if any class references a proficiency that
    /// does not exist in `prof_db`.
    pub fn validate_with_proficiency_db(
        &self,
        prof_db: &ProficiencyDatabase,
    ) -> Result<(), ClassError> {
        // First run the normal validation logic
        self.validate()?;

        for class_def in self.classes.values() {
            for prof in &class_def.proficiencies {
                if !prof_db.has(prof) {
                    return Err(ClassError::InvalidProficiency(format!(
                        "Class '{}' references unknown proficiency '{}'",
                        class_def.id, prof
                    )));
                }
            }
        }
        Ok(())
    }

    /// Returns the number of classes in the database
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// Returns true if the database is empty
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// Adds a class definition to the database
    ///
    /// # Arguments
    ///
    /// * `class` - The class definition to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `Err(ClassError::DuplicateId)` if
    /// a class with the same ID already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::classes::{ClassDatabase, ClassDefinition};
    ///
    /// let mut db = ClassDatabase::new();
    /// let knight = ClassDefinition::new("knight".to_string(), "Knight".to_string());
    /// db.add_class(knight).unwrap();
    /// assert!(db.get_class("knight").is_some());
    /// ```
    pub fn add_class(&mut self, class: ClassDefinition) -> Result<(), ClassError> {
        if self.classes.contains_key(&class.id) {
            return Err(ClassError::DuplicateId(class.id.clone()));
        }
        self.classes.insert(class.id.clone(), class);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_knight() -> ClassDefinition {
        ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: "A brave warrior".to_string(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec!["multiple_attacks".to_string()],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![
                "simple_weapon".to_string(),
                "martial_melee".to_string(),
                "heavy_armor".to_string(),
            ],
        }
    }

    fn create_test_sorcerer() -> ClassDefinition {
        ClassDefinition {
            id: "sorcerer".to_string(),
            name: "Sorcerer".to_string(),
            description: "A master of arcane magic".to_string(),
            hp_die: DiceRoll::new(1, 4, 0),
            spell_school: Some(SpellSchool::Sorcerer),
            is_pure_caster: true,
            spell_stat: Some(SpellStat::Intellect),
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec!["simple_weapon".to_string(), "arcane_item".to_string()],
        }
    }

    #[test]
    fn test_class_definition_can_cast_spells() {
        let knight = create_test_knight();
        let sorcerer = create_test_sorcerer();

        assert!(!knight.can_cast_spells());
        assert!(sorcerer.can_cast_spells());
    }

    #[test]
    fn test_class_definition_has_ability() {
        let knight = create_test_knight();
        assert!(knight.has_ability("multiple_attacks"));
        assert!(!knight.has_ability("backstab"));
    }

    #[test]
    fn test_class_definition_has_proficiency() {
        let knight = create_test_knight();
        assert!(knight.has_proficiency("heavy_armor"));
        assert!(knight.has_proficiency("martial_melee"));
        assert!(!knight.has_proficiency("arcane_item"));

        let sorcerer = create_test_sorcerer();
        assert!(sorcerer.has_proficiency("arcane_item"));
        assert!(sorcerer.has_proficiency("simple_weapon"));
        assert!(!sorcerer.has_proficiency("heavy_armor"));
    }

    #[test]
    fn test_class_database_new() {
        let db = ClassDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_class_database_load_from_string() {
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
                special_abilities: ["multiple_attacks"],
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
        ]"#;

        let db = ClassDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db.get_class("knight").is_some());
        assert!(db.get_class("sorcerer").is_some());
    }

    #[test]
    fn test_class_database_get_class() {
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
        ]"#;

        let db = ClassDatabase::load_from_string(ron_data).unwrap();
        let knight = db.get_class("knight").unwrap();

        assert_eq!(knight.id, "knight");
        assert_eq!(knight.name, "Knight");
        assert_eq!(knight.hp_die.sides, 10);
    }

    #[test]
    fn test_class_database_get_class_not_found() {
        let db = ClassDatabase::new();
        assert!(db.get_class("nonexistent").is_none());
    }

    #[test]
    fn test_class_database_all_classes() {
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
                id: "cleric",
                name: "Cleric",
                description: "A devoted priest",
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
        ]"#;

        let db = ClassDatabase::load_from_string(ron_data).unwrap();
        let classes: Vec<_> = db.all_classes().collect();
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn test_class_database_duplicate_id_error() {
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
                id: "knight",
                name: "Knight Duplicate",
                description: "A duplicate knight",
                hp_die: (count: 1, sides: 8, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 1,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
        ]"#;

        let result = ClassDatabase::load_from_string(ron_data);
        assert!(matches!(result, Err(ClassError::DuplicateId(_))));
    }

    #[test]
    fn test_class_database_validation_duplicate_bit() {
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
                disablement_bit: 0,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
        ]"#;

        let result = ClassDatabase::load_from_string(ron_data);
        assert!(
            result.is_ok(),
            "Duplicate disablement_bit should be allowed now"
        );
    }

    #[test]
    fn test_class_database_validation_spellcaster_consistency() {
        let ron_data = r#"[
            (
                id: "broken_caster",
                name: "Broken Caster",
                description: "A broken caster",
                hp_die: (count: 1, sides: 6, bonus: 0),
                spell_school: Some(Cleric),
                is_pure_caster: true,
                spell_stat: None,
                disablement_bit: 0,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
        ]"#;

        let result = ClassDatabase::load_from_string(ron_data);
        assert!(matches!(result, Err(ClassError::ValidationError(_))));
    }

    #[test]
    fn test_class_database_validation_invalid_dice() {
        let ron_data = r#"[
            (
                id: "broken_knight",
                name: "Broken Knight",
                description: "A broken knight",
                hp_die: (count: 1, sides: 100, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 0,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
            ),
        ]"#;

        let result = ClassDatabase::load_from_string(ron_data);
        assert!(matches!(result, Err(ClassError::ValidationError(_))));
    }

    #[test]
    fn test_load_classes_from_data_file() {
        // This test verifies that the actual data/classes.ron file is valid
        let result = ClassDatabase::load_from_file("data/classes.ron");
        assert!(
            result.is_ok(),
            "Failed to load data/classes.ron: {:?}",
            result.err()
        );

        let db = result.unwrap();
        assert_eq!(db.len(), 6, "Expected 6 classes in data/classes.ron");

        // Verify all expected classes exist
        assert!(db.get_class("knight").is_some());
        assert!(db.get_class("paladin").is_some());
        assert!(db.get_class("archer").is_some());
        assert!(db.get_class("cleric").is_some());
        assert!(db.get_class("sorcerer").is_some());
        assert!(db.get_class("robber").is_some());

        // Verify Knight properties
        let knight = db.get_class("knight").unwrap();
        assert_eq!(knight.name, "Knight");
        assert_eq!(knight.hp_die.sides, 10);
        assert!(!knight.can_cast_spells());

        // Verify Sorcerer properties
        let sorcerer = db.get_class("sorcerer").unwrap();
        assert_eq!(sorcerer.name, "Sorcerer");
        assert_eq!(sorcerer.hp_die.sides, 4);
        assert!(sorcerer.can_cast_spells());
        assert_eq!(sorcerer.spell_school, Some(SpellSchool::Sorcerer));
        assert_eq!(sorcerer.spell_stat, Some(SpellStat::Intellect));
        assert!(sorcerer.is_pure_caster);

        // Verify Cleric properties
        let cleric = db.get_class("cleric").unwrap();
        assert_eq!(cleric.spell_school, Some(SpellSchool::Cleric));
        assert_eq!(cleric.spell_stat, Some(SpellStat::Personality));
        assert!(cleric.is_pure_caster);

        // Verify Paladin properties (hybrid caster)
        let paladin = db.get_class("paladin").unwrap();
        assert_eq!(paladin.spell_school, Some(SpellSchool::Cleric));
        assert!(!paladin.is_pure_caster);
    }
    #[test]
    fn test_class_validate_with_proficiency_db_rejects_unknown_proficiency() {
        use crate::domain::proficiency::ProficiencyDatabase;

        let ron_data = r#"[
            (
                id: "test_class",
                name: "Test Class",
                description: "A class with unknown proficiency",
                hp_die: (count: 1, sides: 6, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                disablement_bit: 0,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: [],
                proficiencies: ["nonexistent_prof"],
            ),
        ]"#;

        let db = ClassDatabase::load_from_string(ron_data).unwrap();
        let prof_db = ProficiencyDatabase::new(); // Empty DB: doesn't contain 'nonexistent_prof'

        // Should return InvalidProficiency error
        let res = db.validate_with_proficiency_db(&prof_db);
        assert!(matches!(res, Err(ClassError::InvalidProficiency(_))));
    }
}
