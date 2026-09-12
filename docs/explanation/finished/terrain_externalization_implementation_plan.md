# Terrain Externalization Implementation Plan

## Overview

Replace the closed `TerrainType` Rust enum with an open, numeric-ID terrain
registry (`TerrainDatabase` / `TerrainDefinition`) loaded from RON, following
the same pattern already used for `LandscapeDatabase`
([landscape.rs](../../src/domain/world/landscape.rs)). This lets a campaign
author define arbitrary terrain types — including the currently-missing Sand,
Snow, and Ice — entirely through `terrain.ron` and a texture file, with no
engine code changes. Built-in terrain (the current nine variants plus Sand,
Snow, Ice) ships as default RON data. This is a clean, breaking
reimplementation: `TerrainType` is deleted, every exhaustive match on it is
rewritten against `TerrainDefinition` fields, and all existing map RON files
and tests are migrated in place. No compatibility shim or dual-format loader
is kept.

## Current State Analysis

### Existing Infrastructure

- `TerrainType` — closed enum, 9 variants (`Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain`), defined at [types.rs:57-76](../../src/domain/world/types.rs#L57-L76). Already derives `Serialize`/`Deserialize`/`Ord`/`Hash`, so it round-trips through RON today (e.g. `EncounterTable.terrain_modifiers: BTreeMap<TerrainType, f32>`).
- RON database convention — `impl_ron_database!` macro and `load_ron_entries`/`load_ron_file` in [database_common.rs](../../src/domain/database_common.rs), used by every other content type (items, monsters, landscape, furniture, etc.).
- Closest template — `LandscapeDatabase` / `LandscapeDefinition` / `LandscapeCategory` in [landscape.rs](../../src/domain/world/landscape.rs): a small closed *category* enum plus an open, RON-authored, numeric-ID *definition* struct with `flags: LandscapeFlags { blocking: bool }`.
- ID ranges — a single shared `u32` number line documented in `docs/reference/monster_creature_mapping_reference.md`: 1–999 monsters, 1000–1999 NPCs, 2000–3999 templates/variants, 4000–8999 custom, 9000–9999 item meshes, 10000–10999 furniture meshes, 11000–11999 landscape meshes (`LANDSCAPE_MESH_ID_MIN = 11_000`, [types.rs:119](../../src/domain/types.rs#L119)), 12000–12999 object meshes (planned). `LANDSCAPE_ID_MIN = 1` ([types.rs:92](../../src/domain/types.rs#L92)) is a separate, smaller entity-ID line. 13000+ is unclaimed — this plan reserves it for terrain.
- Precedent refactor — `docs/explanation/finished/object_mesh_registry_refactor_implementation_plan.md` shows how a prior closed/ad-hoc registry was converted to the numeric-ID array format used everywhere else; useful as a process template (it used a dual-format migration window, which this plan explicitly does **not** do, per user direction).

### Identified Issues

- **Two fully-exhaustive, wildcard-free matches** on `TerrainType` will not compile once a variant is added, and are the only sites Rust forces you to touch: the wall-tint color match in [map.rs:2002-2012](../../src/game/systems/map.rs#L2002-L2012), and the ASCII glyph export in [map_builder.rs:272-282](../../src/sdk/cli/map_builder.rs#L272-L282).
- **Every other terrain-aware site uses a wildcard or `matches!` guard**, so it silently keeps working (with wrong/default behavior) when a new terrain is added instead of failing to compile: `TerrainMaterialCache` (struct-of-fields, [terrain_material_cache.rs](../../src/game/resources/terrain_material_cache.rs)), `terrain_materials.rs` texture/roughness lookup, `Tile::new` blocking (`matches!(terrain, Mountain | Water)`, [types.rs:921](../../src/domain/world/types.rs#L921)), `TileVisualMetadata::effective_height` ([types.rs:538-542](../../src/domain/world/types.rs#L538-L542)), map mesh-spawn dispatch ([map.rs:1704-1995](../../src/game/systems/map.rs#L1704-L1995)), automap color ([hud.rs:2578-2586](../../src/game/systems/hud.rs#L2578-L2586)), dropped-item clearance ([item_world_events.rs:392-395](../../src/game/systems/item_world_events.rs#L392-L395)), vegetation placement ([vegetation_placement.rs:425,472,476](../../src/game/systems/vegetation_placement.rs#L425)), `parse_terrain` string parser ([map_builder.rs:592-607](../../src/sdk/cli/map_builder.rs#L592-L607)).
- **Three independent hardcoded terrain lists** must currently be kept in sync by hand: the enum itself, `TerrainMaterialCache`'s named fields, and `texture_generator.rs`'s `TERRAIN_SPECS` array ([texture_generator.rs:345-418](../../src/sdk/cli/texture_generator.rs#L345-L418)) — the last of which doesn't even reference `TerrainType`, just filenames.
- **`TileCode` (blueprint.rs) duplicates the terrain list a fourth time** and is exhaustive over `TileCode`, not `TerrainType` ([blueprint.rs:47-60](../../src/domain/world/blueprint.rs#L47-L60), mapping at [blueprint.rs:136-149](../../src/domain/world/blueprint.rs#L136-L149)) — adding a terrain variant does not force this file to be updated, a silent gap.
- **Campaign Builder (`sdk/campaign_builder/src/map_editor.rs`) has the heaviest coupling**: hardcoded terrain palette arrays, two separate hardcoded `Color32` swatch tables, and `TerrainEditorState::apply_to_metadata_for_terrain` dispatching per-terrain inspector widgets by exhaustive match.
- **No Sand, Snow, or Ice terrain** exists; `TileVisualMetadata.snow_coverage` is a cosmetic overlay percentage on existing terrain (e.g. a snowy `Forest`/`Mountain`), not a terrain identity.
- **Map RON files encode terrain as the bare variant name** (e.g. `terrain: Grass`) wherever `Tile`/`TerrainType` is serialized; converting to a numeric `TerrainId` is a breaking format change for every existing map file in `campaigns/` and `data/test_campaign/`, and also for player save files (`campaigns/*/saves/*.ron`, gitignored, not part of this repo's migration scope — see Phase 6.1).
- **`Tile.blocked` is a stored, independently-mutable field, not purely terrain-derived.** `Tile::new` seeds it once from `matches!(terrain, Mountain | Water) || wall_type == Normal` ([types.rs:920-921](../../src/domain/world/types.rs#L920-L921)), but ~30 call sites across `lock_ui.rs`, `dialogue.rs`, `events.rs`, `exploration_interact.rs`, `input.rs`, `vegetation_placement.rs`, `map.rs`, `save_game.rs`, and the Walk on Water spell override in `exploration_movement.rs` ([lines 178-213](../../src/game/systems/input/exploration_movement.rs#L178-L213)) mutate `tile.blocked` directly at runtime for reasons unrelated to terrain (door/lock state, dialogue effects, temporary spell overrides, save/load restore). Any redesign must keep `blocked` as a plain mutable field — see Phase 2.1.

## Reserved ID Range

`TERRAIN_ID_MIN = 13_000`, defined alongside `LANDSCAPE_ID_MIN` in [domain/types.rs](../../src/domain/types.rs). Built-in terrain occupies `13000`–`13011` (12 entries: the 9 existing plus Sand, Snow, Ice). Campaign-defined custom terrain starts at `13100` by convention (documented, not enforced) to leave room for future built-ins.

Note: the 12000–12999 object-mesh range referenced above is itself only a convention today (used in RON data and an unmerged `docs/explanation/id_range_consolidation_implementation_plan.md`) — there is no `OBJECT_MESH_ID_MIN` constant in `src/domain/types.rs` yet. `TERRAIN_ID_MIN = 13_000` does not depend on that constant landing first; it only depends on 12000–12999 staying reserved by convention.

| ID | Name | Mesh Style | Vegetation | Blocked |
|----|------|-----------|------------|---------|
| 13000 | Ground | Flat | None | false |
| 13001 | Grass | Flat | GrassCover | false |
| 13002 | Water | Water | None | true |
| 13003 | Lava | Flat | None | false |
| 13004 | Swamp | Flat | None | false |
| 13005 | Stone | Flat | None | false |
| 13006 | Dirt | Flat | None | false |
| 13007 | Forest | Flat | Forest | false |
| 13008 | Mountain | Mountain | None | true |
| 13009 | Sand | Flat | None | false |
| 13010 | Snow | Flat | None | false |
| 13011 | Ice | Flat | None | false |

## Implementation Phases

### Phase 1: Domain Foundation — `TerrainDefinition` and `TerrainDatabase`

#### 1.1 Add `TerrainId` and `TERRAIN_ID_MIN`

In [domain/types.rs](../../src/domain/types.rs), alongside `LandscapeId`/`LANDSCAPE_ID_MIN`: add `pub type TerrainId = u32;` and `pub const TERRAIN_ID_MIN: TerrainId = 13_000;`.

#### 1.2 Add `TerrainMeshStyle` and `TerrainVegetation`

New small closed enums in a new module `src/domain/world/terrain.rs`, mirroring `LandscapeCategory`'s role (broad, fixed *rendering shape*, not open content):

- `TerrainMeshStyle { Flat, Water, Mountain }` — `Flat` is the default; `Water`/`Mountain` select the existing specialized mesh-spawn branches in `map.rs`.
- `TerrainVegetation { None, GrassCover, Forest }` — `GrassCover` triggers grass-blade decoration (today's `should_spawn_grass_cover` guard); `Forest` triggers both grass-blade decoration and tree/shrub placement (today's `vegetation_placement.rs` `Forest`-only checks).

Both derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default`, with `#[default]` on `Flat`/`None` respectively.

#### 1.3 Add `TerrainDefinition`

`pub struct TerrainDefinition` in `terrain.rs`, fields: `id: TerrainId`, `name: String`, `texture_path: String`, `roughness: f32`, `mesh_style: TerrainMeshStyle` (`#[serde(default)]`), `vegetation: TerrainVegetation` (`#[serde(default)]`), `blocked: bool` (`#[serde(default)]`), `height: f32` (`#[serde(default)]`), `color: [f32; 3]`. Follow `LandscapeDefinition`'s doc-comment and `# Examples` doctest conventions.

#### 1.4 Add `TerrainDatabaseError` and `TerrainDatabase`

`TerrainDatabaseError` (thiserror): `ReadError(#[from] std::io::Error)`, `ParseError(#[from] ron::error::SpannedError)`, `DuplicateId(TerrainId)`, `NotFound(TerrainId)`, `InvalidTerrainId { id: TerrainId, min: TerrainId }`.

`TerrainDatabase { items: HashMap<TerrainId, TerrainDefinition> }`, built with `crate::impl_ron_database!` (post_load validates `id >= TERRAIN_ID_MIN`, mirroring `LandscapeDatabase::validate_definition_ids`). Methods: `new`, `add`, `get_by_id`, `get_by_name`, `all_definitions`, `len`, `is_empty`, `has_definition` — same shape as `LandscapeDatabase`.

#### 1.5 Testing Requirements

- Doctests on every public item per repo convention.
- `test_terrain_database_add_and_lookup`, `test_terrain_database_rejects_duplicate_id`, `test_terrain_database_rejects_id_below_min`.
- `test_terrain_definition_ron_roundtrip_preserves_all_fields`.
- `test_terrain_mesh_style_and_vegetation_ron_roundtrip_all_variants` (loop all variants, like `test_landscape_category_ron_roundtrip_all_variants`).
- `test_terrain_definition_minimal_ron_defaults_optional_fields`.

#### 1.6 Deliverables

- [ ] `TerrainId`, `TERRAIN_ID_MIN` in `domain/types.rs`
- [ ] `src/domain/world/terrain.rs` with `TerrainMeshStyle`, `TerrainVegetation`, `TerrainDefinition`, `TerrainDatabaseError`, `TerrainDatabase`
- [ ] Module wired into `src/domain/world/mod.rs` exports
- [ ] All tests pass; no existing code references `terrain.rs` yet (additive only)

#### 1.7 Success Criteria

`cargo check --all-targets --all-features` clean. `TerrainDatabase::load_from_string` round-trips a sample RON array. No other module in the crate references `TerrainDatabase` yet — this phase is purely additive.

---

### Phase 2: Replace `TerrainType` with `TerrainId` in Domain Types

#### 2.1 Migrate `Tile`

In [types.rs](../../src/domain/world/types.rs): change `Tile.terrain: TerrainType` to `Tile.terrain: TerrainId`.

`Tile.blocked` **stays a stored, plain `pub blocked: bool` field** — do not turn it into a derived accessor. It is not purely terrain-derived today: ~30 call sites outside `types.rs` (doors/locks, dialogue effects, event triggers, vegetation clearing, save/load restore, and the Walk on Water spell override in `exploration_movement.rs`) mutate `tile.blocked` directly at runtime for reasons unrelated to terrain. A derived-from-terrain accessor cannot represent "this Mountain tile is temporarily walkable because of a spell" or "this door tile is blocked because it's locked," and would break all of those systems. `Tile::is_blocked(&self) -> bool` ([types.rs:938-940](../../src/domain/world/types.rs#L938-L940)) keeps its existing no-argument signature and existing callers are unaffected.

What does change: `Tile::new` can no longer compute the *initial* value of `blocked` from a `matches!` on `TerrainType`, since terrain is now an opaque `TerrainId`. `Tile::new` gains a `&TerrainDatabase` parameter used only at construction time to seed `blocked = db.get_by_id(terrain).is_some_and(|d| d.blocked) || wall_type == WallType::Normal`; the field is then free to be mutated afterward exactly as it is today. This breaks the signature of every `Tile::new(...)` call site — audit and update all of them, including test helpers in `map.rs`, `map_validator.rs`, `exploration_spells.rs`, `blueprint.rs`, and `sdk/cli/map_builder.rs`.

`TileVisualMetadata::effective_height` ([types.rs:517-542](../../src/domain/world/types.rs#L517-L542)) is already a pure function taking `terrain: TerrainType, wall_type: WallType` as parameters (not a stored field) — no design conflict here. Change its signature to `effective_height(&self, wall_type: WallType, terrain_height: f32) -> f32`, where `terrain_height` is the caller-supplied `TerrainDefinition.height` value (looked up once by the caller, not threaded as a whole `&TerrainDatabase`); it returns `self.height` if set, else `2.5` for wall types other than `None`, else `terrain_height`.

#### 2.2 `movement.rs` requires no change

Because `blocked` remains a stored field, `check_tile_blocked` ([movement.rs:250-262](../../src/domain/world/movement.rs#L250-L262)) keeps calling `tile.is_blocked()` with no arguments and needs no `&TerrainDatabase` threading. Confirm this with a compile check after Phase 2.1 lands — do not add a `db` parameter here.

#### 2.3 Migrate `blueprint.rs`

`TileCode` ([blueprint.rs:47-60](../../src/domain/world/blueprint.rs#L47-L60)) keeps its 12 fixed variants as an authoring shorthand for the *built-in* terrain set only (not a general terrain mechanism) — update its `From<TileCode>` mapping ([blueprint.rs:136-149](../../src/domain/world/blueprint.rs#L136-L149)) to emit the `terrain13000`–`13011` built-in `TerrainId` constants from Phase 3 instead of `TerrainType` variants. Document in the module doc comment that custom campaign terrain must be authored via full map RON (`terrain: 13105`), not `TileCode`.

#### 2.4 Migrate `EncounterTable`

`EncounterTable.terrain_modifiers: BTreeMap<TerrainType, f32>` → `BTreeMap<TerrainId, f32>` ([events.rs](../../src/domain/world/events.rs) usages at lines 512, 545, 1143).

#### 2.5 Delete `TerrainType`

Remove the enum definition ([types.rs:57-76](../../src/domain/world/types.rs#L57-L76)) once no reference remains in this file or `movement.rs`/`blueprint.rs`/`events.rs`. Also remove `TerrainType` from the re-export list in [src/domain/world/mod.rs:69](../../src/domain/world/mod.rs#L69).

Downstream crates — `game`, `sdk`, `campaign_builder` — are migrated in Phases 4–5 before this compiles cleanly; treat 2.1–2.5 plus 4 and 5 as one non-shippable branch until all crates compile. In addition to the files named in Phases 4–5, these sites also construct `Tile`/reference `TerrainType` and must be updated before the crate compiles (surfaced by grep, not by the phase 4/5 task descriptions below, so track them explicitly):

- `src/game/systems/input/exploration_movement.rs` — see Phase 4.4a (Walk on Water).
- `src/game/systems/exploration_spells.rs:1174-1182` — test helper constructing `Tile::new(.., TerrainType::Mountain, ..)`.
- `src/sdk/cli/map_validator.rs:690,898,927` — test helpers constructing tiles with `TerrainType::Grass`.

#### 2.5a `Tile::new` call-site ripple

`Tile::new` now takes a `&TerrainDatabase`. Every existing call site (test and production) that previously passed only `(x, y, TerrainType::X, WallType::Y)` must be updated to also pass a database reference — this is the majority of the mechanical work in this phase. Prefer a small shared test helper (e.g. `test_terrain_db()` returning a `TerrainDatabase` pre-populated with the Phase 3 built-ins) so individual tests don't each construct their own database.

#### 2.6 Testing Requirements

- Update every unit/doc test in `types.rs`, `movement.rs`, `blueprint.rs`, `events.rs` that constructs a `Tile`/`TileCode`/`EncounterTable` with a `TerrainType` literal to use the Phase 3 built-in `TerrainId` constants, threading a test `TerrainDatabase` into `Tile::new`.
- `test_tile_new_seeds_blocked_from_terrain_database` — Water/Mountain IDs seed `blocked = true`, others `false`; asserts `blocked` is still a plain field that can be mutated afterward (e.g. `tile.blocked = false;` compiles and holds).
- `test_tile_code_maps_to_builtin_terrain_ids`.
- `test_effective_height_uses_supplied_terrain_height`.

#### 2.7 Deliverables

- [ ] `Tile.terrain: TerrainId`
- [ ] `Tile.blocked` remains a stored, mutable `bool` field; only its Phase-2.1 seeding logic in `Tile::new` changes
- [ ] `Tile::new` takes `&TerrainDatabase`; all call sites updated
- [ ] `effective_height` takes `terrain_height: f32` instead of `TerrainType`
- [ ] `check_tile_blocked` / `Tile::is_blocked()` unchanged (still no `db` parameter)
- [ ] `TileCode` → built-in `TerrainId` mapping
- [ ] `EncounterTable.terrain_modifiers: BTreeMap<TerrainId, f32>`
- [ ] `TerrainType` enum and its `mod.rs` re-export deleted
- [ ] All domain-layer tests pass

#### 2.8 Success Criteria

`cargo check -p antares --lib` fails only in `src/game/` and `src/sdk/` (expected — migrated in later phases), with zero remaining references to `TerrainType` in `src/domain/`.

---

### Phase 3: Built-in Terrain Content and Loading

#### 3.1 Author `data/terrain.ron`

12 `TerrainDefinition` entries per the Reserved ID Range table, texture paths pointing at `assets/textures/terrain/{ground,grass,water,lava,swamp,stone,dirt,forest_floor,mountain,sand,snow,ice}.png`, roughness values carried over from the current `roughness_for` match ([terrain_materials.rs:84-96](../../src/game/systems/terrain_materials.rs#L84-L96)) for the 9 existing terrains, and reasonable new values for Sand (~0.85), Snow (~0.3), Ice (~0.05, near-water slipperiness).

#### 3.2 Add built-in `TerrainId` constants

`pub mod builtin` in `terrain.rs` with named `pub const GROUND: TerrainId = 13_000;` etc. for all 12 — gives engine code and tests a compile-time-checked way to reference built-ins (used by Phase 2's `TileCode` mapping and Phase 4/5's engine logic) without reintroducing a closed enum.

#### 3.3 Wire loading into `ContentDatabase` / `campaign_loader`

Add `TerrainDatabase` alongside the other databases in `src/sdk/database.rs`'s `ContentDatabase` and `src/domain/campaign_loader.rs`, loading `data/terrain.ron` as the base set with a per-campaign `campaigns/<name>/data/terrain.ron` **merge** (same merge convention as `merge_landscape`/`merge_furniture` — campaign entries with new IDs add, campaign entries re-using a built-in ID override it). Use the `missing_ok` arm of `impl_ron_database!` so an absent campaign `terrain.ron` is not an error.

#### 3.4 Generate Sand/Snow/Ice textures

Extend the placeholder generator (Phase 5.2 also touches this file) or hand-author `assets/textures/terrain/{sand,snow,ice}.png` at the same resolution/convention as the existing 9.

#### 3.5 Testing Requirements

- `test_load_default_terrain_ron` — asserts `data/terrain.ron` parses and contains all 12 built-in IDs.
- `test_terrain_database_merge_campaign_override` — a campaign `terrain.ron` re-using ID `13001` overrides the built-in Grass definition; a new ID `13100` adds a custom entry.
- `test_campaign_loader_missing_terrain_ron_uses_builtin_only`.

#### 3.6 Deliverables

- [ ] `data/terrain.ron` with 12 entries
- [ ] `terrain::builtin::*` constants
- [ ] `ContentDatabase`/`campaign_loader` load + merge terrain data
- [ ] Sand/Snow/Ice textures present under `assets/textures/terrain/`
- [ ] All tests pass

#### 3.7 Success Criteria

`ContentDatabase::load_campaign("data/test_campaign")` returns a `TerrainDatabase` with 12+ entries and no errors.

---

### Phase 4: Rendering and Gameplay Systems

#### 4.1 Rewrite `TerrainMaterialCache`

Replace the 9 named `Option<Handle<StandardMaterial>>` fields in [terrain_material_cache.rs](../../src/game/resources/terrain_material_cache.rs) with `handles: HashMap<TerrainId, Handle<StandardMaterial>>`; `get`/`set`/`is_fully_loaded` become map operations (`is_fully_loaded` compares `handles.len()` against the injected `&TerrainDatabase`'s `len()`).

#### 4.2 Rewrite `terrain_materials.rs`

Delete the `TEXTURE_*` constants and `texture_path_for`/`roughness_for` matches ([terrain_materials.rs:24-96](../../src/game/systems/terrain_materials.rs#L24-L96)). `load_terrain_materials_system` iterates `TerrainDatabase::all_definitions()` (injected as a `Res<TerrainDatabase>`, populated during campaign load before this startup system runs) instead of the hardcoded `[TerrainType; 9]` array ([terrain_materials.rs:140-150](../../src/game/systems/terrain_materials.rs#L140-L150)), reading `texture_path`/`roughness` straight from each `TerrainDefinition`.

#### 4.3 Rewrite `map.rs` mesh-spawn dispatch

Replace the exhaustive wall-tint match ([map.rs:2002-2012](../../src/game/systems/map.rs#L2002-L2012)) with `definition.color`. Replace the tile-spawn match ([map.rs:1704-1995](../../src/game/systems/map.rs#L1704-L1995)) with a match on `definition.mesh_style` (`Water` → existing water branch, `Mountain` → existing mountain branch, `Flat` → existing generic-floor branch) plus `should_spawn_grass_cover` becoming `definition.vegetation != TerrainVegetation::None`.

#### 4.4 Update `hud.rs`, `item_world_events.rs`, `vegetation_placement.rs`

- `automap_tile_color` ([hud.rs:2578-2586](../../src/game/systems/hud.rs#L2578-L2586)): bucket by `definition.mesh_style == Water` vs `definition.vegetation != None` vs default.
- `effective_floor_clearance` ([item_world_events.rs:392-395](../../src/game/systems/item_world_events.rs#L392-L395)): `definition.vegetation != TerrainVegetation::None`.
- `vegetation_placement.rs` ([lines 425, 472, 476](../../src/game/systems/vegetation_placement.rs#L425)): tree/shrub spawning gated on `definition.vegetation == TerrainVegetation::Forest`; grass-blade decoration on `definition.vegetation != TerrainVegetation::None`.

#### 4.4a Update Walk on Water spell (`exploration_movement.rs`)

`should_override_water` ([exploration_movement.rs:178-186](../../src/game/systems/input/exploration_movement.rs#L178-L186)) checks `matches!(t.terrain, TerrainType::Water)` to decide whether the Walk on Water buff applies to the target tile. Inject `Res<TerrainDatabase>` and replace with `db.get_by_id(t.terrain).is_some_and(|d| d.mesh_style == TerrainMeshStyle::Water)`, so the buff also works on any custom campaign terrain authored with `mesh_style: Water`. `with_water_override` ([exploration_movement.rs:196-213](../../src/game/systems/input/exploration_movement.rs#L196-L213)) needs no change — it mutates `tile.blocked` directly, which per Phase 2.1 remains a plain field.

#### 4.5 Testing Requirements

- Port every existing test in `terrain_material_cache.rs`, `terrain_materials.rs` that iterates the old fixed 9-array to iterate `TerrainDatabase::all_definitions()` instead (e.g. `test_terrain_material_cache_is_fully_loaded_true_when_all_set` becomes DB-driven).
- `test_map_spawns_mountain_mesh_for_mountain_style`, `test_map_spawns_water_mesh_for_water_style`, `test_grass_cover_spawns_for_vegetation_terrains` (parametrized over `TerrainVegetation` variants).
- `test_automap_color_buckets_by_mesh_style_and_vegetation`.
- Add coverage for the new Sand/Snow/Ice built-ins reaching the generic `Flat` branch with correct material/roughness.
- `test_walk_on_water_override_applies_to_any_water_mesh_style_terrain` — including a custom, non-built-in terrain authored with `mesh_style: Water`.

#### 4.6 Deliverables

- [ ] `TerrainMaterialCache` is `HashMap`-backed
- [ ] `terrain_materials.rs` fully data-driven, no hardcoded texture/roughness constants
- [ ] `map.rs`, `hud.rs`, `item_world_events.rs`, `vegetation_placement.rs` dispatch off `TerrainDefinition` fields
- [ ] `exploration_movement.rs`'s Walk on Water check is `TerrainDatabase`-driven
- [ ] All tests pass

#### 4.7 Success Criteria

Running the game against `data/test_campaign` (or `campaigns/tutorial`) renders all 12 built-in terrain types correctly, including a manually-placed Sand/Snow/Ice tile, with correct blocking, height, and vegetation behavior — verified via the `run` skill.

---

### Phase 5: SDK and Campaign Builder Tooling

#### 5.1 Update `src/sdk/cli/map_builder.rs`

`parse_terrain` ([map_builder.rs:592-607](../../src/sdk/cli/map_builder.rs#L592-L607)) looks up `TerrainDatabase::get_by_name` (case-insensitive) instead of matching a closed set of string literals, falling back to `builtin::GROUND` with the existing warning on no match. The glyph export match ([map_builder.rs:272-282](../../src/sdk/cli/map_builder.rs#L272-L282)) becomes a lookup: built-ins keep their existing glyphs via a small `builtin_glyph(id) -> Option<char>` table; anything else (custom campaign terrain) exports a generic glyph (e.g. `'?'` or first letter of name) since ASCII export is a lossy convenience format.

#### 5.2 Update `src/sdk/cli/texture_generator.rs`

Replace the hardcoded `TERRAIN_SPECS` array ([texture_generator.rs:345-418](../../src/sdk/cli/texture_generator.rs#L345-L418)) with generation driven by `TerrainDatabase::all_definitions()` (loaded from `data/terrain.ron` plus campaign overrides), so `antares-sdk textures generate` produces a placeholder texture for every defined terrain, built-in or custom, without editing Rust.

#### 5.3 Update `src/sdk/templates.rs`

Replace the three literal `TerrainType::Grass|Stone|Forest` calls (lines 765, 817, 857) with `terrain::builtin::GRASS`/`STONE`/`FOREST`.

#### 5.4 Add `terrain_editor.rs` to Campaign Builder

New `sdk/campaign_builder/src/terrain_editor.rs`, mirroring `landscape_editor.rs`: list/create/edit/delete `TerrainDefinition` entries (name, texture path with an asset picker, roughness slider, mesh style dropdown, vegetation dropdown, blocked checkbox, height, color swatch), backed by `campaign_io.rs` load/save of `terrain.ron`.

#### 5.5 Rework `map_editor.rs` terrain coupling

Replace the hardcoded terrain palette arrays (e.g. lines ~5415-5423, ~5605-5613) and the two hardcoded `Color32` swatch tables (~3806-3814, ~8844-8852) with reads from the loaded `TerrainDatabase`. Two separate functions need updating, not one repeated in two places:

- `TerrainEditorState::apply_to_metadata_for_terrain` (~1368-1420): exhaustive per-variant dispatch — replace with dispatch on `TerrainDefinition.vegetation`/`mesh_style` (e.g. show grass-density control when `vegetation != None`, tree-type control when `vegetation == Forest`, rock-variant control when `mesh_style == Mountain`, water-flow control when `mesh_style == Water`).
- `show_terrain_specific_controls` (~9279-9520): a separate helper that also dispatches per-terrain-variant to render inspector widgets — apply the same `vegetation`/`mesh_style`-based dispatch here.

#### 5.6 Testing Requirements

- `test_parse_terrain_resolves_custom_campaign_terrain_by_name`.
- `test_texture_generator_covers_all_database_terrains`.
- New `terrain_editor.rs` tests mirroring `landscape_editor.rs`'s CRUD test suite (add/edit/delete/round-trip through `terrain.ron`).
- `map_editor.rs` tests updated wherever they assert a fixed terrain palette length/contents.

#### 5.7 Deliverables

- [ ] `map_builder.rs` terrain parsing/export is DB-driven
- [ ] `texture_generator.rs` generates from `TerrainDatabase`
- [ ] `templates.rs` uses `builtin::*` constants
- [ ] `terrain_editor.rs` added to Campaign Builder
- [ ] `map_editor.rs` terrain palette/swatches/inspector are DB-driven
- [ ] All SDK and Campaign Builder tests pass

#### 5.8 Success Criteria

Campaign Builder's terrain editor can create a new "Volcanic Rock" terrain with a custom texture and `blocked: true`, save it to a campaign's `terrain.ron`, and immediately use it in the map editor's terrain palette without any Rust rebuild.

---

### Phase 6: Data Migration, Repo-Wide Cleanup, and Docs

#### 6.1 Migrate map RON terrain values

Audit every `campaigns/**/data/maps/*.ron` and `data/test_campaign/data/maps/*.ron` for `terrain: <Variant>` fields (RON adjacently-tagged enum names) and rewrite to the corresponding built-in `TerrainId` integer literal per the Reserved ID Range table (e.g. `terrain: Grass` → `terrain: 13001`). Write a one-off migration script (`sdk/campaign_builder/src/bin/migrate_maps.rs` already exists as precedent for this kind of tool) rather than hand-editing.

**Save-file compatibility (explicit decision, not silently dropped):** player save files (`campaigns/*/saves/*.ron`, gitignored, not tracked in this repo) serialize `Tile.terrain` the same way map files do, and are not covered by the migration script above. This is a breaking change for existing saves — `SaveGame::validate_version` ([save_game.rs:252](../../src/application/save_game.rs#L252)) already rejects incompatible major versions, so bump the save format's major version component as part of this phase and let existing saves fail the existing version check rather than fail with an opaque RON parse error. No save-data migration is provided.

#### 6.2 Repo-wide `TerrainType::` audit

`grep -rn "TerrainType" src/ sdk/ tests/` must return zero results after this phase. Update every remaining reference, including sites already known from earlier phases (`src/domain/world/mod.rs` re-export, `src/game/systems/input/exploration_movement.rs`, `src/game/systems/exploration_spells.rs`, `src/sdk/cli/map_validator.rs`) plus every test file surfaced by the grep, at minimum: `tests/map_authoring_test.rs`, `tests/visual_quality_validation_test.rs`, `tests/map_content_tests.rs`, `tests/rendering_visual_metadata_test.rs`, `sdk/campaign_builder/tests/rotation_test.rs`, `sdk/campaign_builder/tests/bug_verification.rs`, `sdk/campaign_builder/tests/integration_tests.rs`. Use `terrain::builtin::*` constants or raw `TerrainId` literals.

#### 6.3 Update documentation

- `docs/explanation/modding_guide.md` — add a Terrain section matching the existing items/monsters/landscape RON-authoring pattern.
- `docs/explanation/terrain_texture.md` — rewrite; texture paths are now per-`TerrainDefinition`, not fixed filenames.
- `docs/how-to/use_terrain_specific_controls.md` — update to describe vegetation/mesh-style-driven inspector controls and the new Sand/Snow/Ice terrains.
- `docs/reference/map_ron_format.md` — update the terrain legend to numeric IDs.
- `docs/reference/architecture.md` §4.2 — update `TerrainType`/`Tile` spec to `TerrainDefinition`/`TerrainId`.
- `docs/reference/monster_creature_mapping_reference.md` — add the 13000+ terrain range to the shared ID-range table.

#### 6.4 Testing Requirements

- Full workspace test suite green after migration (excluding the intentionally-skipped `cargo test --doc --workspace`, per repo convention).
- `test_all_campaign_maps_load_with_numeric_terrain_ids` — loads every campaign under `campaigns/` and `data/test_campaign` and asserts no RON parse errors.
- Manual smoke test via the `run` skill: load the tutorial campaign, walk across Grass/Forest/Water/Mountain tiles, confirm blocking and vegetation behavior is unchanged from pre-migration.

#### 6.5 Deliverables

- [ ] All map RON files migrated to numeric `terrain` IDs
- [ ] Zero remaining `TerrainType` references anywhere in the repo
- [ ] All six docs above updated
- [ ] Full test suite green

#### 6.6 Success Criteria

`cargo nextest run --all-features` green across both workspace crates. `grep -rn "TerrainType" .` (excluding `target/` and this plan doc) returns nothing. The tutorial campaign is playable end-to-end with no visual or behavioral regression versus the pre-migration build.

## Copyright

We will follow the [SPDX Spec](https://spdx.github.io/spdx-spec/) for copyright and licensing information.
