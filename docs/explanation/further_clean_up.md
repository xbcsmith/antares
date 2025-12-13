# Further Clean Up

Follow-ups / Recommended next steps

- CLI Editors: I recommend migrating the CLI editors (under `src/bin/`) next so they use typed typed input instead of CSV parsing, aligning them with the SDK UI.
- Remove Legacy Helpers: Once you confirm all migration tasks are complete and no CLI code relies on helper CSV parsing, remove `ui_helpers::parse_id_csv_to_vec` / `format_vec_to_csv`.
- Add UI automation tests (optional): If an eframe/egui harness is available, add integration tests for `searchable_selector_*` for better UI assurance (search filtering in large lists, chip handling, keyboard interactions).
- Extend validation script: Consider updating the validation script to do a more robust check for inevitable edge-cases (e.g., multi-line CSV splitting usage or `split` followed by `trim()` on another line), but the current approach is adequate with the `Legitimate` marker for temporary helpers.

If you'd like, I can:

- Run a final repo-wide sweep to find any remaining `split(',') | join(',')` patterns and add justify markers or migrate them
- Migrate the CLI editors (I can follow the same pattern and add tests)
- Remove the CSV helper when everything else is converted, and update docs to mark it as deprecated/removed
