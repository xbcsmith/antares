# Remove per-tile event triggers Implementation Plan

## Overview

We will remove the per-tile `event_trigger` field (an optional `EventId` on each `Tile`) and consolidate to a single canonical event model: `Map.events: HashMap<Position, MapEvent>`. The `MapBlueprint.events: Vec<MapEventBlueprint>` format already exists and will become the only way to define map events. All runtime event dispatch will query events by position. The map editor, map RON files, and related code will be updated to remove deprecated `event_trigger` artifacts.

**Key Outcome:** Single source of truth for map events - position-keyed event lookup via `Map.events`, eliminating dual representation and sync issues.

## Current State Analysis

### Existing Infrastructure

**Event System (already implemented):**

- `MapBlueprint.events: Vec<MapEventBlueprint>` - blueprint format for events with `position`, `name`, `description`, `event_type`
- `Map.events: HashMap<Position, MapEvent>` - runtime event storage, position-indexed
- `MapEvent` enum with variants: `Sign`, `Treasure`, `Encounter`, `Teleport`, `Trap`, `NpcDialogue`
- Event dispatch via `antares/src/game/systems/events.rs` - `check_for_events` system calls `Map.get_event(position)`

**Tile-Level Event Triggers (to be removed):**

- `Tile.event_trigger: Option<EventId>` at `antares/src/domain/world/types.rs` L85
- `trigger_tile_event` function at `antares/src/domain/world/movement.rs` L197-199
- Test code using `tile.event_trigger` at `antares/src/domain/world/movement.rs` L393-395
- All tutorial map tiles contain `event_trigger: None` in `antares/campaigns/tutorial/data/maps/map_1.ron`

### Identified Issues

1. **Duplicate Event Representation:** Events can be defined in `events` list OR via `tile.event_trigger`, causing ambiguity
2. **Sync Burden:** Numeric `EventId` in tiles must be manually kept in sync with event definitions
3. **Dead Code:** `trigger_tile_event` function exists but event dispatch already uses position-based lookup via `Map.get_event`
4. **Serialization Bloat:** Every tile in RON files contains `event_trigger: None` (400+ lines in map_1.ron)
5. **Editor Complexity:** Map editor must handle two event formats

## Decisions Made

**Migration Strategy:** Automatic in-place migration (all map RON files updated in Phase 2)

**Editor Behavior:** Events list is explicit - no implicit event creation when selecting tiles. User must use "Add Event" action in Events panel.

**EventId Type Alias:** Keep `EventId` as `u16` type alias for potential future use (dialogue/quest event references), but remove from Tile.

**Breaking Change Handling:** All phases completed in single implementation to avoid intermediate broken state.

## Implementation Phases

### Phase 1: Core Implementation - Remove Tile Event Triggers

#### 1.1 Foundation Work

**Files to modify:**

1. **`antares/src/domain/world/types.rs`**

   - **L85:** DELETE `pub event_trigger: Option<EventId>,` field from `Tile` struct
   - **L114:** DELETE `event_trigger: None,` from `Tile::new` initialization
   - Verify SPDX header present at L1-2

2. **`antares/src/domain/world/movement.rs`**
   - **L191-199:** DELETE entire `trigger_tile_event` function and its doc comment
   - **L393-395:** UPDATE test `test_trigger_tile_event_exists` - remove `tile.event_trigger = Some(42)` lines

**Exact changes:**

```rust
// antares/src/domain/world/types.rs L67-86
// BEFORE:
pub struct Tile {
    pub terrain: TerrainType,
    pub wall_type: WallType,
    pub blocked: bool,
    pub is_special: bool,
    pub is_dark: bool,
    pub visited: bool,
    pub x: i32,
    pub y: i32,
    pub event_trigger: Option<EventId>,  // DELETE THIS LINE
}

// AFTER:
pub struct Tile {
    pub terrain: TerrainType,
    pub wall_type: WallType,
    pub blocked: bool,
    pub is_special: bool,
    pub is_dark: bool,
    pub visited: bool,
    pub x: i32,
    pub y: i32,
}
```

#### 1.2 Add Foundation Functionality

**Add helper method to `Map` struct:**

1. **File:** `antares/src/domain/world/types.rs`
2. **Location:** Add after `Map::remove_event` (around L405)
3. **Function to add:**

````rust
/// Gets the event at a specific position, if one exists
///
/// # Arguments
///
/// * `position` - The position to check for events
///
/// # Returns
///
/// Returns `Some(&MapEvent)` if an event exists at the position, `None` otherwise
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
///
/// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
/// let pos = Position::new(5, 5);
/// let event = MapEvent::Sign {
///     name: "Test".to_string(),
///     description: "A sign".to_string(),
///     text: "Hello!".to_string(),
/// };
/// map.add_event(pos, event);
///
/// assert!(map.get_event_at_position(pos).is_some());
/// assert!(map.get_event_at_position(Position::new(0, 0)).is_none());
/// ```
pub fn get_event_at_position(&self, position: Position) -> Option<&MapEvent> {
    self.get_event(position)
}
````

**Note:** This is an alias to `get_event` for explicit naming clarity in game systems.

#### 1.3 Integrate Foundation Work

**Update event dispatch code (if needed):**

1. **File:** `antares/src/game/systems/events.rs`
2. **Function:** `check_for_events` (L25-43)
3. **Current code already correct** - uses `map.get_event(current_pos)`, no changes needed
4. **Action:** Verify no references to `tile.event_trigger` exist

**Verification command:**

```bash
# Must return 0 matches in event systems:
grep -n "event_trigger" antares/src/game/systems/events.rs
```

#### 1.4 Testing Requirements

**Unit Tests to Add:**

1. **File:** `antares/src/domain/world/types.rs` (in `mod tests` section)
2. **Test functions to create:**

```rust
#[test]
fn test_map_get_event_at_position_returns_event() {
    // Arrange
    let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    let pos = Position::new(5, 5);
    let event = MapEvent::Sign {
        name: "Test Sign".to_string(),
        description: "A test sign".to_string(),
        text: "Hello, World!".to_string(),
    };
    map.add_event(pos, event.clone());

    // Act
    let result = map.get_event_at_position(pos);

    // Assert
    assert!(result.is_some());
    match result.unwrap() {
        MapEvent::Sign { text, .. } => assert_eq!(text, "Hello, World!"),
        _ => panic!("Expected Sign event"),
    }
}

#[test]
fn test_map_get_event_at_position_returns_none_when_no_event() {
    // Arrange
    let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    let pos = Position::new(5, 5);

    // Act
    let result = map.get_event_at_position(pos);

    // Assert
    assert!(result.is_none());
}
```

3. **File:** `antares/src/game/systems/events.rs` (add `mod tests` if not present)
4. **Integration test to create:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::{Map, MapEvent};
    use crate::domain::types::Position;
    use crate::game::state::GlobalState;
    use bevy::prelude::*;

    #[test]
    fn test_event_triggered_when_party_moves_to_event_position() {
        // Arrange
        let mut app = App::new();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(event_pos, MapEvent::Sign {
            name: "Test".to_string(),
            description: "Test sign".to_string(),
            text: "You found it!".to_string(),
        });

        let mut global_state = GlobalState::default();
        global_state.world.add_map(map);
        global_state.world.set_party_position(event_pos);

        app.insert_resource(global_state);

        // Act
        app.update();

        // Assert
        let events = app.world.resource::<Events<MapEventTriggered>>();
        let mut reader = events.get_reader();
        assert!(reader.read(events).count() > 0);
    }
}
```

**Regression Tests:**

1. **File:** `antares/src/domain/world/movement.rs`
2. **Action:** DELETE or UPDATE test `test_trigger_tile_event_exists` (L393-400) - this test uses removed functionality
3. **Option A (recommended):** Delete entire test
4. **Option B:** Replace with test that verifies events work via position lookup

#### 1.5 Deliverables

- [ ] `Tile.event_trigger` field removed from `antares/src/domain/world/types.rs` L85
- [ ] `event_trigger: None` initialization removed from `Tile::new` L114
- [ ] `trigger_tile_event` function deleted from `antares/src/domain/world/movement.rs` L191-199
- [ ] Doc comments referencing `event_trigger` removed (L191-192 in movement.rs)
- [ ] `Map::get_event_at_position` helper added to types.rs
- [ ] Unit tests added: `test_map_get_event_at_position_returns_event`, `test_map_get_event_at_position_returns_none_when_no_event`
- [ ] Integration test added: `test_event_triggered_when_party_moves_to_event_position`
- [ ] Test `test_trigger_tile_event_exists` removed/updated in movement.rs
- [ ] SPDX headers verified in all modified files

#### 1.6 Success Criteria

**Automated Validation Commands:**

```bash
# 1. No event_trigger references in src/ (MUST return 0):
grep -r "event_trigger" antares/src/ | wc -l

# Expected output: 0

# 2. Compilation succeeds with no errors:
cargo check --all-targets --all-features

# Expected: "Finished ... with 0 errors"

# 3. Linting passes with no warnings:
cargo clippy --all-targets --all-features -- -D warnings

# Expected: "Finished ... with 0 warnings"

# 4. All tests pass:
cargo nextest run --all-features

# Expected: "test result: ok. N passed; 0 failed"

# 5. Verify Tile struct has no event_trigger field (compile-time check):
# This is verified by cargo check passing
```

**Manual Verification:**

- [ ] Search results for `event_trigger` in `antares/src/` return zero matches
- [ ] `Tile::new` function has 9 fields (not 10)
- [ ] `Map::get_event_at_position` method exists and is public
- [ ] New unit tests appear in test output

### Phase 2: Feature Implementation - Editor & Data Migration

#### 2.1 Feature Work - Map Editor Updates

**CRITICAL:** The map editor has 20+ references to `tile.event_trigger` that must be removed.

**File:** `antares/sdk/campaign_builder/src/map_editor.rs`

**Changes Required (in order):**

**1. Remove `next_available_event_id` function (L458-466):**

This function scans tiles for `event_trigger` to find next ID. DELETE entire function.

```rust
// DELETE THIS ENTIRE FUNCTION:
fn next_available_event_id(&self) -> EventId {
    self.map
        .tiles
        .iter()
        .filter_map(|t| t.event_trigger)
        .max()
        .map(|id| id + 1)
        .unwrap_or(1)
}
```

**2. Update `add_event` function (L469-502):**

Current code reads `tile.event_trigger` and assigns event IDs to tiles. Remove this logic.

**BEFORE (L476-491):**

```rust
let existing_id = if let Some(tile) = self.map.get_tile(pos) {
    tile.event_trigger
} else {
    None
};

let id = existing_id.unwrap_or_else(|| self.next_available_event_id());

// Set the event_trigger on the tile
if let Some(tile) = self.map.get_tile_mut(pos) {
    tile.event_trigger = Some(id);
}
```

**AFTER:**

```rust
// Event ID no longer stored on tiles - Map.events is canonical
```

**3. Update `remove_event` function (L505-521):**

Remove `tile.event_trigger.take()` logic.

**BEFORE (L507-509):**

```rust
let event_id = if let Some(tile) = self.map.get_tile_mut(pos) {
    tile.event_trigger.take()
} else {
    None
};
```

**AFTER:**

```rust
// Event removed from Map.events, no tile cleanup needed
let event_id: Option<EventId> = None; // Keep for undo stack compatibility
```

**4. Update `apply_undo` function (L556-590):**

Remove tile event_trigger manipulation in undo operations.

**DELETE these sections:**

- L567-569: `tile.event_trigger = None;`
- L578-580: `tile.event_trigger = Some(id);`

**5. Update `apply_redo` function (L592-626):**

Remove tile event_trigger manipulation in redo operations.

**DELETE these sections:**

- L608-610: `tile.event_trigger = Some(id);`
- L615-617: `tile.event_trigger = None;`

**6. Update `load_maps` function (L3191-3272):**

Remove event ID backfilling logic (L3214-3232).

**DELETE this entire block:**

```rust
let mut next_id = map
    .tiles
    .iter()
    .filter_map(|t| t.event_trigger)
    .max()
    .map(|id| id + 1)
    .unwrap_or(1);

for pos in map.events.keys() {
    if let Some(tile) = map.get_tile_mut(pos) {
        if tile.event_trigger.is_none() {
            tile.event_trigger = Some(next_id);
            next_id += 1;
        }
    }
}
```

**7. Update `show_event_editor` comment (L2912-2918):**

Remove comment about preserving `tile.event_trigger`.

**BEFORE:**

```rust
// Replace the event in-place (preserve tile.event_trigger id).
```

**AFTER:**

```rust
// Replace the event in-place at this position.
```

**8. Update Tests:**

**DELETE/UPDATE these test functions:**

- `test_undo_redo_event_id_preserved` (L3752-3786) - tests tile.event_trigger preservation
- `test_load_maps_backfills_event_ids` (L3789-3838) - tests backfilling logic
- `test_edit_event_replaces_existing_event` (L3976-4012) - references tile.event_trigger

**For each test:**

- Remove assertions checking `tile.event_trigger`
- Focus on verifying `Map.events` contains correct data
- Keep event position and type validation

**Example update for `test_undo_redo_event_id_preserved`:**

```rust
#[test]
fn test_undo_redo_event_preserved() {
    // Renamed: no longer testing event_trigger ID
    let mut state = MapEditorState::new(Map::new(
        1,
        "UndoRedo Map".to_string(),
        "Desc".to_string(),
        10,
        10,
    ));
    let pos = Position::new(3, 3);
    let event = MapEvent::Sign {
        name: "Test".to_string(),
        description: "Desc".to_string(),
        text: "Hello".to_string(),
    };

    state.add_event(pos, event.clone());
    assert!(state.map.get_event(pos).is_some());

    state.undo();
    assert!(state.map.get_event(pos).is_none());

    state.redo();
    assert!(state.map.get_event(pos).is_some());
    // Verify event data matches
    match state.map.get_event(pos).unwrap() {
        MapEvent::Sign { text, .. } => assert_eq!(text, "Hello"),
        _ => panic!("Wrong event type"),
    }
}
```

**UI Updates:**

The editor already has an Events panel via `show_event_editor` (L2642+) and inspector (L2395-2534). The inspector shows events at selected positions but does NOT show a per-tile `event_trigger` field - it shows event details from `Map.events`. **No UI widget removal needed** - only backend code changes above.

**Verify no UI shows event_trigger:**

```bash
grep -n "event_trigger" antares/sdk/campaign_builder/src/map_editor.rs
# After changes: should return 0 results
```

#### 2.2 Integrate Feature - Map Data Migration

**Files to migrate:**

1. **`antares/campaigns/tutorial/data/maps/map_1.ron`**

**Current state:** Every tile has `event_trigger: None` (400+ occurrences)

**Migration process:**

1. Create backup: `cp map_1.ron map_1.ron.backup`
2. Remove all `event_trigger: None,` lines from tiles
3. Ensure `events: []` list exists in map structure (add if missing)
4. If any tiles had `event_trigger: Some(id)`, convert to event in `events` list

**Automated migration script (recommended):**

**File to create:** `antares/sdk/campaign_builder/src/bin/migrate_maps.rs`

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Migration tool to remove event_trigger from map RON files
//!
//! Usage: cargo run --bin migrate_maps -- <map_file_path>

use std::fs;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Path to map RON file to migrate
    map_file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Migrating map file: {}", args.map_file.display());

    // Read file
    let content = fs::read_to_string(&args.map_file)?;

    // Remove event_trigger lines
    let migrated = content
        .lines()
        .filter(|line| !line.trim().starts_with("event_trigger:"))
        .collect::<Vec<_>>()
        .join("\n");

    // Create backup
    let backup_path = args.map_file.with_extension("ron.backup");
    fs::copy(&args.map_file, &backup_path)?;
    println!("Backup created: {}", backup_path.display());

    // Write migrated content
    fs::write(&args.map_file, migrated)?;
    println!("Migration complete!");

    Ok(())
}
```

**Add to `antares/sdk/campaign_builder/Cargo.toml`:**

```toml
[[bin]]
name = "migrate_maps"
path = "src/bin/migrate_maps.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

**Run migration:**

```bash
cd antares/sdk/campaign_builder
cargo run --bin migrate_maps -- ../../campaigns/tutorial/data/maps/map_1.ron
```

#### 2.3 Configuration Updates

**Documentation to update:**

1. **Create:** `antares/docs/explanation/map_event_system.md`

````markdown
# Map Event System

## Overview

Events in Antares maps use a single, canonical position-based event model.

## Event Definition Format

Events are defined in map RON files in the `events` list:

```ron
MapBlueprint(
    id: 1,
    name: "Tutorial Town",
    // ... other fields ...
    events: [
        MapEventBlueprint(
            position: Position(x: 5, y: 5),
            name: "Welcome Sign",
            description: "A wooden sign",
            event_type: Text("Welcome to Antares!"),
        ),
        MapEventBlueprint(
            position: Position(x: 10, y: 3),
            name: "Treasure Chest",
            description: "A locked chest",
            event_type: Treasure([
                LootItem(item_id: 1, quantity: 50),  // 50 gold
                LootItem(item_id: 42, quantity: 1),  // Magic sword
            ]),
        ),
    ],
)
```
````

## Event Types

- `Text(String)` - Display message to player
- `Treasure(Vec<LootItem>)` - Award items/gold
- `Combat(Vec<MonsterSpawn>)` - Trigger encounter
- `Teleport { map_id, x, y }` - Transport party
- `Trap { damage, effect }` - Deal damage/apply condition
- `NpcDialogue(u16)` - Start conversation

## Runtime Behavior

When the party moves to a position, the game queries `Map.events` by position.
If an event exists, a `MapEventTriggered` message is sent to event handlers.

## Migration from Tile Event Triggers

Old format (deprecated, DO NOT USE):

```ron
Tile(
    x: 5,
    y: 5,
    event_trigger: Some(42),  // REMOVED - don't use this
)
```

New format (use this):

```ron
events: [
    MapEventBlueprint(
        position: Position(x: 5, y: 5),
        event_type: Text("Event content here"),
        // ...
    ),
]
```

2. **Update:** `antares/docs/how-to/creating_and_validating_campaigns.md`

Add section on creating events in maps with examples.

3. **Update:** `antares/docs/explanation/implementations.md`

Add entry documenting this implementation:

````markdown
## Remove Per-Tile Event Triggers

**Date:** 2025-01-XX
**Status:** Completed

Consolidated event representation to single canonical model: `Map.events: HashMap<Position, MapEvent>`.
Removed deprecated `Tile.event_trigger` field and related code.

**Changes:**

- Removed `Tile.event_trigger` field from core types
- Deleted `trigger_tile_event` function from movement module
- Updated map editor to use Events panel (no per-tile triggers)
- Migrated all tutorial maps to new format
- Added validation to reject maps with event_trigger

**Benefits:**

- Single source of truth for events eliminates sync issues
- Reduced map file size (no event_trigger: None on every tile)
- Clearer event management in map editor

#### 2.4 Testing Requirements

**Editor Tests:**

1. **File:** `antares/sdk/campaign_builder/tests/map_editor_tests.rs` (create if not exists)

```rust
#[test]
fn test_editor_can_add_event_to_map() {
    // Test that Events panel can add new event
    // Verify event appears in events list
    // Verify event is serialized to RON correctly
}

#[test]
fn test_editor_serialization_excludes_event_trigger() {
    // Create map with events
    // Save to RON
    // Verify RON contains no "event_trigger:" strings
}

#[test]
fn test_editor_can_edit_existing_event() {
    // Load map with event
    // Edit event properties
    // Verify changes saved correctly
}

#[test]
fn test_editor_can_delete_event() {
    // Load map with event
    // Delete event
    // Verify event removed from events list and RON
}
```
````

**Migration Tests:**

1. **File:** `antares/sdk/campaign_builder/tests/migration_tests.rs` (create new)

```rust
#[test]
fn test_migration_removes_event_trigger_lines() {
    // Create temp file with event_trigger entries
    // Run migration
    // Verify no event_trigger lines remain
    // Verify valid RON structure preserved
}

#[test]
fn test_migrated_map_loads_correctly() {
    // Migrate a test map
    // Load into Map struct
    // Verify all fields correct
    // Verify events present if any were defined
}
```

**End-to-End Tests:**

1. **File:** `antares/tests/integration_tests.rs`

```rust
#[test]
fn test_map_with_events_loads_and_triggers_correctly() {
    // Load tutorial map
    // Move party to event position
    // Verify MapEventTriggered fires
    // Verify event content matches definition
}
```

#### 2.5 Deliverables

- [ ] Map editor Events panel implemented (add/edit/delete functionality)
- [ ] Per-tile `event_trigger` UI field removed from tile inspector
- [ ] Migration script `migrate_maps.rs` created and tested
- [ ] All tutorial maps migrated (`campaigns/tutorial/data/maps/*.ron`)
- [ ] Backup files created for all migrated maps (\*.ron.backup)
- [ ] Documentation created: `docs/explanation/map_event_system.md`
- [ ] Documentation updated: `docs/how-to/creating_and_validating_campaigns.md`
- [ ] Documentation updated: `docs/explanation/implementations.md`
- [ ] Editor tests added (4 test functions)
- [ ] Migration tests added (2 test functions)
- [ ] Integration test added
- [ ] SPDX headers added to new files

#### 2.6 Success Criteria

**Automated Validation:**

```bash
# 1. No event_trigger in SDK code (MUST return 0):
grep -r "event_trigger" antares/sdk/campaign_builder/src/map_editor.rs | wc -l

# Expected: 0

# 2. No event_trigger in tutorial maps (MUST return 0):
grep -r "event_trigger:" antares/campaigns/tutorial/data/maps/*.ron | wc -l

# Expected: 0

# 2. All maps have events field:
grep -c "events:" antares/campaigns/tutorial/data/maps/map_1.ron

# Expected: 1 (or more if multiple maps)

# 3. Migration script compiles:
cargo build --bin migrate_maps

# Expected: successful build

# 4. All SDK tests pass:
cd antares/sdk/campaign_builder
cargo nextest run --all-features

# Expected: all tests pass

# 5. Integration tests pass:
cd antares
cargo nextest run --all-features

# Expected: all tests pass

# 6. Map editor compiles without warnings:
cd antares/sdk/campaign_builder
cargo clippy --all-targets --all-features -- -D warnings

# Expected: 0 warnings
```

**Manual Verification:**

- [ ] Open map editor, verify Events panel visible
- [ ] Click "Add Event", verify dialog opens with all fields
- [ ] Create event, save map, verify event in RON file
- [ ] Load map in game, walk to event position, verify event triggers
- [ ] Verify no `event_trigger:` tokens in any `.ron` files
- [ ] Verify backup files exist for all migrated maps

## Validation Checklist (Final Verification)

### Code Quality

- [ ] `cargo fmt --all` applied successfully
- [ ] `cargo check --all-targets --all-features` passes (0 errors)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes (0 warnings)
- [ ] `cargo nextest run --all-features` passes (100% tests passing)
- [ ] SPDX headers present in all new/modified `.rs` files
- [ ] All public items have `///` doc comments with examples

### Architecture Compliance

- [ ] No modifications to core data structures beyond removing `event_trigger`
- [ ] Type aliases used consistently (`EventId`, `MapId`, `Position`)
- [ ] No hardcoded constants (uses defined constants)
- [ ] RON format used for all map data files
- [ ] Module placement follows architecture.md Section 3.2

### Documentation

- [ ] `docs/explanation/implementations.md` updated with summary
- [ ] `docs/explanation/map_event_system.md` created
- [ ] `docs/how-to/creating_and_validating_campaigns.md` updated
- [ ] Filenames use `lowercase_with_underscores.md`
- [ ] All code blocks specify file paths (not language names)

### Testing

- [ ] Unit tests added for `Map::get_event_at_position`
- [ ] Integration test for event triggering via movement
- [ ] Editor tests for add/edit/delete events
- [ ] Migration tests for RON file transformation
- [ ] All test names follow pattern: `test_{function}_{condition}_{expected}`
- [ ] Test coverage >80% for modified code

### Data Migration

- [ ] All maps in `campaigns/tutorial/data/maps/` migrated
- [ ] Backup files created (\*.ron.backup)
- [ ] No `event_trigger:` tokens in any RON files
- [ ] All migrated maps load successfully in game
- [ ] All events from old format preserved in new format

### Automated Validation Commands

Run these commands in sequence. ALL must pass:

```bash
# 1. Format check
cargo fmt --all -- --check
# Expected: no output (already formatted)

# 2. Compilation
cargo check --all-targets --all-features
# Expected: "Finished ... with 0 errors"

# 3. Linting
cargo clippy --all-targets --all-features -- -D warnings
# Expected: "Finished ... with 0 warnings"

# 4. Tests
cargo nextest run --all-features
# Expected: "test result: ok. N passed; 0 failed"

# 5. Grep validation - event_trigger in source (MUST be 0)
grep -r "event_trigger" antares/src/ | wc -l
# Expected: 0

# 6. Grep validation - event_trigger in maps (MUST be 0)
grep -r "event_trigger:" antares/campaigns/ | wc -l
# Expected: 0

# 7. Verify events field exists in maps
grep -c "events:" antares/campaigns/tutorial/data/maps/map_1.ron
# Expected: >= 1

# 8. Documentation exists
ls antares/docs/explanation/map_event_system.md
# Expected: file exists
```

## Success Metrics

**Definition of Done:**

1. Zero references to `event_trigger` in `antares/src/` directory
2. Zero `event_trigger:` tokens in map RON files
3. All quality gates pass (fmt, check, clippy, nextest run)
4. Map editor Events panel functional (add/edit/delete)
5. Tutorial maps load and events trigger correctly in-game
6. All documentation updated
7. Migration script available and tested

**Rollback Plan:**

If critical issues found:

1. Restore map backups: `cp *.ron.backup *.ron`
2. Revert code changes via git
3. Re-run tests to verify rollback successful

**Risk Mitigation:**

- All changes in single PR/branch (no intermediate broken state)
- Backup files created before migration
- Comprehensive test coverage prevents regressions
- Migration script tested on copy before running on real maps

---

## AI Agent Execution Notes

**This plan is designed for automated execution. For each phase:**

1. Read the exact file paths and line numbers specified
2. Make ONLY the changes described (no creative additions)
3. Run the validation commands after each phase
4. If any validation fails, STOP and report the failure
5. Update deliverables checklist as items complete
6. Verify SPDX headers in all touched files

**Do not proceed to next phase until all deliverables and success criteria for current phase are met.**
