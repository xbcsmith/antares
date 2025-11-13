// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for starter map content
//!
//! This module tests the three starter maps created in Phase 3:
//! - Starter Town (safe zone with NPCs and shops)
//! - Starter Dungeon (combat encounters and treasure)
//! - Forest Area (natural terrain with mid-level encounters)

use antares::domain::types::Position;
use antares::domain::world::{Map, MapEvent, TerrainType, WallType};
use std::fs;

/// Helper function to load a map from RON file
fn load_map_from_file(filename: &str) -> Result<Map, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)?;
    let map: Map = ron::from_str(&content)?;
    Ok(map)
}

/// Helper function to count tiles matching a terrain type
fn count_terrain_tiles(map: &Map, terrain: TerrainType) -> usize {
    map.tiles
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| tile.terrain == terrain)
        .count()
}

/// Helper function to count tiles matching a wall type
fn count_wall_tiles(map: &Map, wall: WallType) -> usize {
    map.tiles
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| tile.wall_type == wall)
        .count()
}

#[test]
fn test_load_starter_town() {
    let map =
        load_map_from_file("data/maps/starter_town.ron").expect("Failed to load starter_town.ron");

    // Verify basic map properties
    assert_eq!(map.id, 1, "Starter town should have ID 1");
    assert_eq!(map.width, 20, "Starter town width should be 20");
    assert_eq!(map.height, 15, "Starter town height should be 15");

    // Verify tile grid dimensions
    assert_eq!(
        map.tiles.len(),
        15,
        "Map should have 15 rows of tiles (height)"
    );
    for (y, row) in map.tiles.iter().enumerate() {
        assert_eq!(row.len(), 20, "Row {} should have 20 tiles (width)", y);
    }

    // Verify total tile count
    let total_tiles: usize = map.tiles.iter().map(|row| row.len()).sum();
    assert_eq!(total_tiles, 300, "Should have 300 total tiles (20x15)");

    // Verify terrain types - should be mostly grass with some ground and stone
    let grass_count = count_terrain_tiles(&map, TerrainType::Grass);
    let ground_count = count_terrain_tiles(&map, TerrainType::Ground);
    let stone_count = count_terrain_tiles(&map, TerrainType::Stone);

    assert!(grass_count > 100, "Should have significant grass areas");
    assert!(ground_count > 20, "Should have ground border");
    assert!(stone_count > 0, "Should have stone for buildings");

    // Verify wall types - should have normal walls and doors
    let normal_walls = count_wall_tiles(&map, WallType::Normal);
    let doors = count_wall_tiles(&map, WallType::Door);

    assert!(normal_walls > 30, "Should have walls for buildings");
    assert_eq!(
        doors, 4,
        "Should have 4 doors (inn, shop, temple, dungeon exit)"
    );

    // Verify NPCs
    assert_eq!(map.npcs.len(), 4, "Should have 4 NPCs");

    // Check specific NPCs exist
    let elder = map.npcs.iter().find(|npc| npc.name == "Village Elder");
    assert!(elder.is_some(), "Village Elder should exist");
    assert_eq!(elder.unwrap().id, 1, "Village Elder should have ID 1");

    let innkeeper = map.npcs.iter().find(|npc| npc.name == "Innkeeper");
    assert!(innkeeper.is_some(), "Innkeeper should exist");

    let merchant = map.npcs.iter().find(|npc| npc.name == "Merchant");
    assert!(merchant.is_some(), "Merchant should exist");

    let priest = map.npcs.iter().find(|npc| npc.name == "High Priest");
    assert!(priest.is_some(), "High Priest should exist");

    // Verify events (signs)
    assert_eq!(map.events.len(), 4, "Should have 4 sign events");

    // Check for dungeon exit sign
    let dungeon_exit = map.events.get(&Position { x: 19, y: 7 });
    assert!(dungeon_exit.is_some(), "Should have dungeon exit at (19,7)");
}

#[test]
fn test_load_starter_dungeon() {
    let map = load_map_from_file("data/maps/starter_dungeon.ron")
        .expect("Failed to load starter_dungeon.ron");

    // Verify basic map properties
    assert_eq!(map.id, 2, "Starter dungeon should have ID 2");
    assert_eq!(map.width, 16, "Starter dungeon width should be 16");
    assert_eq!(map.height, 16, "Starter dungeon height should be 16");

    // Verify tile grid dimensions
    assert_eq!(
        map.tiles.len(),
        16,
        "Map should have 16 rows of tiles (height)"
    );
    for (y, row) in map.tiles.iter().enumerate() {
        assert_eq!(row.len(), 16, "Row {} should have 16 tiles (width)", y);
    }

    // Verify total tile count
    let total_tiles: usize = map.tiles.iter().map(|row| row.len()).sum();
    assert_eq!(total_tiles, 256, "Should have 256 total tiles (16x16)");

    // Verify terrain - should be all stone (dungeon)
    let stone_count = count_terrain_tiles(&map, TerrainType::Stone);
    assert_eq!(stone_count, 256, "All tiles should be stone in dungeon");

    // Verify wall types
    let normal_walls = count_wall_tiles(&map, WallType::Normal);
    let doors = count_wall_tiles(&map, WallType::Door);

    assert!(normal_walls > 60, "Should have many walls in dungeon");
    assert!(doors >= 3, "Should have multiple doors");

    // Verify no NPCs in dungeon
    assert_eq!(map.npcs.len(), 0, "Dungeon should have no NPCs");

    // Verify events (encounters, treasure, traps)
    assert!(map.events.len() >= 5, "Should have multiple events");

    // Count event types
    let encounters: Vec<&MapEvent> = map
        .events
        .values()
        .filter(|e| matches!(e, MapEvent::Encounter { .. }))
        .collect();
    let treasures: Vec<&MapEvent> = map
        .events
        .values()
        .filter(|e| matches!(e, MapEvent::Treasure { .. }))
        .collect();
    let traps: Vec<&MapEvent> = map
        .events
        .values()
        .filter(|e| matches!(e, MapEvent::Trap { .. }))
        .collect();

    assert!(encounters.len() >= 3, "Should have multiple encounters");
    assert!(treasures.len() >= 2, "Should have treasure chests");
    assert!(!traps.is_empty(), "Should have at least one trap");

    // Verify town exit at (0,7)
    let town_exit = map.events.get(&Position { x: 0, y: 7 });
    assert!(town_exit.is_some(), "Should have town exit at (0,7)");

    // Verify boss area encounter exists (near bottom-right)
    let has_boss = map.events.iter().any(|(pos, _)| pos.x >= 12 && pos.y >= 12);
    assert!(has_boss, "Should have encounter in boss area");
}

#[test]
fn test_load_forest_area() {
    let map =
        load_map_from_file("data/maps/forest_area.ron").expect("Failed to load forest_area.ron");

    // Verify basic map properties
    assert_eq!(map.id, 3, "Forest area should have ID 3");
    assert_eq!(map.width, 20, "Forest area width should be 20");
    assert_eq!(map.height, 20, "Forest area height should be 20");

    // Verify tile grid dimensions
    assert_eq!(
        map.tiles.len(),
        20,
        "Map should have 20 rows of tiles (height)"
    );
    for (y, row) in map.tiles.iter().enumerate() {
        assert_eq!(row.len(), 20, "Row {} should have 20 tiles (width)", y);
    }

    // Verify total tile count
    let total_tiles: usize = map.tiles.iter().map(|row| row.len()).sum();
    assert_eq!(total_tiles, 400, "Should have 400 total tiles (20x20)");

    // Verify terrain diversity - forest, grass, and water
    let forest_count = count_terrain_tiles(&map, TerrainType::Forest);
    let grass_count = count_terrain_tiles(&map, TerrainType::Grass);
    let water_count = count_terrain_tiles(&map, TerrainType::Water);

    assert!(
        forest_count > 30,
        "Should have significant forest areas (got {})",
        forest_count
    );
    assert!(
        grass_count > 30,
        "Should have significant grass areas (got {})",
        grass_count
    );
    assert!(
        water_count > 20,
        "Should have water features (got {})",
        water_count
    );

    // Verify wall types - mostly none with some normal walls on border
    let normal_walls = count_wall_tiles(&map, WallType::Normal);
    let doors = count_wall_tiles(&map, WallType::Door);

    assert!(normal_walls > 30, "Should have border walls");
    assert_eq!(doors, 1, "Should have 1 door (exit to town)");

    // Verify NPCs (should have at least one ranger/guide)
    assert!(!map.npcs.is_empty(), "Should have at least one NPC");
    let ranger = map.npcs.iter().find(|npc| npc.name == "Lost Ranger");
    assert!(ranger.is_some(), "Lost Ranger should exist");

    // Verify events (encounters, treasure, traps)
    assert!(map.events.len() >= 6, "Should have multiple events");

    // Count event types
    let encounters: Vec<&MapEvent> = map
        .events
        .values()
        .filter(|e| matches!(e, MapEvent::Encounter { .. }))
        .collect();
    let treasures: Vec<&MapEvent> = map
        .events
        .values()
        .filter(|e| matches!(e, MapEvent::Treasure { .. }))
        .collect();

    assert!(encounters.len() >= 3, "Should have multiple encounters");
    assert!(treasures.len() >= 2, "Should have hidden treasures");

    // Verify town exit at (0,10)
    let town_exit = map.events.get(&Position { x: 0, y: 10 });
    assert!(town_exit.is_some(), "Should have town exit at (0,10)");
}

#[test]
fn test_map_connections() {
    let town =
        load_map_from_file("data/maps/starter_town.ron").expect("Failed to load starter_town.ron");
    let dungeon = load_map_from_file("data/maps/starter_dungeon.ron")
        .expect("Failed to load starter_dungeon.ron");
    let forest =
        load_map_from_file("data/maps/forest_area.ron").expect("Failed to load forest_area.ron");

    // Verify map IDs are unique
    assert_ne!(town.id, dungeon.id, "Town and dungeon IDs must be unique");
    assert_ne!(town.id, forest.id, "Town and forest IDs must be unique");
    assert_ne!(
        dungeon.id, forest.id,
        "Dungeon and forest IDs must be unique"
    );

    // Verify town has exits to dungeon and forest
    let town_to_dungeon = town.events.get(&Position { x: 19, y: 7 });
    assert!(
        town_to_dungeon.is_some(),
        "Town should have exit to dungeon"
    );

    // Verify dungeon has exit back to town
    let dungeon_to_town = dungeon.events.get(&Position { x: 0, y: 7 });
    assert!(
        dungeon_to_town.is_some(),
        "Dungeon should have exit to town"
    );

    // Verify forest has exit back to town
    let forest_to_town = forest.events.get(&Position { x: 0, y: 10 });
    assert!(forest_to_town.is_some(), "Forest should have exit to town");
}

#[test]
fn test_map_tile_consistency() {
    let maps = vec![
        ("starter_town.ron", 20_u32, 15_u32),
        ("starter_dungeon.ron", 16_u32, 16_u32),
        ("forest_area.ron", 20_u32, 20_u32),
    ];

    for (filename, expected_width, expected_height) in maps {
        let map = load_map_from_file(&format!("data/maps/{}", filename))
            .unwrap_or_else(|_| panic!("Failed to load {}", filename));

        // Verify all tiles are initialized (not uninitialized state)
        for (y, row) in map.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                // All tiles should start as not visited
                assert!(
                    !tile.visited,
                    "Tile at ({},{}) in {} should start unvisited",
                    x, y, filename
                );
            }
        }

        // Verify dimensions match header
        assert_eq!(map.width, expected_width, "{} width mismatch", filename);
        assert_eq!(map.height, expected_height, "{} height mismatch", filename);
        assert_eq!(
            map.tiles.len() as u32,
            expected_height,
            "{} tile row count mismatch",
            filename
        );
        for row in &map.tiles {
            assert_eq!(
                row.len() as u32,
                expected_width,
                "{} tile column count mismatch",
                filename
            );
        }
    }
}

#[test]
fn test_event_positions_valid() {
    let maps = vec!["starter_town.ron", "starter_dungeon.ron", "forest_area.ron"];

    for filename in maps {
        let map = load_map_from_file(&format!("data/maps/{}", filename))
            .unwrap_or_else(|_| panic!("Failed to load {}", filename));

        for pos in map.events.keys() {
            assert!(
                pos.x < map.width as i32,
                "Event at ({},{}) in {} is outside map width {}",
                pos.x,
                pos.y,
                filename,
                map.width
            );
            assert!(
                pos.y < map.height as i32,
                "Event at ({},{}) in {} is outside map height {}",
                pos.x,
                pos.y,
                filename,
                map.height
            );
        }
    }
}

#[test]
fn test_npc_positions_valid() {
    let maps = vec!["starter_town.ron", "starter_dungeon.ron", "forest_area.ron"];

    for filename in maps {
        let map = load_map_from_file(&format!("data/maps/{}", filename))
            .unwrap_or_else(|_| panic!("Failed to load {}", filename));

        for npc in &map.npcs {
            assert!(
                npc.position.x < map.width as i32,
                "NPC '{}' at ({},{}) in {} is outside map width {}",
                npc.name,
                npc.position.x,
                npc.position.y,
                filename,
                map.width
            );
            assert!(
                npc.position.y < map.height as i32,
                "NPC '{}' at ({},{}) in {} is outside map height {}",
                npc.name,
                npc.position.x,
                npc.position.y,
                filename,
                map.height
            );
        }
    }
}

#[test]
fn test_npc_ids_unique_per_map() {
    let maps = vec!["starter_town.ron", "starter_dungeon.ron", "forest_area.ron"];

    for filename in maps {
        let map = load_map_from_file(&format!("data/maps/{}", filename))
            .unwrap_or_else(|_| panic!("Failed to load {}", filename));

        let npc_ids: Vec<u16> = map.npcs.iter().map(|npc| npc.id).collect();
        let mut unique_ids = npc_ids.clone();
        unique_ids.sort_unstable();
        unique_ids.dedup();

        assert_eq!(
            npc_ids.len(),
            unique_ids.len(),
            "Duplicate NPC IDs found in {}",
            filename
        );
    }
}
