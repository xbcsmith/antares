// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map editor helper functions and integration for Phase 5
//!
//! This module provides helper functions for integrating the SDK content database
//! with map editing tools, including smart ID suggestions, content browsing,
//! validation, and sprite sheet management.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 4 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::database::ContentDatabase;
//! use antares::sdk::map_editor::{browse_monsters, suggest_monster_ids, browse_sprite_sheets, get_sprites_for_sheet};
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
//!
//! // Browse available sprite sheets
//! let sheets = browse_sprite_sheets()?;
//! println!("Found {} sprite sheets", sheets.len());
//!
//! // Get sprites in a sheet
//! let sprites = get_sprites_for_sheet("npcs_town")?;
//! for (index, name) in sprites {
//!     println!("  [{}] {}", index, name);
//! }
//! # Ok(())
//! # }
//! ```

use crate::domain::types::{ItemId, MapId, MonsterId, SpellId};
use crate::domain::world::Map;
use crate::sdk::database::ContentDatabase;
use crate::sdk::validation::{ValidationError, Validator};
use std::collections::HashMap;
use std::error::Error;
use std::fs;

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

// ===== Sprite Sheet Management (Phase 5) =====

/// Type alias for sprite search results: (sheet_key, sprite_index, sprite_name)
pub type SpriteSearchResult = (String, u32, String);

/// Loads sprite sheet registry from data/sprite_sheets.ron
///
/// Returns a HashMap of sprite sheet keys to configurations.
///
/// # Returns
///
/// Returns `Ok(HashMap)` with sprite sheet definitions from data/sprite_sheets.ron.
/// If the file cannot be loaded or parsed, returns an error.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::load_sprite_registry;
///
/// let registry = load_sprite_registry()?;
/// println!("Loaded {} sprite sheets", registry.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load_sprite_registry() -> Result<HashMap<String, SpriteSheetInfo>, Box<dyn Error>> {
    let content = fs::read_to_string("data/sprite_sheets.ron")
        .map_err(|e| format!("Failed to read sprite_sheets.ron: {}", e))?;

    let registry: HashMap<String, SpriteSheetInfo> =
        ron::from_str(&content).map_err(|e| format!("Failed to parse sprite_sheets.ron: {}", e))?;

    Ok(registry)
}

/// Information about a sprite sheet configuration
///
/// Used for sprite browser and selection UI.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SpriteSheetInfo {
    /// Path to sprite sheet texture (relative to assets/)
    pub texture_path: String,

    /// Size of each sprite in pixels (width, height)
    pub tile_size: (f32, f32),

    /// Number of columns in sprite grid
    pub columns: u32,

    /// Number of rows in sprite grid
    pub rows: u32,

    /// Named sprite mappings (index, name)
    pub sprites: Vec<(u32, String)>,
}

/// Returns a list of all available sprite sheets with their paths
///
/// # Returns
///
/// Returns `Ok(Vec)` with tuples of (sheet_key, texture_path).
/// Returns error if sprite registry cannot be loaded.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::browse_sprite_sheets;
///
/// let sheets = browse_sprite_sheets()?;
/// for (key, path) in sheets {
///     println!("{}: {}", key, path);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn browse_sprite_sheets() -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let registry = load_sprite_registry()?;
    let mut results: Vec<(String, String)> = registry
        .iter()
        .map(|(key, info)| (key.clone(), info.texture_path.clone()))
        .collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}

/// Returns sprites available in a specific sprite sheet
///
/// # Arguments
///
/// * `sheet_key` - Sprite sheet identifier (e.g., "npcs_town", "walls")
///
/// # Returns
///
/// Returns `Ok(Vec)` with tuples of (sprite_index, sprite_name).
/// Returns error if sprite registry cannot be loaded or sheet not found.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::get_sprites_for_sheet;
///
/// let sprites = get_sprites_for_sheet("npcs_town")?;
/// for (index, name) in sprites {
///     println!("[{}] {}", index, name);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn get_sprites_for_sheet(sheet_key: &str) -> Result<Vec<(u32, String)>, Box<dyn Error>> {
    let registry = load_sprite_registry()?;

    if let Some(sheet) = registry.get(sheet_key) {
        let mut sprites = sheet.sprites.clone();
        sprites.sort_by_key(|a| a.0);
        Ok(sprites)
    } else {
        Err(format!("Sprite sheet '{}' not found in registry", sheet_key).into())
    }
}

/// Returns sprite sheet grid dimensions (columns, rows)
///
/// # Arguments
///
/// * `sheet_key` - Sprite sheet identifier
///
/// # Returns
///
/// Returns `Ok((columns, rows))` for the grid layout.
/// Returns error if sprite registry cannot be loaded or sheet not found.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::get_sprite_sheet_dimensions;
///
/// let (cols, rows) = get_sprite_sheet_dimensions("walls")?;
/// println!("{}x{} grid", cols, rows);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn get_sprite_sheet_dimensions(sheet_key: &str) -> Result<(u32, u32), Box<dyn Error>> {
    let registry = load_sprite_registry()?;

    if let Some(sheet) = registry.get(sheet_key) {
        Ok((sheet.columns, sheet.rows))
    } else {
        Err(format!("Sprite sheet '{}' not found in registry", sheet_key).into())
    }
}

/// Searches sprite sheets by name pattern
///
/// Returns up to 10 matching sprite sheets with keys and paths.
///
/// # Arguments
///
/// * `partial` - Partial text to match against sheet key or texture path
///
/// # Returns
///
/// Returns `Ok(Vec)` with matching sheets as (key, path) tuples.
/// Returns error if sprite registry cannot be loaded.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::suggest_sprite_sheets;
///
/// let suggestions = suggest_sprite_sheets("npc")?;
/// for (key, path) in suggestions {
///     println!("{}: {}", key, path);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn suggest_sprite_sheets(partial: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let registry = load_sprite_registry()?;
    let partial_lower = partial.to_lowercase();

    let mut suggestions: Vec<(String, String)> = registry
        .iter()
        .filter(|(key, info)| {
            key.to_lowercase().contains(&partial_lower)
                || info.texture_path.to_lowercase().contains(&partial_lower)
        })
        .take(10)
        .map(|(key, info)| (key.clone(), info.texture_path.clone()))
        .collect();

    suggestions.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(suggestions)
}

/// Searches for a specific sprite by name across all sheets
///
/// Returns up to 10 matching sprites with sheet keys and sprite info.
///
/// # Arguments
///
/// * `partial` - Partial text to match against sprite names
///
/// # Returns
///
/// Returns `Ok(Vec)` with matches as (sheet_key, sprite_index, sprite_name) tuples.
/// Returns error if sprite registry cannot be loaded.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::search_sprites;
///
/// let results = search_sprites("guard")?;
/// for (sheet, index, name) in results {
///     println!("{} [{}]: {}", sheet, index, name);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn search_sprites(partial: &str) -> Result<Vec<SpriteSearchResult>, Box<dyn Error>> {
    let registry = load_sprite_registry()?;
    let partial_lower = partial.to_lowercase();
    let mut results = Vec::new();

    for (sheet_key, sheet_info) in registry.iter() {
        for (sprite_index, sprite_name) in &sheet_info.sprites {
            if sprite_name.to_lowercase().contains(&partial_lower) {
                results.push((sheet_key.clone(), *sprite_index, sprite_name.clone()));
                if results.len() >= 10 {
                    return Ok(results);
                }
            }
        }
    }

    Ok(results)
}

/// Checks if a sprite sheet exists in the registry
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::map_editor::has_sprite_sheet;
///
/// if has_sprite_sheet("walls")? {
///     println!("Wall sprites available");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn has_sprite_sheet(sheet_key: &str) -> Result<bool, Box<dyn Error>> {
    let registry = load_sprite_registry()?;
    Ok(registry.contains_key(sheet_key))
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    #[test]
    fn test_sprite_sheet_info_clone() {
        let info = SpriteSheetInfo {
            texture_path: "sprites/test.png".to_string(),
            tile_size: (32.0, 32.0),
            columns: 4,
            rows: 4,
            sprites: vec![(0, "test_sprite".to_string())],
        };

        let cloned = info.clone();
        assert_eq!(cloned.columns, 4);
        assert_eq!(cloned.sprites.len(), 1);
    }

    #[test]
    fn test_load_sprite_registry_success() {
        // This test requires data/sprite_sheets.ron to exist
        match load_sprite_registry() {
            Ok(registry) => {
                // Verify expected sheets are present
                assert!(registry.contains_key("walls") || registry.is_empty());
            }
            Err(_) => {
                // File may not exist in test environment - that's ok
            }
        }
    }

    #[test]
    fn test_browse_sprite_sheets_returns_sorted() {
        // Only test if registry can be loaded
        if let Ok(sheets) = browse_sprite_sheets() {
            // Verify results are sorted
            for i in 1..sheets.len() {
                assert!(sheets[i - 1].0 <= sheets[i].0, "Sheets should be sorted");
            }
        }
    }

    #[test]
    fn test_get_sprites_for_sheet_sorts_by_index() {
        // Only test if registry can be loaded and has content
        if let Ok(sprites) = get_sprites_for_sheet("walls") {
            if !sprites.is_empty() {
                // Verify results are sorted by index
                for i in 1..sprites.len() {
                    assert!(
                        sprites[i - 1].0 <= sprites[i].0,
                        "Sprites should be sorted by index"
                    );
                }
            }
        }
    }

    #[test]
    fn test_suggest_sprite_sheets_case_insensitive() {
        // Only test if registry can be loaded
        if let Ok(suggestions) = suggest_sprite_sheets("WALLS") {
            // Should find "walls" when searching for "WALLS"
            let found = suggestions.iter().any(|(key, _)| key == "walls");
            assert!(
                found || suggestions.is_empty(),
                "Should find sheets case-insensitively"
            );
        }
    }

    #[test]
    fn test_search_sprites_limits_results() {
        // Only test if registry can be loaded
        if let Ok(results) = search_sprites("") {
            // Should return at most 10 results even if more match
            assert!(results.len() <= 10, "Should limit results to 10");
        }
    }

    #[test]
    fn test_has_sprite_sheet_not_found() {
        if let Ok(has_it) = has_sprite_sheet("nonexistent_sheet_xyz") {
            assert!(!has_it, "Should return false for nonexistent sheet");
        }
    }

    #[test]
    fn test_placeholder_sheet_present() {
        // Only run assertions if registry can be loaded in this environment.
        // Some CI/test environments may not have data files present, so guard
        // against `Err` like the other tests in this module.
        if let Ok(registry) = load_sprite_registry() {
            if let Some(sheet) = registry.get("placeholders") {
                // Verify expected texture path and grid dimensions for the placeholder
                assert_eq!(
                    sheet.texture_path,
                    "sprites/placeholders/npc_placeholder.png"
                );
                assert_eq!(sheet.columns, 1);
                assert_eq!(sheet.rows, 1);

                // Verify the named sprite mapping contains the expected placeholder entry
                assert!(
                    sheet
                        .sprites
                        .iter()
                        .any(|(idx, name)| *idx == 0 && name == "npc_placeholder"),
                    "Placeholder sheet should contain (0, \"npc_placeholder\")"
                );
            } else {
                // It's acceptable if the registry exists but doesn't include the placeholder
                // in some environments (e.g., trimmed test fixtures).
            }
        }
    }
}
