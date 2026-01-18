<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Character Definition Implementation Plan

## Overview

This plan introduces a data-driven character definition system following the
established patterns for classes, items, spells, and monsters. Characters will
be defined in `characters.ron` files per campaign, edited via the SDK Campaign
Builder, and instantiated into runtime `Character` objects by the Game Engine.

The key insight is separating **character templates** (data-driven definitions
in RON) from **character instances** (runtime state managed by the engine). This
keeps character creation logic in the SDK while the engine only maintains state
during gameplay and in saves.

## Current State Analysis

### Existing Infrastructure

The project has established patterns for data-driven entities:

| Entity         | Domain Types                       | Data File        | SDK Editor             |
| -------------- | ---------------------------------- | ---------------- | ---------------------- |
| Classes        | `ClassDefinition`, `ClassDatabase` | `classes.ron`    | `classes_editor.rs`    |
| Items          | `Item`                             | `items.ron`      | `items_editor.rs`      |
| Monsters       | `MonsterDefinition`                | `monsters.ron`   | `monsters_editor.rs`   |
| Spells         | `Spell`                            | `spells.ron`     | `spells_editor.rs`     |
| Conditions     | `ConditionDefinition`              | `conditions.ron` | `conditions_editor.rs` |
| Races          | `RaceDefinition` (partial)         | `races.ron`      | None                   |
| **Characters** | None                               | None             | None                   |

The runtime `Character` struct in `src/domain/character.rs` handles gameplay
state (HP, SP, inventory, conditions, etc.) but has no corresponding
`CharacterDefinition` for pre-made or template characters.

### Identified Issues

1. **No character templates**: Cannot define starting characters, NPCs, or
   pre-generated party members in campaign data files
2. **No SDK character editor**: Campaign designers cannot create/edit character
   definitions visually
3. **Hard-coded character creation**: `Character::new()` only accepts basic
   parameters with default stats
4. **Missing instantiation layer**: No mechanism to create a runtime `Character`
   from a data-driven template with starting equipment, items, and customized
   stats

## Implementation Phases

### Phase 1: Domain Types

Create the foundational types for character definitions in the domain layer.

#### 1.1 Create Character Definition Module

Create `src/domain/character_definition.rs` with:

- `CharacterDefinitionId` type alias (`String`)
- `CharacterDefinition` struct containing:
  - `id: CharacterDefinitionId` - Unique identifier (e.g., "pregen_human_knight")
  - `name: String` - Character display name
  - `race_id: RaceId` - Reference to races.ron
  - `class_id: ClassId` - Reference to classes.ron
  - `sex: Sex` - Character sex (reuse existing enum)
  - `alignment: Alignment` - Starting alignment (reuse existing enum)
  - `base_stats: Stats` - Starting stats (before race/class modifiers)
  - `portrait_id: String` - Portrait/avatar identifier (filename stem; normalized to lowercase with spaces replaced by underscores; empty string `""` indicates no portrait)
  - `starting_gold: u32` - Initial gold amount
  - `starting_items: Vec<ItemId>` - Items to add to inventory
  - `starting_equipment: StartingEquipment` - Items to equip
  - `description: String` - Character backstory/bio
  - `is_premade: bool` - Distinguishes pre-made vs template characters
- `StartingEquipment` struct for slot-based starting gear
- `CharacterDefinitionError` enum for validation errors
- `CharacterDatabase` struct with `load_from_file()`, `get_character()`,
  `all_characters()`, `validate()` methods

#### 1.2 Add Serde Support

Implement `Serialize` and `Deserialize` for all new types with appropriate
`#[serde(default)]` annotations for optional fields.

#### 1.3 Export from Domain Module

Update `src/domain/mod.rs` to export the new `character_definition` module and
its public types.

#### 1.4 Testing Requirements

- Unit tests for `CharacterDefinition` creation and field access
- Unit tests for `CharacterDatabase::load_from_string()` with valid RON
- Unit tests for validation (missing race_id, invalid class_id, duplicate IDs)
- Unit tests for `StartingEquipment` serialization/deserialization

#### 1.5 Deliverables

- `src/domain/character_definition.rs` with complete type definitions
- Updated `src/domain/mod.rs` exports
- Comprehensive unit test coverage (>80%)

#### 1.6 Success Criteria

- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test --all-features` passes with all new tests
- Types match patterns established in `classes.rs`

### Phase 2: Data Files

Create RON data files with character definitions for core data and campaigns.

#### 2.1 Create Core Characters Data File

Create `data/characters.ron` with example pre-made characters:

- One character per class (6 total) as starting party options
- Balanced stats appropriate for level 1
- Basic starting equipment referencing `data/items.ron`
- Descriptive backstories

#### 2.2 Create Campaign Characters Data File

Create `campaigns/tutorial/data/characters.ron` with:

- Tutorial-specific pre-made characters
- NPCs that can join the party
- Template characters for the campaign

#### 2.3 Validate Data File Format

Ensure RON files:

- Follow established patterns from `classes.ron` and `items.ron`
- Include helpful comments explaining the format
- Reference valid IDs from races.ron, classes.ron, and items.ron

#### 2.4 Testing Requirements

- Integration test loading `data/characters.ron`
- Integration test loading `campaigns/tutorial/data/characters.ron`
- Validation tests for cross-references (race_id, class_id, item_id existence)

#### 2.5 Deliverables

- `data/characters.ron` with 6+ pre-made characters
- `campaigns/tutorial/data/characters.ron` with tutorial characters
- Integration tests for data file loading

#### 2.6 Success Criteria

- Both RON files parse successfully via `CharacterDatabase::load_from_file()`
- All referenced IDs exist in corresponding data files
- Data file comments are clear and follow existing patterns

### Phase 3: SDK Integration

Integrate character definitions into the SDK content database and campaign
loader.

#### 3.1 Update Content Database

Modify `src/sdk/database.rs`:

- Add `pub characters: CharacterDatabase` field to `ContentDatabase`
- Update `ContentDatabase::new()` to initialize empty `CharacterDatabase`
- Update `ContentDatabase::load_campaign()` to load `characters.ron`
- Update `ContentDatabase::load_core()` to load core characters
- Update `ContentDatabase::validate()` to validate character references
- Update `ContentStats` to include `character_count`

#### 3.2 Update Campaign Loader

Modify `src/sdk/campaign_loader.rs`:

- Add character loading to campaign load process
- Validate character definitions against loaded races, classes, and items

#### 3.3 Add Validation Rules

Implement validation in `sdk/campaign_builder/src/validation.rs`:

- Character references valid race_id
- Character references valid class_id
- Starting items reference valid item IDs
- Starting equipment references valid item IDs
- No duplicate character definition IDs

#### 3.4 Testing Requirements

- Unit tests for `ContentDatabase` character loading
- Integration tests for campaign loading with characters
- Validation tests for invalid references

#### 3.5 Deliverables

- Updated `src/sdk/database.rs` with character support
- Updated `src/sdk/campaign_loader.rs` with character loading
- Updated validation rules in Campaign Builder
- Test coverage for new functionality

#### 3.6 Success Criteria

- `ContentDatabase::load_campaign()` successfully loads characters
- Validation catches missing/invalid references
- SDK tests pass with new functionality

### Phase 4: SDK Character Editor

Create a visual editor for character definitions in the Campaign Builder.

#### 4.1 Create Editor State

Create `sdk/campaign_builder/src/characters_editor.rs` with:

- `CharactersEditorMode` enum (List, Add, Edit)
- `CharactersEditorState` struct with search, filters, edit buffer
- Filter options: by race, by class, by alignment, premade only

#### 4.2 Implement Editor UI

Following the pattern from `items_editor.rs`:

- List view with search and filters
- Add/Edit form with all `CharacterDefinition` fields
- Race and class dropdowns populated from loaded databases
- Starting items/equipment selection with item picker
- Stats editor with validation (use existing stat range constants)
- Portrait ID selector

#### 4.3 Integrate into Campaign Builder

Update `sdk/campaign_builder/src/main.rs`:

- Add "Characters" tab to the editor
- Wire up `CharactersEditorState` to app state
- Add save/load for characters.ron
- Add import/export functionality

#### 4.4 Testing Requirements

- Unit tests for `CharactersEditorState` default values
- Unit tests for filter logic
- Integration tests for save/load round-trip

#### 4.5 Deliverables

- `sdk/campaign_builder/src/characters_editor.rs` with full editor
- Updated `sdk/campaign_builder/src/main.rs` with Characters tab
- Test coverage for editor functionality

#### 4.6 Success Criteria

- Characters tab appears in Campaign Builder
- Can create, edit, delete character definitions
- Changes save to `characters.ron` in correct format
- Filters and search work correctly

### Phase 5: Character Instantiation

Implement the mechanism to create runtime `Character` objects from definitions.

#### 5.1 Add Instantiation Method

Add to `CharacterDefinition` in `src/domain/character_definition.rs`:

- `instantiate()` method that creates a `Character` from the definition
- Accepts `ClassDatabase`, `RaceDatabase`, `ItemDatabase` for lookups
- Applies race stat modifiers (when race system is complete)
- Applies class starting bonuses
- Populates inventory with starting items
- Equips starting equipment
- Sets gold, portrait, and other fields

#### 5.2 Add Instantiation Helpers

Create helper functions:

- `apply_race_modifiers()` - Apply race stat bonuses (placeholder until race
  system complete)
- `apply_class_bonuses()` - Apply class-specific starting bonuses
- `populate_starting_inventory()` - Add items to character inventory
- `equip_starting_gear()` - Equip items in appropriate slots

#### 5.3 Integration Points

Identify where instantiation will be called:

- New game character selection
- NPC recruitment
- Test/debug character creation

#### 5.4 Testing Requirements

- Unit tests for `instantiate()` with mock databases
- Tests for stat modifier application
- Tests for inventory population
- Tests for equipment assignment
- Edge case tests (invalid item IDs, full inventory)

#### 5.5 Deliverables

- `CharacterDefinition::instantiate()` method
- Helper functions for instantiation steps
- Comprehensive test coverage

#### 5.6 Success Criteria

- Can create a fully functional `Character` from any `CharacterDefinition`
- Starting items appear in inventory
- Starting equipment is equipped
- Stats reflect race/class modifiers
- All tests pass

### Phase 6: Documentation and Cleanup

Complete documentation and finalize the implementation.

#### 6.1 Update Architecture Documentation

Update `docs/reference/architecture.md`:

- Add `CharacterDefinition` to data structures section
- Document `characters.ron` format in data files section
- Add character instantiation to game flow

#### 6.2 Update Implementation Documentation

Update `docs/explanation/implementations.md`:

- Document character definition system
- Explain separation of definition vs instance
- Provide usage examples

#### 6.3 Add How-To Guide

Create `docs/how-to/create_characters.md`:

- Step-by-step guide for creating character definitions
- Examples of different character types (premade, template, NPC)
- Tips for balancing starting stats and equipment

#### 6.4 Code Cleanup

- Remove any TODO comments from implementation
- Ensure consistent code style
- Verify all doc comments are complete

#### 6.5 Deliverables

- Updated architecture documentation
- Updated implementation documentation
- New how-to guide for character creation
- Clean, documented codebase

#### 6.6 Success Criteria

- Documentation is complete and accurate
- All public APIs have doc comments with examples
- Code passes all quality checks

## Dependencies

This plan has the following dependencies:

- **Race System**: Full race modifier application in Phase 5 depends on the race
  system implementation (can use placeholder until complete)
- **Hard-coded Removal**: While this plan works with current enums, full
  data-driven character creation benefits from the hard-coded removal plan

## Open Questions

1. **Portrait System**: How are portraits stored/referenced? Current plan uses
   `portrait_id: u8` but may need path-based system for custom portraits.

2. **NPC vs Player Characters**: Should there be a distinction in the
   definition? Current plan uses `is_premade` flag but NPCs may need additional
   fields (dialogue_id, location, etc.).

3. **Level Scaling**: Should `CharacterDefinition` support defining characters
   at levels other than 1? May need `starting_level` field and scaled HP/SP.

## File Summary

| File                                            | Action | Description                |
| ----------------------------------------------- | ------ | -------------------------- |
| `src/domain/character_definition.rs`            | Create | Domain types               |
| `src/domain/mod.rs`                             | Modify | Export new module          |
| `data/characters.ron`                           | Create | Core character definitions |
| `campaigns/tutorial/data/characters.ron`        | Create | Tutorial characters        |
| `src/sdk/database.rs`                           | Modify | Add CharacterDatabase      |
| `src/sdk/campaign_loader.rs`                    | Modify | Load characters            |
| `sdk/campaign_builder/src/characters_editor.rs` | Create | Visual editor              |
| `sdk/campaign_builder/src/main.rs`              | Modify | Add Characters tab         |
| `sdk/campaign_builder/src/validation.rs`        | Modify | Character validation       |
| `docs/reference/architecture.md`                | Modify | Document types             |
| `docs/explanation/implementations.md`           | Modify | Document system            |
| `docs/how-to/create_characters.md`              | Create | Usage guide                |
