# CSV-to-Vec Migration — Refactor Checklist (Phase 1: Discovery & Inventory)

Document Summary
----------------
This checklist is the Phase 1 output for the CSV→Vec migration plan. It consolidates the CSV inventory and ComboBox inventory into a prioritized, actionable set of migration tasks and their dependencies. Use this checklist to track progress from UI helper creation through core editor conversions and final validation.

Prerequisites
-------------
- Read: `docs/reference/architecture.md` (Sections on data types and module placement).
- Ensure type aliases `ItemId`, `MonsterId`, `SpellId`, etc. exist and are used.
- Keep domain models intact in Phase 1 (no structural changes in core domain without approval).
- Run discovery commands to validate inventory (examples below).

Key Discovery Commands (for traceability)
-----------------------------------------
Run these from the project root:

```bash
cd /home/bsmith/go/src/github.com/xbcsmith/antares

# CSV split occurrences in SDK editor code
grep -rn "split\s*(\s*'\\s*,|\"\\s*,)" sdk/campaign_builder/src/ || true

# CSV join occurrences
grep -rn "\.join\s*(\s*'\\s*,|\"\\s*,)" sdk/campaign_builder/src/ || true

# ComboBox usage
grep -rn "egui::ComboBox" sdk/campaign_builder/src/ || true
```

Phase 1 Deliverables (Completed)
-------------------------------
- [x] `docs/explanation/csv_migration_inventory.md` — CSV field inventory
- [x] `docs/explanation/combobox_inventory.md` — ComboBox usage inventory
- [x] `docs/explanation/csv_migration_checklist.md` — This checklist

Success Criteria (Phase 1)
--------------------------
- [x] All CSV fields documented with file path, struct, field name, current type, and suggested target type.
- [x] All ComboBox instances cataloged (single vs multi).
- [x] Prioritized refactor checklist created (15–25 tasks recommended).
- [x] A concise testing plan and validation commands are included.
- [x] No grep patterns return new, undocumented CSV/ComboBox results after the inventory check.

Checklist: Prioritized Tasks
----------------------------
Note: "Owner" and "ETA" should be assigned by the migration lead during planning. Use `TBD` for now.

Phase 2: UI Helper Foundation (Pre-requisite for conversions)
- [ ] TASK-02.01 — Implement `searchable_selector_single<T, ID>` in `sdk/campaign_builder/src/ui_helpers.rs`
  - Owner: TBD
  - ETA: 1–2d
  - Notes: Supports single selection with optional nullable selection. Should accept id_salt / label, custom item->label mapping, and support keyboard navigation.

- [ ] TASK-02.02 — Implement `searchable_selector_multi<T, ID>` in `sdk/campaign_builder/src/ui_helpers.rs`
  - Owner: TBD
  - ETA: 1–2d
  - Notes: Multi-select widget supporting chips, search, add/remove actions and reorder if necessary.

- [ ] TASK-02.03 — Implement `parse_id_csv_to_vec` and `format_vec_to_csv` helpers
  - Owner: TBD
  - ETA: 0.5–1d
  - Notes: These are temporary compatibility helpers to ease the transition. Must trim whitespaces, ignore empty entries, and handle numeric ID parsing errors gracefully.

- [ ] TASK-02.04 — Write `ui_helpers` unit tests
  - Owner: TBD
  - ETA: 1–2d
  - Notes: Tests must cover single/multi select behaviors, search filtering, selection changes, and parsing helper edge cases (whitespace, invalid numbers).

Phase 3: Core Editor Conversions (HIGH PRIORITY)
- [ ] TASK-03.01 — Convert Map Editor (EventEditorState)
  - File: `sdk/campaign_builder/src/map_editor.rs`
  - Subtasks:
    - [ ] TASK-03.01.A — Change `EventEditorState.encounter_monsters` (String -> `Vec<MonsterId>`)
    - [ ] TASK-03.01.B — Change `EventEditorState.treasure_items` (String -> `Vec<ItemId>`)
    - [ ] TASK-03.01.C — Replace CSV UI text edit with `searchable_selector_multi` for monsters and items
    - [ ] TASK-03.01.D — Update `EventEditorState::default()` to use empty vectors
    - [ ] TASK-03.01.E — Update `to_map_event` conversion logic to use typed vectors
    - [ ] TASK-03.01.F — Add unit tests verifying `to_map_event` conversions and UI rendering
  - Owner: TBD
  - ETA: 2–4d

- [ ] TASK-03.02 — Convert Characters Editor (CharacterEditBuffer)
  - File: `sdk/campaign_builder/src/characters_editor.rs`
  - Subtasks:
    - [ ] TASK-03.02.A — Change `CharacterEditBuffer.starting_items` (String -> `Vec<ItemId>`)
    - [ ] TASK-03.02.B — Update `start_edit_character` to populate `Vec<ItemId>` directly
    - [ ] TASK-03.02.C — Change `save_character` to read the `Vec<ItemId>` directly and not parse CSV
    - [ ] TASK-03.02.D — Replace UI input with `searchable_selector_multi` (with quick add by ID still allowed if needed)
    - [ ] TASK-03.02.E — Add tests for round-trip behavior
  - Owner: TBD
  - ETA: 1–3d

- [ ] TASK-03.03 — Add test coverage for the map/characters core conversions
  - Owner: TBD
  - ETA: 1d
  - Notes: Tests must exercise both save/load JSON/RON conversions and UI-based selections.

Phase 4: Complete Sweep & Unification (MEDIUM PRIORITY)
- [ ] TASK-04.01 — Convert Class Editor buffers
  - File: `sdk/campaign_builder/src/classes_editor.rs`
  - Subtasks:
    - [ ] TASK-04.01.A — Convert `special_abilities` (String -> `Vec<String>`)
    - [ ] TASK-04.01.B — Convert `proficiencies` (String -> `Vec<String>`)
    - [ ] TASK-04.01.C — Replace text inputs with `searchable_selector_multi` or `selectable_label` chips depending on domain
    - [ ] TASK-04.01.D — Update `start_edit_class` and `save_class`, add tests
  - Owner: TBD
  - ETA: 2–4d

- [ ] TASK-04.02 — Convert Race Editor buffers
  - File: `sdk/campaign_builder/src/races_editor.rs`
  - Subtasks:
    - [ ] TASK-04.02.A — Convert `special_abilities`, `proficiencies`, `incompatible_item_tags` to `Vec<String>`
    - [ ] TASK-04.02.B — Update `start_edit_race` and `save_race` conversions
    - [ ] TASK-04.02.C — Replace text input fields with `searchable_selector_multi` or candidate tag UI
    - [ ] TASK-04.02.D — Add tests
  - Owner: TBD
  - ETA: 2–3d

- [ ] TASK-04.03 — Items Editor UI improvements
  - File: `sdk/campaign_builder/src/items_editor.rs`
  - Subtasks:
    - [ ] TASK-04.03.A — Replace `tags` CSV input UI with `searchable_selector_multi` chip UI (domain already uses `Vec<String>`)
    - [ ] TASK-04.03.B — Add tests and update examples/docs
  - Owner: TBD
  - ETA: 1–2d

- [ ] TASK-04.04 — Conditions & Spells UI adjustments (where CSV was used)
  - Files: conditions/spell editors
  - Subtasks:
    - Replace any CSV text fields with typed vectors and `searchable_selector_*` as appropriate
    - Add tests for boundary conditions
  - Owner: TBD
  - ETA: 1–3d

Phase 5: Engine Integration & Domain Model (LOW/MEDIUM PRIORITY)
- [ ] TASK-05.01 — Verify Domain Model Consistency
  - Files: `antares/src/domain/*`
  - Notes: Ensure all domain consumers expect typed `Vec<T>` for the changed fields. Update any type aliases or domain functions that accepted CSV strings (unlikely but check edge cases).
  - Owner: TBD
  - ETA: 1–2d

- [ ] TASK-05.02 — Update Engine Consumers & Validation
  - Files: `sdk/campaign_builder/src/validation.rs`, runtime consumers
  - Subtasks:
    - Update validation rules to use typed `Vec` fields.
    - Ensure RON save/load round-trip behavior remains correct (or adapt as necessary based on Phase policy).
  - Owner: TBD
  - ETA: 1–2d

Phase 6: Documentation & Final Validation (LOW PRIORITY)
- [ ] TASK-06.01 — Update documentation and how-to guides
  - Files: `docs/explanation/*`, `docs/how-to/*`
  - Subtasks:
    - Document new `searchable_selector_*` usage patterns
    - Update editor-specific documentation about changed fields
  - Owner: TBD
  - ETA: 1–2d

- [ ] TASK-06.02 — Final QA & Performance Testing
  - Subtasks:
    - Run `cargo format/check/clippy/test` on SDK & domain
    - Manual QA pass for each editor
    - Performance profiling of selector widgets if list sizes are large
  - Owner: TBD
  - ETA: 1–2d

- [ ] TASK-06.03 — Finalize migration & release notes
  - Subtasks:
    - Add migration notes to SDK release (breaking changes, persistence changes)
    - Add conversion scripts to tooling if needed
  - Owner: TBD
  - ETA: 1d

Test Cases (Phase 2–6)
----------------------
- For Parse/Format helpers:
  - `test_parse_id_csv_to_vec_simple`: "1,2,3" -> vec[1,2,3]
  - `test_parse_id_csv_to_vec_empty`: "" -> vec[]
  - `test_parse_id_csv_to_vec_whitespace`: " 1, 2, 3 " -> vec[1,2,3]
  - `test_parse_id_csv_to_vec_invalid`: "1, bad, 2" -> produces error or skips `bad` with a parsing warning depending on policy

- For UI helpers:
  - `test_searchable_selector_single_search`: Show filtered list on search term
  - `test_searchable_selector_multi_select_add_remove`: Multi-selection add and remove works and persists
  - `test_searchable_selector_single_keyboard_navigation`: Keyboard controls work (up/down/select)

- For Editor conversions:
  - `test_event_editor_state_to_encounter_roundtrip`
  - `test_character_save_load_starting_items_roundtrip`
  - `test_class_save_load_proficiencies_roundtrip`
  - `test_race_save_load_incompatible_tags_roundtrip`

Validation Commands (reference)
------------------------------
- Check Phase 1 deliverables:
  ```bash
  test -f docs/explanation/csv_migration_inventory.md || echo "FAIL: inventory not found"
  test -f docs/explanation/combobox_inventory.md || echo "FAIL: combo inventory not found"
  test -f docs/explanation/csv_migration_checklist.md || echo "FAIL: checklist not found"
  ```

- Re-run discovery grep (sanity check):
  ```bash
  # No new split/join should remain undocumented (post-Phase 1 update)
  grep -rn "split\s*(\s*'\\s*,|\"\\s*,)" sdk/campaign_builder/src/ | tee /tmp/split-results.txt
  grep -rn "egui::ComboBox" sdk/campaign_builder/src/ | tee /tmp/combobox-results.txt
  # Ensure each result is present in the inventory files by visual/manual check, or script-assisted checks.
  ```

Risk Mitigation (PHASE-01)
--------------------------
- RISK-01: Test Failures During Migration
  - Mitigation: Add unit tests first for helpers; migrate one editor at a time, verify behavior, and keep fallback to old parsing helpers until the round is complete.

- RISK-02: Engine Expectation Mismatch
  - Mitigation: Add tests verifying the engine's `MapEvent`, `CharacterDefinition`, etc., still accept or handle the data. If engine expects strings in RON files, create a migration step or compatibility helper to deserialize from both formats.

- RISK-03: UI Performance Degradation
  - Mitigation: Use a debounced search for large lists and pagination or virtualized lists when the domain data set is large (e.g., thousands of items).

- RISK-04: Breaking Changes in Saved Data
  - Mitigation: Keep RON schema compatibility or provide a conversion tool (optional future work) to transform old files to new format. For now, migration is internal to editor buffers; RON format should remain stable unless Phase 5 requires otherwise.

Rollback Plan (Per Phase)
-------------------------
- PHASE-01 Rollback:
  - No code changes — simply update documentation back to previous revision.
- PHASE-02 Rollback:
  - Keep `parse_id_csv_to_vec` & `format_vec_to_csv` until UI is fully swapped.
  - Avoid deleting old parsing code until tests validate new code.
- PHASE-03+ Rollback:
  - Revert editor buffer back to CSV string type in case of major breakage, run tests and reattempt.

Progress Tracking
-----------------
A minimal progress tracking table should be maintained and updated as tasks progress. This is a template:

| Task ID | Title | Owner | Status | ETA | Notes |
| ------- | ----- | ----- | ------ | --- | ----- |
| TASK-02.01 | `searchable_selector_single` impl | TBD | TODO | 1–2d | Prereq for all UI changes |
| TASK-03.01 | Map Editor conversion | TBD | TODO | 2–4d | High priority, affects EventEditorState |

Guidelines & Code Quality
-------------------------
- Use type aliases (`ItemId`, `MonsterId`) not raw integers.
- Use `AttributePair` pattern for stats; do not modify core domain structs without approval.
- Unit tests and doc comments must be added for public functions.
- Do not use `unwrap()` or `expect()` in new code unless safety is proven and documented.

Maintainers & Contacts
----------------------
- Migration Lead: TBD
- Core SDK Owner: TBD
- UI Helper Owner: TBD
- Testing & QA: TBD

When This Checklist Is Complete
-------------------------------
- All CSV fields previously captured are typed vectors in editor buffers (Phase 3).
- UI uses `searchable_selector_single` / `_multi` consistently.
- Domain model remains stable (or documented changes are accepted).
- All tests and quality gates pass:
  - cargo fmt --all
  - cargo check --all-targets --all-features
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all-features

Notes
-----
- This checklist is iterative. If during Phase 2/3 we discover more CSV/ComboBox occurrences, update `docs/explanation/csv_migration_inventory.md` and `docs/explanation/combobox_inventory.md`, then adjust task priorities.
- Always keep documentation and tests updated for each conversion step.
