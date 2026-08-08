# Mesh Editor Removal Implementation Plan

## Overview

Removes all interactive mesh-editing surfaces from the Campaign Builder SDK
(`sdk/campaign_builder/`) while fully preserving the existing glTF/OBJ → RON
import pipeline (the "Importer" tab). The user's workflow authors 3D models in
Blender and imports them as glTF/OBJ; the in-SDK editing UI is unused and
measurably slows the SDK. Four independently-shippable removals are staged as
phases so each can be built/tested in isolation: the standalone "Item Meshes"
tab, the mesh-editing panel embedded in the Creatures editor, three orphaned
raw mesh-editing modules never wired into any UI, and one dead breadcrumb
helper. Nothing in the runtime game crates (outside `sdk/`) depends on any
code removed here — confirmed by exploration.

---

## Current State Analysis

### Existing Infrastructure

| Area | Files | Notes |
|---|---|---|
| Item Meshes tab | `sdk/campaign_builder/src/item_mesh_editor.rs` (~3071 lines), `item_mesh_workflow.rs` (~472 lines), `item_mesh_undo_redo.rs` | Self-contained trio; `EditorTab::ItemMeshes` in [lib.rs](../../sdk/campaign_builder/src/lib.rs); edits `ItemMeshDescriptor`, writes `CreatureDefinition` RON under `assets/items/` |
| Creature mesh editor | `sdk/campaign_builder/src/creatures_editor/mod.rs` (~4566 lines), `creatures_editor/mesh_ui.rs` | Baked into the main creature edit screen (not toggle-gated — `show_mesh_editor`/`show_mesh_list` fields are dead, never read); three-panel layout at `mod.rs:1775-1798` |
| Read-only preview (kept) | `sdk/campaign_builder/src/creatures_editor/preview_panel.rs` | `show_preview_panel`, `sync_preview_renderer_from_edit_buffer`, `current_mesh_visibility`, `build_preview_statistics` — independent of the mesh editor |
| Orphaned raw editors | `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs` (~1000+ lines each), declared in `lib.rs:70-74` | Zero UI callers anywhere in the repo; only consumers are each other and `tests/mesh_editing_tests.rs` |
| Import pipeline (kept) | `mesh_glb_io.rs`, `mesh_obj_io.rs`, `obj_importer.rs`, `obj_importer_ui.rs`, `EditorTab::Importer` | Already writes the same `CreatureDefinition` RON format the runtime reads; architecturally independent of both editors above |
| Runtime consumer | `src/domain/campaign_loader.rs:490-495`, `src/domain/items/database.rs` (`ItemMeshDatabase`), `src/game/systems/item_world_events.rs:317-347` | Reads `data/item_mesh_registry.ron` → `CreatureDefinition`; registry is populated only by the Importer, never by `item_mesh_editor.rs` |

### Identified Issues

1. `item_mesh_editor.rs` + `item_mesh_workflow.rs` + `item_mesh_undo_redo.rs` implement a full interactive editor for an asset type the user now authors exclusively in Blender.
2. `creatures_editor/mod.rs` unconditionally renders an interactive mesh-list/properties panel on every creature edit screen, adding UI weight with no current use.
3. `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs` are dead code — declared as modules but never reachable from any tab.
4. `creatures_workflow.rs::enter_mesh_editor` is a dead breadcrumb helper with zero production call sites.
5. `tests/mesh_editing_tests.rs` mixes coverage for modules being deleted (`mesh_vertex_editor`, `mesh_normal_editor`, `mesh_index_editor`) with coverage for modules being kept (`mesh_obj_io`, `mesh_validation`), so it cannot be deleted wholesale without first checking for coverage loss.

---

## Implementation Phases

### Phase 1: Remove the Standalone "Item Meshes" Tab

#### 1.1 Foundation Work — Delete the Editor Trio

Delete [item_mesh_editor.rs](../../sdk/campaign_builder/src/item_mesh_editor.rs),
[item_mesh_workflow.rs](../../sdk/campaign_builder/src/item_mesh_workflow.rs), and
[item_mesh_undo_redo.rs](../../sdk/campaign_builder/src/item_mesh_undo_redo.rs) in
full — no other code depends on their internals outside the integration
points below.

#### 1.2 Remove Tab Plumbing in `lib.rs`

In [lib.rs](../../sdk/campaign_builder/src/lib.rs):

- Remove the three `mod` declarations (~lines 57-59).
- Remove the `EditorTab::ItemMeshes` variant and its `"Item Meshes"` name arm
  (~lines 649, 679).
- Remove the `item_mesh_editor_state` field and its `Default` initializer
  (~lines 755, 805).
- Remove the `EditorTab::ItemMeshes` central-panel match arm, including
  `ItemMeshEditorSignal::OpenInItemsEditor` handling and the reverse
  `requested_open_item_mesh` handling (~lines 1236-1268).
- Remove the `item_mesh_editor_state.load_from_campaign(dir)` call inside the
  Importer's `Item` export signal handler (~line 1486).

#### 1.3 Integrate — Remove Cross-Tab Link and Load Hook

- [campaign_io.rs](../../sdk/campaign_builder/src/campaign_io.rs): remove the
  `item_mesh_editor_state.load_from_campaign(dir)` call in the campaign-open
  flow (~lines 3355-3365).
- [items_editor.rs](../../sdk/campaign_builder/src/items_editor.rs): remove the
  `requested_open_item_mesh: Option<ItemId>` field (decl ~line 96, default
  init ~line 112) and its "Open in Item Mesh Editor" button/handler (~line
  1074), plus the associated test.

#### 1.4 Testing Requirements

- `cargo build -p campaign_builder` compiles clean with no dangling
  references to `item_mesh_editor`, `ItemMeshEditorState`,
  `ItemMeshEditorSignal`, or `requested_open_item_mesh`.
- `cargo test -p campaign_builder` passes (existing suite, minus anything
  removed in this phase).

#### 1.5 Deliverables

- [ ] `item_mesh_editor.rs`, `item_mesh_workflow.rs`, `item_mesh_undo_redo.rs`
      deleted
- [ ] `lib.rs` — mod decls, `EditorTab::ItemMeshes`, `item_mesh_editor_state`
      field, tab match arm, load-hook call all removed
- [ ] `campaign_io.rs` — load hook removed
- [ ] `items_editor.rs` — `requested_open_item_mesh` field, button, handler,
      and test removed
- [ ] All quality gates pass (`cargo fmt`, `cargo check`, `cargo clippy`,
      `cargo test -p campaign_builder`)

#### 1.6 Success Criteria

The "Item Meshes" tab no longer appears in the sidebar; the SDK builds,
starts, and loads an existing campaign (e.g. `campaigns/tutorial`) without
error or panic.

---

### Phase 2: Remove the Embedded Creature Mesh Editor

#### 2.1 Foundation Work — Identify Editing vs. Display Code

In [creatures_editor/mod.rs](../../sdk/campaign_builder/src/creatures_editor/mod.rs),
confirm the split (already verified by exploration): editing UI is entirely
separable from `preview_panel.rs` (read-only 3D render, untouched) and from
general creature CRUD/validation (`refresh_validation_state`,
`show_creature_level_properties`, `show_registry_preview_panel`, save/export/
delete logic — all untouched).

#### 2.2 Remove Editing UI and State

- Remove `mesh_list_panel` / `mesh_properties_panel` egui panel wiring
  (~lines 1776-1794) and the primitive-dialog invocation (~lines 1801-1803).
- Remove functions: `show_mesh_list_panel` (~1846-1950),
  `validate_selected_mesh` (~2219-2272), `estimate_primitive_geometry`
  (~2479-2490), `show_primitive_replacement_dialog` (~2493-2643),
  `apply_primitive_replacement` (~2643-2717), and the dead
  `_legacy_show_mesh_list_and_editor` (~2719-2947).
- Remove editing-only struct fields and initializers: `show_mesh_list`,
  `show_mesh_editor`, `selected_mesh_index`, `mesh_edit_buffer`,
  `mesh_transform_buffer`, `mesh_visibility`, `show_primitive_dialog`,
  `primitive_type`, `primitive_size`, `primitive_segments`,
  `primitive_rings`, `primitive_use_current_color`, `primitive_custom_color`,
  `primitive_preserve_transform`, `primitive_keep_name`, `uniform_scale`
  (decls ~138-216, inits ~316-353), and the scattered reset calls at
  Back/Cancel/Save/Revert/Delete handlers (~1710-1712, 1751-1753, 2288-2290,
  2303-2305, 2379-2381, 3039-3041) that become dead once the fields are gone.
- Remove `mod mesh_ui;` (~line 30) and delete
  [creatures_editor/mesh_ui.rs](../../sdk/campaign_builder/src/creatures_editor/mesh_ui.rs)
  wholesale (single function, 100% editing UI, no shared helpers).

#### 2.3 Integrate — Simplify Preview Panel

`preview_panel.rs` reads `selected_mesh_index` and `mesh_visibility` for
highlight/visibility-toggle behavior. Since those fields are removed in 2.2,
**drop them entirely**: simplify `preview_panel.rs` so it always renders all
meshes with no selection/highlight state, removing any parameter/field
plumbing that existed solely to pass selection/visibility through from the
editor.

#### 2.4 Testing Requirements

- `cargo build -p campaign_builder` compiles clean.
- Manually open a creature in the Creatures editor; confirm the read-only 3D
  preview still renders with no leftover mesh-list/properties panels.

#### 2.5 Deliverables

- [ ] `creatures_editor/mesh_ui.rs` deleted; `mod mesh_ui;` removed
- [ ] All editing-only functions and fields removed from
      `creatures_editor/mod.rs`
- [ ] `preview_panel.rs` simplified: `selected_mesh_index`/`mesh_visibility`
      plumbing removed, always renders all meshes unselected
- [ ] All quality gates pass

#### 2.6 Success Criteria

Opening any creature for editing shows only the read-only preview and
creature-level property panel — no mesh list, no mesh properties panel, no
primitive-replacement dialog.

---

### Phase 3: Remove Orphaned Raw Mesh-Editing Modules

#### 3.1 Foundation Work — Confirm Zero Callers

Re-confirm (already verified by exploration) that `mesh_vertex_editor.rs`,
`mesh_normal_editor.rs`, `mesh_index_editor.rs` have no callers anywhere in
the repo outside their own module and `tests/mesh_editing_tests.rs`.

#### 3.2 Delete Modules and Update References

- Delete `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`,
  `mesh_index_editor.rs`.
- Remove their `mod` declarations in `lib.rs` (~lines 70-74).
- Update the stale doc comment in
  [linear_history.rs](../../sdk/campaign_builder/src/linear_history.rs) (lines
  10-11) referencing `crate::mesh_vertex_editor::VertexOperation` — no
  compiled dependency, doc-only fix.

#### 3.3 Handle `tests/mesh_editing_tests.rs`

This file imports both removed modules (`mesh_index_editor`,
`mesh_normal_editor`, `mesh_vertex_editor`) and kept modules (`mesh_obj_io`,
`mesh_validation`). Their coverage is not duplicated elsewhere: **port** the
`mesh_obj_io`/`mesh_validation` test cases into an appropriate existing test
file (e.g. an `obj_importer`-focused test file, creating one if none exists)
before deleting `tests/mesh_editing_tests.rs`.

#### 3.4 Testing Requirements

- `cargo build -p campaign_builder` and `cargo test -p campaign_builder`
  pass with no loss of `mesh_obj_io`/`mesh_validation` coverage.

#### 3.5 Deliverables

- [ ] `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs`
      deleted; `lib.rs` mod decls removed
- [ ] `linear_history.rs` doc comment updated
- [ ] `mesh_obj_io`/`mesh_validation` test cases ported to an existing (or
      new) `obj_importer`-focused test file
- [ ] `tests/mesh_editing_tests.rs` deleted
- [ ] All quality gates pass

#### 3.6 Success Criteria

No orphaned mesh-editing modules remain in the crate; full test coverage for
kept modules (`mesh_obj_io`, `mesh_validation`) is retained.

---

### Phase 4: Remove Dead Breadcrumb Helper

#### 4.1 Foundation Work — Confirm Zero Production Call Sites

Re-confirm `enter_mesh_editor` in
[creatures_workflow.rs](../../sdk/campaign_builder/src/creatures_workflow.rs) is
only referenced from its own tests and
`tests/creature_workflow_tests.rs:203`.

#### 4.2 Remove Helper and Its Tests

- Remove `enter_mesh_editor` (~line 363) and its doc example (~line 358) from
  `creatures_workflow.rs`, plus its `#[cfg(test)]` tests (~lines 744, 746,
  820).
- Remove the corresponding call in `tests/creature_workflow_tests.rs:203`.

#### 4.3 Testing Requirements

- `cargo test -p campaign_builder` passes with the reduced test set.

#### 4.4 Deliverables

- [ ] `enter_mesh_editor` and its tests removed from `creatures_workflow.rs`
- [ ] `tests/creature_workflow_tests.rs:203` call site removed
- [ ] All quality gates pass

#### 4.5 Success Criteria

No references to `enter_mesh_editor` remain anywhere in the repo.

---

### Phase 5: Documentation and Final Verification

#### 5.1 Update Architecture Docs

Update [sdk/campaign_builder/README.md](../../sdk/campaign_builder/README.md) line
352 (architecture tree entry for `item_mesh_editor.rs`) and any other
references to removed modules/tabs found by a final repo-wide grep for
`item_mesh_editor`, `ItemMeshEditor`, `mesh_ui`, `mesh_vertex_editor`,
`mesh_normal_editor`, `mesh_index_editor`, `enter_mesh_editor`.

#### 5.2 Full Workspace Verification

- `cargo build` (full workspace) and `cargo test -p campaign_builder` (note:
  per project convention, `cargo test --doc --workspace` is intentionally
  excluded from quality gates).
- Manually launch the SDK: confirm the Item Meshes tab is gone, creature
  editing shows only the read-only preview, and the Importer tab still
  successfully imports both a `.glb` and a `.obj` file end-to-end into the
  appropriate registry (item/creature/furniture/landscape/object).
- Open `campaigns/tutorial` to confirm campaign load succeeds with the
  removed load-hook.

#### 5.3 Deliverables

- [ ] `sdk/campaign_builder/README.md` architecture tree updated
- [ ] Final repo-wide grep for removed symbols returns no matches
- [ ] Full workspace build and test suite pass
- [ ] Manual SDK smoke test (tab list, creature preview, glTF/OBJ import,
      campaign load) confirmed

#### 5.4 Success Criteria

The SDK builds and runs with no interactive mesh-editing UI anywhere, the
glTF/OBJ import pipeline is fully functional and unchanged, and no dead code
or stale documentation referencing the removed editors remains.

---

## Copyright

SPDX-License-Identifier: Apache-2.0

This document follows the [SPDX Spec](https://spdx.github.io/spdx-spec/) for
copyright and licensing information.
