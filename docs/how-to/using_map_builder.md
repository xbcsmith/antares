# How to Use the Map Builder Tool

This guide explains how to use the interactive Map Builder tool to create and edit maps for Antares RPG.

## Overview

The Map Builder is a command-line tool that provides an interactive REPL interface for creating, editing, and visualizing game maps. It supports:

- Creating new maps with custom dimensions
- Loading and editing existing RON map files
- Setting individual tiles or filling regions
- Adding events (encounters, treasures, signs, traps)
- Adding NPCs with dialogue
- Real-time ASCII visualization
- Saving maps in RON format

## Prerequisites

Ensure you have Rust and Cargo installed, then build the project:

```bash
cargo build --bin map_builder
```

## Starting the Map Builder

Run the tool from the project root:

```bash
cargo run --bin map_builder
```

You'll see the welcome screen:

```
╔═══════════════════════════════════════╗
║   Antares RPG - Map Builder v1.0     ║
╚═══════════════════════════════════════╝

Type 'help' for available commands.

>
```

## Quick Start: Creating Your First Map

### Step 1: Create a New Map

Create a 20x20 map with ID 1:

```
> new 1 20 20
✅ Created 20x20 map with ID 1
```

### Step 2: Add Border Walls

Fill the top edge with walls:

```
> fill 0 0 19 0 ground normal
✅ Filled 20 tiles from (0,0) to (19,0) with Ground/Normal
```

Repeat for other edges:

```
> fill 0 19 19 19 ground normal
> fill 0 0 0 19 ground normal
> fill 19 0 19 19 ground normal
```

### Step 3: Add Some Terrain

Add a water feature:

```
> fill 8 8 11 11 water none
✅ Filled 16 tiles from (8,8) to (11,11) with Water/None
```

### Step 4: Add a Door

```
> set 10 0 ground door
✅ Set tile at (10, 0) to Ground/Door
```

### Step 5: Add an NPC

```
> npc 1 5 5 Guard Welcome to the town!
✅ Added NPC 'Guard' (ID: 1) at (5, 5)
```

### Step 6: Add a Treasure Event

```
> event 15 15 treasure 10 11 12
✅ Added event at (15, 15)
```

### Step 7: View Your Map

```
> show
```

### Step 8: Save the Map

```
> save data/maps/my_first_map.ron
✅ Saved map to data/maps/my_first_map.ron
```

## Command Reference

### Map Management

#### new <id> <width> <height>

Creates a new map with the specified dimensions.

```
> new 1 30 30
✅ Created 30x30 map with ID 1
```

**Parameters:**

- `id`: Map identifier (MapId)
- `width`: Map width in tiles (1-255 recommended)
- `height`: Map height in tiles (1-255 recommended)

#### load <path>

Loads an existing map from a RON file.

```
> load data/maps/starter_town.ron
✅ Loaded map 1 (20x20) with 3 NPCs and 5 events
```

#### save <path>

Saves the current map to a RON file.

```
> save data/maps/my_map.ron
✅ Saved map to data/maps/my_map.ron
```

### Tile Editing

#### set <x> <y> <terrain> [wall]

Sets a single tile at the specified coordinates.

```
> set 5 5 grass none
✅ Set tile at (5, 5) to Grass/None

> set 10 10 ground door
✅ Set tile at (10, 10) to Ground/Door
```

**Parameters:**

- `x`, `y`: Tile coordinates
- `terrain`: Terrain type (see Terrain Types below)
- `wall`: Optional wall type (see Wall Types below)

#### fill <x1> <y1> <x2> <y2> <terrain> [wall]

Fills a rectangular region with the specified tile type.

```
> fill 0 0 19 0 ground normal
✅ Filled 20 tiles from (0,0) to (19,0) with Ground/Normal
```

**Parameters:**

- `x1`, `y1`: Starting corner coordinates
- `x2`, `y2`: Ending corner coordinates
- `terrain`: Terrain type
- `wall`: Optional wall type

**Note:** The coordinates are automatically sorted, so (10,10) to (5,5) works the same as (5,5) to (10,10).

#### bulk <terrain_csv> <wall> <blocked>

Bulk-updates tiles across the entire map whose current terrain is in the comma-separated list. This command sets the `wall_type` and `blocked` flag on matching tiles without changing their terrain.

```
> bulk Ground,Grass,Forest None false
✅ Bulk updated 134 tiles (terrains: Ground,Grass,Forest, wall: None, blocked: false)
```

**Parameters:**

- `terrain_csv`: Comma-separated list of terrain names (e.g., `Ground,Grass,Forest`). Case-insensitive; surrounding spaces are ignored.
- `wall`: Wall type to apply to matching tiles (`none`, `normal`, `door`, `torch`)
- `blocked`: Movement blocking flag (`true` or `false`)

**Notes:**

- The `bulk` command operates over the entire map (not a rectangle). If you want to limit changes to an area, use `fill` or `set` first to narrow the region.
- Use `show` or `info` after running `bulk` to verify changes visually.
- Example for your earlier request:
  ```
  > bulk Ground,Dirt,Swamp,Forest,Grass None false
  ```
  This will set `wall_type` to `None` and `blocked` to `false` for all tiles with those terrains.

### Events

#### event <x> <y> <type> <data>

Adds an event at the specified position.

**Event Types:**

##### encounter <monster_ids...>

Creates a monster encounter event.

```
> event 10 10 encounter 1 2 3
✅ Added event at (10, 10)
```

Monster IDs are space-separated.

##### treasure <item_ids...>

Creates a treasure chest event.

```
> event 15 15 treasure 10 11 12
✅ Added event at (15, 15)
```

Item IDs are space-separated.

##### sign <text...>

Creates a sign with text.

```
> event 5 5 sign Welcome to the village!
✅ Added event at (5, 5)
```

##### trap <damage> [effect]

Creates a trap event.

```
> event 8 8 trap 20 poison
✅ Added event at (8, 8)
```

### NPCs

#### npc <id> <x> <y> <name> <dialogue...>

Adds an NPC at the specified position.

```
> npc 1 10 10 Merchant Buy something from my shop!
✅ Added NPC 'Merchant' (ID: 1) at (10, 10)
```

**Parameters:**

- `id`: NPC identifier (must be unique)
- `x`, `y`: NPC position
- `name`: NPC name (no spaces)
- `dialogue`: Dialogue text (can contain spaces)

**Warning:** If you add an NPC with a duplicate ID, the tool will show a warning but still add it.

### Viewing

#### show

Displays the current map as ASCII art.

```
> show

╔═══ Map 1 (20x20) ═══╗
  01234567890123456789
 0###################
 1#.................#
 2#.................#
 3#.................#
 4#.................#
 5#....@............#
 6#.................#
...

Legend:
  # = Wall    + = Door    * = Torch
  . = Ground  , = Grass   ~ = Water  ^ = Lava
  % = Swamp   ░ = Stone   : = Dirt   ♣ = Forest  ▲ = Mountain
  ! = Event   @ = NPC
```

#### info

Shows detailed map information including NPCs and events.

```
> info

╔═══ Map Information ═══╗
Map ID:     1
Dimensions: 20x20 (400 tiles)
NPCs:       2
Events:     3

NPCs:
  - ID 1: Guard at (5, 5)
  - ID 2: Merchant at (10, 10)

Events:
  - Treasure at (15, 15): Treasure
  - Sign at (5, 5): Sign
  - Encounter at (12, 8): Encounter
```

### Other Commands

#### help

Shows the command reference.

#### quit

Exits the Map Builder (also accepts `exit`).

## Terrain Types

Available terrain types:

- `ground` - Normal walkable ground (ASCII: `.`)
- `grass` - Grass terrain (ASCII: `,`)
- `water` - Water (blocks movement) (ASCII: `~`)
- `lava` - Lava (damages party) (ASCII: `^`)
- `swamp` - Swamp (slows movement) (ASCII: `%`)
- `stone` - Stone floor (ASCII: `░`)
- `dirt` - Dirt path (ASCII: `:`)
- `forest` - Forest (ASCII: `♣`)
- `mountain` - Mountain (blocks movement) (ASCII: `▲`)

**Terrain names are case-insensitive.**

## Wall Types

Available wall types:

- `none` - No wall (default)
- `normal` - Solid wall (blocks movement) (ASCII: `#`)
- `door` - Door (can be opened) (ASCII: `+`)
- `torch` - Torch (light source) (ASCII: `*`)

**Wall names are case-insensitive.**

## Common Workflows

### Creating a Town Map

1. Create map with appropriate size
2. Add border walls with fill command
3. Add building outlines using walls
4. Add doors for entrances
5. Add NPCs (merchants, guards, quest givers)
6. Add signs with town information
7. Save the map

Example session:

```
> new 1 30 30
> fill 0 0 29 0 ground normal
> fill 0 29 29 29 ground normal
> fill 0 0 0 29 ground normal
> fill 29 0 29 29 ground normal
> set 15 0 ground door
> npc 1 15 15 Mayor Welcome to our town!
> event 10 10 sign Town Square
> save data/maps/town.ron
```

### Creating a Dungeon Map

1. Create map
2. Add walls to create corridors and rooms
3. Add doors between rooms
4. Add encounter events for monsters
5. Add treasure chests
6. Add traps
7. Save the map

Example session:

```
> new 2 20 20
> fill 0 0 19 19 stone normal
> fill 5 5 14 14 stone none
> set 9 5 stone door
> event 10 10 encounter 5 6
> event 12 8 treasure 20 21
> event 7 7 trap 15 poison
> save data/maps/dungeon.ron
```

### Editing an Existing Map

1. Load the map file
2. View current state with `show` and `info`
3. Make changes
4. Save (optionally to a different file for backup)

Example session:

```
> load data/maps/starter_town.ron
✅ Loaded map 1 (20x20) with 3 NPCs and 5 events
> info
> show
> npc 4 18 18 Blacksmith I can forge weapons for you!
> save data/maps/starter_town.ron
```

## Tips and Best Practices

### Map Design

1. **Always create border walls** - Use the fill command to quickly add walls around the perimeter
2. **Leave entry/exit points** - Add doors or gaps in walls for navigation
3. **Use terrain strategically** - Water and mountains can guide player movement
4. **Balance encounters** - Don't overwhelm new players; spread encounters appropriately
5. **Add visual variety** - Mix terrain types to make maps interesting

### Using the Tool

1. **Work incrementally** - Save your work frequently using different filenames
2. **Use info command** - Check NPC IDs and event counts to avoid duplicates
3. **Visualize often** - Use `show` command regularly to see your progress
4. **Test coordinates** - The tool validates positions and shows errors immediately
5. **Use fill for efficiency** - Fill large regions, then set individual tiles for details

### Coordinate System

- Origin (0,0) is the **top-left** corner
- X increases to the right
- Y increases downward
- Valid coordinates: 0 to (width-1), 0 to (height-1)

### Validation

The tool performs basic validation:

- Position bounds checking
- Duplicate NPC ID warnings
- Invalid dimensions (width/height = 0)
- Large map warnings (>255 tiles)

For comprehensive validation, use the `validate_map` tool after saving:

```bash
cargo run --bin validate_map -- data/maps/my_map.ron
```

## Monster and Item IDs

When creating events, you'll need to know valid Monster IDs and Item IDs. Refer to:

- `data/monsters.ron` for monster definitions
- `data/items.ron` for item definitions
- `docs/reference/map_ron_format.md` for ID reference lists

Common Monster IDs:

- 1: Rat
- 2: Goblin
- 3: Wolf
- 4: Skeleton
- 5: Orc

Common Item IDs:

- 10: Short Sword
- 11: Long Sword
- 20: Leather Armor
- 30: Health Potion
- 40: Gold (small)

## Troubleshooting

### "No map loaded" Error

You see: `❌ Error: No map loaded. Use 'new' or 'load' first.`

**Solution:** You must create a new map with `new` or load an existing map with `load` before using editing commands.

### "Position out of bounds" Error

You see: `❌ Error: Position (25, 25) is out of bounds`

**Solution:** Check your map dimensions with `info` and ensure coordinates are within 0 to (width-1) and 0 to (height-1).

### Duplicate NPC ID Warning

You see: `⚠️  Warning: NPC with ID 5 already exists`

**Solution:** Choose a unique ID for each NPC. Use `info` to see existing NPC IDs.

### Map Won't Load

You see: `❌ Error: Failed to parse RON: ...`

**Solution:**

1. Check that the file is valid RON format
2. Run `validate_map` on the file to identify syntax errors
3. Ensure file exists at the specified path

## Next Steps

After creating your maps:

1. **Validate** - Run `validate_map` to check for errors:

   ```bash
   cargo run --bin validate_map -- data/maps/your_map.ron
   ```

2. **Test in-game** - Load the map in the game and playtest it

3. **Iterate** - Load, edit, save, and repeat based on playtesting

4. **Document** - Add map details to `docs/explanation/world_layout.md`

## See Also

- [Map RON Format Reference](../reference/map_ron_format.md) - Detailed RON format specification
- [Creating Maps Guide](creating_maps.md) - Map templates and design patterns
- [Architecture Reference](../reference/architecture.md) - Section 4.2 (World structures)
- [Map Content Implementation Plan](../explanation/map_content_implementation_plan_v2.md) - Overall implementation plan
