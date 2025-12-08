# How to Create Maps for Antares RPG

This guide walks you through creating custom maps for Antares RPG using the RON format.

## Prerequisites

Before creating maps, you should:

1. Read [`docs/reference/map_ron_format.md`](../reference/map_ron_format.md) - Complete format specification
2. Have a text editor that supports RON syntax (VS Code with Rust extension recommended)
3. Understand the coordinate system (0,0 = top-left corner)
4. Familiarity with the item, monster, and character definition systems

## Quick Start: Your First Map

### Step 1: Create the Map File

Create a new file in `data/maps/` with a descriptive name:

```text
data/maps/my_first_map.ron
```

### Step 2: Start with a Minimal Template

Copy this minimal valid map:

```ron
(
    id: 100,
    name: "My First Map",
    map_type: Town,
    width: 10,
    height: 10,
    outdoor: true,
    allow_resting: true,
    danger_level: 0,
    tiles: [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ],
    events: [],
    npcs: [],
    exits: [],
)
```

### Step 3: Validate the Map

Run the validation tool:

```bash
cargo run --bin validate_map data/maps/my_first_map.ron
```

If successful, you'll see:

```text
✅ VALID

Map Summary:
  ID: 100
  Name: My First Map
  Type: Town
  Size: 10x10
  Events: 0
  NPCs: 0
  Exits: 0
```

## Map Design Workflow

### 1. Plan Your Map

Before writing RON code, sketch your map:

- **Purpose**: What is this map for? (Town, dungeon, wilderness)
- **Size**: How big? (Start small: 16x16 or 20x20)
- **Key Locations**: Where are shops, encounters, exits?
- **Connectivity**: How does this connect to other maps?

### 2. Choose Map Type

Select the appropriate `map_type`:

- **Town**: Safe areas with NPCs, shops, trainers

  - `outdoor: true`
  - `allow_resting: true`
  - `danger_level: 0`

- **Dungeon**: Indoor combat areas with treasure and monsters

  - `outdoor: false`
  - `allow_resting: false`
  - `danger_level: 3-7`

- **Outdoor**: Wilderness areas between towns
  - `outdoor: true`
  - `allow_resting: false` (usually)
  - `danger_level: 2-6`

### 3. Design Tile Layout

Use a grid to plan your tiles:

```text
Tile Types:
0 = Floor (walkable)
1 = Wall (impassable)
2 = Door
3 = Water
4 = Lava
5 = Forest
6 = Mountain
7 = Stairs Up
8 = Stairs Down
```

**Tips:**

- Always use walls (1) for the outer border
- Create paths with floors (0)
- Use doors (2) for room entrances
- Leave space for events and NPCs

**Example: Simple Room**

```text
1 1 1 1 1
1 0 0 0 1
1 0 0 0 1
1 0 2 0 1
1 1 1 1 1
```

Converts to:

```ron
tiles: [
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 2, 0, 1],
    [1, 1, 1, 1, 1],
],
```

### 4. Add Events

Events trigger when the party steps on a tile.

#### Treasure Event

Place a treasure chest:

```ron
events: [
    (
        position: (x: 5, y: 5),
        event_type: Treasure((
            gold: 100,
            gems: 5,
            items: [50],  // Item ID 50 (Healing Potion)
        )),
        repeatable: false,
        triggered: false,
    ),
],
```

#### Combat Encounter

Create a monster fight:

```ron
events: [
    (
        position: (x: 8, y: 8),
        event_type: Combat((
            monster_ids: [1, 1, 2],  // 2 Goblins + 1 Orc
            ambush: false,
        )),
        repeatable: true,  // Respawns when re-entering map
        triggered: false,
    ),
],
```

#### Text Message

Display narrative text:

```ron
events: [
    (
        position: (x: 3, y: 3),
        event_type: Text((
            message: "You see ancient runes carved into the wall.",
        )),
        repeatable: true,
        triggered: false,
    ),
],
```

#### Healing Fountain

Create a rest area:

```ron
events: [
    (
        position: (x: 10, y: 10),
        event_type: Healing((
            hp_restore: 0,    // 0 = full heal
            sp_restore: 0,    // 0 = full restore
            cure_conditions: true,
        )),
        repeatable: true,
        triggered: false,
    ),
],
```

### 5. Add NPCs

Place interactive characters:

```ron
npcs: [
    (
        id: 1,
        name: "Blacksmith",
        position: (x: 5, y: 5),
        dialogue_id: 201,
        shop_id: Some(10),  // Sells weapons/armor
    ),
    (
        id: 2,
        name: "Town Guard",
        position: (x: 10, y: 5),
        dialogue_id: 202,
        shop_id: None,  // Not a merchant
    ),
],
```

**NPC Placement Tips:**

- Don't place on walls or doors
- Leave space around NPCs for player movement
- Unique IDs per map (can reuse IDs across different maps)

### 6. Add Exits

Connect maps together:

```ron
exits: [
    (
        position: (x: 10, y: 0),      // Exit on north edge
        destination_map: 2,           // Goes to Map ID 2
        destination_pos: (x: 10, y: 19),  // Spawn at south of Map 2
        direction: Some(North),
    ),
    (
        position: (x: 10, y: 19),     // Exit on south edge
        destination_map: 3,
        destination_pos: (x: 10, y: 1),
        direction: Some(South),
    ),
],
```

**Exit Guidelines:**

- Place exits on map edges
- Ensure destination position is walkable
- Use appropriate direction for player facing

## Common Map Templates

### Template 1: Safe Town (20x20)

```ron
(
    id: 1,
    name: "Starter Town",
    map_type: Town,
    width: 20,
    height: 20,
    outdoor: true,
    allow_resting: true,
    danger_level: 0,
    tiles: [
        // Border walls, open interior
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        // ... (18 rows total)
    ],
    events: [
        // Healing fountain at center
        (
            position: (x: 10, y: 10),
            event_type: Healing((hp_restore: 0, sp_restore: 0, cure_conditions: true)),
            repeatable: true,
            triggered: false,
        ),
    ],
    npcs: [
        (id: 1, name: "Innkeeper", position: (x: 5, y: 5), dialogue_id: 101, shop_id: None),
        (id: 2, name: "Blacksmith", position: (x: 15, y: 5), dialogue_id: 102, shop_id: Some(10)),
    ],
    exits: [
        (position: (x: 10, y: 19), destination_map: 2, destination_pos: (x: 10, y: 1), direction: Some(South)),
    ],
)
```

### Template 2: Small Dungeon (16x16)

```ron
(
    id: 2,
    name: "Dark Cave",
    map_type: Dungeon,
    width: 16,
    height: 16,
    outdoor: false,
    allow_resting: false,
    danger_level: 3,
    tiles: [
        // Maze-like corridors with rooms
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1],
        // ... (corridors and rooms)
    ],
    events: [
        // Combat encounters
        (
            position: (x: 5, y: 5),
            event_type: Combat((monster_ids: [1, 1, 2], ambush: false)),
            repeatable: true,
            triggered: false,
        ),
        // Treasure chest
        (
            position: (x: 14, y: 14),
            event_type: Treasure((gold: 500, gems: 10, items: [3, 50])),
            repeatable: false,
            triggered: false,
        ),
    ],
    npcs: [],
    exits: [
        (position: (x: 1, y: 1), destination_map: 1, destination_pos: (x: 10, y: 18), direction: Some(North)),
    ],
)
```

### Template 3: Wilderness Area (32x32)

```ron
(
    id: 3,
    name: "Darkwood Forest",
    map_type: Outdoor,
    width: 32,
    height: 32,
    outdoor: true,
    allow_resting: false,
    danger_level: 5,
    tiles: [
        // Large open area with forests (5) scattered
        [1, 1, 1, 1, 1, 1, 1, 1, /* ... 32 tiles ... */],
        [1, 5, 5, 0, 0, 0, 5, 5, /* ... */],
        // ...
    ],
    events: [
        // Random encounters scattered throughout
        (
            position: (x: 10, y: 10),
            event_type: Combat((monster_ids: [3, 3, 3], ambush: false)),
            repeatable: true,
            triggered: false,
        ),
        // More encounters...
    ],
    npcs: [],
    exits: [
        (position: (x: 16, y: 0), destination_map: 1, destination_pos: (x: 16, y: 30), direction: Some(North)),
    ],
)
```

## Best Practices

### Map Size Guidelines

- **Towns**: 20x20 to 32x32
- **Dungeons**: 16x16 to 48x48
- **Wilderness**: 32x32 to 64x64
- **Special Areas**: 8x8 to 16x16

### Event Placement

- Don't place events on impassable tiles (walls, mountains)
- Space out combat encounters (at least 3-5 tiles apart)
- Put treasure in corners or dead ends (reward exploration)
- Use `repeatable: false` for unique story events

### Testing Your Map

1. **Validate syntax**:

   ```bash
   cargo run --bin validate_map data/maps/your_map.ron
   ```

2. **Check for common issues**:

   - All positions within bounds (0 to width-1, 0 to height-1)
   - Tiles array matches dimensions exactly
   - No events on walls
   - Exits lead to valid maps

3. **Playtest**:
   - Walk through the entire map
   - Trigger all events
   - Test exits work correctly
   - Verify NPCs are accessible

## Troubleshooting

### "Failed to parse RON" Error

**Problem**: Syntax error in RON file

**Solution**:

- Check for missing commas
- Ensure all parentheses and brackets match
- Verify field names are spelled correctly

### "Position out of bounds" Error

**Problem**: Event/NPC/Exit coordinates exceed map size

**Solution**:

- Remember coordinates are 0-indexed
- Maximum valid position: (width-1, height-1)
- Example: 20x20 map → valid range is (0,0) to (19,19)

### "Event is on impassable tile" Error

**Problem**: Event placed on wall or other blocked tile

**Solution**:

- Move event to floor tile (0)
- Check tile type at event position

### "Tiles array has wrong dimensions" Error

**Problem**: Row count or column count doesn't match width/height

**Solution**:

- Verify each row has exactly `width` elements
- Verify there are exactly `height` rows
- Use a text editor with column selection to help

## Advanced Topics

### Map Builder Tool

For visual map creation, see the map builder tools:

- [Using Map Builder](using_map_builder.md) - Visual map creation tool
- [Using SDK Map Editor](using_sdk_map_editor.md) - SDK map editor

### Procedural Generation

For randomly generated dungeons, see:

- `src/world/generator.rs` (when implemented)
- Architecture Section 11.4 (Map Special Events)

### Custom Events

To add new event types:

1. Define event enum in `src/world/map.rs`
2. Implement handler in game engine
3. Add to RON deserializer
4. Update this documentation

## Content IDs Reference

### Finding Valid IDs

Check these files for available content:

- **Monsters**: `data/monsters.ron` - Monster definitions with stats and abilities
- **Items**: `data/items.ron` - Items using proficiency and classification system
- **Spells**: `data/spells.ron` - Spell definitions for spell-casting NPCs
- **Character Definitions**: `data/character_definitions.ron` - Pre-made character templates

### Monster IDs (MonsterId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Check `data/monsters.ron` for your campaign's available monsters. Common examples:

```text
1  - Goblin (weak, starter enemy)
2  - Orc (medium strength)
3  - Wolf (fast, pack enemy)
4  - Skeleton (undead, resistant to certain attacks)
5  - Zombie (slow, high HP)
```

### Item IDs (ItemId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Items now use the proficiency system with classifications and tags. Check `data/items.ron` for available items:

```text
1-9   - Basic weapons (Club, Dagger, Short Sword, etc.)
10-20 - Armor pieces (Leather, Chain, Plate)
50-60 - Consumables (Healing Potion, Antidote, etc.)
100+  - Special/quest items
```

**Note**: Items are no longer restricted by class/race disablement bits. Instead, they use:

- **Proficiencies**: Skills required to use the item (e.g., LongBow, PlateArmor)
- **Classifications**: Item categories (WeaponType, ArmorWeight, etc.)
- **Alignment Restrictions**: Good/Neutral/Evil restrictions
- **Racial Tags**: Race-specific size or biological restrictions

## Examples

### Example 1: Inn Map

A small 12x12 indoor map with NPCs and a rest area:

```ron
(
    id: 10,
    name: "The Rusty Tankard Inn",
    map_type: Town,
    width: 12,
    height: 12,
    outdoor: false,
    allow_resting: true,
    danger_level: 0,
    tiles: [
        [1, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1, 1, 2, 1, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ],
    events: [
        (
            position: (x: 6, y: 5),
            event_type: Healing((hp_restore: 0, sp_restore: 0, cure_conditions: true)),
            repeatable: true,
            triggered: false,
        ),
    ],
    npcs: [
        (id: 1, name: "Innkeeper", position: (x: 3, y: 3), dialogue_id: 301, shop_id: Some(20)),
        (id: 2, name: "Bard", position: (x: 9, y: 9), dialogue_id: 302, shop_id: None),
    ],
    exits: [
        (position: (x: 5, y: 0), destination_map: 1, destination_pos: (x: 8, y: 10), direction: None),
    ],
)
```

## Advanced Map Features

### Using Character Definitions in Events

Maps can reference pre-made character definitions for special encounters or NPCs:

```ron
events: [
    (
        position: (x: 15, y: 15),
        event_type: Text((
            message: "A mysterious warrior challenges you!",
        )),
        repeatable: false,
        triggered: false,
    ),
],
```

Character definitions are stored in `data/character_definitions.ron` and can be instantiated into full characters during gameplay.

### Quest Integration

Maps can trigger quest events that set flags in the campaign state:

```ron
events: [
    (
        position: (x: 10, y: 10),
        event_type: Quest((
            quest_id: 1,
            flag_name: "found_ancient_artifact",
            message: "You have discovered the Ancient Artifact of Power!",
        )),
        repeatable: false,
        triggered: false,
    ),
],
```

## References

- **Format Specification**: [`docs/reference/map_ron_format.md`](../reference/map_ron_format.md)
- **Architecture**: [`docs/reference/architecture.md`](../reference/architecture.md) Section 4.2
- **Character Definitions**: [`docs/how-to/character_definitions.md`](character_definitions.md)
- **Proficiency System**: [`docs/reference/architecture.md`](../reference/architecture.md) Section 4.4
- **Validation Tool**: `src/bin/validate_map.rs`
- **Map Editor Tools**:
  - CLI Map Validator: `cargo run --bin validate_map`
  - SDK Map Editor: Campaign Builder UI

---

**Last Updated**: 2025-01-25
**Version**: 2.0
