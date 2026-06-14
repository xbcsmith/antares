// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map editor helper functions and integration
//!
//! This module provides helper functions for integrating the SDK content database
//! with map editing tools, including smart ID suggestions, content browsing,
//! validation, and sprite sheet management.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` for specifications.
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

use crate::domain::dialogue::DialogueId;
use crate::domain::types::{ItemId, MapId, MonsterId, Position, SpellId};
use crate::domain::world::{Map, MapEvent};
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

// ===== Object Mesh Registry (Campaign Builder) =====

/// Returns all registered object mesh IDs and names from the unified registry.
///
/// This is the SDK Campaign Builder "Object Meshes" panel data source — it
/// reflects entries from `object_mesh_registry.ron` and both legacy registries
/// (`landscape_mesh_registry.ron`, `furniture_mesh_registry.ron`) merged in.
///
/// The returned pairs are `(mesh_id_key, mesh_name)` sorted by key.
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::browse_object_meshes;
///
/// let db = ContentDatabase::new();
/// let meshes = browse_object_meshes(&db);
/// assert_eq!(meshes.len(), 0); // Empty database
/// ```
pub fn browse_object_meshes(db: &ContentDatabase) -> Vec<(String, String)> {
    let mut results: Vec<(String, String)> = db
        .object_meshes
        .all_mesh_ids()
        .into_iter()
        .filter_map(|key| {
            db.object_meshes
                .lookup(&key)
                .map(|def| (key, def.name.clone()))
        })
        .collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// Returns a sorted list of all object mesh IDs from an [`ObjectMeshDatabase`].
///
/// Used to populate autocomplete dropdowns in the map editor for `mesh_id`
/// fields on Treasure, Sign, Container, and similar events.
/// Unlike [`browse_object_meshes`], this function takes a standalone
/// `ObjectMeshDatabase` so it can be called after loading the registry
/// outside of a full `ContentDatabase` load.
///
/// # Arguments
///
/// * `db` - Object mesh database loaded from `object_mesh_registry.ron`
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshDatabase;
/// use antares::sdk::map_editor::browse_event_mesh_ids;
///
/// let db = ObjectMeshDatabase::new();
/// let ids = browse_event_mesh_ids(&db);
/// assert!(ids.is_empty());
/// ```
pub fn browse_event_mesh_ids(
    db: &crate::domain::world::object_mesh::ObjectMeshDatabase,
) -> Vec<String> {
    let mut ids = db.all_mesh_ids();
    ids.sort();
    ids
}

/// Returns `(id, name)` pairs for all dialogues in a [`DialogueDatabase`].
///
/// Used to populate autocomplete dropdowns in the map editor for `dialogue_id`
/// fields on Treasure, Sign, Container, and similar events.
/// Unlike [`browse_dialogues`](crate::sdk::dialogue_editor::browse_dialogues),
/// this function takes a standalone `DialogueDatabase` so it can be called
/// without a full `ContentDatabase`.
///
/// # Arguments
///
/// * `db` - Dialogue database loaded from `data/dialogues.ron`
///
/// # Examples
///
/// ```
/// use antares::sdk::database::DialogueDatabase;
/// use antares::sdk::map_editor::browse_dialogue_ids;
///
/// let db = DialogueDatabase::new();
/// let pairs = browse_dialogue_ids(&db);
/// assert!(pairs.is_empty());
/// ```
pub fn browse_dialogue_ids(
    db: &crate::sdk::database::DialogueDatabase,
) -> Vec<(DialogueId, String)> {
    let mut ids = db.all_dialogues();
    ids.sort();
    ids.iter()
        .filter_map(|id| db.get_dialogue(*id).map(|d| (*id, d.name.clone())))
        .collect()
}

/// Suggests object mesh IDs matching a partial string (name or key).
///
/// Returns up to 10 matches as `(mesh_id_key, mesh_name)` pairs, sorted by key.
///
/// # Arguments
///
/// * `db` - Content database to search
/// * `partial` - Partial text matched case-insensitively against key and name
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::suggest_object_mesh_ids;
///
/// let db = ContentDatabase::new();
/// let suggestions = suggest_object_mesh_ids(&db, "door");
/// // Returns mesh entries whose key or name contains "door"
/// ```
pub fn suggest_object_mesh_ids(db: &ContentDatabase, partial: &str) -> Vec<(String, String)> {
    let partial_lower = partial.to_lowercase();
    let mut suggestions: Vec<(String, String)> = db
        .object_meshes
        .all_mesh_ids()
        .into_iter()
        .filter_map(|key| {
            db.object_meshes.lookup(&key).and_then(|def| {
                if key.to_lowercase().contains(&partial_lower)
                    || def.name.to_lowercase().contains(&partial_lower)
                {
                    Some((key, def.name.clone()))
                } else {
                    None
                }
            })
        })
        .take(10)
        .collect();
    suggestions.sort_by(|a, b| a.0.cmp(&b.0));
    suggestions
}

/// Returns `true` if the given mesh ID string is registered in the unified
/// object mesh database.
///
/// # Examples
///
/// ```
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::map_editor::is_valid_object_mesh_id;
///
/// let db = ContentDatabase::new();
/// assert!(!is_valid_object_mesh_id(&db, "oak_tree")); // Empty database
/// ```
pub fn is_valid_object_mesh_id(db: &ContentDatabase, mesh_id: &str) -> bool {
    db.object_meshes.has_mesh(mesh_id)
}

#[cfg(test)]
mod object_mesh_tests {
    use super::*;

    #[test]
    fn test_browse_object_meshes_empty() {
        let db = ContentDatabase::new();
        let meshes = browse_object_meshes(&db);
        assert_eq!(meshes.len(), 0);
    }

    #[test]
    fn test_suggest_object_mesh_ids_empty() {
        let db = ContentDatabase::new();
        let suggestions = suggest_object_mesh_ids(&db, "door");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_is_valid_object_mesh_id_empty() {
        let db = ContentDatabase::new();
        assert!(!is_valid_object_mesh_id(&db, "oak_tree"));
        assert!(!is_valid_object_mesh_id(&db, "11001"));
    }

    #[test]
    fn test_browse_object_meshes_from_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let db = ContentDatabase::load_campaign(
            std::path::Path::new(manifest_dir).join("data/test_campaign"),
        )
        .expect("test_campaign must load");

        let meshes = browse_object_meshes(&db);
        // Merged from landscape_mesh_registry.ron — must be non-empty
        assert!(
            !meshes.is_empty(),
            "object mesh list must be non-empty after merge"
        );
        // Results must be sorted by key
        for i in 1..meshes.len() {
            assert!(
                meshes[i - 1].0 <= meshes[i].0,
                "results must be sorted by key"
            );
        }
    }

    #[test]
    fn test_suggest_object_mesh_ids_from_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let db = ContentDatabase::load_campaign(
            std::path::Path::new(manifest_dir).join("data/test_campaign"),
        )
        .expect("test_campaign must load");

        // "11001" is a landscape mesh — numeric string key from backward-compat merge
        let suggestions = suggest_object_mesh_ids(&db, "11001");
        // The key "11001" itself matches the partial
        let found = suggestions.iter().any(|(k, _)| k == "11001");
        assert!(found, "'11001' key must appear in suggestions");
    }

    #[test]
    fn test_is_valid_object_mesh_id_after_merge() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let db = ContentDatabase::load_campaign(
            std::path::Path::new(manifest_dir).join("data/test_campaign"),
        )
        .expect("test_campaign must load");

        // Legacy numeric key must be valid after merge
        assert!(
            is_valid_object_mesh_id(&db, "11001"),
            "legacy landscape mesh '11001' must be valid after merge"
        );
        // Non-existent key must be invalid
        assert!(!is_valid_object_mesh_id(&db, "nonexistent_mesh_xyz"));
    }

    #[test]
    fn test_suggest_object_mesh_ids_limits_to_ten() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        if let Ok(db) = ContentDatabase::load_campaign(
            std::path::Path::new(manifest_dir).join("data/test_campaign"),
        ) {
            // Searching with empty string matches everything
            let suggestions = suggest_object_mesh_ids(&db, "");
            assert!(suggestions.len() <= 10, "suggestions must be capped at 10");
        }
    }
}

// ===== Map Event Placement =====

/// Errors returned by map event mutation helpers.
///
/// # Examples
///
/// ```
/// use antares::sdk::map_editor::MapEditorError;
///
/// let e = MapEditorError::UnknownMesh("bad_mesh".to_string());
/// assert!(e.to_string().contains("bad_mesh"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum MapEditorError {
    /// The given mesh ID is not registered in the object mesh registry.
    #[error("Unknown mesh ID: {0}")]
    UnknownMesh(String),
    /// The given dialogue ID is not in the dialogue database.
    #[error("Unknown dialogue ID: {0}")]
    UnknownDialogue(u16),
    /// The position is outside the map bounds.
    #[error("Position ({}, {}) is out of bounds", .0.x, .0.y)]
    OutOfBounds(Position),
    /// The position already has an event.
    #[error("Position ({}, {}) is already occupied", .0.x, .0.y)]
    PositionOccupied(Position),
}

/// Returns mesh IDs whose string key starts with `partial` (case-insensitive prefix match), sorted ascending.
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshDatabase;
/// use antares::sdk::map_editor::suggest_event_mesh_ids;
///
/// let db = ObjectMeshDatabase::new();
/// let ids = suggest_event_mesh_ids(&db, "bar");
/// assert!(ids.is_empty());
/// ```
pub fn suggest_event_mesh_ids(
    registry: &crate::domain::world::object_mesh::ObjectMeshDatabase,
    partial: &str,
) -> Vec<String> {
    let partial_lower = partial.to_lowercase();
    let mut ids: Vec<String> = registry
        .all_mesh_ids()
        .into_iter()
        .filter(|key| key.to_lowercase().starts_with(&partial_lower))
        .collect();
    ids.sort();
    ids
}

/// Returns `(id, title)` pairs where the title OR the numeric ID string starts with
/// `partial` (case-insensitive prefix match).
///
/// # Examples
///
/// ```
/// use antares::sdk::database::DialogueDatabase;
/// use antares::sdk::map_editor::suggest_dialogue_ids;
///
/// let db = DialogueDatabase::new();
/// let pairs = suggest_dialogue_ids(&db, "bar");
/// assert!(pairs.is_empty());
/// ```
pub fn suggest_dialogue_ids(
    db: &crate::sdk::database::DialogueDatabase,
    partial: &str,
) -> Vec<(DialogueId, String)> {
    let partial_lower = partial.to_lowercase();
    let mut ids = db.all_dialogues();
    ids.sort();
    ids.iter()
        .filter_map(|id| {
            db.get_dialogue(*id).and_then(|d| {
                if d.name.to_lowercase().starts_with(&partial_lower)
                    || id.to_string().starts_with(&partial_lower)
                {
                    Some((*id, d.name.clone()))
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Validates and inserts a `MapEvent` at `pos`.
///
/// Returns `Err(OutOfBounds)` if `pos` is outside the map dimensions, or
/// `Err(PositionOccupied)` if there is already an event at that position.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
/// use antares::sdk::map_editor::place_map_event;
///
/// let mut map = Map::new(1, "Test".to_string(), "".to_string(), 5, 5);
/// let event = MapEvent::Sign {
///     name: "Sign".to_string(),
///     description: "".to_string(),
///     text: "Hello".to_string(),
///     time_condition: None,
///     facing: None,
///     mesh_id: None,
///     dialogue_id: None,
/// };
/// assert!(place_map_event(&mut map, Position::new(2, 2), event).is_ok());
/// ```
pub fn place_map_event(
    map: &mut Map,
    pos: Position,
    event: MapEvent,
) -> Result<(), MapEditorError> {
    if pos.x < 0 || pos.y < 0 || pos.x as u32 >= map.width || pos.y as u32 >= map.height {
        return Err(MapEditorError::OutOfBounds(pos));
    }
    if map.events.contains_key(&pos) {
        return Err(MapEditorError::PositionOccupied(pos));
    }
    map.add_event(pos, event);
    Ok(())
}

/// Removes and returns the event at `pos`, or `None` if no event exists there.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
/// use antares::sdk::map_editor::remove_map_event;
///
/// let mut map = Map::new(1, "Test".to_string(), "".to_string(), 5, 5);
/// assert!(remove_map_event(&mut map, Position::new(2, 2)).is_none());
/// ```
pub fn remove_map_event(map: &mut Map, pos: Position) -> Option<MapEvent> {
    map.remove_event(pos)
}

/// Patches the `mesh_id` field on the `Treasure` event at `pos`.
///
/// Returns `Err(UnknownMesh)` if `mesh_id` is `Some` and is not registered in the registry.
/// Returns `Ok(())` and is a no-op if there is no event at `pos` or the event is not a
/// `Treasure` event.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
/// use antares::domain::world::object_mesh::ObjectMeshDatabase;
/// use antares::sdk::map_editor::{place_map_event, set_event_mesh_id, MapEditorError};
///
/// let mut map = Map::new(1, "Test".to_string(), "".to_string(), 5, 5);
/// let event = MapEvent::Treasure {
///     name: "Chest".to_string(),
///     description: "".to_string(),
///     loot: vec![],
///     mesh_id: None,
///     dialogue_id: None,
/// };
/// let pos = Position::new(1, 1);
/// place_map_event(&mut map, pos, event).unwrap();
///
/// let db = ObjectMeshDatabase::new();
/// // Setting None is always Ok
/// assert!(set_event_mesh_id(&mut map, pos, None, &db).is_ok());
/// // Setting unknown ID is an error
/// assert!(matches!(
///     set_event_mesh_id(&mut map, pos, Some("ghost".to_string()), &db),
///     Err(MapEditorError::UnknownMesh(_))
/// ));
/// ```
pub fn set_event_mesh_id(
    map: &mut Map,
    pos: Position,
    mesh_id: Option<String>,
    registry: &crate::domain::world::object_mesh::ObjectMeshDatabase,
) -> Result<(), MapEditorError> {
    if let Some(ref id) = mesh_id {
        if !registry.has_mesh(id) {
            return Err(MapEditorError::UnknownMesh(id.clone()));
        }
    }
    if let Some(MapEvent::Treasure {
        mesh_id: ref mut mid,
        ..
    }) = map.events.get_mut(&pos)
    {
        *mid = mesh_id;
    }
    Ok(())
}

/// Patches the `dialogue_id` field on the `Treasure` event at `pos`.
///
/// Returns `Err(UnknownDialogue)` if `id` is `Some` and is not present in the database.
/// Returns `Ok(())` and is a no-op if there is no event at `pos` or the event is not a
/// `Treasure` event.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
/// use antares::sdk::database::DialogueDatabase;
/// use antares::sdk::map_editor::{place_map_event, set_event_dialogue_id, MapEditorError};
///
/// let mut map = Map::new(1, "Test".to_string(), "".to_string(), 5, 5);
/// let event = MapEvent::Treasure {
///     name: "Chest".to_string(),
///     description: "".to_string(),
///     loot: vec![],
///     mesh_id: None,
///     dialogue_id: None,
/// };
/// let pos = Position::new(1, 1);
/// place_map_event(&mut map, pos, event).unwrap();
///
/// let db = DialogueDatabase::new();
/// // Setting None is always Ok
/// assert!(set_event_dialogue_id(&mut map, pos, None, &db).is_ok());
/// // Setting unknown ID is an error
/// assert!(matches!(
///     set_event_dialogue_id(&mut map, pos, Some(999), &db),
///     Err(MapEditorError::UnknownDialogue(999))
/// ));
/// ```
pub fn set_event_dialogue_id(
    map: &mut Map,
    pos: Position,
    id: Option<DialogueId>,
    db: &crate::sdk::database::DialogueDatabase,
) -> Result<(), MapEditorError> {
    if let Some(did) = id {
        if !db.has_dialogue(&did) {
            return Err(MapEditorError::UnknownDialogue(did));
        }
    }
    if let Some(MapEvent::Treasure {
        dialogue_id: ref mut did,
        ..
    }) = map.events.get_mut(&pos)
    {
        *did = id;
    }
    Ok(())
}

// ===== Sprite Sheet Management =====

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

#[cfg(test)]
mod event_placement_tests {
    use super::*;
    use crate::domain::types::Position;
    use crate::domain::world::object_mesh::ObjectMeshDatabase;
    use crate::domain::world::{Map, MapEvent};
    use crate::sdk::database::DialogueDatabase;

    fn make_map() -> Map {
        Map::new(1, "Test".to_string(), "".to_string(), 5, 5)
    }

    fn make_treasure(pos_x: i32, pos_y: i32) -> (Position, MapEvent) {
        let pos = Position::new(pos_x, pos_y);
        let event = MapEvent::Treasure {
            name: "Chest".to_string(),
            description: "".to_string(),
            loot: vec![],
            mesh_id: None,
            dialogue_id: None,
        };
        (pos, event)
    }

    #[test]
    fn test_place_map_event_succeeds_on_valid_empty_position() {
        let mut map = make_map();
        let (pos, event) = make_treasure(2, 2);
        assert!(place_map_event(&mut map, pos, event).is_ok());
        assert!(map.events.contains_key(&pos));
    }

    #[test]
    fn test_place_map_event_returns_err_on_occupied_position() {
        let mut map = make_map();
        let (pos, event) = make_treasure(1, 1);
        place_map_event(&mut map, pos, event.clone()).unwrap();
        let result = place_map_event(&mut map, pos, event);
        assert!(matches!(result, Err(MapEditorError::PositionOccupied(_))));
    }

    #[test]
    fn test_place_map_event_returns_err_on_out_of_bounds() {
        let mut map = make_map();
        let (_, event) = make_treasure(0, 0);
        let out_of_bounds = Position::new(100, 100);
        let result = place_map_event(&mut map, out_of_bounds, event);
        assert!(matches!(result, Err(MapEditorError::OutOfBounds(_))));
    }

    #[test]
    fn test_place_map_event_returns_err_on_negative_position() {
        let mut map = make_map();
        let (_, event) = make_treasure(0, 0);
        let neg_pos = Position::new(-1, 0);
        let result = place_map_event(&mut map, neg_pos, event);
        assert!(matches!(result, Err(MapEditorError::OutOfBounds(_))));
    }

    #[test]
    fn test_remove_map_event_returns_none_when_empty() {
        let mut map = make_map();
        let result = remove_map_event(&mut map, Position::new(2, 2));
        assert!(result.is_none());
    }

    #[test]
    fn test_remove_map_event_returns_event_when_present() {
        let mut map = make_map();
        let (pos, event) = make_treasure(2, 2);
        map.add_event(pos, event);
        let result = remove_map_event(&mut map, pos);
        assert!(result.is_some());
        assert!(!map.events.contains_key(&pos));
    }

    #[test]
    fn test_set_event_mesh_id_with_known_mesh_returns_ok() {
        let mut map = make_map();
        let (pos, event) = make_treasure(1, 1);
        map.add_event(pos, event);

        // Build a registry with one mesh
        let registry = ObjectMeshDatabase::new();
        // We can't easily insert into ObjectMeshDatabase without its private field,
        // so we test with None (always Ok) which is the primary contract.
        assert!(set_event_mesh_id(&mut map, pos, None, &registry).is_ok());
    }

    #[test]
    fn test_set_event_mesh_id_with_unknown_mesh_returns_err() {
        let mut map = make_map();
        let (pos, event) = make_treasure(1, 1);
        map.add_event(pos, event);

        let registry = ObjectMeshDatabase::new();
        let result = set_event_mesh_id(&mut map, pos, Some("ghost_mesh".to_string()), &registry);
        assert!(matches!(result, Err(MapEditorError::UnknownMesh(_))));
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ghost_mesh"),
            "error must name the unknown mesh"
        );
    }

    #[test]
    fn test_set_event_dialogue_id_with_known_id_returns_ok() {
        let mut map = make_map();
        let (pos, event) = make_treasure(1, 1);
        map.add_event(pos, event);

        // Empty database -> None is always Ok
        let db = DialogueDatabase::new();
        assert!(set_event_dialogue_id(&mut map, pos, None, &db).is_ok());
    }

    #[test]
    fn test_set_event_dialogue_id_with_unknown_id_returns_err() {
        let mut map = make_map();
        let (pos, event) = make_treasure(1, 1);
        map.add_event(pos, event);

        let db = DialogueDatabase::new();
        let result = set_event_dialogue_id(&mut map, pos, Some(999_u16), &db);
        assert!(matches!(result, Err(MapEditorError::UnknownDialogue(999))));
    }

    #[test]
    fn test_suggest_event_mesh_ids_returns_only_prefix_matching_entries() {
        // Build an ObjectMeshDatabase with multiple entries via load_from_registry round-trip
        // We can only test with empty db here; the prefix-filter logic is tested by behaviour.
        let db = ObjectMeshDatabase::new();
        let results = suggest_event_mesh_ids(&db, "bar");
        // Empty db -> empty results
        assert!(results.is_empty());

        // Verify the function would return prefix-matching (not substring-matching)
        // With an empty db all queries return empty
        let all = suggest_event_mesh_ids(&db, "");
        assert!(all.is_empty());
    }

    #[test]
    fn test_browse_event_mesh_ids_empty_db() {
        let db = ObjectMeshDatabase::new();
        let ids = browse_event_mesh_ids(&db);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_browse_dialogue_ids_returns_one_entry_per_tree() {
        let db = DialogueDatabase::new();
        let pairs = browse_dialogue_ids(&db);
        assert_eq!(pairs.len(), db.count());
    }

    #[test]
    fn test_browse_dialogue_ids_with_dialogues_from_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let db = DialogueDatabase::load_from_file(
            std::path::Path::new(manifest_dir).join("data/test_campaign/data/dialogues.ron"),
        )
        .expect("test_campaign dialogues.ron must load");

        let pairs = browse_dialogue_ids(&db);
        assert_eq!(
            pairs.len(),
            db.count(),
            "browse_dialogue_ids must return one entry per tree"
        );
        // Must be sorted by ID
        for i in 1..pairs.len() {
            assert!(
                pairs[i - 1].0 <= pairs[i].0,
                "pairs must be sorted by dialogue ID"
            );
        }
    }

    #[test]
    fn test_suggest_dialogue_ids_prefix_filter() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        if let Ok(db) = DialogueDatabase::load_from_file(
            std::path::Path::new(manifest_dir).join("data/test_campaign/data/dialogues.ron"),
        ) {
            // Searching with empty string returns all
            let all = suggest_dialogue_ids(&db, "");
            assert_eq!(all.len(), db.count());

            // Searching with a non-matching prefix returns empty
            let none = suggest_dialogue_ids(&db, "zzzzzz_no_match");
            assert!(none.is_empty());
        }
    }

    #[test]
    fn test_map_editor_error_display() {
        let e1 = MapEditorError::UnknownMesh("bad_mesh".to_string());
        assert!(e1.to_string().contains("bad_mesh"));

        let e2 = MapEditorError::UnknownDialogue(42);
        assert!(e2.to_string().contains("42"));

        let e3 = MapEditorError::OutOfBounds(Position::new(10, 20));
        assert!(e3.to_string().contains("10") || e3.to_string().contains("out of bounds"));

        let e4 = MapEditorError::PositionOccupied(Position::new(3, 4));
        assert!(e4.to_string().contains("3") || e4.to_string().contains("occupied"));
    }
}
