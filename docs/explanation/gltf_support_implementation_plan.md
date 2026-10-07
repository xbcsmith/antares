# GLTF 2.0 Support Implementation Plan

## Overview

Add native GLTF 2.0 / GLB support to the Antares engine and Campaign Builder SDK
in five sequential phases. The goal is to replace the current RON-baked vertex-data
pipeline for imported 3D assets with Bevy's built-in GLTF loader, reducing asset
file sizes by 8–10×, enabling full PBR material maps (normal, AO,
metallic/roughness), and simplifying the artist workflow to a direct
Blender → GLB export path.

The existing procedural mesh system (`procedural_meshes.rs`: trees, furniture,
portals, items) is **out of scope** — it generates geometry in code and will
continue to do so regardless of this feature.

**Current problem in numbers:**

| File                               | Lines | Size       |
| ---------------------------------- | ----- | ---------- |
| `assets/meshes/brush.ron`          | 796 K | **32 MB**  |
| `assets/meshes/oak_tree_short.ron` | 56 K  | 2.1 MB     |
| `assets/meshes/pine_tree.ron`      | 52 K  | 2.0 MB     |
| `assets/meshes/palm_tree.ron`      | 56 K  | 2.1 MB     |
| `assets/meshes/dead_tree.ron`      | 45 K  | 1.7 MB     |
| **Total (5 mesh files)**           |       | **~40 MB** |

The same meshes as binary GLB would total approximately 4–5 MB — an 8–10× reduction.

---

## Current State Analysis

### Existing Infrastructure

- **SDK GLB reader** — `sdk/campaign_builder/src/mesh_glb_io.rs` already parses
  GLB files using `gltf = "1"` and produces `MeshDefinition` values. It reads
  positions, normals, UVs, indices, base-color materials, and embedded textures.
  It currently converts GLB → RON (lossy; discards normal/AO/metallic-roughness
  texture maps).

- **RON mesh pipeline** — `src/domain/visual/mod.rs::MeshDefinition` (raw vertex
  store) → `src/game/systems/creature_meshes.rs::mesh_definition_to_bevy()` →
  Bevy `Mesh`. Used by `spawn_imported_landscape_mesh()`,
  `spawn_imported_furniture_mesh()`, and `spawn_creature()` in
  `src/game/systems/map.rs` and `src/game/systems/creature_spawning.rs`.

- **Bevy GLTF support** — `bevy = { version = "0.19", default-features = true }`
  in `Cargo.toml` includes `GltfPlugin` and `SceneRoot`-based async scene
  loading. No new Cargo dependencies are needed for any phase.

- **`MaterialDefinition`** — defined in `src/domain/visual/mod.rs`; carries
  `base_color`, `metallic`, `roughness`, `emissive`, `alpha_mode`. No texture
  map fields (`normal_map_texture`, `occlusion_texture`,
  `metallic_roughness_texture` are all absent).

- **Spawning pattern** — `spawn_imported_landscape_mesh` /
  `spawn_imported_furniture_mesh` in `src/game/systems/map.rs` spawn a root
  entity with `(Name, Transform, GlobalTransform, Visibility, InheritedVisibility,
ViewVisibility, MapEntity, TileCoord, …)` then loop over
  `creature_def.meshes` spawning one child entity per `MeshDefinition`.

### Identified Issues

1. **RON text inflation** — each `[f32; 3]` vertex takes ~45 bytes as RON text
   vs 12 bytes as a binary float buffer. 32 MB for a bush mesh is impractical
   for version control and load-time performance.

2. **Normal/AO/PBR maps are discarded** — `mesh_glb_io.rs::convert_gltf_material()`
   maps base-color only; all other PBR texture channels are silently dropped.
   `has_unsupported_pbr_channels` is set to `true` but nothing acts on it.

3. **No runtime GLB path** — `CreatureDefinition` has no `glb_path` field; the
   engine has no code path that calls `AssetServer::load()` on a `.glb` file.

4. **SDK has no "Raw GLB" export mode** — every GLB import is converted to RON,
   making a high-poly import produce an even bigger RON file.

5. **`material_definition_to_bevy()`** only sets `base_color_texture` on
   `StandardMaterial`; `normal_map_texture`, `occlusion_texture`, and
   `metallic_roughness_texture` are never set.

---

## Implementation Phases

### Phase 1: Domain & Data Model Foundation

Extend the domain layer to carry an optional GLB path reference. No rendering
changes. Zero breaking changes to existing RON data files (all new fields use
`#[serde(default)]`).

#### 1.1 Extend `CreatureDefinition`

File: `src/domain/visual/mod.rs`

Add two fields to the `CreatureDefinition` struct (defined at L410–L433):

- `glb_path: Option<String>` — campaign-relative path; **must** start with
  `"assets/"` (same convention as `MeshDefinition::texture_path`). Used by
  the runtime in Phase 3. Annotate with `#[serde(default)]`.
- `glb_scene_index: usize` — which GLTF scene within the file to load. Default
  `0`. Allows multi-scene GLB files. Annotate with
  `#[serde(default)]`.

Both fields use `#[serde(default)]`; existing RON data deserializes without
modification.

Update `impl Default for CreatureDefinition` (L435–L446) to include:
`glb_path: None, glb_scene_index: 0`.

**CRITICAL — Update `CreatureDefinition::validate()`** (L490–L523): the current
implementation immediately returns `Err(ValidationError::EmptyField(...))` when
`self.meshes.is_empty()`. This must be changed so that a GLB-only entry is valid:

- When `self.glb_path.is_some()` AND `self.meshes.is_empty()`: skip the
  empty-mesh check and skip the `mesh_transforms` length check. A GLB-only
  definition has no inline vertex data.
- When `self.glb_path.is_none()` AND `self.meshes.is_empty()`: keep the
  existing `Err(ValidationError::EmptyField(...))`.
- When `self.glb_path.is_some()` AND `!self.meshes.is_empty()`: return
  `Err(ValidationError::InvalidValue("CreatureDefinition has both glb_path and inline meshes; use one or the other".to_string()))`.

Update the `/// # Examples` doc comment on `CreatureDefinition::validate()` (L461–L488)
to show a GLB-path variant alongside the existing RON-mesh example. Existing
doctests must compile; add `glb_path: None, glb_scene_index: 0` to every
struct literal in existing doctests that constructs a `CreatureDefinition`.

#### 1.2 Update `CreatureDatabase` validation

File: `src/domain/visual/creature_database.rs`

The `CreatureDatabaseError` enum (L56–L76) currently has five variants.
Add two new variants:

- `AmbiguousMeshSource(CreatureId)` — creature has both `glb_path` set and
  non-empty `meshes`.
- `GlbPathError(CreatureId, String)` — `glb_path` value failed path-security
  validation.

Update `CreatureDatabase::validate()` (L666–L673) to enforce per-creature:

- **Valid**: `glb_path: Some(_)` with `meshes: []` — a GLB-only definition.
- **Valid**: `glb_path: None` with `meshes: [...]` — existing RON-mesh definition.
- **Error**: `glb_path: Some(_)` with non-empty `meshes` → `AmbiguousMeshSource(id)`.
- **Error**: `glb_path` present but does not start with `"assets/"` → call
  `validate_campaign_relative_path()` from `src/domain/path_security.rs` and
  wrap the result as `GlbPathError(id, message)`.

Note: `CreatureDefinition::validate()` (updated in §1.1) also catches the
ambiguous-source case. The `CreatureDatabase::validate()` layer adds the same
check at the database level so loading a registry file with a bad entry produces
a `CreatureDatabaseError` rather than a `ValidationError`.

#### 1.3 Update `ObjectMeshDatabase` and `ItemMeshDatabase`

File: `src/domain/world/object_mesh.rs`

`LandscapeMeshDatabase` and `FurnitureMeshDatabase` are both loaded and then
merged into `ContentDatabase::object_meshes` (an `ObjectMeshDatabase`) at
campaign load time. All rendering code in `map.rs` and `events.rs` calls
`object_meshes.lookup(key)` — **`LandscapeMeshDatabase` is never queried
directly by the rendering layer**. No change is needed to `LandscapeMeshDatabase`
or `FurnitureMeshDatabase`.

Because `ObjectMeshDatabase::lookup()` already returns `&CreatureDefinition`,
rendering code can check `creature_def.glb_path.is_some()` inline with zero
additional API surface. No helper method is required.

The one change needed: add a convenience iterator over all GLB-path entries
for use by the cache pre-loader in Phase 3 §3.2:

```rust
pub fn glb_paths(&self) -> impl Iterator<Item = &str>
```

Returns the `glb_path` string from every entry whose `CreatureDefinition` has
`glb_path.is_some()`, deduplicated. This drives `GlbHandleCache` population
at campaign load time without requiring callers to iterate the full entry list
manually.

File: `src/domain/items/database.rs`

`ItemMeshDatabase` wraps an inner `CreatureDatabase` (field: `inner: CreatureDatabase`)
and exposes it via `as_creature_database()`. Add the same iterator:

```rust
pub fn glb_paths(&self) -> impl Iterator<Item = &str>
```

Implement by delegating to `self.inner.all_creatures()` and filtering/mapping
the same way as `ObjectMeshDatabase::glb_paths()`.

#### 1.4 Add test fixtures to `data/test_campaign`

- Create directory `data/test_campaign/assets/meshes/creatures/`.
- Create directory `data/test_campaign/assets/meshes/items/`.
- Create directory `data/test_campaign/assets/meshes/furniture/`.
- Add a minimal synthetic GLB (a single-triangle mesh) to
  `data/test_campaign/assets/meshes/test_triangle.glb`. The binary can be
  produced by adapting the existing `build_glb()` helper in
  `mesh_glb_io.rs` tests into a standalone fixture generator or by writing
  the minimal GLB binary directly in Rust test helper code.
- Add one fixture entry with `glb_path: Some("assets/meshes/test_triangle.glb")`
  and `meshes: []` to each of the following registries:
  - `data/test_campaign/data/landscape_mesh_registry.ron`
  - `data/test_campaign/data/creatures.ron`
  - `data/test_campaign/data/item_mesh_registry.ron`
  - `data/test_campaign/data/object_mesh_registry.ron`

Do **not** reference `campaigns/tutorial` from any test (per Implementation Rule 5).

#### 1.5 Testing Requirements

New tests in `src/domain/visual/mod.rs` (under the existing `mod tests`):

- `test_creature_definition_glb_path_only_is_valid` — construct
  `CreatureDefinition { glb_path: Some("assets/meshes/test.glb".to_string()), meshes: vec![], ... }`,
  call `validate()`, expect `Ok(())`.
- `test_creature_definition_glb_and_meshes_both_set_returns_error` — set both
  `glb_path` and non-empty `meshes`, expect
  `Err(ValidationError::InvalidValue(_))`.
- `test_creature_definition_no_glb_no_meshes_returns_error` — `glb_path: None`,
  `meshes: []`, expect `Err(ValidationError::EmptyField(_))` (existing
  behaviour preserved).

New tests in `src/domain/visual/creature_database.rs`:

- `test_creature_database_glb_path_only_is_valid` — database with one
  GLB-only entry; `validate()` returns `Ok`.
- `test_creature_database_glb_and_meshes_both_set_returns_ambiguous_error` —
  expect `Err(AmbiguousMeshSource(_))`.
- `test_creature_database_glb_path_must_start_with_assets_prefix` — path
  `"meshes/foo.glb"` (no `assets/` prefix), expect `Err(GlbPathError(_, _))`.

New tests in `src/domain/world/object_mesh.rs`:

- `test_object_mesh_database_glb_paths_returns_glb_entries` — database with one
  GLB entry and one RON entry; assert `glb_paths()` yields exactly the GLB path.
- `test_object_mesh_database_glb_paths_empty_for_all_ron` — all-RON database;
  assert iterator is empty.

New test in `src/domain/items/database.rs`:

- `test_item_mesh_database_glb_paths_returns_glb_entries` — same shape as the
  `ObjectMeshDatabase` test.

Run all four quality gates after implementation:
`cargo fmt --all`, `cargo check --all-targets --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`cargo nextest run --all-features`.

#### 1.6 Deliverables

- [ ] `glb_path: Option<String>` and `glb_scene_index: usize` fields on
      `CreatureDefinition` with `#[serde(default)]`
- [ ] `Default` impl for `CreatureDefinition` updated with `glb_path: None, glb_scene_index: 0`
- [ ] `CreatureDefinition::validate()` updated: allows `glb_path: Some` + empty
      meshes, rejects `glb_path: Some` + non-empty meshes, rejects no-glb + empty
      meshes (existing behaviour)
- [ ] Existing doctests on `CreatureDefinition` updated to include the two new fields
- [ ] `AmbiguousMeshSource` and `GlbPathError` variants in `CreatureDatabaseError`
- [ ] Updated `CreatureDatabase::validate()` enforcing GLB-path rules
- [ ] `glb_paths()` iterator on `ObjectMeshDatabase` (`src/domain/world/object_mesh.rs`)
- [ ] `glb_paths()` iterator on `ItemMeshDatabase` (`src/domain/items/database.rs`)
- [ ] Minimal GLB fixture at `data/test_campaign/assets/meshes/test_triangle.glb`
- [ ] GLB-path fixture entries in all four `data/test_campaign/data/` registries
- [ ] All new `data/test_campaign/assets/meshes/` subdirectories created
- [ ] All nine new tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 1.7 Success Criteria

All existing tests pass without modification. New `glb_path` entries in RON data
files deserialize and validate correctly. `CreatureDefinition::validate()` permits
GLB-only entries and rejects ambiguous entries. `cargo clippy -- -D warnings`
reports zero new issues. No test references `campaigns/tutorial`.

---

### Phase 2: SDK Raw GLB Export Mode

Enable the Campaign Builder importer to copy a `.glb` file directly into the
campaign asset tree and write a registry entry with `glb_path` set, instead of
converting the geometry to a RON `MeshDefinition`.

#### 2.1 Add `GlbExportMode` to the importer state

File: `sdk/campaign_builder/src/obj_importer.rs`

Add a new public enum **before** `ObjImporterState`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlbExportMode {
    #[default]
    ConvertToRon,   // existing behaviour — bake MeshDefinition into RON
    RawGlb,         // new — copy source .glb; write glb_path in registry entry
}
```

Add field `pub glb_export_mode: GlbExportMode` to `ObjImporterState` (defined at
L196–L252). The `#[default]`-derived value is `ConvertToRon` so all existing
behaviour is preserved. `RawGlb` is only offered in the UI when
`source_format == ImportSourceFormat::Glb`; OBJ sources have no Raw mode.

Update `impl Default for ObjImporterState` (L484–L511) to include
`glb_export_mode: GlbExportMode::ConvertToRon`.

#### 2.2 Implement the Raw GLB export path

File: `sdk/campaign_builder/src/obj_importer.rs` (export/save phase)

When `glb_export_mode == GlbExportMode::RawGlb`, handle all five `ExportType`
variants:

| `ExportType` | Destination path                                                |
| ------------ | --------------------------------------------------------------- |
| `Creature`   | `{campaign_root}/assets/meshes/creatures/{name}.glb`            |
| `Item`       | `{campaign_root}/assets/meshes/items/{name}.glb`                |
| `Furniture`  | `{campaign_root}/assets/meshes/furniture/{name}.glb`            |
| `Landscape`  | `{campaign_root}/assets/meshes/landscape/{category}/{name}.glb` |
| `ObjectMesh` | `{campaign_root}/assets/meshes/objects/{name}.glb`              |

For each:

1. Copy the source `.glb` file to the destination path.
2. Write the registry entry (`creatures.ron`, `item_mesh_registry.ron`,
   `furniture_mesh_registry.ron`, `landscape_mesh_registry.ron`, or
   `object_mesh_registry.ron`) with:
   - `glb_path: Some("assets/meshes/…/{name}.glb")` (campaign-relative path)
   - `meshes: []`
   - `id`, `name`, `scale`, `color_tint`, and other metadata populated as in the
     existing RON export path.
3. Do **not** call `ground_meshes_to_y_zero()` or serialize any `MeshDefinition`
   vertex data.

When `glb_export_mode == ConvertToRon`, the existing code path runs unchanged.

#### 2.3 Update the importer UI

File: `sdk/campaign_builder/src/obj_importer_ui.rs`

Add a radio button group labelled "Export Format" below the Export Type selector.
Only show the group when `source_format == ImportSourceFormat::Glb`. Options:
`○ RON Mesh (convert geometry)` / `● Raw GLB (copy file, preserve full PBR)`.

Follow Implementation Rule 6 for any column layout changes in the importer tab.

#### 2.4 Verify packager covers `.glb` files

File: `src/sdk/campaign_packager.rs`

The `CampaignPackager::add_directory_to_archive()` method (L463–L511) uses
`fs::read_dir()` recursively and adds **every** file it encounters. It does not
filter by file extension; the only exclusions are hidden files (names starting
with `.`), the `target` directory, and `node_modules`. Therefore `.glb` files
placed under `assets/meshes/` are already included in campaign packages with
**no code change required**.

Confirm this behaviour by reading `add_directory_to_archive()` and verifying no
extension-based exclusion list exists. No code change is expected; if one is
found, remove the extension filter so `.glb` files pass through.

#### 2.5 Campaign Builder GLB Preview

Files: `sdk/campaign_builder/src/creatures_editor/preview_panel.rs`,
`sdk/campaign_builder/src/landscape_editor.rs`

When an entry uses Raw GLB export (`meshes: []`, `glb_path: Some(_)`), the
existing preview renderer receives an empty mesh list and shows nothing. Fix this
so the preview reflects the actual imported geometry.

**Approach** — reuse the existing `mesh_glb_io` parser rather than building a
new GLTF renderer. The `gltf` crate is already a dependency in
`sdk/campaign_builder/Cargo.toml`.

1. In `preview_panel.rs`, detect when the creature/landscape entry has
   `glb_path.is_some()` and `meshes.is_empty()`.
2. Call `import_glb_scene_from_file(path, GlbImportOptions::default())` to
   obtain an `ImportedGlbScene`.
3. Collect the `mesh_def` from each `ImportedGlbMesh` in the scene and pass
   them to the existing software preview renderer in place of the empty
   `meshes` vec. The preview renderer already knows how to draw a
   `MeshDefinition`; no renderer changes are needed.
4. Cache the parsed preview geometry in the editor state (key: `glb_path`
   string) so the file is not re-parsed on every repaint.
5. Apply the same preview update to the landscape importer tab in
   `landscape_editor.rs` wherever it calls into the preview renderer.

**Error handling** — if `import_glb_scene_from_file` fails (file not found,
parse error), display the error string in the preview panel area instead of
a blank canvas. Do not propagate the error to the export pipeline.

#### 2.6 Testing Requirements

New tests in `sdk/campaign_builder/src/obj_importer.rs` (or
`sdk/campaign_builder/tests/`):

- `test_glb_export_mode_default_is_convert_to_ron` — `ObjImporterState::default()`
  has `glb_export_mode == GlbExportMode::ConvertToRon`.
- `test_raw_glb_export_copies_file_to_campaign_assets` — given a temp directory
  as campaign root and a source `.glb`, trigger a Raw GLB landscape export and
  assert the file exists at the expected destination path.
- `test_raw_glb_export_registry_entry_has_glb_path_and_empty_meshes` — assert
  the written RON registry entry has `glb_path: Some(…)` and `meshes: []`.
- `test_raw_glb_export_not_available_for_obj_source` — `source_format == Obj`,
  `glb_export_mode` is forced back to `ConvertToRon` on export.
  Test covers all five `ExportType` variants.
- `test_preview_panel_glb_path_returns_imported_geometry` — call the preview
  update path with a GLB-only entry pointing to `test_triangle.glb`; assert
  the preview mesh list is non-empty.
- `test_preview_panel_glb_missing_file_displays_error_string` — missing path;
  assert the preview state contains an error message string, not a panic.

Run `cargo nextest run --all-features` in the SDK workspace.

#### 2.7 Deliverables

- [ ] `GlbExportMode` enum in `obj_importer.rs` with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]`
- [ ] Raw GLB export branch covering all five `ExportType` variants
- [ ] "Export Format" radio group in `obj_importer_ui.rs` (GLB source only)
- [ ] Packager behaviour confirmed: `assets/meshes/**/*.glb` is included by the existing recursive walk in `src/sdk/campaign_packager.rs`
- [ ] GLB preview geometry in `preview_panel.rs` using `import_glb_scene_from_file`
- [ ] Cached preview geometry keyed by `glb_path` in editor state
- [ ] Error display for missing/invalid GLB paths in preview panel
- [ ] All six new tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 2.8 Success Criteria

Importing a `.glb` file with Raw GLB mode produces a `.glb` under campaign
assets and a registry entry with `glb_path` set and `meshes: []`. The preview
panel shows the actual imported mesh geometry for GLB-only entries. Existing OBJ
and RON-conversion import paths pass unchanged. `cargo clippy -- -D warnings`
reports zero issues in both crates.

---

### Phase 3: Runtime GLB Rendering

Add a GLB-backed rendering path to the engine. When a `CreatureDefinition` or
landscape mesh registry entry has `glb_path` set, spawn the entity using Bevy's
native `SceneRoot` + `AssetServer` instead of the `mesh_definition_to_bevy()`
RON path. Both paths coexist; the engine selects at spawn time.

#### 3.1 Add `GlbHandleCache` resource

New file: `src/game/resources/glb_assets.rs`

All new `.rs` files must begin with the SPDX copyright/license header
(Implementation Rule 1):

```
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

Define the resource:

```rust
#[derive(Resource, Default)]
pub struct GlbHandleCache {
    /// Maps campaign-relative GLB path → Bevy `Handle<Scene>`.
    ///
    /// Keys are the raw `glb_path` string from `CreatureDefinition`
    /// (e.g. `"assets/meshes/creatures/oak_tree.glb"`). The Handle is
    /// loaded using the Bevy scene-fragment syntax:
    /// `asset_server.load(format!("{}#Scene{}", path, scene_index))`.
    pub scenes: HashMap<String, Handle<Scene>>,
}
```

Add `pub mod glb_assets;` to `src/game/resources/mod.rs` and
`pub use glb_assets::GlbHandleCache;` in the same file's re-export block.

The cache is populated during game startup (see §3.2) so assets are warm before
any map spawns.

#### 3.2 New system: `glb_scene_loader_system`

New file: `src/game/systems/glb_scene_loader.rs`

Add SPDX header (see §3.1 for format).

Add `pub mod glb_scene_loader;` to `src/game/systems/mod.rs`.

A startup system that pre-loads every GLB-backed asset into `GlbHandleCache` so
handles are warm before any map spawns. It iterates all three databases that can
contain GLB-path entries:

1. `ContentDatabase::creatures` — combat and world creature visuals.
2. `ContentDatabase::object_meshes` — the unified runtime database that
   already contains merged landscape and furniture mesh entries; use
   `object_meshes.glb_paths()` (added in Phase 1 §1.3).
3. `ContentDatabase::item_meshes` — dropped item and in-world item visuals;
   use `item_meshes.glb_paths()`.

For each unique `(glb_path, glb_scene_index)` pair across all three sources,
construct the Bevy scene-fragment asset key:

```rust
let asset_key = format!("{}#Scene{}", glb_path, scene_index);
let handle: Handle<Scene> = asset_server.load(asset_key);
cache.scenes.insert(glb_path.to_string(), handle);
```

Deduplicate paths before issuing load calls — the same GLB may appear in
multiple databases. Note: Bevy 0.19 requires the `#SceneN` fragment to obtain a
`Handle<Scene>`; loading without the fragment yields a `Handle<Gltf>`, which
cannot be used with `SceneRoot`.

Register the system as a `Startup` system in `src/bin/antares.rs`:

```rust
app.add_systems(Startup, antares::game::systems::glb_scene_loader::glb_scene_loader_system);
```

The game does not use a `States`-based `AppState` enum; the `Startup` schedule
runs after `AntaresPlugin::build()` inserts the `GameContent` resource, which is
the correct ordering for this system. Register `GlbHandleCache` as an
`init_resource` before the system runs:

```rust
app.init_resource::<antares::game::resources::GlbHandleCache>();
```

#### 3.3 GLB branch in `spawn_creature()`

File: `src/game/systems/creature_spawning.rs`

**Signature change** — add `glb_cache: Option<&GlbHandleCache>` as a new
parameter to `spawn_creature()`. All existing callers pass `None`. This is the
only public API change required; the GLB path is fully encapsulated in
`spawn_creature_glb()`.

Updated signature:

```rust
pub fn spawn_creature(
    commands: &mut Commands,
    creature_def: &CreatureDefinition,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    scale_override: Option<f32>,
    animation: Option<AnimationDefinition>,
    facing: Option<Direction>,
    glb_cache: Option<&GlbHandleCache>,
) -> Entity
```

Update all existing callers of `spawn_creature()` to pass `None` as the final
argument. Known call sites:

- `src/game/systems/item_world_events.rs::spawn_dropped_item_system()` (L434)
- `src/game/systems/map.rs` — wherever creature visuals are spawned for NPCs,
  encounter markers, and recruitable characters
- Any other location identified by `grep -rn "spawn_creature(" src/`

At the top of `spawn_creature()`, add a branch:

```
if creature_def.glb_path.is_some() {
    return spawn_creature_glb(commands, creature_def, position, scale_override, animation, facing, glb_cache);
}
// existing RON path unchanged below
```

New private function `spawn_creature_glb()`:

- Requires `glb_cache: Option<&GlbHandleCache>`. If `None`, log a warning and
  fall through to the RON path (graceful degradation).
- Look up `Handle<Scene>` from `GlbHandleCache::scenes` using
  `creature_def.glb_path.as_deref().unwrap()` as the key.
- If the handle is not in the cache, log a warning and fall through to the RON
  path (the pre-load in §3.2 should prevent this in normal operation, but the
  GLB-only definition has no `meshes` to fall back to — in this edge case, spawn
  a zero-vertex placeholder entity and log an error).
- Spawn the root entity:

  ```rust
  commands.spawn((
      CreatureVisual { creature_id: 0, scale_override },
      FacingComponent::new(effective_direction),
      Transform::from_translation(position)
          .with_scale(Vec3::splat(effective_scale)),
      GlobalTransform::default(),
      Visibility::default(),
      InheritedVisibility::default(),
      ViewVisibility::default(),
      SceneRoot(handle),
  ))
  ```

- If `animation: Some(anim_def)`, also insert `CreatureAnimation::new(anim_def)`
  on the root entity (matches existing RON path behaviour at
  `creature_spawning.rs` L160–L162).
- Return the root entity ID. Bevy's `SceneSpawner` attaches child entities
  automatically when the asset finishes loading.

#### 3.4 GLB branch in all `map.rs` mesh-spawning functions

File: `src/game/systems/map.rs`

Three private functions in `map.rs` obtain a `&CreatureDefinition` from
`ObjectMeshDatabase::lookup()` and then loop over `creature_def.meshes`.
All three need the same GLB branch:

| Function                          | Triggered by                   | Game components on root entity                              |
| --------------------------------- | ------------------------------ | ----------------------------------------------------------- |
| `spawn_imported_landscape_mesh()` | `LandscapeDefinition::mesh_id` | `MapEntity`, `TileCoord`, `LandscapeEntity`                 |
| `spawn_imported_furniture_mesh()` | `FurnitureDefinition::mesh_id` | `MapEntity`, `TileCoord`, `FurnitureEntity`, `Interactable` |
| `spawn_event_meshes()`            | `MapEvent::*::mesh_id`         | `MapEntity`, `TileCoord`, `EventMeshMarker`                 |

These are private functions called from `spawn_map()`. They already receive
`asset_server: &AssetServer` as a parameter (e.g., `spawn_imported_landscape_mesh`
at L1553–L1564). Add `glb_cache: &GlbHandleCache` as an additional parameter to
each of the three functions and thread it through from `spawn_map()`, which
already has access to `GlbHandleCache` as a `Res<GlbHandleCache>` system parameter.

For each function, add an early branch **before** the `for mesh_def in creature_def.meshes`
loop:

- When `creature_def.glb_path.is_some()`:
  - Look up `Handle<Scene>` from `glb_cache.scenes`.
  - If found: spawn the root entity with the existing game components
    (`MapEntity`, `TileCoord`, and the type-specific marker) plus
    `SceneRoot(handle)`. Apply position, rotation, and scale via `Transform`.
  - If not found: log a warning and continue to the mesh loop as fallback
    (mesh loop produces nothing for a GLB-only definition but avoids panic).
  - `return root_entity_id;` after spawning.
- When `glb_path.is_none()`: existing mesh loop runs unchanged.

#### 3.5 GLB branch in `spawn_dropped_item_system()`

File: `src/game/systems/item_world_events.rs`

`spawn_dropped_item_system()` looks up a `CreatureDefinition` from
`ContentDatabase::item_meshes` when `item.mesh_id` is set and then calls
`spawn_creature()` with it. Because Phase 3 §3.3 adds a GLB branch to
`spawn_creature()`, dropped items will automatically render via `SceneRoot`
once their `CreatureDefinition` has `glb_path` set.

`spawn_dropped_item_system()` must pass the `GlbHandleCache` to `spawn_creature()`
via the new `glb_cache` parameter. Add `Res<GlbHandleCache>` to the system's
parameter list and pass `Some(&glb_cache)` to the `spawn_creature()` call at L434.

The `item_min_z` floor-clearance calculation (L406–L411) iterates
`creature_def.meshes` to find the lowest vertex. Guard it:

- When `creature_def.glb_path.is_some()` and `meshes` is empty: skip the
  `item_min_z` fold entirely. Use `effective_floor_clearance.max(DROPPED_ITEM_MIN_HEIGHT)`
  as the spawn Y (the GLB's own geometry is not yet available at spawn time since
  Bevy loads it asynchronously).

#### 3.6 Asset path resolution

GLTF paths passed to `AssetServer::load::<Scene>()` must resolve relative to
`BEVY_ASSET_ROOT`. Confirm that at campaign load time `BEVY_ASSET_ROOT` is set
to the campaign root directory (the same convention enforced for
`MeshDefinition::texture_path`). If it is not yet set for GLB paths, add the
path normalisation logic in `glb_scene_loader_system` before calling
`asset_server.load()`.

Reminder: the asset key format is `"assets/meshes/creatures/foo.glb#Scene0"` —
the scene fragment is mandatory for `Handle<Scene>` as opposed to `Handle<Gltf>`.

#### 3.7 Add integration test fixtures and tests

In `data/test_campaign/assets/meshes/`, the `test_triangle.glb` from Phase 1
serves as the fixture.

New tests (can use a headless Bevy `App` with `MinimalPlugins` +
`AssetPlugin` + `GltfPlugin`):

- `test_spawn_creature_with_glb_path_uses_scene_root` — creature with
  `glb_path: Some(…)` and pre-populated cache; assert spawned entity has
  `SceneRoot` component, assert entity does **not** have `Mesh3d` directly.
- `test_spawn_creature_without_glb_path_uses_mesh_bundle` — creature with
  `glb_path: None`; assert spawned child entities have `Mesh3d`.
- `test_glb_scene_loader_populates_cache_from_all_databases` — after running
  `glb_scene_loader_system` with a `ContentDatabase` containing GLB entries in
  `object_meshes`, `item_meshes`, and `creatures`, assert `GlbHandleCache`
  contains handles for all three paths.
- `test_spawn_dropped_item_glb_uses_floor_clearance_height` — item with
  `mesh_id` pointing to a GLB-only `CreatureDefinition`; assert spawn Y equals
  `effective_floor_clearance.max(DROPPED_ITEM_MIN_HEIGHT)` (no `meshes` to
  compute floor clearance from).

#### 3.8 Testing Requirements

- All existing `creature_spawning.rs` tests must pass unchanged (the new
  `glb_cache: None` argument is backward-compatible).
- All existing `map.rs` landscape/furniture rendering tests must pass unchanged.
- All existing `item_world_events.rs` dropped-item tests must pass unchanged.
- Four new integration tests listed in §3.7.
- Full quality gate: `cargo fmt --all`, `cargo check`, `cargo clippy -- -D warnings`,
  `cargo nextest run --all-features`.

#### 3.9 Deliverables

- [ ] `GlbHandleCache` resource in `src/game/resources/glb_assets.rs` with SPDX header
- [ ] `pub mod glb_assets;` and `pub use glb_assets::GlbHandleCache;` in
      `src/game/resources/mod.rs`
- [ ] `glb_scene_loader_system` in `src/game/systems/glb_scene_loader.rs` with SPDX header,
      iterating `creatures`, `object_meshes`, and `item_meshes`; using
      `format!("{}#Scene{}", path, index)` for scene fragment keys
- [ ] `pub mod glb_scene_loader;` in `src/game/systems/mod.rs`
- [ ] `GlbHandleCache` initialised via `app.init_resource::<GlbHandleCache>()` in
      `src/bin/antares.rs`
- [ ] `glb_scene_loader_system` registered as `Startup` system in `src/bin/antares.rs`
- [ ] `spawn_creature()` signature updated with `glb_cache: Option<&GlbHandleCache>`
      parameter; all existing callers updated to pass `None`
- [ ] GLB branch in `spawn_creature()` delegating to `spawn_creature_glb()`
- [ ] `spawn_creature_glb()` private function with `CreatureAnimation` support
- [ ] GLB branches in `spawn_imported_landscape_mesh()`,
      `spawn_imported_furniture_mesh()`, and `spawn_event_meshes()` (`map.rs`)
- [ ] `GlbHandleCache` parameter threaded through all three `map.rs` spawning functions
      from `spawn_map()` system
- [ ] `item_min_z` guard in `spawn_dropped_item_system()` (`item_world_events.rs`)
- [ ] `Res<GlbHandleCache>` added to `spawn_dropped_item_system()` parameter list
- [ ] Four new integration tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 3.10 Success Criteria

A `CreatureDefinition` with `glb_path` set renders correctly via `SceneRoot`
when spawned as a creature, landscape object, furniture piece, map-event mesh,
or dropped item. Existing RON-backed assets render identically to pre-phase
behaviour. `cargo clippy -- -D warnings` is clean. Startup time does not regress
measurably for campaigns that contain no GLB entries.

---

### Phase 4: Full PBR Material Pipeline

Extend the RON mesh conversion path to carry normal maps, AO, and PBR texture
maps that GLB files contain and the SDK currently discards. This phase benefits
content that still uses the RON path during the migration period. Entities using
the native GLB path from Phase 3 already receive all PBR maps automatically
through Bevy's GLTF loader — they do not need this phase to get normal maps.

#### 4.1 Extend `MaterialDefinition`

File: `src/domain/visual/mod.rs`

Add four texture path fields to `MaterialDefinition` (defined at L221–L242),
all with `#[serde(default)]`:

- `normal_map_path: Option<String>` — tangent-space normal map. Path must start
  with `"assets/"`.
- `occlusion_map_path: Option<String>` — ambient-occlusion texture.
- `metallic_roughness_map_path: Option<String>` — combined metallic (B) /
  roughness (G) texture (GLTF standard layout).
- `emissive_map_path: Option<String>` — emissive texture.

Backward compatible: all existing RON `MaterialDefinition` blocks deserialise
without changes.

Update validation in `src/domain/visual/mesh_validation.rs` to apply
`validate_campaign_relative_path()` to each new `Option<String>` when `Some`.

#### 4.2 Update `material_definition_to_bevy()`

File: `src/game/systems/creature_meshes.rs`

Add `asset_server: &AssetServer` parameter to `material_definition_to_bevy()`
(currently defined at L308–L336).

Updated signature:

```rust
pub fn material_definition_to_bevy(
    material_def: &MaterialDefinition,
    asset_server: &AssetServer,
) -> StandardMaterial
```

Update the returned `StandardMaterial` to conditionally set:

- `normal_map_texture: material_def.normal_map_path.as_ref().map(|p| asset_server.load(p))`
- `occlusion_texture: material_def.occlusion_map_path.as_ref().map(|p| asset_server.load(p))`
- `metallic_roughness_texture: material_def.metallic_roughness_map_path.as_ref().map(|p| asset_server.load(p))`
- `emissive_texture: material_def.emissive_map_path.as_ref().map(|p| asset_server.load(p))`

**Update all call sites** of `material_definition_to_bevy()`. Known call sites:

1. `src/game/systems/creature_spawning.rs` L174 — inside the `for mesh_def in creature_def.meshes` loop. The `AssetServer` is already available in `spawn_creature()`'s context (or can be passed as a parameter).
2. `src/game/systems/map.rs::landscape_material()` (L1768–L1814) — the private
   helper that wraps `material_definition_to_bevy()`. Add `asset_server: &AssetServer`
   to `landscape_material()`'s signature and pass it through. `landscape_material()`
   is called from `spawn_imported_landscape_mesh()`, which already receives
   `asset_server: &AssetServer` at L1557.

Run `grep -rn "material_definition_to_bevy" src/` to discover any additional
call sites before implementation.

Update the doctest on `material_definition_to_bevy()` (L293–L307) to show the
new `asset_server` parameter. Since doctests are compiled, the example must
either use a real `AssetServer` in a test app context or use `no_run`.

#### 4.3 Update `mesh_glb_io.rs` texture extraction

File: `sdk/campaign_builder/src/mesh_glb_io.rs`

Current: `extract_base_color_texture_payload()` (L543–L607) extracts only the
base-color texture. `has_unsupported_pbr_channels` is set to `true` when a
normal or occlusion texture is found.

Changes required:

1. Add a `TextureKind` enum:

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TextureKind { BaseColor, NormalMap, OcclusionMap, MetallicRoughness, Emissive }
   ```

2. Change `ImportedGlbMesh::texture_payload: Option<ImportedGlbTexturePayload>`
   (field at L203) to
   `texture_payloads: Vec<(TextureKind, ImportedGlbTexturePayload)>`.
   Update all construction sites and test assertions that reference
   `texture_payload`.

3. In `convert_gltf_material()` (L629–L669), extract payloads for all five
   channels when present (normal, occlusion, metallic_roughness, emissive,
   base-color).

4. Set `has_unsupported_pbr_channels = false` — all standard PBR channels are
   now extracted and surfaced.

#### 4.4 Update SDK export pipeline

File: `sdk/campaign_builder/src/obj_importer.rs`

In the `ConvertToRon` export path, after writing texture bytes for each
`(TextureKind, ImportedGlbTexturePayload)`:

- Copy each texture file to `assets/textures/imported/{kind}_{name}.{ext}`
- Set the corresponding `MaterialDefinition` field (e.g., `normal_map_path`,
  `occlusion_map_path`) to the campaign-relative path.
- Update `ImportedMesh` (field `texture_payload` at L179) to carry the full
  `texture_payloads: Vec<(TextureKind, ImportedGlbTexturePayload)>` vec instead
  of the single `texture_payload` field.

#### 4.5 Testing Requirements

New tests in `src/game/systems/creature_meshes.rs`:

- `test_material_definition_normal_map_sets_bevy_normal_map_texture` — assert
  `StandardMaterial::normal_map_texture` is `Some(…)` when `normal_map_path` is set.
- `test_material_definition_no_normal_map_leaves_field_none` — existing
  `MaterialDefinition` with no texture paths; assert all new map fields are `None`.

New tests in `sdk/campaign_builder/src/mesh_glb_io.rs`:

- `test_import_glb_normal_texture_extracted_as_payload` — adapt
  `build_normal_texture_glb()` (already in the test module at L1050–L1072);
  assert `texture_payloads` contains a `NormalMap` entry.
- `test_import_glb_occlusion_texture_extracted_as_payload` — similarly for AO,
  using `build_occlusion_texture_glb()` (L1078–L1100).
- Update `test_glb_normal_texture_sets_unsupported_pbr_channels_flag` (L1484) →
  assert `has_unsupported_pbr_channels == false` after this phase.
- Update `test_glb_occlusion_texture_sets_unsupported_pbr_channels_flag` (L1502) →
  same.

#### 4.6 Deliverables

- [ ] Four new texture path fields on `MaterialDefinition` with `#[serde(default)]`
- [ ] Updated `material_definition_to_bevy()` with `asset_server: &AssetServer`
      parameter and four new texture bindings
- [ ] Updated doctest on `material_definition_to_bevy()` using `no_run` or test-app
      pattern
- [ ] `landscape_material()` in `map.rs` updated to accept and forward `asset_server`
- [ ] `TextureKind` enum in `mesh_glb_io.rs`
- [ ] `texture_payloads: Vec<(TextureKind, ImportedGlbTexturePayload)>` on
      `ImportedGlbMesh`
- [ ] All five PBR channels extracted in `convert_gltf_material()`
- [ ] `has_unsupported_pbr_channels` set to `false`
- [ ] SDK export pipeline writes all texture paths into `MaterialDefinition`
- [ ] Updated tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 4.7 Success Criteria

A `MeshDefinition` carrying a `normal_map_path` applies that normal map at
runtime via `StandardMaterial`. The SDK no longer discards normal/AO/PBR
textures during GLB import. `has_unsupported_pbr_channels` is `false` for
all standard PBR channels. All tests pass; `cargo clippy -- -D warnings` clean.

---

### Phase 5: Asset Migration & Optimisation

Convert existing imported RON mesh assets to GLB, add KTX2 texture compression,
update all registry entries to use `glb_path`, and update architecture
documentation. Migration covers **all five mesh registry types**.

GLB files for the landscape mesh assets are already available. Item, furniture,
and object mesh asset availability should be confirmed before the corresponding
migration sub-steps are started.

#### 5.1 Enable KTX2 texture support

File: `Cargo.toml` (root workspace)

Update the `bevy` dependency to add the KTX2 and Zstd features:

```toml
bevy = { version = "0.19", default-features = true, features = ["ktx2", "zstd"] }
```

KTX2 enables GPU-native texture compression (BC7 / ASTC) and mipmap storage
directly in the texture file. The `zstd` feature adds an additional compression
layer on top, reducing file size further. Bevy loads KTX2 textures natively;
no runtime code changes are needed beyond the Cargo feature flags.

Verify the build compiles cleanly with the new features before proceeding:
`cargo check --all-targets --all-features`.

#### 5.2 Convert textures in each GLB to KTX2

For each of the five GLB files, convert embedded PNG/JPEG textures to KTX2
using the `toktx` tool from the KTX-Software suite
(https://github.com/KhronosGroup/KTX-Software). Alternatively, `basisu` from
the Basis Universal toolchain produces `.ktx2` files with Basis LZ compression
that Bevy can load via the `ktx2` feature.

Recommended per-texture settings:

| Texture type                | Format                | Rationale                               |
| --------------------------- | --------------------- | --------------------------------------- |
| Base color (sRGB)           | BC7 / ASTC 4×4 (sRGB) | Highest quality for colour data         |
| Normal map (linear)         | BC5 (RG only)         | Preserves XY normal channels accurately |
| Metallic/roughness (linear) | BC5 or BC7            | Preserves G/B channel accuracy          |
| AO (linear, single channel) | BC4                   | Single-channel compression              |
| Emissive (sRGB)             | BC7 (sRGB)            | Matches base-colour format              |

After converting, use `gltf-transform` (Node.js,
`npm install -g @gltf-transform/cli`) to replace the embedded PNG/JPEG
bytes inside each GLB with the KTX2 bytes:

```bash
gltf-transform ktx2 input.glb output_ktx2.glb \
  --slots "baseColorTexture,normalTexture,metallicRoughnessTexture" \
  --codec uastc
```

The `--codec uastc` flag produces UASTC-encoded KTX2, which Bevy 0.19 supports
via the `ktx2` feature. Verify each output GLB loads without warnings in Bevy
before proceeding to §5.3.

#### 5.3 Register each GLB in the Campaign Builder

**Landscape meshes** (GLB files confirmed available) — for each of the five
assets:

1. Open the Campaign Builder, navigate to the Importer tab.
2. Load the KTX2-converted `.glb` file.
3. Select **Raw GLB** export mode, `ExportType::Landscape`, and the appropriate
   `LandscapeCategory`.
4. Export into the tutorial campaign. This copies the file to
   `campaigns/tutorial/assets/meshes/landscape/{category}/{name}.glb` and
   writes a `glb_path`-based entry in
   `campaigns/tutorial/data/landscape_mesh_registry.ron`.
5. Validate in-game rendering is visually correct on a map that uses the asset.
6. Delete the corresponding `.ron` file from `assets/meshes/` once confirmed.

Process one asset at a time; validate before moving to the next.

**Furniture meshes** — repeat the process above using `ExportType::Furniture`
for every entry in `campaigns/tutorial/data/furniture_mesh_registry.ron` that
currently references a RON vertex file. If GLB source files are not yet
available for furniture assets, document which entries remain on the RON path
and schedule their migration separately.

**Item meshes** — repeat using `ExportType::Item` for every entry in
`campaigns/tutorial/data/item_mesh_registry.ron` that references a RON vertex
file. Note that items without a `mesh_id` set on the `Item` struct fall back to
the procedural `ItemMeshDescriptor` path and are unaffected by this migration.

**Object meshes** — repeat using `ExportType::ObjectMesh` for every entry in
`campaigns/tutorial/data/object_mesh_registry.ron`. These cover named
map-event objects (treasure chests, doors, signs, containers). Each
`ObjectMeshEntry::filepath` currently points to a `CreatureDefinition` RON
file; the Raw GLB export writes a new `CreatureDefinition` RON that has
`glb_path: Some(…)` and `meshes: []`, leaving the `filepath` reference
mechanism intact.

#### 5.4 Update `data/test_campaign` fixtures

For the GLB-path fixture entries added in Phase 1
(`data/test_campaign/data/landscape_mesh_registry.ron` and `creatures.ron`):

- Keep the `test_triangle.glb` minimal fixture for unit tests.
- Do not replace with references to `campaigns/tutorial` data.
- If broader integration tests are needed, add additional self-contained GLB
  fixtures to `data/test_campaign/assets/meshes/`.

#### 5.5 Update architecture documentation

File: `docs/reference/architecture.md`

- **Section 4.2** (`LandscapeDefinition`, `LandscapeMeshDatabase`): document
  `glb_path` and `glb_scene_index` fields on `CreatureDefinition`, and the
  updated `validate()` rules permitting GLB-only entries.
- **Section 6.2** (Rendering Architecture): document the two imported-mesh
  rendering paths and the selection logic:
  - `glb_path.is_some()` → `SceneRoot` + Bevy GLTF loader; asset key uses
    `#SceneN` fragment syntax
  - `glb_path.is_none()` → `mesh_definition_to_bevy()` → `Mesh3d` / `MeshMaterial3d`
  - Document `GlbHandleCache` resource and `glb_scene_loader_system`
- **Section 7.1** (External Data Files): add `*.glb` under campaign assets
  directory listing; note that mesh registry RON entries may carry either inline
  `meshes` or a `glb_path` reference.
- **Section 7.2** (Example Data Format): add a GLB registry entry example
  alongside the existing RON mesh example.

#### 5.6 Record performance measurements

After converting all five assets, measure and record in this document:

| Metric                                  | Before   | After         |
| --------------------------------------- | -------- | ------------- |
| `assets/meshes/` total size             | ~40 MB   | target ≤ 5 MB |
| Campaign load time (map with landscape) | baseline | target −20 %  |
| `brush.ron` / equivalent GLB size       | 32 MB    | target ≤ 4 MB |

#### 5.7 Deliverables

- [ ] `bevy` dependency updated with `ktx2` and `zstd` features in `Cargo.toml`
- [ ] KTX2-converted GLB files for all five landscape mesh assets
- [ ] KTX2-converted GLB files for furniture, item, and object mesh assets
      (or documented list of assets deferred due to missing source GLBs)
- [ ] `campaigns/tutorial/data/landscape_mesh_registry.ron` entries updated
      with `glb_path`
- [ ] `campaigns/tutorial/data/furniture_mesh_registry.ron` entries updated
      with `glb_path` (where GLB source available)
- [ ] `campaigns/tutorial/data/item_mesh_registry.ron` entries updated with
      `glb_path` (where GLB source available)
- [ ] `campaigns/tutorial/data/object_mesh_registry.ron` referenced
      `CreatureDefinition` RON files updated to `glb_path: Some(…)`, `meshes: []`
- [ ] Old `.ron` vertex data files removed from `assets/meshes/`
- [ ] `data/test_campaign` fixtures updated; zero `campaigns/tutorial` references
      in tests
- [ ] `docs/reference/architecture.md` updated (Sections 4.2, 6.2, 7.1, 7.2)
- [ ] Performance measurements recorded in §5.6
- [ ] `docs/explanation/implementations.md` updated

#### 5.8 Success Criteria

`assets/meshes/` total size reduced by ≥ 70 %. Campaign startup time reduced
by ≥ 20 % for maps with landscape meshes. KTX2 textures load without Bevy
warnings. All tests pass. Architecture document accurately describes both
rendering paths. `cargo clippy -- -D warnings` clean. No test references
`campaigns/tutorial`.

---

## Dependency Graph

```text
Phase 1 (Domain)
    ├── Phase 2 (SDK Raw Export + GLB Preview)  ← requires glb_path field from Phase 1
    └── Phase 3 (Runtime Rendering)              ← requires glb_path field from Phase 1
            └── Phase 4 (Full PBR)              ← independent of Phase 3; improves RON path
                    └── Phase 5 (Migration)    ← requires Phase 2 (export) + Phase 3 (rendering)
```

Phases 2 and 3 are independent of each other and can be developed in parallel
once Phase 1 is complete. Phase 4 is independent of Phase 3 (it improves the
RON path, not the GLB path) and can be scheduled at any point after Phase 1.
Phase 5 requires Phase 2 (Raw GLB export to register the new assets), Phase 3
(runtime rendering of those assets), and the `ktx2`/`zstd` Cargo features added
in Phase 5 itself.

---

## Resolved Design Decisions

1. **GLB source files available** — GLB files exist for all five landscape mesh
   assets. Phase 5 proceeds directly to KTX2 conversion and registration;
   no re-import-to-Blender step is required.

2. **KTX2 textures** — Phase 5 targets KTX2 for all embedded textures (BC7 for
   colour/emissive in sRGB, BC5 for normal/metallic-roughness in linear, BC4 for
   single-channel AO). Requires adding `features = ["ktx2", "zstd"]` to the
   `bevy` dependency in `Cargo.toml` and converting texture data using `toktx`
   or `gltf-transform ktx2 --codec uastc`.

3. **Campaign Builder GLB preview** — The preview panel must render imported GLB
   geometry directly. Implementation: call `import_glb_scene_from_file()` (already
   in `mesh_glb_io.rs`) to extract `MeshDefinition` values and feed them to the
   existing software preview renderer. No new renderer is needed. Preview geometry
   is cached by `glb_path` to avoid re-parsing on every repaint. Covered in
   Phase 2 §2.5.

4. **`spawn_creature()` signature change** — The new `glb_cache: Option<&GlbHandleCache>`
   parameter uses `Option` so all existing callers can pass `None` without
   structural changes. The GLB path is only activated when both the field is `Some`
   and the cache is `Some`. This avoids a forced refactor of all spawning call sites
   while still enabling GLB rendering at new call sites.

5. **No `AppState` enum** — Antares does not use Bevy's `States` system. The
   `glb_scene_loader_system` runs in the `Startup` schedule, which executes after
   `AntaresPlugin::build()` inserts the `GameContent` resource containing the
   loaded `ContentDatabase`. This ordering is sufficient because content is loaded
   synchronously in `AntaresPlugin::build()` before any systems run.

6. **Bevy GLTF scene fragment syntax** — To obtain a `Handle<Scene>` (required
   by `SceneRoot`) rather than `Handle<Gltf>`, the asset path must include a scene
   fragment: `"assets/meshes/foo.glb#Scene0"`. The `glb_scene_index` field on
   `CreatureDefinition` maps to this index. `GlbHandleCache` stores handles keyed
   by the raw `glb_path` string (without fragment); the fragment is applied only
   when calling `asset_server.load()` in `glb_scene_loader_system`.
