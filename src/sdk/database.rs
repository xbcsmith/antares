// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unified content database for all game content types
//!
//! This module provides the ContentDatabase structure that loads and manages
//! all game content (classes, races, items, monsters, spells, maps) from a
//! campaign directory structure.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_implementation_plan.md` Phase 3.2 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::database::ContentDatabase;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load core game content
//! let core_db = ContentDatabase::load_core("data")?;
//!
//! // Load campaign-specific content
//! let campaign_db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
//!
//! // Get statistics
//! let stats = campaign_db.stats();
//! println!("Loaded {} items, {} monsters", stats.item_count, stats.monster_count);
//! # Ok(())
//! # }
//! ```

use crate::domain::character_definition::CharacterDatabase;
use crate::domain::classes::ClassDatabase;
use crate::domain::combat::monster::Monster;
use crate::domain::conditions::{ConditionDefinition, ConditionId};
use crate::domain::dialogue::{DialogueId, DialogueTree};
use crate::domain::items::ItemDatabase;
use crate::domain::magic::types::Spell;
use crate::domain::quest::{Quest, QuestId};
use crate::domain::races::{RaceDatabase, RaceError};
use crate::domain::types::{MapId, MonsterId, SpellId};
use crate::domain::world::{Map, MapBlueprint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with the content database
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to load classes: {0}")]
    ClassLoadError(String),

    #[error("Failed to load races: {0}")]
    RaceLoadError(String),

    #[error("Failed to load items: {0}")]
    ItemLoadError(String),

    #[error("Failed to load monsters: {0}")]
    MonsterLoadError(String),

    #[error("Failed to load spells: {0}")]
    SpellLoadError(String),

    #[error("Failed to load quests: {0}")]
    QuestLoadError(String),

    #[error("Failed to load dialogues: {0}")]
    DialogueLoadError(String),

    #[error("Failed to load conditions: {0}")]
    ConditionLoadError(String),

    #[error("Failed to load characters: {0}")]
    CharacterLoadError(String),

    #[error("Failed to load map {map_id}: {error}")]
    MapLoadError { map_id: MapId, error: String },

    #[error("Campaign directory not found: {0}")]
    CampaignNotFound(PathBuf),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("RON parsing error: {0}")]
    RonError(#[from] ron::Error),

    #[error("Validation failed: {0}")]
    ValidationError(String),
}

impl From<RaceError> for DatabaseError {
    fn from(err: RaceError) -> Self {
        DatabaseError::RaceLoadError(err.to_string())
    }
}

// ===== Race System =====
// Race types are now imported from crate::domain::races module.
// See docs/explanation/hardcoded_removal_implementation_plan.md Phase 4.

// ===== Spell System =====

/// Spell database for loading and managing spells
#[derive(Debug, Clone, Default)]
pub struct SpellDatabase {
    spells: HashMap<SpellId, Spell>,
}

impl SpellDatabase {
    /// Creates an empty spell database
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
        }
    }

    /// Loads spells from a RON file
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
    /// Returns `DatabaseError::SpellLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::SpellDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = SpellDatabase::load_from_file("campaigns/tutorial/data/spells.ron")?;
    /// println!("Loaded {} spells", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return empty database if file doesn't exist
        if !path.exists() {
            return Ok(Self::new());
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| DatabaseError::SpellLoadError(format!("Failed to read file: {}", e)))?;

        let spells: Vec<Spell> = ron::from_str(&contents)
            .map_err(|e| DatabaseError::SpellLoadError(format!("Failed to parse RON: {}", e)))?;

        // Build HashMap from vector
        let mut spell_map = HashMap::new();
        for spell in spells {
            spell_map.insert(spell.id, spell);
        }

        Ok(Self { spells: spell_map })
    }

    /// Gets a spell by ID
    pub fn get_spell(&self, id: SpellId) -> Option<&Spell> {
        self.spells.get(&id)
    }

    /// Returns all spell IDs
    pub fn all_spells(&self) -> Vec<SpellId> {
        self.spells.keys().copied().collect()
    }

    /// Returns the number of spells
    pub fn count(&self) -> usize {
        self.spells.len()
    }

    /// Checks if a spell exists in the database
    pub fn has_spell(&self, id: &SpellId) -> bool {
        self.spells.contains_key(id)
    }

    /// Gets a spell by name (case-insensitive)
    pub fn get_spell_by_name(&self, name: &str) -> Option<&Spell> {
        let name_lower = name.to_lowercase();
        self.spells
            .values()
            .find(|s| s.name.to_lowercase() == name_lower)
    }

    /// Returns all spells for a given school
    pub fn spells_by_school(
        &self,
        school: crate::domain::magic::types::SpellSchool,
    ) -> Vec<&Spell> {
        self.spells
            .values()
            .filter(|s| s.school == school)
            .collect()
    }

    /// Returns all spells of a given level
    pub fn spells_by_level(&self, level: u8) -> Vec<&Spell> {
        self.spells.values().filter(|s| s.level == level).collect()
    }
}

// ===== Monster System =====

/// Monster database for loading and managing monsters
#[derive(Debug, Clone, Default)]
pub struct MonsterDatabase {
    monsters: HashMap<MonsterId, Monster>,
}

impl MonsterDatabase {
    /// Creates an empty monster database
    pub fn new() -> Self {
        Self {
            monsters: HashMap::new(),
        }
    }

    /// Loads monsters from a RON file
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
    /// Returns `DatabaseError::MonsterLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::MonsterDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = MonsterDatabase::load_from_file("campaigns/tutorial/data/monsters.ron")?;
    /// println!("Loaded {} monsters", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return empty database if file doesn't exist
        if !path.exists() {
            return Ok(Self::new());
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| DatabaseError::MonsterLoadError(format!("Failed to read file: {}", e)))?;

        let monsters: Vec<Monster> = ron::from_str(&contents)
            .map_err(|e| DatabaseError::MonsterLoadError(format!("Failed to parse RON: {}", e)))?;

        // Build HashMap from vector
        let mut monster_map = HashMap::new();
        for monster in monsters {
            monster_map.insert(monster.id, monster);
        }

        Ok(Self {
            monsters: monster_map,
        })
    }

    /// Gets a monster by ID
    pub fn get_monster(&self, id: MonsterId) -> Option<&Monster> {
        self.monsters.get(&id)
    }

    /// Returns all monster IDs
    pub fn all_monsters(&self) -> Vec<MonsterId> {
        self.monsters.keys().copied().collect()
    }

    /// Returns the number of monsters
    pub fn count(&self) -> usize {
        self.monsters.len()
    }

    /// Checks if a monster exists in the database
    pub fn has_monster(&self, id: &MonsterId) -> bool {
        self.monsters.contains_key(id)
    }

    /// Gets a monster by name (case-insensitive)
    pub fn get_monster_by_name(&self, name: &str) -> Option<&Monster> {
        let name_lower = name.to_lowercase();
        self.monsters
            .values()
            .find(|m| m.name.to_lowercase() == name_lower)
    }

    /// Returns all undead monsters
    pub fn undead_monsters(&self) -> Vec<&Monster> {
        self.monsters.values().filter(|m| m.is_undead).collect()
    }

    /// Returns monsters within an experience value range
    pub fn monsters_by_experience_range(&self, min_xp: u32, max_xp: u32) -> Vec<&Monster> {
        self.monsters
            .values()
            .filter(|m| m.loot.experience >= min_xp && m.loot.experience <= max_xp)
            .collect()
    }
}

// ===== Map Database =====

/// Map database for loading and managing maps
#[derive(Debug, Clone, Default)]
pub struct MapDatabase {
    maps: HashMap<MapId, Map>,
}

impl MapDatabase {
    /// Creates an empty map database
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
        }
    }

    /// Loads maps from a directory
    /// Loads maps from a directory
    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();
        let mut maps = HashMap::new();

        if !path.exists() {
            return Ok(Self::new());
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "ron") {
                let contents = std::fs::read_to_string(&path)?;

                // Try to load as Map (Engine/SDK format) first
                if let Ok(map) = ron::from_str::<Map>(&contents) {
                    maps.insert(map.id, map);
                    continue;
                }

                // Fallback to MapBlueprint
                let blueprint: MapBlueprint =
                    ron::from_str(&contents).map_err(|e| DatabaseError::RonError(e.code))?;
                let map: Map = blueprint.into();
                maps.insert(map.id, map);
            }
        }

        Ok(Self { maps })
    }

    /// Gets a map by ID
    pub fn get_map(&self, id: MapId) -> Option<&Map> {
        self.maps.get(&id)
    }

    /// Returns all map IDs
    pub fn all_maps(&self) -> Vec<MapId> {
        self.maps.keys().copied().collect()
    }

    /// Returns the number of maps
    pub fn count(&self) -> usize {
        self.maps.len()
    }

    /// Checks if a map exists in the database
    pub fn has_map(&self, id: &MapId) -> bool {
        self.maps.contains_key(id)
    }
}

// ===== Quest Database =====

/// Quest database for loading and managing quests
#[derive(Debug, Clone, Default)]
pub struct QuestDatabase {
    quests: HashMap<QuestId, Quest>,
}

impl QuestDatabase {
    /// Creates an empty quest database
    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
        }
    }

    /// Loads quests from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing quest definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(QuestDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::QuestLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::QuestDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = QuestDatabase::load_from_file("campaigns/tutorial/data/quests.ron")?;
    /// println!("Loaded {} quests", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return empty database if file doesn't exist
        if !path.exists() {
            return Ok(Self::new());
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| DatabaseError::QuestLoadError(format!("Failed to read file: {}", e)))?;

        let quests: Vec<Quest> = ron::from_str(&contents)
            .map_err(|e| DatabaseError::QuestLoadError(format!("Failed to parse RON: {}", e)))?;

        // Build HashMap from vector
        let mut quest_map = HashMap::new();
        for quest in quests {
            quest_map.insert(quest.id, quest);
        }

        Ok(Self { quests: quest_map })
    }

    /// Gets a quest by ID
    pub fn get_quest(&self, id: QuestId) -> Option<&Quest> {
        self.quests.get(&id)
    }

    /// Returns all quest IDs
    pub fn all_quests(&self) -> Vec<QuestId> {
        self.quests.keys().copied().collect()
    }

    /// Returns the number of quests
    pub fn count(&self) -> usize {
        self.quests.len()
    }

    /// Checks if a quest exists in the database
    pub fn has_quest(&self, id: &QuestId) -> bool {
        self.quests.contains_key(id)
    }

    /// Adds a quest to the database
    pub fn add_quest(&mut self, quest: Quest) {
        self.quests.insert(quest.id, quest);
    }

    /// Gets a quest by name (case-insensitive)
    pub fn get_quest_by_name(&self, name: &str) -> Option<&Quest> {
        let name_lower = name.to_lowercase();
        self.quests
            .values()
            .find(|q| q.name.to_lowercase() == name_lower)
    }

    /// Returns all main quests
    pub fn main_quests(&self) -> Vec<&Quest> {
        self.quests.values().filter(|q| q.is_main_quest).collect()
    }

    /// Returns all repeatable quests
    pub fn repeatable_quests(&self) -> Vec<&Quest> {
        self.quests.values().filter(|q| q.repeatable).collect()
    }

    /// Returns quests available at a given level
    pub fn quests_for_level(&self, level: u8) -> Vec<&Quest> {
        self.quests
            .values()
            .filter(|q| {
                let min_ok = q.min_level.is_none_or(|min| level >= min);
                let max_ok = q.max_level.is_none_or(|max| level <= max);
                min_ok && max_ok
            })
            .collect()
    }
}

// ===== Condition Database =====

/// Condition database for loading and managing condition definitions
#[derive(Debug, Clone, Default)]
pub struct ConditionDatabase {
    conditions: HashMap<ConditionId, ConditionDefinition>,
}

impl ConditionDatabase {
    /// Creates an empty condition database
    pub fn new() -> Self {
        Self {
            conditions: HashMap::new(),
        }
    }

    /// Loads conditions from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing condition definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(ConditionDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::ConditionLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::ConditionDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ConditionDatabase::load_from_file("campaigns/tutorial/data/conditions.ron")?;
    /// println!("Loaded {} conditions", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return empty database if file doesn't exist
        if !path.exists() {
            return Ok(Self::new());
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path).map_err(|e| {
            DatabaseError::ConditionLoadError(format!("Failed to read file: {}", e))
        })?;

        let conditions: Vec<ConditionDefinition> = ron::from_str(&contents).map_err(|e| {
            DatabaseError::ConditionLoadError(format!("Failed to parse RON: {}", e))
        })?;

        // Build HashMap from vector
        let mut condition_map = HashMap::new();
        for condition in conditions {
            condition_map.insert(condition.id.clone(), condition);
        }

        Ok(Self {
            conditions: condition_map,
        })
    }

    /// Gets a condition by ID
    pub fn get_condition(&self, id: &ConditionId) -> Option<&ConditionDefinition> {
        self.conditions.get(id)
    }

    /// Returns all condition IDs
    pub fn all_conditions(&self) -> Vec<&ConditionId> {
        self.conditions.keys().collect()
    }

    /// Returns the number of conditions
    pub fn count(&self) -> usize {
        self.conditions.len()
    }

    /// Checks if a condition exists in the database
    pub fn has_condition(&self, id: &ConditionId) -> bool {
        self.conditions.contains_key(id)
    }

    /// Adds a condition to the database
    pub fn add_condition(&mut self, condition: ConditionDefinition) {
        self.conditions.insert(condition.id.clone(), condition);
    }

    /// Gets a condition by name (case-insensitive)
    pub fn get_condition_by_name(&self, name: &str) -> Option<&ConditionDefinition> {
        let name_lower = name.to_lowercase();
        self.conditions
            .values()
            .find(|c| c.name.to_lowercase() == name_lower)
    }
}

// ===== Dialogue Database =====

/// Dialogue database for loading and managing dialogue trees
#[derive(Debug, Clone, Default)]
pub struct DialogueDatabase {
    dialogues: HashMap<DialogueId, DialogueTree>,
}

impl DialogueDatabase {
    /// Creates an empty dialogue database
    pub fn new() -> Self {
        Self {
            dialogues: HashMap::new(),
        }
    }

    /// Loads dialogues from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing dialogue definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(DialogueDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::DialogueLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::DialogueDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = DialogueDatabase::load_from_file("campaigns/tutorial/data/dialogues.ron")?;
    /// println!("Loaded {} dialogues", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return empty database if file doesn't exist
        if !path.exists() {
            return Ok(Self::new());
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| DatabaseError::DialogueLoadError(format!("Failed to read file: {}", e)))?;

        let dialogues: Vec<DialogueTree> = ron::from_str(&contents)
            .map_err(|e| DatabaseError::DialogueLoadError(format!("Failed to parse RON: {}", e)))?;

        // Build HashMap from vector
        let mut dialogue_map = HashMap::new();
        for dialogue in dialogues {
            dialogue_map.insert(dialogue.id, dialogue);
        }

        Ok(Self {
            dialogues: dialogue_map,
        })
    }

    /// Gets a dialogue by ID
    pub fn get_dialogue(&self, id: DialogueId) -> Option<&DialogueTree> {
        self.dialogues.get(&id)
    }

    /// Returns all dialogue IDs
    pub fn all_dialogues(&self) -> Vec<DialogueId> {
        self.dialogues.keys().copied().collect()
    }

    /// Returns the number of dialogues
    pub fn count(&self) -> usize {
        self.dialogues.len()
    }

    /// Checks if a dialogue exists in the database
    pub fn has_dialogue(&self, id: &DialogueId) -> bool {
        self.dialogues.contains_key(id)
    }

    /// Adds a dialogue to the database
    pub fn add_dialogue(&mut self, dialogue: DialogueTree) {
        self.dialogues.insert(dialogue.id, dialogue);
    }

    /// Validates all dialogues in the database
    pub fn validate(&self) -> Result<(), String> {
        for dialogue in self.dialogues.values() {
            dialogue.validate()?;
        }
        Ok(())
    }

    /// Gets a dialogue by name (case-insensitive)
    pub fn get_dialogue_by_name(&self, name: &str) -> Option<&DialogueTree> {
        let name_lower = name.to_lowercase();
        self.dialogues
            .values()
            .find(|d| d.name.to_lowercase() == name_lower)
    }

    /// Returns all repeatable dialogues
    pub fn repeatable_dialogues(&self) -> Vec<&DialogueTree> {
        self.dialogues.values().filter(|d| d.repeatable).collect()
    }

    /// Returns dialogues associated with a specific quest
    pub fn dialogues_for_quest(&self, quest_id: QuestId) -> Vec<&DialogueTree> {
        self.dialogues
            .values()
            .filter(|d| d.associated_quest == Some(quest_id))
            .collect()
    }
}

// ===== Content Database =====

/// Unified content database containing all game content
///
/// This structure provides centralized access to all content types
/// (classes, races, items, monsters, spells, maps) loaded from a
/// campaign directory.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::database::ContentDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load core game content
/// let db = ContentDatabase::load_core("data")?;
///
/// // Query content
/// if let Some(class) = db.classes.get_class("knight") {
///     println!("Found class: {}", class.name);
/// }
///
/// // Get statistics
/// let stats = db.stats();
/// println!("Total content items: {}",
///     stats.class_count + stats.race_count + stats.item_count
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ContentDatabase {
    /// Class definitions database
    pub classes: ClassDatabase,

    /// Race definitions database
    pub races: RaceDatabase,

    /// Item definitions database
    pub items: ItemDatabase,

    /// Monster definitions database
    pub monsters: MonsterDatabase,

    /// Spell definitions database
    pub spells: SpellDatabase,

    /// Map definitions database
    pub maps: MapDatabase,

    /// Quest definitions database
    pub quests: QuestDatabase,

    /// Dialogue definitions database
    pub dialogues: DialogueDatabase,

    /// Condition definitions database
    pub conditions: ConditionDatabase,

    /// Character definitions database
    pub characters: CharacterDatabase,
}

impl ContentDatabase {
    /// Creates an empty content database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// let db = ContentDatabase::new();
    /// assert_eq!(db.stats().class_count, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            classes: ClassDatabase::new(),
            races: RaceDatabase::new(),
            items: ItemDatabase::new(),
            monsters: MonsterDatabase::new(),
            spells: SpellDatabase::new(),
            maps: MapDatabase::new(),
            quests: QuestDatabase::new(),
            dialogues: DialogueDatabase::new(),
            conditions: ConditionDatabase::new(),
            characters: CharacterDatabase::new(),
        }
    }

    /// Loads content from a campaign directory structure
    ///
    /// Expected directory structure:
    /// ```text
    /// campaign_dir/
    /// ├── data/
    /// │   ├── classes.ron
    /// │   ├── races.ron
    /// │   ├── items.ron
    /// │   ├── monsters.ron
    /// │   ├── spells.ron
    /// │   ├── quests.ron
    /// │   ├── dialogues.ron
    /// │   └── maps/
    /// │       ├── map001.ron
    /// │       └── map002.ron
    /// ```
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` - Path to the campaign directory
    ///
    /// # Returns
    ///
    /// Returns `Ok(ContentDatabase)` if all content loads successfully.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if:
    /// - Campaign directory doesn't exist
    /// - Any content file fails to load
    /// - RON parsing errors occur
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
    /// println!("Loaded campaign with {} classes", db.stats().class_count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_campaign<P: AsRef<Path>>(campaign_dir: P) -> Result<Self, DatabaseError> {
        let campaign_path = campaign_dir.as_ref();

        if !campaign_path.exists() {
            return Err(DatabaseError::CampaignNotFound(campaign_path.to_path_buf()));
        }

        let data_dir = campaign_path.join("data");

        // Load classes
        let classes = if data_dir.join("classes.ron").exists() {
            ClassDatabase::load_from_file(data_dir.join("classes.ron"))
                .map_err(|e| DatabaseError::ClassLoadError(e.to_string()))?
        } else {
            ClassDatabase::new()
        };

        // Load races (Phase 2 - currently placeholder)
        let races = if data_dir.join("races.ron").exists() {
            RaceDatabase::load_from_file(data_dir.join("races.ron"))?
        } else {
            RaceDatabase::new()
        };

        // Load items
        let items = if data_dir.join("items.ron").exists() {
            ItemDatabase::load_from_file(data_dir.join("items.ron"))
                .map_err(|e| DatabaseError::ItemLoadError(e.to_string()))?
        } else {
            ItemDatabase::new()
        };

        // Load monsters (placeholder)
        let monsters = if data_dir.join("monsters.ron").exists() {
            MonsterDatabase::load_from_file(data_dir.join("monsters.ron"))?
        } else {
            MonsterDatabase::new()
        };

        // Load spells (placeholder)
        let spells = if data_dir.join("spells.ron").exists() {
            SpellDatabase::load_from_file(data_dir.join("spells.ron"))?
        } else {
            SpellDatabase::new()
        };

        // Load maps (placeholder)
        let maps = if data_dir.join("maps").exists() {
            MapDatabase::load_from_directory(data_dir.join("maps"))?
        } else {
            MapDatabase::new()
        };

        // Load quests
        let quests = if data_dir.join("quests.ron").exists() {
            QuestDatabase::load_from_file(data_dir.join("quests.ron"))?
        } else {
            QuestDatabase::new()
        };

        // Load dialogues
        let dialogues = if data_dir.join("dialogues.ron").exists() {
            DialogueDatabase::load_from_file(data_dir.join("dialogues.ron"))?
        } else {
            DialogueDatabase::new()
        };

        // Load conditions
        let conditions = if data_dir.join("conditions.ron").exists() {
            ConditionDatabase::load_from_file(data_dir.join("conditions.ron"))?
        } else {
            ConditionDatabase::new()
        };

        // Load characters
        let characters = if data_dir.join("characters.ron").exists() {
            CharacterDatabase::load_from_file(data_dir.join("characters.ron"))
                .map_err(|e| DatabaseError::CharacterLoadError(e.to_string()))?
        } else {
            CharacterDatabase::new()
        };

        Ok(Self {
            classes,
            races,
            items,
            monsters,
            spells,
            maps,
            quests,
            dialogues,
            conditions,
            characters,
        })
    }

    /// Loads core game content from the data directory
    ///
    /// This is a convenience wrapper around `load_campaign` for loading
    /// the base game content.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ContentDatabase::load_core("data")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_core<P: AsRef<Path>>(data_dir: P) -> Result<Self, DatabaseError> {
        let data_path = data_dir.as_ref();

        if !data_path.exists() {
            return Err(DatabaseError::CampaignNotFound(data_path.to_path_buf()));
        }

        // Load classes
        let classes = if data_path.join("classes.ron").exists() {
            ClassDatabase::load_from_file(data_path.join("classes.ron"))
                .map_err(|e| DatabaseError::ClassLoadError(e.to_string()))?
        } else {
            ClassDatabase::new()
        };

        // Load races (Phase 2)
        let races = if data_path.join("races.ron").exists() {
            RaceDatabase::load_from_file(data_path.join("races.ron"))?
        } else {
            RaceDatabase::new()
        };

        // Load items
        let items = if data_path.join("items.ron").exists() {
            ItemDatabase::load_from_file(data_path.join("items.ron"))
                .map_err(|e| DatabaseError::ItemLoadError(e.to_string()))?
        } else {
            ItemDatabase::new()
        };

        // Load monsters
        let monsters = if data_path.join("monsters.ron").exists() {
            MonsterDatabase::load_from_file(data_path.join("monsters.ron"))?
        } else {
            MonsterDatabase::new()
        };

        // Load spells
        let spells = if data_path.join("spells.ron").exists() {
            SpellDatabase::load_from_file(data_path.join("spells.ron"))?
        } else {
            SpellDatabase::new()
        };

        // Load maps
        let maps = if data_path.join("maps").exists() {
            MapDatabase::load_from_directory(data_path.join("maps"))?
        } else {
            MapDatabase::new()
        };

        // Load quests
        let quests = if data_path.join("quests.ron").exists() {
            QuestDatabase::load_from_file(data_path.join("quests.ron"))?
        } else {
            QuestDatabase::new()
        };

        // Load dialogues
        let dialogues = if data_path.join("dialogues.ron").exists() {
            DialogueDatabase::load_from_file(data_path.join("dialogues.ron"))?
        } else {
            DialogueDatabase::new()
        };

        // Load conditions
        let conditions = if data_path.join("conditions.ron").exists() {
            ConditionDatabase::load_from_file(data_path.join("conditions.ron"))?
        } else {
            ConditionDatabase::new()
        };

        // Load characters
        let characters = if data_path.join("characters.ron").exists() {
            CharacterDatabase::load_from_file(data_path.join("characters.ron"))
                .map_err(|e| DatabaseError::CharacterLoadError(e.to_string()))?
        } else {
            CharacterDatabase::new()
        };

        Ok(Self {
            classes,
            races,
            items,
            monsters,
            spells,
            maps,
            quests,
            dialogues,
            conditions,
            characters,
        })
    }

    /// Validates all content in the database
    ///
    /// Performs basic validation checks on all content types.
    /// For comprehensive cross-reference validation, use the `Validator` from
    /// the `validation` module.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ContentDatabase::load_core("data")?;
    /// db.validate()?;
    /// println!("All content is valid!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<(), DatabaseError> {
        // Validate classes
        self.classes
            .validate()
            .map_err(|e| DatabaseError::ValidationError(e.to_string()))?;

        // Validate dialogues
        self.dialogues
            .validate()
            .map_err(|e| DatabaseError::ValidationError(e.to_string()))?;

        // Cross-reference validation

        // Check quests
        for quest in self.quests.quests.values() {
            // Check quest stages objectives
            for stage in &quest.stages {
                for objective in &stage.objectives {
                    match objective {
                        crate::domain::quest::QuestObjective::KillMonsters {
                            monster_id, ..
                        } => {
                            if !self.monsters.has_monster(monster_id) {
                                return Err(DatabaseError::ValidationError(format!(
                                    "Quest '{}' references non-existent monster {}",
                                    quest.name, monster_id
                                )));
                            }
                        }
                        crate::domain::quest::QuestObjective::CollectItems { item_id, .. } => {
                            if !self.items.has_item(item_id) {
                                return Err(DatabaseError::ValidationError(format!(
                                    "Quest '{}' references non-existent item {}",
                                    quest.name, item_id
                                )));
                            }
                        }
                        crate::domain::quest::QuestObjective::DeliverItem { item_id, .. } => {
                            if !self.items.has_item(item_id) {
                                return Err(DatabaseError::ValidationError(format!(
                                    "Quest '{}' references non-existent item {}",
                                    quest.name, item_id
                                )));
                            }
                        }
                        crate::domain::quest::QuestObjective::ReachLocation { map_id, .. } => {
                            if !self.maps.has_map(map_id) {
                                return Err(DatabaseError::ValidationError(format!(
                                    "Quest '{}' references non-existent map {}",
                                    quest.name, map_id
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Validate character definitions
        self.characters
            .validate()
            .map_err(|e| DatabaseError::ValidationError(e.to_string()))?;

        // Validate character references against loaded races, classes, and items
        for character in self.characters.all_characters() {
            // Check race reference
            if !self.races.has_race(&character.race_id) {
                return Err(DatabaseError::ValidationError(format!(
                    "Character '{}' references non-existent race '{}'",
                    character.id, character.race_id
                )));
            }

            // Check class reference
            if self.classes.get_class(&character.class_id).is_none() {
                return Err(DatabaseError::ValidationError(format!(
                    "Character '{}' references non-existent class '{}'",
                    character.id, character.class_id
                )));
            }

            // Check starting items references
            for item_id in &character.starting_items {
                if !self.items.has_item(item_id) {
                    return Err(DatabaseError::ValidationError(format!(
                        "Character '{}' references non-existent starting item {}",
                        character.id, item_id
                    )));
                }
            }

            // Check starting equipment references
            for item_id in character.starting_equipment.all_item_ids() {
                if !self.items.has_item(&item_id) {
                    return Err(DatabaseError::ValidationError(format!(
                        "Character '{}' references non-existent equipment item {}",
                        character.id, item_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Returns statistics about loaded content
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// let db = ContentDatabase::new();
    /// let stats = db.stats();
    /// assert_eq!(stats.class_count, 0);
    /// assert_eq!(stats.item_count, 0);
    /// ```
    pub fn stats(&self) -> ContentStats {
        ContentStats {
            class_count: self.classes.all_classes().count(),
            race_count: self.races.len(),
            item_count: self.items.len(),
            monster_count: self.monsters.count(),
            spell_count: self.spells.count(),
            map_count: self.maps.count(),
            quest_count: self.quests.count(),
            dialogue_count: self.dialogues.count(),
            condition_count: self.conditions.count(),
            character_count: self.characters.len(),
        }
    }
}

impl Default for ContentDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Content Statistics =====

/// Statistics about loaded content in a database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::{ContentDatabase, ContentStats};
///
/// let db = ContentDatabase::new();
/// let stats = db.stats();
///
/// println!("Classes: {}", stats.class_count);
/// println!("Items: {}", stats.item_count);
/// println!("Total: {}", stats.total());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentStats {
    /// Number of class definitions
    pub class_count: usize,

    /// Number of race definitions
    pub race_count: usize,

    /// Number of item definitions
    pub item_count: usize,

    /// Number of monster definitions
    pub monster_count: usize,

    /// Number of spell definitions
    pub spell_count: usize,

    /// Number of map definitions
    pub map_count: usize,

    /// Number of quest definitions
    pub quest_count: usize,

    /// Number of dialogue definitions
    pub dialogue_count: usize,

    /// Number of condition definitions
    pub condition_count: usize,

    /// Number of character definitions
    pub character_count: usize,
}

impl ContentStats {
    /// Returns the total number of content items
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentStats;
    ///
    /// let stats = ContentStats {
    ///     class_count: 5,
    ///     race_count: 3,
    ///     item_count: 100,
    ///     monster_count: 50,
    ///     spell_count: 30,
    ///     map_count: 10,
    ///     quest_count: 20,
    ///     dialogue_count: 15,
    ///     condition_count: 10,
    ///     character_count: 8,
    /// };
    ///
    /// assert_eq!(stats.total(), 251);
    /// ```
    pub fn total(&self) -> usize {
        self.class_count
            + self.race_count
            + self.item_count
            + self.monster_count
            + self.spell_count
            + self.map_count
            + self.quest_count
            + self.dialogue_count
            + self.condition_count
            + self.character_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_database_new() {
        let db = ContentDatabase::new();
        let stats = db.stats();

        assert_eq!(stats.class_count, 0);
        assert_eq!(stats.race_count, 0);
        assert_eq!(stats.item_count, 0);
        assert_eq!(stats.monster_count, 0);
        assert_eq!(stats.spell_count, 0);
        assert_eq!(stats.map_count, 0);
        assert_eq!(stats.character_count, 0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_content_stats_total() {
        let stats = ContentStats {
            class_count: 5,
            race_count: 3,
            item_count: 100,
            monster_count: 50,
            spell_count: 30,
            map_count: 10,
            quest_count: 20,
            dialogue_count: 15,
            condition_count: 10,
            character_count: 8,
        };
        assert_eq!(stats.total(), 251);
    }

    #[test]
    fn test_race_database_new() {
        let db = RaceDatabase::new();
        assert_eq!(db.len(), 0);
        assert_eq!(db.all_races().count(), 0);
    }

    #[test]
    fn test_spell_database_new() {
        let db = SpellDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_spells().is_empty());
    }

    #[test]
    fn test_spell_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary RON file with test spells
        let mut temp_file = NamedTempFile::new().unwrap();
        let ron_data = r#"[
            (
                id: 1,
                name: "Magic Missile",
                school: Sorcerer,
                level: 1,
                sp_cost: 5,
                gem_cost: 0,
                context: CombatOnly,
                target: SingleMonster,
                description: "A bolt of magical energy",
                damage: Some((count: 1, sides: 4, bonus: 1)),
                duration: 0,
                saving_throw: false,
                applied_conditions: [],
            ),
            (
                id: 2,
                name: "Bless",
                school: Cleric,
                level: 1,
                sp_cost: 5,
                gem_cost: 0,
                context: Anytime,
                target: AllCharacters,
                description: "Blesses the party",
                damage: None,
                duration: 10,
                saving_throw: false,
                applied_conditions: ["bless"],
            ),
        ]"#;
        temp_file.write_all(ron_data.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test loading
        let db = SpellDatabase::load_from_file(temp_file.path()).unwrap();
        assert_eq!(db.count(), 2);
        assert!(db.has_spell(&1));
        assert!(db.has_spell(&2));

        // Test retrieval
        let spell1 = db.get_spell(1).unwrap();
        assert_eq!(spell1.name, "Magic Missile");
        assert_eq!(spell1.level, 1);

        let spell2 = db.get_spell(2).unwrap();
        assert_eq!(spell2.name, "Bless");
        assert_eq!(spell2.applied_conditions.len(), 1);
    }

    #[test]
    fn test_spell_database_load_nonexistent_file() {
        // Should return empty database for missing file
        let result = SpellDatabase::load_from_file("/nonexistent/path/spells.ron");
        assert!(result.is_ok());
        let db = result.unwrap();
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_monster_database_new() {
        let db = MonsterDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_monsters().is_empty());
    }

    #[test]
    fn test_monster_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary RON file with test monsters
        let mut temp_file = NamedTempFile::new().unwrap();
        let ron_data = r#"[
            (
                id: 1,
                name: "Goblin",
                stats: (
                    might: (current: 10, base: 10),
                    intellect: (current: 8, base: 8),
                    personality: (current: 8, base: 8),
                    endurance: (current: 10, base: 10),
                    speed: (current: 12, base: 12),
                    accuracy: (current: 10, base: 10),
                    luck: (current: 10, base: 10)
                ),
                hp: (current: 5, base: 5),
                ac: (current: 12, base: 12),
                attacks: [],
                loot: (
                    gold_min: 1,
                    gold_max: 5,
                    gems_min: 0,
                    gems_max: 0,
                    items: [],
                    experience: 10
                ),
                flee_threshold: 20,
                special_attack_threshold: 0,
                resistances: (physical: false, fire: false, cold: false, electricity: false, energy: false, paralysis: false, fear: false, sleep: false),
                can_regenerate: false,
                can_advance: true,
                is_undead: false,
                magic_resistance: 0,
                conditions: Normal,
                active_conditions: [],
                has_acted: false,
            ),
        ]"#;
        temp_file.write_all(ron_data.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test loading
        let db = MonsterDatabase::load_from_file(temp_file.path()).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.has_monster(&1));

        // Test retrieval
        let monster = db.get_monster(1).unwrap();
        assert_eq!(monster.name, "Goblin");
        assert_eq!(monster.hp.base, 5);
    }

    #[test]
    fn test_map_database_new() {
        let db = MapDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_maps().is_empty());
    }

    #[test]
    fn test_quest_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary RON file with test quests
        let mut temp_file = NamedTempFile::new().unwrap();
        let ron_data = r#"[
            (
                id: 1,
                name: "Rat Problem",
                description: "Kill the rats in the cellar.",
                stages: [
                    (
                        stage_number: 1,
                        name: "Kill Rats",
                        description: "Kill 5 rats",
                        objectives: [
                            KillMonsters(monster_id: 1, quantity: 5)
                        ],
                        require_all_objectives: true,
                    ),
                    (
                        stage_number: 2,
                        name: "Return",
                        description: "Return to innkeeper",
                        objectives: [
                            TalkToNpc(npc_id: 1, map_id: 1)
                        ],
                        require_all_objectives: true,
                    )
                ],
                rewards: [Experience(100), Gold(50)],
                min_level: Some(1),
                max_level: None,
                required_quests: [],
                repeatable: false,
                is_main_quest: false,
                quest_giver_npc: Some(1),
                quest_giver_map: Some(1),
                quest_giver_position: None,
            ),
        ]"#;
        temp_file.write_all(ron_data.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test loading
        let db = QuestDatabase::load_from_file(temp_file.path()).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.has_quest(&1));

        // Test retrieval
        let quest = db.get_quest(1).unwrap();
        assert_eq!(quest.name, "Rat Problem");
        assert_eq!(quest.stages.len(), 2);
    }

    #[test]
    fn test_dialogue_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary RON file with test dialogues
        let mut temp_file = NamedTempFile::new().unwrap();
        let ron_data = r#"[
            (
                id: 1,
                name: "Innkeeper Greeting",
                root_node: 1,
                nodes: {
                    1: (
                        id: 1,
                        text: "Welcome to the inn!",
                        speaker_override: None,
                        choices: [
                            (
                                text: "I need a room.",
                                target_node: Some(2),
                                conditions: [],
                                actions: [],
                                ends_dialogue: false,
                            ),
                            (
                                text: "Goodbye.",
                                target_node: None,
                                conditions: [],
                                actions: [],
                                ends_dialogue: true,
                            )
                        ],
                        conditions: [],
                        actions: [],
                        is_terminal: false,
                    ),
                    2: (
                        id: 2,
                        text: "That will be 10 gold.",
                        speaker_override: None,
                        choices: [
                            (
                                text: "Here is the gold.",
                                target_node: None,
                                conditions: [],
                                actions: [],
                                ends_dialogue: true,
                            )
                        ],
                        conditions: [],
                        actions: [],
                        is_terminal: true,
                    )
                },
                speaker_name: Some("Innkeeper"),
                repeatable: true,
                associated_quest: None,
            ),
        ]"#;
        temp_file.write_all(ron_data.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test loading
        let db = DialogueDatabase::load_from_file(temp_file.path()).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.has_dialogue(&1));

        // Test retrieval
        let dialogue = db.get_dialogue(1).unwrap();
        assert_eq!(dialogue.name, "Innkeeper Greeting");
        assert_eq!(dialogue.nodes.len(), 2);
    }

    #[test]
    fn test_content_database_default() {
        let db = ContentDatabase::default();
        assert_eq!(db.stats().total(), 0);
    }

    #[test]
    fn test_content_database_validate_empty() {
        let db = ContentDatabase::new();
        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_condition_database_new() {
        let db = ConditionDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_conditions().is_empty());
    }

    #[test]
    fn test_condition_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary RON file with test conditions
        let mut temp_file = NamedTempFile::new().unwrap();
        let ron_data = r#"[
            (
                id: "poisoned",
                name: "Poisoned",
                description: "Taking damage over time from poison",
                effects: [
                    DamageOverTime(
                        damage: (count: 1, sides: 4, bonus: 0),
                        element: "poison",
                    ),
                ],
                default_duration: Rounds(3),
                icon_id: None,
            ),
            (
                id: "blessed",
                name: "Blessed",
                description: "Receiving divine protection",
                effects: [
                    AttributeModifier(
                        attribute: "luck",
                        value: 5,
                    ),
                ],
                default_duration: Rounds(10),
                icon_id: None,
            ),
        ]"#;
        temp_file.write_all(ron_data.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test loading
        let db = ConditionDatabase::load_from_file(temp_file.path()).unwrap();
        assert_eq!(db.count(), 2);
        assert!(db.has_condition(&"poisoned".to_string()));
        assert!(db.has_condition(&"blessed".to_string()));

        // Test retrieval
        let poisoned = db.get_condition(&"poisoned".to_string()).unwrap();
        assert_eq!(poisoned.name, "Poisoned");
        assert_eq!(poisoned.effects.len(), 1);

        let blessed = db.get_condition(&"blessed".to_string()).unwrap();
        assert_eq!(blessed.name, "Blessed");
    }

    #[test]
    fn test_condition_database_load_nonexistent_file() {
        let db = ConditionDatabase::load_from_file("nonexistent_file.ron").unwrap();
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_content_database_character_loading() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Create a ContentDatabase and add characters manually
        let mut db = ContentDatabase::new();

        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        db.characters.add_character(knight).unwrap();

        let stats = db.stats();
        assert_eq!(stats.character_count, 1);
        assert!(db.characters.get_character("test_knight").is_some());
    }

    #[test]
    fn test_content_database_load_core_characters() {
        // Test loading core characters from data directory
        let result = ContentDatabase::load_core("data");

        // Should succeed if data directory exists with characters.ron
        if let Ok(db) = result {
            // If characters.ron exists, we should have loaded characters
            let stats = db.stats();
            // Characters count depends on whether file exists
            println!("Loaded {} characters from core data", stats.character_count);
        }
    }

    #[test]
    fn test_content_database_load_campaign_characters() {
        // Test loading campaign characters
        let result = ContentDatabase::load_campaign("campaigns/tutorial");

        if let Ok(db) = result {
            let stats = db.stats();
            println!(
                "Loaded {} characters from tutorial campaign",
                stats.character_count
            );
        }
    }

    #[test]
    fn test_content_database_validate_with_characters() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add a valid race
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add a valid class
        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character that references the valid race and class
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        // Validation should pass
        let result = db.validate();
        assert!(
            result.is_ok(),
            "Validation should pass with valid references"
        );
    }

    #[test]
    fn test_content_database_validate_invalid_race_reference() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;

        let mut db = ContentDatabase::new();

        // Add a valid class but NO races
        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character that references a non-existent race
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(), // This race doesn't exist
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        // Validation should fail due to missing race
        let result = db.validate();
        assert!(
            result.is_err(),
            "Validation should fail with invalid race reference"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("race"), "Error should mention race");
        assert!(
            err_msg.contains("human"),
            "Error should mention the invalid race id"
        );
    }

    #[test]
    fn test_content_database_validate_invalid_class_reference() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add a valid race but NO classes
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add a character that references a non-existent class
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(), // This class doesn't exist
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        // Validation should fail due to missing class
        let result = db.validate();
        assert!(
            result.is_err(),
            "Validation should fail with invalid class reference"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("class"), "Error should mention class");
        assert!(
            err_msg.contains("knight"),
            "Error should mention the invalid class id"
        );
    }

    #[test]
    fn test_content_database_validate_invalid_item_reference() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add valid race and class
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character with invalid starting items
        let mut knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_items = vec![255]; // Item ID that doesn't exist
        db.characters.add_character(knight).unwrap();

        // Validation should fail due to missing item
        let result = db.validate();
        assert!(
            result.is_err(),
            "Validation should fail with invalid item reference"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("item"), "Error should mention item");
    }

    #[test]
    fn test_content_stats_includes_characters() {
        let stats = ContentStats {
            class_count: 6,
            race_count: 6,
            item_count: 50,
            monster_count: 20,
            spell_count: 40,
            map_count: 5,
            quest_count: 10,
            dialogue_count: 8,
            condition_count: 12,
            character_count: 9,
        };

        // Total should include character_count
        assert_eq!(stats.total(), 166);
        assert_eq!(stats.character_count, 9);
    }
}
