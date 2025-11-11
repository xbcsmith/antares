# Map Content Implementation Plan for Antares RPG (v2 - CORRECTED)

**Purpose**: This plan provides a structured approach to creating map content and tooling for the Antares RPG. The world system (Phase 3) is complete, but no actual map data exists yet.

**Status**: World system implemented, `data/maps/` directory empty

**Architecture Reference**: `docs/reference/architecture.md` Section 4.2 (World System)

**Data Reference**: `docs/reference/data_dependencies.md` (Monster/Item ID reference)

**Version**: 2.0 (Corrected Monster/Item IDs, Revised Phase Ordering)

**Changes from v1**:

- Fixed Monster ID references (ID 2 is Kobold not Orc, ID 3 is Giant Rat not Wolf)
- Removed reference to non-existent Bear (ID 4)
- Revised phase ordering: Tools before manual maps
- Standardized on `src/bin/` for binaries
- Added coordinate system documentation
- Clarified type alias usage

---

## Table of Contents

1. [Current State Assessment](#current-state-assessment)
2. [Phase 1: Documentation & Foundation](#phase-1-documentation--foundation)
3. [Phase 2: Map Builder Tool](#phase-2-map-builder-tool)
4. [Phase 3: Content Creation](#phase-3-content-creation)
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

**Serialization**: All types derive `Serialize`/`Deserialize` for RON format ✓

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

## Phase 1: Documentation & Foundation

### Goals

1. Document the RON format for maps comprehensively
2. Create map validation utility
3. Provide templates for common map patterns
4. Establish coordinate system and ID usage clearly

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.2: World (Map, Tile, MapEvent, Npc)
- Section 4.6: Type Aliases (MapId, EventId, Position)
- Section 7.2: Data Files (RON format examples)
- `docs/reference/data_dependencies.md` (Monster/Item IDs)

### Task 1.1: Map Format Documentation

**Objective**: Create comprehensive reference documentation for map RON format.

**File**: `docs/reference/map_format.md`

**Content Structure**:

```markdown
# Map RON Format Reference

## Overview

Maps are defined in RON format in `data/maps/` directory.

## Coordinate System

Maps use a 2D coordinate system:

- Origin (0, 0) is the **top-left** corner
- X-axis increases to the **right**
- Y-axis increases **down**
- All map positions must be non-negative
- Position type uses `i32` internally (for movement calculations) but map coordinates are always >= 0

Example 5x5 map:
```

0 1 2 3 4
0 . . . . .
1 . . . . .
2 . . X . . <- Position(x: 2, y: 2)
3 . . . . .
4 . . . . .

```

## Type Aliases

When working with maps, use these type aliases:
- `MapId = u16` - Unique map identifier
- `EventId = u16` - Event identifier (optional per tile)
- `MonsterId = u8` - Monster identifier (in Encounter events)
- `ItemId = u8` - Item identifier (in Treasure events)

**Important**: In RON files, you'll see raw integers (e.g., `id: 1`), but these correspond to the type aliases in code.

## Map Structure

Complete field-by-field breakdown with examples...
(Full documentation follows)
```

**Implementation Steps**:

1. Create `docs/reference/map_format.md`

2. Add **Coordinate System** section (see above)

3. Add **Type Aliases** section explaining MonsterId/ItemId usage:

   ```markdown
   ## Monster and Item IDs

   ### Monster IDs (MonsterId = u8)

   Used in Encounter events. See `docs/reference/data_dependencies.md` for complete list.

   Common monsters:

   - ID 1: Goblin
   - ID 10: Orc
   - ID 12: Wolf

   ### Item IDs (ItemId = u8)

   Used in Treasure events. See `docs/reference/data_dependencies.md` for complete list.

   Common items:

   - ID 1: Club
   - ID 20: Leather Armor
   - ID 50: Healing Potion
   ```

4. Document Map structure:

   - `id: MapId` - Unique map identifier (u16)
   - `width: u32` - Map width in tiles
   - `height: u32` - Map height in tiles
   - `tiles: Vec<Vec<Tile>>` - 2D grid [y][x] (row-major order)
   - `events: HashMap<Position, MapEvent>` - Events at positions
   - `npcs: Vec<Npc>` - NPCs on this map

5. Document Tile structure with all fields:

   - `terrain: TerrainType` - Ground type
   - `wall_type: WallType` - Wall/door/torch
   - `blocked: bool` - Blocks movement
   - `is_special: bool` - Special tile marker
   - `is_dark: bool` - Requires light
   - `visited: bool` - Party has been here (always start false)
   - `event_trigger: Option<EventId>` - Optional event reference

6. Document all TerrainType variants:

   - Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain
   - Which are blocked by default (Water, Mountain)
   - Visual/gameplay differences

7. Document all WallType variants:

   - None, Normal, Door, Torch
   - Which block movement (Normal blocks, Door is passable)
   - Door mechanics, Torch provides light

8. Document all 6 MapEvent types with **CORRECTED** examples:

   **Encounter Event**:

   ```ron
   Position(x: 10, y: 10): Encounter(
       monster_group: [1, 1], // 2 Goblins (MonsterId)
   ),
   ```

   Note: Refer to `docs/reference/data_dependencies.md` for valid Monster IDs.

   **Treasure Event**:

   ```ron
   Position(x: 5, y: 5): Treasure(
       loot: [1, 20, 50], // Club, Leather Armor, Healing Potion (ItemId)
   ),
   ```

   Note: Refer to `docs/reference/data_dependencies.md` for valid Item IDs.

   **Teleport Event**:

   ```ron
   Position(x: 15, y: 15): Teleport(
       destination: Position(x: 5, y: 5),
       map_id: 2,
   ),
   ```

   **Trap Event**:

   ```ron
   Position(x: 8, y: 8): Trap(
       damage: 15,
       effect: Some("poison"),
   ),
   ```

   **Sign Event**:

   ```ron
   Position(x: 12, y: 12): Sign(
       text: "Beware! Monsters ahead.",
   ),
   ```

   **NpcDialogue Event**:

   ```ron
   Position(x: 7, y: 7): NpcDialogue(
       npc_id: 1,
   ),
   ```

9. Provide common patterns:

   - 4-wall room template
   - Corridor template
   - Door placement
   - NPC + dialogue event combination
   - Treasure room
   - Trap corridor

10. Add validation rules section:
    - Map dimensions must be > 0
    - Tile grid must match width × height
    - Event positions must be within bounds
    - NPC positions must be within bounds
    - Monster IDs must exist in monsters.ron
    - Item IDs must exist in items.ron

**Validation**:

- [ ] All Map fields documented
- [ ] All Tile fields documented
- [ ] Coordinate system clearly explained
- [ ] Type aliases explained (MonsterId, ItemId)
- [ ] All terrain types explained
- [ ] All wall types explained
- [ ] All 6 event types with corrected examples
- [ ] Position system uses i32 but coordinates are >= 0
- [ ] Common patterns provided
- [ ] Validation rules listed
- [ ] File uses lowercase_with_underscores.md naming
- [ ] Cross-references data_dependencies.md

### Task 1.2: Map Validation Utility

**Objective**: Create a command-line tool to validate map RON files.

**File**: `src/bin/validate_map.rs`

**Note**: Changed from `examples/` to `src/bin/` for consistency.

**Functionality**:

- Load map from RON file
- Validate structure
- Check Monster/Item IDs against known data
- Report issues clearly

**Implementation**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/bin/validate_map.rs

use antares::domain::world::{Map, MapEvent};
use antares::domain::types::Position;
use std::env;
use std::fs;
use std::process;

// Known valid IDs from data files
const VALID_MONSTER_IDS: &[u8] = &[1, 2, 3, 10, 11, 12, 20, 21, 22, 30, 31];
const VALID_ITEM_IDS: &[u8] = &[
    1, 2, 3, 4, 5, 6, 7,        // Basic weapons
    10, 11, 12,                  // Magic weapons
    20, 21, 22,                  // Basic armor
    30, 31,                      // Magic armor
    40, 41, 42,                  // Accessories
    50, 51, 52,                  // Consumables
    60, 61,                      // Ammo
    100, 101,                    // Quest/Cursed
];

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: cargo run --bin validate_map <path/to/map.ron>");
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
    for (pos, event) in &map.events {
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

        // Validate Monster IDs
        if let MapEvent::Encounter { monster_group } = event {
            for &id in monster_group {
                if !VALID_MONSTER_IDS.contains(&id) {
                    errors.push(format!(
                        "Invalid Monster ID {} at ({}, {}). Check data/monsters.ron",
                        id, pos.x, pos.y
                    ));
                }
            }
        }

        // Validate Item IDs
        if let MapEvent::Treasure { loot } = event {
            for &id in loot {
                if !VALID_ITEM_IDS.contains(&id) {
                    errors.push(format!(
                        "Invalid Item ID {} at ({}, {}). Check data/items.ron",
                        id, pos.x, pos.y
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        println!("✅ Event positions valid");
        println!("✅ Monster/Item IDs valid");
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

    // Count event types
    let mut encounters = 0;
    let mut treasures = 0;
    let mut traps = 0;
    for event in map.events.values() {
        match event {
            MapEvent::Encounter { .. } => encounters += 1,
            MapEvent::Treasure { .. } => treasures += 1,
            MapEvent::Trap { .. } => traps += 1,
            _ => {}
        }
    }
    println!("  - Encounters: {}", encounters);
    println!("  - Treasures: {}", treasures);
    println!("  - Traps: {}", traps);
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
cargo run --bin validate_map data/maps/starter_town.ron
```

**Validation**:

- [ ] Binary compiles
- [ ] Validates dimension mismatches
- [ ] Validates out-of-bounds events
- [ ] Validates out-of-bounds NPCs
- [ ] Validates Monster IDs against known list
- [ ] Validates Item IDs against known list
- [ ] Clear error messages
- [ ] File in src/bin/ directory
- [ ] File uses .rs extension
- [ ] SPDX copyright header present

### Task 1.3: Map Templates Documentation

**Objective**: Create reusable RON snippets for common map patterns.

**File**: `docs/how_to/map_templates.md`

**Content**: Provide copy-paste templates for:

1. **Empty Map Template**:

   ```ron
   Map(
       id: 1,
       width: 20,
       height: 20,
       tiles: [
           // TODO: Add tile rows (use map builder tool recommended)
       ],
       events: {},
       npcs: [],
   )
   ```

2. **Single Tile Template** (with all fields):

   ```ron
   Tile(
       terrain: Ground,
       wall_type: None,
       blocked: false,
       is_special: false,
       is_dark: false,
       visited: false,
       event_trigger: None,
   )
   ```

3. **Tile Row Template** (20 ground tiles):

   ```ron
   [
       Tile(terrain: Ground, wall_type: None, blocked: false, is_special: false, is_dark: false, visited: false, event_trigger: None),
       Tile(terrain: Ground, wall_type: None, blocked: false, is_special: false, is_dark: false, visited: false, event_trigger: None),
       // ... repeat 18 more times
   ],
   ```

4. **Event Templates** (with CORRECTED IDs):

   **Starter Encounters**:

   ```ron
   Position(x: 5, y: 5): Encounter(
       monster_group: [1, 1], // 2 Goblins
   ),
   Position(x: 10, y: 10): Encounter(
       monster_group: [2, 2, 2], // 3 Kobolds
   ),
   Position(x: 15, y: 15): Encounter(
       monster_group: [3, 3, 3, 3], // 4 Giant Rats
   ),
   ```

   **Mid-Level Encounters**:

   ```ron
   Position(x: 8, y: 8): Encounter(
       monster_group: [10], // 1 Orc
   ),
   Position(x: 12, y: 12): Encounter(
       monster_group: [12, 12], // 2 Wolves
   ),
   Position(x: 18, y: 18): Encounter(
       monster_group: [11, 11], // 2 Skeletons
   ),
   ```

   **Starter Treasure**:

   ```ron
   Position(x: 5, y: 5): Treasure(
       loot: [1, 2, 20], // Club, Dagger, Leather Armor
   ),
   Position(x: 10, y: 10): Treasure(
       loot: [50, 60], // Healing Potion, Arrows
   ),
   ```

   **Mid-Level Treasure**:

   ```ron
   Position(x: 15, y: 15): Treasure(
       loot: [4, 21, 50], // Long Sword, Chain Mail, Healing Potion
   ),
   Position(x: 20, y: 20): Treasure(
       loot: [10, 40], // Club +1, Ring of Protection
   ),
   ```

5. **NPC + Dialogue Template**:

   ```ron
   // In npcs array:
   Npc(
       id: 1,
       name: "Merchant Marcus",
       position: Position(x: 10, y: 10),
       dialogue: "Welcome! I have supplies for sale.",
   ),

   // In events:
   Position(x: 10, y: 10): NpcDialogue(
       npc_id: 1,
   ),
   ```

6. **Sign Template**:

   ```ron
   Position(x: 8, y: 8): Sign(
       text: "Welcome to Sorpigal! Beware the dungeons to the east.",
   ),
   ```

7. **Trap Template**:

   ```ron
   Position(x: 7, y: 7): Trap(
       damage: 10,
       effect: Some("poison"),
   ),
   ```

8. **Teleport Template**:
   ```ron
   Position(x: 5, y: 5): Teleport(
       destination: Position(x: 15, y: 15),
       map_id: 2,
   ),
   ```

**Important Note in Templates**:

```markdown
## Important: Use Map Builder Tool

Creating maps manually by typing out tiles is tedious and error-prone.
We recommend using the map builder tool (Phase 2) for actual map creation.

These templates are provided for:

- Understanding the RON format
- Quick reference
- Small edits to existing maps
```

**Validation**:

- [ ] All templates syntactically correct
- [ ] Templates use CORRECTED Monster/Item IDs
- [ ] Templates reference data_dependencies.md
- [ ] Note about using map builder tool
- [ ] File in docs/how_to/ (task-oriented)
- [ ] File uses lowercase_with_underscores.md

### Phase 1 Completion Checklist

**Documentation**:

- [ ] `docs/reference/map_format.md` created and complete
- [ ] `docs/how_to/map_templates.md` created with templates
- [ ] Coordinate system documented (0,0 = top-left, i32 type)
- [ ] Type aliases explained (MonsterId, ItemId)
- [ ] Both files use lowercase_with_underscores.md naming
- [ ] Architecture.md Section 4.2 referenced correctly
- [ ] data_dependencies.md cross-referenced

**Validation Tool**:

- [ ] `src/bin/validate_map.rs` created
- [ ] Tool validates dimensions
- [ ] Tool validates positions
- [ ] Tool validates Monster IDs against known list
- [ ] Tool validates Item IDs against known list
- [ ] File uses .rs extension
- [ ] SPDX copyright header present

**Quality Gates**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo build --bin validate_map` succeeds

**Documentation Update**:

- [ ] Updated `docs/explanation/implementations.md` with Phase 1 summary

---

## Phase 2: Map Builder Tool

### Goals

1. Create interactive CLI tool for building maps
2. Support all tile types, events, and NPCs
3. Export to valid RON format
4. Make map creation 10x faster than manual editing

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.2: World (Map, Tile, MapEvent, Npc)
- Section 4.6: Type Aliases (MapId, EventId, Position)
- `docs/reference/data_dependencies.md` (Monster/Item IDs)

### Task 2.1: Map Builder Core (MVP)

**Objective**: Create a minimal viable map builder with essential commands.

**File**: `src/bin/map_builder.rs`

**Functionality**:

- Create new map with dimensions
- Set individual tiles
- Fill all tiles
- Add events at positions (with ID validation)
- Add NPCs
- Save to RON file
- Load existing RON file for editing
- Display ASCII map
- Show map info

**Commands**:

- `new <width> <height> <id>` - Create new map
- `load <path>` - Load existing map
- `set <x> <y> <terrain> <wall>` - Set tile
- `fill <terrain> <wall>` - Fill all tiles
- `event <x> <y> <type> <params>` - Add event
- `npc <id> <x> <y> <name> <dialogue>` - Add NPC
- `show` - Display map ASCII
- `info` - Show map summary
- `save <path>` - Save to RON file
- `help` - Show commands
- `quit` - Exit

**Implementation Structure**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

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
        let mut tiles = Vec::new();
        for _ in 0..height {
            let mut row = Vec::new();
            for _ in 0..width {
                row.push(Tile::new(TerrainType::Ground, WallType::None));
            }
            tiles.push(row);
        }

        self.map = Some(Map {
            id,
            width,
            height,
            tiles,
            events: HashMap::new(),
            npcs: Vec::new(),
        });

        println!("Created {}x{} map with ID {}", width, height, id);
    }

    fn load_map(&mut self, path: &str) -> Result<(), String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let map: Map = ron::from_str(&contents)
            .map_err(|e| format!("Failed to parse RON: {}", e))?;

        println!("Loaded map ID {} ({}x{})", map.id, map.width, map.height);
        self.map = Some(map);
        Ok(())
    }

    fn set_tile(&mut self, x: i32, y: i32, terrain: TerrainType, wall: WallType) -> Result<(), String> {
        let map = self.map.as_mut().ok_or("No map loaded")?;

        if x < 0 || x >= map.width as i32 || y < 0 || y >= map.height as i32 {
            return Err(format!("Position ({}, {}) out of bounds", x, y));
        }

        let tile = &mut map.tiles[y as usize][x as usize];
        tile.terrain = terrain;
        tile.wall_type = wall;
        tile.blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
            || matches!(wall, WallType::Normal);

        println!("Set tile at ({}, {}) to {:?}/{:?}", x, y, terrain, wall);
        Ok(())
    }

    fn fill_tiles(&mut self, terrain: TerrainType, wall: WallType) -> Result<(), String> {
        let map = self.map.as_mut().ok_or("No map loaded")?;

        for row in &mut map.tiles {
            for tile in row {
                tile.terrain = terrain;
                tile.wall_type = wall;
                tile.blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
                    || matches!(wall, WallType::Normal);
            }
        }

        println!("Filled all tiles with {:?}/{:?}", terrain, wall);
        Ok(())
    }

    fn add_event(&mut self, x: i32, y: i32, event: MapEvent) -> Result<(), String> {
        let map = self.map.as_mut().ok_or("No map loaded")?;

        if x < 0 || x >= map.width as i32 || y < 0 || y >= map.height as i32 {
            return Err(format!("Position ({}, {}) out of bounds", x, y));
        }

        let pos = Position::new(x, y);
        map.events.insert(pos, event);

        println!("Added event at ({}, {})", x, y);
        Ok(())
    }

    fn add_npc(&mut self, id: u16, x: i32, y: i32, name: String, dialogue: String) -> Result<(), String> {
        let map = self.map.as_mut().ok_or("No map loaded")?;

        if x < 0 || x >= map.width as i32 || y < 0 || y >= map.height as i32 {
            return Err(format!("Position ({}, {}) out of bounds", x, y));
        }

        let npc = Npc::new(id, name, Position::new(x, y), dialogue);
        map.npcs.push(npc);

        println!("Added NPC at ({}, {})", x, y);
        Ok(())
    }

    fn show_map(&self) {
        let map = match &self.map {
            Some(m) => m,
            None => {
                println!("No map loaded");
                return;
            }
        };

        println!("\n=== Map Display ({}x{}) ===", map.width, map.height);

        // Legend
        println!("Legend: . = Ground  : = Grass  ~ = Water  ^ = Mountain");
        println!("        # = Stone   T = Forest | = Wall   + = Door\n");

        for row in &map.tiles {
            for tile in row {
                let ch = match tile.wall_type {
                    WallType::Normal => '|',
                    WallType::Door => '+',
                    WallType::Torch => '*',
                    WallType::None => match tile.terrain {
                        TerrainType::Ground => '.',
                        TerrainType::Grass => ':',
                        TerrainType::Water => '~',
                        TerrainType::Mountain => '^',
                        TerrainType::Stone => '#',
                        TerrainType::Forest => 'T',
                        TerrainType::Lava => '%',
                        TerrainType::Swamp => '&',
                        TerrainType::Dirt => '-',
                    },
                };
                print!("{}", ch);
            }
            println!();
        }
        println!();
    }

    fn show_info(&self) {
        let map = match &self.map {
            Some(m) => m,
            None => {
                println!("No map loaded");
                return;
            }
        };

        println!("\n=== Map Info ===");
        println!("ID: {}", map.id);
        println!("Size: {}x{}", map.width, map.height);
        println!("Events: {}", map.events.len());
        println!("NPCs: {}", map.npcs.len());

        // Count event types
        let mut encounters = 0;
        let mut treasures = 0;
        let mut traps = 0;
        for event in map.events.values() {
            match event {
                MapEvent::Encounter { .. } => encounters += 1,
                MapEvent::Treasure { .. } => treasures += 1,
                MapEvent::Trap { .. } => traps += 1,
                _ => {}
            }
        }
        println!("  - Encounters: {}", encounters);
        println!("  - Treasures: {}", treasures);
        println!("  - Traps: {}", traps);
        println!();
    }

    fn save_map(&self, path: &str) -> Result<(), String> {
        let map = self.map.as_ref().ok_or("No map loaded")?;

        let ron_str = ron::ser::to_string_pretty(map, Default::default())
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        std::fs::write(path, ron_str)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        println!("Saved map to {}", path);
        Ok(())
    }

    fn process_command(&mut self, cmd: &str) -> Result<bool, String> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "quit" | "exit" => Ok(false),

            "help" => {
                println!("\nAvailable commands:");
                println!("  new <w> <h> <id>       - Create new map");
                println!("  load <path>            - Load map from file");
                println!("  set <x> <y> <t> <w>    - Set tile (terrain/wall)");
                println!("  fill <terrain> <wall>  - Fill all tiles");
                println!("  event <x> <y> <type>   - Add event (see docs)");
                println!("  npc <id> <x> <y> <n>   - Add NPC");
                println!("  show                   - Display ASCII map");
                println!("  info                   - Show map summary");
                println!("  save <path>            - Save to RON file");
                println!("  help                   - Show this help");
                println!("  quit                   - Exit\n");
                Ok(true)
            }

            "new" => {
                if parts.len() < 4 {
                    return Err("Usage: new <width> <height> <id>".to_string());
                }
                let width: u32 = parts[1].parse()
                    .map_err(|_| "Invalid width")?;
                let height: u32 = parts[2].parse()
                    .map_err(|_| "Invalid height")?;
                let id: u16 = parts[3].parse()
                    .map_err(|_| "Invalid id")?;
                self.create_map(width, height, id);
                Ok(true)
            }

            "load" => {
                if parts.len() < 2 {
                    return Err("Usage: load <path>".to_string());
                }
                self.load_map(parts[1])?;
                Ok(true)
            }

            "show" => {
                self.show_map();
                Ok(true)
            }

            "info" => {
                self.show_info();
                Ok(true)
            }

            "save" => {
                if parts.len() < 2 {
                    return Err("Usage: save <path>".to_string());
                }
                self.save_map(parts[1])?;
                Ok(true)
            }

            // TODO: Implement set, fill, event, npc commands
            // (Full implementation would be here)

            _ => Err(format!("Unknown command: {}", parts[0])),
        }
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

**Note**: This is an MVP implementation. Full command parsing (set, fill, event, npc) would be implemented in the actual code.

**Validation**:

- [ ] Binary compiles: `cargo build --bin map_builder`
- [ ] Can create new map
- [ ] Can load existing map
- [ ] Can display ASCII map
- [ ] Can show map info
- [ ] Can save to valid RON
- [ ] Help command lists all commands
- [ ] File in src/bin/ directory
- [ ] File uses .rs extension
- [ ] SPDX copyright header present

### Task 2.2: Map Builder Documentation

**Objective**: Document the map builder tool for users.

**File**: `docs/how_to/using_map_builder.md`

**Content**:

- Installation/running instructions
- Command reference with examples
- Workflow guide (create → edit → save)
- Tips and best practices
- Common patterns (room, corridor, etc.)
- Troubleshooting

**Example Section**:

````markdown
## Creating Your First Map

1. Run the map builder:
   ```bash
   cargo run --bin map_builder
   ```
````

2. Create a new 10x10 map:

   ```
   > new 10 10 1
   Created 10x10 map with ID 1
   ```

3. Fill with grass terrain:

   ```
   > fill Grass None
   Filled all tiles with Grass/None
   ```

4. View the map:

   ```
   > show
   ```

5. Add a sign:

   ```
   > event 5 5 Sign "Welcome to the test map!"
   ```

6. Save the map:

   ```
   > save data/maps/test_map.ron
   Saved map to data/maps/test_map.ron
   ```

7. Validate it:
   ```bash
   cargo run --bin validate_map data/maps/test_map.ron
   ```

## Monster and Item IDs

When adding encounters and treasure, use IDs from `docs/reference/data_dependencies.md`.

**Common Monster IDs**:

- 1 = Goblin
- 10 = Orc
- 12 = Wolf

**Common Item IDs**:

- 1 = Club
- 20 = Leather Armor
- 50 = Healing Potion

**Validation**:

- [ ] File created in docs/how_to/
- [ ] All commands documented with examples
- [ ] Workflow guide complete
- [ ] References data_dependencies.md for IDs
- [ ] File uses lowercase_with_underscores.md

### Phase 2 Completion Checklist

**Map Builder Tool**:

- [ ] `src/bin/map_builder.rs` created (MVP)
- [ ] Basic commands work (new, load, show, info, save)
- [ ] ASCII display functional
- [ ] File uses .rs extension
- [ ] SPDX copyright header present

**Documentation**:

- [ ] `docs/how_to/using_map_builder.md` created
- [ ] Commands documented with examples
- [ ] Workflow guide included
- [ ] References data_dependencies.md
- [ ] File uses lowercase_with_underscores.md

**Testing**:

- [ ] Tool builds: `cargo build --bin map_builder`
- [ ] Can create simple map
- [ ] Generated RON files are valid
- [ ] Validation tool accepts generated maps

**Quality Gates**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo build --bin map_builder` succeeds

**Documentation Update**:

- [ ] Updated `docs/explanation/implementations.md` with Phase 2 summary

---

## Phase 3: Content Creation

### Goals

1. Create starter town map using map builder
2. Create starter dungeon with encounters and treasure
3. Create outdoor forest area
4. Document world layout and connections
5. Use CORRECTED Monster/Item IDs throughout

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.2: World (Map, Tile, MapEvent)
- Section 5.4: Map and Movement (gameplay context)
- `docs/reference/data_dependencies.md` (REQUIRED - Monster/Item IDs)

### Task 3.1: Starter Town Map

**Objective**: Create a small town as the starting area.

**File**: `data/maps/starter_town.ron`

**Design Specifications**:

- Size: 16x16 tiles
- Map ID: 1
- Layout: Simple town with:
  - Town entrance (south)
  - Central plaza with sign
  - 2-3 buildings with doors
  - NPC merchant
  - Small treasure chest (starter items)
  - Safe area (no encounters)

**Implementation Steps**:

1. Use map builder to create base:

   ```bash
   cargo run --bin map_builder
   > new 16 16 1
   > fill Ground None
   > save data/maps/starter_town.ron
   ```

2. Edit RON file or use builder to add:

   - Border: Mountain terrain (blocked)
   - Buildings: Stone walls with Doors
   - Paths: Dirt terrain
   - Grass: Decorative areas

3. Add Sign event at plaza center (8, 8):

   ```ron
   Position(x: 8, y: 8): Sign(
       text: "Welcome to Sorpigal! Visit the merchant for supplies.",
   ),
   ```

4. Add NPC merchant at (5, 5):

   ```ron
   Npc(
       id: 1,
       name: "Marcus the Merchant",
       position: Position(x: 5, y: 5),
       dialogue: "Welcome, traveler! I have weapons, armor, and supplies.",
   ),
   ```

5. Add NpcDialogue event:

   ```ron
   Position(x: 5, y: 5): NpcDialogue(
       npc_id: 1,
   ),
   ```

6. Add starter Treasure at (12, 12):

   ```ron
   Position(x: 12, y: 12): Treasure(
       loot: [1, 2, 20], // Club, Dagger, Leather Armor
   ),
   ```

7. Add exit teleport to dungeon at (14, 14):

   ```ron
   Position(x: 14, y: 14): Teleport(
       destination: Position(x: 1, y: 1),
       map_id: 2,
   ),
   ```

8. Validate:
   ```bash
   cargo run --bin validate_map data/maps/starter_town.ron
   ```

**Testing**:

Add to `tests/map_loading_test.rs`:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::world::Map;
use antares::domain::types::Position;
use std::fs;

#[test]
fn test_load_starter_town() {
    let contents = fs::read_to_string("data/maps/starter_town.ron")
        .expect("Failed to read starter_town.ron");

    let map: Map = ron::from_str(&contents)
        .expect("Failed to parse starter_town.ron");

    assert_eq!(map.id, 1);
    assert_eq!(map.width, 16);
    assert_eq!(map.height, 16);
    assert_eq!(map.tiles.len(), 16);
    assert_eq!(map.tiles[0].len(), 16);

    assert!(map.events.len() > 0, "Map should have events");
    assert_eq!(map.npcs.len(), 1, "Should have merchant NPC");
    assert_eq!(map.npcs[0].id, 1);
}
```

**Validation**:

- [ ] Map file created and parses without errors
- [ ] Map dimensions correct (16x16)
- [ ] Sign event present
- [ ] Merchant NPC present
- [ ] Treasure event present
- [ ] Teleport to dungeon present
- [ ] Tests pass
- [ ] File uses .ron extension

### Task 3.2: Starter Dungeon

**Objective**: Create a small dungeon with encounters and treasure.

**File**: `data/maps/starter_dungeon.ron`

**Design Specifications**:

- Size: 20x20 tiles
- Map ID: 2
- Layout: Small dungeon with 3-4 rooms
- Features: Encounters, treasure, traps, dark areas

**Implementation Steps**:

1. Create base map with builder

2. Add encounters with CORRECTED Monster IDs:

   ```ron
   // Goblin patrol
   Position(x: 10, y: 10): Encounter(
       monster_group: [1, 1], // 2 Goblins (ID 1, not 2!)
   ),

   // Kobold ambush
   Position(x: 8, y: 12): Encounter(
       monster_group: [2, 2, 2], // 3 Kobolds (ID 2)
   ),

   // Orc guard
   Position(x: 15, y: 5): Encounter(
       monster_group: [10], // 1 Orc (ID 10, NOT 2!)
   ),

   // Skeleton warriors
   Position(x: 18, y: 18): Encounter(
       monster_group: [11, 11], // 2 Skeletons (ID 11)
   ),
   ```

3. Add treasure with appropriate IDs:

   ```ron
   // Better equipment
   Position(x: 18, y: 18): Treasure(
       loot: [3, 21, 50], // Short Sword, Chain Mail, Healing Potion
   ),

   // Magic item
   Position(x: 5, y: 15): Treasure(
       loot: [10, 52], // Club +1, Cure Poison Potion
   ),
   ```

4. Add traps:

   ```ron
   Position(x: 8, y: 8): Trap(
       damage: 15,
       effect: Some("poison"),
   ),

   Position(x: 12, y: 14): Trap(
       damage: 10,
       effect: None,
   ),
   ```

5. Add exit teleport back to town:

   ```ron
   Position(x: 1, y: 1): Teleport(
       destination: Position(x: 14, y: 14),
       map_id: 1,
   ),
   ```

6. Set dark tiles (is_dark: true) in areas without torches

7. Validate:
   ```bash
   cargo run --bin validate_map data/maps/starter_dungeon.ron
   ```

**Testing**:

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
        .filter(|e| matches!(e, antares::domain::world::MapEvent::Encounter { .. }))
        .count();
    assert!(encounter_count >= 3, "Should have at least 3 encounters");
}
```

**Validation**:

- [ ] Map file created and parses without errors
- [ ] Map dimensions correct (20x20)
- [ ] Encounters use CORRECT Monster IDs (1, 2, 10, 11)
- [ ] Treasure uses valid Item IDs
- [ ] Traps present
- [ ] Teleport back to town present
- [ ] Dark areas marked
- [ ] Tests pass
- [ ] File uses .ron extension

### Task 3.3: Forest Area

**Objective**: Create an outdoor wilderness map.

**File**: `data/maps/forest_area.ron`

**Design Specifications**:

- Size: 30x30 tiles (larger outdoor space)
- Map ID: 3
- Layout: Forest with clearings
- Features: Outdoor encounters, hidden treasure

**Implementation Steps**:

1. Create base with varied terrain

2. Add encounters with CORRECTED Monster IDs:

   ```ron
   // Wolf pack
   Position(x: 5, y: 5): Encounter(
       monster_group: [12, 12], // 2 Wolves (ID 12, NOT 3!)
   ),

   // Orc hunter
   Position(x: 15, y: 10): Encounter(
       monster_group: [10], // 1 Orc (ID 10, NOT "Bear" which doesn't exist!)
   ),

   // Goblin band
   Position(x: 20, y: 20): Encounter(
       monster_group: [1, 1, 1], // 3 Goblins (ID 1)
   ),

   // Mixed encounter
   Position(x: 25, y: 15): Encounter(
       monster_group: [12, 10], // Wolf + Orc (IDs 12, 10)
   ),

   // Giant Rats
   Position(x: 10, y: 18): Encounter(
       monster_group: [3, 3, 3], // 3 Giant Rats (ID 3 IS Giant Rat, not Wolf)
   ),
   ```

3. Add treasure:

   ```ron
   Position(x: 25, y: 25): Treasure(
       loot: [4, 40, 50], // Long Sword, Ring of Protection, Healing Potion
   ),
   ```

4. Add sign:

   ```ron
   Position(x: 2, y: 2): Sign(
       text: "Beware! Wild creatures roam these woods.",
   ),
   ```

5. Add teleport to/from town:

   ```ron
   Position(x: 1, y: 15): Teleport(
       destination: Position(x: 8, y: 1),
       map_id: 1,
   ),
   ```

6. Validate:
   ```bash
   cargo run --bin validate_map data/maps/forest_area.ron
   ```

**Testing**:

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

    let encounter_count = map.events.values()
        .filter(|e| matches!(e, antares::domain::world::MapEvent::Encounter { .. }))
        .count();
    assert!(encounter_count >= 5, "Outdoor area should have many encounters");
}
```

**Validation**:

- [ ] Map file created and parses without errors
- [ ] Map dimensions correct (30x30)
- [ ] Encounters use CORRECT Monster IDs (1, 3, 10, 12)
- [ ] NO reference to non-existent Bear (ID 4)
- [ ] Treasure uses valid Item IDs
- [ ] Sign present
- [ ] Teleport links to town
- [ ] Tests pass
- [ ] File uses .ron extension

### Task 3.4: World Layout Documentation

**Objective**: Document how all maps connect.

**File**: `docs/explanation/world_layout.md`

**Content**:

````markdown
# World Layout

## Map Connections

```text
┌─────────────────┐
│  Starter Town   │
│    (Map 1)      │
│   16x16 tiles   │
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
│  20x20      │  │   30x30      │
│             │  │              │
│ - Goblins   │  │ - Wolves     │
│ - Kobolds   │  │ - Orcs       │
│ - Orc       │  │ - Giant Rats │
│ - Skeletons │  │ - Goblins    │
│ - Treasure  │  │ - Treasure   │
│ - Traps     │  │ - Sign       │
└─────────────┘  └──────────────┘
```
````

## Map Details

### Map 1: Starter Town

- **Size**: 16x16
- **Safe**: Yes (no encounters)
- **Events**: 3 (Sign, NPC, Treasure)
- **NPCs**: 1 (Merchant)
- **Connections**: Dungeon (map 2), Forest (map 3)
- **Recommended Level**: 1

### Map 2: Starter Dungeon

- **Size**: 20x20
- **Safe**: No
- **Encounters**: 4 (Goblins, Kobolds, Orc, Skeletons)
- **Treasure**: 2 chests
- **Traps**: 2
- **Connections**: Town (map 1)
- **Recommended Level**: 1-3

### Map 3: Forest Area

- **Size**: 30x30
- **Safe**: No
- **Encounters**: 5+ (Wolves, Orcs, Goblins, Giant Rats)
- **Treasure**: 1 hidden chest
- **Connections**: Town (map 1)
- **Recommended Level**: 2-4

## Event Summary

| Map ID | Name         | Size  | Events | NPCs | Encounters | Treasure | Traps |
| ------ | ------------ | ----- | ------ | ---- | ---------- | -------- | ----- |
| 1      | Starter Town | 16x16 | 3      | 1    | 0          | 1        | 0     |
| 2      | Dungeon      | 20x20 | 7      | 0    | 4          | 2        | 2     |
| 3      | Forest Area  | 30x30 | 7      | 0    | 5          | 1        | 0     |

## Monster Distribution

### Weak Monsters (Starter Dungeon)

- Goblin (ID 1): 2 encounters
- Kobold (ID 2): 1 encounter
- Orc (ID 10): 1 encounter
- Skeleton (ID 11): 1 encounter

### Mid-Level Monsters (Forest)

- Goblin (ID 1): 1 encounter
- Giant Rat (ID 3): 1 encounter
- Orc (ID 10): 2 encounters
- Wolf (ID 12): 2 encounters

## Treasure Distribution

### Starter Town

- Basic equipment: Club, Dagger, Leather Armor

### Starter Dungeon

- Mid-tier gear: Short Sword, Chain Mail
- Magic item: Club +1
- Consumables: Healing Potion, Cure Poison

### Forest Area

- Better gear: Long Sword
- Magic accessories: Ring of Protection
- Consumables: Healing Potion

````

**Validation**:
- [ ] File created in docs/explanation/
- [ ] All maps documented
- [ ] Connection diagram included
- [ ] Event summary table complete
- [ ] Monster IDs are CORRECT (no wrong references)
- [ ] File uses lowercase_with_underscores.md

### Phase 3 Completion Checklist

**Map Content**:
- [ ] `data/maps/starter_town.ron` created (16x16)
- [ ] `data/maps/starter_dungeon.ron` created (20x20)
- [ ] `data/maps/forest_area.ron` created (30x30)
- [ ] All maps parse without errors
- [ ] All maps use CORRECT Monster IDs
- [ ] All maps use valid Item IDs
- [ ] Teleports link maps correctly
- [ ] Files use .ron extension

**Testing**:
- [ ] `tests/map_loading_test.rs` created
- [ ] Tests for all 3 maps
- [ ] All tests pass
- [ ] `cargo test map_loading` succeeds

**Documentation**:
- [ ] `docs/explanation/world_layout.md` created
- [ ] All maps documented
- [ ] Connection diagram included
- [ ] Monster distribution documented with CORRECT IDs
- [ ] File uses lowercase_with_underscores.md

**Quality Gates**:
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all-features` passes
- [ ] All maps validate: `cargo run --bin validate_map data/maps/*.ron`

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
````

**Verify Map Files** (Phase 3):

```bash
cargo run --bin validate_map data/maps/starter_town.ron
cargo run --bin validate_map data/maps/starter_dungeon.ron
cargo run --bin validate_map data/maps/forest_area.ron
```

**Test Map Loading**:

```bash
cargo test map_loading
```

### Architecture Compliance Checklist

- [ ] All Map structures match architecture.md Section 4.2
- [ ] All Tile fields present and correct
- [ ] All MapEvent types used correctly
- [ ] Position coordinates correct (0-based, top-left origin, i32 type)
- [ ] Type aliases understood (MapId, EventId, MonsterId, ItemId)
- [ ] RON format used for all data files (.ron extension)
- [ ] No JSON or YAML for map data
- [ ] TerrainType and WallType variants from architecture
- [ ] Event types match architecture enum
- [ ] **Monster IDs verified against data_dependencies.md**
- [ ] **Item IDs verified against data_dependencies.md**

### Documentation Checklist

- [ ] `docs/reference/map_format.md` - Complete RON reference
- [ ] `docs/reference/data_dependencies.md` - Monster/Item ID reference (already exists)
- [ ] `docs/how_to/map_templates.md` - Reusable templates
- [ ] `docs/how_to/using_map_builder.md` - Tool guide
- [ ] `docs/explanation/world_layout.md` - Map connections
- [ ] All files use lowercase_with_underscores.md naming
- [ ] Proper Diataxis categories (reference, how-to, explanation)
- [ ] SPDX copyright headers on all .rs files

### File Structure Checklist

- [ ] `data/maps/starter_town.ron` - 16x16 town map
- [ ] `data/maps/starter_dungeon.ron` - 20x20 dungeon map
- [ ] `data/maps/forest_area.ron` - 30x30 outdoor map
- [ ] `src/bin/validate_map.rs` - Validation tool
- [ ] `src/bin/map_builder.rs` - Map builder binary
- [ ] `tests/map_loading_test.rs` - Map loading tests
- [ ] All .rs files have SPDX copyright headers
- [ ] All data files use .ron extension

### Testing Checklist

- [ ] Map parsing tests pass
- [ ] Map validation tests pass
- [ ] Monster ID validation works
- [ ] Item ID validation works
- [ ] Event type tests pass
- [ ] NPC tests pass
- [ ] Teleport linking tests pass
- [ ] All tests run: `cargo test --all-features`
- [ ] Test coverage >80% for new code

---

## Summary

This plan provides a complete path from zero map content to a fully functional world with:

1. **Phase 1**: Documentation, validation utility, templates
2. **Phase 2**: Interactive map builder tool
3. **Phase 3**: Three diverse, playable maps

**Key Improvements in v2**:

- ✅ Corrected Monster IDs (Orc = 10, Wolf = 12, etc.)
- ✅ Removed non-existent monsters (Bear)
- ✅ Corrected Item ID usage
- ✅ Revised phase ordering (tools before manual maps)
- ✅ Standardized on `src/bin/` for tools
- ✅ Added coordinate system documentation
- ✅
