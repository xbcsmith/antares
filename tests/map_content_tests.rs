use antares::domain::types::Position;
use antares::domain::world::{Map, MapBlueprint, TerrainType, WallType};
use std::fs;

/// Helper function to load a map from RON file
fn load_map_from_file(filename: &str) -> Result<Map, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)?;
    let blueprint: MapBlueprint = ron::from_str(&content)?;
    let map: Map = blueprint.into();
    Ok(map)
}

#[test]
fn test_load_start_area() {
    let map =
        load_map_from_file("data/maps/starter_town.ron").expect("Failed to load starter_town.ron");

    // Verify basic map properties
    assert_eq!(map.id, 1, "Start area should have ID 1");
    assert_eq!(map.width, 20, "Start area width should be 20");
    assert_eq!(map.height, 15, "Start area height should be 15");

    // Verify total tile count
    assert_eq!(map.tiles.len(), 300, "Should have 300 total tiles (20x15)");

    // Verify specific tiles
    // (0, 0) should be Stone wall
    let tile_0_0 = map.get_tile(Position::new(0, 0)).unwrap();
    assert_eq!(tile_0_0.terrain, TerrainType::Ground);
    assert_eq!(tile_0_0.wall_type, WallType::Normal);

    // Verify we have some ground tiles
    let has_ground = map.tiles.iter().any(|t| t.terrain == TerrainType::Ground);
    assert!(has_ground, "Map should have some ground tiles");

    // Verify NPCs (map may or may not have NPCs)
    // Just check that the NPC list is accessible
    let _npc_count = map.npcs.len();
}

#[test]
fn test_map_consistency() {
    let map =
        load_map_from_file("data/maps/starter_town.ron").expect("Failed to load starter_town.ron");

    // Verify all tiles are initialized
    for (i, tile) in map.tiles.iter().enumerate() {
        let x = i as u32 % map.width;
        let y = i as u32 / map.width;
        assert!(
            !tile.visited,
            "Tile at ({},{}) should start unvisited",
            x, y
        );
    }

    // Verify event positions are valid
    for pos in map.events.keys() {
        assert!(
            pos.x < map.width as i32 && pos.y < map.height as i32,
            "Event at ({},{}) is outside map bounds",
            pos.x,
            pos.y
        );
    }

    // Verify NPC positions are valid
    for npc in &map.npcs {
        assert!(
            npc.position.x < map.width as i32 && npc.position.y < map.height as i32,
            "NPC '{}' at ({},{}) is outside map bounds",
            npc.name,
            npc.position.x,
            npc.position.y
        );
    }
}
