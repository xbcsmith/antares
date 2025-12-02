# Implementation Summary

Note: As of 2025-12-02, two pre-existing UI tests in `sdk/campaign_builder/tests/bug_verification.rs` were updated to reflect refactoring that moved editor implementations into separate module files. These tests — `test_items_tab_widget_ids_unique` and `test_monsters_tab_widget_ids_unique` — now inspect the refactored editor files (`src/items_editor.rs` and `src/monsters_editor.rs`, respectively) and validate the correct use of `egui::ComboBox::from_id_salt` rather than implicit ID generation methods (e.g., `from_label`) to avoid widget ID collisions.

This document tracks completed implementations and changes to the Antares project.

## Phase 1: SDK Campaign Builder UI - Foundation Components (2025-01-XX)

**Objective**: Create reusable, centralized UI components for the Campaign Builder SDK to reduce code duplication, ensure consistency across editors, and improve maintainability.

### Background

Per the SDK QOL Implementation Plan (`docs/explanation/sdk_qol_implementation_plan.md`), Phase 1 focuses on creating shared UI components that all editors can use. This phase establishes the foundation components without refactoring the existing editors, allowing incremental adoption.

### Components Created

All components were added to `sdk/campaign_builder/src/ui_helpers.rs`:

#### 1. EditorToolbar Component

A reusable toolbar component with standard buttons for all editors:

- **`ToolbarAction` enum**: `New`, `Save`, `Load`, `Import`, `Export`, `Reload`, `None`
- **`EditorToolbar` struct**: Builder pattern for configuring toolbar options
- Features:
  - Optional search field with customizable id salt
  - Optional merge mode checkbox
  - Optional total count display
  - Configurable save button visibility
- Usage: `EditorToolbar::new("Items").with_search(&mut query).show(ui)`

#### 2. ActionButtons Component

Reusable action buttons for detail panels:

- **`ItemAction` enum**: `Edit`, `Delete`, `Duplicate`, `Export`, `None`
- **`ActionButtons` struct**: Builder pattern for button configuration
- Features:
  - Enable/disable state
  - Per-button visibility control
  - Consistent button styling across editors
- Usage: `ActionButtons::new().enabled(has_selection).show(ui)`

#### 3. TwoColumnLayout Component

Standard two-column list/detail layout:

- **`TwoColumnLayout` struct**: Manages consistent column layout
- Features:
  - Uses `DEFAULT_LEFT_COLUMN_WIDTH` (300.0 points)
  - Uses `compute_panel_height()` for responsive sizing
  - Automatic scroll area setup with unique id salts
  - `show_split()` method for separate left/right closures
- Usage: `TwoColumnLayout::new("items").show_split(ui, left_fn, right_fn)`

#### 4. ImportExportDialog Component

Reusable import/export dialog for RON data:

- **`ImportExportResult` enum**: `Import(String)`, `Cancel`, `Open`
- **`ImportExportDialogState` struct**: Manages dialog state
- **`ImportExportDialog` struct**: Modal dialog implementation
- Features:
  - Separate import (editable) and export (read-only) modes
  - Error message display
  - Copy to clipboard support
  - Configurable dimensions
- Usage: `ImportExportDialog::new("Title", &mut state).show(ctx)`

#### 5. AttributePairInput Widget

Widget for editing `AttributePair` (u8 base/current):

- **`AttributePairInputState` struct**: Tracks auto-sync behavior
- **`AttributePairInput` struct**: Widget implementation
- Features:
  - Dual input fields for base and current values
  - Auto-sync option (current follows base when enabled)
  - Reset button to restore current to base
  - Customizable id salt
- Usage: `AttributePairInput::new("AC", &mut value).show(ui)`

#### 6. AttributePair16Input Widget

Widget for editing `AttributePair16` (u16 base/current):

- Same features as `AttributePairInput` but for 16-bit values
- Configurable maximum value
- Used for HP, SP, and other larger value attributes
- Usage: `AttributePair16Input::new("HP", &mut hp).with_max_value(9999).show(ui)`

#### 7. File I/O Helper Functions

Utility functions for common file operations:

- `load_ron_file<T>()` - Load and deserialize RON files
- `save_ron_file<T>()` - Serialize and save RON files
- `handle_file_load<T>()` - Complete file load with merge support
- `handle_file_save<T>()` - Complete file save with dialog
- `handle_reload<T>()` - Reload from campaign directory

### Tests Added

Comprehensive tests added to `ui_helpers.rs`:

- Panel height calculation tests (existing + new edge cases)
- `ToolbarAction` enum value tests
- `EditorToolbar` builder pattern tests
- `ItemAction` enum value tests
- `ActionButtons` builder pattern tests
- `TwoColumnLayout` configuration tests
- `ImportExportDialogState` lifecycle tests
- `ImportExportResult` enum tests
- `AttributePairInputState` tests
- `AttributePair` and `AttributePair16` reset behavior tests
- Constants validation tests

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - No compilation errors
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 218 tests pass

### Architecture Compliance

- Type aliases used consistently (no raw types)
- `AttributePair` pattern respected for modifiable stats
- RON format used for all data files
- Module structure follows architecture.md Section 3.2
- No circular dependencies introduced

### Success Criteria Met

- [x] All shared components are created and tested
- [x] Components are callable from any editor
- [x] All existing tests continue to pass
- [x] New component tests pass
- [x] AttributePair widgets support dual base/current editing

### Files Modified

- `sdk/campaign_builder/src/ui_helpers.rs` - Added all shared components and tests

### Deferred to Phase 1.6

The following refactoring was deferred to allow incremental adoption:

- Refactor items_editor.rs to use shared components
- Refactor monsters_editor.rs to use shared components
- Refactor spells_editor.rs to use shared components

This refactoring requires careful attention to type compatibility between the shared components and each editor's specific domain types (e.g., `ConsumableEffect` variants, `AmmoType` enum values, `Disablement` flags). It is recommended to refactor one editor at a time with thorough testing.

### Next Steps (Phase 1.6 / Phase 3+)

Per the implementation plan:

**Phase 1.6 - Editor Refactoring (when ready):**

- Incrementally refactor items_editor, monsters_editor, spells_editor to use shared components
- Test each editor thoroughly before moving to the next

**Phase 3+ - Layout Continuity & Further Improvements:**

- Update editor layouts for consistency
- Apply AttributePair widgets across all editors
- Improve validation and asset panels

## Phase 2: Extract Editor UI Code from main.rs (2025-01-XX)

**Objective**: Extract Classes and Dialogues editor UI code from `main.rs` into their respective module files, following the standard editor pattern with `show()` methods.

### Background

Per the SDK QOL Implementation Plan (`docs/explanation/sdk_qol_implementation_plan.md`), Phase 2 focuses on extracting UI code from `main.rs` into dedicated editor files. This reduces complexity in `main.rs` and enables test file searches to target specific editor files.

### Changes Implemented

#### 2.1 Classes Editor UI Extraction

Extracted all Classes editor UI from `main.rs` to `classes_editor.rs`:

**Methods Added to `ClassesEditorState`:**

- `show()` - Main UI rendering method following standard signature
- `show_classes_list()` - List panel rendering
- `show_class_form()` - Edit form rendering
- `next_available_class_id()` - ID generation helper
- `load_from_file()` - Load classes from RON file
- `save_to_file()` - Save classes to RON file

**Standard Signature:**

```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    items: &[Item],  // For starting equipment selection
    campaign_dir: Option<&PathBuf>,
    classes_file: &str,
    unsaved_changes: &mut bool,
    status_message: &mut String,
    file_load_merge_mode: &mut bool,
)
```

**Tests Added:**

- `test_classes_editor_state_creation`
- `test_start_new_class`
- `test_save_class_creates_new`
- `test_save_class_empty_id_error`
- `test_save_class_empty_name_error`
- `test_save_class_duplicate_id_error`
- `test_delete_class`
- `test_cancel_edit`
- `test_filtered_classes`
- `test_next_available_class_id`
- `test_start_edit_class`
- `test_edit_class_saves_changes`
- `test_class_edit_buffer_default`
- `test_editor_mode_transitions`

#### 2.2 Dialogues Editor UI Extraction

Extracted all Dialogues editor UI from `main.rs` to `dialogue_editor.rs`:

**Methods Added to `DialogueEditorState`:**

- `show()` - Main UI rendering method following standard signature
- `show_import_dialog_window()` - Import dialog rendering
- `show_dialogue_list()` - List view rendering
- `show_dialogue_form()` - Edit form rendering
- `show_dialogue_nodes_editor()` - Node tree editor
- `show_choice_editor_panel()` - Choice editor panel
- `next_available_dialogue_id()` - ID generation helper
- `load_from_file()` - Load dialogues from RON file
- `save_to_file()` - Save dialogues to RON file

**State Fields Added to `DialogueEditorState`:**

- `show_preview: bool` - Preview toggle (moved from main.rs)
- `show_import_dialog: bool` - Import dialog toggle (moved from main.rs)
- `import_buffer: String` - Import text buffer (moved from main.rs)

**Standard Signature:**

```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    dialogues: &mut Vec<DialogueTree>,
    campaign_dir: Option<&PathBuf>,
    dialogue_file: &str,
    unsaved_changes: &mut bool,
    status_message: &mut String,
    file_load_merge_mode: &mut bool,
)
```

**Tests Added:**

- `test_next_available_dialogue_id`
- Updated existing state creation test to verify new fields

#### 2.3 Auto-load Verification

All editors now auto-load when a campaign is opened:

| Editor     | Load Method                    | Status      |
| ---------- | ------------------------------ | ----------- |
| Items      | `load_items()`                 | ✓           |
| Spells     | `load_spells()`                | ✓           |
| Monsters   | `load_monsters()`              | ✓           |
| Conditions | `load_conditions()`            | ✓           |
| Classes    | `load_classes_from_campaign()` | ✓ (Updated) |
| Maps       | `load_maps()`                  | ✓           |
| Quests     | `load_quests()`                | ✓           |
| Dialogues  | `load_dialogues()`             | ✓           |

#### 2.4 main.rs Simplification

**Removed from `CampaignBuilderApp` struct:**

- `dialogues_search_filter: String`
- `dialogues_show_preview: bool`
- `dialogues_import_buffer: String`
- `dialogues_show_import_dialog: bool`

**Removed Methods:**

- `show_classes_editor()` - Replaced with `classes_editor_state.show()`
- `show_classes_list()` - Moved to `classes_editor.rs`
- `show_class_form()` - Moved to `classes_editor.rs`
- `load_classes()` - Replaced with `load_classes_from_campaign()`
- `save_classes()` - Moved to `classes_editor.rs`
- `show_dialogues_editor()` - Replaced with `dialogue_editor_state.show()`
- `show_dialogue_list()` - Moved to `dialogue_editor.rs`
- `show_dialogue_form()` - Moved to `dialogue_editor.rs`
- `show_dialogue_nodes_editor()` - Moved to `dialogue_editor.rs`
- `load_dialogues_from_file()` - Inlined into `load_dialogues()`
- `save_dialogues_to_file()` - Moved to `dialogue_editor.rs`
- `next_available_dialogue_id()` - Moved to `dialogue_editor.rs`

**Updated Tab Handling:**

```rust
EditorTab::Classes => self.classes_editor_state.show(
    ui,
    &self.items,
    self.campaign_dir.as_ref(),
    &self.campaign.classes_file,
    &mut self.unsaved_changes,
    &mut self.status_message,
    &mut self.file_load_merge_mode,
),
EditorTab::Dialogues => self.dialogue_editor_state.show(
    ui,
    &mut self.dialogues,
    self.campaign_dir.as_ref(),
    &self.campaign.dialogue_file,
    &mut self.unsaved_changes,
    &mut self.status_message,
    &mut self.file_load_merge_mode,
),
```

### Validation

All quality checks pass:

- `cargo fmt --all` - Code formatted
- `cargo check --all-targets --all-features` - No compilation errors
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo test --all-features` - 218 tests pass

### Success Criteria Met

- [x] `main.rs` no longer contains Classes editor UI rendering logic
- [x] `main.rs` no longer contains Dialogues editor UI rendering logic
- [x] `ClassesEditorState::show()` method implemented with consistent signature
- [x] `DialogueEditorState::show()` method implemented with consistent signature
- [x] Auto-load works for all editors when campaign opens
- [x] Test file searches can target specific editor files
- [x] All existing tests continue to pass
- [x] New editor tests added and passing

### Files Modified

- `sdk/campaign_builder/src/classes_editor.rs` - Added `show()` and UI methods, tests
- `sdk/campaign_builder/src/dialogue_editor.rs` - Added `show()` and UI methods, state fields, tests
- `sdk/campaign_builder/src/main.rs` - Removed extracted UI code, updated tab handlers

### Lines of Code Removed from main.rs

Approximately 920 lines of editor-specific UI code were removed from `main.rs` and consolidated into the respective editor modules.

## Phase 0: Conditions Editor — Discovery & Preparation (2025-11-XX)

Summary:

- Completed Phase 0: Discovery & Preparation for the Conditions Editor refactor (toolbar, per-variant effect editor, file I/O).
- Documented all usages of `ConditionEffect` across the codebase, validated runtime integration points, and captured a prioritized list of changes to implement in Phase 1.
- Created a dedicated discovery summary with full audit, references, and sample RON data:
  - `docs/explanation/conditions_editor_phase_0_discovery.md`
- Key outcomes:
  - Identified domain & runtime usage (domain definitions, character/monster effect application, combat engine, and spell effects).
  - Verified `ActiveCondition.magnitude` is a runtime-only field (should not be serialized by the editor).
  - Identified reusable UI components: dice editor (from `spells_editor.rs`), toolbar/import-save patterns (from `items_editor.rs` and `monsters_editor.rs`), and file merge/load patterns.
  - Prepared example RON test data for DOT/HOT/Status/AttributeModifier cases and edge cases (to be used by Phases 1–3).
  - Documented the exact state additions and API shape recommended for the Conditions editor integration (e.g., `ConditionsEditorState::show`, new state fields, `EffectEditBuffer`).
  - Added follow-up task list for Phase 1 through Phase 5 (see discovery doc for details and examples).
- Files created during Phase 0:
  - `docs/explanation/conditions_editor_phase_0_discovery.md` — contains the full audit and next steps.
  - Example/test RON file(s) are prepared for Phase 1 usage (see the discovery doc).
- Next steps (Phase 1 priority):
  - Convert `render_conditions_editor(...)` to `ConditionsEditorState::show(...)` with the toolbar and import/export.
  - Implement load/save/import/merge using patterns from `items_editor.rs` and `monsters_editor.rs`.
  - Add ID uniqueness checks and editor validation similar to existing validators (e.g., `validate_item_ids`).
  - Add per-variant effect editors using the dice editor patterns for DOT/HOT and ComboBox/validation for attributes/elements.

(See the detailed Phase 0 discovery document for complete findings, file references, UI/UX mockups, data samples, and follow-up tasks.)

## Clippy Error Fixes (2025-01-15)

**Objective**: Fix clippy warnings that were treated as errors in the Campaign Builder SDK, ensuring code quality and successful builds.

### Changes Implemented

#### 1. Fixed Empty Line After Doc Comment

**File**: `sdk/campaign_builder/src/main.rs`

**Issue**: Clippy reported `empty_line_after_doc_comments` warning for the `show_maps_editor` function, where there was an empty line immediately after the `///` doc comment.

**Fix**: Removed the empty line after the doc comment to comply with clippy's style expectations.

**Impact**: Eliminates the clippy warning, improving code style consistency.

#### 2. Replaced Match with Matches! Macro

**File**: `sdk/campaign_builder/src/items_editor.rs`

**Issue**: Clippy suggested using the `matches!` macro instead of an explicit `match` statement in the `ItemTypeFilter::matches` method for better readability and conciseness.

**Fix**: Replaced the `match` expression with `matches!` macro:

```rust
matches!(
    (self, &item.item_type),
    (ItemTypeFilter::Weapon, ItemType::Weapon(_)) |
    (ItemTypeFilter::Armor, ItemType::Armor(_)) |
    (ItemTypeFilter::Accessory, ItemType::Accessory(_)) |
    (ItemTypeFilter::Consumable, ItemType::Consumable(_)) |
    (ItemTypeFilter::Ammo, ItemType::Ammo(_)) |
    (ItemTypeFilter::Quest, ItemType::Quest(_))
)
```

**Impact**: Code is more concise and follows clippy's recommendations for pattern matching.

#### 3. Suppressed Too Many Arguments Warnings

**Files**:

- `sdk/campaign_builder/src/items_editor.rs`
- `sdk/campaign_builder/src/spells_editor.rs`
- `sdk/campaign_builder/src/monsters_editor.rs`

**Issue**: Clippy flagged the `show` methods in the editor state structs as having too many arguments (>7 parameters), which is considered a code smell.

**Fix**: Added `#[allow(clippy::too_many_arguments)]` attribute above each `show` function to suppress the warning, as refactoring to reduce parameters would require significant architectural changes beyond the scope of this fix.

**Impact**: Eliminates clippy warnings while preserving existing functionality. Future refactoring could group parameters into context structs.

### Testing

All quality checks now pass:

- ✅ `cargo fmt --all` - Code formatted successfully
- ✅ `cargo check --all-targets --all-features` - Compilation successful
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- ✅ `cargo test --all-features` - 270 tests passed

### Architecture Compliance

- No changes to core domain structures or business logic
- Maintained existing function signatures to avoid breaking changes
- Used clippy allow attributes judiciously only where necessary
- All fixes are localized to the Campaign Builder SDK

### Success Criteria Met

- ✅ All clippy warnings eliminated
- ✅ Code compiles without errors
- ✅ All existing tests continue to pass
- ✅ No functional changes to the application

## ClassDefinition Test Updates (2024-12-01)

**Objective**: Update tests, data files, and documentation examples to include all ClassDefinition fields, ensuring completeness and fixing compilation errors in doc examples.

### Changes Implemented

#### 1. Updated data/classes.ron

**File**: `data/classes.ron`

**Issue**: The RON file was missing the `description`, `starting_weapon_id`, `starting_armor_id`, and `starting_items` fields for all class definitions, relying on serde defaults.

**Fix**: Added appropriate descriptions and default values (None for IDs, empty vec for items) to all six class definitions (Knight, Paladin, Archer, Cleric, Sorcerer, Robber).

**Impact**: Data file now explicitly defines all fields, improving clarity and maintainability.

#### 2. Updated Documentation Examples in src/domain/classes.rs

**File**: `src/domain/classes.rs`

**Issue**: Doc examples for `can_cast_spells`, `disablement_mask`, and `has_ability` methods were missing the `description`, `starting_weapon_id`, `starting_armor_id`, and `starting_items` fields in ClassDefinition initializers, causing compilation errors.

**Fix**: Added the missing fields with appropriate default values to all doc example code blocks.

**Impact**: Documentation examples now compile correctly and serve as runnable tests.

#### 3. Updated RON Data in Test Functions

**File**: `src/domain/classes.rs`

**Issue**: Test functions using RON strings (e.g., `test_class_database_load_from_string`, `test_class_database_get_class`, etc.) were missing the same fields, potentially causing issues if serde defaults were not applied consistently.

**Fix**: Added the missing fields to all RON data strings in test functions, ensuring explicit completeness.

**Impact**: Tests are more robust and consistent with the full struct definition.

### Testing

All existing tests pass:

- ✅ `cargo fmt --all` - Code formatted successfully
- ✅ `cargo check --all-targets --all-features` - Compilation successful
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- ✅ `cargo test --all-features` - 354 tests passed

### Architecture Compliance

- Used existing type aliases (`ItemId`) and patterns
- No changes to core domain structures (ClassDefinition remains unchanged)
- Maintained RON format for data files as per architecture.md Section 7.1
- All fields now properly initialized in examples and tests
- Followed serde default attribute usage for optional fields

### Success Criteria Met

- ✅ All ClassDefinition initializers include all required fields
- ✅ Data file loads successfully with explicit field definitions
- ✅ Documentation examples compile without errors
- ✅ All quality gates pass

## Phase 1: Critical Quest Editor Fixes (2025-11-25)

## Phase 1: Conditions Editor — Core Implementation (Toolbar & File I/O) (2025-11-XX)

### Changes Implemented

- Converted the legacy `render_conditions_editor` module-level function into a `ConditionsEditorState::show(...)` method that mirrors other editors (`Items`, `Monsters`, `Spells`) and accepts the campaign directory, file path, status messages, unsaved change flag, and `file_load_merge_mode`.
- Added editor-level UI state fields to `ConditionsEditorState`:
  - `show_import_dialog: bool` and `import_export_buffer: String` for import/export clipboard flow.
  - `duplicate_dialog_open: bool`, `delete_confirmation_open: bool`, and `selected_for_delete: Option<ConditionId>` for delete/duplicate UX flow.
  - `effect_edit_buffer: Option<EffectEditBuffer>` as a phase-1 placeholder for per-effect editing.
- Implemented a consistent top toolbar for the Conditions editor:
  - Search field, "➕ New Condition" (creates default edit buffer), "🔄 Reload" (loads from campaign folder), "📥 Import", "📂 Load from File" (Open & merge/replace toggle), "💾 Save to File", and "📋 Export Selected/All".
  - Reused `ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH`, `compute_panel_height`, and standard toolbar layout from `items_editor.rs`.
- Implemented import & file I/O patterns:
  - `show_import_dialog(ctx, ...)` allows pasting single `ConditionDefinition` or `Vec<ConditionDefinition>` RON content and will import them, ensuring unique IDs when collisions occur.
  - `Load from File` uses a `rfd::FileDialog` to choose a file; `Merge` toggle supports merging loaded conditions (replace-by-id) or replacing all.
  - `Save to File` supports choosing an arbitrary save path and serializes the conditions as RON using `ron::ser::to_string_pretty`.
  - Implemented a `save_conditions` helper on the Conditions editor that writes to `campaign_dir.join(conditions_file)` for autosave-capable behavior.
- Implemented duplicate and delete:
  - Duplicate clones a selected `ConditionDefinition` and generates a unique id suffix until there’s no conflict.
  - Delete shows a confirmation dialog and removes the selected condition upon confirmation.
- Validation & integration:
  - Added `CampaignBuilderApp::validate_condition_ids()` to check for duplicate IDs and integrated its usage into `validate_campaign()` and into the `load_conditions()` flow so ID collisions are surfaced as validation warnings.
  - Inline validation for new conditions: ID required and uniqueness checked before saving.
- Tests:
  - `test_condition_id_uniqueness_validation` added to `sdk/campaign_builder/src/main.rs` to validate duplicate detection.
  - `test_conditions_save_load_roundtrip` added to verify `CampaignBuilderApp::save_conditions()` / `load_conditions()` round-trip behavior (saves conditions to a temporary campaign directory and loads them back).
- UX notes:
  - `ActiveCondition.magnitude` remains a runtime-only field (not persisted to RON).
  - A placeholder `EffectEditBuffer` was added for Phase 2 per-variant effect editor work.

### Implementation Notes

- Reused many UI and file I/O patterns from existing editors (`items_editor.rs`, `monsters_editor.rs`, and `spells_editor.rs`) to preserve cross-editor UX and to maintain consistent code style.
- The new `ConditionsEditorState::show()` signature is consistent with other editors and is used by `main.rs`:
  - `self.conditions_editor_state.show(ui, &mut self.conditions, self.campaign_dir.as_ref(), &self.campaign.conditions_file, &mut self.unsaved_changes, &mut self.status_message, &mut self.file_load_merge_mode);`
- `render_conditions_editor(ui, state, conditions)` wrapper retained for backward compatibility (it calls `ConditionsEditorState::show()` with sane defaults), easing the transition for code that still referenced the prior function.
- The code respects the domain separation policy: no changes were made to core domain types (`ConditionDefinition`, `ConditionEffect`) or runtime-only fields (`ActiveCondition.magnitude`).

### Tests & Next Steps

- The implemented tests verify RON load/parse, duplicate detection, and load/save roundtrip. Additional tests and UI-level tests will be added in later phases:
  - Per-variant effect editors (Phase 3) will require coverage for dice UI, attribute selectors, and effect list manipulation.
  - UI integration tests for import/export clipboard, file load/merge behavior, and duplicate/delete confirmation behavior.
- Phase 2 implementation completed; the next step is Phase 3 — Effects List & Basic Effect Editing.

## Phase 2: ConditionDefinition Core Editing Fields (2025-11-XX)

### Changes Implemented

- Core editing approach:

  - Introduced `editing_original_id: Option<ConditionId>` and kept `edit_buffer: Option<ConditionDefinition>` as an editing staging area to support both "create new" and "edit existing" flows consistently.
  - Left-list selection sets only `selected_condition_id`. Users open the editor using the "✏️ Edit" or "➕ New Condition" actions which populate `edit_buffer`.
  - The editor uses the `edit_buffer` to present editable fields for both new and existing condition edits; changes are applied or rejected atomically when saving.

- New UI fields & behavior added:

  - ID (editable): text field with uniqueness checks on Save; ID cannot be empty.
  - Name: text field (no domain change).
  - Description: multiline text field (unchanged but editable).
  - Default Duration: ComboBox with variants:
    - Instant
    - Rounds(u16) — numeric input for rounds
    - Minutes(u16) — numeric input for minutes
    - Permanent
  - Icon ID: Optional string text field with `Clear` button (entered `None` when cleared).
  - Effects: Read-only display in Phase 2 (Phase 3 will provide the effect list editing UI).

- Save path:

  - Introduced `apply_condition_edits(conditions, original_id, edited)` helper (module-level function) that validates IDs and handles:
    - Updating an existing condition (if `original_id` is Some) while ensuring ID changes do not collide.
    - Inserting a new condition (if `original_id` is None) ensuring ID uniqueness.
  - `apply_condition_edits` returns a descriptive error string on failure, enabling the editor to show inline status and not apply invalid saves.

- Editing workflow:

  - `Edit` button: The right-side details view exposes an "✏️ Edit" button which populates the `edit_buffer` and `editing_original_id`.
  - Save button: Calls `apply_condition_edits` with `editing_original_id.as_deref()` and the editing buffer; if success, updates the conditions collection and selection, clears the edit buffer and original id; otherwise shows an error in the status.
  - Cancel button: Discards changes and clears the edit buffer.

- Validation & messaging:
  - ID must be non-empty and unique (except when editing the same record).
  - The editor shows inline messages and status updates on validation or save errors.

### Tests & Validation

- Added focused unit tests for the `apply_condition_edits` helper that cover:
  - Insertion success
  - Update success (including ID change)
  - Update failure due to collision with existing IDs
  - Insertion failure due to duplicate IDs
- These tests ensure the logic applied by the UI is consistent and avoids data corruption.

### Outcome & Acceptance Criteria

- The right-side editor now supports editing and saving of the core `ConditionDefinition` fields (ID, name, description, icon id, and default duration).
- The editing workflow is consistent for both new and existing conditions, with atomic validation & save semantics and clear error messages for ID collisions and missing fields.
- The `apply_condition_edits` helper centralizes edit validation logic making it easier to test and maintain.
- Tests exist that verify the core editing logic works and prevents invalid updates.

## Phase 3: Effects List & Basic Effect Editing (2025-12-02)

Summary:

- Implemented the Phase 3 changes to the Conditions Editor in the Campaign Builder SDK to support an editable Effects list and basic per-effect editors for the canonical `ConditionEffect` variants.

Changes Implemented:

- UI & State:

  - Implemented an interactive Effects list inside the editor's `edit_buffer` flow within `ConditionsEditorState::show(...)`. The list supports:
    - Add Effect (default: `StatusEffect`),
    - Edit Effect (invokes an `EffectEditBuffer` typed editor),
    - Duplicate Effect (clone and insert copy),
    - Delete Effect,
    - Reorder (⬆️ / ⬇️) effects.
  - Effects editing happens within the `edit_buffer` context (changes persist to the in-memory `edit_buffer` immediately; the whole `ConditionDefinition` must be saved with the existing "Save" button to commit into the campaign).
  - Added the `EffectEditBuffer` state to hold typed edit values:
    - `effect_type: Option<String>` - current variant string,
    - `editing_index: Option<usize>` - index within parent `effects` vector if editing an existing effect,
    - `attribute: String`, `attribute_value: i16` - for `AttributeModifier`,
    - `status_tag: String` - for `StatusEffect`,
    - `dice: DiceRoll`, `element: String` - for `DamageOverTime` and `HealOverTime`.
  - Created `render_condition_effect_summary()` to provide concise UI summaries for effect preview lists.

- Per-Variant Editors:

  - `AttributeModifier`: attribute selector (default attribute names suggested) + signed value via `egui::DragValue`.
  - `StatusEffect`: text field for the status tag (suggested tags in a ComboBox with a 'Custom' option).
  - `DamageOverTime`: re-used the dice UI (count, sides, bonus) and element selection/Custom element text input.
  - `HealOverTime`: re-used dice UI for healing amounts.

- Reuse & Design:

  - Reused `DiceRoll` UI design (from `spells_editor.rs`) for DOT/HOT editing.
  - Adopted the attribute & element suggestion lists from project documentation for UI convenience but kept text-edit freedom (no strict enforcement of enumerations).
  - Preserved the Domain separation: editor changes only touch the UI & ephemeral `edit_buffer` and the per-session `conditions` collection — no domain type changes were performed.

- New editor helpers (module-level `sdk/campaign_builder/src/conditions_editor.rs`):
  - `add_effect_to_condition(condition, effect)` - Append effect to `ConditionDefinition::effects`.
  - `update_effect_in_condition(condition, index, effect)` - In-place update of an effect.
  - `delete_effect_from_condition(condition, index)` - Remove effect at index.
  - `duplicate_effect_in_condition(condition, index)` - Clone and insert at index+1.
  - `move_effect_in_condition(condition, index, dir)` - Reorder effects (-1 or +1).
  - These helpers are `pub(crate)` so they can be tested against the editor state.

Testing & Validation:

- Added unit tests validating basic effect operations (helper functions):
  - `test_condition_effect_helpers_success_flow` — adds, duplicates, moves, updates, and deletes effects successfully.
  - `test_update_effect_out_of_range` — verifies errors are returned if effect index is invalid.
- Editor-level tests and UI interactions will be expanded in future phases to cover the full end-to-end Add/Edit/Delete flows, import/export merge behaviors, dice validation, attribute/element suggestions, and preview interactions.
- Maintained `apply_condition_edits(...)` semantics (atomic validation and save semantics for the `ConditionDefinition`) and ensured that effect edits remain within `edit_buffer` until the overarching Save is performed.

UX Details and Notes:

- The editing workflow honors user expectations in other editors:
  - Selection vs Edit separation: selecting a condition shows a read-only preview; clicking ✏️ Edit opens the `edit_buffer` to edit fields and effects.
  - Effects are visible in read-only preview with a concise summary string; the edit UI is only available inside the active `edit_buffer`.
- The Edit Effect panel allows choosing a variant and editing variant-specific fields with inline layout & Save Effect / Cancel buttons.
- Add Effect opens the same editor panel in "create" mode with `editing_index = None` and defaults to `StatusEffect`.
- Saving an effect updates the `edit_buffer`'s `effects` list (not the persisted campaign) until the `ConditionDefinition` is saved by calling `apply_condition_edits`.
- Effects editing preserves the `ActiveCondition.magnitude` runtime-only semantics — this runtime-only field is not part of saved RON data.

Files & Artifacts:

- `sdk/campaign_builder/src/conditions_editor.rs`:
  - `ConditionsEditorState::show(...)` updated with the Effects list UI & effect edit panel using `EffectEditBuffer`.
  - `EffectEditBuffer` structure added (typed fields).
  - New helper functions for effect operations and `render_condition_effect_summary`.
- `sdk/campaign_builder/src/main.rs`:
  - Added unit tests for effect helper functions.
- No domain model changes (no modifications to `antares/src/domain/conditions.rs`) — the domain remains the source of truth with `ConditionEffect` variants unchanged.

Phase 4: Per-Variant Effect Editor Implementation & UX Polish (2025-12-02)

Summary

- Per-variant editors fully implemented for all canonical `ConditionEffect` variants:
  - `AttributeModifier`: attribute ComboBox + optional custom attribute text + signed `DragValue`
  - `StatusEffect`: tag text input (non-empty validation)
  - `DamageOverTime`: dice editor (count, sides, bonus) + element ComboBox + custom element text
  - `HealOverTime`: dice editor (count, sides, bonus)
- Inline validation improvements:
  - `ConditionDefinition` ID uniqueness and non-empty ID checks in the editor (inline hint and prevented saves).
  - Per-effect validations (dice ranges: count >= 1, sides >= 2; attribute value range checks; non-empty tags/elements).
  - Disabled Save Effect button and inline error messages for invalid fields in the per-variant effect editor.
- Preview & UX:
  - Added a `Preview` pane with an interactive `Magnitude` slider (simulation-only) that displays scaled effect summaries:
    - Attribute modifiers scaled by magnitude (rounded).
    - DOT/HOT average per-tick scaled by magnitude (approximate, shown as single-line summary).
  - `Used by` panel displays spells referencing the selected condition (lists spell names).
    - Each referencing spell includes a small "Copy" button to copy the spell name to clipboard and provide a quick status message.
    - Delete confirmation highlights referencing spells and offers the option to "Remove references from spells when deleting".
  - Tooltips & hover help added to attribute & element selectors in the per-variant editor for discoverability.
  - Effects list now supports Add / Edit / Duplicate / Delete / Reorder (Up/Down) with immediate edit buffer support and Save/Cancel semantics.
- Helper APIs & Tests:
  - Added UI and logic helper functions:
    - `validate_effect_edit_buffer` (per-variant UI validation).
    - `validate_condition_definition` (complete ConditionDefinition-level validation).
    - `render_condition_effect_preview` (human-readable preview with magnitude scaling).
    - `spells_referencing_condition` and `remove_condition_references_from_spells` for "Used by" detection / cleanup.
    - Effect list helpers: `add_effect_to_condition`, `update_effect_in_condition`, `duplicate_effect_in_condition`, `delete_effect_from_condition`, `move_effect_in_condition`.
  - Unit & integration coverage:
    - Tests added for effect list helpers, effect validation failures (e.g., invalid dice), `apply_condition_edits` validations, and spells referencing & removal logic.
- Domain Preservation:
  - No structural changes to `ConditionDefinition`, `ConditionEffect`, or `ActiveCondition` domain types were made (domain remains authoritative).
  - `ActiveCondition.magnitude` remains runtime-only and is never persisted to RON; the preview is a UX-only simulation.
- Deliverables & Impact:
  - A significantly more usable and discoverable Conditions Editor where designers can edit complex condition effects with immediate validation and preview.
  - Clear warnings and actionable UI when conditions are referenced by existing spells (preventing accidental broken references).
  - Reusable helper functions make effects manipulation easier to test and maintain.

Next Steps

- Add keyboard shortcuts and/or context menu for reorder/duplicate actions.
- Consider adding undo/redo for effect and condition edits (requires integration with existing undo stack).
- Add editor-level integration tests for end-to-end Add / Edit / Delete flows (covering import/export and spells that reference conditions).
- Explore in-editor quick navigation to referenced spells (open spells editor and select the spell), and a safer "rename with references update" flow.

### Changes Implemented

- Converted the legacy `render_conditions_editor(...)` module-level function into a `ConditionsEditorState::show(...)` method that mirrors other editors (`Items`, `Monsters`, `Spells`) and accepts the campaign directory, file path, status messages, unsaved change flag, and `file_load_merge_mode`.
- Added editor-level UI state fields to `ConditionsEditorState`:
  - `show_import_dialog: bool` and `import_export_buffer: String` for import/export clipboard flow.
  - `duplicate_dialog_open: bool`, `delete_confirmation_open: bool`, and `selected_for_delete: Option<ConditionId>` for delete/duplicate UX flow.
  - `effect_edit_buffer: Option<EffectEditBuffer>` as a phase-1 placeholder for per-effect editing.
- Implemented a consistent top toolbar for the Conditions editor:
  - Search field, "➕ New Condition" (creates default edit buffer), "🔄 Reload" (loads from campaign folder), "📥 Import", "📂 Load from File" (Open & merge/replace toggle), "💾 Save to File", and "📋 Export Selected/All".
  - Reused `ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH`, `compute_panel_height`, and standard toolbar layout from `items_editor.rs`.
- Implemented import & file I/O patterns:
  - `show_import_dialog(ctx, ...)` allows pasting single `ConditionDefinition` or `Vec<ConditionDefinition>` RON content and will import them, ensuring unique IDs when collisions occur.
  - `Load from File` uses a `rfd::FileDialog` to choose a file; `Merge` toggle supports merging loaded conditions (replace-by-id) or replacing all.
  - `Save to File` supports choosing an arbitrary save path and serializes the conditions as RON using `ron::ser::to_string_pretty`.
  - Implemented a `save_conditions` helper on the Conditions editor that writes to `campaign_dir.join(conditions_file)` for autosave-capable behavior.
- Implemented duplicate and delete:
  - Duplicate clones a selected `ConditionDefinition` and generates a unique id suffix until there’s no conflict.
  - Delete shows a confirmation dialog and removes the selected condition upon confirmation.
- Validation & integration:
  - Added `CampaignBuilderApp::validate_condition_ids()` to check for duplicate IDs and integrated its usage into `validate_campaign()` and into the `load_conditions()` flow so ID collisions are surfaced as validation warnings.
  - Inline validation for new conditions: ID required and uniqueness checked before saving.
- Tests:
  - `test_condition_id_uniqueness_validation` added to `sdk/campaign_builder/src/main.rs` to validate duplicate detection.
  - `test_conditions_save_load_roundtrip` added to verify `CampaignBuilderApp::save_conditions()` / `load_conditions()` round-trip behavior (saves conditions to a temporary campaign directory and loads them back).
- UX notes:
  - `ActiveCondition.magnitude` remains a runtime-only field (not persisted to RON).
  - A placeholder `EffectEditBuffer` was added for Phase 2 per-variant effect editor work.

### Implementation Notes

- Reused many UI and file I/O patterns from existing editors (`items_editor.rs`, `monsters_editor.rs`, and `spells_editor.rs`) to preserve cross-editor UX and to maintain consistent code style.
- The new `ConditionsEditorState::show()` signature is consistent with other editors and is used by `main.rs`:
  - `self.conditions_editor_state.show(ui, &mut self.conditions, self.campaign_dir.as_ref(), &self.campaign.conditions_file, &mut self.unsaved_changes, &mut self.status_message, &mut self.file_load_merge_mode);`
- `render_conditions_editor(ui, state, conditions)` wrapper retained for backward compatibility (it calls `ConditionsEditorState::show()` with sane defaults), easing the transition for code that still referenced the prior function.
- The code respects the domain separation policy: no changes were made to core domain types (`ConditionDefinition`, `ConditionEffect`) or runtime-only fields (`ActiveCondition.magnitude`).

### Tests & Next Steps

- The implemented tests verify RON load/parse, duplicate detection, and load/save roundtrip. Additional tests and UI-level tests will be added in later phases:
  - Per-variant effect editors (Phase 2/3) will require coverage for dice UI, attribute selectors, and effect list manipulation.
  - UI integration tests for import/export clipboard, file load/merge behavior, and duplicate/delete confirmation behavior.
- Phase 2 will implement per-variant (effect) editing UI, the dice editor reuse, and more validation regarding attribute/element names.

**Objective**: Restore basic quest editing functionality in the Campaign Builder SDK.

### Changes Implemented

#### 1.1 Removed Duplicate Stage Editor Call

**File**: `sdk/campaign_builder/src/main.rs`

**Issue**: The `show_quest_form` method was calling `show_quest_stages_editor(ui)` twice (lines 5187 and 5190), causing UI ID clashes that prevented proper quest editing.

**Fix**: Removed the duplicate call at line 5189-5190.

**Impact**: Eliminates UI ID conflicts, allowing the quest editor to function properly without egui ID collision errors.

#### 1.2 Fixed Selected Stage Tracking

**File**: `sdk/campaign_builder/src/main.rs`

**Issue**: The `selected_stage` field was never set when viewing stages, blocking the ability to add objectives to stages. The "Add Objective" button requires `selected_stage` to be set to know which stage to add the objective to.

**Fix**: Added tracking logic in `show_quest_stages_editor` to set `selected_stage` when a stage collapsing header is clicked or opened:

```rust
// Track which stage is expanded for objective addition
if header.header_response.clicked() || header.body_returned.is_some() {
    self.quest_editor_state.selected_stage = Some(stage_idx);
}
```

**Impact**: Users can now add objectives to stages by expanding the stage header first, then clicking the "Add Objective" button.

#### 1.3 Fixed Quest ID Auto-Population

**Files**:

- `sdk/campaign_builder/src/main.rs` (call site)
- `sdk/campaign_builder/src/quest_editor.rs` (already had the parameter)

**Issue**: When creating a new quest, the ID field was not auto-populated with the next available ID, requiring users to manually determine and enter the ID.

**Fix**: Modified the "New Quest" button handler to compute and pass the next available quest ID:

```rust
if ui.button("➕ New Quest").clicked() {
    let next_id = self.next_available_quest_id();
    self.quest_editor_state.start_new_quest(next_id.to_string());
    self.unsaved_changes = true;
}
```

The `start_new_quest` method already accepted a `next_id: String` parameter and populated the buffer with it, so only the call site needed updating.

**Impact**: New quests automatically receive the next available ID, improving UX and preventing ID conflicts.

### Testing

All existing tests pass:

- ✅ `cargo fmt --all` - Code formatted successfully
- ✅ `cargo check --all-targets --all-features` - Compilation successful
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- ✅ `cargo test --all-features` - 212 tests passed

Updated test in `main.rs` to pass the next available ID to `start_new_quest()` method, matching the updated signature.

### Success Criteria Met

- ✅ Can create a new quest with auto-generated ID
- ✅ Can add stages without UI ID clashes
- ✅ Can add objectives to any stage (by expanding the stage first)
- ✅ Quest Save button successfully persists changes (existing functionality preserved)

### Architecture Compliance

- Used existing type aliases (`QuestId`)
- Followed existing patterns for ID generation (`next_available_quest_id()`)
- No changes to core domain structures
- Maintained separation of concerns between UI and state management
- All changes localized to the Campaign Builder SDK

### Next Steps

Phase 2 of the SDK UI Improvements plan can now proceed, focusing on:

- Classes Editor enhancements
- Pre-populating classes from campaign directory
- Adding description and starting equipment fields

### Phase 2.1: Shared UI Helper and Editor Refactor (2025-11-29)

Objective:
Introduce a shared UI helper module to centralize layout logic for editor panels and refactor existing editors to use it, improving consistency and maintainability.

Summary:

- Added `sdk/campaign_builder/src/ui_helpers.rs` which exposes:
  - `pub const DEFAULT_LEFT_COLUMN_WIDTH: f32` — default left column width used in editors (300.0).
  - `pub const DEFAULT_PANEL_MIN_HEIGHT: f32` — default minimum panel height to avoid collapse (100.0).
  - `pub fn compute_panel_height(ui: &mut egui::Ui, min_height: f32) -> f32` — computes a panel height using `ui.available_size_before_wrap()`.
  - `pub fn compute_panel_height_from_size(size: egui::Vec2, min_height: f32) -> f32` — pure function that computes height from a size, suitable for unit tests.
- Declared the module in `main.rs` (added `mod ui_helpers;`).
- Refactored the following editors to use the new constants and helpers:
  - `sdk/campaign_builder/src/items_editor.rs`
  - `sdk/campaign_builder/src/monsters_editor.rs`
  - `sdk/campaign_builder/src/spells_editor.rs`
  - `sdk/campaign_builder/src/conditions_editor.rs`
- Refactor details:
  - Replace manual `available_size` computations with calls to `crate::ui_helpers::compute_panel_height(...)`.
  - Replace hard-coded left column width `300.0` with `crate::ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH`.
  - Use the `panel_height` computed from the helper to set `ui.set_min_height(panel_height)` and `egui::ScrollArea::vertical().max_height(panel_height)`.
- Tests:
  - Added unit tests for `compute_panel_height_from_size` in `ui_helpers.rs` to validate pure logic and ensure it respects the minimum height threshold.
- Rationale:
  - Centralizes layout defaults and logic so multiple editors show consistent behavior when windows are resized.
  - Enables developers to maintain UI constants and adjustments in one place.
  - Facilitates unit testing for layout computation logic.

Files changed (high-level):

- `sdk/campaign_builder/src/ui_helpers.rs` — new module with constants, helpers, and tests
- `sdk/campaign_builder/src/main.rs` — `mod ui_helpers;` was added to the module list
- `sdk/campaign_builder/src/items_editor.rs` — refactored to use `compute_panel_height` and `DEFAULT_LEFT_COLUMN_WIDTH`
- `sdk/campaign_builder/src/monsters_editor.rs` — refactored to use `compute_panel_height` and `DEFAULT_LEFT_COLUMN_WIDTH`
- `sdk/campaign_builder/src/spells_editor.rs` — refactored to use `compute_panel_height` and `DEFAULT_LEFT_COLUMN_WIDTH`
- `sdk/campaign_builder/src/conditions_editor.rs` — refactored to use `compute_panel_height` and `DEFAULT_LEFT_COLUMN_WIDTH`

Validation:

- The campaign builder package compiles and the test suite for the package passes.
- The shared helper uses a pure function (`compute_panel_height_from_size`) that is unit tested and isolates the logic for correct behavior under varying sizes.

Follow-up:

- Are there other editors or UI regions you’d like refactored to use `ui_helpers` for consistent layout behavior? If so, I’ll update them in the same way and add any additional tests necessary.

## Phase 2: Campaign Builder UI - List & Preview Panel Scaling (2025-11-29)

Objective:
Ensure the items, monster, spell, and conditions editor list and details/preview panels expand with the window height so users can view more rows without unnecessary vertical scrolling.

Summary:

- Problem: The editors' list and preview columns were constrained to a small height because they computed the height using `ui.available_height()` inside a nested `ui.horizontal(|ui| { ... })` closure. In some UI scenarios this returns a small height because the layout hadn't yet allocated the full available panel area, resulting in only a few rows visible even with a large window.
- Root cause: calculating height from the nested `ui` instead of from the parent UI/context meant `available_height()` could be lower than expected and would not grow to fill the window.
- Fix implemented:
  - Compute available vertical space before starting the horizontal split using `ui.available_size_before_wrap()` and derive a `panel_height` with a sensible minimum: `let panel_height = ui.available_size_before_wrap().y.max(100.0)`.
  - For both the list and preview columns:
    - Use `ui.set_min_height(panel_height)` on the vertical containers so they use this minimum and grow with the window.
    - Add `.max_height(panel_height)` to the `egui::ScrollArea::vertical()` builder to allow it to expand up to the panel height while still allowing scrolling when content exceeds that.
  - Apply these changes consistently across the three editors to normalize behavior:
    - `sdk/campaign_builder/src/items_editor.rs`
    - `sdk/campaign_builder/src/monsters_editor.rs`
    - `sdk/campaign_builder/src/spells_editor.rs`
  - Where appropriate, the preview `show_preview` scroll areas were also updated to use `.max_height(panel_height)` so preview content will expand with the window.

Test & Validation:

- Confirmed compile (`cargo check`) and campaign-builder package unit tests (`cargo test -p campaign_builder`) are unchanged and still pass.
- Visual verification is still recommended: enlarge the Campaign Builder window and confirm that the lists/preview panels grow vertically and show additional rows before the scrollbar becomes necessary.
- This pattern should be applied to other editor split layouts where list and details panes are set side-by-side and should scale with the window (e.g., classes editor, quest editor, etc.) as a follow-up task.

Notes:

- This change is intentionally minimal and localized to UI layout improvements.
- The implementation avoids changing domain data structures or the editor state shape.
- If there are other UI areas that exhibit similar fixed-height behavior, extend the same pattern to them.

## Conditions Editor QoL Improvements (2025-01-XX)

**Objective**: Enhance the Conditions Editor with quality-of-life features for improved usability and workflow efficiency.

### Changes Implemented

#### 1. Effect Type Filter

**Files**: `sdk/campaign_builder/src/conditions_editor.rs`

- Added `EffectTypeFilter` enum with variants: `All`, `AttributeModifier`, `StatusEffect`, `DamageOverTime`, `HealOverTime`
- Filter dropdown in toolbar allows filtering conditions by effect type
- Selecting a filter automatically clears the current selection
- Filter uses the `matches()` method to check if any effect in a condition matches the selected type

#### 2. Sort Order Options

- Added `ConditionSortOrder` enum with variants: `NameAsc`, `NameDesc`, `IdAsc`, `IdDesc`, `EffectCount`
- Sort dropdown in toolbar allows sorting conditions by name, ID, or effect count
- Sorting is applied after filtering for consistent results

#### 3. Effect Type Indicators in List

- Conditions list now shows emoji indicators for the primary effect type:
  - `⬆` = Buff (positive attribute modifier)
  - `⬇` = Debuff (negative attribute modifier)
  - `◆` = Status effect
  - `🔥` = Damage over time
  - `💚` = Heal over time
  - `○` = Empty (no effects)

#### 4. Statistics Panel

- Added optional statistics panel (toggle via "Stats" checkbox in toolbar)
- Shows counts: Attribute modifiers, Status effects, DOT, HOT, Empty conditions, Multi-effect conditions
- Hovering over "Total" count shows condensed statistics tooltip

#### 5. Jump to Spell Navigation

**Files**: `sdk/campaign_builder/src/conditions_editor.rs`, `sdk/campaign_builder/src/main.rs`

- Added "→" button next to referenced spell names in "Used by" panel
- Clicking the button sets `navigate_to_spell` request in editor state
- Main app handles navigation request: switches to Spells tab and selects the spell
- Works in both view mode and edit mode for conditions

#### 6. Clear All Effects Button

- Added "🗑️ Clear All" button next to the Effects label when editing a condition
- Button only appears when the condition has effects
- Includes hover tooltip explaining the action

#### 7. Enhanced Tooltips

Added descriptive tooltips throughout the editor:

- Filter and Sort dropdowns explain their purpose
- Duration types: Explains Instant, Rounds, Minutes, Permanent
- Icon ID field: Explains it's optional for UI display
- Preview panel: Explains magnitude scaling is for design visualization only
- Remove refs on delete: Explains the automatic reference removal feature

### State Changes

Added to `ConditionsEditorState`:

- `filter_effect_type: EffectTypeFilter` - Current effect type filter
- `sort_order: ConditionSortOrder` - Current sort order
- `show_statistics: bool` - Whether statistics panel is visible
- `navigate_to_spell: Option<String>` - Navigation request for jump-to-spell

### Tests Added

- `test_effect_type_filter_matches` - Verifies filter matching logic for all effect types
- `test_condition_sort_order_as_str` - Verifies sort order display names
- `test_condition_statistics_computation` - Verifies statistics calculation
- `test_conditions_editor_navigation_request` - Verifies navigation request handling
- `test_conditions_editor_state_qol_defaults` - Verifies default values for new fields
- `test_effect_type_filter_all_variants` - Verifies filter enum variants
- `test_effect_type_filter_as_str` - Verifies filter display names

### Validation

- All cargo quality checks pass:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test -p campaign_builder --bin campaign-builder` (291 tests pass)

### Architecture Compliance

- No domain model changes
- All changes confined to SDK/editor code
- New types (`EffectTypeFilter`, `ConditionSortOrder`, `ConditionStatistics`) are UI-only
- Navigation request pattern allows loose coupling between editors via app state

### Future Improvements Suggested

- Undo/redo integration for condition edits (hook into global `UndoRedoManager`)
- Collapsible sections for Effects and Preview panels
- Effect templates/presets for common effect combinations
- Better visual feedback for ID uniqueness (green checkmark when unique)
- Multi-select for batch operations

## Database Placeholder Implementation & QoL Improvements (2025-01-XX)

**Objective**: Complete the game engine's database loading implementation and add quality-of-life improvements for compatibility with the Campaign Builder SDK.

### Background

The database placeholder implementation plan (`docs/explanation/database_placeholder_implementation_plan.md`) addressed the separation between:

- **Campaign Builder SDK** (`sdk/campaign_builder/src/main.rs`) - Editor UI for creating/modifying content (already working)
- **Content Database** (`src/sdk/database.rs`) - Runtime game engine content loading (needed implementation)

### Changes Implemented

#### 1. Monster Serde Defaults for Cleaner Campaign Data

**File**: `src/domain/combat/monster.rs`

Added `#[serde(default)]` attributes to Monster runtime state fields:

- `flee_threshold`
- `special_attack_threshold`
- `resistances`
- `can_regenerate`
- `can_advance`
- `is_undead`
- `magic_resistance`
- `conditions` (runtime state, defaults to Normal)
- `active_conditions` (runtime state, defaults to empty)
- `has_acted` (runtime state, defaults to false)

**Impact**: Campaign data files can now omit these fields and they'll default to sensible values. This makes hand-edited RON files cleaner and allows backward compatibility when new fields are added.

#### 2. ConditionDatabase Added to ContentDatabase

**File**: `src/sdk/database.rs`

Added `ConditionDatabase` struct with methods:

- `new()` - Creates empty database
- `load_from_file()` - Loads conditions from RON file
- `get_condition()` - Get condition by ID
- `get_condition_by_name()` - Get condition by name (case-insensitive)
- `all_conditions()` - List all condition IDs
- `count()` - Number of conditions
- `has_condition()` - Check if condition exists
- `add_condition()` - Add condition to database

**Integration**:

- Added `conditions: ConditionDatabase` field to `ContentDatabase`
- Updated `ContentDatabase::load_campaign()` to load `conditions.ron`
- Updated `ContentDatabase::load_core()` to load `conditions.ron`
- Added `condition_count` to `ContentStats`

#### 3. QoL Helper Methods for All Databases

Added convenient query methods to make database usage more ergonomic:

**SpellDatabase**:

- `get_spell_by_name()` - Case-insensitive name lookup
- `spells_by_school()` - Filter by spell school
- `spells_by_level()` - Filter by spell level

**MonsterDatabase**:

- `get_monster_by_name()` - Case-insensitive name lookup
- `undead_monsters()` - Get all undead monsters
- `monsters_by_experience_range()` - Filter by XP range

**QuestDatabase**:

- `get_quest_by_name()` - Case-insensitive name lookup
- `main_quests()` - Get all main quests
- `repeatable_quests()` - Get all repeatable quests
- `quests_for_level()` - Get quests available at a given level

**ConditionDatabase**:

- `get_condition_by_name()` - Case-insensitive name lookup

**DialogueDatabase**:

- `get_dialogue_by_name()` - Case-insensitive name lookup
- `repeatable_dialogues()` - Get all repeatable dialogues
- `dialogues_for_quest()` - Get dialogues associated with a quest

#### 4. DatabaseError Enhancement

Added new error variant:

- `ConditionLoadError(String)` - For condition loading failures

#### 5. Doctest Fix

**File**: `src/domain/combat/database.rs`

Fixed `MonsterDefinition` doctest example to include the new default fields (`conditions`, `active_conditions`, `has_acted`).

### Tests Added

- `test_condition_database_new()` - Verifies empty database creation
- `test_condition_database_load_from_file()` - Tests RON loading with multiple conditions
- `test_condition_database_load_nonexistent_file()` - Verifies graceful handling of missing files

### Validation

- All cargo quality checks pass:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test -p antares --lib` (370 tests pass)
  - `cargo test -p campaign_builder --bin campaign-builder` (291 tests pass)

### Campaign Builder Compatibility

The Campaign Builder and game engine database now use compatible formats:

- Both load `conditions.ron` for condition definitions
- Spell `applied_conditions` field uses `#[serde(default)]` for backward compatibility
- Monster files can omit runtime state fields (they'll default appropriately)

### Architecture Compliance

- No breaking changes to public API
- Condition loading follows existing patterns from Spell/Monster/Quest databases
- Error handling uses existing `DatabaseError` enum pattern
- All new fields have `#[serde(default)]` for backward compatibility

## Conditions Editor Edit Mode Bug Fix (2025-01-XX)

**Objective**: Fix bug where clicking "Edit" on an existing condition only allowed editing name and description, not duration, icon, or effects.

### Problem

When selecting a condition from the list and clicking "Edit", users could only edit the `name` and `description` fields. The full editor (with duration, icon_id, and effects editing) was not shown.

### Root Cause

The right panel rendering had an if-else chain that checked `selected_condition_id` first:

```rust
if let Some(condition_id) = &self.selected_condition_id.clone() {
    // READ-ONLY view - only name/description editable inline
} else if self.edit_buffer.is_some() {
    // FULL EDITOR - all fields editable including effects
}
```

When the "Edit" button was clicked, it correctly populated:

- `edit_buffer` with the condition clone
- `editing_original_id` with the original condition ID

However, it did **not** clear `selected_condition_id`. This caused the first branch to still match, showing the read-only view instead of the full editor.

### Fix

**File**: `sdk/campaign_builder/src/conditions_editor.rs`

Added one line to clear the selection when entering edit mode:

```rust
if let Some(cond) = conditions.iter().find(|c| c.id == edit_id) {
    self.edit_buffer = Some(cond.clone());
    self.editing_original_id = Some(edit_id);
    // Clear selection so the full editor panel shows instead of read-only view
    self.selected_condition_id = None;
}
```

### Impact

- Users can now fully edit existing conditions from `conditions.ron`
- All fields are editable: ID, name, description, default duration, icon ID, and effects
- Effects can be added, edited, duplicated, deleted, and reordered
- Preview and validation work correctly in edit mode

### Testing

- All quality checks pass:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test -p campaign_builder conditions` (5 tests pass)
  - `cargo test --all-features` (all tests pass)

### Architecture Compliance

- No changes to domain types
- Single-line fix localized to UI state management
- Follows existing editor patterns for edit mode transitions
