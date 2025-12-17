# ComboBox Usage Inventory (Phase 1: Discovery & Inventory)

## Document Summary

This document catalogs `egui::ComboBox` usage across the SDK campaign builder code targeted by the CSV→Vec migration plan (Phase 1). The objective is to identify all ComboBox instances in the editor UI and classify them as replacement candidates for the unified searchable selectors (`searchable_selector_single` / `searchable_selector_multi`). The document contains:

- A concise inventory table of ComboBox occurrences
- Observations & notes about each entry
- Prioritization guidance for refactoring
- Tests and validation steps to confirm discovery completeness

This is a Phase 1 deliverable: the inventory created here is an input to Phase 2 (UI helper creation) and Phase 3 (core editor conversions).

---

## Methodology

Discovery approach (repeatable):

- Search the SDK editor source for ComboBox patterns:
  - `grep -rn "ComboBox::"` and `grep -rn "egui::ComboBox::" sdk/campaign_builder/src/`
- Manually review the context for each match to determine:
  - What the ComboBox is selecting
  - Whether it is single-select or multi-select
  - Which editor(s) it occurs in and its relation to editor buffers/domain fields
- Classify the ComboBox usage as:
  - Single-select (replaces with `searchable_selector_single`)
  - Multi-select (replaces with `searchable_selector_multi`)
- Assign priority based on editor criticality and conversion impact:
  - HIGH: Core editors and fields used in many workflows (Map Editor, Characters Editor)
  - MEDIUM: Important editors (Classes, Races, Conditions)
  - LOW: Filters, UI helper combos, or editors with lesser usage

Note: The objective at this stage is discovery and identification — not to change any code. The referenced lines and contexts reflect the state of the code at the time of discovery.

---

## ComboBox Inventory Table

| File Path | Line(s) | Context / Location | Selection Type | Suggested Replacement | Priority | Notes |
| --------- | ------- | ------------------ | -------------- | --------------------- | -------- | ----- |
| `sdk/campaign_builder/src/campaign_editor.rs` | `pub fn show` L822-826 | `campaign_starting_direction` selector | single-select | `searchable_selector_single<Direction, _>` | MEDIUM | Enum-based drop-down. Replace to unify UX and allow search/predictive selection. |
| `sdk/campaign_builder/src/campaign_editor.rs` | `pub fn show` L869-873 | `campaign_difficulty` selector | single-select | `searchable_selector_single<String, _>` | MEDIUM | Small enum/label set; conversion is low risk and improves consistency. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_filters` L685-695 | `filter_race` dropdown in filters UI | single-select | `searchable_selector_single<Race, RaceId>` | LOW | Filter only; not part of core buffer conversion, but benefits from consistent UX. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_filters` L710-714 | `filter_class` dropdown in filters UI | single-select | `searchable_selector_single<Class, ClassId>` | LOW | Filter only. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_filters` L733-737 | `filter_alignment` dropdown in filters UI | single-select | `searchable_selector_single<Alignment, _>` | LOW | Filter only. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_character_form` L1105-1109 | `race_select` selection for editing character | single-select | `searchable_selector_single<Race, RaceId>` | HIGH | Core character editor. Important to unify & inline validation. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_character_form` L1127-1131 | `class_select` selection for editing character | single-select | `searchable_selector_single<Class, ClassId>` | HIGH | Core character editor. Candidate for immediate conversion after UI helper creation. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_character_form` L1149-1153 | `sex_select` selection | single-select | `searchable_selector_single<Sex, _>` | MEDIUM | Simple enum; unify control usage. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_character_form` L1164-1168 | `alignment_select` selection | single-select | `searchable_selector_single<Alignment, _>` | MEDIUM | Simple enum. |
| `sdk/campaign_builder/src/characters_editor.rs` | `fn show_item_selector` L1380-1390 | Generic item selector widget using `ComboBox` | single-select | `searchable_selector_single<Item, ItemId>` | MEDIUM | This helper is used across many editors; modernizing it yields broad benefits. |
| `sdk/campaign_builder/src/classes_editor.rs` | `fn show_class_form` L658-668 | `spell_school` selection | single-select | `searchable_selector_single<SpellSchool, _>` | MEDIUM | Enum selection. |
| `sdk/campaign_builder/src/classes_editor.rs` | `fn show_class_form` L677-687 | `spell_stat` selection | single-select | `searchable_selector_single<SpellStat, _>` | MEDIUM | Enum selection. |
| `sdk/campaign_builder/src/classes_editor.rs` | `fn show_class_form` L719-723 | `starting_weapon` selector | single-select | `searchable_selector_single<Item, ItemId>` | MEDIUM | Replace the ComboBox with a searchable selector to find items quickly. |
| `sdk/campaign_builder/src/classes_editor.rs` | `fn show_class_form` L759-763 | `starting_armor` selector | single-select | `searchable_selector_single<Item, ItemId>` | MEDIUM | Replace similarly to starting weapon. |
| `sdk/campaign_builder/src/conditions_editor.rs` | `pub fn show` L394-404 | `condition_effect_filter` dropdown | single-select | `searchable_selector_single<EffectTypeFilter, _>` | LOW | Filter only; multi-field filter combos are likely less critical. |
| `sdk/campaign_builder/src/conditions_editor.rs` | `pub fn show` L414-418 | `condition_sort_order` dropdown | single-select | `searchable_selector_single<SortOrder, _>` | LOW | Filter/sort UI; convert for unification. |
| `sdk/campaign_builder/src/conditions_editor.rs` | `fn show_form` L883-893 | `condition_default_duration` dropdown | single-select | `searchable_selector_single<ConditionDuration, _>` | MEDIUM | Editing a property of conditions; conversion aids consistency. |
| `sdk/campaign_builder/src/conditions_editor.rs` | `fn show_form` L1141-1145 | `effect_type_select` dropdown | single-select | `searchable_selector_single<EffectType, _>` | MEDIUM | Editor form property. |
| `sdk/campaign_builder/src/conditions_editor.rs` | `fn show_form` L1187-1191 | `attribute_select` dropdown | single-select | `searchable_selector_single<Attribute, _>` | MEDIUM | Controls a stat attribute selection - Quality-of-life improvement to convert. |

> Observed count: ~18 ComboBox occurrences scanned across primary SDK editors (`campaign_editor`, `characters_editor`, `classes_editor`, `conditions_editor`) during this initial discovery pass.

---

## Observations & Notes

- Most `ComboBox` instances are single-select controls (e.g., enum selection, resource lookup).
- The `show_item_selector` (Characters editor) is a shared item selector helper used widely; converting it to `searchable_selector_single` yields massive UX benefits and reduces repetition.
- Filter ComboBoxes (`filter_race`, `filter_class`, `filter_alignment`) are low-risk conversions since they are purely UI filter controls — low priority.
- Enum-based ComboBoxes (Spell School, Spell Stat, Sex, Alignment, etc.) are straightforward to convert to `searchable_selector_single` with minimal domain changes — medium priority.
- Multi-selection is not typically implemented with `ComboBox` in the SDK codebase; checkboxes and `selectable_label` (button lists) are used for multi-select semantics (proficiencies/tags). `searchable_selector_multi` will be the replacement for multi-select lists (e.g., tags, starting items, monsters), but these were not ComboBox-based in the code; they are often CSV strings or checkbox lists.

---

## Prioritization Guidance (Shortlist)

High priority conversions (Phase 3):

- `race_select` and `class_select` in `Characters Editor` → critical area in many workflows (core buffer interaction).
- `show_item_selector` helper → used across editors; unified search enhances UX drastically.

Medium priority conversions (Phase 3/4):

- `starting_weapon` / `starting_armor`, `spell_school`, `spell_stat` in `Classes Editor` → important but limited to single-form conversions.
- `condition_default_duration` / `effect_type_select` / `attribute_select` in `Conditions Editor` → medium priority for domain clarity.

Low priority conversions (Phase 4+):

- Filter combos in `CharactersEditor` & other small filter combos used for list filtering.
- Campaign-level combos (`campaign_difficulty`, `starting_direction`) are simple enumerations and can be chosen for later cleanup.

---

## Testing & Validation

Phase 1 tests (inventory validation):

- Validate that all ComboBox occurrences were recorded:
  - Check `grep -rn "egui::ComboBox" sdk/campaign_builder/src/` yields no new unexpected occurrences beyond the inventory.
- CLI or unit tests (Phase 2/Phase 3):
  - `searchable_selector_single` and `searchable_selector_multi` must have unit tests that:
    - Render with a list of values
    - Respect `selected` and `selected_text`
    - Support search input and filter behavior
    - Correctly handle `None` (empty selection) and `Change` events
  - Replace a few ComboBoxes in a test-only branch and assert:
    - The UI behavior (visually) is the same for non-search scenarios
    - Tests for persistence (start_edit -> show -> save -> to domain model) preserve intended values

Suggested validation commands for inventory completeness:

```bash
# Confirm inventory file includes ComboBox references
test -f docs/explanation/combobox_inventory.md || echo "FAIL: ComboBox inventory missing"
# Confirm discovered occurrences subset size
grep -c "ComboBox::from" docs/explanation/combobox_inventory.md
```

---

## Deliverables & Next Steps

Deliverables created in Phase 1:

- `docs/explanation/combobox_inventory.md` (this document)
- Completed `docs/explanation/csv_migration_inventory.md` (Phase 1 CSV inventory)
- `docs/explanation/csv_migration_checklist.md` (prioritized checklist for migration - Phase 1)

Actionable next steps:

1. Phase 2: Implement `ui_helpers` `searchable_selector_single` and `searchable_selector_multi`.
   - Create the API and add tests to demonstrate how they replace `ComboBox` usage in a drop-in fashion.
2. Phase 3: Start conversions on high priority entries:
   - `Characters Editor` `race_select` and `class_select` to `searchable_selector_single`.
   - `show_item_selector` to use `searchable_selector_single` (refactor helper).
3. Phase 4: Migrate medium & low priority occurrences.
4. Add UI tests and adjust the editor's `start_edit` and `save` flows to use typed vectors as necessary.

---

## Appendix — Example grep commands used (repeatable)

```bash
cd /home/bsmith/go/src/github.com/xbcsmith/antares

# Find all ComboBox usages:
grep -rn "egui::ComboBox" sdk/campaign_builder/src/ || true
grep -rn "ComboBox::from" sdk/campaign_builder/src/ || true

# Narrow to a specific file (example Characters Editor):
grep -n "ComboBox::" sdk/campaign_builder/src/characters_editor.rs || true
```

---

## FAQs (Phase 1)

Q: Should `ComboBox` be removed entirely?
A: No. `ComboBox` is a valid widget. The plan is not to remove `ComboBox` but to unify UX around the `searchable_selector_*` API that wraps or replaces `ComboBox` where appropriate, and to provide a consistent powered search & multi-select experience across editors.

Q: Will this change affect domain models?
A: Ideally no. The `ComboBox` → `searchable_selector_*` refactor is a UI change. When we also move from CSV strings to typed vectors in edit buffers, the domain will follow architecture guidelines (typed vectors) and the editor will mirror them.

Q: Are all `ComboBox` instances safe to replace with a `searchable_selector`?
A: Most single-selection `ComboBox` replacements are safe. Multi-select behavior needs special attention; `ComboBox` is not normally used for multi-select in current code, but some current multi-value fields are implemented as CSV strings or `selectable_label` arrays — these will be migrated to `searchable_selector_multi`.

---

Document prepared by: Antares Engineering Team (Phase 1 — Discovery & Inventory)
Date: 2025-01-13
