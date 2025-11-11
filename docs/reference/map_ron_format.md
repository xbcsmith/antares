# Map RON Format Reference

## Overview

This document specifies the RON (Rusty Object Notation) format for map files in Antares RPG. Maps define the tile-based world, events, NPCs, and encounters that players experience.

## File Location

All map files are stored in:

```text
data/maps/
├── town_starter.ron
├── dungeon_starter.ron
├── forest_area.ron
└── ...
```

## Coordinate System

Antares uses a **zero-indexed coordinate system** with the origin at the **top-left corner**:

```text
     0   1   2   3   (x →)
   ┌───┬───┬───┬───┐
 0 │   │   │   │   │
   ├───┼───┼───┼───┤
 1 │   │   │   │   │
   ├───┼───┼───┼───┤
 2 │   │   │   │   │
   └───┴───┴───┴───┘
(y ↓)
```

- **X-axis**: Increases from left to right (0 to width-1)
- **Y-axis**: Increases from top to bottom (0 to height-1)
- **Position (x, y)**: (0, 0) is top-left, (width-1, height-1) is bottom-right

## Monster and Item IDs

### Monster IDs (MonsterId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Example monsters (see `data/monsters.ron` for complete list):

- `1` - Goblin
- `2` - Orc
- `3` - Wolf
- `4` - Skeleton
- `5` - Zombie

### Item IDs (ItemId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Example items (see `data/items.ron` for complete list):

- `1` - Club
- `2` - Dagger
- `3` - Short Sword
- `10` - Leather Armor
- `50` - Healing Potion
- `100` - Gold (varies)

## RON Format Specification

### Complete Map Structure

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
        // Row 0 (y=0)
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        // Row 1 (y=1)
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        // ... (remaining rows)
    ],
    events: [
        (
            position: (x: 10, y: 10),
            event_type: Treasure((
                gold: 100,
                gems: 0,
                items: [50],
            )),
            repeatable: false,
            triggered: false,
        ),
        (
            position: (x: 5, y: 5),
            event_type: Combat((
                monster_ids: [1, 1, 2],
                ambush: false,
            )),
            repeatable: true,
            triggered: false,
        ),
    ],
    npcs: [
        (
            id: 1,
            name: "Town Guard",
            position: (x: 10, y: 5),
            dialogue_id: 101,
            shop_id: None,
        ),
    ],
    exits: [
        (
            position: (x: 10, y: 19),
            destination_map: 2,
            destination_pos: (x: 10, y: 1),
            direction: Some(South),
        ),
    ],
)
```

### Field Descriptions

#### Map Metadata

- **id** (`MapId = u16`): Unique map identifier (1-65535)
- **name** (`String`): Human-readable map name
- **map_type** (`MapType` enum): `Town`, `Dungeon`, `Outdoor`, `Special`
- **width** (`u32`): Map width in tiles (recommended: 16-64)
- **height** (`u32`): Map height in tiles (recommended: 16-64)
- **outdoor** (`bool`): `true` for outdoor maps (affects light), `false` for indoor
- **allow_resting** (`bool`): Whether party can rest on this map
- **danger_level** (`u8`): Encounter difficulty (0 = safe, 10 = deadly)

#### Tiles Array

**Format**: 2D array `Vec<Vec<u8>>` where each `u8` is a tile type ID

**Structure**:

```ron
tiles: [
    // Row 0 (y=0): [tile(0,0), tile(1,0), tile(2,0), ...]
    [1, 1, 1, 1, 1],
    // Row 1 (y=1): [tile(0,1), tile(1,1), tile(2,1), ...]
    [1, 0, 0, 0, 1],
    // Row 2 (y=2)
    [1, 0, 2, 0, 1],
    // ...
],
```

**Tile Type IDs**:

- `0` - Floor (walkable, no special properties)
- `1` - Wall (blocked, impassable)
- `2` - Door (walkable, transition/barrier)
- `3` - Water (special terrain, may require swimming)
- `4` - Lava (damaging terrain)
- `5` - Forest (outdoor terrain, blocks line of sight)
- `6` - Mountain (impassable outdoor terrain)
- `7` - Stairs Up (transition to different level)
- `8` - Stairs Down (transition to different level)

**Important**:

- Outer border should typically be walls (`1`) to prevent out-of-bounds
- Each row must have exactly `width` elements
- Must have exactly `height` rows

#### Events Array

Events are triggered when the party enters a specific position.

**Event Structure**:

```ron
(
    position: (x: u32, y: u32),
    event_type: EventType,
    repeatable: bool,
    triggered: bool,
)
```

**EventType Variants**:

##### 1. Treasure Event

```ron
event_type: Treasure((
    gold: 100,      // Gold amount (0-65535)
    gems: 5,        // Gem count (0-65535)
    items: [50, 51],  // Item IDs (ItemId list)
)),
```

##### 2. Combat Event

```ron
event_type: Combat((
    monster_ids: [1, 1, 2, 3],  // Monster IDs (MonsterId list)
    ambush: false,              // If true, monsters attack first
)),
```

##### 3. Text Event

```ron
event_type: Text((
    message: "You found a mysterious inscription...",
)),
```

##### 4. Healing Event

```ron
event_type: Healing((
    hp_restore: 50,   // HP restored per character (0 = full)
    sp_restore: 20,   // SP restored per character (0 = full)
    cure_conditions: true,  // Remove poison, disease, etc.
)),
```

##### 5. Teleport Event

```ron
event_type: Teleport((
    destination_map: 3,
    destination_pos: (x: 10, y: 10),
)),
```

##### 6. Quest Event

```ron
event_type: Quest((
    quest_id: 1,
    flag_name: "defeated_dragon",
    message: "The ancient dragon lies defeated.",
)),
```

**Event Flags**:

- **repeatable**: `true` = event can trigger multiple times, `false` = one-time only
- **triggered**: `false` initially (engine sets to `true` after first trigger)

#### NPCs Array

Non-player characters that can be interacted with.

```ron
(
    id: 1,              // NPC ID (unique per map)
    name: "Blacksmith",
    position: (x: 5, y: 10),
    dialogue_id: 201,   // Links to dialogue tree
    shop_id: Some(10),  // Optional shop ID (None if not a merchant)
),
```

#### Exits Array

Map transitions (e.g., town exits, dungeon stairs).

```ron
(
    position: (x: 10, y: 0),       // Exit location on current map
    destination_map: 2,            // Target MapId
    destination_pos: (x: 10, y: 19),  // Spawn position on target map
    direction: Some(North),        // Optional facing direction
),
```

**Direction Enum**: `North`, `South`, `East`, `West`, `None`

## Validation Rules

Maps must satisfy these constraints:

### Structure Validation

1. **Map ID**: Must be unique and non-zero (1-65535)
2. **Dimensions**: `width` and `height` must be ≥ 4 and ≤ 256
3. **Tiles Array**:
   - Must have exactly `height` rows
   - Each row must have exactly `width` elements
   - All tile type IDs must be valid (0-8)

### Content Validation

4. **Events**:

   - Position must be within map bounds: `0 ≤ x < width`, `0 ≤ y < height`
   - Monster IDs must exist in `data/monsters.ron`
   - Item IDs must exist in `data/items.ron`
   - Cannot place events on impassable tiles (e.g., walls)

5. **NPCs**:

   - Position must be within map bounds
   - NPC IDs must be unique per map
   - Cannot place NPCs on impassable tiles

6. **Exits**:
   - Position must be within map bounds
   - Destination map ID must exist
   - Destination position must be valid on target map

### Gameplay Validation

7. **Reachability**: All walkable areas should be reachable from the spawn point
8. **Border**: Recommend wall (`1`) border to prevent edge-walking bugs
9. **Exits**: At least one exit in Town/Outdoor maps (Dungeons can be self-contained)

## Common Patterns

### Town Map Pattern

```ron
(
    id: 1,
    name: "Starting Town",
    map_type: Town,
    width: 20,
    height: 20,
    outdoor: true,
    allow_resting: true,
    danger_level: 0,
    // Walls on border, open interior, NPCs (shops, trainers), exits
)
```

### Dungeon Map Pattern

```ron
(
    id: 2,
    name: "Dark Cave Level 1",
    map_type: Dungeon,
    width: 32,
    height: 32,
    outdoor: false,
    allow_resting: false,
    danger_level: 3,
    // Corridors, rooms, combat events, treasure chests, stairs
)
```

### Outdoor/Wilderness Pattern

```ron
(
    id: 3,
    name: "Darkwood Forest",
    map_type: Outdoor,
    width: 64,
    height: 64,
    outdoor: true,
    allow_resting: false,
    danger_level: 5,
    // Large open areas, trees, random encounters, landmarks
)
```

## Example: Minimal Valid Map

```ron
(
    id: 99,
    name: "Test Chamber",
    map_type: Special,
    width: 5,
    height: 5,
    outdoor: false,
    allow_resting: true,
    danger_level: 0,
    tiles: [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    events: [],
    npcs: [],
    exits: [],
)
```

## Tools and Validation

- **Validator**: Use `src/bin/validate_map.rs` to check map files before deployment
- **Map Builder**: Use the Map Builder tool (Phase 2) for interactive map creation
- **Testing**: Load maps in test mode to verify events and connectivity

## References

- **Architecture**: `docs/reference/architecture.md` Section 4.2 (World & Map)
- **Monsters**: `data/monsters.ron`
- **Items**: `data/items.ron`
- **Type Definitions**: `src/world/map.rs`, `src/world/tile.rs`

## Best Practices

1. **Start Small**: Begin with 16x16 or 20x20 maps
2. **Test Frequently**: Validate after each major change
3. **Document Events**: Use comments to explain complex event chains
4. **Consistent IDs**: Use a spreadsheet to track map/event/NPC IDs
5. **Playtest**: Walk through maps to verify exits and event triggers work

---

**Last Updated**: 2024
**Version**: 1.0
