<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# SDK Campaign Builder UI Continuity and Quality of Life Implementation Plan

## Overview

This plan outlines a phased approach to refactor the Campaign Builder SDK to
achieve UI consistency across all editors, reduce cognitive complexity in
`main.rs`, improve validation feedback, and enhance the overall developer
experience. The goal is to ensure all editors follow the same layout patterns,
extract UI code into dedicated modules, and make the codebase more maintainable
and testable.

## Current State Analysis

### Existing Infrastructure

The Campaign Builder SDK currently has the following editor modules with
`show()` methods properly extracted:

| Editor     | File                   | Has `show()`      | Layout Pattern                          |
| ---------- | ---------------------- | ----------------- | --------------------------------------- |
| Items      | `items_editor.rs`      | ✓                 | Standard (Edit/Delete/Duplicate/Export) |
| Monsters   | `monsters_editor.rs`   | ✓                 | Standard (Edit/Delete/Duplicate/Export) |
| Spells     | `spells_editor.rs`     | ✓                 | Standard (Edit/Delete/Duplicate/Export) |
| Conditions | `conditions_editor.rs` | ✓                 | Non-standard (needs buttons)            |
| Quests     | `quest_editor.rs`      | ✓                 | Non-standard (needs buttons)            |
| Classes    | `classes_editor.rs`    | ✗ (UI in main.rs) | Non-standard                            |
| Dialogues  | `dialogue_editor.rs`   | ✗ (UI in main.rs) | Non-standard                            |
| Maps       | `map_editor.rs`        | Widget pattern    | Different (needs refactor)              |

Shared UI utilities exist in `ui_helpers.rs` with panel height computation and
column width constants.

### Identified Issues

1. **Inconsistent Editor Patterns**: Not all editors follow the same toolbar
   and display panel layout
2. **UI Code in main.rs**: Classes and Dialogues editors have UI rendering
   logic in `main.rs` instead of their respective modules
3. **Toolbar Redundancy**: Current buttons include entity names (e.g., "Add
   Item", "Add Monster") which is redundant when already in the editor
4. **Missing Standard Buttons**: Conditions, Quests, Dialogues, and Maps
   editors lack Edit/Delete/Duplicate/Export buttons in the display panel
5. **Validation UI**: Current validation panel uses simple list display without
   table-like alignment
6. **Assets Panel**: Incorrectly reports loaded files as "unreferenced"
7. **Auto-load**: Not all editors automatically load data when campaign opens
8. **Testing**: Current tests use brittle string matching
9. **AttributePair Editing**: UI does not support editing both `base` and
   `current` values for `AttributePair` and `AttributePair16` types (e.g.,
   creating a wounded dragon with HP base=500, current=25)

## Implementation Phases

### Phase 1: Foundation and Centralized UI Components

**Goal:** Create truly reusable components that all editors will use, not just
patterns to copy. This reduces code duplication and ensures consistency.

**Current State:** `ui_helpers.rs` already shares `compute_panel_height()`,
`DEFAULT_LEFT_COLUMN_WIDTH`, and `DEFAULT_PANEL_MIN_HEIGHT`. However, toolbar
buttons, action buttons, two-column layouts, and import/export dialogs are
duplicated across items_editor, monsters_editor, spells_editor, and
conditions_editor.

#### 1.1 Create Shared Toolbar Component

Create a **reusable, callable component** (not just a pattern) in `ui_helpers.rs`:

- Define `EditorToolbar` struct with standard button configuration
- Implement `EditorToolbar::show()` method that renders the toolbar and returns
  which action was triggered
- Define `ToolbarAction` enum: `New`, `Save`, `Load`, `Import`, `Export`,
  `Reload`, `None`
- Include merge checkbox state management
- Remove ALL entity-specific names from buttons (e.g., "➕ Add Item" becomes
  "➕ New")
- Ensure toolbar scales to fit screen width

Standard toolbar layout (fits screen width):

```text
┌─────────────────────────────────────────────────────────────────┐
│ ➕New  💾Save  📂Load  📥Import [x] Merge  📋Export             │
├─────────────────────────────────────────────────────────────────┤
│ 🔍 Search: [________]                                           │
├─────────────────────────────────────────────────────────────────┤
│ Filters: [Type ▼] [✨ Magical] [💀 Cursed] [🔄 Clear]           │
└─────────────────────────────────────────────────────────────────┘
```

**Usage pattern:**

```rust
// In any editor's show() method:
let action = EditorToolbar::new("Items")
    .with_search(&mut self.search_query)
    .with_merge_mode(&mut self.file_load_merge_mode)
    .show(ui);

match action {
    ToolbarAction::New => { /* create new item */ }
    ToolbarAction::Save => { /* save to campaign */ }
    // ...
}
```

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add `EditorToolbar`, `ToolbarAction`

#### 1.2 Create Shared Action Buttons Component

Create a **reusable component** for the Edit/Delete/Duplicate/Export buttons:

- Define `ItemAction` enum: `Edit`, `Delete`, `Duplicate`, `Export`, `None`
- Define `ActionButtons::show()` that renders buttons and returns triggered
  action
- Standardize button layout, icons, and spacing
- Handle disabled states (e.g., no selection)

**Usage pattern:**

```rust
// In any editor's detail panel:
let action = ActionButtons::new()
    .enabled(self.selected_item.is_some())
    .show(ui);

match action {
    ItemAction::Edit => { /* enter edit mode */ }
    ItemAction::Delete => { /* delete selected */ }
    // ...
}
```

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add `ActionButtons`, `ItemAction`

#### 1.3 Create Shared Two-Column Layout Component

Create a reusable two-column layout helper:

- Define `TwoColumnLayout::show()` that handles the standard list/detail split
- Manage panel heights using existing `compute_panel_height()`
- Provide consistent column widths using `DEFAULT_LEFT_COLUMN_WIDTH`
- Handle ScrollArea setup with proper id_salt

**Usage pattern:**

```rust
TwoColumnLayout::new("items")
    .show(ui, |left_ui, right_ui| {
        // left_ui: render list
        // right_ui: render detail/preview
    });
```

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add `TwoColumnLayout`

#### 1.4 Create Shared Import/Export Dialog Component

Create a reusable import/export dialog:

- Define `ImportExportDialog` struct
- Handle RON parsing and validation
- Provide consistent error messaging
- Support both single-item and batch import

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add `ImportExportDialog`

#### 1.5 Create AttributePair Input Widget

Create a reusable widget for editing `AttributePair` and `AttributePair16`:

- Define `AttributePairInput::show()` for u8 values
- Define `AttributePair16Input::show()` for u16 values
- Show dual input fields: Base and Current
- Auto-sync current to base when base changes (unless manually overridden)
- Include "Reset to Base" button

**Usage pattern:**

```rust
// For HP (u16):
AttributePair16Input::new("HP", &mut monster.hp).show(ui);

// For AC (u8):
AttributePairInput::new("AC", &mut monster.ac).show(ui);
```

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add `AttributePairInput`,
  `AttributePair16Input`

#### 1.6 Refactor Existing Editors to Use Shared Components

Update items_editor, monsters_editor, and spells_editor to use the new shared
components. This validates the components work correctly before applying to
other editors.

**Files to modify:**

- `sdk/campaign_builder/src/items_editor.rs` - Use shared components
- `sdk/campaign_builder/src/monsters_editor.rs` - Use shared components
- `sdk/campaign_builder/src/spells_editor.rs` - Use shared components

#### 1.7 Testing Requirements

- Unit tests for `EditorToolbar` action dispatch
- Unit tests for `ActionButtons` action dispatch
- Unit tests for `TwoColumnLayout` height calculations
- Unit tests for `AttributePairInput` base/current sync behavior
- Integration test verifying refactored editors still work correctly

#### 1.8 Deliverables

- `EditorToolbar` component with `ToolbarAction` enum
- `ActionButtons` component with `ItemAction` enum
- `TwoColumnLayout` component
- `ImportExportDialog` component
- `AttributePairInput` and `AttributePair16Input` widgets
- Refactored items_editor, monsters_editor, spells_editor
- Tests for all new components

#### 1.9 Success Criteria

- All shared components are callable from any editor
- items_editor, monsters_editor, spells_editor use shared components
- No duplicated toolbar/button code in refactored editors
- All existing tests continue to pass
- New component tests pass

---

### Phase 2: Extract Editor UI Code from main.rs

#### 2.1 Extract Classes Editor UI

Move all Classes editor UI code from `main.rs` to `classes_editor.rs`:

- Implement `ClassesEditorState::show()` method following standard signature
- Move `show_classes_list()`, `show_class_form()` logic into the editor module
- Implement auto-load from campaign directory
- Update `main.rs` to call `classes_editor_state.show()`

**Files to modify:**

- `sdk/campaign_builder/src/classes_editor.rs` - Add `show()` method
- `sdk/campaign_builder/src/main.rs` - Remove classes UI code, add delegation

#### 2.2 Extract Dialogues Editor UI

Move all Dialogues editor UI code from `main.rs` to `dialogue_editor.rs`:

- Implement `DialogueEditorState::show()` method following standard signature
- Move `show_dialogue_list()`, `show_dialogue_form()` logic into the editor
- Implement auto-load from campaign directory
- Update `main.rs` to call `dialogue_editor_state.show()`

**Files to modify:**

- `sdk/campaign_builder/src/dialogue_editor.rs` - Add `show()` method
- `sdk/campaign_builder/src/main.rs` - Remove dialogues UI code, add delegation

#### 2.3 Verify Auto-load for All Editors

Ensure every editor implements auto-load when a campaign is opened:

- Items, Spells, Monsters, Conditions already load (verify)
- Add auto-load for Classes, Dialogues, Quests, Maps

**Files to modify:**

- Various `*_editor.rs` files as needed

#### 2.4 Testing Requirements

- Test that `ClassesEditorState::show()` renders correctly
- Test that `DialogueEditorState::show()` renders correctly
- Test auto-load triggers for each editor type
- Verify bug_verification tests still pass

#### 2.5 Deliverables

- `ClassesEditorState::show()` in `classes_editor.rs`
- `DialogueEditorState::show()` in `dialogue_editor.rs`
- Updated `main.rs` with reduced complexity
- Auto-load implementation for all editors

#### 2.6 Success Criteria

- `main.rs` no longer contains editor-specific UI rendering logic
- All editors have `show()` methods with consistent signatures
- Auto-load works for all editors when campaign opens
- Test file searches can target specific editor files

---

### Phase 3: Editor Layout Continuity

#### 3.1 Update Conditions Editor Layout

Refactor `conditions_editor.rs` to use standard layout:

- Update toolbar to use `EditorToolbar` component
- Add Edit/Delete/Duplicate/Export buttons to display panel
- Ensure two-column layout (list on left, detail on right)
- Match items_editor/monsters_editor/spells_editor pattern

**Files to modify:**

- `sdk/campaign_builder/src/conditions_editor.rs`

#### 3.2 Update Quests Editor Layout

Refactor `quest_editor.rs` to use standard layout:

- Update toolbar to use `EditorToolbar` component
- Add Edit/Delete/Duplicate/Export buttons to display panel
- Ensure consistent two-column layout
- Maintain stage/objective sub-editors within detail view

**Files to modify:**

- `sdk/campaign_builder/src/quest_editor.rs`

#### 3.3 Update Dialogues Editor Layout

Refactor `dialogue_editor.rs` to use standard layout:

- Update toolbar to use `EditorToolbar` component
- Add Edit/Delete/Duplicate/Export buttons to display panel
- Ensure consistent two-column layout
- Maintain node/choice sub-editors within detail view

**Files to modify:**

- `sdk/campaign_builder/src/dialogue_editor.rs`

#### 3.4 Update Classes Editor Layout

Refactor `classes_editor.rs` to use standard layout:

- Update toolbar to use `EditorToolbar` component
- Add Edit/Delete/Duplicate/Export buttons to display panel
- Ensure consistent two-column layout

**Files to modify:**

- `sdk/campaign_builder/src/classes_editor.rs`

#### 3.5 Apply AttributePair Widgets to All Editors

The `AttributePairInput` and `AttributePair16Input` widgets are created in Phase

1. This task applies them to all editors that have AttributePair fields.

**Affected editors and fields:**

- **Monsters Editor**: HP (AttributePair16), AC (AttributePair), all Stats
  (AttributePair for might, intellect, personality, endurance, speed, accuracy,
  luck)
- **Conditions Editor**: Any effects that modify AttributePair values

**Example UI (using shared widget from Phase 1):**

```text
HP: Base [500____] Current [25_____] [Reset]
AC: Base [18_____] Current [18_____] [Reset]
```

**Files to modify:**

- `sdk/campaign_builder/src/monsters_editor.rs` - Use `AttributePair16Input`
  for HP, `AttributePairInput` for AC and Stats
- `sdk/campaign_builder/src/conditions_editor.rs` - Use widgets where applicable

#### 3.6 Refactor Maps Editor (Major)

This is the most significant refactor. The Maps editor will be fully integrated
into `MapsEditorState::show()` to behave exactly like `items_editor`:

- Create `MapsEditorState::show()` following standard signature
- Fully integrate canvas/grid rendering into the `show()` method (remove
  separate `MapEditorWidget` pattern)
- Implement two-column layout: map list on left, map preview/editor on right
- Add standard toolbar: New, Save, Load, Import (w/Merge), Export
- Add Edit/Delete/Duplicate/Export buttons to display panel
- Move tile editing tools into the detail panel when in edit mode
- Ensure map list shows preview thumbnails like items show type icons

**Files to modify:**

- `sdk/campaign_builder/src/map_editor.rs` - Major refactor to match
  items_editor pattern
- `sdk/campaign_builder/src/main.rs` - Update delegation, remove
  `show_maps_editor()`, `show_maps_list()`, `show_map_editor_panel()` functions

#### 3.7 Testing Requirements

- Visual regression tests (manual) for each editor
- Test Edit/Delete/Duplicate/Export actions work correctly
- Test toolbar actions (New/Save/Load/Import/Export) for each editor

#### 3.8 Deliverables

- All editors using `EditorToolbar` component
- All editors with Edit/Delete/Duplicate/Export buttons
- Consistent two-column layout across all editors
- Maps editor refactored to match pattern

#### 3.9 Success Criteria

- Every editor has identical toolbar layout: New, Save, Load, Import
  (w/Merge), Export
- Every editor display panel has Edit, Delete, Duplicate, Export buttons
- AttributePair fields show both base and current values with proper editing
- Two-column layout is consistent (list width, panel heights)
- User can navigate any editor with same mental model

---

### Phase 4: Validation and Assets UI Improvements

#### 4.1 Improve Validation Panel Layout

Refactor validation panel to use table-like layout:

- Create validation result table with columns: Status Icon, Category, Message
- Use ✅ (green check) for passed validations
- Use ❌ (red X) for failed validations
- Use ⚠️ (yellow warning) for warnings
- Group validations by category (Metadata, Items, Spells, etc.)
- Add count summary at top

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` - `show_validation_panel()` function

#### 4.2 Create Validation Result Types

Define structured types for validation results:

- `ValidationCategory` enum (Metadata, Items, Spells, Monsters, Maps, etc.)
- `ValidationResult` struct with status, category, file path, message
- Update `validate_campaign()` to return structured results

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` or create `validation.rs` module

#### 4.3 Fix Assets Panel Reporting

Fix the assets panel to correctly report file status:

- Show "Loaded" status for files successfully loaded from campaign
- Show "Error" status for files that failed to load
- Remove "unreferenced" reporting for campaign data files
- Only report truly orphaned asset files (images, sounds not referenced)

**Files to modify:**

- `sdk/campaign_builder/src/asset_manager.rs`
- `sdk/campaign_builder/src/main.rs` - `show_assets_editor()`

#### 4.4 Testing Requirements

- Test validation panel renders table correctly
- Test all validation categories display properly
- Test assets panel shows correct status for loaded files

#### 4.5 Deliverables

- Table-based validation panel layout
- Structured `ValidationResult` types
- Fixed assets panel reporting

#### 4.6 Success Criteria

- Validation results display in aligned table format
- Each validated item shows clear pass/fail icon
- Assets panel correctly identifies loaded vs error files

---

### Phase 5: Testing Infrastructure Improvements

#### 5.1 Improve Test Resilience with Pattern Matching

Replace brittle string-matching tests with improved pattern matching:

- Search all editor files (`src/*.rs`) instead of just `main.rs`
- Use regex pattern matching for widget ID detection
- Add helper functions for common test assertions
- Create patterns that match function signatures, not exact strings

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` - test module
- Consider creating `sdk/campaign_builder/src/test_utils.rs`

#### 5.2 Add ComboBox ID Salt Verification

Create tests that verify `ComboBox::from_id_salt` usage:

- Scan all editor files for ComboBox usage patterns
- Verify no `ComboBox::from_label` usage (causes ID conflicts)
- Use regex to detect pattern violations
- Make test fail if `from_label` pattern detected

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` - add new test functions

#### 5.3 Add Editor Pattern Compliance Tests

Create tests that verify editors implement required patterns:

- Test each editor file contains `pub fn show` method
- Test each editor uses standard toolbar buttons
- Test each editor has action buttons pattern
- Use file scanning with regex, not AST parsing

**Files to create/modify:**

- `sdk/campaign_builder/src/main.rs` - add compliance tests

#### 5.4 Testing Requirements

- New tests must not break on minor refactors
- Tests should target behavior patterns, not exact implementation
- Coverage for all editor types
- Use regex patterns that allow for whitespace/formatting variations

#### 5.5 Deliverables

- Improved test utilities with regex helpers
- ComboBox ID salt verification tests
- Editor pattern compliance tests

#### 5.6 Success Criteria

- Tests pass when editor files are renamed/reorganized
- Tests catch ComboBox ID conflicts via pattern matching
- Tests verify all editors follow standard patterns

#### 5.7 Future Considerations

AST-based testing using the `syn` crate could provide more robust source code
analysis in the future. This would allow:

- Parsing Rust source files into AST nodes
- Inspecting method signatures and bodies programmatically
- Detecting specific function calls with full type awareness
- Avoiding false positives from comments or string literals

This approach requires additional complexity and dependencies, so it is
documented here for future consideration if pattern matching proves
insufficient.

---

### Phase 6: Data Files Update

#### 6.1 Update Core Data Files

After all UI changes are complete, update the core data files:

- `data/items.ron`
- `data/spells.ron`
- `data/monsters.ron`
- `data/conditions.ron`
- `data/classes.ron`
- `data/races.ron`

**Files to modify:**

- `data/*.ron` - Re-export using updated editors to ensure format consistency

#### 6.2 Update Tutorial Campaign

Update the tutorial campaign to work with the updated editors:

- `campaigns/tutorial/campaign.ron`
- `campaigns/tutorial/data/*.ron`

**Files to modify:**

- `campaigns/tutorial/**/*.ron` - Re-validate and re-export all data files

#### 6.3 Testing Requirements

- Verify all data files load correctly in updated editors
- Run full campaign validation on tutorial campaign
- Test gameplay with updated data files

#### 6.4 Deliverables

- Updated `data/` files
- Updated `campaigns/tutorial/` files
- Validation passing for all campaigns

#### 6.5 Success Criteria

- All data files load without errors
- Tutorial campaign passes full validation
- No data loss during migration

---

### Phase 7: Logging and Developer Experience (Lower Priority)

#### 7.1 Add Verbose Logging Level

Implement configurable verbose logging:

- Add `VERBOSE` log level (more detailed than DEBUG)
- Add command-line flag `--verbose` or `-v`
- Log editor state transitions
- Log file I/O operations
- Log validation details

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` - main function and logging setup

#### 7.2 Add Debug Panel

Create optional debug panel for development:

- Show current editor state
- Show loaded data counts
- Show recent log messages
- Toggle via menu or keyboard shortcut

**Files to modify:**

- `sdk/campaign_builder/src/main.rs` - add debug panel toggle and rendering

#### 6.3 Testing Requirements

- Test verbose logging produces expected output
- Test debug panel toggles correctly

#### 6.4 Deliverables

- Verbose logging implementation
- Debug panel (optional feature)

#### 6.5 Success Criteria

- Developers can enable verbose logging for debugging
- Debug panel shows useful runtime information

---

## Implementation Order Summary

| Phase | Focus                        | Priority | Estimated Effort     |
| ----- | ---------------------------- | -------- | -------------------- |
| 1     | Foundation (Toolbar/Buttons) | High     | Medium               |
| 2     | Extract UI from main.rs      | High     | Medium               |
| 3     | Layout Continuity            | High     | High (Maps is major) |
| 4     | Validation/Assets UI         | Medium   | Medium               |
| 5     | Testing Improvements         | Medium   | Low                  |
| 6     | Data Files Update            | High     | Low                  |
| 7     | Logging/Debug                | Low      | Low                  |

## Design Decisions

1. **Centralized Components**: All shared UI patterns (toolbar, action buttons,
   two-column layout, import/export dialogs, AttributePair inputs) will be
   implemented as **reusable, callable components** in `ui_helpers.rs`, not just
   patterns to copy. This reduces code duplication and ensures consistency.

2. **Maps Editor**: Fully integrate into `MapsEditorState::show()` to behave
   exactly like `items_editor`. The separate `MapEditorWidget` pattern will be
   removed, and all canvas/grid rendering will be part of the `show()` method.

3. **Button Naming**: All buttons will use generic labels without entity names.
   For example, "➕ Add Item" becomes "➕ New". This applies to ALL toolbar
   buttons across all editors.

4. **Filter Placement**: Filters will be moved to a separate row below the
   toolbar, following the ASCII diagram layout pattern.

5. **Testing Strategy**: Use improved pattern matching with regex. AST-based
   testing (using `syn` crate) is documented as a future consideration but will
   not be implemented in this plan.

6. **AttributePair Editing**: All editors will support editing both `base` and
   `current` values for `AttributePair` and `AttributePair16` types. This
   enables scenarios like creating a mortally wounded dragon (base HP=500,
   current HP=25). Implemented via shared `AttributePairInput` widget.

7. **No Backward Compatibility**: Data files will be updated after all changes
   are complete. The `data/` directory and `campaigns/tutorial/` will be
   re-exported using the updated editors.

## Dependencies

- Phase 2 depends on Phase 1 (needs toolbar components)
- Phase 3 depends on Phase 2 (editors must be extracted first)
- Phase 4 can run in parallel with Phase 3
- Phase 5 can run in parallel with Phases 3-4
- Phase 6 depends on Phases 1-5 (data update after all UI changes)
- Phase 7 can run at any time (independent)

## Risk Mitigation

1. **Large Refactor Risk (Maps Editor)**: Break into sub-tasks, maintain
   existing functionality while adding new pattern
2. **Test Breakage**: Run full test suite after each phase
3. **UI Regression**: Manual testing of each editor after changes
4. **Merge Conflicts**: Complete phases in order to minimize conflicts
