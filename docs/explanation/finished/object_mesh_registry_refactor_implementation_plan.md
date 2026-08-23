# Object Mesh Registry Refactor Implementation Plan

## Overview

Replace the bespoke string-keyed `object_mesh_registry.ron` format with the
same numeric-ID array format used by every other mesh registry (landscape,
furniture, item).  This removes a one-off serialisation path, enables multiple
variants of the same mesh concept, and aligns `ObjectMeshDatabase` lookup keys
with the `"12001"` string-of-id convention already used by the landscape and
furniture merge paths.

The refactor touches four layers: the domain type in `object_mesh.rs`, two
campaign data files (tutorial + test fixture), the SDK data model, and the SDK
map-editor mesh picker UI.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Notes |
|--------|----------|-------|
| `struct ObjectMeshRegistry` | `src/domain/world/object_mesh.rs` L104 | **Private** read-only RON deserializer for the old `HashMap<String,String>` format |
| `struct ObjectMeshRegistryFile` | `src/domain/world/object_mesh.rs` L138 | **Public** editable file: `BTreeMap<String, String>`; `upsert(key,path)`, `rename(old,new)`, `remove(key)` |
| `ObjectMeshRegistryFile::load` | `…/object_mesh.rs` L182 | Uses `ron::Value` workaround to accept `ObjectMeshRegistry(meshes: {...})` on disk |
| `struct ObjectMeshDatabase` | `…/object_mesh.rs` L321 | Runtime `HashMap<String, CreatureDefinition>`; `load_from_registry` uses private `ObjectMeshRegistry` |
| `browse_event_mesh_ids` | `src/sdk/map_editor.rs` L599 | Returns `Vec<String>` (sorted); current strings are name-based keys like `"barred_passage"` |
| `available_mesh_ids: Vec<String>` | `sdk/…/map_editor.rs` L4349 | Populated from `browse_event_mesh_ids`; passed to autocomplete event-mesh picker |
| `EventEditorState::treasure_mesh_id` | `sdk/…/map_editor.rs` L2448 | `String`; free-form key today; will become numeric-id-as-string after migration |
| `struct ObjectEntry` | `sdk/…/objects_editor.rs` L99 | Has `key: String`, `file_path`, `definition`; no `id` or `name` fields |

### Current `object_mesh_registry.ron` Files

**`campaigns/tutorial/data/object_mesh_registry.ron`** (4 entries, all string-keyed):

| String key | File path |
|------------|-----------|
| `ancient_treasure_chest` | `assets/furniture/seating/ancient_treasure_chest.ron` |
| `barred_passage` | `assets/meshes/objects/barred_door.ron` |
| `ironbound_treasure_chest` | `assets/furniture/storage/ironbound_treasure_chest.ron` |
| `weathered_wooden_hutch` | `assets/furniture/storage/weathered_wooden_hutch.ron` |

**`data/test_campaign/data/object_mesh_registry.ron`** (6 entries, all string-keyed):

| String key | File path |
|------------|-----------|
| `oak_tree` | `assets/meshes/landscape/oak_tree_short.ron` |
| `pine_tree` | `assets/meshes/landscape/pine_tree.ron` |
| `dead_tree` | `assets/meshes/landscape/dead_tree.ron` |
| `palm_tree` | `assets/meshes/landscape/palm_tree.ron` |
| `brush` | `assets/meshes/landscape/brush.ron` |
| `barred_passage` | `assets/meshes/objects/barred_door.ron` |

### Map Event String References (`campaigns/tutorial/data/maps/map_1.ron`)

| Location | Current value | Target value |
|----------|---------------|--------------|
| Treasure event (line ~8263) | `mesh_id: Some("ironbound_treasure_chest")` | `mesh_id: Some("12003")` |
| LockedDoor event (line ~8411) | `mesh_id: Some("barred_passage")` | `mesh_id: Some("12002")` |

### Identified Issues

1. **One-off format**: Every other registry is a `Vec<Entry { id, name, filepath }>` array; object
   mesh is the only named-struct `HashMap<String,String>`.
2. **`ron::Value` workaround**: `ObjectMeshRegistryFile::load` goes through untyped
   `ron::Value` to ignore the `ObjectMeshRegistry(...)` struct name; this adds
   complexity and is unnecessary once the format becomes a plain array.
3. **No unique ID**: String key collisions prevent two variants of the same mesh (e.g.
   `ancient_treasure_chest_open` / `ancient_treasure_chest_closed`).
4. **Opaque lookup keys**: `browse_event_mesh_ids` returns name strings; after the
   refactor the lookup key becomes an opaque numeric string — the map picker must
   display a human-readable name alongside the stored ID.
5. **Rule 5 violation**: `test_object_mesh_registry_file_load_real_tutorial_campaign_file`
   (L820–827) directly loads `campaigns/tutorial/data/object_mesh_registry.ron`
   instead of using `data/test_campaign`.

---

## Target ID Assignments

### Tutorial Campaign (`OBJECT_MESH_ID_MIN = 12000`, IDs start at 12001)

| ID | Name | Filepath |
|----|------|---------|
| 12001 | Ancient Treasure Chest | `assets/furniture/seating/ancient_treasure_chest.ron` |
| 12002 | Barred Passage | `assets/meshes/objects/barred_door.ron` |
| 12003 | Ironbound Treasure Chest | `assets/furniture/storage/ironbound_treasure_chest.ron` |
| 12004 | Weathered Wooden Hutch | `assets/furniture/storage/weathered_wooden_hutch.ron` |

### Test Campaign Fixture

| ID | Name | Filepath |
|----|------|---------|
| 12001 | Oak Tree | `assets/meshes/landscape/oak_tree_short.ron` |
| 12002 | Pine Tree | `assets/meshes/landscape/pine_tree.ron` |
| 12003 | Dead Tree | `assets/meshes/landscape/dead_tree.ron` |
| 12004 | Palm Tree | `assets/meshes/landscape/palm_tree.ron` |
| 12005 | Brush | `assets/meshes/landscape/brush.ron` |
| 12006 | Barred Passage | `assets/meshes/objects/barred_door.ron` |

---

## Implementation Phases

### Phase 1: Domain Types — `ObjectMeshEntry`, Updated Registry and Database

All changes in `src/domain/world/object_mesh.rs`.  No data files change yet.

#### 1.1 Add `ObjectMeshEntry`

Add `pub struct ObjectMeshEntry` following the pattern of `LandscapeMeshEntry` and
`FurnitureMeshEntry`:

- `pub id: u32`
- `pub name: String`
- `pub filepath: String`
- Derive `Debug, Clone, Serialize, Deserialize, PartialEq`
- Full `///` doc comments with an `# Examples` doctest

#### 1.2 Replace `ObjectMeshRegistryFile` Internal Storage

Change `pub meshes: BTreeMap<String, String>` to `pub entries: Vec<ObjectMeshEntry>`.

Replace the existing `upsert`, `rename`, `remove` methods:

| New method | Signature | Behaviour |
|------------|-----------|-----------|
| `upsert` | `(id: u32, name: &str, filepath: &str)` | Insert new or replace existing entry matching `id`; keep entries sorted by id |
| `rename` | `(id: u32, new_name: &str) -> bool` | Update `name` field for entry with matching `id`; return `false` if not found |
| `remove` | `(id: u32) -> Option<ObjectMeshEntry>` | Remove and return entry with matching `id`; `None` if absent |

Update `save()` to serialize `entries` as a RON array (no named-struct wrapper),
matching the landscape/furniture format.

#### 1.3 Update `ObjectMeshRegistryFile::load()` — Dual-Format Support

The `ron::Value` workaround currently handles the old `ObjectMeshRegistry(meshes: {...})`
name mismatch.  Extend it to support both formats during the data-file migration
window:

- **Array format** (target): try `ron::from_str::<Vec<ObjectMeshEntry>>` first.
- **Old named-struct format** (fallback): on failure, fall through to the existing
  `ron::Value` path that deserializes `meshes: BTreeMap<String,String>` and
  synthesizes `ObjectMeshEntry` values with `id = 0`, `name = key.clone()`,
  `filepath = path` as a placeholder.

Callers (and tests) that still read the old format on disk will continue to work.
The `ron::Value` path will be removed in Phase 2 once data files are migrated.

#### 1.4 Update `ObjectMeshDatabase::load_from_registry`

Replace the private `ObjectMeshRegistry { meshes: HashMap<String,String> }` struct
(used only in this function) with a call to `ObjectMeshRegistryFile::load()`.
Iterate `registry.entries` instead of `registry.meshes`:

- For each `ObjectMeshEntry`, use `entry.id.to_string()` as the `HashMap` key —
  matching the convention already used by `merge_landscape` (L426) and
  `merge_furniture` (L450).
- Security: the existing `validate_campaign_relative_path` path-traversal check must
  continue to apply to `entry.filepath`.

Delete the private `struct ObjectMeshRegistry` once `load_from_registry` no longer
uses it.

#### 1.5 Testing Requirements

- Update doctest on `ObjectMeshRegistryFile` to call `registry.upsert(12001, "Barred Door", "assets/…/door.ron")`.
- `test_object_mesh_registry_file_round_trip` — updated to use new `upsert(id, …)` API.
- `test_object_mesh_registry_file_rename` — `rename(id, new_name)` form.
- `test_object_mesh_registry_file_remove` — `remove(id)` form.
- `test_object_mesh_registry_file_load_legacy_named_format` — the old
  `ObjectMeshRegistry(meshes: {...})` format must still parse (dual-format support).
- `test_load_from_registry_round_trip` — adapted to numeric-ID keyed lookup.
- `test_primary_entry_not_overwritten_by_merge` — string key `"test_chest"` becomes `"12001"`.
- **Replace** `test_object_mesh_registry_file_load_real_tutorial_campaign_file` (Rule 5
  violation) with `test_object_mesh_registry_file_load_test_campaign_fixture` that
  loads `data/test_campaign/data/object_mesh_registry.ron` — **after Phase 2 migrates
  that file to the new format**.  For Phase 1, update the test to use the test campaign
  fixture path but keep expecting the old string-keyed format until Phase 2 converts
  the file.

#### 1.6 Deliverables

- [ ] `ObjectMeshEntry` struct added and exported from `object_mesh.rs`
- [ ] `ObjectMeshRegistryFile::entries: Vec<ObjectMeshEntry>` replaces `meshes: BTreeMap`
- [ ] New `upsert(id, name, filepath)`, `rename(id, new_name)`, `remove(id)` methods
- [ ] `ObjectMeshRegistryFile::load()` supports old + new formats
- [ ] `ObjectMeshRegistryFile::save()` writes array format
- [ ] Private `ObjectMeshRegistry` struct removed
- [ ] `ObjectMeshDatabase::load_from_registry` iterates `entries` and keys by `id.to_string()`
- [ ] All modified tests pass

#### 1.7 Success Criteria

`cargo check --all-targets --all-features` clean.  A `ObjectMeshRegistryFile`
round-tripped through `save()` + `load()` produces an identical `entries` vec.
`ObjectMeshDatabase::load_from_registry` still resolves `"12001"` as a lookup key.

---

### Phase 2: Data File Migration

Convert both registry RON files to the array format and update map event string
references to numeric IDs.  After this phase, the `ron::Value` fallback path in
`ObjectMeshRegistryFile::load()` can be removed.

#### 2.1 Convert `campaigns/tutorial/data/object_mesh_registry.ron`

Rewrite the file to the target array format using the ID assignments in the table
above (12001–12004).  The names are the human-readable versions of the old string
keys.

#### 2.2 Create / Update `data/test_campaign/data/object_mesh_registry.ron`

Rewrite (or create if absent) using IDs 12001–12006 as assigned in the Target ID
Assignments table above.

#### 2.3 Migrate String-Keyed `mesh_id` References in `campaigns/tutorial/data/maps/`

Audit all `map_*.ron` files for `mesh_id: Some("<string>")` values.  Replace each
with its corresponding numeric-id-as-string:

- `"ironbound_treasure_chest"` → `"12003"` (map_1.ron, line ~8263)
- `"barred_passage"` → `"12002"` (map_1.ron, line ~8411)

#### 2.4 Remove `ron::Value` Fallback from `ObjectMeshRegistryFile::load()`

Now that both data files are in the new format, remove the `ron::Value` path that
handled the legacy `ObjectMeshRegistry(meshes: {...})` format.  Change `load()` to
a direct `ron::from_str::<Vec<ObjectMeshEntry>>`.

Retain a specific unit test for the old format to confirm a clear parse error is
returned (not a silent success) — this protects against accidental re-introduction of
old-format files.

#### 2.5 Testing Requirements

- `test_object_mesh_registry_file_load_test_campaign_fixture` (replaces the Rule 5
  violating test): loads `data/test_campaign/data/object_mesh_registry.ron` and
  asserts `entries.len() == 6` and `entries[0].id == 12001`.
- `test_campaign_loader_object_meshes_keyed_by_numeric_id` — runs
  `ObjectMeshDatabase::load_from_registry` against `data/test_campaign`; asserts
  `db.has_mesh("12001")` and `!db.has_mesh("oak_tree")`.
- `test_load_legacy_format_returns_error_after_removal` — asserts the old named-struct
  format now returns `Err(ParseError)`.

#### 2.6 Deliverables

- [ ] `campaigns/tutorial/data/object_mesh_registry.ron` converted (IDs 12001–12004)
- [ ] `data/test_campaign/data/object_mesh_registry.ron` converted (IDs 12001–12006)
- [ ] `map_1.ron` string keys migrated to numeric-id-as-string values
- [ ] `ron::Value` fallback removed from `ObjectMeshRegistryFile::load()`
- [ ] No `campaigns/tutorial` references in test code (Rule 5)
- [ ] All tests pass

#### 2.7 Success Criteria

`cargo nextest run --all-features` green.  `ObjectMeshDatabase::load_from_registry`
called against `data/test_campaign` returns a database where `db.has_mesh("12001")`
is `true` and `db.has_mesh("oak_tree")` is `false`.

---

### Phase 3: SDK Data Model — `ObjectEntry` and `campaign_io.rs`

Update the SDK's Campaign Builder to use the new entry-based API throughout.

#### 3.1 Update `ObjectEntry` in `objects_editor.rs`

Add `pub id: u32` and `pub name: String` fields to `pub struct ObjectEntry`:

```
pub struct ObjectEntry {
    pub id: u32,        // NEW — numeric registry ID (e.g. 12001)
    pub name: String,   // NEW — human-readable name from the registry entry
    pub file_path: String,
    pub definition: CreatureDefinition,
}
```

Update the doctest to include `id` and `name`.

#### 3.2 Update `load_objects` in `campaign_io.rs`

Replace the iteration over `registry.meshes` (BTreeMap) with iteration over
`registry.entries` (Vec):

```
for entry in registry.entries {
    let asset_path = dir.join(&entry.filepath);
    // … load CreatureDefinition …
    entries.push(objects_editor::ObjectEntry {
        id: entry.id,
        name: entry.name.clone(),
        file_path: entry.filepath.clone(),
        definition,
    });
}
```

#### 3.3 Update `save_objects` in `campaign_io.rs`

Replace `registry.upsert(&entry.key, &entry.file_path)` with
`registry.upsert(entry.id, &entry.name, &entry.file_path)`:

```
for entry in &self.campaign_data.objects {
    // … write CreatureDefinition asset file …
    registry.upsert(entry.id, &entry.name, &entry.file_path);
}
```

#### 3.4 Update All SDK Tests Using the Old `upsert(key, path)` API

`grep -r "registry.upsert\|ObjectMeshRegistryFile::default" sdk/` to find all
sites.  Update each to use `upsert(id, name, filepath)`.  For tests that do not
care about a specific id, use `12001` as a sentinel.

#### 3.5 Testing Requirements

- All existing `campaign_io.rs` tests for `load_objects` / `save_objects` updated to
  use new `ObjectEntry { id, name, … }` construction.
- `test_save_objects_round_trips_through_registry_and_asset_files` — asserts that
  `registry.entries` contains the expected `id` and `name` after round-trip.
- `test_load_objects_skips_entry_with_missing_asset_file_but_keeps_others` — uses
  `upsert(12001, "Good", "…")` + `upsert(12002, "Missing", "…")`.

#### 3.6 Deliverables

- [ ] `ObjectEntry` carries `id: u32` and `name: String`
- [ ] `load_objects` iterates `registry.entries`
- [ ] `save_objects` calls `registry.upsert(id, name, filepath)`
- [ ] All SDK tests using old `upsert(key, path)` updated
- [ ] All tests pass

#### 3.7 Success Criteria

`cargo nextest run --all-features` green.  A round-trip `save_objects` →
`load_objects` cycle preserves `id` and `name` for each entry.

---

### Phase 4: SDK Objects Editor and Map Editor Mesh Picker

Update the Objects editor to surface `id` and `name`, and the map-editor event
forms to display human-readable names while storing numeric-id-as-strings.

#### 4.1 Update `ObjectsEditorState` to Manage by ID

In `objects_editor.rs`:

- Replace `key_buffer: String` (the mutable edit buffer for the old string key) with
  `name_buffer: String` — the author edits the display name, not the ID.
- `id` is immutable in the editor (assigned by the importer; displayed read-only).
- Update `enter_edit` to populate `name_buffer = entry.name.clone()`.
- Update `apply_edit` uniqueness check from `e.key == new_key` to `e.id == target_id`
  (ids are already unique by construction) — duplicate name is a warning, not an error.
- Update `show_list` and `show_edit` to display `entry.id` (read-only label) and
  `entry.name` (editable text input).

#### 4.2 Update `sync_object_mesh_registry_entry` in `objects_editor.rs`

This helper writes a changed `ObjectEntry` back to `ObjectMeshRegistryFile`.
Change the call from `registry.upsert(&entry.key, &entry.file_path)` to
`registry.upsert(entry.id, &entry.name, &entry.file_path)`.

#### 4.3 Update `browse_event_mesh_ids` in `src/sdk/map_editor.rs`

Change the return type from `Vec<String>` to `Vec<(String, String)>` where the
tuple is `(id_as_string, display_name)`:

```rust
pub fn browse_event_mesh_ids(
    db: &ObjectMeshDatabase,
) -> Vec<(String, String)> {
    // ObjectMeshDatabase::all_mesh_ids_with_names() — see §4.4
}
```

Sort by numeric id (parse `id_as_string` to `u32`, fall back to lexicographic for
legacy numeric landscape/furniture entries).

#### 4.4 Add `all_mesh_ids_with_names` to `ObjectMeshDatabase`

To supply names to `browse_event_mesh_ids`, `ObjectMeshDatabase` needs to carry
names as well as `CreatureDefinition` values.  Add a parallel `HashMap<String, String>`
field (`names: HashMap<String, String>`) and populate it alongside `meshes` in
`load_from_registry` and the `merge_*` methods.

Landscape and furniture entries use the `CreatureDefinition::name` field as the
display name (they already have it set when loaded from their respective registries).

The new `pub fn all_mesh_ids_with_names(&self) -> Vec<(String, String)>` returns
the `(id_as_string, name)` pairs.

#### 4.5 Update `available_mesh_ids` in `MapsEditorState`

Change `available_mesh_ids: Vec<String>` to `available_mesh_ids: Vec<(String, String)>`.
Update the population call (L4548) from `browse_event_mesh_ids(&db)` to the new
signature.

#### 4.6 Update the Autocomplete Mesh Picker in Map Event Editor

The `autocomplete_mesh_id_selector` function and all call sites (for
`treasure_mesh_id`, `sign_mesh_id`, `container_mesh_id`, `lock_mesh_id`) receive
`&[(String, String)]` instead of `&[String]`:

- Dropdown options display `name` (e.g. "Ironbound Treasure Chest").
- Filtering (autocomplete) matches against `name` AND `id_as_string`.
- The selected value stored in `EventEditorState::treasure_mesh_id` is the
  `id_as_string` (e.g. `"12003"`).

Existing map event editor tests that assert `treasure_mesh_id == "barred_passage"`
must be updated to `"12002"` (or the test-appropriate id-string).

#### 4.7 Testing Requirements

- `test_browse_event_mesh_ids_returns_id_and_name_pairs` — asserts the first element
  of a non-empty result is a `(String, String)` tuple.
- `test_browse_event_mesh_ids_sorts_numeric_ids_ascending`.
- `test_objects_editor_enter_edit_populates_name_buffer_not_key_buffer`.
- `test_apply_edit_renames_name_and_updates_registry` — key uniqueness replaced by
  name uniqueness check (warning only).
- Update all map-editor tests that construct `available_mesh_ids` with `Vec<String>`
  to use `Vec<(String, String)>`.

#### 4.8 Deliverables

- [ ] `ObjectsEditorState::name_buffer` replaces `key_buffer`; `id` displayed read-only
- [ ] `browse_event_mesh_ids` returns `Vec<(String, String)>`
- [ ] `ObjectMeshDatabase::all_mesh_ids_with_names` added
- [ ] `MapsEditorState::available_mesh_ids: Vec<(String, String)>`
- [ ] `autocomplete_mesh_id_selector` accepts `&[(String, String)]`; displays names
- [ ] `EventEditorState` mesh-id fields store numeric-id-as-string
- [ ] All modified tests pass; `docs/explanation/implementations.md` updated

#### 4.9 Success Criteria

Opening the map event editor shows "Ironbound Treasure Chest" (not `"12003"`) in the
mesh picker dropdown.  Selecting it stores `"12003"` in the event data.  Saving and
reloading a map preserves the numeric id and resolves the correct mesh at runtime.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/world/object_mesh.rs` | 1, 2 | Add `ObjectMeshEntry`; replace `BTreeMap` in `ObjectMeshRegistryFile`; new `upsert/rename/remove` by id; dual-format `load`; update `load_from_registry`; remove private `ObjectMeshRegistry`; remove `ron::Value` fallback |
| `campaigns/tutorial/data/object_mesh_registry.ron` | 2 | Convert to array format (IDs 12001–12004) |
| `data/test_campaign/data/object_mesh_registry.ron` | 2 | Convert to array format (IDs 12001–12006) |
| `campaigns/tutorial/data/maps/map_1.ron` | 2 | Replace `"ironbound_treasure_chest"` → `"12003"`, `"barred_passage"` → `"12002"` |
| `src/domain/campaign_loader.rs` | 2 | Verify `load_object_meshes` — no signature change expected |
| `src/sdk/map_editor.rs` | 4 | `browse_event_mesh_ids` → `Vec<(String, String)>`; add `all_mesh_ids_with_names` |
| `sdk/campaign_builder/src/campaign_io.rs` | 3 | `load_objects`/`save_objects` use `entries` + new `upsert(id, name, filepath)` |
| `sdk/campaign_builder/src/objects_editor.rs` | 3, 4 | `ObjectEntry` adds `id`/`name`; editor shows id (read-only) + editable name |
| `sdk/campaign_builder/src/map_editor.rs` | 4 | `available_mesh_ids: Vec<(String, String)>`; update autocomplete picker |
| `docs/explanation/implementations.md` | 4 | Implementation summary |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| Dual-format load during migration? | Yes — `ron::Value` fallback kept in Phase 1; removed in Phase 2 once both data files are converted |
| `ObjectMeshDatabase` internal key format after migration? | `id.to_string()` (e.g. `"12001"`) — consistent with `merge_landscape` / `merge_furniture` conventions |
| Map-editor picker: display names or IDs? | Display names, store ID-as-string — same pattern as `browse_dialogue_ids` which returns `(id, title)` pairs |
| Object name editing: allow in editor? | Yes — `name_buffer` editable; id is immutable (assigned at import time) |
| Rule 5 violation in existing test? | Fix in Phase 2 — replace `test_object_mesh_registry_file_load_real_tutorial_campaign_file` with test_campaign-based equivalent |
