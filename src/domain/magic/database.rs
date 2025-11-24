// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell database - Loading and managing spell definitions from RON files
//!
//! This module provides functionality to load spell definitions from RON data files
//! and query them at runtime.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 7.1-7.2 for data file specifications.

use crate::domain::magic::types::{Spell, SpellSchool};
use crate::domain::types::SpellId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when loading spell data
#[derive(Error, Debug)]
pub enum SpellDatabaseError {
    #[error("Failed to read spell data file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    #[error("Spell ID {0} not found in database")]
    SpellNotFound(SpellId),

    #[error("Duplicate spell ID {0} detected")]
    DuplicateId(SpellId),
}

// ===== Spell Database =====

/// Spell database - stores all spell definitions
///
/// # Examples
///
/// ```no_run
/// use antares::domain::magic::SpellDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load spells from RON file
/// let db = SpellDatabase::load_from_file("data/spells.ron")?;
///
/// // Query spell by ID
/// if let Some(spell) = db.get_spell(257) {
///     println!("Found spell: {}", spell.name);
/// }
///
/// // Get all Cleric spells
/// let cleric_spells = db.get_spells_by_school(antares::domain::magic::SpellSchool::Cleric);
/// println!("Total Cleric spells: {}", cleric_spells.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellDatabase {
    /// All spells indexed by ID
    spells: HashMap<SpellId, Spell>,
}

impl SpellDatabase {
    /// Create an empty spell database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::SpellDatabase;
    ///
    /// let db = SpellDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
        }
    }

    /// Load spell database from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing spell definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(SpellDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `SpellDatabaseError::ReadError` if file cannot be read
    /// Returns `SpellDatabaseError::ParseError` if RON parsing fails
    /// Returns `SpellDatabaseError::DuplicateId` if duplicate spell IDs found
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::magic::SpellDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = SpellDatabase::load_from_file("data/spells.ron")?;
    /// println!("Loaded {} spells", db.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SpellDatabaseError> {
        let contents = std::fs::read_to_string(path)?;
        Self::load_from_string(&contents)
    }

    /// Load spell database from a RON string
    ///
    /// # Arguments
    ///
    /// * `ron_data` - RON-formatted string containing spell definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(SpellDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `SpellDatabaseError::ParseError` if RON parsing fails
    /// Returns `SpellDatabaseError::DuplicateId` if duplicate spell IDs found
    pub fn load_from_string(ron_data: &str) -> Result<Self, SpellDatabaseError> {
        let spells: Vec<Spell> = ron::from_str(ron_data)?;
        let mut db = Self::new();

        for spell in spells {
            if db.spells.contains_key(&spell.id) {
                return Err(SpellDatabaseError::DuplicateId(spell.id));
            }
            db.spells.insert(spell.id, spell);
        }

        Ok(db)
    }

    /// Add a spell to the database
    ///
    /// # Arguments
    ///
    /// * `spell` - Spell to add
    ///
    /// # Errors
    ///
    /// Returns `SpellDatabaseError::DuplicateId` if spell ID already exists
    pub fn add_spell(&mut self, spell: Spell) -> Result<(), SpellDatabaseError> {
        if self.spells.contains_key(&spell.id) {
            return Err(SpellDatabaseError::DuplicateId(spell.id));
        }
        self.spells.insert(spell.id, spell);
        Ok(())
    }

    /// Get a spell by ID
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::{SpellDatabase, Spell, SpellSchool, SpellContext, SpellTarget};
    ///
    /// let mut db = SpellDatabase::new();
    /// let spell = Spell {
    ///     id: 257,
    ///     name: "Awaken".to_string(),
    ///     school: SpellSchool::Cleric,
    ///     level: 1,
    ///     sp_cost: 5,
    ///     gem_cost: 0,
    ///     context: SpellContext::Anytime,
    ///     target: SpellTarget::SingleCharacter,
    ///     description: "Awakens a sleeping character".to_string(),
    /// };
    /// db.add_spell(spell).unwrap();
    ///
    /// assert!(db.get_spell(257).is_some());
    /// assert!(db.get_spell(999).is_none());
    /// ```
    pub fn get_spell(&self, id: SpellId) -> Option<&Spell> {
        self.spells.get(&id)
    }

    /// Get all spells in the database
    pub fn all_spells(&self) -> Vec<&Spell> {
        self.spells.values().collect()
    }

    /// Get all spells of a specific school
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::{SpellDatabase, Spell, SpellSchool, SpellContext, SpellTarget};
    ///
    /// let mut db = SpellDatabase::new();
    /// let cleric_spell = Spell {
    ///     id: 257,
    ///     name: "Awaken".to_string(),
    ///     school: SpellSchool::Cleric,
    ///     level: 1,
    ///     sp_cost: 5,
    ///     gem_cost: 0,
    ///     context: SpellContext::Anytime,
    ///     target: SpellTarget::SingleCharacter,
    ///     description: "Awakens sleeping".to_string(),
    /// };
    /// db.add_spell(cleric_spell).unwrap();
    ///
    /// let cleric_spells = db.get_spells_by_school(SpellSchool::Cleric);
    /// assert_eq!(cleric_spells.len(), 1);
    /// ```
    pub fn get_spells_by_school(&self, school: SpellSchool) -> Vec<&Spell> {
        self.spells
            .values()
            .filter(|spell| spell.school == school)
            .collect()
    }

    /// Get all spells of a specific level
    pub fn get_spells_by_level(&self, level: u8) -> Vec<&Spell> {
        self.spells
            .values()
            .filter(|spell| spell.level == level)
            .collect()
    }

    /// Get all spells of a specific school and level
    pub fn get_spells_by_school_and_level(&self, school: SpellSchool, level: u8) -> Vec<&Spell> {
        self.spells
            .values()
            .filter(|spell| spell.school == school && spell.level == level)
            .collect()
    }

    /// Get number of spells in database
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}

impl Default for SpellDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::magic::{SpellContext, SpellTarget};

    fn create_test_spell(id: SpellId, name: &str, school: SpellSchool, level: u8) -> Spell {
        Spell {
            id,
            name: name.to_string(),
            school,
            level,
            sp_cost: 5,
            gem_cost: 0,
            context: SpellContext::Anytime,
            target: SpellTarget::SingleCharacter,
            description: "Test spell".to_string(),
            damage: None,
            duration: 0,
            saving_throw: false,
        }
    }

    #[test]
    fn test_new_database_is_empty() {
        let db = SpellDatabase::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_spell() {
        let mut db = SpellDatabase::new();
        let spell = create_test_spell(257, "Awaken", SpellSchool::Cleric, 1);

        db.add_spell(spell.clone()).unwrap();

        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());

        let retrieved = db.get_spell(257).unwrap();
        assert_eq!(retrieved.name, "Awaken");
    }

    #[test]
    fn test_duplicate_id_error() {
        let mut db = SpellDatabase::new();
        let spell1 = create_test_spell(257, "First", SpellSchool::Cleric, 1);
        let spell2 = create_test_spell(257, "Second", SpellSchool::Cleric, 1);

        db.add_spell(spell1).unwrap();
        let result = db.add_spell(spell2);

        assert!(result.is_err());
        assert!(matches!(result, Err(SpellDatabaseError::DuplicateId(257))));
    }

    #[test]
    fn test_get_nonexistent_spell() {
        let db = SpellDatabase::new();
        assert!(db.get_spell(999).is_none());
    }

    #[test]
    fn test_filter_by_school() {
        let mut db = SpellDatabase::new();
        db.add_spell(create_test_spell(257, "Cleric1", SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(create_test_spell(
            513,
            "Sorcerer1",
            SpellSchool::Sorcerer,
            1,
        ))
        .unwrap();
        db.add_spell(create_test_spell(258, "Cleric2", SpellSchool::Cleric, 2))
            .unwrap();

        let cleric_spells = db.get_spells_by_school(SpellSchool::Cleric);
        assert_eq!(cleric_spells.len(), 2);

        let sorcerer_spells = db.get_spells_by_school(SpellSchool::Sorcerer);
        assert_eq!(sorcerer_spells.len(), 1);
    }

    #[test]
    fn test_filter_by_level() {
        let mut db = SpellDatabase::new();
        db.add_spell(create_test_spell(257, "L1Spell1", SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(create_test_spell(258, "L1Spell2", SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(create_test_spell(259, "L2Spell", SpellSchool::Cleric, 2))
            .unwrap();

        let level1 = db.get_spells_by_level(1);
        assert_eq!(level1.len(), 2);

        let level2 = db.get_spells_by_level(2);
        assert_eq!(level2.len(), 1);
    }

    #[test]
    fn test_filter_by_school_and_level() {
        let mut db = SpellDatabase::new();
        db.add_spell(create_test_spell(257, "C1", SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(create_test_spell(513, "S1", SpellSchool::Sorcerer, 1))
            .unwrap();
        db.add_spell(create_test_spell(258, "C2", SpellSchool::Cleric, 2))
            .unwrap();

        let cleric_l1 = db.get_spells_by_school_and_level(SpellSchool::Cleric, 1);
        assert_eq!(cleric_l1.len(), 1);

        let sorcerer_l1 = db.get_spells_by_school_and_level(SpellSchool::Sorcerer, 1);
        assert_eq!(sorcerer_l1.len(), 1);
    }

    #[test]
    fn test_load_from_ron_string() {
        let ron_data = r#"
[
    (
        id: 257,
        name: "Awaken",
        school: Cleric,
        level: 1,
        sp_cost: 5,
        gem_cost: 0,
        context: Anytime,
        target: SingleCharacter,
        description: "Awakens a sleeping character",
        damage: None,
        duration: 0,
        saving_throw: false,
    ),
    (
        id: 513,
        name: "Light",
        school: Sorcerer,
        level: 1,
        sp_cost: 3,
        gem_cost: 0,
        context: NonCombatOnly,
        target: Self_,
        description: "Creates light",
        damage: None,
        duration: 0,
        saving_throw: false,
    ),
]
"#;

        let db = SpellDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db.get_spell(257).is_some());
        assert!(db.get_spell(513).is_some());
    }

    #[test]
    fn test_all_spells() {
        let mut db = SpellDatabase::new();
        db.add_spell(create_test_spell(257, "Spell1", SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(create_test_spell(258, "Spell2", SpellSchool::Cleric, 2))
            .unwrap();
        db.add_spell(create_test_spell(259, "Spell3", SpellSchool::Cleric, 3))
            .unwrap();

        let all = db.all_spells();
        assert_eq!(all.len(), 3);
    }
}
