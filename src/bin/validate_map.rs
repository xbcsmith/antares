// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map Validation Utility
//!
//! This tool validates RON map files for Antares RPG before deployment.
//! It checks structural integrity, content validity, and gameplay constraints.
//!
//! Usage:
//!   cargo run --bin validate_map data/maps/starter_town.ron
//!   cargo run --bin validate_map data/maps/*.ron

use antares::domain::types::Position;
use antares::domain::world::{Map, MapEvent};
use std::env;
use std::fs;
use std::process;

// Known valid Monster IDs from data files
// TODO: Load dynamically from data/monsters.ron when database loader is implemented
const VALID_MONSTER_IDS: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Known valid Item IDs from data files
// TODO: Load dynamically from data/items.ron when database loader is implemented
const VALID_ITEM_IDS: &[u8] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
];

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <map_file.ron> [map_file2.ron ...]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} data/maps/starter_town.ron", args[0]);
        eprintln!("  {} data/maps/*.ron", args[0]);
        process::exit(1);
    }

    let mut total_files = 0;
    let mut valid_files = 0;
    let mut failed_files = 0;

    for file_path in &args[1..] {
        total_files += 1;
        println!("\n{:=<70}", "");
        println!("Validating: {}", file_path);
        println!("{:-<70}", "");

        match validate_map_file(file_path) {
            Ok(map) => {
                valid_files += 1;
                println!("✅ VALID");
                print_map_summary(&map);
            }
            Err(errors) => {
                failed_files += 1;
                println!("❌ INVALID ({} errors)", errors.len());
                println!("\nErrors:");
                for error in errors {
                    println!("  • {}", error);
                }
            }
        }
    }

    println!("\n{:=<70}", "");
    println!("Validation Summary:");
    println!("  Total files: {}", total_files);
    println!("  Valid: {}", valid_files);
    println!("  Failed: {}", failed_files);
    println!("{:=<70}", "");

    if failed_files > 0 {
        process::exit(1);
    }
}

/// Validate a map file and return the parsed map or validation errors
fn validate_map_file(file_path: &str) -> Result<Map, Vec<String>> {
    let mut errors = Vec::new();

    // Read file
    let contents = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("Failed to read file: {}", e));
            return Err(errors);
        }
    };

    // Parse RON
    let blueprint: antares::domain::world::MapBlueprint = match ron::from_str(&contents) {
        Ok(bp) => bp,
        Err(e) => {
            errors.push(format!("Failed to parse RON: {}", e));
            return Err(errors);
        }
    };

    let map: Map = blueprint.into();

    // Validate structure
    validate_structure(&map, &mut errors);

    // Validate content
    validate_content(&map, &mut errors);

    // Validate gameplay constraints
    validate_gameplay(&map, &mut errors);

    if errors.is_empty() {
        Ok(map)
    } else {
        Err(errors)
    }
}

/// Validate map structure (dimensions, tile array integrity)
fn validate_structure(map: &Map, errors: &mut Vec<String>) {
    // Check map ID
    if map.id == 0 {
        errors.push("Map ID must be non-zero (valid range: 1-65535)".to_string());
    }

    // Check dimensions
    if map.width == 0 || map.height == 0 {
        errors.push(format!(
            "Map dimensions must be > 0 (got {}x{})",
            map.width, map.height
        ));
    }

    if map.width > 256 || map.height > 256 {
        errors.push(format!(
            "Map dimensions exceed maximum (got {}x{}, max 256x256)",
            map.width, map.height
        ));
    }

    // Validate tiles
    let expected_tiles = (map.width * map.height) as usize;
    if map.tiles.len() != expected_tiles {
        errors.push(format!(
            "Tile count mismatch: expected {}, found {}",
            expected_tiles,
            map.tiles.len()
        ));
    }
}

/// Validate map content (events, NPCs, within bounds and valid IDs)
fn validate_content(map: &Map, errors: &mut Vec<String>) {
    // Validate event positions and IDs
    for (pos, event) in &map.events {
        // Check position bounds
        if !is_position_valid(pos, map.width, map.height) {
            errors.push(format!(
                "Event at ({}, {}) out of bounds (map: {}x{})",
                pos.x, pos.y, map.width, map.height
            ));
        }

        // Validate Monster IDs in encounters
        if let MapEvent::Encounter { monster_group } = event {
            for &monster_id in monster_group {
                if !VALID_MONSTER_IDS.contains(&monster_id) {
                    errors.push(format!(
                        "Invalid Monster ID {} at ({}, {}). Valid IDs: {:?}",
                        monster_id, pos.x, pos.y, VALID_MONSTER_IDS
                    ));
                }
            }
        }

        // Validate Item IDs in treasure
        if let MapEvent::Treasure { loot } = event {
            for &item_id in loot {
                if !VALID_ITEM_IDS.contains(&item_id) {
                    errors.push(format!(
                        "Invalid Item ID {} at ({}, {}). Check data/items.ron",
                        item_id, pos.x, pos.y
                    ));
                }
            }
        }
    }

    // Validate NPC positions
    for npc in &map.npcs {
        if !is_position_valid(&npc.position, map.width, map.height) {
            errors.push(format!(
                "NPC '{}' at ({}, {}) out of bounds (map: {}x{})",
                npc.name, npc.position.x, npc.position.y, map.width, map.height
            ));
        }
    }
}

/// Validate gameplay constraints (recommended practices)
fn validate_gameplay(map: &Map, errors: &mut Vec<String>) {
    // Check for overlapping events and NPCs
    let mut occupied_positions = std::collections::HashSet::new();

    for pos in map.events.keys() {
        if !occupied_positions.insert((pos.x, pos.y)) {
            errors.push(format!(
                "Multiple events at same position ({}, {})",
                pos.x, pos.y
            ));
        }
    }

    for npc in &map.npcs {
        let pos_tuple = (npc.position.x, npc.position.y);
        if occupied_positions.contains(&pos_tuple) {
            errors.push(format!(
                "NPC '{}' overlaps with event at ({}, {})",
                npc.name, npc.position.x, npc.position.y
            ));
        }
    }

    // Warn about maps with no content
    if map.events.is_empty() && map.npcs.is_empty() {
        errors.push("Map has no events or NPCs (empty map)".to_string());
    }
}

/// Check if position is within map bounds
fn is_position_valid(pos: &Position, width: u32, height: u32) -> bool {
    pos.x >= 0 && pos.x < width as i32 && pos.y >= 0 && pos.y < height as i32
}

/// Print detailed map summary
fn print_map_summary(map: &Map) {
    println!("\nMap Summary:");
    println!("  ID: {}", map.id);
    println!("  Size: {}x{}", map.width, map.height);
    println!("  Total tiles: {}", map.width * map.height);
    println!("  Events: {}", map.events.len());
    println!("  NPCs: {}", map.npcs.len());

    // Count event types
    let mut encounters = 0;
    let mut treasures = 0;
    let mut teleports = 0;
    let mut traps = 0;
    let mut signs = 0;
    let mut dialogues = 0;

    for event in map.events.values() {
        match event {
            MapEvent::Encounter { .. } => encounters += 1,
            MapEvent::Treasure { .. } => treasures += 1,
            MapEvent::Teleport { .. } => teleports += 1,
            MapEvent::Trap { .. } => traps += 1,
            MapEvent::Sign { .. } => signs += 1,
            MapEvent::NpcDialogue { .. } => dialogues += 1,
        }
    }

    if !map.events.is_empty() {
        println!("\n  Event Breakdown:");
        if encounters > 0 {
            println!("    - Encounters: {}", encounters);
        }
        if treasures > 0 {
            println!("    - Treasures: {}", treasures);
        }
        if teleports > 0 {
            println!("    - Teleports: {}", teleports);
        }
        if traps > 0 {
            println!("    - Traps: {}", traps);
        }
        if signs > 0 {
            println!("    - Signs: {}", signs);
        }
        if dialogues > 0 {
            println!("    - NPC Dialogues: {}", dialogues);
        }
    }

    if !map.npcs.is_empty() {
        println!("\n  NPCs:");
        for npc in &map.npcs {
            println!(
                "    - {} at ({}, {})",
                npc.name, npc.position.x, npc.position.y
            );
        }
    }
}
