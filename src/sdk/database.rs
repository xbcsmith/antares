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
//! See `docs/explanation/sdk_implementation_plan.md` for specifications.
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
use crate::domain::database_common::load_ron_entries;
use crate::domain::dialogue::{DialogueId, DialogueTree};
use crate::domain::items::ItemDatabase;
use crate::domain::magic::types::Spell;
use crate::domain::quest::{Quest, QuestId};
use crate::domain::races::{RaceDatabase, RaceError};
use crate::domain::types::{MapId, MonsterId, SpellId};
use crate::domain::world::furniture::FurnitureDatabase;
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

    #[error("Failed to load NPCs: {0}")]
    NpcLoadError(String),

    #[error("Failed to load characters: {0}")]
    CharacterLoadError(String),

    #[error("Failed to load creatures: {0}")]
    CreatureLoadError(String),

    #[error("Failed to load NPC stock templates: {0}")]
    NpcStockTemplateLoadError(String),

    #[error("Failed to load furniture: {0}")]
    FurnitureLoadError(String),

    #[error("Failed to load map {map_id}: {error}")]
    MapLoadError { map_id: String, error: String },

    #[error("Campaign not found: {0:?}")]
    CampaignNotFound(PathBuf),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("RON parsing error: {0}")]
    RonError(#[from] ron::error::SpannedError),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<RaceError> for DatabaseError {
    fn from(err: RaceError) -> Self {
        DatabaseError::RaceLoadError(err.to_string())
    }
}

// ===== Race System =====
// Race types are now imported from crate::domain::races module.
// See docs/explanation/hardcoded_removal_implementation_plan.md

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

    /// Adds a spell to the database
    ///
    /// This helper is used by tooling and tests that build up a content database
    /// programmatically. It inserts or replaces the spell entry keyed by ID.
    pub fn add_spell(&mut self, spell: Spell) -> Result<(), DatabaseError> {
        self.spells.insert(spell.id, spell);
        Ok(())
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
    /// let db = SpellDatabase::load_from_file("data/test_campaign/data/spells.ron")?;
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

        let spells = load_ron_entries(
            &contents,
            |s: &Spell| s.id,
            |id| DatabaseError::SpellLoadError(format!("Duplicate spell ID: {}", id)),
            |e| DatabaseError::SpellLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { spells })
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
    /// let db = MonsterDatabase::load_from_file("data/test_campaign/data/monsters.ron")?;
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

        let monsters = load_ron_entries(
            &contents,
            |m: &Monster| m.id,
            |id| DatabaseError::MonsterLoadError(format!("Duplicate monster ID: {}", id)),
            |e| DatabaseError::MonsterLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { monsters })
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

    /// Adds a monster definition to the SDK database by converting it into a runtime
    /// `Monster` instance. This helper is intended for tests and tooling that
    /// construct content databases programmatically.
    pub fn add_monster(
        &mut self,
        def: crate::domain::combat::database::MonsterDefinition,
    ) -> Result<(), DatabaseError> {
        self.monsters.insert(def.id, def.to_monster());
        Ok(())
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
                // Read file contents, but be tolerant of read errors and continue on failure
                let contents = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: failed to read map file {}: {}", path.display(), e);
                        // Skip unreadable map files instead of failing the whole load
                        continue;
                    }
                };

                // Try to load as Map (Engine/SDK format) first
                if let Ok(map) = ron::from_str::<Map>(&contents) {
                    maps.insert(map.id, map);
                    continue;
                }

                // Fallback to MapBlueprint; if parsing fails, log and skip the file
                match ron::from_str::<MapBlueprint>(&contents) {
                    Ok(blueprint) => {
                        let map: Map = blueprint.into();
                        maps.insert(map.id, map);
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to parse map file {}: {}. Skipping this map.",
                            path.display(),
                            e
                        );
                        // Do not treat a single bad map as fatal for campaign loading
                        continue;
                    }
                }
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
    /// let db = QuestDatabase::load_from_file("data/test_campaign/data/quests.ron")?;
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

        let quests = load_ron_entries(
            &contents,
            |q: &Quest| q.id,
            |id| DatabaseError::QuestLoadError(format!("Duplicate quest ID: {}", id)),
            |e| DatabaseError::QuestLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { quests })
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
    /// Creates a condition database pre-populated with the two system conditions.
    ///
    /// Every campaign must contain `"unconscious"` and `"dead"` for HP-damage
    /// and death-state handling to work correctly. Pre-populating them here
    /// ensures that any code path that constructs a `ContentDatabase`
    /// programmatically (SDK tooling, unit tests) has them available by
    /// default.
    ///
    /// Campaigns that load their own `conditions.ron` file will replace these
    /// defaults with the file contents via [`load_from_file`], so existing RON
    /// files that define `"unconscious"` and `"dead"` are not affected.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ConditionDatabase;
    ///
    /// let db = ConditionDatabase::new();
    /// // System conditions are always present.
    /// assert!(db.has_condition(&"unconscious".to_string()));
    /// assert!(db.has_condition(&"dead".to_string()));
    /// ```
    pub fn new() -> Self {
        use crate::domain::conditions::{ConditionDuration, ConditionEffect};

        let mut db = Self {
            conditions: HashMap::new(),
        };

        // Pre-populate the two system conditions that every campaign requires.
        // These defaults are overwritten when a campaign loads its own
        // conditions.ron (which should also contain these entries).
        let system_conditions = vec![
            ConditionDefinition {
                id: "unconscious".to_string(),
                name: "Unconscious".to_string(),
                description: "Character is at 0 HP and cannot act. \
                     Revived by healing above 0 HP or by resting."
                    .to_string(),
                effects: vec![ConditionEffect::StatusEffect("unconscious".to_string())],
                default_duration: ConditionDuration::Permanent,
                icon_id: Some("icon_unconscious".to_string()),
            },
            ConditionDefinition {
                id: "dead".to_string(),
                name: "Dead".to_string(),
                description: "Character is dead. Requires resurrection to revive. \
                     Cannot act, be targeted, or be healed by rest."
                    .to_string(),
                effects: vec![ConditionEffect::StatusEffect("dead".to_string())],
                default_duration: ConditionDuration::Permanent,
                icon_id: Some("icon_dead".to_string()),
            },
        ];

        for c in system_conditions {
            db.conditions.insert(c.id.clone(), c);
        }

        db
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
    /// let db = ConditionDatabase::load_from_file("data/test_campaign/data/conditions.ron")?;
    /// println!("Loaded {} conditions", db.count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Return a truly empty database if the file doesn't exist.
        // We intentionally do NOT call Self::new() here so that the
        // pre-populated system conditions are absent — callers can then
        // detect the missing file via ContentDatabase::validate() rather
        // than silently operating with stale defaults.
        if !path.exists() {
            return Ok(Self {
                conditions: HashMap::new(),
            });
        }

        // Read and parse RON file
        let contents = std::fs::read_to_string(path).map_err(|e| {
            DatabaseError::ConditionLoadError(format!("Failed to read file: {}", e))
        })?;

        let conditions = load_ron_entries(
            &contents,
            |c: &ConditionDefinition| c.id.clone(),
            |id| DatabaseError::ConditionLoadError(format!("Duplicate condition ID: {}", id)),
            |e| DatabaseError::ConditionLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { conditions })
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
    /// let db = DialogueDatabase::load_from_file("data/test_campaign/data/dialogues.ron")?;
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

        let dialogues = load_ron_entries(
            &contents,
            |d: &DialogueTree| d.id,
            |id| DatabaseError::DialogueLoadError(format!("Duplicate dialogue ID: {}", id)),
            |e| DatabaseError::DialogueLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { dialogues })
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

// ===== NPC Database =====

/// Database of NPC definitions
///
/// NPCs are loaded from `npcs.ron` files and referenced by string ID.
/// The same NPC definition can be placed on multiple maps.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::database::NpcDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = NpcDatabase::load_from_file("data/test_campaign/data/npcs.ron")?;
/// println!("Loaded {} NPCs", db.count());
///
/// if let Some(npc) = db.get_npc("village_elder") {
///     println!("Found NPC: {}", npc.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct NpcDatabase {
    npcs: HashMap<crate::domain::world::NpcId, crate::domain::world::NpcDefinition>,
}

impl NpcDatabase {
    /// Creates an empty NPC database
    pub fn new() -> Self {
        Self {
            npcs: HashMap::new(),
        }
    }

    /// Adds an NPC to the database
    ///
    /// This helper is used by tooling and tests that build up a content database
    /// programmatically. It inserts or replaces the NPC entry keyed by ID.
    pub fn add_npc(
        &mut self,
        npc: crate::domain::world::NpcDefinition,
    ) -> Result<(), DatabaseError> {
        self.npcs.insert(npc.id.clone(), npc);
        Ok(())
    }

    /// Loads NPCs from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing NPC definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(NpcDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NpcLoadError` if file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::database::NpcDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = NpcDatabase::load_from_file("data/test_campaign/data/npcs.ron")?;
    /// println!("Loaded {} NPCs", db.count());
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
            .map_err(|e| DatabaseError::NpcLoadError(format!("Failed to read file: {}", e)))?;

        let npcs = load_ron_entries(
            &contents,
            |n: &crate::domain::world::NpcDefinition| n.id.clone(),
            |id| DatabaseError::NpcLoadError(format!("Duplicate NPC ID: {}", id)),
            |e| DatabaseError::NpcLoadError(format!("Failed to parse RON: {}", e)),
        )?;

        Ok(Self { npcs })
    }

    /// Gets an NPC by ID
    pub fn get_npc(&self, id: &str) -> Option<&crate::domain::world::NpcDefinition> {
        self.npcs.get(id)
    }

    /// Returns all NPC IDs
    pub fn all_npcs(&self) -> Vec<String> {
        self.npcs.keys().cloned().collect()
    }

    /// Returns the number of NPCs
    pub fn count(&self) -> usize {
        self.npcs.len()
    }

    /// Checks if an NPC exists in the database
    pub fn has_npc(&self, id: &str) -> bool {
        self.npcs.contains_key(id)
    }

    /// Gets an NPC by name (case-insensitive)
    pub fn get_npc_by_name(&self, name: &str) -> Option<&crate::domain::world::NpcDefinition> {
        let name_lower = name.to_lowercase();
        self.npcs
            .values()
            .find(|n| n.name.to_lowercase() == name_lower)
    }

    /// Returns all merchant NPCs
    pub fn merchants(&self) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs.values().filter(|n| n.is_merchant).collect()
    }

    /// Returns all innkeeper NPCs
    pub fn innkeepers(&self) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs.values().filter(|n| n.is_innkeeper).collect()
    }

    /// Returns all priest NPCs
    pub fn priests(&self) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs.values().filter(|n| n.is_priest).collect()
    }

    /// Returns NPCs that give quests
    pub fn quest_givers(&self) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs.values().filter(|n| n.gives_quests()).collect()
    }

    /// Returns NPCs associated with a specific quest
    pub fn npcs_for_quest(&self, quest_id: QuestId) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs
            .values()
            .filter(|n| n.quest_ids.contains(&quest_id))
            .collect()
    }

    /// Returns NPCs by faction
    pub fn npcs_by_faction(&self, faction: &str) -> Vec<&crate::domain::world::NpcDefinition> {
        self.npcs
            .values()
            .filter(|n| {
                if let Some(npc_faction) = &n.faction {
                    npc_faction == faction
                } else {
                    false
                }
            })
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

    /// NPC definitions database
    pub npcs: NpcDatabase,

    /// Creature visual definitions database
    pub creatures: crate::domain::visual::creature_database::CreatureDatabase,

    /// Merchant stock template database
    ///
    /// Loaded from `npc_stock_templates.ron` in the data or campaign directory.
    /// Used to initialize the runtime stock for merchant NPCs when a session begins.
    pub npc_stock_templates: crate::domain::world::npc_runtime::MerchantStockTemplateDatabase,

    /// Furniture definition database — named, reusable furniture templates
    ///
    /// Loaded from `data/furniture.ron` in the campaign directory.
    /// Missing file is not an error — furniture support is opt-in per campaign.
    pub furniture: FurnitureDatabase,
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
            npcs: NpcDatabase::new(),
            creatures: crate::domain::visual::creature_database::CreatureDatabase::new(),
            npc_stock_templates:
                crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new(),
            furniture: FurnitureDatabase::new(),
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
    /// │   ├── npcs.ron
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

        // Load races (currently placeholder)
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

        // Load NPCs
        let npcs = if data_dir.join("npcs.ron").exists() {
            NpcDatabase::load_from_file(data_dir.join("npcs.ron"))?
        } else {
            NpcDatabase::new()
        };

        // Load NPC stock templates
        let npc_stock_templates = if data_dir.join("npc_stock_templates.ron").exists() {
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
                data_dir.join("npc_stock_templates.ron"),
            )
            .map_err(|e| DatabaseError::NpcStockTemplateLoadError(e.to_string()))?
        } else {
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new()
        };

        // Load creatures
        let creatures = if data_dir.join("creatures.ron").exists() {
            crate::domain::visual::creature_database::CreatureDatabase::load_from_registry(
                &data_dir.join("creatures.ron"),
                campaign_path,
            )
            .map_err(|e| DatabaseError::CreatureLoadError(e.to_string()))?
        } else {
            crate::domain::visual::creature_database::CreatureDatabase::new()
        };

        // Load furniture definitions (opt-in per campaign; missing file is not an error)
        let furniture = if data_dir.join("furniture.ron").exists() {
            FurnitureDatabase::load_from_file(data_dir.join("furniture.ron"))
                .map_err(|e| DatabaseError::FurnitureLoadError(e.to_string()))?
        } else {
            FurnitureDatabase::new()
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
            npcs,
            npc_stock_templates,
            creatures,
            furniture,
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

        // Load races
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

        // Load NPCs
        let data_path = data_dir.as_ref();
        let npcs = if data_path.join("npcs.ron").exists() {
            NpcDatabase::load_from_file(data_path.join("npcs.ron"))?
        } else {
            NpcDatabase::new()
        };

        // Load NPC stock templates
        let npc_stock_templates = if data_path.join("npc_stock_templates.ron").exists() {
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
                data_path.join("npc_stock_templates.ron"),
            )
            .map_err(|e| DatabaseError::NpcStockTemplateLoadError(e.to_string()))?
        } else {
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new()
        };

        // Load creatures
        let creatures = if data_path.join("creatures.ron").exists() {
            let root_path = data_path.parent().unwrap_or(data_path);
            crate::domain::visual::creature_database::CreatureDatabase::load_from_registry(
                &data_path.join("creatures.ron"),
                root_path,
            )
            .map_err(|e| DatabaseError::CreatureLoadError(e.to_string()))?
        } else {
            crate::domain::visual::creature_database::CreatureDatabase::new()
        };

        // Load furniture definitions (opt-in per campaign; missing file is not an error)
        let furniture = if data_path.join("furniture.ron").exists() {
            FurnitureDatabase::load_from_file(data_path.join("furniture.ron"))
                .map_err(|e| DatabaseError::FurnitureLoadError(e.to_string()))?
        } else {
            FurnitureDatabase::new()
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
            npcs,
            npc_stock_templates,
            creatures,
            furniture,
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

        // System conditions: "unconscious" and "dead" must always be present.
        // Their absence causes silent bugs — characters at 0 HP will not
        // receive the UNCONSCIOUS condition, and dead characters will not be
        // recognised by rest / resurrection logic.
        for required_id in &["unconscious", "dead"] {
            if !self.conditions.has_condition(&required_id.to_string()) {
                return Err(DatabaseError::ValidationError(format!(
                    "Campaign is missing required system condition '{}'. \
                     Characters at 0 HP will not behave correctly.",
                    required_id
                )));
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
            class_count: self.classes.count(),
            race_count: self.races.count(),
            item_count: self.items.len(),
            monster_count: self.monsters.count(),
            spell_count: self.spells.count(),
            map_count: self.maps.count(),
            quest_count: self.quests.count(),
            dialogue_count: self.dialogues.count(),
            condition_count: self.conditions.count(),
            character_count: self.characters.len(),
            npc_count: self.npcs.count(),
            creature_count: self.creatures.count(),
            npc_stock_template_count: self.npc_stock_templates.len(),
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

    /// Number of NPC definitions
    pub npc_count: usize,

    /// Number of creature visual definitions
    pub creature_count: usize,

    /// Number of merchant stock template definitions
    pub npc_stock_template_count: usize,
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
    ///     npc_count: 12,
    ///     creature_count: 0,
    ///     npc_stock_template_count: 0,
    /// };
    /// assert_eq!(stats.total(), 263);
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
            + self.npc_count
            + self.creature_count
            + self.npc_stock_template_count
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
        // ConditionDatabase::new() pre-populates "unconscious" and "dead".
        // Every other sub-database starts empty, so total equals 2.
        assert_eq!(
            stats.condition_count, 2,
            "new() must pre-populate 2 system conditions"
        );
        assert_eq!(
            stats.total(),
            2,
            "total() must be 2 (only the 2 pre-populated system conditions)"
        );
    }

    #[test]
    fn test_content_stats_total() {
        let stats = ContentStats {
            class_count: 6,
            race_count: 5,
            item_count: 0,
            monster_count: 0,
            spell_count: 0,
            map_count: 0,
            quest_count: 0,
            dialogue_count: 0,
            condition_count: 0,
            character_count: 6,
            npc_count: 0,
            creature_count: 0,
            npc_stock_template_count: 0,
        };
        assert_eq!(stats.total(), 17);
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
                            TalkToNpc(npc_id: "1", map_id: 1)
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
                quest_giver_npc: Some("1"),
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
        // ContentDatabase::default() delegates to new(), which pre-populates
        // the two system conditions ("unconscious" and "dead") via
        // ConditionDatabase::new(). All other content databases start empty.
        let stats = db.stats();
        assert_eq!(
            stats.condition_count, 2,
            "new() must pre-populate 2 system conditions"
        );
        assert_eq!(stats.class_count, 0);
        assert_eq!(stats.race_count, 0);
        assert_eq!(stats.item_count, 0);
        assert_eq!(stats.monster_count, 0);
        assert_eq!(stats.spell_count, 0);
        assert_eq!(stats.map_count, 0);
        assert_eq!(stats.quest_count, 0);
        assert_eq!(stats.dialogue_count, 0);
        assert_eq!(stats.character_count, 0);
        assert_eq!(stats.npc_count, 0);
        assert_eq!(stats.creature_count, 0);
    }

    #[test]
    fn test_content_database_validate_empty() {
        let db = ContentDatabase::new();
        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_condition_database_new() {
        let db = ConditionDatabase::new();
        // ConditionDatabase::new() pre-populates the two system conditions.
        assert_eq!(
            db.count(),
            2,
            "new() must pre-populate 'unconscious' and 'dead'"
        );
        assert!(db.has_condition(&"unconscious".to_string()));
        assert!(db.has_condition(&"dead".to_string()));
    }

    /// `ConditionDatabase::new()` must contain `"unconscious"` so that
    /// any programmatically-created database is immediately valid.
    #[test]
    fn test_default_condition_database_includes_unconscious() {
        let db = ConditionDatabase::new();
        assert!(
            db.has_condition(&"unconscious".to_string()),
            "ConditionDatabase::new() must pre-populate 'unconscious'"
        );
        let cond = db.get_condition(&"unconscious".to_string()).unwrap();
        assert_eq!(cond.name, "Unconscious");
    }

    /// `ConditionDatabase::new()` must contain `"dead"` so that
    /// any programmatically-created database is immediately valid.
    #[test]
    fn test_default_condition_database_includes_dead() {
        let db = ConditionDatabase::new();
        assert!(
            db.has_condition(&"dead".to_string()),
            "ConditionDatabase::new() must pre-populate 'dead'"
        );
        let cond = db.get_condition(&"dead".to_string()).unwrap();
        assert_eq!(cond.name, "Dead");
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
        // load_from_file returns an *empty* database (not pre-populated) when
        // the file does not exist, so that callers can detect the missing file
        // via validate() rather than silently using stale defaults.
        let db = ConditionDatabase::load_from_file("nonexistent_file.ron").unwrap();
        assert_eq!(db.count(), 0);
    }

    /// `ContentDatabase::validate()` must return an error when the
    /// `"unconscious"` system condition is absent from the database.
    #[test]
    fn test_validate_warns_missing_unconscious() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Build a conditions file that has "dead" but NOT "unconscious".
        let mut tmp = NamedTempFile::new().unwrap();
        let ron = r#"[
            (
                id: "dead",
                name: "Dead",
                description: "Character is dead.",
                effects: [StatusEffect("dead")],
                default_duration: Permanent,
                icon_id: None,
            ),
        ]"#;
        tmp.write_all(ron.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let cond_db = ConditionDatabase::load_from_file(tmp.path()).unwrap();
        assert_eq!(cond_db.count(), 1, "fixture must contain only 'dead'");

        // Replace the conditions database so "unconscious" is absent.
        let mut db = ContentDatabase::new();
        db.conditions = cond_db;

        let result = db.validate();
        assert!(
            result.is_err(),
            "validate() must fail when 'unconscious' is missing"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unconscious"),
            "error must mention the missing condition id; got: {msg}"
        );
    }

    /// `ContentDatabase::validate()` must return an error when the
    /// `"dead"` system condition is absent from the database.
    #[test]
    fn test_validate_warns_missing_dead() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Build a conditions file that has "unconscious" but NOT "dead".
        let mut tmp = NamedTempFile::new().unwrap();
        let ron = r#"[
            (
                id: "unconscious",
                name: "Unconscious",
                description: "Character is at 0 HP.",
                effects: [StatusEffect("unconscious")],
                default_duration: Permanent,
                icon_id: None,
            ),
        ]"#;
        tmp.write_all(ron.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let cond_db = ConditionDatabase::load_from_file(tmp.path()).unwrap();
        assert_eq!(
            cond_db.count(),
            1,
            "fixture must contain only 'unconscious'"
        );

        // Replace the conditions database so "dead" is absent.
        let mut db = ContentDatabase::new();
        db.conditions = cond_db;

        let result = db.validate();
        assert!(
            result.is_err(),
            "validate() must fail when 'dead' is missing"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("dead"),
            "error must mention the missing condition id; got: {msg}"
        );
    }

    /// Loading `data/test_campaign/data/spells.ron` must yield at least one
    /// spell that has `resurrect_hp: Some(1)` — the "Resurrect" entry added
    /// previously.
    #[test]
    fn test_resurrect_spell_loads_from_test_campaign() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/test_campaign/data/spells.ron");
        let db =
            SpellDatabase::load_from_file(&path).expect("should load spells from test_campaign");

        // Spell ID 776 is the "Resurrect" entry.
        let spell = db
            .get_spell(776)
            .expect("test_campaign/data/spells.ron must contain spell with id 776 (Resurrect)");

        assert_eq!(spell.name, "Resurrect");
        assert_eq!(
            spell.resurrect_hp,
            Some(1),
            "Resurrect must have resurrect_hp: Some(1)"
        );
        assert_eq!(spell.level, 5, "Resurrect must be level 5");
        assert_eq!(spell.gem_cost, 2, "Resurrect must cost 2 gems");
    }

    #[test]
    fn test_unconscious_condition_in_ron_loaded() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/test_campaign/data/conditions.ron");
        let db = ConditionDatabase::load_from_file(&path)
            .expect("should load conditions from test_campaign");
        assert!(
            db.has_condition(&"unconscious".to_string()),
            "conditions.ron must contain an 'unconscious' entry"
        );
    }

    /// `ConditionDatabase` loaded from `data/test_campaign/data/conditions.ron`
    /// must contain a `"dead"` entry.
    #[test]
    fn test_dead_condition_in_ron_loaded() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/test_campaign/data/conditions.ron");
        let db = ConditionDatabase::load_from_file(&path)
            .expect("should load conditions from test_campaign");
        assert!(
            db.has_condition(&"dead".to_string()),
            "conditions.ron must contain a 'dead' entry"
        );
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
        let result = ContentDatabase::load_campaign("data/test_campaign");

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
            character_count: 6,
            npc_count: 0,
            creature_count: 0,
            npc_stock_template_count: 0,
        };

        // Total should include character_count, npc_count, and creature_count
        assert_eq!(stats.total(), 163);
        assert_eq!(stats.character_count, 6);
        assert_eq!(stats.npc_count, 0);
        assert_eq!(stats.creature_count, 0);
    }

    #[test]
    fn test_npc_database_new() {
        let db = NpcDatabase::new();
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_npc_database_add_npc() {
        let mut db = NpcDatabase::new();

        let npc = crate::domain::world::NpcDefinition::new("test_npc", "Test NPC", "test.png");

        db.add_npc(npc.clone()).expect("Failed to add NPC");
        assert_eq!(db.count(), 1);
        assert!(db.has_npc("test_npc"));
    }

    #[test]
    fn test_npc_database_get_npc() {
        let mut db = NpcDatabase::new();

        let npc = crate::domain::world::NpcDefinition {
            id: "village_elder".to_string(),
            name: "Elder Theron".to_string(),
            description: "The wise village elder".to_string(),
            portrait_id: "elder.png".to_string(),
            dialogue_id: Some(1),
            creature_id: None,
            sprite: None,
            quest_ids: vec![1, 2],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        };

        db.add_npc(npc.clone()).expect("Failed to add NPC");

        let retrieved = db.get_npc("village_elder").expect("NPC not found");
        assert_eq!(retrieved.id, "village_elder");
        assert_eq!(retrieved.name, "Elder Theron");
        assert_eq!(retrieved.dialogue_id, Some(1));
        assert_eq!(retrieved.quest_ids, vec![1, 2]);
    }

    #[test]
    fn test_npc_database_get_npc_not_found() {
        let db = NpcDatabase::new();
        assert!(db.get_npc("nonexistent").is_none());
    }

    #[test]
    fn test_npc_database_get_npc_by_name() {
        let mut db = NpcDatabase::new();

        let npc =
            crate::domain::world::NpcDefinition::new("merchant_1", "Merchant Bob", "merchant.png");

        db.add_npc(npc).expect("Failed to add NPC");

        let retrieved = db.get_npc_by_name("Merchant Bob").expect("NPC not found");
        assert_eq!(retrieved.id, "merchant_1");

        // Case insensitive
        let retrieved = db.get_npc_by_name("merchant bob").expect("NPC not found");
        assert_eq!(retrieved.id, "merchant_1");
    }

    #[test]
    fn test_npc_database_merchants() {
        let mut db = NpcDatabase::new();

        let merchant = crate::domain::world::NpcDefinition::merchant(
            "merchant_1",
            "Bob's Shop",
            "merchant.png",
        );

        let guard = crate::domain::world::NpcDefinition::new("guard_1", "City Guard", "guard.png");

        db.add_npc(merchant).expect("Failed to add merchant");
        db.add_npc(guard).expect("Failed to add guard");

        let merchants = db.merchants();
        assert_eq!(merchants.len(), 1);
        assert_eq!(merchants[0].id, "merchant_1");
    }

    #[test]
    fn test_npc_database_innkeepers() {
        let mut db = NpcDatabase::new();

        let innkeeper =
            crate::domain::world::NpcDefinition::innkeeper("inn_1", "Mary's Inn", "innkeeper.png");

        let merchant =
            crate::domain::world::NpcDefinition::merchant("merchant_1", "Shop", "merchant.png");

        db.add_npc(innkeeper).expect("Failed to add innkeeper");
        db.add_npc(merchant).expect("Failed to add merchant");

        let innkeepers = db.innkeepers();
        assert_eq!(innkeepers.len(), 1);
        assert_eq!(innkeepers[0].id, "inn_1");
    }

    #[test]
    fn test_npc_database_priests() {
        let mut db = NpcDatabase::new();

        let priest =
            crate::domain::world::NpcDefinition::priest("priest_1", "Father Alaric", "priest.png");

        let merchant =
            crate::domain::world::NpcDefinition::merchant("merchant_1", "Shop", "merchant.png");

        let guard = crate::domain::world::NpcDefinition::new("guard_1", "City Guard", "guard.png");

        db.add_npc(priest).expect("Failed to add priest");
        db.add_npc(merchant).expect("Failed to add merchant");
        db.add_npc(guard).expect("Failed to add guard");

        let priests = db.priests();
        assert_eq!(priests.len(), 1);
        assert_eq!(priests[0].id, "priest_1");
        assert!(priests[0].is_priest);
    }

    #[test]
    fn test_npc_database_quest_givers() {
        let mut db = NpcDatabase::new();

        let mut quest_giver =
            crate::domain::world::NpcDefinition::new("elder", "Village Elder", "elder.png");
        quest_giver.quest_ids = vec![1, 2];

        let regular_npc = crate::domain::world::NpcDefinition::new("guard", "Guard", "guard.png");

        db.add_npc(quest_giver).expect("Failed to add quest giver");
        db.add_npc(regular_npc).expect("Failed to add regular NPC");

        let quest_givers = db.quest_givers();
        assert_eq!(quest_givers.len(), 1);
        assert_eq!(quest_givers[0].id, "elder");
    }

    #[test]
    fn test_npc_database_npcs_for_quest() {
        let mut db = NpcDatabase::new();

        let mut npc1 = crate::domain::world::NpcDefinition::new("elder", "Elder", "elder.png");
        npc1.quest_ids = vec![1, 2];

        let mut npc2 = crate::domain::world::NpcDefinition::new("priest", "Priest", "priest.png");
        npc2.quest_ids = vec![2, 3];

        let npc3 = crate::domain::world::NpcDefinition::new("guard", "Guard", "guard.png");

        db.add_npc(npc1).expect("Failed to add npc1");
        db.add_npc(npc2).expect("Failed to add npc2");
        db.add_npc(npc3).expect("Failed to add npc3");

        let npcs_for_quest_2 = db.npcs_for_quest(2);
        assert_eq!(npcs_for_quest_2.len(), 2);

        let npcs_for_quest_1 = db.npcs_for_quest(1);
        assert_eq!(npcs_for_quest_1.len(), 1);
        assert_eq!(npcs_for_quest_1[0].id, "elder");
    }

    #[test]
    fn test_npc_database_npcs_by_faction() {
        let mut db = NpcDatabase::new();

        let mut npc1 = crate::domain::world::NpcDefinition::new("guard1", "Guard 1", "guard.png");
        npc1.faction = Some("City Guard".to_string());

        let mut npc2 = crate::domain::world::NpcDefinition::new("guard2", "Guard 2", "guard.png");
        npc2.faction = Some("City Guard".to_string());

        let mut npc3 =
            crate::domain::world::NpcDefinition::new("merchant", "Merchant", "merchant.png");
        npc3.faction = Some("Merchants Guild".to_string());

        db.add_npc(npc1).expect("Failed to add npc1");
        db.add_npc(npc2).expect("Failed to add npc2");
        db.add_npc(npc3).expect("Failed to add npc3");

        let city_guards = db.npcs_by_faction("City Guard");
        assert_eq!(city_guards.len(), 2);

        let merchants = db.npcs_by_faction("Merchants Guild");
        assert_eq!(merchants.len(), 1);
        assert_eq!(merchants[0].id, "merchant");
    }

    #[test]
    fn test_npc_database_all_npcs() {
        let mut db = NpcDatabase::new();

        let npc1 = crate::domain::world::NpcDefinition::new("npc1", "NPC 1", "1.png");
        let npc2 = crate::domain::world::NpcDefinition::new("npc2", "NPC 2", "2.png");

        db.add_npc(npc1).expect("Failed to add npc1");
        db.add_npc(npc2).expect("Failed to add npc2");

        let all_ids = db.all_npcs();
        assert_eq!(all_ids.len(), 2);
        assert!(all_ids.contains(&"npc1".to_string()));
        assert!(all_ids.contains(&"npc2".to_string()));
    }

    #[test]
    fn test_npc_database_load_nonexistent_file() {
        let result = NpcDatabase::load_from_file("nonexistent_file.ron");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().count(), 0);
    }

    #[test]
    fn test_npc_database_load_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

        let ron_content = r#"[
    (
        id: "village_elder",
        name: "Elder Theron",
        description: "The wise village elder",
        portrait_id: "elder",
        dialogue_id: Some(1),
        quest_ids: [1, 2],
        faction: Some("Village Council"),
        is_merchant: false,
        is_innkeeper: false,
    ),
    (
        id: "merchant_bob",
        name: "Bob the Merchant",
        description: "A traveling merchant",
        portrait_id: "merchant",
        dialogue_id: Some(5),
        quest_ids: [],
        faction: Some("Merchants Guild"),
        is_merchant: true,
        is_innkeeper: false,
    ),
]"#;

        temp_file
            .write_all(ron_content.as_bytes())
            .expect("Failed to write to temp file");
        temp_file.flush().expect("Failed to flush temp file");

        let db =
            NpcDatabase::load_from_file(temp_file.path()).expect("Failed to load NPC database");

        assert_eq!(db.count(), 2);
        assert!(db.has_npc("village_elder"));
        assert!(db.has_npc("merchant_bob"));

        let elder = db.get_npc("village_elder").expect("Elder not found");
        assert_eq!(elder.name, "Elder Theron");
        assert_eq!(elder.dialogue_id, Some(1));
        assert_eq!(elder.quest_ids, vec![1, 2]);
        assert!(!elder.is_merchant);

        let merchant = db.get_npc("merchant_bob").expect("Merchant not found");
        assert_eq!(merchant.name, "Bob the Merchant");
        assert!(merchant.is_merchant);
    }

    #[test]
    fn test_content_database_includes_npcs() {
        let db = ContentDatabase::new();
        assert_eq!(db.npcs.count(), 0);
    }

    #[test]
    fn test_content_stats_includes_npcs() {
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
            creature_count: 0,
            npc_count: 15,
            npc_stock_template_count: 0,
        };

        assert_eq!(stats.total(), 181);
        assert_eq!(stats.npc_count, 15);
    }

    #[test]
    fn test_load_core_npcs_file() {
        // Test loading the actual core NPCs data file
        let core_npcs_path = "data/npcs.ron";

        // Skip test if file doesn't exist (in CI environments)
        if !std::path::Path::new(core_npcs_path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(core_npcs_path).expect("Failed to load core npcs.ron");

        // Verify base archetypes are loaded
        assert!(db.has_npc("base_merchant"), "base_merchant not found");
        assert!(db.has_npc("base_innkeeper"), "base_innkeeper not found");
        assert!(db.has_npc("base_priest"), "base_priest not found");
        assert!(db.has_npc("base_elder"), "base_elder not found");
        assert!(db.has_npc("base_guard"), "base_guard not found");
        assert!(db.has_npc("base_ranger"), "base_ranger not found");
        assert!(db.has_npc("base_wizard"), "base_wizard not found");

        // Verify archetype properties
        let merchant = db
            .get_npc("base_merchant")
            .expect("Merchant archetype not found");
        assert!(
            merchant.is_merchant,
            "Merchant should have is_merchant=true"
        );
        assert!(!merchant.is_innkeeper, "Merchant should not be innkeeper");
        assert_eq!(merchant.faction, Some("Merchants Guild".to_string()));

        let innkeeper = db
            .get_npc("base_innkeeper")
            .expect("Innkeeper archetype not found");
        assert!(
            innkeeper.is_innkeeper,
            "Innkeeper should have is_innkeeper=true"
        );
        assert!(!innkeeper.is_merchant, "Innkeeper should not be merchant");
        assert_eq!(innkeeper.faction, Some("Innkeepers Guild".to_string()));

        let priest = db
            .get_npc("base_priest")
            .expect("Priest archetype not found");
        assert_eq!(priest.faction, Some("Temple".to_string()));

        // Verify count
        assert_eq!(db.count(), 7, "Should have 7 base archetypes");
    }

    #[test]
    fn test_load_tutorial_npcs_file() {
        // Test loading a stable tutorial NPC fixture (not the live tutorial data)
        // Use test fixture so that evolving tutorial data doesn't break this unit test.
        let tutorial_npcs_path = "tests/data/tutorial_npcs_fixture.ron";

        // Skip test if fixture file doesn't exist (in CI environments)
        if !std::path::Path::new(tutorial_npcs_path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(tutorial_npcs_path)
            .expect("Failed to load tutorial npcs fixture");

        // Verify key tutorial NPCs are loaded
        assert!(
            db.has_npc("tutorial_elder_village"),
            "Village elder not found"
        );
        assert!(db.has_npc("tutorial_wizard_arcturus"), "Arcturus not found");
        assert!(
            db.has_npc("tutorial_wizard_arcturus_brother"),
            "Arcturus's brother not found"
        );
        assert!(db.has_npc("tutorial_ranger_lost"), "Lost ranger not found");

        // Verify Arcturus's properties (quest giver with dialogue)
        let arcturus = db
            .get_npc("tutorial_wizard_arcturus")
            .expect("Arcturus not found");
        assert_eq!(arcturus.name, "Arcturus");
        assert_eq!(
            arcturus.dialogue_id,
            Some(1),
            "Arcturus should reference dialogue 1"
        );
        assert_eq!(arcturus.quest_ids, vec![0], "Arcturus should give quest 0");
        assert_eq!(arcturus.faction, Some("Wizards".to_string()));

        // Verify Arcturus's brother (multiple quests)
        let brother = db
            .get_npc("tutorial_wizard_arcturus_brother")
            .expect("Arcturus's brother not found");
        assert_eq!(
            brother.quest_ids,
            vec![1, 3],
            "Brother should give quests 1 and 3"
        );

        // Verify merchants and innkeepers
        let merchants = db.merchants();
        assert!(merchants.len() >= 2, "Should have at least 2 merchants");

        let innkeepers = db.innkeepers();
        assert!(innkeepers.len() >= 2, "Should have at least 2 innkeepers");

        // Verify quest givers
        let quest_givers = db.quest_givers();
        assert!(
            quest_givers.len() >= 3,
            "Should have at least 3 quest givers"
        );

        // Verify Village Elder has quest 5 (The Lich's Tomb)
        let elder = db
            .get_npc("tutorial_elder_village")
            .expect("Elder not found");
        assert_eq!(elder.quest_ids, vec![5], "Elder should give quest 5");

        // Verify total count (12 NPCs in tutorial)
        assert_eq!(db.count(), 12, "Should have 12 tutorial NPCs");
    }

    #[test]
    fn test_tutorial_npcs_reference_valid_dialogues() {
        // Test that tutorial NPCs reference valid dialogue IDs using stable fixtures
        // Tests should not rely on the mutable tutorial campaign data.
        let tutorial_npcs_path = "tests/data/tutorial_npcs_fixture.ron";
        let tutorial_dialogues_path = "tests/data/tutorial_dialogues_fixture.ron";

        // Skip test if files don't exist
        if !std::path::Path::new(tutorial_npcs_path).exists()
            || !std::path::Path::new(tutorial_dialogues_path).exists()
        {
            return;
        }

        let npc_db = NpcDatabase::load_from_file(tutorial_npcs_path)
            .expect("Failed to load tutorial npcs fixture");
        let dialogue_db = DialogueDatabase::load_from_file(tutorial_dialogues_path)
            .expect("Failed to load tutorial dialogues fixture");

        // Verify all NPCs with dialogue_id reference valid dialogues
        for npc_id in npc_db.all_npcs() {
            let npc = npc_db.get_npc(&npc_id).expect("NPC not found");
            if let Some(dialogue_id) = npc.dialogue_id {
                assert!(
                    dialogue_db.has_dialogue(&dialogue_id),
                    "NPC {} references invalid dialogue_id {}",
                    npc.id,
                    dialogue_id
                );
            }
        }
    }

    #[test]
    fn test_tutorial_npcs_reference_valid_quests() {
        // Test that tutorial NPCs reference valid quest IDs
        let tutorial_npcs_path = "data/test_campaign/data/npcs.ron";
        let tutorial_quests_path = "data/test_campaign/data/quests.ron";

        // Skip test if files don't exist
        if !std::path::Path::new(tutorial_npcs_path).exists()
            || !std::path::Path::new(tutorial_quests_path).exists()
        {
            return;
        }

        let npc_db = NpcDatabase::load_from_file(tutorial_npcs_path)
            .expect("Failed to load tutorial npcs.ron");
        let quest_db = QuestDatabase::load_from_file(tutorial_quests_path)
            .expect("Failed to load tutorial quests.ron");

        // Verify all NPCs with quest_ids reference valid quests
        for npc_id in npc_db.all_npcs() {
            let npc = npc_db.get_npc(&npc_id).expect("NPC not found");
            for quest_id in &npc.quest_ids {
                assert!(
                    quest_db.has_quest(quest_id),
                    "NPC {} references invalid quest_id {}",
                    npc.id,
                    quest_id
                );
            }
        }
    }

    // ===== MerchantStockTemplateDatabase and ContentDatabase Integration Tests =====

    #[test]
    fn test_merchant_stock_template_database_new() {
        // Assert that a freshly constructed database is empty
        let db = crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_file() {
        // Load the core npc_stock_templates.ron and assert at least 3 templates exist
        let path = "data/npc_stock_templates.ron";

        if !std::path::Path::new(path).exists() {
            return;
        }

        let db =
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(path)
                .expect("Failed to load npc_stock_templates.ron");

        assert!(
            db.len() >= 3,
            "Expected at least 3 templates, got {}",
            db.len()
        );
        assert!(
            db.get("blacksmith_basic").is_some(),
            "blacksmith_basic not found"
        );
        assert!(
            db.get("general_store_basic").is_some(),
            "general_store_basic not found"
        );
        assert!(
            db.get("alchemist_basic").is_some(),
            "alchemist_basic not found"
        );
    }

    #[test]
    fn test_content_database_includes_npc_stock_templates() {
        // Load core content and assert npc_stock_templates is populated
        let data_path = "data";

        if !std::path::Path::new(data_path).exists()
            || !std::path::Path::new("data/npc_stock_templates.ron").exists()
        {
            return;
        }

        let db =
            ContentDatabase::load_core(data_path).expect("Failed to load core content database");

        assert!(
            !db.npc_stock_templates.is_empty(),
            "npc_stock_templates should be populated after load_core"
        );
        assert!(
            db.stats().npc_stock_template_count > 0,
            "ContentStats::npc_stock_template_count should be > 0"
        );
    }

    #[test]
    fn test_base_merchant_has_stock_template() {
        // Load data/npcs.ron and verify base_merchant has stock_template = Some("blacksmith_basic")
        let path = "data/npcs.ron";

        if !std::path::Path::new(path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(path).expect("Failed to load data/npcs.ron");

        let merchant = db
            .get_npc("base_merchant")
            .expect("base_merchant not found");
        assert_eq!(
            merchant.stock_template,
            Some("blacksmith_basic".to_string()),
            "base_merchant should have stock_template = Some(\"blacksmith_basic\")"
        );
        assert!(
            merchant.economy.is_some(),
            "base_merchant should have economy settings"
        );
    }

    #[test]
    fn test_base_priest_has_service_catalog() {
        // Load data/npcs.ron and verify base_priest has a service_catalog with at least 4 entries
        let path = "data/npcs.ron";

        if !std::path::Path::new(path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(path).expect("Failed to load data/npcs.ron");

        let priest = db.get_npc("base_priest").expect("base_priest not found");
        let catalog = priest
            .service_catalog
            .as_ref()
            .expect("base_priest should have a service_catalog");

        assert!(
            catalog.services.len() >= 4,
            "base_priest service_catalog should have at least 4 services, got {}",
            catalog.services.len()
        );

        // Verify the expected service IDs are present
        let service_ids: Vec<&str> = catalog
            .services
            .iter()
            .map(|s| s.service_id.as_str())
            .collect();
        assert!(
            service_ids.contains(&"heal_all"),
            "Expected 'heal_all' service"
        );
        assert!(
            service_ids.contains(&"cure_poison"),
            "Expected 'cure_poison' service"
        );
        assert!(
            service_ids.contains(&"cure_disease"),
            "Expected 'cure_disease' service"
        );
        assert!(
            service_ids.contains(&"resurrect"),
            "Expected 'resurrect' service"
        );
    }

    #[test]
    fn test_base_innkeeper_has_service_catalog() {
        // Load data/npcs.ron and verify base_innkeeper has a service_catalog with "rest"
        let path = "data/npcs.ron";

        if !std::path::Path::new(path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(path).expect("Failed to load data/npcs.ron");

        let innkeeper = db
            .get_npc("base_innkeeper")
            .expect("base_innkeeper not found");
        let catalog = innkeeper
            .service_catalog
            .as_ref()
            .expect("base_innkeeper should have a service_catalog");

        assert!(
            !catalog.services.is_empty(),
            "base_innkeeper service_catalog should have at least one service"
        );

        let has_rest = catalog.services.iter().any(|s| s.service_id == "rest");
        assert!(has_rest, "base_innkeeper should offer 'rest' service");
    }

    #[test]
    fn test_content_database_npc_stock_template_count_in_stats() {
        // Assert that stats() reflects the npc_stock_templates field correctly
        let mut db = ContentDatabase::new();

        // Empty database should have 0
        assert_eq!(db.stats().npc_stock_template_count, 0);

        // Add a template manually
        db.npc_stock_templates
            .add(crate::domain::world::npc_runtime::MerchantStockTemplate {
                id: "test_template".to_string(),
                entries: vec![],
                magic_item_pool: vec![],
                magic_slot_count: 0,
                magic_refresh_days: 7,
            });

        assert_eq!(db.stats().npc_stock_template_count, 1);
        assert_eq!(db.npc_stock_templates.len(), 1);
    }

    #[test]
    fn test_content_database_load_campaign_includes_npc_stock_templates() {
        // Load the test campaign and verify npc_stock_templates is populated
        let campaign_path = "data/test_campaign";

        if !std::path::Path::new(campaign_path).exists()
            || !std::path::Path::new("data/test_campaign/data/npc_stock_templates.ron").exists()
        {
            return;
        }

        let db = ContentDatabase::load_campaign(campaign_path)
            .expect("Failed to load test campaign database");

        assert!(
            !db.npc_stock_templates.is_empty(),
            "Test campaign npc_stock_templates should be populated"
        );
        assert!(
            db.npc_stock_templates
                .get("tutorial_merchant_stock")
                .is_some(),
            "tutorial_merchant_stock template not found in test campaign"
        );
    }

    /// Merchant and Innkeeper Integration
    ///
    /// Verify that the core `npc_stock_templates.ron` contains food items
    /// (Food Ration id 53, Trail Ration id 54) in the `general_store_basic`
    /// and `innkeeper_basic` templates.  This test is the acceptance gate for
    /// the deliverable: "Merchant stock templates updated with food items".
    #[test]
    fn test_general_store_basic_contains_food_rations() {
        let path = "data/npc_stock_templates.ron";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let db =
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(path)
                .expect("Failed to load data/npc_stock_templates.ron");

        let template = db
            .get("general_store_basic")
            .expect("general_store_basic template must exist");

        let has_food_ration = template.entries.iter().any(|e| e.item_id == 53);
        assert!(
            has_food_ration,
            "general_store_basic must stock Food Ration (item_id 53)"
        );

        let has_trail_ration = template.entries.iter().any(|e| e.item_id == 54);
        assert!(
            has_trail_ration,
            "general_store_basic must stock Trail Ration (item_id 54)"
        );

        // Quantities must be non-zero so the party can actually buy food.
        let food_ration_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 53)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            food_ration_qty > 0,
            "general_store_basic Food Ration quantity must be > 0, got {}",
            food_ration_qty
        );

        let trail_ration_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 54)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            trail_ration_qty > 0,
            "general_store_basic Trail Ration quantity must be > 0, got {}",
            trail_ration_qty
        );
    }

    /// Merchant and Innkeeper Integration
    ///
    /// Verify that the `innkeeper_basic` template exists in the core data and
    /// stocks food rations so an innkeeper NPC can sell food to the party.
    #[test]
    fn test_innkeeper_basic_template_contains_food_rations() {
        let path = "data/npc_stock_templates.ron";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let db =
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(path)
                .expect("Failed to load data/npc_stock_templates.ron");

        let template = db
            .get("innkeeper_basic")
            .expect("innkeeper_basic template must exist");

        let has_food_ration = template.entries.iter().any(|e| e.item_id == 53);
        assert!(
            has_food_ration,
            "innkeeper_basic must stock Food Ration (item_id 53)"
        );

        let has_trail_ration = template.entries.iter().any(|e| e.item_id == 54);
        assert!(
            has_trail_ration,
            "innkeeper_basic must stock Trail Ration (item_id 54)"
        );

        // Innkeepers should keep generous quantities — at least 10 food rations.
        let food_ration_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 53)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            food_ration_qty >= 10,
            "innkeeper_basic Food Ration quantity must be >= 10, got {}",
            food_ration_qty
        );
    }

    /// Merchant and Innkeeper Integration
    ///
    /// Verify that the `general_goods` template alias exists and also contains
    /// food rations.
    #[test]
    fn test_general_goods_template_contains_food_rations() {
        let path = "data/npc_stock_templates.ron";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let db =
            crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(path)
                .expect("Failed to load data/npc_stock_templates.ron");

        let template = db
            .get("general_goods")
            .expect("general_goods template must exist");

        let has_food_ration = template.entries.iter().any(|e| e.item_id == 53);
        assert!(
            has_food_ration,
            "general_goods must stock Food Ration (item_id 53)"
        );

        let has_trail_ration = template.entries.iter().any(|e| e.item_id == 54);
        assert!(
            has_trail_ration,
            "general_goods must stock Trail Ration (item_id 54)"
        );
    }

    /// Merchant and Innkeeper Integration (test campaign)
    ///
    /// Verify that the test campaign's `tutorial_merchant_stock` template
    /// contains food rations (item_id 108 = Food Ration, item_id 109 = Trail
    /// Ration) with non-zero quantities.  This exercises the full load path
    /// from the campaign data directory.
    #[test]
    fn test_test_campaign_merchant_stock_contains_food_rations() {
        let template_path = "data/test_campaign/data/npc_stock_templates.ron";
        if !std::path::Path::new(template_path).exists() {
            return;
        }

        let db = crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
            template_path,
        )
        .expect("Failed to load test_campaign npc_stock_templates.ron");

        let template = db
            .get("tutorial_merchant_stock")
            .expect("tutorial_merchant_stock must exist in test campaign");

        let has_food_ration = template.entries.iter().any(|e| e.item_id == 108);
        assert!(
            has_food_ration,
            "tutorial_merchant_stock must stock Food Ration (item_id 108)"
        );

        let has_trail_ration = template.entries.iter().any(|e| e.item_id == 109);
        assert!(
            has_trail_ration,
            "tutorial_merchant_stock must stock Trail Ration (item_id 109)"
        );

        let food_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 108)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            food_qty > 0,
            "tutorial_merchant_stock Food Ration quantity must be > 0, got {}",
            food_qty
        );
    }

    /// Merchant and Innkeeper Integration (test campaign)
    ///
    /// Verify that the test campaign's `tutorial_general_store` template exists
    /// and contains food rations.
    #[test]
    fn test_test_campaign_general_store_template_contains_food_rations() {
        let template_path = "data/test_campaign/data/npc_stock_templates.ron";
        if !std::path::Path::new(template_path).exists() {
            return;
        }

        let db = crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
            template_path,
        )
        .expect("Failed to load test_campaign npc_stock_templates.ron");

        let template = db
            .get("tutorial_general_store")
            .expect("tutorial_general_store must exist in test campaign");

        let has_food_ration = template.entries.iter().any(|e| e.item_id == 108);
        assert!(
            has_food_ration,
            "tutorial_general_store must stock Food Ration (item_id 108)"
        );

        let has_trail_ration = template.entries.iter().any(|e| e.item_id == 109);
        assert!(
            has_trail_ration,
            "tutorial_general_store must stock Trail Ration (item_id 109)"
        );

        let food_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 108)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            food_qty > 0,
            "tutorial_general_store Food Ration quantity must be > 0, got {}",
            food_qty
        );

        let trail_qty = template
            .entries
            .iter()
            .find(|e| e.item_id == 109)
            .map(|e| e.quantity)
            .unwrap_or(0);
        assert!(
            trail_qty > 0,
            "tutorial_general_store Trail Ration quantity must be > 0, got {}",
            trail_qty
        );
    }

    /// Merchant and Innkeeper Integration (test campaign)
    ///
    /// Verify that the test campaign's `tutorial_innkeeper_stock` template
    /// exists and contains food rations with override prices, confirming the
    /// innkeeper markup is correctly represented in the data.
    #[test]
    fn test_test_campaign_innkeeper_stock_template_contains_food_rations() {
        let template_path = "data/test_campaign/data/npc_stock_templates.ron";
        if !std::path::Path::new(template_path).exists() {
            return;
        }

        let db = crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
            template_path,
        )
        .expect("Failed to load test_campaign npc_stock_templates.ron");

        let template = db
            .get("tutorial_innkeeper_stock")
            .expect("tutorial_innkeeper_stock must exist in test campaign");

        let food_entry = template
            .entries
            .iter()
            .find(|e| e.item_id == 108)
            .expect("tutorial_innkeeper_stock must stock Food Ration (item_id 108)");
        assert!(
            food_entry.quantity > 0,
            "tutorial_innkeeper_stock Food Ration quantity must be > 0"
        );
        // Innkeeper prices are a slight markup; the override must be Some.
        assert!(
            food_entry.override_price.is_some(),
            "tutorial_innkeeper_stock Food Ration should have an override_price (innkeeper markup)"
        );

        let trail_entry = template
            .entries
            .iter()
            .find(|e| e.item_id == 109)
            .expect("tutorial_innkeeper_stock must stock Trail Ration (item_id 109)");
        assert!(
            trail_entry.quantity > 0,
            "tutorial_innkeeper_stock Trail Ration quantity must be > 0"
        );
        assert!(
            trail_entry.override_price.is_some(),
            "tutorial_innkeeper_stock Trail Ration should have an override_price (innkeeper markup)"
        );
    }

    /// Stock template populates MerchantStock correctly
    ///
    /// End-to-end test: load the test campaign, build a runtime NPC state from
    /// the `tutorial_general_store` template, and assert that the resulting
    /// `MerchantStock` contains Food Ration and Trail Ration entries with
    /// non-zero quantities.  This is the full acceptance test for the
    /// success criteria: "Players can interact with merchants and natively buy
    /// food rations into their inventory using gold."
    #[test]
    fn test_stock_template_populates_merchant_runtime_with_food() {
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let template_path = "data/test_campaign/data/npc_stock_templates.ron";
        if !std::path::Path::new(template_path).exists() {
            return;
        }

        let db = crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::load_from_file(
            template_path,
        )
        .expect("Failed to load test_campaign npc_stock_templates.ron");

        let template = db
            .get("tutorial_general_store")
            .expect("tutorial_general_store template must exist");

        // Simulate the initialization that happens when a new game session starts:
        // build an NpcRuntimeState from the template.
        let runtime =
            NpcRuntimeState::initialize_stock_from_template("test_merchant".into(), template);
        let stock = runtime.stock.as_ref().expect("merchant must have stock");

        // Food Ration (item 108) must be present with quantity > 0.
        let food_entry = stock.entries.iter().find(|e| e.item_id == 108);
        assert!(
            food_entry.is_some(),
            "MerchantStock initialized from tutorial_general_store must contain Food Ration (108)"
        );
        assert!(
            food_entry.unwrap().quantity > 0,
            "Food Ration quantity in initialized MerchantStock must be > 0"
        );

        // Trail Ration (item 109) must be present with quantity > 0.
        let trail_entry = stock.entries.iter().find(|e| e.item_id == 109);
        assert!(
            trail_entry.is_some(),
            "MerchantStock initialized from tutorial_general_store must contain Trail Ration (109)"
        );
        assert!(
            trail_entry.unwrap().quantity > 0,
            "Trail Ration quantity in initialized MerchantStock must be > 0"
        );
    }

    /// Verify that the test-campaign priest NPC has a `service_catalog`
    /// containing the `"resurrect"` service.
    ///
    /// This test satisfies the requirement:
    /// `test_temple_npc_has_resurrect_service` — Test campaign priest NPC has
    /// `service_catalog` with `"resurrect"`.
    #[test]
    fn test_temple_npc_has_resurrect_service() {
        let path = "data/test_campaign/data/npcs.ron";

        if !std::path::Path::new(path).exists() {
            return;
        }

        let db = NpcDatabase::load_from_file(path)
            .expect("Failed to load data/test_campaign/data/npcs.ron");

        // The dedicated fixture priest must exist
        let priest = db
            .get_npc("temple_priest")
            .expect("temple_priest NPC not found in test campaign");

        // Must be marked as a priest
        assert!(
            priest.is_priest,
            "temple_priest must have is_priest == true"
        );

        // Must have a service catalog
        let catalog = priest
            .service_catalog
            .as_ref()
            .expect("temple_priest must have a service_catalog");

        // The catalog must contain the resurrect service
        assert!(
            catalog.has_service("resurrect"),
            "temple_priest service_catalog must contain 'resurrect'"
        );

        // Verify cost and gem_cost of the resurrect service
        let service = catalog
            .get_service("resurrect")
            .expect("resurrect service must be retrievable");

        assert!(
            service.cost > 0,
            "resurrect service must have a non-zero gold cost"
        );
        assert!(
            service.gem_cost >= 1,
            "resurrect service must require at least 1 gem"
        );

        // Also verify that the existing tutorial priest NPCs now have resurrect
        let priest2 = db.get_npc("tutorial_priestess_town");
        if let Some(p) = priest2 {
            if let Some(cat) = &p.service_catalog {
                assert!(
                    cat.has_service("resurrect"),
                    "tutorial_priestess_town should have resurrect service"
                );
            }
        }

        let priest3 = db.get_npc("tutorial_priest_town2");
        if let Some(p) = priest3 {
            if let Some(cat) = &p.service_catalog {
                assert!(
                    cat.has_service("resurrect"),
                    "tutorial_priest_town2 should have resurrect service"
                );
            }
        }

        // Verify the NpcDatabase::priests() filter works for the test campaign
        let priests = db.priests();
        assert!(
            !priests.is_empty(),
            "test campaign must have at least one priest NPC"
        );
        assert!(
            priests.iter().any(|p| p.id == "temple_priest"),
            "NpcDatabase::priests() must include temple_priest"
        );
    }
}
