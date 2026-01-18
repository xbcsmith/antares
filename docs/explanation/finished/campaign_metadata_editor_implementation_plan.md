## Plan: Campaign Metadata Editor

TL;DR
Build a new `campaign_editor.rs` module to host a focused Campaign Metadata editor using a `TwoColumnLayout`. Extract metadata-related UI from `main.rs`, and provide a form-based editor (right panel) with a left column that lists metadata sections (overview, files, gameplay settings, limits). Implement robust load/save/import/export and validation checks, reusing as much as possible of existing validation and UI code.

**Steps**

1. Create `src/campaign_editor.rs` and `CampaignMetadataEditorState`.
2. Extract metadata-related UI and helper logic from `src/main.rs`.
3. Implement TwoColumn layout and field groups for editing `campaign.ron`.
4. Add Save/Load/Import/Export operations with `ron` serialization.
5. Add configuration validations and tests, integrate with `validate_campaign`.
6. Polishing: UX, accessibility, docs, and CI tests.

**Open Questions**

1. Should metadata editing be per campaign file only, or also support inline per-campaign instances? File-only
2. Preferred left column: list of field groups (Overview/Files/Gameplay) or one-row list? Flat
3. Should Save auto-exit back to list or remain in edit mode? Remain

---

# Campaign Metadata Editor Implementation Plan

## Overview

Create a maintainable, dedicated editor for `CampaignMetadata` (defined in `campaigns/tutorial/campaign.ron`) with a `TwoColumnLayout`. This editor will replace the bulky metadata editing code in `main.rs` and provide improved UX, validation, and test coverage.

Goals:

- Clear single-responsibility module: `sdk/campaign_builder/src/campaign_editor.rs`
- `TwoColumnLayout` UI: left panel lists metadata sections, right panel shows form to edit fields
- Modern UX consistent with other editors: bottom action row (Save, Cancel, Back) and validation integration
- Tests and documentation for maintainability
- Reduce `main.rs` complexity by moving metadata logic out

## Current State Analysis

### Existing Infrastructure

- `src/main.rs` contains validation functions and campaign metadata UI logic; `validate_campaign()` and `validate_*` helpers for ID checks exist.
- `validation.rs` defines `ValidationCategory::Configuration` and related data models.
- `ui_helpers.rs`, `items_editor.rs`, `races_editor.rs`, `characters_editor.rs` provide layout & common components (e.g., `TwoColumnLayout`, `EditorToolbar`, `ActionButtons`).
- `campaigns/tutorial/campaign.ron` is the sample campaign metadata instance used for testing.

### Identified Issues

- `main.rs` currently houses large, unwieldy metadata UI code, making it difficult to add maintainable features.
- No single dedicated `campaign_editor.rs` module to edit `CampaignMetadata`.
- Validation checks for `Configuration` are missing or sparse; integration is inconsistent.
- UI layout is inconsistent and lacks a consistent TwoColumn pattern for metadata fields.
- Lack of tests and clear separation of concerns.

## Implementation Phases

### Phase 1: Foundation — Extraction & Module Setup

#### 1.1 Foundation Work

- Create a new file `sdk/campaign_builder/src/campaign_editor.rs`.
- Add `CampaignMetadataEditorState` with:
  - `metadata`: the loaded `CampaignMetadata` (cloned or borrowed as appropriate),
  - `buffer`: `CampaignMetadataEditBuffer`, mirroring `CampaignMetadata` fields for editing,
  - `mode`: `List`/`Editing`/`Creating` (an enum),
  - `search_filter` maybe for field lookups,
  - `has_unsaved_changes` flag,
  - `selected_section` or similar to track form panel.
- Use exact domain types from the architecture and `campaign.ron` to avoid drift.
- Add a small `CampaignEditBuffer` `default()` with sensible defaults for `CampaignMetadata` fields similar to other editors’ patterns.

#### 1.2 Add Foundation Functionality

- Implement `impl CampaignMetadataEditorState` skeleton:
  - `fn new() -> Self` – default initialization
  - `fn start_edit()`, `fn save_metadata()`, `fn load_from_file()`, `fn save_to_file()`, `fn cancel_edit()`
  - Follow patterns used by other editors for file operations in `races_editor.rs` / `characters_editor.rs`.

#### 1.3 Integrate Foundation Work

- Add a new module entry in `main.rs` (or the `sdk::campaign_builder` root) `mod campaign_editor;`.
- Add a slot/tab in the app’s `active_tab` (enum addon: `EditorTab::CampaignMetadata`) to show editor.
- Provide `CampaignBuilderApp` code such that when the `Campaign` metadata tab is selected, call `campaign_editor::CampaignMetadataEditorState::show()`.

#### 1.4 Testing Requirements

- Add unit tests for:
  - `CampaignMetadataEditorState::new`
  - `save_to_file`, `load_from_file` read/write semantics
  - `cancel_edit()` behavior
- Validation: `validate_campaign()` tests on configuration checks exist in `main.rs` — extend accordingly.

#### 1.5 Deliverables

- `sdk/campaign_builder/src/campaign_editor.rs` created.
- `CampaignBuilderApp` tab to open the metadata editor.

#### 1.6 Success Criteria

- Code compiles, passes `cargo fmt`, `cargo clippy`, `cargo check`.
- The `CampaignMetadataEditorState::new()` exists and initial tests pass.

---

### Phase 2: UI Implementation — TwoColumn Form & Edits

#### 2.1 Feature Work

- In `campaign_editor.rs`, implement `fn show(&mut self, ui: &mut egui::Ui)` to render:
  - `EditorToolbar::new("Campaign")` with search, import/export, reload, validate actions.
  - Use `TwoColumnLayout::new("campaign_metadata_layout").with_left_width(300.0).show_split(...)`:
    - Left panel: sections (Overview, Gameplay, Files, Advanced), maybe preview metadata summary.
    - Right panel: form to edit corresponding fields for selected section.
  - Use `egui::Grid` for label + input pairs and `ActionButtons` or `Save + Cancel + Back to List` at the bottom (save on success or leave as configured by the user).

#### 2.2 Integrate Feature

- Reuse `ui_helpers.rs` patterns:
  - Use `compute_left_column_width(...)` for left width clamping.
  - Use `ActionButtons` and `EditorToolbar` for consistent control appearance.
  - Keep `Back to List` as a bottom action button consistent with other editors.

#### 2.3 Configuration Updates

- Add form widgets for each `CampaignMetadata` property:
  - `id`, `name`, `version`, `author`, `description`, `engine_version`, `starting_map`, starting coordinates, `starting_direction`, `starting_gold`, `starting_food`, `max_party_size`, `max_roster_size`, `difficulty`, `permadeath`, `allow_multiclassing`, `starting_level`, `max_level`.
  - Also fields for data files: `items_file`, `spells_file`, `monsters_file`, `classes_file`, `races_file`, `characters_file`, `maps_dir`, `quests_file`, `dialogue_file`, `conditions_file`.
- Use `TextEdit::singleline`, `ComboBox` for direction/enums, `Checkbox` for booleans.
- Implement constraints/inline validation (min/max for levels) using client-side validators.

#### 2.4 Testing Requirements

- UI tests for:
  - Editing fields and saving updates `CampaignMetadata`.
  - Import/export using `ron` roundtrip.
  - `starting_map` selection, file path validation (exists/absent test).
- Add coverage for undo/redo if equipped.

#### 2.5 Deliverables

- Form completion for all `CampaignMetadata` fields.
- Toolbar and bottom action row working (Save/Cancel/Back).
- Edit buffer properly applies validation before saving.

#### 2.6 Success Criteria

- Able to edit metadata fields, save to file, and reload.
- Validation messages are shown and “Save” prevents invalid state.

---

### Phase 3: Integration & Validation

#### 3.1 Feature Work

- Call `validate_campaign()` after Save or when pressing `Validate` in toolbar:
  - Use `ValidationCategory::Configuration` (and `Metadata` where appropriate) to classify validation results for the Campaign metadata.
- Update `validate_campaign()` where needed:
  - Add checks for `starting_map` exists (error),
  - `starting_level` in bounds (error),
  - `max_roster_size >= max_party_size` (error),
  - Resource bounds checks for `starting_gold`, etc., (warning).
- Ensure errors are added to `self.validation_errors` with correct category.

#### 3.2 Integrate Feature

- Show configuration validation results under the `Validation` panel that we refactored in Phase 1.
- Integrate Campaign Editor validate button to switch to the Validation tab and display grouped results.

#### 3.3 Configuration Updates

- If check requires file existence (e.g., `starting_map`), use the existing `AssetManager` or path utilities; show clickable error that opens/locates asset.
- `validate_campaign()` should be idempotent and the new checks should follow the same pattern as current `validate_*` functions.

#### 3.4 Testing Requirements

- Unit tests verifying new `Configuration` checks return the expected `ValidationResult` with `Configuration` category.
- Integration test verifying UI shows those results in the Validation panel.

#### 3.5 Deliverables

- `validate_campaign()` extended with `Configuration` checks triggered by the Campaign Editor.
- Tests for the new validators.

#### 3.6 Success Criteria

- Configuration category appears in Validation panel when a check fails.
- Clicking the "Validate" button in the metadata editor brings up validation results.

---

### Phase 4: Main Refactoring (Extracting from `main.rs`)

#### 4.1 Foundation Work

- Identify and extract metadata-specific logic from `main.rs`:
  - UI widgets, handlers, and memory.
  - Any main app top-level logic related to Campaign metadata.

#### 4.2 Integrate Foundation Work

- Replace direct `main.rs` UI markup with a call such as:
  - `campaign_editor_state.show(ui, ...)` in the `CampaignBuilderApp` update loop, under the metadata tab.
- Keep `main.rs::validate_campaign()` as central validation aggregator; call it from `CampaignEditor` or `Edit` toolbar actions.
- Ensure the `CampaignBuilderApp` holds a single `CampaignMetadataEditorState` instance and persistence.

#### 4.3 UI & Data Integration

- Use `ActiveTab` handing; the Campaign Editor should toggle tab at activation and update `active_tab` consistently.
- Ensure `CampaignMetadata` state syncs with the app state and triggers validation when changed.

#### 4.4 Testing Requirements

- Validate there are no regressions across the other editor tabs.
- Run `cargo test` and `cargo check` for the campaign builder module.

#### 4.5 Deliverables

- `main.rs` reduced in size and complexity; metadata logic moved into `campaign_editor.rs`.

#### 4.6 Success Criteria

- `main.rs` UI is simplified (metadata forms moved out), and `campaign_editor.rs` hosts the metadata editor.
- App tests for other editors still pass.

---

### Phase 5: Docs, Cleanup and Handoff — COMPLETED (2025-12-10)

**Status:** ✅ COMPLETED | **Type:** Documentation, Test Coverage & Handoff | **Files Updated:** `sdk/campaign_builder/src/campaign_editor.rs`, `sdk/campaign_builder/src/main.rs`, `docs/explanation/implementations.md`, `docs/explanation/campaign_metadata_editor_implementation_plan.md`, `docs/how-to/edit_campaign_metadata.md`

#### 5.1 Summary of Work Completed

- Finalized the Campaign Metadata Editor module:
  - Created comprehensive developer notes and inline doc comments in `sdk/campaign_builder/src/campaign_editor.rs` describing the edit-buffer pattern, validation flow, and extension checklist.
  - Ensured all public functions and types include `///` doc comments and, where appropriate, runnable examples consistent with repository patterns.
- Documentation updates:
  - Added a how-to guide: `docs/how-to/edit_campaign_metadata.md` with clear end-user steps and a developer-oriented extension guide.
  - Updated `docs/explanation/implementations.md` with a Phase 5 summary and validation results.
  - Updated this implementation plan (`docs/explanation/campaign_metadata_editor_implementation_plan.md`) to mark Phase 5 as complete and recorded deliverables.
- Tests and QA improvements:
  - Added unit tests for `CampaignMetadataEditorState` behavior (e.g., `apply_buffer_to_metadata` and `consume_validate_request`), and preserved all roundtrip and cancel tests.
  - Verified Save/Load roundtrip behavior and editor state transitions under test.
- Integration & Validation:
  - Verified the editor uses `validate_requested` to signal the main app to run `validate_campaign()`. `main.rs` consumes the flag and switches to the `Validation` tab; this remains the canonical validation path.
  - The editor continues to use RON for Save/Load and export/import; `Save` and `Save As` flows update `campaign_path` and copy the authoritative metadata into the app state.

#### 5.2 Detailed Implementation Notes

- Developer Guidance:
  - To add a new metadata field, follow the checklist:
    1. Update `CampaignMetadata` in domain
    2. Add the field to `CampaignMetadataEditBuffer`
    3. Update `from_metadata()` and `apply_to()` mappings
    4. Add UI controls in `show()` and ensure the buffer flags `has_unsaved_changes`
    5. Add validation in `validate_campaign()` and tests
    6. Update docs/how-to and sample RON files if necessary
- UX & Accessibility:
  - TwoColumn layout maintained with left-to-right section navigation and a RON export option in the Advanced section.
  - Browse buttons and file dialog usage follow the project's UI components.
- Code Quality:
  - Public API documented with examples and `///` doc comments.
  - Code style and lints respected; existing unit tests extended and new tests added where necessary.

#### 5.3 Deliverables

- `sdk/campaign_builder/src/campaign_editor.rs`:
  - Developer-facing module-level doc with usage and extension notes
  - Doc comments for public functions and types, with examples
  - Unit tests for new behaviors (`apply_buffer_to_metadata`, `consume_validate_request`)
- `sdk/campaign_builder/src/main.rs`:
  - Updated integration for consuming `validate_requested` and switching to `EditorTab::Validation`
  - Simplified metadata UI (delegates to the `campaign_editor` module)
- Documentation:
  - `docs/how-to/edit_campaign_metadata.md`: How-to guide for both editors and developers
  - `docs/explanation/implementations.md`: Summary entry for this phase and the overall refactor
  - Plan (this file): Marked Phase 5 as Complete with actionable handoff notes

#### 5.4 Validation & Results

- Confirmation Steps (performed during the handoff):
  - Saved a test campaign RON and reloaded it to verify roundtrip integrity
  - Confirmed `validate_requested` toggle triggers `validate_campaign()` in the main app and switches to the Validation tab
  - Verified that editor changes are applied to the app's authoritative `self.campaign` on Save/Save As and before validation, without requiring a final explicit Save
  - Confirmed doc updates and test coverage: `cargo fmt`, `cargo check`, `cargo clippy` (with -D warnings), and `cargo test` were used as validation gates
  - Updated QA checklist and manual validation steps in the how-to guide

#### 5.5 Success Criteria Revisited

- The editor exposes a TwoColumn layout with full editing flows for campaign metadata fields and clear sectioning.
- Validation integration works via the `validate_requested` flag and `main.rs` remains the single validation orchestrator.
- New developer-oriented documentation and how-to guides are in place, reducing onboarding friction.
- Unit and integration tests exercise the primary workflows, and new tests validate editor-specific state transitions.

#### 5.6 Handoff Checklist (For QA / Maintainability)

- [x] All public functions and types documented with examples or developer notes
- [x] Tests added for editor state transitions and save/load roundtrip
- [x] `docs/how-to/edit_campaign_metadata.md` provides an end-user flow and a developer extension guide
- [x] `main.rs` integration for validation measured and confirmed
- [x] RON roundtrips checked for all metadata fields and file references

#### 5.7 Next Steps (Optional Future Work)

- Implement clickable Validation results to focus the relevant field in the editor.
- Add more automated UI-level tests (egui-based) to simulate Save/Load and Validate in the live UI.
- Consider adding undo/redo support for the edit buffer to improve UX.
- Extend integration tests to cover edge cases for complex metadata references (assets, map existence).

---

---

## Risks and Mitigations

- Risk: Extracting logic from `main.rs` causes regression in app state or UI functionality.
  - Mitigation: Add unit tests for data mutations and UI smoke tests.
- Risk: New validation checks may duplicate existing metadata checks or mis-classify errors.
  - Mitigation: Consolidate configuration checks into `validate_campaign()` and follow `ValidationCategory` usage rules (`Configuration` vs `Metadata`).

## Acceptance Criteria

- The new `campaign_editor.rs` implements TwoColumn layout and a complete editing flow for all `CampaignMetadata` fields in `campaign.ron`.
- `validate_campaign()` contains meaningful `Configuration` validations (map existence, party/roster limits, starting level, etc.).
- Validation UI shows `Configuration` results when present and matches Assets Manager layout.
- `main.rs` is simplified and the campaign metadata module is fully factored out.
- Tests added and passing; docs updated to explain the new editor.

---
