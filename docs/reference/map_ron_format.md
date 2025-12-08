# Map RON Format Reference

## Overview

This document specifies the RON (Rusty Object Notation) format for map files in Antares RPG. Maps define the tile-based world, events, NPCs, and encounters that players experience.

**Note**: This document reflects the current data-driven architecture with proficiency-based item restrictions and character definition system.

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

## Content IDs Reference

### Monster IDs (MonsterId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Example monsters (see `data/monsters.ron` for your campaign's complete list):

- `1` - Goblin
- `2` - Orc
- `3` - Wolf
- `4` - Skeleton
- `5` - Zombie

**Note**: Monster definitions include stats, abilities, and loot tables.

### Item IDs (ItemId = u8)

Valid range: 1-255 (0 is reserved/invalid)

Items now use the **proficiency system** with classifications and tags. See `data/items.ron` for available items:

- `1` - Club (Weapon - requires Club proficiency)
- `2` - Dagger (Weapon - requires Dagger proficiency)
- `3` - Short Sword (Weapon - requires ShortSword proficiency)
- `10` - Leather Armor (Light armor - no proficiency required)
- `50` - Healing Potion (Consumable - no restrictions)
- `100+` - Quest/special items

**Proficiency System**: Items define required proficiencies, classifications, alignment restrictions, and racial tags instead of disablement bits.

### Character Definition IDs (CharacterId = u16)

Valid range: 1-65535 (0 is reserved/invalid)

Character definitions are pre-made character templates stored in `data/character_definitions.ron`. They can be:

- Referenced in events for special encounters
- Used as quick-start characters for players
- Instantiated into full `Character` instances during gameplay

Example: CharacterId `1` might be "Sir Roland the Knight"

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

Provides gold, gems, and/or items when triggered:

```ron
event_type: Treasure((
    gold: 100,      // Gold amount (0-65535)
    gems: 5,        // Gem count (0-65535)
    items: [50, 51],  // Item IDs (ItemId list - must exist in items.ron)
)),
```

**Validation**: All item IDs must exist in `data/items.ron`. Items use proficiency system, so ensure party can use them.

##### 2. Combat Event

Initiates combat with specified monsters:

```ron
event_type: Combat((
    monster_ids: [1, 1, 2, 3],  // Monster IDs (MonsterId list - must exist in monsters.ron)
    ambush: false,              // If true, monsters attack first
)),
```

**Validation**: All monster IDs must exist in `data/monsters.ron`. Duplicate IDs mean multiple instances of the same monster type.

##### 3. Text Event

Displays narrative text to the player:

```ron
event_type: Text((
    message: "You found a mysterious inscription...",
)),
```

**Use Cases**: Lore, environmental storytelling, hints, warnings.

##### 4. Healing Event

Restores party health and removes status conditions:

```ron
event_type: Healing((
    hp_restore: 50,   // HP restored per character (0 = full heal)
    sp_restore: 20,   // SP restored per character (0 = full restore)
    cure_conditions: true,  // Remove poison, disease, paralysis, etc.
)),
```

**Use Cases**: Healing fountains, rest areas, shrines. Set `repeatable: true` for renewable healing sources.

##### 5. Teleport Event

Instantly moves the party to another location:

```ron
event_type: Teleport((
    destination_map: 3,            // Target MapId
    destination_pos: (x: 10, y: 10),  // Spawn position on target map
)),
```

**Use Cases**: Magical portals, trap doors, fast travel. Destination position must be walkable.

##### 6. Quest Event

Sets a quest flag in the campaign state:

```ron
event_type: Quest((
    quest_id: 1,                           // Quest system ID
    flag_name: "defeated_dragon",          // Unique flag identifier
    message: "The ancient dragon lies defeated.",  // Display message
)),
```

**Use Cases**: Story progression, quest completion, unlocking new areas. Flags persist across game sessions.

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
    dialogue_id: 201,   // Links to dialogue tree (future implementation)
    shop_id: Some(10),  // Optional shop ID (None if not a merchant)
),
```

**NPC Placement**:

- Position must be on walkable tile
- NPC IDs must be unique within the same map (can be reused across different maps)
- Dialogue and shop systems reference external data files

#### Exits Array

Map transitions connect maps together (e.g., town exits, dungeon stairs).

```ron
(
    position: (x: 10, y: 0),          // Exit location on current map
    destination_map: 2,               // Target MapId (must exist)
    destination_pos: (x: 10, y: 19),  // Spawn position on target map
    direction: Some(North),           // Optional facing direction after transition
),
```

**Direction Enum**: `North`, `South`, `East`, `West`, or `None`

**Exit Design**:

- Typically placed on map edges for intuitive navigation
- Destination position should be walkable and not blocked
- Two-way connections require exits on both maps
- Direction sets which way the party faces after transitioning

## Validation Rules

Maps must satisfy these constraints to be valid:

### Structure Validation

1. **Map ID**: Must be unique campaign-wide and non-zero (1-65535)
2. **Dimensions**: `width` and `height` must be ≥ 4 and ≤ 256
3. **Tiles Array**:
   - Must have exactly `height` rows
   - Each row must have exactly `width` elements
   - All tile type IDs must be valid (0-8)

### Content Validation

4. **Events**:

   - Position must be within map bounds: `0 ≤ x < width`, `0 ≤ y < height`
   - **Monster IDs** must exist in campaign's `data/monsters.ron`
   - **Item IDs** must exist in campaign's `data/items.ron`
   - Cannot place events on impassable tiles (walls, mountains)
   - Treasure events should not be placed on frequently traversed paths

5. **NPCs**:

   - Position must be within map bounds
   - NPC IDs must be unique per map (can reuse IDs across different maps)
   - Cannot place NPCs on impassable tiles or doors
   - Leave space around NPCs for player movement

6. **Exits**:
   - Position must be within map bounds
   - Destination map ID must exist in the campaign
   - Destination position must be walkable on target map
   - Consider two-way connectivity for non-linear exploration

### Gameplay Validation

7. **Reachability**: All walkable areas should be reachable from the spawn point (use flood-fill validation)
8. **Border**: Strongly recommend wall (`1`) border to prevent edge-walking bugs
9. **Exits**: At least one exit in Town/Outdoor maps (Dungeons can be self-contained if intentional)
10. **Difficulty**: `danger_level` should match actual encounter difficulty

### Proficiency System Validation

11. **Item Proficiencies**: Ensure treasure items can be used by at least some classes
12. **Equipment Balance**: Consider class/race proficiencies when placing equipment rewards
13. **Alignment Restrictions**: Verify alignment-restricted items match expected party alignments

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

### CLI Tools

- **Map Validator**: `cargo run --bin validate_map data/maps/your_map.ron`
- **Campaign Validator**: `cargo run --bin validate_campaign campaigns/your_campaign/`

### SDK Tools

- **Campaign Builder**: `cargo run --bin campaign_builder`
  - Visual map editor with tile placement
  - Event editor with validation
  - NPC and exit management
  - Real-time validation feedback

### Testing Workflow

1. Create/edit map in RON format
2. Run map validator to check structure
3. Run campaign validator to check cross-references
4. Load in game test mode to verify gameplay
5. Playtest to ensure events trigger correctly

## References

- **Architecture**: `docs/reference/architecture.md` Section 4.2 (World & Map)
- **How-To Guide**: `docs/how-to/creating_maps.md` - Step-by-step map creation
- **Tutorial**: `docs/tutorials/creating_campaigns.md` - Complete campaign example
- **Monster Definitions**: `data/monsters.ron` (campaign-specific)
- **Item Definitions**: `data/items.ron` (campaign-specific, proficiency-based)
- **Character Definitions**: `data/character_definitions.ron` (for pre-made characters)
- **Type Definitions**: `src/world/map.rs`, `src/world/tile.rs`, `src/domain/`

## Best Practices

1. **Start Small**: Begin with 16x16 or 20x20 maps, expand as needed
2. **Test Frequently**: Run validators after each major change
3. **Document Events**: Use RON comments (`//`) to explain complex event chains
4. **Consistent IDs**: Maintain a spreadsheet tracking all IDs across the campaign
5. **Playtest**: Walk through maps to verify exits, events, and NPC interactions
6. **Proficiency Balance**: When placing equipment, consider which classes/races can use it
7. **Alignment Consideration**: Place alignment-restricted items appropriately
8. **Progressive Difficulty**: Match `danger_level` to actual encounter/treasure balance
9. **Two-Way Exits**: Ensure players can return from areas unless intentionally one-way
10. **Visual Consistency**: Use tile types consistently (e.g., always use `2` for doors)

## Design Patterns

### Safe Town Pattern

- `map_type: Town`
- `outdoor: true`, `allow_resting: true`, `danger_level: 0`
- NPCs with shops and dialogue
- Healing fountain at center
- Multiple exits to adventure areas

### Dungeon Pattern

- `map_type: Dungeon`
- `outdoor: false`, `allow_resting: false`, `danger_level: 3-7`
- Linear or maze-like layout with rooms
- Combat encounters with treasure rewards
- Boss encounter at the end
- Exit back to surface/previous level

### Wilderness Pattern

- `map_type: Outdoor`
- `outdoor: true`, `allow_resting: false`, `danger_level: 2-6`
- Large open areas with scattered terrain features
- Random encounters using `repeatable: true`
- Paths connecting towns/dungeons
- Landmarks for navigation

---

**Last Updated**: 2025-01-25
**Version**: 2.0
