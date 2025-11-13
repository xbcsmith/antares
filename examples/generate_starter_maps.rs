// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Generate Starter Maps for Antares RPG
//!
//! This example creates the three starter maps for the game:
//! - Map 1: Starter Town (safe zone with NPCs)
//! - Map 2: Starter Dungeon (combat encounters)
//! - Map 3: Forest Area (wilderness exploration)
//!
//! Run with: cargo run --example generate_starter_maps

use antares::domain::types::Position;
use antares::domain::world::{Map, MapEvent, Npc, TerrainType, Tile, WallType};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure data/maps directory exists
    fs::create_dir_all("data/maps")?;

    // Generate all three maps
    generate_starter_town()?;
    generate_starter_dungeon()?;
    generate_forest_area()?;

    println!("✅ All starter maps generated successfully!");
    Ok(())
}

fn generate_starter_town() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Map::new(1, 20, 15);

    // Set borders to ground with walls
    for x in 0..20 {
        map.get_tile_mut(Position::new(x, 0)).unwrap().wall_type = WallType::Normal;
        map.get_tile_mut(Position::new(x, 14)).unwrap().wall_type = WallType::Normal;
    }
    for y in 0..15 {
        map.get_tile_mut(Position::new(0, y)).unwrap().wall_type = WallType::Normal;
        map.get_tile_mut(Position::new(19, y)).unwrap().wall_type = WallType::Normal;
    }

    // Fill interior with grass
    for y in 1..14 {
        for x in 1..19 {
            *map.get_tile_mut(Position::new(x, y)).unwrap() =
                Tile::new(TerrainType::Grass, WallType::None);
        }
    }

    // Inn building (3-5, 1-5)
    for y in 1..6 {
        for x in 3..6 {
            *map.get_tile_mut(Position::new(x, y)).unwrap() =
                Tile::new(TerrainType::Stone, WallType::Normal);
        }
    }
    *map.get_tile_mut(Position::new(4, 4)).unwrap() = Tile::new(TerrainType::Stone, WallType::Door);

    // Shop building (14-16, 1-5)
    for y in 1..6 {
        for x in 14..17 {
            *map.get_tile_mut(Position::new(x, y)).unwrap() =
                Tile::new(TerrainType::Stone, WallType::Normal);
        }
    }
    *map.get_tile_mut(Position::new(15, 4)).unwrap() =
        Tile::new(TerrainType::Stone, WallType::Door);

    // Temple building (9-11, 8-11)
    for y in 8..12 {
        for x in 9..12 {
            *map.get_tile_mut(Position::new(x, y)).unwrap() =
                Tile::new(TerrainType::Stone, WallType::Normal);
        }
    }
    *map.get_tile_mut(Position::new(10, 10)).unwrap() =
        Tile::new(TerrainType::Stone, WallType::Door);

    // Dungeon exit door
    *map.get_tile_mut(Position::new(19, 7)).unwrap() =
        Tile::new(TerrainType::Stone, WallType::Door);

    // Add NPCs
    map.add_npc(Npc::new(
        1,
        "Village Elder".to_string(),
        Position::new(10, 4),
        "Greetings, brave adventurers! Dark forces stir in the dungeon to the east. Will you help us?".to_string(),
    ));
    map.add_npc(Npc::new(
        2,
        "Innkeeper".to_string(),
        Position::new(4, 3),
        "Welcome to my inn! You look weary from your travels. Rest here and recover your strength."
            .to_string(),
    ));
    map.add_npc(Npc::new(
        3,
        "Merchant".to_string(),
        Position::new(15, 3),
        "I have the finest goods in the land! Take a look at my wares.".to_string(),
    ));
    map.add_npc(Npc::new(
        4,
        "High Priest".to_string(),
        Position::new(10, 9),
        "May the light guide you. I can heal your wounds and cure your ailments.".to_string(),
    ));

    // Add sign events
    map.add_event(
        Position::new(4, 4),
        MapEvent::Sign {
            text: "Welcome to the Cozy Inn! Rest and manage your party here.".to_string(),
        },
    );
    map.add_event(
        Position::new(15, 4),
        MapEvent::Sign {
            text: "General Store - Buy and sell items for your adventures.".to_string(),
        },
    );
    map.add_event(
        Position::new(10, 10),
        MapEvent::Sign {
            text: "Temple of Healing - The priests can restore your health.".to_string(),
        },
    );
    map.add_event(
        Position::new(19, 7),
        MapEvent::Sign {
            text: "WARNING: Dungeon entrance ahead. Prepare for combat!".to_string(),
        },
    );

    let ron = ron::ser::to_string_pretty(&map, ron::ser::PrettyConfig::default())?;
    fs::write("data/maps/starter_town.ron", ron)?;
    println!("✅ Created starter_town.ron");
    Ok(())
}

fn generate_starter_dungeon() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Map::new(2, 16, 16);

    // Fill entire dungeon with stone
    for y in 0..16 {
        for x in 0..16 {
            *map.get_tile_mut(Position::new(x, y)).unwrap() =
                Tile::new(TerrainType::Stone, WallType::Normal);
        }
    }

    // Create rooms and corridors by removing walls
    // Room 1: Top-left (1-3, 1-3)
    for y in 1..4 {
        for x in 1..4 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Corridor to room 2
    for x in 4..8 {
        map.get_tile_mut(Position::new(x, 2)).unwrap().wall_type = WallType::None;
    }

    // Room 2: Top-middle (5-11, 1-3)
    for y in 1..4 {
        for x in 5..12 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Door to room 3
    map.get_tile_mut(Position::new(12, 3)).unwrap().wall_type = WallType::Door;

    // Room 3: Top-right (13-14, 1-3)
    for y in 1..4 {
        for x in 13..15 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Western corridor (1-6, 5-9)
    for y in 5..10 {
        for x in 1..7 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Door from west corridor
    map.get_tile_mut(Position::new(7, 5)).unwrap().wall_type = WallType::Door;
    map.get_tile_mut(Position::new(7, 9)).unwrap().wall_type = WallType::Door;

    // Central corridor (8-14, 5-9)
    for y in 5..10 {
        for x in 8..15 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Bottom corridor and rooms (1-14, 11-14)
    for y in 11..15 {
        for x in 1..15 {
            map.get_tile_mut(Position::new(x, y)).unwrap().wall_type = WallType::None;
        }
    }

    // Exit door (west side)
    map.get_tile_mut(Position::new(0, 7)).unwrap().wall_type = WallType::Door;

    // Add encounters
    map.add_event(
        Position::new(3, 2),
        MapEvent::Encounter {
            monster_group: vec![1, 2],
        },
    );
    map.add_event(
        Position::new(2, 6),
        MapEvent::Encounter {
            monster_group: vec![2, 1],
        },
    );
    map.add_event(
        Position::new(5, 11),
        MapEvent::Encounter {
            monster_group: vec![1, 3],
        },
    );
    map.add_event(
        Position::new(14, 14),
        MapEvent::Encounter {
            monster_group: vec![3, 3, 3],
        },
    );

    // Add treasure chests
    map.add_event(
        Position::new(6, 2),
        MapEvent::Treasure {
            loot: vec![10, 20, 30],
        },
    );
    map.add_event(
        Position::new(13, 2),
        MapEvent::Treasure { loot: vec![11, 21] },
    );
    map.add_event(
        Position::new(10, 12),
        MapEvent::Treasure {
            loot: vec![12, 22, 31],
        },
    );

    // Add trap
    map.add_event(
        Position::new(10, 6),
        MapEvent::Trap {
            damage: 5,
            effect: None,
        },
    );

    // Add exit sign
    map.add_event(
        Position::new(0, 7),
        MapEvent::Sign {
            text: "Exit to town.".to_string(),
        },
    );

    let ron = ron::ser::to_string_pretty(&map, ron::ser::PrettyConfig::default())?;
    fs::write("data/maps/starter_dungeon.ron", ron)?;
    println!("✅ Created starter_dungeon.ron");
    Ok(())
}

fn generate_forest_area() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = Map::new(3, 20, 20);

    // Fill with forest borders
    for x in 0..20 {
        *map.get_tile_mut(Position::new(x, 0)).unwrap() =
            Tile::new(TerrainType::Forest, WallType::Normal);
        *map.get_tile_mut(Position::new(x, 19)).unwrap() =
            Tile::new(TerrainType::Forest, WallType::Normal);
    }
    for y in 0..20 {
        *map.get_tile_mut(Position::new(0, y)).unwrap() =
            Tile::new(TerrainType::Forest, WallType::Normal);
        *map.get_tile_mut(Position::new(19, y)).unwrap() =
            Tile::new(TerrainType::Forest, WallType::Normal);
    }

    // Fill interior with mixed terrain
    for y in 1..19 {
        for x in 1..19 {
            let terrain = if (6..=14).contains(&y) && (4..=16).contains(&x) {
                // Central lake
                TerrainType::Water
            } else if (x + y) % 3 == 0 {
                TerrainType::Forest
            } else {
                TerrainType::Grass
            };
            *map.get_tile_mut(Position::new(x, y)).unwrap() = Tile::new(terrain, WallType::None);
        }
    }

    // Exit door
    *map.get_tile_mut(Position::new(0, 10)).unwrap() =
        Tile::new(TerrainType::Grass, WallType::Door);

    // Add NPC
    map.add_npc(Npc::new(
        5,
        "Lost Ranger".to_string(),
        Position::new(2, 2),
        "These woods are more dangerous than they used to be. Watch your step, traveler."
            .to_string(),
    ));

    // Add encounters
    map.add_event(
        Position::new(5, 3),
        MapEvent::Encounter {
            monster_group: vec![4, 4],
        },
    );
    map.add_event(
        Position::new(14, 4),
        MapEvent::Encounter {
            monster_group: vec![5, 4],
        },
    );
    map.add_event(
        Position::new(3, 11),
        MapEvent::Encounter {
            monster_group: vec![6, 5],
        },
    );
    map.add_event(
        Position::new(17, 16),
        MapEvent::Encounter {
            monster_group: vec![6, 6],
        },
    );

    // Add treasure
    map.add_event(
        Position::new(8, 8),
        MapEvent::Treasure {
            loot: vec![13, 23, 32],
        },
    );
    map.add_event(
        Position::new(16, 2),
        MapEvent::Treasure { loot: vec![14, 24] },
    );
    map.add_event(
        Position::new(10, 13),
        MapEvent::Treasure {
            loot: vec![15, 25, 33, 40],
        },
    );

    // Add trap
    map.add_event(
        Position::new(7, 17),
        MapEvent::Trap {
            damage: 8,
            effect: None,
        },
    );

    // Add exit sign
    map.add_event(
        Position::new(0, 10),
        MapEvent::Sign {
            text: "Exit to town.".to_string(),
        },
    );

    let ron = ron::ser::to_string_pretty(&map, ron::ser::PrettyConfig::default())?;
    fs::write("data/maps/forest_area.ron", ron)?;
    println!("✅ Created forest_area.ron");
    Ok(())
}
