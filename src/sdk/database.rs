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

use crate::domain::classes::ClassDatabase;
use crate::domain::items::ItemDatabase;
use crate::domain::types::{MapId, MonsterId, SpellId};
use crate::domain::world::Map;
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

// ===== Race System (Placeholder) =====

/// Race identifier
pub type RaceId = String;

/// Race definition structure (Phase 2 implementation pending)
///
/// This is a placeholder that will be fully implemented in Phase 2
/// of the SDK implementation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceDefinition {
    /// Unique identifier (e.g., "human", "elf")
    pub id: RaceId,
    /// Display name
    pub name: String,
}

/// Race database (Phase 2 implementation pending)
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

    /// Loads races from a RON file
    pub fn load_from_file<P: AsRef<Path>>(_path: P) -> Result<Self, DatabaseError> {
        // Placeholder implementation
        Ok(Self::new())
    }

    /// Gets a race by ID
    pub fn get_race(&self, id: &RaceId) -> Option<&RaceDefinition> {
        self.races.get(id)
    }

    /// Returns all race IDs
    pub fn all_races(&self) -> Vec<&RaceId> {
        self.races.keys().collect()
    }

    /// Returns the number of races
    pub fn count(&self) -> usize {
        self.races.len()
    }
}

// ===== Spell System (Placeholder) =====

/// Spell definition structure (full implementation pending)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellDefinition {
    /// Spell ID
    pub id: SpellId,
    /// Spell name
    pub name: String,
}

/// Spell database
#[derive(Debug, Clone, Default)]
pub struct SpellDatabase {
    spells: HashMap<SpellId, SpellDefinition>,
}

impl SpellDatabase {
    /// Creates an empty spell database
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
        }
    }

    /// Loads spells from a RON file
    pub fn load_from_file<P: AsRef<Path>>(_path: P) -> Result<Self, DatabaseError> {
        // Placeholder implementation
        Ok(Self::new())
    }

    /// Gets a spell by ID
    pub fn get_spell(&self, id: SpellId) -> Option<&SpellDefinition> {
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
}

// ===== Monster System (Placeholder) =====

/// Monster definition structure (full implementation pending)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDefinition {
    /// Monster ID
    pub id: MonsterId,
    /// Monster name
    pub name: String,
}

/// Monster database
#[derive(Debug, Clone, Default)]
pub struct MonsterDatabase {
    monsters: HashMap<MonsterId, MonsterDefinition>,
}

impl MonsterDatabase {
    /// Creates an empty monster database
    pub fn new() -> Self {
        Self {
            monsters: HashMap::new(),
        }
    }

    /// Loads monsters from a RON file
    pub fn load_from_file<P: AsRef<Path>>(_path: P) -> Result<Self, DatabaseError> {
        // Placeholder implementation
        Ok(Self::new())
    }

    /// Gets a monster by ID
    pub fn get_monster(&self, id: MonsterId) -> Option<&MonsterDefinition> {
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
    pub fn load_from_directory<P: AsRef<Path>>(_path: P) -> Result<Self, DatabaseError> {
        // Placeholder implementation - will load all .ron files from maps directory
        Ok(Self::new())
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

        Ok(Self {
            classes,
            races,
            items,
            monsters,
            spells,
            maps,
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

        Ok(Self {
            classes,
            races,
            items,
            monsters,
            spells,
            maps,
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

        // Additional validation can be added here for other content types

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
            race_count: self.races.count(),
            item_count: self.items.all_items().len(),
            monster_count: self.monsters.count(),
            spell_count: self.spells.count(),
            map_count: self.maps.count(),
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
    ///     class_count: 6,
    ///     race_count: 5,
    ///     item_count: 100,
    ///     monster_count: 50,
    ///     spell_count: 40,
    ///     map_count: 10,
    /// };
    ///
    /// assert_eq!(stats.total(), 211);
    /// ```
    pub fn total(&self) -> usize {
        self.class_count
            + self.race_count
            + self.item_count
            + self.monster_count
            + self.spell_count
            + self.map_count
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
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_content_stats_total() {
        let stats = ContentStats {
            class_count: 6,
            race_count: 5,
            item_count: 100,
            monster_count: 50,
            spell_count: 40,
            map_count: 10,
        };

        assert_eq!(stats.total(), 211);
    }

    #[test]
    fn test_race_database_new() {
        let db = RaceDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_races().is_empty());
    }

    #[test]
    fn test_spell_database_new() {
        let db = SpellDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_spells().is_empty());
    }

    #[test]
    fn test_monster_database_new() {
        let db = MonsterDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_monsters().is_empty());
    }

    #[test]
    fn test_map_database_new() {
        let db = MapDatabase::new();
        assert_eq!(db.count(), 0);
        assert!(db.all_maps().is_empty());
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
}
