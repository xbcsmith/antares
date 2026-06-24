# Object Editor SDK Implementation Plan

## Overview

The Campaign Builder has no editor tab for **Objects** — the entries in
`data/object_mesh_registry.ron` that back the unified interactive-object mesh
system (`docs/explanation/unified_objects_and_events.md`, Phase 4). Today the
only way to create an Object is the Importer tab's `ExportType::ObjectMesh`
path, which writes directly to `object_mesh_registry.ron` on disk and never
updates any in-memory list — because no in-memory list exists. Authors cannot
browse, rename, re-path, or delete an Object, and cannot edit any visual
property (scale, color tint, per-mesh material) without hand-editing RON
files. This plan adds an **Objects** tab that follows the same list/detail
editor pattern used by Items, Landscape, and Furniture, and wires the
Importer's `ObjectMesh` export path to refresh that tab's list immediately,
with no campaign reload required.

## Current State Analysis

### Existing Infrastructure

| Concern | Current state |
|---|---|
| Registry file | `data/object_mesh_registry.ron`, format `ObjectMeshRegistry(meshes: {"key": "path", ...})` — string key to campaign-relative `CreatureDefinition` RON path. |
| Read-only domain type | [`ObjectMeshDatabase`](../../src/domain/world/object_mesh.rs) (`object_mesh.rs:127`) — loads the registry, eagerly resolves every entry to a full `CreatureDefinition`, exposes `lookup`/`has_mesh`/`all_mesh_ids`/`validate`. No add, rename, remove, or save methods; not meant for editing. |
| Write path | [`upsert_object_mesh_registry_entry`](../../sdk/campaign_builder/src/obj_importer_ui.rs) (`obj_importer_ui.rs:2057`) — defines a **private, local** `ObjectMeshRegistry { meshes: BTreeMap<String,String> }` struct (duplicate of the domain one) and hand-writes RON text line-by-line (`obj_importer_ui.rs:2085-2095`) instead of using `ron::ser`. Insert-or-replace by key only; no rename, no delete. |
| Import signal | [`ObjImporterUiSignal::ObjectMesh`](../../sdk/campaign_builder/src/lib.rs) handled at `lib.rs:1458-1463` — only calls `self.editor_registry.maps_editor_state.invalidate_mesh_cache()` (refreshes the Map Editor's mesh-ID autocomplete). No campaign-data list exists to refresh. |
| Comparable editors | [`LandscapeEditorState`](../../sdk/campaign_builder/src/landscape_editor.rs) — list/detail editor over `Vec<LandscapeDefinition>`, owned in `CampaignData::landscape_definitions` (`editor_state.rs:44`), loaded/saved via `load_landscape`/`save_landscape` (`campaign_io.rs:1903`, `:1944`) using `read_ron_collection::<T>`/`write_ron_collection::<T>` (`campaign_io.rs:47`, `:119`) — both `Vec<T>`-shaped, which does **not** match the `ObjectMeshRegistry(meshes: {...})` named-map shape. [`ItemMeshEditorState`](../../sdk/campaign_builder/src/item_mesh_editor.rs) — closer structural analogue: it scans a directory of raw `CreatureDefinition`/descriptor RON files directly (no separate "definition" layer with category/tags), same as how an Object entry *is* a `CreatureDefinition`, not a wrapper around one. |
| Tab registration | [`EditorTab`](../../sdk/campaign_builder/src/lib.rs) enum (`lib.rs:621-646`), its `name()` impl (`:649-676`), the tab strip list (`:1102-1125`), and the `match` dispatch in the central panel (`:1199` onward) — no `Objects` variant exists in any of the four places. |

### Identified Issues

1. No `EditorTab::Objects` variant, no tab button, no dispatch arm — Objects
   are completely invisible in the SDK UI.
2. No in-memory representation of the registry — `CampaignData` has no
   `objects` field, so there is nothing for a tab to display even if one
   existed.
3. `upsert_object_mesh_registry_entry` duplicates the domain's private
   `ObjectMeshRegistry` struct and serializes RON by hand-building strings
   instead of `ron::ser::to_string_pretty`, risking malformed output if a key
   or path ever contains a character that needs escaping beyond `{:?}`.
4. `ObjectMeshDatabase` has no save/rename/remove capability — it is a
   read-only runtime lookup, not an editor-shaped type. Building the editor
   directly on top of it would require bypassing it for every write.
5. Importing an Object mesh (`ExportType::ObjectMesh`) only invalidates the
   Map Editor's mesh cache; nothing refreshes an Objects list because no list
   exists. This is the literal bug the task description names: "Importing
   objects should update the list of Objects in the SDK immediately."

## Implementation Phases

### Phase 1: Domain Foundation — Editable Object Mesh Registry

#### 1.1 Foundation Work

Add a public, round-trippable raw registry type to
[`src/domain/world/object_mesh.rs`](../../src/domain/world/object_mesh.rs),
alongside the existing `ObjectMeshDatabase`:

- `ObjectMeshRegistryFile { pub meshes: BTreeMap<String, String> }` —
  `#[derive(Debug, Clone, Default, Serialize, Deserialize)]`. `BTreeMap` keeps
  serialized output key-sorted, matching the existing `sort_by_key` convention
  used by `upsert_mesh_registry_entry` for the numeric registries.
- `ObjectMeshRegistryFile::load(path: &Path) -> Result<Self, ObjectMeshError>` —
  reads and parses; returns `Err(ObjectMeshError::ReadError)` if `path` does
  not exist or cannot be read, and `Err(ObjectMeshError::ParseError)` if the
  contents are not valid RON. **`load` does not check `path.exists()` itself
  and does not return a default-empty registry on a missing file** — the
  caller (Phase 3's `load_objects`, Rule 13) owns the `path.exists()` guard
  and decides what an absent file means. `load` does **not** resolve
  referenced assets — that stays in `ObjectMeshDatabase`.
- `ObjectMeshRegistryFile::save(&self, path: &Path) -> Result<(), ObjectMeshError>` —
  creates parent directories, serializes with
  `ron::ser::to_string_pretty(self, PrettyConfig::new())`, writes the file.
  Add an `ObjectMeshError::WriteError(String)` variant for this path.
- `ObjectMeshRegistryFile::upsert(&mut self, key: &str, path: &str)`,
  `::rename(&mut self, old_key: &str, new_key: &str) -> bool`,
  `::remove(&mut self, key: &str) -> Option<String>` — pure in-memory map
  operations the editor and the importer both call before `save`.

#### 1.2 Add Foundation Functionality

Update [`upsert_object_mesh_registry_entry`](../../sdk/campaign_builder/src/obj_importer_ui.rs:2057)
to delegate to `ObjectMeshRegistryFile::load` → `::upsert` → `::save`,
deleting the private duplicate struct and the hand-built RON string lines
(`obj_importer_ui.rs:2069-2095`). Behavior must stay identical (insert-or-
replace by key, file created if absent) — this is a pure refactor, not a
behavior change.

#### 1.3 Integrate Foundation Work

No call sites outside `obj_importer_ui.rs` change in this phase.
`ObjectMeshDatabase` is untouched — it remains the read-only, asset-resolving
type used by game-runtime loading paths (`src/domain/campaign_loader.rs:553`,
`src/sdk/database.rs:1390,1623`).

#### 1.4 Testing Requirements

- Round-trip test: `upsert` two keys, `save`, `load`, assert both keys and
  paths survive.
- `rename` test: existing key renamed, old key gone, new key present with the
  same path; renaming a non-existent key returns `false` and leaves the map
  unchanged.
- `remove` test: existing key removed and returned; removing a missing key
  returns `None`.
- **Interop test (cross-type compatibility — mandatory, not optional):**
  write an `object_mesh_registry.ron` using `ObjectMeshRegistryFile::save`,
  then load that same file with the existing, untouched
  `ObjectMeshDatabase::load_from_registry` (`object_mesh.rs:174` — the
  function that deserializes into the private `ObjectMeshRegistry` struct at
  `object_mesh.rs:100`) and assert it succeeds and returns the same keys.
  This is the test that protects game-runtime loading
  (`src/domain/campaign_loader.rs:553`, `src/sdk/database.rs:1390,1623`) from
  a silent format break introduced by the new serializer — `load`/`save`
  round-tripping with itself (the test above) does **not** prove this; it must
  be a separate test.
- **New regression test for the importer write path (no pre-existing test to
  preserve):** confirmed by reading `obj_importer_ui.rs`'s `#[cfg(test)]`
  module (starts at `obj_importer_ui.rs:2579`) that no test currently exercises
  `upsert_object_mesh_registry_entry` or `ExportType::ObjectMesh` — the only
  existing mesh-registry regression tests cover `ExportType::Landscape`
  (`test_export_landscape_upserts_existing_mesh_registry_entry_by_id`,
  `obj_importer_ui.rs:3825`) and `ExportType::Item`. This phase must **write a
  new test**, `test_export_object_mesh_upserts_existing_registry_entry_by_key`,
  modeled directly on the Landscape test, asserting: exporting the same key
  twice replaces the path rather than duplicating the entry, and the resulting
  file round-trips through the Interop test above.

#### 1.5 Deliverables

- [x] `ObjectMeshRegistryFile` struct + `load`/`save`/`upsert`/`rename`/`remove`
  added to `src/domain/world/object_mesh.rs`
- [x] `ObjectMeshError::WriteError` variant added
- [x] `upsert_object_mesh_registry_entry` in `obj_importer_ui.rs` refactored
  to use `ObjectMeshRegistryFile` (duplicate struct + manual RON lines
  removed)
- [x] New unit tests for `load`/`save`/`upsert`/`rename`/`remove` (round-trip,
  rename existing/missing, remove existing/missing, plus dedicated
  nonexistent-path and garbage-content error-path tests)
- [x] New interop test: `ObjectMeshDatabase::load_from_registry` successfully
  reads a file written by `ObjectMeshRegistryFile::save`
  (`test_object_mesh_registry_file_interop_with_object_mesh_database`)
- [x] New test `test_export_object_mesh_upserts_existing_registry_entry_by_key`
  added to `obj_importer_ui.rs` (no pre-existing test covered this path)
- [x] Bonus fix found during implementation: `load()` routes through
  `ron::Value` rather than `ron::from_str::<Self>` directly, because RON
  rejects a named struct whose on-disk identifier doesn't exactly match the
  target type name — every pre-existing `object_mesh_registry.ron` (including
  `campaigns/tutorial/data/object_mesh_registry.ron`) is on disk as
  `ObjectMeshRegistry(...)`, not `ObjectMeshRegistryFile(...)`, and would have
  failed to load without this. Locked in by
  `test_object_mesh_registry_file_load_legacy_named_format` and
  `test_object_mesh_registry_file_load_real_tutorial_campaign_file`.

#### 1.6 Success Criteria

`cargo test` passes. `object_mesh_registry.ron` produced by the refactored
`upsert_object_mesh_registry_entry` is **semantically equivalent** to entries
the prior hand-written serializer produced — same keys, same paths, valid RON
— though exact whitespace/formatting will differ because `ron::ser` and the
hand-rolled string builder do not format identically; this is expected and
not a regression. The interop test (§ 1.4) passes, confirming
`ObjectMeshDatabase::load_from_registry` still parses the new output.

---

### Phase 2: Objects Editor Tab — Registry List and Edit Form

This phase must satisfy every applicable rule in
[`sdk/AGENTS.md`](../../sdk/AGENTS.md). Rule numbers below map directly to
that document.

#### 2.1 Foundation Work — `objects_editor.rs`

Create `sdk/campaign_builder/src/objects_editor.rs`, modeled on
[`landscape_editor.rs`](../../sdk/campaign_builder/src/landscape_editor.rs)
for the list/edit-mode skeleton (List/Edit enum, `show`/`show_list`/`show_edit`
split) and on
[`item_mesh_editor.rs`](../../sdk/campaign_builder/src/item_mesh_editor.rs)
for the fact that an entry *is* a mesh asset directly (no separate
"definition" layer with category/tags/flags):

- `ObjectEntry { pub key: String, pub file_path: String, pub definition: CreatureDefinition }` —
  one row per registry entry; `definition` is the parsed `CreatureDefinition`
  loaded from `file_path`.
- `ObjectsEditorState` fields: `search_query: String`, `selected: Option<usize>`,
  edit-mode fields (`mode: ObjectsEditorMode`, `edit_index: Option<usize>`,
  `key_buffer: String`, `edit_buffer: Option<CreatureDefinition>`,
  `color_tint_enabled: bool` (mirrors the `Option<[f32;4]>` toggle pattern
  used elsewhere), `key_error: Option<String>` (set when a rename collides
  with an existing key), `requested_signal: Option<ObjectsEditorSignal>`,
  **`needs_initial_load: bool`** (drives the Rule 13 auto-load guard in
  `show()`; see below — do **not** model this on `LandscapeEditorState`, which
  predates Rule 13 and has no such flag).
- `ObjectsEditorState` needs **two** distinct reset methods — conflating them
  causes either a Rule 13 violation or a stale-selection bug, so keep them
  separate:
  - `reset_for_new_campaign(&mut self)` — clears `search_query`, `selected`,
    resets `mode` to `List`, clears `edit_buffer`/`key_buffer`/`key_error`,
    **and sets `needs_initial_load = true`**. Called only from
    `do_new_campaign`/`do_open_campaign` (Phase 3.2). Required by the
    `sdk/AGENTS.md` Rule 13 checklist (`AGENTS.md:776-796`); model it on
    `levels_editor.rs:367` / `skills_editor.rs:161`, not on
    `landscape_editor.rs`'s inline `ObjectsEditorState::new()` reset, which
    predates Rule 13.
  - `reset_selection(&mut self)` — clears `selected`, resets `mode` to
    `List`, clears `edit_buffer`/`key_buffer`/`key_error`, but **does not**
    touch `needs_initial_load`. Call this from `load_objects` (Phase 3.1)
    on *every* call, including importer-triggered reloads (Phase 4) — not
    just on new/open campaign. This is necessary because `ObjectsEditorState`
    does not own `Vec<ObjectEntry>` (it receives `&mut Vec<ObjectEntry>` by
    reference from `CampaignData.objects` each frame, same architecture as
    `LandscapeEditorState`); if the Vec is rebuilt by a reload while the user
    has an in-progress edit, a stale `edit_index` can point at the wrong
    entry or panic on out-of-bounds access. Landscape's `load_landscape`
    avoids this today by resetting unconditionally on every load
    (`campaign_io.rs:1911-1925`) — `reset_selection` exists so Objects gets
    the same safety without re-triggering an unwanted auto-load on every
    importer reload (which calling `reset_for_new_campaign` instead would
    do, since it flips `needs_initial_load` back to `true`).
- `ObjectsEditorSignal::OpenInObjImporter` — same cross-tab pattern as
  `LandscapeEditorSignal::OpenInObjImporter` (`landscape_editor.rs:30-33`),
  for the "re-import geometry" workflow (deep vertex/mesh edits stay in the
  Importer; this tab edits registry-level and material-level properties).
- Add `pub mod objects_editor;` to the module declarations in
  [`lib.rs`](../../sdk/campaign_builder/src/lib.rs) (alongside
  `pub mod landscape_editor;`) — required for the new module to be visible to
  the rest of the crate; easy to forget since no other phase step names it
  explicitly.

#### 2.2 List View (Rules 1, 2, 7, 9, 10, 13, 15)

- `show()`'s signature takes `campaign_dir: Option<&Path>` (already required
  for the `OpenInObjImporter` signal) and begins with the Rule 13 auto-load
  guard, modeled on `levels_editor.rs:314-332`: if `self.needs_initial_load`
  and `campaign_dir` is `Some`, read `data/object_mesh_registry.ron` via
  `ObjectMeshRegistryFile::load` (Phase 1) and parse each referenced
  `CreatureDefinition`, applying the same per-entry leniency as Phase 3's
  `load_objects` (one unreadable entry is skipped, not fatal); rebuild the
  passed-in `&mut Vec<ObjectEntry>` from the result. Set
  `needs_initial_load = false` unconditionally afterward — never retry on
  every frame. This makes `objects_editor.rs` self-sufficient for the
  isolated-fixture testing in § 2.6: tests call `show()` with
  `campaign_dir: None`, which leaves the guard inert and the fixture `Vec`
  untouched, exactly as `levels_editor.rs`'s own tests do.
- `ui.horizontal_wrapped` toolbar row: search box, entry count, "📥 Import
  Object Mesh" button setting `requested_signal = Some(OpenInObjImporter)`
  (Rule 12 — more than two controls, must wrap).
- `TwoColumnLayout::new("objects_editor").show_split(ui, left, right)` (Rule 9
  — never raw `SidePanel`).
- Left column: `ScrollArea::vertical().id_salt("objects_editor_list_scroll")`,
  each row `ui.push_id(&entry.key, |ui| { ... })` (Rule 1 — string key is the
  stable unique identifier here, unlike Landscape's numeric `id`).
  `show_standard_list_item` with `StandardListItemConfig::new(&entry.definition.name)`
  and a `MetadataBadge::new(&entry.key)` showing the registry key (Rule 15 —
  no `selectable_label` with embedded metadata string).
- Right column: preview panel showing key, resolved name, mesh count, scale,
  color tint, and a per-mesh table (name, vertex count, texture path) — read
  only, mirrors `show_landscape_preview` (`landscape_editor.rs:623-667`).
- Pre-compute `filtered_rows: Vec<usize>` and
  `preview_snapshot: Option<ObjectEntry>` (clone) before `show_split` (Rule 10).
  Deferred `pending_selection`/`pending_edit`/`pending_delete` applied after
  `show_split` returns, exactly as `show_list` does in
  `landscape_editor.rs:238-348`.

#### 2.3 Edit Form (Rules 3, 7, 12, 14, 16)

Editable fields, in order:

- **Key** — `egui::TextEdit::singleline(&mut self.key_buffer)`. This is the
  one field exempt from Rule 14 (it is the primary key being assigned, not a
  reference *to* another registry) — but on Save, validate uniqueness against
  every other entry's key and set `self.key_error` instead of saving if it
  collides; render the error inline above the action row.
- **Name** — `egui::TextEdit::singleline(&mut buf.name)` (writes
  `CreatureDefinition.name`).
- **Scale** — `egui::DragValue::new(&mut buf.scale).speed(0.01).range(0.001..=100.0)`.
- **Color tint** — checkbox to toggle `Option<[f32;4]>` on/off (mirrors the
  `description_buffer`/`icon_buffer` empty-means-None convention in
  `landscape_editor.rs:385-394`), `egui::widgets::color_picker::color_edit_button_rgba`
  when enabled.
- **Per-mesh material summary** — `egui::Grid` listing each mesh's `name`
  (`MeshDefinition.name: Option<String>` — display `"(unnamed)"` when `None`),
  `material.base_color`, `material.metallic`, `material.roughness`,
  `material.emissive`, and `texture_path` (the last is on `MeshDefinition`
  directly, not nested under `material`); base_color/metallic/roughness/
  emissive are editable inline (same fields the Game Engine Sky / GLB-import
  bug fix in `docs/explanation/next_plans.md` flagged as needing sane
  defaults — this editor is where an author corrects a bad import after the
  fact). **`MeshDefinition.material` is `Option<MaterialDefinition>`** — a
  raw OBJ import commonly leaves this `None`. Before rendering editable
  fields for a mesh, call
  `mesh.material.get_or_insert_with(MaterialDefinition::default)` so the grid
  always has a concrete `MaterialDefinition` to bind widgets to; do not skip
  rendering the row or leave the fields unenterable when `material` starts as
  `None`. Editing vertex/index/UV data and adding/removing meshes is **out of
  scope** — that is the Importer's job; provide an "↻ Re-import in Importer"
  button next to the grid that sets `requested_signal`.
- Bottom action row, `ui.horizontal_wrapped` (Rule 12): `⬅ Back to List`,
  `💾 Save`, `✕ Cancel`, in that order (Rule 16). `Save`:
  1. Validates the key (uniqueness; non-empty).
  2. Writes the edited `CreatureDefinition` back to `file_path` via
     `ron::ser::to_string_pretty`.
  3. If the key changed, calls `ObjectMeshRegistryFile::rename` (Phase 1) and
     `save`; if only the path changed, `upsert`.
  4. Updates the in-memory `ObjectEntry` and calls `ui.ctx().request_repaint()`.

#### 2.4 Testing Requirements (Rule 11)

- `filtered_rows` matches by key and by resolved name (contract test, not a
  hardcoded count).
- `enter_edit` populates `key_buffer` and `edit_buffer` from the selected
  entry.
- `apply_edit` rejects a rename to a key that already exists elsewhere in the
  list (assert `key_error.is_some()`, assert the original entry list is
  unmodified) and accepts a rename to a unique key.
- Delete removes the entry and adjusts `selected` using the same
  select-then-shift contract as `landscape_editor.rs`'s delete tests
  (`test_delete_clears_selection_when_selected_item_removed`, etc. — copy the
  three-case pattern: removed-is-selected, earlier-removed, later-removed).
- `reset_for_new_campaign` sets `needs_initial_load == true` and clears
  `selected`/`mode`/`edit_buffer`.
- `reset_selection` clears `selected`/`mode`/`edit_buffer` but leaves
  `needs_initial_load` unchanged (assert both `true→true` and `false→false`
  cases).
- `show()`'s auto-load guard, called with `campaign_dir: None`, leaves the
  passed-in fixture `Vec<ObjectEntry>` untouched (proves § 2.6's isolated
  testing remains valid once the guard does real I/O when `campaign_dir` is
  `Some`).

#### 2.5 Deliverables

- [x] `sdk/campaign_builder/src/objects_editor.rs` created with
  `ObjectEntry`, `ObjectsEditorState`, `ObjectsEditorSignal`
- [x] `pub mod objects_editor;` added to `lib.rs`
- [x] `reset_for_new_campaign` and `reset_selection` methods added, with
  distinct, tested behavior re: `needs_initial_load`
- [x] List view: `TwoColumnLayout`, `show_standard_list_item`, push_id per row,
  search filter
- [x] Edit view: key/name/scale/color-tint/per-mesh-material fields,
  key-uniqueness validation, `material.get_or_insert_with` handling for
  meshes with no material yet, `Back to List`/`Save`/`Cancel` row
  (`horizontal_wrapped`)
- [x] Unit tests per § 2.4 (20 tests total, including two `show()` smoke tests
  through a real `egui::Ui` via the `egui::Context::run` harness — modeled on
  `obj_importer_ui.rs`'s `test_show_obj_importer_tab_renders_*` pattern —
  that caught a missing `request_repaint()` on the Save-failure path during
  self-review)
- [x] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` pass for the new
  module (and for the full workspace: `cargo nextest run --workspace
  --all-features` → 7944 passed, 0 failed, 8 skipped)
- [x] sdk/AGENTS.md egui ID audit checklist passes for this module, with one
  documented deviation: the per-mesh material rows use `push_id` +
  `group`/`horizontal_wrapped` (mirroring `obj_importer_ui.rs`'s per-mesh
  color editor) instead of `egui::Grid`, because `push_id` wrapping a
  `Grid::end_row()` call is an untested combination in this codebase and the
  established precedent for "interactive per-mesh rows" already solves this
  exact problem without `egui::Grid`. The read-only preview's per-mesh table
  *does* use `egui::Grid` (no `push_id` needed — plain `ui.label` calls
  allocate no persistent widget IDs to collide).

#### 2.6 Success Criteria

An author can open the (not-yet-wired) `ObjectsEditorState::show` in
isolation, see a list of `ObjectEntry` rows built from a `Vec<ObjectEntry>`
fixture, select one, edit its name/scale/color tint, rename its key to a free
key, save, and see the change reflected in the passed-in `Vec`. Renaming to a
key already in use is rejected with a visible inline error and no data loss.

---

### Phase 3: Campaign Builder Integration — Load, Save, Tab Wiring

#### 3.1 Feature Work — `CampaignData` and Load/Save

- Add `pub objects: Vec<objects_editor::ObjectEntry>` to `CampaignData`
  ([`editor_state.rs:27`](../../sdk/campaign_builder/src/editor_state.rs)),
  next to `landscape_definitions`.
- Add `pub objects_editor_state: objects_editor::ObjectsEditorState` to
  `EditorRegistry` ([`editor_state.rs:78`](../../sdk/campaign_builder/src/editor_state.rs))
  and its `Default` impl (`editor_state.rs:155-185`).
- Add `load_objects`/`save_objects` to
  [`campaign_io.rs`](../../sdk/campaign_builder/src/campaign_io.rs). Use
  `load_landscape`/`save_landscape` (`campaign_io.rs:1903`, `:1944`) only as a
  *file-path-resolution* reference — **do not** copy their reset pattern,
  which predates `sdk/AGENTS.md` Rule 13 and resets editor state inline with
  no `needs_initial_load` flag. Use `ObjectMeshRegistryFile::load` (Phase 1)
  instead of `read_ron_collection::<Vec<T>>` (the registry is a named map, not
  a `Vec`). `load_objects` reads `data/object_mesh_registry.ron` via
  `ObjectMeshRegistryFile::load`, then for each `(key, path)` reads and parses
  the `CreatureDefinition` at `campaign_dir.join(path)`, building
  `Vec<ObjectEntry>`. Follow the Rule 13 checklist exactly, modeled on
  `levels_editor.rs`'s compliant pattern (`reset_for_new_campaign` at
  `levels_editor.rs:367`, `needs_initial_load` field at `levels_editor.rs:192`)
  rather than Landscape's:
  - guard with `registry_path.exists()` before any read;
  - missing file → `self.logger.debug(...)`, `objects.clear()`, never touch
    `status_message`;
  - parse/asset-read errors → `self.logger.warn(...)`, never
    `status_message`;
  - call `self.editor_registry.objects_editor_state.reset_selection()`
    (Phase 2, **not** `reset_for_new_campaign()`) on both the success and
    failure path, every time `load_objects` runs — this drops any
    in-progress edit/selection so a rebuilt `Vec<ObjectEntry>` never leaves a
    stale index behind, without disturbing `needs_initial_load`;
  - on success, additionally set `self.editor_registry.objects_editor_state
    .needs_initial_load = false` (the explicit `load_objects()` call satisfies
    the same contract `show()`'s auto-load guard exists to backstop, so the
    guard must not re-run on the next frame).
  - `reset_for_new_campaign()` is called separately, only from
    `do_new_campaign`/`do_open_campaign` (Phase 3.2) — `load_objects` itself
    never calls it.
- `save_objects` writes each entry's `CreatureDefinition` back to its
  `file_path` and rewrites `object_mesh_registry.ron` via
  `ObjectMeshRegistryFile::save` from the current `Vec<ObjectEntry>` keys/paths.

#### 3.2 Integrate Feature — Tab Registration

- Add `Objects` to `EditorTab` enum
  ([`lib.rs:621-646`](../../sdk/campaign_builder/src/lib.rs)), to `name()`
  (`:649-676`, label `"Objects"`), to the tab strip list (`:1102-1125`), and a
  dispatch arm in the central-panel `match` (alongside `EditorTab::Landscape`
  at `:1344-1366`):

  ```text
  EditorTab::Objects => {
      self.editor_registry.objects_editor_state.show(
          ui,
          &mut self.campaign_data.objects,
          self.campaign_dir.as_deref(),
          &mut self.unsaved_changes,
      );
      if let Some(objects_editor::ObjectsEditorSignal::OpenInObjImporter) =
          self.editor_registry.objects_editor_state.requested_signal.take()
      {
          self.ui_state.active_tab = EditorTab::Importer;
          self.obj_importer_state.export_type = obj_importer::ExportType::ObjectMesh;
          ui.ctx().request_repaint();
      }
  }
  ```

- Call `load_objects()` everywhere `load_landscape()` is called today: the
  campaign-open path (`lib.rs:256`).
- In `do_new_campaign` (`campaign_io.rs:2799`) and `do_open_campaign`
  (`campaign_io.rs:3132`), call
  `self.editor_registry.objects_editor_state.reset_for_new_campaign()` and
  clear `self.campaign_data.objects` — at the same point Landscape's
  equivalent inline reset happens today (`campaign_io.rs:2857-2858`), but
  using the Phase 2 `reset_for_new_campaign()` method, not
  `ObjectsEditorState::new()`. In `do_open_campaign`, this call must come
  **before** the `load_objects()` call, per the Rule 13 checklist order.
- Call `save_objects()` everywhere `save_landscape()` is called today
  (`campaign_io.rs:2993` — the "save all" sweep).

#### 3.3 Configuration Updates

No `CampaignConfig` field is required — `object_mesh_registry.ron` is a
fixed, well-known path (matching how `furniture_mesh_registry.ron` and
`landscape_mesh_registry.ron` are also hardcoded, not configurable filenames).

#### 3.4 Testing Requirements (Rule 13 checklist)

- `load_objects` with no registry file: `objects` ends up empty,
  `status_message` is untouched, a debug-level log line is emitted.
- `load_objects` with a registry referencing a missing asset file: that one
  entry is skipped with a `warn`-level log line; other valid entries still
  load (do not fail the whole load on one bad entry — note this is stricter
  than `ObjectMeshDatabase::load_from_registry`, which errors out entirely on
  the first bad asset; the editor's load should be lenient so one corrupt
  Object doesn't hide every other Object from the author).
- `save_objects` round-trips: load a fixture campaign dir (use `tempfile::tempdir`,
  the established pattern used throughout `sdk/campaign_builder/src/*.rs`),
  save, reload, assert the same keys/paths/names survive.
- New-campaign creation clears `objects` and calls
  `objects_editor_state.reset_for_new_campaign()`, leaving
  `needs_initial_load == true`.
- `load_objects()` success sets `needs_initial_load == false` on
  `objects_editor_state`.
- `do_open_campaign` ordering: `reset_for_new_campaign()` runs before
  `load_objects()` (assert via the loaded data, not implementation
  internals — e.g. open a fixture campaign and confirm `objects` reflects the
  new campaign's registry, not a previous one left over from a prior
  `do_open_campaign` call in the same test).

#### 3.5 Deliverables

- [x] `CampaignData::objects` field added
- [x] `EditorRegistry::objects_editor_state` field added (+ `Default` impl)
- [x] `load_objects`/`save_objects` added to `campaign_io.rs`, Rule 13
  compliant (`reset_for_new_campaign()` + `needs_initial_load`, not the
  Landscape-style inline reset) — `needs_initial_load` is set to `false`
  only on a genuinely successful load, matching `load_levels`'s own
  precedent (missing-file and parse-error branches leave it untouched)
- [x] `EditorTab::Objects` variant + name + tab strip + dispatch arm added to
  `lib.rs`
- [x] `load_objects()`/`save_objects()` wired into the same call sites as
  `load_landscape()`/`save_landscape()` (`do_new_campaign`, `do_open_campaign`,
  the `do_save_campaign` save-all sweep, and the auto-load-on-startup sequence
  in `lib.rs`'s `run()`)
- [x] `do_new_campaign`/`do_open_campaign` call
  `objects_editor_state.reset_for_new_campaign()` in the correct order
  relative to `load_objects()`
- [x] Tests per § 3.4 pass (6 new tests in `campaign_io.rs`'s `mod tests`;
  the literal "`do_open_campaign` ordering" test could not call
  `do_open_campaign()` directly since it blocks on a native file-picker
  dialog — it instead replicates `do_open_campaign`'s exact
  reset/clear/load sequence against two distinct fixture campaign
  directories in succession and asserts no key leaks from the first into
  the second)

#### 3.6 Success Criteria

Opening a campaign with a populated `object_mesh_registry.ron` shows every
entry in the new Objects tab. Editing and saving an entry persists to disk
and survives a reload. Creating a new campaign starts with an empty Objects
list, not stale data from a previously opened campaign.

---

### Phase 4: Importer Integration — Immediate List Refresh on Import

#### 4.1 Feature Work

Update the `ObjImporterUiSignal::ObjectMesh` handler in
[`lib.rs:1458-1463`](../../sdk/campaign_builder/src/lib.rs).

**Decision: also switch the active tab to `Objects`, matching the Furniture
and Landscape handlers exactly** — `ObjImporterUiSignal::Furniture`
(`lib.rs:1438-1444`) and `::Landscape` (`lib.rs:1445-1456`) both call their
`load_*()` *and* set `self.ui_state.active_tab`. The `ObjectMesh` handler
must do the same for UX consistency across all three mesh-import flows;
there is no stated reason for Objects to behave differently, so it should
not silently diverge from its two siblings:

```text
obj_importer_ui::ObjImporterUiSignal::ObjectMesh => {
    let importer_status = self.obj_importer_state.status_message.clone();
    self.editor_registry.maps_editor_state.invalidate_mesh_cache();
    self.load_objects();                         // NEW — refresh the Objects tab list
    self.ui_state.status_message = importer_status;
    self.ui_state.active_tab = EditorTab::Objects; // NEW — matches Furniture/Landscape
    ui.ctx().request_repaint();
}
```

This is the direct fix for "Importing objects should update the list of
Objects in the SDK immediately": `load_objects()` re-reads
`object_mesh_registry.ron` (already updated on disk by
`upsert_object_mesh_registry_entry`, Phase 1) and rebuilds
`CampaignData.objects`, so the very next frame the Objects tab reflects the
new entry with no save-and-reopen cycle required — matching how
`ObjImporterUiSignal::Furniture` / `::Landscape` already call
`load_furniture()` / `load_landscape()` (`lib.rs:1440`, `:1447`) **and**
switch tabs.

#### 4.2 Integrate Feature

No further wiring needed — `load_objects` (Phase 3) already calls
`objects_editor_state.reset_selection()` on every invocation, so any
in-progress edit-mode selection in the Objects tab is safely dropped back to
List mode on re-import, consistent with how `load_landscape` resets
`landscape_editor_state` unconditionally on every load. Note this is
deliberately `reset_selection()`, not `reset_for_new_campaign()` — the latter
would also flip `needs_initial_load` back to `true`, which is wrong for an
importer-triggered reload of an already-open campaign.

#### 4.3 Configuration Updates

None.

#### 4.4 Testing Requirements

- Integration-style test (or a focused unit test on the handler logic):
  simulate an `ObjectMesh` export outcome, call the equivalent of
  `load_objects()`, assert the new key appears in `CampaignData.objects`
  without an explicit reload call or campaign reopen.
- Assert `self.ui_state.active_tab == EditorTab::Objects` after the handler
  runs, matching the Furniture/Landscape handlers' tab-switch behavior.
- Regression: `maps_editor_state.invalidate_mesh_cache()` is still called —
  the Map Editor's mesh-ID autocomplete must keep working exactly as before.

#### 4.5 Deliverables

- [x] `self.load_objects()` call added to the `ObjectMesh` import signal
  handler in `lib.rs`
- [x] `self.ui_state.active_tab = EditorTab::Objects` added to the same
  handler, matching Furniture/Landscape
- [x] Test confirming immediate in-memory refresh after import
  (`test_object_mesh_import_refreshes_objects_list_without_reload`)
- [x] Test confirming the tab switch
  (`test_object_mesh_import_handler_switches_to_objects_tab`) — the literal
  match arm in `lib.rs` isn't independently callable from a unit test (it's
  embedded in the `eframe::App::update` central-panel closure with no
  exposed `egui::Ui`/`ctx` seam), so the test inline-replicates the handler's
  exact `&mut self`-only sequence, omitting only the literal
  `ui.ctx().request_repaint()` line — same approach Phase 3 used for the
  dialog-gated `do_open_campaign()`.
- [x] Existing Map Editor mesh-cache invalidation behavior unchanged
  (`test_object_mesh_import_handler_still_invalidates_mesh_cache`, asserting
  `maps_editor_state.mesh_cache_dirty` directly)

#### 4.6 Success Criteria

An author imports an OBJ/GLB mesh with export type "Object Mesh", and without
reopening the campaign or clicking any refresh button, the SDK switches to
the Objects tab and the list already contains the new entry — consistent
with what already happens today for Furniture and Landscape imports.

---

### Phase 5: Documentation

#### 5.1 Author Guide

Add an "Objects" subsection to
[`docs/explanation/modding_guide.md`](./modding_guide.md) (or extend the
existing "Interactive Objects" section added by the unified-objects plan,
`unified_objects_and_events.md` § 7.1) covering: what an Object is (a
string-keyed mesh entry in `object_mesh_registry.ron`), how to create one
(Importer → Object Mesh export, or the Objects tab's Import button), and how
to edit one (Objects tab: name, scale, color tint, per-mesh material; full
geometry re-import goes back through the Importer).

#### 5.2 Inline Code Documentation

- Module-level doc comment on `objects_editor.rs` describing scope: registry-
  and material-level editing, not vertex/geometry editing.
- Doc comment on `ObjectMeshRegistryFile` (Phase 1) cross-referencing
  `ObjectMeshDatabase` and explaining why two types exist (one read-only +
  asset-resolving for runtime, one edit-shaped for the SDK).

#### 5.3 Deliverables

- [x] `modding_guide.md` Objects section written — extended the existing
  `## Interactive Objects — Meshes and Events` → `### Placement Workflow`
  → step 1 (`#### 1. Register the mesh in object_mesh_registry.ron`) with a
  new `#### Editing registry entries with the Objects tab` subsection
  (lines 518-563), per the plan's "extend the existing section" option,
  rather than adding a disconnected new top-level section
- [x] `objects_editor.rs` module doc comment — already had a "Scope:"
  paragraph from Phase 2 satisfying §5.2's first bullet; extended it with
  two new paragraphs covering `sdk/AGENTS.md` rule compliance (Rules
  1/2/5/7/9/10/12/13/15/16 cited by number, the Rule 14 Key-field exemption
  explained, and Rules 3/4/6/8 explicitly noted as not applicable with the
  reason) and how the module wires into `editor_state.rs`/`campaign_io.rs`/
  `lib.rs`, satisfying the "without reading `lib.rs` first" success
  criterion
- [x] `ObjectMeshRegistryFile` doc comment explaining the split from
  `ObjectMeshDatabase` — already written during Phase 1 and verified
  sufficient as-is (cross-references `ObjectMeshDatabase` via intra-doc
  link, explains the read-only/runtime vs. edit-shaped/SDK split, and
  justifies the `BTreeMap` choice); no change needed

#### 5.4 Success Criteria

A new SDK contributor can read `objects_editor.rs`'s module doc and
`sdk/AGENTS.md` and understand the editor's scope and rule compliance without
reading `lib.rs` first.

## Copyright

New and modified source files in this plan use the
[SPDX](https://spdx.github.io/spdx-spec/) header convention already present
in every file referenced above (`// SPDX-FileCopyrightText: ...` /
`// SPDX-License-Identifier: Apache-2.0`).
