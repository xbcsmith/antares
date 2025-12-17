docs/how-to/use_searchable_selectors.md#L1-200
# How to Use Searchable Selectors

This how-to guide covers the `searchable_selector_single` and `searchable_selector_multi` helpers used throughout the Antares SDK Campaign Builder UI. These helpers replace the older `egui::ComboBox` patterns (and CSV entries in text inputs) by allowing robust, typed, and searchable selection of single and multiple values while maintaining type-safety and good UX.

Why use searchable selectors?
- Strong typing: Work with `Vec<T>` or `Option<T>` instead of comma-separated `String`s.
- Unified UX: Searchable selectors have a consistent look & behavior across editors.
- Better validation: Typed vectors let the domain layer and serializations guarantee correct types.
- Search and filtering: Quickly find items in long lists with search queries.

Table of contents
- Signature reference
- Single-selection usage
- Multi-selection usage
- Integrating with domain types and serialization
- Incorrect usage and pitfalls
- Testing & QA
- Performance notes
- FAQ

---

## Signature reference

Below are the public helper functions implementing the selectors. Inspect the actual implementations for more details (they are in `sdk/campaign_builder/src/ui_helpers.rs`).

```sdk/campaign_builder/src/ui_helpers.rs#L299-309
pub fn searchable_selector_single<T, IdToStringFn, IdToNameFn>(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    selected: &mut Option<T>,
    all_values: &[T],
    id_to_string: IdToStringFn,
    id_to_name: IdToNameFn,
    query: &mut String,
) -> bool
where
    T: Clone + Eq + PartialEq + std::fmt::Debug,
    IdToStringFn: Fn(&T) -> String,
    IdToNameFn: Fn(&T) -> Option<String>;
```

```sdk/campaign_builder/src/ui_helpers.rs#L338-348
pub fn searchable_selector_multi<T, IdToStringFn, IdToNameFn>(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    selected: &mut Vec<T>,
    all_values: &[T],
    id_to_string: IdToStringFn,
    id_to_name: IdToNameFn,
    query: &mut String,
) -> bool
where
    T: Clone + Eq + PartialEq + std::fmt::Debug,
    IdToStringFn: Fn(&T) -> String,
    IdToNameFn: Fn(&T) -> Option<String>;
```

> Note: The helper function signatures above are approximate — consult the `sdk/campaign_builder/src/ui_helpers.rs` file for the exact version in your checkout. The usage patterns below are compatible with the common function signatures described here.

---

## Single-selection usage

Use `searchable_selector_single` when you should select a single item (e.g., a starting weapon or armor slot). For example, to select a starting weapon using `ItemId`:

```/dev/null/examples/weapon_selector.rs#L1-24
// Example: single item selector in a `show()` function of an editor.
let mut selected_weapon: Option<ItemId> = None;
let mut weapon_query: String = String::new();

// Build suggestion list from domain `items` (Vec<Item>)
let weapon_suggestions: Vec<ItemId> = items
    .iter()
    .filter(|i| matches!(i.item_type, ItemType::Weapon { .. }))
    .map(|i| i.id)
    .collect();

if searchable_selector_single(
    ui,
    "starting_weapon_selector",
    "Select Starting Weapon",
    &mut selected_weapon,
    &weapon_suggestions,
    |id| id.to_string(),      // ID mapping => text used in search and identification
    |id| get_item_name(id),   // Display name used in UI (fallback to id if None)
    &mut weapon_query,        // Query string stored in editor buffer
) {
    // Selection changed -- mark editor as dirty or update dependent fields
    *unsaved_changes = true;
}
```

Best practices for single-selection:
- Keep `query` in the edit buffer so it persists across UI refreshes (e.g., `buffer.weapon_query`).
- Use `id_to_name` closure to show friendly names (e.g., localized text) instead of raw IDs, if available.
- Use `Option<T>` for the selection so you can express "no selection".
- Always validate the selected `Option<T>` against your domain `items` database before applying to domain models.

---

## Multi-selection usage

Use `searchable_selector_multi` when a field accepts multiple values (e.g., starting items, event monsters, class proficiencies). The `selected` parameter is `&mut Vec<T>`.

```/dev/null/examples/starting_items_selector.rs#L1-32
// Example: multi-select starting items for a character.
let mut starting_items: Vec<ItemId> = Vec::new();
let mut items_query: String = String::new();
let item_suggestions: Vec<ItemId> = items.iter().map(|i| i.id).collect();

if searchable_selector_multi(
    ui,
    "starting_items_selector",
    "Starting Items",
    &mut starting_items,
    &item_suggestions,
    |id| id.to_string(),             // id mapper function
    |id| Some(get_item_name(id)),    // returns human-friendly name of the id
    &mut items_query,                // search query stored in buffer
) {
    // Starting items updated
    *unsaved_changes = true;
}
```

Best practices for multi-selection:
- Use typed vectors (e.g., `Vec<ItemId>`) that map directly to your domain types.
- Deduplicate suggestions where necessary (e.g., if duplicate items are present in the source).
- Use `.retain()`/`.iter().filter()` to validate and clean selection when saving the buffer to domain types.

---

## Integrating with the domain & RON serialization

- The `selected` and `selected` containers (Option<T> / Vec<T>) should map directly to your domain definitions (e.g., `CharacterDefinition::starting_items: Vec<ItemId>`).
- Because selectors use typed IDs, the domain’s RON serialization and engine pass-through is safe (no CSV string parsing step required).
- Example: If your editor `CharacterEditBuffer` maintains `starting_items: Vec<ItemId>`, then `save_character()` should copy that to your final `CharacterDefinition` directly.

Serialization example (test / round-trip):
```/dev/null/examples/test_round_trip.rs#L1-40
// After saving an entity via an editor:
// Serialize to RON and back to ensure typed Vec fields persist.
let ron_str = ron::ser::to_string(&saved).expect("Failed to serialize to RON");
let parsed: antares::domain::character_definition::CharacterDefinition =
    ron::from_str(&ron_str).expect("Failed to deserialize from RON");

assert_eq!(parsed.starting_items, saved.starting_items);
```

Note: the `ron::ser::to_string` variant is convenient for tests; `to_string_pretty` requires a second parameter with `PrettyConfig`.

---

## Validation & error handling

- Double-check user inputs (IDs) before saving:
  - If you accept raw IDs typed by the user (legacy workflows), validate them against the database.
  - Prefer to drive selection from existing suggestion lists (prevents invalid IDs).
- Filtering: `id_to_string` closures are used for normalized matching (e.g., convert `u8` to string or `ItemId::to_string()`).
- For multi-select fields, remove blank entries and duplicates before saving:
```/dev/null/examples/cleanup.rs#L1-18
let cleaned: Vec<ItemId> = buffer
    .starting_items
    .iter()
    .map(|id| *id)
    .filter(|id| item_db.contains(*id))
    .collect();
```

---

## Performance & UI Tips

- For large suggestion sets (100+ items), pre-build `Vec<T>` suggestions once per `show()` call or cache them to avoid repeated allocations.
- De-duplicate suggestions using a `HashSet` prior to building the suggestions vector.
- Keep the `query` value as a string in the edit buffer to preserve the user's partial text across redraws and to avoid accidental clearing.

---

## Testing the selectors

- Unit test the editor round-trips and serialization, e.g.:
  - `test_character_starting_items_roundtrip` — ensure `starting_items: Vec<ItemId>` is persisted and restored via RON.
  - `test_class_special_abilities_and_proficiencies_roundtrip` — ensure vectors of `String`/IDs persist.
- UI testing (manual / automation):
  - Create an editor instance, add items via selectors, save, re-open, and verify values persist.
  - Verify that the selector `query` is preserved after window changes.

Example test (concrete):
```/dev/null/examples/unit_test.rs#L1-48
#[test]
fn test_selector_round_trip() {
    let mut state = CharactersEditorState::new();
    state.start_new_character();
    state.buffer.id = "items_rt".to_string();
    state.buffer.name = "Items RoundTrip".to_string();
    state.buffer.race_id = "human".to_string();
    state.buffer.class_id = "knight".to_string();

    state.buffer.starting_items = vec![50_u8, 60_u8];

    state.save_character().unwrap();
    let saved = state.characters.iter()
        .find(|c| c.id == "items_rt").unwrap().clone();

    let ron_str = ron::ser::to_string(&saved).expect("serialize");
    let parsed: antares::domain::character_definition::CharacterDefinition =
        ron::from_str(&ron_str).expect("deserialize");

    assert_eq!(parsed.starting_items, vec![50_u8, 60_u8]);
}
```

---

## Common pitfalls & FAQs

- Q: Why does my selector return empty suggestions?
  - A: Ensure your suggestion list (e.g., `Vec<ItemId>`) is populated from domain data and that `id_to_name` returns `Some(_)` for items that should be listed.

- Q: My tests fail due to `ron::ser::to_string_pretty` signature mismatch.
  - A: Use `ron::ser::to_string(&value)` for a simple 1-argument serialization, or call `to_string_pretty(value, Default::default())` with a `PrettyConfig`.

- Q: Should I keep legacy CSV parsers in the code?
  - A: No — the migration project goal is to remove CSV parsing from SDK `campaign_builder` code. The legacy helpers can remain temporarily for CLI compatibility, but prefer typed `Vec<T>` in editor buffers.

- Q: Why `id_to_string` vs `id_to_name`?
  - A: `id_to_string` is a stable machine-friendly string used for search & identification. `id_to_name` provides a human-friendly display name and is optional. Prefer to show names in the UI.

---

## Next steps / Maintenance

- Once all editors have migrated to typed vectors, remove CSV helpers from `ui_helpers` or mark them deprecated.
- Consider integrating keyboard shortcuts to make multi-selection manipulation easier.
- Add UI automation tests (if you have an eframe/egui testing harness) to exercise the selectors directly.

---

## References
- Core helpers: `sdk/campaign_builder/src/ui_helpers.rs`
- Map/characters/classes editors: `sdk/campaign_builder/src/{map_editor.rs,characters_editor.rs,classes_editor.rs}`
- Editor tests in `sdk/campaign_builder/src/*_editor.rs` include round-trip checks for typed vectors and serialization.

If you need, I can add small sample code snippets to the SDK editors or create unit tests demonstrating multi- and single-select usage directly in the editor test suites — just tell me which editor to focus on (items, characters, map events, classes).
