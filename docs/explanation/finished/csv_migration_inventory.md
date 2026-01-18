# CSV Migration Inventory (Phase 1: Discovery & Inventory)

**Document**: CSV Migration Inventory — Phase 1
**Location**: `docs/explanation/csv_migration_inventory.md`
**Author**: Antares Engineering Team (AI-assisted)
**Date**: 2025-01-13
**Purpose**: Catalog all comma-separated (CSV) fields used in the SDK `campaign_builder` and related tooling that require conversion from `String` to strongly-typed `Vec<T>` fields as part of the CSV→Vec migration.

---

## Summary

This inventory lists CSV-encoded editor fields (fields in edit buffers or UI inputs that are currently string-encoded comma-separated lists) that will be converted to typed vectors (`Vec<ItemId>`, `Vec<MonsterId>`, `Vec<String>`, etc.). For each entry we capture:

- File path
- Relevant line(s) showing either the structure definition or where parsing/formatting occurs (`split`, `join`)
- Struct containing the CSV field
- Field name
- Current type (typically `String` in editor buffers)
- Proposed target type
- Priority for migration (HIGH / MEDIUM / LOW)
- Short notes (e.g. UI implications, tests to update)

Phase 1 (Discovery) expects all CSV occurrences documented in this file. This inventory will be used to generate the prioritized refactor checklist in `docs/explanation/csv_migration_checklist.md`.

---

## Methodology

The following commands were used to discover CSV usages (run from project root):

```bash
cd /home/bsmith/go/src/github.com/xbcsmith/antares

# Find split/join patterns (CSV parsing & formatting)
grep -rn "split\s*(\s*\",\"" sdk/campaign_builder/src/ || true
grep -rn "split\s*(\s*',')" sdk/campaign_builder/src/ || true
grep -rn "\.join\s*(\s*\",\"" sdk/campaign_builder/src/ || true

# Find ComboBox (UI) usage for replacement candidates
grep -rn "egui::ComboBox" sdk/campaign_builder/src/ || true
grep -rn "ComboBox::from" sdk/campaign_builder/src/ || true
```

Comments:
- We focused primarily on `sdk/campaign_builder/src/` since that houses the editor UI and buffers.
- We also reviewed CLI editor tooling under `antares/src/bin/*` for legacy CLI CSV input.
- We intentionally excluded `split('.')` and other non-comma usages (version parsing, numeric dot-splitting).

---

## CSV Fields Inventory

| File Path | Line(s) | Struct | Field | Current Type | Target Type | Priority | Notes |
| --------- | ------- | ------ | ----- | ------------ | ----------- | -------- | ----- |
| `sdk/campaign_builder/src/map_editor.rs` | `EventEditorState: L685-695` — parse: L789-791 | `EventEditorState` | `encounter_monsters` | `String` (CSV) | `Vec<MonsterId>` | HIGH | UI currently shows numeric IDs as CSV. Convert to multi selector UI. Current parse: `self.encounter_monsters.split(',')` |
| `sdk/campaign_builder/src/map_editor.rs` | `EventEditorState: L685-695` — parse: L804-806 | `EventEditorState` | `treasure_items` | `String` (CSV) | `Vec<ItemId>` | HIGH | Convert to `Vec<ItemId>`, use searchable multi-selector for items. Current parse: `self.treasure_items.split(',')` |
| `sdk/campaign_builder/src/characters_editor.rs` | Buffer def: ~L84-94; parse/save: L321-324; UI join: L197-201 | `CharacterEditBuffer` | `starting_items` | `String` (CSV) | `Vec<ItemId>` | HIGH | Convert and replace CSV text input with `searchable_selector_multi<Item, ItemId>` for better UX. `split(',')` used in `save_character()`; `join(", ")` used in `start_edit_character()` |
| `sdk/campaign_builder/src/classes_editor.rs` | Buffer def: ~L50-60; parse: L179-190; formatting: L120-130 | `ClassEditBuffer` | `special_abilities` | `String` (CSV multiline) | `Vec<String>` | MEDIUM | `special_abilities.join(", ")` in `start_edit_class`, `special_abilities.split(',')` in `save_class`. Should become `Vec<String>` and UI should be multi-select / 'add' style input. |
| `sdk/campaign_builder/src/classes_editor.rs` | Buffer def: ~L50-60; parse: L179-190; formatting: L142-146 | `ClassEditBuffer` | `proficiencies` | `String` (CSV) | `Vec<String>` | MEDIUM | Class proficiencies are currently CSV strings. The UI already uses `selectable_label`s (buttons) and can be moved to typed `Vec<String>` with UI mapping. `split(',')` found in `save_class()` |
| `sdk/campaign_builder/src/races_editor.rs` | Buffer def: ~L55-65; parse: L264-283; formatting: L142-152 | `RaceEditBuffer` | `special_abilities` | `String` (CSV) | `Vec<String>` | MEDIUM | Convert `special_abilities` to typed vector; UI can remain a multi-entry text area or a selector. `split(',')` used in `save_race()` |
| `sdk/campaign_builder/src/races_editor.rs` | parse: L272-275; formatting: L142-152 | `RaceEditBuffer` | `proficiencies` | `String` (CSV) | `Vec<String>` | MEDIUM | Convert `proficiencies` to `Vec<String>`; UI uses `selectable_label` pattern and needs update. `split(',')` used in `save_race()` |
| `sdk/campaign_builder/src/races_editor.rs` | parse: L280-283; formatting: L142-152 | `RaceEditBuffer` | `incompatible_item_tags` | `String` (CSV) | `Vec<String>` | MEDIUM | Tags used to restrict items -> should be `Vec<String>` typed. UI currently converts using join/split for editing. |
| `sdk/campaign_builder/src/items_editor.rs` | UI edit: L930-940 | (domain `Item`) / UI `edit_buffer` | `tags` | `Vec<String>` (already typed) / UI is CSV on edit | `Vec<String>` | NOTE | `tags` is already `Vec<String>` in the model. The UI uses a temporary `tags_string` and converts via `split(',')`. No data model change required—consider replacing with `searchable_selector_multi` for improved UX. |
| `sdk/campaign_builder/src/…` | Various | Multiple editors (conditions, quests, validators) | Display-only / diagnostic `join` usage | N/A | N/A | LOW | These `join(", ")` occurrences are for display (debug / validation messages) and do not require migration. |
| `antares/src/bin/class_editor.rs` | parse: lines around ~L561-L600 | CLI edit tool: `ClassEditor` | `input_special_abilities`, `input_proficiencies` | `String` (CSV CLI input) | `Vec<String>` | LOW | CLI editors parse CSV inputs (split(',')) — convert CLI to accept repeated flags or typed vector parsing. Not in the SDK UI, but should be updated as follow-up. |
| `antares/src/bin/item_editor.rs` | parse: lines around ~L561-L600 | CLI `ItemEditor` | `input_item_tags` | `String` (CSV CLI input) | `Vec<String>` | LOW | CLI parsing using `split(',')` (CSV) — update CLI interfaces and docs to accept multi flags or maintain CSV parsing with migration notes. |
| `antares/src/bin/race_editor.rs` | parse: lines around ~L661-L700 | CLI `RaceEditor` | `input_special_abilities`, `input_proficiencies` | `String` (CSV CLI input) | `Vec<String>` | LOW | Convert CLI parsing or keep compatibility with new `Vec` types; Update CLI docs. |

Notes:
- In several places `Vec<T>` exists in the domain model (e.g., `CharacterDefinition.starting_items: Vec<ItemId>`, `MapEvent::monster_group: Vec<u8>`). The editor buffers store CSV `String` for ease-of-text-editing and to support copy/paste. The target is to unify editor buffers with domain models (typed vectors).
- Conversion tasks must ensure round-trip serialization for persistence / RON files as part of the migration (RON data files remain unchanged except where we update the editor's save/load).
- When converting a field, update `start_edit_*` code to populate the buffer from domain `Vec` using typed mapping rather than `join(", ")` into a `String`.
- UI changes: Where the field is multi-valued (items, monsters), convert the UI from a free-text CSV input to a `searchable_selector_multi` with chips, filtering, and validation; for single-selection fields that show a `ComboBox`, plan to use `searchable_selector_single`.

---

## Priority Guidance

- HIGH: Editor buffers that are central to campaign editing & frequently used — Map events, Character starting items, etc. These conversions should be implemented in Phase 3 (Core Editor Conversions).
- MEDIUM: Secondary editors for which conversions improve consistency but do not block core workflows — Classes, Races, Items (if necessary).
- LOW: CLI tools, display-only join usage, debug/validation join results, and simple text displays—deal with after core conversions.

---

## Findings / Observations

- Several domain model fields are already typed (`Vec<T>`) but the Editor buffer uses `String` as a CSV for quick text editing and copy/paste. This is consistent across classes, races, characters, and maps.
- Items Editor: `tags` are already `Vec<String>` at the type level; only UI uses CSV for edits — this is a quick win to change UI to typed multiselect while leaving domain model intact.
- `MapEvent` and `Character` types already use `Vec<u8>` or `Vec<ItemId>` in domain models but editors still use `String`.
- Some `join` / `split` occurrences are for display/logging and must not be converted.

---

## Suggested Safety Checks & Migration Caveats

- Maintain backward compatibility for RON data files for now (ADR-003 states "No backward compatibility", but per project rules, the engine may still need to read older RON—discuss in Phase 5).
- Ensure `serde` and `ron` semantics are respected; tests must confirm round-trip (save → load).
- Avoid changing core domain structs in Phase 1; Phase 3/5 will handle conversions with architecture review.
- Where `split(',')` is used, ensure whitespace trimming (`.trim()`) and empty string filtering occur, and handle parsing errors gracefully.
- Add unit tests and integration tests that validate:
  - Editor save/load round-trip behavior with typed vectors.
  - UI behavior for selecting items (search & multi-select).
  - Validation for invalid IDs and graceful error messages.

---

## Phase 1 Deliverables (Checklist)

- [x] CSV usage inventory (this file: `docs/explanation/csv_migration_inventory.md`)
- [x] ComboBox usage inventory (`docs/explanation/combobox_inventory.md`) — created alongside this effort
- [x] Refactor checklist (`docs/explanation/csv_migration_checklist.md`) linking items in this inventory with migration task owners and priorities
- [x] Scripts and `grep` command notes used to find occurrences (see Methodology)
- [x] Basic test plan for Phase 1 validation

---

## Quick Commands for Verifying Inventory (repeatable)

To confirm we captured the primary CSV usage in the SDK:

```bash
cd /home/bsmith/go/src/github.com/xbcsmith/antares

# Count occurrences of split(","), split(',') in the SDK
grep -rn --include="*.rs" -n "\\.split\\s*(\\s*['\\\"]\\s*,\\s*['\\\"]" sdk/campaign_builder/src/ | wc -l

# Inspect lines of interest (example for EventEditorState)
grep -n "encounter_monsters" sdk/campaign_builder/src/map_editor.rs
grep -n "treasure_items" sdk/campaign_builder/src/map_editor.rs
```

---

## Next Steps (Phase 2 Planning Input)

- Implement `ui_helpers::searchable_selector_single` and `ui_helpers::searchable_selector_multi` as the unified replacement for `ComboBox` and item list editing.
- Implement `ui_helpers::parse_id_csv_to_vec` and `ui_helpers::format_vec_to_csv` helpers for initial compatibility while rolling out typed vector fields in editors.
- Create a merged prioritized checklist and assign owners & estimates.
- Implement tests that:
  - Verify CSV parsing methods (`parse_id_csv_to_vec`) behave for `item`, `monster`, `proficiency` and `tag` types (including trimming/empty element ignoring).
  - Ensure round-trip: Editor → domain `Vec<T>` → save → load → editor buffer behaves as expected.

---

## Appendix: Example code snippets found (grep excerpts)

- `split(',')` occurrences (examples captured during discovery):
  - `self.encounter_monsters.split(',')` (`map_editor.rs`, `to_map_event`)
  - `self.treasure_items.split(',')` (`map_editor.rs`, `to_map_event`)
  - `self.buffer.starting_items.split(',')` (`characters_editor.rs`, `save_character`)
  - `self.buffer.proficiencies.split(',')` (`classes_editor.rs`, `save_class`)
  - `self.edit_buffer.tags = tags_string.split(',')...` (`items_editor.rs`, `show_form`)
  - `self.buffer.special_abilities.split(',')` (`classes_editor.rs`, `save_class`)
  - `self.buffer.incompatible_item_tags.split(',')` (`races_editor.rs`, `save_race`)

- `join(", ")` occurrences (examples used to format typed vectors for editor display):
  - `class.proficiencies.join(", ")` (`classes_editor.rs`, `start_edit_class`)
  - `race.proficiencies.join(", ")` (`races_editor.rs`, `start_edit_race`)
  - `character.starting_items.iter().map(...).collect::<Vec<_>>().join(", ")` (`characters_editor.rs`, `start_edit_character`)

These snippets are present to demonstrate how CSV parsing/formatting is used in the live code.

---

## Contact & Ownership

- Migration Plan: `docs/explanation/csv_to_vec_migration_implementation_plan.md`
- Phase 1 Author: Antares Engineering Team
- Owners for conversion tasks will be assigned in `docs/explanation/csv_migration_checklist.md`.

---

## Final Notes

This inventory is intended to be the source-of-truth for Phase 1 migration work. It should be updated frequently (daily progress notes) during the discovery & inventory completion phase. After Phase 1 completion, we will proceed to Phase 2 (UI Helper Foundation) implementing `searchable_selector_*` helpers and the `parse`/`format` helpers before converting editor buffers to typed vectors.
