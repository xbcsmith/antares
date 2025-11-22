# Systematic Bug Fixing Plan - Test-Driven Approach

**Date**: 2025-01-25
**Status**: IN PROGRESS
**Strategy**: Test First, Fix Second, Verify Third

---

## Executive Summary

Initial bug fixes were applied, but testing reveals **bugs still exist**. This document outlines a systematic, test-driven approach to identify, fix, and verify all remaining bugs.

**Key Finding**: Integration tests reveal actual bugs that weren't caught by unit tests.

---

## Current Situation

### What We Fixed (Claimed)
1. ✅ Bug #1: Data persistence (items/monsters save)
2. ⚠️ Bug #2: UI ID clashes (partially fixed)
3. ✅ Bug #3: Map terrain/wall independence

### What Tests Reveal (Reality)
1. ❌ **Bug #2 Still Exists**: ID pattern issues detected
2. ❌ **New Bug Found**: `.json` reference exists (should be `.ron`)
3. ⚠️ **Asset Loading**: Not verified yet
4. ⚠️ **Data Persistence**: Not fully tested in real workflow

### Test Results
```
Running tests/bug_verification.rs
test result: FAILED. 6 passed; 2 failed; 2 ignored
```

**Failures**:
- `test_bug_2_verify_unique_widget_ids` - ID patterns appear multiple times
- `test_ron_file_format_used_not_json` - Found `.json` reference

---

## Strategic Approach

### Phase 1: Improve Test Suite (CURRENT)
**Goal**: Create tests that accurately detect bugs without false positives

**Tasks**:
1. Fix test logic to distinguish between actual IDs and test references
2. Add more granular tests for specific UI scenarios
3. Create integration tests that simulate actual user workflows
4. Add tests for asset loading functionality

**Outcome**: Reliable test suite that proves bugs exist or are fixed

---

### Phase 2: Systematic Bug Discovery
**Goal**: Run all tests and catalog every failure

**Process**:
```bash
# Run all tests and capture output
cd sdk/campaign_builder
cargo test --test bug_verification 2>&1 | tee test_results.txt

# Analyze failures
grep "FAILED\|panicked" test_results.txt

# Document each bug with:
# - Test name that fails
# - Expected behavior
# - Actual behavior
# - Root cause hypothesis
```

**Outcome**: Complete list of verified bugs with reproduction steps

---

### Phase 3: Prioritize and Fix
**Goal**: Fix bugs in order of severity and user impact

**Priority Order**:
1. **P0 - Data Loss Bugs**: Anything that causes user to lose work
2. **P1 - UI Blocking**: Crashes, freezes, unusable UI
3. **P2 - UX Issues**: Confusing behavior, poor workflow
4. **P3 - Polish**: Nice-to-haves, cosmetic issues

**For Each Bug**:
```
1. Write failing test (if doesn't exist)
2. Verify test fails (proves bug exists)
3. Implement fix
4. Verify test passes (proves bug fixed)
5. Run full test suite (ensure no regression)
6. Document fix in implementations.md
```

---

### Phase 4: Verification
**Goal**: Prove all bugs are fixed through automated and manual testing

**Automated Verification**:
```bash
# All integration tests must pass
cargo test --test bug_verification

# All unit tests must pass
cargo test

# Code quality checks
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

**Manual Verification**:
- Run full manual test suite from `test_campaign_builder_ui.md`
- Test each bug scenario manually
- Verify fixes don't introduce new bugs

---

## Detailed Bug Analysis

### Bug #2: UI ID Clashes (Partially Fixed)

**Test That Failed**: `test_bug_2_verify_unique_widget_ids`

**Error Message**:
```
ID pattern 'item_type_filter' appears 3 times. Each ID should be unique.
```

**Investigation Needed**:
```bash
# Find all occurrences
grep -rn "item_type_filter" sdk/campaign_builder/src/

# Distinguish between:
# - Actual widget IDs in UI code (MUST be unique)
# - Test function names (OK to reuse)
# - String literals in tests (OK)
```

**Hypothesis**:
- Either multiple widgets use same ID (BUG)
- Or test is counting false positives (TEST BUG)

**Next Steps**:
1. Manually verify each occurrence
2. If real duplicates exist, rename to make unique
3. If test is wrong, fix test logic
4. Re-run test to verify

---

### Bug: JSON Reference Found

**Test That Failed**: `test_ron_file_format_used_not_json`

**Error Message**:
```
assertion failed: Should not use .json files, must use .ron format
left: 1 (found 1 occurrence of "items.json")
```

**Investigation**:
```bash
# Find the .json reference
grep -n "items.json" sdk/campaign_builder/src/main.rs
```

**Likely Causes**:
1. Old code not cleaned up
2. Comment referencing .json
3. Error message mentioning .json
4. Import/export feature using .json

**Fix Strategy**:
- If actual .json usage: Replace with .ron
- If comment/string: Update to reference .ron
- If intentional (export feature): Exclude from test

---

### Bug: Assets Not Loaded from Existing RON

**Reported By**: User feedback

**Symptom**: Assets (icons, sprites) not loading from campaign data files

**Tests Needed**:
```rust
#[test]
fn test_assets_load_from_campaign_dir() {
    // 1. Create campaign with assets
    // 2. Save campaign
    // 3. Create new app instance
    // 4. Load campaign
    // 5. Verify assets are accessible
}

#[test]
fn test_asset_manager_scans_directories() {
    // 1. Create assets/icons/ directory
    // 2. Add test icon files
    // 3. Initialize asset manager
    // 4. Verify icons appear in asset list
}
```

**Investigation Areas**:
- Does asset manager scan file system?
- Are asset paths stored in campaign metadata?
- Is asset loading triggered on campaign open?
- Are there asset references in items/monsters?

---

### Bug: Items and Monsters ID Clashes (User Reported)

**Reported By**: User feedback

**Symptom**: Despite fixes, ID clashes still occur in UI

**Investigation Needed**:
1. Run campaign builder manually
2. Open Items tab
3. Open Monsters tab
4. Switch between tabs multiple times
5. Use dropdowns and inputs
6. Document exact steps to reproduce issue

**Possible Root Causes**:
- Widget IDs are unique per-file but clash across files
- Widget IDs are unique but egui context is shared incorrectly
- Some widgets still don't have explicit IDs
- ID generation in loops creates duplicates

---

## Test Improvements Needed

### Current Test Issues

1. **Test Counts Occurrences in Test Files**
   - `item_type_filter` appears in test names
   - Test incorrectly counts these as duplicates
   - **Fix**: Only search in `src/`, exclude `tests/`

2. **Tests Are Too Generic**
   - Need specific scenario tests
   - Need actual UI interaction tests (if possible)
   - **Fix**: Create focused tests per component

3. **Missing Integration Tests**
   - No save/load roundtrip test
   - No actual app lifecycle test
   - **Fix**: Add tests that use real app instances

### Improved Test Structure

```rust
// Separate test categories
#[test]
fn test_unique_ids_in_items_editor_source_only() {
    let src = fs::read_to_string("src/main.rs").unwrap();

    // Only search in items editor function
    let items_fn = extract_function(&src, "show_items_editor");

    // Count actual from_id_salt calls
    let ids = extract_widget_ids(&items_fn);

    // Verify no duplicates
    assert_no_duplicates(&ids);
}

#[test]
fn test_save_load_roundtrip_preserves_items() {
    // Create temp campaign
    let temp_dir = tempfile::tempdir().unwrap();

    // Create app instance
    let mut app = CampaignBuilderApp::default();
    app.campaign_dir = Some(temp_dir.path().to_path_buf());

    // Add test item
    let test_item = create_test_item();
    app.items.push(test_item.clone());

    // Save campaign
    app.do_save_campaign().unwrap();

    // Create NEW app instance (simulates reopen)
    let mut app2 = CampaignBuilderApp::default();
    app2.campaign_dir = Some(temp_dir.path().to_path_buf());

    // Load campaign
    app2.load_items();

    // Verify item persists
    assert_eq!(app2.items.len(), 1);
    assert_eq!(app2.items[0].name, test_item.name);
}
```

---

## Action Plan - Next Steps

### Immediate (Next 1 Hour)

1. **Fix Test Suite**:
   - Update `test_bug_2_verify_unique_widget_ids` to only search `src/`
   - Add helper function to extract widget IDs accurately
   - Re-run tests to get accurate failure count

2. **Investigate JSON Reference**:
   ```bash
   grep -n "\.json" sdk/campaign_builder/src/main.rs
   ```
   - Document where it appears
   - Determine if it's a bug or false positive
   - Fix if needed

3. **Manual Testing Session**:
   - Run campaign builder manually
   - Test Items tab thoroughly
   - Test Monsters tab thoroughly
   - Document any UI freezes, crashes, or weird behavior

### Short Term (Next 2-3 Hours)

1. **Create Focused Integration Tests**:
   - Save/load roundtrip for items
   - Save/load roundtrip for monsters
   - Tab switching without crashes
   - Asset loading from filesystem

2. **Fix Verified Bugs**:
   - Fix JSON reference if it's a bug
   - Fix any real ID duplicates found
   - Fix asset loading if broken

3. **Run Full Test Suite**:
   - Verify all tests pass
   - Run manual tests from `test_campaign_builder_ui.md`
   - Document results

### Medium Term (Next 4-8 Hours)

1. **Complete Manual Test Suite**:
   - Work through all 10 test suites in `test_campaign_builder_ui.md`
   - Document every failure as a new bug
   - Prioritize bugs found

2. **Fix P0 and P1 Bugs**:
   - Data loss bugs first
   - UI blocking bugs second
   - Verify with tests after each fix

3. **Documentation Update**:
   - Update `implementations.md` with accurate status
   - Document all bugs found
   - Document all fixes applied

---

## Success Criteria

### Tests Must Show
- ✅ All integration tests pass
- ✅ All unit tests pass
- ✅ No ID clashes detected
- ✅ RON format used exclusively
- ✅ Assets load from filesystem

### Manual Testing Must Show
- ✅ Items persist after save/load
- ✅ Monsters persist after save/load
- ✅ Tab switching works smoothly (no freezes)
- ✅ All dropdowns work in all tabs
- ✅ Map editor allows terrain + wall selection
- ✅ Asset manager shows available assets

### Code Quality Must Show
- ✅ `cargo check` passes
- ✅ `cargo clippy` has zero warnings
- ✅ `cargo fmt` applied
- ✅ No `from_label()` usage in UI code
- ✅ All widgets have explicit unique IDs

---

## Risk Mitigation

### Risk: Test Suite Has False Positives
**Mitigation**: Manually verify each test failure before fixing code

### Risk: Fixing One Bug Breaks Another
**Mitigation**: Run full test suite after each fix

### Risk: Manual Testing Takes Too Long
**Mitigation**: Automate critical workflows with integration tests

### Risk: User-Reported Bugs We Can't Reproduce
**Mitigation**: Ask user for exact steps, test data, screenshots

---

## Documentation Strategy

### For Each Bug Fixed
1. Document in `implementations.md`:
   - Bug description
   - Root cause
   - Fix applied
   - Verification method

2. Update test suite:
   - Add test that proves bug fixed
   - Keep test in suite for regression prevention

3. Update `test_campaign_builder_ui.md`:
   - Add manual test case if needed
   - Mark bug as fixed in relevant test suite

---

## Conclusion

**Current Status**: Partial fixes applied, but bugs remain

**Recommended Approach**: Test-driven systematic fixing
1. Improve test suite for accuracy
2. Run tests to get complete bug list
3. Fix bugs one at a time with verification
4. Manual testing for final validation

**Timeline Estimate**:
- Test improvements: 1 hour
- Bug investigation: 2 hours
- Bug fixes: 3-4 hours
- Manual testing: 2 hours
- **Total**: 8-9 hours for complete bug elimination

**Next Action**: Fix test suite logic, then re-run to get accurate bug count.

---

**Author**: AI Agent
**Date**: 2025-01-25
**Status**: PLAN - Ready for Execution
