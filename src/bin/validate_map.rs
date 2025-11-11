//! Map Validation Utility
//!
//! This tool validates RON map files for Antares RPG before deployment.
//! It checks structural integrity, content validity, and gameplay constraints.
//!
//! Usage:
//!   cargo run --bin validate_map data/maps/town_starter.ron
//!   cargo run --bin validate_map data/maps/*.ron

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

// Valid Monster IDs (1-255, from data/monsters.ron)
// TODO: Load dynamically from monsters.ron when database loader is implemented
#[allow(dead_code)]
const VALID_MONSTER_IDS: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Valid Item IDs (1-255, from data/items.ron)
// TODO: Load dynamically from items.ron when database loader is implemented
#[allow(dead_code)]
const VALID_ITEM_IDS: &[u8] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30,
];

// Valid tile type IDs
const VALID_TILE_TYPES: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];

/// Simplified map structure for validation (mirrors architecture.md Section 4.2)
#[derive(Debug, serde::Deserialize)]
struct MapData {
    id: u16, // MapId
    name: String,
    map_type: String,
    width: u32,
    height: u32,
    #[allow(dead_code)]
    outdoor: bool,
    #[allow(dead_code)]
    allow_resting: bool,
    danger_level: u8,
    tiles: Vec<Vec<u8>>,
    events: Vec<EventData>,
    npcs: Vec<NpcData>,
    exits: Vec<ExitData>,
}

#[derive(Debug, serde::Deserialize)]
struct EventData {
    position: PositionData,
    #[allow(dead_code)]
    event_type: String, // Simplified for validation
    #[allow(dead_code)]
    repeatable: bool,
    #[allow(dead_code)]
    triggered: bool,
}

#[derive(Debug, serde::Deserialize)]
struct NpcData {
    id: u32,
    name: String,
    position: PositionData,
    #[allow(dead_code)]
    dialogue_id: u32,
    #[allow(dead_code)]
    shop_id: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
struct ExitData {
    position: PositionData,
    destination_map: u16, // MapId
    #[allow(dead_code)]
    destination_pos: PositionData,
    #[allow(dead_code)]
    direction: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone, Copy)]
struct PositionData {
    x: u32,
    y: u32,
}

/// Validation error types
#[derive(Debug)]
enum ValidationError {
    Structure(String),
    Content(String),
    Gameplay(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Structure(msg) => write!(f, "[STRUCTURE] {}", msg),
            ValidationError::Content(msg) => write!(f, "[CONTENT] {}", msg),
            ValidationError::Gameplay(msg) => write!(f, "[GAMEPLAY] {}", msg),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <map_file.ron> [map_file2.ron ...]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} data/maps/town_starter.ron", args[0]);
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
                println!("\nMap Summary:");
                println!("  ID: {}", map.id);
                println!("  Name: {}", map.name);
                println!("  Type: {}", map.map_type);
                println!("  Size: {}x{}", map.width, map.height);
                println!("  Events: {}", map.events.len());
                println!("  NPCs: {}", map.npcs.len());
                println!("  Exits: {}", map.exits.len());
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
fn validate_map_file(file_path: &str) -> Result<MapData, Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Read file
    let path = Path::new(file_path);
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(ValidationError::Structure(format!(
                "Failed to read file: {}",
                e
            )));
            return Err(errors);
        }
    };

    // Parse RON
    let map: MapData = match ron::from_str(&contents) {
        Ok(m) => m,
        Err(e) => {
            errors.push(ValidationError::Structure(format!(
                "Failed to parse RON: {}",
                e
            )));
            return Err(errors);
        }
    };

    // Run validation checks
    validate_structure(&map, &mut errors);
    validate_content(&map, &mut errors);
    validate_gameplay(&map, &mut errors);

    if errors.is_empty() {
        Ok(map)
    } else {
        Err(errors)
    }
}

/// Validate map structure (dimensions, tile array integrity)
fn validate_structure(map: &MapData, errors: &mut Vec<ValidationError>) {
    // Check map ID
    if map.id == 0 {
        errors.push(ValidationError::Structure(
            "Map ID must be non-zero (valid range: 1-65535)".to_string(),
        ));
    }

    // Check dimensions
    if map.width < 4 || map.width > 256 {
        errors.push(ValidationError::Structure(format!(
            "Width {} out of range (valid: 4-256)",
            map.width
        )));
    }

    if map.height < 4 || map.height > 256 {
        errors.push(ValidationError::Structure(format!(
            "Height {} out of range (valid: 4-256)",
            map.height
        )));
    }

    // Check tiles array structure
    if map.tiles.len() != map.height as usize {
        errors.push(ValidationError::Structure(format!(
            "Tiles array has {} rows, expected {}",
            map.tiles.len(),
            map.height
        )));
    }

    for (row_idx, row) in map.tiles.iter().enumerate() {
        if row.len() != map.width as usize {
            errors.push(ValidationError::Structure(format!(
                "Row {} has {} tiles, expected {}",
                row_idx,
                row.len(),
                map.width
            )));
        }

        // Check tile type IDs
        for (col_idx, &tile_type) in row.iter().enumerate() {
            if !VALID_TILE_TYPES.contains(&tile_type) {
                errors.push(ValidationError::Structure(format!(
                    "Invalid tile type {} at ({}, {})",
                    tile_type, col_idx, row_idx
                )));
            }
        }
    }

    // Check danger level range
    if map.danger_level > 10 {
        errors.push(ValidationError::Structure(format!(
            "Danger level {} exceeds maximum (0-10)",
            map.danger_level
        )));
    }
}

/// Validate map content (events, NPCs, exits within bounds and valid)
fn validate_content(map: &MapData, errors: &mut Vec<ValidationError>) {
    // Validate events
    for (idx, event) in map.events.iter().enumerate() {
        if !is_position_valid(&event.position, map.width, map.height) {
            errors.push(ValidationError::Content(format!(
                "Event {} position ({}, {}) out of bounds (map: {}x{})",
                idx, event.position.x, event.position.y, map.width, map.height
            )));
        }

        // Check if event is on impassable tile
        if let Some(tile_type) = get_tile_type(&map.tiles, &event.position) {
            if tile_type == 1 {
                // Wall
                errors.push(ValidationError::Content(format!(
                    "Event {} at ({}, {}) is on impassable tile (wall)",
                    idx, event.position.x, event.position.y
                )));
            }
        }
    }

    // Validate NPCs
    let mut npc_ids = HashSet::new();
    for npc in &map.npcs {
        if !is_position_valid(&npc.position, map.width, map.height) {
            errors.push(ValidationError::Content(format!(
                "NPC '{}' position ({}, {}) out of bounds",
                npc.name, npc.position.x, npc.position.y
            )));
        }

        // Check NPC ID uniqueness
        if !npc_ids.insert(npc.id) {
            errors.push(ValidationError::Content(format!(
                "Duplicate NPC ID {} ('{}')",
                npc.id, npc.name
            )));
        }

        // Check if NPC is on impassable tile
        if let Some(tile_type) = get_tile_type(&map.tiles, &npc.position) {
            if tile_type == 1 {
                errors.push(ValidationError::Content(format!(
                    "NPC '{}' at ({}, {}) is on impassable tile (wall)",
                    npc.name, npc.position.x, npc.position.y
                )));
            }
        }
    }

    // Validate exits
    for exit in &map.exits {
        if !is_position_valid(&exit.position, map.width, map.height) {
            errors.push(ValidationError::Content(format!(
                "Exit position ({}, {}) out of bounds",
                exit.position.x, exit.position.y
            )));
        }

        if exit.destination_map == 0 {
            errors.push(ValidationError::Content(
                "Exit destination_map must be non-zero".to_string(),
            ));
        }
    }
}

/// Validate gameplay constraints (recommended practices)
fn validate_gameplay(map: &MapData, errors: &mut Vec<ValidationError>) {
    // Check for border walls (recommended)
    let has_border_walls = check_border_walls(map);
    if !has_border_walls {
        errors.push(ValidationError::Gameplay(
            "Map lacks complete wall border (recommended for safety)".to_string(),
        ));
    }

    // Check for exits in non-dungeon maps
    if map.map_type != "Dungeon" && map.exits.is_empty() {
        errors.push(ValidationError::Gameplay(format!(
            "{} map should have at least one exit",
            map.map_type
        )));
    }

    // Warn about large danger levels in town
    if map.map_type == "Town" && map.danger_level > 2 {
        errors.push(ValidationError::Gameplay(format!(
            "Town has high danger level ({}) - typically towns are safe (0-2)",
            map.danger_level
        )));
    }
}

/// Check if position is within map bounds
fn is_position_valid(pos: &PositionData, width: u32, height: u32) -> bool {
    pos.x < width && pos.y < height
}

/// Get tile type at position (if valid)
fn get_tile_type(tiles: &[Vec<u8>], pos: &PositionData) -> Option<u8> {
    tiles
        .get(pos.y as usize)
        .and_then(|row| row.get(pos.x as usize))
        .copied()
}

/// Check if map has complete border of walls
fn check_border_walls(map: &MapData) -> bool {
    if map.tiles.is_empty() {
        return false;
    }

    let width = map.width as usize;
    let height = map.height as usize;

    // Check top and bottom rows
    for x in 0..width {
        if map.tiles[0].get(x) != Some(&1) || map.tiles[height - 1].get(x) != Some(&1) {
            return false;
        }
    }

    // Check left and right columns
    for y in 0..height {
        if map.tiles[y].first() != Some(&1) || map.tiles[y].last() != Some(&1) {
            return false;
        }
    }

    true
}
