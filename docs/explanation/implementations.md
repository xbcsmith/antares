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
