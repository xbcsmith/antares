// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Monster database - Loading and managing monster definitions from RON files
//!
//! This module provides functionality to load monster definitions from RON data files
//! and query them at runtime.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 7.1-7.2 for data file specifications.

use crate::domain::character::Stats;
use crate::domain::combat::monster::{LootTable, Monster, MonsterResistances, MonsterCondition};
use crate::domain::combat::types::Attack;
use crate::domain::types::MonsterId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when loading monster data
#[derive(Error, Debug)]
pub enum MonsterDatabaseError {
    #[error("Failed to read monster data file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    #[error("Monster ID {0} not found in database")]
    MonsterNotFound(MonsterId),

    #[error("Duplicate monster ID {0} detected")]
    DuplicateId(MonsterId),
}

// ===== Loot Table =====

// ===== Monster Definition =====

/// Complete monster definition for data files
///
/// This is the master definition that gets loaded from RON files.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::database::MonsterDefinition;
/// use antares::domain::combat::monster::{LootTable, MonsterResistances};
/// use antares::domain::character::{Stats, AttributePair, AttributePair16};
/// use antares::domain::combat::types::Attack;
/// use antares::domain::types::DiceRoll;
///
/// let goblin = MonsterDefinition {
///     id: 1,
///     name: "Goblin".to_string(),
///     stats: Stats {
///         might: AttributePair::new(6),
///         intellect: AttributePair::new(3),
///         personality: AttributePair::new(4),
///         endurance: AttributePair::new(5),
///         speed: AttributePair::new(8),
///         accuracy: AttributePair::new(6),
///         luck: AttributePair::new(4),
///     },
///     hp: AttributePair16::new(8),
///     ac: AttributePair::new(10),
///     attacks: vec![
///         Attack::physical(DiceRoll::new(1, 4, 0)),
///     ],
///     flee_threshold: 3,
///     special_attack_threshold: 20,
///     resistances: MonsterResistances::new(),
///     can_regenerate: false,
///     can_advance: false,
///     is_undead: false,
///     magic_resistance: 0,
///     loot: LootTable::new(1, 10, 0, 0, 10),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDefinition {
    /// Unique monster identifier
    pub id: MonsterId,
    /// Display name
    pub name: String,
    /// Monster attributes
    pub stats: Stats,
    /// Hit points (current/base)
    pub hp: crate::domain::character::AttributePair16,
    /// Armor class (higher is better)
    pub ac: crate::domain::character::AttributePair,
    /// Attack patterns
    pub attacks: Vec<Attack>,
    /// HP threshold to attempt fleeing
    pub flee_threshold: u8,
    /// Percentage chance to use special attack
    pub special_attack_threshold: u8,
    /// Damage resistances
    pub resistances: MonsterResistances,
    /// Can regenerate HP each turn
    pub can_regenerate: bool,
    /// Can move toward party
    pub can_advance: bool,
    /// Is undead (affected by Turn Undead)
    pub is_undead: bool,
    /// Magic resistance percentage (0-100)
    pub magic_resistance: u8,
    /// Loot table
    pub loot: LootTable,
    /// Current condition
    #[serde(default)]
    pub conditions: crate::domain::combat::monster::MonsterCondition,
    /// Active conditions
    #[serde(default)]
    pub active_conditions: Vec<crate::domain::conditions::ActiveCondition>,
    /// Has acted flag
    #[serde(default)]
    pub has_acted: bool,
}

impl MonsterDefinition {
    /// Converts this definition into a Monster instance
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::database::MonsterDefinition;
    /// use antares::domain::combat::monster::{LootTable, MonsterResistances, MonsterCondition};
    /// use antares::domain::character::{Stats, AttributePair, AttributePair16};
    /// use antares::domain::combat::types::Attack;
    ///
    /// let definition = MonsterDefinition {
    ///     id: 1,
    ///     name: "Goblin".to_string(),
    ///     stats: Stats::new(10, 8, 6, 10, 12, 10, 8),
    ///     hp: AttributePair16::new(15),
    ///     ac: AttributePair::new(6),
    ///     attacks: vec![],
    ///     flee_threshold: 5,
    ///     special_attack_threshold: 0,
    ///     resistances: MonsterResistances::new(),
    ///     can_regenerate: false,
    ///     can_advance: true,
    ///     is_undead: false,
    ///     magic_resistance: 0,
    ///     loot: LootTable::new(5, 15, 0, 0, 25),
    ///     conditions: MonsterCondition::Normal,
    ///     active_conditions: vec![],
    ///     has_acted: false,
    /// };
    ///
    /// let monster = definition.to_monster();
    /// assert_eq!(monster.name, "Goblin");
    /// assert_eq!(monster.hp.base, 15);
    /// ```
    pub fn to_monster(&self) -> Monster {
        let mut monster = Monster::new(
            self.id,
            self.name.clone(),
            self.stats.clone(),
            self.hp.base,
            self.ac.base,
            self.attacks.clone(),
            self.loot.clone(),
            self.loot.experience,
        );

        monster.flee_threshold = self.flee_threshold;
        monster.special_attack_threshold = self.special_attack_threshold;
        monster.resistances = self.resistances.clone();
        monster.can_regenerate = self.can_regenerate;
        monster.can_advance = self.can_advance;
        monster.is_undead = self.is_undead;
        monster.magic_resistance = self.magic_resistance;
        monster.conditions = self.conditions;
        monster.active_conditions = self.active_conditions.clone();
        monster.has_acted = self.has_acted;

        monster
    }
}

// ===== Monster Database =====

/// Monster database - stores all monster definitions
///
/// # Examples
///
/// ```no_run
/// use antares::domain::combat::database::MonsterDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load monsters from RON file
/// let db = MonsterDatabase::load_from_file("data/monsters.ron")?;
///
/// // Query monster by ID
/// if let Some(monster) = db.get_monster(1) {
///     println!("Found monster: {}", monster.name);
/// }
///
/// // Get all undead monsters
/// let undead = db.get_undead_monsters();
/// println!("Total undead: {}", undead.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDatabase {
    /// All monsters indexed by ID
    monsters: HashMap<MonsterId, MonsterDefinition>,
}

impl MonsterDatabase {
    /// Create an empty monster database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::database::MonsterDatabase;
    ///
    /// let db = MonsterDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            monsters: HashMap::new(),
        }
    }

    /// Load monster database from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing monster definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(MonsterDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `MonsterDatabaseError::ReadError` if file cannot be read
    /// Returns `MonsterDatabaseError::ParseError` if RON parsing fails
    /// Returns `MonsterDatabaseError::DuplicateId` if duplicate monster IDs found
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::combat::database::MonsterDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = MonsterDatabase::load_from_file("data/monsters.ron")?;
    /// println!("Loaded {} monsters", db.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, MonsterDatabaseError> {
        let contents = std::fs::read_to_string(path)?;
        Self::load_from_string(&contents)
    }

    /// Load monster database from a RON string
    ///
    /// # Arguments
    ///
    /// * `ron_data` - RON-formatted string containing monster definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(MonsterDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `MonsterDatabaseError::ParseError` if RON parsing fails
    /// Returns `MonsterDatabaseError::DuplicateId` if duplicate monster IDs found
    pub fn load_from_string(ron_data: &str) -> Result<Self, MonsterDatabaseError> {
        let monsters: Vec<MonsterDefinition> = ron::from_str(ron_data)?;
        let mut db = Self::new();

        for monster in monsters {
            if db.monsters.contains_key(&monster.id) {
                return Err(MonsterDatabaseError::DuplicateId(monster.id));
            }
            db.monsters.insert(monster.id, monster);
        }

        Ok(db)
    }

    /// Add a monster to the database
    ///
    /// # Arguments
    ///
    /// * `monster` - Monster to add
    ///
    /// # Errors
    ///
    /// Returns `MonsterDatabaseError::DuplicateId` if monster ID already exists
    pub fn add_monster(&mut self, monster: MonsterDefinition) -> Result<(), MonsterDatabaseError> {
        if self.monsters.contains_key(&monster.id) {
            return Err(MonsterDatabaseError::DuplicateId(monster.id));
        }
        self.monsters.insert(monster.id, monster);
        Ok(())
    }

    /// Get a monster by ID
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::database::{MonsterDatabase, MonsterDefinition};
    /// use antares::domain::combat::monster::{LootTable, MonsterResistances, MonsterCondition};
    /// use antares::domain::character::{Stats, AttributePair, AttributePair16};
    ///
    /// let mut db = MonsterDatabase::new();
    /// let goblin = MonsterDefinition {
    ///     id: 1,
    ///     name: "Goblin".to_string(),
    ///     stats: Stats {
    ///         might: AttributePair::new(6),
    ///         intellect: AttributePair::new(3),
    ///         personality: AttributePair::new(4),
    ///         endurance: AttributePair::new(5),
    ///         speed: AttributePair::new(8),
    ///         accuracy: AttributePair::new(6),
    ///         luck: AttributePair::new(4),
    ///     },
    ///     hp: AttributePair16::new(8),
    ///     ac: AttributePair::new(10),
    ///     attacks: vec![],
    ///     flee_threshold: 3,
    ///     special_attack_threshold: 20,
    ///     resistances: MonsterResistances::new(),
    ///     can_regenerate: false,
    ///     can_advance: false,
    ///     is_undead: false,
    ///     magic_resistance: 0,
    ///     loot: LootTable::new(1, 10, 0, 0, 10),
    ///     conditions: MonsterCondition::Normal,
    ///     active_conditions: vec![],
    ///     has_acted: false,
    /// };
    /// db.add_monster(goblin).unwrap();
    ///
    /// assert!(db.get_monster(1).is_some());
    /// assert!(db.get_monster(99).is_none());
    /// ```
    pub fn get_monster(&self, id: MonsterId) -> Option<&MonsterDefinition> {
        self.monsters.get(&id)
    }

    /// Get all monsters in the database
    pub fn all_monsters(&self) -> Vec<&MonsterDefinition> {
        self.monsters.values().collect()
    }

    /// Get all undead monsters
    pub fn get_undead_monsters(&self) -> Vec<&MonsterDefinition> {
        self.monsters
            .values()
            .filter(|monster| monster.is_undead)
            .collect()
    }

    /// Get all monsters that can regenerate
    pub fn get_regenerating_monsters(&self) -> Vec<&MonsterDefinition> {
        self.monsters
            .values()
            .filter(|monster| monster.can_regenerate)
            .collect()
    }

    /// Get monsters by HP range (for encounter balancing)
    pub fn get_monsters_by_hp_range(&self, min_hp: u16, max_hp: u16) -> Vec<&MonsterDefinition> {
        self.monsters
            .values()
            .filter(|monster| monster.hp.base >= min_hp && monster.hp.base <= max_hp)
            .collect()
    }

    /// Get number of monsters in database
    pub fn len(&self) -> usize {
        self.monsters.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.monsters.is_empty()
    }
}

impl Default for MonsterDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{AttributePair, AttributePair16};

    fn create_test_monster(id: MonsterId, name: &str, hp: u16) -> MonsterDefinition {
        MonsterDefinition {
            id,
            name: name.to_string(),
            stats: Stats {
                might: AttributePair::new(10),
                intellect: AttributePair::new(10),
                personality: AttributePair::new(10),
                endurance: AttributePair::new(10),
                speed: AttributePair::new(10),
                accuracy: AttributePair::new(10),
                luck: AttributePair::new(10),
            },
            hp: AttributePair16::new(hp),
            ac: AttributePair::new(10),
            attacks: vec![],
            flee_threshold: 5,
            special_attack_threshold: 20,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable::new(1, 10, 0, 0, 10),
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }
    }

    #[test]
    fn test_new_database_is_empty() {
        let db = MonsterDatabase::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_monster() {
        let mut db = MonsterDatabase::new();
        let monster = create_test_monster(1, "Goblin", 8);

        db.add_monster(monster.clone()).unwrap();

        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());

        let retrieved = db.get_monster(1).unwrap();
        assert_eq!(retrieved.name, "Goblin");
    }

    #[test]
    fn test_duplicate_id_error() {
        let mut db = MonsterDatabase::new();
        let monster1 = create_test_monster(1, "First", 8);
        let monster2 = create_test_monster(1, "Second", 10);

        db.add_monster(monster1).unwrap();
        let result = db.add_monster(monster2);

        assert!(result.is_err());
        assert!(matches!(result, Err(MonsterDatabaseError::DuplicateId(1))));
    }

    #[test]
    fn test_get_nonexistent_monster() {
        let db = MonsterDatabase::new();
        assert!(db.get_monster(99).is_none());
    }

    #[test]
    fn test_filter_undead() {
        let mut db = MonsterDatabase::new();
        let mut zombie = create_test_monster(1, "Zombie", 20);
        zombie.is_undead = true;
        db.add_monster(zombie).unwrap();

        let goblin = create_test_monster(2, "Goblin", 8);
        db.add_monster(goblin).unwrap();

        let undead = db.get_undead_monsters();
        assert_eq!(undead.len(), 1);
        assert_eq!(undead[0].name, "Zombie");
    }

    #[test]
    fn test_filter_by_hp_range() {
        let mut db = MonsterDatabase::new();
        db.add_monster(create_test_monster(1, "Weak", 5)).unwrap();
        db.add_monster(create_test_monster(2, "Medium", 15))
            .unwrap();
        db.add_monster(create_test_monster(3, "Strong", 50))
            .unwrap();

        let mid_range = db.get_monsters_by_hp_range(10, 20);
        assert_eq!(mid_range.len(), 1);
        assert_eq!(mid_range[0].name, "Medium");
    }

    #[test]
    fn test_all_monsters() {
        let mut db = MonsterDatabase::new();
        db.add_monster(create_test_monster(1, "Monster1", 10))
            .unwrap();
        db.add_monster(create_test_monster(2, "Monster2", 20))
            .unwrap();
        db.add_monster(create_test_monster(3, "Monster3", 30))
            .unwrap();

        let all = db.all_monsters();
        assert_eq!(all.len(), 3);
    }
}
