// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Binary to update tutorial maps with terrain-specific visual metadata
//!
//! This program updates the tutorial maps to add terrain features:
//! - starter_town.ron: Wall height variations and structure metadata
//! - forest_area.ron: Tree types and grass density variations
//! - starter_dungeon.ron: Rock variants and water flow directions
//!
//! Usage: cargo run --bin update_tutorial_maps

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use ron::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum TerrainType {
    Ground,
    Grass,
    Water,
    Lava,
    Swamp,
    Stone,
    Dirt,
    Forest,
    Mountain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum WallType {
    None,
    Normal,
    Door,
    Torch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum GrassDensity {
    None,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum TreeType {
    Oak,
    Pine,
    Dead,
    Palm,
    Willow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum RockVariant {
    Smooth,
    Jagged,
    Layered,
    Crystal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum WaterFlowDirection {
    Still,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct TileVisualMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width_z: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_tint: Option<(f32, f32, f32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y_offset: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grass_density: Option<GrassDensity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tree_type: Option<TreeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rock_variant: Option<RockVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    water_flow_direction: Option<WaterFlowDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foliage_density: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snow_coverage: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tile {
    terrain: TerrainType,
    wall_type: WallType,
    blocked: bool,
    is_special: bool,
    is_dark: bool,
    visited: bool,
    x: i32,
    y: i32,
    event_trigger: Option<String>,
    #[serde(default)]
    visual: TileVisualMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct Map {
    id: u32,
    width: u32,
    height: u32,
    name: String,
    description: String,
    tiles: Vec<Tile>,
    #[serde(default)]
    events: HashMap<String, Value>,
    #[serde(default)]
    npc_placements: Vec<Value>,
}

fn update_starter_town_metadata(tiles: &mut [Tile]) {
    // Add wall height variations
    // Town outer walls - tall fortifications
    for tile in tiles {
        if (tile.x == 0 || tile.x == 19 || tile.y == 0 || tile.y == 14)
            && tile.wall_type != WallType::None
        {
            tile.visual.height = Some(3.5);
            tile.visual.color_tint = Some((0.7, 0.7, 0.7));
        }
        // Interior divider walls
        if (tile.x == 10 || tile.x == 11)
            && (5..=6).contains(&tile.y)
            && tile.wall_type != WallType::None
        {
            tile.visual.height = Some(1.5);
        }
        // Decorative pillars at entrance
        if (tile.x == 9 || tile.x == 10) && tile.y == 0 && tile.wall_type != WallType::None {
            tile.visual.height = Some(4.0);
            tile.visual.scale = Some(0.3);
            tile.visual.color_tint = Some((0.8, 0.8, 0.8));
        }
    }
}

fn update_forest_area_metadata(tiles: &mut [Tile]) {
    // Add tree types and grass density variations
    for tile in tiles {
        // Dense oak forest (north section)
        if (5..=7).contains(&tile.x)
            && (2..=4).contains(&tile.y)
            && matches!(tile.terrain, TerrainType::Forest)
        {
            tile.visual.tree_type = Some(TreeType::Oak);
            tile.visual.foliage_density = Some(1.8);
            tile.visual.color_tint = Some((0.2, 0.6, 0.2));
        }
        // Pine grove (east section)
        if (15..=17).contains(&tile.x)
            && (8..=10).contains(&tile.y)
            && matches!(tile.terrain, TerrainType::Forest)
        {
            tile.visual.tree_type = Some(TreeType::Pine);
            tile.visual.foliage_density = Some(1.2);
            tile.visual.color_tint = Some((0.1, 0.5, 0.15));
        }
        // Dead trees near dungeon entrance
        if (10..=12).contains(&tile.x)
            && (18..=19).contains(&tile.y)
            && matches!(tile.terrain, TerrainType::Forest)
        {
            tile.visual.tree_type = Some(TreeType::Dead);
            tile.visual.color_tint = Some((0.4, 0.3, 0.2));
        }
        // Grassland with varying density
        if tile.y == 10
            && matches!(
                tile.terrain,
                TerrainType::Ground | TerrainType::Grass | TerrainType::Forest
            )
        {
            match tile.x {
                2 => tile.visual.grass_density = Some(GrassDensity::Low),
                3 => tile.visual.grass_density = Some(GrassDensity::Medium),
                4 => tile.visual.grass_density = Some(GrassDensity::High),
                5 => tile.visual.grass_density = Some(GrassDensity::VeryHigh),
                _ => {}
            }
        }
    }
}

fn update_starter_dungeon_metadata(tiles: &mut [Tile]) {
    // Add rock variants and water flow
    for tile in tiles {
        // Jagged cave walls (stone with jagged texture)
        if (1..=3).contains(&tile.x)
            && (1..=3).contains(&tile.y)
            && (matches!(tile.terrain, TerrainType::Mountain | TerrainType::Stone)
                || tile.wall_type != WallType::None)
        {
            tile.visual.rock_variant = Some(RockVariant::Jagged);
            tile.visual.color_tint = Some((0.5, 0.45, 0.4));
        }
        // Crystal formations in treasure room
        if (15..=16).contains(&tile.x)
            && (15..=16).contains(&tile.y)
            && matches!(tile.terrain, TerrainType::Mountain | TerrainType::Stone)
        {
            tile.visual.rock_variant = Some(RockVariant::Crystal);
            tile.visual.color_tint = Some((0.5, 0.5, 1.0));
        }
        // Layered sedimentary rocks
        if (8..=9).contains(&tile.x)
            && (8..=9).contains(&tile.y)
            && matches!(tile.terrain, TerrainType::Mountain | TerrainType::Stone)
        {
            tile.visual.rock_variant = Some(RockVariant::Layered);
            tile.visual.color_tint = Some((0.6, 0.55, 0.5));
        }
        // Underground river with flow direction
        if tile.y == 5 && matches!(tile.terrain, TerrainType::Water) {
            match tile.x {
                10..=12 => tile.visual.water_flow_direction = Some(WaterFlowDirection::East),
                13..=14 => tile.visual.water_flow_direction = Some(WaterFlowDirection::South),
                _ => tile.visual.water_flow_direction = Some(WaterFlowDirection::Still),
            }
            tile.visual.color_tint = Some((0.3, 0.4, 0.6));
        }
        // Dungeon water pools
        if (tile.x >= 6 && tile.x <= 8)
            && (tile.y >= 12 && tile.y <= 14)
            && matches!(tile.terrain, TerrainType::Water)
        {
            tile.visual.water_flow_direction = Some(WaterFlowDirection::Still);
            tile.visual.color_tint = Some((0.2, 0.3, 0.5));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Updating tutorial maps with terrain features...\n");

    let maps = vec![
        ("data/maps/starter_town.ron", "Map 1: Town Square", 1_u32),
        ("data/maps/forest_area.ron", "Map 2: Forest Entrance", 2_u32),
        (
            "data/maps/starter_dungeon.ron",
            "Map 3: Dungeon Level 1",
            3_u32,
        ),
    ];

    let mut success_count = 0;
    let mut failed_count = 0;

    for (map_path, description, expected_id) in maps {
        if !Path::new(map_path).exists() {
            eprintln!("✗ {} not found at {}", description, map_path);
            failed_count += 1;
            continue;
        }

        println!("Processing {}...", description);

        match fs::read_to_string(map_path) {
            Ok(content) => {
                match ron::from_str::<Map>(&content) {
                    Ok(mut map) => {
                        if map.id != expected_id {
                            eprintln!(
                                "  Warning: Expected map ID {}, found {}",
                                expected_id, map.id
                            );
                        }

                        // Apply terrain-specific updates
                        match map.id {
                            1 => update_starter_town_metadata(&mut map.tiles),
                            2 => update_forest_area_metadata(&mut map.tiles),
                            3 => update_starter_dungeon_metadata(&mut map.tiles),
                            _ => eprintln!("  Skipping unknown map ID: {}", map.id),
                        }

                        // Serialize back with ron
                        let pretty_config = ron::ser::PrettyConfig::default()
                            .depth_limit(4)
                            .separate_tuple_members(true)
                            .enumerate_arrays(true);

                        match ron::ser::to_string_pretty(&map, pretty_config) {
                            Ok(serialized) => match fs::write(map_path, serialized) {
                                Ok(_) => {
                                    println!("  ✓ Successfully updated {}", map_path);
                                    success_count += 1;
                                }
                                Err(e) => {
                                    eprintln!("  ✗ Failed to write {}: {}", map_path, e);
                                    failed_count += 1;
                                }
                            },
                            Err(e) => {
                                eprintln!("  ✗ Failed to serialize {}: {}", map_path, e);
                                failed_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  ✗ Failed to parse {}: {}", map_path, e);
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("  ✗ Failed to read {}: {}", map_path, e);
                failed_count += 1;
            }
        }
    }

    println!("\n{}", "=".repeat(50));
    println!(
        "Results: {} succeeded, {} failed",
        success_count, failed_count
    );
    println!("{}", "=".repeat(50));

    if failed_count == 0 {
        println!("\n✓ All tutorial maps updated successfully!");
        Ok(())
    } else {
        Err("Some maps failed to update".into())
    }
}
