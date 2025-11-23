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
    let map = load_map_from_file("campaigns/tutorial/data/maps/start_area.ron")
        .expect("Failed to load start_area.ron");

    // Verify basic map properties
    assert_eq!(map.id, 1, "Start area should have ID 1");
    assert_eq!(map.width, 15, "Start area width should be 15");
    assert_eq!(map.height, 15, "Start area height should be 15");

    // Verify total tile count
    assert_eq!(map.tiles.len(), 225, "Should have 225 total tiles (15x15)");

    // Verify specific tiles
    // (0, 0) should be Floor (Ground)
    let tile_0_0 = map.get_tile(Position::new(0, 0)).unwrap();
    assert_eq!(tile_0_0.terrain, TerrainType::Ground);
    assert_eq!(tile_0_0.wall_type, WallType::None);

    // (3, 1) should be Wall (Ground + WallType::Normal)
    // Row 1, Col 3 -> index 15 + 3 = 18
    // start_area.ron: Row 1: Floor, Floor, Floor, Wall...
    let tile_3_1 = map.get_tile(Position::new(3, 1)).unwrap();
    assert_eq!(tile_3_1.terrain, TerrainType::Ground);
    assert_eq!(tile_3_1.wall_type, WallType::Normal);

    // Verify events
    // (7, 3) has Text event
    let event_pos = Position::new(7, 3);
    assert!(
        map.events.contains_key(&event_pos),
        "Should have event at (7, 3)"
    );

    // Verify NPCs
    assert_eq!(map.npcs.len(), 1, "Should have 1 NPC");
    let npc = &map.npcs[0];
    assert_eq!(npc.name, "Training Master");
    assert_eq!(npc.position, Position::new(7, 2));
}

#[test]
fn test_map_consistency() {
    let map = load_map_from_file("campaigns/tutorial/data/maps/start_area.ron")
        .expect("Failed to load start_area.ron");

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
