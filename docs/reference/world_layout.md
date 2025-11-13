# world_layout.md - Antares RPG World Structure

## Overview

This document describes the game world structure for Antares RPG, including map connections, content distribution, and navigation between areas. The world consists of three starter maps designed to provide a complete introductory gameplay experience.

## Map Index

| Map ID | Name            | Size  | Type       | Purpose                          |
|--------|-----------------|-------|------------|----------------------------------|
| 1      | Starter Town    | 20×15 | Safe Zone  | Hub, shops, NPCs, quests         |
| 2      | Starter Dungeon | 16×16 | Combat     | Combat training, weak monsters   |
| 3      | Forest Area     | 20×20 | Wilderness | Exploration, mid-level encounters|

## Map Connections

The world uses a hub-and-spoke design with Starter Town as the central hub.

```
    Forest Area (3)
         |
         | (0,10) → Town
         |
    Starter Town (1)
         |
         | (19,7) → Dungeon
         |
    Starter Dungeon (2)
         |
         | (0,7) → Town
```

### Connection Details

#### Starter Town Exits

- **Position (19,7)**: Exit to Starter Dungeon (East)
  - Event type: Sign
  - Warning message about dungeon dangers
  - Leads to Map ID 2

- **Future Connection**: Exit to Forest Area (planned for west side)
  - Currently Forest Area has return exit implemented
  - Forward connection from town to forest pending

#### Starter Dungeon Exits

- **Position (0,7)**: Exit to Starter Town (West)
  - Event type: Sign
  - Returns to Map ID 1
  - Safe retreat from combat

#### Forest Area Exits

- **Position (0,10)**: Exit to Starter Town (West)
  - Event type: Sign
  - Returns to Map ID 1
  - Primary navigation back to hub

## Map Details

### Map 1: Starter Town

**Type**: Safe zone (no random encounters)

**Dimensions**: 20 tiles wide × 15 tiles high (300 total tiles)

**Terrain Composition**:
- Grass: Primary walkable area (~60%)
- Ground: Border walls and boundaries (~15%)
- Stone: Building floors and structures (~25%)

**Buildings**:
1. **Inn** (4,4): Party management, rest, roster access
   - Stone floor, door at (4,4)
   - NPC: Innkeeper (ID 2) at (4,3)

2. **General Store** (15,4): Item shop for buying/selling
   - Stone floor, door at (15,4)
   - NPC: Merchant (ID 3) at (15,3)

3. **Temple** (10,10): Healing and status cure services
   - Stone floor, door at (10,10)
   - NPC: High Priest (ID 4) at (10,9)

**NPCs**:
- **Village Elder** (ID 1, position 10,4): Quest giver, main storyline
- **Innkeeper** (ID 2, position 4,3): Inn services
- **Merchant** (ID 3, position 15,3): Shop services
- **High Priest** (ID 4, position 10,9): Healing services

**Events**: 4 sign events marking buildings and dungeon exit

**Gameplay Purpose**:
- Safe hub for party management
- Shopping and equipment upgrades
- Quest initiation and storyline
- Healing and recovery between adventures

---

### Map 2: Starter Dungeon

**Type**: Combat dungeon (beginner difficulty)

**Dimensions**: 16 tiles wide × 16 tiles high (256 total tiles)

**Terrain Composition**:
- Stone: 100% (dungeon environment)

**Layout**:
- Multiple rooms connected by corridors
- Door count: 4+ (separating rooms)
- Boss area in southeast corner (14-15, 14-15)

**Encounters**:
- **Weak Monsters** (IDs 1-3): Scattered throughout
- **Boss Encounter** (14,14): 3× Monster ID 3
- Encounter positions: (3,2), (2,6), (5,11), (14,14)

**Treasure**:
- **Chest locations**: (6,2), (13,2), (10,12)
- Contains Items IDs: 10-12, 20-22, 30-31

**Traps**:
- Trap at (10,6): 5 damage

**NPCs**: None (hostile environment)

**Gameplay Purpose**:
- Combat training for new players
- Basic loot acquisition
- Level progression (levels 1-3)
- Introduction to dungeon mechanics

---

### Map 3: Forest Area

**Type**: Wilderness exploration (intermediate difficulty)

**Dimensions**: 20 tiles wide × 20 tiles high (400 total tiles)

**Terrain Composition**:
- Forest: Dense tree coverage (~40%)
- Grass: Clearings and paths (~35%)
- Water: Lake and streams (~25%)

**Natural Features**:
- Large lake in center-south (rows 6-15)
- Forest border around perimeter
- Open clearings for encounters
- Natural pathways between areas

**Encounters**:
- **Mid-Level Monsters** (IDs 4-6): Higher challenge than dungeon
- Encounter positions: (5,3), (14,4), (3,11), (17,16)
- Monster combinations: pairs and mixed groups

**Treasure**:
- **Hidden caches**: (8,8), (16,2), (10,13)
- Contains Items IDs: 13-15, 23-25, 32-33, 40

**Traps**:
- Trap at (7,17): 8 damage (higher than dungeon)

**NPCs**:
- **Lost Ranger** (ID 5, position 2,2): Wilderness guide, warnings

**Gameplay Purpose**:
- Open exploration vs linear dungeon
- Environmental hazards (water obstacles)
- Intermediate combat challenge
- Reward for exploration and discovery

## Event Summary

### Event Type Distribution

| Event Type | Starter Town | Starter Dungeon | Forest Area | Total |
|------------|--------------|-----------------|-------------|-------|
| Sign       | 4            | 1               | 1           | 6     |
| Encounter  | 0            | 4               | 4           | 8     |
| Treasure   | 0            | 3               | 3           | 6     |
| Trap       | 0            | 1               | 1           | 2     |
| **Total**  | **4**        | **9**           | **9**       | **22**|

### Event Parameters

**Encounter Events**:
- Format: `["monster_id_1", "monster_id_2", ...]`
- Monster count varies (1-3 per encounter)

**Treasure Events**:
- Format: `["item_id_1", "item_id_2", ...]`
- Item count varies (1-4 per chest)

**Trap Events**:
- Format: `["damage_amount"]`
- Damage range: 5-8 HP

**Sign Events**:
- Format: `["message_text"]`
- Used for navigation and building markers

## Monster Distribution

### Weak Monsters (Starter Dungeon)

**IDs**: 1, 2, 3

**Usage**:
- Encounter at (3,2): Monsters 1, 2
- Encounter at (2,6): Monsters 2, 1
- Encounter at (5,11): Monsters 1, 3
- Boss at (14,14): Monster 3 × 3

**Expected Player Level**: 1-3

### Mid-Level Monsters (Forest Area)

**IDs**: 4, 5, 6

**Usage**:
- Encounter at (5,3): Monsters 4, 4
- Encounter at (14,4): Monsters 5, 4
- Encounter at (3,11): Monsters 6, 5
- Encounter at (17,16): Monsters 6, 6

**Expected Player Level**: 3-5

## Treasure Distribution

### Item ID Ranges

- **10-15**: Basic equipment tier
- **20-25**: Intermediate equipment tier
- **30-33**: Consumables
- **40**: Special/rare item

### Starter Town

**Available**: Shop inventory only (no loot)
- Players must purchase starting equipment
- Merchant NPC (ID 3) provides shop interface

### Starter Dungeon

**Loot Locations**:
- Chest (6,2): Items 10, 20, 30
- Chest (13,2): Items 11, 21
- Chest (10,12): Items 12, 22, 31

**Total Items**: 8 items across 3 chests

### Forest Area

**Loot Locations**:
- Cache (8,8): Items 13, 23, 32
- Cache (16,2): Items 14, 24
- Cache (10,13): Items 15, 25, 33, 40

**Total Items**: 9 items across 3 caches

**Special**: Item 40 (rare) found only in forest

## Progression Path

### Recommended Flow

1. **Start in Starter Town**
   - Meet Village Elder, accept quest
   - Visit shops, purchase basic equipment
   - Talk to NPCs for information

2. **First Dungeon Run**
   - Enter Starter Dungeon via east exit
   - Clear encounters (monsters 1-3)
   - Collect treasure chests
   - Defeat boss at (14,14)
   - Return to town for healing/shopping

3. **Forest Exploration**
   - Enter Forest Area via west exit (when ready)
   - Navigate natural terrain
   - Face mid-level encounters (monsters 4-6)
   - Find hidden treasure caches
   - Return to town to sell loot

4. **Repeat and Progress**
   - Repeat dungeon/forest for experience
   - Upgrade equipment at shop
   - Level up party members

### Difficulty Curve

```
Easy:     Starter Town (safe) → Starter Dungeon (level 1-3)
Medium:   Starter Dungeon (boss) → Forest Area (level 3-5)
```

## Design Notes

### Navigation

- **Hub Design**: Town acts as central safe point
- **Return Paths**: All areas have clear exits back to town
- **No Dead Ends**: Players can always retreat

### Balance

- **Safe Zone**: Town prevents combat burnout
- **Gradual Difficulty**: Dungeon easier than forest
- **Resource Management**: Town provides healing between runs

### Future Expansion

- Additional connections from town (north, south)
- Higher-level dungeons beyond forest
- Optional side areas and secrets
- Multi-floor dungeons
- Overworld map for long-distance travel

## Data File Locations

- `data/maps/starter_town.ron`
- `data/maps/starter_dungeon.ron`
- `data/maps/forest_area.ron`

All maps use RON (Rusty Object Notation) format and follow the `Map` structure defined in `antares::domain::world`.

---

**Document Version**: 1.0
**Last Updated**: Phase 3 Implementation
**Related Documents**:
- `docs/reference/architecture.md` (Section 5.4 - Map System)
- `docs/how_to/using_map_builder.md` (Map creation guide)
- `docs/explanation/map_content_implementation_plan_v2.md` (Implementation plan)
