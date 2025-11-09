# Map Content Implementation Plan for Antares RPG

**Purpose**: This plan provides a structured approach to creating map content and tooling for the Antares RPG. The world system (Phase 3) is complete, but no actual map data exists yet.

**Status**: World system implemented, `data/maps/` directory empty

**Architecture Reference**: `docs/reference/architecture.md` Section 4.2 (World System)

---

## Table of Contents

1. [Current State Assessment](#current-state-assessment)
2. [Phase 1: Map Format Documentation & Starter Map](#phase-1-map-format-documentation--starter-map)
3. [Phase 2: Map Builder Tool](#phase-2-map-builder-tool)
4. [Phase 3: Additional Content Maps](#phase-3-additional-content-maps)
5. [Validation Requirements](#validation-requirements)

---

## Current State Assessment

### ✅ Already Implemented (Phase 3)

**World Data Structures** (`src/domain/world/types.rs`):
- `Map` - 2D grid with tiles, events, NPCs
- `Tile` - terrain, walls, blocking, visited state, event triggers
- `World` - collection of maps, party position/facing
- `MapEvent` enum - Encounter, Treasure, Teleport, Trap, Sign, NpcDialogue
- `Npc` - non-player character definitions
- `TerrainType` - Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain
- `WallType` - None, Normal, Door, Torch

**Movement System** (`src/domain/world/movement.rs`):
- Party movement with collision detection
- Map boundary enforcement
- Tile visited tracking
- Integration with Direction type

**Event System** (`src/domain/world/events.rs`):
- Event triggering at positions
- One-time vs. repeatable events
- Event result handling for all 6 event types

**Testing**: 22 unit tests covering all movement and event scenarios

### ❌ What's Missing

**Map Data Files**:
- No town maps (starter town, merchant areas)
- No dungeon maps (starter dungeon, caves, castles)
- No outdoor maps (overworld, forests, mountains)
- No example/template maps

**Documentation**:
- No RON format guide for map creation
- No best practices for map design
- No examples showing event placement

**Tooling**:
- No map builder/editor
- No map validator
- No map loader utility for testing

---

## Phase 1: Map Format Documentation & Starter Map

### Goals

1. Document the RON format for maps comprehensively
2. Create a small, well-commented example map (starter town)
3. Provide templates for common map patterns
4. Create validation that maps load correctly

### Architecture References

**READ BEFORE IMPLEMENTING:**
- Section 4.2: World (Map, Tile, MapEvent, Npc)
- Section 4.6: Type Aliases (MapId, EventId, Position)
- Section 7.2: Data Files (RON format examples)

### Task 1.1: Map Format Documentation

**Objective**: Create comprehensive reference documentation for map RON format.

**File**: `docs/reference/map_format.md`

**Content Structure**:

```markdown
# Map RON Format Reference

## Overview
Maps are defined in RON format in `data/maps/` directory.

## Map Structure
(Complete field-by-field breakdown)

## Tile Types
(All TerrainType and WallType values with descriptions)

## Event Types
(All 6 MapEvent types with field descriptions and examples)

## Position System
(Coordinate system, origin, boundaries)

## NPC Definitions
(NPC structure with fields)

## Complete Example
(Fully annotated small map)

## Common Patterns
(Templates for common scenarios)

## Validation Rules
(What makes a valid map)
```

**Documentation Requirements**:
- Use Diataxis "Reference" category (factual, technical)
- Include complete field descriptions
- Provide examples for each event type
- Show coordinate system clearly (0,0 = top-left)
- Explain blocking vs. non-blocking terrain
- Document wall types and their effects

**Implementation Steps**:

1. Create `docs/reference/map_format.md`
2. Document Map structure:
   - `id: MapId` - Unique map identifier
   - `width: u32` - Map width in tiles
   - `height: u32` - Map height in tiles
   - `tiles: Vec<Vec<Tile>>` - 2D grid [y][x]
   - `events: HashMap<Position, MapEvent>` - Events at positions
   - `npcs: Vec<Npc>` - NPCs on this map

3. Document Tile structure with all fields:
   - `terrain: TerrainType` - Ground type
   - `wall_type: WallType` - Wall/door/torch
   - `blocked: bool` - Blocks movement
   - `is_special: bool` - Special tile marker
   - `is_dark: bool` - Requires light
   - `visited: bool` - Party has been here
   - `event_trigger: Option<EventId>` - Optional event

4. Document all TerrainType variants:
   - Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain
   - Which are blocked by default
   - Visual/gameplay differences

5. Document all WallType variants:
   - None, Normal, Door, Torch
   - Which block movement
   - Door mechanics

6. Document all 6 MapEvent types with examples:
   - Encounter { monster_group: Vec<u8> }
   - Treasure { loot: Vec<u8> }
   - Teleport { destination: Position, map_id: MapId }
   - Trap { damage: u16, effect: Option<String> }
   - Sign { text: String }
   - NpcDialogue { npc_id: u16 }

7. Show Position coordinate system:
   - (0, 0) = top-left corner
   - x increases right
   - y increases down

8. Provide common patterns:
   - 4-wall room template
   - Corridor template
   - Door placement
   - NPC + dialogue event combination
   - Treasure room
   - Trap corridor

**Validation**:
- [ ] All Map fields documented
- [ ] All Tile fields documented
- [ ] All terrain types explained
- [ ] All wall types explained
- [ ] All 6 event types with examples
- [ ] Coordinate system clearly shown
- [ ] Common patterns provided
- [ ] File uses lowercase_with_underscores.md naming

### Task 1.2: Starter Town Map

**Objective**: Create a small, well-documented example map that demonstrates all features.

**File**: `data/maps/starter_town.ron`

**Design Specifications**:
- Size: 16x16 tiles (small, manageable)
- Layout: Simple town with:
  - Town entrance (south)
  - Central plaza with sign
  - 2-3 buildings with doors
  - NPC merchant
  - Small treasure chest (starter items)
  - Safe area (no encounters)

**Map ID**: Use `MapId = 1` for starter town

**Implementation Steps**:

1. Create `data/maps/starter_town.ron`

2. Define map header:
   ```ron
   Map(
       id: 1,
       width: 16,
       height: 16,
       tiles: [
           // Row-by-row tile definitions
       ],
       events: {
           // Position -> Event mappings
       },
       npcs: [
           // NPC definitions
       ],
   )
   ```

3. Build tile grid (16x16 = 256 tiles):
   - Outer border: Mountain terrain (blocked)
   - Entrance: South side, Ground terrain, no walls
   - Central plaza: Ground terrain (4x4 area)
   - Buildings: Stone terrain with Normal walls, Doors for entrances
   - Paths: Dirt terrain connecting areas
   - Grass: Grass terrain for decoration

4. Add Sign event at plaza center:
   ```ron
   Position(x: 8, y: 8): Sign(
       text: "Welcome to Sorpigal! Visit the merchant for supplies.",
   ),
   ```

5. Add NPC merchant:
   - Position: Inside building at (5, 5)
   - ID: 1
   - Name: "Marcus the Merchant"
   - Dialogue: "Welcome, traveler! I have weapons, armor, and supplies."

6. Add NpcDialogue event at merchant position:
   ```ron
   Position(x: 5, y: 5): NpcDialogue(
       npc_id: 1,
   ),
   ```

7. Add Treasure event:
   - Position: Small room at (12, 12)
   - Loot: Starter items (IDs 1-3)
   ```ron
   Position(x: 12, y: 12): Treasure(
       loot: [1, 2, 3],
   ),
   ```

8. Add Torch lights:
   - Place 4-5 torch walls around town for lighting
   - Positions: Near buildings, plaza

9. Add extensive comments:
   ```ron
   // === STARTER TOWN - SORPIGAL ===
   // Map ID: 1
   // Size: 16x16
   //
   // Layout:
   //   - Entrance: South side (x: 8, y: 15)
   //   - Plaza: Center (8x8 to 9x9)
   //   - Merchant: Building northwest (5x5)
   //   - Treasure: Small room northeast (12x12)
   //
   // Events:
   //   - Sign at plaza
   //   - Merchant NPC
   //   - Starter treasure chest
   ```

**Testing Requirements**:

Create `tests/map_loading_test.rs`:

```rust
use antares::domain::world::Map;
use std::fs;

#[test]
fn test_load_starter_town() {
    let contents = fs::read_to_string("data/maps/starter_town.ron")
        .expect("Failed to read starter_town.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse starter_town.ron");

    // Validate map structure
    assert_eq!(map.id, 1);
    assert_eq!(map.width, 16);
    assert_eq!(map.height, 16);
    assert_eq!(map.tiles.len(), 16); // 16 rows
    assert_eq!(map.tiles[0].len(), 16); // 16 columns

    // Validate events
    assert!(map.events.len() > 0, "Map should have events");

    // Validate NPCs
    assert_eq!(map.npcs.len(), 1, "Should have merchant NPC");
    assert_eq!(map.npcs[0].id, 1);
    assert_eq!(map.npcs[0].name, "Marcus the Merchant");
}

#[test]
fn test_starter_town_has_entrance() {
    let contents = fs::read_to_string("data/maps/starter_town.ron")
        .expect("Failed to read starter_town.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse starter_town.ron");

    // South entrance should be passable
    let entrance_tile = &map.tiles[15][8];
    assert!(!entrance_tile.blocked, "Entrance should not be blocked");
}

#[test]
fn test_starter_town_has_sign() {
    let contents = fs::read_to_string("data/maps/starter_town.ron")
        .expect("Failed to read starter_town.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse starter_town.ron");

    let plaza_pos = Position::new(8, 8);
    assert!(map.events.contains_key(&plaza_pos), "Plaza should have sign event");
}
```

**Validation**:
- [ ] Map file parses without errors
- [ ] Map dimensions correct (16x16)
- [ ] All tiles have valid terrain/wall types
- [ ] Border tiles are blocked (mountains)
- [ ] Entrance is passable
- [ ] Sign event at plaza
- [ ] Merchant NPC present
- [ ] NpcDialogue event at merchant position
- [ ] Treasure event present
- [ ] Tests pass: `cargo test test_load_starter_town`
- [ ] File uses .ron extension
- [ ] Well-commented for reference

### Task 1.3: Map Templates

**Objective**: Create reusable RON snippets for common map patterns.

**File**: `docs/how_to/map_templates.md`

**Content**: Provide copy-paste templates for:

1. **Empty Map Template**:
   ```ron
   Map(
       id: 0,
       width: 20,
       height: 20,
       tiles: [
           // TODO: Add tile rows
       ],
       events: {},
       npcs: [],
   )
   ```

2. **Tile Row Template** (20 ground tiles):
   ```ron
   [
       Tile(terrain: Ground, wall_type: None, blocked: false, is_special: false, is_dark: false, visited: false, event_trigger: None),
       // ... repeat 19 more times
   ],
   ```

3. **4-Wall Room** (5x5):
   ```ron
   // Row 0: North wall
   [
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
   ],
   // Row 1-3: Side walls + interior
   [
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: None, blocked: false, ...),
       Tile(terrain: Stone, wall_type: None, blocked: false, ...),
       Tile(terrain: Stone, wall_type: None, blocked: false, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
   ],
   // Row 4: South wall with door
   [
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Door, blocked: false, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
       Tile(terrain: Stone, wall_type: Normal, blocked: true, ...),
   ],
   ```

4. **Corridor Template** (1x10 horizontal):
   ```ron
   [
       Tile(terrain: Stone, wall_type: None, blocked: false, ...),
       // ... repeat 9 more times
   ],
   ```

5. **Sign Event**:
   ```ron
   Position(x: 10, y: 10): Sign(
       text: "Your message here.",
   ),
   ```

6. **Treasure Event**:
   ```ron
   Position(x: 5, y: 5): Treasure(
       loot: [1, 2, 3], // Item IDs
   ),
   ```

7. **NPC + Dialogue**:
   ```ron
   // In npcs array:
   Npc(
       id: 1,
       name: "NPC Name",
       position: Position(x: 10, y: 10),
       dialogue: "Greeting message.",
   ),

   // In events:
   Position(x: 10, y: 10): NpcDialogue(
       npc_id: 1,
   ),
   ```

8. **Trap Event**:
   ```ron
   Position(x: 7, y: 7): Trap(
       damage: 10,
       effect: Some("poison"),
   ),
   ```

9. **Encounter Event**:
   ```ron
   Position(x: 15, y: 15): Encounter(
       monster_group: [1, 1, 2], // Monster IDs
   ),
   ```

10. **Teleport Event**:
    ```ron
    Position(x: 20, y: 20): Teleport(
        destination: Position(x: 5, y: 5),
        map_id: 2,
    ),
    ```

**Validation**:
- [ ] All templates syntactically correct
- [ ] Templates cover common scenarios
- [ ] File in docs/how_to/ (task-oriented)
- [ ] File uses lowercase_with_underscores.md

### Task 1.4: Map Validation Utility

**Objective**: Create a command-line tool to validate map RON files.

**File**: `examples/validate_map.rs`

**Functionality**:
- Load map from RON file
- Validate structure
- Check for common errors
- Report issues clearly

**Implementation**:

```rust
// examples/validate_map.rs

use antares::domain::world::Map;
use antares::domain::types::Position;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: cargo run --example validate_map <path/to/map.ron>");
        process::exit(1);
    }

    let path = &args[1];

    println!("Validating map: {}", path);
    println!();

    // Load file
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to read file: {}", e);
            process::exit(1);
        }
    };

    // Parse RON
    let map: Map = match ron::from_str(&contents) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("❌ Failed to parse RON: {}", e);
            process::exit(1);
        }
    };

    println!("✅ RON syntax valid");

    // Validate structure
    let mut errors = Vec::new();

    // Check dimensions
    if map.width == 0 || map.height == 0 {
        errors.push("Map dimensions must be > 0".to_string());
    }

    // Check tile grid
    if map.tiles.len() != map.height as usize {
        errors.push(format!(
            "Tile rows ({}) don't match height ({})",
            map.tiles.len(),
            map.height
        ));
    }

    for (y, row) in map.tiles.iter().enumerate() {
        if row.len() != map.width as usize {
            errors.push(format!(
                "Row {} has {} tiles, expected {}",
                y,
                row.len(),
                map.width
            ));
        }
    }

    if errors.is_empty() {
        println!("✅ Tile grid dimensions valid");
    }

    // Check event positions
    for (pos, _event) in &map.events {
        if pos.x < 0 || pos.x >= map.width as i32 {
            errors.push(format!(
                "Event at ({}, {}) has x out of bounds [0, {})",
                pos.x, pos.y, map.width
            ));
        }
        if pos.y < 0 || pos.y >= map.height as i32 {
            errors.push(format!(
                "Event at ({}, {}) has y out of bounds [0, {})",
                pos.x, pos.y, map.height
            ));
        }
    }

    if errors.is_empty() {
        println!("✅ Event positions valid");
    }

    // Check NPC positions
    for npc in &map.npcs {
        if npc.position.x < 0 || npc.position.x >= map.width as i32 {
            errors.push(format!(
                "NPC '{}' at ({}, {}) has x out of bounds [0, {})",
                npc.name, npc.position.x, npc.position.y, map.width
            ));
        }
        if npc.position.y < 0 || npc.position.y >= map.height as i32 {
            errors.push(format!(
                "NPC '{}' at ({}, {}) has y out of bounds [0, {})",
                npc.name, npc.position.x, npc.position.y, map.height
            ));
        }
    }

    if errors.is_empty() {
        println!("✅ NPC positions valid");
    }

    // Report
    println!();
    println!("=== Map Summary ===");
    println!("ID: {}", map.id);
    println!("Size: {}x{}", map.width, map.height);
    println!("Tiles: {}", map.width * map.height);
    println!("Events: {}", map.events.len());
    println!("NPCs: {}", map.npcs.len());
    println!();

    if errors.is_empty() {
        println!("✅ All validations passed!");
        process::exit(0);
    } else {
        println!("❌ Validation errors found:");
        for error in errors {
            println!("  - {}", error);
        }
        process::exit(1);
    }
}
```

**Usage**:
```bash
cargo run --example validate_map data/maps/starter_town.ron
```

**Validation**:
- [ ] Example compiles
- [ ] Validates starter_town.ron successfully
- [ ] Detects dimension mismatches
- [ ] Detects out-of-bounds events
- [ ] Detects out-of-bounds NPCs
- [ ] Clear error messages
- [ ] File in examples/ directory
- [ ] File uses .rs extension

### Phase 1 Completion Checklist

**Documentation**:
- [ ] `docs/reference/map_format.md` created and complete
- [ ] `docs/how_to/map_templates.md` created with templates
- [ ] Both files use lowercase_with_underscores.md naming
- [ ] Architecture.md Section 4.2 referenced correctly

**Map Content**:
- [ ] `data/maps/starter_town.ron` created
- [ ] Map is 16x16 with valid structure
- [ ] Map has sign, merchant NPC, treasure
- [ ] Map parses without errors
- [ ] File uses .ron extension

**Testing**:
- [ ] `tests/map_loading_test.rs` created
- [ ] Tests load and validate starter_town.ron
- [ ] All tests pass: `cargo test map_loading`

**Validation Tool**:
- [ ] `examples/validate_map.rs` created
- [ ] Tool validates starter_town.ron successfully
- [ ] File uses .rs extension

**Quality Gates**:
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all-features` passes (all existing + new tests)

**Documentation Update**:
- [ ] Updated `docs/explanation/implementations.md` with Phase 1 summary

---

## Phase 2: Map Builder Tool

### Goals

1. Create interactive CLI tool for building maps
2. Support all tile types, events, and NPCs
3. Export to RON format
4. Make map creation faster and less error-prone

### Architecture References

**READ BEFORE IMPLEMENTING:**
- Section 4.2: World (Map, Tile, MapEvent, Npc)
- Section 4.6: Type Aliases (MapId, EventId, Position)

### Task 2.1: Map Builder Core

**Objective**: Create a CLI map builder with interactive commands.

**File**: `src/bin/map_builder.rs`

**Note**: This is a binary crate, not an example. Use `src/bin/` directory.

**Functionality**:
- Create new map with dimensions
- Set individual tiles
- Add events at positions
- Add NPCs
- Save to RON file
- Load existing RON file for editing

**Commands**:
- `new <width> <height> <id>` - Create new map
- `load <path>` - Load existing map
- `set <x> <y> <terrain> <wall>` - Set tile
- `fill <terrain> <wall>` - Fill all tiles
- `rect <x1> <y1> <x2> <y2> <terrain> <wall>` - Fill rectangle
- `event <x> <y> <type> <params>` - Add event
- `npc <id> <x> <y> <name> <dialogue>` - Add NPC
- `show` - Display map ASCII
- `info` - Show map summary
- `save <path>` - Save to RON file
- `help` - Show commands
- `quit` - Exit

**Implementation Structure**:

```rust
// src/bin/map_builder.rs

use antares::domain::world::{Map, Tile, MapEvent, Npc, TerrainType, WallType};
use antares::domain::types::Position;
use std::collections::HashMap;
use std::io::{self, Write};

struct MapBuilder {
    map: Option<Map>,
}

impl MapBuilder {
    fn new() -> Self {
        Self { map: None }
    }

    fn create_map(&mut self, width: u32, height: u32, id: u16) {
        // Create map with default ground tiles
    }

    fn load_map(&mut self, path: &str) -> Result<(), String> {
        // Load from RON file
    }

    fn set_tile(&mut self, x: i32, y: i32, terrain: TerrainType, wall: WallType) -> Result<(), String> {
        // Set individual tile
    }

    fn fill_tiles(&mut self, terrain: TerrainType, wall: WallType) -> Result<(), String> {
        // Fill all tiles
    }

    fn fill_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, terrain: TerrainType, wall: WallType) -> Result<(), String> {
        // Fill rectangular area
    }

    fn add_event(&mut self, x: i32, y: i32, event: MapEvent) -> Result<(), String> {
        // Add event at position
    }

    fn add_npc(&mut self, id: u16, x: i32, y: i32, name: String, dialogue: String) -> Result<(), String> {
        // Add NPC
    }

    fn show_map(&self) {
        // Display ASCII representation
    }

    fn show_info(&self) {
        // Show map summary
    }

    fn save_map(&self, path: &str) -> Result<(), String> {
        // Save to RON file
    }

    fn process_command(&mut self, cmd: &str) -> Result<bool, String> {
        // Parse and execute command
        // Returns Ok(true) to continue, Ok(false) to quit
    }
}

fn main() {
    let mut builder = MapBuilder::new();

    println!("=== Antares Map Builder ===");
    println!("Type 'help' for commands");
    println!();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let cmd = input.trim();
        if cmd.is_empty() {
            continue;
        }

        match builder.process_command(cmd) {
            Ok(true) => continue,
            Ok(false) => break,
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("Goodbye!");
}
```

**ASCII Map Display**:
```text
Show map terrain types:
  . = Ground
  : = Grass
  ~ = Water
  ^ = Mountain
  # = Stone
  T = Forest

Show wall types overlaid:
  | = Normal wall
  + = Door
  * = Torch

Example display:
  ^^^^^^^^^^^^^^^^
  ^...........:..^
  ^.#####+####...^
  ^.#........#...^
  ^.#...T....#...^
  ^.#........#...^
  ^.#####+####...^
  ^..............^
  ^^^^^^^^^^^^^^^^
```

**Validation**:
- [ ] Binary compiles: `cargo build --bin map_builder`
- [ ] Can create new map
- [ ] Can set tiles
- [ ] Can add events
- [ ] Can add NPCs
- [ ] Can save to valid RON
- [ ] Can load existing RON
- [ ] ASCII display works
- [ ] Help command lists all commands
- [ ] File in src/bin/ directory
- [ ] File uses .rs extension

### Task 2.2: Map Builder Enhancements

**Objective**: Add advanced features to make building easier.

**Features to Add**:

1. **Border Command**:
   ```
   border <terrain> <wall>
   ```
   Sets all edge tiles to specified type.

2. **Room Command**:
   ```
   room <x> <y> <width> <height> <door_side>
   ```
   Creates a walled room with door on specified side (n/s/e/w).

3. **Corridor Command**:
   ```
   corridor <x1> <y1> <x2> <y2>
   ```
   Creates a corridor between two points.

4. **Copy Command**:
   ```
   copy <x1> <y1> <x2> <y2> <dest_x> <dest_y>
   ```
   Copies rectangular area to new location.

5. **Undo/Redo**:
   Store command history, allow undo/redo.

6. **Template Command**:
   ```
   template <name>
   ```
   Load pre-defined templates (4x4 room, corridor, etc.).

7. **Validate Command**:
   ```
   validate
   ```
   Run validation checks on current map.

**Validation**:
- [ ] All new commands work correctly
- [ ] Border command fills edges
- [ ] Room command creates proper structure
- [ ] Corridor command connects points
- [ ] Copy command duplicates area
- [ ] Undo/redo works for tile changes
- [ ] Templates load correctly
- [ ] Validate command reports issues

### Task 2.3: Map Builder Documentation

**Objective**: Document the map builder tool for users.

**File**: `docs/how_to/using_map_builder.md`

**Content**:
- Installation/running instructions
- Command reference with examples
- Workflow guide (create -> edit -> save)
- Tips and best practices
- Common patterns (room, corridor, etc.)
- Troubleshooting

**Example Section**:
```markdown
## Creating a Simple Room

1. Create new map:
   ```
   > new 10 10 2
   ```

2. Fill with ground:
   ```
   > fill Ground None
   ```

3. Add border walls:
   ```
   > border Mountain Normal
   ```

4. Create room:
   ```
   > room 3 3 4 4 s
   ```

5. Add sign:
   ```
   > event 5 5 Sign "Welcome to the room!"
   ```

6. Save:
   ```
   > save data/maps/simple_room.ron
   ```
```

**Validation**:
- [ ] File created in docs/how_to/
- [ ] All commands documented with examples
- [ ] Workflow guide complete
- [ ] File uses lowercase_with_underscores.md

### Phase 2 Completion Checklist

**Map Builder Tool**:
- [ ] `src/bin/map_builder.rs` created
- [ ] All basic commands work (new, set, fill, event, npc, save, load)
- [ ] Enhanced commands work (border, room, corridor, copy, undo, template)
- [ ] ASCII display functional
- [ ] Validation command works
- [ ] File uses .rs extension

**Documentation**:
- [ ] `docs/how_to/using_map_builder.md` created
- [ ] All commands documented
- [ ] Examples provided
- [ ] File uses lowercase_with_underscores.md

**Testing**:
- [ ] Tool builds: `cargo build --bin map_builder`
- [ ] Can create and save simple map
- [ ] Generated RON files are valid
- [ ] Can load and edit existing maps

**Quality Gates**:
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo build --bin map_builder` succeeds

**Documentation Update**:
- [ ] Updated `docs/explanation/implementations.md` with Phase 2 summary

---

## Phase 3: Additional Content Maps

### Goals

1. Create starter dungeon map
2. Create outdoor area map
3. Create additional town areas
4. Demonstrate all event types in actual gameplay

### Architecture References

**READ BEFORE IMPLEMENTING:**
- Section 4.2: World (Map, Tile, MapEvent)
- Section 5.4: Map and Movement (gameplay context)

### Task 3.1: Starter Dungeon

**Objective**: Create a small dungeon with combat encounters and treasure.

**File**: `data/maps/starter_dungeon.ron`

**Design Specifications**:
- Size: 20x20 tiles
- Map ID: 2
- Layout:
  - Entrance room (connects to town via teleport)
  - 2-3 rooms connected by corridors
  - Dead ends with treasure
  - Monster encounters in rooms
  - Traps in corridors
  - Dark areas (requires light)

**Features to Include**:
- 3-4 Encounter events (different monster groups)
- 2 Treasure events (loot chests)
- 2-3 Trap events (damage + poison)
- Teleport back to town
- Stone walls, doors, torches
- Dark tiles (is_dark: true)

**Implementation Steps**:

1. Use map_builder tool or hand-edit RON
2. Create dungeon layout:
   - Border: Mountain (impassable)
   - Rooms: Stone terrain, Normal walls, Doors
   - Corridors: Stone terrain, torch walls for light
   - Dark areas: Set is_dark: true on tiles

3. Add entrance teleport (from town):
   ```ron
   // In starter_town.ron, add:
   Position(x: 14, y: 14): Teleport(
       destination: Position(x: 1, y: 1),
       map_id: 2,
   ),
   ```

4. Add exit teleport (back to town):
   ```ron
   // In starter_dungeon.ron:
   Position(x: 1, y: 1): Teleport(
       destination: Position(x: 14, y: 14),
       map_id: 1,
   ),
   ```

5. Add encounters:
   ```ron
   Position(x: 10, y: 10): Encounter(
       monster_group: [1, 1], // 2 goblins
   ),
   Position(x: 15, y: 5): Encounter(
       monster_group: [2], // 1 orc
   ),
   ```

6. Add treasure:
   ```ron
   Position(x: 18, y: 18): Treasure(
       loot: [5, 6, 7], // Magic items
   ),
   ```

7. Add traps:
   ```ron
   Position(x: 8, y: 8): Trap(
       damage: 15,
       effect: Some("poison"),
   ),
   ```

8. Add torches for lighting:
   - Place torch walls every 3-4 tiles in corridors
   - Rooms should have 2-4 torches

**Testing**:

Add to `tests/map_loading_test.rs`:

```rust
#[test]
fn test_load_starter_dungeon() {
    let contents = fs::read_to_string("data/maps/starter_dungeon.ron")
        .expect("Failed to read starter_dungeon.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse starter_dungeon.ron");

    assert_eq!(map.id, 2);
    assert_eq!(map.width, 20);
    assert_eq!(map.height, 20);

    // Should have encounters
    let encounter_count = map.events.values()
        .filter(|e| matches!(e, MapEvent::Encounter { .. }))
        .count();
    assert!(encounter_count >= 3, "Should have at least 3 encounters");

    // Should have treasure
    let treasure_count = map.events.values()
        .filter(|e| matches!(e, MapEvent::Treasure { .. }))
        .count();
    assert!(treasure_count >= 2, "Should have at least 2 treasures");

    // Should have traps
    let trap_count = map.events.values()
        .filter(|e| matches!(e, MapEvent::Trap { .. }))
        .count();
    assert!(trap_count >= 2, "Should have at least 2 traps");
}
```

**Validation**:
- [ ] Map file parses without errors
- [ ] Map dimensions correct (20x20)
- [ ] Entrance/exit teleports link correctly
- [ ] Encounters present
- [ ] Treasure chests present
- [ ] Traps present
- [ ] Dark areas marked
- [ ] Torches provide lighting
- [ ] Tests pass
- [ ] File uses .ron extension

### Task 3.2: Outdoor Area

**Objective**: Create an outdoor wilderness map.

**File**: `data/maps/forest_area.ron`

**Design Specifications**:
- Size: 30x30 tiles (larger outdoor space)
- Map ID: 3
- Layout:
  - Forest terrain
  - Grass clearings
  - Water stream (impassable)
  - Mountain borders
  - Open area (no walls)
  - Random encounters
  - Hidden treasure
  - Sign markers

**Features to Include**:
- Multiple Encounter events (wilderness creatures)
- Treasure hidden in forest
- Sign pointing to town/dungeon
- Varied terrain (grass, forest, water, mountain)
- No walls/doors (outdoor)
- Brighter (not dark)

**Implementation Steps**:

1. Create base terrain:
   - Border: Mountain (impassable)
   - Main area: Forest terrain
   - Clearings: Grass terrain
   - Stream: Water terrain (blocked)

2. Add encounters (more frequent outdoors):
   ```ron
   Position(x: 5, y: 5): Encounter(monster_group: [3, 3]), // Wolves
   Position(x: 15, y: 10): Encounter(monster_group: [4]), // Bear
   Position(x: 20, y: 20): Encounter(monster_group: [1, 1, 1]), // Goblins
   // Add 5-6 more encounters scattered
   ```

3. Add hidden treasure:
   ```ron
   Position(x: 25, y: 25): Treasure(
       loot: [10, 11], // Rare items
   ),
   ```

4. Add sign at entrance:
   ```ron
   Position(x: 2, y: 2): Sign(
       text: "Beware! Wild creatures roam these woods.",
   ),
   ```

5. Add teleport to/from town:
   ```ron
   // In forest_area.ron:
   Position(x: 1, y: 15): Teleport(
       destination: Position(x: 8, y: 1),
       map_id: 1,
   ),

   // In starter_town.ron, add:
   Position(x: 8, y: 0): Teleport(
       destination: Position(x: 1, y: 15),
       map_id: 3,
   ),
   ```

**Testing**:

Add to `tests/map_loading_test.rs`:

```rust
#[test]
fn test_load_forest_area() {
    let contents = fs::read_to_string("data/maps/forest_area.ron")
        .expect("Failed to read forest_area.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse forest_area.ron");

    assert_eq!(map.id, 3);
    assert_eq!(map.width, 30);
    assert_eq!(map.height, 30);

    // Should have many encounters (outdoor area)
    let encounter_count = map.events.values()
        .filter(|e| matches!(e, MapEvent::Encounter { .. }))
        .count();
    assert!(encounter_count >= 5, "Outdoor area should have many encounters");
}
```

**Validation**:
- [ ] Map file parses without errors
- [ ] Map dimensions correct (30x30)
- [ ] Varied terrain types present
- [ ] Multiple encounters
- [ ] Treasure present
- [ ] Sign present
- [ ] Teleport links to town
- [ ] Water blocks movement
- [ ] No walls (outdoor)
- [ ] Tests pass
- [ ] File uses .ron extension

### Task 3.3: Map Integration Documentation

**Objective**: Document how all maps connect and the overall world structure.

**File**: `docs/explanation/world_layout.md`

**Content**:
- Overview of all maps
- Map connections (teleports)
- Recommended progression path
- Event summary per map
- ASCII diagrams showing connections

**Map Connection Diagram**:
```text
┌─────────────────┐
│  Starter Town   │
│    (Map 1)      │
│                 │
│ - Sign          │
│ - Merchant NPC  │
│ - Treasure      │
└────┬──────┬─────┘
     │      │
     │      └──────────┐
     │                 │
     ▼                 ▼
┌─────────────┐  ┌──────────────┐
│   Dungeon   │  │    Forest    │
│   (Map 2)   │  │   (Map 3)    │
│             │  │              │
│ - Encounters│  │ - Encounters │
│ - Treasure  │  │ - Treasure   │
│ - Traps     │  │ - Sign       │
└─────────────┘  └──────────────┘
```

**Event Summary Table**:
```markdown
| Map ID | Name          | Size  | Events | NPCs | Encounters | Treasure | Traps |
|--------|---------------|-------|--------|------|------------|----------|-------|
| 1      | Starter Town  | 16x16 | 3      | 1    | 0          | 1        | 0     |
| 2      | Dungeon       | 20x20 | 7      | 0    | 3          | 2        | 2     |
| 3      | Forest Area   | 30x30 | 8      | 0    | 5          | 1        | 0     |
```

**Validation**:
- [ ] File created in docs/explanation/
- [ ] All maps documented
- [ ] Connection diagram included
- [ ] Event summary table complete
- [ ] File uses lowercase_with_underscores.md

### Phase 3 Completion Checklist

**Map Content**:
- [ ] `data/maps/starter_dungeon.ron` created (20x20)
- [ ] `data/maps/forest_area.ron` created (30x30)
- [ ] All maps parse without errors
- [ ] All maps have proper events
- [ ] Teleports link maps correctly
- [ ] Files use .ron extension

**Testing**:
- [ ] Added tests for new maps
- [ ] All map loading tests pass
- [ ] `cargo test map_loading` succeeds

**Documentation**:
- [ ] `docs/explanation/world_layout.md` created
- [ ] All maps documented
- [ ] Connection diagram included
- [ ] File uses lowercase_with_underscores.md

**Quality Gates**:
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all-features` passes

**Documentation Update**:
- [ ] Updated `docs/explanation/implementations.md` with Phase 3 summary

---

## Validation Requirements

### After Each Phase

**Run Quality Checks**:
```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**Verify Map Files**:
```bash
# Validate each map
cargo run --example validate_map data/maps/starter_town.ron
cargo run --example validate_map data/maps/starter_dungeon.ron
cargo run --example validate_map data/maps/forest_area.ron
```

**Test Map Loading**:
```bash
cargo test map_loading
```

### Architecture Compliance Checklist

- [ ] All Map structures match architecture.md Section 4.2
- [ ] All Tile fields present and correct
- [ ] All MapEvent types used correctly
- [ ] Position coordinates correct (0-based, top-left origin)
- [ ] Type aliases used (MapId, EventId, Position)
- [ ] RON format used for all data files (.ron extension)
- [ ] No JSON or YAML for map data
- [ ] TerrainType and WallType variants from architecture
- [ ] Event types match architecture enum

### Documentation Checklist

- [ ] `docs/reference/map_format.md` - Complete RON reference
- [ ] `docs/how_to/map_templates.md` - Reusable templates
- [ ] `docs/how_to/using_map_builder.md` - Tool guide
- [ ] `docs/explanation/world_layout.md` - Map connections
- [ ] All files use lowercase_with_underscores.md naming
- [ ] Proper Diataxis categories (reference, how-to, explanation)

### File Structure Checklist

- [ ] `data/maps/starter_town.ron` - 16x16 town map
- [ ] `data/maps/starter_dungeon.ron` - 20x20 dungeon map
- [ ] `data/maps/forest_area.ron` - 30x30 outdoor map
- [ ] `examples/validate_map.rs` - Validation tool
- [ ] `src/bin/map_builder.rs` - Map builder binary
- [ ] `tests/map_loading_test.rs` - Map loading tests
- [ ] All .rs files have SPDX copyright headers
- [ ] All data files use .ron extension

### Testing Checklist

- [ ] Map parsing tests pass
- [ ] Map validation tests pass
- [ ] Event type tests pass
- [ ] NPC tests pass
- [ ] Teleport linking tests pass
- [ ] All tests run: `cargo test --all-features`
- [ ] Test coverage >80% for new code

---

## Summary

This plan provides a complete path from zero map content to a fully functional world with:

1. **Phase 1**: Documentation and reference map (starter town)
2. **Phase 2**: Tools to accelerate map creation (builder)
3. **Phase 3**: Diverse content (dungeon, outdoor area)

**Total Deliverables**:
- 3 playable maps (town, dungeon, forest)
- 4 documentation files (format reference, templates, builder guide, world layout)
- 2 tools (validator, builder)
- Comprehensive tests

**Estimated Effort**:
- Phase 1: 8-12 hours (documentation + reference map)
- Phase 2: 12-16 hours (builder tool development)
- Phase 3: 6-8 hours (content creation)
- Total: ~26-36 hours

**Dependencies**:
- Phase 3 world system (already complete)
- RON format understanding
- Architecture.md Section 4.2 compliance

**Next Steps After Completion**:
- Create additional towns/dungeons
- Add procedural generation
- Implement map loader in game loop
- Add map rendering system
