# NPC Data Externalization Implementation Plan

## Overview

This plan moves NPC data from being embedded within map files to standalone `npcs.ron` files. NPCs will be referenceable by ID from maps, and can link to dialogues and quests by their IDs. This improves:

- **Reusability**: The same NPC definition can appear on multiple maps
- **Separation of Concerns**: NPC data (name, appearance, behavior) separate from placement (map position)
- **Rich Linking**: NPCs link to dialogue trees and quests via ID references
- **Editor UX**: SDK can offer NPC picking/search rather than inline definition

## Current State Analysis

### Existing Infrastructure

| Component               | Location                                 | Current State                                                         |
| ----------------------- | ---------------------------------------- | --------------------------------------------------------------------- |
| `Npc` struct            | `src/domain/world/types.rs:222-234`      | Has `id`, `name`, `description`, `position`, `dialogue` (inline text) |
| `NpcBlueprint`          | `src/domain/world/blueprint.rs`          | Has `dialogue_id: Option<String>` for loading                         |
| `Map` struct            | `src/domain/world/types.rs:280-300`      | Contains `npcs: Vec<Npc>` (embedded)                                  |
| `MapEvent::NpcDialogue` | `src/domain/world/types.rs:206-215`      | Triggers dialogue by `npc_id: u16`                                    |
| `DialogueDatabase`      | `src/sdk/database.rs:645`                | Loads from `dialogues.ron`, supports `associated_quest`               |
| `ContentDatabase`       | `src/sdk/database.rs:800-830`            | No `npcs` database currently                                          |
| SDK Map Editor          | `sdk/campaign_builder/src/map_editor.rs` | Has `PlaceNpc` tool, creates inline NPC data                          |

### Identified Issues

1. **NPCs embedded in maps**: NPC definitions are duplicated if the same NPC appears on multiple maps
2. **Dialogue is inline text**: `Npc.dialogue` is a String, not a reference to a `DialogueTree`
3. **No quest linkage**: NPCs cannot be easily associated with quests they give/complete
4. **No NpcDatabase**: Unlike items, monsters, spells - NPCs lack a dedicated database

## Implementation Phases

---

### Phase 1: Core Domain Module

**Objective**: Create `NpcDefinition` struct and `NpcDatabase` for loading NPC data from RON files.

#### 1.1 Create NPC Definition Struct

**File**: `src/domain/world/npc.rs` (NEW)

Create a new NPC definition struct with proper ID-based references:

- `NpcId` type alias (`pub type NpcId = String`)
- `NpcDefinition` struct with fields:
  - `id: NpcId` - Unique identifier (e.g., "village_elder", "merchant_tom")
  - `name: String` - Display name
  - `description: String` - Description for tooltips/inspection
  - `portrait_path: String` - Path to portrait image (required)
  - `dialogue_id: Option<DialogueId>` - Reference to `DialogueTree.id`
  - `quest_ids: Vec<QuestId>` - Quests this NPC gives or is involved with
  - `faction: Option<String>` - Optional faction affiliation
  - `is_merchant: bool` - If true, can open shop interface
  - `is_innkeeper: bool` - If true, can rest party

#### 1.2 Create NPC Placement Struct

**File**: `src/domain/world/npc.rs`

Create a lightweight placement struct for maps:

- `NpcPlacement` struct with fields:
  - `npc_id: NpcId` - Reference to `NpcDefinition.id`
  - `position: Position` - Where on the map
  - `facing: Option<Direction>` - Which way NPC faces
  - `dialogue_override: Option<DialogueId>` - Override default dialogue for this placement

#### 1.3 Update Map Struct

**File**: `src/domain/world/types.rs`

Modify `Map` struct:

- Change `npcs: Vec<Npc>` to `npc_placements: Vec<NpcPlacement>`
- Keep existing `Npc` struct temporarily for backward compatibility during migration

#### 1.4 Create NpcDatabase

**File**: `src/sdk/database.rs` (add to existing)

Add `NpcDatabase` following the existing pattern:

- `NpcDatabase` struct with `HashMap<NpcId, NpcDefinition>`
- `load_from_file()` method
- `get_npc()`, `get_npc_by_name()`, `all_npcs()`, `count()` methods

#### 1.5 Update ContentDatabase

**File**: `src/sdk/database.rs`

Add NPC database to `ContentDatabase`:

- Add field `pub npcs: NpcDatabase`
- Update `ContentDatabase::new()` to initialize empty NpcDatabase
- Update `load_campaign()` to load from `data/npcs.ron`
- Update `load_core()` similarly

#### 1.6 Testing Requirements

- Unit tests for `NpcDefinition` serialization/deserialization
- Unit tests for `NpcDatabase::load_from_file()`
- Unit tests for `NpcPlacement` serialization
- Integration test: Load tutorial campaign NPCs

**Command**: `cargo nextest run --all-features`

#### 1.7 Deliverables

- [ ] `src/domain/world/npc.rs` - New module with `NpcId`, `NpcDefinition`, `NpcPlacement`
- [ ] `src/domain/world/mod.rs` - Export new npc module
- [ ] `src/sdk/database.rs` - Add `NpcDatabase`, update `ContentDatabase`
- [ ] Tests achieving >80% coverage for new code

#### 1.8 Success Criteria

- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes

---

### Phase 2: Data File Creation

**Objective**: Create `npcs.ron` data files for both global data and tutorial campaign.

#### 2.1 Create Global Data File

**File**: `data/npcs.ron` (NEW)

Create a base `npcs.ron` with standard NPC archetypes:

```ron
[
    (
        id: "generic_merchant",
        name: "Merchant",
        description: "A traveling merchant selling goods",
        portrait_path: "assets/portraits/merchant.png",
        dialogue_id: Some(1),  // References dialogues.ron
        quest_ids: [],
        faction: None,
        is_merchant: true,
        is_innkeeper: false,
    ),
    // ... more generic NPCs
]
```

#### 2.2 Create Tutorial Campaign Data File

**File**: `campaigns/tutorial/data/npcs.ron` (NEW)

Extract NPCs from tutorial maps:

- Village Elder (from map_1.ron)
- InnKeeper (from map_1.ron)
- Merchant (from map_1.ron)
- High Priestess (from map_1.ron)
- NPCs from other maps (map_2, map_4, map_5, map_6)

Link to existing dialogues where applicable.

#### 2.3 Deliverables

- [ ] `data/npcs.ron` - Global NPC definitions
- [ ] `campaigns/tutorial/data/npcs.ron` - Tutorial campaign NPCs

#### 2.4 Success Criteria

- RON files parse without errors
- NPCs reference valid dialogue IDs from `dialogues.ron`

---

### Phase 3: SDK Campaign Builder Updates - COMPLETED

**Objective**: Update the SDK to support the new NPC data model with a dedicated NPC Editor.

#### 3.1 Create NPC Editor Module

**File**: `sdk/campaign_builder/src/npc_editor.rs` (NEW)

Create NPC editor with:

- List view showing all NPCs
- Add/Edit/Delete functionality
- Fields for all `NpcDefinition` properties
- Autocomplete for `dialogue_id` (from loaded dialogues)
- Multi-select for `quest_ids` (from loaded quests)

#### 3.2 Update Map Editor

**File**: `sdk/campaign_builder/src/map_editor.rs`

Update `PlaceNpc` tool:

- Change from creating inline NPC to selecting from `NpcDatabase`
- Show NPC picker with search/filter
- Store `NpcPlacement` instead of full `Npc`
- Add option to override dialogue for specific placement

#### 3.3 Update Main SDK

**File**: `sdk/campaign_builder/src/main.rs`

- Add NPC Editor tab
- Load/save NPCs alongside other data types
- Update file operations menu for npcs.ron

#### 3.4 Update Validation

**File**: `sdk/campaign_builder/src/validation.rs`

Add validation rules:

- NPC placements reference valid NPC IDs
- NPC dialogue_id references valid dialogue
- NPC quest_ids reference valid quests

#### 3.5 Testing Requirements

- Unit tests for NPC editor state
- Integration test: Create NPC, place on map, save/reload

**Command**: `cargo nextest run --all-features -p campaign_builder`

#### 3.6 Deliverables

- [x] `sdk/campaign_builder/src/npc_editor.rs` - New NPC editor module (already existed, fixed borrowing issue)
- [x] `sdk/campaign_builder/src/map_editor.rs` - Updated for NpcPlacement
- [x] `sdk/campaign_builder/src/main.rs` - NPC tab integration (already integrated, updated to pass NPCs)
- [x] `sdk/campaign_builder/src/validation.rs` - NPC validation rules
- [x] `sdk/campaign_builder/src/ui_helpers.rs` - Updated NPC candidate extraction

#### 3.7 Success Criteria

- [x] SDK launches without errors
- [x] Can create, edit, delete NPCs (NPC editor already existed)
- [x] Can place NPC references on maps (NPC placement picker implemented)
- [x] Validation catches invalid references (3 validation functions added with tests)
- [x] All tests pass (971/971)
- [x] No clippy warnings
- [x] Proper error handling

#### 3.8 Implementation Summary

**Date Completed:** 2025-01-26

**Changes Made:**

1. **Map Editor Updates** (`sdk/campaign_builder/src/map_editor.rs`):

   - Replaced inline NPC creation with NPC placement picker
   - Updated `EditorAction` enum: `NpcAdded` → `NpcPlacementAdded`
   - Changed `add_npc()` to `add_npc_placement()`
   - Updated all references from `map.npcs` to `map.npc_placements`
   - Added NPC picker UI with dropdown, facing, and dialogue override options
   - Updated `show()` signature to accept `npcs: &[NpcDefinition]` parameter

2. **Validation Module** (`sdk/campaign_builder/src/validation.rs`):

   - Added `validate_npc_placement_reference()` - validates NPC ID exists
   - Added `validate_npc_dialogue_reference()` - validates dialogue ID
   - Added `validate_npc_quest_references()` - validates quest IDs
   - Added 8 comprehensive tests for all validation functions

3. **Main SDK Integration** (`sdk/campaign_builder/src/main.rs`):

   - Updated map editor call to pass `&self.npc_editor_state.npcs`
   - Fixed `LogLevel::Warning` → `LogLevel::Warn`
   - Added missing `npcs_file` field to test data

4. **UI Helpers** (`sdk/campaign_builder/src/ui_helpers.rs`):

   - Updated `extract_npc_candidates()` to use `npc_placements`
   - Updated tests to use `NpcPlacement` instead of legacy `Npc`

5. **NPC Editor Fixes** (`sdk/campaign_builder/src/npc_editor.rs`):
   - Fixed borrowing issue in `show_list_view()` using deferred action pattern

**Test Results:**

- ✅ 971/971 tests passing
- ✅ Zero clippy warnings
- ✅ All quality checks passed

**Documentation:**

- ✅ Updated `docs/explanation/implementations.md` with comprehensive Phase 3 summary

---

### Phase 4: Engine Integration

**Objective**: Update the game engine to load NPCs from the database and resolve references at runtime.

#### 4.1 Update Map Loading

**File**: `src/domain/world/blueprint.rs`

Update `MapBlueprint` and `Map::from()`:

- `MapBlueprint.npcs` becomes `Vec<NpcPlacementBlueprint>`
- `NpcPlacementBlueprint` has `npc_id` and `position`
- Conversion resolves NPC data from database at runtime

#### 4.2 Update Event System

**File**: `src/game/systems/events.rs`

Update `MapEvent::NpcDialogue` handling:

- Look up NPC by ID from database
- Get dialogue_id from NpcDefinition
- Start dialogue tree with proper ID

#### 4.3 Update World Module

**File**: `src/domain/world/types.rs`

- Add method `Map::resolve_npcs(&self, npc_db: &NpcDatabase) -> Vec<ResolvedNpc>`
- `ResolvedNpc` contains merged placement + definition data

#### 4.4 Testing Requirements

- Integration test: Load tutorial campaign with new format
- Test NPC dialogue triggers correctly

**Command**: `cargo nextest run --all-features`

#### 4.5 Deliverables

- [ ] `src/domain/world/blueprint.rs` - Updated for NpcPlacement loading
- [ ] `src/game/systems/events.rs` - Updated dialogue triggering
- [ ] Updated tests

#### 4.6 Success Criteria

- Game engine loads campaign without errors
- NPC dialogues trigger correctly
- No regressions in existing functionality

---

### Phase 5: Data Migration & Cleanup - COMPLETED

**Objective**: Migrate tutorial campaign to new format and remove deprecated code.

#### 5.1 Migrate Tutorial Maps

**Files**: `campaigns/tutorial/data/maps/map_*.ron`

For each map:

- Extract NPC definitions to `npcs.ron` (already done in Phase 2)
- Replace `npcs: [...]` with `npc_placements: [...]`
- Each placement references NPC by ID

#### 5.2 Cleanup Deprecated Code

- Remove old `Npc` struct (replaced by `NpcDefinition`)
- Update all references to use new types

#### 5.3 Deliverables

- [x] Migrated map files (all 6 tutorial maps)
- [x] Cleaned up deprecated code (Npc struct, npcs field, validation code)

#### 5.4 Implementation Summary

**Maps Migrated**:

- Map 1 (Town Square): 4 NPC placements
- Map 2 (Fizban's Cave): 2 NPC placements
- Map 3 (Ancient Ruins): 0 NPC placements
- Map 4 (Dark Forest): 1 NPC placement
- Map 5 (Mountain Pass): 4 NPC placements
- Map 6 (Harrow Downs): 1 NPC placement

**Code Removed**:

- Legacy `Npc` struct and implementation
- `npcs` field from `Map` struct
- Legacy validation code in `src/sdk/validation.rs`
- Deprecated tests and examples
- ~200 lines of legacy code

**Files Updated**:

- Core: `src/domain/world/types.rs`, `src/domain/world/mod.rs`, `src/domain/world/blueprint.rs`
- SDK: `src/sdk/validation.rs`, `src/sdk/templates.rs`
- Binaries: `src/bin/map_builder.rs`, `src/bin/validate_map.rs`
- Examples: `examples/npc_blocking_example.rs`, `examples/generate_starter_maps.rs`
- Tests: `tests/map_content_tests.rs`

#### 5.5 Success Criteria

- [x] Tutorial campaign loads and plays correctly
- [x] All tests pass (971/971 passing)
- [x] No deprecated code warnings

---

## Verification Plan

### Automated Tests

| Test           | Command                                                    | Expected Result      |
| -------------- | ---------------------------------------------------------- | -------------------- |
| All unit tests | `cargo nextest run --all-features`                         | All tests pass       |
| Clippy lints   | `cargo clippy --all-targets --all-features -- -D warnings` | No warnings          |
| Format check   | `cargo fmt --all -- --check`                               | No formatting issues |
| SDK tests      | `cargo nextest run --all-features -p campaign_builder`     | All tests pass       |

### Manual Verification

1. **Load Tutorial Campaign**: Run game and verify tutorial loads without errors
2. **NPC Interaction**: Walk to an NPC and verify dialogue triggers
3. **SDK Workflow**: Open SDK, create new NPC, place on map, save, reload, verify persisted
4. **Cross-reference Validation**: In SDK, verify invalid NPC/dialogue/quest references are caught

---

## Design Decisions

1. **NPC ID Format**: String IDs (human-readable like "village_elder") for better readability and debugging.

2. **Legacy Support**: No backward compatibility - old embedded NPC format will not be supported. Maps must be migrated to new format.

3. **Portrait Assets**: Portraits are **required** fields with `portrait_path: String`. All NPCs must have a portrait.

---

## File Summary

| File                                     | Action | Description                             |
| ---------------------------------------- | ------ | --------------------------------------- |
| `src/domain/world/npc.rs`                | NEW    | NpcId, NpcDefinition, NpcPlacement      |
| `src/domain/world/mod.rs`                | MODIFY | Export npc module                       |
| `src/domain/world/types.rs`              | MODIFY | Update Map to use NpcPlacement          |
| `src/domain/world/blueprint.rs`          | MODIFY | Update for NpcPlacement loading         |
| `src/sdk/database.rs`                    | MODIFY | Add NpcDatabase, update ContentDatabase |
| `data/npcs.ron`                          | NEW    | Global NPC definitions                  |
| `campaigns/tutorial/data/npcs.ron`       | NEW    | Tutorial campaign NPCs                  |
| `campaigns/tutorial/data/maps/map_*.ron` | MODIFY | Replace inline NPCs with placements     |
| `sdk/campaign_builder/src/npc_editor.rs` | NEW    | NPC Editor module                       |
| `sdk/campaign_builder/src/map_editor.rs` | MODIFY | Update PlaceNpc tool                    |
| `sdk/campaign_builder/src/main.rs`       | MODIFY | Add NPC tab                             |
| `sdk/campaign_builder/src/validation.rs` | MODIFY | Add NPC validation rules                |
| `src/game/systems/events.rs`             | MODIFY | Update NPC dialogue handling            |

---

## Timeline Estimate

| Phase                        | Estimated Duration | Dependencies              |
| ---------------------------- | ------------------ | ------------------------- |
| Phase 1: Core Domain Module  | 2-3 days           | None                      |
| Phase 2: Data File Creation  | 0.5 days           | Phase 1                   |
| Phase 3: SDK Updates         | 2-3 days           | Phase 1                   |
| Phase 4: Engine Integration  | 1-2 days           | Phase 1, Phase 2          |
| Phase 5: Migration & Cleanup | 1 day              | Phase 2, Phase 3, Phase 4 |
| **Total**                    | **7-10 days**      |                           |

Phases 2 and 3 can run in parallel after Phase 1 completes.
