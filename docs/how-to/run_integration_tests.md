# How to Run Campaign Builder Integration Tests

## Overview

This guide explains how to run the integration tests for the Campaign Builder UI. These tests verify that the three critical bugs have been fixed and provide regression prevention.

## Quick Start

```bash
# Navigate to campaign builder directory
cd sdk/campaign_builder

# Run all integration tests
cargo test --test integration_tests --test bug_verification

# Run with output visible
cargo test --test integration_tests --test bug_verification -- --nocapture
```

## Test Categories

### 1. Integration Tests (`integration_tests.rs`)

Comprehensive end-to-end tests that verify data persistence and file operations.

```bash
# Run all integration tests
cargo test --test integration_tests

# Run specific test
cargo test --test integration_tests test_campaign_save_load_roundtrip
cargo test --test integration_tests test_items_persist_after_save
cargo test --test integration_tests test_no_ui_id_clashes
```

**Tests included:**
- `test_campaign_save_load_roundtrip` - Verifies complete save/load cycle
- `test_items_persist_after_save` - Bug #1: Items persistence
- `test_monsters_persist_after_save` - Bug #1: Monsters persistence
- `test_no_ui_id_clashes` - Bug #2: Widget ID uniqueness
- `test_ron_format_used_not_json` - Architecture compliance
- `test_map_editor_terrain_wall_independence` - Bug #3: Map editor fix
- `test_all_data_files_use_ron_format` - Data format validation
- `test_campaign_directory_structure` - Directory creation
- `test_empty_ron_files_valid` - RON format validation

### 2. Bug Verification Tests (`bug_verification.rs`)

Source code analysis tests that detect specific bug patterns.

```bash
# Run all bug verification tests
cargo test --test bug_verification

# Run including ignored tests
cargo test --test bug_verification -- --ignored --include-ignored

# Run specific bug test
cargo test --test bug_verification test_bug_2_verify_unique_widget_ids
cargo test --test bug_verification test_bug_3_map_editor_terrain_wall_independence
```

**Tests included:**
- `test_bug_1_items_persist_after_campaign_save` (ignored - placeholder)
- `test_bug_1_monsters_persist_after_campaign_save` (ignored - placeholder)
- `test_bug_2_verify_unique_widget_ids` - ID clash detection
- `test_bug_3_map_editor_terrain_wall_independence` - Map editor structure
- `test_items_tab_widget_ids_unique` - Items tab ID check
- `test_monsters_tab_widget_ids_unique` - Monsters tab ID check
- `test_ron_file_format_used_not_json` - File format check
- `test_asset_loading_from_ron` - Asset loading check
- `test_campaign_save_creates_all_data_files` - File creation check
- `test_no_implicit_widget_id_generation` - Widget ID patterns

### 3. All Tests (Including Unit Tests)

```bash
# Run ALL tests (unit + integration)
cargo test

# Run with verbose output
cargo test -- --nocapture --test-threads=1
```

## Expected Results

### Successful Test Run

```
running 10 tests (bug_verification)
test result: ok. 8 passed; 0 failed; 2 ignored

running 9 tests (integration_tests)
test result: ok. 9 passed; 0 failed; 0 ignored

running 270 tests (unit tests)
test result: ok. 270 passed; 0 failed; 0 ignored
```

**Total: 287 tests passing**

### Understanding Test Output

- **passed** - Test executed successfully and assertions passed
- **failed** - Test detected a bug or regression
- **ignored** - Test is marked with `#[ignore]` and skipped by default

## Quality Gate Commands

Before committing changes, run these commands in order:

```bash
# 1. Format code
cargo fmt --all

# 2. Check compilation
cargo check --all-targets --all-features

# 3. Run clippy (no warnings allowed)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run all tests
cargo test --all-features
```

**All four must pass with zero errors/warnings.**

## Troubleshooting

### Test Failures

If a test fails, read the error message carefully:

```bash
# Run specific failing test with output
cargo test --test integration_tests test_name -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test --test integration_tests test_name
```

### Common Issues

#### Issue: Tests can't find source files

**Solution:** Make sure you're in the `sdk/campaign_builder` directory:

```bash
pwd  # Should show: .../antares/sdk/campaign_builder
cd sdk/campaign_builder
```

#### Issue: Clippy warnings in main code (not tests)

**Solution:** Fix the main code warnings first. Integration tests are clean, but main application may have warnings to fix separately.

#### Issue: Permission denied creating temp directories

**Solution:** Check disk space and temp directory permissions:

```bash
df -h /tmp
ls -ld /tmp
```

### Debugging Tests

Add debug output to tests:

```rust
#[test]
fn test_something() {
    println!("Debug: checking value = {:?}", value);
    dbg!(&some_variable);
    assert_eq!(expected, actual);
}
```

Run with output visible:

```bash
cargo test test_something -- --nocapture
```

## Test Organization

### File Structure

```
sdk/campaign_builder/
├── src/
│   ├── main.rs              # Application code
│   ├── map_editor.rs        # Map editor module
│   └── ...
├── tests/
│   ├── integration_tests.rs  # End-to-end tests
│   └── bug_verification.rs   # Source code analysis tests
└── Cargo.toml               # Dependencies (includes tempfile)
```

### Test Naming Convention

Tests follow the pattern: `test_{component}_{condition}_{expected}`

Examples:
- `test_campaign_save_load_roundtrip` - Campaign save/load cycle
- `test_items_persist_after_save` - Items survive save operation
- `test_no_ui_id_clashes` - No duplicate widget IDs

## CI/CD Integration

These tests should run automatically in CI:

```yaml
# .github/workflows/campaign_builder.yml
- name: Run integration tests
  run: cd sdk/campaign_builder && cargo test --test '*'
```

## What Each Bug Test Verifies

### Bug #1: Data Persistence ✅ FIXED

Tests verify items, monsters, spells, quests, and dialogues are saved to .ron files.

- `test_items_persist_after_save`
- `test_monsters_persist_after_save`
- `test_campaign_save_load_roundtrip`

### Bug #2: UI ID Clashes ✅ FIXED

Tests verify all egui widgets use unique IDs (no `ComboBox::from_label`).

- `test_no_ui_id_clashes`
- `test_bug_2_verify_unique_widget_ids`
- `test_items_tab_widget_ids_unique`
- `test_monsters_tab_widget_ids_unique`

### Bug #3: Map Editor Independence ✅ FIXED

Tests verify terrain and wall selections don't reset each other.

- `test_map_editor_terrain_wall_independence`

### Architecture Compliance ✅ VERIFIED

Tests verify .ron format is used (not .json or .yaml for game data).

- `test_ron_format_used_not_json`
- `test_all_data_files_use_ron_format`

## Performance

Integration tests are fast:
- **Bug verification tests**: ~10ms
- **Integration tests**: ~10ms
- **Total time**: < 100ms

Tests use temporary directories that are automatically cleaned up.

## Next Steps

After running tests successfully:

1. **Manual GUI Testing**: Follow `docs/how-to/test_campaign_builder_ui.md`
2. **Commit Changes**: All tests passing = safe to commit
3. **Pull Request**: CI will run tests automatically

## Related Documentation

- **Test Plan**: `docs/how-to/test_campaign_builder_ui.md` - Complete testing checklist
- **Bug Fixes**: `docs/explanation/implementations.md` - Summary of fixes
- **Architecture**: `docs/reference/architecture.md` - Data format specifications
- **AGENTS.md**: Development rules and quality gates

## Help

If tests fail unexpectedly:

1. Check that you're on the correct branch
2. Ensure all code is formatted (`cargo fmt --all`)
3. Read the test failure message carefully
4. Check `docs/how-to/test_campaign_builder_ui.md` for context
5. Review recent changes to source files

For persistent issues, review the implementation summary in `docs/explanation/implementations.md`.
