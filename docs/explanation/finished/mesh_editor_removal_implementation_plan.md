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
code removed here.

---

## Current State Analysis

### Existing Infrastructure

| Area | Files | Notes |
|---|---|---|
| Item Meshes tab | `sdk/campaign_builder/src/item_mesh_editor.rs`, `item_mesh_workflow.rs`, `item_mesh_undo_redo.rs` | Self-contained trio; `EditorTab::ItemMeshes` in `lib.rs`; edits `ItemMeshDescriptor`, writes `CreatureDefinition` RON under `assets/items/` |
| Creature mesh editor | `sdk/campaign_builder/src/creatures_editor/mod.rs`, `creatures_editor/mesh_ui.rs` | Baked into the main creature edit screen; four-panel layout in `show_edit_mode` (left: mesh list, right: mesh properties, central: 3D preview, bottom: creature properties) |
| Read-only preview (kept) | `sdk/campaign_builder/src/creatures_editor/preview_panel.rs` | `show_preview_panel`, `sync_preview_renderer_from_edit_buffer`, `current_mesh_visibility`, `build_preview_statistics` — must be simplified but NOT deleted |
| Orphaned raw editors | `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs` (~1 000+ lines each), declared in `lib.rs:70-74` | Zero UI callers anywhere in the repo; only consumers are each other and `tests/mesh_editing_tests.rs` |
| Import pipeline (kept) | `mesh_glb_io.rs`, `mesh_obj_io.rs`, `obj_importer.rs`, `obj_importer_ui.rs`, `EditorTab::Importer` | Architecturally independent of both editors above |
| Runtime consumer (kept) | `src/domain/campaign_loader.rs`, `src/domain/items/database.rs`, `src/game/systems/item_world_events.rs` | Reads `data/item_mesh_registry.ron` → `CreatureDefinition`; registry populated only by the Importer, never by `item_mesh_editor.rs` |

### Identified Issues

1. `item_mesh_editor.rs` + `item_mesh_workflow.rs` + `item_mesh_undo_redo.rs` implement a full interactive editor for an asset type the user now authors exclusively in Blender.
2. `creatures_editor/mod.rs` unconditionally renders an interactive mesh-list/properties panel on every creature edit screen.
3. `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs` are dead code — declared as modules but never reachable from any tab.
4. `creatures_workflow.rs::enter_mesh_editor` is a dead breadcrumb helper with zero production call sites.
5. `tests/mesh_editing_tests.rs` mixes coverage for modules being deleted with coverage for modules being kept (`mesh_obj_io`, `mesh_validation`), so it cannot be deleted wholesale without first porting the kept-module tests.
6. **`lib.rs` has two separate `requested_open_item_mesh.take()` call sites** — one inside the `ItemMeshes` arm (L1255) and one inside the `Items` arm (L1284-1288). Both must be removed.
7. **`tests/creature_asset_editor_tests.rs`** imports `PrimitiveType` from `creatures_editor` and has three tests that directly set fields being deleted (`primitive_type`, `primitive_size`, `primitive_use_current_color`, `primitive_custom_color`, `primitive_segments`, `primitive_rings`, `selected_mesh_index`). These tests must be removed in Phase 2.
8. **`PrimitiveType` enum** (L239-245 in `creatures_editor/mod.rs`) is used only by the primitive replacement dialog and its associated tests. It must be deleted when those features are removed.
9. **`preview_panel.rs`** reads `selected_mesh_index` and `mesh_visibility` in `sync_preview_renderer_from_edit_buffer`, `current_mesh_visibility`, and `build_preview_statistics`. These call sites must be excised and those helpers simplified.
10. **Dead constants** `PRIMITIVE_SEGMENTS_MAX` (L108), `PRIMITIVE_RINGS_MAX` (L111), `PRIMITIVE_SOFT_TRIANGLE_BUDGET` (L117) in `creatures_editor/mod.rs` are used only by the primitive dialog. When that code is removed, Clippy `-D warnings` will fail on unused constants.

---

## Implementation Phases

### Phase 1: Remove the Standalone "Item Meshes" Tab

#### 1.1 Foundation Work — Delete the Editor Trio

Delete the following three files in full — no other code depends on their
internals outside the integration points listed below:

- `sdk/campaign_builder/src/item_mesh_editor.rs`
- `sdk/campaign_builder/src/item_mesh_workflow.rs`
- `sdk/campaign_builder/src/item_mesh_undo_redo.rs`

#### 1.2 Remove Tab Plumbing in `lib.rs`

File: `sdk/campaign_builder/src/lib.rs`

Perform all of the following removals in this file:

**Module declarations** — remove these three lines (L57-L59):
```
pub mod item_mesh_editor;
pub mod item_mesh_undo_redo;
pub mod item_mesh_workflow;
```

**`EditorTab` enum** — remove the `ItemMeshes` variant (L649) and its name
arm (L679):
```
ItemMeshes,                           // delete this variant
EditorTab::ItemMeshes => "Item Meshes",  // delete this match arm
```

**`CampaignBuilderApp` struct** — remove the `item_mesh_editor_state` field
(L755):
```
item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState,
```

**`Default` impl** — remove the initializer (L805):
```
item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState::new(),
```

**`EditorTab::ItemMeshes` central-panel match arm** (L1236-L1260) — delete
the entire arm, including the `ItemMeshEditorSignal::OpenInItemsEditor`
signal handler and the cross-tab `requested_open_item_mesh.take()` guard at
L1255-L1259.

**`EditorTab::Items` cross-tab navigation guard** (L1282-L1288) — inside the
`EditorTab::Items` match arm, delete the block that drains
`requested_open_item_mesh` and switches to `EditorTab::ItemMeshes`:
```rust
// Handle cross-tab navigation: items editor wants to open the
// Item Mesh Editor for a specific item.
if let Some(item_id) = self.editor_registry.items_editor_state.requested_open_item_mesh.take() {
    self.ui_state.active_tab = EditorTab::ItemMeshes;
    self.ui_state.status_message = format!("Opening Item Mesh Editor for item #{}", item_id);
    ui.ctx().request_repaint();
}
```

**Importer export-signal handler** — remove the `ObjImporterUiSignal::Item`
arm's call to `item_mesh_editor_state.load_from_campaign` (~L1486):
```rust
obj_importer_ui::ObjImporterUiSignal::Item => {
    let importer_status = self.obj_importer_state.status_message.clone();
    if let Some(ref dir) = self.campaign_dir.clone() {
        self.item_mesh_editor_state.load_from_campaign(dir);  // remove this block
    }
    ...
}
```

#### 1.3 Remove Cross-Tab Link in `campaign_io.rs`

File: `sdk/campaign_builder/src/campaign_io.rs`

In `do_open_campaign` (~L3353-L3367), remove the block that loads item mesh
assets into the Item Mesh Editor registry:
```rust
// Load item mesh assets into the Item Mesh Editor registry.
if let Some(ref dir) = self.campaign_dir.clone() {
    self.logger.debug(category::FILE_IO, "Loading item mesh assets...");
    self.item_mesh_editor_state.load_from_campaign(dir);
    self.logger.info(category::FILE_IO,
        &format!("Loaded {} item mesh entries",
                 self.item_mesh_editor_state.registry.len()));
}
```

#### 1.4 Remove Field, Button, and Tests in `items_editor.rs`

File: `sdk/campaign_builder/src/items_editor.rs`

Remove the following:

- **Struct field** `requested_open_item_mesh: Option<ItemId>` (L96), its
  doc comment (L92-L95), and the `Default` initializer (L112).
- **Button and handler** `"✏️ Open in Item Mesh Editor"` (L1073-L1076)
  inside `show_form`.
- **Test** `test_items_editor_requested_open_item_mesh_set_on_button`
  (L1859-L1876) inside `mod tests`.

#### 1.5 Testing Requirements

Run after completing steps 1.1-1.4:

```bash
cargo fmt --all
cargo check -p campaign_builder --all-targets --all-features
cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings
cargo nextest run -p campaign_builder --all-features
```

Verify: no dangling references to `item_mesh_editor`, `ItemMeshEditorState`,
`ItemMeshEditorSignal`, `EditorTab::ItemMeshes`, or
`requested_open_item_mesh`.

#### 1.6 Deliverables

- [x] `item_mesh_editor.rs`, `item_mesh_workflow.rs`, `item_mesh_undo_redo.rs` deleted
- [x] `lib.rs` — mod decls, `EditorTab::ItemMeshes`, `item_mesh_editor_state`
      field and Default init, `ItemMeshes` match arm, `Items`-arm cross-tab
      guard, and Importer signal load-hook call all removed
- [x] `campaign_io.rs` — `do_open_campaign` load-hook block removed
- [x] `items_editor.rs` — `requested_open_item_mesh` field, button, handler,
      and test removed
- [x] All quality gates pass

#### 1.7 Success Criteria

The "Item Meshes" tab no longer appears in the sidebar. The SDK builds,
starts, and loads an existing campaign without error or panic.

---

### Phase 2: Remove the Embedded Creature Mesh Editor

#### 2.1 Foundation Work — Map Edit vs. Display Code

Confirm the clean split in `creatures_editor/` before touching anything:

| Keep | Remove |
|---|---|
| `preview_panel.rs` — `show_preview_panel`, `show_preview_fallback`, `sync_preview_renderer_from_edit_buffer` (simplified), `current_mesh_visibility` (simplified), `build_preview_statistics` (simplified) | `mesh_ui.rs` (entire file) |
| `mod.rs` — `show_creature_level_properties`, `show_registry_mode`, `show_registry_preview_panel`, `export_current_creature_to_ron`, save/revert/delete logic | `mod.rs` — `show_mesh_list_panel`, `validate_selected_mesh`, `estimate_primitive_geometry`, `show_primitive_replacement_dialog`, `apply_primitive_replacement`, `_legacy_show_mesh_list_and_editor` |
| `mod.rs` — struct fields `show_grid`, `show_wireframe`, `show_normals`, `show_axes`, `background_color`, `camera_distance` (all used by `preview_panel.rs`) | `mod.rs` — struct fields `show_mesh_list`, `show_mesh_editor`, `selected_mesh_index`, `mesh_edit_buffer`, `mesh_transform_buffer`, `mesh_visibility`, `show_primitive_dialog`, `primitive_type`, `primitive_size`, `primitive_segments`, `primitive_rings`, `primitive_use_current_color`, `primitive_custom_color`, `primitive_preserve_transform`, `primitive_keep_name`, `uniform_scale` |

#### 2.2 Delete `mesh_ui.rs` and Remove Editing State

**Delete the file** `sdk/campaign_builder/src/creatures_editor/mesh_ui.rs` in
full — it contains a single function, `show_mesh_properties_panel`, that is
100% editing UI with no shared helpers.

In `sdk/campaign_builder/src/creatures_editor/mod.rs`:

**Remove the module declaration** at L30:
```rust
mod mesh_ui;
```

**Remove the four-panel layout block** in `show_edit_mode` (L1775-L1803):
```rust
// Three-panel layout: Mesh List | 3D Preview | Mesh Properties
egui::Panel::left("mesh_list_panel") ...     // L1776-1783 — remove
egui::Panel::right("mesh_properties_panel") ... // L1785-1794 — remove
egui::CentralPanel::default().show(ui, |ui| {   // L1796-1798 — keep,
    self.show_preview_panel(ui);                 // renders the 3D preview
});
// Show primitive replacement dialog if active
if self.show_primitive_dialog { ... }           // L1800-1803 — remove
```

After this edit, `show_edit_mode` retains the `egui::CentralPanel::default()`
that wraps `show_preview_panel` and the `egui::Panel::bottom` that wraps
`show_creature_level_properties` — both untouched.

**Remove the scattered reset calls** that will no longer compile once the
fields are deleted. These appear in button handlers inside `show_edit_mode`
at the following locations — delete only the lines that reference the
removed fields (`selected_mesh_index`, `mesh_edit_buffer`,
`mesh_transform_buffer`):

- Back to List button (~L1710-1712)
- Cancel button (~L1751-1753)
- Any Save/Revert/Delete handlers that reset these same three fields
  (~L2288-2290, ~L2303-2305, ~L2379-2381, ~L3039-3041)

**Remove the following functions** from `mod.rs`:

| Function | Approx. lines |
|---|---|
| `fn show_mesh_list_panel` | L1847-L1950 |
| `fn validate_selected_mesh` | L2219-L2272 |
| `fn estimate_primitive_geometry` | L2479-L2490 |
| `fn show_primitive_replacement_dialog` | L2493-L2640 |
| `fn apply_primitive_replacement` | L2643-L2717 |
| `fn _legacy_show_mesh_list_and_editor` | L2719-L2947 |

**Remove the editing-only struct fields** from `CreaturesEditorState` (decls
~L138-L216) and their initializers in `Default` (~L316-L353):

```
show_mesh_list, show_mesh_editor, selected_mesh_index,
mesh_edit_buffer, mesh_transform_buffer, mesh_visibility,
show_primitive_dialog, primitive_type, primitive_size,
primitive_segments, primitive_rings, primitive_use_current_color,
primitive_custom_color, primitive_preserve_transform,
primitive_keep_name, uniform_scale
```

**Remove the `PrimitiveType` enum** (L239-L245) — it is used only by the
deleted primitive dialog code.

**Remove the three dead constants** that are used only by the deleted dialog:
```rust
const PRIMITIVE_SEGMENTS_MAX: u32 = 64;       // L108
const PRIMITIVE_RINGS_MAX: u32 = 64;           // L111
const PRIMITIVE_SOFT_TRIANGLE_BUDGET: usize = 8_192;  // L117
```

#### 2.3 Simplify `preview_panel.rs`

File: `sdk/campaign_builder/src/creatures_editor/preview_panel.rs`

The preview panel uses two fields that are deleted in 2.2. Apply these
targeted edits — all other preview panel logic is unchanged.

**In `sync_preview_renderer_from_edit_buffer`** (~L195-L229):
- Remove `let selected_mesh_index = self.selected_mesh_index;`
- Remove `renderer.set_selected_mesh_index(selected_mesh_index);`
- Remove `let visible = self.current_mesh_visibility();` — replace with a
  locally-computed full-visibility vector: `let visible: Vec<bool> = vec![true; self.edit_buffer.meshes.len()];`
- Remove `renderer.set_mesh_visibility(visible);`
- Remove `let stats = self.build_preview_statistics(&visible);` and the
  subsequent `self.preview_state.update_statistics(stats);` call

**In `current_mesh_visibility`** (~L232-L239):
- Remove the function body that reads `self.mesh_visibility`; replace with:
  ```rust
  pub(super) fn current_mesh_visibility(&self) -> Vec<bool> {
      vec![true; self.edit_buffer.meshes.len()]
  }
  ```
  (The function signature is kept because `build_preview_statistics` still
  calls it for vertex/triangle counting.)

**In `build_preview_statistics`** (~L242-L255):
- Remove the line `stats.selected_meshes = usize::from(self.selected_mesh_index.is_some());`
- Replace with `stats.selected_meshes = 0;`

#### 2.4 Remove Broken Tests

**In `sdk/campaign_builder/src/creatures_editor/mod.rs`**, inside `mod tests`,
delete the following tests and test-helper functions — they directly reference
fields or functions removed in 2.2:

| Symbol | Approx. lines | Reason |
|---|---|---|
| `fn preview_selected_mesh_for_tests` (impl helper) | L3165-L3169 | uses `selected_mesh_index` |
| `fn preview_visibility_for_tests` (impl helper) | L3172-L3177 | uses `mesh_visibility` |
| `fn test_mesh_selection_state` | L3712-L3721 | uses `selected_mesh_index` |
| `fn test_validate_selected_mesh_reports_invalid_mesh_errors` | L3893-L3915 | calls deleted `validate_selected_mesh` |
| `fn test_preview_sync_reflects_color_changes_and_visibility` | L3832-L3859 | uses `mesh_visibility` |

**In `sdk/campaign_builder/tests/creature_asset_editor_tests.rs`**:

- Remove the `PrimitiveType` import (L20):
  ```rust
  use campaign_builder::creatures_editor::{CreaturesEditorState, PrimitiveType};
  // change to:
  use campaign_builder::creatures_editor::CreaturesEditorState;
  ```
- Remove the following three tests that set deleted fields or reference the
  deleted `PrimitiveType` enum:
  - `fn test_replace_mesh_with_primitive_cube` (L241-L266)
  - `fn test_replace_mesh_with_primitive_sphere` (L268-L301)
  - `fn test_primitive_type_enum` (L411-L425)

The remaining tests in `creature_asset_editor_tests.rs` (load, add, remove,
duplicate, reorder, update transform/color, scale, save) do not reference any
deleted field and require no further changes.

#### 2.5 Testing Requirements

```bash
cargo fmt --all
cargo check -p campaign_builder --all-targets --all-features
cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings
cargo nextest run -p campaign_builder --all-features
```

Verify: no references to `show_mesh_list_panel`, `show_mesh_properties_panel`,
`mesh_visibility`, `selected_mesh_index`, `PrimitiveType`,
`PRIMITIVE_SEGMENTS_MAX`, `show_primitive_replacement_dialog`, or
`_legacy_show_mesh_list_and_editor` remain anywhere in the crate.

#### 2.6 Deliverables

- [x] `creatures_editor/mesh_ui.rs` deleted; `mod mesh_ui;` removed from `mod.rs`
- [x] All editing-only functions, fields, the `PrimitiveType` enum, and the
      three dead constants removed from `creatures_editor/mod.rs`
- [x] Left and right panel wiring and primitive-dialog invocation removed from
      `show_edit_mode`; `CentralPanel` (preview) and `Panel::bottom`
      (creature properties) untouched
- [x] `preview_panel.rs` simplified: `selected_mesh_index` and `mesh_visibility`
      plumbing removed; always renders all meshes visible, no selection highlight
- [x] Broken tests removed from `creatures_editor/mod.rs` (5 items) and
      `creature_asset_editor_tests.rs` (5 tests + 1 import line; 2 extra tests
      found during implementation: `test_mesh_visibility_tracking`,
      `test_uniform_scale_toggle`)
- [x] All quality gates pass

#### 2.7 Success Criteria

Opening any creature for editing shows only the read-only 3D preview and the
creature-level property panel — no mesh list, no mesh properties panel, no
primitive-replacement dialog. The 3D preview retains all existing controls
(grid, wireframe, normals, axes, camera distance, background colour).

---

### Phase 3: Remove Orphaned Raw Mesh-Editing Modules

#### 3.1 Foundation Work — Confirm Zero Callers

Re-confirm that `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`,
`mesh_index_editor.rs` have no callers anywhere in the repo outside their own
modules and `tests/mesh_editing_tests.rs`.

`primitive_generators.rs` is **not** removed here: it is independently used
by `creature_templates.rs` and the intact tests in
`creature_asset_editor_tests.rs`.

#### 3.2 Delete Modules and Update References

- Delete `sdk/campaign_builder/src/mesh_vertex_editor.rs`
- Delete `sdk/campaign_builder/src/mesh_normal_editor.rs`
- Delete `sdk/campaign_builder/src/mesh_index_editor.rs`

In `sdk/campaign_builder/src/lib.rs`, remove the three `pub mod` declarations:
```rust
pub mod mesh_index_editor;   // L70
pub mod mesh_normal_editor;  // L71
pub mod mesh_vertex_editor;  // L74
```

In `sdk/campaign_builder/src/linear_history.rs`, update the doc comment at
lines 9-11 that references the deleted types. Remove or replace the two
now-invalid cross-references:
```rust
// current (invalid):
//! [`crate::mesh_vertex_editor::VertexOperation`] or
//! [`crate::mesh_index_editor::IndexOperation`]

// replacement — use a generic placeholder:
//! operations (each carrying "before" and "after" state).
```

#### 3.3 Port Kept-Module Tests and Delete `mesh_editing_tests.rs`

`sdk/campaign_builder/tests/mesh_editing_tests.rs` contains tests for both
deleted modules (`mesh_vertex_editor`, `mesh_normal_editor`,
`mesh_index_editor`) and kept modules (`mesh_obj_io`, `mesh_validation`).
The tests that cover kept modules must be preserved.

**Step A — Create** `sdk/campaign_builder/tests/obj_importer_tests.rs` (new
file). Give it an SPDX header and move the following test functions from
`mesh_editing_tests.rs` into it, updating any imports as needed:

| Function | Source lines |
|---|---|
| `fn test_validation_valid_mesh` | L98-L103 |
| `fn test_validation_no_vertices` | L106-L115 |
| `fn test_validation_no_indices` | L118-L127 |
| `fn test_validation_invalid_index` | L130-L139 |
| `fn test_validation_degenerate_triangle` | L142-L151 |
| `fn test_validation_mismatched_normals` | L154-L163 |
| `fn test_validation_unnormalized_normal_warning` | L166-L176 |
| `fn test_validation_info_contains_stats` | L179-L185 |
| `fn test_validation_helper_function` | L188-L195 |
| `fn test_obj_export_simple` | L627-L634 |
| `fn test_obj_import_simple` | L637-L643 |
| `fn test_obj_export_import_roundtrip` | L646-L653 |
| `fn test_obj_import_with_normals` | L656-L664 |
| `fn test_obj_import_quad_triangulation` | L667-L673 |
| `fn test_obj_export_with_options` | L676-L688 |
| `fn test_edge_case_obj_import_malformed` | L924-L928 |

Also port the shared helper functions used by the above tests:
- `fn create_simple_triangle` (L24-L37)
- `fn create_quad_mesh` (L39-L57)
- `fn create_cube_mesh` (L59-L91)

**Step B — Delete** `sdk/campaign_builder/tests/mesh_editing_tests.rs` in
full.

#### 3.4 Testing Requirements

```bash
cargo fmt --all
cargo check -p campaign_builder --all-targets --all-features
cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings
cargo nextest run -p campaign_builder --all-features
```

Verify: no references to `mesh_vertex_editor`, `mesh_normal_editor`, or
`mesh_index_editor` remain anywhere in the crate. The new
`obj_importer_tests.rs` must pass in full.

#### 3.5 Deliverables

- [ ] `mesh_vertex_editor.rs`, `mesh_normal_editor.rs`, `mesh_index_editor.rs`
      deleted; corresponding `pub mod` lines removed from `lib.rs`
- [ ] `linear_history.rs` doc comment updated (stale cross-references removed)
- [ ] `tests/obj_importer_tests.rs` created with all `mesh_obj_io` and
      `mesh_validation` test cases ported
- [ ] `tests/mesh_editing_tests.rs` deleted
- [ ] All quality gates pass

#### 3.6 Success Criteria

No orphaned mesh-editing modules remain in the crate. Full test coverage for
kept modules (`mesh_obj_io`, `mesh_validation`) is retained in the new
`obj_importer_tests.rs`.

---

### Phase 4: Remove Dead Breadcrumb Helper

#### 4.1 Foundation Work — Confirm Zero Production Call Sites

Re-confirm `enter_mesh_editor` in
`sdk/campaign_builder/src/creatures_workflow.rs` is referenced only from:
- Its own `#[cfg(test)]` block
- `tests/creature_workflow_tests.rs:203`

No production call sites exist.

#### 4.2 Remove Helper and Its Tests

In `sdk/campaign_builder/src/creatures_workflow.rs`:

- Remove the `enter_mesh_editor` function (L363-L372) and its doc comment /
  example block (L350-L362).
- Remove the two internal unit tests that exercise it:
  - `fn test_enter_mesh_editor_extends_breadcrumbs` (L744-L749)
  - `fn test_breadcrumb_string_mesh_editor` (L818-L825)

In `sdk/campaign_builder/tests/creature_workflow_tests.rs`:

- Remove the single call site at L203:
  ```rust
  workflow.enter_mesh_editor("orc.ron", "Orc", "club");
  ```
  and the two lines immediately following it (L204-L205) that assert on
  `breadcrumb_labels`. The enclosing function `test_registry_to_asset_navigation`
  continues to compile and pass after removing these three lines.

#### 4.3 Testing Requirements

```bash
cargo fmt --all
cargo check -p campaign_builder --all-targets --all-features
cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings
cargo nextest run -p campaign_builder --all-features
```

Verify: no references to `enter_mesh_editor` remain anywhere in the repo.

#### 4.4 Deliverables

- [ ] `enter_mesh_editor` and its doc/example removed from `creatures_workflow.rs`
- [ ] Two internal unit tests removed from `creatures_workflow.rs`
- [ ] Three lines removed from `test_registry_to_asset_navigation` in
      `creature_workflow_tests.rs`
- [ ] All quality gates pass

#### 4.5 Success Criteria

No references to `enter_mesh_editor` remain anywhere in the repo.

---

### Phase 5: Documentation and Final Verification

#### 5.1 Update Architecture Docs

In `sdk/campaign_builder/README.md`, remove the architecture-tree entry for
`item_mesh_editor.rs` (~L352) and any other references to removed
modules/tabs.

Run a repo-wide grep for each of the following symbols and remove any
stale doc-only references found in `*.md` files:

```
item_mesh_editor  ItemMeshEditor  ItemMeshEditorState  ItemMeshEditorSignal
mesh_ui           mesh_vertex_editor  mesh_normal_editor  mesh_index_editor
enter_mesh_editor PrimitiveType   PRIMITIVE_SEGMENTS_MAX
```

#### 5.2 Update `docs/explanation/implementations.md`

Add an entry summarising the mesh editor removal: what was deleted, why,
and which import pipeline functionality was preserved.

#### 5.3 Full Workspace Verification

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Manually launch the SDK and verify:
1. The "Item Meshes" tab is gone from the sidebar
2. Opening any creature shows only the read-only 3D preview and creature
   properties (no mesh list, no mesh properties panel)
3. The 3D preview controls (grid, wireframe, normals, axes, background
   colour, camera distance) still function
4. The Importer tab successfully imports a `.glb` file and a `.obj` file
   end-to-end
5. Opening `campaigns/tutorial` completes without error

#### 5.4 Deliverables

- [ ] `sdk/campaign_builder/README.md` architecture tree updated
- [ ] `docs/explanation/implementations.md` updated
- [ ] Final repo-wide grep for all removed symbols returns zero matches in
      source files (`.rs`); any `.md` matches are verified as doc-only references
      that have been cleaned up
- [ ] Full workspace build and test suite pass
- [ ] Manual SDK smoke test completed (tab list, creature preview, glTF/OBJ
      import, campaign load)

#### 5.5 Success Criteria

The SDK builds and runs with no interactive mesh-editing UI anywhere, the
glTF/OBJ import pipeline is fully functional and unchanged, and no dead code
or stale documentation referencing the removed editors remains.

---

## Copyright

SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
