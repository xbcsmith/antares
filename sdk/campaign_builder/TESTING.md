# Campaign Builder Testing Guide

**Quick Reference for Testing Campaign Builder**

## Run All Tests

```bash
cargo test
```

**Expected:** 287 tests pass (270 unit + 17 integration)

## Test Categories

### 1. Unit Tests (270 tests)
Located in `src/` files with `#[cfg(test)]` modules.

```bash
cargo test --lib
```

### 2. Integration Tests (9 tests)
Located in `tests/integration_tests.rs`.

```bash
cargo test --test integration_tests
```

### 3. Bug Verification Tests (10 tests, 2 ignored)
Located in `tests/bug_verification.rs`.

```bash
cargo test --test bug_verification
```

## Quality Gates (Run Before Commit)

```bash
# 1. Format
cargo fmt --all

# 2. Compile
cargo check --all-targets --all-features

# 3. Lint
cargo clippy --all-targets --all-features -- -D warnings

# 4. Test
cargo test --all-features
```

**All four must pass.**

## What Each Test Verifies

### Bug #1: Data Persistence ✅
- `test_items_persist_after_save`
- `test_monsters_persist_after_save`
- `test_campaign_save_load_roundtrip`

### Bug #2: UI ID Clashes ✅
- `test_no_ui_id_clashes`
- `test_items_tab_widget_ids_unique`
- `test_monsters_tab_widget_ids_unique`

### Bug #3: Map Editor Independence ✅
- `test_map_editor_terrain_wall_independence`

### Architecture Compliance ✅
- `test_ron_format_used_not_json`
- `test_all_data_files_use_ron_format`

## Debugging Tests

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test test_name
```

## CI/CD

Tests run automatically in GitHub Actions on pull requests.

## Documentation

- **Test Plan:** `docs/how-to/test_campaign_builder_ui.md`
- **Test Guide:** `docs/how-to/run_integration_tests.md`
- **Summary:** `docs/explanation/implementations.md`

## Quick Validation

```bash
# One-liner to check everything
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all-features
```

If all pass: **✅ Ready to commit**
