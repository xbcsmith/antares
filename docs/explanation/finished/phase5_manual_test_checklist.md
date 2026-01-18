# Phase 5D: Manual Test Checklist

## Overview

This document provides step-by-step manual testing scenarios to verify the Phase 5 CLI editor implementations. Each test includes detailed steps, expected results, and pass/fail checkboxes.

Quick index

- Test Suite 1: Class Editor - Proficiency System (Tests 1.1 - 1.4)
- Test Suite 2: Race Editor - Proficiencies and Incompatible Tags (Tests 2.1 - 2.4)
- Test Suite 3: Item Editor - Classifications, Tags, and Alignment (Tests 3.1 - 3.5)
- Test Suite 4: Legacy Data Compatibility (Tests 4.1 - 4.3)
- Test Suite 5: Integration Testing (Tests 5.1 - 5.2)
- Test Suite 6: Error Handling (Tests 6.1 - 6.2)

Test artifacts and references:

- Test data directory: `test_data/` (created by the environment setup below)
- Implementation reference: `docs/explanation/phase5_cli_editors_implementation.md`
- Architecture reference: `docs/reference/architecture.md`

Use this quick index to jump to the appropriate test suite for focused verification.

## Test Execution Guidelines

- Execute tests in order (dependencies exist between tests)
- Mark each test as **[PASS]** or **[FAIL]**
- Record any issues in the "Notes" section for each test
- Use fresh test data directories for each test run
- Verify both happy path and error cases

## Test Environment Setup

Pre-test checklist (run BEFORE starting manual tests)

- Ensure Rust toolchain components and tools are installed:
  - `rustup component add clippy rustfmt`
  - `cargo install nextest` (recommended)
  - `cargo install cargo-audit` (optional, recommended)
- Verify quality gates succeed before running manual tests:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo nextest run --all-features` (optional but recommended)
- Use a clean test data area for repeatable runs:
  - `rm -rf test_data/*` (run between test passes if re-running)

Commands to build editors and prepare test data:

```bash
# Build all editors
cargo build --release --bin class_editor
cargo build --release --bin race_editor
cargo build --release --bin item_editor

# Create test data directory
mkdir -p test_data
```

Notes:

- If any build or test step fails, address compilation/lint/test issues before performing manual scenarios.
- Keep a copy of failing `test_data/*.ron` artifacts for triage and issue reports.

---

## Test Suite 1: Class Editor - Proficiency System

### Test 1.1: Add Class with Standard Proficiencies

**Objective**: Verify class editor accepts standard proficiency IDs and saves correctly.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin class_editor`
2. Select option `1` (Add New Class)
3. Enter the following values:
   - **Class ID**: `test_knight`
   - **Display Name**: `Test Knight`
   - **HP Die**: `4` (1d10)
   - **Spell Access**: `1` (None)
   - **Proficiencies**: `simple_weapon,martial_melee,heavy_armor,shield`
4. Verify success message: `✅ Added proficiencies: simple_weapon, martial_melee, heavy_armor, shield`
5. Verify class created: `✅ Class 'test_knight' created successfully!`
6. Select option `4` (Save to File)
7. Save to: `test_data/test_classes.ron`

**Expected Results**:

- No errors or warnings displayed
- Success messages shown with checkmarks
- File `test_data/test_classes.ron` created
- File contains `proficiencies: ["simple_weapon", "martial_melee", "heavy_armor", "shield"]`

**Verification**:

```bash
cat test_data/test_classes.ron | grep -A 1 "proficiencies"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 1.2: Add Class with Custom Proficiency (Warning Path)

**Objective**: Verify warning system for non-standard proficiency IDs.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin class_editor`
2. Select option `1` (Add New Class)
3. Enter values:
   - **Class ID**: `test_mystic`
   - **Display Name**: `Test Mystic`
   - **HP Die**: `2` (1d6)
   - **Spell Access**: `3` (Arcane)
   - **Proficiencies**: `arcane_item,custom_psychic_weapon`
4. Observe warning: `⚠️  Warning: The following proficiency IDs are not standard: custom_psychic_weapon`
5. When prompted "Continue with these proficiencies? (y/n):", enter `y`
6. Verify class created successfully
7. Save to: `test_data/test_classes.ron`

**Expected Results**:

- Warning message displayed with ⚠️ symbol
- Confirmation prompt shown
- Class created after confirmation
- Custom proficiency saved in RON file

**Verification**:

```bash
cat test_data/test_classes.ron | grep "custom_psychic_weapon"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 1.3: Edit Existing Class - Modify Proficiencies

**Objective**: Verify editing proficiencies in existing class definition.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin class_editor`
2. Select option `3` (Load from File)
3. Load: `test_data/test_classes.ron`
4. Select option `2` (Edit Existing Class)
5. Select class: `test_knight`
6. Observe current proficiencies displayed in menu
7. Select option `5` (Edit Proficiencies)
8. Enter new proficiencies: `simple_weapon,martial_melee,light_armor,medium_armor,heavy_armor,shield`
9. Verify success message
10. Select option `0` (Back to Main Menu)
11. Select option `4` (Save to File)

**Expected Results**:

- Current proficiencies shown in edit menu: `(simple_weapon, martial_melee, heavy_armor, shield)`
- New proficiencies accepted
- Updated proficiencies saved to file

**Verification**:

```bash
cat test_data/test_classes.ron | grep -A 2 "test_knight"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 1.4: Class with Empty Proficiencies

**Objective**: Verify classes can be created with no proficiencies.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin class_editor`
2. Select option `1` (Add New Class)
3. Enter values:
   - **Class ID**: `test_commoner`
   - **Display Name**: `Test Commoner`
   - **HP Die**: `1` (1d4)
   - **Spell Access**: `1` (None)
   - **Proficiencies**: _(press Enter without typing)_
4. Verify message: `No proficiencies added.`
5. Save to: `test_data/test_classes.ron`

**Expected Results**:

- No error for empty proficiency list
- Class created successfully
- RON file contains `proficiencies: []`

**Verification**:

```bash
cat test_data/test_classes.ron | grep -B 1 -A 1 "test_commoner"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Suite 2: Race Editor - Proficiencies and Incompatible Tags

### Test 2.1: Add Race with Proficiencies and Tags

**Objective**: Verify race editor handles both proficiencies and incompatible_item_tags.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin race_editor`
2. Select option `1` (Add New Race)
3. Enter values:
   - **Race ID**: `test_halfling`
   - **Display Name**: `Test Halfling`
   - **Size Category**: `1` (Small)
   - **Attribute Modifiers**: (enter defaults or skip)
   - **Proficiencies**: `simple_weapon,light_armor`
   - **Incompatible Tags**: `large_weapon,heavy_armor`
4. Verify both success messages displayed
5. Save to: `test_data/test_races.ron`

**Expected Results**:

- Success for proficiencies: `✅ Added proficiencies: simple_weapon, light_armor`
- Success for tags: `✅ Added incompatible tags: large_weapon, heavy_armor`
- Race created successfully
- File contains both fields correctly

**Verification**:

```bash
cat test_data/test_races.ron | grep -A 5 "test_halfling"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 2.2: Race with Custom Incompatible Tags (Warning Path)

**Objective**: Verify warning system for non-standard item tags.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin race_editor`
2. Select option `1` (Add New Race)
3. Enter values:
   - **Race ID**: `test_pixie`
   - **Display Name**: `Test Pixie`
   - **Size Category**: `0` (Tiny)
   - **Proficiencies**: `simple_weapon`
   - **Incompatible Tags**: `large_weapon,heavy_armor,custom_iron_allergy`
4. Observe warning for custom tag
5. Confirm with `y`
6. Save to: `test_data/test_races.ron`

**Expected Results**:

- Warning shown: `⚠️  Warning: The following item tags are not standard: custom_iron_allergy`
- Confirmation prompt displayed
- Race created after confirmation
- Custom tag saved in file

**Verification**:

```bash
cat test_data/test_races.ron | grep "custom_iron_allergy"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 2.3: Edit Race - Modify Incompatible Tags

**Objective**: Verify editing incompatible_item_tags field.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin race_editor`
2. Load: `test_data/test_races.ron`
3. Select option `2` (Edit Existing Race)
4. Select race: `test_halfling`
5. Verify current tags shown in menu: `(large_weapon, heavy_armor)`
6. Select option `8` (Edit Incompatible Item Tags)
7. Enter new tags: `large_weapon,heavy_armor,two_handed`
8. Verify update success
9. Save to file

**Expected Results**:

- Current tags displayed correctly
- New tags accepted and saved
- File updated with three tags

**Verification**:

```bash
cat test_data/test_races.ron | grep -A 6 "test_halfling" | grep "incompatible_item_tags"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 2.4: Race with No Restrictions

**Objective**: Verify races can have empty proficiencies and tags.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin race_editor`
2. Select option `1` (Add New Race)
3. Enter values:
   - **Race ID**: `test_human`
   - **Display Name**: `Test Human`
   - **Size Category**: `2` (Medium)
   - **Proficiencies**: _(empty)_
   - **Incompatible Tags**: _(empty)_
4. Verify both "No ... added" messages
5. Save to: `test_data/test_races.ron`

**Expected Results**:

- Both fields can be empty
- Race created successfully
- RON file contains empty arrays: `proficiencies: []` and `incompatible_item_tags: []`

**Verification**:

```bash
cat test_data/test_races.ron | grep -A 6 "test_human"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Suite 3: Item Editor - Classifications, Tags, and Alignment

### Test 3.1: Create Weapon with Classification and Tags

**Objective**: Verify item editor handles weapon classification, tags, and derived proficiency.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin item_editor`
2. Select option `1` (Add New Item)
3. Enter values:
   - **Item Type**: `1` (Weapon)
   - **Item ID**: `42`
   - **Name**: `Test Greatsword`
   - **Damage**: `2d6` (dice: 2, sides: 6, bonus: 0)
   - **Hands**: `2`
   - **Weapon Classification**: `2` (Martial Melee)
   - **Base Cost**: `500`
   - **Sell Cost**: `250`
   - **Alignment Restriction**: `1` (None/Any)
   - **Item Tags**: `large_weapon,two_handed`
4. Select option `2` (Preview Item)
5. Verify preview shows:
   - `Alignment: Any`
   - `Tags: large_weapon, two_handed`
   - `⚔️  Required Proficiency: martial_melee`
6. Save to: `test_data/test_items.ron`

**Expected Results**:

- Classification correctly mapped to proficiency requirement
- Tags displayed in preview
- Alignment shown as "Any"
- All fields saved correctly

**Verification**:

```bash
cat test_data/test_items.ron | grep -A 10 "Test Greatsword"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 3.2: Create Armor with Classification

**Objective**: Verify armor classification system.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin item_editor`
2. Select option `1` (Add New Item)
3. Enter values:
   - **Item Type**: `2` (Armor)
   - **Item ID**: `50`
   - **Name**: `Test Plate Mail`
   - **AC Bonus**: `8`
   - **Armor Classification**: `3` (Heavy)
   - **Weight**: `50`
   - **Base Cost**: `1500`
   - **Alignment Restriction**: `1` (None)
   - **Item Tags**: `heavy_armor`
4. Preview item
5. Verify: `⚔️  Required Proficiency: heavy_armor`
6. Save to: `test_data/test_items.ron`

**Expected Results**:

- Heavy armor classification saved
- Proficiency requirement shown as `heavy_armor`
- Weight field saved correctly

**Verification**:

```bash
cat test_data/test_items.ron | grep -A 8 "Test Plate Mail"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 3.3: Create Item with Alignment Restriction

**Objective**: Verify alignment restriction system.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin item_editor`
2. Create three items with different alignment restrictions:

**Item A - Good Only**:

- **Item ID**: `100`
- **Name**: `Test Holy Sword`
- **Type**: Weapon (Simple Melee)
- **Alignment**: `2` (Good Only)
- **Preview**: Shows `Alignment: Good Only`

**Item B - Evil Only**:

- **Item ID**: `101`
- **Name**: `Test Cursed Blade`
- **Type**: Weapon (Simple Melee)
- **Alignment**: `3` (Evil Only)
- **Preview**: Shows `Alignment: Evil Only`

**Item C - Any**:

- **Item ID**: `102`
- **Name**: `Test Iron Sword`
- **Type**: Weapon (Simple Melee)
- **Alignment**: `1` (None/Any)
- **Preview**: Shows `Alignment: Any`

4. Save to: `test_data/test_items.ron`

**Expected Results**:

- All three alignment options work correctly
- Preview displays correct alignment text
- RON file contains correct `alignment_restriction` field

**Verification**:

```bash
cat test_data/test_items.ron | grep -B 2 "alignment_restriction"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 3.4: Create Accessory with Magic Classification

**Objective**: Verify magic item classification for accessories.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin item_editor`
2. Select option `1` (Add New Item)
3. Enter values:
   - **Item Type**: `4` (Accessory)
   - **Item ID**: `200`
   - **Name**: `Test Arcane Ring`
   - **Accessory Slot**: `1` (Ring)
   - **Magic Item Classification**: `1` (Arcane)
   - **Base Cost**: `1000`
4. Preview item
5. Verify: `⚔️  Required Proficiency: arcane_item`
6. Save to: `test_data/test_items.ron`

**Expected Results**:

- Arcane classification saved correctly
- Proficiency shown as `arcane_item`
- Accessory slot saved

**Verification**:

```bash
cat test_data/test_items.ron | grep -A 6 "Test Arcane Ring"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 3.5: Item with Custom Tags (Warning Path)

**Objective**: Verify warning system for non-standard item tags.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Run: `cargo run --bin item_editor`
2. Create item with custom tag:
   - **Item ID**: `300`
   - **Name**: `Test Experimental Weapon`
   - **Type**: Weapon (Simple Melee)
   - **Tags**: `two_handed,custom_unstable,custom_prototype`
3. Observe warning message listing custom tags
4. Confirm with `y`
5. Verify item created
6. Save to: `test_data/test_items.ron`

**Expected Results**:

- Warning displayed for custom tags
- Confirmation prompt shown
- Item created after confirmation
- Custom tags saved in file

**Verification**:

```bash
cat test_data/test_items.ron | grep "custom_unstable"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Suite 4: Legacy Data Compatibility

### Test 4.1: Load Legacy Class File (No Proficiencies Field)

**Objective**: Verify backward compatibility with old class definitions.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Create legacy class file: `test_data/legacy_classes.ron`
   ```ron
   {
       "knight": (
           display_name: "Knight",
           hp_die: 4,
           spell_access: None,
           disablement_bit_index: Some(0),
       ),
   }
   ```
2. Run: `cargo run --bin class_editor`
3. Load: `test_data/legacy_classes.ron`
4. Verify file loads without errors
5. Select option `2` (Edit Existing Class)
6. Select: `knight`
7. Observe proficiencies shown as: `(None)`
8. Add proficiencies: `simple_weapon,martial_melee`
9. Save file

**Expected Results**:

- Legacy file loads successfully
- Missing `proficiencies` field defaults to empty array
- Can add proficiencies to legacy class
- Saved file includes new field

**Verification**:

```bash
cat test_data/legacy_classes.ron | grep "proficiencies"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 4.2: Load Legacy Race File (No Incompatible Tags)

**Objective**: Verify backward compatibility with old race definitions.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Create legacy race file: `test_data/legacy_races.ron`
   ```ron
   {
       "elf": (
           display_name: "Elf",
           size_category: Medium,
           base_stats: (might: 10, intellect: 12, personality: 11, endurance: 9, speed: 11, accuracy: 12, luck: 10),
           disablement_bit_index: Some(1),
       ),
   }
   ```
2. Run: `cargo run --bin race_editor`
3. Load: `test_data/legacy_races.ron`
4. Verify loads without errors
5. Edit race `elf`
6. Observe both proficiencies and tags shown as: `(None)`
7. Add values and save

**Expected Results**:

- Legacy file loads successfully
- Missing fields default to empty arrays
- Can add new fields
- File saves with new structure

**Verification**:

```bash
cat test_data/legacy_races.ron | grep -E "(proficiencies|incompatible_item_tags)"
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 4.3: Load Legacy Item File (No Tags or Classifications)

**Objective**: Verify backward compatibility with old item definitions.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Create legacy item file: `test_data/legacy_items.ron`
   ```ron
   {
       1: (
           name: "Longsword",
           item_type: Weapon((
               damage: (dice: 1, sides: 8, bonus: 0),
               hands: 1,
           )),
           base_cost: 100,
           sell_cost: 50,
           disablements: (255),
       ),
   }
   ```
2. Run: `cargo run --bin item_editor`
3. Load: `test_data/legacy_items.ron`
4. Verify loads without errors
5. Preview item
6. Verify shows: `Disablement Flags (legacy): 0xFF`
7. Edit item to add classification and tags
8. Save file

**Expected Results**:

- Legacy file loads successfully
- Missing `alignment_restriction` defaults
- Missing `tags` defaults to empty
- Missing `classification` can be added
- Legacy `disablements` field preserved and shown
- File saves with mixed old/new fields

**Verification**:

```bash
cat test_data/legacy_items.ron
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Suite 5: Integration Testing

### Test 5.1: Full Workflow - Class, Race, Item Creation

**Objective**: Verify complete data creation workflow across all three editors.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Create full set of test data:

**Class**: Create "test_battle_mage"

- Proficiencies: `simple_weapon,martial_melee,light_armor,arcane_item`
- Save to: `test_data/integration_classes.ron`

**Race**: Create "test_wood_elf"

- Proficiencies: `simple_weapon,martial_ranged,light_armor`
- Incompatible Tags: `heavy_armor`
- Save to: `test_data/integration_races.ron`

**Item**: Create "test_elven_longbow"

- Classification: Martial Ranged
- Tags: `elven_crafted,two_handed`
- Alignment: Any
- Save to: `test_data/integration_items.ron`

2. Verify all files created
3. Verify cross-references make sense:
   - Battle Mage has `arcane_item` proficiency
   - Wood Elf cannot use `heavy_armor` (incompatible tag)
   - Elven Longbow requires `martial_ranged` proficiency
   - Wood Elf has `martial_ranged`, so can use bow
   - Elven Longbow has `elven_crafted` tag (no race restriction against it)

**Expected Results**:

- All three files created successfully
- Data is internally consistent
- Proficiency requirements align across editors
- Tag system properly restricts items

**Verification**:

```bash
ls -l test_data/integration_*.ron
cat test_data/integration_classes.ron
cat test_data/integration_races.ron
cat test_data/integration_items.ron
```

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 5.2: Cross-Editor Data Validation

**Objective**: Verify data created in one editor can be read by others.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Use files from Test 5.1
2. Verify references:
   - Item `test_elven_longbow` requires proficiency `martial_ranged`
   - Race `test_wood_elf` has proficiency `martial_ranged`
   - Race `test_wood_elf` has incompatible tag `heavy_armor`
   - Create a heavy armor item with tag `heavy_armor`
   - Verify proficiency/tag consistency

**Expected Results**:

- Proficiency IDs match across editors
- Tag IDs match across editors
- Data relationships are logically sound

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Suite 6: Error Handling

### Test 6.1: Invalid Input Handling

**Objective**: Verify editors handle invalid input gracefully.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Test each editor with invalid inputs:
   - Non-numeric values for numeric fields
   - Out-of-range menu selections
   - Invalid file paths
   - Malformed RON syntax in loaded files

**Expected Results**:

- Clear error messages displayed
- Editor doesn't crash
- User can retry input
- No data corruption

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

### Test 6.2: File I/O Error Handling

**Objective**: Verify file operation error handling.

**Status**: [ ] PASS / [ ] FAIL

**Steps**:

1. Test error scenarios:
   - Load non-existent file
   - Save to read-only directory
   - Load corrupted RON file
   - Save with no data entered

**Expected Results**:

- Errors reported clearly
- Editor remains stable
- User can continue after error

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

---

## Test Summary

**Total Tests**: 20

**Passed**: **\_\_\_**

**Failed**: **\_\_\_**

**Success Rate**: **\_\_\_**%

---

## Issues Found

| Test ID | Issue Description | Severity | Notes |
| ------- | ----------------- | -------- | ----- |
|         |                   |          |       |
|         |                   |          |       |
|         |                   |          |       |

---

## Recommendations

Based on manual testing results, list any recommendations for:

- Bug fixes needed
- UX improvements
- Additional validation
- Documentation updates
- Edge cases to handle

## Reporting & Automation Guidance

Reporting issues (recommended contents)

- Create an issue in the repository (label: `phase5-manual-test`) and include:
  - Test ID (e.g., `Test 3.1`) and test title
  - Exact reproduction steps (copy the "Steps" section from the checklist)
  - Expected result and actual output (attach screenshots or copy/paste text)
  - A copy of any produced RON files (e.g., `test_data/test_items.ron`) and relevant stdout/stderr
  - Environment details (OS, Rust version, commit hash)
  - Severity (Critical / Major / Minor / Trivial) and suggested priority
- If multiple tests fail with the same root cause, link them from a single parent issue to avoid duplication.

Automation tips (optional)

- Interactive CLI editors can often be automated for repeatable acceptance tests using `expect` / `pexpect` or other CLI-driving tools. Example `expect` skeleton (adapt to the editor's exact prompts and timing):

```bash
#!/usr/bin/expect -f
set timeout -1
spawn cargo run --bin class_editor
expect "Select option" { send "1\r" }
expect "Class ID" { send "test_knight\r" }
expect "Display Name" { send "Test Knight\r" }
expect "Enter proficiencies" { send "simple_weapon,martial_melee\r" }
expect "Continue with these proficiencies? (y/n)" { send "y\r" }
expect "Save to File" { send "test_data/test_classes.ron\r" }
expect eof
```

- Use automation as a complement to manual verification; interactive prompts can vary and should be validated manually at least once.

Test data cleanup and retest workflow

- After fixing issues, rerun the failing tests and update the Test Summary and Issues table.
- Keep failing artifacts in `test_data/fails/<test-id>/...` for triage.

---

## Tester Information & Sign-off

**Tester Name**: **********\_\_\_**********

**Test Date**: **********\_\_\_**********

**Environment**:

- OS: **********\_\_\_**********
- Rust Version: **********\_\_\_**********
- Commit Hash: **********\_\_\_**********

**Final Status**: [ ] PASS / [ ] FAIL

**All critical issues opened**: [ ] Yes / [ ] No

**Notes**:

```
_____________________________________________________________________________
_____________________________________________________________________________
```

**Sign-off (name & date)**: **********\_\_\_**********

Sign-off Guidelines:

- All tests have been executed and recorded in the Test Summary.
- All failing tests have associated issues documented in the Issues Found table (include issue number).
- Critical regressions are either fixed and re-tested or have an agreed mitigation plan.
- Build/commit hash is recorded for traceability.

---

## Notes

Use this space for any additional observations or comments:

```
_____________________________________________________________________________
_____________________________________________________________________________
_____________________________________________________________________________
_____________________________________________________________________________
_____________________________________________________________________________
```
