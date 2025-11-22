// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Interactive Map Builder Tool
//!
//! A command-line tool for creating and editing Antares RPG maps.
//!
//! # Features
//!
//! - Create new maps with specified dimensions
//! - Load and edit existing map RON files
//! - Set individual tiles or fill regions
//! - Add events (encounters, treasures, teleports, etc.)
//! - Add NPCs with dialogue
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
//! - `npc <id> <x> <y> <name> <dialogue>` - Add NPC
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
use antares::domain::world::{Map, MapEvent, Npc, TerrainType, Tile, WallType};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Map builder context
struct MapBuilder {
    map: Option<Map>,
    auto_show: bool,
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

        self.map = Some(Map::new(id, width, height));
        println!("✅ Created {}x{} map with ID {}", width, height, id);

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
            *tile = Tile::new(terrain, wall);
            println!("✅ Set tile at ({}, {}) to {:?}/{:?}", x, y, terrain, wall);

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
                } else if map.npcs.iter().any(|npc| npc.position == pos) {
                    '@'
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
    println!("  npc <id> <x> <y> <name> <dialogue>");
    println!("                                 Add NPC (id must be a number)");
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
    println!("  npc 1 10 10 Guard \"Halt!\"      Add guard NPC (ID must be number)");
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
            text: "Test sign".to_string(),
        };
        builder.add_event(3, 3, event);

        let map = builder.map.as_ref().unwrap();
        assert!(map.get_event(Position::new(3, 3)).is_some());
    }

    #[test]
    fn test_add_npc() {
        let mut builder = MapBuilder::new();
        builder.create_map(1, 10, 10);
        builder.add_npc(1, 5, 5, "Merchant".to_string(), "Welcome!".to_string());

        let map = builder.map.as_ref().unwrap();
        assert_eq!(map.npcs.len(), 1);
        assert_eq!(map.npcs[0].name, "Merchant");
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
}
