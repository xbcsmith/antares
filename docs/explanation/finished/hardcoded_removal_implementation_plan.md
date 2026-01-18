<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Hard-coded Removal Implementation Plan

## Overview

This plan removes hard-coded Class and Race enums from the Game Engine, replacing
them with data-driven lookups using `ClassId` and `RaceId` references to RON
files. The goal is a fully data-driven engine where adding or modifying classes
and races requires only data file changes, not code modifications.

The key change is migrating `Character` from using static enums (`race: Race`,
`class: Class`) to using string IDs (`race_id: RaceId`, `class_id: ClassId`) that
reference definitions loaded from `classes.ron` and `races.ron`.

This plan incorporates and supersedes the Race System Implementation Plan
(`race_system_implementation_plan.md`), merging Phase 4 of this plan with
the race system work.

## Current State Analysis

### Existing Infrastructure

The codebase has partial data-driven support:

- `ClassDefinition` and `ClassDatabase` exist in `src/domain/classes.rs`
- `classes.ron` defines class properties (hp_die, spell_school, etc.)
- `RaceDefinition` exists but is incomplete (placeholder in SDK)
- `races.ron` exists but only contains id and name fields

However, runtime code still uses static enums:

```rust
// src/domain/character.rs - Static enums
pub enum Race { Human, Elf, Dwarf, Gnome, HalfOrc }
pub enum Class { Knight, Paladin, Archer, Cleric, Sorcerer, Robber }

// Character uses enums, not IDs
pub struct Character {
    pub race: Race,   // Hard-coded enum
    pub class: Class, // Hard-coded enum
}
```

### Identified Issues

1. **Static Class enum**: `src/domain/character.rs` defines `Class` enum with
   fixed variants; adding classes requires code changes

2. **Static Race enum**: `src/domain/character.rs` defines `Race` enum with
   fixed variants; adding races requires code changes

3. **Hard-coded spell logic**: `src/domain/magic/casting.rs` uses `match class`
   patterns for `can_class_cast_school()`, `get_required_level_for_spell()`,
   and `calculate_spell_points()`

4. **Hard-coded HP progression**: `src/domain/progression.rs` `roll_hp_gain()`
   uses `match class` to determine HP dice

5. **Hard-coded SpellBook mapping**: `SpellBook::get_spell_list()` in
   `src/domain/character.rs` matches Class enum to spell lists

6. **Static Disablement constants**: `src/domain/items/types.rs` defines
   `Disablement::KNIGHT`, `Disablement::PALADIN`, etc. as compile-time constants

7. **Incomplete Race System**: `RaceDefinition` in SDK is a placeholder with
   only id/name fields; domain has no races module

## Implementation Phases

### Phase 1: Character Struct Migration

Migrate the `Character` struct from enum-based to ID-based class and race
references.

#### 1.1 Add ID Fields to Character

Modify `src/domain/character.rs`:

- Add `class_id: ClassId` field to `Character` struct
- Add `race_id: RaceId` field to `Character` struct
- Keep existing `class: Class` and `race: Race` fields temporarily for
  compatibility during migration
- Update `Character::new()` to accept both enum and ID parameters
- Add `#[serde(default)]` to new fields for save file compatibility

#### 1.2 Add Conversion Utilities

Create conversion functions in `src/domain/character.rs`:

- `class_id_from_enum(class: Class) -> ClassId` - Convert enum to string ID
- `race_id_from_enum(race: Race) -> RaceId` - Convert enum to string ID
- `class_enum_from_id(id: &ClassId) -> Option<Class>` - Convert ID to enum
- `race_enum_from_id(id: &RaceId) -> Option<Race>` - Convert ID to enum

These enable gradual migration of call sites.

#### 1.3 Update Character Construction

Modify `Character::new()` signature options:

- `Character::new()` - Keep existing signature, populate both enum and ID
- `Character::from_ids()` - New constructor using ClassId and RaceId only
- Ensure both constructors produce consistent state

#### 1.4 Testing Requirements

- Unit tests for ID/enum conversion functions
- Unit tests for `Character::new()` populates both fields correctly
- Unit tests for `Character::from_ids()` creates valid character
- Serialization tests for save/load with new fields
- Backward compatibility tests (old saves without ID fields)

#### 1.5 Deliverables

- Updated `src/domain/character.rs` with dual enum/ID support
- Conversion utility functions
- Comprehensive test coverage

#### 1.6 Success Criteria

- Existing code continues to work with enum-based access
- New code can use ID-based access
- All existing tests pass
- Save/load works with both old and new formats

### Phase 2: Class Logic Migration

Migrate class-dependent logic from `match Class` patterns to `ClassDatabase`
lookups.

#### 2.1 Migrate HP Progression

Modify `src/domain/progression.rs`:

- Update `roll_hp_gain()` to accept `ClassId` and `&ClassDatabase`
- Use `class_db.get_class(class_id).hp_die` instead of `match class`
- Keep old signature as deprecated wrapper for compatibility
- Add `roll_hp_gain_by_id(class_id: &ClassId, class_db: &ClassDatabase, rng)`

#### 2.2 Migrate Spell Casting Logic

Modify `src/domain/magic/casting.rs`:

- Update `can_class_cast_school()` to use `ClassDefinition.spell_school`
- Update `get_required_level_for_spell()` to use class metadata
- Update `calculate_spell_points()` to use `ClassDefinition.spell_stat`
- Add new `*_by_id()` variants that accept `ClassId` and `&ClassDatabase`

#### 2.3 Migrate SpellBook Logic

Modify `SpellBook` in `src/domain/character.rs`:

- Update `get_spell_list()` to accept `ClassId` and `&ClassDatabase`
- Use `class_def.spell_school` to determine which spell list to return
- Add `get_spell_list_by_id(class_id, class_db)` method

#### 2.4 Testing Requirements

- Unit tests for `roll_hp_gain_by_id()` with various class definitions
- Unit tests for spell casting functions with class database lookups
- Integration tests loading classes.ron and using new functions
- Tests verifying enum-based and ID-based functions produce same results

#### 2.5 Deliverables

- Updated `src/domain/progression.rs` with data-driven HP rolls
- Updated `src/domain/magic/casting.rs` with data-driven spell logic
- Updated `SpellBook` with data-driven spell list access
- Comprehensive test coverage

#### 2.6 Success Criteria

- All class-dependent behavior reads from `ClassDefinition`
- Adding a new class to `classes.ron` automatically works with all systems
- No `match Class` patterns remain for behavioral logic
- All tests pass

### Phase 3: Disablement System Migration

Migrate the item disablement system from static bit constants to dynamic lookups.

#### 3.1 Add Dynamic Disablement Methods

Modify `Disablement` in `src/domain/items/types.rs`:

- Add `can_use_class_id(&self, class_id: &ClassId, class_db: &ClassDatabase)`
- Use `class_db.get_class(class_id).disablement_bit_index` for lookup
- Add `can_use_alignment(&self, alignment: Alignment)` for clarity
- Add `can_use(&self, class_id: &ClassId, alignment: Alignment, class_db)`

#### 3.2 Deprecate Static Constants

Mark existing constants as deprecated but keep for compatibility:

- `Disablement::KNIGHT` - Add `#[deprecated]` attribute
- `Disablement::PALADIN` - Add `#[deprecated]` attribute
- (Continue for all class constants)

#### 3.3 Update Item Validation

Update item validation and equipment checks:

- Modify equipment checks to use `can_use_class_id()` instead of bit constants
- Update SDK validation to use dynamic class names from `ClassDatabase`
- Update editors to build class checkboxes from database, not constants

#### 3.4 Testing Requirements

- Unit tests for `can_use_class_id()` with various bit configurations
- Tests verifying dynamic lookup matches static constant results
- Integration tests with items.ron and classes.ron loaded together
- Tests for edge cases (unknown class_id, missing class definition)

#### 3.5 Deliverables

- Updated `src/domain/items/types.rs` with dynamic disablement methods
- Deprecated static constants
- Updated validation code
- Comprehensive test coverage

#### 3.6 Success Criteria

- Item restrictions work with dynamically loaded classes
- Static constants still work but emit deprecation warnings
- SDK editors show class names from database
- All tests pass

### Phase 4: Race System Implementation

Complete the race system to match the class system pattern. This phase
incorporates the work from `race_system_implementation_plan.md`.

#### 4.1 Create Race Domain Module

Create `src/domain/races.rs` with full implementation:

- `RaceId` type alias (`String`)
- `RaceError` enum for validation errors
- `StatModifiers` struct for stat bonuses/penalties
- `Resistances` struct for damage resistances (or reuse from character.rs)
- `SizeCategory` enum (Small, Medium, Large)
- `RaceDefinition` struct with:
  - `id: RaceId`
  - `name: String`
  - `description: String`
  - `stat_modifiers: StatModifiers`
  - `resistances: Resistances`
  - `special_abilities: Vec<String>`
  - `size: SizeCategory`
  - `disablement_bit_index: u8`
  - `proficiencies: Vec<String>` (forward-compatible with proficiency system)
  - `incompatible_item_tags: Vec<String>` (forward-compatible)
- `RaceDatabase` struct with:
  - `load_from_file()`
  - `load_from_string()`
  - `get_race()`
  - `all_races()`
  - `validate()`

#### 4.2 Update races.ron Data Files

Expand `data/races.ron` with complete race data:

```ron
[
    (
        id: "human",
        name: "Human",
        description: "Versatile and adaptable",
        stat_modifiers: (
            might: 0, intellect: 0, personality: 0,
            endurance: 0, speed: 0, accuracy: 0, luck: 0,
        ),
        resistances: (
            magic: 0, fire: 0, cold: 0, electricity: 0,
            acid: 0, fear: 0, poison: 0, psychic: 0,
        ),
        special_abilities: [],
        size: Medium,
        disablement_bit_index: 0,
        proficiencies: [],
        incompatible_item_tags: [],
    ),
    // ... other races
]
```

Update `campaigns/tutorial/data/races.ron` similarly.

#### 4.3 Update SDK Database Integration

Modify `src/sdk/database.rs`:

- Remove placeholder `RaceDefinition` and `RaceDatabase`
- Import domain types: `use crate::domain::races::{RaceDefinition, RaceDatabase}`
- Update `ContentDatabase` to use domain `RaceDatabase`
- Update `load_campaign()` to load races via domain types

#### 4.4 Update Race Editor CLI

Modify `src/bin/race_editor.rs`:

- Remove duplicate type definitions
- Import domain types from `antares::domain::races`
- Update editor to use domain `RaceDefinition` fields
- Ensure backward compatibility with older RON files via `#[serde(default)]`

#### 4.5 Create SDK Races Editor

Create `sdk/campaign_builder/src/races_editor.rs`:

- `RacesEditorMode` enum (List, Add, Edit)
- `RacesEditorState` struct with search, filters, edit buffer
- Full UI following `classes_editor.rs` pattern
- Stat modifier sliders
- Resistance inputs
- Special abilities list
- Size category dropdown

Integrate into `sdk/campaign_builder/src/main.rs`:

- Add "Races" tab to Campaign Builder
- Wire up state and save/load

#### 4.6 Testing Requirements

- Unit tests for `RaceDefinition` and `RaceDatabase`
- Integration tests loading races.ron
- Tests for race validation (duplicate IDs, invalid values)
- SDK editor tests

#### 4.7 Deliverables

- Complete `src/domain/races.rs` implementation
- Expanded `data/races.ron` with full race data
- Expanded `campaigns/tutorial/data/races.ron`
- Updated `src/sdk/database.rs` using domain types
- Updated `src/bin/race_editor.rs` using domain types
- New `sdk/campaign_builder/src/races_editor.rs`
- Comprehensive test coverage

#### 4.8 Success Criteria

- Race system matches class system pattern
- Adding races requires only data file changes
- SDK and CLI editors work with domain types
- All tests pass

### Phase 5: Enum Removal

Remove the static Class and Race enums, completing the migration to fully
data-driven.

#### 5.1 Remove Enum Fields from Character

Modify `src/domain/character.rs`:

- Remove `race: Race` field from `Character`
- Remove `class: Class` field from `Character`
- Keep only `race_id: RaceId` and `class_id: ClassId`
- Update all `Character` constructors

#### 5.2 Remove Enum Definitions

Remove from `src/domain/character.rs`:

- Remove `pub enum Race { ... }` definition
- Remove `pub enum Class { ... }` definition
- Remove conversion utility functions (no longer needed)

#### 5.3 Update All References

Search and update all code referencing removed enums:

- Update function signatures accepting `Class` or `Race`
- Update pattern matches on class/race
- Update tests using enum values
- Update SDK code using enums

#### 5.4 Update Serialization

Ensure save/load works with ID-only format:

- Update serde attributes on `Character`
- Update save file documentation

#### 5.5 Testing Requirements

- Full test suite passes without enum definitions
- Save/load tests with new format
- Integration tests for complete game flow
- SDK tests for character editing

#### 5.6 Deliverables

- `Character` struct using only ID fields
- Removed Class and Race enums
- Updated all references throughout codebase
- Updated save file format

#### 5.7 Success Criteria

- No static Class or Race enums exist
- All class/race behavior is data-driven
- Full test suite passes
- SDK functions correctly with new structure

### Phase 6: SDK and Editor Updates

Update SDK editors to be fully dynamic, removing any remaining hard-coded
references.

#### 6.1 Update Class References in Editors

Update `sdk/campaign_builder/src/`:

- `items_editor.rs` - Use `ClassDatabase` for disablement checkboxes
- `monsters_editor.rs` - Remove any class enum references
- `spells_editor.rs` - Use `ClassDatabase` for class restrictions

#### 6.2 Update Validation Code

Update `sdk/campaign_builder/src/validation.rs`:

- Validate class_id references against loaded `ClassDatabase`
- Validate race_id references against loaded `RaceDatabase`
- Dynamic error messages using class/race names from database

#### 6.3 Keep Templates and Examples

Per requirements, keep useful examples:

- Default item templates in editors (useful for users)
- Example configurations and presets
- Documentation examples

#### 6.4 Testing Requirements

- Editor tests with dynamically loaded class/race databases
- Validation tests for invalid class_id/race_id references
- UI tests for dynamic dropdown population

#### 6.5 Deliverables

- Updated SDK editors with dynamic class/race lists
- Updated validation using databases
- Preserved useful templates and examples

#### 6.6 Success Criteria

- Editors show class/race names from loaded data files
- Validation catches invalid references
- Templates remain useful for users
- All SDK tests pass

### Phase 7: Documentation and Cleanup

Complete documentation and remove deprecated code.

#### 7.1 Remove Deprecated Code

- Remove `#[deprecated]` Disablement constants
- Remove any remaining compatibility wrappers
- Remove unused conversion functions

#### 7.2 Update Architecture Documentation

Update `docs/reference/architecture.md`:

- Document `Character` struct with ID fields only
- Remove references to Class and Race enums
- Document data-driven class/race system

#### 7.3 Update Implementation Documentation

Update `docs/explanation/implementations.md`:

- Document the migration from enums to IDs
- Explain the data-driven architecture
- Provide examples of adding new classes/races

#### 7.4 Create Migration Guide

Create `docs/how-to/add_classes_races.md`:

- Step-by-step guide for adding new classes
- Step-by-step guide for adding new races
- Examples of class/race definitions

#### 7.5 Archive Superseded Plans

Mark the following as superseded by this plan:

- `docs/explanation/race_system_implementation_plan.md` - Merged into Phase 4

#### 7.6 Deliverables

- Clean codebase without deprecated code
- Updated architecture documentation
- Updated implementation documentation
- New how-to guide for content creators

#### 7.7 Success Criteria

- No deprecated code remains
- Documentation accurately reflects current system
- Content creators can add classes/races without code changes
- All quality checks pass

## Relationship to Other Plans

### Character Definition Plan

The Character Definition Implementation Plan (`character_definition_implementation_plan.md`)
should be executed **after** Phases 1-4 of this plan complete. This provides:

- Clean `class_id`/`race_id` fields for `CharacterDefinition` to reference
- Complete `RaceDatabase` for race modifier application during instantiation
- No conversion layers needed between definition and runtime character

Recommended sequence:

1. Hard-coded Removal Phases 1-4 (this plan)
2. Character Definition Plan (all phases)
3. Hard-coded Removal Phases 5-7 (this plan)

### Proficiency Migration Plan

The Proficiency Migration Plan (`proficiency_migration_plan.md`) should be
executed **after** this plan completes. Rationale:

- Phase 4 creates `RaceDefinition` with `proficiencies` and `incompatible_item_tags`
  fields (empty by default, forward-compatible)
- Proficiency Phase 1 creates proficiency types
- Proficiency Phase 2 populates class/race proficiency data
- Proficiency Phase 3+ replaces Disablement system entirely

This plan's Phase 3 (Disablement Migration) provides a bridge - the Disablement
system becomes dynamic but remains functional until proficiencies fully replace it.

### Race System Plan

The Race System Implementation Plan (`race_system_implementation_plan.md`) is
**merged into Phase 4** of this plan. That plan should be marked as superseded.

## File Summary

| File                                       | Action | Phase | Description                 |
| ------------------------------------------ | ------ | ----- | --------------------------- |
| `src/domain/character.rs`                  | Modify | 1, 5  | Add IDs, later remove enums |
| `src/domain/progression.rs`                | Modify | 2     | Data-driven HP rolls        |
| `src/domain/magic/casting.rs`              | Modify | 2     | Data-driven spell logic     |
| `src/domain/items/types.rs`                | Modify | 3     | Dynamic disablement         |
| `src/domain/races.rs`                      | Create | 4     | Complete race system        |
| `src/domain/mod.rs`                        | Modify | 4     | Export races module         |
| `data/races.ron`                           | Modify | 4     | Full race definitions       |
| `campaigns/tutorial/data/races.ron`        | Modify | 4     | Campaign races              |
| `src/sdk/database.rs`                      | Modify | 4     | Use domain race types       |
| `src/bin/race_editor.rs`                   | Modify | 4     | Use domain types            |
| `sdk/campaign_builder/src/races_editor.rs` | Create | 4     | Visual race editor          |
| `sdk/campaign_builder/src/main.rs`         | Modify | 4, 6  | Add Races tab               |
| `sdk/campaign_builder/src/items_editor.rs` | Modify | 6     | Dynamic classes             |
| `sdk/campaign_builder/src/validation.rs`   | Modify | 6     | Dynamic validation          |
| `docs/reference/architecture.md`           | Modify | 7     | Update documentation        |
| `docs/explanation/implementations.md`      | Modify | 7     | Document changes            |
| `docs/how-to/add_classes_races.md`         | Create | 7     | Content creator guide       |

## Estimated Effort

| Phase                          | Complexity  | Estimated Files | Risk   |
| ------------------------------ | ----------- | --------------- | ------ |
| Phase 1: Character Migration   | Medium      | 3-5             | Low    |
| Phase 2: Class Logic Migration | Medium-High | 5-8             | Medium |
| Phase 3: Disablement Migration | Medium      | 3-5             | Low    |
| Phase 4: Race System           | Medium-High | 8-12            | Low    |
| Phase 5: Enum Removal          | High        | 15-25           | Medium |
| Phase 6: SDK Updates           | Medium      | 8-12            | Low    |
| Phase 7: Documentation         | Low         | 5-8             | Low    |

Total: ~50-75 files modified across all phases

## Risk Mitigation

1. **Breaking Changes**: Each phase maintains compatibility until Phase 5; can
   pause at any phase boundary

2. **Test Coverage**: Extensive tests at each phase catch regressions early

3. **Incremental Migration**: Dual enum/ID support allows gradual call site
   updates

4. **Rollback Points**: Git commits at each phase deliverable enable rollback

5. **Forward Compatibility**: Phase 4 includes proficiency fields with defaults,
   avoiding rework when proficiency migration begins
