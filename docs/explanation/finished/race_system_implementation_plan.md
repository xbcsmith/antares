# Race System Implementation Plan

> **SUPERSEDED**: This plan has been merged into and superseded by
> [hardcoded_removal_implementation_plan.md](hardcoded_removal_implementation_plan.md).
> The race system was implemented as Phase 4 of that plan.
> This document is preserved for historical reference only.

## Overview

This document outlines a phased approach to implement a complete, data-driven race system for Antares. The goal is to create a `src/domain/races.rs` module following the established pattern in `classes.rs`, consolidate the three existing race definitions into a single source of truth, and integrate race mechanics into character creation, the SDK UI, and campaign data files.

The implementation will be forward-compatible with the proficiency migration plan by including `proficiencies` and `incompatible_item_tags` fields from the start.

## Current State Analysis

### Existing Infrastructure

Race is currently implemented in three separate locations with inconsistent definitions:

1. **Domain Layer** (`src/domain/character.rs` L400-406): A simple enum with no mechanical properties

   - `pub enum Race { Human, Elf, Dwarf, Gnome, HalfOrc }`
   - No stat modifiers, resistances, or special abilities

2. **SDK Database** (`src/sdk/database.rs` L102-145): A placeholder stub

   - `RaceDefinition` with only `id` and `name` fields
   - `RaceDatabase::load_from_file()` returns an empty database
   - Marked as "Phase 2 implementation pending"

3. **Race Editor CLI** (`src/bin/race_editor.rs` L44-86): Complete standalone implementation

   - Full `RaceDefinition` with `stat_modifiers`, `resistances`, `special_abilities`
   - Supporting types: `StatModifiers`, `Resistances`
   - Not shared with the game engine

4. **Data Files** (`data/races.ron`, `campaigns/tutorial/data/races.ron`): Minimal structure
   - Only `id` and `name` fields populated
   - No stat modifiers or abilities defined

### Identified Issues

1. **Type Duplication**: Three different `RaceDefinition` structs exist, creating maintenance burden and inconsistency
2. **No Game Integration**: Character creation ignores race mechanics entirely; `Character::new()` accepts `Race` enum but applies no modifiers
3. **Placeholder SDK**: `RaceDatabase` in SDK returns empty results, breaking race-related features
4. **Missing Domain Module**: No `src/domain/races.rs` exists despite architecture.md specifying `race.rs` in character module
5. **Incomplete Data Files**: RON files lack stat modifiers, resistances, and abilities
6. **No SDK UI Editor**: Campaign Builder has no races editor (unlike `classes_editor.rs`, `items_editor.rs`)
7. **Proficiency Incompatibility**: Current structure lacks fields needed for proficiency migration

## Implementation Phases

### Phase 1: Core Domain Module

Create the foundational `src/domain/races.rs` module following the `classes.rs` pattern.

#### 1.1 Create RaceError and Type Aliases

Add to new file `src/domain/races.rs`:

- `RaceError` enum with variants: `RaceNotFound`, `LoadError`, `ParseError`, `ValidationError`, `DuplicateId`
- `RaceId` type alias (`pub type RaceId = String`)

#### 1.2 Create Supporting Types

Add to `src/domain/races.rs`:

- `StatModifiers` struct with fields for all 7 attributes (`might`, `intellect`, `personality`, `endurance`, `speed`, `accuracy`, `luck`) as `i16` values
- `Resistances` struct with elemental resistance fields (`fire`, `cold`, `electricity`, `poison`, `energy`, `magic`) as `i16` values
- `SizeCategory` enum (`Small`, `Medium`, `Large`) with `Default` impl returning `Medium`
- Implement `Default` for `StatModifiers` and `Resistances` (all zeros)

#### 1.3 Create RaceDefinition Struct

Add `RaceDefinition` to `src/domain/races.rs` with fields:

- `id: RaceId` - Unique identifier
- `name: String` - Display name
- `description: String` - Flavor text (with `#[serde(default)]`)
- `stat_modifiers: StatModifiers` - Attribute adjustments
- `resistances: Resistances` - Elemental resistances
- `special_abilities: Vec<String>` - Racial traits
- `size_category: SizeCategory` - Physical size (with `#[serde(default)]`)
- `proficiencies: Vec<String>` - For proficiency migration (with `#[serde(default)]`)
- `incompatible_item_tags: Vec<String>` - Item restrictions (with `#[serde(default)]`)
- `disablement_bit: u8` - Legacy field for backward compatibility (with `#[serde(default)]`)

#### 1.4 Create RaceDatabase Struct

Add `RaceDatabase` to `src/domain/races.rs` with methods:

- `new()` - Creates empty database
- `load_from_file(path)` - Loads from RON file
- `load_from_string(data)` - Parses RON string
- `get_race(id)` - Returns `Option<&RaceDefinition>`
- `all_races()` - Returns all races as slice
- `validate()` - Validates database integrity (duplicate IDs, disablement bits)
- `len()` and `is_empty()` - Size helpers

#### 1.5 Export from Domain Module

Update `src/domain/mod.rs`:

- Add `pub mod races;`
- Add `pub use races::RaceId;`

#### 1.6 Testing Requirements

- Unit tests for `StatModifiers::default()` and `Resistances::default()`
- Unit tests for `RaceDatabase::load_from_string()` with valid RON
- Unit tests for `RaceDatabase::get_race()` success and failure cases
- Unit tests for `RaceDatabase::validate()` detecting duplicate IDs
- Unit tests for `RaceDatabase::validate()` detecting duplicate disablement bits
- Integration test loading from `data/races.ron`
- Test coverage >80%

#### 1.7 Deliverables

- [ ] `src/domain/races.rs` - Complete module with error types, structs, and database
- [ ] `src/domain/mod.rs` - Updated exports
- [ ] Tests achieving >80% coverage

#### 1.8 Success Criteria

- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes with no warnings
- `cargo test --all-features` passes
- `RaceDatabase` can load from RON file
- Validation detects duplicate IDs and disablement bits

### Phase 2: Data File Updates

Expand data files with full race definitions.

#### 2.1 Update Core Data File

Modify `data/races.ron` to include complete race definitions:

- Human: Balanced (no modifiers), Medium size
- Elf: +1 Intellect, +1 Speed, -1 Endurance, +10 magic resistance, "Infravision" ability
- Dwarf: +1 Might, +1 Endurance, -1 Speed, +10 poison resistance, "Stonecunning" ability
- Gnome: +1 Intellect, +1 Luck, -1 Might, Small size, "Tinker" ability
- Half-Elf: +1 Personality, "Adaptable" ability
- Half-Orc: +2 Might, -1 Intellect, -1 Personality, "Darkvision" ability

#### 2.2 Update Campaign Data File

Modify `campaigns/tutorial/data/races.ron` with same structure (can be simplified subset for tutorial).

#### 2.3 Validate Data Files Load

Add integration test verifying both data files load without errors.

#### 2.4 Testing Requirements

- Integration test for `data/races.ron` loading
- Integration test for `campaigns/tutorial/data/races.ron` loading
- Verify race counts match expected values
- Verify stat modifiers are applied correctly

#### 2.5 Deliverables

- [ ] `data/races.ron` - Complete race definitions with all fields
- [ ] `campaigns/tutorial/data/races.ron` - Tutorial-appropriate race definitions
- [ ] Integration tests for data file loading

#### 2.6 Success Criteria

- Both data files parse without errors
- All quality gates pass
- Race definitions include stat modifiers, resistances, abilities

### Phase 3: SDK Database Integration

Replace the SDK placeholder with real implementation.

#### 3.1 Update SDK Database Module

Modify `src/sdk/database.rs`:

- Remove placeholder `RaceDefinition` struct (L102-107)
- Remove placeholder `RaceDatabase` struct and impl (L111-145)
- Import from domain: `use crate::domain::races::{RaceDatabase, RaceDefinition, RaceId, RaceError};`
- Re-export types for SDK consumers

#### 3.2 Update ContentDatabase

Modify `src/sdk/database.rs` `ContentDatabase` struct:

- Update `races` field type to use domain `RaceDatabase`
- Update `load_campaign()` to call domain `RaceDatabase::load_from_file()`
- Update `load_core()` similarly
- Add race validation to `validate()` method

#### 3.3 Fix Dependent Code

Update any code that depends on the old SDK `RaceDefinition`:

- Check `sdk/campaign_builder/src/` for usages
- Update imports as needed

#### 3.4 Testing Requirements

- Update existing SDK database tests for new types
- Test `ContentDatabase::load_campaign()` loads races correctly
- Test `ContentDatabase::load_core()` loads races correctly
- Verify `RaceDatabase::get_race()` works through SDK

#### 3.5 Deliverables

- [ ] `src/sdk/database.rs` - Updated to use domain types
- [ ] Updated tests for SDK database

#### 3.6 Success Criteria

- No duplicate type definitions in SDK
- `ContentDatabase` loads races from files
- All quality gates pass

### Phase 4: Race Editor CLI Consolidation

Update the CLI editor to use domain types instead of standalone definitions.

#### 4.1 Remove Duplicate Type Definitions

Modify `src/bin/race_editor.rs`:

- Remove local `RaceId`, `RaceDefinition`, `StatModifiers`, `Resistances` definitions (L38-86)
- Import from domain: `use antares::domain::races::{RaceDatabase, RaceDefinition, RaceId, StatModifiers, Resistances, SizeCategory};`

#### 4.2 Update Editor Implementation

Modify `src/bin/race_editor.rs`:

- Update `RaceEditor` struct to use domain types
- Update `load()` method to use `RaceDatabase::load_from_file()` pattern
- Update `save()` method to serialize using domain types
- Add support for new fields: `size_category`, `proficiencies`, `incompatible_item_tags`
- Update menu options to edit new fields

#### 4.3 Add New Field Editing

Add to race editor menu:

- Option to set size category (Small/Medium/Large)
- Option to edit proficiencies list (for future proficiency migration)
- Option to edit incompatible item tags

#### 4.4 Testing Requirements

- Update existing race editor tests for domain types
- Test loading files created with old format (backward compatibility)
- Test saving and reloading with new fields

#### 4.5 Deliverables

- [ ] `src/bin/race_editor.rs` - Updated to use domain types
- [ ] Tests for backward compatibility

#### 4.6 Success Criteria

- No duplicate type definitions
- Race editor loads/saves using domain types
- Backward compatible with existing data files
- All quality gates pass

### Phase 5: SDK UI Races Editor

Create a visual races editor for Campaign Builder following the `classes_editor.rs` pattern.

#### 5.1 Create Races Editor Module

Create new file `sdk/campaign_builder/src/races_editor.rs`:

- `RacesEditorState` struct with fields: `races`, `selected_race`, `mode`, `buffer`, `search_filter`, `has_unsaved_changes`
- `RacesEditorMode` enum: `List`, `Creating`, `Editing`
- `RaceEditBuffer` struct for form fields

#### 5.2 Implement Editor UI

Add to `races_editor.rs`:

- `show()` method for main UI rendering using `TwoColumnLayout`
- `render_list_panel()` for race list with search
- `render_detail_panel()` for selected race details
- `render_edit_form()` for creating/editing races
- Stat modifier editing with +/- buttons
- Resistance editing with +/- buttons
- Special abilities list editing
- Size category dropdown

#### 5.3 Integrate with Campaign Builder

Update `sdk/campaign_builder/src/main.rs`:

- Add `mod races_editor;`
- Add `RacesEditorState` to `CampaignBuilderApp`
- Add "Races" tab to editor tabs
- Load races on campaign open
- Save races on campaign save

#### 5.4 Update Validation

Modify `sdk/campaign_builder/src/validation.rs`:

- Remove info message about "use Race Editor CLI to manage"
- Add actual race validation checks
- Validate race definitions have required fields

#### 5.5 Testing Requirements

- Unit tests for `RacesEditorState::default()`
- Unit tests for buffer conversion to/from `RaceDefinition`
- Tests for validation integration

#### 5.6 Deliverables

- [ ] `sdk/campaign_builder/src/races_editor.rs` - Complete UI editor
- [ ] `sdk/campaign_builder/src/main.rs` - Integration with app
- [ ] `sdk/campaign_builder/src/validation.rs` - Updated validation

#### 5.7 Success Criteria

- Races editor appears in Campaign Builder
- Can create, edit, delete races visually
- Changes persist to `data/races.ron`
- All quality gates pass

### Phase 6: Character Creation Integration

Wire race mechanics into the game engine.

#### 6.1 Update Character Module

Modify `src/domain/character.rs`:

- Keep `Race` enum for backward compatibility (existing code uses it)
- Add `race_id: Option<RaceId>` field to `Character` struct for data-driven lookup
- Add method `apply_race_modifiers(&mut self, race: &RaceDefinition)` to apply stat modifiers
- Add method `get_race_definition(&self, db: &RaceDatabase) -> Option<&RaceDefinition>`

#### 6.2 Create Race Application Function

Add to `src/domain/races.rs`:

- `apply_to_stats(stats: &mut Stats, modifiers: &StatModifiers)` function
- `apply_resistances(character: &mut Character, resistances: &Resistances)` function

#### 6.3 Update Character Creation Flow

Modify relevant character creation code:

- After race selection, lookup `RaceDefinition` from `RaceDatabase`
- Call `apply_race_modifiers()` to adjust base stats
- Store `race_id` for later reference

#### 6.4 Add Race Validation

Add validation for race-based restrictions:

- Check `incompatible_item_tags` when equipping items
- Check race special abilities when relevant

#### 6.5 Testing Requirements

- Unit tests for `apply_to_stats()` with positive/negative modifiers
- Unit tests for stat clamping (stays within valid range)
- Unit tests for character creation with race modifiers
- Integration test: create character, verify stats include race modifiers

#### 6.6 Deliverables

- [ ] `src/domain/character.rs` - Updated with race integration
- [ ] `src/domain/races.rs` - Stat application functions
- [ ] Tests for race-based stat modification

#### 6.7 Success Criteria

- Character stats reflect race modifiers after creation
- Race special abilities are stored and accessible
- All quality gates pass
- Backward compatible with existing `Race` enum usage

## File Change Summary

### New Files

| File                                       | Description                              |
| ------------------------------------------ | ---------------------------------------- |
| `src/domain/races.rs`                      | Core race module with types and database |
| `sdk/campaign_builder/src/races_editor.rs` | Visual races editor for Campaign Builder |

### Modified Files

| File                                     | Changes                                       |
| ---------------------------------------- | --------------------------------------------- |
| `src/domain/mod.rs`                      | Export races module                           |
| `src/domain/character.rs`                | Add race_id field and modifier application    |
| `src/sdk/database.rs`                    | Replace placeholder with domain imports       |
| `src/bin/race_editor.rs`                 | Use domain types instead of local definitions |
| `sdk/campaign_builder/src/main.rs`       | Integrate races editor                        |
| `sdk/campaign_builder/src/validation.rs` | Add race validation                           |
| `data/races.ron`                         | Full race definitions                         |
| `campaigns/tutorial/data/races.ron`      | Full race definitions                         |

### Files to Remove/Consolidate

None - we preserve backward compatibility by keeping the `Race` enum and adding new functionality alongside it.

## Open Questions

1. **Enum vs String ID**: Should `Character` store `Race` enum, `RaceId` string, or both for transition period? Recommend: Keep enum, add optional `race_id` for data-driven lookup.

2. **Stat Modifier Range**: The race_system_prompt.md specifies -255 to +255, but race_editor.rs uses -5 to +5. Which range should we use? Recommend: Use `i16` for storage (allows full range) but validate input to reasonable bounds (-10 to +10 for races).

3. **Resistance Application**: How should resistances interact with the combat system? This may require additional work in the combat module. Recommend: Defer detailed resistance mechanics to a future combat enhancement phase.

## Timeline Estimate

| Phase                             | Estimated Duration | Dependencies     |
| --------------------------------- | ------------------ | ---------------- |
| Phase 1: Core Domain Module       | 2-3 days           | None             |
| Phase 2: Data File Updates        | 0.5 days           | Phase 1          |
| Phase 3: SDK Database Integration | 1 day              | Phase 1          |
| Phase 4: Race Editor CLI          | 1 day              | Phase 1          |
| Phase 5: SDK UI Races Editor      | 2-3 days           | Phase 3          |
| Phase 6: Character Integration    | 2 days             | Phase 1, Phase 2 |
| **Total**                         | **8-11 days**      |                  |

Phases 2, 3, and 4 can be done in parallel after Phase 1 completes.

## Risk Mitigation

| Risk                                | Mitigation                                                   |
| ----------------------------------- | ------------------------------------------------------------ |
| Breaking existing save files        | Keep `Race` enum, add `race_id` as optional field            |
| RON format changes                  | Use `#[serde(default)]` for all new fields                   |
| SDK type conflicts                  | Remove SDK placeholders before adding domain imports         |
| Data file incompatibility           | Test loading old format files, ensure backward compatibility |
| Scope creep into proficiency system | Only add proficiency fields as `Vec<String>` placeholders    |

## Relationship to Proficiency Migration

This plan prepares the race system for the proficiency migration (documented in `proficiency_migration_plan.md`) by:

1. Including `proficiencies: Vec<String>` field from the start
2. Including `incompatible_item_tags: Vec<String>` field from the start
3. Both fields use `#[serde(default)]` so they're optional until needed

When proficiency migration Phase 2 begins, the race infrastructure will already exist and Phase 2.2 ("Create/Update RaceDefinition") will be trivial - just populate the proficiency data in RON files.
