# Implementations

Consolidated log of major implementation work, newest themes first. Per-phase
detail (file lists, quality-gate output, test inventories) was removed; see git
history for the full record.

---

## GLTF 2.0 Support — Phase 1: Domain & Data Model Foundation

Extended the domain layer to carry an optional GLB path reference on
`CreatureDefinition`. All new fields use `#[serde(default)]`; existing RON
data files deserialize without modification.

### 1.1 `CreatureDefinition` struct extension (`src/domain/visual/mod.rs`)

- Added `pub glb_path: Option<String>` — campaign-relative path to a GLB file
  (e.g. `"assets/meshes/creatures/foo.glb"`). Must start with `"assets/"`.
  Mutually exclusive with non-empty `meshes`. Tagged `#[serde(default)]`.
- Added `pub glb_scene_index: usize` — GLTF scene index within the GLB file
  (default: 0). Tagged `#[serde(default)]`.
- Updated `Default` impl with `glb_path: None, glb_scene_index: 0`.
- Updated `validate()` to enforce mutual-exclusion:
  - `glb_path: Some` + non-empty `meshes` → `ValidationError::Structural`.
  - `glb_path: Some` + empty `meshes` → GLB-only: validates scale only.
  - `glb_path: None` + empty `meshes` → `ValidationError::EmptyField` (preserved).
- Updated all doc-comment and test struct literals (15 sites) with the two new fields.
- Added 3 new tests: `test_creature_definition_glb_path_only_is_valid`,
  `test_creature_definition_glb_and_meshes_both_set_returns_error`,
  `test_creature_definition_no_glb_no_meshes_returns_error`.

### 1.2 `CreatureDatabase` validation (`src/domain/visual/creature_database.rs`)

- Added `AmbiguousMeshSource(CreatureId)` variant to `CreatureDatabaseError` —
  fires when a creature has both `glb_path` set and non-empty `meshes`.
- Added `GlbPathError(CreatureId, String)` variant — fires when `glb_path`
  does not start with `"assets/"`.
- Updated `CreatureDatabase::validate()` with DB-level checks before
  per-creature validation.
- Updated 6 struct literal sites in doctests and test helpers.
- Added 3 new tests: `test_creature_database_glb_path_only_is_valid`,
  `test_creature_database_glb_and_meshes_both_set_returns_ambiguous_error`,
  `test_creature_database_glb_path_must_start_with_assets_prefix`.

### 1.3 `ObjectMeshDatabase` and `ItemMeshDatabase` iterator methods

- Added `pub fn glb_paths(&self) -> impl Iterator<Item = &str>` on
  `ObjectMeshDatabase` (`src/domain/world/object_mesh.rs`). Walks only the
  primary `meshes` `HashMap`; fallback registries use lazy loading and are
  handled by the Phase 3 pre-loader separately.
- Added 2 new tests: `test_object_mesh_database_glb_paths_returns_glb_entries`,
  `test_object_mesh_database_glb_paths_empty_for_all_ron`.
- Added `pub fn glb_paths(&self) -> impl Iterator<Item = &str>` on
  `ItemMeshDatabase` (`src/domain/items/database.rs`). Delegates to
  `self.inner.all_creatures()`.
- Added 1 new test: `test_item_mesh_database_glb_paths_returns_glb_entries`.

### 1.4 Struct literal call-site updates (all src/ and sdk/ files)

Every explicit `CreatureDefinition { ... }` literal across the codebase had
`glb_path: None, glb_scene_index: 0,` added. Files updated:

- `src/domain/visual/creature_variations.rs` (4 sites)
- `src/domain/visual/item_mesh.rs` (1 site)
- `src/domain/world/landscape.rs` (1 site)
- `src/game/systems/creature_spawning.rs` (4 sites)
- `src/game/systems/map.rs` (5 sites — collateral fixes for compilation)
- `src/sdk/creature_validation.rs` (2 sites — collateral)
- `sdk/campaign_builder/src/creature_templates.rs` (24 sites)
- `sdk/campaign_builder/src/creature_assets.rs` (2 sites)
- `sdk/campaign_builder/src/auto_save.rs` (1 site)
- `sdk/campaign_builder/src/creatures_editor/mod.rs` (6 sites)
- `sdk/campaign_builder/src/obj_importer_ui.rs` (2 sites)

### 1.5 Data fixtures (`data/test_campaign/`)

- New directories: `assets/meshes/creatures/`, `assets/meshes/items/`,
  `assets/meshes/furniture/` (empty, for organizational structure).
- New GLB binary: `assets/meshes/test_triangle.glb` — minimal valid GLB 2.0
  (540 bytes), one triangle mesh, embedded buffer.
- New RON asset: `assets/creatures/test_triangle_glb.ron` — `CreatureDefinition`
  id 999, `glb_path: Some("assets/meshes/test_triangle.glb")`, `meshes: []`.
- New RON asset: `assets/meshes/objects/test_triangle_glb.ron` — same structure,
  id 12099.
- Registry additions in `data/`:
  - `creatures.ron` — entry id 999 `TestTriangleGlb`.
  - `landscape_mesh_registry.ron` — entry id 11099 `TestTriangleGlb`.
  - `item_mesh_registry.ron` — entry id 9999 `TestTriangleGlb`.
  - `object_mesh_registry.ron` — entry id 12099 `Test Triangle GLB`.
- Updated `test_test_campaign_landscape_mesh_fixture_integrity` in
  `landscape.rs` to count 6 entries and skip the inline-mesh assertions for
  GLB-only entries.
- Updated `test_object_mesh_registry_file_load_test_campaign_fixture` in
  `object_mesh.rs` to count 7 entries.

### Quality gates (final run)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- `cargo nextest run --all-features` — **5610 passed, 8 skipped, 0 failed**.

---

## GLTF 2.0 Support — Phase 3: Runtime GLB Rendering

Added a startup-populated handle cache and GLB rendering branches to all
creature/object/item spawn sites. Entities backed by a `glb_path` now spawn
with `WorldAssetRoot` instead of inline `Mesh3d` components; Bevy's scene
spawner attaches the GLB child hierarchy asynchronously.

### 3.1 `GlbHandleCache` resource (`src/game/resources/glb_assets.rs`)

- New `pub struct GlbHandleCache` (`#[derive(Resource, Default)]`) with one
  field: `pub scenes: HashMap<String, Handle<WorldAsset>>`.
- Keys are campaign-relative GLB paths (e.g. `"assets/meshes/foo.glb"`),
  values are pre-loaded `Handle<WorldAsset>` handles.
- Re-exported from `src/game/resources/mod.rs` as `pub use glb_assets::GlbHandleCache`.
- Tests: `test_glb_handle_cache_default_is_empty`,
  `test_glb_handle_cache_insert_and_lookup`,
  `test_glb_handle_cache_multiple_entries`.

### 3.2 `glb_scene_loader_system` (`src/game/systems/glb_scene_loader.rs`)

- New startup system that iterates all three content databases:
  1. `ContentDatabase::creatures` — uses `all_creatures()` to preserve
     per-entry `glb_scene_index`.
  2. `ContentDatabase::object_meshes` — uses `glb_paths()` (Scene 0).
  3. `ContentDatabase::item_meshes` — uses `glb_paths()` (Scene 0).
- Collects all paths into owned `Vec<(String, usize)>` / `Vec<String>` _before_
  calling `asset_server.load()`. This releases the `Res<GameContent>` borrow,
  satisfying the `'static` lifetime required by
  `GltfAssetLabel::Scene(n).from_asset(owned_string)`.
- Deduplicates paths via `HashSet<String>` so each GLB file is loaded at most once.
- Graceful-degradation: if `GameContent` is absent at startup the system logs a
  `warn!` and returns early; the cache stays empty (map spawning falls back to
  RON meshes).
- Registered ONLY in `MapRenderingPlugin::build()` immediately before
  `spawn_map_system` in the startup chain. `antares.rs` calls
  `init_resource::<GlbHandleCache>()` (idempotent) but does not register the
  system a second time.
- Tests: `test_glb_scene_loader_populates_cache_from_all_databases`
  (requires `app.init_asset::<WorldAsset>()` before `app.update()`),
  `test_glb_scene_loader_no_content_does_not_panic`.

### 3.3 GLB branch in `spawn_creature()` (`src/game/systems/creature_spawning.rs`)

- Added `glb_cache: Option<&GlbHandleCache>` parameter to `pub fn spawn_creature()`.
  All call-sites in `map.rs` and `item_world_events.rs` pass `Some(glb_cache)`;
  test code passes `None`.
- When `creature_def.glb_path.is_some()` the function delegates immediately to the
  new private `fn spawn_creature_glb()`.
- `spawn_creature_glb()` logic:
  - Uses `let Some(glb_path) = ... else { warn!; return placeholder }` instead of
    `.expect()` to satisfy `clippy::expect-used`.
  - Looks up `glb_cache.scenes.get(glb_path)` for the pre-loaded handle.
  - On cache hit: spawns root entity with `WorldAssetRoot(handle)` + `CreatureVisual`
    - `FacingComponent` + transforms; Bevy scene spawner attaches children async.
  - On cache miss or absent cache: logs `warn!` and spawns a zero-mesh placeholder
    entity so the caller always gets a valid `Entity` back.
- Tests: `test_spawn_creature_with_glb_path_uses_scene_root` (requires
  `init_asset::<Mesh>` + `init_asset::<StandardMaterial>` + `RunSystemOnce` trait),
  `test_spawn_creature_without_glb_path_uses_mesh_bundle`.

### 3.4 GLB branches in map mesh-spawning functions (`src/game/systems/map.rs`)

- `spawn_event_meshes()`: when the resolved `CreatureDefinition` has `glb_path`,
  looks up the handle in `GlbHandleCache` and spawns `WorldAssetRoot` instead of
  inline mesh children.
- `spawn_furniture_mesh()`: same GLB-first guard before RON procedural path.
- `spawn_landscape_root()` / imported landscape path: threads `glb_cache` through;
  GLB landscape definitions spawn `WorldAssetRoot` on the root entity.
- All three `spawn_creature()` call-sites in `spawn_map()` updated to pass
  `Some(glb_cache)` (NPC / encounter creatures now use the GLB path when available).
- `make_spawn_app` test helper: added `app.init_asset::<WorldAsset>()` so that the
  `glb_scene_loader_system` (run via `MapRenderingPlugin`) can call
  `asset_server.load::<WorldAsset>()` without panicking.

### 3.5 GLB branch in `spawn_dropped_item_system()` (`src/game/systems/item_world_events.rs`)

- Added `Option<Res<GlbHandleCache>>` parameter.
- Guard: GLB items are spawned with a floor-clearance Y offset
  (`DROPPED_ITEM_GLB_FLOOR_CLEARANCE = 0.05`) so they rest visually on the tile.
- Calls `spawn_creature(…, Some(&*glb_cache))` on the matching `CreatureDefinition`.
- Test: `test_spawn_dropped_item_glb_uses_floor_clearance_height`.

### 3.6 `antares.rs` registration (`src/bin/antares.rs`)

- Added `app.init_resource::<GlbHandleCache>()`. The system itself is registered
  only inside `MapRenderingPlugin` to avoid double-execution.

### Quality gates (final run)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- `cargo nextest run --all-features` — **5618 passed, 8 skipped, 0 failed**.

---

Added a `RawGlb` export mode to the Campaign Builder importer. When selected, the
source `.glb` is copied directly into `assets/meshes/{type}/` with a companion
`CreatureDefinition` `.ron` file whose `glb_path` is set and `meshes` is empty.
The preview panel now loads and shows geometry for GLB-only creatures.

### 2.1 `GlbExportMode` enum (`sdk/campaign_builder/src/obj_importer.rs`)

- Added `pub enum GlbExportMode { ConvertToRon (default), RawGlb }` with
  `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]`.
- Added `pub glb_export_mode: GlbExportMode` field to `ObjImporterState`.
- `clear()` resets to `ConvertToRon` via `..Self::default()` (not in the
  preserved-fields list).
- Tests: `test_glb_export_mode_default_is_convert_to_ron`,
  `test_glb_export_mode_resets_on_clear`.

### 2.2 Raw GLB export path (`sdk/campaign_builder/src/obj_importer_ui.rs`)

- Added `GlbCopy(String)` variant to `ObjImporterExportError`.
- New private `export_raw_glb(state, campaign_dir, landscape_file)` function:
  - Copies source `.glb` to `assets/meshes/{type}/{category?}/{stem}.glb`
  - Writes companion `.ron` (same path, `.ron` extension) with `meshes: []`
    and `glb_path: Some("assets/meshes/.../{stem}.glb")`
  - Updates registry files for all five `ExportType` variants
  - Does **not** call `ground_meshes_to_y_zero` or serialize any `MeshDefinition`
- `export_state_to_campaign_with_landscape_file` dispatches to `export_raw_glb`
  when `glb_export_mode == RawGlb && source_format == Glb`; OBJ sources always
  fall through to the existing RON conversion path.
- Import `GlbExportMode` from `crate::obj_importer`.
- Tests: `test_raw_glb_export_copies_file_to_campaign_assets`,
  `test_raw_glb_export_registry_entry_has_glb_path_and_empty_meshes`,
  `test_raw_glb_export_not_available_for_obj_source`,
  `test_raw_glb_creature_export_writes_registry_entry`.

### 2.3 Export Format UI (`sdk/campaign_builder/src/obj_importer_ui.rs`)

- Added "Export Format" radio button row in `render_loaded_mode` grid,
  shown only when `source_format == ImportSourceFormat::Glb`.
  Options: `RON Mesh (convert geometry)` / `Raw GLB (copy file, preserve full PBR)`.
- Export button label is now dynamic: `"Export GLB"` in `RawGlb` mode,
  `"Export RON"` otherwise.
- `export_enabled` guard extended: Raw GLB with a source path is enabled even
  when `meshes` is empty (i.e., the GLB was loaded but no meshes are listed).

### 2.4 Packager verification (`src/sdk/campaign_packager.rs`)

`CampaignPackager::add_directory_to_archive()` recursively walks the campaign
directory and adds every file it finds. The only exclusions are hidden files
(`.` prefix), `target/`, and `node_modules/` — no extension filtering exists.
`.glb` files placed under `assets/meshes/` are therefore included in campaign
packages with **no code change required**.

### 2.5 GLB preview in `CreaturesEditorState`

- `sdk/campaign_builder/src/creatures_editor/mod.rs`: Added private
  `glb_preview_cache: HashMap<String, Vec<MeshDefinition>>` field (initialized
  as `HashMap::new()`). Caches parsed preview geometry keyed by `glb_path`
  string to avoid re-parsing on every repaint.
- `sdk/campaign_builder/src/creatures_editor/preview_panel.rs`:
  - Added `ensure_glb_preview_cache(&mut self, path_str: &str)` helper — resolves
    campaign-relative path via `last_campaign_dir`, calls
    `import_glb_scene_from_file`, stores `Vec<MeshDefinition>` in cache
    (or empty `Vec` + error message on failure).
  - Updated `sync_preview_renderer_from_edit_buffer` to detect GLB-only
    creatures (`glb_path.is_some() && meshes.is_empty()`), load/cache geometry,
    and substitute it into `preview_creature` before passing to the renderer.
  - No changes to `show_preview_panel`, `show_preview_fallback`, or
    `build_preview_statistics`.
- Landscape editor (`landscape_editor.rs`): the `show_landscape_preview` function
  is text-only (shows ID, category, scale metadata) — no 3D renderer is present,
  so no changes were needed.
- Tests: `test_preview_panel_glb_path_returns_imported_geometry` (uses
  `data/test_campaign/assets/meshes/test_triangle.glb` fixture),
  `test_preview_panel_glb_missing_file_displays_error_string`.

### 2.6 Pre-existing struct literal call-site fixes (Phase 1 missed)

Phase 1 missed several SDK files; Phase 2 added `glb_path: None, glb_scene_index: 0`
to struct literals in:

- `sdk/campaign_builder/src/creature_undo_redo.rs` (2 sites)
- `sdk/campaign_builder/src/creatures_workflow.rs` (1 site)
- `sdk/campaign_builder/src/objects_editor.rs` (2 sites)
- `sdk/campaign_builder/src/preview_renderer.rs` (3 sites)
- `sdk/campaign_builder/src/template_metadata.rs` (1 site)
- `sdk/campaign_builder/src/variation_editor.rs` (1 site)
- `sdk/campaign_builder/tests/creature_asset_editor_tests.rs` (10 sites)
- `sdk/campaign_builder/tests/creature_preview_integration_test.rs` (2 sites)
- `sdk/campaign_builder/tests/creature_workflow_tests.rs` (1 site)

### 2.7 Pre-existing clippy warning fixes

Several pre-existing clippy lints were tripped by the newer toolchain:

- `campaign_io.rs`, `lib.rs`: `let_underscore_must_use` → `.ok()` call
- `lib.rs`: `needless_range_loop` → `.iter().enumerate()`
- `characters_editor.rs`: `manual_repeat_n` → `std::iter::repeat_n()`
- `creatures_editor/mod.rs`, `monsters_editor.rs`: `field_reassign_with_default`
  → `#[allow]` on test functions
- `map_editor.rs`: `too_many_arguments` → `#[allow]` on `fn show_list`

### Quality gates (final run)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- `cargo nextest run --all-features` — **2566 passed, 0 failed** (SDK).

---

## Audio

### Audio Manager (Phases 1–3 + SDK layout compliance)

- **Domain**: `AudioMap` struct and `audio.ron` format for per-campaign audio
  mapping.
- **Game**: `src/game/systems/audio.rs` loads and applies the `AudioMap` at
  runtime.
- **SDK**: `AudioEditorState` and the Audio tab in the Campaign Builder, with
  audio file preview; layout fixed to comply with SDK AGENTS.md Rules 13, 15, 17.

### Audio RON & SFX Mapping (Phases 1–4)

- `AudioManifest` definition (Phase 1) and campaign loader support for
  `audio.ron` (Phase 2).
- Bevy integration: `AudioManifestResource` and an updated `resolve_audio_path`
  (Phase 3).
- Test fixtures under `data/test_campaign` and integration tests, with no
  references to `campaigns/tutorial` (Phase 4).

---

## Terrain Externalization (Phases 1–6)

Replaced the closed `TerrainType` enum with the open numeric `TerrainId = u32`
backed by data-driven definitions. Breaking, non-backwards-compatible change.

1. **Domain foundation**: `TerrainDefinition` / `TerrainDatabase` registry.
2. **Type migration**: `TerrainType` removed across domain, game resources,
   game systems and SDK; built-in `TERRAIN_*` constants.
3. **Built-in content**: `data/terrain.ron` with 12 built-in terrains and a
   `builtin` short-name module.
4. **Rendering/gameplay**: mesh style, vegetation, height and color now read
   from `TerrainDefinition`.
5. **SDK tooling**: `map_builder`, `texture_generator`, templates, Terrain
   Editor, and `map_editor.rs` migrated to `TerrainId`.
6. **Cleanup**: remaining legacy data migrated, no Rust references to
   `TerrainType` remain, docs updated.

Related fixes: Terrain Editor stored absolute machine-specific texture paths
(now campaign-relative); `map_7` and `map_8` updated to use Sand, Ice and Snow.

---

## Object Mesh Registry Refactor (Phases 1–4)

- `ObjectMeshEntry { id, name, ... }` introduced as the domain type; registry
  is now an array of entries instead of a string-keyed map.
- Legacy `ObjectMeshRegistry(meshes: {...})` fallback removed and all data
  converted.
- SDK data model (`ObjectEntry` with numeric `id` and `name`) and all SDK call
  sites migrated.
- Objects Editor and Map Editor mesh picker show `(id, name)` pairs.

---

## NPC Combat Switch and Eonir Boss Fight (Phases 1–4)

- **Domain**: `NpcCombatSwitch` on `NpcDefinition`.
- **Engine**: `PendingNpcCombatResource`, `npc_combat_switch_system`,
  `start_npc_combat_system`, and an NPC spawn guard.
- **Post-combat**: `defeat_flag` propagated to `GlobalFlags` on victory.
- **Tutorial**: Eonir the Still boss encounter with a combat path and a
  dialogue path.

---

## Inventory UI Refactor (Phases 1–5)

- `InventoryViewMode { Multi, Single }` with number-key switching.
- 8×8 icon grids replaced by scrollable text lists with linear navigation in
  the character, merchant and container panels.
- Dead silhouette-rendering code removed.
- Items now group/stack in character inventory; `ScrollArea` `auto_shrink` and
  merchant scroll-to-selection fixed.

---

## SDK / Campaign Builder

- **Skill trainer repair (Phases 1–4)**: `DialogueTree` repair methods, wired
  into the dialogue editor; Dialogue ID picker in NPC trainer sections; trainer
  badges in the NPC preview panel.
- **Creature Editor**: mesh metadata editing panel (plus visibility fix),
  `registry_dirty` flag, creature browser matching the Character Editor picker.
- **Editor toolbars**: Landscape and Objects editors migrated to `EditorToolbar`
  / `EditorContext`; three broken editors and a missing Campaign Metadata field
  fixed.
- **Creature persistence**: fixed `creatures.ron` failing to load and being
  wiped to `[]` on save.
- **Starting items**: stacked display and correct `charges` in
  `populate_starting_inventory`.
- **Objects Editor**: right-click Duplicate now works.
- **Performance**: NPC tab O(N×D) per-frame validation hotspot removed.

---

## Bug Fixes

| Area         | Problem                                                                              | Fix                                                                                                                 |
| ------------ | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Inn          | Party members still listed as available to recruit                                   | Match party member to roster by name + `InParty`, not position                                                      |
| Recruitment  | Recruiting a premade at the inn left its map recruitment event and mesh in the world | `GameState::remove_recruitable_events_for` removes the events on all maps; inn recruit/swap also despawn the visual |
| Recruitment  | Premade characters duplicated in roster when recruited from the map                  | `recruit_from_map` and `execute_recruit_to_inn` reuse the existing roster entry                                     |
| Inventory    | Healing Potions / Food Rations did not stack                                         | `build_grouped_inventory` no longer requires `charges == 0`                                                         |
| Combat       | Long Bow caused endless attacks and always showed "Miss"                             | Two combined bugs in ranged attack handling fixed                                                                   |
| HUD          | Grey portrait frame missing for starting characters                                  | Timing difference in portrait loading corrected                                                                     |
| Consumables  | Potions and scrolls had no persistent effect                                         | Two independent root causes fixed                                                                                   |
| Merchant     | Sold items missing from stock; sell price ignored `sell_cost`                        | Stock entry created on sale; shared price helper used                                                               |
| Map events   | Static `MapEvent::DroppedItem` could not be picked up                                | Pickup now handles both dropped-item storage locations                                                              |
| Rendering    | Furniture `mesh_id` dropped; event objects floating / untextured                     | Imported-mesh branch added; Y offset and `asset_server` textures fixed                                              |
| Containers   | Chest could break after taking an item then pressing ESC                             | `ContainerInventory` added to `movement_blocked_for_mode`                                                           |
| Locked doors | Dungeon Gate Key did not open Barred Passage                                         | Key check runs before `dialogue_id` handling; Pick Lock / Bash fallback added                                       |

---

## Tutorial Campaign Content

- **Dialogue overhaul**: every character recruitable, lore-consistent trees,
  duplicate id bug (Eonir vs Isolde, 1003) fixed.
- **Maps**: names and descriptions aligned to _The Stillness Prophecy_; Frostspire
  Peaks (`map_8`) populated with Dire Wolves, Skeletons and Eonir via fixed
  events and a random encounter table.
- **Lore**: all eight premade-character lore files and NPC data updated.

---

## Character Sheet

- Character Bio & Navigation Phase 5: mouse input fix for the character sheet.
