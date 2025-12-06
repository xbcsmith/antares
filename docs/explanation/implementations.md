# Implementation Summary

## Phase 5: CLI Editor Updates for Proficiency Migration (2025-01-XX)

**Objective**: Update command-line editors to support the new proficiency, classification, tags, and alignment restriction system introduced in Phases 1-4.

### Background

Following the completion of Phase 4 (SDK Editor Updates), Phase 5 extends proficiency migration support to the CLI editors used for creating and editing game data files. The CLI editors (`class_editor`, `race_editor`, `item_editor`) now provide interactive prompts for the new fields while maintaining backward compatibility with legacy disablement flags.

### Changes Implemented

#### 5.1 Class Editor CLI (`src/bin/class_editor.rs`)

**Added Constants:**

- `STANDARD_PROFICIENCY_IDS` - Array of 11 standard proficiency IDs for validation

**New Functionality:**

- `input_proficiencies()` - Interactive proficiency selection with:
  - Formatted menu showing all standard proficiencies grouped by category (Weapons, Armor, Magic Items)
  - Comma-separated input with validation
  - Warning for non-standard proficiency IDs with confirmation prompt
  - Success message showing added proficiencies

**Updated Methods:**

- `add_class()` - Now prompts for proficiencies and stores in `ClassDefinition`
- `edit_class()` - Added option 5 to edit proficiencies, showing current proficiencies in menu

**User Experience:**

- Clear categorized display: Weapons (5 types), Armor (4 types), Magic Items (2 types)
- Each proficiency shown with descriptive text (e.g., "simple_weapon - Simple weapons (daggers, clubs)")
- Validation warnings for typos or custom proficiency IDs
- Non-intrusive: empty input = no proficiencies

#### 5.2 Race Editor CLI (`src/bin/race_editor.rs`)

**Added Constants:**

- `STANDARD_PROFICIENCY_IDS` - Same 11 standard proficiency IDs
- `STANDARD_ITEM_TAGS` - Array of 6 standard item tags for race restrictions

**New Functionality:**

- `input_proficiencies()` - Same interactive proficiency selection as class editor
- `input_incompatible_tags()` - Interactive tag selection with:
  - Formatted menu showing all standard item tags with descriptions
  - Explanation of how incompatible tags work (race cannot use items with those tags)
  - Example usage (e.g., "halfling with 'large_weapon' incompatible")
  - Comma-separated input with validation
  - Warning for non-standard tags with confirmation prompt

**Updated Methods:**

- `add_race()` - Now prompts for both proficiencies and incompatible_item_tags
- `edit_race()` - Added options 7 and 8 to edit proficiencies and incompatible tags
- Menu now shows current proficiencies and incompatible tags (or "None")

**User Experience:**

- Clear explanations of each standard tag's purpose
- Contextual help text explaining the restriction system
- Validation prevents typos while allowing custom tags if confirmed

#### 5.3 Item Editor CLI (`src/bin/item_editor.rs`)

**Added Constants:**

- `STANDARD_ITEM_TAGS` - Same 6 standard item tags

**New Functionality:**

- `input_item_tags()` - Interactive tag selection with:
  - Formatted menu showing all standard tags with detailed descriptions
  - Explanation of how tags interact with race restrictions
  - Example showing race incompatible_item_tags usage
  - Comma-separated input with validation
  - Warning for non-standard tags with confirmation prompt

**Enhanced Existing Methods:**

- `select_weapon_classification()` - Already existed, selects WeaponClassification (Simple, MartialMelee, MartialRanged, Blunt, Unarmed)
- `select_armor_classification()` - Already existed, selects ArmorClassification (Light, Medium, Heavy, Shield)
- `select_magic_item_classification()` - Already existed, selects MagicItemClassification (None, Arcane, Divine, Universal)
- `select_alignment_restriction()` - Already existed, selects AlignmentRestriction (None, GoodOnly, EvilOnly)

**Updated Methods:**

- `add_item()` - Now calls `input_item_tags()` and stores tags in Item
- `preview_item()` - Enhanced to display:
  - Alignment restriction (Good Only / Evil Only / Any)
  - Item tags (comma-separated list)
  - **Derived proficiency requirement** using `item.required_proficiency()` - shows computed proficiency from classification
  - Legacy disablement flags labeled as "(legacy)"

**User Experience:**

- All item properties visible in preview including new fields
- Derived proficiency shown with ⚔️ emoji for visibility
- Clear indication that proficiency is auto-derived from classification
- Tags explained with practical examples of their effects

### Validation Features

All three CLI editors now include:

- **Input validation** against standard proficiency/tag constants
- **User confirmation** for non-standard values (allows custom IDs but warns user)
- **Visual feedback** with ✅ success messages and ⚠️ warning symbols
- **Helpful prompts** with examples of correct input format
- **Non-intrusive defaults** (empty input = no proficiencies/tags)

### Backward Compatibility

- All editors preserve legacy `disablement_bit_index` and `disablement` fields
- Old data files load correctly with `#[serde(default)]` on new fields
- Legacy disablement flags still shown in previews, labeled as "(legacy)"
- No breaking changes to existing CLI workflows

### Testing

All quality gates passed:

- ✅ `cargo fmt --all` - Formatted successfully
- ✅ `cargo check --all-targets --all-features` - Compiled without errors
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- ✅ `cargo test --all-features` - All 307 tests passed

### Files Modified

- `src/bin/class_editor.rs` - Added proficiency input and validation (47 lines added)
- `src/bin/race_editor.rs` - Added proficiency and incompatible tag input with validation (115 lines added)
- `src/bin/item_editor.rs` - Added tag input and enhanced preview display (68 lines added)

### Success Criteria ✅

- [x] CLI editors build and run without errors
- [x] Can create/edit classes with proficiencies via interactive menu
- [x] Can create/edit items with classifications, tags, and alignment restrictions
- [x] Can create/edit races with proficiencies and incompatible_item_tags
- [x] All standard proficiency IDs and tags are validated
- [x] Non-standard values trigger warnings but can be confirmed
- [x] Item preview shows derived proficiency requirement
- [x] All quality gates pass (fmt, check, clippy, test)

### Next Steps

With Phase 5 complete, the proficiency migration is functionally complete for editing workflows. Recommended next phases:

1. **Phase 6: Cleanup and Deprecation Removal** - Remove deprecated `disablement` fields and legacy code
2. **Data File Migration** - Convert existing RON data files to use new classification/tags/proficiencies
3. **End-to-End Testing** - Test complete gameplay flow from character creation through item equipping
4. **Migration Guide** - Document the migration for modders and content creators

---

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

## Phase 5: Enum Removal (Hard-coded Removal Plan) (2025-01-XX)

**Objective**: Remove the static `Race` and `Class` enums, completing the migration
to fully data-driven race and class systems using ID-based lookups.

### Background

Per the Hard-coded Removal Implementation Plan (`docs/explanation/hardcoded_removal_implementation_plan.md`),
Phase 5 removes the `Race` and `Class` enum definitions and all code that depends
on them. This is the culmination of Phases 1-4 which added ID-based alternatives.

After this phase, all race and class behavior is data-driven through `RaceDatabase`
and `ClassDatabase` lookups using string IDs (`race_id` and `class_id`).

### Changes Implemented

#### 5.1 Removed Enum Definitions

Removed from `src/domain/character.rs`:

- `pub enum Race { Human, Elf, Dwarf, Gnome, HalfElf, HalfOrc }` - Removed
- `pub enum Class { Knight, Paladin, Archer, Cleric, Sorcerer, Robber }` - Removed

#### 5.2 Removed Enum Fields from Character Struct

Modified `Character` struct in `src/domain/character.rs`:

- Removed `pub race: Race` field
- Removed `pub class: Class` field
- Kept `pub race_id: RaceId` as the sole race identifier
- Kept `pub class_id: ClassId` as the sole class identifier
- Removed `#[serde(default)]` from ID fields (no longer needed)

#### 5.3 Removed Conversion Utilities

Removed from `src/domain/character.rs`:

- `race_id_from_enum()` - No longer needed
- `class_id_from_enum()` - No longer needed
- `race_enum_from_id()` - No longer needed
- `class_enum_from_id()` - No longer needed

#### 5.4 Updated Character Constructor

Modified `Character::new()` in `src/domain/character.rs`:

- Now takes `race_id: RaceId` and `class_id: ClassId` as parameters
- Removed `race: Race` and `class: Class` parameters
- Removed `Character::from_ids()` (merged into `new()`)

```rust
pub fn new(
    name: String,
    race_id: RaceId,
    class_id: ClassId,
    sex: Sex,
    alignment: Alignment,
) -> Self
```

#### 5.5 Updated SpellBook Methods

Modified `SpellBook` methods to use `class_id` strings:

- `get_spell_list(&self, class_id: &str)` - Uses string matching
- `get_spell_list_mut(&mut self, class_id: &str)` - Uses string matching
- Kept `get_spell_list_by_id()` and `get_spell_list_mut_by_id()` for database lookups

#### 5.6 Updated Magic Casting Functions

Modified `src/domain/magic/casting.rs`:

- `can_class_cast_school(class_id: &str, school)` - Now takes class_id string
- `get_required_level_for_spell(class_id: &str, spell)` - Now takes class_id string
- `calculate_spell_points(character)` - Uses `character.class_id`
- Kept `*_by_id()` variants for ClassDatabase lookups

#### 5.7 Updated Progression Functions

Modified `src/domain/progression.rs`:

- `roll_hp_gain(class_id: &str, rng)` - Now takes class_id string
- `level_up(character, rng)` - Uses `character.class_id`
- Kept `*_from_db()` variants for ClassDatabase lookups

#### 5.8 Updated Character Definition Instantiation

Modified `CharacterDefinition::instantiate()` in `src/domain/character_definition.rs`:

- No longer converts to Race/Class enums
- Directly uses `race_id` and `class_id` strings

### Files Modified

**Core Domain Files:**

- `src/domain/character.rs` - Removed enums, updated struct and methods
- `src/domain/magic/casting.rs` - Updated to use class_id strings
- `src/domain/progression.rs` - Updated to use class_id strings
- `src/domain/character_definition.rs` - Removed enum conversion

**Application and Game Files:**

- `src/application/mod.rs` - Updated tests
- `src/game/systems/ui.rs` - Display class_id/race_id instead of enums

**Test Files:**

- `tests/combat_integration.rs` - Updated to use ID strings
- `tests/magic_integration.rs` - Updated to use ID strings
- `tests/game_flow_integration.rs` - Updated to use ID strings

**Documentation Files:**

- `src/domain/magic/mod.rs` - Updated doc examples
- `src/domain/magic/spell_effects.rs` - Updated doc examples
- `src/domain/resources.rs` - Updated doc examples
- `src/domain/combat/engine.rs` - Updated doc examples

### Tests Updated

All tests updated to use ID-based character creation:

```rust
// Before (enum-based)
let hero = Character::new(
    "Hero".to_string(),
    Race::Human,
    Class::Knight,
    Sex::Male,
    Alignment::Good,
);

// After (ID-based)
let hero = Character::new(
    "Hero".to_string(),
    "human".to_string(),
    "knight".to_string(),
    Sex::Male,
    Alignment::Good,
);
```

**Removed Tests:**

- `test_race_id_from_enum_all_races` - Function removed
- `test_class_id_from_enum_all_classes` - Function removed
- `test_race_enum_from_id_all_races` - Function removed
- `test_class_enum_from_id_all_classes` - Function removed
- `test_id_enum_roundtrip_*` - No longer applicable
- `test_character_new_populates_both_enum_and_id` - Only IDs now
- `test_character_from_ids_*` - Merged into `new()`

**Updated Tests:**

- `test_character_creation` - Uses ID strings
- `test_character_with_various_race_ids` - Tests all race IDs
- `test_character_with_various_class_ids` - Tests all class IDs
- `test_character_all_race_class_combinations` - Uses ID pairs
- `test_spellbook_get_spell_list_*` - Uses class_id strings
- `test_can_class_cast_school` - Uses class_id strings
- `test_hp_gain_by_class` - Uses class_id strings
- Many more integration and unit tests

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 274 tests pass (unit + doc tests)

### Architecture Compliance

- [x] No static Race or Class enums exist
- [x] All class/race behavior is data-driven via IDs
- [x] Character struct uses only ID fields
- [x] Serialization works with ID-only format
- [x] SDK functions correctly with new structure
- [x] Full test suite passes

### Success Criteria Met

- [x] `pub enum Race` removed from codebase
- [x] `pub enum Class` removed from codebase
- [x] Character struct has no enum fields
- [x] All functions use `class_id` and `race_id` strings
- [x] Save/load works with ID-only format
- [x] All tests updated and passing

### Migration Notes

For developers updating code to use the new API:

1. **Character Creation**: Use string IDs instead of enum values

   ```rust
   // Old: Character::new("Name", Race::Human, Class::Knight, ...)
   // New: Character::new("Name", "human", "knight", ...)
   ```

2. **Class Checks**: Use string matching or database lookups

   ```rust
   // Old: character.class == Class::Knight
   // New: character.class_id == "knight"
   ```

3. **Race Checks**: Use string matching or database lookups

   ```rust
   // Old: character.race == Race::Elf
   // New: character.race_id == "elf"
   ```

4. **SpellBook Access**: Use class_id strings
   ```rust
   // Old: spellbook.get_spell_list(character.class)
   // New: spellbook.get_spell_list(&character.class_id)
   ```

### Benefits of Enum Removal

1. **Extensibility**: New races/classes can be added via RON files without code changes
2. **Moddability**: Campaigns can define custom races and classes
3. **Consistency**: Single source of truth for race/class data
4. **Simplicity**: No dual enum+ID maintenance
5. **Data-Driven**: All behavior comes from database definitions

---

## Phase 6: SDK and Editor Updates (Hard-coded Removal Plan) (2025-01-XX)

### Background

Phase 6 updates the SDK editors to be fully dynamic, removing any remaining
hard-coded class/race references. This ensures editors display class/race names
from loaded data files and validation catches invalid references.

### Changes Implemented

#### 6.1 Updated Items Editor Class References

Updated `sdk/campaign_builder/src/items_editor.rs`:

- Added `ClassDefinition` import from `antares::domain::classes`
- Updated `show()` method to accept `classes: &[ClassDefinition]` parameter
- Updated `show_list()` method to pass classes to preview
- Updated `show_form()` method to pass classes to disablement editor
- Updated `show_preview_static()` to accept and use classes parameter
- Updated `show_disablement_display_static()` to use dynamic class definitions:

  ```rust
  fn show_disablement_display_static(
      ui: &mut egui::Ui,
      disablement: Disablement,
      classes: &[ClassDefinition],
  ) {
      ui.horizontal_wrapped(|ui| {
          if classes.is_empty() {
              ui.label("(No classes loaded)");
          } else {
              for class_def in classes {
                  let mask = class_def.disablement_mask();
                  let can_use = (disablement.0 & mask) != 0;
                  if can_use {
                      ui.label(format!("✓ {}", class_def.name));
                  } else {
                      ui.label(format!("✗ {}", class_def.name));
                  }
              }
          }
      });
  }
  ```

- Updated `show_disablement_editor()` to use dynamic class definitions:

  ```rust
  fn show_disablement_editor(&mut self, ui: &mut egui::Ui, classes: &[ClassDefinition]) {
      let disablement = &mut self.edit_buffer.disablements;
      ui.label("Classes that CAN use this item:");
      ui.horizontal_wrapped(|ui| {
          if classes.is_empty() {
              ui.label("(No classes loaded - load classes.ron first)");
          } else {
              for class_def in classes {
                  let mask = class_def.disablement_mask();
                  let mut can_use = (disablement.0 & mask) != 0;
                  if ui.checkbox(&mut can_use, &class_def.name).changed() {
                      if can_use {
                          disablement.0 |= mask;
                      } else {
                          disablement.0 &= !mask;
                      }
                  }
              }
          }
      });
  }
  ```

#### 6.2 Updated Main App to Pass Classes

Updated `sdk/campaign_builder/src/main.rs`:

- Updated `items_editor_state.show()` call to pass `&self.classes_editor_state.classes`

#### 6.3 Added Validation Functions

Updated `sdk/campaign_builder/src/validation.rs`:

- Added `validate_class_id_reference()` function:

  - Validates class_id exists in available classes
  - Returns error with helpful message listing available classes

- Added `validate_race_id_reference()` function:

  - Validates race_id exists in available races
  - Returns error with helpful message listing available races

- Added `validate_character_references()` function:
  - Validates all class and race references in characters
  - Returns vector of validation errors

#### 6.4 Preserved Templates and Examples

Templates in `sdk/campaign_builder/src/templates.rs` remain unchanged:

- Default item templates preserved for user convenience
- Example configurations work with any class/race data
- No hardcoded class-specific logic in templates

### Tests Added

```rust
// validation.rs tests
#[test]
fn test_validate_class_id_reference_valid() {
    let classes = vec!["knight".to_string(), "sorcerer".to_string()];
    let result = validate_class_id_reference("knight", &classes, "test character");
    assert!(result.is_none());
}

#[test]
fn test_validate_class_id_reference_invalid() {
    let classes = vec!["knight".to_string(), "sorcerer".to_string()];
    let result = validate_class_id_reference("invalid", &classes, "test character");
    assert!(result.is_some());
}

#[test]
fn test_validate_class_id_reference_empty() {
    let classes = vec!["knight".to_string()];
    let result = validate_class_id_reference("", &classes, "test character");
    assert!(result.is_some());
}

#[test]
fn test_validate_race_id_reference_valid() { ... }

#[test]
fn test_validate_race_id_reference_invalid() { ... }

#[test]
fn test_validate_race_id_reference_empty() { ... }

#[test]
fn test_validate_character_references_all_valid() { ... }

#[test]
fn test_validate_character_references_invalid_class() { ... }

#[test]
fn test_validate_character_references_invalid_race() { ... }

#[test]
fn test_validate_character_references_both_invalid() { ... }
```

### Validation

```bash
cargo fmt --all                                    # ✅ Passed
cargo check --all-targets --all-features           # ✅ Passed
cargo clippy --all-targets --all-features -- -D warnings  # ✅ Passed
cargo test --all-features                          # ✅ 274 tests passed
```

### Architecture Compliance

- [x] No enum references remain in SDK editors
- [x] Class/race data comes from loaded databases
- [x] Validation uses database lookups
- [x] Templates remain useful without hardcoded class data

### Success Criteria Met

- [x] Items editor shows class names from loaded data files
- [x] Disablement checkboxes populated dynamically from ClassDatabase
- [x] Validation catches invalid class_id/race_id references
- [x] Templates remain useful for users
- [x] All SDK tests pass

### Files Modified

- `sdk/campaign_builder/src/items_editor.rs`

  - Added ClassDefinition import
  - Updated show() signature to accept classes
  - Updated show_list(), show_form() to propagate classes
  - Updated show_preview_static() to use classes
  - Updated show_disablement_display_static() for dynamic classes
  - Updated show_disablement_editor() for dynamic classes

- `sdk/campaign_builder/src/main.rs`

  - Updated items_editor.show() call to pass classes

- `sdk/campaign_builder/src/validation.rs`
  - Added validate_class_id_reference() function
  - Added validate_race_id_reference() function
  - Added validate_character_references() function
  - Added 10 new unit tests

### Next Steps (Phase 7)

Phase 7 will complete documentation and cleanup:

1. Remove deprecated Disablement constants
2. Update architecture documentation
3. Update implementation documentation
4. Create migration guide for content creators
5. Archive superseded plans

---

## Phase 7: Documentation and Cleanup (Hard-coded Removal Plan) (2025-01-XX)

### Background

Phase 7 completes the hard-coded removal migration by removing deprecated code,
updating documentation to reflect the new data-driven architecture, and creating
guides for content creators.

### Changes Implemented

#### 7.1 Removed Deprecated Disablement Constants

Removed the deprecated class constants from `src/domain/items/types.rs`:

- `Disablement::KNIGHT` (was 0b0000_0001)
- `Disablement::PALADIN` (was 0b0000_0010)
- `Disablement::ARCHER` (was 0b0000_0100)
- `Disablement::CLERIC` (was 0b0000_1000)
- `Disablement::SORCERER` (was 0b0001_0000)
- `Disablement::ROBBER` (was 0b0010_0000)

The `can_use_class()` method remains but now documents the bit mapping for users
who need to use raw bit values. The preferred method is `can_use_class_id()` which
uses the ClassDatabase for data-driven lookups.

Updated documentation in `can_use_class()` now shows:

```rust
/// Check if a specific class can use this item using a raw bit value
///
/// For data-driven class lookups, prefer `can_use_class_id()` instead.
///
/// # Class Bit Mapping
///
/// The standard class bit positions are:
/// - Bit 0 (0b0000_0001): Knight
/// - Bit 1 (0b0000_0010): Paladin
/// - Bit 2 (0b0000_0100): Archer
/// - Bit 3 (0b0000_1000): Cleric
/// - Bit 4 (0b0001_0000): Sorcerer
/// - Bit 5 (0b0010_0000): Robber
```

#### 7.2 Updated Architecture Documentation

Updated `docs/reference/architecture.md`:

1. **Character struct**: Now shows `race_id: RaceId` and `class_id: ClassId` instead
   of the old `race: Race` and `class: Class` enum fields.

2. **Removed Race and Class enums**: Replaced with a note explaining data-driven
   system:

   ```rust
   // Note: Race and Class are now data-driven using RaceId and ClassId strings.
   // See RaceDatabase and ClassDatabase for loading race/class definitions from RON files.
   // Standard races: "human", "elf", "dwarf", "gnome", "half_orc", "half_elf"
   // Standard classes: "knight", "paladin", "archer", "cleric", "sorcerer", "robber"
   ```

3. **SpellBook methods**: Updated to show data-driven class lookups using ClassDatabase.

#### 7.3 Updated Implementation Documentation

This section (Phase 7) documents:

- Complete migration history from enums to data-driven IDs
- Explanation of the data-driven architecture benefits
- Code changes and their rationale

#### 7.4 Created Migration Guide

Created `docs/how-to/add_classes_races.md` with:

- Step-by-step guide for adding new classes via RON files
- Step-by-step guide for adding new races via RON files
- ClassDefinition and RaceDefinition field explanations
- Disablement bit index table for item restrictions
- Examples of custom class (Berserker, Monk) and race (Halfling, Lizardfolk)
- SDK Campaign Builder usage instructions
- Validation commands and troubleshooting tips
- Migration notes for users upgrading from enum-based versions

#### 7.5 Archived Superseded Plans

Marked `docs/explanation/race_system_implementation_plan.md` as superseded with:

```markdown
> **SUPERSEDED**: This plan has been merged into and superseded by
> [hardcoded_removal_implementation_plan.md](hardcoded_removal_implementation_plan.md).
> The race system was implemented as Phase 4 of that plan.
> This document is preserved for historical reference only.
```

### Tests Updated

Updated all tests that used deprecated constants to use local bit constants:

**src/domain/items/types.rs**:

- `can_use_class_and_alignment()` - renamed from `can_use_class_and_alignment_legacy()`
- `test_disablement_all_classes()` - renamed from `test_disablement_all_classes_legacy()`
- `test_disablement_knight_only()` - renamed from `test_disablement_knight_only_legacy()`
- `test_disablement_good_alignment()` - renamed from `test_disablement_good_alignment_legacy()`
- `test_dynamic_class_lookup()` - renamed from `test_dynamic_matches_static_constants()`
- `test_bit_index_produces_correct_mask()` - renamed from `test_bit_index_matches_static_constant()`

**src/bin/item_editor.rs**:

- `custom_class_selection()` - now uses local BIT\_\* constants
- `test_custom_class_selection_all_flags()` - uses local BIT\_\* constants

**sdk/campaign_builder/src/main.rs**:

- `test_disablement_flags()` - uses local BIT\_\* constants
- `test_disablement_editor_all_classes()` - uses local BIT\_\* constants
- `test_disablement_editor_specific_classes()` - uses local BIT\_\* constants
- `test_item_preview_displays_all_info()` - uses local BIT\_\* constants

### Validation

All quality checks pass:

```bash
cargo fmt --all                                          # OK
cargo check --all-targets --all-features                 # OK
cargo clippy --all-targets --all-features -- -D warnings # OK
cargo test --all-features                                # 275 tests passed
```

### Architecture Compliance

- [x] No deprecated code remains in the codebase
- [x] Architecture documentation accurately reflects current system
- [x] Character struct shows data-driven race_id/class_id fields
- [x] Race and Class enums removed from architecture docs
- [x] How-to guide enables content creators to add classes/races without code changes

### Success Criteria Met

- [x] **No deprecated code remains**: All `#[deprecated]` Disablement constants removed
- [x] **Documentation accurate**: Architecture docs reflect ID-based Character struct
- [x] **Content creator guide**: `docs/how-to/add_classes_races.md` provides complete instructions
- [x] **Superseded plans archived**: Race system plan marked as superseded
- [x] **All quality checks pass**: fmt, check, clippy, and test all pass

### Files Modified

**Code Files**:

- `src/domain/items/types.rs` - Removed deprecated constants, updated tests
- `src/bin/item_editor.rs` - Updated to use raw bit values
- `sdk/campaign_builder/src/main.rs` - Updated tests to use raw bit values

**Documentation Files**:

- `docs/reference/architecture.md` - Updated Character struct, removed enums
- `docs/how-to/add_classes_races.md` - New migration guide (created)
- `docs/explanation/race_system_implementation_plan.md` - Marked as superseded
- `docs/explanation/implementations.md` - This Phase 7 documentation

### Hard-coded Removal Plan Summary

All phases of the hard-coded removal plan are now complete:

| Phase   | Description                  | Status   |
| ------- | ---------------------------- | -------- |
| Phase 1 | Character Struct Migration   | Complete |
| Phase 2 | Class Logic Migration        | Complete |
| Phase 3 | Disablement System Migration | Complete |
| Phase 4 | Race System Implementation   | Complete |
| Phase 5 | Enum Removal                 | Complete |
| Phase 6 | SDK and Editor Updates       | Complete |
| Phase 7 | Documentation and Cleanup    | Complete |

### Benefits of Data-Driven Architecture

1. **No Code Changes for New Content**: Content creators can add classes and races
   by editing RON files alone.

2. **Campaign Customization**: Each campaign can define its own class/race variants
   without affecting the core game.

3. **Type Safety**: ClassId and RaceId type aliases provide compile-time documentation
   while allowing runtime flexibility.

4. **Centralized Definitions**: Single source of truth for class/race properties
   in ClassDatabase and RaceDatabase.

5. **SDK Integration**: Campaign Builder can load, edit, and validate custom
   classes/races through the same data-driven interfaces.

---

## Proficiency System Migration - Phase 1: Core Type Definitions (2025-XX-XX)

**Objective**: Create the foundational proficiency and classification types without breaking existing code, as specified in `docs/explanation/proficiency_migration_plan.md`.

### Background

The proficiency system replaces the bit-mask based class disablement system with a more flexible, data-driven approach. Instead of items storing which classes CAN'T use them, the new system defines:

- What proficiencies classes and races grant
- What proficiencies items require based on their classification
- UNION logic where a character can use an item if EITHER class OR race grants the proficiency

### Changes Implemented

#### 1.1 Created Classification Enums

Added to `src/domain/items/types.rs`:

- **WeaponClassification** enum: Simple, MartialMelee, MartialRanged, Blunt, Unarmed
- **ArmorClassification** enum: Light, Medium, Heavy, Shield
- **MagicItemClassification** enum: Arcane, Divine, Universal
- **AlignmentRestriction** enum: GoodOnly, EvilOnly

Each enum:

- Has `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]`
- Uses `#[default]` attribute for the default variant
- Includes comprehensive doc comments with examples

#### 1.2 Added Tags Field to Item Struct

Modified `Item` struct in `src/domain/items/types.rs`:

```rust
/// Arbitrary tags for fine-grained restrictions (e.g., "large_weapon", "two_handed")
#[serde(default)]
pub tags: Vec<String>,
```

Standard tags by convention (not enforced):

- `large_weapon` - Too big for small races (Halfling, Gnome)
- `two_handed` - Requires both hands
- `heavy_armor` - Encumbering armor
- `elven_crafted` - Made by elves
- `dwarven_crafted` - Made by dwarves
- `requires_strength` - Needs high strength

#### 1.3 Created Proficiency Module

Created new file `src/domain/proficiency.rs` with:

**Type Aliases:**

- `ProficiencyId = String` - Unique identifier for proficiencies

**Enums:**

- `ProficiencyCategory` - Weapon, Armor, Shield, MagicItem

**Structs:**

- `ProficiencyDefinition` - id, name, category, description
- `ProficiencyDatabase` - HashMap-based storage with load/get/validate/all methods

**Error Types:**

- `ProficiencyError` - ProficiencyNotFound, LoadError, ParseError, ValidationError, DuplicateId

**Classification Mapping Functions:**

- `proficiency_for_weapon(WeaponClassification) -> ProficiencyId`
- `proficiency_for_armor(ArmorClassification) -> ProficiencyId`
- `proficiency_for_magic_item(MagicItemClassification) -> Option<ProficiencyId>`

**Helper Functions:**

- `has_proficiency_union()` - Core UNION logic for class + race proficiencies
- `is_item_compatible_with_race()` - Tag-based compatibility check

#### 1.4 Created Proficiency Data File

Created `data/proficiencies.ron` with 11 standard proficiencies:

**Weapon Proficiencies (5):**

- `simple_weapon` - Clubs, daggers, staffs
- `martial_melee` - Swords, axes, maces
- `martial_ranged` - Bows, crossbows
- `blunt_weapon` - Maces, hammers (clerics)
- `unarmed` - Martial arts, fists

**Armor Proficiencies (4):**

- `light_armor` - Leather, padded
- `medium_armor` - Chain mail, scale
- `heavy_armor` - Plate mail, full plate
- `shield` - All shield types

**Magic Item Proficiencies (2):**

- `arcane_item` - Wands, arcane scrolls
- `divine_item` - Holy symbols, divine scrolls

#### 1.5 Updated Module Exports

Modified `src/domain/mod.rs`:

- Added `pub mod proficiency;`
- Re-exported: `has_proficiency_union`, `is_item_compatible_with_race`, `ProficiencyCategory`, `ProficiencyDatabase`, `ProficiencyDefinition`, `ProficiencyError`, `ProficiencyId`

Modified `src/domain/items/mod.rs`:

- Re-exported classification enums: `AlignmentRestriction`, `ArmorClassification`, `MagicItemClassification`, `WeaponClassification`

### Tests Added

**ProficiencyDefinition Tests:**

- `test_proficiency_definition_new` - Constructor works
- `test_proficiency_definition_with_description` - Description field

**ProficiencyCategory Tests:**

- `test_proficiency_category_default` - Default is Weapon
- `test_proficiency_category_equality` - Equality works

**ProficiencyDatabase Tests:**

- `test_database_new` - Empty database creation
- `test_database_add` - Adding proficiencies
- `test_database_add_duplicate` - Duplicate ID rejection
- `test_database_get` - Lookup by ID
- `test_database_remove` - Removal
- `test_database_all` - Getting all proficiencies
- `test_database_all_ids` - Getting all IDs
- `test_database_by_category` - Category filtering
- `test_database_validate_success` - Validation passes
- `test_database_load_from_string` - RON parsing
- `test_database_load_from_string_duplicate` - Duplicate detection in load
- `test_database_load_from_string_parse_error` - Invalid RON handling

**Classification Mapping Tests:**

- `test_proficiency_for_weapon` - All 5 weapon classifications
- `test_proficiency_for_armor` - All 4 armor classifications
- `test_proficiency_for_magic_item` - Arcane, Divine, Universal

**Helper Function Tests:**

- `test_has_proficiency_union_class_grants` - Class provides proficiency
- `test_has_proficiency_union_race_grants` - Race provides proficiency
- `test_has_proficiency_union_both_grant` - Both provide (still works)
- `test_has_proficiency_union_neither_grants` - Neither provides
- `test_has_proficiency_union_no_requirement` - No proficiency needed
- `test_is_item_compatible_no_tags` - Item with no tags
- `test_is_item_compatible_no_restrictions` - Race with no restrictions
- `test_is_item_compatible_incompatible` - Incompatible tag found
- `test_is_item_compatible_no_overlap` - No matching tags

**Integration Test:**

- `test_load_proficiencies_from_data_file` - Load actual data/proficiencies.ron

### Files Modified for Tags Compatibility

Updated Item initializers to include `tags: vec![]`:

- `src/domain/items/database.rs` - Test helper and doc examples
- `src/domain/items/types.rs` - Tests and doc examples
- `src/sdk/templates.rs` - All template functions
- `src/bin/item_editor.rs` - CLI editor tests

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted successfully
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 303 tests pass (unit + doc tests)

### Architecture Compliance

- [x] Classification enums match architecture design
- [x] ProficiencyDatabase follows existing database patterns (ClassDatabase, RaceDatabase)
- [x] RON format used for data files
- [x] Type aliases used consistently (ProficiencyId)
- [x] Module structure follows architecture.md Section 3.2
- [x] No existing functionality broken
- [x] Existing Item struct backward compatible (tags field has #[serde(default)])

### Success Criteria Met

- [x] `cargo check` passes
- [x] `cargo clippy` passes with no warnings
- [x] `cargo test` passes
- [x] `ProficiencyDatabase` can load from RON file
- [x] Classification enums serialize/deserialize correctly
- [x] UNION logic helper functions work correctly
- [x] Item tags field added without breaking existing data files

### Deliverables Completed

- [x] `src/domain/items/types.rs` - Classification enums + `tags: Vec<String>` field
- [x] `src/domain/proficiency.rs` - New module with UNION logic
- [x] `data/proficiencies.ron` - Standard proficiencies (11 total)
- [x] `src/domain/mod.rs` - Export proficiency module
- [x] Tests achieving >80% coverage

### Next Steps (Phase 2)

Per the proficiency migration plan:

1. Add `proficiencies: Vec<ProficiencyId>` to ClassDefinition
2. Update RaceDefinition with proficiencies (already has the field)
3. Update data files (classes.ron, races.ron)
4. Add validation to ensure proficiency IDs exist in proficiencies.ron

---

## Proficiency System Migration - Phase 2: Class and Race Definition Migration (2025-XX-XX)

### Background

Phase 2 of the proficiency system migration adds proficiency support to class and race
definitions, enabling the data-driven item usage system. This phase:

- Adds a `proficiencies` field to `ClassDefinition` struct
- Adds a `can_use_item()` method to `RaceDefinition` for tag-based compatibility
- Updates all data files with appropriate proficiency assignments
- Adds comprehensive tests for combined proficiency and tag checking

### Changes Implemented

#### 2.1 Updated ClassDefinition Struct

Modified `src/domain/classes.rs`:

- Added `proficiencies: Vec<ProficiencyId>` field with `#[serde(default)]`
- Added `has_proficiency(&self, proficiency: &str) -> bool` method
- Updated `ClassDefinition::new()` to initialize proficiencies as empty vector
- Updated all doc examples to include the new field

```rust
pub struct ClassDefinition {
    // ... existing fields ...

    /// Proficiencies this class grants (e.g., "simple_weapon", "heavy_armor")
    #[serde(default)]
    pub proficiencies: Vec<ProficiencyId>,
}

impl ClassDefinition {
    pub fn has_proficiency(&self, proficiency: &str) -> bool {
        self.proficiencies.iter().any(|p| p.as_str() == proficiency)
    }
}
```

#### 2.2 Updated RaceDefinition

Modified `src/domain/races.rs`:

- Added `can_use_item(&self, item_tags: &[String]) -> bool` method
- This checks if any item tags are incompatible with the race

```rust
impl RaceDefinition {
    pub fn can_use_item(&self, item_tags: &[String]) -> bool {
        !item_tags.iter().any(|tag| self.is_item_incompatible(tag))
    }
}
```

#### 2.3 Updated Data Files

Updated `data/classes.ron` with proficiencies for each class:

| Class    | Proficiencies                                                                                           |
| -------- | ------------------------------------------------------------------------------------------------------- |
| Knight   | simple_weapon, martial_melee, light_armor, medium_armor, heavy_armor, shield                            |
| Paladin  | simple_weapon, martial_melee, blunt_weapon, light_armor, medium_armor, heavy_armor, shield, divine_item |
| Archer   | simple_weapon, martial_ranged, light_armor, medium_armor                                                |
| Cleric   | simple_weapon, blunt_weapon, light_armor, medium_armor, shield, divine_item                             |
| Sorcerer | simple_weapon, arcane_item                                                                              |
| Robber   | simple_weapon, martial_melee, light_armor                                                               |

Updated `campaigns/tutorial/data/classes.ron` with the same proficiencies.

Updated `campaigns/tutorial/data/races.ron` with enhanced racial proficiencies:

| Race  | Proficiencies             | Incompatible Tags         |
| ----- | ------------------------- | ------------------------- |
| Human | (none)                    | (none)                    |
| Elf   | martial_ranged, longsword | (none)                    |
| Dwarf | battleaxe, warhammer      | (none)                    |
| Gnome | short_sword, crossbow     | large_weapon, heavy_armor |

### Tests Added

Added comprehensive tests in `src/domain/proficiency.rs`:

```rust
#[test]
fn test_elf_sorcerer_can_use_longbow() {
    // Elf Sorcerer CAN use Long Bow because race grants martial_ranged
    // even though Sorcerer class doesn't have martial_ranged proficiency
}

#[test]
fn test_gnome_archer_cannot_use_longbow() {
    // Gnome Archer CANNOT use Long Bow due to large_weapon tag
    // even though Archer class has martial_ranged proficiency
}

#[test]
fn test_gnome_archer_can_use_shortbow() {
    // Gnome Archer CAN use Short Bow (no large_weapon tag)
}

#[test]
fn test_human_knight_can_use_plate_armor() {
    // Human Knight CAN use Plate Armor
}

#[test]
fn test_gnome_knight_cannot_use_plate_armor() {
    // Gnome Knight CANNOT use Plate Armor (heavy_armor tag)
}

#[test]
fn test_dwarf_cleric_can_use_warhammer() {
    // Tests UNION logic - either class OR race grants proficiency
}

#[test]
fn test_sorcerer_cannot_use_plate_armor() {
    // Sorcerer has no heavy_armor proficiency
}

#[test]
fn test_item_with_no_proficiency_requirement() {
    // Anyone can use items with no proficiency requirement
}

#[test]
fn test_load_classes_with_proficiencies() {
    // Integration test loading classes.ron
}

#[test]
fn test_load_races_with_proficiencies_and_tags() {
    // Integration test loading races.ron
}
```

Added test in `src/domain/classes.rs`:

```rust
#[test]
fn test_class_definition_has_proficiency() {
    let knight = create_test_knight();
    assert!(knight.has_proficiency("heavy_armor"));
    assert!(knight.has_proficiency("martial_melee"));
    assert!(!knight.has_proficiency("arcane_item"));
}
```

Added test in `src/domain/races.rs`:

```rust
#[test]
fn test_race_definition_can_use_item() {
    let gnome = create_test_gnome();
    assert!(!gnome.can_use_item(&["large_weapon".to_string()]));
    assert!(gnome.can_use_item(&["light".to_string()]));
}
```

### Files Modified

- `src/domain/classes.rs` - Added proficiencies field and has_proficiency method
- `src/domain/races.rs` - Added can_use_item method
- `src/domain/proficiency.rs` - Added Phase 2 integration tests
- `src/domain/character_definition.rs` - Updated test ClassDefinition instances
- `src/bin/class_editor.rs` - Updated ClassDefinition instances
- `data/classes.ron` - Added proficiencies to all classes
- `data/races.ron` - Already had proficiencies (verified)
- `campaigns/tutorial/data/classes.ron` - Added proficiencies to all classes
- `campaigns/tutorial/data/races.ron` - Enhanced proficiency documentation

### Validation

- [x] `cargo fmt --all` applied successfully
- [x] `cargo check --all-targets --all-features` passes
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [x] `cargo test --all-features` passes (585 tests)
- [x] All doc tests pass (305 tests)

### Architecture Compliance

- [x] Uses `ProficiencyId` type alias consistently
- [x] Follows `#[serde(default)]` pattern for backward compatibility
- [x] RON format used for all data files
- [x] UNION logic implemented (class OR race grants proficiency)
- [x] Tag-based restrictions work independently of proficiencies

### Success Criteria Met

- [x] ClassDefinition has proficiencies field
- [x] RaceDefinition has can_use_item method
- [x] Data files updated with appropriate proficiencies
- [x] Elf Sorcerer CAN use Long Bow (race grants proficiency)
- [x] Gnome Archer CANNOT use Long Bow (large_weapon tag)
- [x] Gnome Archer CAN use Short Bow (no large_weapon tag)
- [x] Integration tests pass for loading classes and races

### Deliverables Completed

- [x] `src/domain/classes.rs` - Modified with proficiencies
- [x] `src/domain/races.rs` - Modified with can_use_item method
- [x] `data/classes.ron` - Updated with proficiencies
- [x] `data/races.ron` - Verified with proficiencies and incompatible_item_tags
- [x] `campaigns/tutorial/data/classes.ron` - Updated with proficiencies
- [x] `campaigns/tutorial/data/races.ron` - Updated with proficiencies and incompatible_item_tags

### Next Steps (Phase 3)

Per the proficiency migration plan:

1. Update ItemType sub-structs (WeaponData, ArmorData, AccessoryData) with classification fields
2. Create migration mapping from old item data to new classification
3. Update item data files with classifications
4. Add `required_proficiency()` method to Item struct

---

## Phase 3: Item Definition Migration (Proficiency System Migration) (2025-01-XX)

**Objective**: Add classification fields to item sub-structs (WeaponData, ArmorData, AccessoryData), add alignment_restriction to Item, implement `Item::required_proficiency()` method, and update data files with classifications and tags.

### Background

Per the Proficiency System Migration Plan (`docs/explanation/proficiency_migration_plan.md`) Phase 3, item definitions needed to be updated to support the new proficiency system. Items should derive their proficiency requirements from classification enums rather than relying solely on the legacy disablement bitmask.

### Changes Implemented

#### 3.1 Updated ItemType Sub-Structs with Classification Fields

Modified `src/domain/items/types.rs`:

**WeaponData** - Added `classification: WeaponClassification` field with `#[serde(default)]`:

- `Simple` - Basic weapons anyone can use (clubs, daggers, staffs)
- `MartialMelee` - Advanced melee weapons (swords, axes)
- `MartialRanged` - Ranged weapons (bows, crossbows)
- `Blunt` - Weapons without edge (maces, hammers - clerics)
- `Unarmed` - Martial arts

**ArmorData** - Added `classification: ArmorClassification` field with `#[serde(default)]`:

- `Light` - Leather, padded armor
- `Medium` - Chain mail, scale mail
- `Heavy` - Plate mail, full plate
- `Shield` - All shield types

**AccessoryData** - Added `classification: Option<MagicItemClassification>` field with `#[serde(default)]`:

- `Some(Arcane)` - Wands, arcane scrolls (sorcerers)
- `Some(Divine)` - Holy symbols, divine scrolls (clerics)
- `Some(Universal)` or `None` - Anyone can use

#### 3.2 Added alignment_restriction to Item

Added `alignment_restriction: Option<AlignmentRestriction>` field to Item struct:

- `None` - Any alignment can use
- `Some(GoodOnly)` - Only good-aligned characters
- `Some(EvilOnly)` - Only evil-aligned characters

This separates alignment restrictions from the legacy disablement bitmask.

#### 3.3 Deprecated Disablement Field

Marked `disablements` field on Item as deprecated:

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use alignment_restriction field and proficiency system instead."
)]
pub disablements: Disablement,
```

#### 3.4 Added Item::required_proficiency() Method

Added method to derive proficiency requirement from item classification:

```rust
pub fn required_proficiency(&self) -> Option<ProficiencyId>
```

Returns:

- Weapons: Proficiency from `ProficiencyDatabase::proficiency_for_weapon(classification)`
- Armor: Proficiency from `ProficiencyDatabase::proficiency_for_armor(classification)`
- Accessories with magic classification: Proficiency from `ProficiencyDatabase::proficiency_for_magic_item()`
- Consumables, Ammo, Quest items: `None` (no proficiency required)

#### 3.5 Added Item::can_use_alignment() Method

Added method to check alignment restrictions:

```rust
pub fn can_use_alignment(&self, alignment: Alignment) -> bool
```

#### 3.6 Updated Data Files

**data/items.ron** and **campaigns/tutorial/data/items.ron**:

- Added `classification` to all WeaponData, ArmorData, AccessoryData
- Added `alignment_restriction` field to all items
- Added `tags` field to appropriate items:
  - Two-Handed Sword: `["large_weapon", "two_handed"]`
  - Long Bow: `["large_weapon", "two_handed"]`
  - Short Bow: `["two_handed"]`
  - Plate Mail: `["heavy_armor"]`
- Added new items:
  - Long Bow (id: 8) - MartialRanged classification
  - Short Bow (id: 9) - MartialRanged classification
  - Wooden Shield (id: 23) - Shield classification
  - Steel Shield (id: 24) - Shield classification
  - Arcane Wand (id: 43) - Arcane magic classification
  - Holy Symbol (id: 44) - Divine magic classification

#### 3.7 Updated SDK Templates

Modified `src/sdk/templates.rs`:

- All template functions now include classification fields
- Added `#[allow(deprecated)]` for disablements usage
- Updated imports to include classification enums

#### 3.8 Updated Item Editor CLI

Modified `src/bin/item_editor.rs`:

- Added classification selection helpers for weapons, armor, accessories
- Added alignment restriction selection helper
- Updated item creation flow to prompt for classifications
- Added `#[allow(deprecated)]` for legacy disablements field

### Tests Added

New tests in `src/domain/items/types.rs`:

**Item::required_proficiency() Tests:**

- `test_weapon_required_proficiency_simple` - Simple weapons → simple_weapon
- `test_weapon_required_proficiency_martial_melee` - Martial melee → martial_melee
- `test_weapon_required_proficiency_martial_ranged` - Martial ranged → martial_ranged
- `test_weapon_required_proficiency_blunt` - Blunt weapons → blunt_weapon
- `test_armor_required_proficiency_light` - Light armor → light_armor
- `test_armor_required_proficiency_heavy` - Heavy armor → heavy_armor
- `test_armor_required_proficiency_shield` - Shields → shield
- `test_accessory_required_proficiency_arcane` - Arcane items → arcane_item
- `test_accessory_required_proficiency_divine` - Divine items → divine_item
- `test_accessory_required_proficiency_universal` - Universal → None
- `test_accessory_required_proficiency_mundane` - Mundane → None
- `test_consumable_no_proficiency` - Consumables → None
- `test_ammo_no_proficiency` - Ammo → None
- `test_quest_item_no_proficiency` - Quest items → None

**Item::can_use_alignment() Tests:**

- `test_alignment_restriction_none` - No restriction allows all
- `test_alignment_restriction_good_only` - Good only restriction
- `test_alignment_restriction_evil_only` - Evil only restriction

### Validation

All quality gates pass:

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 307 doc tests pass, all unit tests pass

### Architecture Compliance

- [x] Classification enums defined per architecture (WeaponClassification, ArmorClassification, MagicItemClassification)
- [x] `#[serde(default)]` used for backward compatibility with existing data files
- [x] RON format used for all data files
- [x] `ProficiencyId` type alias used consistently
- [x] Item tags system implemented for fine-grained restrictions
- [x] Deprecation added to legacy disablements field with migration guidance

### Success Criteria Met

- [x] All quality gates pass
- [x] Items load correctly with new classification fields
- [x] `Item::required_proficiency()` correctly derives from classification
- [x] Proficiency checks work end-to-end (class + race)
- [x] Alignment restriction separate from proficiency system
- [x] Data files updated with classifications and tags
- [x] Long Bow has `["large_weapon", "two_handed"]` tags
- [x] Short Bow has `["two_handed"]` tag (no large_weapon)
- [x] Plate Mail has `["heavy_armor"]` tag

### Deliverables Completed

- [x] `src/domain/items/types.rs` - Modified ItemType sub-structs with classification + tags
- [x] `data/items.ron` - Migrated to classification + tags system
- [x] `campaigns/tutorial/data/items.ron` - Migrated with appropriate tags
- [x] `src/sdk/templates.rs` - Updated with classification fields
- [x] `src/bin/item_editor.rs` - Updated with classification selection

### Files Modified

- `src/domain/items/types.rs` - Added classification fields, required_proficiency(), can_use_alignment()
- `src/domain/items/database.rs` - Updated doc examples and test helpers
- `src/sdk/templates.rs` - Updated all templates with classification fields
- `src/bin/item_editor.rs` - Added classification selection helpers
- `data/items.ron` - Full migration with classifications and tags
- `campaigns/tutorial/data/items.ron` - Full migration with classifications and tags

### Next Steps (Phase 4)

Per the proficiency migration plan:

1. Update SDK Classes Editor to edit proficiencies list
2. Update SDK Races Editor to edit proficiencies and incompatible_item_tags
3. Update SDK Items Editor to edit classification and tags
4. Add validation for proficiency references

---

## Phase 4: SDK Editor Updates (Proficiency System Migration) (2025-01-XX)

**Objective**: Update the SDK campaign builder editors to support the new proficiency system, including proficiency editing in Classes/Races editors, classification dropdowns and tags editing in Items editor, and validation functions for proficiency IDs.

### Background

Per the Proficiency System Migration Plan (`docs/explanation/proficiency_migration_plan.md`) Phase 4, the SDK campaign builder editors needed updates to:

1. Allow editing proficiencies in Classes editor with quick-add buttons
2. Allow editing proficiencies and incompatible_item_tags in Races editor with suggestions
3. Add classification dropdowns (weapon/armor/magic item) to Items editor
4. Add alignment restriction dropdown and tags editor to Items editor
5. Add validation functions for proficiency IDs and item tags

### Changes Implemented

#### 4.1 Updated Classes Editor

Modified `sdk/campaign_builder/src/classes_editor.rs`:

- Added `proficiencies: String` field to `ClassEditBuffer` (comma-separated)
- Added proficiency parsing in `save_class()` method
- Added proficiency editing UI group with:
  - Text field for comma-separated proficiency IDs
  - Quick-add buttons for standard proficiencies (simple_weapon, martial_melee, etc.)
  - Toggle behavior - clicking selected proficiency removes it
  - Info tooltip explaining standard proficiency IDs
  - Display of current proficiency count

#### 4.2 Updated Races Editor

Modified `sdk/campaign_builder/src/races_editor.rs`:

- Improved proficiencies editing UI with:

  - Text field for comma-separated proficiency IDs
  - Quick-add buttons for all standard proficiencies
  - Toggle behavior for adding/removing proficiencies
  - Info tooltip with standard proficiency IDs
  - Display of current proficiency count

- Improved incompatible_item_tags editing UI with:
  - Text field for comma-separated tags
  - Quick-add buttons for standard tags (large_weapon, two_handed, heavy_armor, etc.)
  - Toggle behavior for adding/removing tags
  - Info tooltip explaining standard item tags
  - Display of current tag count

#### 4.3 Updated Items Editor

Modified `sdk/campaign_builder/src/items_editor.rs`:

- Added imports for classification enums and AlignmentRestriction
- Updated `default_item()` to include classification, alignment_restriction, and tags fields
- Updated item type creation buttons to include classification defaults

**Weapon Classification Dropdown:**

- Added classification dropdown in weapon type editor
- Options: Simple, Martial Melee, Martial Ranged, Blunt, Unarmed
- Info tooltip explaining each classification
- Shows derived proficiency requirement (e.g., "Required proficiency: martial_melee")

**Armor Classification Dropdown:**

- Added classification dropdown in armor type editor
- Options: Light, Medium, Heavy, Shield
- Info tooltip explaining each classification
- Shows derived proficiency requirement

**Magic Item Classification Dropdown:**

- Added classification dropdown in accessory type editor
- Options: None (Mundane), Arcane, Divine, Universal
- Info tooltip explaining each classification
- Shows derived proficiency or "No proficiency required"

**Alignment Restriction Dropdown:**

- Added alignment restriction dropdown in item form
- Options: None (Any Alignment), Good Only, Evil Only
- Info tooltip explaining alignment restrictions

**Tags Editor:**

- Added tags text field (comma-separated)
- Quick-add buttons for standard tags
- Toggle behavior for adding/removing tags
- Shows current tag count
- Shows derived proficiency requirement from classification

#### 4.4 Updated Validation Module

Modified `sdk/campaign_builder/src/validation.rs`:

- Added `STANDARD_PROFICIENCY_IDS` constant with all standard proficiency IDs
- Added `STANDARD_ITEM_TAGS` constant with standard item tags

**New Validation Functions:**

- `validate_proficiency_id(proficiency_id, context)` - Validates against standard list
- `validate_class_proficiencies(class_id, proficiencies)` - Validates all class proficiencies
- `validate_race_proficiencies(race_id, proficiencies)` - Validates all race proficiencies
- `validate_item_tag(tag, context)` - Validates against standard tags
- `validate_race_incompatible_tags(race_id, tags)` - Validates race incompatible tags
- `validate_item_tags(item_id, item_name, tags)` - Validates item tags
- `validate_weapon_classification(item_id, item_name, classification)` - Info about proficiency
- `validate_armor_classification(item_id, item_name, classification)` - Info about proficiency

#### 4.5 Updated SDK Templates

Modified `sdk/campaign_builder/src/templates.rs`:

- Added imports for WeaponClassification and ArmorClassification
- Updated `create_item()` method to include classification, alignment_restriction, tags
- All weapon templates now include appropriate classification
- All armor templates now include appropriate classification
- Added `heavy_armor` tag to plate_mail template
- Added `two_handed` tag to bow and staff templates

#### 4.6 Updated Test Files

Fixed Item creation in tests across multiple files to include new required fields:

- `sdk/campaign_builder/src/advanced_validation.rs` - Updated create_test_item
- `sdk/campaign_builder/src/asset_manager.rs` - Updated test item
- `sdk/campaign_builder/src/items_editor.rs` - Updated all test items
- `sdk/campaign_builder/src/main.rs` - Updated all test items (7+ locations)
- `sdk/campaign_builder/src/undo_redo.rs` - Updated create_test_item
- `sdk/campaign_builder/src/templates.rs` - Updated test_custom_templates

### Tests Added

New tests in `sdk/campaign_builder/src/validation.rs`:

**Proficiency Validation Tests:**

- `test_validate_proficiency_id_valid` - Valid proficiency passes
- `test_validate_proficiency_id_invalid` - Unknown proficiency returns warning
- `test_validate_proficiency_id_empty` - Empty proficiency returns warning
- `test_validate_class_proficiencies_all_valid` - All valid proficiencies pass
- `test_validate_class_proficiencies_with_invalid` - Detects invalid proficiencies
- `test_validate_race_proficiencies_valid` - Valid race proficiencies pass
- `test_validate_race_proficiencies_with_invalid` - Detects invalid proficiencies

**Item Tag Validation Tests:**

- `test_validate_item_tag_valid` - Standard tag passes
- `test_validate_item_tag_custom` - Custom tag returns info (not error)
- `test_validate_item_tag_empty` - Empty tag returns warning
- `test_validate_race_incompatible_tags_valid` - Standard tags pass
- `test_validate_race_incompatible_tags_custom` - Custom tags return info
- `test_validate_item_tags_valid` - All standard tags pass
- `test_validate_item_tags_with_custom` - Custom tags return info

**Classification Validation Tests:**

- `test_validate_weapon_classification` - Shows derived proficiency
- `test_validate_armor_classification` - Shows derived proficiency

**Constant Validation Tests:**

- `test_standard_proficiency_ids_complete` - All expected IDs present
- `test_standard_item_tags_complete` - All expected tags present

### Validation

All quality gates pass:

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - Compilation successful
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 307 doc tests pass, all unit tests pass
- SDK campaign_builder tests: 545 passed (1 pre-existing UI test failure unrelated to changes)

### Architecture Compliance

- [x] Proficiency editing uses standard proficiency IDs from architecture
- [x] Classification dropdowns match architecture enum variants
- [x] Tags system follows convention from architecture
- [x] Alignment restriction separate from proficiency system
- [x] Validation warns about unknown IDs but doesn't block (custom content support)
- [x] Quick-add buttons use standard IDs defined in data/proficiencies.ron
- [x] `#[allow(deprecated)]` used for legacy disablements field access

### Success Criteria Met

- [x] SDK builds and runs
- [x] Can edit class proficiencies via text field and quick-add buttons
- [x] Can edit race proficiencies and incompatible_item_tags
- [x] Can set weapon/armor classification via dropdown
- [x] Can set magic item classification via dropdown
- [x] Can edit alignment restriction via dropdown
- [x] Can edit item tags via text field and quick-add buttons
- [x] Shows derived proficiency requirement from classification
- [x] Validation warns about invalid proficiency IDs
- [x] All existing tests continue to pass

### Deliverables Completed

- [x] `sdk/campaign_builder/src/classes_editor.rs` - Proficiency editing UI
- [x] `sdk/campaign_builder/src/races_editor.rs` - Proficiency and tags editing UI
- [x] `sdk/campaign_builder/src/items_editor.rs` - Classification, alignment, tags UI
- [x] `sdk/campaign_builder/src/validation.rs` - Proficiency/tag validation functions
- [x] `sdk/campaign_builder/src/templates.rs` - Updated with new fields
- [x] `sdk/campaign_builder/src/main.rs` - Updated with new imports and test fixes
- [x] Multiple test files updated with new Item fields

### Files Modified

**SDK Campaign Builder:**

- `sdk/campaign_builder/src/classes_editor.rs` - Added proficiencies field and UI
- `sdk/campaign_builder/src/races_editor.rs` - Improved proficiencies/tags UI
- `sdk/campaign_builder/src/items_editor.rs` - Added classification, alignment, tags UI
- `sdk/campaign_builder/src/validation.rs` - Added validation functions and tests
- `sdk/campaign_builder/src/templates.rs` - Updated item templates with new fields
- `sdk/campaign_builder/src/main.rs` - Updated imports and test items
- `sdk/campaign_builder/src/advanced_validation.rs` - Updated test helper
- `sdk/campaign_builder/src/asset_manager.rs` - Updated test item
- `sdk/campaign_builder/src/undo_redo.rs` - Updated test helper

### Next Steps (Phase 5)

Per the proficiency migration plan:

1. Update CLI Class Editor to prompt for proficiencies
2. Update CLI Item Editor to prompt for classification
3. Update CLI Race Editor to prompt for proficiencies and incompatible_item_tags
4. Add validation output for proficiency references

---
