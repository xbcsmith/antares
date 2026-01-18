# Proficiencies Editor Implementation Plan

## Overview

The Campaign Builder currently lacks a dedicated editor for managing proficiencies, which are fundamental to the game's character and item systems. This plan adds a Proficiencies Editor tab following the established two-column layout pattern used by Items, Monsters, and Spells editors.

## Current State Analysis

### Existing Infrastructure

- **Domain Model**: [`proficiency.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/proficiency.rs) defines `ProficiencyDefinition` with fields:
  - `id: ProficiencyId` (String type)
  - `name: String`
  - `category: ProficiencyCategory` (enum: Weapon, Armor, Shield, MagicItem)
  - `description: String`
- **Data File**: [`data/proficiencies.ron`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/data/proficiencies.ron) contains 13 proficiency definitions organized by category
- **UI Components**: [`ui_helpers.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs) provides:
  - `TwoColumnLayout` for list/detail split view
  - `EditorToolbar` for common actions (New, Save, Load, Import, Export, Reload)
  - `ActionButtons` for Edit, Delete, Duplicate, Export actions
- **Editor Patterns**: Existing editors ([`items_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs), [`spells_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/spells_editor.rs)) provide proven patterns

### Identified Issues

1. **No Proficiencies Editor Exists**
   - No `proficiencies_editor.rs` module in SDK
   - No tab in Campaign Builder main UI for proficiencies
   - Users cannot create, edit, or manage proficiencies through the UI

2. **Missing Integration Points**
   - Main application doesn't load or manage proficiency data
   - No state management for proficiencies
   - No file I/O for campaign-specific proficiencies

3. **No Validation or Preview**
   - No UI to validate proficiency definitions
   - No preview panel to view proficiency details
   - No category filtering or search

## Implementation Phases

### Phase 1: Create Proficiencies Editor Module

#### 1.1 Create proficiencies_editor.rs

Create new file `sdk/campaign_builder/src/proficiencies_editor.rs` with:

- **Editor Mode Enum**:
  ```rust
  pub enum ProficienciesEditorMode {
      List,
      Add,
      Edit,
  }
  ```

- **Editor State Struct**:
  ```rust
  pub struct ProficienciesEditorState {
      pub mode: ProficienciesEditorMode,
      pub search_query: String,
      pub selected_proficiency: Option<usize>,
      pub edit_buffer: ProficiencyDefinition,
      pub show_import_dialog: bool,
      pub import_export_buffer: String,
      pub filter_category: Option<ProficiencyCategory>,
  }
  ```

- **Default Implementation**: Provide `Default` and `new()` methods
- **Default Proficiency**: Create `default_proficiency()` helper for new proficiencies

#### 1.2 Implement Main UI Method

Add `show()` method following items_editor pattern:

- Accept parameters: `ui`, `proficiencies: &mut Vec<ProficiencyDefinition>`, `campaign_dir`, `proficiencies_file`, `unsaved_changes`, `status_message`, `file_load_merge_mode`
- Use `EditorToolbar` component for toolbar actions
- Add category filter dropdown (Weapon, Armor, Shield, MagicItem, All)
- Route to `show_list()` or `show_form()` based on mode

#### 1.3 Implement List View

Create `show_list()` method with two-column layout:

- **Left Column**: Scrollable proficiency list
  - Filter by search query (name/id matching)
  - Filter by category if selected
  - Display format: `"{id}: {name}"` with category emoji (⚔️ Weapon, 🛡️ Armor, etc.)
  - Selectable labels for each proficiency
  - Sort by category then by id

- **Right Column**: Detail preview panel
  - Show selected proficiency details using `show_preview_static()`
  - Display: name, id, category, description
  - Use `ActionButtons` component (Edit, Delete, Duplicate, Export)
  - Handle action button clicks after closures

#### 1.4 Implement Form View

Create `show_form()` method for Add/Edit modes:

- **Basic Properties Group**:
  - ID field (disabled for edit, auto-generated for add)
  - Name field (text input)
  - Category dropdown (Weapon, Armor, Shield, MagicItem)
  - Description field (multiline text, 3-5 rows)

- **Action Buttons**:
  - "💾 Save" - validates and saves proficiency
  - "❌ Cancel" - returns to list mode
  - "🔄 Reset" - resets form to original values (edit mode only)

- **Validation**:
  - ID must not be empty and must be unique
  - Name must not be empty
  - Description should not be empty (warning, not error)

#### 1.5 Implement Helper Methods

Add supporting methods:

- `show_preview_static()` - static preview of proficiency details
- `show_import_dialog()` - RON import/export dialog window
- `save_proficiencies()` - save to campaign directory
- `next_proficiency_id()` - generate next available ID (based on category prefix)

#### 1.6 Testing Requirements

- Unit tests for state creation and defaults
- Unit tests for category filtering logic
- Unit tests for ID generation
- Manual testing: create, edit, delete, duplicate proficiencies

#### 1.7 Deliverables

- [ ] `proficiencies_editor.rs` created with all methods
- [ ] Editor state and mode enums implemented
- [ ] Two-column layout integrated
- [ ] Form validation working
- [ ] Import/export functionality complete
- [ ] Unit tests passing

#### 1.8 Success Criteria

- Proficiencies editor compiles without errors
- Can create new proficiencies with auto-generated IDs
- Can edit existing proficiencies
- Can delete proficiencies
- Can filter by category
- Can search by name/id
- Import/export RON works correctly

---

### Phase 2: Integrate into Main Application

#### 2.1 Add Module Declaration

In `sdk/campaign_builder/src/lib.rs`:

- Add `pub mod proficiencies_editor;` declaration
- Re-export `ProficienciesEditorState` if needed

#### 2.2 Update Main Application State

In `sdk/campaign_builder/src/main.rs` (or equivalent):

- Add `proficiencies: Vec<ProficiencyDefinition>` to app state
- Add `proficiencies_editor: ProficienciesEditorState` to app state
- Load proficiencies from `data/proficiencies.ron` on startup
- Load campaign-specific proficiencies if they exist

#### 2.3 Add Proficiencies Tab

Update main UI to include Proficiencies tab:

- Add tab button: "🎯 Proficiencies" or similar
- Add tab panel that calls `proficiencies_editor.show()`
- Pass required parameters: proficiencies data, campaign dir, file paths, etc.
- Ensure tab switching preserves editor state

#### 2.4 File I/O Integration

Implement proficiency file loading/saving:

- Default file: `data/proficiencies.ron`
- Campaign file: `{campaign_dir}/data/proficiencies.ron`
- Merge mode: combine global + campaign proficiencies
- Replace mode: use only loaded proficiencies

#### 2.5 Testing Requirements

- Manual testing: Navigate to Proficiencies tab
- Manual testing: Create proficiency and verify it saves to campaign directory
- Manual testing: Reload proficiencies and verify data persists
- Manual testing: Test merge mode vs replace mode

#### 2.6 Deliverables

- [ ] Module integrated into `lib.rs`
- [ ] Main app state includes proficiencies
- [ ] Proficiencies tab added to UI
- [ ] File loading/saving works correctly
- [ ] Tab navigation functional

#### 2.7 Success Criteria

- Proficiencies tab appears in Campaign Builder
- Can navigate to proficiencies tab and back
- Proficiencies load from global data file on startup
- Campaign-specific proficiencies load if present
- Changes save to campaign directory
- Reload button refreshes from file

---

### Phase 3: Validation and Polish

#### 3.1 Add Category-Based ID Suggestions

Enhance ID generation with category prefixes:

- Weapon: `weapon_*` or `martial_*` or `simple_*`
- Armor: `*_armor`
- Shield: `shield*`
- MagicItem: `*_item`
- Auto-suggest ID based on name and category

#### 3.2 Add Proficiency Usage Tracking

Show where proficiencies are used:

- Check classes for proficiency grants
- Check races for proficiency grants
- Check items for proficiency requirements
- Display usage count in preview panel
- Warn before deleting used proficiencies

#### 3.3 Add Category Icons and Colors

Enhance visual feedback:

- Weapon: ⚔️ (red/orange)
- Armor: 🛡️ (blue)
- Shield: 🛡️ (cyan)
- MagicItem: ✨ (purple)
- Use colors in list view and preview

#### 3.4 Add Bulk Operations

Implement bulk actions:

- "Export All" - export all proficiencies to RON
- "Import Multiple" - import array of proficiencies
- "Reset to Defaults" - reload from global data file
- "Duplicate Category" - duplicate all proficiencies in a category

#### 3.5 Testing Requirements

- Manual testing: Verify ID suggestions work for each category
- Manual testing: Create proficiency used by class, verify usage shown
- Manual testing: Try to delete used proficiency, verify warning
- Manual testing: Test bulk export/import

#### 3.6 Deliverables

- [ ] Category-based ID generation implemented
- [ ] Usage tracking functional
- [ ] Visual enhancements complete
- [ ] Bulk operations working
- [ ] All manual tests passing

#### 3.7 Success Criteria

- ID suggestions make sense for each category
- Usage tracking accurately shows where proficiencies are used
- Cannot accidentally delete proficiencies in use
- Bulk operations work correctly
- UI is visually consistent with other editors

## Verification Plan

### Automated Tests

Run SDK tests to ensure no regressions:

```bash
cd /Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder
cargo test proficiencies_editor --lib
cargo test
```

Expected tests to pass:
- `test_proficiencies_editor_state_new`
- `test_proficiencies_editor_state_default`
- `test_default_proficiency_creation`
- `test_category_filter_logic`
- `test_proficiency_id_generation`

### Manual Verification

After each phase, perform the following manual tests:

**Phase 1 - Editor Module:**
1. Run Campaign Builder: `cargo run --bin campaign_builder`
2. Navigate to Proficiencies tab (once integrated)
3. Verify proficiencies list loads
4. Click "New" and create a new proficiency
5. Fill in all fields and save
6. Verify new proficiency appears in list
7. Select proficiency and click "Edit"
8. Modify fields and save
9. Verify changes persist
10. Select proficiency and click "Delete"
11. Verify proficiency is removed

**Phase 2 - Integration:**
1. Start Campaign Builder
2. Verify Proficiencies tab appears
3. Click Proficiencies tab
4. Verify global proficiencies load (13 default proficiencies)
5. Create a new proficiency
6. Click "Save" toolbar button
7. Verify file created at `{campaign}/data/proficiencies.ron`
8. Close and reopen Campaign Builder
9. Verify proficiency persists

**Phase 3 - Polish:**
1. Create new weapon proficiency
2. Verify ID suggestion includes "weapon" or similar
3. Create a class that grants the proficiency
4. Return to Proficiencies tab
5. Select the proficiency
6. Verify usage count shows "Used by 1 class"
7. Try to delete the proficiency
8. Verify warning appears
9. Test category filter dropdown
10. Verify filtering works for each category

### Build Verification

Ensure the SDK compiles without errors:

```bash
cd /Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder
cargo check
cargo clippy -- -D warnings
cargo fmt --check
```

All commands should complete successfully with no errors or warnings.

### Integration Verification

Test proficiencies integration with other editors:

1. **Class Editor**: Verify proficiency autocomplete works
2. **Race Editor**: Verify proficiency autocomplete works
3. **Item Editor**: Verify proficiency tags work correctly
4. **Character Editor**: Verify proficiency display works

## Notes

- Follow the exact pattern from `items_editor.rs` for consistency
- Use `ProficiencyCategory` enum for filtering (not string matching)
- Proficiency IDs are strings, not integers (unlike items/spells)
- Consider adding proficiency icons/images in future phases
- May want to add proficiency inheritance/hierarchy in future
