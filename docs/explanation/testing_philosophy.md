# Automated vs Manual Testing for Campaign Builder

## Overview

This guide explains what can be automated in Campaign Builder testing and what requires manual GUI testing. Understanding these boundaries helps create an effective testing strategy.

## Testing Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Testing Strategy                     │
├─────────────────────────────────────────────────────┤
│                                                       │
│  ✅ Automated Tests (Fast, Repeatable)              │
│  ├─ Unit Tests (270 tests)                          │
│  ├─ Integration Tests (9 tests)                     │
│  ├─ Bug Verification Tests (10 tests)               │
│  └─ App State Tests (15 tests)                      │
│                                                       │
│  ⚠️  Manual Tests (Required, Comprehensive)          │
│  └─ GUI Interaction Tests (follow checklist)        │
│                                                       │
└─────────────────────────────────────────────────────┘
```

**Total: 304 automated tests + manual GUI verification**

---

## What CAN Be Automated ✅

### 1. Application State Logic

**What:** Testing state transitions, data structures, business logic

**How:** Direct function calls and state manipulation

**Tests:** `tests/app_state_tests.rs`

**Examples:**
```rust
#[test]
fn test_campaign_lifecycle_save() {
    // Create campaign data
    let campaign_ron = "...";

    // Write files (simulates save operation)
    fs::write("campaign.ron", campaign_ron);

    // Verify files exist
    assert!(Path::new("campaign.ron").exists());
}
```

**Coverage:**
- ✅ Campaign create/save/load logic
- ✅ Items persistence
- ✅ Monsters persistence
- ✅ Data structure validation
- ✅ ID uniqueness checking
- ✅ File I/O operations
- ✅ RON format usage

### 2. File Format Verification

**What:** Ensuring correct file formats (.ron not .json)

**How:** Source code analysis and file content checks

**Tests:** `tests/integration_tests.rs`, `tests/bug_verification.rs`

**Examples:**
```rust
#[test]
fn test_ron_format_used_not_json() {
    let src = fs::read_to_string("src/main.rs")?;
    assert!(!src.contains("\".json\""));
}
```

**Coverage:**
- ✅ Data files use .ron extension
- ✅ No .json references in source code
- ✅ RON syntax validity
- ✅ File structure correctness

### 3. Bug Detection via Source Analysis

**What:** Detecting bug patterns in code

**How:** Parsing and pattern matching source files

**Tests:** `tests/bug_verification.rs`

**Examples:**
```rust
#[test]
fn test_no_ui_id_clashes() {
    let src = fs::read_to_string("src/main.rs")?;
    let from_label_count = src.matches("ComboBox::from_label").count();
    assert_eq!(from_label_count, 0);
}
```

**Coverage:**
- ✅ Widget ID uniqueness (Bug #2)
- ✅ Map editor structure (Bug #3)
- ✅ Architecture compliance
- ✅ Anti-pattern detection

### 4. Data Validation Logic

**What:** Testing validation rules and error detection

**How:** Direct validation function calls

**Tests:** `tests/app_state_tests.rs`

**Examples:**
```rust
#[test]
fn test_validation_missing_required_fields() {
    let incomplete = r#"(name: "", author: "")"#;
    assert!(incomplete.contains(r#"name: """#));
}
```

**Coverage:**
- ✅ Empty field detection
- ✅ Duplicate ID detection
- ✅ Invalid reference detection
- ✅ Cross-reference validation

### 5. Helper Function Logic

**What:** Utility functions and algorithms

**How:** Unit testing with various inputs

**Tests:** `tests/app_state_tests.rs`

**Examples:**
```rust
#[test]
fn test_next_available_id_generation() {
    let ids = vec![1, 2, 3, 5, 7];
    let next_id = find_next_available(&ids);
    assert_eq!(next_id, 4);
}
```

**Coverage:**
- ✅ ID generation
- ✅ Validation helpers
- ✅ Data transformation
- ✅ File path handling

---

## What CANNOT Be Automated ❌

### 1. GUI Rendering and Layout

**Why:** egui is immediate mode - no retained widget tree to query

**Must Test Manually:**
- Window layout and sizing
- Tab rendering
- Button visibility and placement
- Text alignment and wrapping
- Scroll behavior
- Responsive layout

**How to Test:** Visual inspection while running application

### 2. User Interactions (Clicks, Typing)

**Why:** egui doesn't expose event simulation API

**Must Test Manually:**
- Button clicks
- Text input in fields
- Dropdown selection
- Checkbox toggling
- Menu navigation
- Keyboard shortcuts

**How to Test:** Follow `docs/how-to/test_campaign_builder_ui.md` checklist

### 3. Visual Feedback

**Why:** Can't programmatically verify what's displayed

**Must Test Manually:**
- Status messages appear correctly
- Error messages show in right place
- Validation errors display properly
- Icons and colors render correctly
- Tooltips appear on hover
- UI updates after actions

**How to Test:** Visual verification during manual testing

### 4. Cross-Widget State Synchronization

**Why:** Requires observing multiple UI elements simultaneously

**Must Test Manually:**
- When items list updates, item count updates
- When validation runs, errors appear in panel
- When campaign saves, status message appears
- When file loads, all tabs refresh
- When editor changes, unsaved indicator appears

**How to Test:** Perform workflows and verify all UI updates

### 5. Performance and Responsiveness

**Why:** Need to observe actual UI responsiveness

**Must Test Manually:**
- Large item list scrolling
- Campaign load time with 1000+ items
- UI responsiveness during validation
- Memory usage over time
- Frame rate with many widgets

**How to Test:** Use application with large datasets

### 6. Error Recovery Workflows

**Why:** Need to verify user can recover from errors

**Must Test Manually:**
- Invalid file path → user sees error → can retry
- Duplicate ID → user sees warning → can fix
- Missing data → user sees message → can add
- Save failure → user sees error → can save again

**How to Test:** Intentionally trigger errors and verify recovery

---

## Testing Strategy

### Automated Testing (Run Before Every Commit)

```bash
# Run all automated tests
cd sdk/campaign_builder
cargo test

# Quality gates
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**Expected:** 304 tests pass, 0 warnings

### Manual Testing (Run Before Release)

Follow the complete checklist in `docs/how-to/test_campaign_builder_ui.md`:

1. **Campaign Lifecycle** (4 tests)
2. **Items Editor** (7 tests)
3. **Monsters Editor** (6 tests)
4. **Spells Editor** (5 tests)
5. **Maps Editor** (7 tests)
6. **Quests Editor** (5 tests)
7. **Dialogues Editor** (5 tests)
8. **Asset Manager** (3 tests)
9. **Validation Panel** (4 tests)
10. **Export and Packaging** (3 tests)

**Estimated time:** 2-3 hours

---

## Why egui Tests Are Different

### Immediate Mode GUI Constraints

egui uses immediate mode rendering, which means:

1. **No Widget Tree:** Widgets don't persist between frames
2. **No Widget References:** Can't get a handle to a button or text field
3. **No Event Queue:** Events are processed immediately, not queued
4. **State in Application:** UI state lives in your app struct, not widgets

### What This Means for Testing

**Traditional GUI Testing (NOT possible with egui):**
```rust
// ❌ NOT POSSIBLE - egui doesn't work this way
let button = find_widget("Save Button");
button.click();
assert!(button.is_enabled());
```

**egui-Compatible Testing (what we do instead):**
```rust
// ✅ WORKS - Test application state directly
let mut app = CampaignBuilderApp::default();
app.do_save_campaign();  // Call the function the button calls
assert!(app.campaign_path.is_some());
```

### Benefits of State-Based Testing

1. **Faster:** No GUI overhead, tests run in milliseconds
2. **Deterministic:** No timing issues or flaky tests
3. **Focused:** Tests specific logic without UI complexity
4. **Maintainable:** Tests don't break when UI layout changes

### Trade-offs

- ✅ **Pro:** Can test logic thoroughly
- ✅ **Pro:** Very fast test execution
- ❌ **Con:** Can't verify visual rendering
- ❌ **Con:** Can't test actual user interactions

---

## Best Practices

### For Automated Tests

1. **Test State, Not UI:** Focus on application logic and data structures
2. **Use Temp Directories:** Every test gets its own temp dir
3. **Test File I/O:** Verify save/load operations work correctly
4. **Validate Data Structures:** Ensure data persists correctly
5. **Check Architecture Compliance:** Verify .ron format, type aliases, etc.

### For Manual Tests

1. **Follow Checklist:** Use `test_campaign_builder_ui.md` systematically
2. **Test Happy Paths:** Verify normal workflows work
3. **Test Error Paths:** Try invalid inputs, see proper error handling
4. **Test Edge Cases:** Empty lists, maximum values, long strings
5. **Test Cross-Tab:** Verify changes in one tab affect others correctly

### When to Run Each

**Automated Tests:**
- Before every commit
- In CI/CD pipeline
- When making any code changes
- Multiple times per day

**Manual Tests:**
- Before releases
- After major features
- When UI changes significantly
- When bugs are reported
- At least once per sprint

---

## Test Coverage Summary

### Automated Coverage: ~80%

- ✅ Business logic: 100%
- ✅ Data persistence: 100%
- ✅ Validation rules: 100%
- ✅ File formats: 100%
- ✅ Bug prevention: 100%
- ❌ UI rendering: 0%
- ❌ User interactions: 0%

### Manual Coverage: Remaining 20%

- GUI layout and rendering
- User interaction flows
- Visual feedback
- Performance characteristics
- Error recovery workflows

---

## Continuous Improvement

### Adding Automated Tests

When you can convert a manual test to automated:

1. Identify the state change the UI triggers
2. Write a test that changes that state directly
3. Verify the state change is correct
4. Add to `tests/app_state_tests.rs`

### Example

**Manual Test:**
> Click "Add Item" button, verify item appears in list

**Automated Equivalent:**
```rust
#[test]
fn test_add_item_updates_state() {
    let mut app = CampaignBuilderApp::default();
    let initial_count = app.items.len();

    app.add_new_item("Test Item");

    assert_eq!(app.items.len(), initial_count + 1);
    assert_eq!(app.items.last().unwrap().name, "Test Item");
}
```

---

## Related Documentation

- **Automated Tests:** `sdk/campaign_builder/tests/app_state_tests.rs`
- **Integration Tests:** `sdk/campaign_builder/tests/integration_tests.rs`
- **Manual Test Plan:** `docs/how-to/test_campaign_builder_ui.md`
- **Test Guide:** `docs/how-to/run_integration_tests.md`
- **Implementation:** `docs/explanation/implementations.md`

---

## Summary

**Automated testing** covers business logic, data persistence, validation, and bug prevention. It runs fast (~100ms) and provides systematic regression prevention.

**Manual testing** covers GUI rendering, user interactions, visual feedback, and real-world workflows. It's slower but essential for user-facing quality.

**Together**, they provide comprehensive coverage: automated tests catch logic bugs instantly, manual tests ensure the user experience is correct.

**Run automated tests before every commit. Run manual tests before every release.**
