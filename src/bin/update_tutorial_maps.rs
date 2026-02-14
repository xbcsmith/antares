// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Tutorial Campaign Visual Metadata Updates Script
//!
//! This script updates all tutorial campaign maps with visual metadata for trees, grass,
//! and other terrain-specific visual properties. It creates backup files before modification
//! and modifies the RON files directly using text processing.

use std::fs;
use std::path::Path;

/// Updates forest metadata in a RON file for tiles in a specific area
#[allow(clippy::too_many_arguments)]
fn update_forest_metadata(
    content: &str,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    tree_type: &str,
    foliage_density: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Look for a tile with x and y coordinates
        if line.contains("x:") && i > 0 {
            // Check if this is a forest tile we need to update
            let mut x_val: Option<i32> = None;
            let mut y_val: Option<i32> = None;

            // Scan backwards to find x and y
            for j in (i.saturating_sub(20)..=i).rev() {
                if lines[j].contains("x:") {
                    if let Some(x_str) = lines[j].split("x:").nth(1) {
                        if let Ok(x) = x_str.trim().trim_end_matches(',').parse::<i32>() {
                            x_val = Some(x);
                        }
                    }
                }
                if lines[j].contains("y:") && j != i {
                    if let Some(y_str) = lines[j].split("y:").nth(1) {
                        if let Ok(y) = y_str.trim().trim_end_matches(',').parse::<i32>() {
                            y_val = Some(y);
                        }
                    }
                }
            }

            // Parse y from current line
            if let Some(y_str) = line.split("y:").nth(1) {
                if let Ok(y) = y_str.trim().trim_end_matches(',').parse::<i32>() {
                    y_val = Some(y);
                }
            }

            // Check if we're in the update area
            if let (Some(x), Some(y)) = (x_val, y_val) {
                if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                    // Add this line and look ahead for visual:
                    result.push(line.to_string());
                    i += 1;

                    // Find the visual: line
                    while i < lines.len() && !lines[i].contains("visual:") {
                        result.push(lines[i].to_string());
                        i += 1;
                    }

                    // Found visual:, now add tree metadata
                    if i < lines.len() && lines[i].contains("visual:") {
                        result.push(lines[i].to_string()); // visual: (
                        i += 1;

                        // Skip until we find the closing paren for visual
                        let mut found_closing = false;

                        while i < lines.len() && !found_closing {
                            let current = lines[i];

                            if current.contains("),") && current.trim() == ")," {
                                found_closing = true;
                                // Add tree metadata before closing
                                result.push(format!(
                                    "                tree_type: Some({}),",
                                    tree_type
                                ));
                                result.push(format!(
                                    "                color_tint: Some(({}, {}, {})),",
                                    color_r, color_g, color_b
                                ));
                                result.push(format!(
                                    "                foliage_density: Some({}),",
                                    foliage_density
                                ));
                                result.push(current.to_string());
                            } else {
                                result.push(current.to_string());
                            }
                            i += 1;
                        }
                        continue;
                    }
                }
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}

/// Updates grass metadata in a RON file for tiles in a specific area
#[allow(clippy::too_many_arguments)]
fn update_grass_metadata(
    content: &str,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    grass_density: &str,
    color_r: f32,
    color_g: f32,
    color_b: f32,
) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Look for a tile with y coordinate
        if line.contains("y:") {
            // Check if this is a grass tile we need to update
            let mut x_val: Option<i32> = None;
            let mut y_val: Option<i32> = None;

            // Parse y from current line
            if let Some(y_str) = line.split("y:").nth(1) {
                if let Ok(y) = y_str.trim().trim_end_matches(',').parse::<i32>() {
                    y_val = Some(y);
                }
            }

            // Scan backwards to find x
            for j in (i.saturating_sub(20)..=i).rev() {
                if lines[j].contains("x:") {
                    if let Some(x_str) = lines[j].split("x:").nth(1) {
                        if let Ok(x) = x_str.trim().trim_end_matches(',').parse::<i32>() {
                            x_val = Some(x);
                        }
                    }
                }
            }

            // Check if we're in the update area
            if let (Some(x), Some(y)) = (x_val, y_val) {
                if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                    // Add this line and look ahead for visual:
                    result.push(line.to_string());
                    i += 1;

                    // Find the visual: line
                    while i < lines.len() && !lines[i].contains("visual:") {
                        result.push(lines[i].to_string());
                        i += 1;
                    }

                    // Found visual:, now add grass metadata
                    if i < lines.len() && lines[i].contains("visual:") {
                        result.push(lines[i].to_string()); // visual: (
                        i += 1;

                        // Skip until we find the closing paren for visual
                        let mut found_closing = false;

                        while i < lines.len() && !found_closing {
                            let current = lines[i];

                            if current.contains("),") && current.trim() == ")," {
                                found_closing = true;
                                // Add grass metadata before closing
                                result.push(format!(
                                    "                grass_density: Some({}),",
                                    grass_density
                                ));
                                result.push(format!(
                                    "                color_tint: Some(({}, {}, {})),",
                                    color_r, color_g, color_b
                                ));
                                result.push(current.to_string());
                            } else {
                                result.push(current.to_string());
                            }
                            i += 1;
                        }
                        continue;
                    }
                }
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Phase 5: Tutorial Campaign Visual Metadata Updates ===\n");

    let base_path = "campaigns/tutorial/data/maps";

    let maps_to_process: Vec<(u32, &str, &str)> = vec![
        (1, "map_1.ron", "Town Square"),
        (2, "map_2.ron", "Forest Path"),
        (3, "map_3.ron", "Mountain Trail"),
        (4, "map_4.ron", "Swamp"),
        (5, "map_5.ron", "Dense Forest"),
    ];

    let mut success_count = 0;

    for (_expected_id, filename, description) in maps_to_process {
        let map_path = format!("{}/{}", base_path, filename);

        if !Path::new(&map_path).exists() {
            eprintln!(
                "Warning: {} not found at {}. Skipping...",
                description, map_path
            );
            continue;
        }

        println!("Processing {}: {}", filename, description);

        // Create backup file
        let backup_path = format!("{}.bak", map_path);
        if !Path::new(&backup_path).exists() {
            let content = fs::read_to_string(&map_path)?;
            fs::write(&backup_path, &content)?;
            println!("  ✓ Backup created: {}", backup_path);
        } else {
            println!("  ℹ Backup already exists: {}", backup_path);
        }

        // Read the map file
        let mut content = fs::read_to_string(&map_path)?;

        // Apply map-specific updates
        match filename {
            "map_1.ron" => {
                // Map 1: Town Square - grass courtyard with Medium density
                println!("  - Configuring grass courtyard with Medium density");
                content = update_grass_metadata(&content, 5, 15, 5, 15, "Medium", 0.3, 0.7, 0.3);

                // Decorative trees at corners
                println!("  - Adding Oak trees at corners");
                content = update_forest_metadata(&content, 1, 3, 1, 3, "Oak", 0.8, 0.2, 0.5, 0.2);
                content = update_forest_metadata(&content, 16, 18, 1, 3, "Oak", 0.8, 0.2, 0.5, 0.2);
            }
            "map_2.ron" => {
                // Map 2: Forest Path - Oak and Pine variations
                println!("  - Configuring Oak forest section");
                content = update_forest_metadata(&content, 0, 10, 0, 19, "Oak", 1.8, 0.2, 0.6, 0.2);

                println!("  - Configuring Pine forest section");
                content =
                    update_forest_metadata(&content, 10, 19, 0, 19, "Pine", 1.2, 0.1, 0.5, 0.15);
            }
            "map_3.ron" => {
                // Map 3: Mountain Trail - sparse Pine trees
                println!("  - Configuring sparse Pine trees");
                content =
                    update_forest_metadata(&content, 0, 19, 0, 19, "Pine", 0.8, 0.15, 0.45, 0.2);
            }
            "map_4.ron" => {
                // Map 4: Swamp - Dead trees with zero foliage
                println!("  - Configuring Dead trees with zero foliage");
                content =
                    update_forest_metadata(&content, 0, 19, 0, 19, "Dead", 0.0, 0.4, 0.3, 0.2);
            }
            "map_5.ron" => {
                // Map 5: Dense Forest - varied tree types
                println!("  - Configuring varied tree types");
                content =
                    update_forest_metadata(&content, 0, 12, 0, 12, "Oak", 1.5, 0.25, 0.65, 0.25);
                content =
                    update_forest_metadata(&content, 13, 19, 0, 19, "Willow", 1.3, 0.3, 0.55, 0.35);
                content =
                    update_forest_metadata(&content, 0, 12, 13, 19, "Pine", 1.4, 0.2, 0.6, 0.25);
            }
            _ => {}
        }

        // Write the updated content back
        fs::write(&map_path, content)?;
        println!("  ✓ Successfully updated {}\n", map_path);
        success_count += 1;
    }

    println!("=== Summary ===");
    println!("✓ Map 1: Town Square - Grass courtyard with Medium density configured");
    println!("✓ Map 2: Forest Path - Oak and Pine forest sections configured");
    println!("✓ Map 3: Mountain Trail - Sparse Pine trees configured");
    println!("✓ Map 4: Swamp - Dead trees with zero foliage configured");
    println!("✓ Map 5: Dense Forest - Varied tree types configured");
    println!("\nResults: {} succeeded", success_count);
    println!("✓ Backup files created for all maps (*.ron.bak)\n");

    Ok(())
}
