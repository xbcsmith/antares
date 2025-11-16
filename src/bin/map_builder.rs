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

        self.map = Some(Map::new(id, width, height));
        println!("✅ Created {}x{} map with ID {}", width, height, id);

        if self.auto_show {
            self.show_map();
        }
    }

    /// Loads a map from a RON file
    fn load_map(&mut self, path: &str) -> Result<(), String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let map: Map =
            ron::from_str(&contents).map_err(|e| format!("Failed to parse RON: {}", e))?;

        println!(
            "✅ Loaded map {} ({}x{}) with {} NPCs and {} events",
            map.id,
            map.width,
            map.height,
            map.npcs.len(),
            map.events.len()
        );

        self.map = Some(map);

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
    }

    /// Fills a rectangular region with tiles
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

        let (min_x, max_x) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (min_y, max_y) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };

        let mut count = 0;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = Position::new(x, y);
                if map.is_valid_position(pos) {
                    if let Some(tile) = map.get_tile_mut(pos) {
                        *tile = Tile::new(terrain, wall);
                        count += 1;
                    }
                }
            }
        }

        println!(
            "✅ Filled {} tiles from ({},{}) to ({},{}) with {:?}/{:?}",
            count, min_x, min_y, max_x, max_y, terrain, wall
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

    /// Adds an NPC
    fn add_npc(&mut self, id: u16, x: i32, y: i32, name: String, dialogue: String) {
        let Some(ref mut map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        let pos = Position::new(x, y);
        if !map.is_valid_position(pos) {
            println!("❌ Error: Position ({}, {}) is out of bounds", x, y);
            return;
        }

        // Check for duplicate NPC IDs
        if map.npcs.iter().any(|npc| npc.id == id) {
            println!("⚠️  Warning: NPC with ID {} already exists", id);
        }

        let npc = Npc::new(id, name.clone(), pos, dialogue);
        map.add_npc(npc);
        println!("✅ Added NPC '{}' (ID: {}) at ({}, {})", name, id, x, y);

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

        println!("\nLegend:");
        println!("  # = Wall    + = Door    * = Torch");
        println!("  . = Ground  , = Grass   ~ = Water  ^ = Lava");
        println!("  % = Swamp   ░ = Stone   : = Dirt   ♣ = Forest  ▲ = Mountain");
        println!("  ! = Event   @ = NPC");
        println!();
    }

    /// Shows map information
    fn show_info(&self) {
        let Some(ref map) = self.map else {
            println!("❌ Error: No map loaded. Use 'new' or 'load' first.");
            return;
        };

        println!("\n╔═══ Map Information ═══╗");
        println!("Map ID:     {}", map.id);
        println!(
            "Dimensions: {}x{} ({} tiles)",
            map.width,
            map.height,
            map.width * map.height
        );
        println!("NPCs:       {}", map.npcs.len());
        println!("Events:     {}", map.events.len());

        if !map.npcs.is_empty() {
            println!("\nNPCs:");
            for npc in &map.npcs {
                println!(
                    "  - ID {}: {} at ({}, {})",
                    npc.id, npc.name, npc.position.x, npc.position.y
                );
            }
        }

        if !map.events.is_empty() {
            println!("\nEvents:");
            for (pos, event) in &map.events {
                let event_type = match event {
                    MapEvent::Encounter { .. } => "Encounter",
                    MapEvent::Treasure { .. } => "Treasure",
                    MapEvent::Teleport { .. } => "Teleport",
                    MapEvent::Trap { .. } => "Trap",
                    MapEvent::Sign { .. } => "Sign",
                    MapEvent::NpcDialogue { .. } => "NPC Dialogue",
                };
                println!(
                    "  - {} at ({}, {}): {}",
                    event_type, pos.x, pos.y, event_type
                );
            }
        }

        println!();
    }

    /// Saves the map to a RON file
    fn save_map(&self, path: &str) -> Result<(), String> {
        let Some(ref map) = self.map else {
            return Err("No map loaded".to_string());
        };

        let ron_string = ron::ser::to_string_pretty(map, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        std::fs::write(path, ron_string).map_err(|e| format!("Failed to write file: {}", e))?;

        println!("✅ Saved map to {}", path);
        Ok(())
    }

    /// Processes a command
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
                let id = parts[1].parse().unwrap_or(0);
                let width = parts[2].parse().unwrap_or(0);
                let height = parts[3].parse().unwrap_or(0);
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
                if parts.len() < 4 {
                    println!("Usage: set <x> <y> <terrain> [wall]");
                    println!("Terrains: ground, grass, water, lava, swamp, stone, dirt, forest, mountain");
                    println!("Walls: none, normal, door, torch");
                    return true;
                }
                let x = parts[1].parse().unwrap_or(0);
                let y = parts[2].parse().unwrap_or(0);
                let terrain = parse_terrain(parts[3]);
                let wall = if parts.len() > 4 {
                    parse_wall(parts[4])
                } else {
                    WallType::None
                };
                self.set_tile(x, y, terrain, wall);
            }
            "fill" => {
                if parts.len() < 6 {
                    println!("Usage: fill <x1> <y1> <x2> <y2> <terrain> [wall]");
                    return true;
                }
                let x1 = parts[1].parse().unwrap_or(0);
                let y1 = parts[2].parse().unwrap_or(0);
                let x2 = parts[3].parse().unwrap_or(0);
                let y2 = parts[4].parse().unwrap_or(0);
                let terrain = parse_terrain(parts[5]);
                let wall = if parts.len() > 6 {
                    parse_wall(parts[6])
                } else {
                    WallType::None
                };
                self.fill_tiles(x1, y1, x2, y2, terrain, wall);
            }
            "event" => {
                if parts.len() < 4 {
                    println!("Usage: event <x> <y> <type> <data>");
                    println!("Types: encounter, treasure, teleport, trap, sign, dialogue");
                    return true;
                }
                let x = parts[1].parse().unwrap_or(0);
                let y = parts[2].parse().unwrap_or(0);
                let event_type = parts[3];

                let event = match event_type {
                    "encounter" => {
                        if parts.len() < 5 {
                            println!("Usage: event <x> <y> encounter <monster_ids>");
                            return true;
                        }
                        let monsters: Vec<u8> =
                            parts[4..].iter().filter_map(|s| s.parse().ok()).collect();
                        MapEvent::Encounter {
                            monster_group: monsters,
                        }
                    }
                    "treasure" => {
                        if parts.len() < 5 {
                            println!("Usage: event <x> <y> treasure <item_ids>");
                            return true;
                        }
                        let loot: Vec<u8> =
                            parts[4..].iter().filter_map(|s| s.parse().ok()).collect();
                        MapEvent::Treasure { loot }
                    }
                    "sign" => {
                        if parts.len() < 5 {
                            println!("Usage: event <x> <y> sign <text>");
                            return true;
                        }
                        let text = parts[4..].join(" ");
                        MapEvent::Sign { text }
                    }
                    "trap" => {
                        if parts.len() < 5 {
                            println!("Usage: event <x> <y> trap <damage> [effect]");
                            return true;
                        }
                        let damage = parts[4].parse().unwrap_or(10);
                        let effect = if parts.len() > 5 {
                            Some(parts[5].to_string())
                        } else {
                            None
                        };
                        MapEvent::Trap { damage, effect }
                    }
                    _ => {
                        println!("Unknown event type: {}", event_type);
                        return true;
                    }
                };

                self.add_event(x, y, event);
            }
            "npc" => {
                if parts.len() < 6 {
                    println!("Usage: npc <id> <x> <y> <name> <dialogue>");
                    return true;
                }
                let id = parts[1].parse().unwrap_or(0);
                let x = parts[2].parse().unwrap_or(0);
                let y = parts[3].parse().unwrap_or(0);
                let name = parts[4].to_string();
                let dialogue = parts[5..].join(" ");
                self.add_npc(id, x, y, name, dialogue);
            }
            "show" => {
                self.show_map();
            }
            "auto" => {
                if parts.len() == 1 {
                    println!(
                        "Auto-show is currently: {}",
                        if self.auto_show { "ON" } else { "OFF" }
                    );
                } else {
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
