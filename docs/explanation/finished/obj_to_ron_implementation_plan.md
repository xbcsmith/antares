# OBJ to RON Conversion Implementation Plan

## Overview

Add an OBJ-to-RON conversion pipeline to the Campaign Builder SDK. Users will
be able to load a Wavefront OBJ file, see each mesh/object-group listed in the
UI, assign colors via a color picker and preset palette, and export the result
as a `CreatureDefinition` RON file (used for both creatures and items). The
default export paths are `assets/creatures/` and `assets/items/` respectively.

## Current State Analysis

### Existing Infrastructure

- **OBJ I/O** — `sdk/campaign_builder/src/mesh_obj_io.rs` provides
  `import_mesh_from_obj` and `export_mesh_to_obj`.  The importer already
  handles vertices, normals, UVs, face triangulation (quads, n-gons), and
  `flip_yz` / `flip_uv_v` options.  However, it **flattens** all
  object/group blocks (`o`/`g`) into a single `MeshDefinition`, so it
  cannot produce the per-object mesh list that the RON creature format
  requires.
- **Domain types** — `antares::domain::visual::MeshDefinition` and
  `CreatureDefinition` (in `src/domain/visual/mod.rs`) are
  `Serialize + Deserialize` with RON and already match the target file
  format (see `examples/apprenticezara.ron`).
- **Creature persistence** — `CreatureAssetManager` in
  `sdk/campaign_builder/src/creature_assets.rs` provides
  `save_creature`, `write_creature_asset`, and registry management.
- **Item mesh editor** — `sdk/campaign_builder/src/item_mesh_editor.rs` and
  `item_mesh_workflow.rs` already have registry, edit, undo/redo, and file
  I/O patterns that can be reused.
- **Creature editor** — `sdk/campaign_builder/src/creatures_editor.rs`
  manages the creature editing flow including preview rendering.
- **Tabs** — `EditorTab` enum in `lib.rs` lists all sidebar tabs.
  There is no `Importer` tab yet.
- **Color helpers** — `ui_helpers.rs` contains various egui layout helpers
  (`TwoColumnLayout`, etc.) and some color editing utilities already used in
  other editors.
- **Python reference** — `examples/obj_to_ron_universal.py` demonstrates the
  full conversion workflow: multi-object OBJ parsing, per-mesh vertex
  de-duplication and index remapping, name-based color assignment, and RON
  output.

### Identified Issues

1. The existing `import_mesh_from_obj` flattens all `o`/`g` sections into one
   mesh.  A new **multi-mesh** import function is needed that returns
   `Vec<MeshDefinition>`, one per object group.
2. There is no UI for browsing OBJ meshes, assigning colors, or exporting to
   the creature/item RON formats.
3. There is no predefined color palette on the Rust side (the palette lives
   only in the Python script).
4. Export path configuration (creatures vs. items) requires new state in the
   importer UI.

## Implementation Phases

### Phase 1: Multi-Mesh OBJ Import

Extend the OBJ parser to emit one `MeshDefinition` per `o`/`g` group, with
per-mesh vertex de-duplication and index remapping.

#### 1.1 Add `import_meshes_from_obj` to `mesh_obj_io.rs`

Add a new public function alongside the existing single-mesh importer:

```
import_meshes_from_obj(obj_string: &str) -> Result<Vec<MeshDefinition>, ObjError>
import_meshes_from_obj_with_options(obj_string: &str, options: &ObjImportOptions) -> Result<Vec<MeshDefinition>, ObjError>
import_meshes_from_obj_file(path: &str) -> Result<Vec<MeshDefinition>, ObjError>
import_meshes_from_obj_file_with_options(path: &str, options: &ObjImportOptions) -> Result<Vec<MeshDefinition>, ObjError>
```

Internally, the parser should:

- Maintain a running list of global vertices, normals, and UVs (as currently).
- On encountering `o` or `g`, flush the current object's faces into a new
  `MeshDefinition`, re-mapping global vertex indices to local (0-based)
  indices and copying only the vertices referenced by that object's faces
  (following the Python script's `convert_to_meshes` algorithm).
- Set `MeshDefinition.name = Some(clean_name)` where `clean_name` is the
  sanitized object/group name (replace non-alphanumerics with `_`, collapse
  runs, trim).
- After the last line, flush the final object.
- If the OBJ has **no** `o`/`g` lines, fall back to a single mesh named
  `"mesh_0"`.

The existing single-mesh `import_mesh_from_obj` should remain unchanged for
backward compatibility.

#### 1.2 ObjImportOptions Extension

Add an optional field to `ObjImportOptions`:

- `scale: f32` (default `1.0`) — uniform scale applied to every vertex on
  import (the Python script defaults to `0.01` for Blender-scale models).

#### 1.3 Mesh Name Sanitization Helper

Add a private helper `sanitize_mesh_name(raw: &str) -> String` in
`mesh_obj_io.rs` that replaces non-`[a-zA-Z0-9_]` chars with `_`, collapses
consecutive underscores, and trims leading/trailing underscores.

#### 1.4 Testing Requirements

- Unit test: Parse `examples/skeleton.obj` → verify the returned
  `Vec<MeshDefinition>` has the expected number of meshes, each with a
  non-empty `name`, non-empty `vertices`, and indices divisible by 3.
- Unit test: Parse `examples/female_1.obj` → same structural checks.
- Unit test: Parse an OBJ with **no** `o`/`g` lines → single mesh named
  `"mesh_0"`.
- Unit test: Verify `sanitize_mesh_name` edge cases (empty string, special
  characters, consecutive underscores).
- Unit test: Verify `scale` option multiplies all vertex coordinates.
- Existing round-trip tests must continue to pass.

#### 1.5 Deliverables

- [ ] `import_meshes_from_obj` and variants in `mesh_obj_io.rs`
- [ ] `sanitize_mesh_name` helper in `mesh_obj_io.rs`
- [ ] `scale` field on `ObjImportOptions`
- [ ] Unit tests for all new functionality

#### 1.6 Success Criteria

`import_meshes_from_obj_file("examples/skeleton.obj")` returns a
`Vec<MeshDefinition>` whose length matches the number of `o` lines in the
file, with each mesh having a cleaned name, correct vertex count, and
index count divisible by 3.

---

### Phase 2: Color Palette and Mesh Color Mapping

Define a built-in color palette (ported from the Python script) and
name-based auto-assignment logic, plus data structures for per-mesh color
state in the importer.

#### 2.1 Color Palette Module

Create `sdk/campaign_builder/src/color_palette.rs`:

- Define `PALETTE: &[(&str, [f32; 4])]` containing the categories from
  the Python script (skin, hair, eyes, clothing, materials, etc.).
- Provide `fn suggest_color_for_mesh(mesh_name: &str) -> [f32; 4]` that
  checks the lowercase mesh name for keywords (same priority order as
  `get_color_for_mesh` in `obj_to_ron_universal.py`) and returns the
  matching color or the default grey `[0.8, 0.8, 0.8, 1.0]`.
- Expose the palette as a `Vec<PaletteEntry>` for the UI to iterate
  over:
  ```rust
  pub struct PaletteEntry {
      pub label: &'static str,
      pub color: [f32; 4],
  }
  ```
- Register the module in `lib.rs`.

#### 2.2 Custom Palette Persistence

Add support for user-defined colors:

- Define `CustomPalette` struct:
  `pub struct CustomPalette { pub colors: Vec<(String, [f32; 4])> }`.
- Load from `<campaign_dir>/config/importer_palette.ron` on startup.
- UI should allow adding/removing colors from this custom list.

#### 2.3 Importer State Struct

Create `sdk/campaign_builder/src/obj_importer.rs` with:

```rust
pub struct ObjImporterState {
    pub mode: ImporterMode,               // Idle | Loaded | Exporting
    pub source_path: Option<PathBuf>,     // OBJ file path
    pub meshes: Vec<ImportedMesh>,        // Parsed meshes
    pub export_type: ExportType,          // Creature | Item
    pub creature_id: u32,                 // Auto-filled from next available ID
    pub creature_name: String,            // User-provided name
    pub scale: f32,                       // Default 0.01
    pub status_message: String,           // Feedback to user
}

pub struct ImportedMesh {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub color: [f32; 4],
    pub selected: bool,                   // For bulk operations
    pub mesh_def: MeshDefinition,         // The actual mesh data
}

pub enum ImporterMode { Idle, Loaded, Exporting }
pub enum ExportType { Creature, Item }
```

- On load, auto-assign colors via `suggest_color_for_mesh`.
- Register the module in `lib.rs`.

#### 2.3 Add Importer State to `CampaignBuilderApp`

Add `obj_importer_state: obj_importer::ObjImporterState` to
`CampaignBuilderApp` and initialize it in `Default::default()`.

#### 2.4 Testing Requirements

- Unit test: `suggest_color_for_mesh("EM3D_Base_Body")` returns the skin
  color `(0.92, 0.85, 0.78, 1.0)`.
- Unit test: `suggest_color_for_mesh("Hair_Pink")` returns the pink hair
  color.
- Unit test: `suggest_color_for_mesh("unknown_xyz")` returns the default
  grey.
- Unit test: Verify `PaletteEntry` list covers all entries from the Python
  palette.

#### 2.5 Deliverables

- [ ] `color_palette.rs` module
- [ ] `obj_importer.rs` state module
- [ ] Integration into `CampaignBuilderApp`
- [ ] Unit tests for color suggestion and palette coverage

#### 2.6 Success Criteria

After loading `examples/female_1.obj`, every `ImportedMesh` has a
non-default color assigned by the keyword matcher for body-part meshes, and
the user can override any color through the `ImportedMesh.color` field.

---

### Phase 3: Importer Tab UI and RON Export

Build the Importer tab in the egui sidebar, integrate the color picker and
palette, and implement RON export for both creatures and items.

#### 3.1 Add `Importer` to `EditorTab`

Add `Importer` variant to the `EditorTab` enum in `lib.rs`. Position it
**directly below `Creatures`** in the enum and sidebar menu.

#### 3.2 Importer Tab UI Module

Create `sdk/campaign_builder/src/obj_importer_ui.rs`:

- **Idle mode panel:**
  - File picker button (via `rfd::FileDialog`) filtered to `*.obj`.
  - Export type selector (`Creature` / `Item`).
  - Scale input with default `0.01`.
  - "Load OBJ" button.

- **Loaded mode panel:**
  - Creature/item metadata inputs (ID, name).
  - Scrollable mesh list showing each `ImportedMesh`:
    - Mesh name (read-only label).
    - Vertex / triangle counts.
    - Color swatch + small color edit button.
    - Checkbox for selection.
  - **Color picker panel** (shown when a mesh color edit is active):
    - `egui::color_picker::color_edit_button_rgba` for free-form RGBA
      editing.
    - Palette grid: a row of clickable color swatches from `PALETTE`.
    - "Auto-assign all" button that re-runs `suggest_color_for_mesh` on
      every mesh.
  - Summary bar showing total meshes, vertices, triangles.
  - "Export RON" button.
  - "Back / Clear" button to return to Idle.

- **Exporting mode:**
  - Build a `CreatureDefinition` from `ObjImporterState`:
    - `id` from `creature_id`.
    - `name` from `creature_name`.
    - `meshes` from `Vec<ImportedMesh>` (cloning each `mesh_def`, applying
      the user's `color`).
    - `mesh_transforms` filled with `MeshTransform::identity()` for each
      mesh.
    - `scale` from `ObjImporterState.scale`.
  - Use `CreatureAssetManager::write_creature_asset` to save creatures.
  - For items, serialize as a `CreatureDefinition` (same as creatures) but
    write to `assets/items/<name>.ron`.
  - After writing, update the `status_message` and return to Idle mode.

#### 3.3 Wire UI into `lib.rs`

In the `update()` method's tab dispatch (`match self.active_tab { ... }`),
add a branch for `EditorTab::Importer` that calls the importer UI render
function, passing `&mut self.obj_importer_state`, `&self.campaign_dir`, and
`&self.logger`.

Handle the `ItemMeshEditorSignal`-style pattern if cross-tab navigation is
needed (e.g., after export, optionally switch to the Creatures tab).

#### 3.4 Export Path Logic

- **Creature export:** `<campaign_dir>/assets/creatures/<name>.ron`
  Ensure `assets/creatures/` directory exists (create via `fs::create_dir_all`).
  After writing the RON file, register the creature in the creature
  registry via `CreatureAssetManager::save_creature`.
- **Item export:** `<campaign_dir>/assets/items/<name>.ron`
  Ensure `assets/items/` directory exists.  Write the RON file
  directly (items follow a simpler persistence model than creatures).

#### 3.5 Testing Requirements

- Integration test: Load `examples/skeleton.obj` → verify importer state
  transitions `Idle → Loaded`.
- Integration test: Modify a mesh color in `ObjImporterState` → verify the
  exported `CreatureDefinition` reflects the new color.
- Integration test: Export to a temp directory → verify the RON file is
  valid RON and deserializes back to a `CreatureDefinition` with matching
  mesh count, names, and colors.
- Integration test: Export as item → verify file lands in the items
  directory.

#### 3.6 Deliverables

- [ ] `Importer` variant added to `EditorTab`
- [ ] `obj_importer_ui.rs` module with full UI
- [ ] Export logic for creatures (via `CreatureAssetManager`)
- [ ] Export logic for items (direct RON write)
- [ ] Directory creation for `assets/creatures/` and `assets/items/`
- [ ] Integration tests for load, color edit, and export workflows

#### 3.7 Success Criteria

A user can open the Importer tab (below Creatures), pick an OBJ file, see the
mesh list with auto-assigned colors, adjust colors via the palette or
picker, enter a name and ID, click "Export RON", and find a valid
`CreatureDefinition` RON file in the expected campaign subdirectory.
