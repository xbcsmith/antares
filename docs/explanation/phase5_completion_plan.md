# Phase 5 Completion Plan: CLI Editor Proficiency Migration

## Overview

This plan addresses the remaining deliverables from Phase 5 (CLI Editor Updates) of the Proficiency System Migration. While the core proficiency input functionality was successfully implemented, several cleanup tasks, edit flows, and testing requirements remain incomplete.

**Scope**: Complete Phase 5 by addressing deprecated code removal, implementing full edit flows, adding automated tests, and creating validation documentation.

## Current State Analysis

### Existing Infrastructure

**Completed in Phase 5 (Initial Implementation)**:
- `src/bin/class_editor.rs` - Proficiency input added to add flow
- `src/bin/race_editor.rs` - Proficiency and incompatible tags input added
- `src/bin/item_editor.rs` - Tags, classification, and alignment added to add flow
- Preview functions enhanced to show new fields
- Validation against standard proficiency/tag lists
- Implementation documentation created

**What Works**:
- All three CLI editors (`class_editor.rs`, `item_editor.rs`, `race_editor.rs`) compile and run
- Add flows support all new proficiency fields (classifications, tags, alignment restrictions)
- Preview functions display new fields correctly
- Input validation works for standard proficiency IDs and item tags
- RON serialization/deserialization handles new and legacy fields via `#[serde(default)]`

### Identified Issues

**Missing from Phase 5 Requirements**:

1. **Deprecated Code Not Removed**: The `get_next_disablement_bit()` function in `class_editor.rs` (L555-575) still exists and is called in `add_class()` (L203). This was explicitly required for removal in Phase 5.1 of the migration plan.

2. **Incomplete Edit Flows**: The `edit_item()` function in `item_editor.rs` (L665-695) cannot edit classifications, tags, or alignment restrictions. Users must delete and recreate items to change these fields.

3. **Missing Test Coverage**: CLI editors have no unit tests for new proficiency-related functionality. Existing tests (L705-760 in `class_editor.rs`) only test deprecated `get_next_disablement_bit()` function.

4. **No Integration Tests**: The migration plan (section 5.4) explicitly requires "round-trip tests (create → save → load → verify)" and "classification → proficiency derivation display tests" - neither exists.

5. **Documentation Gap**: No manual test checklist documented in deliverables (section 5.5 requirement).

## Implementation Phases

### Phase 5A: Deprecated Code Removal

Remove all legacy disablement bit infrastructure from class editor while preserving backward compatibility for data loading.

#### 5A.1 Remove Deprecated Function Calls

**File:** `src/bin/class_editor.rs`

- Remove `get_next_disablement_bit()` call from `add_class()` function (L203)
- Remove assignment to `disablement_bit_index` field in `ClassDefinition` construction (L215)
- The field itself must remain in the struct for backward compatibility with existing RON files

**Expected Changes:**
```
add_class() function:
  - Remove: let disablement_bit = self.get_next_disablement_bit();
  - Update ClassDefinition construction to use default value: disablement_bit_index: 0
```

#### 5A.2 Remove Deprecated Function Implementation

**File:** `src/bin/class_editor.rs`

- Delete `get_next_disablement_bit()` function (L555-575)
- Remove disablement bit display from `preview_class()` function (L405-409)

**Expected Changes:**
```
Remove entire function:
  - fn get_next_disablement_bit(&self) -> u8 { ... }

Update preview_class():
  - Remove: println!("  Disablement Bit Index: {} (mask: 0b{:08b})", ...)
  - Or mark as "(legacy, unused)" if keeping for reference
```

#### 5A.3 Remove Deprecated Tests

**File:** `src/bin/class_editor.rs`

- Delete `test_get_next_disablement_bit_empty()` (L705-715)
- Delete `test_get_next_disablement_bit_sequential()` (L716-760)
- Keep `test_truncate()` as it tests active functionality

**Expected Changes:**
```
Remove test functions:
  - fn test_get_next_disablement_bit_empty() { ... }
  - fn test_get_next_disablement_bit_sequential() { ... }
```

#### 5A.4 Testing Requirements

- Verify `class_editor` compiles after changes: `cargo check --bin class_editor`
- Run clippy to ensure no warnings: `cargo clippy --bin class_editor -- -D warnings`
- Manual test: Launch class_editor, add new class, verify it saves and loads correctly
- Verify existing RON files with `disablement_bit_index` still load without errors

#### 5A.5 Deliverables

- [ ] Updated `src/bin/class_editor.rs` with deprecated code removed
- [ ] All quality checks pass (fmt, check, clippy)
- [ ] Manual verification that class creation/loading still works

#### 5A.6 Success Criteria

- `get_next_disablement_bit()` function no longer exists in codebase
- No references to disablement bit in add/preview flows (except as legacy field in struct)
- Existing campaign data loads without errors
- New classes can be created and saved successfully

---

### Phase 5B: Item Editor Edit Flow Implementation

Implement full editing capability for item classifications, tags, and alignment restrictions.

#### 5B.1 Design Edit Flow Structure

**File:** `src/bin/item_editor.rs`

Design the edit menu structure for `edit_item()` function (currently a stub at L665-695).

**Required Menu Options:**
1. Edit basic info (name, costs)
2. Edit item type data (classification, weapon/armor stats)
3. Edit tags
4. Edit alignment restriction
5. Edit bonuses (constant, temporary)
6. Edit spell effect and charges
7. Toggle cursed flag
8. Preview changes
9. Save changes / Cancel

**Constraints:**
- Cannot change core item type (Weapon → Armor) without recreating item
- Must validate all inputs same as `add_item()` flow
- Should reuse existing helper methods: `select_weapon_classification()`, `select_armor_classification()`, `input_item_tags()`, `select_alignment_restriction()`, etc.

#### 5B.2 Implement Basic Info Editing

**File:** `src/bin/item_editor.rs`

Add menu option handlers for editing basic fields that don't require complex validation.

**Fields to support:**
- `name: String` - simple text input
- `base_cost: u32` - numeric input with validation
- `sell_cost: u32` - numeric input with validation
- `is_cursed: bool` - boolean toggle

**Implementation approach:**
```
Match menu choice:
  "1" => Edit name (read_input, update self.items[idx].name)
  "2" => Edit costs (read_u32 for base_cost and sell_cost)
  "7" => Toggle cursed (flip bool, show new state)
```

#### 5B.3 Implement Classification Editing

**File:** `src/bin/item_editor.rs`

Add menu option to edit item type-specific classifications.

**Fields to support based on ItemType:**
- `ItemType::Weapon` - edit `WeaponData.classification` via `select_weapon_classification()`
- `ItemType::Armor` - edit `ArmorData.classification` via `select_armor_classification()`
- `ItemType::Accessory` - edit `AccessoryData.classification` via `select_magic_item_classification()`
- Other types - show "No classification for this item type"

**Implementation approach:**
```
Match item type:
  ItemType::Weapon(ref mut data) => {
    let new_class = self.select_weapon_classification();
    data.classification = new_class;
  }
  // Similar for Armor, Accessory
```

#### 5B.4 Implement Tags and Alignment Editing

**File:** `src/bin/item_editor.rs`

Add menu options to edit tags and alignment restrictions.

**Fields to support:**
- `tags: Vec<String>` - use existing `input_item_tags()` method
- `alignment_restriction: Option<AlignmentRestriction>` - use existing `select_alignment_restriction()` method

**Implementation approach:**
```
"3" => Edit tags:
  let new_tags = self.input_item_tags();
  self.items[idx].tags = new_tags;

"4" => Edit alignment:
  let new_alignment = self.select_alignment_restriction();
  self.items[idx].alignment_restriction = new_alignment;
```

#### 5B.5 Implement Save/Cancel Logic

**File:** `src/bin/item_editor.rs`

Add proper state management for edit mode.

**Requirements:**
- Track if changes were made (`modified` flag)
- Show preview before final save
- Confirm before discarding changes if modified
- Update `self.modified = true` when item is changed

**Implementation approach:**
```
Loop edit menu until user chooses Save or Cancel:
  - Track local modified flag
  - On Save: set self.modified = true, break loop
  - On Cancel: if local modified, confirm discard, then break
```

#### 5B.6 Testing Requirements

- Manual test: Edit each field type (classification, tags, alignment) and verify changes persist after save/reload
- Manual test: Cancel edit without saving, verify no changes persist
- Manual test: Edit multiple fields in one session, verify all changes save correctly
- Verify clippy passes: `cargo clippy --bin item_editor -- -D warnings`

#### 5B.7 Deliverables

- [ ] Fully functional `edit_item()` in `src/bin/item_editor.rs`
- [ ] Support for editing all new proficiency-related fields
- [ ] Input validation matching `add_item()` flow
- [ ] Manual test verification documented

#### 5B.8 Success Criteria

- Users can edit item classifications, tags, and alignment restrictions without recreating items
- Edit flow reuses existing validation helpers
- Changes persist across save/reload cycles
- Quality checks pass (fmt, check, clippy)

---

### Phase 5C: Automated Test Coverage

Add unit and integration tests for CLI editor proficiency functionality.

#### 5C.1 Create CLI Test Infrastructure

**New File:** `tests/cli_editor_tests.rs`

Create integration test file for CLI editor round-trip testing.

**Test Structure:**
```rust
// Test data creation helpers
fn create_test_class_with_proficiencies() -> ClassDefinition { ... }
fn create_test_item_with_classification() -> Item { ... }
fn create_test_race_with_restrictions() -> RaceDefinition { ... }

// Round-trip test pattern:
// 1. Create test data
// 2. Save to temp RON file
// 3. Reload from file
// 4. Assert all fields match
```

**Dependencies:**
- `tempfile` crate for temporary test files
- Access to domain types: `ClassDefinition`, `Item`, `RaceDefinition`
- RON serialization/deserialization

#### 5C.2 Add Class Editor Round-Trip Tests

**File:** `tests/cli_editor_tests.rs`

Test that classes with proficiencies can be saved and reloaded correctly.

**Test Cases:**
1. `test_class_proficiency_round_trip()` - Create class with multiple proficiencies, save, reload, verify proficiencies match
2. `test_class_empty_proficiency_round_trip()` - Create class with empty proficiencies list, verify it loads as empty (not null)
3. `test_class_non_standard_proficiency_round_trip()` - Create class with custom proficiency ID, verify it persists

**Verification Points:**
- Proficiencies list length matches
- Proficiency IDs match exactly (order-independent comparison)
- All other fields unchanged (HP die, spell school, etc.)

#### 5C.3 Add Item Editor Round-Trip Tests

**File:** `tests/cli_editor_tests.rs`

Test that items with classifications, tags, and alignment restrictions can be saved and reloaded.

**Test Cases:**
1. `test_weapon_classification_round_trip()` - Create weapon with `WeaponClassification::MartialMelee`, save, reload, verify classification matches
2. `test_item_tags_round_trip()` - Create item with tags `["two_handed", "heavy"]`, save, reload, verify tags match
3. `test_alignment_restriction_round_trip()` - Create item with `AlignmentRestriction::GoodOnly`, save, reload, verify restriction matches
4. `test_item_derived_proficiency()` - Create weapon with classification, verify `item.required_proficiency()` returns correct proficiency ID

**Verification Points:**
- Classification enum matches exactly
- Tags list matches (order-independent)
- Alignment restriction matches
- Derived proficiency matches expected value

#### 5C.4 Add Race Editor Round-Trip Tests

**File:** `tests/cli_editor_tests.rs`

Test that races with proficiencies and incompatible tags can be saved and reloaded.

**Test Cases:**
1. `test_race_proficiency_round_trip()` - Create race with proficiencies, save, reload, verify proficiencies match
2. `test_race_incompatible_tags_round_trip()` - Create race with incompatible_item_tags, save, reload, verify tags match
3. `test_race_combined_restrictions()` - Create race with both proficiencies and incompatible tags, verify both persist

**Verification Points:**
- Proficiencies list matches
- Incompatible tags list matches
- All stat modifiers unchanged
- Resistances unchanged

#### 5C.5 Add Legacy Data Compatibility Tests

**File:** `tests/cli_editor_tests.rs`

Test that old RON files with deprecated fields load correctly into new structures.

**Test Cases:**
1. `test_load_legacy_class_with_disablement_bit()` - Load class RON with `disablement_bit_index`, verify it loads without error
2. `test_load_legacy_item_without_tags()` - Load item RON without `tags` field, verify `tags` defaults to empty vec
3. `test_load_legacy_item_without_classification()` - Load weapon RON without `classification` field, verify it defaults correctly

**Test Data:**
Create test RON strings with old schema format:
```ron
ClassDefinition(
  id: "knight",
  name: "Knight",
  disablement_bit_index: 0,
  // Missing proficiencies field
)
```

**Verification Points:**
- Deserialization succeeds (no errors)
- Missing fields get correct defaults (`#[serde(default)]` values)
- Legacy fields are preserved (backward compatibility)

#### 5C.6 Testing Requirements

- All new tests pass: `cargo test --test cli_editor_tests`
- Tests run in CI/CD pipeline
- Test coverage report shows >80% coverage for round-trip logic
- No test flakiness (deterministic results)

#### 5C.7 Deliverables

- [ ] New file `tests/cli_editor_tests.rs` with comprehensive test suite
- [ ] Round-trip tests for classes, items, races
- [ ] Legacy data compatibility tests
- [ ] All tests passing in CI

#### 5C.8 Success Criteria

- Automated tests verify all proficiency fields persist correctly across save/load cycles
- Legacy data loads without errors (backward compatibility proven)
- Classification to proficiency derivation tested
- Test suite runs in <5 seconds
- Zero test failures on main branch

---

### Phase 5D: Documentation and Manual Testing

Complete documentation deliverables and create manual test checklist.

#### 5D.1 Create Manual Test Checklist

**New File:** `docs/explanation/phase5_manual_test_checklist.md`

Document manual test procedures and results for Phase 5 completion.

**Required Sections:**
1. **Environment Setup** - How to run CLI editors for testing
2. **Class Editor Tests** - Step-by-step procedures for testing proficiency input/editing
3. **Item Editor Tests** - Procedures for testing classification, tags, alignment input/editing
4. **Race Editor Tests** - Procedures for testing proficiency and incompatible tags input/editing
5. **Validation Tests** - Test that invalid inputs are rejected with clear error messages
6. **Data Persistence Tests** - Verify changes persist after save/reload
7. **Test Results** - Checklist with pass/fail status for each test

**Format:**
```markdown
## Test: Class Editor - Add Class with Proficiencies
**Procedure:**
1. Run `cargo run --bin class_editor data/classes.ron`
2. Choose option 2 (Add class)
3. Enter proficiencies: simple_weapon, light_armor
4. Complete class creation
5. Save file
6. Reload file
7. Verify proficiencies appear in class list

**Expected Result:** Class loads with proficiencies intact

**Status:** [ ] Pass [ ] Fail

**Notes:**
```

#### 5D.2 Update Implementation Documentation

**File:** `docs/explanation/implementations.md`

Update Phase 5 entry with completion status and reference to completion plan.

**Changes:**
- Add reference to `phase5_completion_plan.md`
- Update status from "Partial" to "Complete" after all sub-phases done
- List completed deliverables with links

#### 5D.3 Update Phase 5 Implementation Document

**File:** `docs/explanation/phase5_cli_editors_implementation.md`

Add sections documenting completion work (deprecated code removal, edit flow implementation, testing).

**New Sections:**
- **Phase 5A: Deprecated Code Removal** - What was removed and why
- **Phase 5B: Edit Flow Completion** - How edit functionality works
- **Phase 5C: Test Coverage** - Overview of test infrastructure and test cases
- **Known Limitations** - Document any remaining gaps or future improvements

#### 5D.4 Testing Requirements

- Manual test checklist executed and documented with results
- All tests marked as Pass
- Any failures documented with workaround or fix plan

#### 5D.5 Deliverables

- [ ] `docs/explanation/phase5_manual_test_checklist.md` created and completed
- [ ] `docs/explanation/implementations.md` updated with Phase 5 completion status
- [ ] `docs/explanation/phase5_cli_editors_implementation.md` updated with completion work details

#### 5D.6 Success Criteria

- Manual test checklist covers all user-facing functionality
- All tests pass
- Documentation provides complete picture of Phase 5 work (original + completion)
- Clear evidence that Phase 5 requirements are fully met

---

## File Change Summary

### New Files

- `tests/cli_editor_tests.rs` - Integration tests for CLI editor round-trip functionality
- `docs/explanation/phase5_manual_test_checklist.md` - Manual test procedures and results
- `docs/explanation/phase5_completion_plan.md` - This document

### Modified Files

- `src/bin/class_editor.rs` - Remove deprecated `get_next_disablement_bit()` function and tests
- `src/bin/item_editor.rs` - Implement full `edit_item()` functionality
- `docs/explanation/implementations.md` - Update Phase 5 status to complete
- `docs/explanation/phase5_cli_editors_implementation.md` - Add completion work documentation

### Deleted Code

- `class_editor.rs`: Function `get_next_disablement_bit()` (~20 lines)
- `class_editor.rs`: Tests `test_get_next_disablement_bit_empty()` and `test_get_next_disablement_bit_sequential()` (~55 lines)
- `class_editor.rs`: Disablement bit preview output in `preview_class()` (~4 lines)

---

## Timeline Estimate

- **Phase 5A (Deprecated Code Removal):** 1-2 hours
  - Straightforward deletion of well-defined code
  - Manual testing to verify backward compatibility

- **Phase 5B (Edit Flow Implementation):** 4-6 hours
  - Most complex phase - requires menu implementation
  - Reuses existing helpers but needs integration work
  - Manual testing for each field type

- **Phase 5C (Automated Tests):** 3-4 hours
  - Test infrastructure setup
  - Writing test cases (repetitive but important)
  - Debugging test failures

- **Phase 5D (Documentation):** 2-3 hours
  - Manual test checklist creation and execution
  - Documentation updates

**Total Estimated Time:** 10-15 hours

**Recommended Sequence:** Execute phases in order (5A → 5B → 5C → 5D) as each builds on previous work.

---

## Risk Mitigation

**Risk:** Breaking existing CLI editors during deprecated code removal
**Mitigation:** Manual test existing functionality after each change; keep commits small

**Risk:** Edit flow implementation introduces inconsistencies with add flow
**Mitigation:** Reuse existing validation helper methods; manual testing of both flows

**Risk:** Test infrastructure takes longer than estimated
**Mitigation:** Start with simple round-trip tests; add complexity incrementally

**Risk:** Manual testing discovers bugs in original Phase 5 implementation
**Mitigation:** Document bugs, fix high-priority issues, defer low-priority to backlog

**Risk:** Integration tests fail in CI due to missing test data files
**Mitigation:** Use in-memory test data; create temporary files in tests; document any required test fixtures

---

## Success Criteria for Phase 5 Completion

**All of the following must be true:**

1. ✅ `get_next_disablement_bit()` function removed from `class_editor.rs`
2. ✅ `edit_item()` function fully implements classification/tags/alignment editing
3. ✅ Automated round-trip tests exist and pass for all three CLI editors
4. ✅ Manual test checklist completed with all tests passing
5. ✅ Documentation updated to reflect completion status
6. ✅ All quality checks pass: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`
7. ✅ No regressions in existing CLI editor functionality
8. ✅ Legacy RON data files load without errors (backward compatibility preserved)

**When all criteria met:** Phase 5 is complete and Phase 6 (Cleanup and Deprecation Removal) can begin.

---

## References

- `docs/explanation/proficiency_migration_plan.md` - Master migration plan
- `docs/reference/architecture.md` - Domain architecture reference
- `AGENTS.md` - Development guidelines
- `docs/explanation/phase5_cli_editors_implementation.md` - Phase 5 initial implementation
- `docs/explanation/implementations.md` - Implementation history
