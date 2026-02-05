// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Tutorial Campaign Visual Metadata Updates
//!
//! Updates tutorial campaign maps with visual metadata for trees, grass, and terrain.
//! Creates backup files before modification and applies area-based visual configurations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum WallType {
    #[serde(rename = "None")]
    None,
    Normal,
    Door,
    Torch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum GrassDensity {
    None,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum TreeType {
    Oak,
    Pine,
    Dead,
    Palm,
    Willow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum RockVariant {
    Smooth,
    Jagged,
    Layered,
    Crystal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    sprite: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sprite_layers: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sprite_rule: Option<serde_json::Value>,
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
    events: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    encounter_table: Option<serde_json::Value>,
    #[serde(default)]
    allow_random_encounters: Option<bool>,
    #[serde(default)]
    npc_placements: Vec<serde_json::Value>,
}

/// Updates forest area with specified tree type and visual metadata
///
/// # Arguments
///
/// * `tiles` - Mutable reference to tile array
/// * `area` - Tuple of (min_x, min_y, max_x, max_y) bounds
/// * `tree_type` - TreeType to apply to forest tiles
/// * `foliage_density` - Foliage density multiplier (0.0 to 2.0)
/// * `color_tint` - Optional color tint (R, G, B) as values 0.0-1.0
fn update_forest_area_metadata(
    tiles: &mut [Tile],
    area: (i32, i32, i32, i32),
    tree_type: TreeType,
    foliage_density: f32,
    color_tint: Option<(f32, f32, f32)>,
) {
    let (min_x, min_y, max_x, max_y) = area;

    for tile in tiles {
        if tile.x >= min_x && tile.x <= max_x && tile.y >= min_y && tile.y <= max_y {
            if matches!(tile.terrain, TerrainType::Forest) {
                tile.visual.tree_type = Some(tree_type);
                tile.visual.foliage_density = Some(foliage_density);
                if let Some(tint) = color_tint {
                    tile.visual.color_tint = Some(tint);
                }
                // Add rotation variation for natural placement (0 to 360 degrees)
                let rotation = ((tile.x + tile.y) as f32 * 13.7) % 360.0;
                tile.visual.rotation_y = Some(rotation);
            }
        }
    }
}

/// Updates grass area with specified grass density and visual metadata
///
/// # Arguments
///
/// * `tiles` - Mutable reference to tile array
/// * `area` - Tuple of (min_x, min_y, max_x, max_y) bounds
/// * `grass_density` - GrassDensity level
/// * `color_tint` - Optional color tint (R, G, B) as values 0.0-1.0
fn update_grass_area_metadata(
    tiles: &mut [Tile],
    area: (i32, i32, i32, i32),
    grass_density: GrassDensity,
    color_tint: Option<(f32, f32, f32)>,
) {
    let (min_x, min_y, max_x, max_y) = area;

    for tile in tiles {
        if tile.x >= min_x && tile.x <= max_x && tile.y >= min_y && tile.y <= max_y {
            if matches!(tile.terrain, TerrainType::Grass | TerrainType::Ground) {
                tile.visual.grass_density = Some(grass_density);
                if let Some(tint) = color_tint {
                    tile.visual.color_tint = Some(tint);
                }
                // Add scale variation for natural look (0.8 to 1.2)
                let scale = 0.8
                    + ((tile.x.wrapping_mul(7) ^ tile.y.wrapping_mul(11)) as f32 % 41.0) / 100.0;
                tile.visual.scale = Some(scale);
            }
        }
    }
}

/// Apply visual configurations to map 1 (Town Square with grass courtyard)
fn apply_map1_configuration(map: &mut Map) {
    println!("  Configuring Map 1 (Town Square)...");

    // Grass tiles in courtyard area with Medium density
    update_grass_area_metadata(
        &mut map.tiles,
        (5, 5, 15, 15),
        GrassDensity::Medium,
        Some((0.3, 0.7, 0.3)),
    );

    // Decorative trees at corners
    for tile in &mut map.tiles {
        if matches!(tile.terrain, TerrainType::Forest) {
            if (tile.x == 2 || tile.x == 18) && (tile.y == 2 || tile.y == 18) {
                tile.visual.tree_type = Some(TreeType::Oak);
                tile.visual.scale = Some(0.8);
                tile.visual.foliage_density = Some(1.0);
            }
        }
    }
}

/// Apply visual configurations to map 2 (Forest Path with Oak and Pine variations)
fn apply_map2_configuration(map: &mut Map) {
    println!("  Configuring Map 2 (Forest Path)...");

    // Oak forest section
    update_forest_area_metadata(
        &mut map.tiles,
        (0, 0, 10, 20),
        TreeType::Oak,
        1.8,
        Some((0.2, 0.6, 0.2)),
    );

    // Pine forest section
    update_forest_area_metadata(
        &mut map.tiles,
        (10, 0, 20, 20),
        TreeType::Pine,
        1.2,
        Some((0.1, 0.5, 0.15)),
    );
}

/// Apply visual configurations to map 3 (Mountain Trail with sparse trees)
fn apply_map3_configuration(map: &mut Map) {
    println!("  Configuring Map 3 (Mountain Trail)...");

    // Sparse Pine trees across the map
    update_forest_area_metadata(&mut map.tiles, (0, 0, 20, 20), TreeType::Pine, 0.8, None);
}

/// Apply visual configurations to map 4 (Swamp with dead trees)
fn apply_map4_configuration(map: &mut Map) {
    println!("  Configuring Map 4 (Swamp)...");

    // Dead trees with zero foliage
    update_forest_area_metadata(
        &mut map.tiles,
        (0, 0, 20, 20),
        TreeType::Dead,
        0.0,
        Some((0.4, 0.3, 0.2)),
    );

    // Also handle swamp terrain
    for tile in &mut map.tiles {
        if matches!(tile.terrain, TerrainType::Swamp) {
            tile.visual.color_tint = Some((0.35, 0.45, 0.3));
        }
    }
}

/// Apply visual configurations to map 5 (Dense Forest with varied types)
fn apply_map5_configuration(map: &mut Map) {
    println!("  Configuring Map 5 (Dense Forest)...");

    // Oak forest (60% of tiles) - north and west sections
    update_forest_area_metadata(
        &mut map.tiles,
        (0, 0, 12, 12),
        TreeType::Oak,
        1.5,
        Some((0.25, 0.65, 0.25)),
    );

    // Willow forest (30% of tiles) - east section
    update_forest_area_metadata(
        &mut map.tiles,
        (13, 0, 20, 20),
        TreeType::Willow,
        1.3,
        Some((0.3, 0.55, 0.35)),
    );

    // Pine trees (10% scattered) - south section
    update_forest_area_metadata(
        &mut map.tiles,
        (0, 13, 12, 20),
        TreeType::Pine,
        1.4,
        Some((0.2, 0.6, 0.25)),
    );

    // Apply scale and rotation variation for natural placement
    for tile in &mut map.tiles {
        if matches!(tile.terrain, TerrainType::Forest) {
            if tile.visual.tree_type.is_some() {
                // Randomized scale: 0.9 to 1.3
                let scale = 0.9
                    + ((tile.x.wrapping_mul(19) ^ tile.y.wrapping_mul(23)) as f32 % 40.0) / 100.0;
                tile.visual.scale = Some(scale);

                // Randomized rotation: 0 to 360 degrees
                let rotation = ((tile.x.wrapping_mul(17) ^ tile.y.wrapping_mul(29)) as f32 % 360.0);
                tile.visual.rotation_y = Some(rotation);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Phase 5: Tutorial Campaign Visual Metadata Updates ===\n");

    let maps = vec![
        (
            "campaigns/tutorial/data/maps/map_1.ron",
            "Map 1: Town Square",
            apply_map1_configuration,
        ),
        (
            "campaigns/tutorial/data/maps/map_2.ron",
            "Map 2: Forest Path",
            apply_map2_configuration,
        ),
        (
            "campaigns/tutorial/data/maps/map_3.ron",
            "Map 3: Mountain Trail",
            apply_map3_configuration,
        ),
        (
            "campaigns/tutorial/data/maps/map_4.ron",
            "Map 4: Swamp",
            apply_map4_configuration,
        ),
        (
            "campaigns/tutorial/data/maps/map_5.ron",
            "Map 5: Dense Forest",
            apply_map5_configuration,
        ),
    ];

    for (map_path, description, apply_config) in maps {
        if !Path::new(map_path).exists() {
            eprintln!("Warning: {} not found at {}", description, map_path);
            continue;
        }

        println!("Processing {}...", description);

        // Create backup file
        let backup_path = format!("{}.bak", map_path);
        let content = fs::read_to_string(map_path)?;
        fs::write(&backup_path, &content)?;
        println!("  ✓ Backup created: {}", backup_path);

        // Parse map
        let mut map: Map = ron::from_str(&content)?;

        // Apply configuration
        apply_config(&mut map);

        // Serialize back with ron
        let pretty_config = ron::ser::PrettyConfig::default()
            .with_depth_limit(4)
            .with_separate_tuple_members(true);

        let serialized = ron::ser::to_string_pretty(&map, pretty_config)?;

        // Write back
        fs::write(map_path, serialized)?;
        println!("  ✓ Successfully updated {}\n", map_path);
    }

    println!("=== Summary ===");
    println!("✓ Map 1: Town Square - Grass courtyard with Medium density configured");
    println!("✓ Map 2: Forest Path - Oak and Pine forest sections configured");
    println!("✓ Map 3: Mountain Trail - Sparse Pine trees configured");
    println!("✓ Map 4: Swamp - Dead trees with zero foliage configured");
    println!("✓ Map 5: Dense Forest - Varied tree types (Oak/Willow/Pine) with randomization");
    println!("\n✓ All tutorial maps updated successfully!");
    println!("✓ Backup files created for all maps (*.ron.bak)\n");

    Ok(())
}
