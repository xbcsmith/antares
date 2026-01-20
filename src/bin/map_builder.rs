// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Interactive Map Builder Tool
//!
//! **DEPRECATED**: NPC functionality has been removed. Use the campaign builder
//! and NPC database system instead. This tool now only supports basic map creation
//! and event placement.
//!
//! A command-line tool for creating and editing Antares RPG maps.
//!
//! # Features
//!
//! - Create new maps with specified dimensions
//! - Load and edit existing map RON files
//! - Set individual tiles or fill regions
//! - Add events (encounters, treasures, teleports, etc.)
//! - Real-time ASCII visualization
//! - Inline validation feedback
//! - Save maps in RON format
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin map_builder
//! ```
//!
//! # Commands
//!
//! - `new <id> <width> <height>` - Create new map
//! - `load <path>` - Load existing map
//! - `set <x> <y> <terrain> [wall]` - Set tile
//! - `fill <x1> <y1> <x2> <y2> <terrain> [wall]` - Fill region
//! - `event <x> <y> <type> <data>` - Add event
//! - `show` - Display map
//! - `info` - Show map info
//! - `save <path>` - Save map
//! - `help` - Show help
//! - `quit` - Exit builder
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 (Map structures).
//! See `docs/reference/map_ron_format.md` for RON format specification.

use antares::domain::types::{MapId, Position};
use antares::domain::world::{Map, MapEvent, TerrainType, Tile, WallType};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::io;

/// Map builder context
struct MapBuilder {
    map: Option<Map>,
    auto_show: bool,
}

impl MapBuilder {
    /// Creates a new map builder
    fn new() -> Self {
        Self {
            map: None,
            auto_show: true,
        }
    }

    /// Creates a new map with the specified dimensions
    fn create_map(&mut self, id: MapId, width: u32, height: u32) {
        if width == 0 || height == 0 {
            println!("❌ Error: Width and height must be greater than 0");
            return;
        }

        if width > 255 || height > 255 {
            println!("⚠️  Warning: Large maps (>255 tiles) may have performance issues");
        }

        self.map = Some(Map::new(
            id,
            format!("Map {}", id),
            String::new(),
            width,
            height,
        ));
        println!("✅ Created {}x{} map with ID {}", width, height, id);

        if self.auto_show {
            self.show_map();
        }
    }

    /// Loads a map from a RON file
    fn load_map(&mut self, path: &str) -> io::Result<()> {
        let contents = fs::read_to_string(path)?;
        let map: Map = ron::from_str(&contents).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("RON parse error: {}", e),
            )
        })?;

        self.map = Some(map);
        println!("✅ Loaded map from {}", path);

        if self.auto_show {
            self.show_map();
        }

        Ok(())
    }

    /// Sets a single tile
    fn set_tile(&mut self, x: i32, y: i32, terrain: TerrainType, wall: WallType) {
        let Some(ref mut map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        let pos = Position::new(x, y);
        if !map.is_valid_position(pos) {
            println!("❌ Error: Position ({}, {}) is out of bounds", x, y);
            return;
        }

        if let Some(tile) = map.get_tile_mut(pos) {
            *tile = Tile::new(x, y, terrain, wall);
            println!("✅ Set tile at ({}, {}) to {:?}/{:?}", x, y, terrain, wall);

            if self.auto_show {
                self.show_map();
            }
        }
    }

    /// Fills a rectangular region with the specified terrain and wall
    fn fill_tiles(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        terrain: TerrainType,
        wall: WallType,
    ) {
        let Some(ref mut map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        let mut count = 0;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = Position::new(x, y);
                if map.is_valid_position(pos) {
                    if let Some(tile) = map.get_tile_mut(pos) {
                        *tile = Tile::new(x, y, terrain, wall);
                        count += 1;
                    }
                }
            }
        }

        println!("✅ Filled {} tiles with {:?}/{:?}", count, terrain, wall);

        if self.auto_show {
            self.show_map();
        }
    }

    /// Bulk updates tiles that match any terrain in a comma-separated list.
    ///
    /// Example:
    ///   bulk Ground,Grass,Forest None false
    ///
    /// This updates only tiles whose current `terrain` is in the provided CSV,
    /// setting their `wall_type` and `blocked` flag to the provided values.
    fn bulk_set_for_terrains(&mut self, terrains_csv: &str, wall: WallType, blocked: bool) {
        let Some(ref mut map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        // Parse terrains CSV into TerrainType vector (using parse_terrain for robustness)
        let terrains: Vec<TerrainType> = terrains_csv
            .split(',')
            .map(|s| parse_terrain(s.trim()))
            .collect();

        let mut count = 0;
        for y in 0..map.height {
            for x in 0..map.width {
                let pos = Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile_mut(pos) {
                    if terrains.contains(&tile.terrain) {
                        tile.wall_type = wall;
                        tile.blocked = blocked;
                        count += 1;
                    }
                }
            }
        }

        println!(
            "✅ Bulk updated {} tiles (terrains: {}, wall: {:?}, blocked: {})",
            count, terrains_csv, wall, blocked
        );

        if self.auto_show {
            self.show_map();
        }
    }

    /// Adds an event at the specified position
    fn add_event(&mut self, x: i32, y: i32, event: MapEvent) {
        let Some(ref mut map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        let pos = Position::new(x, y);
        if !map.is_valid_position(pos) {
            println!("❌ Error: Position ({}, {}) is out of bounds", x, y);
            return;
        }

        map.add_event(pos, event);
        println!("✅ Added event at ({}, {})", x, y);

        if self.auto_show {
            self.show_map();
        }
    }

    /// Displays the map as ASCII art
    fn show_map(&self) {
        let Some(ref map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        println!("\n╔═══ Map {} ({}x{}) ═══╗", map.id, map.width, map.height);

        // X-axis label
        println!("     X-AXIS →");

        // Top border with X coordinates
        print!("   ");
        for x in 0..map.width {
            print!("{}", (x % 10));
        }
        println!();

        for y in 0..map.height {
            print!("{:2} ", y);
            for x in 0..map.width {
                let pos = Position::new(x as i32, y as i32);
                let tile = map.get_tile(pos).unwrap();

                let c = if tile.wall_type != WallType::None {
                    match tile.wall_type {
                        WallType::Normal => '#',
                        WallType::Door => '+',
                        WallType::Torch => '*',
                        WallType::None => unreachable!(),
                    }
                } else if map.events.contains_key(&pos) {
                    '!'
                } else {
                    match tile.terrain {
                        TerrainType::Ground => '.',
                        TerrainType::Grass => ',',
                        TerrainType::Water => '~',
                        TerrainType::Lava => '^',
                        TerrainType::Swamp => '%',
                        TerrainType::Stone => '░',
                        TerrainType::Dirt => ':',
                        TerrainType::Forest => '♣',
                        TerrainType::Mountain => '▲',
                    }
                };
                print!("{}", c);
            }
            println!();
        }

        // Y-axis label
        println!("↑");
        println!("Y");
        println!("A");
        println!("X");
        println!("I");
        println!("S");
        println!();
    }

    /// Shows map information
    fn show_info(&self) {
        let Some(ref map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        println!("\n╔═══ Map Information ═══╗");
        println!("ID: {}", map.id);
        println!("Dimensions: {}x{}", map.width, map.height);
        println!("Total tiles: {}", map.width * map.height);
        println!("Events: {}", map.events.len());
        println!("NPC Placements: {}", map.npc_placements.len());
        println!();
    }

    /// Saves the map to a RON file
    fn save_map(&self, path: &str) -> io::Result<()> {
        let Some(ref map) = self.map else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No map loaded"));
        };

        let ron_string = ron::ser::to_string_pretty(map, ron::ser::PrettyConfig::default())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("RON error: {}", e)))?;

        fs::write(path, ron_string)?;
        println!("✅ Saved map to {}", path);

        Ok(())
    }

    /// Processes a command and returns true if the program should continue
    fn process_command(&mut self, line: &str) -> bool {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return true;
        }

        match parts[0] {
            "new" => {
                if parts.len() != 4 {
                    println!("Usage: new <id> <width> <height>");
                    return true;
                }

                let id: MapId = match parts[1].parse() {
                    Ok(id) => id,
                    Err(_) => {
                        println!("❌ Error: Invalid map ID (must be a number)");
                        return true;
                    }
                };

                let width: u32 = match parts[2].parse() {
                    Ok(w) => w,
                    Err(_) => {
                        println!("❌ Error: Invalid width (must be a number)");
                        return true;
                    }
                };

                let height: u32 = match parts[3].parse() {
                    Ok(h) => h,
                    Err(_) => {
                        println!("❌ Error: Invalid height (must be a number)");
                        return true;
                    }
                };

                self.create_map(id, width, height);
            }
            "load" => {
                if parts.len() != 2 {
                    println!("Usage: load <path>");
                    return true;
                }
                if let Err(e) = self.load_map(parts[1]) {
                    println!("❌ Error: {}", e);
                }
            }
            "set" => {
                if parts.len() < 4 || parts.len() > 5 {
                    println!("Usage: set <x> <y> <terrain> [wall]");
                    return true;
                }

                let x: i32 = match parts[1].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("❌ Error: Invalid X coordinate");
                        return true;
                    }
                };

                let y: i32 = match parts[2].parse() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("❌ Error: Invalid Y coordinate");
                        return true;
                    }
                };

                let terrain = parse_terrain(parts[3]);
                let wall = if parts.len() == 5 {
                    parse_wall(parts[4])
                } else {
                    WallType::None
                };

                self.set_tile(x, y, terrain, wall);
            }
            "fill" => {
                if parts.len() < 6 || parts.len() > 7 {
                    println!("Usage: fill <x1> <y1> <x2> <y2> <terrain> [wall]");
                    return true;
                }

                let x1: i32 = match parts[1].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("❌ Error: Invalid X1 coordinate");
                        return true;
                    }
                };

                let y1: i32 = match parts[2].parse() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("❌ Error: Invalid Y1 coordinate");
                        return true;
                    }
                };

                let x2: i32 = match parts[3].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("❌ Error: Invalid X2 coordinate");
                        return true;
                    }
                };

                let y2: i32 = match parts[4].parse() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("❌ Error: Invalid Y2 coordinate");
                        return true;
                    }
                };

                let terrain = parse_terrain(parts[5]);
                let wall = if parts.len() == 7 {
                    parse_wall(parts[6])
                } else {
                    WallType::None
                };

                self.fill_tiles(x1, y1, x2, y2, terrain, wall);
            }

            "bulk" => {
                // Usage: bulk <terrain_csv> <wall> <blocked>
                // Example: bulk Ground,Grass,Forest None false
                if parts.len() != 4 {
                    println!("Usage: bulk <terrain_csv> <wall> <blocked>");
                    return true;
                }

                let terrains_csv = parts[1];
                let wall = parse_wall(parts[2]);
                let blocked = match parts[3].to_lowercase().as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        println!("❌ Error: blocked must be 'true' or 'false'");
                        return true;
                    }
                };

                self.bulk_set_for_terrains(terrains_csv, wall, blocked);
            }
            "event" => {
                if parts.len() < 5 {
                    println!("Usage: event <x> <y> <type> <data>");
                    return true;
                }

                let x: i32 = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("❌ Error: Invalid x coordinate");
                        return true;
                    }
                };

                let y: i32 = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("❌ Error: Invalid y coordinate");
                        return true;
                    }
                };

                let event_type = parts[3];
                let data = parts[4..].join(" ");

                let event = match event_type {
                    "encounter" => MapEvent::Encounter {
                        name: format!("Encounter at ({}, {})", x, y),
                        description: data,
                        monster_group: vec![],
                    },
                    "treasure" => MapEvent::Treasure {
                        name: format!("Treasure at ({}, {})", x, y),
                        description: data,
                        loot: vec![],
                    },
                    "sign" => MapEvent::Sign {
                        name: format!("Sign at ({}, {})", x, y),
                        description: String::new(),
                        text: data,
                    },
                    "trap" => MapEvent::Trap {
                        name: format!("Trap at ({}, {})", x, y),
                        description: data,
                        damage: 10,
                        effect: None,
                    },
                    _ => {
                        println!(
                            "❌ Error: Unknown event type. Use: encounter, treasure, sign, trap"
                        );
                        return true;
                    }
                };

                self.add_event(x, y, event);
            }
            "npc" => {
                println!("❌ NPC command is deprecated. Use the campaign builder and NPC database instead.");
                println!("   See docs/how-to/npc_externalization.md for migration guide.");
            }
            "show" => {
                self.show_map();
            }
            "auto" => {
                if parts.len() != 2 {
                    println!("Usage: auto [on|off]");
                    println!("Current: {}", if self.auto_show { "ON" } else { "OFF" });
                    return true;
                }

                match parts[1] {
                    "on" => {
                        self.auto_show = true;
                        println!("✅ Auto-show enabled");
                    }
                    "off" => {
                        self.auto_show = false;
                        println!("✅ Auto-show disabled");
                    }
                    _ => {
                        println!("Usage: auto [on|off]");
                    }
                }
            }
            "info" => {
                self.show_info();
            }
            "save" => {
                if parts.len() != 2 {
                    println!("Usage: save <path>");
                    return true;
                }
                if let Err(e) = self.save_map(parts[1]) {
                    println!("❌ Error: {}", e);
                }
            }
            "help" => {
                print_help();
            }
            "quit" | "exit" => {
                println!("Goodbye!");
                return false;
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }

        true
    }
}

/// Parses terrain type from string
fn parse_terrain(s: &str) -> TerrainType {
    match s.to_lowercase().as_str() {
        "ground" => TerrainType::Ground,
        "grass" => TerrainType::Grass,
        "water" => TerrainType::Water,
        "lava" => TerrainType::Lava,
        "swamp" => TerrainType::Swamp,
        "stone" => TerrainType::Stone,
        "dirt" => TerrainType::Dirt,
        "forest" => TerrainType::Forest,
        "mountain" => TerrainType::Mountain,
        _ => {
            println!("⚠️  Unknown terrain '{}', using Ground", s);
            TerrainType::Ground
        }
    }
}

/// Parses wall type from string
fn parse_wall(s: &str) -> WallType {
    match s.to_lowercase().as_str() {
        "none" => WallType::None,
        "normal" | "wall" => WallType::Normal,
        "door" => WallType::Door,
        "torch" => WallType::Torch,
        _ => {
            println!("⚠️  Unknown wall type '{}', using None", s);
            WallType::None
        }
    }
}

/// Prints help information
fn print_help() {
    println!("\n╔═══ Map Builder Commands ═══╗");
    println!("bulk <terrains_csv> <wall> <blocked> - Update tiles whose terrain is in comma-separated list, set wall type and blocked flag (e.g. bulk Ground,Grass None false)");
    println!("\n📋 Map Creation:");
    println!("  new <id> <width> <height>     Create new map (id must be a number)");
    println!("  load <path>                    Load existing map RON file");
    println!("  save <path>                    Save map to RON file");
    println!("\n✏️  Editing:");
    println!("  set <x> <y> <terrain> [wall]  Set single tile");
    println!("  fill <x1> <y1> <x2> <y2> <terrain> [wall]");
    println!("                                 Fill rectangular region");
    println!("\n🎭 Content:");
    println!("  event <x> <y> <type> <data>    Add event");
    println!("    Types: encounter, treasure, sign, trap");
    println!("\n👁️  Viewing:");
    println!("  show                           Display map (ASCII)");
    println!("  info                           Show map details");
    println!("  auto [on|off]                  Toggle auto-show after edits (default: ON)");
    println!("\n⚙️  Other:");
    println!("  help                           Show this help");
    println!("  quit                           Exit builder");
    println!("\n🎨 Terrain Types:");
    println!("  ground, grass, water, lava, swamp, stone, dirt, forest, mountain");
    println!("\n🧱 Wall Types:");
    println!("  none, normal, door, torch");
    println!("\n📝 EXAMPLES:");
    println!("  new 0 16 16                    Create 16x16 map with ID 0");
    println!("  fill 0 0 15 15 grass           Fill entire map with grass");
    println!("  set 8 8 stone normal           Place stone wall at center");
    println!("  set 8 9 stone door             Place door south of wall");
    println!("  event 5 5 sign Welcome!        Add welcome sign");
    println!("  save data/my_map.ron           Save to data directory");
    println!("  auto off                       Disable auto-show");
    println!();
}

fn main() {
    println!("╔═══════════════════════════════════════╗");
    println!("║   Antares RPG - Map Builder v1.0     ║");
    println!("╚═══════════════════════════════════════╝");
    println!("\nType 'help' for available commands.");
    println!("Type 'quit' or 'exit' to exit.");
    println!("Use ↑/↓ arrow keys for command history.\n");

    let mut builder = MapBuilder::new();
    let mut rl = DefaultEditor::new().expect("Failed to create readline editor");

    // Try to load history from file (ignore if it doesn't exist)
    let history_file = "data/.map_builder_history";
    let _ = rl.load_history(history_file);

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                let line = line.trim();
                if !line.is_empty() {
                    // Add to history
                    let _ = rl.add_history_entry(line);

                    // Process command
                    if !builder.process_command(line) {
                        break;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                println!("Use 'quit' or 'exit' to exit.");
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                eprintln!("Error reading input: {}", err);
                break;
            }
        }
    }

    // Save history on exit
    let _ = rl.save_history(history_file);
    println!("Goodbye!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_terrain() {
        assert_eq!(parse_terrain("ground"), TerrainType::Ground);
        assert_eq!(parse_terrain("grass"), TerrainType::Grass);
        assert_eq!(parse_terrain("water"), TerrainType::Water);
        assert_eq!(parse_terrain("FOREST"), TerrainType::Forest);
    }

    #[test]
    fn test_parse_wall() {
        assert_eq!(parse_wall("none"), WallType::None);
        assert_eq!(parse_wall("normal"), WallType::Normal);
        assert_eq!(parse_wall("door"), WallType::Door);
        assert_eq!(parse_wall("TORCH"), WallType::Torch);
    }

    #[test]
    fn test_create_map() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 10, 10);
        assert!(builder.map.is_some());

        let map = builder.map.unwrap();
        assert_eq!(map.id, 1);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);
    }

    #[test]
    fn test_set_tile() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 10, 10);
        builder.set_tile(5, 5, TerrainType::Water, WallType::None);

        let map = builder.map.as_ref().unwrap();
        let tile = map.get_tile(Position::new(5, 5)).unwrap();
        assert_eq!(tile.terrain, TerrainType::Water);
    }

    #[test]
    fn test_add_event() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 10, 10);

        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: String::new(),
            text: "Test sign".to_string(),
        };
        builder.add_event(3, 3, event);

        let map = builder.map.as_ref().unwrap();
        assert!(map.get_event(Position::new(3, 3)).is_some());
    }

    #[test]
    fn test_fill_tiles() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 10, 10);
        builder.fill_tiles(0, 0, 9, 0, TerrainType::Ground, WallType::Normal);

        let map = builder.map.as_ref().unwrap();
        for x in 0..10 {
            let tile = map.get_tile(Position::new(x, 0)).unwrap();
            assert_eq!(tile.wall_type, WallType::Normal);
        }
    }

    #[test]
    fn test_bulk_set_for_terrains() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 5, 5);

        // Prepare tiles: (1,1) Forest Normal (and blocked true), (2,2) Ground Normal (blocked true)
        builder.set_tile(1, 1, TerrainType::Forest, WallType::Normal);
        if let Some(ref mut map) = builder.map {
            map.get_tile_mut(Position::new(1, 1)).unwrap().blocked = true;
        }

        builder.set_tile(2, 2, TerrainType::Ground, WallType::Normal);
        if let Some(ref mut map) = builder.map {
            map.get_tile_mut(Position::new(2, 2)).unwrap().blocked = true;
        }

        // A tile that should remain unchanged: Stone
        builder.set_tile(3, 3, TerrainType::Stone, WallType::Normal);
        if let Some(ref mut map) = builder.map {
            map.get_tile_mut(Position::new(3, 3)).unwrap().blocked = true;
        }

        // Run bulk change for Ground and Forest -> set wall None, blocked false
        builder.bulk_set_for_terrains("Ground,Forest", WallType::None, false);

        let map = builder.map.as_ref().unwrap();
        let t11 = map.get_tile(Position::new(1, 1)).unwrap();
        assert_eq!(t11.wall_type, WallType::None);
        assert!(!t11.blocked);

        let t22 = map.get_tile(Position::new(2, 2)).unwrap();
        assert_eq!(t22.wall_type, WallType::None);
        assert!(!t22.blocked);

        let t33 = map.get_tile(Position::new(3, 3)).unwrap();
        assert_eq!(t33.wall_type, WallType::Normal);
        assert!(t33.blocked);
    }
}
