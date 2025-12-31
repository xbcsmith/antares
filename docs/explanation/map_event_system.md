# Map Event System

## Overview

Events in Antares maps use a single, canonical position-based event model. All events are stored in `Map.events: HashMap<Position, MapEvent>`, providing a clear single source of truth for event data.

This document explains how map events work, how to define them, and how they interact with the game runtime.

## Event Definition Format

Events are defined in map RON files in the `events` field as a HashMap keyed by position:

```ron
MapBlueprint(
    id: 1,
    name: "Tutorial Town",
    description: "A peaceful starting town",
    width: 20,
    height: 20,
    tiles: [
        // ... tile definitions ...
    ],
    events: {
        Position(x: 5, y: 5): MapEvent::Sign(
            name: "Welcome Sign",
            description: "A wooden sign with carved letters",
            text: "Welcome to Antares! Explore the town and talk to the townsfolk.",
        ),
        Position(x: 10, y: 3): MapEvent::Treasure(
            name: "Treasure Chest",
            description: "A locked wooden chest",
            loot: [
                LootItem(item_id: 1, quantity: 50),  // 50 gold
                LootItem(item_id: 42, quantity: 1),  // Magic sword
            ],
        ),
        Position(x: 15, y: 8): MapEvent::Combat(
            name: "Goblin Ambush",
            description: "Goblins leap from the shadows!",
            monsters: [
                MonsterSpawn(monster_id: 1, count: 3),  // 3 goblins
            ],
        ),
    },
    npcs: [],
)
```

## Event Types

The `MapEvent` enum defines all possible event types:

### Sign Events

Display text to the player when triggered.

```ron
MapEvent::Sign(
    name: "Town Notice",
    description: "A notice board",
    text: "The mayor seeks brave adventurers for a dangerous quest!",
)
```

### Treasure Events

Award items, gold, or other loot to the party.

```ron
MapEvent::Treasure(
    name: "Hidden Cache",
    description: "A secret stash",
    loot: [
        LootItem(item_id: 1, quantity: 100),   // 100 gold
        LootItem(item_id: 15, quantity: 5),    // 5 healing potions
        LootItem(item_id: 23, quantity: 1),    // Enchanted ring
    ],
)
```

### Combat Events

Trigger a tactical battle when the party enters the position.

```ron
MapEvent::Combat(
    name: "Orc Patrol",
    description: "A patrol of orcs blocks your path",
    monsters: [
        MonsterSpawn(monster_id: 5, count: 2),  // 2 orc warriors
        MonsterSpawn(monster_id: 6, count: 1),  // 1 orc shaman
    ],
)
```

### Teleport Events

Transport the party to another map or position.

```ron
MapEvent::Teleport(
    name: "Town Portal",
    description: "A shimmering magical portal",
    destination_map: 2,
    destination_x: 10,
    destination_y: 10,
)
```

### Trap Events

Deal damage or apply conditions to the party.

```ron
MapEvent::Trap(
    name: "Poison Dart Trap",
    description: "A hidden pressure plate triggers poison darts!",
    damage: 10,
    effect: Some(Condition::Poisoned),
)
```

### NPC Dialogue Events

Start a conversation with an NPC.

```ron
MapEvent::NpcDialogue(
    name: "Village Elder",
    description: "The village elder greets you",
    dialogue_id: 42,
)
```

## Runtime Behavior

### Event Triggering

When the party moves to a position, the game queries `Map.events` by position:

```rust
// In game/systems/events.rs
pub fn check_for_events(party_position: Position, map: &Map) -> Option<MapEvent> {
    map.get_event_at_position(party_position).cloned()
}
```

If an event exists at that position, a `MapEventTriggered` message is dispatched to the appropriate handler based on event type.

### Event Handlers

Each event type has a dedicated handler:

- **Sign**: Display message in UI, wait for player acknowledgment
- **Treasure**: Add loot to party inventory, show loot dialog
- **Combat**: Transition to combat mode with specified monsters
- **Teleport**: Change active map and party position
- **Trap**: Calculate damage, apply effects, show notification
- **NpcDialogue**: Load dialogue tree, enter dialogue mode

### One-Time vs. Repeatable Events

Currently, all events are repeatable - they trigger every time the party enters the position. Future implementations may add event flags to mark events as one-time only.

## Migration from Tile Event Triggers

### Old Format (Deprecated - DO NOT USE)

The old system stored event triggers on individual tiles:

```ron
// WRONG - This format is no longer supported
Tile(
    x: 5,
    y: 5,
    terrain: Grass,
    event_trigger: Some(42),  // REMOVED - don't use this
)
```

This created several problems:

1. **Duplicate representation**: Event ID on tile, event data in separate structure
2. **Sync issues**: Tile trigger could reference non-existent event
3. **File bloat**: Every tile had `event_trigger: None` in serialization
4. **Unclear semantics**: Was the tile or the event the source of truth?

### New Format (Current - USE THIS)

The new system uses only the `events` HashMap:

```ron
MapBlueprint(
    id: 1,
    name: "Example Map",
    tiles: [
        Tile(
            x: 5,
            y: 5,
            terrain: Grass,
            // No event_trigger field
        ),
        // ... more tiles ...
    ],
    events: {
        Position(x: 5, y: 5): MapEvent::Sign(
            name: "Example Event",
            description: "An example",
            text: "Event content here",
        ),
    },
)
```

### Migration Process

All tutorial campaign maps were migrated automatically using the `migrate_maps` tool:

```bash
cd sdk/campaign_builder
cargo run --bin migrate_maps -- ../../campaigns/tutorial/data/maps/map_1.ron
```

The migration:

1. Creates a `.backup` file of the original
2. Removes all `event_trigger:` lines from tiles
3. Preserves all other content unchanged
4. Reduces file size (removed 13,200+ bytes per map)

### Validation

After migration, verify:

```bash
# Should return 0 (no event_trigger fields remain)
grep -r "event_trigger:" campaigns/tutorial/data/maps/*.ron | wc -l

# Should return number of maps (backups created)
ls campaigns/tutorial/data/maps/*.backup | wc -l

# Maps should load correctly
cargo run --bin campaign-builder
```

## Creating Events in the Map Editor

### Adding an Event

1. Open the map in the campaign builder
2. Select the **Events** panel
3. Click **Add Event** or click on a tile in the map view
4. Choose the event type from the dropdown
5. Fill in required fields:
   - Name (shown to player)
   - Description (flavor text)
   - Type-specific data (text, loot, monsters, etc.)
6. Click **Save**

The event is immediately added to `Map.events` at the selected position.

### Editing an Event

1. Click on a tile with an existing event
2. The event editor opens with current values
3. Modify fields as needed
4. Click **Save Changes**

The event is updated in-place at the same position.

### Deleting an Event

1. Click on a tile with an event
2. Click **Delete Event**
3. Confirm deletion

The event is removed from `Map.events`.

### Undo/Redo

Event operations (add, edit, delete) are fully integrated with the editor's undo/redo system. Event data is preserved across undo/redo operations.

## Best Practices

### Event Placement

- **Don't block required paths**: Avoid placing combat/trap events on the only path to objectives
- **Provide context**: Use sign events before dangerous areas
- **Balance density**: Too many events overwhelm players; too few make maps boring
- **Test accessibility**: Ensure all events can be reached by valid party movement

### Event Design

- **Clear names**: Use names that hint at event content ("Goblin Ambush", not "Event 42")
- **Descriptive text**: Provide flavor and context, not just mechanics
- **Balanced rewards**: Treasure events should match party level and map difficulty
- **Fair traps**: Traps should warn players (description, earlier hints) unless intentionally hidden

### Performance Considerations

- **HashMap lookup**: Event checking is O(1) - no performance concerns
- **Event count**: Maps can have hundreds of events without impact
- **Memory usage**: Events are loaded with map, kept in memory during gameplay

## Technical Details

### Data Structures

```rust
// Core map structure (domain/world/types.rs)
pub struct Map {
    pub id: MapId,
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
    pub events: HashMap<Position, MapEvent>,  // Position-keyed events
    pub npcs: Vec<Npc>,
}

// Event lookup helper
impl Map {
    pub fn get_event_at_position(&self, position: Position) -> Option<&MapEvent> {
        self.events.get(&position)
    }
}
```

### Serialization Format

Events serialize as a RON HashMap:

```ron
events: {
    Position(x: 5, y: 5): MapEvent::Sign(/* ... */),
    Position(x: 10, y: 3): MapEvent::Treasure(/* ... */),
}
```

Empty maps have:

```ron
events: {}
```

### Type Safety

Event positions use the `Position` type, ensuring type safety:

```rust
pub struct Position {
    pub x: i32,
    pub y: i32,
}
```

Event IDs (when needed for references) use the `EventId` type alias:

```rust
pub type EventId = u16;
```

## Troubleshooting

### Event Not Triggering

**Symptom**: Party walks over event position, nothing happens

**Causes**:
1. Event position doesn't match party position (check coordinates)
2. Event not saved to map file (verify in editor and RON file)
3. Event handler not implemented for event type
4. Game mode prevents event processing (events don't trigger in combat mode)

**Solution**: Verify event exists in `Map.events` and position matches exactly.

### Map Won't Load After Edit

**Symptom**: RON deserialization error when loading map

**Causes**:
1. Syntax error in event definition
2. Invalid event type or field names
3. Missing required fields
4. Leftover `event_trigger` field (use migration tool)

**Solution**: Check RON syntax, compare to examples in this document.

### Editor Shows Wrong Event

**Symptom**: Clicking a tile shows unexpected event or no event

**Causes**:
1. Event at different position than expected
2. Event removed but tile visual not updated
3. Multiple events at same position (only one allowed)

**Solution**: Close and reopen the map, verify event position in Events panel.

## Future Enhancements

Potential future additions to the event system:

- **Event flags**: Mark events as one-time, repeatable, or conditional
- **Event chains**: Trigger multiple events in sequence
- **Conditional events**: Events that only trigger if conditions met (quest state, items, etc.)
- **Scripted events**: Custom Lua/Rhai scripts for complex behaviors
- **Area events**: Events that trigger in a radius, not just exact position
- **Event groups**: Related events that share state or progression

## References

- **Architecture**: `docs/reference/architecture.md` (Section 4: Core Data Structures)
- **Event System**: `src/game/systems/events.rs` (Event dispatch and handling)
- **Map Editor**: `sdk/campaign_builder/src/map_editor.rs` (Event UI and editing)
- **Migration Tool**: `sdk/campaign_builder/src/bin/migrate_maps.rs` (Format migration)

## Change History

- **2025-01**: Removed per-tile `event_trigger` field, consolidated to position-keyed `Map.events`
- **2024-12**: Initial event system implementation with multiple event types
