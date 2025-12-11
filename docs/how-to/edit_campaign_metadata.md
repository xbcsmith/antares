# How-To: Edit Campaign Metadata

TL;DR
- The Campaign Metadata Editor is a dedicated UI for `CampaignMetadata` fields in the SDK Campaign Builder.
- Open the editor in the Campaign Builder app's `Metadata` tab.
- Edit fields in the right-side form and use Save/Load/Validate actions in the toolbar or action row.
- Developers: When adding fields, update the `CampaignMetadata` domain type, `CampaignMetadataEditBuffer`, `from_metadata`, `apply_to`, `show()` UI, validation logic, and tests.

Overview
- Location (implementation):
  - Editor implementation: `sdk/campaign_builder/src/campaign_editor.rs`
  - Main app usage: `sdk/campaign_builder/src/main.rs`
  - Validation helpers: `sdk/campaign_builder/src/validation.rs`
  - Shared UI components: `sdk/campaign_builder/src/ui_helpers.rs`
- Pattern:
  - The editor uses a TwoColumn layout with a left column for section selection and a right column for editing the fields.
  - It uses an edit-buffer pattern, i.e., `CampaignMetadataEditBuffer`, to keep changes transient until saved.

Open the Editor
1. Run the Campaign Builder app (or use the Campaign Builder SDK).
2. Click the `Metadata` tab (or `EditorTab::Metadata`).
3. The metadata editor opens and displays the TwoColumn layout: left column shows sections and preview; right column shows a form for the selected section (Overview, Files, Gameplay, Advanced).

Editor Layout & Fields
- Left Column:
  - Sections: Overview, Files, Gameplay, Advanced (selectable).
  - Live preview: ID, Name, Version, and a short description preview.

- Right Column (by section)
  - Overview:
    - `Campaign ID` (string)
    - `Name` (string)
    - `Version` (string)
    - `Author` (string)
    - `Engine Version` (string)
    - `Description` (multiline)
  - Files:
    - `Items File`, `Spells File`, `Monsters File`, `Classes File`, `Races File`
    - `Characters File`, `Maps Directory`, `Quests File`, `Dialogue File`, `Conditions File`
    - Each path has a text input + "Browse" button.
  - Gameplay:
    - `Starting Map`, `Starting Position (x,y)`, `Starting Direction` (North/East/South/West)
    - `Starting Gold`, `Starting Food`
    - `Difficulty` (ComboBox)
    - `Permadeath`, `Allow Multiclassing` (Checkboxes)
    - `Starting Level`, `Max Level`, `Max Party Size`, `Max Roster Size`
    - Inline validation hints appear for common checks: roster >= party, starting level ≤ max level, food/ranges.
  - Advanced:
    - Engine metadata preview, paths, and “Export RON” button (opens an import/export dialog with the serialized buffer content).

Toolbar & Actions
- EditorToolbar (top):
  - Search, Save, Load, Import/Export button(s).
  - Save writes the authoritative `metadata` RON to `campaign_path` (Save uses `campaign_path` if set; Save As prompts a file dialog).
- Bottom Action Row:
  - "Back to List" — exit editor mode to list view.
  - "Save Campaign" — save to current `campaign_path`. If none, Save-As prompt.
  - "Validate" — applies buffer to app metadata, sets validate request flag, and the main app runs `validate_campaign()`; the app switches to the Validation tab for results.
  - "Cancel" (when editing) — discards changes in the buffer and leaves `metadata` untouched.

Save / Load / Export Behavior
- Save:
  - If a `campaign_path` exists, Save overwrites the file.
  - If no `campaign_path`, Save prompts for Save As.
  - Save copies the editor's authoritative `metadata` into the app’s `self.campaign` so the rest of the application sees changes.
  - After `Save`, editor sets `validate_requested = true`, prompting the app to run validation and show results in the Validation tab.

- Load:
  - Choose a `.ron` campaign file. `load_from_file(path)` reads the file and replaces the authoritative metadata and buffer.

- Export RON:
  - Serializes the edit buffer as RON (string) and opens the import/export dialog for copy/paste or save in external editors.

Input Controls & UX Patterns
- Text inputs: singleline for strings; multiline for descriptions.
- File paths and folder browsing use the OS file dialogs (`rfd` crate).
- Numeric inputs use `egui::DragValue` with enforced ranges for valid values.
- ComboBoxes and selectable labels for enums (e.g., `Difficulty`, `Starting Direction`).
- Inline validation messages are shown in red in the UI for immediate feedback (e.g., roster/party inconsistency).

Developer Notes — How to Add a New Metadata Field
Follow these steps to extend/add a field to `CampaignMetadata`:
1. Update the Domain Type
   - Add the new field to the `CampaignMetadata` domain definition (aligned with the architecture).
   - Keep naming & types consistent with architecture and use exact type aliases (i.e., use `ItemId`, `MapId`, not raw `u32`).

2. Add Field in the `CampaignMetadataEditBuffer`
   - Add the same field (matching type) to `CampaignMetadataEditBuffer` in `sdk/campaign_builder/src/campaign_editor.rs`.
   - Implement default values in `Default` for the buffer or rely on `CampaignMetadata::default()`.

3. Update `from_metadata()` and `apply_to()`
   - Update `CampaignMetadataEditBuffer::from_metadata(m: &CampaignMetadata)` to copy the field.
   - Update `CampaignMetadataEditBuffer::apply_to(&self, dest: &mut CampaignMetadata)` to copy into the authoritative metadata.

4. Add UI Control in `show()` UI
   - Insert the field into the right section of `show()` (Overview/Files/Gameplay/Advanced).
   - Use an appropriate input widget (TextEdit, DragValue, ComboBox, Checkbox, etc.).
   - Ensure `self.has_unsaved_changes` toggles when the input changes and update `*unsaved_changes` parameter passed by the main app.

5. Validation
   - Add validations in `validate_campaign()` or `validation.rs` to enforce bounds or referential integrity for new fields (file existence, valid id references, etc.).
   - Classify the validation results under the right category (Configuration vs Metadata).

6. Tests
   - Provide unit tests for:
     - `from_metadata()` and `apply_to()` roundtrip.
     - Buffer-to-metadata flush (`apply_buffer_to_metadata()`).
     - Save/load roundtrip for the new field.
     - UI integration tests if possible (mock / simulate show behavior).
   - Update integration tests for save/load cycles if the metadata affects data files.

7. Documentation
   - Update `docs/explanation/implementations.md` and `docs/explanation/campaign_metadata_editor_implementation_plan.md` with the new field and rationale.
   - Add a line in `docs/how-to/edit_campaign_metadata.md` describing the field in the appropriate editor section.

Validation & Test Guidance
- Edge Cases to test manually and automatically:
  - Starting level cannot be less than 1 or greater than max level.
  - `max_roster_size >= max_party_size`.
  - `starting_food` within valid range (`FOOD_MIN`..`FOOD_MAX`).
  - File paths exist when required (use `AssetManager` or simple `fs::metadata()` checks).
  - ComboBox values are consistent with allowed enumerations (e.g., `Difficulty::all()`).
  - File save permissions and the Save-As flow properly update `campaign_path`.

Manual QA Checklist
- Setup:
  1. Start the Campaign Builder app.
  2. Open the `Metadata` tab.

- Basic Workflow:
  1. Verify the current campaign ID & Name show in the preview (left panel).
  2. Switch between sections and verify the UI updates accordingly.
  3. Edit fields in multiple sections (Overview, Files, Gameplay).
  4. Use Browse buttons to pick paths and see the fields update.
  5. Click `Validate` — the app should run the validator and switch to the Validation tab with results.
  6. Try `Save` → With `campaign_path` set, file contents should update; with no `campaign_path`, Save should prompt Save As.
  7. Save then `Load` the file: fields should round-trip to the editor.
  8. Change values, then `Cancel`: the buffer should revert to authoritative `metadata` values.
  9. Try invalid values and confirm UI inline errors and Validation errors where appropriate.

- File-level checks:
  1. Confirm Save/Save-As create proper RON files.
  2. Edit the saved RON manually (optional) and re-load to verify `load_from_file()` success or correct error behavior.

Automated Test Checklist (Add or run these tests)
- Unit:
  - `CampaignMetadataEditorState::new()`
  - `CampaignMetadataEditorState::start_edit` & `cancel_edit`
  - `apply_buffer_to_metadata` behavior
  - `save_to_file` & `load_from_file` roundtrip (use `tempfile` or `tempdir` in tests)
  - `consume_validate_request` toggles correctly
- Integration:
  - Editor validation integration: when `validate_requested` is set by the editor, the app runs `validate_campaign()` and shows the `Validation` tab.
  - Save/Load cycles preserve all metadata fields and file references.
- Run the standard quality commands:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features` (ensure good coverage and passing tests)

Handoff & Maintenance Checklist
- Confirm documentation is updated:
  - Add the field to `docs/explanation/campaign_metadata_editor_implementation_plan.md`.
  - Add or update `docs/explanation/implementations.md` summarizing the change.
  - Add a `docs/how-to/edit_campaign_metadata.md` step for the new field if required.

- Code Review Checklist:
  - No changes to core domain structures (unless approved).
  - Use type aliases defined in architecture (`ItemId`, `SpellId`, `MapId`, etc.).
  - Inline tests for new field(s) should be added in the editor tests.
  - Update RON sample campaign data files if a new required field is added.
  - Avoid breaking changes that impact existing fields or saved campaigns.

- Final Validation:
  - Run Build and Test steps above.
  - Validate that sample campaign files (e.g., `campaigns/tutorial/campaign.ron`) still load correctly and that the editor can Save & Load them without data loss.

Troubleshooting
- Save fails with permission errors:
  - Verify the directory exists and has write access.
  - Use Save As and select a writable directory.

- `Validate` does not show results:
  - The editor sets `validate_requested` which must be consumed by the app:
    - In `sdk/campaign_builder/src/main.rs` the `show_metadata_editor()` function consumes the validate request: if it’s missed, ensure the main app still checks and runs `validate_campaign()`.
    - Verify that `active_tab` is switched to the validation tab for visible feedback.

- UI input changes not persisted:
  - Check that `apply_buffer_to_metadata()` is invoked before Save or Validate.
  - Confirm `apply_to()` includes the field and is mirrored in `CampaignMetadata`.

References
- Editor implementation: `sdk/campaign_builder/src/campaign_editor.rs`
- App entrypoint and integration: `sdk/campaign_builder/src/main.rs`
- Validation helpers: `sdk/campaign_builder/src/validation.rs`
- Shared UI helpers: `sdk/campaign_builder/src/ui_helpers.rs`

Appendix — Quick Example Developer Flow
- Add field `foo: String` to metadata:
  1. Update `CampaignMetadata` domain struct to `foo: String`.
  2. Update `CampaignMetadataEditBuffer` and `default()`.
  3. Update `from_metadata()` and `apply_to()` to include `foo`.
  4. Add UI control in `show()`:
     - In the right section:
       ```
       ui.label("Foo:");
       if ui.text_edit_singleline(&mut self.buffer.foo).changed() {
           self.has_unsaved_changes = true;
           *unsaved_changes = true;
       }
       ui.end_row();
       ```
  5. Add validation in `validate_campaign()` under Configuration (if needed).
  6. Add tests to assert roundtrip and UI behavior.
  7. Run the quality checks (format, check, clippy, test).

If you want—after you’ve made a domain change—I can:
- Create the unit/integration tests for the new field,
- Update example campaign RON files to include the field,
- Add doc updates about the new field and any validation rules.

Thanks — feel free to ask me to:
- Expand the QA automation coverage,
- Create clickable validation results that jump to the relevant editor field,
- Add undo/redo support for the edit buffer.
