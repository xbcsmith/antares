#!/bin/bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# CSV-to-Vec Migration Validation Script
# Run after all phases complete to verify migration success

set -e

PROJECT_ROOT="/home/bsmith/go/src/github.com/xbcsmith/antares"
cd "$PROJECT_ROOT"

echo "======================================"
echo "CSV-to-Vec Migration Validation"
echo "======================================"
echo ""

# Phase 1: Check deliverables exist
echo "=== Phase 1: Deliverables Check ==="
test -f docs/explanation/csv_migration_inventory.md && echo "✓ CSV inventory exists" || exit 1
test -f docs/explanation/combobox_inventory.md && echo "✓ ComboBox inventory exists" || exit 1
test -f docs/explanation/csv_migration_checklist.md && echo "✓ Checklist exists" || exit 1
echo ""

# Phase 2: Check ui_helpers implementation
echo "=== Phase 2: UI Helpers Check ==="
grep -q "pub fn searchable_selector_single" antares/sdk/campaign_builder/src/ui_helpers.rs && echo "✓ searchable_selector_single exists" || exit 1
grep -q "pub fn searchable_selector_multi" antares/sdk/campaign_builder/src/ui_helpers.rs && echo "✓ searchable_selector_multi exists" || exit 1
grep -q "pub fn parse_id_csv_to_vec" antares/sdk/campaign_builder/src/ui_helpers.rs && echo "✓ parse_id_csv_to_vec exists" || exit 1
grep -q "pub fn format_vec_to_csv" antares/sdk/campaign_builder/src/ui_helpers.rs && echo "✓ format_vec_to_csv exists" || exit 1
echo ""

# Phase 3: Check core conversions
echo "=== Phase 3: Core Conversions Check ==="
grep -q "encounter_monsters: Vec<MonsterId>" antares/sdk/campaign_builder/src/map_editor.rs && echo "✓ EventEditorState converted" || exit 1
grep -q "starting_items: Vec<ItemId>" antares/sdk/campaign_builder/src/characters_editor.rs && echo "✓ CharacterEditBuffer converted" || exit 1
echo ""

# Phase 4: Check for unauthorized CSV usage
echo "=== Phase 4: CSV Elimination Check ==="
CSV_COUNT=$(grep -r "split.*['\"]," antares/sdk/campaign_builder/src/ | grep -v "test\|// Legitimate:" | wc -l)
if [ "$CSV_COUNT" -eq 0 ]; then
    echo "✓ No unauthorized CSV usage found"
else
    echo "✗ Found $CSV_COUNT unauthorized CSV usages"
    grep -rn "split.*['\"]," antares/sdk/campaign_builder/src/ | grep -v "test\|// Legitimate:"
    exit 1
fi
echo ""

# Code quality checks
echo "=== Code Quality Checks ==="
echo "Running cargo fmt..."
cargo fmt --all --check || exit 1
echo "✓ Code formatted correctly"

echo "Running cargo check..."
cargo check --all-targets --all-features --quiet || exit 1
echo "✓ Compilation successful"

echo "Running cargo clippy..."
cargo clippy --all-targets --all-features --quiet -- -D warnings || exit 1
echo "✓ No clippy warnings"

echo "Running cargo test..."
cargo test --all-features --quiet || exit 1
echo "✓ All tests pass"
echo ""

# Summary
echo "======================================"
echo "✓ ALL VALIDATIONS PASSED"
echo "======================================"
echo ""
echo "Migration complete! Summary:"
echo "- All CSV fields converted to typed vectors"
echo "- Unified searchable selectors implemented"
echo "- All tests passing"
echo "- Code quality verified"
echo ""
