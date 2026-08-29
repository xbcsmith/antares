use antares::domain::types::Position;
use antares::domain::world::terrain::TERRAIN_GROUND;
use antares::domain::world::{Map, WallType};
use std::fs;

/// Helper function to load a map from RON file
fn load_map_from_file(filename: &str) -> Result<Map, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)?;
    let map: Map = ron::from_str(&content)?;
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
    assert_eq!(tile_0_0.terrain, TERRAIN_GROUND);
    assert_eq!(tile_0_0.wall_type, WallType::Normal);

    // Verify we have some ground tiles
    let has_ground = map.tiles.iter().any(|t| t.terrain == TERRAIN_GROUND);
    assert!(has_ground, "Map should have some ground tiles");

    // Verify NPC placements (map may or may not have NPCs)
    // Just check that the NPC placement list is accessible
    let _npc_count = map.npc_placements.len();
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

    // Verify NPC placement positions are valid
    for placement in &map.npc_placements {
        assert!(
            placement.position.x < map.width as i32 && placement.position.y < map.height as i32,
            "NPC placement '{}' at ({},{}) is outside map bounds",
            placement.npc_id,
            placement.position.x,
            placement.position.y
        );
    }
}

/// Verify all campaign map RON files use numeric terrain IDs (>= TERRAIN_ID_MIN).
///
/// This is the Phase 6 migration verification test: every tile's terrain field
/// must be a valid TerrainId integer, not a legacy enum variant name.
#[test]
fn test_all_campaign_maps_load_with_numeric_terrain_ids() {
    use antares::domain::types::TERRAIN_ID_MIN;
    use std::path::PathBuf;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // Map directories to scan: data/test_campaign and campaigns/
    let mut map_dirs: Vec<PathBuf> =
        vec![PathBuf::from(manifest_dir).join("data/test_campaign/data/maps")];

    // Dynamically discover any campaign map directories under campaigns/
    let campaigns_root = PathBuf::from(manifest_dir).join("campaigns");
    if campaigns_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&campaigns_root) {
            for entry in entries.flatten() {
                let maps_dir = entry.path().join("data/maps");
                if maps_dir.is_dir() {
                    map_dirs.push(maps_dir);
                }
            }
        }
    }

    let mut total_checked = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for maps_dir in &map_dirs {
        let entries = match std::fs::read_dir(maps_dir) {
            Ok(e) => e,
            Err(err) => {
                errors.push(format!("Cannot read dir {:?}: {}", maps_dir, err));
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ron") {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(err) => {
                    errors.push(format!("Cannot read {:?}: {}", path, err));
                    continue;
                }
            };

            let map: Map = match ron::from_str(&content) {
                Ok(m) => m,
                Err(err) => {
                    errors.push(format!("RON parse error in {:?}: {}", path, err));
                    continue;
                }
            };

            for tile in &map.tiles {
                if tile.terrain < TERRAIN_ID_MIN {
                    errors.push(format!(
                        "{:?}: tile ({},{}) has terrain ID {} which is below TERRAIN_ID_MIN={}",
                        path, tile.x, tile.y, tile.terrain, TERRAIN_ID_MIN
                    ));
                }
            }

            total_checked += 1;
        }
    }

    assert!(
        errors.is_empty(),
        "Campaign map migration errors ({} maps checked):\n{}",
        total_checked,
        errors.join("\n")
    );
    assert!(
        total_checked > 0,
        "No map RON files found — check that data/test_campaign/data/maps/ exists"
    );
}
