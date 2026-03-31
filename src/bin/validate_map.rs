// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map Validation Utility
//!
//! This tool validates RON map files for Antares RPG before deployment.
//! It checks structural integrity, content validity, and gameplay constraints.
//!
//! Usage:
//!   cargo run --bin validate_map data/maps/starter_town.ron
//!   cargo run --bin validate_map data/maps/*.ron

use antares::domain::items::ItemDatabase;
use antares::domain::types::Position;
use antares::domain::world::{Map, MapEvent};
use antares::sdk::database::MonsterDatabase;
use std::env;
use std::fs;
use std::process;

/// Loads valid monster IDs from the RON data file.
///
/// Falls back to a hardcoded default set if the file cannot be loaded.
fn load_monster_ids() -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("data/test_campaign/data/monsters.ron");

    match MonsterDatabase::load_from_file(&path) {
        Ok(db) => {
            let ids: Vec<u8> = db.all_monsters();
            if ids.is_empty() {
                eprintln!(
                    "Warning: {} loaded but contains no monsters, using defaults",
                    path.display()
                );
                default_monster_ids()
            } else {
                println!("Loaded {} monster IDs from {}", ids.len(), path.display());
                ids
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not load {}: {}. Using default IDs.",
                path.display(),
                e
            );
            default_monster_ids()
        }
    }
}

/// Loads valid item IDs from the RON data file.
///
/// Falls back to a hardcoded default set if the file cannot be loaded.
fn load_item_ids() -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("data/test_campaign/data/items.ron");

    match ItemDatabase::load_from_file(&path) {
        Ok(db) => {
            let ids: Vec<u8> = db.all_items().iter().map(|item| item.id).collect();
            if ids.is_empty() {
                eprintln!(
                    "Warning: {} loaded but contains no items, using defaults",
                    path.display()
                );
                default_item_ids()
            } else {
                println!("Loaded {} item IDs from {}", ids.len(), path.display());
                ids
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not load {}: {}. Using default IDs.",
                path.display(),
                e
            );
            default_item_ids()
        }
    }
}

/// Default monster IDs (fallback when data files unavailable)
fn default_monster_ids() -> Vec<u8> {
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
}

/// Default item IDs (fallback when data files unavailable)
fn default_item_ids() -> Vec<u8> {
    (1..=40).collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <map_file.ron> [map_file2.ron ...]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} data/maps/starter_town.ron", args[0]);
        eprintln!("  {} data/maps/*.ron", args[0]);
        process::exit(1);
    }

    // Load IDs dynamically from data files (with hardcoded fallback)
    let valid_monster_ids = load_monster_ids();
    let valid_item_ids = load_item_ids();

    let mut total_files = 0;
    let mut valid_files = 0;
    let mut failed_files = 0;

    for file_path in &args[1..] {
        total_files += 1;
        println!("\n{:=<70}", "");
        println!("Validating: {}", file_path);
        println!("{:-<70}", "");

        match validate_map_file(file_path, &valid_monster_ids, &valid_item_ids) {
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
fn validate_map_file(
    file_path: &str,
    valid_monster_ids: &[u8],
    valid_item_ids: &[u8],
) -> Result<Map, Vec<String>> {
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
    validate_content(&map, &mut errors, valid_monster_ids, valid_item_ids);

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
fn validate_content(
    map: &Map,
    errors: &mut Vec<String>,
    valid_monster_ids: &[u8],
    valid_item_ids: &[u8],
) {
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
        if let MapEvent::Encounter { monster_group, .. } = event {
            for &monster_id in monster_group {
                if !valid_monster_ids.contains(&monster_id) {
                    errors.push(format!(
                        "Invalid Monster ID {} at ({}, {}). Valid IDs: {:?}",
                        monster_id, pos.x, pos.y, valid_monster_ids
                    ));
                }
            }
        }

        // Validate Item IDs in treasure
        if let MapEvent::Treasure { loot, .. } = event {
            for &item_id in loot {
                if !valid_item_ids.contains(&item_id) {
                    errors.push(format!(
                        "Invalid Item ID {} at ({}, {}). Check data/items.ron",
                        item_id, pos.x, pos.y
                    ));
                }
            }
        }
    }

    // Validate NPC placements
    for placement in &map.npc_placements {
        if !is_position_valid(&placement.position, map.width, map.height) {
            errors.push(format!(
                "NPC placement '{}' at ({}, {}) out of bounds (map: {}x{})",
                placement.npc_id, placement.position.x, placement.position.y, map.width, map.height
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

    for placement in &map.npc_placements {
        let pos_tuple = (placement.position.x, placement.position.y);
        if occupied_positions.contains(&pos_tuple) {
            errors.push(format!(
                "NPC placement '{}' overlaps with event at ({}, {})",
                placement.npc_id, placement.position.x, placement.position.y
            ));
        }
    }

    // Warn about maps with no content
    if map.events.is_empty() && map.npc_placements.is_empty() {
        errors.push("Map has no events or NPC placements (empty map)".to_string());
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
    println!("  NPC Placements: {}", map.npc_placements.len());

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
            MapEvent::RecruitableCharacter { .. } => {
                // Count recruitable characters (could add separate counter if needed)
                dialogues += 1
            }
            MapEvent::EnterInn { .. } => {
                // Count inn entrances (could add separate counter if needed)
                signs += 1
            }
            MapEvent::Furniture { .. } => {
                // Count furniture events (could add separate counter if needed)
                signs += 1
            }
            MapEvent::Container { .. } => {
                // Count container events (could add separate counter if needed)
                signs += 1
            }
            MapEvent::DroppedItem { .. } => {
                // Count dropped item events (static map-authored drops)
                signs += 1
            }
            MapEvent::LockedDoor { .. } => {
                // Count locked door events
                signs += 1
            }
            MapEvent::LockedContainer { .. } => {
                // Count locked container events
                signs += 1
            }
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

    if !map.npc_placements.is_empty() {
        println!("\n  NPC Placements:");
        for placement in &map.npc_placements {
            println!(
                "    - {} at ({}, {})",
                placement.npc_id, placement.position.x, placement.position.y
            );
        }
    }
}
