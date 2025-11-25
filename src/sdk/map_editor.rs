// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map editor helper functions and integration for Phase 4
//!
//! This module provides helper functions for integrating the SDK content database
//! with map editing tools, including smart ID suggestions, content browsing,
//! and validation.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 4 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::database::ContentDatabase;
//! use antares::sdk::map_editor::{browse_monsters, suggest_monster_ids};
//! use antares::domain::world::Map;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
//! let map = Map::new(0, "Test Map".to_string(), "Description".to_string(), 10, 10);
//!
//! // Browse available monsters
//! let monsters = browse_monsters(&db);
//! println!("Found {} monsters", monsters.len());
//!
//! // Get suggestions for monster IDs
//! let suggestions = suggest_monster_ids(&db, "gob");
//! for (id, name) in suggestions {
//!     println!("  [{}] {}", id, name);
//! }
//! # Ok(())
//! # }
//! ```

use crate::domain::types::{ItemId, MapId, MonsterId, SpellId};
use crate::domain::world::Map;
use crate::sdk::database::ContentDatabase;
use crate::sdk::validation::{ValidationError, Validator};

// ===== Content Browsing =====

/// Returns a list of all monster IDs and names in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::browse_monsters;
///
/// let db = ContentDatabase::new();
/// let monsters = browse_monsters(&db);
/// assert_eq!(monsters.len(), 0); // Empty database
/// ```
pub fn browse_monsters(db: &ContentDatabase) -> Vec<(MonsterId, String)> {
    let mut results = Vec::new();
    for monster_id in db.monsters.all_monsters() {
        if let Some(monster) = db.monsters.get_monster(monster_id) {
            results.push((monster_id, monster.name.clone()));
        }
    }
    results
}

/// Returns a list of all item IDs and names in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::browse_items;
///
/// let db = ContentDatabase::new();
/// let items = browse_items(&db);
/// assert_eq!(items.len(), 0); // Empty database
/// ```
pub fn browse_items(db: &ContentDatabase) -> Vec<(ItemId, String)> {
    let mut results = Vec::new();
    for item in db.items.all_items() {
        results.push((item.id, item.name.clone()));
    }
    results
}

/// Returns a list of all spell IDs and names in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::browse_spells;
///
/// let db = ContentDatabase::new();
/// let spells = browse_spells(&db);
/// assert_eq!(spells.len(), 0); // Empty database
/// ```
pub fn browse_spells(db: &ContentDatabase) -> Vec<(SpellId, String)> {
    let mut results = Vec::new();
    for spell_id in db.spells.all_spells() {
        if let Some(spell) = db.spells.get_spell(spell_id) {
            results.push((spell_id, spell.name.clone()));
        }
    }
    results
}

/// Returns a list of all map IDs and dimensions in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::browse_maps;
///
/// let db = ContentDatabase::new();
/// let maps = browse_maps(&db);
/// assert_eq!(maps.len(), 0); // Empty database
/// ```
pub fn browse_maps(db: &ContentDatabase) -> Vec<(MapId, u32, u32)> {
    let mut results = Vec::new();
    for map_id in db.maps.all_maps() {
        if let Some(map) = db.maps.get_map(map_id) {
            results.push((map_id, map.width, map.height));
        }
    }
    results
}

// ===== Smart ID Suggestions =====

/// Suggests monster IDs based on partial input (name or ID)
///
/// Returns up to 10 matching monsters with IDs and names.
///
/// # Arguments
///
/// * `db` - Content database to search
/// * `partial` - Partial text to match against monster ID or name
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::suggest_monster_ids;
///
/// let db = ContentDatabase::new();
/// let suggestions = suggest_monster_ids(&db, "gob");
/// // Would return monsters with "gob" in name or ID
/// ```
pub fn suggest_monster_ids(db: &ContentDatabase, partial: &str) -> Vec<(MonsterId, String)> {
    let partial_lower = partial.to_lowercase();
    let mut suggestions = Vec::new();

    for monster_id in db.monsters.all_monsters() {
        if let Some(monster) = db.monsters.get_monster(monster_id) {
            if monster_id.to_string().contains(&partial_lower)
                || monster.name.to_lowercase().contains(&partial_lower)
            {
                suggestions.push((monster_id, monster.name.clone()));
                if suggestions.len() >= 10 {
                    break;
                }
            }
        }
    }

    suggestions
}

/// Suggests item IDs based on partial input (name or ID)
///
/// Returns up to 10 matching items with IDs and names.
///
/// # Arguments
///
/// * `db` - Content database to search
/// * `partial` - Partial text to match against item ID or name
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::suggest_item_ids;
///
/// let db = ContentDatabase::new();
/// let suggestions = suggest_item_ids(&db, "sword");
/// // Would return items with "sword" in name or ID
/// ```
pub fn suggest_item_ids(db: &ContentDatabase, partial: &str) -> Vec<(ItemId, String)> {
    let partial_lower = partial.to_lowercase();
    let items = db.items.all_items();

    items
        .iter()
        .filter(|item| {
            item.id.to_string().contains(&partial_lower)
                || item.name.to_lowercase().contains(&partial_lower)
        })
        .take(10)
        .map(|item| (item.id, item.name.clone()))
        .collect()
}

/// Suggests spell IDs based on partial input (name or ID)
///
/// Returns up to 10 matching spells with IDs and names.
///
/// # Arguments
///
/// * `db` - Content database to search
/// * `partial` - Partial text to match against spell ID or name
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::suggest_spell_ids;
///
/// let db = ContentDatabase::new();
/// let suggestions = suggest_spell_ids(&db, "fire");
/// // Would return spells with "fire" in name or ID
/// ```
pub fn suggest_spell_ids(db: &ContentDatabase, partial: &str) -> Vec<(SpellId, String)> {
    let partial_lower = partial.to_lowercase();
    let mut suggestions = Vec::new();

    for spell_id in db.spells.all_spells() {
        if let Some(spell) = db.spells.get_spell(spell_id) {
            if spell_id.to_string().contains(&partial_lower)
                || spell.name.to_lowercase().contains(&partial_lower)
            {
                suggestions.push((spell_id, spell.name.clone()));
                if suggestions.len() >= 10 {
                    break;
                }
            }
        }
    }

    suggestions
}

/// Suggests map IDs based on partial input
///
/// Returns up to 10 matching maps with IDs.
///
/// # Arguments
///
/// * `db` - Content database to search
/// * `partial` - Partial text to match against map ID
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::suggest_map_ids;
///
/// let db = ContentDatabase::new();
/// let suggestions = suggest_map_ids(&db, "1");
/// // Would return maps with "1" in ID
/// ```
pub fn suggest_map_ids(db: &ContentDatabase, partial: &str) -> Vec<MapId> {
    let partial_lower = partial.to_lowercase();

    db.maps
        .all_maps()
        .into_iter()
        .filter(|map_id| map_id.to_string().contains(&partial_lower))
        .take(10)
        .collect()
}

// ===== Map Validation =====

/// Validates a map against the content database
///
/// Checks for missing references, invalid positions, and balance issues.
///
/// # Arguments
///
/// * `db` - Content database for cross-reference validation
/// * `map` - Map to validate
///
/// # Returns
///
/// Returns `Ok(Vec<ValidationError>)` with all validation issues found.
/// An empty vector means the map is valid.
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::validate_map;
/// use antares::domain::world::Map;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::new();
/// let map = Map::new(0, "Test Map".to_string(), "Description".to_string(), 10, 10);
///
/// let errors = validate_map(&db, &map)?;
/// println!("Map has {} validation issues", errors.len());
/// # Ok(())
/// # }
/// ```
pub fn validate_map(
    db: &ContentDatabase,
    map: &Map,
) -> Result<Vec<ValidationError>, Box<dyn std::error::Error>> {
    let validator = Validator::new(db);
    validator.validate_map(map)
}

/// Checks if a monster ID exists in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::is_valid_monster_id;
///
/// let db = ContentDatabase::new();
/// assert!(!is_valid_monster_id(&db, 42)); // Empty database
/// ```
pub fn is_valid_monster_id(db: &ContentDatabase, monster_id: MonsterId) -> bool {
    db.monsters.has_monster(&monster_id)
}

/// Checks if an item ID exists in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::is_valid_item_id;
///
/// let db = ContentDatabase::new();
/// assert!(!is_valid_item_id(&db, 42)); // Empty database
/// ```
pub fn is_valid_item_id(db: &ContentDatabase, item_id: ItemId) -> bool {
    db.items.has_item(&item_id)
}

/// Checks if a spell ID exists in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::is_valid_spell_id;
///
/// let db = ContentDatabase::new();
/// assert!(!is_valid_spell_id(&db, 0x1000)); // Empty database
/// ```
pub fn is_valid_spell_id(db: &ContentDatabase, spell_id: SpellId) -> bool {
    db.spells.has_spell(&spell_id)
}

/// Checks if a map ID exists in the database
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::is_valid_map_id;
///
/// let db = ContentDatabase::new();
/// assert!(!is_valid_map_id(&db, 42)); // Empty database
/// ```
pub fn is_valid_map_id(db: &ContentDatabase, map_id: MapId) -> bool {
    db.maps.has_map(&map_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browse_monsters_empty() {
        let db = ContentDatabase::new();
        let monsters = browse_monsters(&db);
        assert_eq!(monsters.len(), 0);
    }

    #[test]
    fn test_browse_items_empty() {
        let db = ContentDatabase::new();
        let items = browse_items(&db);
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_browse_spells_empty() {
        let db = ContentDatabase::new();
        let spells = browse_spells(&db);
        assert_eq!(spells.len(), 0);
    }

    #[test]
    fn test_browse_maps_empty() {
        let db = ContentDatabase::new();
        let maps = browse_maps(&db);
        assert_eq!(maps.len(), 0);
    }

    #[test]
    fn test_suggest_monster_ids_empty() {
        let db = ContentDatabase::new();
        let suggestions = suggest_monster_ids(&db, "goblin");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_suggest_item_ids_empty() {
        let db = ContentDatabase::new();
        let suggestions = suggest_item_ids(&db, "sword");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_suggest_spell_ids_empty() {
        let db = ContentDatabase::new();
        let suggestions = suggest_spell_ids(&db, "fire");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_suggest_map_ids_empty() {
        let db = ContentDatabase::new();
        let suggestions = suggest_map_ids(&db, "1");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_validate_map_empty_database() {
        let db = ContentDatabase::new();
        let map = Map::new(0, "Test Map".to_string(), "Description".to_string(), 10, 10);

        let result = validate_map(&db, &map);
        assert!(result.is_ok());

        let errors = result.unwrap();
        // Empty map with empty database should be valid
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_is_valid_monster_id_empty() {
        let db = ContentDatabase::new();
        assert!(!is_valid_monster_id(&db, 42));
    }

    #[test]
    fn test_is_valid_item_id_empty() {
        let db = ContentDatabase::new();
        assert!(!is_valid_item_id(&db, 42));
    }

    #[test]
    fn test_is_valid_spell_id_empty() {
        let db = ContentDatabase::new();
        assert!(!is_valid_spell_id(&db, 0x1000));
    }

    #[test]
    fn test_is_valid_map_id_empty() {
        let db = ContentDatabase::new();
        assert!(!is_valid_map_id(&db, 42));
    }
}
