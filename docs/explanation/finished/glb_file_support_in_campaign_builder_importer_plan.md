# GLB File Support in Campaign Builder Importer Implementation Plan

## Overview

Add binary glTF (`.glb`) import support to the Campaign Builder Importer tab.
The importer accepts a single `.glb` file, converts supported mesh primitives
into Antares `MeshDefinition` values, extracts embedded base-color texture images
into the campaign asset tree, and exports Creature, Item, and Furniture RON files
that reference those copied textures through `MeshDefinition.texture_path`.

This plan builds deliberately on the existing OBJ importer pipeline rather than
creating a parallel export workflow. OBJ and GLB parsing converge into the same
`ObjImporterState` and export code paths so texture copying, RON export, color
editing, and target-type handling remain consistent across both formats.

Initial GLB support targets the practical Campaign Builder use case:

- One `.glb` file selected in the Importer tab.
- One or more mesh primitives converted into importer mesh rows.
- Material base-color factors mapped into `MaterialDefinition`.
- Material base-color textures extracted and copied into
  `assets/textures/imported/<asset_name>/`.
- Multiple model parts with different materials/textures represented as multiple
  `MeshDefinition` entries.
- Creature, Item, and Furniture exports remain RON, not JSON or YAML.

Advanced PBR channels (normal, metallic-roughness, occlusion, emissive maps)
require domain material schema changes and are deferred to a later compatibility
phase after base GLB import works end-to-end.

---

## Current State Analysis

### Existing Infrastructure

| Area                    | Existing File(s)                                                 | Current Capability                                                                                                                                                                                                                                                     | Relevance                                                                                                   |
| ----------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Importer state          | `sdk/campaign_builder/src/obj_importer.rs`                       | Tracks loaded meshes, source path, export target, scale, colors, material swatches, and per-mesh `texture_source_path: Option<PathBuf>`. `clear()` preserves scale, palette, IDs, and export-type across resets.                                                       | Extend with `source_format: ImportSourceFormat` and generalized `ImportedTexturePayload` on `ImportedMesh`. |
| OBJ parser              | `sdk/campaign_builder/src/mesh_obj_io.rs`                        | Parses OBJ/MTL geometry, material color, UVs, `map_Kd`, and source texture paths. Returns `ImportedObjScene`.                                                                                                                                                          | Model the GLB parser module on this structure.                                                              |
| Importer UI             | `sdk/campaign_builder/src/obj_importer_ui.rs`                    | Renders Importer tab heading `"OBJ Importer"`, file picker (`pick_obj_file`), mesh list, color editor, and RON export for Creature/Item/Furniture. `load_obj_into_state` dispatches OBJ loads.                                                                         | Add `.glb` file selection and dispatch; update heading and help text.                                       |
| Texture export          | `sdk/campaign_builder/src/obj_importer_ui.rs`                    | `copy_imported_textures_into_campaign` copies source texture files into `assets/textures/imported/<asset_name>/` by resolving `mesh.texture_source_path`. `resolve_imported_texture_source` looks up filesystem paths only.                                            | Extend both functions to handle embedded byte payloads.                                                     |
| Module registry         | `sdk/campaign_builder/src/lib.rs`                                | Declares all public modules via `pub mod …;` statements (e.g., `pub mod mesh_obj_io;`). Tab switch on Creature export at `L1354`: `self.ui_state.active_tab = EditorTab::Creatures`.                                                                                   | Register `pub mod mesh_glb_io;` here. Make post-export tab switch conditional here.                         |
| Domain mesh schema      | `src/domain/visual/mod.rs`                                       | `MeshDefinition` has `vertices`, `indices`, `normals`, `uvs`, `color`, `material: Option<MaterialDefinition>`, and `texture_path: Option<String>`. `MaterialDefinition` has `base_color`, `metallic`, `roughness`, `emissive`, `alpha_mode`.                           | Base GLB support maps to existing fields. No changes required.                                              |
| Runtime texture loading | `src/game/systems/creature_meshes.rs`, `src/bin/antares.rs`      | `texture_loading_system` loads `MeshDefinition.texture_path` into Bevy `StandardMaterial.base_color_texture`.                                                                                                                                                          | GLB exports must produce identical `texture_path` conventions as OBJ exports.                               |
| Validation caching      | `sdk/campaign_builder/src/creatures_editor/mod.rs` `L1577–L2054` | `show_edit_mode` calls `refresh_validation_state()` unconditionally on **every frame** at `L1578`. `refresh_validation_state` iterates every mesh and calls `mesh_validation::validate_mesh` — O(meshes). No dirty guard.                                              | Phase 6: add dirty/cache guard.                                                                             |
| Preview rendering       | `sdk/campaign_builder/src/creatures_editor/preview_panel.rs`     | `show_preview_panel` checks `self.preview_dirty` before calling `sync_preview_renderer_from_edit_buffer()`. However `renderer.show(ui)` runs **every egui repaint** (including uninteracted frames) and calls `render_preview`, which redraws all wireframe triangles. | Phase 6: add live-preview gating so `renderer.show` skips expensive redraws when not needed.                |
| Campaign data format    | `docs/reference/campaign_content_format.md`                      | Documents RON-based campaign data and importer texture-map behavior.                                                                                                                                                                                                   | Update in Phase 7 after implementation.                                                                     |
| SDK rules               | `sdk/AGENTS.md`                                                  | Mandatory egui ID rules (Rules 1–16) and egui audit checklist at `L1197–L1268`.                                                                                                                                                                                        | All Importer UI changes must pass this checklist before marking a phase complete.                           |
| Test fixtures           | `data/test_fixtures/`                                            | Contains `skeleton.obj` and `female_1.obj` used by SDK unit tests. Tests access via `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/test_fixtures/…")`.                                                                                                    | Binary GLB fixtures go here too.                                                                            |

### Identified Issues

1. **OBJ-only Importer UI**: The tab heading reads `"OBJ Importer"` and the file
   picker only filters `.obj` files. There is no GLB dispatch path.
2. **No GLB parser module**: `mesh_glb_io.rs` does not exist. The `gltf` crate
   is not in `sdk/campaign_builder/Cargo.toml`.
3. **No embedded-image representation**: `ImportedMesh.texture_source_path:
Option<PathBuf>` cannot hold embedded GLB image bytes. A generalized
   `ImportedTexturePayload` type is needed.
4. **Texture copy flow is filesystem-only**: `copy_imported_textures_into_campaign`
   resolves source images via `resolve_imported_texture_source`, which returns
   `Option<PathBuf>` and uses only filesystem resolution. It cannot write embedded
   byte payloads.
5. **GLB structural complexity**: GLB scenes contain nodes, meshes, primitives,
   transforms, materials, and embedded images. The importer needs deterministic
   subset selection and explicit unsupported-feature error behavior.
6. **PBR field mismatch**: GLB `pbrMetallicRoughness` fields exceed the Antares
   `MaterialDefinition` schema. Base-color factor and base-color texture can be
   mapped now; additional channels are deferred.
7. **No texture preview in Campaign Builder**: The preview currently shows flat
   colors. GLB support should at minimum display texture metadata and export
   status; thumbnail rendering is deferred.
8. **Per-frame validation cost**: `refresh_validation_state()` is called
   unconditionally at the start of every `show_edit_mode` call (no dirty guard),
   iterating all meshes each frame.
9. **Per-frame preview repaint**: `PreviewRenderer::show` (and its inner
   `render_preview`) runs every egui repaint even when no camera interaction or
   mesh change occurred, because there is no idle-guard on `renderer.show(ui)`.
10. **Importer mesh list is not virtualized**: All mesh rows are rendered
    unconditionally; large GLB imports with many primitives will render all rows.
11. **Unconditional tab switch on Creature export**: `lib.rs` always sets
    `active_tab = EditorTab::Creatures` after a Creature export signal, dropping
    the user out of the Importer unexpectedly.

---

## Implementation Phases

### Phase 1: GLB Parser Foundation

> **Scope**: Create `mesh_glb_io.rs`, add the `gltf` dependency, implement all
> parser logic, and add unit tests. No UI changes. No `ObjImporterState` changes.

#### 1.1 Foundation Work

**Create** `sdk/campaign_builder/src/mesh_glb_io.rs`.

Add SPDX header as the **first two lines** of the file:

```
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

Add module-level `//!` doc comment explaining the module's role (parser-only,
no UI, mirrors `mesh_obj_io.rs`).

**Add dependency** in `sdk/campaign_builder/Cargo.toml` under `[dependencies]`:

```
gltf = { version = "1", features = ["import"] }
```

The `"import"` feature enables buffer and image loading from `.glb` files.
Confirm the version is compatible with the existing dependency tree by running
`cargo check -p campaign_builder --all-targets --all-features` after adding.

**Register the module** in `sdk/campaign_builder/src/lib.rs` by adding the
following line alongside the other `pub mod` declarations (alphabetical order,
between `mesh_obj_io` and `mesh_validation`):

```rust
pub mod mesh_glb_io;
```

**Define public or crate-visible parser types** in `mesh_glb_io.rs`:

| Type                        | Visibility   | Purpose                                                                                                                                                                                                                                                                                     |
| --------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GlbImportOptions`          | `pub(crate)` | Parser options: `source_path: Option<PathBuf>`, `scale: f32`, `flip_uv_v: bool`, `scene_index: usize`, `default_color: [f32; 4]`.                                                                                                                                                           |
| `GlbImportError`            | `pub`        | `thiserror`-derived error enum. Variants: `Io(#[from] std::io::Error)`, `GltfError(String)`, `UnsupportedPrimitive { mode: String }`, `MissingPositions { mesh: String, primitive: usize }`, `MissingBuffer { detail: String }`, `EmptyScene`, `InvalidIndex`. Use `#[error("…")]` on each. |
| `ImportedGlbScene`          | `pub(crate)` | Top-level result: `meshes: Vec<ImportedGlbMesh>`, `embedded_image_count: usize`, `material_count: usize`, `scene_count: usize`, `has_skinning: bool`, `has_animations: bool`.                                                                                                               |
| `ImportedGlbMesh`           | `pub(crate)` | One mesh row: `mesh_def: MeshDefinition`, `texture_payload: Option<ImportedGlbTexturePayload>`, `node_name: Option<String>`, `material_name: Option<String>`.                                                                                                                               |
| `ImportedGlbTexturePayload` | `pub(crate)` | Embedded image data — see field table below.                                                                                                                                                                                                                                                |

**`ImportedGlbTexturePayload` fields**:

| Field            | Type             | Meaning                                                                                                                        |
| ---------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `source_label`   | `String`         | Human-readable label: GLB image name when present, otherwise `"image_<index>"`.                                                |
| `file_name_hint` | `String`         | Sanitized export filename including extension (e.g., `"albedo_0.png"`). Derived from image name, MIME type, or index fallback. |
| `bytes`          | `Vec<u8>`        | Embedded image bytes. For base GLB support, this is always populated (external URI images fail with an explicit error).        |
| `mime_type`      | `Option<String>` | MIME type from glTF image metadata (e.g., `"image/png"`, `"image/jpeg"`).                                                      |

#### 1.2 Add Foundation Functionality

Implement the following **public or `pub(crate)` functions** in `mesh_glb_io.rs`:

| Function Signature                                                                                                                                                                                           | Purpose                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `pub fn import_glb_scene_from_file(path: &Path, options: &GlbImportOptions) -> Result<ImportedGlbScene, GlbImportError>`                                                                                     | Read a `.glb` file and call `import_glb_scene_from_bytes`.                                                   |
| `pub(crate) fn import_glb_scene_from_bytes(bytes: &[u8], options: &GlbImportOptions) -> Result<ImportedGlbScene, GlbImportError>`                                                                            | Main testable entry point. Parse the glTF document, iterate scene primitives, and return `ImportedGlbScene`. |
| `fn glb_primitive_to_mesh_definition(primitive: &gltf::Primitive, buffers: &[gltf::buffer::Data], options: &GlbImportOptions, mesh_name: &str, prim_index: usize) -> Result<MeshDefinition, GlbImportError>` | Convert one glTF primitive to `MeshDefinition`. Private helper.                                              |
| `fn extract_base_color_texture_payload(primitive: &gltf::Primitive, images: &[gltf::image::Data]) -> Result<Option<ImportedGlbTexturePayload>, GlbImportError>`                                              | Extract base-color texture image bytes and filename metadata. Private helper.                                |
| `fn convert_gltf_material(material: &gltf::Material) -> MaterialDefinition`                                                                                                                                  | Map PBR fields to `MaterialDefinition`. Private helper.                                                      |
| `fn sanitize_glb_file_name(label: &str, mime_type: Option<&str>) -> String`                                                                                                                                  | Produce a lowercase-snake-case filename with extension from label + MIME. Private helper.                    |

**Geometry conversion requirements**:

| glTF Attribute                          | Antares Field                                              | Required Behavior                                                                                                                                           |
| --------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `POSITION`                              | `MeshDefinition.vertices`                                  | Required. Return `GlbImportError::MissingPositions` if absent. Apply `options.scale` to each vertex.                                                        |
| Indices                                 | `MeshDefinition.indices`                                   | Use explicit index buffer when present. When absent (unindexed), generate sequential `[0, 1, 2, 3, …]` indices.                                             |
| `NORMAL`                                | `MeshDefinition.normals`                                   | Preserve as `Some(normals)` when present; `None` otherwise (runtime calculates).                                                                            |
| `TEXCOORD_0`                            | `MeshDefinition.uvs`                                       | Preserve when present. When `options.flip_uv_v` is `true`, set `uv[1] = 1.0 - uv[1]` for every UV pair.                                                     |
| `pbrMetallicRoughness.baseColorFactor`  | `MeshDefinition.color` AND `MaterialDefinition.base_color` | Use RGBA factor. Default `[1.0, 1.0, 1.0, 1.0]` when material is absent.                                                                                    |
| `pbrMetallicRoughness.baseColorTexture` | `MeshDefinition.texture_path`                              | Set to placeholder `"__glb_texture_{mesh_index}_{prim_index}"` during parsing. The export step (Phase 4) rewrites this to the final campaign-relative path. |
| `pbrMetallicRoughness.metallicFactor`   | `MaterialDefinition.metallic`                              | Preserve float.                                                                                                                                             |
| `pbrMetallicRoughness.roughnessFactor`  | `MaterialDefinition.roughness`                             | Preserve float.                                                                                                                                             |
| `emissiveFactor`                        | `MaterialDefinition.emissive`                              | Store as `Some([r, g, b])` when nonzero. `None` when `[0.0, 0.0, 0.0]`.                                                                                     |
| `alphaMode`                             | `MaterialDefinition.alpha_mode`                            | Map `OPAQUE → AlphaMode::Opaque`, `BLEND → AlphaMode::Blend`, `MASK → AlphaMode::Mask`.                                                                     |

**Primitive mode requirements**:

| Primitive Mode                               | Support                                                                                               |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `TRIANGLES`                                  | Required.                                                                                             |
| `TRIANGLE_STRIP`                             | Triangulate OR return `GlbImportError::UnsupportedPrimitive`. Document the choice in module comments. |
| `TRIANGLE_FAN`                               | Same as `TRIANGLE_STRIP`.                                                                             |
| `LINES`, `POINTS`, `LINE_STRIP`, `LINE_LOOP` | Return `GlbImportError::UnsupportedPrimitive { mode: "Lines" }` (or appropriate name).                |

**Transform requirements**:

- **Do not** flatten node world transforms into vertex positions.
- **Do** preserve each primitive's node transform metadata in `ImportedGlbMesh`
  as an optional field for future use: `node_transform: Option<[[f32; 4]; 4]>`.
- `MeshTransform` is applied by the existing export code via
  `CreatureDefinition.mesh_transforms`; the parser should not duplicate that logic.

**Scene iteration**:

- Import `document.default_scene()` when it exists; fall back to
  `document.scenes().next()`. Return `GlbImportError::EmptyScene` when no
  scenes are present.
- Iterate scene nodes depth-first. Collect all primitives across all meshes
  referenced by nodes.
- Preserve node name in `ImportedGlbMesh.node_name` for UI display.

**Texture payload extraction**:

- Only base-color texture is extracted in this phase.
- External URI textures: return `Err(GlbImportError::MissingBuffer { detail:
"external URI textures are not supported; embed textures in the GLB file" })`.
- Embedded images: copy bytes from `gltf::image::Data.pixels` and the MIME type.
- When no base-color texture is present on a primitive, `texture_payload` is
  `None` and `texture_path` stays `None`.

#### 1.3 Integrate Foundation Work

Confirm `pub mod mesh_glb_io;` is added to `sdk/campaign_builder/src/lib.rs`
(done in 1.1). Verify `cargo check -p campaign_builder --all-targets
--all-features` succeeds. No other integration steps in Phase 1.

Do **not** modify `src/domain/visual/mod.rs` or any domain types in Phase 1.

#### 1.4 Testing Requirements

Add unit tests in `#[cfg(test)] mod tests` at the bottom of `mesh_glb_io.rs`.

Use `import_glb_scene_from_bytes` for all tests. Generate in-memory GLB fixtures
using the `gltf` crate's builder API where possible to avoid binary file
dependencies. If a helper cannot be written without a binary file, place the
binary fixture at `data/test_fixtures/<name>.glb` and access it via:

```rust
fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/test_fixtures")
        .join(name)
}
```

Never reference `campaigns/tutorial` from any test.

Required tests:

| Test Name                                                  | Assertion                                                                                                                         |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `test_import_glb_rejects_missing_positions`                | GLB primitive without `POSITION` returns `GlbImportError::MissingPositions`.                                                      |
| `test_import_glb_triangle_positions_indices_uvs`           | Triangle mesh: `vertices.len() == 3`, `indices == [0, 1, 2]`, UVs preserved.                                                      |
| `test_import_glb_applies_scale_to_vertices`                | `options.scale = 2.0` doubles all vertex coordinates.                                                                             |
| `test_import_glb_material_base_color_maps_to_mesh_def`     | Base-color factor `[0.5, 0.5, 0.5, 1.0]` maps to `mesh_def.color` and `material.base_color`.                                      |
| `test_import_glb_embedded_base_color_texture_payload`      | Embedded PNG bytes are extracted into `texture_payload.bytes`; `file_name_hint` ends with `.png`.                                 |
| `test_import_glb_multiple_primitives_distinct_payloads`    | Two primitives with distinct materials produce two `ImportedGlbMesh` entries with distinct `texture_payload.source_label` values. |
| `test_import_glb_unsupported_primitive_mode_returns_error` | Primitive mode `Lines` returns `GlbImportError::UnsupportedPrimitive`.                                                            |
| `test_import_glb_no_texture_gives_none_payload`            | Primitive with no base-color texture: `texture_payload` is `None` and `mesh_def.texture_path` is `None`.                          |
| `test_import_glb_empty_scene_returns_error`                | GLB with no scenes returns `GlbImportError::EmptyScene`.                                                                          |
| `test_import_glb_external_uri_texture_returns_error`       | GLB referencing an external texture URI returns `GlbImportError::MissingBuffer`.                                                  |
| `test_import_glb_flip_uv_v`                                | `flip_uv_v: true` produces `uv[1] = 1.0 - original_uv[1]`.                                                                        |

#### 1.5 Deliverables

- [ ] `sdk/campaign_builder/src/mesh_glb_io.rs` created with SPDX header, module doc comment, and all types/functions documented with `///` doc comments.
- [ ] `gltf = { version = "1", features = ["import"] }` added to `sdk/campaign_builder/Cargo.toml`.
- [ ] `pub mod mesh_glb_io;` added to `sdk/campaign_builder/src/lib.rs`.
- [ ] GLB parser converts positions, indices, normals, UVs, material color, metallic, roughness, emissive, alpha mode, and base-color texture payloads.
- [ ] Embedded image bytes are captured in `ImportedGlbTexturePayload.bytes`.
- [ ] Texture placeholder format `"__glb_texture_{mesh_index}_{prim_index}"` is used consistently.
- [ ] Unsupported primitive modes return `GlbImportError::UnsupportedPrimitive`.
- [ ] External URI textures return `GlbImportError::MissingBuffer`.
- [ ] All 11 unit tests pass.
- [ ] Zero `cargo clippy` warnings.

#### 1.6 Success Criteria

- A valid `.glb` with one textured triangle mesh imports into one `ImportedGlbMesh` with `texture_payload.bytes` populated.
- A valid `.glb` with multiple textured primitives imports into multiple `ImportedGlbMesh` entries, each with a distinct `texture_payload`.
- `cargo nextest run -p campaign_builder --all-features` exits zero.
- No test references `campaigns/tutorial`.

---

### Phase 2: Format-Neutral Importer State

> **Scope**: Extend `obj_importer.rs` with `ImportSourceFormat`,
> `ImportedTexturePayload`, and `load_glb_file`. No UI changes. OBJ behavior
> must be unchanged.
>
> **Depends on**: Phase 1 complete (`mesh_glb_io.rs` compiles).

#### 2.1 Feature Work

Edit `sdk/campaign_builder/src/obj_importer.rs`.

**Add import** at the top of the file's `use` block:

```rust
use crate::mesh_glb_io::{
    import_glb_scene_from_file, GlbImportError, GlbImportOptions, ImportedGlbScene,
};
```

**Add `ImportSourceFormat` enum** after the existing `ImportedMtlSourceKind` enum:

```rust
/// Records whether the current importer session loaded an OBJ or GLB source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportSourceFormat {
    #[default]
    Obj,
    Glb,
}
```

**Add `ImportedTexturePayload` struct** after `ImportedMaterialSwatch`:

```rust
/// Generalized texture source for export: covers both OBJ filesystem paths and
/// GLB embedded image bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTexturePayload {
    /// Human-readable label for UI display (e.g., image name or material name).
    pub source_label: String,
    /// Sanitized export filename candidate including extension (e.g., `"albedo_0.png"`).
    pub file_name_hint: String,
    /// Embedded image bytes from a GLB file. `None` for OBJ filesystem textures.
    pub bytes: Option<Vec<u8>>,
    /// Filesystem source path for OBJ/MTL textures. `None` for embedded GLB images.
    pub source_path: Option<PathBuf>,
    /// MIME type when known from GLB metadata.
    pub mime_type: Option<String>,
}
```

**Extend `ImportedMesh`** by replacing the existing `texture_source_path:
Option<PathBuf>` field with a generalized payload field. Keep
`texture_source_path` as a deprecated shim only if existing tests require it;
otherwise replace it:

```rust
pub struct ImportedMesh {
    // … existing fields unchanged …

    /// Generalized texture source used at export time.
    /// Replaces the OBJ-only `texture_source_path` field.
    pub texture_payload: Option<ImportedTexturePayload>,
}
```

If removing `texture_source_path` breaks existing tests in `obj_importer.rs` or
`obj_importer_ui.rs`, update those sites to use `texture_payload.as_ref().and_then(|p| p.source_path.as_ref())` as a direct replacement for the old field access.

**Extend `ObjImporterState`** by adding the `source_format` field:

```rust
pub struct ObjImporterState {
    // … existing fields …

    /// Which format was loaded in the current importer session.
    pub source_format: ImportSourceFormat,
}
```

Update `Default for ObjImporterState` to initialize `source_format:
ImportSourceFormat::Obj`.

Update `ObjImporterState::clear()` to reset `source_format` to
`ImportSourceFormat::Obj`. The `clear()` function must NOT preserve `source_format`
across resets; it should reset to the default.

**Add `ObjImporterError` variant** for GLB failures:

```rust
pub enum ObjImporterError {
    Obj(#[from] ObjError),
    Palette(#[from] PaletteError),

    /// GLB loading failed.
    #[error("GLB import failed: {0}")]
    Glb(#[from] GlbImportError),
}
```

#### 2.2 Integrate Feature

**Add `ObjImporterState::load_glb_file`** method in the `impl ObjImporterState`
block, alongside `load_obj_file`:

```rust
/// Loads meshes from a GLB file and updates importer state.
pub fn load_glb_file(&mut self, path: &Path) -> Result<(), ObjImporterError> {
    let options = GlbImportOptions {
        source_path: Some(path.to_path_buf()),
        scale: self.scale,
        ..Default::default()
    };
    let scene = import_glb_scene_from_file(path, &options)?;
    self.load_imported_glb_scene(Some(path.to_path_buf()), scene);
    Ok(())
}
```

**Add `load_imported_glb_scene`** private helper:

```rust
fn load_imported_glb_scene(&mut self, source_path: Option<PathBuf>, scene: ImportedGlbScene) {
    let meshes: Vec<ImportedMesh> = scene
        .meshes
        .into_iter()
        .enumerate()
        .map(|(i, glb_mesh)| {
            let payload = glb_mesh.texture_payload.map(|p| ImportedTexturePayload {
                source_label: p.source_label,
                file_name_hint: p.file_name_hint,
                bytes: Some(p.bytes),
                source_path: None,
                mime_type: p.mime_type,
            });
            ImportedMesh::from_imported_glb_mesh(i, glb_mesh.mesh_def, payload)
        })
        .collect();

    let glb_metadata_summary = format!(
        "GLB: {} mesh(es), {} embedded image(s), {} material(s){}{}",
        meshes.len(),
        scene.embedded_image_count,
        scene.material_count,
        if scene.has_skinning { " [skinning ignored]" } else { "" },
        if scene.has_animations { " [animations ignored]" } else { "" },
    );

    self.load_imported_mesh_rows(
        source_path,
        meshes,
        ImportedMtlSourceKind::None,   // GLB has no MTL
        Vec::new(),                     // no declared_mtl_libraries
        Vec::new(),                     // no resolved_mtl_paths
        Vec::new(),                     // no imported_material_palette
    );
    self.source_format = ImportSourceFormat::Glb;
    self.status_message = glb_metadata_summary;
}
```

**Add `ImportedMesh::from_imported_glb_mesh`** private constructor in `impl
ImportedMesh`:

```rust
fn from_imported_glb_mesh(
    index: usize,
    mesh_def: MeshDefinition,
    payload: Option<ImportedTexturePayload>,
) -> Self {
    let color = mesh_def.color;
    let color_source = if color != [1.0, 1.0, 1.0, 1.0] {
        ImportedMeshColorSource::ImportedMaterial
    } else {
        ImportedMeshColorSource::AutoAssigned
    };
    Self {
        name: mesh_def.name.clone().unwrap_or_else(|| format!("mesh_{}", index)),
        vertex_count: mesh_def.vertices.len(),
        triangle_count: mesh_def.indices.len() / 3,
        color,
        color_source,
        selected: false,
        mesh_def,
        texture_payload: payload,
    }
}
```

**Update `ObjImporterState::load_obj_file`** to populate `texture_payload` from
the existing OBJ `texture_source_path` data, and set `source_format =
ImportSourceFormat::Obj`. This preserves all existing OBJ behavior through the
new generalized field.

#### 2.3 Configuration Updates

No campaign data format changes. The importer still exports RON.

Update the module-level `//!` doc comment in `obj_importer.rs` to reflect that
the state now supports both OBJ and GLB formats.

#### 2.4 Testing Requirements

Required tests (add to `mod tests` in `obj_importer.rs`):

| Test Name                                                       | Assertion                                                                                                                      |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `test_obj_importer_state_load_glb_file_sets_loaded_mode`        | After `load_glb_file`, `state.mode == ImporterMode::Loaded` and `state.source_format == ImportSourceFormat::Glb`.              |
| `test_obj_importer_state_load_glb_preserves_texture_payload`    | Imported mesh from a textured GLB has `texture_payload.is_some()` and `mesh_def.texture_path` contains the placeholder string. |
| `test_obj_importer_state_load_glb_metadata_summary_in_status`   | `state.status_message` contains mesh count and image count after GLB load.                                                     |
| `test_obj_importer_clear_resets_source_format_to_obj`           | After `load_glb_file` then `clear()`, `state.source_format == ImportSourceFormat::Obj`.                                        |
| `test_obj_importer_load_obj_still_works_after_glb_fields_added` | Existing OBJ load test succeeds; `source_format == ImportSourceFormat::Obj`.                                                   |

#### 2.5 Deliverables

- [ ] `ImportSourceFormat` enum added to `obj_importer.rs`.
- [ ] `ImportedTexturePayload` struct added to `obj_importer.rs`.
- [ ] `ImportedMesh.texture_source_path` replaced by `texture_payload: Option<ImportedTexturePayload>`.
- [ ] All OBJ call sites updated to use `texture_payload.source_path` instead of the removed field.
- [ ] `ObjImporterState.source_format` field added and initialized/cleared correctly.
- [ ] `ObjImporterState::load_glb_file` implemented.
- [ ] `ObjImporterError::Glb` variant added.
- [ ] All 5 new tests pass; all existing OBJ tests still pass.
- [ ] Zero `cargo clippy` warnings.

#### 2.6 Success Criteria

- The importer can load both GLB and OBJ into the same `ObjImporterState.meshes` list.
- Export target selection (Creature / Item / Furniture) works identically for both formats.
- All existing OBJ workflow tests pass without modification.

---

### Phase 3: Importer UI for GLB Selection

> **Scope**: Update `obj_importer_ui.rs` to present a multi-format importer,
> add `.glb` file picking and dispatch, and show GLB-specific metadata. No
> changes to `mesh_glb_io.rs` or `obj_importer.rs`.
>
> **Depends on**: Phase 2 complete (`load_glb_file` exists on `ObjImporterState`).

#### 3.1 Feature Work

Edit `sdk/campaign_builder/src/obj_importer_ui.rs`.

**Do NOT rename the file** `obj_importer_ui.rs`. Update its module-level `//!`
doc comment to say "OBJ and GLB importer tab UI."

Required UI text changes:

| Location                           | Old Text                                                                                            | New Text                                                                                                                       |
| ---------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `show_obj_importer_tab` heading    | `"OBJ Importer"`                                                                                    | `"Model Importer"`                                                                                                             |
| `show_obj_importer_tab` help label | `"Load a Wavefront OBJ, adjust mesh colors, then export a creature, item, or furniture RON asset."` | `"Load a Wavefront OBJ or binary glTF (GLB) model, adjust mesh colors, then export a creature, item, or furniture RON asset."` |
| `render_idle_mode` Source label    | `"Source OBJ:"`                                                                                     | `"Source File:"`                                                                                                               |
| `render_idle_mode` Load button     | `"Load OBJ"`                                                                                        | `"Load Model"`                                                                                                                 |
| `render_loaded_mode` metadata row  | (none currently)                                                                                    | Add `"Format:"` grid row showing `"OBJ"` or `"GLB"` based on `state.source_format`.                                            |

Required function additions and changes:

| Function                     | Change                                                                                                                                                                                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pick_obj_file`              | Rename to `pick_model_file`. Add a second filter entry for GLB: `.add_filter("Binary glTF", &["glb"])`. Keep OBJ filter.                                                                                                                                  |
| `load_obj_into_state`        | Replace with `load_model_into_state(state, path) -> Result<(), ObjImportError>`. Dispatch on file extension: `.obj` → `state.load_obj_file(path)`, `.glb` → `state.load_glb_file(path)`, other → `Err(ObjImportError::UnknownFormat { extension: "…" })`. |
| `render_idle_mode`           | Call `pick_model_file` instead of `pick_obj_file`. Call `load_model_into_state` when Load button clicked.                                                                                                                                                 |
| `render_loaded_mode`         | Add format metadata row. Show MTL controls only when `state.source_format == ImportSourceFormat::Obj`. When GLB: show embedded image count, material count, skinning/animation warning banners from `state.status_message`.                               |
| `render_mtl_source_controls` | No changes to the function body; it is only called from `render_loaded_mode` for OBJ format.                                                                                                                                                              |

**Add `ObjImportError` variant** for unknown formats:

```rust
enum ObjImportError {
    LoadFailed { path: String, message: String },
    UnknownFormat { extension: String },
}
```

Implement `Display` for `UnknownFormat` as:
`"Unsupported file format '.{extension}'; supported formats are .obj and .glb"`.

**GLB metadata display** (in `render_loaded_mode` when `source_format == Glb`):

- Show a text row: `"Format: GLB (binary glTF)"`.
- Parse `state.status_message` for embedded image count and display as:
  `"Embedded textures: N"`.
- If `state.status_message` contains `"[skinning ignored]"`, display an orange
  warning label: `"⚠ Skinning/animations present but not imported"`.
- Do NOT show MTL controls for GLB format.

**egui rules**: Verify ALL of the following before marking Phase 3 complete:

- Every new `ScrollArea` has an `id_salt`.
- Every loop body uses `push_id`.
- Any state change that affects layout calls `ui.ctx().request_repaint()`.
- No `SidePanel`, `TopBottomPanel`, or `CentralPanel` is conditionally skipped
  without a placeholder. Use `sdk/AGENTS.md` L1197–L1268 audit checklist.

#### 3.2 Integrate Feature

No changes to `lib.rs` module registration in this phase (the module is already
`obj_importer_ui`).

No changes to the `ObjImporterUiSignal` enum; GLB and OBJ exports emit the same
signals.

#### 3.3 Configuration Updates

No RON data changes.

#### 3.4 Testing Requirements

Required tests (add to `mod tests` in `obj_importer_ui.rs`):

| Test Name                                              | Assertion                                                       |
| ------------------------------------------------------ | --------------------------------------------------------------- |
| `test_load_model_into_state_dispatches_obj`            | Path ending in `.obj` calls OBJ loader; `source_format == Obj`. |
| `test_load_model_into_state_dispatches_glb`            | Path ending in `.glb` calls GLB loader; `source_format == Glb`. |
| `test_load_model_into_state_rejects_unknown_extension` | Path ending in `.fbx` returns `ObjImportError::UnknownFormat`.  |

#### 3.5 Deliverables

- [ ] `show_obj_importer_tab` heading updated to `"Model Importer"`.
- [ ] Help text updated to mention both OBJ and GLB.
- [ ] `pick_obj_file` renamed to `pick_model_file` with `.glb` filter added.
- [ ] `load_obj_into_state` replaced by `load_model_into_state` with extension dispatch.
- [ ] `render_idle_mode` uses new function names.
- [ ] `render_loaded_mode` shows `"Format: GLB"` or `"Format: OBJ"`.
- [ ] MTL controls hidden for GLB-loaded state.
- [ ] GLB metadata (image count, skinning warning) visible for GLB-loaded state.
- [ ] `ObjImportError::UnknownFormat` added with descriptive message.
- [ ] SDK egui ID audit checklist (`sdk/AGENTS.md` L1197–L1268) passes for all touched UI.
- [ ] All 3 new tests pass; all existing importer UI tests still pass.

#### 3.6 Success Criteria

- A user can select a `.glb`, click Load, inspect mesh rows, and proceed to
  export without interacting with MTL controls.
- OBJ import behavior remains unchanged.
- Unknown file extensions produce a clear user-facing error message.

---

### Phase 4: Embedded Texture Export

> **Scope**: Extend `copy_imported_textures_into_campaign` and
> `resolve_imported_texture_source` in `obj_importer_ui.rs` to write embedded
> GLB image bytes to campaign texture assets. No parser changes.
>
> **Depends on**: Phase 3 complete (`ImportedTexturePayload` is populated by
> both loaders).

#### 4.1 Feature Work

Edit `sdk/campaign_builder/src/obj_importer_ui.rs`.

**Current behavior of `copy_imported_textures_into_campaign`**:
Iterates `creature.meshes`, finds `texture_path`, calls
`resolve_imported_texture_source` → `Option<PathBuf>`, copies the file via
`fs::copy`. Fails with `MissingTexture` when the source path is `None`.

**Required new behavior**:

For each mesh with a `texture_path`:

1. If the path already resolves to a campaign-relative file, skip (no copy).
2. Otherwise look up `state.meshes[mesh_index].texture_payload`:
   - If `payload.bytes` is `Some(bytes)`: write the bytes to the destination file.
   - If `payload.source_path` is `Some(path)` and `path.exists()`: copy the file.
   - If neither: fail with `ObjImporterExportError::MissingTexture`.
3. Rewrite `mesh_def.texture_path` to the campaign-relative destination path.

**Destination naming rules** (all producing lowercase-snake-case filenames):

| Source                                | Filename                                                              |
| ------------------------------------- | --------------------------------------------------------------------- |
| `payload.file_name_hint` is non-empty | Use it directly after sanitization.                                   |
| `payload.source_label` present        | Sanitize to snake_case; append extension from MIME type.              |
| Fallback                              | `"texture_{mesh_index}"` + extension from MIME, or `.bin` if unknown. |
| MIME `image/png`                      | Extension `.png`.                                                     |
| MIME `image/jpeg`                     | Extension `.jpg`.                                                     |
| Unknown MIME                          | Extension `.bin`.                                                     |

Deduplication rule: if two meshes reference payloads with identical `bytes`
content, map both to the same destination path. Use a
`HashMap<u64 /* content hash */, String /* destination */>` keyed on a hash of
the byte content.

**Refactor `resolve_imported_texture_source`**:

Replace the current `fn resolve_imported_texture_source(state, mesh_index,
texture_path, campaign_dir) -> Option<PathBuf>` with an enum-returning version:

```rust
enum ResolvedTextureSource {
    /// Texture is already inside the campaign directory; no copy needed.
    AlreadyCampaignRelative,
    /// Copy from this filesystem path.
    FilesystemPath(PathBuf),
    /// Write these bytes to the destination.
    EmbeddedBytes { bytes: Vec<u8>, file_name_hint: String },
    /// No source found.
    Missing,
}
```

Adapt `copy_imported_textures_into_campaign` to use this enum instead of the
old `Option<PathBuf>`.

**Guard on missing texture before RON write**: The existing
`ObjImporterExportError::MissingTexture` must be returned BEFORE calling any
RON serialization or `fs::write` for the creature file. This was already the
behavior; confirm it is preserved.

#### 4.2 Integrate Feature

`copy_imported_textures_into_campaign` is called from `export_state_to_campaign`
before the RON write. Confirm the call order is preserved:

1. `build_creature_definition` (produces RON-ready struct)
2. `copy_imported_textures_into_campaign` (writes textures, rewrites `texture_path`)
3. RON serialization and file write

`build_creature_definition` must not permanently mutate `state.meshes`; it clones
the definition. Confirm the mutable `creature` passed to
`copy_imported_textures_into_campaign` is the export clone, not the live state.

#### 4.3 Configuration Updates

No new campaign configuration files. Exported texture assets are regular campaign
files under `assets/textures/imported/<asset_name>/`. The path convention is
unchanged from OBJ exports.

#### 4.4 Testing Requirements

Required tests (add to `mod tests` in `obj_importer_ui.rs`):

| Test Name                                                         | Assertion                                                                                 |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `test_export_glb_embedded_texture_writes_campaign_texture_file`   | After export, `assets/textures/imported/<asset>/texture_0.png` exists with correct bytes. |
| `test_export_glb_rewrites_mesh_texture_path_to_campaign_relative` | Exported RON `texture_path` is `"assets/textures/imported/<asset>/texture_0.png"`.        |
| `test_export_glb_multiple_embedded_textures_get_distinct_paths`   | Two meshes with distinct payloads produce two distinct files with no collision.           |
| `test_export_glb_deduplicates_identical_texture_payload`          | Two meshes with identical byte payloads produce one file; both RON entries reference it.  |
| `test_export_glb_missing_texture_payload_fails_before_ron_write`  | Missing payload: `MissingTexture` error is returned; no RON file is created.              |
| `test_export_obj_texture_copy_still_passes`                       | All pre-existing OBJ texture copy tests still pass without modification.                  |

#### 4.5 Deliverables

- [ ] `ResolvedTextureSource` enum added.
- [ ] `resolve_imported_texture_source` refactored to return `ResolvedTextureSource`.
- [ ] `copy_imported_textures_into_campaign` writes embedded byte payloads to campaign files.
- [ ] Payload deduplication by content hash prevents duplicate texture files.
- [ ] RON write guard confirmed: `MissingTexture` error fires before any RON write.
- [ ] All 6 new tests pass; all pre-existing OBJ texture tests pass.
- [ ] Zero `cargo clippy` warnings.

#### 4.6 Success Criteria

- A textured GLB exported as a Creature produces campaign texture files under
  `assets/textures/imported/<name>/` and RON that references them correctly.
- Multiple GLB model parts with different textures produce distinct texture files.
- Runtime `texture_loading_system` can load the exported paths without modification.

---

### Phase 5: Runtime and Domain Compatibility

> **Scope**: Verify exported GLB RON and texture paths are compatible with the
> game runtime. Confirm Item and Furniture export targets preserve texture
> references. Document unsupported PBR fields. No parser changes.
>
> **Depends on**: Phase 4 complete (export produces valid texture files and RON).

#### 5.1 Feature Work

**GLB-to-domain material field mapping** (implemented in Phase 1/2; verified here):

| glTF / GLB Material Field                       | Antares Field                                            | Supported                                 |
| ----------------------------------------------- | -------------------------------------------------------- | ----------------------------------------- |
| `pbrMetallicRoughness.baseColorFactor`          | `MaterialDefinition.base_color` + `MeshDefinition.color` | ✅ Phase 1                                |
| `pbrMetallicRoughness.baseColorTexture`         | `MeshDefinition.texture_path`                            | ✅ Phase 4                                |
| `pbrMetallicRoughness.metallicFactor`           | `MaterialDefinition.metallic`                            | ✅ Phase 1                                |
| `pbrMetallicRoughness.roughnessFactor`          | `MaterialDefinition.roughness`                           | ✅ Phase 1                                |
| `emissiveFactor`                                | `MaterialDefinition.emissive`                            | ✅ Phase 1                                |
| `alphaMode`                                     | `MaterialDefinition.alpha_mode`                          | ✅ Phase 1                                |
| `normalTexture`                                 | (none)                                                   | ❌ Deferred — log warning if present      |
| `occlusionTexture`                              | (none)                                                   | ❌ Deferred                               |
| `pbrMetallicRoughness.metallicRoughnessTexture` | (none)                                                   | ❌ Deferred                               |
| Skinning / animation                            | (none)                                                   | ❌ Ignored with warning in status message |

**Runtime compatibility check**:

Review `src/game/systems/creature_meshes.rs::texture_loading_system`. Confirm
the system reads `MeshDefinition.texture_path` as a campaign-relative path
string and loads it with Bevy's asset loader. GLB exports produce the same
`"assets/textures/imported/<name>/<file>"` convention as OBJ exports. If the
path convention matches, no runtime changes are needed.

**Item and Furniture export path**:

The importer uses `ExportType::Item` and `ExportType::Furniture` via
`export_state_to_campaign`, which calls `build_creature_definition` (produces a
`CreatureDefinition`) and serializes it as RON. `CreatureDefinition.meshes` is a
`Vec<MeshDefinition>`, and `texture_path` is on `MeshDefinition`. Confirm:

- Item RON is written to `data/creatures/items/<category>/<name>.ron` or
  the path returned by `preview_export_relative_path(ExportType::Item, …)`.
- `texture_path` fields survive the RON round-trip.
- The game runtime uses the same `texture_loading_system` for item mesh
  rendering (verify in `src/game/systems/` or document the gap if it does not).

**Add unsupported-channel warnings** to `mesh_glb_io.rs`: When
`normalTexture`, `occlusionTexture`, or `metallicRoughnessTexture` is present on
a primitive material, add a `has_unsupported_pbr_channels: bool` flag on
`ImportedGlbMesh` (or `ImportedGlbScene`). Surface this in the GLB metadata
status message so users know what was silently skipped.

#### 5.2 Integrate Feature

If runtime texture loading is confirmed compatible (no code changes needed),
this phase is documentation-only plus test coverage.

If a gap is found in item/furniture runtime rendering:

- Document it explicitly in `docs/explanation/implementations.md` under a
  "Known Limitations" section.
- Do NOT add JSON or YAML game data files to work around the gap.

#### 5.3 Configuration Updates

Document the GLB texture export convention in `docs/reference/campaign_content_format.md`:

- Embedded GLB textures are written to `assets/textures/imported/<asset_name>/`.
- Filenames are derived from GLB image names or fallback to `texture_<index>.<ext>`.
- The exported RON `texture_path` is a campaign-relative path to the copied file.

#### 5.4 Testing Requirements

Required tests:

| Test Name                                                | Assertion                                                                                                           |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `test_glb_material_base_color_maps_to_domain_material`   | PBR base color `[0.8, 0.2, 0.1, 1.0]` maps to `MaterialDefinition.base_color` and `MeshDefinition.color`.           |
| `test_glb_material_metallic_roughness_maps_to_domain`    | Metallic `0.3` and roughness `0.7` map to `MaterialDefinition` fields.                                              |
| `test_glb_exported_texture_path_round_trips_through_ron` | A `CreatureDefinition` with GLB-derived `texture_path` serializes and deserializes via RON without path corruption. |
| `test_glb_item_export_preserves_texture_path`            | Exporting with `ExportType::Item` preserves `texture_path` in the RON output.                                       |
| `test_glb_furniture_export_preserves_texture_path`       | Exporting with `ExportType::Furniture` preserves `texture_path` in the RON output.                                  |

#### 5.5 Deliverables

- [ ] Runtime compatibility with `texture_loading_system` confirmed (no code change or documented gap).
- [ ] Item and Furniture RON exports confirmed to preserve `texture_path`.
- [ ] Unsupported PBR channel warning flag added to `ImportedGlbScene` or `ImportedGlbMesh`.
- [ ] GLB metadata status message lists unsupported PBR channels when present.
- [ ] `docs/reference/campaign_content_format.md` updated with GLB texture export convention.
- [ ] All 5 new tests pass.
- [ ] Zero `cargo clippy` warnings.

#### 5.6 Success Criteria

- GLB-imported Creature, Item, and Furniture RON exports preserve material color and
  base-color texture references through RON round-trip.
- Runtime rendering does not require GLB files after export; it uses RON plus
  copied texture assets from `assets/textures/imported/`.
- Unsupported PBR channels fail loudly (warning in UI status) rather than silently.

---

### Phase 6: Importer and Creature Preview Responsiveness

> **Scope**: Fix confirmed per-frame performance problems in Campaign Builder that
> become more visible when loading large OBJ or GLB models.
>
> **Depends on**: Phase 3 complete (GLB import is usable; problems are observable
> with large GLB files).

**Note**: The three issues below are independent and may be implemented in any
order within this phase. Implement all three before marking Phase 6 complete.

#### 6.1 Feature Work

**Finding A — Unconditional per-frame validation** (file:
`sdk/campaign_builder/src/creatures_editor/mod.rs`):

`show_edit_mode` calls `self.refresh_validation_state()` unconditionally on
line ~L1578 every frame. `refresh_validation_state` calls
`mesh_validation::validate_mesh` for every mesh. No dirty guard exists.

Required fix:

- Add `validation_dirty: bool` field to `CreaturesEditorState` (default `true`).
- Set `validation_dirty = true` whenever meshes, transforms, or creature-level
  properties change (wherever `preview_dirty` is also set to `true`, set
  `validation_dirty = true` too).
- In `show_edit_mode`, call `refresh_validation_state()` only when
  `self.validation_dirty` is `true`. After running, set `validation_dirty = false`.

Do **not** add a complex signature or content cache. The dirty flag alone is
sufficient because the edit buffer can only change through explicit user
interactions that already set `preview_dirty`.

---

**Finding B — Per-frame preview repaint** (files:
`sdk/campaign_builder/src/creatures_editor/preview_panel.rs`,
`sdk/campaign_builder/src/preview_renderer.rs`):

The existing `preview_dirty` flag gates `sync_preview_renderer_from_edit_buffer`
correctly, but `renderer.show(ui)` is called every frame regardless — causing
`render_preview` to repaint all wireframe triangles on every egui repaint, even
with no camera or mesh change.

**Clarification**: `renderer.show(ui)` must still run every frame to respond to
mouse/keyboard camera interactions and to paint the UI widget. The fix is to
make the internal `render_preview` call within `show` skip the expensive drawing
pass when no interaction occurred and no update is pending.

Required fix in `preview_renderer.rs::show`:

- Add `last_rendered_signature: Option<u64>` field to `PreviewRenderer`.
- In `show`, compute a "render signature" by hashing the camera state and
  `needs_update` flag. Compare against `last_rendered_signature`.
- Call `render_preview` only when the signature changed or `needs_update` is true.
- Store the new signature in `last_rendered_signature` after rendering.
- When skipping `render_preview`, still call `ui.allocate_response` to occupy
  the correct amount of space and return the `interacted` boolean.

Add a user-visible **Live Preview** toggle button to `show_preview_panel`:

```
[ ] Live Preview   [🔄 Refresh]
```

- When `Live Preview` is **checked** (default for ≤ 5000 triangles): normal
  behavior — `preview_dirty` triggers re-sync on every change.
- When `Live Preview` is **unchecked**: sync only when `Refresh` is clicked.
  Set `preview_dirty = false` after refresh. Show a `"Preview paused"` label.
- Add `live_preview_enabled: bool` field (default `true`) to
  `CreaturesEditorState`.
- Auto-uncheck for new creatures with `total_triangles() > 50_000`.

---

**Finding C — Non-virtualized importer mesh list** (file:
`sdk/campaign_builder/src/obj_importer_ui.rs`):

`render_loaded_mode` renders all mesh rows unconditionally inside a `ScrollArea`.
For large GLB imports (100+ primitives), all rows are painted each frame.

Required fix using `ScrollArea::show_rows`:

```rust
let row_height = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
let num_rows = state.meshes.len();
egui::ScrollArea::vertical()
    .id_salt("importer_mesh_list_scroll")
    .show_rows(ui, row_height, num_rows, |ui, row_range| {
        for i in row_range {
            ui.push_id(i, |ui| {
                // render row i from state.meshes[i]
            });
        }
    });
```

Use a fixed row height equal to one line of body text plus item spacing. If
any mesh row requires variable height (e.g., a color swatch on a second line),
use a uniform expanded row height estimated from the tallest expected row.

---

**Finding D — Unconditional tab switch after Creature export** (file:
`sdk/campaign_builder/src/lib.rs`, lines ~L1354–L1358):

The `ObjImporterUiSignal::Creature` branch unconditionally sets
`self.ui_state.active_tab = EditorTab::Creatures`.

Required fix:

- Add `open_after_export: bool` field to `ObjImporterState` (default `false`).
- Add a checkbox in `render_loaded_mode` under the export controls:
  `[ ] Open exported creature in editor after export`.
- In `lib.rs`, make the tab switch conditional:

```rust
obj_importer_ui::ObjImporterUiSignal::Creature => {
    self.load_creatures();
    self.sync_obj_importer_campaign_state();
    if self.obj_importer_state.open_after_export {
        self.ui_state.active_tab = EditorTab::Creatures;
    }
    ui.ctx().request_repaint();
}
```

The `open_after_export` preference is preserved by `ObjImporterState::clear()`
(add it to the list of fields kept across resets, alongside `scale`).

#### 6.2 Integrate Feature

| File                                                         | Required Change                                                                                                                                   |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/creatures_editor/mod.rs`           | Add `validation_dirty: bool` field; set to `true` wherever `preview_dirty = true`; guard `refresh_validation_state` call with `validation_dirty`. |
| `sdk/campaign_builder/src/preview_renderer.rs`               | Add `last_rendered_signature: Option<u64>`; skip `render_preview` when signature unchanged and `needs_update == false`.                           |
| `sdk/campaign_builder/src/creatures_editor/preview_panel.rs` | Add `live_preview_enabled: bool` to `CreaturesEditorState`; add Live Preview toggle; auto-disable for dense models.                               |
| `sdk/campaign_builder/src/obj_importer_ui.rs`                | Replace unconditional mesh-row loop with `ScrollArea::show_rows`; add `open_after_export` checkbox.                                               |
| `sdk/campaign_builder/src/lib.rs`                            | Conditionalize Creatures tab switch.                                                                                                              |
| `sdk/campaign_builder/src/obj_importer.rs`                   | Add `open_after_export: bool` to `ObjImporterState`; preserve in `clear()`.                                                                       |

#### 6.3 Configuration Updates

No campaign data configuration changes.

Optional: persist `live_preview_enabled` and `open_after_export` in
`ToolConfig` if that struct already serializes per-session preferences.
If not, keep them as in-memory state only (they reset to defaults on restart).

#### 6.4 Testing Requirements

Required tests:

| Test Name                                                      | File                          | Assertion                                                                                                                          |
| -------------------------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `test_validation_dirty_flag_prevents_recompute_without_change` | `creatures_editor/mod.rs`     | After `refresh_validation_state`, `validation_dirty == false`; a second call does not invoke `validate_mesh`.                      |
| `test_validation_dirty_set_when_mesh_edited`                   | `creatures_editor/mod.rs`     | Mutating `edit_buffer.meshes[0]` sets `validation_dirty = true`.                                                                   |
| `test_importer_large_mesh_list_renders_bounded_rows`           | `obj_importer_ui.rs`          | With 200 meshes, only rows visible in a fixed-height egui region are rendered (requires test harness that can invoke `show_rows`). |
| `test_importer_creature_export_stays_in_importer_by_default`   | `obj_importer.rs` or `lib.rs` | `open_after_export` defaults to `false`; after export signal, `active_tab` remains `EditorTab::Importer`.                          |
| `test_importer_creature_export_switches_tab_when_enabled`      | `lib.rs`                      | `open_after_export = true`: after export signal, `active_tab == EditorTab::Creatures`.                                             |
| `test_preview_renderer_skips_render_when_signature_unchanged`  | `preview_renderer.rs`         | Calling `show` twice with identical camera and no `needs_update` calls `render_preview` only once.                                 |

#### 6.5 Deliverables

- [ ] **A**: `validation_dirty` field added; `refresh_validation_state` only called when dirty.
- [ ] **B**: `last_rendered_signature` field added to `PreviewRenderer`; preview skips expensive redraw when idle.
- [ ] **B**: `live_preview_enabled` field and Live Preview toggle added to Creature Editor preview panel.
- [ ] **B**: Dense models (> 50,000 triangles) auto-disable Live Preview.
- [ ] **C**: Importer mesh list uses `ScrollArea::show_rows` for virtualized rendering.
- [ ] **D**: `open_after_export` field added to `ObjImporterState`; tab switch in `lib.rs` conditionalized.
- [ ] **D**: `open_after_export` checkbox visible in `render_loaded_mode` export controls.
- [ ] All 6 new tests pass.
- [ ] SDK egui ID audit (`sdk/AGENTS.md` L1197–L1268) passes for all touched UI.
- [ ] Zero `cargo clippy` warnings.

#### 6.6 Success Criteria

- Creature Editor edit mode does not call `validate_mesh` every frame when no
  meshes have changed.
- Creature preview panel does not invoke `render_preview` every frame when the
  camera and mesh data are unchanged.
- Dense models can disable Live Preview to prevent continuous repaint.
- The Importer mesh list renders only visible rows for large imports.
- Creature export can complete without switching away from the Importer tab.

---

### Phase 7: Documentation, Validation, and QA

> **Scope**: Update documentation, add user-facing error messages for common
> GLB problems, and verify full regression coverage. All prior phases must be
> complete.
>
> **Depends on**: Phases 1–6 complete and all quality gates passing.

#### 7.1 Feature Work

**Documentation updates**:

| File                                        | Required Update                                                                                                                       |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/explanation/implementations.md`       | Add GLB importer feature summary: parser, state, UI dispatch, texture export, runtime compatibility, and responsiveness improvements. |
| `docs/reference/campaign_content_format.md` | Document GLB importer texture export convention (already started in Phase 5).                                                         |
| `docs/explanation/next_plans.md`            | Mark the GLB importer item as complete.                                                                                               |
| `docs/reference/architecture.md`            | Update SDK / Campaign Builder importer description if it still reads as OBJ-only.                                                     |

**User-facing error messages** (add or verify in `obj_importer_ui.rs` and
`mesh_glb_io.rs` — these should already be covered by Phase 1 parser errors,
but add UI-layer translations here for any that are not already surfaced):

| GLB Problem                     | Required User-Facing Behavior                                                                            |
| ------------------------------- | -------------------------------------------------------------------------------------------------------- |
| No mesh primitives in GLB       | Load error: `"No mesh primitives found in GLB file."`                                                    |
| Primitive missing `POSITION`    | Load error: `"Mesh '<name>' primitive <N> has no position data."`                                        |
| Unsupported primitive mode      | Load error: `"Unsupported primitive mode '<mode>' in mesh '<name>'."`                                    |
| External URI texture            | Load error: `"Embedded textures required; external URI textures are not supported."`                     |
| Embedded image unknown MIME     | Export warning shown in `status_message`; export proceeds with `.bin` extension.                         |
| Texture bytes missing at export | Export error before RON write: `"Texture for mesh '<name>' could not be resolved."`                      |
| Multiple scenes                 | Import default scene; include in `status_message`: `"GLB contains N scene(s); importing default scene."` |
| Skinning present                | Non-blocking warning in `status_message`: `"Skinning/animations present but not imported."`              |

#### 7.2 Integrate Feature

Verify error messages are displayed in the Importer tab UI through
`state.status_message` (for load-time errors) or `ObjImporterExportError`
display strings (for export-time errors).

Confirm `show_obj_importer_tab` renders `state.status_message` with an
appropriate color:

- Normal messages: `egui::RichText::new(msg).italics()`
- Error messages: `ui.colored_label(egui::Color32::RED, msg)`

Add an `is_error: bool` field to importer state to distinguish error status
from informational messages, OR prefix error messages with `"Error: "` and
detect the prefix in the UI.

#### 7.3 Configuration Updates

No game configuration changes.

#### 7.4 Testing Requirements

Full regression suite verification — confirm the following all pass:

| Test Area                                                                                         | Requirement                                                                                                     |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Parser (`mesh_glb_io.rs`)                                                                         | All 11 unit tests from Phase 1 pass.                                                                            |
| Importer state (`obj_importer.rs`)                                                                | All 5 unit tests from Phase 2 pass; all pre-existing OBJ state tests pass.                                      |
| UI dispatch (`obj_importer_ui.rs`)                                                                | All 3 tests from Phase 3 pass; all pre-existing importer UI tests pass.                                         |
| Texture export (`obj_importer_ui.rs`)                                                             | All 6 tests from Phase 4 pass.                                                                                  |
| Domain/runtime (`obj_importer_ui.rs`, domain)                                                     | All 5 tests from Phase 5 pass.                                                                                  |
| Responsiveness (`creatures_editor/mod.rs`, `preview_renderer.rs`, `obj_importer_ui.rs`, `lib.rs`) | All 6 tests from Phase 6 pass.                                                                                  |
| OBJ regression                                                                                    | All pre-existing OBJ import and export tests pass without modification.                                         |
| Quality gates                                                                                     | `cargo check`, `cargo clippy -D warnings`, `cargo nextest run` pass for workspace and `campaign_builder` crate. |

#### 7.5 Deliverables

- [ ] `docs/explanation/implementations.md` updated with GLB importer summary.
- [ ] `docs/reference/campaign_content_format.md` updated with texture export convention.
- [ ] `docs/explanation/next_plans.md` GLB item marked complete.
- [ ] `docs/reference/architecture.md` importer description updated if needed.
- [ ] All user-facing error messages implemented and tested.
- [ ] All 36 new tests (11 + 5 + 3 + 6 + 5 + 6) pass.
- [ ] All pre-existing OBJ tests pass.
- [ ] Full quality gate run exits zero errors and zero warnings.

#### 7.6 Success Criteria

- Campaign Builder users can import a `.glb`, export as Creature/Item/Furniture
  RON, and get self-contained embedded texture files in `assets/textures/imported/`
  without manually managing sidecar files.
- Textured GLB assets produce exported RON plus campaign texture files that are
  self-contained inside the campaign directory.
- OBJ import remains fully functional with zero regressions.
- Unsupported GLB features fail or warn clearly rather than producing silently
  untextured models.
- Campaign Builder remains responsive when loading and exporting large GLB models.

---

## Recommended Implementation Order

| Order | Phase                                                 | Why                                                                                          |
| ----- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 1     | Phase 1: GLB Parser Foundation                        | Establish reliable geometry/material/texture extraction before any state or UI work.         |
| 2     | Phase 2: Format-Neutral Importer State                | Wire GLB parser output into the existing importer state and mesh row model.                  |
| 3     | Phase 3: Importer UI for GLB Selection                | Expose GLB loading to users; depends on Phase 2 state support.                               |
| 4     | Phase 4: Embedded Texture Export                      | Make exported campaigns self-contained; depends on Phase 3 texture payloads being populated. |
| 5     | Phase 5: Runtime and Domain Compatibility             | Verify end-to-end export correctness before addressing performance.                          |
| 6     | Phase 6: Importer and Creature Preview Responsiveness | Address performance problems that become more visible with large GLB imports.                |
| 7     | Phase 7: Documentation, Validation, and QA            | Close all docs, user-facing errors, and regression coverage after all features are complete. |

---

## Cross-Phase Architecture Rules

1. **GLB is not runtime campaign data**: Never store `.glb` files in the campaign
   directory. Export RON plus copied texture assets only.
2. **RON format only**: Exported creature, item, and furniture model definitions
   are always RON. Never JSON or YAML.
3. **Parser isolation**: `mesh_glb_io.rs` contains only parsing logic. It does
   not import `egui` or render anything.
4. **OBJ backward compatibility**: OBJ import/export behavior must remain
   unchanged. New fields (`texture_payload`, `source_format`) must have defaults
   that preserve existing OBJ workflows.
5. **No domain changes for base GLB**: `MeshDefinition`, `MaterialDefinition`,
   and `CreatureDefinition` in `src/domain/visual/mod.rs` must not be modified
   for base GLB support. Existing fields are sufficient.
6. **Test fixture location**: All test fixtures are under `data/test_fixtures/`
   or generated in temp directories. Never reference `campaigns/tutorial` from
   tests.
7. **SDK egui ID compliance**: Every Importer UI change must pass the
   `sdk/AGENTS.md` L1197–L1268 egui ID audit checklist before the phase is
   marked complete.
8. **SPDX headers**: Every new `.rs` file begins with:
   ```
   // SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
   // SPDX-License-Identifier: Apache-2.0
   ```
9. **No `unwrap()` without justification**: Use `?`, `thiserror`, and
   descriptive error messages throughout.

---

## Quality Gates For Each Implementation Phase

Run these commands **after completing each phase** before starting the next:

```
# 1. Format
cargo fmt --all

# 2. Compile check (entire workspace)
cargo check --all-targets --all-features

# 3. Compile check (campaign_builder only, faster)
cargo check -p campaign_builder --all-targets --all-features

# 4. Lint (entire workspace — zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 5. Lint (campaign_builder only)
cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings

# 6. Tests (entire workspace)
cargo nextest run --all-features

# 7. Tests (campaign_builder only, faster)
cargo nextest run -p campaign_builder --all-features
```

Each phase is complete only when **all** applicable gates pass with zero errors
and zero warnings.
