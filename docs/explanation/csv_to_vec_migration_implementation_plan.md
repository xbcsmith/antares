# CSV-to-Vec Migration Implementation Plan

**Document**: CSV-to-Vec Migration & ComboBox Unification  
**Version**: 3.0 (Unified AI-Optimized)  
**Project**: Antares Turn-Based RPG  
**Project Root**: `/home/bsmith/go/src/github.com/xbcsmith/antares`  
**Status**: Ready for Execution  
**Estimated Effort**: 15-25 person-hours across 6 phases  
**Last Updated**: 2025-01-13

---

## Document Summary

This implementation plan unifies and supersedes:

- `csv_to_vec_migration_plan.md` (Version 2.0 - detailed execution plan)
- `csv_migration_plan_improvements.md` (Version 2.0 - improvements summary)

**What's New in Version 3.0**:

- Restructured to follow PLAN.md template format
- Consolidated all content into single cohesive document
- Enhanced phase structure with standardized subsections
- Improved readability with clearer section hierarchy
- Retained all AI-optimization features (task IDs, validation commands, dependency matrix)
- Added comprehensive architectural decision documentation upfront
- Streamlined validation and success criteria per phase

**For AI Agents**: This is your primary implementation document. Follow phases sequentially, execute validation commands after each task, and update the progress tracking table as you work.

**For Human Developers**: Review the Architectural Decisions section first, then proceed phase-by-phase. Use the Quick Command Reference and Glossary as needed.

---

## Overview

This plan details the migration of all editor UI fields from comma-separated string (CSV) encoding to strongly-typed vectors (`Vec<ItemId>`, `Vec<MonsterId>`, `Vec<String>`). The migration includes standardizing selection UX by replacing ad-hoc `egui::ComboBox` usage with unified searchable selector functions in `ui_helpers`.

**Core Objectives**:

1. Eliminate CSV string parsing from SDK codebase
2. Introduce type-safe vector-based list fields
3. Unify selection UI with searchable selectors (single & multi-select)
4. Improve data validation and testability
5. Reduce technical debt and brittle string parsing logic

**Rationale**: CSV string fields are error-prone, difficult to validate, and lead to format bugs. Converting to typed vectors improves type safety, simplifies UI code, maintains consistency with domain types, and enables clearer validation semantics.

---

## Current State Analysis

### Existing Infrastructure

The Antares SDK (`antares/sdk/campaign_builder/`) currently implements editor UIs for game content creation:

- **Map Editor** (`map_editor.rs`): Handles event creation with monster encounters and treasure items stored as CSV strings
- **Character Editor** (`characters_editor.rs`): Manages character starting items and equipment as CSV strings
- **Class Editor** (`classes_editor.rs`): Stores proficiencies and disablements as CSV strings
- **Item Editor** (`items_editor.rs`): Handles item tags and effects as CSV strings
- **Spell Editor** (`spells_editor.rs`): Manages spell effects and targets as CSV strings

**Current Pattern**:

```rust
pub struct EventEditorState {
    pub encounter_monsters: String,  // "1,2,5,10"
    pub treasure_items: String,      // "42,43,50"
}
```

**UI Code Pattern**:

```rust
// Manual CSV parsing scattered throughout codebase
let monster_ids: Vec<u32> = event.encounter_monsters
    .split(',')
    .filter(|s| !s.is_empty())
    .filter_map(|s| s.trim().parse().ok())
    .collect();
```

### Identified Issues

1. **Type Safety**: String fields accept any content; invalid IDs only discovered at parse time
2. **Parsing Duplication**: CSV parsing logic duplicated across 15+ locations
3. **Error Handling**: Inconsistent handling of malformed CSV (empty strings, whitespace, invalid IDs)
4. **UI Inconsistency**: ComboBox widgets implemented differently in each editor
5. **Testing Difficulty**: String fields make unit testing complex; need test data in CSV format
6. **Validation Gaps**: No validation that referenced IDs exist in game database
7. **Maintenance Burden**: Changes to CSV format require updates in multiple places
8. **Domain Model Drift**: Editor buffer types (String) don't match domain types (Vec)

---

## Architectural Decisions (Resolved)

### ADR-001: Convert ALL List Fields to Typed Vectors

**Decision**: Convert every list-like string field to strongly-typed vector

**Examples**:

- `proficiencies: String` → `proficiencies: Vec<String>` or `Vec<ProficiencyId>`
- `tags: String` → `tags: Vec<String>`
- `encounter_monsters: String` → `encounter_monsters: Vec<MonsterId>`
- `starting_items: String` → `starting_items: Vec<ItemId>`

**Rationale**:

- Eliminates parsing errors
- Provides compile-time type safety
- Simplifies serialization/deserialization
- Aligns editor buffers with domain model types

**Trade-offs**:

- Backward incompatible with existing CSV-based saved files
- Requires migration for existing content (acceptable for pre-1.0 project)

### ADR-002: Implement Two Specialized Selector Functions

**Decision**: Create two separate UI helper functions in `ui_helpers.rs`:

1. `searchable_selector_single<T, ID>()` - Single selection with search (replaces ComboBox)
2. `searchable_selector_multi<T, ID>()` - Multi-selection with chips + search

**Rationale**:

- Single-select and multi-select have fundamentally different UX patterns
- Specialized APIs are more ergonomic than one generic function
- Clearer function signatures reduce cognitive load

**Alternative Rejected**: One generic `searchable_selector<T, ID>(mode: SelectionMode)`

- Would require complex enum matching in UI code
- Less discoverable API

### ADR-003: No Backward Compatibility

**Decision**: Do NOT maintain backward compatibility with CSV format in saved files

**Rationale**:

- Cleaner codebase without dual-format support
- Project is pre-1.0; breaking changes acceptable
- Reduces complexity and technical debt
- Migration tool can be added post-migration if needed

**Mitigation**: Optional standalone migration utility can convert old saves (future enhancement)

---

## Implementation Phases

### Phase 1: Discovery & Inventory

**Phase ID**: PHASE-01  
**Estimated Effort**: 4-6 person-hours  
**Prerequisites**: None  
**Dependencies**: None  
**Blocks**: PHASE-02

#### 1.1 Catalog CSV Usage

**Task ID**: TASK-01.01

**Objective**: Identify all CSV string fields in SDK requiring conversion

**Actions**:

1. Search for CSV parsing patterns in `sdk/campaign_builder/src/`:

   ```bash
   cd /home/bsmith/go/src/github.com/xbcsmith/antares
   grep -rn "split\s*(\s*\",\"" sdk/campaign_builder/src/
   grep -rn "split\s*(\s*',')" sdk/campaign_builder/src/
   grep -rn "\.join\s*(\s*\",\"" sdk/campaign_builder/src/
   ```

2. Document each occurrence in CSV inventory table:

   ```markdown
   | File Path | Line | Struct | Field | Current Type | Target Type | Priority |
   | --------- | ---- | ------ | ----- | ------------ | ----------- | -------- |
   ```

3. Classify by priority:
   - HIGH: Core editor buffers (EventEditorState, CharacterEditBuffer)
   - MEDIUM: Secondary editors (ClassEditBuffer, ItemEditBuffer)
   - LOW: Utility fields or rarely-used features

**Expected Discoveries** (estimated):

- `map_editor.rs`: EventEditorState (encounter_monsters, treasure_items)
- `characters_editor.rs`: CharacterEditBuffer (starting_items, weapon_id, armor_id)
- `classes_editor.rs`: ClassEditBuffer (proficiencies, disablements)
- `items_editor.rs`: Item tags and effects
- `spells_editor.rs`: Spell effects and targeting

#### 1.2 Catalog ComboBox Usage

**Task ID**: TASK-01.02

**Objective**: Find all `egui::ComboBox` usage for replacement with unified selectors

**Actions**:

1. Search for ComboBox patterns:

   ```bash
   grep -rn "ComboBox::" sdk/campaign_builder/src/
   grep -rn "egui::ComboBox" sdk/campaign_builder/src/
   ```

2. Document in inventory table:

   ```markdown
   | File Path | Line | Context | Selection Type | Replacement Function |
   | --------- | ---- | ------- | -------------- | -------------------- |
   ```

3. Classify as single-select or multi-select

**Expected Count**: 10-15 ComboBox instances

#### 1.3 Create Refactor Checklist

**Task ID**: TASK-01.03  
**Dependencies**: TASK-01.01, TASK-01.02

**Objective**: Generate prioritized task list from inventories

**Actions**:

1. Merge CSV and ComboBox inventories
2. Sort by phase priority:
   - Phase 2: UI helper prerequisites
   - Phase 3: High priority conversions (core editors)
   - Phase 4: Medium priority conversions
   - Phase 5: Low priority / remaining
3. Create markdown checklist with checkboxes

**Deliverable**: `CSV-to-Vec Migration Checklist` (embedded in this document or separate file)

#### 1.4 Testing Requirements

**Validation Commands**:

```bash
# Verify inventories created
test -f docs/explanation/csv_migration_inventory.md || echo "FAIL: Inventory not found"

# Verify CSV patterns documented
grep -c "split.*',' *)" docs/explanation/csv_migration_inventory.md
test $? -eq 0 || echo "FAIL: No CSV patterns documented"

# Verify ComboBox patterns documented
grep -c "ComboBox::" docs/explanation/csv_migration_inventory.md
test $? -eq 0 || echo "FAIL: No ComboBox patterns documented"
```

#### 1.5 Deliverables

- [ ] CSV usage inventory table (markdown format)
- [ ] ComboBox usage inventory table (markdown format)
- [ ] Prioritized refactor checklist
- [ ] Phase dependencies documented

#### 1.6 Success Criteria

- [ ] All CSV fields identified with file paths and line numbers
- [ ] All ComboBox instances cataloged
- [ ] Checklist contains 15-25 conversion tasks
- [ ] Tasks prioritized HIGH/MEDIUM/LOW
- [ ] No grep patterns return new results after inventory complete

---

### Phase 2: UI Helper Foundation

**Phase ID**: PHASE-02  
**Estimated Effort**: 6-8 person-hours  
**Prerequisites**: PHASE-01 complete  
**Dependencies**: TASK-01.03  
**Blocks**: PHASE-03

#### 2.1 Implement searchable_selector_single

**Task ID**: TASK-02.01  
**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`

**Objective**: Create single-selection searchable dropdown widget

**Function Signature**:

```rust
pub fn searchable_selector_single<T, ID>(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_id: &mut Option<ID>,
    available_items: &[(ID, T)],
    display_fn: impl Fn(&T) -> String,
) -> egui::Response
where
    ID: Copy + Eq + std::fmt::Display,
    T: Clone,
```

**Implementation Details**:

1. Store search query in `egui::Memory` keyed by `id_salt`
2. Render search TextEdit widget
3. Filter `available_items` by search query (case-insensitive)
4. Render filtered results as selectable list
5. Update `selected_id` on click
6. Return combined Response for UI chaining

**Key Features**:

- Live search filtering
- Keyboard navigation support
- Persistent search state across frames
- Scrollable results list

#### 2.2 Implement searchable_selector_multi

**Task ID**: TASK-02.02  
**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`

**Objective**: Create multi-selection widget with chips UI

**Function Signature**:

```rust
pub fn searchable_selector_multi<T, ID>(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_ids: &mut Vec<ID>,
    available_items: &[(ID, T)],
    display_fn: impl Fn(&T) -> String,
) -> egui::Response
where
    ID: Copy + Eq + std::fmt::Display,
    T: Clone,
```

**Implementation Details**:

1. Render selected items as removable chips (horizontal wrap)
2. Render search input below chips
3. Filter available items (exclude already selected)
4. Render filtered results as selectable list
5. Add to `selected_ids` on click
6. Remove from `selected_ids` on chip X button

**Key Features**:

- Visual chips for selected items
- Remove button (X) on each chip
- Search excludes already-selected items
- Maintains selection order

#### 2.3 Add Conversion Helpers

**Task ID**: TASK-02.03  
**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`  
**Dependencies**: TASK-02.01, TASK-02.02

**Objective**: Provide utilities for temporary CSV-to-Vec conversion during migration

**Functions**:

```rust
/// Parse CSV string to Vec of IDs (migration helper)
pub fn parse_id_csv_to_vec<ID>(csv: &str) -> Vec<ID>
where
    ID: std::str::FromStr,
{
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Format Vec of IDs to CSV string (for backward compat testing)
pub fn format_vec_to_csv<ID>(ids: &[ID]) -> String
where
    ID: std::fmt::Display,
{
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
```

**Note**: These are temporary utilities for migration; remove after PHASE-06

#### 2.4 Write ui_helpers Tests

**Task ID**: TASK-02.04  
**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`  
**Dependencies**: TASK-02.01, TASK-02.02, TASK-02.03

**Objective**: Achieve 80%+ test coverage for new functions

**Test Cases**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_id_csv_to_vec_simple() {
        assert_eq!(parse_id_csv_to_vec::<u32>("1,2,3"), vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_id_csv_to_vec_empty() {
        assert_eq!(parse_id_csv_to_vec::<u32>(""), Vec::<u32>::new());
    }

    #[test]
    fn test_parse_id_csv_to_vec_whitespace() {
        assert_eq!(parse_id_csv_to_vec::<u32>("1 , 2 , 3"), vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_id_csv_to_vec_invalid() {
        // Invalid entries should be filtered out
        assert_eq!(parse_id_csv_to_vec::<u32>("1,abc,3"), vec![1, 3]);
    }

    #[test]
    fn test_format_vec_to_csv_simple() {
        assert_eq!(format_vec_to_csv(&[1u32, 2, 3]), "1,2,3");
    }

    #[test]
    fn test_format_vec_to_csv_empty() {
        assert_eq!(format_vec_to_csv(&Vec::<u32>::new()), "");
    }

    // Note: UI function tests require egui test harness (optional)
}
```

**Coverage Target**: 80%+ for helper functions

#### 2.5 Testing Requirements

**Validation Commands**:

```bash
# Verify functions exist
grep -q "pub fn searchable_selector_single" sdk/campaign_builder/src/ui_helpers.rs || exit 1
grep -q "pub fn searchable_selector_multi" sdk/campaign_builder/src/ui_helpers.rs || exit 1
grep -q "pub fn parse_id_csv_to_vec" sdk/campaign_builder/src/ui_helpers.rs || exit 1

# Run quality checks
cargo fmt --all
cargo check --package campaign_builder
cargo clippy --package campaign_builder -- -D warnings
cargo test --package campaign_builder ui_helpers::tests
```

#### 2.6 Deliverables

- [ ] `searchable_selector_single` function implemented
- [ ] `searchable_selector_multi` function implemented
- [ ] `parse_id_csv_to_vec` helper implemented
- [ ] `format_vec_to_csv` helper implemented
- [ ] Test suite with 8+ test cases
- [ ] All tests passing
- [ ] Clippy warnings resolved

#### 2.7 Success Criteria

- [ ] `cargo test --package campaign_builder ui_helpers::tests` passes
- [ ] `cargo clippy --package campaign_builder` returns 0 warnings
- [ ] Functions documented with `///` doc comments and examples
- [ ] Test coverage ≥80% for helper functions

---

### Phase 3: Core Editor Conversions

**Phase ID**: PHASE-03  
**Estimated Effort**: 5-8 person-hours  
**Prerequisites**: PHASE-02 complete  
**Dependencies**: TASK-02.04  
**Blocks**: PHASE-04

#### 3.1 Convert Map Editor (EventEditorState)

**Task ID**: TASK-03.01  
**File**: `antares/sdk/campaign_builder/src/map_editor.rs`  
**Dependencies**: TASK-02.04

**Sub-Tasks**:

##### 3.1.A: Modify EventEditorState Structure

**Current** (~line 690):

```rust
pub struct EventEditorState {
    pub event_type: String,
    pub position: (i32, i32),
    pub name: String,
    pub description: String,
    pub encounter_monsters: String,  // CSV
    pub treasure_items: String,      // CSV
}
```

**Target**:

```rust
pub struct EventEditorState {
    pub event_type: String,
    pub position: (i32, i32),
    pub name: String,
    pub description: String,
    pub encounter_monsters: Vec<MonsterId>,  // Typed vector
    pub treasure_items: Vec<ItemId>,         // Typed vector
}
```

##### 3.1.B: Update EventEditorState::default()

**Current**:

```rust
fn default() -> Self {
    Self {
        encounter_monsters: String::new(),
        treasure_items: String::new(),
        // ...
    }
}
```

**Target**:

```rust
fn default() -> Self {
    Self {
        encounter_monsters: Vec::new(),
        treasure_items: Vec::new(),
        // ...
    }
}
```

##### 3.1.C: Update UI Rendering

Replace CSV parsing + ComboBox with searchable selectors:

**Current Pattern**:

```rust
let monster_ids: Vec<u32> = event.encounter_monsters
    .split(',')
    .filter_map(|s| s.trim().parse().ok())
    .collect();

egui::ComboBox::from_label("Monster")
    .show_ui(ui, |ui| { /* ... */ });
```

**Target Pattern**:

```rust
use crate::ui_helpers::searchable_selector_multi;

searchable_selector_multi(
    ui,
    "event_encounter_monsters",
    "Encounter Monsters",
    &mut event.encounter_monsters,
    &available_monsters,
    |monster| monster.name.clone(),
);
```

##### 3.1.D: Update to_map_event() Conversion

**Current**:

```rust
fn to_map_event(&self) -> MapEvent {
    MapEvent {
        encounter_monsters: self.encounter_monsters
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect(),
        // ...
    }
}
```

**Target**:

```rust
fn to_map_event(&self) -> MapEvent {
    MapEvent {
        encounter_monsters: self.encounter_monsters.clone(),
        // ...
    }
}
```

##### 3.1.E: Update Tests

Update test fixtures to use Vec instead of CSV strings:

**Current**:

```rust
#[test]
fn test_event_with_monsters() {
    let event = EventEditorState {
        encounter_monsters: "1,2,5".to_string(),
        ..Default::default()
    };
}
```

**Target**:

```rust
#[test]
fn test_event_with_monsters() {
    let event = EventEditorState {
        encounter_monsters: vec![1, 2, 5],
        ..Default::default()
    };
}
```

**Validation Commands**:

```bash
# Verify struct change
grep "pub encounter_monsters: Vec<MonsterId>" sdk/campaign_builder/src/map_editor.rs || exit 1

# Verify no CSV parsing remains
! grep -n "split.*',' *)" sdk/campaign_builder/src/map_editor.rs | grep "encounter_monsters"

# Run tests
cargo test --package campaign_builder map_editor::tests
```

#### 3.2 Convert Character Editor (CharacterEditBuffer)

**Task ID**: TASK-03.02  
**File**: `antares/sdk/campaign_builder/src/characters_editor.rs`  
**Dependencies**: TASK-02.04

**Sub-Tasks**: Similar pattern to TASK-03.01

##### 3.2.A: Modify CharacterEditBuffer Structure

**Current**:

```rust
pub struct CharacterEditBuffer {
    pub starting_items: String,  // CSV
    pub weapon_id: String,       // CSV (single item, but stored as CSV)
    pub armor_id: String,
}
```

**Target**:

```rust
pub struct CharacterEditBuffer {
    pub starting_items: Vec<ItemId>,
    pub weapon_id: Option<ItemId>,  // Single selection
    pub armor_id: Option<ItemId>,
}
```

##### 3.2.B: Update default() Implementation

##### 3.2.C: Update UI Rendering

Use `searchable_selector_multi` for starting_items, `searchable_selector_single` for weapon/armor

##### 3.2.D: Update Save/Apply Functions

##### 3.2.E: Update Tests

**Validation Commands**:

```bash
grep "pub starting_items: Vec<ItemId>" sdk/campaign_builder/src/characters_editor.rs || exit 1
! grep -n "split.*',' *)" sdk/campaign_builder/src/characters_editor.rs
cargo test --package campaign_builder characters_editor::tests
```

#### 3.3 Convert Other Editors

**Task ID**: TASK-03.03  
**Files**: `classes_editor.rs`, `items_editor.rs`, `spells_editor.rs`  
**Dependencies**: TASK-02.04

**Targets**:

- `ClassEditBuffer.proficiencies: String` → `Vec<String>` or `Vec<ProficiencyId>`
- `ClassEditBuffer.disablements: String` → `Vec<String>`
- `ItemEditBuffer.tags: String` → `Vec<String>`
- `SpellEditBuffer.effects: String` → `Vec<String>`

**Approach**: Repeat pattern from TASK-03.01 for each editor

**Validation Commands**:

```bash
# Verify all editors converted
! grep -rn "split.*',' *)" sdk/campaign_builder/src/classes_editor.rs
! grep -rn "split.*',' *)" sdk/campaign_builder/src/items_editor.rs
! grep -rn "split.*',' *)" sdk/campaign_builder/src/spells_editor.rs

# Run all editor tests
cargo test --package campaign_builder
```

#### 3.4 Testing Requirements

**Validation Script**:

```bash
#!/bin/bash
# Run from project root

echo "=== Phase 3 Validation ==="

# Check all core editors converted
EDITORS=(
    "map_editor"
    "characters_editor"
    "classes_editor"
    "items_editor"
    "spells_editor"
)

for editor in "${EDITORS[@]}"; do
    echo "Checking $editor.rs..."
    if grep -q "split.*',' *)" "sdk/campaign_builder/src/${editor}.rs"; then
        echo "FAIL: CSV parsing still exists in ${editor}.rs"
        exit 1
    fi
done

# Run quality checks
cargo fmt --all
cargo check --package campaign_builder || exit 1
cargo clippy --package campaign_builder -- -D warnings || exit 1
cargo test --package campaign_builder || exit 1

echo "✓ Phase 3 validation passed"
```

#### 3.5 Deliverables

- [ ] EventEditorState converted to Vec fields
- [ ] CharacterEditBuffer converted to Vec fields
- [ ] ClassEditBuffer converted to Vec fields
- [ ] ItemEditBuffer converted to Vec fields (tags)
- [ ] SpellEditBuffer converted to Vec fields (effects)
- [ ] All UI rendering uses searchable selectors
- [ ] All tests updated and passing
- [ ] Zero CSV parsing in editor files

#### 3.6 Success Criteria

- [ ] `cargo test --package campaign_builder` passes (all tests)
- [ ] No grep results for `split.*','` in editor files
- [ ] All ComboBox replaced with searchable selectors in core editors
- [ ] Test coverage maintained or improved (≥70%)
- [ ] `cargo clippy --package campaign_builder` returns 0 warnings

---

### Phase 4: Complete Sweep & Unification

**Phase ID**: PHASE-04  
**Estimated Effort**: 3-5 person-hours  
**Prerequisites**: PHASE-03 complete  
**Dependencies**: TASK-03.01, TASK-03.02, TASK-03.03  
**Blocks**: PHASE-05

#### 4.1 Sweep Remaining CSV Fields

**Task ID**: TASK-04.01  
**Dependencies**: TASK-03.03

**Objective**: Find and convert any remaining CSV fields missed in Phase 3

**Actions**:

1. Run comprehensive search:

   ```bash
   grep -rn "\.split\s*(" sdk/campaign_builder/src/ | grep -v "test" | grep -v "//"
   grep -rn "\.join\s*(" sdk/campaign_builder/src/ | grep -v "test" | grep -v "//"
   ```

2. For each result:

   - Determine if it's legitimate CSV usage (e.g., file paths, display formatting)
   - If internal data structure, convert to Vec
   - Document decision in sweep report

3. Update sweep report with:
   - Remaining legitimate CSV usage (with justification)
   - Converted fields
   - Summary statistics

**Validation Commands**:

```bash
# Verify sweep completed
test -f docs/explanation/csv_sweep_report.md || exit 1

# Verify minimal CSV usage remains
CSV_COUNT=$(grep -rn "\.split\s*(" sdk/campaign_builder/src/ | grep -v "test" | grep -v "//" | wc -l)
test $CSV_COUNT -lt 5 || echo "WARN: $CSV_COUNT CSV usages remain"
```

#### 4.2 Unify All ComboBox Usage

**Task ID**: TASK-04.02  
**Dependencies**: TASK-03.03

**Objective**: Replace all remaining ComboBox instances with searchable selectors

**Actions**:

1. Search for remaining ComboBox usage:

   ```bash
   grep -rn "ComboBox::" sdk/campaign_builder/src/
   ```

2. For each instance:

   - Determine if single or multi-select
   - Replace with appropriate searchable selector
   - Update UI code and tests

3. Document exceptions (if any) in unification report

**Validation Commands**:

```bash
# Verify minimal ComboBox usage
COMBO_COUNT=$(grep -rn "ComboBox::" sdk/campaign_builder/src/ | wc -l)
test $COMBO_COUNT -lt 3 || echo "WARN: $COMBO_COUNT ComboBox instances remain"

# Verify searchable selectors used
SELECTOR_COUNT=$(grep -rn "searchable_selector" sdk/campaign_builder/src/ | wc -l)
test $SELECTOR_COUNT -gt 10 || echo "WARN: Only $SELECTOR_COUNT selector usages found"
```

#### 4.3 Testing Requirements

**Validation Commands**:

```bash
# Full SDK test suite
cargo test --package campaign_builder --all-features

# Quality checks
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

#### 4.4 Deliverables

- [ ] CSV sweep report documenting remaining usage
- [ ] ComboBox unification report
- [ ] All unjustified CSV usage eliminated
- [ ] All ComboBox replaced (except documented exceptions)
- [ ] Updated tests passing

#### 4.5 Success Criteria

- [ ] CSV usage ≤5 instances (all justified and documented)
- [ ] ComboBox usage ≤3 instances (all justified and documented)
- [ ] `cargo test --package campaign_builder` passes
- [ ] No clippy warnings
- [ ] Sweep and unification reports completed

---

### Phase 5: Engine Integration & Domain Model

**Phase ID**: PHASE-05  
**Estimated Effort**: 2-4 person-hours  
**Prerequisites**: PHASE-04 complete  
**Dependencies**: TASK-04.01, TASK-04.02  
**Blocks**: PHASE-06

#### 5.1 Verify Domain Model Consistency

**Task ID**: TASK-05.01  
**Dependencies**: TASK-04.01, TASK-04.02

**Objective**: Ensure domain types (`antares/domain/src/`) match editor buffer types

**Actions**:

1. Compare editor buffer structs with domain structs:

   ```bash
   # Example: Check Character type
   grep "pub starting_items:" sdk/campaign_builder/src/characters_editor.rs
   grep "pub starting_items:" domain/src/character.rs
   ```

2. Verify type consistency:

   - EventEditorState → MapEvent
   - CharacterEditBuffer → Character
   - ClassEditBuffer → Class
   - ItemEditBuffer → Item
   - SpellEditBuffer → Spell

3. Update domain models if mismatches found

**Validation Commands**:

```bash
# Verify domain types use Vec
grep -rn "Vec<ItemId>" domain/src/
grep -rn "Vec<MonsterId>" domain/src/

# Ensure no CSV in domain layer
! grep -rn "split.*',' *)" domain/src/

# Domain tests pass
cargo test --package domain
```

#### 5.2 Update Engine Consumers

**Task ID**: TASK-05.02  
**Dependencies**: TASK-04.01, TASK-04.02

**Objective**: Verify game engine correctly handles Vec-based data

**Actions**:

1. Check engine code (`antares/engine/src/`) for CSV assumptions
2. Update any CSV parsing in engine layer
3. Verify save/load functionality with new format
4. Test game runtime with converted data

**Validation Commands**:

```bash
# Engine has no CSV parsing
! grep -rn "split.*',' *)" engine/src/

# Engine tests pass
cargo test --package engine
```

#### 5.3 Verify Packager and Export Tools

**Task ID**: TASK-05.03  
**Dependencies**: TASK-05.01, TASK-05.02

**Objective**: Ensure campaign export/package tools work with Vec format

**Actions**:

1. Test campaign export with converted data
2. Verify RON serialization handles Vec types
3. Test campaign import (if applicable)

**Validation Commands**:

```bash
# Packager builds
cargo check --package campaign_packager

# Integration tests pass
cargo test --all-features
```

#### 5.4 Testing Requirements

**Full Integration Tests**:

```bash
# Build all packages
cargo build --all-targets

# Run all tests
cargo test --all-features

# Run game in test mode (if applicable)
# cargo run --example test_campaign
```

#### 5.5 Deliverables

- [ ] Domain model types verified consistent
- [ ] Engine layer updated (if needed)
- [ ] Packager tools tested
- [ ] Integration tests passing
- [ ] End-to-end workflow validated

#### 5.6 Success Criteria

- [ ] `cargo test --all-features` passes
- [ ] Domain and editor buffer types consistent
- [ ] Campaign export/import works with Vec format
- [ ] No CSV parsing in domain or engine layers
- [ ] Full game workflow functional

---

### Phase 6: Documentation & Final Validation

**Phase ID**: PHASE-06  
**Estimated Effort**: 2-4 person-hours  
**Prerequisites**: PHASE-05 complete  
**Dependencies**: TASK-05.01, TASK-05.02, TASK-05.03  
**Blocks**: None (final phase)

#### 6.1 Update Documentation

**Task ID**: TASK-06.01  
**Dependencies**: TASK-05.03

**Sub-Tasks**:

##### 6.1.A: Update implementations.md

**File**: `antares/docs/explanation/implementations.md`

**Add Section**:

```markdown
## CSV-to-Vec Migration (2025-01)

**Summary**: Migrated all editor UI fields from CSV string encoding to typed vectors.

**Changes**:

- Converted 15+ CSV fields to `Vec<ItemId>`, `Vec<MonsterId>`, `Vec<String>`
- Implemented searchable selector UI helpers (single & multi-select)
- Replaced all ComboBox instances with unified searchable selectors
- Eliminated CSV parsing from SDK codebase

**Files Modified**:

- `sdk/campaign_builder/src/ui_helpers.rs`: Added searchable selector functions
- `sdk/campaign_builder/src/map_editor.rs`: Converted EventEditorState
- `sdk/campaign_builder/src/characters_editor.rs`: Converted CharacterEditBuffer
- `sdk/campaign_builder/src/classes_editor.rs`: Converted ClassEditBuffer
- `sdk/campaign_builder/src/items_editor.rs`: Converted tags
- `sdk/campaign_builder/src/spells_editor.rs`: Converted effects

**Testing**: All tests passing, 80%+ coverage for ui_helpers

**Backward Compatibility**: BREAKING - CSV format no longer supported
```

##### 6.1.B: Create How-To Guide (Optional)

**File**: `antares/docs/how-to/use_searchable_selectors.md`

**Content**:

```markdown
# How to Use Searchable Selectors

## Single Selection

Use `searchable_selector_single` for selecting one item from a list...

## Multi Selection

Use `searchable_selector_multi` for selecting multiple items...
```

#### 6.2 Final QA and Validation

**Task ID**: TASK-06.02  
**Dependencies**: TASK-06.01

**Sub-Tasks**:

##### 6.2.A: Run Automated Validation

**Script**: `scripts/validate_csv_migration.sh`

```bash
#!/bin/bash
set -e

echo "=== CSV-to-Vec Migration Validation ==="

# Phase 1: Verify inventories exist
test -f docs/explanation/csv_migration_inventory.md || exit 1

# Phase 2: Verify UI helpers exist
grep -q "pub fn searchable_selector_single" sdk/campaign_builder/src/ui_helpers.rs || exit 1
grep -q "pub fn searchable_selector_multi" sdk/campaign_builder/src/ui_helpers.rs || exit 1

# Phase 3: Verify core editors converted
! grep -rn "split.*',' *)" sdk/campaign_builder/src/map_editor.rs | grep "encounter_monsters"
! grep -rn "split.*',' *)" sdk/campaign_builder/src/characters_editor.rs | grep "starting_items"

# Phase 4: Verify minimal CSV usage
CSV_COUNT=$(grep -rn "\.split\s*(" sdk/campaign_builder/src/ | grep -v "test" | grep -v "//" | wc -l)
if [ $CSV_COUNT -gt 5 ]; then
    echo "FAIL: Too many CSV usages remain: $CSV_COUNT"
    exit 1
fi

# Phase 5: Verify domain consistency
grep -q "Vec<ItemId>" domain/src/character.rs || echo "WARN: Domain may need updates"

# Phase 6: Quality checks
cargo fmt --all
cargo check --all-targets --all-features || exit 1
cargo clippy --all-targets --all-features -- -D warnings || exit 1
cargo test --all-features || exit 1

echo "✅ All validations passed"
```

##### 6.2.B: Manual QA Checklist

**QA Tasks**:

- [ ] Open campaign builder UI
- [ ] Create new event with monsters (multi-select)
- [ ] Create new character with starting items (multi-select)
- [ ] Select character weapon (single-select)
- [ ] Verify search functionality works
- [ ] Verify chips render correctly
- [ ] Save and reload campaign
- [ ] Export campaign package
- [ ] Load campaign in game engine

##### 6.2.C: Performance Testing

**Metrics**:

- [ ] UI responsiveness with 100+ item lists
- [ ] Search filtering latency (<50ms)
- [ ] Memory usage comparable to pre-migration

#### 6.3 Testing Requirements

**Final Validation**:

```bash
# Run validation script
bash scripts/validate_csv_migration.sh

# Verify documentation updated
grep -q "CSV-to-Vec Migration" docs/explanation/implementations.md || exit 1

# All tests pass
cargo test --all-features
```

#### 6.4 Deliverables

- [ ] `implementations.md` updated with migration summary
- [ ] How-to guide created (optional)
- [ ] Validation script created and passing
- [ ] QA checklist completed
- [ ] Performance metrics documented
- [ ] Migration marked complete in progress tracking table

#### 6.5 Success Criteria

- [ ] `scripts/validate_csv_migration.sh` exits with code 0
- [ ] All manual QA tasks completed successfully
- [ ] Documentation updated
- [ ] No regressions in functionality
- [ ] Performance acceptable
- [ ] Stakeholder sign-off obtained

---

## Risk Assessment & Mitigation

### RISK-01: Test Failures During Migration

**Probability**: Medium (30%)  
**Impact**: 2-4 hours delay

**Symptoms**:

- Tests fail after struct changes
- Serialization tests break
- UI tests fail with new widget types

**Mitigation**:

1. Update tests incrementally as structs change
2. Use `parse_id_csv_to_vec` helper for test fixture migration
3. Run tests after each sub-task
4. Keep CSV-to-Vec conversion helpers temporarily

**Detection**:

```bash
cargo test --package campaign_builder
```

**Rollback**: Revert individual file if tests fail unrecoverably

### RISK-02: Engine Expectation Mismatch

**Probability**: Low (15%)  
**Impact**: 4-6 hours delay

**Symptoms**:

- Game engine expects CSV format
- Save/load functionality breaks
- Runtime errors with Vec serialization

**Mitigation**:

1. Audit engine code in PHASE-05
2. Verify RON serialization handles Vec types
3. Test save/load before finalizing migration
4. Update domain types first if mismatches found

**Detection**:

```bash
grep -rn "split.*',' *)" engine/src/
cargo test --package engine
```

**Rollback**: Revert domain changes, keep editor buffer changes temporarily

### RISK-03: UI Performance Degradation

**Probability**: Low (10%)  
**Impact**: 2-4 hours optimization

**Symptoms**:

- Searchable selectors laggy with large lists
- Chip rendering slow with many selections
- Memory usage increases

**Mitigation**:

1. Test with realistic data sizes (100+ items)
2. Implement list virtualization if needed
3. Cache filtered results per frame
4. Profile UI code if issues arise

**Detection**: Manual testing with large lists

**Rollback**: Keep new data types, optimize or revert UI widgets

### RISK-04: Breaking Changes in Saved Data

**Probability**: High (70%)  
**Impact**: User impact (acceptable for pre-1.0)

**Symptoms**:

- Existing campaign files won't load
- Deserialization errors
- Data loss for users

**Mitigation**:

1. Document breaking change prominently
2. Provide migration tool (future enhancement)
3. Version campaign file format
4. Test with sample campaign files

**Detection**: Load existing campaign files after migration

**Rollback**: Not applicable (accept breaking change as per ADR-003)

---

## Rollback Procedures by Phase

### PHASE-01 Rollback

**Risk Level**: Very Low (no code changes)

**Procedure**:

```bash
# Delete inventory files if needed
rm -f docs/explanation/csv_migration_inventory.md
rm -f docs/explanation/csv_migration_checklist.md
```

### PHASE-02 Rollback

**Risk Level**: Low (isolated changes)

**Procedure**:

```bash
# Revert ui_helpers.rs
git checkout main -- sdk/campaign_builder/src/ui_helpers.rs

# Verify compilation
cargo check --package campaign_builder
cargo test --package campaign_builder

# Clean up
git status
```

### PHASE-03 Rollback

**Risk Level**: Medium (multiple files)

**Procedure**:

```bash
# Revert editor files
git checkout main -- sdk/campaign_builder/src/map_editor.rs
git checkout main -- sdk/campaign_builder/src/characters_editor.rs
git checkout main -- sdk/campaign_builder/src/classes_editor.rs

# Verify compilation
cargo check --package campaign_builder
cargo test --package campaign_builder
```

### PHASE-04+ Rollback

**Risk Level**: High (widespread changes)

**Procedure**:

```bash
# Full rollback to migration start
git log --oneline | grep "Start CSV migration"
git reset --hard <commit-hash>

# Or revert merge
git revert -m 1 <merge-commit>

# Verify
cargo check --all-targets
cargo test --all-features
```

---

## Task Dependency Matrix

| Task ID    | Description                 | Depends On          | Blocks     | Parallel With |
| ---------- | --------------------------- | ------------------- | ---------- | ------------- |
| TASK-01.01 | Catalog CSV usage           | None                | TASK-01.03 | TASK-01.02    |
| TASK-01.02 | Catalog ComboBox usage      | None                | TASK-01.03 | TASK-01.01    |
| TASK-01.03 | Create refactor checklist   | 01.01, 01.02        | PHASE-02   | -             |
| TASK-02.01 | Implement selector_single   | 01.03               | PHASE-03   | TASK-02.02    |
| TASK-02.02 | Implement selector_multi    | 01.03               | PHASE-03   | TASK-02.01    |
| TASK-02.03 | Add conversion helpers      | 02.01, 02.02        | PHASE-03   | -             |
| TASK-02.04 | Write ui_helpers tests      | 02.01, 02.02, 02.03 | PHASE-03   | -             |
| TASK-03.01 | Convert EventEditorState    | 02.04               | PHASE-04   | 03.02, 03.03  |
| TASK-03.02 | Convert CharacterEditBuffer | 02.04               | PHASE-04   | 03.01, 03.03  |
| TASK-03.03 | Convert other editors       | 02.04               | PHASE-04   | 03.01, 03.02  |
| TASK-04.01 | Sweep remaining CSV         | 03.01, 03.02, 03.03 | PHASE-05   | TASK-04.02    |
| TASK-04.02 | Unify ComboBox usage        | 03.01, 03.02, 03.03 | PHASE-05   | TASK-04.01    |
| TASK-05.01 | Verify domain consistency   | 04.01, 04.02        | PHASE-06   | TASK-05.02    |
| TASK-05.02 | Update engine consumers     | 04.01, 04.02        | PHASE-06   | TASK-05.01    |
| TASK-05.03 | Verify packager tools       | 05.01, 05.02        | PHASE-06   | -             |
| TASK-06.01 | Update documentation        | 05.03               | -          | TASK-06.02    |
| TASK-06.02 | Final QA & validation       | 05.03               | -          | TASK-06.01    |

---

## Progress Tracking Table

| Phase    | Status      | Start Date | Completion Date | Blocked By | Owner | Notes                   |
| -------- | ----------- | ---------- | --------------- | ---------- | ----- | ----------------------- |
| PHASE-01 | Not Started | -          | -               | -          | -     | Discovery & inventory   |
| PHASE-02 | Not Started | -          | -               | PHASE-01   | -     | UI helpers foundation   |
| PHASE-03 | Not Started | -          | -               | PHASE-02   | -     | Core editor conversions |
| PHASE-04 | Not Started | -          | -               | PHASE-03   | -     | Complete sweep          |
| PHASE-05 | Not Started | -          | -               | PHASE-04   | -     | Engine integration      |
| PHASE-06 | Not Started | -          | -               | PHASE-05   | -     | Documentation & QA      |

**Status Values**: Not Started | In Progress | Blocked | Completed | Failed

---

## File Modification Summary

| File                                              | Phase    | Changes                        | Est. LOC |
| ------------------------------------------------- | -------- | ------------------------------ | -------- |
| `docs/explanation/csv_migration_inventory.md`     | PHASE-01 | CSV inventory table            | +50      |
| `docs/explanation/csv_migration_checklist.md`     | PHASE-01 | Refactor checklist             | +30      |
| `sdk/campaign_builder/src/ui_helpers.rs`          | PHASE-02 | Searchable selectors + tests   | +250     |
| `sdk/campaign_builder/src/map_editor.rs`          | PHASE-03 | EventEditorState conversion    | ~50      |
| `sdk/campaign_builder/src/characters_editor.rs`   | PHASE-03 | CharacterEditBuffer conversion | ~40      |
| `sdk/campaign_builder/src/classes_editor.rs`      | PHASE-03 | ClassEditBuffer conversion     | ~30      |
| `sdk/campaign_builder/src/items_editor.rs`        | PHASE-03 | Item tags conversion           | ~20      |
| `sdk/campaign_builder/src/spells_editor.rs`       | PHASE-03 | Spell effects conversion       | ~20      |
| `docs/explanation/csv_sweep_report.md`            | PHASE-04 | Sweep documentation            | +20      |
| `docs/explanation/combobox_unification_report.md` | PHASE-04 | Unification documentation      | +20      |
| `domain/src/*.rs` (as needed)                     | PHASE-05 | Domain model updates           | ~30      |
| `docs/explanation/implementations.md`             | PHASE-06 | Migration summary              | +50      |
| `docs/how-to/use_searchable_selectors.md`         | PHASE-06 | User guide                     | +100     |
| `scripts/validate_csv_migration.sh`               | PHASE-06 | Validation script              | +80      |
| **Estimated Total**                               |          | **~790 LOC added/modified**    |          |

---

## Test Coverage Targets

| Module              | Before Migration | Target After | Notes                          |
| ------------------- | ---------------- | ------------ | ------------------------------ |
| `ui_helpers`        | 0%               | 80%+         | New module                     |
| `map_editor`        | ~60%             | 70%+         | Improved with Vec tests        |
| `characters_editor` | ~65%             | 75%+         | Improved with Vec tests        |
| `classes_editor`    | ~50%             | 65%+         | Improved with Vec tests        |
| `items_editor`      | ~55%             | 65%+         | Improved with Vec tests        |
| `spells_editor`     | ~55%             | 65%+         | Improved with Vec tests        |
| **Overall SDK**     | ~55%             | **70%+**     | Significant improvement target |

**Measurement Tool**: `cargo tarpaulin` or `cargo llvm-cov`

---

## Quick Command Reference

```bash
# Project root
cd /home/bsmith/go/src/github.com/xbcsmith/antares

# Search commands (Phase 1)
grep -rn "split.*',' *)" sdk/campaign_builder/src/
grep -rn "join.*',' *)" sdk/campaign_builder/src/
grep -rn "ComboBox::" sdk/campaign_builder/src/

# Quality checks (run after each phase)
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# Package-specific checks
cargo check --package campaign_builder
cargo test --package campaign_builder
cargo test --package campaign_builder ui_helpers::tests

# Coverage report (if tarpaulin installed)
cargo tarpaulin --out Html --output-dir coverage

# Validation script (Phase 6)
bash scripts/validate_csv_migration.sh
```

---

## Glossary

**CSV (Comma-Separated Values)**: Text format where list items are separated by commas; error-prone for internal data structures.

**ComboBox**: egui dropdown widget for selecting from a list; being replaced with searchable selectors.

**id_salt**: Unique identifier string for egui widgets, used for state persistence across frames.

**searchable_selector**: Custom UI widget combining search input with filtered dropdown; replaces ComboBox.

**chip**: Visual tag representing a selected item in multi-select UI, typically with a remove button.

**RON (Rusty Object Notation)**: Rust-friendly data serialization format, human-readable with Rust syntax.

**ItemId, MonsterId, etc.**: Type aliases wrapping `u32` for type-safe ID handling in domain model.

**domain model**: Core data structures representing game entities (Character, Item, Monster, etc.).

**editor buffer**: UI-layer struct holding temporary state during editing, before committing to domain.

**migration helper**: Temporary utility function for CSV-to-Vec conversion during migration; removed in Phase 6.

---

## Final Summary

This implementation plan provides a comprehensive, AI-optimized guide for migrating the Antares SDK from CSV string encoding to typed vector-based data structures. The plan is divided into 6 discrete phases with atomic tasks, executable validation commands, and clear success criteria.

**Key Features**:

- ✅ All architectural decisions resolved (ADRs)
- ✅ Zero ambiguity in task descriptions
- ✅ Executable validation commands for every phase
- ✅ Complete dependency matrix
- ✅ Risk assessment with mitigation strategies
- ✅ Phase-specific rollback procedures
- ✅ Machine-readable format (tables, checklists)
- ✅ Estimated effort and file modification summary

**Expected Outcomes**:

- Type-safe vector-based list fields throughout SDK
- Unified searchable selector UI pattern
- Zero CSV parsing in editor code
- Improved testability and maintainability
- 70%+ test coverage across SDK modules
- Cleaner, more maintainable codebase

**Timeline**: 15-25 person-hours across 6 phases

**Status**: Ready for execution by AI agents or human developers

---

**END OF IMPLEMENTATION PLAN**
