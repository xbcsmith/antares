# Phase 6 Cleanup Plan: Disablement System Removal

## Overview
This plan completes Phase 6 of the Proficiency Migration Plan by removing the deprecated `Disablement` bitmask system. The proficiency-based classification system is already implemented (Phases 1-5); this cleanup removes legacy code and data.

**Note**: No backward compatibility. Tutorial campaign will be fixed after core changes are verified.

## Phase 1: Remove Disablement Struct from Domain
**Objective**: Remove the deprecated `Disablement` struct and all related code.

- [ ] **Task 1.1**: In `src/domain/items/types.rs`, delete the `Disablement` struct and its `impl` block (~lines 407-860).
- [ ] **Task 1.2**: In `src/domain/items/types.rs`, remove `disablements: Disablement` field from `Item` struct (~line 937).
- [ ] **Task 1.3**: In `src/domain/items/mod.rs`, remove `Disablement` from the public exports (~line 43).
- [ ] **Task 1.4**: In `src/domain/items/database.rs`, remove all `Disablement` references from doc examples.
- [ ] **Task 1.5**: Run `cargo check --all-targets` and fix any remaining compile errors.

## Phase 2: Remove Deprecated Code from Race Editor
**Objective**: Remove legacy disablement bit allocation from race editor CLI.

- [ ] **Task 2.1**: In `src/bin/race_editor.rs`, remove the call to `get_next_disablement_bit()` (~line 215).
- [ ] **Task 2.2**: In `src/bin/race_editor.rs`, delete the `get_next_disablement_bit()` function (~lines 655-700).
- [ ] **Task 2.3**: In `src/bin/race_editor.rs`, delete test functions `test_get_next_disablement_bit_empty()` and `test_get_next_disablement_bit_sequential()` (~lines 857-902).
- [ ] **Task 2.4**: Run `cargo test --bin race_editor` to verify remaining tests pass.

## Phase 3: Update Core Data Files
**Objective**: Remove deprecated fields from `data/` RON files.

- [ ] **Task 3.1**: In `data/items.ron`, remove `disablements: (N)` from all item definitions (~30 occurrences).
- [ ] **Task 3.2**: In `data/classes.ron`, remove `disablement_bit: N` from all class definitions (6 occurrences).
- [ ] **Task 3.3**: In `data/races.ron`, remove `disablement_bit: N` from all race definitions (6 occurrences).
- [ ] **Task 3.4**: Run `cargo test` to verify data files load correctly.

## Phase 4: Update Tutorial Campaign Data Files
**Objective**: Remove deprecated fields from tutorial campaign RON files.

- [ ] **Task 4.1**: In `campaigns/tutorial/data/items.ron`, remove `disablements: (N)` from all items (~30 occurrences).
- [ ] **Task 4.2**: In `campaigns/tutorial/data/classes.ron`, remove `disablement_bit: N` from all classes (6 occurrences).
- [ ] **Task 4.3**: In `campaigns/tutorial/data/races.ron`, remove `disablement_bit: N` from all races (4 occurrences).
- [ ] **Task 4.4**: Run `cargo test --test phase14_campaign_integration_test` to verify tutorial loads.

## Phase 5: Add Proficiency UNION Logic Integration Tests
**Objective**: Add explicit integration tests for proficiency resolution with UNION logic.

- [ ] **Task 5.1**: In `tests/cli_editor_tests.rs` (or new file `tests/proficiency_integration_test.rs`), add test `test_proficiency_union_class_grants()`: verify character CAN use item when CLASS grants proficiency.
- [ ] **Task 5.2**: Add test `test_proficiency_union_race_grants()`: verify character CAN use item when RACE grants proficiency (e.g., Elf Sorcerer with Long Bow).
- [ ] **Task 5.3**: Add test `test_proficiency_union_neither_grants()`: verify character CANNOT use item when neither class nor race grants proficiency.
- [ ] **Task 5.4**: Add test `test_race_incompatible_tags()`: verify character CANNOT use item when race has incompatible tag (e.g., Halfling with large_weapon).
- [ ] **Task 5.5**: Add test `test_proficiency_overrides_race_tag()`: verify proficiency does NOT override race tag restrictions.
- [ ] **Task 5.6**: Run `cargo test --test proficiency_integration_test` to verify all tests pass.

## Phase 6: Update Documentation
**Objective**: Rename and rewrite disablement documentation to describe the proficiency system.

- [ ] **Task 6.1**: Delete `docs/explanation/disablement_bits.md`.
- [ ] **Task 6.2**: Create `docs/explanation/proficiency_system.md` documenting:
    - Classification enums (`WeaponClassification`, `ArmorClassification`, `MagicItemClassification`)
    - Proficiency resolution logic (UNION of class and race proficiencies)
    - Item tags system for fine-grained restrictions
    - Data file formats for classes, races, and items
- [ ] **Task 6.3**: Update `docs/explanation/implementations.md` to mark Phase 6 complete.

## Phase 7: Final Verification
**Objective**: Verify all deprecated code is removed and the system works correctly.

- [ ] **Task 7.1**: Run `grep -r "Disablement" src/ --include="*.rs"` and verify no non-comment matches.
- [ ] **Task 7.2**: Run `grep -r "disablement" data/ campaigns/ --include="*.ron"` and verify no matches.
- [ ] **Task 7.3**: Run full test suite: `cargo test --all-features`.
- [ ] **Task 7.4**: Run clippy: `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] **Task 7.5**: Build and verify: `cargo build --all-targets`.

## Execution Strategy
Execute phases sequentially. Each phase has verification steps. Do not proceed to the next phase if verification fails.
