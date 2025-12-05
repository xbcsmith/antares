# Implementation Summary

## Phase 1: Character Struct Migration (Hard-coded Removal Plan) (2025-01-XX)

**Objective**: Migrate the `Character` struct from enum-based to ID-based class and race references, enabling gradual migration to data-driven lookups.

### Background

Per the Hard-coded Removal Implementation Plan (`docs/explanation/hardcoded_removal_implementation_plan.md`), Phase 1 adds `class_id: ClassId` and `race_id: RaceId` fields to the `Character` struct while preserving the existing enum fields for backward compatibility.

### Changes Implemented

#### 1.1 Added RaceId Type Alias

Added `RaceId` type alias to `src/domain/types.rs`:

- `pub type RaceId = String;` - Race identifier (e.g., "human", "elf", "dwarf")
- Re-exported in `src/domain/mod.rs`

#### 1.2 Added ID Fields to Character Struct

Modified `src/domain/character.rs`:

- Added `race_id: RaceId` field with `#[serde(default)]` for backward compatibility
- Added `class_id: ClassId` field with `#[serde(default)]` for backward compatibility
- Both fields default to empty strings when deserializing old save files

#### 1.3 Added Conversion Utilities

Created four conversion functions in `src/domain/character.rs`:

- `race_id_from_enum(race: Race) -> RaceId` - Converts Race enum to string ID
- `class_id_from_enum(class: Class) -> ClassId` - Converts Class enum to string ID
- `race_enum_from_id(id: &RaceId) -> Option<Race>` - Converts string ID to Race enum
- `class_enum_from_id(id: &ClassId) -> Option<Class>` - Converts string ID to Class enum

ID mappings:

- Human → "human", Elf → "elf", Dwarf → "dwarf", Gnome → "gnome", HalfOrc → "half_orc"
- Knight → "knight", Paladin → "paladin", Archer → "archer", Cleric → "cleric", Sorcerer → "sorcerer", Robber → "robber"

#### 1.4 Updated Character Constructors

- `Character::new()` - Now populates both enum fields AND ID fields automatically
- `Character::from_ids()` - New constructor that accepts RaceId and ClassId, converts to enums, returns `Option<Character>`

### Tests Added

Comprehensive test coverage added to `src/domain/character.rs`:

**ID/Enum Conversion Tests:**

- `test_race_id_from_enum_all_races` - All race conversions
- `test_class_id_from_enum_all_classes` - All class conversions
- `test_race_enum_from_id_all_races` - All reverse race conversions
- `test_class_enum_from_id_all_classes` - All reverse class conversions
- `test_race_enum_from_id_invalid` - Invalid/empty/case-sensitive inputs
- `test_class_enum_from_id_invalid` - Invalid/empty/case-sensitive inputs
- `test_id_enum_roundtrip_races` - Round-trip conversion validation
- `test_id_enum_roundtrip_classes` - Round-trip conversion validation

**Character Constructor Tests:**

- `test_character_new_populates_both_enum_and_id` - Verifies new() sets both fields
- `test_character_new_all_race_class_combinations` - Multiple combo tests
- `test_character_from_ids_success` - Valid ID constructor
- `test_character_from_ids_invalid_race` - Invalid race returns None
- `test_character_from_ids_invalid_class` - Invalid class returns None
- `test_character_from_ids_both_invalid` - Both invalid returns None
- `test_character_from_ids_empty_strings` - Empty strings return None
- `test_character_from_ids_default_values` - Verifies default starting values
- `test_character_new_and_from_ids_produce_equivalent_state` - Equivalence test

**Serialization Tests:**

- `test_character_serialization_with_ids` - RON serialization includes ID fields
- `test_character_deserialization_with_ids` - Round-trip preserves all fields
- `test_character_backward_compatibility_missing_ids` - serde(default) behavior

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 390 unit tests pass, 224 doc tests pass

### Architecture Compliance

- [x] Type aliases used consistently (ClassId, RaceId)
- [x] `#[serde(default)]` used for backward compatibility
- [x] Existing enum fields preserved for gradual migration
- [x] Module structure follows architecture.md Section 3.2
- [x] No circular dependencies introduced

### Success Criteria Met

- [x] Existing code continues to work with enum-based access
- [x] New code can use ID-based access
- [x] All existing tests pass
- [x] Save/load works with both old and new formats (via serde default)
- [x] Both constructors produce consistent state

### Files Modified

- `src/domain/types.rs` - Added RaceId type alias
- `src/domain/mod.rs` - Re-exported RaceId
- `src/domain/character.rs` - Added ID fields, conversion functions, from_ids() constructor, and tests

### Next Steps (Phase 2)

Per the implementation plan:

- Migrate HP progression logic to use ClassDatabase lookups
- Migrate spell casting logic to use ClassDatabase
- Migrate SpellBook logic to use ClassDatabase

---

## Phase 2: Class Logic Migration (Hard-coded Removal Plan) (2025-01-XX)

**Objective**: Migrate class-dependent logic from `match Class` patterns to `ClassDatabase` lookups, enabling data-driven class behavior.

### Background

Per the Hard-coded Removal Implementation Plan (`docs/explanation/hardcoded_removal_implementation_plan.md`), Phase 2 migrates runtime class logic to use the ClassDatabase instead of hardcoded match patterns on the Class enum. This enables:

- Adding new classes via data files without code changes
- Campaign-specific class modifications
- Modding support for custom classes

### Changes Implemented

#### 2.1 HP Progression Migration

Modified `src/domain/progression.rs`:

- Added `level_up_from_db()` function that uses ClassDatabase for HP calculation
- Existing `roll_hp_gain_from_db()` already existed from prior work
- `level_up_from_db()` uses `calculate_spell_points_by_id()` for SP updates
- Added deprecation note to `roll_hp_gain()` recommending data-driven version

#### 2.2 Spell Casting Logic Migration

Modified `src/domain/magic/casting.rs`:

- Added `can_class_cast_school_by_id(class_id, class_db, school)` - Uses ClassDefinition.spell_school
- Added `get_required_level_for_spell_by_id(class_id, class_db, spell)` - Uses is_pure_caster for hybrid delay
- Added `calculate_spell_points_by_id(character, class_db)` - Uses ClassDefinition.spell_stat
- Handles SpellSchool type conversion between classes.rs and magic/types.rs enums

#### 2.3 SpellBook Logic Migration

Modified `src/domain/character.rs`:

- Added `SpellBook::get_spell_list_by_id(class_id, class_db)` - Returns spell list based on class's spell_school
- Added `SpellBook::get_spell_list_mut_by_id(class_id, class_db)` - Mutable version
- Both methods look up ClassDefinition.spell_school to determine Cleric vs Sorcerer list

#### 2.4 Module Exports

Updated `src/domain/magic/mod.rs`:

- Re-exported new functions: `calculate_spell_points_by_id`, `can_class_cast_school_by_id`, `get_required_level_for_spell_by_id`

### Tests Added

**Progression Tests (src/domain/progression.rs):**

- `test_level_up_from_db` - Basic level up with database
- `test_level_up_from_db_increases_hp` - HP gain verification
- `test_level_up_from_db_not_enough_xp` - XP requirement check
- `test_level_up_from_db_max_level` - Max level boundary
- `test_level_up_from_db_spellcaster_gains_sp` - SP gain for casters
- `test_level_up_from_db_non_spellcaster_no_sp` - No SP for non-casters
- `test_enum_and_db_hp_rolls_same_range` - Equivalence verification

**Spell Casting Tests (src/domain/magic/casting.rs):**

- `test_can_class_cast_school_by_id_cleric` - Cleric school access
- `test_can_class_cast_school_by_id_sorcerer` - Sorcerer school access
- `test_can_class_cast_school_by_id_paladin` - Hybrid caster school access
- `test_can_class_cast_school_by_id_non_caster` - Non-casters return false
- `test_can_class_cast_school_by_id_unknown_class` - Unknown class handling
- `test_get_required_level_for_spell_by_id_pure_caster` - Pure caster levels
- `test_get_required_level_for_spell_by_id_hybrid_caster` - Hybrid caster delay
- `test_get_required_level_for_spell_by_id_non_caster` - Non-casters return MAX
- `test_get_required_level_for_spell_by_id_unknown_class` - Unknown class handling
- `test_get_required_level_for_spell_by_id_higher_level_spells` - Higher spell levels
- `test_calculate_spell_points_by_id_cleric` - Cleric SP calculation
- `test_calculate_spell_points_by_id_sorcerer` - Sorcerer SP calculation
- `test_calculate_spell_points_by_id_paladin` - Paladin SP calculation
- `test_calculate_spell_points_by_id_non_caster` - Non-casters return 0
- `test_calculate_spell_points_by_id_unknown_class` - Unknown class handling
- `test_calculate_spell_points_by_id_low_stat` - Low stat handling
- `test_enum_and_db_spell_points_match` - Equivalence verification
- `test_enum_and_db_can_cast_school_match` - Equivalence verification
- `test_enum_and_db_required_level_match` - Equivalence verification

**SpellBook Tests (src/domain/character.rs):**

- `test_spellbook_get_spell_list_by_id_cleric` - Cleric spell list access
- `test_spellbook_get_spell_list_by_id_sorcerer` - Sorcerer spell list access
- `test_spellbook_get_spell_list_by_id_paladin` - Paladin (hybrid) spell list access
- `test_spellbook_get_spell_list_by_id_knight` - Non-caster default behavior
- `test_spellbook_get_spell_list_by_id_unknown_class` - Unknown class handling
- `test_spellbook_get_spell_list_mut_by_id_cleric` - Mutable cleric access
- `test_spellbook_get_spell_list_mut_by_id_sorcerer` - Mutable sorcerer access
- `test_spellbook_get_spell_list_mut_by_id_paladin` - Mutable paladin access
- `test_spellbook_get_spell_list_mut_by_id_knight` - Mutable non-caster default
- `test_spellbook_enum_and_db_methods_match` - Pointer equivalence test
- `test_spellbook_multiple_spell_levels` - Multiple spell level handling

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 427 unit tests pass, 230 doc tests pass

### Architecture Compliance

- [x] Data-driven functions use ClassDatabase lookups
- [x] Existing enum-based functions preserved for compatibility
- [x] SpellSchool type conversion handled between modules
- [x] Module structure follows architecture.md Section 3.2
- [x] No circular dependencies introduced
- [x] AttributePair pattern respected (spell_stat lookup)

### Success Criteria Met

- [x] All class-dependent behavior can read from ClassDefinition
- [x] Adding a new class to classes.ron works with all systems (when using \*\_by_id functions)
- [x] Enum-based and ID-based functions produce identical results
- [x] All existing tests pass
- [x] New comprehensive test coverage for data-driven functions

### Files Modified

- `src/domain/progression.rs` - Added `level_up_from_db()`, tests
- `src/domain/magic/casting.rs` - Added `*_by_id()` functions, tests
- `src/domain/magic/mod.rs` - Re-exported new functions
- `src/domain/character.rs` - Added SpellBook `*_by_id()` methods, tests

### Next Steps (Phase 4)

Per the implementation plan:

- Create Race domain module with RaceDefinition and RaceDatabase
- Update races.ron data files
- Update SDK database integration for races

---

## Phase 3: Disablement System Migration (Hard-coded Removal Plan) (2025-01-XX)

### Background

Phase 3 continues the hard-coded removal effort by migrating the item disablement
system from static bit constants to dynamic lookups using `ClassDatabase`. This
enables the game to support dynamically defined classes for item restrictions.

Previously, item restrictions used hardcoded constants like `Disablement::KNIGHT`
which required code changes to add new classes. Now the system uses
`ClassDefinition.disablement_bit_index` for data-driven lookups.

### Changes Implemented

#### 3.1 Added Dynamic Disablement Methods

Added to `Disablement` in `src/domain/items/types.rs`:

- `can_use_class_id(&self, class_id: &str, class_db: &ClassDatabase) -> bool`

  - Looks up class by ID and checks if the class's disablement bit is set
  - Returns `false` for unknown class IDs (safe default)

- `can_use_alignment(&self, alignment: Alignment) -> bool`

  - Checks if the alignment restriction allows item use
  - Handles Good-only, Evil-only, and no-restriction cases

- `can_use(&self, class_id: &str, alignment: Alignment, class_db: &ClassDatabase) -> bool`

  - Combined check for both class and alignment restrictions
  - The comprehensive method for item validation

- `from_class_ids<I>(class_ids: I, class_db: &ClassDatabase) -> Self`

  - Builds a disablement mask from a list of class IDs
  - Useful for constructing restrictions programmatically

- `allowed_class_ids(&self, class_db: &ClassDatabase) -> Vec<String>`
  - Returns list of class IDs that can use an item
  - Useful for UI display and validation

#### 3.2 Deprecated Static Constants

Marked the following constants with `#[deprecated]`:

- `Disablement::KNIGHT`
- `Disablement::PALADIN`
- `Disablement::ARCHER`
- `Disablement::CLERIC`
- `Disablement::SORCERER`
- `Disablement::ROBBER`

Deprecation message directs users to `can_use_class_id()` with `ClassDatabase`.

Note: `Disablement::GOOD` and `Disablement::EVIL` are NOT deprecated as alignment
is a fixed concept, not data-driven.

#### 3.3 Updated Existing Code

Added `#[allow(deprecated)]` annotations to maintain backward compatibility:

- `src/bin/item_editor.rs` - CLI item editor using legacy constants
- `sdk/campaign_builder/src/items_editor.rs` - GUI item editor
- `sdk/campaign_builder/src/main.rs` - Campaign builder tests

### Tests Added

New tests in `src/domain/items/types.rs`:

- `test_can_use_class_id_single_class` - Single class restriction
- `test_can_use_class_id_multiple_classes` - Multiple class restrictions
- `test_can_use_class_id_all_classes` - Universal item (ALL)
- `test_can_use_class_id_none` - Quest item (NONE)
- `test_can_use_class_id_unknown_class` - Unknown class returns false
- `test_can_use_alignment_any` - No alignment restriction
- `test_can_use_alignment_good_only` - Good alignment required
- `test_can_use_alignment_evil_only` - Evil alignment required
- `test_can_use_combined` - Combined class + alignment checks
- `test_from_class_ids` - Building mask from class list
- `test_from_class_ids_empty` - Empty class list
- `test_from_class_ids_with_unknown` - Unknown classes ignored
- `test_allowed_class_ids` - Getting allowed class list
- `test_allowed_class_ids_all` - All classes allowed
- `test_allowed_class_ids_none` - No classes allowed
- `test_dynamic_matches_static_constants` - Equivalence verification
- `test_bit_index_matches_static_constant` - Bit mask equivalence

### Validation

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 444 unit tests pass, 235 doc tests pass

### Architecture Compliance

- [x] Dynamic methods use ClassDatabase lookups via `get_class()`
- [x] Uses `ClassDefinition.disablement_mask()` for bit calculations
- [x] Static constants deprecated with clear migration path
- [x] Alignment enum from `domain::character` module used correctly
- [x] No circular dependencies introduced
- [x] Backward compatibility maintained with `#[allow(deprecated)]`

### Success Criteria Met

- [x] Item restrictions work with dynamically loaded classes
- [x] Static constants still work but emit deprecation warnings
- [x] Data-driven results match static constant results (verified by tests)
- [x] Unknown class IDs handled gracefully (return false)
- [x] All existing tests pass

### Files Modified

- `src/domain/items/types.rs` - Added dynamic methods, deprecated constants, tests
- `src/bin/item_editor.rs` - Added `#[allow(deprecated)]` annotations
- `sdk/campaign_builder/src/items_editor.rs` - Added `#[allow(deprecated)]` annotations
- `sdk/campaign_builder/src/main.rs` - Added `#[allow(deprecated)]` annotations

### Next Steps (Phase 4)

Per the implementation plan:

- Create Race domain module with RaceDefinition and RaceDatabase
- Update races.ron data files
- Update SDK database integration for races
- Update Race Editor CLI

---

## Phase 4: Race System Implementation (Hard-coded Removal Plan) (2025-01-XX)

### Background

Per `docs/explanation/hardcoded_removal_implementation_plan.md` Phase 4, the race system needed
to be implemented following the same data-driven pattern as the class system. This phase creates
a complete race domain module with `RaceDefinition`, `RaceDatabase`, stat modifiers, resistances,
size categories, and integrates with the SDK and editors.

### Changes Implemented

#### 4.1 Created Race Domain Module (`src/domain/races.rs`)

Complete implementation including:

- `RaceId` type alias (String)
- `RaceError` enum for validation errors (RaceNotFound, LoadError, ParseError, ValidationError, DuplicateId)
- `SizeCategory` enum (Small, Medium, Large) with Default impl
- `StatModifiers` struct for stat bonuses/penalties (might, intellect, personality, endurance, speed, accuracy, luck)
- `Resistances` struct for damage resistances (magic, fire, cold, electricity, acid, fear, poison, psychic)
- `RaceDefinition` struct with all fields:
  - `id`, `name`, `description`
  - `stat_modifiers`, `resistances`
  - `special_abilities`, `proficiencies`, `incompatible_item_tags`
  - `size`, `disablement_bit_index`
- `RaceDatabase` struct with:
  - `new()`, `load_from_file()`, `load_from_string()`
  - `get_race()`, `all_races()`, `all_race_ids()`
  - `add_race()`, `remove_race()`
  - `validate()`, `len()`, `is_empty()`
- Helper methods on `RaceDefinition`:
  - `disablement_mask()`, `has_ability()`, `has_proficiency()`
  - `is_item_incompatible()`, `is_small()`, `is_medium()`, `is_large()`
- Helper methods on `StatModifiers` and `Resistances`:
  - `new()`, `is_empty()`, `total()`, `validate()`

#### 4.2 Updated races.ron Data Files

Expanded `data/races.ron` with complete race data for 6 races:

- Human (balanced, no modifiers)
- Elf (+2 INT, +2 ACC, -1 MIG, -1 END, infravision, resist_sleep, resist_charm)
- Dwarf (+1 MIG, +2 END, -1 PER, -1 SPD, poison/magic resistance, stonecunning)
- Gnome (-2 MIG, +1 INT, +2 LCK, Small size, magic/psychic resistance)
- Half-Elf (+1 INT, +1 ACC, partial resistances)
- Half-Orc (+2 MIG, +1 END, -1 INT, -2 PER, fear resistance)

Updated `campaigns/tutorial/data/races.ron` with 4 races (Human, Elf, Dwarf, Gnome).

#### 4.3 Updated SDK Database Integration

Modified `src/sdk/database.rs`:

- Removed placeholder `RaceDefinition` and `RaceDatabase` types
- Added import: `use crate::domain::races::{RaceDatabase, RaceError}`
- Added `From<RaceError> for DatabaseError` conversion
- Updated `ContentDatabase` to use domain `RaceDatabase`
- Fixed `stats()` method to use `races.len()` instead of removed `count()`

#### 4.4 Updated Race Editor CLI

Rewrote `src/bin/race_editor.rs` to use domain types:

- Removed duplicate type definitions
- Import domain types: `use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers}`
- Updated all fields to match domain types (u8 resistances, 8 resistance types)
- Added size category input/display
- Added proficiencies and incompatible_item_tags editing
- Updated version to v0.2.0
- Maintained backward compatibility with `#[serde(default)]` on domain types

#### 4.5 Created SDK Races Editor

Created `sdk/campaign_builder/src/races_editor.rs`:

- `RacesEditorMode` enum (List, Creating, Editing)
- `RacesEditorState` struct with races, selection, mode, buffer, search filter
- `RaceEditBuffer` struct for form fields
- Full UI following `classes_editor.rs` pattern:
  - List view with two-column layout
  - Race details panel with stats, resistances, abilities
  - Create/edit form with all fields
  - Size category dropdown
  - Stat modifier inputs (-10 to +10)
  - Resistance inputs (0-100%)
  - Special abilities, proficiencies, incompatible tags (comma-separated)
- Toolbar with New, Save, Load, Export, Reload actions
- Context menu with Edit, Delete, Duplicate

Integrated into `sdk/campaign_builder/src/main.rs`:

- Added `mod races_editor` declaration
- Added `EditorTab::Races` variant
- Added `races_editor_state` field to `CampaignBuilderApp`
- Added Races tab to tab array
- Added Races tab handler calling `races_editor_state.show()`

### Tests Added

Domain module tests (`src/domain/races.rs`):

- `test_stat_modifiers_default`, `test_stat_modifiers_is_empty`, `test_stat_modifiers_total`
- `test_resistances_default`, `test_resistances_is_empty`, `test_resistances_validate_*`
- `test_race_definition_disablement_mask`, `test_race_definition_has_ability`
- `test_race_definition_has_proficiency`, `test_race_definition_is_item_incompatible`
- `test_race_definition_size_checks`
- `test_race_database_new`, `test_race_database_add_race`, `test_race_database_duplicate_id_error`
- `test_race_database_remove_race`, `test_race_database_load_from_string`
- `test_race_database_load_minimal`, `test_race_database_get_race_not_found`
- `test_race_database_all_races`, `test_race_database_all_race_ids`
- `test_race_database_duplicate_id_in_load`, `test_race_database_validation_*`
- `test_size_category_default`, `test_size_category_serialization`
- `test_load_races_from_data_file` (integration test)

SDK editor tests (`sdk/campaign_builder/src/races_editor.rs`):

- `test_races_editor_state_creation`, `test_start_new_race`
- `test_save_race_creates_new`, `test_save_race_empty_id_error`, `test_save_race_empty_name_error`
- `test_save_race_duplicate_id_error`, `test_delete_race`, `test_cancel_edit`
- `test_filtered_races`, `test_next_available_race_id`
- `test_start_edit_race`, `test_edit_race_saves_changes`
- `test_race_edit_buffer_default`, `test_editor_mode_transitions`

Race editor CLI tests (`src/bin/race_editor.rs`):

- `test_truncate`, `test_get_next_disablement_bit_*`
- `test_stat_modifiers_default`, `test_resistances_default`, `test_size_category_default`

### Validation

All quality gates pass:

- `cargo fmt --all` - OK
- `cargo check --all-targets --all-features` - OK (0 errors)
- `cargo clippy --all-targets --all-features -- -D warnings` - OK (0 warnings)
- `cargo test --all-features` - OK (245 doc tests passed, all unit tests pass)

### Architecture Compliance

- Race system matches class system pattern exactly
- Type aliases used consistently (`RaceId`)
- Constants extracted (resistance max 100, stat modifier range -10 to +10)
- RON format used for data files
- Module placed correctly in `src/domain/races.rs`
- SDK database uses domain types (no placeholders)
- Backward compatibility via `#[serde(default)]` attributes

### Success Criteria Met

- [x] Race system matches class system pattern
- [x] Adding races requires only data file changes
- [x] SDK and CLI editors work with domain types
- [x] All tests pass
- [x] Data files expanded with full race definitions
- [x] SDK Campaign Builder has Races tab

### Files Created

- `src/domain/races.rs` - Complete race domain module
- `sdk/campaign_builder/src/races_editor.rs` - SDK races editor UI

### Files Modified

- `src/domain/mod.rs` - Added `pub mod races` export
- `src/sdk/database.rs` - Removed placeholders, use domain types
- `src/sdk/validation.rs` - Fixed RaceId import
- `src/bin/race_editor.rs` - Rewrote to use domain types
- `sdk/campaign_builder/src/main.rs` - Added Races tab and editor state
- `data/races.ron` - Expanded with full race data
- `campaigns/tutorial/data/races.ron` - Expanded with full race data

### Next Steps (Phase 5)

Per the implementation plan:

- Remove enum fields from Character struct
- Remove Race and Class enum definitions
- Update all references to use string IDs
- Update serialization for ID-based approach

---

> NOTE: The "Engine Support for SDK Data Changes" full implementation plan has been moved to:
> `docs/explanation/engine_sdk_support_plan.md`
>
> This document is now a summary record for completed implementations and associated artifacts.
> The detailed phased plan is maintained separately in the file above. Implementers should keep `implementations.md` as a summary and update it once each phase is completed and merged.

## Disablement Bit — Implementation & Impact

The full details for "Disablement Bit — Implementation & Impact" have been moved to:
`docs/explanation/disablement_bits.md`

## Database / Campaign Data Fixes (2025-12-02)

- `campaigns/tutorial/data/monsters.ron` - Added missing top-level `experience_value` field to all monster entries so they conform with the current `Monster` schema. Each added `experience_value` was set to the value previously present in the monster's `loot.experience` field to preserve intended XP awards.

Note: As of 2025-12-02, two pre-existing UI tests in `sdk/campaign_builder/tests/bug_verification.rs` were updated to reflect refactoring that moved editor implementations into separate module files. These tests — `test_items_tab_widget_ids_unique` and `test_monsters_tab_widget_ids_unique` — now inspect the refactored editor files (`src/items_editor.rs` and `src/monsters_editor.rs`, respectively) and validate the correct use of `egui::ComboBox::from_id_salt` rather than implicit ID generation methods (e.g., `from_label`) to avoid widget ID collisions.

This document tracks completed implementations and changes to the Antares project.

## Phase 1: SDK Campaign Builder UI - Foundation Components (2025-01-XX)

**Objective**: Create reusable, centralized UI components for the Campaign Builder SDK to reduce code duplication, ensure consistency across editors, and improve maintainability.

### Background

Per the SDK QOL Implementation Plan (`docs/explanation/sdk_qol_implementation_plan.md`), Phase 1 focuses on creating shared UI components that all editors can use. This phase establishes the foundation components without refactoring the existing editors, allowing incremental adoption.

### Components Created

All components were added to `sdk/campaign_builder/src/ui_helpers.rs`:

#### 1. EditorToolbar Component

A reusable toolbar component with standard buttons for all editors:

- **`ToolbarAction` enum**: `New`, `Save`, `Load`, `Import`, `Export`, `Reload`, `None`
- **`EditorToolbar` struct**: Builder pattern for configuring toolbar options
- Features:
  - Optional search field with customizable id salt
  - Optional merge mode checkbox
  - Optional total count display
  - Configurable save button visibility
- Usage: `EditorToolbar::new("Items").with_search(&mut query).show(ui)`

#### 2. ActionButtons Component

Reusable action buttons for detail panels:

- **`ItemAction` enum**: `Edit`, `Delete`, `Duplicate`, `Export`, `None`
- **`ActionButtons` struct**: Builder pattern for button configuration
- Features:
  - Enable/disable state
  - Per-button visibility control
  - Consistent button styling across editors
- Usage: `ActionButtons::new().enabled(has_selection).show(ui)`

#### 3. TwoColumnLayout Component

Standard two-column list/detail layout:

- **`TwoColumnLayout` struct**: Manages consistent column layout
- Features:
  - Uses `DEFAULT_LEFT_COLUMN_WIDTH` (300.0 points)
  - Uses `compute_panel_height()` for responsive sizing
  - Automatic scroll area setup with unique id salts
  - `show_split()` method for separate left/right closures
- Usage: `TwoColumnLayout::new("items").show_split(ui, left_fn, right_fn)`

#### 4. ImportExportDialog Component

Reusable import/export dialog for RON data:

- **`ImportExportResult` enum**: `Import(String)`, `Cancel`, `Open`
- **`ImportExportDialogState` struct**: Manages dialog state
- **`ImportExportDialog` struct**: Modal dialog implementation
- Features:
  - Separate import (editable) and export (read-only) modes
  - Error message display
  - Copy to clipboard support
  - Configurable dimensions
- Usage: `ImportExportDialog::new("Title", &mut state).show(ctx)`

#### 5. AttributePairInput Widget

Widget for editing `AttributePair` (u8 base/current):

- **`AttributePairInputState` struct**: Tracks auto-sync behavior
- **`AttributePairInput` struct**: Widget implementation
- Features:
  - Dual input fields for base and current values
  - Auto-sync option (current follows base when enabled)
  - Reset button to restore current to base
  - Customizable id salt
- Usage: `AttributePairInput::new("AC", &mut value).show(ui)`

#### 6. AttributePair16Input Widget

Widget for editing `AttributePair16` (u16 base/current):

- Same features as `AttributePairInput` but for 16-bit values
- Configurable maximum value
- Used for HP, SP, and other larger value attributes
- Usage: `AttributePair16Input::new("HP", &mut hp).with_max_value(9999).show(ui)`

#### 7. File I/O Helper Functions

Utility functions for common file operations:

- `load_ron_file<T>()` - Load and deserialize RON files
- `save_ron_file<T>()` - Serialize and save RON files
- `handle_file_load<T>()` - Complete file load with merge support
- `handle_file_save<T>()` - Complete file save with dialog
- `handle_reload<T>()` - Reload from campaign directory

### Tests Added

Comprehensive tests added to `ui_helpers.rs`:

- Panel height calculation tests (existing + new edge cases)
- `ToolbarAction` enum value tests
- `EditorToolbar` builder pattern tests
- `ItemAction` enum value tests
- `ActionButtons` builder pattern tests
- `TwoColumnLayout` configuration tests
- `ImportExportDialogState` lifecycle tests
- `ImportExportResult` enum tests
- `AttributePairInputState` tests
- `AttributePair` and `AttributePair16` reset behavior tests
- Constants validation tests

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 218 tests pass

### Architecture Compliance

- Type aliases used consistently (no raw types)
- `AttributePair` pattern respected for modifiable stats
- RON format used for all data files
- Module structure follows architecture.md Section 3.2
- No circular dependencies introduced

### Success Criteria Met

- [x] All shared components are created and tested
- [x] Components are callable from any editor
- [x] All existing tests continue to pass
- [x] New component tests pass
- [x] AttributePair widgets support dual base/current editing

### Files Modified

- `sdk/campaign_builder/src/ui_helpers.rs` - Added all shared components and tests

### Deferred to Phase 1.6

The following refactoring was deferred to allow incremental adoption:

- Refactor `items_editor.rs` to use shared components
- Refactor `monsters_editor.rs` to use shared components
- Refactor `spells_editor.rs` to use shared components

This refactoring requires careful attention to type compatibility between the shared components and each editor's specific domain types (e.g., `ConsumableEffect` variants, `AmmoType` enum values, `Disablement` flags). It is recommended to refactor one editor at a time with thorough testing.

### Next Steps (Phase 1.6 / Phase 3+)

Per the implementation plan:

**Phase 1.6 - Editor Refactoring (when ready)**

- Incrementally refactor `items_editor`, `monsters_editor`, `spells_editor` to use shared components
- Test each editor thoroughly before moving to the next

**Phase 3+ - Layout Continuity & Further Improvements**

- Update editor layouts for consistency
- Apply AttributePair widgets across all editors
- Improve validation and asset panels

---

## Phase 3: Editor Layout Continuity (2025-01-XX)

**Objective**: Update all editors to use shared UI components (EditorToolbar, ActionButtons, TwoColumnLayout) for consistent layout and behavior across the SDK Campaign Builder.

### Background

Phase 1 created shared UI components. Phase 2 extracted Classes and Dialogues editors from main.rs. Phase 3 applies the shared components to all editors for layout consistency.

### Changes Implemented

#### 3.1 Classes Editor Layout Update

Updated `classes_editor.rs` to use shared components:

- Replaced manual toolbar with `EditorToolbar` component
- Added `ActionButtons` (Edit/Delete/Duplicate/Export) to detail panel
- Implemented `TwoColumnLayout` for list/detail split view
- Toolbar actions: New, Save, Load, Import (placeholder), Export, Reload

#### 3.2 Dialogues Editor Layout Update

Updated `dialogue_editor.rs` to use shared components:

- Replaced manual toolbar with `EditorToolbar` component
- Added `ActionButtons` to detail panel
- Implemented `TwoColumnLayout` for list/detail split view
- Proper handling of HashMap-based nodes structure

#### 3.3 Quests Editor Toolbar Update

Updated `quest_editor.rs` to use shared toolbar:

- Replaced manual toolbar with `EditorToolbar` component
- Consolidated Save/Load/Reload actions
- Maintained existing list/form mode structure (complex sub-editors)

#### 3.4 Monsters Editor AttributePair Widgets

Updated `monsters_editor.rs` to use AttributePair widgets:

- HP using `AttributePair16Input` widget
- AC using `AttributePairInput` widget
- All Stats (Might, Intellect, Personality, Endurance, Speed, Accuracy, Luck) using `AttributePairInput`
- Each widget shows Base/Current values with Reset button

## Phase 8: Complete Phase 1.6 and Phase 3 Skipped Items (2025-01-XX)

**Objective**: Complete the previously deferred refactoring of editors to use shared UI components (EditorToolbar, ActionButtons, TwoColumnLayout) for consistent layout across all SDK Campaign Builder editors.

### Summary

This patch refactors the major editors to use the shared components created in Phase 1 and brings all editors to a consistent layout and behavior model:

- `Items`, `Spells`, `Monsters`, `Conditions`, `Quests`, `Classes`, `Dialogues`, and `Maps` editors now share:
  - `EditorToolbar`
  - `TwoColumnLayout`
  - `ActionButtons`

Key details

- Implemented the standard `show()` method for each editor state (e.g., `ItemsEditorState::show()`).
- Reused helper functions and constants like `DEFAULT_LEFT_COLUMN_WIDTH` and `compute_panel_height`.
- Updated tests and added new tests for editor patterns and behavior where missing.

**Files modified**: Multiple files under `sdk/campaign_builder/src/*_editor.rs` and `ui_helpers.rs`.

**Validation**: All checks pass (`fmt`, `cargo check`, `clippy`, `tests`).

---

## Phase 2: Extract Editor UI Code from main.rs (2025-01-XX)

**Objective**: Extract Classes and Dialogues editor UI code from `main.rs` into their respective module files, following the standard editor pattern with `show()` methods.

### Background

This extracts large UI blocks from `main.rs` into per-editor modules.

### Changes Implemented

- Extracted Classes and Dialogues editor UI into `classes_editor.rs` and `dialogue_editor.rs`.
- Implemented consistent `show()` signatures and moved state and helper functions into editor modules.
- Updated `main.rs` to delegate to the per-editor `show()` methods.

### Validation

- Quality gates pass: `fmt`, `check`, `clippy`, `tests`.

---

## Phase 0: Conditions Editor — Discovery & Preparation (2025-11-XX)

Summary:

- Completed Phase 0 discovery and scoping for the Conditions Editor refactor (toolbar & file I/O).
- Created `docs/explanation/conditions_editor_phase_0_discovery.md` that captures audit results, usage & references, RON examples, and migration recommendations.

Key outcomes:

- Identified per-effect editing needs and dependencies across domain & runtime code.
- Verified `ActiveCondition.magnitude` is a runtime-only field.
- Provided action list for subsequent Phases for the Conditions editor.

---

## Clippy Error Fixes (2025-01-15)

**Objective**: Fix clippy warnings that were treated as errors in the Campaign Builder SDK, ensuring code quality.

### Changes Implemented

- Reworked a number of UI utils and editor code to remove clippy warnings.
- Switched `match` usages for item filtering to `matches!` macro.
- Added `#[allow(clippy::too_many_arguments)]` to `show()` functions where reducing arguments would require a larger refactor.

### Validation

- All quality gates pass (`fmt`, `check`, `clippy`, `tests`).

---

## ClassDefinition Test Updates (2024-12-01)

**Objective**: Update class test data and doc examples to include all `ClassDefinition` fields.

### Changes Implemented

- Updated `data/classes.ron` and doc examples in `src/domain/classes.rs`.
- Updated tests that expect fully defined `ClassDefinition` objects.

### Validation

- Tests pass; documentation examples compile.

---

## Phase 1: Critical Quest Editor Fixes (2025-11-25)

- Fixed duplicate stage call in quest editor.
- Fixed `selected_stage` behavior in quest editor.
- Auto-fill quest ID for new quests.

## Phase 2.1: Shared UI Helper and Editor Refactor (2025-11-29)

- Added `sdk/campaign_builder/src/ui_helpers.rs`.
- Refactored multiple editors to use shared helpers.

## Phase 2: Campaign Builder UI - List & Preview Panel Scaling (2025-11-29)

- Made the left list and preview panes scale with available window height.

## Conditions Editor QoL Improvements (2025-01-XX)

- Added filter/sort/statistics panel, jump-to-spell navigation, and tooltip improvements.

---

## Phase 4: Validation and Assets UI Improvements (2025-01-XX)

- Implemented a validation module and an asset manager status tracker.
- UI improvements to validation panel and assets panel.

---

## Phase 5: Testing Infrastructure Improvements (2025-12-XX)

- Created `test_utils` for regex-based code checks.
- Added compliance tests and ComboBox ID salt validators.

---

## Phase 6: Data Files Update (2025-01-XX)

- Created `data/races.ron` and updated other core data files to fill out fields and add `icon_path` or `applied_conditions` as needed.
- Ensured all data files parse as RON.

---

## Phase 7: Logging and Developer Experience (2025-01-XX)

- Added a logging module and integrated debug UI (F12).
- Logging flags `--verbose`/`-v`, `--debug`/`-d`, `--quiet`/`-q` added.

---

## Phase 9: Maps Editor Major Refactor (2025-01-XX)

- Converted Maps Editor to standard pattern, added TwoColumnLayout and ActionButtons.
- Added zoom controls and preview thumbnails.

---

## Stat Ranges Documentation (2025-01-XX)

- Added stat range constants to `src/domain/character.rs` and documentation `docs/reference/stat_ranges.md`.

---

## Phase 10: Final Polish and Verification (2025-01-XX)

- Editor pattern compliance check (EditorToolbar, ActionButtons, TwoColumnLayout).
- Enforced consistency across all editors and added compliance tests and more unit tests to the editors.

---

## Character Definition System - Phase 1: Domain Types (2025-01-XX)

### Background

The character definition system introduces data-driven character templates that can be
defined in RON files and used to create pre-made characters, NPCs, and character templates
for campaigns. This separates **character templates** (data definitions) from **character
instances** (runtime state).

### Changes Implemented

#### 1.1 Created CharacterDefinition Module (`src/domain/character_definition.rs`)

- **CharacterDefinitionId**: Type alias (`String`) for unique character definition identifiers
- **CharacterDefinitionError**: Error enum with variants:
  - `CharacterNotFound` - Definition not found in database
  - `LoadError` - Failed to load from file
  - `ParseError` - RON parsing failed
  - `ValidationError` - Definition validation failed
  - `DuplicateId` - Duplicate ID in database
  - `InvalidRaceId` / `InvalidClassId` / `InvalidItemId` - Invalid references

#### 1.2 Created StartingEquipment Struct

Mirrors the Equipment struct for defining starting gear:

- `weapon`, `armor`, `shield`, `helmet`, `boots`, `accessory1`, `accessory2`
- Helper methods: `new()`, `is_empty()`, `equipped_count()`, `all_item_ids()`
- Full Serialize/Deserialize support with `#[serde(default)]` for optional fields

#### 1.3 Created BaseStats Struct

Simple stat values for character definitions (before AttributePair conversion):

- All seven stats: `might`, `intellect`, `personality`, `endurance`, `speed`, `accuracy`, `luck`
- `to_stats()` method converts to runtime `Stats` type with `AttributePair` values
- Default values of 10 for all stats

#### 1.4 Created CharacterDefinition Struct

Complete template for character creation:

- `id`: Unique identifier (e.g., "pregen_human_knight")
- `name`: Character display name
- `race_id`: Reference to races.ron
- `class_id`: Reference to classes.ron
- `sex`: Character sex (reuses existing enum)
- `alignment`: Starting alignment (reuses existing enum)
- `base_stats`: Starting stats (BaseStats)
- `portrait_id`: Portrait/avatar identifier
- `starting_gold`, `starting_gems`, `starting_food`: Initial resources
- `starting_items`: Items to add to inventory
- `starting_equipment`: Items to equip
- `description`: Character backstory/bio
- `is_premade`: Distinguishes pre-made vs template characters

#### 1.5 Created CharacterDatabase Struct

Database for managing character definitions:

- `new()` - Creates empty database
- `load_from_file()` - Loads from RON file
- `load_from_string()` - Loads from RON string (with validation)
- `add_character()` / `remove_character()` - Mutation methods
- `get_character()` - Lookup by ID
- `all_characters()` / `all_character_ids()` - Iteration
- `premade_characters()` / `template_characters()` - Filtered iteration
- `validate()` - Validates all definitions
- `merge()` - Combines two databases
- `len()` / `is_empty()` - Size queries

#### 1.6 Updated Domain Module Exports

Updated `src/domain/mod.rs` to export:

- `character_definition` module
- Public types: `BaseStats`, `CharacterDatabase`, `CharacterDefinition`,
  `CharacterDefinitionError`, `CharacterDefinitionId`, `StartingEquipment`

### Tests Added

34 comprehensive unit tests covering:

**StartingEquipment tests:**

- `test_starting_equipment_new` - Empty construction
- `test_starting_equipment_is_empty` - Empty detection
- `test_starting_equipment_equipped_count` - Count calculation
- `test_starting_equipment_all_item_ids` - Item ID extraction
- `test_starting_equipment_serialization` - RON round-trip

**BaseStats tests:**

- `test_base_stats_new` - Construction with values
- `test_base_stats_default` - Default values
- `test_base_stats_to_stats` - Conversion to runtime Stats
- `test_base_stats_serialization` - RON round-trip

**CharacterDefinition tests:**

- `test_character_definition_new` - Basic construction
- `test_character_definition_all_item_ids` - Combined item extraction
- `test_character_definition_validate_success` - Valid definition
- `test_character_definition_validate_empty_id` - Empty ID validation
- `test_character_definition_validate_empty_name` - Empty name validation
- `test_character_definition_validate_empty_race_id` - Empty race_id validation
- `test_character_definition_validate_empty_class_id` - Empty class_id validation
- `test_character_definition_serialization` - RON round-trip

**CharacterDatabase tests:**

- `test_character_database_new` - Empty database
- `test_character_database_add_character` - Adding definitions
- `test_character_database_add_duplicate_error` - Duplicate ID detection
- `test_character_database_remove_character` - Removal
- `test_character_database_get_character` - Lookup
- `test_character_database_all_characters` - Iteration
- `test_character_database_all_character_ids` - ID iteration
- `test_character_database_premade_characters` - Premade filtering
- `test_character_database_template_characters` - Template filtering
- `test_character_database_validate` - Database validation
- `test_character_database_merge` - Database merging
- `test_character_database_merge_duplicate_error` - Merge conflict
- `test_character_database_load_from_string` - Full RON loading
- `test_character_database_load_minimal` - Minimal RON with defaults
- `test_character_database_load_duplicate_id_error` - Duplicate in RON
- `test_character_database_load_invalid_ron` - Parse error handling
- `test_character_database_load_validation_error` - Validation during load

### Validation

All quality gates pass:

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - Zero errors
- `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- `cargo test --all-features` - 272 tests pass (34 new)

### Architecture Compliance

- Follows `classes.rs` and `races.rs` patterns exactly
- Uses existing type aliases (`RaceId`, `ClassId`, `ItemId`)
- Uses existing enums (`Sex`, `Alignment`)
- RON format for serialization (not JSON/YAML)
- Proper error types with `thiserror`
- Comprehensive doc comments with examples on all public items

### Success Criteria Met

- [x] `CharacterDefinitionId` type alias defined
- [x] `CharacterDefinition` struct with all required fields
- [x] `StartingEquipment` struct for slot-based gear
- [x] `CharacterDefinitionError` enum for validation errors
- [x] `CharacterDatabase` with load/get/validate methods
- [x] Serde support with appropriate defaults
- [x] Module exported from `src/domain/mod.rs`
- [x] > 80% test coverage (34 tests)
- [x] All cargo quality checks pass

### Files Created

- `src/domain/character_definition.rs` - Complete domain types (1643 lines)

### Files Modified

- `src/domain/mod.rs` - Added module export and re-exports

### Next Steps (Phase 2)

- Create `data/characters.ron` with pre-made characters
- Create `campaigns/tutorial/data/characters.ron` with tutorial characters
- Integration tests for data file loading

---

## Phase 2: Character Definition Data Files (Character Definition Implementation Plan) (2025-01-XX)

**Objective**: Create RON data files with character definitions for core data and the tutorial campaign.

### Background

Per the Character Definition Implementation Plan (`docs/explanation/character_definition_implementation_plan.md`), Phase 2 creates the actual data files that define pre-made characters, NPCs, and character templates. These files follow the RON format established by other domain types (classes.ron, items.ron, races.ron).

### Changes Implemented

#### 2.1 Core Characters Data File

Created `data/characters.ron` with 6 pre-made characters (one per class):

| Character ID            | Name             | Race     | Class    | Alignment | Role                  |
| ----------------------- | ---------------- | -------- | -------- | --------- | --------------------- |
| `pregen_human_knight`   | Sir Aldric       | Human    | Knight   | Good      | Tank/Melee            |
| `pregen_elf_paladin`    | Sister Elara     | Elf      | Paladin  | Good      | Hybrid Fighter/Healer |
| `pregen_halfelf_archer` | Finn Swiftarrow  | Half-Elf | Archer   | Neutral   | Ranged DPS            |
| `pregen_dwarf_cleric`   | Brother Marcus   | Dwarf    | Cleric   | Good      | Healer/Support        |
| `pregen_gnome_sorcerer` | Lyria Starweaver | Gnome    | Sorcerer | Neutral   | Arcane Caster         |
| `pregen_halforc_robber` | Shadow           | Half-Orc | Robber   | Neutral   | Utility/Stealth       |

All characters:

- Have balanced stats appropriate for level 1 (3-18 range)
- Include starting equipment referencing valid `items.ron` IDs
- Have descriptive backstories
- Are marked as `is_premade: true`

#### 2.2 Tutorial Campaign Characters Data File

Created `campaigns/tutorial/data/characters.ron` with 9 characters in three categories:

**Tutorial Pre-Made Characters (3):**
| Character ID | Name | Race | Class | Purpose |
|--------------|------|------|-------|---------|
| `tutorial_human_knight` | Kira | Human | Knight | Learning combat basics |
| `tutorial_elf_sorcerer` | Sage | Elf | Sorcerer | Learning magic basics |
| `tutorial_human_cleric` | Mira | Human | Cleric | Learning support mechanics |

**Recruitable NPCs (3):**
| Character ID | Name | Race | Class | Description |
|--------------|------|------|-------|-------------|
| `npc_old_gareth` | Old Gareth | Dwarf | Knight | Retired adventurer at smithy |
| `npc_whisper` | Whisper | Half-Elf | Robber | Thief/locksmith NPC |
| `npc_apprentice_zara` | Apprentice Zara | Gnome | Sorcerer | Quest reward recruit |

**Character Templates (3):**
| Template ID | Name | Race | Class | Purpose |
|-------------|------|------|-------|---------|
| `template_human_fighter` | Human Fighter | Human | Knight | Character generation |
| `template_elf_mage` | Elf Mage | Elf | Sorcerer | Character generation |
| `template_dwarf_cleric` | Dwarf Cleric | Dwarf | Cleric | Character generation |

#### 2.3 Data File Format

Both files follow the established RON patterns:

- Header comments explaining the format and references
- Consistent field ordering matching `CharacterDefinition` struct
- Comments for each character section
- All `race_id` values reference valid IDs from `races.ron`
- All `class_id` values reference valid IDs from `classes.ron`
- All item IDs reference valid IDs from `items.ron`

### Tests Added

Added 6 integration tests to `src/domain/character_definition.rs`:

**Core Data File Tests:**

- `test_load_core_characters_data_file` - Verifies `data/characters.ron` loads successfully, contains 6 characters, all have expected IDs, and all are pre-made
- `test_core_characters_have_valid_references` - Validates race_id and class_id against known valid values, checks stat ranges (3-18), verifies descriptions exist

**Tutorial Campaign Tests:**

- `test_load_tutorial_campaign_characters` - Verifies tutorial characters.ron loads, has 9+ characters, pre-made/NPC/template categorization
- `test_tutorial_campaign_characters_valid_references` - Validates all race_id and class_id values

**Cross-Validation Tests:**

- `test_premade_vs_template_characters` - Verifies core data has only pre-made characters, tutorial has both pre-made and templates/NPCs
- `test_character_starting_equipment_items_exist` - Validates all starting_items and starting_equipment IDs reference valid item IDs from `items.ron`

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 512 unit tests pass, 272 doc tests pass

### Architecture Compliance

- [x] RON format used for data files (not JSON/YAML)
- [x] Data files follow patterns from `classes.ron` and `items.ron`
- [x] All referenced IDs exist in corresponding data files
- [x] Comments explain format and references
- [x] Character stats use standard RPG range (3-18)
- [x] Pre-made characters have backstory descriptions

### Success Criteria Met

- [x] `data/characters.ron` with 6 pre-made characters (one per class)
- [x] `campaigns/tutorial/data/characters.ron` with 9 characters
- [x] Both files parse via `CharacterDatabase::load_from_file()`
- [x] All referenced IDs validated against source files
- [x] Data file comments clear and follow existing patterns
- [x] Integration tests for both data files

### Files Created

- `data/characters.ron` - Core pre-made characters (240 lines)
- `campaigns/tutorial/data/characters.ron` - Tutorial campaign characters (349 lines)

### Files Modified

- `src/domain/character_definition.rs` - Added 6 integration tests (297 lines)

### Next Steps (Phase 3)

- Add `CharacterDatabase` to `ContentDatabase` in SDK
- Update campaign loader to load `characters.ron`
- Add validation rules for character references

---

## Phase 3: SDK Integration (Character Definition Implementation Plan) (2025-01-XX)

**Objective**: Integrate character definitions into the SDK content database and campaign loader, enabling automatic loading and validation of character data.

### Background

Per the Character Definition Implementation Plan (`docs/explanation/character_definition_implementation_plan.md`), Phase 3 integrates the character definition system into the SDK's `ContentDatabase`. This allows campaigns to define characters in `characters.ron` files that are automatically loaded and validated against races, classes, and items.

### Changes Implemented

#### 3.1 Updated DatabaseError Enum

Added to `src/sdk/database.rs`:

- `CharacterLoadError(String)` variant for character loading failures

#### 3.2 Updated ContentDatabase Struct

Modified `src/sdk/database.rs`:

- Added `pub characters: CharacterDatabase` field
- Updated `ContentDatabase::new()` to initialize empty `CharacterDatabase`
- Updated `ContentDatabase::load_campaign()` to load `characters.ron` from campaign data directory
- Updated `ContentDatabase::load_core()` to load `characters.ron` from core data directory

#### 3.3 Updated ContentDatabase::validate()

Added comprehensive character validation:

- Validates `CharacterDatabase.validate()` for internal consistency
- Validates each character's `race_id` against loaded `RaceDatabase`
- Validates each character's `class_id` against loaded `ClassDatabase`
- Validates each character's `starting_items` against loaded `ItemDatabase`
- Validates each character's `starting_equipment` against loaded `ItemDatabase`

#### 3.4 Updated ContentStats Struct

Added to `src/sdk/database.rs`:

- `pub character_count: usize` field
- Updated `ContentStats::total()` to include character count
- Updated `ContentDatabase::stats()` to populate character count

#### 3.5 Added ValidationCategory::Characters

Updated `sdk/campaign_builder/src/validation.rs`:

- Added `Characters` variant to `ValidationCategory` enum
- Added display name "Characters"
- Added icon "🧑" for the category
- Added to `ValidationCategory::all()` list

#### 3.6 Added Character Validation to Validator

Updated `src/sdk/validation.rs`:

- Added `validate_character_references()` method
- Validates character race_id references
- Validates character class_id references
- Validates character starting_items references
- Validates character starting_equipment references
- Integrated into `validate_references()` method

#### 3.7 Added Helper Methods to Domain Types

Added convenience methods for testing:

- `RaceDefinition::new(id, name, description)` - Creates race with defaults
- `ClassDefinition::new(id, name)` - Creates class with defaults
- `ClassDatabase::add_class(class)` - Adds class to database
- `RaceDatabase::has_race(id)` - Checks if race exists

#### 3.8 Fixed Tutorial Campaign Data

Updated `campaigns/tutorial/data/characters.ron`:

- Changed `npc_whisper` from `race_id: "half_elf"` to `race_id: "elf"`
- Tutorial campaign only has 4 races (human, elf, dwarf, gnome)
- Updated character description to match

### Tests Added

**ContentDatabase Tests (src/sdk/database.rs):**

- `test_content_database_character_loading` - Verifies manual character addition
- `test_content_database_load_core_characters` - Tests loading from core data directory
- `test_content_database_load_campaign_characters` - Tests loading from campaign directory
- `test_content_database_validate_with_characters` - Valid references pass validation
- `test_content_database_validate_invalid_race_reference` - Invalid race detected
- `test_content_database_validate_invalid_class_reference` - Invalid class detected
- `test_content_database_validate_invalid_item_reference` - Invalid item detected
- `test_content_stats_includes_characters` - Character count in stats total

**Validator Tests (src/sdk/validation.rs):**

- `test_validator_character_references_valid` - Valid references produce no errors
- `test_validator_character_invalid_race` - Missing race generates error
- `test_validator_character_invalid_class` - Missing class generates error
- `test_validator_character_invalid_starting_items` - Missing items generate errors
- `test_validator_character_invalid_starting_equipment` - Missing equipment generates errors

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 276 doc tests pass, all unit tests pass

### Architecture Compliance

- [x] Uses existing `CharacterDatabase` from domain layer
- [x] Follows pattern from other database types (classes, races, items)
- [x] Validation checks cross-references against loaded databases
- [x] Error messages include context (character ID, invalid reference)
- [x] ContentStats includes character_count
- [x] ValidationCategory has Characters variant

### Success Criteria Met

- [x] `ContentDatabase::load_campaign()` loads characters.ron
- [x] `ContentDatabase::load_core()` loads characters.ron
- [x] `ContentDatabase::validate()` validates character references
- [x] Invalid race/class/item references detected and reported
- [x] `ContentStats.character_count` tracks loaded characters
- [x] `ValidationCategory::Characters` added for campaign builder
- [x] All tests pass including integration tests

### Files Modified

- `src/sdk/database.rs` - Added CharacterDatabase support, validation, stats
- `src/sdk/validation.rs` - Added character reference validation
- `sdk/campaign_builder/src/validation.rs` - Added Characters category
- `src/domain/races.rs` - Added `RaceDefinition::new()` and `RaceDatabase::has_race()`
- `src/domain/classes.rs` - Added `ClassDefinition::new()` and `ClassDatabase::add_class()`
- `campaigns/tutorial/data/characters.ron` - Fixed invalid race reference

### Next Steps (Phase 4)

- Create `sdk/campaign_builder/src/characters_editor.rs` with visual editor
- Add Characters tab to Campaign Builder UI
- Implement list/add/edit modes with filters
- Add race/class dropdowns, item picker, portrait selector

---

## Phase 4: SDK Character Editor (Character Definition Implementation Plan) (2025-01-XX)

### Background

Following the completion of Phase 3 (SDK Integration), Phase 4 implements the
visual editor for character definitions in the Campaign Builder. This editor
allows campaign designers to create, edit, and manage character definitions
(both pre-made characters and templates) through a graphical interface.

### Changes Implemented

#### 4.1 Created CharactersEditorState (`sdk/campaign_builder/src/characters_editor.rs`)

- `CharactersEditorMode` enum with `List`, `Add`, `Edit` variants
- `CharactersEditorState` struct with:
  - Characters list and selection tracking
  - Search filter and multiple filter options (race, class, alignment, premade)
  - Edit buffer for form fields
  - Unsaved changes tracking
- `CharacterEditBuffer` struct for form field values with string representations

#### 4.2 Implemented Editor State Management

- `start_new_character()` - Initialize buffer for new character
- `start_edit_character(idx)` - Populate buffer from existing character
- `save_character()` - Validate and save buffer to characters list
- `delete_character(idx)` - Remove character at index
- `cancel_edit()` - Return to list mode without saving
- `filtered_characters()` - Apply all filters to character list
- `next_available_character_id()` - Generate unique ID
- `clear_filters()` - Reset all filter options

#### 4.3 Implemented Editor UI

- `show()` - Main UI rendering method with toolbar and mode routing
- `show_filters()` - Filter controls with dropdowns for race, class, alignment
- `show_list()` - Two-column layout with character list and preview panel
- `show_character_preview()` - Detailed view of selected character
- `show_character_form()` - Add/Edit form with all CharacterDefinition fields
- `show_equipment_editor()` - Equipment slot editors with item selection
- `show_item_selector()` - Reusable item dropdown with ID input

#### 4.4 Integrated into Campaign Builder

- Added `mod characters_editor` declaration to `main.rs`
- Added `EditorTab::Characters` variant to tab enum
- Added `characters_file` field to `CampaignMetadata` struct
- Added `characters_editor_state` field to `CampaignBuilderApp`
- Added Characters tab to sidebar tabs list
- Added match arm for `EditorTab::Characters` rendering
- Added `load_characters_from_campaign()` method
- Added `load_races_from_campaign()` method (needed for race dropdown)
- Connected character loading to campaign open workflow

#### 4.5 Filter System

Implemented comprehensive filtering:

- Search filter (matches name or ID)
- Race filter (dropdown populated from loaded races)
- Class filter (dropdown populated from loaded classes)
- Alignment filter (Good/Neutral/Evil)
- Premade only checkbox

### Tests Added

```rust
// CharactersEditorState tests
test_characters_editor_state_creation      - Default state initialization
test_start_new_character                   - Mode transition to Add
test_save_character_creates_new            - Character creation from buffer
test_save_character_empty_id_error         - ID validation
test_save_character_empty_name_error       - Name validation
test_save_character_empty_race_error       - Race ID validation
test_save_character_empty_class_error      - Class ID validation
test_save_character_duplicate_id_error     - Duplicate ID detection
test_delete_character                      - Character deletion
test_cancel_edit                           - Edit cancellation
test_filtered_characters                   - All filter combinations
test_next_available_character_id           - ID generation
test_start_edit_character                  - Edit mode initialization
test_edit_character_saves_changes          - Update existing character
test_character_edit_buffer_default         - Buffer default values
test_editor_mode_transitions               - Complete mode flow
test_clear_filters                         - Filter reset
test_sex_name_helper                       - Sex display name
test_alignment_name_helper                 - Alignment display name
test_save_character_with_starting_items    - Starting items parsing
test_save_character_with_equipment         - Equipment slot parsing
test_save_character_invalid_stat           - Stat validation error
test_save_character_invalid_gold           - Gold validation error
test_has_unsaved_changes_flag              - Change tracking
```

### Validation

```bash
cargo fmt --all                                    # ✓ No changes
cargo check --all-targets --all-features           # ✓ 0 errors
cargo clippy --all-targets --all-features -- -D warnings  # ✓ 0 warnings
cargo test --all-features                          # ✓ 276 tests passed
```

### Architecture Compliance

- [x] Follows existing editor patterns (classes_editor, races_editor, items_editor)
- [x] Uses shared UI components (EditorToolbar, ActionButtons, TwoColumnLayout)
- [x] Uses type aliases (ItemId, RaceId, ClassId)
- [x] Uses RON format for data files
- [x] Implements proper validation with descriptive error messages
- [x] Separates state from UI rendering
- [x] Includes comprehensive unit tests

### Success Criteria Met

- [x] Characters tab appears in Campaign Builder
- [x] Can create new character definitions
- [x] Can edit existing character definitions
- [x] Can delete character definitions
- [x] Changes save to characters.ron in correct format
- [x] Race dropdown populated from loaded races
- [x] Class dropdown populated from loaded classes
- [x] Filters work correctly (search, race, class, alignment, premade)
- [x] Starting equipment slots editable with item picker
- [x] Starting items editable as comma-separated IDs
- [x] All quality checks pass

### Files Created

- `sdk/campaign_builder/src/characters_editor.rs` - Full visual editor

### Files Modified

- `sdk/campaign_builder/src/main.rs`:
  - Added `mod characters_editor` declaration
  - Added `EditorTab::Characters` enum variant
  - Added `characters_file` to `CampaignMetadata`
  - Added `characters_editor_state` to `CampaignBuilderApp`
  - Added Characters to sidebar tabs array
  - Added rendering match arm for Characters tab
  - Added `load_characters_from_campaign()` method
  - Added `load_races_from_campaign()` method

### Next Steps (Phase 6)

- Create documentation (`docs/how-to/create_characters.md`)
- Update `docs/reference/architecture.md` with CharacterDefinition types
- Add integration with new game character selection flow

---

## Phase 5: Character Instantiation (2025-01-XX)

**Objective**: Implement the mechanism to create runtime `Character` objects from `CharacterDefinition` templates, completing the data-driven character system.

### Background

Per the Character Definition Implementation Plan (`docs/explanation/character_definition_implementation_plan.md`), Phase 5 adds the instantiation layer that bridges character templates (defined in RON files) with runtime Character instances used during gameplay.

### Changes Implemented

#### 5.1 Added New Error Variants

Extended `CharacterDefinitionError` in `src/domain/character_definition.rs`:

- `InstantiationError { character_id, message }` - General instantiation failures
- `InventoryFull { character_id, item_id }` - Inventory overflow during population

#### 5.2 Added Required Imports

Added imports to `src/domain/character_definition.rs`:

- `Character`, `Class`, `Race`, `Equipment`, `Inventory`, `InventorySlot` from character module
- `AttributePair`, `AttributePair16`, `Condition`, `SpellBook`, `QuestFlags`
- `ClassDatabase`, `ClassDefinition`, `SpellStat` from classes module
- `RaceDatabase`, `RaceDefinition` from races module
- `ItemDatabase` from items module

#### 5.3 Implemented `instantiate()` Method

Added to `CharacterDefinition`:

```rust
pub fn instantiate(
    &self,
    races: &RaceDatabase,
    classes: &ClassDatabase,
    items: &ItemDatabase,
) -> Result<Character, CharacterDefinitionError>
```

The method:

1. Validates race_id exists in RaceDatabase
2. Validates class_id exists in ClassDatabase
3. Validates all starting item IDs exist in ItemDatabase
4. Converts race_id/class_id to Race/Class enums
5. Applies race stat modifiers to base stats
6. Calculates starting HP based on class HP die and endurance
7. Calculates starting SP based on class spell_stat
8. Applies race resistances
9. Populates inventory with starting items
10. Creates equipment from starting equipment
11. Returns fully initialized Character

#### 5.4 Implemented Helper Functions

**`race_enum_from_id(race_id: &str) -> Option<Race>`**

- Converts race ID strings to Race enum
- Case-insensitive matching
- Supports variants: human, elf, dwarf, gnome, half_elf/halfelf/half-elf, half_orc/halforc/half-orc

**`class_enum_from_id(class_id: &str) -> Option<Class>`**

- Converts class ID strings to Class enum
- Case-insensitive matching
- Supports: knight, paladin, archer, cleric, sorcerer, robber

**`apply_race_modifiers(base_stats: &BaseStats, race_def: &RaceDefinition) -> Stats`**

- Applies race stat modifiers to base stats
- Clamps results to valid range (3-25)
- Creates Stats struct with AttributePair values

**`calculate_starting_hp(class_def: &ClassDefinition, endurance: u8) -> AttributePair16`**

- Uses max roll of class HP die for consistent premade characters
- Applies endurance modifier: (endurance - 10) / 2
- Ensures minimum 1 HP

**`calculate_starting_sp(class_def: &ClassDefinition, stats: &Stats) -> AttributePair16`**

- Non-casters get 0 SP
- Pure casters: (relevant_stat - 10), minimum 0
- Hybrid casters (Paladin): (relevant_stat - 10) / 2, minimum 0
- Uses Intellect for Sorcerers, Personality for Clerics/Paladins

**`calculate_starting_spell_level(class_def: &ClassDefinition) -> u8`**

- Pure casters start at spell level 1
- Hybrid and non-casters start at spell level 0

**`apply_race_resistances(race_def: &RaceDefinition) -> CharacterResistances`**

- Converts race resistance values (u8) to CharacterResistances (AttributePair)
- Preserves all eight resistance types

**`populate_starting_inventory(character_id: &str, starting_items: &[ItemId]) -> Result<Inventory, CharacterDefinitionError>`**

- Creates inventory with starting items
- Returns InventoryFull error if too many items

**`create_starting_equipment(starting_equipment: &StartingEquipment) -> Equipment`**

- Maps StartingEquipment slots to Equipment struct
- Handles all seven equipment slots

#### 5.5 Added HalfElf Race Support

Modified `src/domain/character.rs`:

- Added `HalfElf` variant to `Race` enum
- Updated `race_id_from_enum()` to return "half_elf"
- Updated `race_enum_from_id()` to recognize "half_elf"

### Tests Added

**Helper Function Tests (26 new tests):**

```
test_race_enum_from_id_valid               - All valid race IDs including case variants
test_race_enum_from_id_invalid             - Invalid and empty race IDs
test_class_enum_from_id_valid              - All valid class IDs including case variants
test_class_enum_from_id_invalid            - Invalid and empty class IDs
test_apply_race_modifiers_no_modifiers     - Human with no modifiers
test_apply_race_modifiers_with_bonuses     - Elf with +/- modifiers
test_apply_race_modifiers_clamping         - Lower and upper bound clamping
test_calculate_starting_hp_knight          - Knight HP with various endurance
test_calculate_starting_hp_minimum         - Minimum 1 HP guarantee
test_calculate_starting_sp_non_caster      - Knight gets 0 SP
test_calculate_starting_sp_pure_caster     - Sorcerer SP calculation
test_calculate_starting_sp_hybrid_caster   - Paladin SP calculation
test_calculate_starting_spell_level        - All class types
test_apply_race_resistances                - Dwarf resistance values
test_populate_starting_inventory_empty     - Empty inventory creation
test_populate_starting_inventory_with_items - Items added correctly
test_populate_starting_inventory_full      - InventoryFull error
test_create_starting_equipment_empty       - All slots empty
test_create_starting_equipment_with_items  - Slots populated correctly
```

**Integration Tests (7 new tests):**

```
test_instantiate_with_real_databases       - Full instantiation with real data files
test_instantiate_invalid_race              - InvalidRaceId error
test_instantiate_invalid_class             - InvalidClassId error
test_instantiate_invalid_item              - InvalidItemId error
test_instantiate_all_core_characters       - All 6 core characters instantiate
test_instantiate_sorcerer_has_sp           - Sorcerer has SP > 0, spell_level = 1
test_instantiate_knight_has_no_sp          - Knight has SP = 0, spell_level = 0
```

### Validation

```bash
cargo fmt --all                                    # ✓ No changes
cargo check --all-targets --all-features           # ✓ 0 errors
cargo clippy --all-targets --all-features -- -D warnings  # ✓ 0 warnings
cargo test --all-features                          # ✓ 551 lib tests + 277 doc tests passed
cargo test --lib character_definition::            # ✓ 66 tests passed
```

### Architecture Compliance

- [x] Uses type aliases (ItemId, RaceId, ClassId) consistently
- [x] Uses AttributePair pattern for modifiable stats (HP, SP, resistances)
- [x] Constants used where appropriate (Inventory::MAX_ITEMS)
- [x] Error handling follows thiserror pattern
- [x] Race/class modifiers applied through data-driven RaceDefinition/ClassDefinition
- [x] RON format used for all data files
- [x] Comprehensive doc comments with examples

### Success Criteria Met

- [x] Can create a fully functional Character from any CharacterDefinition
- [x] Starting items appear in inventory
- [x] Starting equipment is equipped
- [x] Stats reflect race modifiers
- [x] HP calculated from class HP die and endurance
- [x] SP calculated from class spell_stat and relevant stat
- [x] Resistances reflect race resistances
- [x] All core data file characters instantiate successfully
- [x] All tests pass

### Files Modified

- `src/domain/character_definition.rs`:

  - Added imports for Character, databases, and types
  - Added InstantiationError and InventoryFull error variants
  - Added `instantiate()` method to CharacterDefinition
  - Added 8 helper functions for instantiation steps
  - Added 33 new tests for instantiation functionality

- `src/domain/character.rs`:
  - Added `HalfElf` variant to Race enum
  - Updated `race_id_from_enum()` for HalfElf
  - Updated `race_enum_from_id()` for HalfElf

### Integration Points

The `instantiate()` method can be used in:

1. **New Game Character Selection**: When player selects a premade character
2. **NPC Recruitment**: When an NPC joins the party during gameplay
3. **Test/Debug Character Creation**: For automated testing
4. **Campaign Builder Preview**: To preview character stats in the editor

### Example Usage

```rust
use antares::domain::character_definition::CharacterDatabase;
use antares::domain::races::RaceDatabase;
use antares::domain::classes::ClassDatabase;
use antares::domain::items::ItemDatabase;

// Load databases
let races = RaceDatabase::load_from_file("data/races.ron")?;
let classes = ClassDatabase::load_from_file("data/classes.ron")?;
let items = ItemDatabase::load_from_file("data/items.ron")?;
let characters = CharacterDatabase::load_from_file("data/characters.ron")?;

// Instantiate a premade character
let knight_def = characters.get_character("pregen_human_knight").unwrap();
let knight = knight_def.instantiate(&races, &classes, &items)?;

assert_eq!(knight.name, "Sir Galahad");
assert_eq!(knight.race, Race::Human);
assert_eq!(knight.class, Class::Knight);
assert!(knight.hp.base > 0);
assert!(knight.is_alive());
```

---

## Phase 6: Documentation and Cleanup (Character Definition Implementation Plan) (2025-01-XX)

**Objective**: Complete documentation and finalize the character definition system
implementation with architecture documentation, usage guides, and code cleanup.

### Background

Per the Character Definition Implementation Plan (`docs/explanation/character_definition_implementation_plan.md`),
Phase 6 completes the character definition system by updating all relevant documentation,
creating a how-to guide for campaign designers, and ensuring code quality.

### Changes Implemented

#### 6.1 Updated Architecture Documentation

Updated `docs/reference/architecture.md`:

- Added Section 4.7: Character Definition (Data-Driven Templates)

  - `StartingEquipment` struct with slot-based equipment specification
  - `BaseStats` struct for pre-modifier stat values
  - `CharacterDefinition` struct with all template fields
  - `CharacterDatabase` struct for loading and managing definitions
  - `CharacterDefinitionError` enum for error handling
  - Instantiation flow documentation
  - Example usage code

- Added `characters.ron` to Section 7.1 External Data Files listing
- Added campaign-specific character file path documentation
- Added Character Definition RON format example to Section 7.2
- Documented character definition fields and instantiation process

#### 6.2 Updated Implementation Documentation

Updated `docs/explanation/implementations.md`:

- Documented Phase 6 completion
- Linked all documentation updates
- Summarized character definition system components

#### 6.3 Created How-To Guide

Created `docs/how-to/create_characters.md`:

- Overview of character definition system
- Character types (premade vs template)
- File locations (core vs campaign-specific)
- Campaign Builder usage instructions
- Manual RON file editing guide
- Complete field reference table
- Balancing guidelines with stat recommendations by class
- Race modifier reference
- Starting equipment tier guidelines
- Instantiation process explanation with examples
- Common patterns (premade party, NPC templates)
- Validation instructions and common errors
- Game code integration examples
- Tips and best practices

#### 6.4 Updated Campaign Loader

Updated `src/sdk/campaign_loader.rs`:

- Added `characters` field to `CampaignData` struct
- Added `default_characters_path()` helper function returning `"data/characters.ron"`
- Added `characters_file` field to `CampaignMetadata` struct
- Updated `TryFrom<CampaignMetadata>` implementation to map characters field
- Updated test `test_campaign_data_defaults` to verify characters path

Updated dependent files:

- `src/application/save_game.rs`: Added characters field to test
- `src/sdk/campaign_packager.rs`: Added characters field to two tests
- `tests/phase14_campaign_integration_test.rs`: Added characters field to test helper

#### 6.5 Code Verification

Verified code quality:

- No TODO/FIXME/XXX comments in character_definition.rs
- All public APIs have doc comments with examples
- Code passes all quality checks

### Validation

```bash
cargo fmt --all                                    # ✓ No changes
cargo check --all-targets --all-features           # ✓ 0 errors
cargo clippy --all-targets --all-features -- -D warnings  # ✓ 0 warnings
cargo test --all-features                          # ✓ All tests passed
```

### Architecture Compliance

- [x] Documentation follows Diataxis framework (how-to in how-to/, reference in reference/)
- [x] Markdown filenames use lowercase_with_underscores.md
- [x] RON format documented and exemplified correctly
- [x] Architecture document updated with new data structures
- [x] Type aliases documented (CharacterDefinitionId, RaceId, ClassId)
- [x] No emojis in documentation

### Success Criteria Met

- [x] Architecture documentation is complete and accurate
- [x] Implementation documentation updated
- [x] How-to guide created for character creation
- [x] All public APIs have doc comments with examples
- [x] Code passes all quality checks
- [x] No TODO comments remaining in implementation

### Files Created

- `docs/how-to/create_characters.md`: Step-by-step guide for creating character definitions

### Files Modified

- `docs/reference/architecture.md`:

  - Added Section 4.7 Character Definition
  - Added type aliases (CharacterDefinitionId, RaceId, ClassId)
  - Updated Section 7.1 data files listing
  - Added Section 7.2 Character Definition RON example

- `docs/explanation/implementations.md`:

  - Added Phase 6 documentation

- `src/sdk/campaign_loader.rs`:

  - Added `characters` field to `CampaignData` struct
  - Added `default_characters_path()` function
  - Added `characters_file` field to `CampaignMetadata` struct
  - Updated `TryFrom<CampaignMetadata>` implementation
  - Updated `test_campaign_data_defaults` test

- `src/application/save_game.rs`:

  - Added characters field to test CampaignData

- `src/sdk/campaign_packager.rs`:

  - Added characters field to test CampaignData (2 instances)

- `tests/phase14_campaign_integration_test.rs`:
  - Added characters field to test helper function

### Character Definition System Summary

The complete character definition system now includes:

| Component       | Location                                        | Purpose                      |
| --------------- | ----------------------------------------------- | ---------------------------- |
| Domain Types    | `src/domain/character_definition.rs`            | Core data structures         |
| Core Data       | `data/characters.ron`                           | Default premade characters   |
| Campaign Data   | `campaigns/*/data/characters.ron`               | Campaign-specific characters |
| SDK Integration | `src/sdk/database.rs`                           | ContentDatabase loading      |
| SDK Editor      | `sdk/campaign_builder/src/characters_editor.rs` | Visual editor                |
| Validation      | `sdk/campaign_builder/src/validation.rs`        | Reference validation         |
| Architecture    | `docs/reference/architecture.md`                | Technical specification      |
| How-To Guide    | `docs/how-to/create_characters.md`              | Usage documentation          |

### Next Steps (Future Enhancements)

The character definition system is complete. Potential future enhancements:

1. **Level Scaling**: Add `starting_level` field for higher-level premade characters
2. **Portrait System**: Extend portrait_id to support path-based custom portraits
3. **NPC Extensions**: Add fields for dialogue_id, location, recruitment conditions
4. **Procedural Generation**: Use templates with stat randomization ranges
5. **Import/Export**: Add character definition import/export in Campaign Builder

---
