# NPC Blueprint Examples

This directory contains examples demonstrating the new NPC placement format for map blueprints.

## Overview

As of Phase 4 of the NPC Externalization implementation, maps can reference NPCs from the NPC database instead of inlining all NPC data directly in map files.

## Benefits

1. **Data Normalization**: Define NPCs once in `npcs.ron`, reference them multiple times across maps
2. **Easier Maintenance**: Update NPC data in one place, changes apply everywhere
3. **Context-Specific Dialogue**: Override dialogue per placement without duplicating NPC definitions
4. **Smaller Map Files**: Maps only store NPC placement data (ID, position, facing)
5. **Type Safety**: String-based NPC IDs provide better debugging than numeric IDs

## File Structure

```
campaign/
├── data/
│   ├── npcs.ron              # NPC definitions database
│   └── maps/
│       └── town.ron          # Map blueprint with NPC placements
```

## NPC Placement Format

### Basic Placement

```ron
npc_placements: [
    (
        npc_id: "merchant_bob",      // References NPC from npcs.ron
        position: (x: 8, y: 4),      // Position on map
        facing: Some(South),          // Optional facing direction
        dialogue_override: None,      // Optional dialogue override
    ),
]
```

### With Dialogue Override

```ron
npc_placements: [
    (
        npc_id: "city_guard",
        position: (x: 3, y: 2),
        facing: Some(East),
        dialogue_override: Some(250),  // Uses dialogue ID 250 instead of guard's default
    ),
]
```

### Multiple Instances

The same NPC definition can be placed multiple times with different positions and settings:

```ron
npc_placements: [
    // Guard at north gate
    (
        npc_id: "city_guard",
        position: (x: 10, y: 2),
        facing: Some(South),
        dialogue_override: None,
    ),
    
    // Guard at south gate with custom greeting
    (
        npc_id: "city_guard",
        position: (x: 10, y: 18),
        facing: Some(North),
        dialogue_override: Some(251),
    ),
]
```

## Required Files

### 1. NPC Definitions (`data/npcs.ron`)

Define NPCs with their core attributes:

```ron
[
    (
        id: "merchant_bob",
        name: "Bob the Merchant",
        description: "A friendly traveling merchant",
        portrait_path: "assets/portraits/merchant_male.png",
        dialogue_id: Some(10),
        quest_ids: [],
        faction: Some("Merchants Guild"),
        is_merchant: true,
        is_innkeeper: false,
    ),
    (
        id: "city_guard",
        name: "City Guard",
        description: "A vigilant city guard",
        portrait_path: "assets/portraits/guard.png",
        dialogue_id: Some(20),  // Default greeting
        quest_ids: [],
        faction: Some("City Watch"),
        is_merchant: false,
        is_innkeeper: false,
    ),
]
```

### 2. Map Blueprint (`data/maps/town.ron`)

Reference NPCs by ID:

```ron
(
    id: 1,
    name: "Town Square",
    // ... other map fields ...
    
    npc_placements: [
        (
            npc_id: "merchant_bob",
            position: (x: 8, y: 4),
            facing: Some(South),
            dialogue_override: None,
        ),
    ],
)
```

## Backward Compatibility

The old inline NPC format is still supported:

```ron
npcs: [
    (
        id: 1,
        name: "Old Format NPC",
        description: "Uses legacy format",
        position: (x: 5, y: 5),
        dialogue_id: Some("greeting"),
    ),
]
```

Maps can contain both `npcs` (legacy) and `npc_placements` (new) during migration.

## Runtime Resolution

At runtime, the game engine:

1. Loads the map blueprint
2. Loads the NPC database from `npcs.ron`
3. Calls `map.resolve_npcs(&npc_db)` to create `ResolvedNpc` instances
4. `ResolvedNpc` combines placement data (position, facing) with definition data (name, portrait, etc.)

```rust
// Example runtime code
let map: Map = blueprint.into();
let npc_db = NpcDatabase::load_from_file("data/npcs.ron")?;
let resolved_npcs = map.resolve_npcs(&npc_db);

for npc in resolved_npcs {
    println!("{} at {:?}", npc.name, npc.position);
    if let Some(dialogue_id) = npc.dialogue_id {
        start_dialogue(dialogue_id);
    }
}
```

## Dialogue Override Use Cases

Use `dialogue_override` when:

- Same NPC type needs different greetings at different locations
- NPC has quest-related dialogue at specific plot points
- NPC behavior changes based on map context
- Testing different dialogue without modifying NPC definition

**Example**: A guard at the town entrance says "Welcome to town!" (dialogue 250), while guards inside use the default "Move along" (dialogue 20).

## Field Reference

### NpcPlacementBlueprint

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `npc_id` | String | Yes | ID of NPC definition in `npcs.ron` |
| `position` | Position | Yes | (x, y) coordinates on map |
| `facing` | Option<Direction> | No | North, South, East, West, or None |
| `dialogue_override` | Option<DialogueId> | No | Override NPC's default dialogue |

### Direction Enum

Valid values: `North`, `South`, `East`, `West`

## Migration Guide

### Step 1: Create NPC Definitions

Extract NPC data from maps into `data/npcs.ron`:

```ron
[
    (
        id: "unique_npc_id",
        name: "NPC Name",
        description: "Description",
        portrait_path: "path/to/portrait.png",
        dialogue_id: Some(10),
        quest_ids: [],
        faction: None,
        is_merchant: false,
        is_innkeeper: false,
    ),
]
```

### Step 2: Update Map Blueprints

Replace inline `npcs` with `npc_placements`:

**Before:**
```ron
npcs: [
    (
        id: 1,
        name: "Merchant",
        description: "Sells items",
        position: (x: 5, y: 5),
        dialogue_id: Some("greeting"),
    ),
]
```

**After:**
```ron
npc_placements: [
    (
        npc_id: "merchant_id",
        position: (x: 5, y: 5),
        facing: None,
        dialogue_override: None,
    ),
]
```

### Step 3: Verify Resolution

Ensure all `npc_id` values reference valid NPCs in `npcs.ron`. Missing NPCs will be skipped with a warning.

## See Also

- `docs/reference/architecture.md` - Section 4 (Data Structures)
- `docs/explanation/npc_externalization_implementation_plan.md` - Full implementation plan
- `src/domain/world/npc.rs` - NPC domain types
- `src/domain/world/blueprint.rs` - Blueprint conversion logic
- `src/sdk/database.rs` - NPC database implementation
