## Phase 6: Data Migration, Repo-Wide Cleanup, and Docs

### Summary

Completed the terrain externalization effort by migrating all remaining legacy
terrain data, eliminating every remaining Rust reference to the old TerrainType
system, bumping the save-format major version, and updating six documentation
files to reflect the TerrainId / TerrainDefinition architecture.

### Deliverables

- [x] **6.1 Map RON migration** — `data/test_town.ron` migrated from variant names
  (`Stone`, `Ground`) to numeric IDs (13005, 13000). `notes/map_backups/map_4.ron`,
  `map_5.ron`, `map_6.ron` migrated (all nine variants). Campaign and test-campaign
  maps were already numeric from Phase 2.
- [x] **6.1 Save-format version bump** — `Cargo.toml` antares package version bumped
  from 0.1.0 to 1.0.0. Hardcoded `"0.1.0"` strings in `src/bin/antares.rs`,
  `src/game/systems/menu.rs`, and `src/sdk/campaign_loader.rs` replaced with
  `env!("CARGO_PKG_VERSION")`. Fixture save file `campaigns/tutorial/saves/save_20260809_072524.ron`
  updated to `version: "1.0.0"` to keep the lore-field backward-compat test green.
- [x] **6.2 Repo-wide cleanup** — `sdk/campaign_builder/tests/rotation_test.rs`
  updated: `TerrainType` import removed; all `Tile::new` calls updated to
  `TERRAIN_GROUND + &builtin_terrain_db()`; `effective_height` / `mesh_dimensions`
  calls updated to `(wall_type, terrain_height: f32)` signature. String literals in
  `sdk/campaign_builder/tests/bug_verification.rs` and `integration_tests.rs`
  updated from `"PaintTerrain(TerrainType)"` to `"PaintTerrain("`.
  Zero `TerrainType` references remain in any `.rs` or `.ron` file.
- [x] **6.4 New test** — `test_all_campaign_maps_load_with_numeric_terrain_ids` added
  to `tests/map_content_tests.rs`. Discovers all `.ron` map files under
  `data/test_campaign/data/maps` and `campaigns/*/data/maps`, parses each as `Map`,
  and asserts every tile's `terrain >= TERRAIN_ID_MIN`.
- [x] **6.3 Documentation** — Six documents updated:
  - `docs/explanation/terrain_texture.md` — rewritten for per-`TerrainDefinition`
    texture paths; built-in ID-to-path table, custom terrain RON snippet.
  - `docs/explanation/modding_guide.md` — new `## Terrain Definitions` section
    added: built-in ID table, custom terrain RON, Rust constants.
  - `docs/how-to/use_terrain_specific_controls.md` — overview updated with TerrainId
    context; new Sand/Snow/Ice (13009/13010/13011) terrain section; troubleshooting
    updated with per-ID control mapping.
  - `docs/reference/map_ron_format.md` — new `### Terrain IDs` subsection in
    Content IDs Reference; stale tile-type-ID list replaced with terrain/wall docs.
  - `docs/reference/monster_creature_mapping_reference.md` — terrain ID range
    (13000–13099, 13100+) added to Visual/Mesh Registry table and Known Gaps.
  - `docs/reference/architecture.md` §4.2 — `Tile.terrain` updated from
    `TerrainType` to `TerrainId`; all narrative references updated to
    `TerrainDefinition` / `TerrainId` / numeric IDs.

### Files Changed

- `data/test_town.ron`
- `notes/map_backups/map_4.ron`, `map_5.ron`, `map_6.ron`
- `Cargo.toml`
- `campaigns/tutorial/saves/save_20260809_072524.ron`
- `src/bin/antares.rs`
- `src/game/systems/menu.rs`
- `src/sdk/campaign_loader.rs`
- `sdk/campaign_builder/tests/rotation_test.rs`
- `sdk/campaign_builder/tests/bug_verification.rs`
- `sdk/campaign_builder/tests/integration_tests.rs`
- `tests/map_content_tests.rs` (new test added)
- `docs/explanation/terrain_texture.md`
- `docs/explanation/modding_guide.md`
- `docs/how-to/use_terrain_specific_controls.md`
- `docs/reference/map_ron_format.md`
- `docs/reference/monster_creature_mapping_reference.md`
- `docs/reference/architecture.md`
- `docs/explanation/implementations.md` (this file)

### Success Criteria Met

- `cargo nextest run --all-features` — 5567 tests pass, 8 skipped
- `grep -rn "TerrainType" src/ sdk/ tests/` — zero matches
- `grep -rn "terrain: [A-Z]" campaigns/ data/` — zero matches
- All six documentation files updated

---

## Phase 5 (continued): Campaign Builder Terrain Editor

### Summary

Added the full terrain editor UI and supporting I/O infrastructure to the
Campaign Builder SDK. Campaign authors can now create and edit custom terrain
definitions (IDs >= 13100) through the new `Terrain` tab. Built-in terrain
(IDs 13000-13011) is shown as read-only reference. Custom definitions are
persisted to `data/terrain.ron` and merged with the built-in database on
every load and tab render.

### Deliverables

- [x] `terrain_editor.rs` (NEW) - TerrainEditorState with list/edit/add/delete UI
- [x] `editor_state.rs` - CampaignData.terrain_definitions, terrain_db (seeded from builtin_terrain_db); EditorRegistry.terrain_editor_state
- [x] `campaign_io.rs` - load_terrain(), save_terrain(); wired into do_open_campaign, do_new_campaign, do_save_campaign
- [x] `lib.rs` - CampaignMetadata.terrain_file; EditorTab::Terrain; MapEditorRefs.terrain_db passed
- [x] 9 unit tests in terrain_editor::tests

### Files Changed

- `sdk/campaign_builder/src/terrain_editor.rs` (NEW)
- `sdk/campaign_builder/src/editor_state.rs`
- `sdk/campaign_builder/src/campaign_io.rs`
- `sdk/campaign_builder/src/lib.rs`

---

## Phase 5 (map_editor.rs): TerrainType → TerrainId Migration

### Summary

Completed the final leg of the Terrain Externalization plan inside
`sdk/campaign_builder/src/map_editor.rs`. The deleted `TerrainType` enum has
been fully replaced with `TerrainId` (u32) backed by `TerrainDatabase`. Every
site that previously matched on `TerrainType` variants now does a DB lookup via
`TerrainDatabase::get_by_id`, reading `vegetation`, `mesh_style`, `blocked`, and
`color` from the definition. The map editor palette and tile inspector are now
fully data-driven and will automatically reflect any custom terrain IDs added by
a campaign author.

### Deliverables

- [x] Imports: `TerrainType` removed; `TerrainId`, `TerrainDatabase`, `TerrainDefinition`, `TerrainMeshStyle`, `TerrainVegetation`, `TERRAIN_GROUND` added
- [x] `MapEditorState.selected_terrain`: `TerrainType` → `TerrainId` (default `TERRAIN_GROUND`)
- [x] `MapEditorRefs` / `MapInspectorData`: added `pub terrain_db: &'a TerrainDatabase`
- [x] `MapGridWidget`: added `terrain_db: Option<&'a TerrainDatabase>` field + `.terrain_db()` builder; `tile_color` uses DB lookup for base color
- [x] `paint_tile(pos, db)`, `fill_region(..., db)`, `erase_tile(pos, db)`: all take `&TerrainDatabase`; blocked flag uses `db.get_by_id(id).map(|d| d.blocked)`
- [x] `apply_to_metadata_for_terrain(metadata, terrain_id, db)`: if/else on `vegetation`/`mesh_style` replaces old enum match
- [x] `apply_terrain_state_to_tile(..., db)`, `apply_terrain_state_to_selection(db)`: db propagated through
- [x] Tool palette terrain ComboBox: DB-driven, sorted by id, `push_id(def.id)`, `from_id_salt`
- [x] Inspector tile terrain ComboBox: DB-driven same pattern
- [x] `show_terrain_specific_controls(ui, terrain_id, db, state)`: if/else on vegetation/mesh_style
- [x] `vegetation_runtime_hint(terrain_id, db, metadata)`: DB-driven
- [x] `show_map_preview`: DB color lookup replaces hardcoded match
- [x] Tests: all `TerrainType::*` replaced with `TERRAIN_*` constants; `Tile::new`, `paint_tile`, `fill_region`, `apply_to_metadata_for_terrain`, `apply_terrain_state_to_selection` all pass `&builtin_terrain_db()`
- [x] SPDX year updated to 2026

### Quality Gates

```
cargo fmt --all                                                          → clean
cargo test --package campaign_builder 2>&1 | grep error | grep map_editor → 0 errors
cargo clippy --package campaign_builder -- -D warnings | grep map_editor  → 0 warnings
```

### Remaining Follow-up (not in scope)

`sdk/campaign_builder/src/lib.rs` line ~1495 constructs `MapEditorRefs` without
the new `terrain_db` field. That one-line addition is tracked separately as it
requires access to the campaign's merged `TerrainDatabase` at the call site.

---

## Phase 5: Terrain Externalization — SDK and Campaign Builder Tooling

### Summary

Updated the SDK CLI tools (`map_builder.rs` and `texture_generator.rs`) to use
`TerrainDatabase` dynamically instead of hardcoded constant matches and static
spec arrays. `parse_terrain` now performs case-insensitive lookup against any
campaign-extended `TerrainDatabase`, enabling custom terrain names. Terrain
texture generation is fully data-driven from the DB, expanding from 9 to 12
PNGs and covering any campaign-defined terrain automatically.

### Deliverables

- [x] `parse_terrain(s, db)` — DB-backed, case-insensitive name lookup replaces hardcoded `match`
- [x] `builtin_glyph(id)` — free function for ASCII map display, used by `show_map`
- [x] `show_map` — replaced `match tile.terrain { ... }` with `builtin_glyph(tile.terrain).unwrap_or('?')`
- [x] `TerrainTextureSpec.filename` changed from `&'static str` to `String`
- [x] `TERRAIN_SPECS` static constant removed; replaced with `terrain_specs_for_db(db)`
- [x] `terrain_texture_seed(id)` — deterministic multiplicative hash seed per terrain ID
- [x] `run_generate` — terrain section drives from `builtin_terrain_db()`; now writes 12 PNGs
- [x] All new tests pass: `test_parse_terrain_resolves_custom_campaign_terrain_by_name`, `test_texture_generator_covers_all_database_terrains`
- [x] Updated tests: terrain spec count 9 → 12, TERRAIN_SPECS references → `terrain_specs_for_db()`

### Files Changed

**`src/sdk/cli/map_builder.rs`**
- Added `TERRAIN_ICE` to imports.
- Added `pub fn builtin_glyph(id: TerrainId) -> Option<char>` — maps 12 built-in IDs to ASCII glyphs; returns `None` for campaign terrain.
- `parse_terrain(s, db)` — new signature takes `&TerrainDatabase`; tries numeric parse, then case-insensitive `db.all_definitions()` scan, then falls back to `TERRAIN_GROUND`.
- `show_map` match block replaced with `builtin_glyph(tile.terrain).unwrap_or('?')`.
- `process_command` set/fill branches pass `&self.terrain_db` to `parse_terrain`.
- `bulk_set_for_terrains` uses `let db = &self.terrain_db;` (split-borrow safe) for closure.
- New test: `test_parse_terrain_resolves_custom_campaign_terrain_by_name` — adds custom terrain to DB and verifies lookup by name, lowercase, numeric ID, and unknown fallback.

**`src/sdk/cli/texture_generator.rs`**
- Added `use crate::domain::world::terrain::{builtin_terrain_db, TerrainDatabase};`.
- `TerrainTextureSpec.filename: &'static str` → `String`.
- Removed `const TERRAIN_SPECS: &[TerrainTextureSpec]`.
- Added `fn terrain_texture_seed(id: u32) -> u64` (wrapping multiplicative hash).
- Added `fn terrain_specs_for_db(db: &TerrainDatabase) -> Vec<TerrainTextureSpec>` — sorts by ID, derives filename from lowercased name, converts color `[f32;3]` to u8.
- `run_generate` terrain loop now calls `terrain_specs_for_db(&builtin_terrain_db())`; 12 PNGs instead of 9.
- Module doc updated: 9 → 12 terrain PNGs; removed "no antares library imports" note.
- Updated tests: `test_terrain_specs_count` expects 12; all `TERRAIN_SPECS.iter()` calls use `terrain_specs_for_db(&builtin_terrain_db())` instead.
- New test: `test_texture_generator_covers_all_database_terrains` — verifies count matches DB, spot-checks `ground.png`, `sand.png`, `snow.png`, `ice.png`.

### Quality Gates

```
cargo fmt --all                                → clean
cargo check --package antares                  → 0 errors
cargo clippy --package antares -- -D warnings  → 0 warnings
cargo nextest run --package antares            → 5566 passed, 8 skipped, 0 failed
```

---

## Phase 4: Terrain Externalization — Rendering and Gameplay Systems

### Summary

Made all rendering and gameplay terrain logic fully data-driven by reading
`TerrainDefinition` fields (`mesh_style`, `vegetation`, `height`, `color`,
`texture_path`, `roughness`) from the active campaign's `TerrainDatabase`
(available via `content.0.terrain`). Hardcoded `TERRAIN_*` constant comparisons
in the dispatch, material, automap, vegetation, floor-clearance, and Walk on
Water systems are replaced with db-driven lookups. 5564 tests pass.

### Deliverables

- [x] `TerrainMaterialCache::is_fully_loaded` is DB-driven (takes `&TerrainDatabase`)
- [x] `TerrainMaterialCache::iter_all()` iterator added
- [x] `terrain_materials.rs` fully data-driven — `TEXTURE_*` constants, `texture_path_for`, `roughness_for` deleted; startup system iterates `terrain_db.all_definitions()`
- [x] `map.rs` tile dispatch: `match mesh_style { Water / Mountain / Flat(vegetation) / Flat }` replaces `match tile.terrain {}`; wall-tint from `def.color`; height from `def.height`
- [x] `hud.rs` `automap_tile_color` buckets by `mesh_style == Water` and `vegetation != None`
- [x] `item_world_events.rs` floor clearance from `definition.vegetation != None`
- [x] `vegetation_placement.rs` tree/shrub/grass dispatch from `definition.vegetation` field
- [x] `exploration_movement.rs` Walk on Water checks `mesh_style == Water` (any campaign terrain)
- [x] All required Phase 4 tests pass

### Files Changed

**`src/game/resources/terrain_material_cache.rs`**
- Removed 9-element hardcoded `TERRAIN_*` import.
- `is_fully_loaded(&self, db: &TerrainDatabase) -> bool` — iterates `db.all_definitions()` for O(n) completeness check.
- Added `iter_all() -> impl Iterator<Item=(TerrainId, &Handle<StandardMaterial>)>`.
- Tests: `all_terrain_ids()` expanded to 12 IDs; `is_fully_loaded` calls updated; `test_is_fully_loaded_false_with_eleven_of_twelve` and `test_is_fully_loaded_db_driven_custom_terrain` added.

**`src/game/systems/terrain_materials.rs`**
- Deleted `TEXTURE_*` constants (9), `texture_path_for`, `roughness_for`, `all_terrain_ids()`.
- `load_terrain_materials_system` accepts `content: Option<Res<GameContent>>`; iterates `terrain_db.all_definitions()` for texture and roughness.
- `refresh_terrain_materials_after_startup_allocations_system` uses `terrain_db.all_definitions()` and `def.roughness`.
- 5 tests deleted; 3 DB-property tests + `test_load_terrain_materials_system_loads_sand_snow_ice` added.

**`src/game/systems/map.rs`**
- Deleted `should_spawn_grass_cover`, `terrain_height_for_id` helpers.
- `should_spawn_procedural_vegetation` takes `def: Option<&TerrainDefinition>`.
- Per-tile loop: pre-computes `def`, `mesh_style`, `terrain_height`, `has_vegetation`, `is_forest`.
- Tile dispatch: `match mesh_style { Water / Mountain / Flat(vegetation) / Flat }`.
- Wall tint: `def.map(|d| d.color).unwrap_or(floor_rgb)`.
- Tests replaced and `test_terrain_height_from_terrain_definition`, `test_terrain_color_from_terrain_definition` added.

**`src/game/systems/hud.rs`**
- `automap_tile_color(tile, terrain_db: Option<&TerrainDatabase>)` — uses `mesh_style == Water` and `vegetation != None`.
- `update_mini_map` and `update_automap_image` gain `content: Option<Res<GameContent>>`.
- `test_automap_color_buckets_by_mesh_style_and_vegetation` added.

**`src/game/systems/item_world_events.rs`**
- Floor clearance: `match tile_terrain { TERRAIN_GRASS | TERRAIN_FOREST }` → `definition.vegetation != TerrainVegetation::None`.

**`src/game/systems/vegetation_placement.rs`**
- `tile_vegetation_plan`, `supports_vegetation_cover`, `should_plan_understory_shrubs` all gain `db: &TerrainDatabase`.
- `vegetation != TerrainVegetation::None` / `vegetation == TerrainVegetation::Forest` replace constant matches.
- `test_vegetation_plan_uses_terrain_definition_for_forest_detection` and `test_supports_vegetation_cover_uses_terrain_definition` added.

**`src/game/systems/input/exploration_movement.rs`**
- `should_override_water(gs, target, terrain_db: Option<&TerrainDatabase>)` — checks `mesh_style == TerrainMeshStyle::Water`; falls back to `t.terrain == TERRAIN_WATER` when `terrain_db` is `None`.
- `test_walk_on_water_override_applies_to_any_water_mesh_style_terrain` added.

### Quality Gates

```
cargo fmt --all             → clean
cargo check --all-targets   → 0 errors
cargo clippy -- -D warnings → 0 warnings
cargo nextest run           → 5564 passed, 8 skipped, 0 failed
```

---

## Phase 3: Terrain Externalization — Built-in Terrain Content and Loading

### Summary

Implemented Phase 3 of the terrain externalization plan: authored `data/terrain.ron`
with all 12 built-in terrain definitions, added a `pub mod builtin` short-name alias
module and a `TerrainDatabase::merge` method in `terrain.rs`, wired terrain loading
with base + campaign-override merge into both `ContentDatabase` (`sdk/database.rs`)
and `GameData` (`campaign_loader.rs`), generated placeholder PNG textures for Sand,
Snow, and Ice, and added all required fixture files and tests. 5558 tests pass.

### Deliverables

- [x] `data/terrain.ron` — 12 `TerrainDefinition` entries (IDs 13000–13011) matching `builtin_terrain_definitions()` exactly
- [x] `terrain::builtin::*` constants — `pub mod builtin` in `terrain.rs` re-exporting all 12 `TERRAIN_*` constants under short names (`GROUND`, `GRASS`, …) plus `WATER_BLOCKED` / `MOUNTAIN_BLOCKED` sentinels
- [x] `TerrainDatabase::merge(other: TerrainDatabase)` — inserts/overwrites by ID; enables campaign override/extension of built-ins
- [x] `ContentDatabase.terrain: TerrainDatabase` — always seeded with `builtin_terrain_db()`, merges campaign `data/terrain.ron` on load; `TerrainLoadError` variant added to `DatabaseError`
- [x] `GameData.terrain: TerrainDatabase` — seeded from `data/terrain.ron` (fallback to `builtin_terrain_db()`), merges campaign `data/terrain.ron` on load; `CampaignLoader::load_terrain()` private method added
- [x] `assets/textures/terrain/{sand,snow,ice}.png` — 64×64 placeholder PNGs (matching existing terrain texture resolution)
- [x] `data/test_campaign/data/terrain.ron` — test fixture with 1 override (ID 13001 → "Campaign Grass") and 1 new entry (ID 13100 → "Volcanic Ash")
- [x] Test `test_load_default_terrain_ron` — `data/terrain.ron` parses, has exactly 12 entries, all built-in IDs present, Water/Mountain blocked
- [x] Test `test_terrain_database_merge_campaign_override` — override of ID 13001 + addition of ID 13100; final count 13; other built-ins untouched
- [x] Test `test_campaign_loader_missing_terrain_ron_uses_builtin_only` — temp-dir campaign (no terrain.ron); ≥ 12 entries; Water/Mountain blocked
- [x] Test `test_campaign_loader_terrain_merge_from_fixture` — test_campaign; ≥ 13 entries; Volcanic Ash present; Grass overridden
- [x] Test `test_content_database_terrain_loads_all_builtin_ids` — test_campaign loads; ≥ 12 entries; all built-in IDs present
- [x] Test `test_content_database_new_has_empty_terrain` — `ContentDatabase::new()` has empty terrain database
- [x] All tests pass (5558 passed, 8 skipped, 0 failed)

### Files Changed

**`src/domain/world/terrain.rs`**
- Added `pub mod builtin` with 12 `TerrainId` aliases and 2 `bool` sentinels.
- Added `pub fn merge(&mut self, other: TerrainDatabase)` to `impl TerrainDatabase` after `has_definition`.
- Added `test_load_default_terrain_ron` and `test_terrain_database_merge_campaign_override` to `mod tests`.

**`src/sdk/database.rs`**
- Added `#[error("Failed to load terrain: {0}")] TerrainLoadError(String)` to `DatabaseError`.
- Added `use crate::domain::world::terrain::{builtin_terrain_db, TerrainDatabase}`.
- Added `pub terrain: TerrainDatabase` field to `ContentDatabase`.
- Updated `new()` (empty), `load_campaign_with_skills_file`, and `load_core` (both seed from `builtin_terrain_db()` then merge).
- Added two new tests.

**`src/domain/campaign_loader.rs`**
- Added `use crate::domain::world::terrain::TerrainDatabase` import.
- Added `pub terrain: TerrainDatabase` to `GameData` and its `new()` + doc example.
- Added private `fn load_terrain(&self) -> Result<TerrainDatabase, CampaignError>`.
- Called `load_terrain` from `load_game_data` after wind config.
- Added two new tests.

**`data/terrain.ron`** (new)
- 12 RON `TerrainDefinition` entries; IDs 13000–13011.

**`data/test_campaign/data/terrain.ron`** (new)
- 2 entries: override ID 13001 ("Campaign Grass") + new ID 13100 ("Volcanic Ash").

**`assets/textures/terrain/sand.png`**, **`snow.png`**, **`ice.png`** (new)
- 64×64 placeholder PNGs: sand (warm beige), snow (off-white), ice (light blue).

### Architecture Compliance

- ID range: built-ins at 13000–13011; campaign custom at 13100+ (convention).
- Merge semantics: campaign entry with existing ID overrides built-in; new ID adds.
- Missing campaign `terrain.ron` is not an error (opt-in per campaign).
- `ContentDatabase::new()` terrain is empty (consistent with other fields).
- `GameData` terrain is populated from `data/terrain.ron` or built-in fallback.
- All test data uses `data/test_campaign`, not `campaigns/tutorial`.

### Quality Gates

```
cargo fmt --all             → clean
cargo check --all-targets   → 0 errors
cargo clippy -- -D warnings → 0 warnings
cargo nextest run           → 5558 passed, 8 skipped, 0 failed
```

---

## Phase 2: Terrain Externalization — Replace `TerrainType` with `TerrainId` in Domain Types

### Summary

Removed the closed `TerrainType` enum entirely and replaced it with the open numeric `TerrainId = u32` system across all layers of the codebase — domain, game resources, game systems, and SDK. This is a breaking, non-backwards-compatible migration. All 15 map RON files (7 test-campaign + 8 tutorial) were migrated to use numeric terrain IDs. All 5552 tests pass.

### Deliverables

- [x] `TerrainId`, `TERRAIN_ID_MIN` already present from Phase 1
- [x] `TerrainType` enum **deleted** from `src/domain/world/types.rs`
- [x] `Tile.terrain: TerrainId` (was `TerrainType`)
- [x] `Tile.blocked` remains a stored, mutable `bool` field; seeding logic in `Tile::new` changed
- [x] `Tile::new` takes `&TerrainDatabase`; all call sites updated
- [x] `TileVisualMetadata::effective_height` takes `terrain_height: f32` instead of `TerrainType`
- [x] `TileVisualMetadata::mesh_dimensions` and `mesh_y_position` updated to match
- [x] `check_tile_blocked` / `Tile::is_blocked()` unchanged (no `db` parameter)
- [x] `TileCode` → built-in `TerrainId` mapping in `blueprint.rs`
- [x] `EncounterTable.terrain_modifiers: BTreeMap<TerrainId, f32>`
- [x] `TerrainType` removed from `mod.rs` re-exports
- [x] All domain-layer, game-layer, and SDK-layer tests pass
- [x] Map RON files migrated (Phase 6.1 inlined)

### Files Changed

**`src/domain/world/terrain.rs`**
- Added 12 built-in `TerrainId` constants (`TERRAIN_GROUND` = 13000 through `TERRAIN_ICE` = 13011).
- Added `builtin_terrain_db() -> TerrainDatabase` returning all 12 definitions pre-loaded (Water and Mountain have `blocked: true`).
- Added `builtin_terrain_definitions() -> Vec<TerrainDefinition>` helper.
- Added tests: `test_builtin_terrain_db_has_all_twelve_entries`, `test_builtin_terrain_db_water_and_mountain_are_blocked`, `test_terrain_new_seeds_blocked_from_db`.

**`src/domain/world/types.rs`**
- Deleted `TerrainType` enum entirely.
- `Tile.terrain` changed from `TerrainType` to `TerrainId`.
- `Tile::new` signature changed to `(x, y, terrain: TerrainId, wall_type: WallType, db: &TerrainDatabase) -> Self`; seeds `blocked` from `db.get_by_id(terrain).is_some_and(|d| d.blocked) || wall_type == WallType::Normal`.
- `Map::new()` and `Map::resize()` API unchanged; internally create tiles via struct literals using `TERRAIN_GROUND` (avoids requiring `&TerrainDatabase` at the API level).
- `TileVisualMetadata::effective_height(&self, wall_type: WallType, terrain_height: f32) -> f32`.
- `TileVisualMetadata::mesh_dimensions(&self, wall_type: WallType, terrain_height: f32) -> (f32, f32, f32)`.
- `TileVisualMetadata::mesh_y_position(&self, wall_type: WallType, terrain_height: f32) -> f32`.
- `EncounterTable.terrain_modifiers: BTreeMap<TerrainId, f32>`.
- All unit tests updated; added `test_tile_new_seeds_blocked_from_terrain_database` and `test_effective_height_uses_supplied_terrain_height`.

**`src/domain/world/blueprint.rs`**
- Removed `impl From<MapBlueprint> for Map`; replaced with `impl MapBlueprint { pub fn into_map(self, db: &TerrainDatabase) -> Map }`.
- `TileCode` mapping now emits built-in `TerrainId` constants (TERRAIN_GROUND through TERRAIN_MOUNTAIN).
- Module doc notes custom terrain requires full-map RON, not `TileCode`.
- Added `test_tile_code_maps_to_builtin_terrain_ids` test.
- Blueprint tests updated to use `bp.into_map(&builtin_terrain_db())`.

**`src/domain/world/mod.rs`**
- Removed `TerrainType` from `pub use types::{...}`.
- Extended `pub use terrain::{...}` to include all 12 `TERRAIN_*` constants, `builtin_terrain_db`, and `builtin_terrain_definitions`.

**`src/domain/world/movement.rs`**
- Updated doc comment and test to use `TERRAIN_WATER` constant instead of `TerrainType::Water`.

**`src/game/resources/terrain_material_cache.rs`** (Phase 4.1 inlined)
- Rewrote `TerrainMaterialCache` struct from nine named fields to `items: HashMap<TerrainId, Handle<StandardMaterial>>`.
- `get(terrain: TerrainId)` and `set(terrain: TerrainId, handle)` methods.
- `is_fully_loaded()` checks the nine built-in IDs 13000–13008.
- All tests updated to use `TERRAIN_*` constants.

**`src/game/systems/terrain_materials.rs`** (Phase 4.2 inlined)
- `texture_path_for(terrain: TerrainId) -> &'static str` — uses constant match with `_ => TEXTURE_GROUND` fallback.
- `roughness_for(terrain: TerrainId) -> f32` — uses constant match with `_ => 0.80` fallback.
- Startup and debug systems updated to use `TerrainId` arrays.

**`src/game/systems/map.rs`** (Phase 4.3/4.4 inlined)
- `should_spawn_grass_cover(terrain: TerrainId)` — matches on `TERRAIN_FOREST | TERRAIN_GRASS`.
- Added `terrain_height_for_id(terrain: TerrainId) -> f32` helper (Mountain=3.0, Forest=2.2, others=0.0).
- `terrain_material_with_optional_tint` takes `TerrainId`.
- All `TerrainType::*` match arms replaced with `TERRAIN_*` constants in `spawn_map`.
- All `mesh_dimensions` / `mesh_y_position` calls updated to pass `terrain_height_for_id(tile.terrain)`.
- Wall-tint color match gains `_ => floor_rgb` wildcard for campaign-defined terrain.
- All tests updated.

**`src/game/systems/hud.rs`**
- `automap_tile_color` uses `TERRAIN_WATER`, `TERRAIN_GRASS`, `TERRAIN_FOREST` constants.

**`src/game/systems/item_world_events.rs`**
- Terrain check in `spawn_dropped_item_system` uses `TERRAIN_GRASS | TERRAIN_FOREST`.

**`src/game/systems/vegetation_placement.rs`**
- All `TerrainType` references replaced with `TERRAIN_FOREST`, `TERRAIN_GRASS` constants.
- `should_plan_understory_shrubs(terrain: TerrainId, ...)`.
- Test helper `forest_tile()` uses `builtin_terrain_db()`.

**`src/game/systems/input/exploration_movement.rs`**
- Walk on Water check uses `t.terrain == TERRAIN_WATER`.

**`src/game/systems/exploration_spells.rs`**
- Test helper uses `Tile::new(3, 3, TERRAIN_MOUNTAIN, WallType::None, &db)`.

**`src/sdk/cli/map_builder.rs`** (Phase 5.1 inlined)
- `MapBuilder` struct gains `terrain_db: TerrainDatabase` field (initialized with `builtin_terrain_db()`).
- `set_tile` / `fill_tiles` take `TerrainId`; pass `&self.terrain_db` to `Tile::new`.
- `bulk_set_for_terrains` uses `Vec<TerrainId>`.
- `show_map` match uses `TERRAIN_*` constants with `_ => '?'` fallback.
- `parse_terrain(s: &str) -> TerrainId` accepts both names and numeric IDs; adds Sand/Snow arms.

**`src/sdk/cli/map_validator.rs`**
- Test helpers use `builtin_terrain_db()` and `TERRAIN_GRASS`.

**`src/sdk/templates.rs`** (Phase 5.3 inlined)
- `town_map`, `dungeon_map`, `forest_map` use `builtin_terrain_db()` and `TERRAIN_*` constants.

**Map RON files** (Phase 6.1 inlined)
- 7 test-campaign maps + 8 tutorial maps migrated: `terrain: Grass` → `terrain: 13001`, etc.

### Architecture Compliance

- `TerrainType` deleted; zero references remain in `src/`.
- `Tile.blocked` remains a stored, mutable field; `Tile::new` seeds it from `TerrainDatabase`.
- `check_tile_blocked` / `Tile::is_blocked()` take no `db` parameter (unchanged).
- `Map::new()` API unchanged (creates tiles directly to avoid requiring a database).
- `MapBlueprint::into_map` properly uses `&TerrainDatabase` for blocked seeding.
- All constant IDs are within the reserved 13000–13011 range.

### Quality Gates

```
cargo fmt --all             → clean
cargo check --all-targets   → 0 errors
cargo clippy -- -D warnings → 0 warnings
cargo nextest run           → 5552 passed, 0 failed
```

---

(`u32`) and `TERRAIN_*` constants in place of the removed `TerrainType` enum.
The cache is now a `HashMap<TerrainId, Handle<StandardMaterial>>` instead of a
struct with nine named `Option<Handle<StandardMaterial>>` fields, which allows
campaign-defined terrain IDs to be cached without engine changes.

### Files Changed

**`src/game/resources/terrain_material_cache.rs`**

- Replaced `use crate::domain::world::TerrainType` with `crate::domain::types::TerrainId`
  and the nine `TERRAIN_*` constants from `crate::domain::world::terrain`.
- Replaced nine named `Option<Handle<StandardMaterial>>` fields with a single
  `items: HashMap<TerrainId, Handle<StandardMaterial>>` field.
- `get(terrain: TerrainType)` → `get(terrain: TerrainId)`: now calls `HashMap::get`.
- `set(terrain: TerrainType, handle)` → `set(terrain: TerrainId, handle)`: now calls
  `HashMap::insert`.
- `is_fully_loaded()`: iterates over nine built-in `TERRAIN_*` IDs (Ground–Forest,
  13000–13007) using `HashMap::contains_key`.
- Tests: replaced `all_terrain_types()` helper returning `[TerrainType; 9]` with
  `all_terrain_ids()` returning `[TerrainId; 9]`; updated all assertions to use
  `TERRAIN_*` constants instead of `TerrainType::*` variants.

**`src/game/systems/terrain_materials.rs`**

- Replaced `use crate::domain::world::TerrainType` with `TerrainId` and the nine
  `TERRAIN_*` constants.
- `texture_path_for(terrain: TerrainType)` → `texture_path_for(terrain: TerrainId)`:
  match arms now use `TERRAIN_*` constants; added `_ => TEXTURE_GROUND` fallback
  for campaign-defined terrain.
- `roughness_for(terrain: TerrainType)` → `roughness_for(terrain: TerrainId)`:
  match arms use `TERRAIN_*` constants; added `_ => 0.80` fallback.
- `load_terrain_materials_system`: `terrain_types` array renamed to `terrain_ids`
  with type annotation `[TerrainId; 9]`.
- `all_terrain_types()` helper renamed to `all_terrain_ids()` and returns
  `[TerrainId; 9]`.
- `debug_terrain_texture_bindings_system`: `terrain_types` array renamed to
  `terrain_ids: [TerrainId; 9]`; log format strings updated from `{:?}` to `{}`.
- Tests: all `texture_path_for(TerrainType::*)` calls updated to `TERRAIN_*`
  constants; `roughness_for` tests updated similarly; loop helpers use
  `all_terrain_ids()`.

---

## Phase 2: Terrain Externalization — Game System Migration

### Summary

Updated 6 game-system files to use `TerrainId` (a `u32` type alias) and built-in
`TERRAIN_*` constants in place of the removed `TerrainType` enum. Added the
`terrain_height_for_id` helper to `map.rs` as a temporary bridge until the
`TerrainDatabase` is available as a Bevy resource (Phase 3).

### Files Changed

**`src/game/systems/map.rs`**

- Added `use crate::domain::world::terrain::{TERRAIN_*}` import at module level.
- Changed `should_spawn_grass_cover` parameter from `world::TerrainType` to
  `crate::domain::types::TerrainId`; body now uses `TERRAIN_FOREST | TERRAIN_GRASS`.
- Added `terrain_height_for_id(terrain: TerrainId) -> f32` helper (Mountain → 3.0,
  Forest → 2.2, others → 0.0).
- Changed `terrain_material_with_optional_tint` first parameter to `TerrainId`.
- Replaced all `terrain_cache.get(world::TerrainType::*)` calls with `TERRAIN_*`
  constants in `spawn_map`.
- Replaced exhaustive `TerrainType` match arms in the terrain and wall-tint
  dispatches with `TERRAIN_*` constants; added `_ => floor_rgb` fallback arm.
- Changed `tile.visual.mesh_dimensions(tile.terrain, tile.wall_type)` →
  `tile.visual.mesh_dimensions(tile.wall_type, terrain_height_for_id(tile.terrain))`
  in Mountain branch and both wall branches (Normal, Torch).
- Changed `tile.visual.mesh_y_position` calls analogously.
- Updated all test cases: added terrain constant imports to the test module;
  updated `Tile::new` calls to use `TERRAIN_*` constants and `builtin_terrain_db()`;
  updated `cache.set`/`get` and `terrain_material_with_optional_tint` calls.

**`src/game/systems/hud.rs`**

- `automap_tile_color`: replaced `use crate::domain::world::{TerrainType, WallType}`
  with `use crate::domain::world::terrain::{TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_WATER}`
  and `use crate::domain::world::WallType`; updated match arms.
- `automap_tests` module: replaced `use crate::domain::world::{Map, TerrainType}`
  with separate `Map` and `TERRAIN_GROUND` imports; changed
  `tile.terrain = TerrainType::Ground` to `tile.terrain = TERRAIN_GROUND`.

**`src/game/systems/item_world_events.rs`**

- Replaced `use crate::domain::world::{MapEvent, TerrainType}` with separate
  `MapEvent` import and new `use crate::domain::world::terrain::{TERRAIN_FOREST,
  TERRAIN_GRASS, TERRAIN_GROUND}`.
- Updated `spawn_dropped_item_system`: `.unwrap_or(TerrainType::Ground)` →
  `.unwrap_or(TERRAIN_GROUND)`; match arms updated to `TERRAIN_GRASS | TERRAIN_FOREST`.

**`src/game/systems/vegetation_placement.rs`**

- Updated imports: replaced `TerrainType` with `TerrainId` and terrain constants.
- Updated `tile_vegetation_plan`: `tile.terrain == TerrainType::Forest` →
  `tile.terrain == TERRAIN_FOREST`.
- Updated `supports_vegetation_cover`: `matches!(... TerrainType::Forest | TerrainType::Grass)`
  → `matches!(... TERRAIN_FOREST | TERRAIN_GRASS)`.
- Updated `should_plan_understory_shrubs` parameter from `TerrainType` to `TerrainId`;
  body uses `TERRAIN_FOREST`.
- Updated doc-comment examples to use `builtin_terrain_db()` and `TERRAIN_*` constants.
- Test module: updated `forest_tile()` to call `builtin_terrain_db()` and pass `&db`
  to `Tile::new`.

**`src/game/systems/input/exploration_movement.rs`**

- Replaced `TerrainType` import with `crate::domain::world::terrain::TERRAIN_WATER`.
- `should_override_water`: changed `matches!(t.terrain, TerrainType::Water)` →
  `t.terrain == TERRAIN_WATER`.
- Test `make_world_with_water_tile`: replaced `TerrainType::Water` with `TERRAIN_WATER`.

**`src/game/systems/exploration_spells.rs`**

- Test `test_jump_target_invalid_for_blocked_tile`: replaced `TerrainType::Mountain`
  with `TERRAIN_MOUNTAIN`; added `builtin_terrain_db()` call; updated `Tile::new`
  signature to include `&db`.

---

## Phase 2: Terrain Externalization — SDK Migration

### Summary

Updated SDK files to use `TerrainId` (a `u32` type alias) and built-in
`TERRAIN_*` constants instead of the deleted `TerrainType` enum. All
`Tile::new` calls now pass a `&TerrainDatabase` so the constructor can
seed the tile's `blocked` flag from the data-driven terrain registry.

### Files Changed

**`src/sdk/cli/map_builder.rs`**

- Replaced `use crate::domain::world::TerrainType` import with
  `TerrainId` from `crate::domain::types` and terrain constants/helpers
  from `crate::domain::world::terrain`.
- Added `terrain_db: TerrainDatabase` field to `MapBuilder`; initialised
  via `builtin_terrain_db()` in `MapBuilder::new()`.
- Updated `set_tile` and `fill_tiles` signatures: `terrain: TerrainType`
  → `terrain: TerrainId`; `Tile::new` calls now forward `&self.terrain_db`.
- Updated `bulk_set_for_terrains`: `Vec<TerrainType>` → `Vec<TerrainId>`.
- Updated `show_map` terrain match from `TerrainType` enum arms to
  `TERRAIN_*` constant arms; added `TERRAIN_SAND`, `TERRAIN_SNOW`, and a
  wildcard `_ => '?'` arm for campaign-defined terrain.
- Replaced `parse_terrain` with a new version that accepts numeric ID
  strings (e.g. `"13001"`) for custom campaign terrain, and adds
  `"sand"` and `"snow"` arms.
- Updated all test assertions to use `TERRAIN_*` constants; added
  explicit terrain imports to the test module.

**`src/sdk/cli/map_validator.rs`**

- Updated three test helpers (`test_validate_structure_oversized_map`,
  `make_minimal_map`, `make_map_with_all_event_variants`):
  - Replaced local `use crate::domain::world::TerrainType` with
    `use crate::domain::world::terrain::{builtin_terrain_db, TERRAIN_GRASS}`.
  - Created a single `db = builtin_terrain_db()` per helper function and
    passed `&db` to all `Tile::new` calls.

**`src/sdk/templates.rs`**

- Replaced `use crate::domain::world::TerrainType` with
  `crate::domain::world::terrain::{builtin_terrain_db, TERRAIN_FOREST,
  TERRAIN_GRASS, TERRAIN_STONE}`.
- Updated `town_map`, `dungeon_map`, and `forest_map`: each now creates
  `db = builtin_terrain_db()` once before the tile iterator and passes
  `&db` to `Tile::new`.

---

## Phase 1: Terrain Externalization — Domain Foundation

### Summary

Implemented the foundational `TerrainDefinition` / `TerrainDatabase` registry
(Phase 1 of the terrain externalization plan). This is a purely additive change —
no existing code is modified, no existing tests break. It establishes the
numeric-ID open registry that future phases will use to replace the closed
`TerrainType` enum.

### Files Changed

**`src/domain/types.rs`**

- Added `pub type TerrainId = u32;` alongside `LandscapeId`.
- Added `pub const TERRAIN_ID_MIN: TerrainId = 13_000;` alongside
  `LANDSCAPE_ID_MIN`. Built-in terrain occupies `13000`–`13011`; campaign
  custom terrain starts at `13100` by convention.
- Both items carry full `///` doc-comments with `# Examples` doctests.

**`src/domain/world/terrain.rs`** *(new file)*

Contains the full domain foundation for data-driven terrain:

- `TerrainDatabaseError` (`thiserror`): `ReadError`, `ParseError`,
  `DuplicateId`, `NotFound`, `InvalidTerrainId { id, min }`.
- `TerrainMeshStyle { Flat, Water, Mountain }` — closed enum selecting the
  map-renderer mesh-spawn branch. `Flat` is `#[default]`. Includes `all()`
  const accessor.
- `TerrainVegetation { None, GrassCover, Forest }` — closed enum controlling
  vegetation decoration. `None` is `#[default]`. Includes `all()` const
  accessor.
- `TerrainDefinition` — RON-deserializable struct: required fields `id`,
  `name`, `texture_path`, `roughness`, `color`; optional (with `#[serde(default)]`)
  fields `mesh_style`, `vegetation`, `blocked`, `height`.
- `TerrainDatabase { items: HashMap<TerrainId, TerrainDefinition> }` — in-memory
  index populated via `crate::impl_ron_database!` with `post_load` ID validation
  (`id >= TERRAIN_ID_MIN`). Methods: `new`, `add`, `get_by_id`, `get_by_name`,
  `all_definitions`, `len`, `is_empty`, `has_definition`.
- All public items carry `///` doc-comments with `# Examples` doctests.
- 13 unit tests covering: add-and-lookup, duplicate-ID rejection,
  below-min rejection, full RON roundtrip, mesh-style / vegetation RON
  roundtrip for all variants, minimal RON with default fields, full RON entry,
  missing lookups, defaults, and error message content.

**`src/domain/world/mod.rs`**

- Declared `pub mod terrain;`.
- Added re-exports: `TerrainDatabase, TerrainDatabaseError, TerrainDefinition,
  TerrainMeshStyle, TerrainVegetation`.
- Updated module-level doc comment to list the `terrain` sub-module.

### Architecture Compliance

- `TERRAIN_ID_MIN = 13_000` placed in `domain/types.rs` alongside
  `LANDSCAPE_ID_MIN`, exactly per Section 4.6 of the plan.
- Module placed in `src/domain/world/terrain.rs`, mirroring `landscape.rs`.
- `impl_ron_database!` macro used unchanged, mirroring `LandscapeDatabase`.
- All type aliases use `TerrainId = u32` (not raw `u32`).
- No magic numbers; constants used throughout.

### Quality Gates

```
cargo fmt --all            → clean
cargo check --all-targets  → Finished, 0 errors
cargo clippy -- -D warnings → Finished, 0 warnings
cargo nextest run          → 5539 passed, 0 failed
```

---

## Phase 4: Tutorial — Eonir the Still Boss Fight

### Summary

Wired the generic `NpcCombatSwitch` system into the tutorial campaign to create
the Eonir the Still boss encounter. Both a combat path (fight the Lich King) and
a peaceful path (negotiate the return of the jade relic) are fully playable and
both converge on the Act V quest step. Added `suppress_flag` to `NpcDefinition`
for the peaceful-exit spawn suppression path.

Full lifecycle:
1. Party approaches Eonir → dialogue 1004 opens.
2. Combat path: choose *"I will not bargain with a lich."* (or node 4 equivalent)
   → `SetFlag("eonir_combat_triggered")` fires → NPC despawns → Lich King Eonir
   (monster 137) combat starts → party wins → `eonir_defeated` flag set.
3. Peaceful path: choose *"Return the relic to Jyeshtha."* → `SetFlag("eonir_relic_returned")`
   + `CompleteQuestStage(9, 1)` fire → Eonir never reappears (suppress_flag guard).
4. Both paths advance Quest 9 stage 1; Act V (return relic to Jyeshtha) follows.

### Files Changed

**`src/domain/world/npc.rs`**

- Added `pub suppress_flag: Option<String>` with `#[serde(default)]` to
  `NpcDefinition` (after `combat_switch`). Suppresses NPC spawn when the named
  GlobalFlag is true — used for the peaceful-exit path so Eonir never reappears
  after the relic is returned.
- Updated the doctest struct literal and all 5 constructors (`new`, `merchant`,
  `priest`, `innkeeper`, `trainer`) to include `suppress_flag: None`.
- Updated all other struct-literal usages across the codebase.
- New tests:
  - `test_npc_definition_suppress_flag_defaults_to_none`
  - `test_npc_definition_suppress_flag_round_trip`

**`src/game/systems/map.rs`**

- Extended the NPC spawn guard block from one check to two:
  - Existing: combat-switch trigger flag guard (Phase 2).
  - New: peaceful-exit `suppress_flag` guard — skips spawning when
    `npc_def.suppress_flag` is set and the matching GlobalFlag is true.

**`src/sdk/database.rs`**

- New test: `test_test_campaign_combat_switch_npc_deserialises` — loads
  `data/test_campaign/data/npcs.ron` and asserts `test_combat_switch_npc` has
  `combat_switch.is_some()` with correct field values.

**`data/test_campaign/data/npcs.ron`**

- Added `test_combat_switch_npc` fixture NPC with a fully populated
  `combat_switch` (trigger `"test_switch_trigger"`, monster 1, defeat flag
  `"test_switch_defeated"`, type Normal). Powers Phase 2–4 integration tests
  without depending on live tutorial campaign content.

**`campaigns/tutorial/data/monsters.ron`**

- Added `id: 137` — **Lich King Eonir** (boss). Stats: 400 HP, AC 25,
  magic_resistance 95, speed 4 ("the Still"), can_regenerate, is_undead.
  Two attacks: Physical/Drain (melee) and Energy/Paralysis (ranged).
  Full elemental resistance suite. `creature_id: Some(1021)` matches the NPC
  visual. Loot includes item 210 (jade icosahedron, 80% drop chance).

**`campaigns/tutorial/data/npcs.ron`**

- Updated `tutorial_lich_eonir` — added:
  - `combat_switch: Some(...)` with `trigger_flag: "eonir_combat_triggered"`,
    `monster_id: 137`, `defeat_flag: Some("eonir_defeated")`, `combat_type: Boss`.
  - `suppress_flag: Some("eonir_relic_returned")` for peaceful-exit suppression.

**`campaigns/tutorial/data/dialogues.ron`**

- Dialogue 1004 "The Sovereign's Soliloquy" restructured from 4 nodes to 6:
  - Node 1: gains 4th choice "I will not bargain with a lich." (direct combat).
  - Nodes 2 & 3: converted from terminal to non-terminal; each leads to node 4.
  - Node 4: converted from terminal to non-terminal; two new choices:
    - **Peaceful** → node 6 — fires `GiveItems([(210, 1)])` (awards the jade relic),
      `SetFlag("eonir_relic_returned", true)`, `StartQuest(9)`,
      `CompleteQuestStage(9, 1)`, `CompleteQuestStage(8, 1)` (force-advances
      Quest 8 Stage 1 so Quest 8 Stage 2 "Return to Jyeshtha" activates for
      both combat and peaceful paths — the Act V convergence point).
    - **Combat** → node 5 — fires `StartQuest(9)`, `SetFlag("eonir_combat_triggered", true)`.
  - Node 5 (new): Eonir's combat declaration (terminal).
  - Node 6 (new): Eonir surrenders the relic (terminal).

**`campaigns/tutorial/data/quests.ron`**

- Added Quest 9 "Confront the Lich King" (`is_main_quest: true`, `min_level: 8`).
  - Stage 1: `KillMonsters(137, 1)` — satisfied automatically by combat path;
    peaceful path uses `CompleteQuestStage(9, 1)` dialogue action to bypass it.
  - Rewards: 2000 XP + 1000 gold.
- Act V convergence: both paths activate Quest 8 Stage 2 ("Return the Relic to
  Jyeshtha"). Combat path: monster 137 drops item 210 (80% loot) → Quest 8 Stage 1
  auto-completes. Peaceful path: `GiveItems([(210,1)])` + `CompleteQuestStage(8, 1)`
  force-advances Quest 8 Stage 1 directly. Both paths then share Quest 8 Stage 2.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5526 passed, 8 skipped, 0 failed
- **Post-audit fix**: Added `GiveItems([(210,1)])` + `CompleteQuestStage(8,1)` to peaceful
  dialogue choice — both paths now converge on Quest 8 Stage 2 (Act V).

---

## Phase 3: Post-Combat Hook — Defeat Flag

### Summary

Wired the `defeat_flag` from `PendingNpcCombat` all the way through to
`GlobalFlags` after a victory. When an NPC combat-switch encounter is won,
`handle_combat_victory` now reads `CombatResource::defeat_flag` and sets the
corresponding flag in `GlobalFlags`, completing the full lifecycle:
dialogue sets trigger flag → NPC despawns → combat starts → party wins
→ defeat flag set.

### Files Changed

**`src/game/systems/combat.rs`**

- Added `pub defeat_flag: Option<String>` field to `CombatResource` with doc
  comment explaining its lifecycle.
- `CombatResource::new()` initialises `defeat_flag: None`.
- `CombatResource::clear()` resets `defeat_flag = None`.
- `handle_combat_victory` — just before `global_state.0.exit_combat()`, checks
  `combat_res.defeat_flag` and calls `global_flags.set(flag, true)` if set,
  then clears `defeat_flag = None` to prevent double-setting.
- Three new tests (all pass):
  - `test_combat_resource_clear_resets_defeat_flag`
  - `test_handle_combat_victory_sets_defeat_flag_when_present`
  - `test_handle_combat_victory_no_defeat_flag_does_not_panic`

**`src/game/systems/npc_combat_switch.rs`**

- `start_npc_combat_system` — added
  `mut combat_res: Option<ResMut<crate::game::systems::combat::CombatResource>>`
  as last parameter (optional so test apps without `CombatPlugin` still work).
- Replaced the Phase 2 `// TODO Phase 3:` comment with:
  `if let Some(ref mut cr) = combat_res { cr.defeat_flag = combat.defeat_flag.clone(); }`
- New test (passes):
  - `test_combat_resource_defeat_flag_set_on_npc_combat_start`

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5523 passed, 8 skipped, 0 failed

---

## Phase 2: Exploration Engine — NPC Spawn Guard + Combat Switch System

### Summary

Added the `PendingNpcCombatResource`, `npc_combat_switch_system`, and
`start_npc_combat_system` that together provide the generic flag-triggered
NPC → monster-encounter swap at runtime.  Also added an NPC spawn guard in
`spawn_map` so a "defeated" NPC never re-spawns after save/reload.

### Files Changed

**`src/game/systems/npc_combat_switch.rs`** (new file)

- `PendingNpcCombat` — data struct holding monster ID, tile position, map ID,
  defeat flag, and combat type.
- `PendingNpcCombatResource(pub Option<PendingNpcCombat>)` — Bevy `Resource`
  newtype wrapper; follows the `PendingRecruitConfirm` pattern from `events.rs`.
- `npc_combat_switch_system` — scans live `NpcMarker` entities each exploration
  frame; when `combat_switch.trigger_flag` is `true` in `GlobalFlags`, despawns
  the entity and writes `PendingNpcCombatResource`.
- `start_npc_combat_system` — consumes `PendingNpcCombatResource`, calls
  `start_encounter(&[monster_id])`, and emits `CombatStarted`.
- `NpcCombatSwitchPlugin` — registers the resource and both systems gated to
  `in_exploration_mode`, with `start_npc_combat_system` ordered after
  `npc_combat_switch_system`.
- Phase 2 tests (all pass):
  - `test_npc_combat_switch_system_skips_npc_without_combat_switch`
  - `test_npc_combat_switch_system_skips_when_trigger_flag_not_set`
  - `test_npc_combat_switch_system_despawns_npc_when_flag_set`
  - `test_start_npc_combat_system_clears_pending_and_enters_combat`
  - `test_npc_spawn_guard_suppresses_npc_when_trigger_flag_already_set`

**`src/game/run_conditions.rs`**

- Added `in_exploration_mode` run condition (mirrors `in_combat_mode`).

**`src/game/systems/mod.rs`**

- Added `pub mod npc_combat_switch;`.

**`src/game/systems/map.rs`** — `spawn_map` function

- NPC spawn guard inserted immediately after the `encountered_characters` check
  in the resolved-NPC loop. If the NPC's `combat_switch.trigger_flag` is already
  `true` in `GlobalFlags`, the NPC is skipped with a `debug!` log — preventing
  re-appearance after a save/reload.

**`src/bin/antares.rs`**

- Registered `NpcCombatSwitchPlugin` after `CombatPlugin`.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5519 passed, 8 skipped, 0 failed

---

## Phase 1: Domain — `NpcCombatSwitch` Data Structure

### Summary

Added the `NpcCombatSwitch` struct and `combat_switch` field to `NpcDefinition`
as the foundational domain layer for the generic NPC → monster-encounter swap
feature described in `npc_combat_trigger_implementation_plan.md`.

### Files Changed

**`src/domain/world/npc.rs`**

- Added `use crate::domain::types::MonsterId` to the existing type-alias import.
- Added `pub struct NpcCombatSwitch` immediately before `NpcDefinition`:
  - `trigger_flag: String` — `GlobalFlags` key that activates the swap
  - `monster_id: MonsterId` — monster entry to fight
  - `defeat_flag: Option<String>` — flag set after party victory (`#[serde(default)]`)
  - `combat_type: CombatEventType` — encounter type, defaults to `Normal` via `#[serde(default)]`
  - Derives: `Debug, Clone, Serialize, Deserialize, PartialEq`
- Added `pub combat_switch: Option<NpcCombatSwitch>` field at the end of
  `NpcDefinition` with `#[serde(default)]`; existing RON files deserialize
  without change.
- Updated all five `NpcDefinition` constructors (`new`, `merchant`, `priest`,
  `innkeeper`, `trainer`) to initialise `combat_switch: None`.
- Updated all `NpcDefinition { ... }` struct-literal doctests in the file to
  include `combat_switch: None`.
- Added three required tests:
  - `test_npc_definition_combat_switch_defaults_to_none` — minimal RON without
    the field deserialises to `None`
  - `test_npc_combat_switch_round_trip` — full struct serialises and
    deserialises to an equal value
  - `test_npc_combat_switch_defaults_combat_type_to_normal` — RON block with
    only `trigger_flag` and `monster_id` deserialises with
    `combat_type == Normal`

**Other files (struct-literal updates only)**

`combat_switch: None` added to every `NpcDefinition { ... }` struct literal
across the codebase (no logic changes):

- `src/game/systems/events.rs` — 5 literals
- `src/domain/world/types.rs` — 4 literals
- `src/domain/world/blueprint.rs` — 2 literals
- `src/domain/world/creature_binding.rs` — 1 literal
- `src/sdk/database.rs` — 1 literal
- `sdk/campaign_builder/src/asset_manager.rs` — 5 literals
- `sdk/campaign_builder/tests/campaign_io_tests.rs` — 9 literals
- `tests/campaign_integration_tests.rs` — 1 literal

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5514 passed, 8 skipped, 0 failed
  - `test_npc_definition_combat_switch_defaults_to_none`: PASS
  - `test_npc_combat_switch_round_trip`: PASS
  - `test_npc_combat_switch_defaults_combat_type_to_normal`: PASS

---

### Summary

Comprehensive audit of all four phases against the implementation plan's
deliverables checklist. Four gaps found and resolved:

### Fixes Applied

**`data/test_campaign/data/maps/map_1.ron`** (Phase 2.3 missed)

- The Phase 2 migration only updated `campaigns/tutorial/data/maps/map_1.ron`.
  The test campaign map at position `(17, 12)` still stored
  `mesh_id: Some("barred_passage")` while the registry already used numeric ID
  `12006` — a semantic inconsistency that would cause runtime mesh lookup to fail.
- Fixed: `mesh_id: Some("barred_passage")` → `mesh_id: Some("12006")`.

**`antares/tests/barred_passage_integration_test.rs`** (Phase 2.3 missed)

- Updated all assertions and doc comments that referenced `"barred_passage"`
  as the mesh ID to use `"12006"` (the numeric registry ID).
- `test_barred_passage_event_has_mesh_id`: assertion updated to `Some("12006")`.

**`sdk/campaign_builder/src/map_editor.rs`** (Phase 4.6 missed)

- `test_event_editor_state_to_treasure_with_mesh_and_dialogue`:
  `treasure_mesh_id: "barred_passage"` → `"12002"` (numeric id-string) per
  the plan: *"Existing map event editor tests that assert
  `treasure_mesh_id == "barred_passage"` must be updated to `"12002"`"*.

**`sdk/campaign_builder/src/objects_editor.rs`** (Phase 4.7 missed)

- Added `test_apply_edit_renames_name_and_updates_registry` (the specific test
  name required by Phase 4.7 testing requirements). The previously-added
  `test_apply_edit_allows_duplicate_names` covered the same concept but used a
  different name; the plan-specified test is now also present.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5511 passed, 8 skipped, 0 failed

---

## Phase 4: Object Mesh Registry Refactor — SDK Objects Editor and Map Editor Mesh Picker

### Summary

Completed the final phase of the object mesh registry refactor. The map editor's
mesh picker now displays `(id, name)` pairs (e.g. `"12001 — Ironbound Treasure
Chest"`) instead of raw ID strings, and duplicate registry names are permitted
since the numeric ID is now the unique key.

### Files Changed

**`src/domain/world/object_mesh.rs`**

- `ObjectMeshDatabase` struct: added `names: HashMap<String, String>` field
  (maps ID string → human-readable display name from the registry entry).
- `new()`: initialises `names: HashMap::new()`.
- `load_from_registry`: inserts into `db.names` alongside `db.meshes`.
- `merge_landscape` / `merge_furniture`: clone `key` before passing to
  `meshes.entry(key)` (was a move); add `names.entry(key).or_insert_with(…)`
  so landscape/furniture names are also available.
- New public method `all_mesh_ids_with_names() -> Vec<(String, String)>`.
- New test `test_all_mesh_ids_with_names_round_trip`.

**`src/sdk/map_editor.rs`**

- `browse_event_mesh_ids`: return type changed `Vec<String>` → `Vec<(String, String)>`;
  implementation now calls `db.all_mesh_ids_with_names()` and sorts numerically
  (numeric IDs first, then lexicographic).
- Updated `test_browse_event_mesh_ids_empty_db`.
- Added `test_browse_event_mesh_ids_returns_id_and_name_pairs`.
- Added `test_browse_event_mesh_ids_sorts_numeric_ids_ascending`.

**`sdk/campaign_builder/src/objects_editor.rs`**

- `ObjectsEditorState`: renamed field `key_buffer` → `name_buffer` with updated
  doc comment.
- `reset_for_new_campaign`, `reset_selection`, `enter_edit`: references updated.
- `apply_edit`: removed name-collision check (duplicate names are now allowed
  since IDs are unique); renamed internal variable `new_key` → `new_name`.
- `show_edit` grid: added read-only `"ID:"` row showing `entry.id`; renamed
  `"Key:"` row to `"Name:"` editing `name_buffer`; renamed second `"Name:"` to
  `"Mesh Name:"` (for the `CreatureDefinition.name` in the asset file).
- `show_edit` Save button: captures `entry_id: u32` before `apply_edit`,
  passes it to `sync_object_mesh_registry_entry`.
- `sync_object_mesh_registry_entry`: signature simplified from
  `(campaign_dir, old_key: Option<&str>, new_key: &str, file_path)` to
  `(campaign_dir, id: u32, new_name: &str, file_path)`; body now just calls
  `registry.upsert(id, new_name, file_path)` (no name-based lookup needed).
- Tests: `key_buffer` → `name_buffer` in all assertions; `test_apply_edit_rejects_rename_to_existing_key`
  flipped to expect success; `test_write_and_sync_round_trip` updated call;
  added `test_objects_editor_enter_edit_populates_name_buffer_not_key_buffer`
  and `test_apply_edit_allows_duplicate_names`.

**`sdk/campaign_builder/src/ui_helpers/autocomplete.rs`**

- `autocomplete_mesh_id_selector`: parameter `available_mesh_ids: &[String]` →
  `&[(String, String)]`; candidates now formatted as `"12001 — Name"`; buffer
  initialises to the full display string; commit accepts full display string or
  bare ID and stores only the ID; tooltip shows `"Mesh: id — Name"`.

**`sdk/campaign_builder/src/map_editor.rs`**

- `MapsEditorState.available_mesh_ids`: type `Vec<String>` → `Vec<(String, String)>`.
- `show_inspector_panel` and `show_event_editor`: parameter type
  `available_mesh_ids: &[String]` → `&[(String, String)]`.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5511 passed, 8 skipped, 0 failed

---

## Phase 3: Object Mesh Registry Refactor — SDK Data Model

### Summary

Upgraded the SDK data model so `ObjectEntry` carries a numeric `id: u32` and
`name: String` matching the `ObjectMeshEntry` domain type, replacing the old
free-form `key: String`. Updated `load_objects` and `save_objects` in
`campaign_io.rs` to read/write `id`+`name` throughout. All SDK tests and all
editor logic updated to use `entry.name` and `entry.id` instead of `entry.key`.

### Files Changed

**`sdk/campaign_builder/src/objects_editor.rs`**

- `ObjectEntry`: removed `pub key: String`; added `pub id: u32` and
  `pub name: String`; updated struct doc comment and doctest.
- `enter_edit`: `entry.key` → `entry.name` for `key_buffer` init.
- `apply_edit`: collision predicate, assignment, and error messages updated
  from "key" to "name".
- `show_list`: `push_id(&entry.key, …)` → `push_id(entry.id, …)` (numeric ID
  is now the stable egui identifier); badge shows `&entry.name`.
- `show_edit` Save block: `old_key` and sync call use `entry.name`.
- `filtered_rows`: searches `entry.name` instead of `entry.key`.
- `show_object_preview`: replaced `"Key: {entry.key}"` with
  `"ID: {entry.id}"` + `"Name: {entry.name}"`.
- `load_object_entries_from_registry`: removed Phase 1 `key` workaround;
  `ObjectEntry` constructed with `id: entry_ref.id, name: entry_ref.name`.
- Tests: `object_entry` helper updated; all `entry.key` assertions updated to
  `entry.name`; registry upserts use IDs 12001/12002 with descriptive names.

**`sdk/campaign_builder/src/campaign_io.rs`**

- `load_objects`: removed `entry_ref`/`key` intermediaries; constructs
  `ObjectEntry { id: entry.id, name: entry.name.clone(), … }`.
- `save_objects`: replaced `enumerate()` + `idx as u32` + `entry.key` with
  `registry.upsert(entry.id, &entry.name, &entry.file_path)`.
- All tests updated: `ObjectEntry` constructions use `id`+`name`; upsert
  calls use real IDs (12001/12002); assertions check `entry.id` and
  `entry.name` instead of `entry.key`.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5508 passed, 8 skipped, 0 failed

---

## Phase 2: Object Mesh Registry Refactor — Remove Legacy Fallback

### Summary

Converted all object mesh registry data to the new array-of-entries format and
removed the legacy `ObjectMeshRegistry(meshes: {...})` fallback from
`src/domain/world/object_mesh.rs`. All entries are now keyed exclusively by
`entry.id.to_string()` in `ObjectMeshDatabase`; name-based string keys are gone.

### Files Changed

**`src/domain/world/object_mesh.rs`**

- Removed `BTreeMap` from imports (`std::collections::HashMap` only).
- Deleted `ObjectMeshRegistryLegacy { meshes: BTreeMap<String, String> }` private
  struct (was the fallback deserialize target).
- Removed the four-line "Dual-format support" paragraph from the module-level doc
  comment.
- Simplified `load()`: direct `ron::from_str::<Vec<ObjectMeshEntry>>` — no
  try-then-fallback, no `ron::Value` intermediary.
- Updated `load()` doc comment: removed the `## Format detection` section that
  described both formats; now only documents the array format.
- Updated `load_from_registry()` doc comment: replaced the `**Key selection**`
  paragraph with a single line (`Each entry is keyed by entry.id.to_string()`).
- Simplified key selection in `load_from_registry()` loop: removed the
  `if entry.id > 0 { ... } else { ... }` branch; now always `entry.id.to_string()`.
- Test `test_object_mesh_registry_file_load_legacy_named_format` renamed to
  `test_load_legacy_format_returns_error_after_removal`; assertions flipped to
  confirm the legacy format now returns `ObjectMeshError::ParseError`.
- Test `test_object_mesh_registry_file_load_test_campaign_fixture` tightened:
  asserts exactly 6 entries and first entry id == 12001.
- New test `test_campaign_loader_object_meshes_keyed_by_numeric_id`: verifies
  `has_mesh("12001")` passes and `has_mesh("oak_tree")` fails against the test
  campaign fixture.

**`src/sdk/database.rs`**

- `test_object_mesh_registry_loads_from_primary_file`: updated written registry
  from legacy `ObjectMeshRegistry(meshes: {...})` format to the new array format
  `[(id: 12001, name: "barrel", filepath: "...")]`; updated assertion from
  `has_mesh("barrel")` to `has_mesh("12001")`.

**`tests/barred_passage_integration_test.rs`**

- `test_barred_passage_mesh_registered_in_object_mesh_registry`: updated
  assertion from `has_mesh("barred_passage")` to `has_mesh("12006")` (the
  numeric ID of the Barred Passage entry in the test campaign fixture).

### Key Design Decision: Legacy Fallback Removed

With all data files now in the new array format, the `ron::Value`-based
legacy fallback and `ObjectMeshRegistryLegacy` are dead code. Removing them
simplifies `load()` to a direct parse and makes the error surface cleaner —
any file not in `Vec<ObjectMeshEntry>` shape now immediately returns a
`ParseError` rather than silently synthesizing `id: 0` entries.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features -E 'test(object_mesh) | test(barred_passage)'`:
  38/38 passed

---

## Phase 1: Object Mesh Registry Refactor — Domain Type Changes

### Summary

Rewrote `src/domain/world/object_mesh.rs` to introduce `ObjectMeshEntry` as
the new first-class domain type for registry entries, replacing the old
`BTreeMap<String, String>` storage in `ObjectMeshRegistryFile`. Added dual-
format load support so legacy `ObjectMeshRegistry(meshes: {...})` files
continue to load transparently alongside the new array-of-entries format.

### Files Changed

**`src/domain/world/object_mesh.rs`**

- Added `pub struct ObjectMeshEntry { id: u32, name: String, filepath: String }`
  with `Debug, Clone, Serialize, Deserialize, PartialEq` derives.
- Replaced `ObjectMeshRegistryFile { meshes: BTreeMap<String, String> }` with
  `ObjectMeshRegistryFile { entries: Vec<ObjectMeshEntry> }` (sorted by `id`).
- Removed private `struct ObjectMeshRegistry`; added private
  `struct ObjectMeshRegistryLegacy { meshes: BTreeMap<String, String> }` for
  the legacy fallback path.
- New `upsert(&mut self, id: u32, name: &str, filepath: &str)`: inserts or
  replaces by `id`; re-sorts by `id` on insert.
- New `rename(&mut self, id: u32, new_name: &str) -> bool`: mutates `name`
  field of matching entry in-place.
- New `remove(&mut self, id: u32) -> Option<ObjectMeshEntry>`: removes and
  returns entry by `id`.
- `save()` now serializes `&self.entries` (plain array) directly via
  `ron::ser::to_string_pretty`.
- `load()` dual-format: tries `ron::from_str::<Vec<ObjectMeshEntry>>` first;
  on failure parses via `ron::Value` and deserializes as
  `ObjectMeshRegistryLegacy`, synthesizing `ObjectMeshEntry { id: 0, name: key,
  filepath: path }` per legacy map entry.
- `ObjectMeshDatabase::load_from_registry` now delegates to
  `ObjectMeshRegistryFile::load()`. Key selection: `entry.id > 0` → use
  `entry.id.to_string()`; `entry.id == 0` (legacy sentinel) → use `entry.name`
  so pre-Phase-1 campaigns with human-readable string keys keep resolving.
- Removed `test_object_mesh_registry_file_load_real_tutorial_campaign_file`
  (Rule 5 violation). Added
  `test_object_mesh_registry_file_load_test_campaign_fixture` pointing at
  `data/test_campaign/data/object_mesh_registry.ron`.
- All 18 in-module tests and 11 SDK object-mesh tests pass.

### Key Design Decision: Legacy Key Fallback

Legacy-format entries are synthesized with `id: 0`. If `load_from_registry`
never used `entry.name` as a key, every legacy entry would collide under `"0"`
in the database map. Instead, `id == 0` is treated as the legacy sentinel and
`entry.name` is used as the lookup key. This preserves string-keyed lookups
(e.g. `has_mesh("barrel")`) for any campaign still written in the old format.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- `cargo nextest run --all-features`: 5507 passed, 8 skipped, 0 failed

---

## Phase 1: Object Mesh Registry Refactor — SDK Call-Site Migration

### Summary

Updated all SDK call sites that referenced the old `ObjectMeshRegistryFile` API
(`meshes: BTreeMap<String, String>`, 2-arg `upsert`, string-keyed `rename`) to use
the new domain API (`entries: Vec<ObjectMeshEntry>`, 3-arg `upsert(id, name, filepath)`,
id-keyed `rename(id, new_name)`).

### Files Changed

**`sdk/campaign_builder/src/campaign_io.rs`**

- `load_objects`: Changed iteration from `for (key, path) in registry.meshes` to
  `for entry_ref in registry.entries`, extracting `key = entry_ref.name.clone()` and
  `path = entry_ref.filepath.clone()` (Phase 1: name used as key; Phase 3 adds id).
- `save_objects`: Replaced `for entry in &self.campaign_data.objects` with
  `for (idx, entry) in self.campaign_data.objects.iter().enumerate()`, and updated
  `upsert` call to `registry.upsert(idx as u32, &entry.key, &entry.file_path)`
  (Phase 3 will replace `idx` with a real persistent id).
- Tests: Updated all `registry.upsert("key", "path")` calls to 3-arg form with
  sequential IDs (0, 1, ...).

**`sdk/campaign_builder/src/obj_importer_ui.rs`**

- `upsert_object_mesh_registry_entry`: Updated to preserve existing entry ID by
  looking up the entry by name, falling back to `max(existing_ids) + 1` for new
  entries, then calling `registry.upsert(id, mesh_key, relative_path)`.
- Test `test_export_object_mesh_upserts_existing_registry_entry_by_key`: Updated
  `upsert` calls to 3-arg form and replaced `registry.meshes.*` assertions with
  `registry.entries.iter().find(|e| e.name == ...)` pattern.

**`sdk/campaign_builder/src/objects_editor.rs`**

- `load_object_entries_from_registry`: Changed `for (key, path) in registry.meshes`
  to `for entry_ref in registry.entries` with name-as-key extraction.
- `sync_object_mesh_registry_entry`: Replaced string-keyed `rename(old, new_key)`
  with ID lookup + `rename(id, new_key)`, and updated `upsert` to 3-arg form.
  ID lookup finds existing entry by old name (or new name if no rename), then falls
  back to `max(ids) + 1` for brand-new entries.
- Tests: Updated `upsert` calls and `registry.meshes.*` assertions to new API.

### Verification

- `cargo fmt --all`: clean
- `cargo check --all-targets --all-features`: 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings
- All 13 affected tests pass (`test_load_objects_*`, `test_save_objects_*`,
  `test_open_campaign_*`, `test_object_mesh_import_*`, `test_export_object_mesh_*`,
  `test_load_object_entries_*`, `test_write_and_sync_*`)

---

## Phase 4: Preview Panel — Trainer Badges and Detail Sections

### Summary

Added trainer and skill trainer visibility to the NPC list-view preview panel
in `sdk/campaign_builder/src/npc_editor/portrait_picker.rs`. Authors now see
badges, dialogue binding, and full trainer configuration directly in the
right-hand preview column without opening the edit form.

### Changes (`sdk/campaign_builder/src/npc_editor/portrait_picker.rs`)

**4.1 — Role badge row**

Two new coloured badges added after the existing Priest badge:

| Condition | Badge | Colour |
|---|---|---|
| `npc.is_trainer` | `🎓 Trainer` | `RGB(220, 180, 80)` (amber) |
| `npc.is_skill_trainer` | `🧠 Skill Trainer` | `RGB(180, 220, 80)` (lime) |

Fallback `🧑 NPC` label gate updated to also check `!npc.is_trainer && !npc.is_skill_trainer`.

**4.2 — Identity grid `"Trainer Dialogue:"` row**

Optional row added after `"Merchant Dialogue:"` — shown only when
`npc.is_trainer || npc.is_skill_trainer`. Displays the `dialogue_id` as a
string, or `"no dialogue assigned"` when `None`.

**4.3 — Trainer detail section**

`egui::Grid::new("npc_preview_trainer_grid")` block — shown when `npc.is_trainer`:
- Dialogue ID (red `"no dialogue assigned"` when `None`)
- Fee Base (value in gold/level, or `"(campaign default)"`)
- Fee Multiplier (`× N.NN`, or `"(campaign default)"`)

**4.4 — Skill Trainer detail section**

`egui::Grid::new("npc_preview_skill_trainer_grid")` block — shown when `npc.is_skill_trainer`:
- Dialogue ID (red label when `None`)
- Trainable Skills (comma-joined list, or `"(none)"`)
- Max Rank (only when `Some`)
- Fee Base (gold/rank or campaign default)
- Fee Multiplier or campaign default

All four `egui::Grid` IDs are unique and do not collide with existing grids.

### Tests Added (4, in `sdk/campaign_builder/src/npc_editor/mod.rs`)

| Test | What it verifies |
|------|------------------|
| `test_show_npc_preview_shows_trainer_badge` | Gate logic: `is_trainer = true` suppresses `🧑 NPC` fallback; UI render does not panic |
| `test_show_npc_preview_shows_skill_trainer_badge` | Gate logic: `is_skill_trainer = true` suppresses fallback; UI render does not panic |
| `test_show_npc_preview_trainer_with_no_dialogue_id_shows_red_label` | `dialogue_id: None` with `is_trainer = true` renders without panic (red label path) |
| `test_show_npc_preview_skill_trainer_skills_list` | `is_skill_trainer = true` with multiple skills, `skill_training_max_rank: Some(5)` renders without panic |

### Quality Gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run` (root workspace) — 5507 passed, 0 failed
- `cargo nextest run` (sdk/campaign_builder) — 2506 passed, 0 failed

---

## Phase 3: Edit Panel — Dialogue ID Picker in Trainer Sections

### Summary

Added a `Dialogue:` ComboBox to both the `🎓 Is Trainer` and `🧠 Is Skill Trainer`
sections in `sdk/campaign_builder/src/npc_editor/mod.rs`. Authors can now
assign any loaded dialogue to a trainer or skill trainer directly from the
trainer section, without scrolling back to the unrelated top-level
"Dialogue & Quests" picker.

### Changes (`sdk/campaign_builder/src/npc_editor/mod.rs`)

**Trainer section** — inserted immediately after the coloured status label
and before the "Training Fee Base" fee fields:

- `ComboBox::from_id_salt("npc_trainer_dialogue_picker")` iterates
  `available_dialogues`; each entry wrapped in `push_id(dialogue.id, …)`
- `(none)` entry wrapped in `push_id("npc_trainer_dialogue_none", …)`
- `needs_save = true` on every selection
- `ui.small("Dialogue must contain an OpenTraining action for this NPC.")` hint

**Skill trainer section** — inserted immediately after the coloured status
label and before the "Trainable Skills" multi-selector:

- `ComboBox::from_id_salt("npc_skill_trainer_dialogue_picker")` — identical
  structure, different `id_salt` and hint text
- Hint: `"Dialogue must contain an OpenSkillTraining action for this NPC."`

Both pickers satisfy SDK AGENTS.md rules:
- Rule 1 (`push_id` on every loop iteration body) ✓
- Rule 3 (`ComboBox::from_id_salt`) ✓

### Tests Added

Two unit tests added to `mod tests` in `sdk/campaign_builder/src/npc_editor/mod.rs`:

| Test | Assertion |
|------|-----------|
| `test_edit_panel_trainer_shows_dialogue_combobox` | Selecting dialogue 42 sets `dialogue_id = "42"`; selected_text resolves to `"42: Ranger Trainer Dialogue"` |
| `test_edit_panel_skill_trainer_shows_dialogue_combobox` | Selecting dialogue 55 sets `dialogue_id = "55"`; selected_text resolves to `"55: Mage Skill Dialogue"` |

### Quality Gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run` (sdk/campaign_builder) — 2502 passed, 0 failed

---

## Phase 2: SDK Dialogue Editor — Wire the Repair Path

### Summary

Wired the Phase 1 repair methods into the existing SDK dialogue editor functions
in `sdk/campaign_builder/src/dialogue_editor.rs`. Trainer NPCs whose
auto-generated dialogue was created with the wrong NPC ID are now corrected
in-place instead of silently receiving a duplicate branch.

### Changes

**`ensure_trainer_dialogue_for_npc`** (`sdk/campaign_builder/src/dialogue_editor.rs`)

Inserted a repair path between the `AlreadyValid` early-return and the
branch-insertion path:

```rust
// Repair path: a SDK-managed trainer node exists but targets the wrong NPC ID.
// Update it in-place instead of appending a duplicate branch.
if dialogue.repair_sdk_trainer_npc_id(&npc.id) {
    self.has_unsaved_changes = true;
    return Ok(MerchantDialogueUpdate::AugmentedExisting { dialogue_id });
}
```

**`ensure_skill_trainer_dialogue_for_npc`** — same repair path using
`repair_sdk_skill_trainer_npc_id`.

### Repair Path Flow (now complete)

```
ensure_*_dialogue_for_npc(npc)
  ├─ dialogue.contains_open_*_for_npc(&npc.id)
  │    └─ true  → AlreadyValid (no change)
  ├─ dialogue.repair_sdk_*_npc_id(&npc.id)          ← NEW
  │    └─ true  → AugmentedExisting (wrong ID fixed in-place)
  └─ dialogue.ensure_standard_*_branch(&npc.id, …)
       └─ true  → AugmentedExisting (new branch appended)
       └─ false → Error
```

### Tests Added

Four unit tests added to the existing `mod tests` block in
`sdk/campaign_builder/src/dialogue_editor.rs`:

| Test | Assertion |
|------|-----------|
| `test_ensure_trainer_dialogue_uses_npc_id` | Returns `AugmentedExisting`; correct ID present; stale ID gone; `has_unsaved_changes` set |
| `test_ensure_trainer_dialogue_already_correct_returns_already_valid` | Returns `AlreadyValid`; node count unchanged |
| `test_ensure_skill_trainer_dialogue_uses_npc_id` | Skill trainer mirror of above |
| `test_ensure_skill_trainer_dialogue_already_correct_returns_already_valid` | Skill trainer no-op mirror |

### Quality Gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run` (root workspace) — 5507 passed, 0 failed
- `cargo nextest run` (sdk/campaign_builder) — 2500 passed, 0 failed

---

## Phase 1: Domain — `DialogueTree` Repair Methods

### Summary

Added two new repair methods to `DialogueTree` in `src/domain/dialogue.rs` as
part of the Fix Skill Trainer SDK plan. These methods form the domain layer of
the repair path that prevents duplicate branch insertion when a trainer dialogue
was generated with a stale or wrong NPC ID.

### New Methods

| Method | Location | Purpose |
|--------|----------|---------|
| `DialogueTree::repair_sdk_trainer_npc_id` | `src/domain/dialogue.rs` | Finds all `TrainerOpenNode`-marked nodes and patches any `OpenTraining { npc_id }` action that does not match the supplied correct ID. Returns `true` when at least one action was updated. |
| `DialogueTree::repair_sdk_skill_trainer_npc_id` | `src/domain/dialogue.rs` | Same pattern for `SkillTrainerOpenNode`-marked nodes and `OpenSkillTraining { npc_id }` actions. |

### Design

Both methods follow the same pattern:
- Iterate `self.nodes.values_mut()`
- For each node whose `sdk_metadata.managed_content` contains the relevant
  `TrainerOpenNode` / `SkillTrainerOpenNode` marker:
  - Patch mismatched `npc_id` in `node.actions`
  - Patch mismatched `npc_id` in each `choice.actions`
- Return `true` if any action was changed, `false` otherwise (no-op)

They are inserted immediately after their respective
`ensure_standard_*_branch` siblings and before
`remove_sdk_managed_*_content`, preserving the existing ordering convention.

### Tests Added

Four unit tests added to the existing `mod tests` block in
`src/domain/dialogue.rs`:

| Test | Assertion |
|------|-----------|
| `test_repair_sdk_trainer_npc_id_updates_wrong_id` | Returns `true`; tree contains correct ID; old ID gone |
| `test_repair_sdk_trainer_npc_id_is_noop_when_already_correct` | Returns `false`; tree unchanged |
| `test_repair_sdk_skill_trainer_npc_id_updates_wrong_id` | Returns `true`; tree contains correct ID; old ID gone |
| `test_repair_sdk_skill_trainer_npc_id_is_noop_when_already_correct` | Returns `false`; tree unchanged |

### Quality Gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — 5507 passed, 0 failed

---

## Dialogue Overhaul: All Characters Recruitable, Lore-Consistent Trees

### Summary

Complete overhaul of `campaigns/tutorial/data/dialogues.ron`. Fixed a critical
duplicate-id bug (Eonir was incorrectly assigned id 1003, colliding with Isolde).
Rewrote the two placeholder NPC dialogues (Arcturus, Aetheris), rewrote both
existing stub recruitment dialogues (Zara, Whisper), expanded Jyeshtha with the
full Eonir reveal, applied lore-consistent text polish to all service NPCs, and
created four new recruitment dialogues so every premade character is now
recruitable. Updated `npcs.ron` to reference Eonir's corrected id.

### Critical Bug Fix

- `tutorial_lich_eonir` dialogue was id 1003 (duplicate of Isolde Dawnfang)
- Eonir reassigned to **id 1004**; `npcs.ron` `dialogue_id` updated to match

### Full Rewrites

| id | Old name | New name | Notes |
|----|----------|----------|-------|
| 1 | Arcturus Story | Arcturus - The Eternal Wanderer | 6 nodes; Cosmic Weave lore, quest hook for instruments |
| 2 | Arcturus Brother Story | Aetheris - The Void-Drifter | 6 nodes; Dark Forest hut, Void-Weave origin, leads to Arcturus |
| 101 | Apprentice Zara Recruitment | same | 6 nodes; Lumina-weaving, Great Alignment, Master Elian |
| 102 | Whisper Recruitment | same | 6 nodes; Vespera Moonshadow, Silver Spires, locksmith cover |

### Expanded

- **id 1001 Jyeshtha** — renamed to *The Glass Sea Watcher*; fixed encoding
  corruption (`â\u{80}\u{94}` → `—`); node 2 now delivers the full Eonir reveal
  (jade icosahedron stolen by Eonir Xavian Stalix, Vaelgrim, the stillness);
  node 5 now points to Frostspire Peaks as the destination

### Text Polish (structure preserved, text updated)

ids 3, 4, 5, 6, 7, 8, 9, 10, 11, 1002 — all updated to reference Velmoria
world lore: the Cosmic Weave, the Great Gloom, Ashvale settlements, the
stillness spreading from the north, goblin scouts as directed outriders,
Harrow Downs opening from the inside, Arcturus's instruments.

### New Recruitment Dialogues

| id | Character | character_id | Notes |
|----|-----------|-------------|-------|
| 104 | Kira Valerius | `tutorial_human_knight` | 7 nodes; Eldoria exile, Void-Blight, party leader |
| 105 | Sirius Xylanthir | `tutorial_elf_sorcerer` | 6 nodes; Dark Elf, Void-Weave, demands worthy enemies |
| 106 | Mira | `tutorial_human_cleric` | 6 nodes; Great Gloom, Order of the Resplendent Dawn |
| 107 | Old Gareth | `old_gareth` | 7 nodes; Aethelgard, Great Tremor, Core Shield, reluctant veteran |

### All Recruitable Characters — Dialogue Map

| Character | Dialogue id | Status |
|-----------|------------|--------|
| Kira | 104 | New |
| Sirius | 105 | New |
| Isolde | 1003 | Existing (excellent quality, unchanged) |
| Mira | 106 | New |
| Old Gareth | 107 | New |
| Whisper | 102 | Rewritten |
| Apprentice Zara | 101 | Rewritten |
| Zhaya | 1000 | Existing (good quality, unchanged) |

### Follow-Up Needed

`characters.ron` does not yet have a `dialogue_id` field on character entries.
The game will need a mechanism to wire each character's in-world encounter to
their recruitment dialogue (ids 100–107, 1000, 1003). This is tracked under
the Option A / recruitable-characters feature work.

---

## Map Names, Descriptions & Frostspire Peaks (map_8)

### Summary

Updated `name` and `description` fields across all seven existing maps to align
with *The Stillness Prophecy* world lore. Fixed the long-standing typo
"Dark Forrest" → "Dark Forest" and added the missing apostrophe in
"Astronomer's Temple". Created `map_8.ron` — Frostspire Peaks, the 40×40
Act IV final map where the confrontation with Eonir takes place.

### Files modified

**`campaigns/tutorial/data/maps/map_1.ron`** — Town Square
- Description updated: Ashvale settlement, Kira's gathering point

**`campaigns/tutorial/data/maps/map_2.ron`** — Dark Forest *(typo fixed)*
- Name: "Dark Forrest" → "Dark Forest"
- Description updated: goblin scouts, something larger stirring

**`campaigns/tutorial/data/maps/map_3.ron`** — Ancient Ruins
- Description updated: kobolds and goblins, Arcturus's stolen astrolabe

**`campaigns/tutorial/data/maps/map_4.ron`** — Arcturus's Cave
- Description updated: foothills of Mount Ashkarron

**`campaigns/tutorial/data/maps/map_5.ron`** — Mountain Pass
- Description updated: hidden village, first rumours of the stillness

**`campaigns/tutorial/data/maps/map_6.ron`** — The Harrow Downs
- Description updated: tombs opening from the inside

**`campaigns/tutorial/data/maps/map_7.ron`** — Astronomer's Temple *(apostrophe fixed)*
- Name: "Astronomers Temple" → "Astronomer's Temple"
- Description updated: Glass Sea pyramid, Jyeshtha's vigil

**`campaigns/tutorial/data/maps/map_8.ron`** — Frostspire Peaks *(new)*
- 40×40 (1 600 tiles), matching map_7 in size
- `maps_dir: "data/maps/"` in campaign.ron picks it up automatically
- Layout zones:
  - Mountain border + flanking canyon walls
  - Frozen tundra approach from the south (y=28-38), sparse ice-rock obstacles
  - Outer castle walls (Stone/Normal) with south gate at x=19-20 (y=27)
  - Castle courtyard (Stone floor, lit)
  - Inner keep walls with inner gate at x=19-20 (y=22)
  - Eonir's sanctum (y=13-21, x=13-26): Stone floor, `is_dark: true`
  - Jagged northern frostspire peaks beyond the castle (y=1-7)

---

## Lore Files & NPC Updates: The Stillness Prophecy Character Pass

### Summary

Updated all eight premade-character lore files under
`campaigns/tutorial/assets/characters/lore/` using the canonical lore prompts
in `campaigns/tutorial/prompts/lore/`. Updated matching `description` fields in
`campaigns/tutorial/data/characters.ron` and
`campaigns/tutorial/data/npcs.ron`. Added new NPC **Eonir the Still**
(`tutorial_lich_eonir`) — the Lich King antagonist of Act IV.

### Files modified

**`campaigns/tutorial/assets/characters/lore/kira.ron`**
- Backstory: Eldoria origin, Void-Blight, knight-errant in exile
- Title: *The Exile Knight, Blade of the Shattered Shore*
- Archetype: Soldier / Versatile Warrior

**`campaigns/tutorial/assets/characters/lore/old_gareth.ron`**
- Backstory: Master Architect of Aethelgard, Great Tremor, the Core Shield
- Title: *The Last Pillar of Aethelgard, Warden of the Core Shield*
- Archetype: Bastion Guardian / Stalwart Tank

**`campaigns/tutorial/assets/characters/lore/isolde.ron`**
- Backstory: Sunspire Dynasty princess, secret War Cleric training, rode to war
- Title: *Princess of Sunspire, Holy Warrior of the Radiant Shield*
- Archetype: War Cleric / Martial Divine Champion

**`campaigns/tutorial/assets/characters/lore/mira.ron`**
- Backstory: Village outpost against the Great Gloom, Order of the Resplendent Dawn
- Title: *The Radiant Sentinel, Acolyte of the Eternal Spark*
- Archetype: Light Cleric / Holy Guardian

**`campaigns/tutorial/assets/characters/lore/sirius.ron`**
- Backstory: Dark Elf scholar from subterranean reaches, Void-Weave mastery
- Title: *Master of the Obsidian Veil, Archon of the Void-Weave*
- Archetype: Shadow Sorcerer / Void Mage

**`campaigns/tutorial/assets/characters/lore/whisper.ron`**
- Backstory: Vespera Moonshadow, High Elf of the Silver Spires, traded nobility for stealth
- Title: *The Ghost of the Silver Spires, Master of the Unseen Key*
- Archetype: High Elf Rogue / Shadow Assassin

**`campaigns/tutorial/assets/characters/lore/apprentice_zara.ron`**
- Backstory: Lumina-weaving obsession, Great Alignment experiment, separated from Master Elian
- Title: *Luminous Scholar, Weaver of the Prismatic Spark*
- Archetype: Gnome Sorcerer / Cosmic Apprentice

**`campaigns/tutorial/assets/characters/lore/zhaya.ron`**
- Backstory: Eastern Peaks monasteries, Astrologer's Path visions, pilgrimage to the Astronomers Temple
- Title: *Celestial Monk, Seeker of the Star-Bound Truth*
- Archetype: Astral Monk / Fate-Reader

**`campaigns/tutorial/data/characters.ron`**
- Updated `description` for Kira, Sirius, Mira, Old Gareth, Whisper, Apprentice Zara, and Zhaya
- Isolde's description was already aligned with the prompt; no change

**`campaigns/tutorial/data/npcs.ron`**
- Updated `description` for `tutorial_wizard_arcturus` (Arcturus, Eternal Wanderer)
- Updated `description` for `tutorial_wizard_arcturus_brother` (Aetheris, Void-Drifter)
- Updated `description` for `tutorial_mystic_astronomer` (Jyeshtha, Glass Sea watcher)
- Added new entry `tutorial_lich_eonir` — Eonir the Still, Sovereign of Stillness
  - `creature_id`: 1021, `dialogue_id`: 1003, `faction`: "Sovereign of Stillness"
  - `portrait_id`: "eonir_still" (asset to be created)

---

## Character Bio & Navigation, Phase 5: Mouse Input Fix for Character Sheet

### Summary

Implemented Phase 5 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`, the
final phase of the Character Bio & Navigation plan: fixed the root cause of
`GameMode::CharacterSheet`'s mouse-click regressions (Identified Issues #2-3
in the plan) and resolved the HUD-portrait click-through that compounded it.
Also added a mouse-clickable "Bio" button, since Phase 3 only wired the Bio
panel to the keyboard (`B` key) and Phase 5's own success criteria requires
"mouse and keyboard... both fully functional in every Character Sheet view."

### Files modified

**`src/game/systems/input/mode_guards.rs`**
- Added `GameMode::CharacterSheet(_)` to `movement_blocked_for_mode`'s match
  arms, matching every other modal screen. `interaction_blocked_for_mode`
  and `input_blocked_for_mode` both delegate to `movement_blocked_for_mode`
  internally, so this single change fixes both -- the plan named both
  functions as line-range targets, but the second was already covered by
  the first's delegation; there was no separate match arm list to edit in
  `interaction_blocked_for_mode`.
- This closes the actual root cause identified in the plan: without this
  guard, `handle_exploration_input_interact`/`handle_exploration_input_movement`
  only stood down via the `egui_wants_any_pointer_input` run condition,
  written in `PostUpdate` -- one frame after the Character Sheet UI draws in
  `Update` -- so exploration movement/interaction input could leak through
  on the frame the sheet opened or closed.
- 3 new tests (`movement_blocked_for_mode`/`interaction_blocked_for_mode`/
  `input_blocked_for_mode`, all asserting `true` for `CharacterSheet`),
  mirroring the existing per-mode test convention in this file.

**`src/game/systems/hud.rs`**
- Removed `GameMode::CharacterSheet(_)` from `portrait_click_allowed`.
  Previously, a HUD portrait click while the sheet was already open called
  `enter_character_sheet_at` directly, double-handling the same click
  alongside whatever the sheet's own egui widgets did with it -- the root
  cause named in the plan's Identified Issue #3.
- Updated `portrait_click_allowed` and `handle_portrait_click_system`'s doc
  comments to describe the new blocked mode and why.
- Replaced `test_handle_portrait_click_when_already_in_sheet_updates_index`
  (which asserted the now-removed "click a second portrait to retarget the
  open sheet" behavior) with
  `test_handle_portrait_click_blocked_when_sheet_already_open`, asserting
  the click is blocked and focus is unaffected. Added
  `test_portrait_click_not_allowed_character_sheet` alongside the file's
  other single-mode `portrait_click_allowed` tests. Confirmed the two
  remaining `portrait_click_allowed` assertions that touch `CharacterSheet`
  mode indirectly (`test_handle_portrait_click_selects_correct_party_index`,
  `test_handle_portrait_click_opens_sheet_in_exploration`/`_in_combat`) all
  check the mode *before* entering the sheet, so they were unaffected.

**`src/game/systems/character_sheet_ui.rs`**
- Added a mouse-clickable "Bio"/"Hide Bio" button next to the existing
  "Party Overview"/"Next >"/"< Prev" buttons in the Single-view header,
  gated on `character.lore.is_some()` (same condition as the `B` key and
  hint), calling the same `CharacterSheetState::toggle_bio()` the keyboard
  shortcut uses. This wasn't in Phase 5's deliverables list, but the
  phase's own success criteria ("mouse and keyboard are both fully
  functional in every Character Sheet view") and testing requirements
  (which mention "the new Bio button") both call for it, and Phase 3 had
  only wired the toggle to the keyboard.
- Updated the module doc's Single-view ASCII diagram to show the new
  button.

### Quality gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5501/5501 passed**, 8 skipped (root
  `antares` crate; full regression run, not just the new/touched tests,
  since blocking exploration input in a new mode is exactly the kind of
  change that can surface hidden cross-system assumptions)
- `cargo nextest run -p campaign_builder --all-features` — **2496/2496
  passed** (unaffected by this phase, run anyway for full-workspace
  confidence -- see the Phase 4 entry below for why this must be invoked
  explicitly with `-p`)
- `cargo test --doc -- mode_guards portrait_click_allowed` and
  `cargo test --doc -- character_sheet` (targeted, not the full workspace
  doctest suite) — 4/4 and 15/15 passed
- Manual: launched `./target/debug/antares --campaign campaigns/tutorial`,
  confirmed it starts and runs without panicking for 12+ seconds.
  Interactive mouse-driven verification of the actual click behavior in the
  live window (Party Overview "View" buttons, the new Bio button, blocked
  portrait clicks) was not possible in this sandbox (no Accessibility/
  System Events permission, consistent with every prior phase in this
  plan) -- covered instead by the mode-guard/portrait-click unit tests
  above, which exercise the real blocking predicates the input systems
  gate on.

### Plan status

This completes all 5 phases of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`.

---

## Terrain Externalization — Phase 5: SDK and Campaign Builder Tooling

**Plan**: `docs/explanation/terrain_externalization_implementation_plan.md` §Phase 5

### Summary

Phase 5 wires the terrain data-driven infrastructure (introduced in Phases 1–4)
into the SDK CLI tools and Campaign Builder UI.  Four sub-tasks were
completed.

### 5.1 + 5.2 — `map_builder.rs` and `texture_generator.rs`

**`src/sdk/cli/map_builder.rs`**
- Added `pub fn builtin_glyph(id: TerrainId) -> Option<char>` — maps the 12
  built-in terrain IDs to ASCII display characters (`'.'`, `','`, `'~'`, …);
  custom campaign terrain falls back to `'?'`.
- Updated `parse_terrain` signature to `(s: &str, db: &TerrainDatabase) -> TerrainId`.
  Tries numeric parse first, then `db.all_definitions()` case-insensitive name
  scan, then falls back to `TERRAIN_GROUND` with a warning.  The old hard-coded
  `match` string literal block is gone.
- `show_map` glyph branch replaced with `builtin_glyph(tile.terrain).unwrap_or('?')`.
- All `process_command` call-sites pass `&self.terrain_db`.
- Tests: `test_parse_terrain` updated; new
  `test_parse_terrain_resolves_custom_campaign_terrain_by_name` covers DB
  lookup, case-insensitive match, numeric pass-through, and unknown fallback.

**`src/sdk/cli/texture_generator.rs`**
- `TerrainTextureSpec.filename` changed from `&'static str` to `String`
  (filenames are now derived from `TerrainDefinition.name`).
- `const TERRAIN_SPECS` (9 hard-coded entries) removed.
- Added `fn terrain_texture_seed(id: u32) -> u64` — deterministic
  multiplicative hash for per-terrain noise seeds.
- Added `fn terrain_specs_for_db(db: &TerrainDatabase) -> Vec<TerrainTextureSpec>` —
  sorts definitions by ID, derives filenames (`def.name.to_lowercase()
  .replace(' ', "_") + ".png"`), converts `[f32; 3]` color to `u8` channels.
- `run_generate` now calls `terrain_specs_for_db(&builtin_terrain_db())`,
  writing 12 PNGs (Ground through Ice) instead of the old 9.
- Terrain-spec tests updated for 12 entries; new
  `test_texture_generator_covers_all_database_terrains` asserts count == DB
  size and that `sand.png`, `snow.png`, and `ice.png` are present.

### 5.3 — `templates.rs`

Already complete from a prior phase; templates were already using
`TERRAIN_GRASS`/`TERRAIN_STONE`/`TERRAIN_FOREST` constants.

### 5.4 — `terrain_editor.rs` + Campaign Builder infrastructure

**New file: `sdk/campaign_builder/src/terrain_editor.rs`**
- `TerrainEditorState` — list/edit/add/delete CRUD for campaign-defined
  `TerrainDefinition` entries.  Mirrors `landscape_editor.rs` in structure.
- List view uses `TwoColumnLayout` (SDK Rule 9), `show_standard_list_item`
  (Rule 15), `push_id` per row (Rule 1).  Includes a collapsible read-only
  built-in terrain reference section.
- Edit form: name, texture path + Browse button, roughness slider (0–1),
  mesh-style `ComboBox` (all 3 variants), vegetation `ComboBox` (all 3),
  blocked checkbox, height slider (0–5), R/G/B color sliders + swatch;
  Back / Save / Cancel (Rule 16).
- `next_available_id()` assigns IDs ≥ 13100 (campaign range).
- 9 unit tests covering: new state, enter_edit, apply_edit, 3 delete/
  selection invariants, 2 ID assignment tests, RON round-trip.

**`sdk/campaign_builder/src/editor_state.rs`**
- `CampaignData` gains `terrain_definitions: Vec<TerrainDefinition>` and
  `terrain_db: TerrainDatabase` (seeded from `builtin_terrain_db()` in
  `Default`).
- `EditorRegistry` gains `terrain_editor_state: terrain_editor::TerrainEditorState`.

**`sdk/campaign_builder/src/campaign_io.rs`**
- `load_terrain()` — reads `data/terrain.ron`, builds the merged DB via
  `builtin_terrain_db()` + `merge()`, resets editor state.  Called from
  `do_open_campaign`.
- `save_terrain()` — writes campaign-defined definitions only.  Called from
  `do_save_campaign`.
- `do_new_campaign` clears terrain_definitions and resets terrain_db to
  builtins.

**`sdk/campaign_builder/src/lib.rs`**
- `CampaignMetadata` gains `terrain_file: String` (serde default
  `"data/terrain.ron"`).
- `EditorTab::Terrain` added after `Landscape`; wired into the tab bar and
  central-panel match.
- `Terrain` tab arm calls `terrain_editor_state.show()` then rebuilds the
  merged `terrain_db` from builtins + campaign definitions.
- `MapEditorRefs` construction passes `terrain_db: &self.campaign_data.terrain_db`.

### 5.5 — `map_editor.rs` — `TerrainType` → `TerrainId` migration

`TerrainType` has been removed from the domain.  All references in
`map_editor.rs` were migrated:

- `selected_terrain: TerrainType` → `selected_terrain: TerrainId`;
  initialised with `TERRAIN_GROUND`.
- `MapEditorRefs` and `MapInspectorData` gain `terrain_db: &'a TerrainDatabase`.
- `paint_tile`, `fill_region`, `erase_tile` take a `db: &TerrainDatabase`
  parameter; `Tile::new` calls now pass the db.
- `paint_tile` blocking check: `db.get_by_id(selected_terrain).map(|d|
  d.blocked).unwrap_or(false)`.
- `tile_color` (grid rendering) and `show_map_preview` (mini-map) now look
  up `[r, g, b]` from `TerrainDefinition.color` via the db instead of a
  hard-coded `match`; `MapGridWidget` carries `terrain_db: &'a
  TerrainDatabase`.
- Terrain palette `ComboBox` (tool palette + inspector) iterates
  `db.all_definitions()` sorted by ID, showing the definition `name`.
- `apply_to_metadata_for_terrain` dispatches on
  `TerrainDefinition.vegetation` / `mesh_style` instead of per-enum-variant:
  `GrassCover` → grass controls; `Forest` → tree/canopy controls;
  `Mountain` mesh → rock-variant controls; `Water` mesh → flow controls.
- `show_terrain_specific_controls` applies the same vegetation/mesh-style
  dispatch.
- `vegetation_runtime_hint` updated to use DB lookup.
- All tests updated: `Tile::new` calls pass `&builtin_terrain_db()`,
  `TerrainType::*` literals replaced with `TERRAIN_*` constants, method
  signatures updated.

### Quality gates

- `cargo fmt --all` — clean
- `cargo check --package antares --all-targets --all-features` — 0 errors
- `cargo clippy --package antares --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --package antares --all-features` — **5566/5566 passed**,
  8 skipped
- `cargo check --package campaign_builder` — 0 errors from Phase 5 work;
  11 pre-existing errors in `npc_editor/mod.rs` and `asset_manager.rs`
  (missing `NpcDefinition` fields from a prior phase not yet addressed)
  remain unchanged
- `cargo test --package campaign_builder` — zero Phase 5 errors; npc_editor
  and asset_manager pre-existing build failures block the full test run

### Plan status

This completes Phase 5 of
`docs/explanation/terrain_externalization_implementation_plan.md`.
Phase 6 (data migration, repo-wide cleanup, and docs) remains.
