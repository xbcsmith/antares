# Implementations

---

## Unified Interactive Objects — Phase 4: Mesh Registry Unification (2026)

**Goal:** Implement Phase 4 of the Unified Interactive Objects plan — unify the
split `LandscapeMeshDatabase` / `FurnitureMeshDatabase` lookups used by
rendering code into a single `ObjectMeshDatabase` keyed by arbitrary strings,
so map-event `mesh_id` values can be human-readable names like `"barred_door"`
instead of raw numeric IDs. Legacy numeric IDs are preserved as string keys
(`"11001"`) for backward compatibility.

### What Changed

**`src/domain/world/object_mesh.rs`** (new)

- `ObjectMeshError` — four variants: `ReadError`, `ParseError`,
  `AssetReadError { path, reason }`, `ValidationError`. The `AssetReadError`
  field is named `reason` (not `source`) to avoid thiserror's implicit error
  source trait requirement.
- `ObjectMeshRegistry` — private RON deserialization struct
  `{ meshes: HashMap<String, String> }` mapping string keys to
  campaign-relative asset file paths.
- `ObjectMeshDatabase` — `HashMap<String, CreatureDefinition>` with:
  - `new()` / `Default`
  - `load_from_registry(registry_path, campaign_root)` — reads
    `object_mesh_registry.ron`, iterates entries, parses each as
    `CreatureDefinition`, inserts by key.
  - `merge_landscape(landscape)` — inserts each landscape mesh using
    `id.to_string()` as key; `entry().or_insert_with()` prevents primary
    entries from being overwritten.
  - `merge_furniture(furniture)` — same pattern for furniture meshes.
  - `lookup(key) -> Option<&CreatureDefinition>` — direct HashMap get.
  - `has_mesh`, `all_mesh_ids`, `is_empty`, `count`, `validate`.
- 9 unit tests covering: empty database, lookup miss, merge-empty no-op,
  round-trip load, missing-asset error path, and primary-entry-not-overwritten
  invariant.

**`src/domain/world/mod.rs`** (modified)

- Added `pub mod object_mesh;` declaration.
- Added `pub use object_mesh::{ObjectMeshDatabase, ObjectMeshError};` re-export.

**`src/domain/campaign_loader.rs`** (modified)

- Added `object_meshes: ObjectMeshDatabase` field to `GameData`.
- `GameData::new()` initialises it with `ObjectMeshDatabase::new()`.
- `GameData::validate()` calls `self.object_meshes.validate()`.
- `load_game_data()` calls new `load_object_meshes` helper.
- New `load_object_meshes(&self, landscape_meshes, furniture_meshes)` method:
  loads primary registry if `data/object_mesh_registry.ron` exists (empty
  database otherwise), then merges landscape and furniture mesh databases.

**`src/sdk/database.rs`** (modified)

- Added `ObjectMeshLoadError(String)` variant to `DatabaseError`.
- Added `pub object_meshes: ObjectMeshDatabase` field to `ContentDatabase`.
- `ContentDatabase::new()` initialises it with `ObjectMeshDatabase::new()`.
- Both `load_campaign()` and `load_core()` now:
  1. Load a local `furniture_meshes_local` from `furniture_mesh_registry.ron`
     if present (not stored as a `ContentDatabase` field — merge only).
  2. Build `object_meshes` by loading the primary registry (if present),
     then merging landscape and furniture databases.
- `validate_landscape_content()` calls `self.object_meshes.validate()`.
- 3 integration tests: P4-OM1 (primary registry loads and is queryable),
  P4-OM2 (missing primary falls back to landscape/furniture merge),
  P4-OM3 (primary entry survives merge — not overwritten by legacy IDs).

**`src/game/systems/map.rs`** (modified)

- `spawn_event_meshes`: parameter changed from
  `landscape_meshes: &world::LandscapeMeshDatabase` to
  `object_meshes: &world::ObjectMeshDatabase`; lookup changed to
  `object_meshes.lookup(mesh_id_str)` (no numeric parse).
- `spawn_landscape_placements`: parameter changed to
  `object_meshes: &world::ObjectMeshDatabase`; lookup changed to
  `object_meshes.lookup(&mesh_id.to_string())`.
- `try_spawn_terrain_tree_as_landscape_mesh`: parameter changed to
  `object_meshes: &world::ObjectMeshDatabase`; lookup changed to
  `object_meshes.lookup(&mesh_id.to_string())`.
- All 6 call sites updated from `&content.0.landscape_meshes` to
  `&content.0.object_meshes`.

**`src/sdk/map_editor.rs`** (modified)

- Added "Object Meshes" Campaign Builder panel functions:
  - `browse_object_meshes(db) -> Vec<(String, String)>` — returns all
    registered mesh IDs with their names, sorted by ID.
  - `suggest_object_mesh_ids(db, partial) -> Vec<(String, String)>` — filters
    IDs containing `partial` (case-insensitive).
  - `is_valid_object_mesh_id(db, mesh_id) -> bool` — `has_mesh` wrapper.
- 5 tests in `mod object_mesh_tests` covering: browse returns all IDs,
  suggest filters correctly, invalid ID returns false, empty partial returns
  all, integration test with loaded campaign.

**`data/test_campaign/data/object_mesh_registry.ron`** (new)

Primary string-keyed registry for the test campaign, with entries for
`oak_tree`, `pine_tree`, `dead_tree`, `palm_tree`, and `brush` pointing at
their campaign-relative `CreatureDefinition` asset files.

### Design Decisions

| Decision | Rationale |
| --- | --- |
| String keys (`HashMap<String, …>`) | Allows human-readable names in map event `mesh_id` fields without breaking existing numeric campaigns — legacy IDs become `"11001"` etc. |
| `or_insert_with` in merge | Primary `object_mesh_registry.ron` entries take precedence; legacy registries fill gaps without overwriting. |
| `furniture_meshes` loaded locally in SDK | `ContentDatabase` did not previously expose a `furniture_meshes` field; loading locally for merge avoids adding a new public field and keeps the API surface small. |
| `item_mesh_registry.ron` excluded | Item meshes follow a `DroppedItem` spawn path separate from interactive object rendering. |
| `reason` field name in `AssetReadError` | `thiserror` treats a field named `source` as an error source, requiring `std::error::Error` on the type. The field is `String`, so renaming to `reason` avoids the compile error. |

### Quality Gate Results

```
cargo fmt --all           → clean (no output)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 5268 passed, 0 failed
```

---

## Unified Interactive Objects — Phase 3: Dialogue Routing for `dialogue_id` Events (2026)

**Goal:** Implement Phase 3 of the Unified Interactive Objects plan — route the [E]
interaction for `Treasure`, `Sign`, `Container`, `LockedContainer`, and `LockedDoor`
events through a dialogue tree when the event carries `dialogue_id: Some(id)`, instead
of immediately executing their default effect.

### What Changed

**`src/application/dialogue.rs`**

- Added `EventInteractionContext` struct with `event_position: Position` and `map_id:
MapId` — a generalized context for world-event interaction dialogues, deliberately
  separate from the existing `RecruitmentContext`.
- Added `event_context: Option<EventInteractionContext>` field to `DialogueState`.
- Updated `start()`, `start_simple()` to initialise `event_context: None`.
- Updated `end()` to clear `event_context`.
- Added 3 new tests: default/start/end behaviour of `event_context`.

**`src/application/mod.rs`**

- Added `pub fn collect_treasure_at_position(map_id, position) -> Option<Vec<(usize, ItemId)>>`
  to `GameState` — distributes loot items to party members and removes the event (shared
  logic used by both `handle_events` and `TriggerEvent("collect_treasure")`).

**`src/game/systems/dialogue.rs`**

- Added `PendingEventInteractionContext` resource (parallel to `PendingRecruitmentContext`);
  registered in `DialoguePlugin`.
- Updated `handle_start_dialogue` to consume `PendingEventInteractionContext` and set
  `event_context` on the new `DialogueState`, and to accept a `DespawnEventMesh` message
  writer parameter.
- Updated `handle_select_choice` to accept and pass a `DespawnEventMesh` writer.
- Extended `execute_action` with a `despawn_event_mesh` parameter and four new
  `TriggerEvent` branches:
  - `"collect_treasure"` — calls `collect_treasure_at_position`, logs item distribution,
    emits `DespawnEventMesh`, returns to exploration.
  - `"open_container"` — reads `Container` event at `event_context.event_position`,
    calls `enter_container_inventory`.
  - `"unlock_door"` — checks party for key, unlocks and opens tile, emits
    `DespawnEventMesh`, returns to exploration. Logs failure if no key.
  - `"unlock_container"` — same as unlock_door but for `LockedContainer`; on success
    calls `enter_container_inventory` so the party can take items.
- Added 2 unit tests: `collect_treasure` distributes loot and removes event;
  `collect_treasure` is a no-op when `event_context` is absent.

**`src/game/systems/input/exploration_interact.rs`**

- Added `open_dialogue_for_event(game_state, dialogue_id, event_position, writer,
pending_ctx)` — sets `PendingEventInteractionContext` and writes `StartDialogue`.
- Updated `try_interact_locked_door_event` and `try_interact_locked_container_event`
  to extract `dialogue_id` and call `open_dialogue_for_event` when it is `Some`.
- Updated `try_interact_adjacent_world_events` to:
  - Add `dialogue_id` routing for `Container` (current + adjacent tiles).
  - Add `Treasure` handling for both current and adjacent tiles with `dialogue_id`
    routing.
  - Add `Sign` `dialogue_id` routing in the adjacent-tile loop.
- Updated `handle_exploration_interact` to pass the new `start_dialogue_writer` and
  `pending_event_context` params to all updated sub-functions.
- Added test `test_try_interact_adjacent_world_events_treasure_with_dialogue_id_opens_dialogue`
  verifying the Treasure→dialogue routing and that no `MapEventTriggered` is sent and
  the event is not removed.
- Added test `test_open_dialogue_for_event_sets_pending_context`.

**`src/game/systems/input.rs`**

- Updated `handle_exploration_input_interact` to include `StartDialogue` writer and
  `PendingEventInteractionContext` as Bevy system params and pass them to
  `handle_exploration_interact`.

**`src/game/systems/events.rs`**

- Updated `check_for_events` to skip auto-triggering `MapEvent::Treasure` events that
  carry `dialogue_id: Some(_)` (they require explicit [E] interaction).

### Quality Gate Results

```
cargo fmt --all           → clean (no output)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 5249 passed, 0 failed
```

---

## Unified Interactive Objects — Phase 3: `collect_treasure_at_position` (2026)

**Goal:** Add `GameState::collect_treasure_at_position` to `src/application/mod.rs` as
part of Phase 3 of the Unified Interactive Objects plan. This method provides a direct,
testable API for collecting treasure from a map tile without going through the full
`move_party_and_handle_events` path.

### What Changed

**`src/application/mod.rs`**

- Added `pub fn collect_treasure_at_position(&mut self, map_id: MapId, position: Position) -> Option<Vec<(usize, ItemId)>>` to the `impl GameState` block, placed between `enter_container_inventory` and `return_to_exploration`.
- The method:
  1. Snapshots the `loot: Vec<u8>` from a `MapEvent::Treasure` at the given position (using `world.get_map` + `get_event`), returning `None` for any non-Treasure event or missing position.
  2. Iterates over loot bytes and places each item into the first party member with inventory space via `Inventory::add_item`; items that cannot be placed are silently dropped.
  3. Removes the event from the map via `world.get_map_mut` + `map.remove_event` (one-time collection semantics).
  4. Returns `Some(distributed)` — a `Vec<(usize, ItemId)>` of `(character_index, item_id)` pairs for items successfully placed.
- `ItemId = u8` so no cast is needed; `item_id` is assigned with an explicit type annotation to stay clippy-clean under `-D warnings`.

### Quality Gate Results

```
cargo fmt --all           → clean (no output)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 5245 passed, 0 failed
```

---

## Phase 8: Vegetation Documentation Updates (2026)

**Goal:** Update existing reference documentation for the wind configuration,
domain types, GPU instancing pipeline, and rendering behaviour introduced in
Phases 5–7 of the Vegetation Visual Improvement Plan. Per AGENTS.md, only
existing documents are updated — no new documentation files are created.

### What Changed

**`docs/reference/campaign_content_format.md`**

- Added `data/wind.ron` to the campaign file structure listing.
- Added `## wind.ron Schema` section before `## Validation` — includes: field
  table (name, type, default, valid range), minimal Sine example, full Perlin
  example, `WindSystemKind` reference table, missing-file behaviour, and
  validation rules. A campaign author can implement `wind.ron` from this section
  alone.

**`docs/reference/architecture.md`**

- Module structure (§3.2): added `domain/world/wind.rs` (`CampaignWindConfig`,
  `WindSystemKind`), `game/resources/wind_config.rs` (`WindConfig`),
  `game/systems/advanced_grass.rs` (wind extension material, GPU batch), and
  `game/systems/grass_instancing.rs` (instancing pipeline).
- Game Layer description (§3.3): noted `WindConfig` Bevy resource.
- Data structures (§4.2.2 Wind Configuration): new subsection with
  `WindSystemKind` and `CampaignWindConfig` structs; relationship between domain
  type and Bevy resource.
- Architecture evolution (§13.1): added **Phase 10: Vegetation Visual Improvement
  (Wind + GPU Instancing)** entry covering all seven new source files and the
  bytemuck dependency.
- Compliance (§13.2): added **Wind System** compliance bullet confirming full
  stack from domain through render pipeline.

**`docs/reference/sdk_api.md`**

- Added `wind: CampaignWindConfig` to `ContentDatabase` fields table.
- Added `path/data/wind.ron (optional)` to the `load_campaign` file list.
- Added `#### Wind Configuration` subsection with field table, validation rules,
  and a runnable usage example.

**`CHANGELOG.md`**

- Added four ADDED entries under `## [Unreleased]` for Phases 5, 6, 7, and 8 of
  the Vegetation Visual Improvement Plan, using conventional-commit style
  consistent with the existing changelog format.

### Quality Gate Results

Documentation-only changes — no Rust source modified. The four quality gates
(fmt, check, clippy, nextest) remain passing from Phase 7.

---

## Phase 7: GPU Instancing for Grass (2026)

**Goal:** Connect the existing `GrassInstanceBatch` infrastructure to a real render
pass so all grass clumps in a spatial chunk share a single indexed draw call,
dramatically reducing entity count and CPU overhead on grass-dense maps.

### What Changed

**`Cargo.toml`**

- Added `bytemuck = { version = "1", features = ["derive"] }` as a direct
  dependency for `cast_slice` in the GPU buffer upload path.

**`assets/shaders/grass_instanced.wgsl`** (new)

- Instanced grass vertex shader with two vertex buffer inputs:
  - Buffer 0 (`VertexStepMode::Vertex`): standard mesh attributes from Bevy's
    `Vertex` struct — position, normal, UV, vertex color.
  - Buffer 1 (`VertexStepMode::Instance`): `GrassInstance` struct at
    `@location(8-12)` — world position, wind phase, surface normal, scale,
    Y-axis rotation.
- Reproduces the Phase-6 wind paths (`None` / `Sine` / `Perlin`) and the
  three-stop vertex-color gradient from Phase-4.
- Per-instance `i_phase` offset staggers the sine wave so adjacent clumps
  don't sway in perfect synchrony.
- Wind bind group at `@group(3)` (after Bevy's three standard MeshPipeline
  groups at 0–2): `wind: GrassWindUniform`, `wind_noise`, `wind_sampler`.
- `textureSampleLevel` used in the vertex stage (WGSL vertex stage cannot use
  implicit-derivative `textureSample`).

**`src/game/systems/grass_instancing.rs`** (new)

- **`GrassRenderMode`** resource — `PerEntity` (Phase-6 path) vs. `Instanced`
  (Phase-7, default). Guards both the per-blade `Mesh3d` spawn and the
  instanced batch spawn so the two paths never render simultaneously.
- **`GrassRenderWorldAvailable`** marker resource — only inserted when
  `RenderApp` is present. Prevents `build_grass_instance_batches_system` from
  spawning `Mesh3d` entities (which trigger Bevy's render-world sync hook) in
  unit-test environments that use `MinimalPlugins`.
- **`GrassInstanceGpu`** (`repr(C)`, `bytemuck::Pod + Zeroable`, 48 bytes) —
  per-instance GPU data matching the WGSL `GrassInstance` struct layout.
- **`GrassInstanceBuffer`** render-world component — holds the uploaded GPU
  vertex buffer and instance count.
- **`GrassWindBindGroupResource`** render-world resource — holds the wind
  uniform bind group for `@group(3)`.
- **`GrassInstancedPipeline`** — `SpecializedMeshPipeline` wrapping
  `MeshPipeline`; appends the instance vertex buffer layout and the wind bind
  group layout to every specialized descriptor.
- **`DrawGrassInstanced`** type alias for the render-command chain:
  `SetItemPipeline → SetMeshViewBindGroup<0> →
SetMeshViewBindingArrayBindGroup<1> → SetMeshBindGroup<2> →
SetGrassWindBindGroup<3> → DrawGrassInstancedInner`.
- Systems wired into `RenderApp`:
  - `init_grass_instanced_pipeline` (`RenderStartup`) — creates the pipeline
    and wind bind group layout.
  - `prepare_grass_instance_buffers` (`PrepareResources`) — uploads
    `GrassInstanceBatch.instances` via `bytemuck::cast_slice`.
  - `prepare_grass_wind_bind_group` (`PrepareBindGroups`) — builds the wind
    uniform buffer + noise texture bind group from extracted resources.
  - `queue_grass_instanced` (`QueueMeshes`) — queues each `GrassInstanceBatch`
    entity into `Opaque3d` using `BinnedRenderPhaseType::NonMesh`.
- **`GrassInstancingPlugin`** — gates all render wiring (including
  `ExtractComponentPlugin`) on `RenderApp` being present. Without a render
  world (test environments), only `GrassRenderMode` is registered.

**`src/game/systems/advanced_grass.rs`** (modified)

- `ExtractComponent` implemented for `GrassInstanceBatch` (in
  `grass_instancing.rs`) so it is copied to the render world.
- `spawn_grass_clump` gains a `render_mode: GrassRenderMode` parameter.
  - `PerEntity`: per-clump entity gets `Mesh3d + MeshMaterial3d` (Phase-6 path).
  - `Instanced` (default): per-clump entity spawns WITHOUT render components so
    the standard material pipeline ignores it; instance data is gathered by
    `build_grass_instance_batches_system`.
- `spawn_grass_cached`, `spawn_grass_cached_with_exclusions`, and `spawn_grass`
  gain a matching `render_mode` parameter threaded through to
  `spawn_grass_clump`.
- `build_grass_instance_batches_system` gains two additional system parameters:
  - `render_mode: Option<Res<GrassRenderMode>>` — activates the instanced path
    when `GrassRenderMode::Instanced` (replaces the `config.enabled` flag check).
  - `render_world_available: Option<Res<GrassRenderWorldAvailable>>` — gates
    `Mesh3d` + `NoFrustumCulling` spawning on render world presence.
  - In instanced + render-world mode, each `GrassInstanceBatch` entity gets
    `Mesh3d(clump_mesh)` (so Bevy allocates GPU vertex/index buffers) plus
    `NoFrustumCulling` (culling happens at `GrassCluster` level).
- **Chunk-level visibility** (Phase 7.4/7.6 deliverable): `build_grass_instance_batches_system`
  now queries `Option<&Visibility>` on each `GrassCluster` and skips clusters
  with `Visibility::Hidden`. This makes `grass_distance_culling_system` the
  chunk-level culling mechanism for the instanced path — instances from culled
  clusters are excluded from every frame's batch rebuild. Covered by
  `test_build_grass_instance_batches_skips_hidden_clusters`.
- Imports `NoFrustumCulling` and `GrassRenderMode` from the new module.
- All three in-module test spawn calls updated to pass
  `GrassRenderMode::PerEntity` so existing assertions on `GrassBlade` and
  `GrassClump` counts remain valid.

**`src/game/systems/map.rs`** (modified)

- `MapRenderingPlugin::build` adds `GrassInstancingPlugin` before
  `MaterialPlugin::<GrassMaterial>`.
- `spawn_map_system`, `spawn_map`, and `spawn_map_markers` gain a
  `render_mode: GrassRenderMode` parameter threaded through to the single
  `spawn_grass_cached_with_exclusions` call site.

**`campaigns/tutorial/assets/shaders/`** (new directory)

- `grass.wgsl` and `grass_instanced.wgsl` copied here so Bevy can resolve them.
  The game binary sets `BEVY_ASSET_ROOT` to the active campaign directory with
  `file_path: ""`, so every asset path is resolved relative to that root.
  `"assets/shaders/grass.wgsl"` → `<campaign>/assets/shaders/grass.wgsl`,
  matching the same convention as textures (`"assets/textures/…"`).
  The original `assets/shaders/` copies at the repo root remain as the
  authoritative source; campaign copies are deployed from there.

**`GRASS_WIND_SHADER_PATH`** and **`GRASS_INSTANCED_SHADER_PATH`** updated from
`"shaders/grass.wgsl"` / `"shaders/grass_instanced.wgsl"` to
`"assets/shaders/grass.wgsl"` / `"assets/shaders/grass_instanced.wgsl"`.
The original `"shaders/…"` paths resolved to `<campaign>/shaders/…` which did
not exist, producing `Path not found` errors and no grass rendering.

### Design Decisions

| Decision                                                    | Rationale                                                                                                                                                                                                           |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SpecializedMeshPipeline` (not `SpecializedRenderPipeline`) | Reuses `MeshPipeline::specialize(key, layout)` for the automatic mesh vertex buffer layout + view/mesh bind group setup. Avoids duplicating Bevy's attribute-location mapping.                                      |
| `BinnedRenderPhaseType::NonMesh`                            | Bypasses GPU preprocessing (indirect draw buffers). Our per-instance transforms live in vertex buffer 1, not in Bevy's mesh uniform buffer, so preprocessing would add overhead for no gain.                        |
| `Opaque3d` phase                                            | Grass is opaque/masked — no sorting needed. Binned phase allows multi-draw batching in future.                                                                                                                      |
| Wind at `@group(3)`                                         | `MeshPipeline::specialize` occupies bind group indices 0–2. Appending at index 3 avoids collision without forking the base pipeline.                                                                                |
| `GrassRenderWorldAvailable` marker                          | `ExtractComponentPlugin` adds `SyncComponentPlugin` which registers a component hook requiring `PendingSyncEntity`. Gating both on `RenderApp` presence prevents panics in MinimalPlugins test environments.        |
| Per-clump entities kept (no `Mesh3d`)                       | Preserves the existing `GrassCluster` distance-culling and LOD systems. In instanced mode the per-clump entities are invisible (no render components); their transforms feed `build_grass_instance_batches_system`. |

### Quality Gate Results

```
cargo fmt         → no output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 5222/5222 passed, 0 failed
```

cargo check → Finished with 0 errors
cargo clippy → Finished with 0 warnings
cargo nextest run → 5221/5221 passed, 0 failed

````

---

## Phase 2: Cross-Pattern Leaf Volume (2026)

**Goal:** Add a perpendicular second pass of leaf geometry inside `append_leaf_card`
so the canopy appears volumetric from all camera angles, not just face-on.

### What Changed

**`src/game/systems/advanced_trees.rs`**

- **`append_leaf_shape`** (new private helper) — dispatches to the per-species
  leaf/frond stamping function (Oak, Birch, Willow, Palm, Shrub) for one plane.
  Separates the species dispatch from the cross-pattern logic so both passes
  can reuse the same code path without duplication.

- **`append_leaf_card_cross`** (new helper) — stamps two perpendicular passes:
  first with `side` as-is, then with `side2 = side.cross(up).normalize_or_zero()`.
  `side.cross(up)` is used instead of `direction` to avoid the near-vertical
  collapse that occurs when `up ≈ direction` (because `up = Y.lerp(direction,
  0.35)` for near-vertical branches). `double_sided: true` (from Phase 1)
  covers both winding orders.

- **`append_leaf_card`** updated — dispatches Oak/Birch/Willow/Palm/Shrub
  through `append_leaf_card_cross`; Pine keeps its existing single-pass path
  (triangular needles read well from all angles; cross pass would increase
  overdraw on dense conifer canopy without visual benefit).

- **`LeafPreset.count` reduced 30–40%** for the five cross-pattern species to
  compensate for the doubled polygon budget per card:

  | Species | Old count | New count | Reduction |
  |---------|-----------|-----------|-----------|
  | Oak     | 5         | 3         | 40 %      |
  | Birch   | 3         | 2         | 33 %      |
  | Willow  | 6         | 4         | 33 %      |
  | Palm    | 8         | 5         | 37.5 %    |
  | Shrub   | 6         | 4         | 33 %      |
  | Pine    | 4         | 4         | 0 % (unchanged) |

- **6 new tests** added to the `#[cfg(test)]` module verifying: cross-pattern
  doubles vertex count for Oak, Pine uses single pass, all five cross-pattern
  species produce ≥ 2× vertices vs single pass, `side2` is perpendicular to
  `side`, `LeafPreset.count` reductions are in [30 %, 40 %], Pine count is
  unchanged.

---

## Phase 1: Foliage Detail Texturing + Lit Foliage (2026)

**Goal:** Apply tiling leaf detail textures over existing geometric leaf
silhouettes and switch foliage to lit shading, so each tree species shows
realistic surface detail and light/shade variation.

### What Changed

**`src/bin/generate_foliage_textures.rs`** (new)
Deterministic binary that generates six 512×512 RGBA8 tiling leaf detail
textures. Uses a splitmix64 PRNG (inline, no `rand` crate) and writes to both
`assets/textures/trees/` and `campaigns/tutorial/assets/textures/trees/`.
Each species has a fixed seed (101–106), per-species stamp shape (5-lobe oak,
needle pine, ovate birch, lanceolate willow, frond palm, round shrub), and
neutral-luminance base colour so the `base_color` multiply tint system remains
the species/variant colour mechanism. Self-validates output spec (512×512, 40–85%
opaque coverage, 0.65–0.90 mean luminance) and exits non-zero on violation.

**`Cargo.toml`** — added `[[bin]]` entry for `generate-foliage-textures`.

**`src/game/systems/procedural_meshes.rs`**
- Removed `#[cfg(test)]` from the six `TREE_FOLIAGE_TEXTURE_*` constants,
  `TREE_FOLIAGE_ALPHA_CUTOFF`, and `foliage_texture_path()`.
- `get_or_create_foliage_material`: now loads the species detail texture via
  `super::creature_meshes::load_texture`, sets `base_color_texture: Some(…)`,
  switches to `AlphaMode::Mask(TREE_FOLIAGE_ALPHA_CUTOFF)`, `unlit: false`,
  and `perceptual_roughness: 0.9`. The species `base_color` multiply tint is
  preserved (not forced to WHITE) so the variant-tint system keeps working.
- `get_or_create_foliage_material_variant`: removed the force-reset of
  `base_color_texture = None` and `alpha_mode = Opaque`; variants now inherit
  the detail texture and mask mode from the base material.
- Tests updated: assertions on `base_color_texture`, `alpha_mode`, and `unlit`
  flipped to match the new lit/textured material.

**`tests/foliage_texture_spec_test.rs`** (new)
Integration tests that open the six committed PNGs and assert all output-spec
properties: 512×512 dimensions, 40–85% opaque coverage, 0.65–0.90 mean
luminance, edge-strip presence (confirming seamless wrap). Also asserts that
the `assets/` and `campaigns/tutorial/` copies are byte-identical.

### Per-Card UV Verification

All leaf-card generators in `advanced_trees.rs` already span 0..1 in both U
and V:
- `append_quad`: `[0,0],[1,0],[1,1],[0,1]` ✓
- `append_diamond`: `[0.5,0],[1,0.5],[0.5,1],[0,0.5]` ✓
- `append_triangle`: `[0,0],[1,0],[0.5,1]` ✓

No UV fixes were needed.

### Bug Fixed During Implementation

`draw_ellipse` originally used `semi_a` as the x-bound and `semi_b` as the
y-bound for the pixel iteration loop. A rotated ellipse (e.g. a pine needle
at 90°) extends to `semi_a` pixels in the *y* direction, so the loop was
missing the body of every non-horizontal needle. Fixed by using
`max(semi_a, semi_b)` as the bound in both directions.

---

## Bug Fix: Intermittent Terrain Texture Failure at Startup (2026)

**Goal:** Fix the intermittent startup bug where one terrain tile texture (most
often water or mountain) would display as an error checkerboard instead of its
correct texture.

### Root Cause — Async Texture Loading Race

Bevy's render world assigns each `Assets<Image>` entry a slot in the bindless
texture array when it first prepares the image as a `GpuImage`. The
`StandardMaterial` for each terrain tile stores the slot index for its
`base_color_texture` in a GPU buffer; once written, that slot is never
corrected if the texture was still loading when the bind group was first
prepared.

The terrain textures in `campaigns/tutorial/assets/textures/terrain/` are
**3.8–8.1 MB** each (real photorealistic PNGs). The original implementation
used `asset_server.load()`, which is asynchronous — each PNG is decoded in a
thread-pool worker and inserted into `Assets<Image>` in a later `PreUpdate`
frame. Large files take long enough to decode that, on typical hardware, they
are still loading when the render world's first `PrepareAssets` pass runs.
Whichever texture loses the race has its bind-group slot permanently set to a
fallback (the Bevy error checkerboard), with no subsequent correction.

Two earlier incremental fixes addressed related symptoms but not the root cause:

1. **HudPlugin** — mini-map and automap canvas images moved from `Startup` to
   `PostStartup` to prevent index-based handles from interleaving with terrain
   texture registrations. This eliminated those specific races but left the core
   async-load timing issue intact.
2. **SkyBodyPlugin** — cloud noise texture moved from first-`Update` lazy
   allocation to a `PostStartup` pre-allocation (`preallocate_cloud_noise_system`)
   for the same reason. Also still left the core issue.

### Fix — Stable Terrain Handles at Startup

**`src/game/systems/terrain_materials.rs`** — `load_terrain_materials_system` now uses the stable `AssetServer` path for terrain image registration instead of inserting pathless `Assets<Image>` handles directly. This keeps terrain texture asset IDs reserved before later runtime image allocations occur, which prevents the bind-group slot drift that was causing the intermittent wrong-texture tile result.

The startup path now:

- synchronously decodes each PNG with `Image::from_buffer()`;
- registers the decoded image through `asset_server.add(image)` so its asset ID is reserved early;
- falls back to `asset_server.load()` only if the sync read/decode path fails.

This complements the existing `PostStartup` refresh and keeps terrain materials aligned with the final image set during the first render pass.


- Added `mut images: ResMut<Assets<Image>>` parameter.
- Added a `load_terrain_image_sync` helper that reads each PNG with
  `std::fs::read()` and decodes it with `Image::from_buffer()` in the Startup
  system's thread — **before the first render frame**.
- `images.add(decoded_image)` inserts the decoded image directly into
  `Assets<Image>` with an index-based handle that is immediately valid.
- The material is created with `base_color_texture: Some(handle)` where
  `handle` is the just-inserted, already-loaded image. When the render world's
  `PrepareAssets` first runs, the `GpuImage` already exists and the correct
  slot is written into the material's GPU buffer from the very first frame.
- If the sync read or decode fails (unusual launch context, missing file),
  the helper falls back to `asset_server.load()` (async) and emits a
  `warn!` log, preserving a working path for tests and CI.
- The `BEVY_ASSET_ROOT` env var (set in `main()` before `App::new()`) is
  read to build the full absolute file path, matching the path resolution
  the `AssetPlugin` would use.

### Files Changed

| File                                    | Change                                                                                               |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `src/game/systems/terrain_materials.rs` | Sync load via `load_terrain_image_sync`; added `images` parameter to `load_terrain_materials_system` |
| `src/game/systems/sky_bodies.rs`        | Added `preallocate_cloud_noise_system` PostStartup (earlier fix retained)                            |
| `docs/explanation/implementations.md`   | This entry                                                                                           |

---

## Bug Fix: Map Editor Inspector — Cannot Remove Procedural Trees (2026)

**Goal:** Fix the Campaign Builder Map Editor inspector having no way to remove procedural trees from Forest tiles. The inspector showed `Terrain: Forest` as a read-only label; all vegetation clear/reset buttons only operated on `TileVisualMetadata` (tree species, foliage density, etc.) but never changed `tile.terrain`. Since procedural trees are spawned by `TerrainType::Forest`, they could not be removed from the inspector at all.

### Root Cause

Procedural trees are generated at runtime by `TerrainType::Forest`. The inspector only showed this as a static `ui.label(format!("Terrain: {:?}", tile.terrain))`. Every existing button (Reset Vegetation, Clear Terrain Properties, Reset to Defaults) operated solely on `TileVisualMetadata` fields — not on `tile.terrain`. There was no path from the inspector to change the terrain type.

### Fix

**`show_inspector_panel`** (`sdk/campaign_builder/src/map_editor.rs`):

- Tile data is now captured as an owned snapshot (`tile_snapshot`) before the mutable `ui.group` closure, avoiding borrow conflicts.
- The static `ui.label("Terrain: {:?}")` is replaced with a **`ComboBox`** listing all nine terrain types (`Ground`, `Grass`, `Stone`, `Dirt`, `Forest`, `Water`, `Swamp`, `Lava`, `Mountain`).
- Selecting a different terrain queues a change in `change_terrain_to: Option<TerrainType>`, applied after the group closure via `set_tile` (full undo/redo support).
- Applying the terrain change also:
  - Recalculates `tile.blocked` using the same logic as `paint_tile` (Mountain and Water → blocked; others → unblocked unless wall is Normal).
  - Calls `TerrainEditorState::clear_metadata` on the new tile to remove stale terrain-specific visual overrides (e.g. `tree_type` metadata left on a tile that is no longer Forest).
  - Resets `terrain_editor_state` and invalidates both position caches so both panels reload from the new tile on the next frame.
- **Vegetation Authoring** section now shows a yellow tip when the tile is Forest: `"💡 Procedural trees come from Forest terrain. Change terrain type above to remove them."` — prevents future confusion about which button removes trees.

**`landscape_editor.rs`** (pre-existing clippy violations fixed in same pass):

- `apply_edit` signature changed from `&mut Vec<LandscapeDefinition>` → `&mut [LandscapeDefinition]` (clippy `ptr_arg`).
- `show_edit` signature changed the same way.

### Files Changed

| File                                           | Change                                                                |
| ---------------------------------------------- | --------------------------------------------------------------------- |
| `sdk/campaign_builder/src/map_editor.rs`       | Terrain ComboBox in inspector, Forest vegetation tip, 4 new tests     |
| `sdk/campaign_builder/src/landscape_editor.rs` | `&mut Vec` → `&mut [_]` for `apply_edit` and `show_edit` (clippy fix) |

### Tests Added

- `test_change_terrain_forest_to_ground_clears_tree_type_and_changes_terrain` — terrain changes from Forest to Ground; tree_type and foliage_density are cleared.
- `test_change_terrain_creates_undo_action_for_forest_to_grass` — undo after terrain change restores original terrain and tree_type.
- `test_change_terrain_mountain_to_ground_unblocks_tile` — blocked flag is correctly recalculated; rock_variant cleared.
- `test_change_terrain_preserves_visual_height_when_clearing_metadata` — `clear_metadata` does not touch the visual `height` field.

### Usage

1. Open **Map Editor** → select a tile that has procedural trees (Forest terrain).
2. The **Terrain** row in the inspector is now a dropdown. Change `Forest` → `Ground` (or any other terrain type).
3. The tile is immediately updated with full undo support (`Ctrl+Z` restores it). The terrain-specific visual overrides (tree type, foliage density) are also cleared automatically.
4. Save the map.

---

## Bug Fix: Map Editor Reset/Clear Terrain Buttons Broken (2026)

**Goal:** Fix four compounding bugs that caused "Reset Vegetation", "Reset to Defaults", and "Clear Terrain Properties" to appear to do nothing and, worse, corrupt tile data when Apply was clicked afterwards.

### Root Causes

| #   | Bug                                                                                                       | Effect                                                                                                                                             |
| --- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `last_loaded_terrain_position` never invalidated after clear buttons                                      | Terrain panel never reloads from tile; old (default) values stick                                                                                  |
| 2   | After clearing, `terrain_editor_state = TerrainEditorState::default()` writes `tree_type = TreeType::Oak` | Panel looks identical before/after clear; user thinks nothing happened. Clicking Apply afterwards re-writes Oak to the tile, **undoing the clear** |
| 3   | "Reset Vegetation" called `visual_editor.reset()`                                                         | Incorrectly cleared height/scale/etc. from the staging buffer (one-frame glitch, wrong semantics)                                                  |
| 4   | `apply_terrain_state_to_tile` always wrote `Some(tree_type)`                                              | No way to express "no override — use runtime default" through the Apply pathway                                                                    |

### Fix

**`TerrainEditorState`** gains `use_terrain_override: bool`:

- `default()` → `true` (a freshly-created state is an override by definition)
- `from_metadata()` → `true` only when at least one terrain `Option` field is `Some`; `false` on a tile with no terrain overrides
- `apply_terrain_state_to_tile` → when `false`, calls `clear_metadata` (writes all terrain fields as `None`) instead of writing enum defaults back

**`show_terrain_specific_controls`** adds an "Override terrain settings" checkbox at the top. When unchecked, the dropdowns/sliders are hidden and a grey "No terrain overrides — runtime uses defaults" label is shown.

**"Reset to Defaults"** now sets `use_terrain_override = false` and `last_loaded_terrain_position = None`.

**"Reset Vegetation"** removes the erroneous `visual_editor.reset()` call; sets `use_terrain_override = false`; invalidates both position caches.

**"Clear Terrain Properties"** sets `use_terrain_override = false` and `last_loaded_terrain_position = None`.

**5 new tests** covering: `from_metadata` with no-override tile, `from_metadata` with explicit tree type, clear sets override=false, Apply with override=false clears the tile, Reset Vegetation does not clear visual height.

### Files Changed

`sdk/campaign_builder/src/map_editor.rs`

---

## Map Editor: Landscape Quick-Place + Visual Property Delete Buttons (2026)

**Goal:** Replace the redundant Visual Presets palette panel with a focused Landscape section that lets authors place/remove imported landscape meshes on a tile directly from the inspector. Also add per-property ✕ delete buttons to Visual Properties so individual fields can be cleared without wiping the whole tile.

### Files Changed

| File                                     | Change                                                                                                        |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/map_editor.rs` | New `show_landscape_quick_place`, ✕ buttons in `show_visual_metadata_editor`, 2 new state fields, 3 new tests |
| `docs/explanation/implementations.md`    | This entry                                                                                                    |

### What Changed

**Removed**: The bottom "Visual Presets" `ui.group` (the category-filtered 3×N grid of preset buttons). The preset _dropdown_ inside Visual Properties is kept — it was not redundant, it's a different widget that stages a preset into the editor buffer.

**Added — Landscape section** (replaces Visual Presets panel):

- `MapEditorState` gains `landscape_quick_scale: f32` (default 1.0) and `last_landscape_load_position: Option<Position>`.
- New `show_landscape_quick_place(ui, editor, pos, landscape_definitions)` renders:
  - **Mesh dropdown** — only landscape definitions that have an imported `mesh_id` are shown.
  - **Scale** — `Slider` (0.1–3.0) + `DragValue` text field side by side, matching the Terrain Settings style.
  - **🌳 Place / 🌳 Update** — adds a new `LandscapePlacement` or replaces the existing one for this definition at this tile.
  - **🗑️ Remove** — removes the existing placement (only visible when one exists).
- **Auto-populate**: when `selected_position` changes, the section reads the tile's first `LandscapePlacement` and sets the dropdown + scale to match, so selecting a tile with a placed mesh immediately shows its values.
- Scale is omitted from the stored `LandscapePlacement` when it equals the definition's `default_scale` (keeps RON output clean).

**Added — Visual Property ✕ delete buttons**:

- A `clear_field: [bool; 7]` array is declared at the top of the property block (indices: height, width_x, width_z, scale, y_offset, rotation_y, color_tint).
- Each enabled property row now has a small `✕` button that sets the corresponding flag.
- After all rows, a single block applies any flagged clears: writes `None` directly to the tile (and all selected tiles in multi-select), unchecks the editor buffer, marks `has_changes = true`, and invalidates `last_loaded_visual_position`.
- Result: the user can remove e.g. the custom height from a tile with one click, without affecting any other property and without needing to click the Apply button.

### Usage

1. Select a tile on the map.
2. Scroll to the **🌳 Landscape** section (where Visual Presets used to be).
3. Pick a mesh from the dropdown — only imported meshes (definitions with a `mesh_id`) appear.
4. Adjust Scale if needed.
5. Click **🌳 Place** — the mesh is placed. Click again to return and **🌳 Update** or **🗑️ Remove** it.
6. To clear a single Visual Property (e.g. Height), click the small **✕** next to it — the field is removed from the tile immediately.

---

## Bug Fix: Landscape Editor Mesh Picker — Cannot Assign Mesh to Definition (2026)

**Goal:** Fix the Campaign Builder → Landscape Editor → Edit form having no way to assign or change the `mesh_id` on a `LandscapeDefinition`. The Mesh field was a read-only label; users who imported new meshes via the OBJ Importer had no way to connect them to an existing definition.

### Root Cause

The `show_edit` function displayed `mesh_id` as a static `ui.label(...)`. The field was never made interactive. Additionally, `enter_edit` did not load the list of available meshes from `landscape_mesh_registry.ron`, so there was nothing to pick from even if the UI existed.

### Files Changed

| File                                           | Change                                                                                                          |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/landscape_editor.rs` | Add `available_meshes` field, load registry in `enter_edit`, replace read-only label with ComboBox, add 3 tests |

### What Changed

- `LandscapeEditorState` gains `available_meshes: Vec<CreatureReference>` — populated once per edit session from `data/landscape_mesh_registry.ron`.
- `enter_edit` now takes `campaign_dir: Option<&Path>` and calls `load_available_meshes(campaign_dir)` to populate the field.
- New free function `load_available_meshes` reads the registry file and returns an empty `Vec` gracefully on any error.
- The "Mesh ID" read-only grid row in `show_edit` is replaced with:
  - A **`ComboBox` picker** (when a campaign is open and meshes exist) listing all registry entries as `#<id> – <name>` plus a `None (procedural)` option at the top.
  - A read-only label with a hover hint when no meshes are available (no campaign open, empty registry, or unparseable file).
- `mesh_options_snapshot` is pre-computed before the `ScrollArea`/`Grid` closures to avoid Rust closure-capture conflicts between the `buf` borrow (`self.edit_buffer`) and `self.available_meshes`.
- `apply_edit` already writes `buf.mesh_id` back to `defs[idx]` — no change needed there.
- `show_list`'s `pending_edit` branch now forwards `campaign_dir` to `enter_edit`.
- 3 new tests: registry loaded into state, no-campaign produces empty list, `apply_edit` saves the new mesh ID.

### Usage

1. Import your tree/rock meshes via **Importer → Landscape** (as before). The OBJ importer writes entries to `landscape_mesh_registry.ron` and creates/updates matching definitions in `landscape.ron`.
2. Open **Landscape** tab → right-click any definition → **Edit**.
3. The **Mesh** row now shows a dropdown populated from `landscape_mesh_registry.ron`. Select the desired mesh, click **Save**.
4. The map engine reads `definition.mesh_id` at runtime to spawn the imported mesh. No `.ron` map changes are needed — the definition is the link.

---

**Goal:** Fix the Campaign Builder → Landscape Editor → Delete action doing nothing. Deleting a landscape definition via the right-click context menu should remove the definition from the in-memory list (which propagates to `landscape.ron` on the next save) and immediately prune the corresponding entry from `landscape_mesh_registry.ron` for any definition that had a custom mesh.

### Root Cause

Three compounding issues in `landscape_editor.rs`:

1. `show_list` took `defs: &[LandscapeDefinition]` (immutable slice) — it physically could not remove items even if the delete was handled.
2. `show_list` had no `unsaved_changes: &mut bool` parameter — even if removal worked, it could not mark the campaign dirty.
3. Inside the list loop, the code only reacted to `ItemAction::Edit`; `ItemAction::Delete` (returned by the right-click context menu) fell through silently.

### Files Changed

| File                                           | Change                                                                                                          |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/landscape_editor.rs` | Fix `show_list` signature, handle `ItemAction::Delete`, add `remove_landscape_mesh_registry_entry`, add 6 tests |

### What Changed

- `show_list` now takes `defs: &mut Vec<LandscapeDefinition>` and `unsaved_changes: &mut bool`.
- The call site in `show` passes the mutable defs and `unsaved_changes` through.
- A `pending_delete: Option<usize>` deferred variable is set when `ItemAction::Delete` is captured inside the loop (mirrors the existing `pending_edit` / `pending_selection` pattern).
- After `TwoColumnLayout::show_split` closes, the delete is applied: item removed via `defs.remove(idx)`, selection adjusted (cleared if the deleted row was selected, shifted down if selection was after it), `unsaved_changes` set to `true`, and the texture-validation cache cleared.
- New `remove_landscape_mesh_registry_entry(campaign_dir, mesh_id) -> bool` helper reads `data/landscape_mesh_registry.ron`, retains all entries except the one with the given `id`, and rewrites the file. Silently returns `false` on any I/O error so deletion is never blocked by registry state.
- 6 new unit tests: three for selection-adjustment logic (delete selected item, delete earlier item, delete later item) and three for the registry helper (matching ID removed, missing ID returns false, no file returns false).

---

## Bevy Tonemapping LUT Render Panic Fix (2026)

**Goal:** Fix a startup render panic where Bevy/wgpu rejected view bind groups because tonemapping LUT bindings expected a 3D texture but received a 2D texture view.

### Files Changed

| Area           | Action                                                                                                        |
| -------------- | ------------------------------------------------------------------------------------------------------------- |
| Game binary    | Installed a UUID-backed, known-compatible 1×1×1 3D tonemapping LUT in both the main app and render app        |
| Render startup | Inserts the neutral LUT directly into `RenderAssets<GpuImage>` before view bind groups are prepared           |
| Camera setup   | Set the main 3D camera to `Tonemapping::None` so the game does not depend on Bevy's default LUT sampling path |
| Documentation  | Recorded the render panic fix and validation results                                                          |

### What Changed

- `src/bin/antares.rs` now replaces Bevy's default `TonemappingLuts` handles with a neutral, UUID-backed D3 `Image` whose texture view descriptor is explicitly `TextureViewDimension::D3`.
- The same LUT is inserted directly into the render world's `RenderAssets<GpuImage>` during `RenderStartup`, avoiding timing issues where view bind groups could still see Bevy's incompatible D2 LUT render asset.
- `src/game/systems/camera.rs` now spawns the main game camera with `Tonemapping::None`, avoiding LUT-dependent visual transforms while preserving Bevy's required LUT bind-group shape for sprite, mesh2d, and PBR view bindings.
- This addresses wgpu validation errors like `Texture binding ... expects dimension = D3, but given a view with dimension = D2` in `prepare_sprite_view_bind_groups` and `prepare_mesh_view_bind_groups`.

---

## Landscape Deliverables Audit Fixture Compliance (2026)

**Goal:** Verify the completed landscape implementation plan against repository state and close any remaining fixture-rule gaps found during the audit.

### Files Changed

| Area                | Action                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Test fixtures       | Added `data/config.template.ron` so config-template tests read a stable fixture outside `campaigns/`             |
| Integration tests   | Pointed monster validation tests at `data/` and `data/test_campaign` fixtures instead of scanning live campaigns |
| Shared test helpers | Updated synthetic campaign root paths to use `data/<id>` rather than `campaigns/<id>`                            |
| Implementation docs | Recorded this deliverables audit and fixture-compliance cleanup                                                  |

### What Changed

- The landscape deliverables audit found the landscape feature itself implemented, but also found lingering test fixture reads under `campaigns/`, which violates the project fixture rule for tests.
- `tests/data_validation_tests.rs` now validates only stable monster fixtures in `data/monsters.ron` and `data/test_campaign/data/monsters.ron`.
- `tests/game_config_integration.rs` now validates the copied `data/config.template.ron` fixture instead of `campaigns/config.template.ron`.
- `tests/common/mod.rs` now builds synthetic test campaign paths under `data/`.

---

## Landscape Phase 7 Documentation and Cleanup (2026)

**Goal:** Complete the final landscape documentation and cleanup pass so the feature is documented for modders/SDK users, public landscape APIs have examples, stale tree/brush paths are clearly deprecated or removed, and tutorial/test campaign landscape data matches the validated exporter format.

### Files Changed

| Area                    | Action                                                                                                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Implementation docs     | Recorded this Phase 7 documentation and cleanup slice after Phase 6 testing                                                                                              |
| Reference docs          | Added/refined landscape data files, map placement format, SDK content loading, validation rules, importer output paths, and terrain metadata notes                       |
| Modding/SDK docs        | Documented the Campaign Builder landscape workflow, model importer Landscape export target, category output paths, Landscape tab, and map placement workflow             |
| Rustdoc comments        | Added examples for landscape ID aliases/constants, domain types/databases, runtime landscape metadata, SDK map-editor placement helpers, campaign I/O, and importer APIs |
| Cleanup/data            | Removed the stale tutorial brush furniture mesh registry and normalized tutorial/test landscape mesh RON IDs/names to registry IDs                                       |
| Procedural fallback     | Clarified that foliage texture constants are deprecated test-only paths while imported landscape definitions are preferred                                               |
| Test fixture compliance | Updated parser-only CLI tests and fixture docs to use `data/test_campaign` instead of the live tutorial campaign                                                         |

### What Changed

- `docs/reference/campaign_content_format.md`, `docs/reference/map_ron_format.md`, `docs/reference/sdk_api.md`, `docs/reference/architecture.md`, and `docs/reference/tile_visual_metadata_specification.md` now describe current landscape data files, placement semantics, SDK content loading, terrain-vs-landscape boundaries, and validation behavior.
- `docs/explanation/modding_guide.md`, `docs/how-to/use_terrain_specific_controls.md`, `sdk/campaign_builder/README.md`, and `sdk/campaign_builder/QUICKSTART.md` now explain when to use terrain-specific controls versus Landscape placements, how to import Landscape meshes, and where exported mesh/texture assets are written.
- `docs/explanation/landscape_implementation_plan.md` and `docs/explanation/next_plans.md` now mark the landscape plan as completed/historical rather than describing pre-implementation state as current work.
- Public landscape-related Rust items now include runnable or explicitly ignored examples, including domain database methods, placement validation helpers, `LandscapeEntity`, `LandscapeRenderHints`, `LandscapeEditorState`, `LandscapeEditorSignal`, `MapEditorState` landscape placement helpers, `CampaignBuilderApp` landscape I/O helpers, and `ExportType::Landscape`.
- Tutorial, `data/test_campaign`, and root seed landscape mesh RON files embed the same IDs/names as their registry entries (`11001` through `11005`) instead of the old generic `10001` seed ID.
- The invalid tutorial `data/furniture_mesh_registry.ron` brush entry was removed because brush is now authored through landscape definitions and the referenced furniture mesh file did not exist.
- Procedural fallback tree docs/logging now state that foliage materials use generated geometry and species colors; the old foliage texture constants are gated to tests as deprecated fixture completeness anchors.
- CLI parser tests and test fixture documentation that need a campaign path now use the stable `data/test_campaign` fixture path, keeping tests independent from the live tutorial campaign.

---

## Landscape Phase 6 Testing, Fixtures, and Quality Gates (2026)

**Goal:** Complete the landscape testing and fixture validation slice so domain data, campaign loading, map RON persistence, runtime spawning, importer export, and SDK map-editor state behavior have explicit regression coverage backed by `data/test_campaign` fixtures.

### Files Changed

| Area                 | Action                                                                                                                                          |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Domain tests         | Added exhaustive landscape category RON round-trip/default tests, full `LandscapeDefinition` round-trip coverage, and missing conflict tests    |
| Map RON tests        | Added a `data/test_campaign` map fixture round-trip test proving fixture `landscape_placements` survive serialization                           |
| Content loader tests | Strengthened `ContentDatabase::load_campaign` fixture assertions for landscape definitions and mesh registries                                  |
| Runtime tests        | Added imported/fallback transform assertions, missing-mesh fallback coverage, terrain-derived default tree spawning, and imported child cleanup |
| Importer tests       | Added exhaustive landscape category helper coverage and OBJ texture-copy validation for Landscape exports                                       |
| SDK map editor tests | Added remove redo coverage and out-of-range no-op tests for landscape placement remove/replace actions                                          |
| Fixture compliance   | Kept all test I/O on `data/test_campaign` or temp directories; no tests reference the live tutorial campaign                                    |

### What Changed

- `LandscapeCategory` now has explicit RON round-trip coverage for every variant and direct default checks for `LandscapeCategory::Tree` and non-blocking `LandscapeFlags`.
- `LandscapeDefinition` now has full serialize/deserialize equality coverage across ID, name, category, scale, tint, flags, icon, tags, mesh ID, and description.
- `LandscapeDatabase::validate_map_placements` now has branch coverage for blocking conflicts with wall/blocked tiles, NPC placements, dropped items, and map events, plus existing non-blocking overlap coverage.
- `data/test_campaign/data/maps/map_1.ron` is read directly in a map RON round-trip test to guard authored fixture placement stability.
- Runtime tests now prove authored fallback placements apply rotation/scale, definitions with missing mesh registry entries fall back to marker rendering, imported fixture placements apply transforms, terrain-derived Oak forest visuals spawn imported default landscape meshes, and map reload cleanup despawns imported landscape roots plus child mesh entities.
- Campaign Builder importer tests now prove Landscape OBJ texture copies use `assets/textures/landscape/<asset_stem>/` instead of the generic imported texture root and preserve copied bytes.
- Campaign Builder map editor tests now prove landscape placement remove redo removes again and out-of-range remove/replace calls do not mutate state or enqueue undo history.

---

## Landscape Phase 5 SDK Landscape Editor and Map Placement (2026)

**Goal:** Complete SDK authoring support for landscape definitions and map placements so campaign authors can browse grouped landscape assets, inspect validation metadata, place decorations on maps, edit placement overrides, and save/reopen stable placement RON.

### Files Changed

| Area                    | Action                                                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Landscape editor        | Grouped landscape definitions by category, added texture validation preview status, and kept the multi-column `allocate_ui` layout compliant     |
| Map placement tool      | Expanded `PlaceLandscape` support with selected-placement editing, move-by-position edits, offset/y-offset/rotation/scale/tint/blocking controls |
| Map overlay             | Improved landscape grid markers to show same-tile placement counts without replacing terrain/event/NPC colors                                    |
| Authoring conveniences  | Added edit, rotate, duplicate, delete, and deterministic random-variation actions for landscape placements                                       |
| Undo/save/load coverage | Reused add/remove/replace undo history and added tests proving transform overrides survive save, load, metadata application, and map round trips |
| SDK egui audit          | Added stable `push_id` scopes for new and touched widget loops, wrapped growing toolbar rows, and requested repaint on layout-driving changes    |
| Documentation           | Recorded this Phase 5 SDK implementation slice                                                                                                   |

### What Changed

- The Landscape tab now presents definitions in category sections while still supporting search and category filtering.
- Landscape previews show icon, name, category, default scale, tags, mesh availability, blocking defaults, and texture validation status from the active campaign mesh registry.
- The Map editor can place landscape definitions with `PlaceLandscape`, then edit existing placements through an inspector buffer that controls definition, tile position, sub-tile offset, vertical offset, rotation, scale, tint, and blocking override.
- Landscape placement edits use existing `LandscapePlacementReplaced` undo/redo semantics, so movement and transform changes undo and redo consistently.
- Same-tile landscape overlay markers now display a compact count badge, preserving support for multiple decorations on one tile.
- Save/load regression tests prove `landscape_placements` and all optional transform/override fields survive map RON serialization, `MapsEditorState::load_maps`, `save_map`, and metadata synchronization.

---

## Landscape Phase 4 Importer Landscape Export Support (2026)

**Goal:** Complete Campaign Builder importer support for first-class Landscape exports so imported OBJ/GLB models create reusable landscape mesh registry entries and landscape definitions that reload immediately in the SDK.

### Files Changed

| Area               | Action                                                                                                                                                                   |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Importer state     | Used `LANDSCAPE_MESH_ID_MIN` for default landscape mesh IDs and preserved landscape mesh IDs across importer clears                                                      |
| Importer UI        | Exposed configured landscape export plumbing, landscape-aware copy text, category combo ID scoping, and target-specific export options                                   |
| Export persistence | Wrote landscape mesh assets under `assets/meshes/landscape/`, upserted `data/landscape_mesh_registry.ron`, and upserted the configured landscape definitions file        |
| SDK reload flow    | Suggested the next free `LandscapeMeshId`, refreshed it when opening/exporting landscapes, reloaded landscape definitions, and returned to the Landscape tab             |
| Tests              | Added coverage for ID allocation, registry upsert, definition upsert, configured landscape file export, exported mesh RON IDs, landscape texture paths, and UI rendering |
| Documentation      | Recorded this Phase 4 importer implementation slice                                                                                                                      |

### What Changed

- `ExportType::Landscape` now participates in the full importer export path using `LandscapeMeshId` allocation from `LANDSCAPE_MESH_ID_MIN` and the shared `CreatureDefinition` / `MeshDefinition` RON asset format required by architecture.
- Landscape exports create or update `data/landscape_mesh_registry.ron` and create or update a reusable `LandscapeDefinition` referencing the exported mesh ID.
- The exporter honors the active campaign metadata `landscape_file` instead of hardcoding `data/landscape.ron`, so the Campaign Builder reloads the same definitions file that export updates.
- OBJ/MTL and GLB texture copies for Landscape exports remain portable and campaign-relative under `assets/textures/landscape/<export_stem>/`.
- The importer emits `ObjImporterUiSignal::Landscape`; the app reloads landscape definitions, advances the suggested landscape mesh ID, switches back to the Landscape tab, and requests repaint so the new asset appears without restarting.
- The importer category dropdown loops now wrap row widgets in `push_id`, and the top-level tab loop also uses stable `push_id` scopes to satisfy the SDK egui ID audit.

---

## Landscape Phase 3 Runtime Rendering and Map Spawn Integration (2026)

**Goal:** Complete runtime map-spawn integration for authored landscape placements so imported landscape meshes render from campaign registries, fallback markers remain available, and map reloads cleanly remove old landscape entities.

### Files Changed

| Area              | Action                                                                                                                                                                              |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runtime rendering | Added landscape render hints, propagated imported mesh LOD data, and kept texture-aware landscape material creation in map spawning                                                 |
| Map spawn order   | Confirmed authored landscape placements spawn after terrain/procedural vegetation and before event/NPC visuals                                                                      |
| Map lifecycle     | Chained map-change handling before marker respawn so map reload cleanup/spawn order is deterministic                                                                                |
| Tests             | Added runtime coverage for fallback placements, imported fixture meshes, multiple placements on one tile, transforms, invalid placement skips, render hints, and map reload cleanup |
| Documentation     | Recorded this Phase 3 runtime implementation slice                                                                                                                                  |

### What Changed

- `LandscapeEntity` roots now carry `LandscapeRenderHints`, giving future dense-landscape culling systems a stable query hook without changing persisted `Map` or landscape RON data.
- Imported landscape mesh spawning reuses the existing `MeshDefinition` conversion path and now preserves mesh LOD levels by inserting `LodState` on child mesh entities when LOD data is present.
- Authored placements still apply tile center positioning, sub-tile offsets, vertical offsets, Y-axis rotation in degrees, placement/default scale, mesh scale, and tint-aware materials.
- Fallback markers remain available for definitions without a mesh or with a missing mesh registry entry; blocking metadata remains domain validation/movement metadata rather than making every render entity collidable.
- Runtime tests verify that `data/test_campaign` renders the two authored fixture placements on tile `(6, 6)` from imported mesh IDs `11001` and `11005`, while terrain-only/procedural fallback behavior remains unaffected.
- Map transition tests verify that stale landscape entities from the previous map are despawned through the existing `MapEntity` lifecycle.

---

## Landscape Phase 2 Domain Loading, Validation, and Serialization (2026)

**Goal:** Complete landscape domain loading, map serialization, and validation so campaign content can safely load optional landscape data, round-trip placements, and reject invalid or movement-conflicting landscape placements.

### Files Changed

| Area                    | Action                                                                                                                  |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Domain validation       | Added map-aware landscape placement validation, effective blocking checks, and blocking movement-conflict diagnostics   |
| Content loading         | Integrated landscape mesh, texture, definition, placement, and teleport-destination validation into SDK content loading |
| Map serialization/tests | Added explicit tests for omitted empty placement lists and minimal placement deserialization defaults                   |
| Fixtures                | Added fixture landscape placements to `data/test_campaign/data/maps/map_1.ron`                                          |
| SDK validation          | Reported landscape placement and blocked teleport/starting-position issues through validator checks                     |
| Documentation           | Recorded this Phase 2 implementation slice                                                                              |

### What Changed

- `LandscapeDatabase::validate_map_placements` validates definition references, map bounds, and blocking placement conflicts with wall/blocked tiles, map events, NPC placements, and dropped items.
- `LandscapeDatabase::is_position_blocked_by_landscape` resolves definition defaults plus placement overrides for validation and SDK checks.
- `ContentDatabase` now validates landscape meshes, `assets/` texture paths, definition-to-mesh references, map placements, and teleport destinations blocked by landscapes during `load_campaign`, `load_core`, and `validate`.
- Map RON remains migration-safe: missing `landscape_placements` defaults to an empty vector, empty vectors skip serialization, and minimal placement entries deserialize optional transform/override fields as `None`.
- `data/test_campaign` now includes map-level landscape placements that exercise fixture loading without referencing the live tutorial campaign.

---

## Landscape Phase 1 Tree and Brush Asset Baseline (2026)

**Goal:** Fix the default tree/brush asset baseline so tutorial and test campaign landscape assets use valid campaign-relative texture paths, real mesh payloads, reliable diagnostics, and imported defaults where practical while preserving procedural vegetation fallback.

### Files Changed

| Area                | Action                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Mesh/data assets    | Repaired foliage mesh alpha modes, copied full tree/brush mesh RON files into `data/test_campaign`, and completed tree textures |
| Runtime rendering   | Preferred imported default landscape definitions for Oak, Pine, Dead, Palm, and Shrub terrain vegetation when meshes exist      |
| Procedural fallback | Documented the explicit texture-free foliage fallback decision and retained bark texture loading                                |
| Validation/tests    | Added texture-path diagnostic coverage and fixture-integrity tests for the Phase 1 landscape mesh registry                      |
| Documentation       | Recorded the Phase 1 asset-baseline update                                                                                      |

### What Changed

- The five Phase 1 mesh RON assets now use campaign-relative `assets/textures/trees/...` texture paths with foliage materials using alpha masking.
- `data/test_campaign` now contains full matching copies of the five tree/brush mesh RON assets plus the complete tree texture set, including Birch and Willow foliage textures used by procedural diagnostics.
- Terrain-tied `TreeType::Oak`, `Pine`, `Dead`, `Palm`, and `Shrub` visuals try matching imported landscape definitions first and fall back to procedural trees/brush if definitions or meshes are missing. Birch and Willow intentionally remain procedural because Phase 1 has no imported defaults for them.
- Runtime landscape material creation warns when a mesh texture path does not start with `assets/`, and campaign validation reports the landscape mesh name, ID, mesh part, texture path, and campaign root for texture failures.
- Procedural fallback tree docs now match behavior: bark uses `bark.png`; foliage uses generated geometry and opaque species colours instead of the old round alpha-mask textures.

---

## Landscape Phase 0 Architecture Alignment (2026)

**Goal:** Complete the landscape Phase 0 architecture and scope alignment so later work has approved names, module placement, identifier allocation rules, database responsibilities, and a clear mesh-reuse decision.

### Files Changed

| Path                                  | Action                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `docs/reference/architecture.md`      | Documented landscape identifiers, database types, mesh registry design, placement semantics, and scope split |
| `src/domain/types.rs`                 | Added explicit landscape ID allocation constants and clarified alias ranges                                  |
| `docs/explanation/implementations.md` | Recorded this Phase 0 alignment update                                                                       |

### What Changed

- `LandscapeId` is documented as a campaign-local `u32` allocated from `1..=u32::MAX`, with `0` reserved.
- `LandscapeMeshId` is documented as an imported mesh registry `u32` allocated by SDK/importer workflows from `11000..=u32::MAX`.
- Architecture Section 4.2 now documents `LandscapeDatabase`, `LandscapeMeshDatabase`, `Map.landscape_placements`, placement transform semantics, and migration-safe empty placement behavior.
- The mesh decision is explicit: landscape meshes reuse the existing `CreatureDefinition` / `MeshDefinition` RON format through a `LandscapeMeshDatabase` wrapper rather than introducing a separate `LandscapeMeshDefinition` in this phase.
- The furniture-vs-landscape boundary is documented so interactable gameplay objects remain furniture while static ambient scenery is landscape.

---

## Landscape Category, Importer, Runtime, and Map Placement (2026)

**Goal:** Implement the landscape plan end-to-end: first-class landscape domain data, campaign loading, imported tree/brush asset repair, runtime map spawning, importer export support, SDK landscape browsing, and map-editor placement.

### Files Changed

| Area                | Action                                                                                                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture/domain | Added `LandscapeId`, `LandscapeMeshId`, `LandscapeCategory`, `LandscapeDefinition`, `LandscapePlacement`, landscape databases, and `Map.landscape_placements`  |
| Runtime             | Added `LandscapeEntity` and map-spawn support for imported landscape mesh placements with texture-aware materials and fallback markers                         |
| Data/fixtures       | Added tutorial and `data/test_campaign` landscape definitions, mesh registries, fixture mesh assets, and copied tree textures into the test fixture            |
| Importer/SDK        | Added `ExportType::Landscape`, landscape registry/definition upserts, landscape texture export paths, a Landscape tab, and map-editor `PlaceLandscape` support |
| Tests/docs          | Added domain, loader, importer, and map-editor tests and updated implementation documentation                                                                  |

### What Changed

- Landscape definitions and imported landscape mesh registries now load from `data/landscape.ron` and `data/landscape_mesh_registry.ron`.
- Maps serialize a migration-safe `landscape_placements` vector that allows multiple static props per tile.
- Runtime map spawning renders placed imported landscape meshes using the same mesh format as creature/furniture assets, including `assets/` texture paths.
- The OBJ/GLB importer can export Landscape assets, upsert both landscape RON files, and copy textures under `assets/textures/landscape/`.
- The Campaign Builder has a Landscape tab and the Map editor can select a landscape definition, place it on tiles, and rotate/duplicate/delete placements with undo/redo.
- Default tree and brush RON texture paths were repaired and tutorial/test campaign landscape data was seeded.

---

## Landscape Category and Map Placement Implementation Plan (2026)

**Goal:** Define a phased plan for adding a first-class `landscape` importer/SDK category, map-editor landscape placement support, and default tree/brush visual fixes using the existing tree mesh and texture assets.

### Files Changed

| Path                                                | Action                                                           |
| --------------------------------------------------- | ---------------------------------------------------------------- |
| `docs/explanation/landscape_implementation_plan.md` | Added phased landscape implementation and tree/brush repair plan |
| `docs/explanation/next_plans.md`                    | Updated the landscape plan link to point at the Markdown file    |
| `docs/explanation/implementations.md`               | Recorded this planning/documentation update                      |

### What Was Added

- Proposed a domain model for `LandscapeDefinition`, `LandscapePlacement`, `LandscapeCategory`, `LandscapeDatabase`, and `LandscapeMeshDatabase`.
- Planned the architecture update needed before adding `Map.landscape_placements`.
- Defined phases for tree/brush asset repair, domain loading and validation, runtime rendering, importer export support, SDK landscape editing, map placement, fixtures, tests, and documentation cleanup.
- Captured SDK egui ID and multi-column layout requirements for the planned importer and map-editor work.
- Reiterated that tests and fixtures must use `data/test_campaign` rather than `campaigns/tutorial`.

---

## Campaign Builder Monster Editor — Reliable Edit Save (2026)

**Goal:** Ensure the bottom **Save** button in the Monster Editor always commits
the current edit buffer to the intended monster before returning to the list.

### Files Changed

| Path                                          | Action                                                       |
| --------------------------------------------- | ------------------------------------------------------------ |
| `sdk/campaign_builder/src/monsters_editor.rs` | Hardened edit-buffer commit logic and added regression tests |
| `docs/explanation/implementations.md`         | Recorded this Monster Editor bug fix                         |

### What Changed

- Added a dedicated `commit_edit_buffer_to_monsters` helper.
- Edit saves now prefer the selected row only when it still points at the same
  monster ID.
- If selection is missing or stale due to filtering, sorting, or context-menu
  interactions, saves fall back to locating the monster by `edit_buffer.id`.
- Add-mode saves select the newly inserted monster after committing.
- The bottom Save button now requests repaint after returning to list mode and
  only overwrites the save status when disk save succeeds.

### Tests Added

- `test_commit_edit_buffer_updates_by_id_when_selection_is_missing`
- `test_commit_edit_buffer_updates_by_id_when_selection_points_elsewhere`
- `test_commit_edit_buffer_add_selects_new_monster`

---

## Campaign Builder Importer — Item/Furniture Category and Furniture RON Fix (2026)

**Goal:** Make the Importer tab consistent across creature, item, and furniture
exports, and ensure furniture imports update campaign furniture definitions.

### Files Changed

| Path                                          | Action                                                              |
| --------------------------------------------- | ------------------------------------------------------------------- |
| `sdk/campaign_builder/src/obj_importer_ui.rs` | Added item/furniture category dropdowns and export metadata upserts |
| `sdk/campaign_builder/src/lib.rs`             | Reloaded item mesh/furniture state after importer export signals    |
| `docs/explanation/implementations.md`         | Recorded this importer bug fix                                      |

### What Changed

- Replaced free-text category entry for item and furniture importer exports
  with `ComboBox::from_id_salt` dropdowns.
- Item exports now upsert `data/item_mesh_registry.ron` so newly imported item
  mesh assets are registered for runtime/editor loading.
- Furniture exports now upsert both `data/furniture_mesh_registry.ron` and
  `data/furniture.ron`, creating or updating a `FurnitureDefinition` that
  references the imported mesh ID.
- After item exports, the Item Mesh editor registry is reloaded from the open
  campaign so the new asset appears immediately.
- After furniture exports, furniture definitions are reloaded and the UI returns
  to the Furniture tab with the imported definition available.

### Tests Added

- `test_export_item_updates_item_mesh_registry`
- `test_export_furniture_updates_registry_and_furniture_file`
- `test_export_furniture_upserts_existing_definition_by_mesh_id`
- `test_item_and_furniture_category_helpers_map_dropdown_values`

---

## Sky System — Phase 6 Completion and Compliance Hardening (2026)

**Goal:** Finish the remaining Phase 6 work from the sky system plan: harden SDK
map saving, complete sun/star transition rendering, prevent indoor sky-body
spawns, add reusable procedural cloud noise texturing, and document the final
implementation state.

### Files Changed

| Path                                     | Action                                                                                      |
| ---------------------------------------- | ------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/map_editor.rs` | Hardened metadata save synchronisation and stale indoor sky cleanup                         |
| `src/game/systems/sky_bodies.rs`         | Completed flat sun discs, indoor no-spawn, Dawn/Dusk opacity, and cloud noise texture reuse |
| `docs/explanation/implementations.md`    | Recorded this Phase 6 completion work                                                       |

### What Changed

- Added SDK metadata synchronisation helpers so `save_to_ron` serializes a map
  with current metadata even when callers do not manually call
  `apply_metadata` first.
- Clearing **Outdoor Map** now clears stale `sky_config`, and indoor maps always
  serialize with `sky: None`.
- Replaced sun sphere meshes with flat triangle-fan disc meshes.
- Added `SkyBodyRenderState` and `sky_body_render_state` so Dawn and Dusk keep
  both suns and stars visible while reducing material alpha.
- Prevented indoor maps from spawning sun or star entities at all.
- Added `CloudNoiseTexture`, deterministic cloud `Image` generation, and
  cloud materials that reuse the generated texture while preserving
  `cloud_density * cloud_coverage` alpha.
- Added/strengthened tests for metadata synchronization, stale indoor sky
  cleanup, flat sun meshes, indoor no-spawn behaviour, Dawn/Dusk opacity, and
  cloud texture reuse/material tinting.

---

## Sky System — Phase 6 Domain and Architecture Hardening (2026)

**Goal:** Align the architecture reference with the implemented domain sky model
and strengthen the domain RON round-trip test so every `SkyConfig` field is
covered explicitly.

### Files Changed

| Path                                  | Action                                                                                                                                         |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/reference/architecture.md`      | Updated Section 4.2 to document the real `World`, `Map`, and `SkyConfig` field shapes, including sky defaults and `is_outdoor`/`sky` semantics |
| `src/domain/world/types.rs`           | Renamed and strengthened the map sky RON round-trip test to assert every `SkyConfig` field survives serialization and deserialization          |
| `docs/explanation/implementations.md` | Recorded this Phase 6 domain/architecture update                                                                                               |

### What Changed

- Documented `SkyConfig` in `docs/reference/architecture.md` Section 4.2 with
  all twelve fields and their default values.
- Updated the documented `Map` fields to include `is_outdoor: bool` and
  `sky: Option<SkyConfig>` alongside the other implemented map fields.
- Clarified that `sky` is only consulted when `is_outdoor` is `true`; indoor
  maps ignore sky configuration.
- Replaced the narrower sky round-trip test with
  `test_domain_map_sky_roundtrip_preserves_all_fields`, which uses non-default
  values for every `SkyConfig` field and verifies each one after RON round-trip.

---

## Sky System Implementation Plan — Phase 6 Hardening Added (2026)

**Goal:** Extend `docs/explanation/sky_system_implementation_plan.md` with a
final completion phase that captures the remaining implementation gaps found
after auditing Phases 1–5.

### Files Changed

| Path                                                 | Action                                             |
| ---------------------------------------------------- | -------------------------------------------------- |
| `docs/explanation/sky_system_implementation_plan.md` | Added Phase 6: Completion and Compliance Hardening |
| `docs/explanation/implementations.md`                | Recorded this planning/documentation update        |

### What Was Added

The new Phase 6 defines the remaining work needed to finish the sky system:

- synchronise `docs/reference/architecture.md` with the implemented `SkyConfig`
  and `Map` fields,
- harden SDK map saving so indoor maps cannot retain or serialize stale sky
  configuration,
- strengthen domain and SDK round-trip tests to assert every `SkyConfig` field,
- prevent indoor maps from spawning sun/star entities,
- replace sphere suns with flat disc/billboard rendering,
- add Dawn/Dusk opacity transitions for suns and stars,
- complete the procedural cloud noise texture requirement,
- add explicit tests and success criteria for all remaining gaps.

### Validation

This was a documentation-only planning update. No Rust code or game data was
changed.

---

## Sky System — Phase 4: Celestial Bodies — Suns and Stars (2026)

**Goal:** Spawn sun disc entities and a single star-field mesh entity per
outdoor map, driven by each map's `SkyConfig`. Toggle visibility frame-by-frame
based on the current `TimeOfDay` so suns show during the day, stars at night,
and both during transitional periods.

### Files Changed

| Path                             | Action                           |
| -------------------------------- | -------------------------------- |
| `src/game/systems/sky_bodies.rs` | **New** — Phase 4 implementation |
| `src/game/systems/mod.rs`        | Added `pub mod sky_bodies;`      |

### What Was Built

#### `src/game/systems/sky_bodies.rs` (new file)

**Constants**

| Constant                      | Value                | Purpose                                            |
| ----------------------------- | -------------------- | -------------------------------------------------- |
| `SUN_DISTANCE`                | `500.0`              | World-units distance at which sun discs are placed |
| `SUN_ELEVATION_ANGLE_RADIANS` | `π/4` (45°)          | Elevation above horizon for all suns               |
| `SUN_BASE_RADIUS`             | `SUN_DISTANCE * 0.1` | Disc radius at `sun_size = 1.0`                    |
| `STAR_FIELD_RADIUS`           | `480.0`              | Hemisphere radius for star positions               |
| `STAR_POINT_SIZE`             | `0.8`                | Half-size of each star triangle (world units)      |

**`SkyBodyState` resource**

Tracks `sun_entities: Vec<Entity>` and `star_entity: Option<Entity>` so every
entity spawned by the system can be found and despawned when the map changes.

**`SkyBodyPlugin`**

Registers `SkyBodyState` and two `Update` systems:

- `manage_sky_bodies_on_map_change` — detects map change via `Local<Option<MapId>>`,
  despawns old entities, spawns new ones.
- `update_sky_body_visibility` — runs `.after(manage_sky_bodies_on_map_change)`,
  sets `Visibility::Visible` / `Visibility::Hidden` on all `SunMarker` and
  `StarFieldMarker` entities.

**Pure helper functions**

| Function                                     | Description                                                                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `sun_azimuths(n)`                            | Returns azimuth angles in radians for `n` suns: 0→empty, 1→[-30°], 2→[±60°], n>2→evenly distributed over 120° arc   |
| `sun_world_positions(n, size)`               | Converts azimuths to 3D world-space `(Vec3, radius)` pairs at `SUN_DISTANCE` / `SUN_ELEVATION_ANGLE_RADIANS`        |
| `generate_star_positions(count, seed)`       | Scatters `count` points on the upper hemisphere using a seeded `StdRng`; seed derived from `map.id` for determinism |
| `sky_body_visibility_flags(is_outdoor, tod)` | Returns `(suns_visible, stars_visible)` — pure, no Bevy dependency                                                  |
| `build_star_mesh(positions, density)`        | Builds a `Mesh` of tiny equilateral triangles, one per star; alpha from `density.clamp(0.2, 1.0)`                   |

**Visibility rules implemented**

| `is_outdoor` | `TimeOfDay`         | Suns    | Stars   |
| ------------ | ------------------- | ------- | ------- |
| `false`      | (any)               | Hidden  | Hidden  |
| `true`       | Morning / Afternoon | Visible | Hidden  |
| `true`       | Evening / Night     | Hidden  | Visible |
| `true`       | Dawn / Dusk         | Visible | Visible |

**Spawn / despawn helpers**

`spawn_sky_bodies` and `despawn_sky_bodies` are free functions (not systems)
so they can be called from tests or other contexts without a Bevy world.

### Tests Added (6 unit tests, all in `sky_bodies::tests`)

| Test                                            | What it verifies                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `test_sun_positions_one_sun`                    | `sun_azimuths(1)` returns exactly one element at -30°                                       |
| `test_sun_positions_two_suns`                   | `sun_azimuths(2)` returns symmetric ±60° pair                                               |
| `test_sun_count_zero_spawns_nothing`            | Both `sun_azimuths(0)` and `sun_world_positions(0, …)` return empty vecs                    |
| `test_star_count_zero_spawns_empty_field`       | `generate_star_positions(0, …)` is empty; `build_star_mesh` over empty input has 0 vertices |
| `test_sky_body_visibility_night_shows_stars`    | Night/Evening outdoor → stars only; indoor night → both hidden                              |
| `test_sky_body_visibility_afternoon_shows_suns` | Morning/Afternoon → suns only; Dawn/Dusk → both visible                                     |

---

## Sky System — Phase 5: Cloud Layer (2026)

**Goal:** Add a cloud layer mesh entity to the sky system. Cloud quads are
distributed across a flat horizontal plane, drift east-to-west each frame via
`animate_clouds`, and wrap seamlessly when they exceed the plane boundary.
Cloud spawning is gated by `MIN_CLOUD_COVERAGE` and driven entirely by each
map's `SkyConfig.cloud_coverage`, `.cloud_density`, `.cloud_color`, and
`.cloud_speed` fields.

### Files Changed

| Path                             | Action                                              |
| -------------------------------- | --------------------------------------------------- |
| `src/game/systems/sky_bodies.rs` | **Rewritten** — Phase 5 additions on top of Phase 4 |

### What Was Built

**New constants**

| Constant             | Value   | Purpose                                             |
| -------------------- | ------- | --------------------------------------------------- |
| `MAP_CLOUD_HEIGHT`   | `40.0`  | Y altitude of the cloud plane                       |
| `CLOUD_PLANE_WIDTH`  | `200.0` | Total X/Z extent of the cloud plane (world units)   |
| `CLOUD_QUAD_SIZE`    | `20.0`  | Side length of each individual cloud quad           |
| `MAX_CLOUD_QUADS`    | `50`    | Maximum quads at `cloud_coverage = 1.0`             |
| `MIN_CLOUD_COVERAGE` | `0.05`  | Coverage threshold below which no entity is spawned |

**`SkyBodyState` — new field**

Added `pub cloud_entity: Option<Entity>` so the cloud entity is tracked
alongside sun and star entities and safely despawned on map change.

**`SkyBodyPlugin` — new system**

Registered `animate_clouds` in the `Update` schedule alongside the existing
`manage_sky_bodies_on_map_change` and `update_sky_body_visibility` systems.

**New pure helper functions**

| Function                                           | Description                                                                                      |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `should_spawn_cloud_layer(coverage)`               | Returns `true` when `coverage >= MIN_CLOUD_COVERAGE`                                             |
| `cloud_alpha(density, coverage)`                   | Returns `(density * coverage).clamp(0.0, 1.0)` — cloud opacity                                   |
| `cloud_base_color(cloud_color, density, coverage)` | Preserves RGB from `cloud_color`; computes alpha via `cloud_alpha`                               |
| `wrap_cloud_position(x, half_width)`               | Wraps X translation: subtracts `2*half_width` when `x > half_width`, adds when `x < -half_width` |

**`build_cloud_mesh(coverage, seed)`**

Builds a flat `Mesh` of `coverage * MAX_CLOUD_QUADS` quads randomly placed
across a `CLOUD_PLANE_WIDTH × CLOUD_PLANE_WIDTH` plane using a seeded `StdRng`.
Returns an empty mesh when `should_spawn_cloud_layer` returns `false`.

**`spawn_cloud_layer` / `despawn_cloud_layer`**

Free helper functions (not systems) that create / destroy the cloud entity and
write / clear `state.cloud_entity`. `spawn_cloud_layer` no-ops when
`map.is_outdoor == false` or coverage is below threshold.

**`spawn_sky_bodies` update**

Now calls `spawn_cloud_layer` after spawning suns and stars.

**`despawn_sky_bodies` update**

Now calls `despawn_cloud_layer` to clean up the cloud entity.

**`animate_clouds` system**

Each frame, adds `cloud_speed * delta_secs` to the cloud entity's
`transform.translation.x` and wraps via `wrap_cloud_position`. Queries
`(&mut Transform, &CloudLayerMarker)` — no `Res<GlobalState>` needed.

### Tests Added (4 new unit tests; total 10 in `sky_bodies::tests`)

| Test                                   | What it verifies                                                                                          |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `test_cloud_coverage_zero_skips_spawn` | `should_spawn_cloud_layer` returns `false` below threshold, `true` at and above it                        |
| `test_cloud_density_affects_opacity`   | `cloud_alpha` returns `0.0` for zero density/coverage, `0.4` for 0.5×0.8, `1.0` for max, clamps above 1.0 |
| `test_animate_clouds_wraps_position`   | `wrap_cloud_position` leaves in-range values unchanged, wraps correctly in both directions                |
| `test_cloud_color_applied_to_material` | `cloud_base_color` preserves RGB and computes alpha; verified against `SkyConfig::default()` values       |

---

## Sky System — Phase 3: SDK Map Editor Integration (2026)

**Goal:** Expose `is_outdoor` and `SkyConfig` in the SDK Map Editor's
Metadata panel so campaign authors can configure sky settings visually and
have them persisted to map RON files.

### What Was Built

All changes are in `sdk/campaign_builder/src/map_editor.rs`.

#### `MapMetadata` struct

Added `pub sky_config: Option<SkyConfig>` field (default `None`). The
field gates the entire sky-settings UI section.

#### `MapEditorState::new` (load path)

Now copies both `map.is_outdoor` and `map.sky` into `metadata.is_outdoor`
and `metadata.sky_config` on construction, so existing maps open with their
correct sky state in the editor.

#### `apply_metadata` (save path)

Now writes `map.is_outdoor = metadata.is_outdoor` and
`map.sky = metadata.sky_config.clone()` back to the `Map` struct before
serialisation. Previously `is_outdoor` was never persisted to the RON file;
that gap is now closed.

#### `show_metadata_editor` UI

Immediately after the "Outdoor Map" checkbox, a `CollapsingHeader`
`"\u2601 Sky Settings"` (hidden when `is_outdoor = false`) exposes:

- **Enable Custom Sky** checkbox — toggles `sky_config` between `None` and
  `Some(SkyConfig::default())`.
- When enabled, colour pickers (`color_edit_button_rgba_unmultiplied`) for
  Day Sky, Dusk/Dawn Sky, Night Sky, Sun Color, and Cloud Color.
- `DragValue` widgets for Sun Count (0–8) and Star Count (0–10 000).
- `Slider` widgets for Sun Size (0.1–5.0), Star Density, Cloud Coverage,
  Cloud Density, and Cloud Speed (all 0.0–1.0 or 0.0–5.0).
- Every widget sets `editor.has_changes = true` on change.

#### `classify_map_environment` update

The `is_outdoor` flag is now checked first (authoritative). If `true`,
returns `("Outdoor", green)` immediately. The existing wall-density
heuristic only runs for maps where `is_outdoor == false`; its old
fallthrough path `("Outdoor", …)` is replaced with `("Indoor", …)` so
the heuristic never produces an "Outdoor" label for a map the author
intended as indoor.

#### `SkyConfig` import

Added to the `antares::domain::world::{\u2026}` import block.

### Tests Added (5 new, all in `map_editor::tests`)

| Test                                                 | What it verifies                                                        |
| ---------------------------------------------------- | ----------------------------------------------------------------------- |
| `test_map_metadata_is_outdoor_writes_to_map_ron`     | `apply_metadata` propagates `is_outdoor` and it survives RON round-trip |
| `test_map_metadata_sky_config_round_trip`            | `SkyConfig` fields survive `apply_metadata` + RON round-trip            |
| `test_map_metadata_sky_config_none_not_written`      | `sky:` key absent from RON when `sky_config` is `None`                  |
| `test_classify_map_environment_uses_is_outdoor_flag` | `is_outdoor=true` overrides wall-density heuristic → `"Outdoor"`        |
| `test_metadata_sky_section_only_shown_when_outdoor`  | `MapEditorState::new` mirrors `is_outdoor=false` / `sky=None` from map  |

### Quality Gates

```text
cargo fmt     → clean
cargo check   → 0 errors
cargo clippy  → 0 warnings
cargo nextest (antares)          → 5101 passed, 8 skipped, 0 failed
cargo nextest (campaign_builder) → 2493 passed, 0 skipped, 0 failed
````

---

## Sky System — Phase 2: Sky Background Rendering Engine (2026)

**Goal:** Replace the static grey Bevy `ClearColor` with a live, per-map sky
tint driven by `SkyConfig` and `TimeOfDay`. Indoor maps receive a near-black
cave ceiling colour; outdoor maps blend through dawn, day, dusk, and night
palettes.

### What Was Built

#### `src/game/systems/sky.rs` (new file)

**Constants (4)**

| Constant                              | Value                     | Purpose                           |
| ------------------------------------- | ------------------------- | --------------------------------- |
| `INDOOR_SKY_COLOR`                    | `[0.05, 0.04, 0.03, 1.0]` | Near-black warm grey for dungeons |
| `DEFAULT_OUTDOOR_DAY_SKY_COLOR`       | `[0.53, 0.81, 0.98, 1.0]` | Sky blue for Morning/Afternoon    |
| `DEFAULT_OUTDOOR_NIGHT_SKY_COLOR`     | `[0.02, 0.02, 0.08, 1.0]` | Deep navy for Evening/Night       |
| `DEFAULT_OUTDOOR_DUSK_DAWN_SKY_COLOR` | `[0.98, 0.60, 0.20, 1.0]` | Warm amber for Dawn/Dusk          |

**`sky_color_for_time(config: &SkyConfig, tod: TimeOfDay) -> [f32; 4]`**

Pure function (no Bevy dependencies) that maps each `TimeOfDay` variant to an
exact colour or a blended colour:

| `TimeOfDay` | Result                                              |
| ----------- | --------------------------------------------------- |
| `Night`     | `night_sky_color`                                   |
| `Evening`   | lerp(`night_sky_color`, `dusk_dawn_sky_color`, 0.3) |
| `Dawn`      | `dusk_dawn_sky_color`                               |
| `Morning`   | lerp(`dusk_dawn_sky_color`, `day_sky_color`, 0.7)   |
| `Afternoon` | `day_sky_color`                                     |
| `Dusk`      | `dusk_dawn_sky_color`                               |

**`update_sky_background` Bevy system**

Reads `GlobalState` (current map + time), computes the RGBA via
`sky_color_for_time`, and writes `Color::srgba(…)` to `ResMut<ClearColor>`.
Selection logic:

1. No active map → `INDOOR_SKY_COLOR`
2. `is_outdoor == false` → `INDOOR_SKY_COLOR`
3. `is_outdoor == true`, `sky == None` → `SkyConfig::default()`
4. `is_outdoor == true`, `sky == Some(cfg)` → per-map `cfg`

**`SkyPlugin`**

Registers `update_sky_background` in `Update`, ordered
`.after(apply_time_advance).before(update_ambient_light)`, guaranteeing the
sky colour is always computed from the up-to-date time value.

#### `src/game/systems/mod.rs`

`pub mod sky;` added (alphabetically between `rest` and `skill_training_ui`).

#### `src/bin/antares.rs`

`app.add_plugins(antares::game::systems::sky::SkyPlugin)` registered
immediately after `TimeOfDayPlugin`.

### Tests Added (6 new unit tests in `game::systems::sky::tests`)

| Test                                            | What it verifies                                |
| ----------------------------------------------- | ----------------------------------------------- |
| `test_sky_color_for_time_night`                 | Night returns `night_sky_color` exactly         |
| `test_sky_color_for_time_afternoon`             | Afternoon returns `day_sky_color` exactly       |
| `test_sky_color_for_time_dusk`                  | Dusk returns `dusk_dawn_sky_color` exactly      |
| `test_sky_color_for_time_evening_is_blend`      | Evening is strictly between Night and Dusk/Dawn |
| `test_sky_color_for_time_morning_is_blend`      | Morning is strictly between Dusk/Dawn and Day   |
| `test_sky_color_all_periods_produce_valid_rgba` | All 6 variants stay in `[0.0, 1.0]` per channel |

### Quality Gates

```text
cargo fmt     → clean
cargo check   → 0 errors
cargo clippy  → 0 warnings
cargo nextest → 5101 passed, 8 skipped, 0 failed
```

---

## Sky System — Phase 1: Domain Foundation (2026)

**Goal:** Add `is_outdoor` and `SkyConfig` to the `Map` domain struct and RON
format with full backward compatibility. No rendering changes — pure data
foundation for Phases 2–5.

### What Was Built

#### `SkyConfig` struct (`src/domain/world/types.rs`)

New public struct inserted before `pub struct Map`, carrying twelve fields that
describe per-map sky rendering:

| Field                 | Type       | Default                   | Purpose                            |
| --------------------- | ---------- | ------------------------- | ---------------------------------- |
| `day_sky_color`       | `[f32; 4]` | `[0.53, 0.81, 0.98, 1.0]` | RGBA sky during Morning/Afternoon  |
| `dusk_dawn_sky_color` | `[f32; 4]` | `[0.98, 0.60, 0.20, 1.0]` | RGBA sky during Dawn/Dusk          |
| `night_sky_color`     | `[f32; 4]` | `[0.02, 0.02, 0.08, 1.0]` | RGBA sky during Evening/Night      |
| `sun_count`           | `u8`       | `1`                       | Number of sun discs (0 = overcast) |
| `sun_color`           | `[f32; 4]` | `[1.0, 0.95, 0.80, 1.0]`  | Sun disc RGBA                      |
| `sun_size`            | `f32`      | `1.0`                     | Sun disc scale multiplier          |
| `star_count`          | `u32`      | `2000`                    | Total stars in night sky           |
| `star_density`        | `f32`      | `0.5`                     | 0–1 density distribution           |
| `cloud_coverage`      | `f32`      | `0.3`                     | 0–1 sky fraction covered           |
| `cloud_color`         | `[f32; 4]` | `[0.9, 0.9, 0.9, 0.8]`    | Cloud layer RGBA                   |
| `cloud_density`       | `f32`      | `0.5`                     | 0–1 cloud opacity/thickness        |
| `cloud_speed`         | `f32`      | `1.0`                     | Cloud animation speed multiplier   |

All fields carry `#[serde(default = "…")]` so existing RON files that omit the
block continue to deserialize without error.

#### `Map` struct additions (`src/domain/world/types.rs`)

- `pub is_outdoor: bool` — `#[serde(default)]` → `false`; enables sky
  rendering for outdoor maps.
- `pub sky: Option<SkyConfig>` — `#[serde(default,
skip_serializing_if = "Option::is_none")]`; per-map sky config, `None`
  for indoor maps. `Map::new()` initialises both fields to their defaults.

#### Export (`src/domain/world/mod.rs`)

`SkyConfig` added to the `pub use types::{ … }` block, making it available
as `antares::domain::world::SkyConfig`.

#### Struct-literal fixes

Four files that construct `Map` using struct-literal syntax were updated to
include the two new fields:

- `src/domain/world/blueprint.rs` — `is_outdoor: false, sky: None`
- `src/sdk/templates.rs` — `town_map` / `forest_map` get `is_outdoor: true`;
  `dungeon_map` gets `is_outdoor: false`
- `src/sdk/cli/map_validator.rs` (two helpers) — `is_outdoor: false, sky: None`

#### Test campaign fixture (`data/test_campaign/data/maps/map_1.ron`)

Added `is_outdoor: true` and a full `sky: Some(( … ))` block to the Town
Square map so integration tests exercise the new sky data path. All other
fixture maps (`map_2.ron` – `map_7.ron`) remain untouched and continue to
load correctly via serde defaults.

### Tests Added (4 new unit tests in `mod tests`)

| Test                                          | What it verifies                                                                |
| --------------------------------------------- | ------------------------------------------------------------------------------- |
| `test_sky_config_default_values`              | All 12 `SkyConfig::default()` fields match documented defaults                  |
| `test_map_with_sky_config_ron_roundtrip`      | `Map` with `sky: Some(…)` survives `ron::to_string` → `ron::from_str` roundtrip |
| `test_map_without_sky_config_backward_compat` | RON without `is_outdoor`/`sky` deserializes to `false`/`None`                   |
| `test_sky_config_partial_fields_deserialize`  | Omitting `cloud_speed` from RON yields the `1.0` default                        |

### RON Format Note

The `ron` crate serialises Rust `[f32; 4]` fixed-size arrays as RON **tuple**
syntax `(r, g, b, a)` — not list syntax `[r, g, b, a]`. All RON files and test
strings use parentheses accordingly.

### Quality Gates

```text
cargo fmt     → clean (no output)
cargo check   → Finished 0 errors
cargo clippy  → Finished 0 warnings
cargo nextest → 5095 passed, 8 skipped, 0 failed
```

---

## Phase 3: Bark Normal Map + Lit Bark

**Branch**: `pr-vegetation-updates`
**Date**: 2026-06-12

### Overview

Switched bark materials from unlit to lit shading and added a Sobel-derived
normal map so tree trunks show groove depth under the scene directional light.

### Files Changed

| File                                                       | Change                                                                                             |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src/bin/generate_normal_map.rs`                           | New binary — reads `bark.png`, applies 3×3 Sobel, writes `bark_normal.png` to both asset locations |
| `assets/textures/trees/bark_normal.png`                    | Generated RGB normal map (9.2 KB)                                                                  |
| `campaigns/tutorial/assets/textures/trees/bark_normal.png` | Copied from assets                                                                                 |
| `src/game/systems/procedural_meshes.rs`                    | Constant + material + test updates (see below)                                                     |
| `Cargo.toml`                                               | Added `[[bin]]` entry for `generate-normal-map`                                                    |

### Key Decisions

- **Sobel scale 4.0**: Moderate bumpiness; tunable via `SOBEL_SCALE` constant without regenerating from scratch.
- **`flip_normal_map_y: false`**: DirectX convention; flip to `true` if grooves appear inverted in one axis.
- **Clamp-to-edge sampling**: Avoids seam artefacts at bark texture borders; no wrapping.

### Constant Added

```rust
const TREE_BARK_NORMAL_TEXTURE: &str = "assets/textures/trees/bark_normal.png";
```

### Material Changes (`get_or_create_bark_material` + variant fallback)

- `unlit: true` → `unlit: false` in both `get_or_create_bark_material` and the inline fallback in `get_or_create_bark_material_variant`.
- `normal_map_texture: Some(normal_handle)` and `flip_normal_map_y: false` added to both.
- Normal handle loaded via `super::creature_meshes::load_texture`.

### Tests Updated

| Test                                                                        | Change                                                                                 |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `test_bark_material_uses_texture_and_is_lit_with_normal_map`                | Renamed from `…is_unlit`; asserts `!material.unlit` and `normal_map_texture.is_some()` |
| `test_bark_material_variant_cache_reuses_equivalent_tint_bucket_and_is_lit` | Renamed from `…is_unlit`; same assertion updates                                       |

### Quality Gates

```text
cargo fmt     → clean (no output)
cargo check   → Finished 0 errors
cargo clippy  → Finished 0 warnings
cargo nextest → 5204 passed, 8 skipped, 0 failed
```

---

## Phase 4: Cubic Bezier Grass Blades + Three-Color Gradient

**Branch**: `pr-vegetation-updates`
**Date**: 2026-06-12

### Overview

Upgraded grass blade geometry from quadratic to cubic Bezier (S-curve shape) and replaced the two-stop base/tip color gradient with a three-stop AO base / mid-green / tip highlight gradient.

### Files Changed

| File                                 | Change                                              |
| ------------------------------------ | --------------------------------------------------- |
| `src/game/systems/advanced_grass.rs` | All geometry, color, and struct changes (see below) |

### GrassColorScheme: New Fields

| Old field    | New field   | Default value                                    |
| ------------ | ----------- | ------------------------------------------------ |
| `base_color` | `ao_color`  | `srgb(0.08, 0.12, 0.06)` — dark AO base          |
| _(new)_      | `mid_color` | `srgb(0.2, 0.5, 0.1)` — primary mid-blade green  |
| `tip_color`  | `tip_color` | `srgb(0.72, 0.82, 0.45)` — lighter tip highlight |

### Cubic Bezier Control Points

| Point | Lateral (X)                                | Height (Y)      |
| ----- | ------------------------------------------ | --------------- |
| `p0`  | `0`                                        | `0`             |
| `p1`  | `tilt * height * 0.25`                     | `height * 0.33` |
| `p2`  | `tilt * height * 0.4 + curve_amount * 0.5` | `height * 0.66` |
| `p3`  | `curve_amount`                             | `height`        |

Formula: `B(t) = (1-t)³p0 + 3(1-t)²t·p1 + 3(1-t)t²·p2 + t³p3`

### Three-Stop Gradient

- `t ∈ [0.0, 0.4]` → lerp `ao_color` → `mid_color`
- `t ∈ [0.4, 1.0]` → lerp `mid_color` → `tip_color`

### Sites Updated

- `GrassColorScheme` struct definition and doc example
- `sample_blade_color`: blends `mid_color` + `tip_color` (70/30)
- `GrassColorScheme::default`: three new field values
- `cached_material_color`: `ao_color` as zero-variation endpoint
- `create_curved_grass_card_mesh`: cubic Bezier + `&[Color; 3]`
- `create_grass_clump_mesh`: three-color derivation per card
- `create_grass_blade_mesh` (test helper): `&[Color::WHITE; 3]`
- `spawn_grass_cached_with_exclusions`: `ao_color`, `mid_color`, `tip_color` from tint
- Doc comment: `GrassColorScheme.base_color` → `GrassColorScheme.mid_color`
- 3 test construction sites updated

### Quality Gates

```text
cargo fmt     → clean (no output)
cargo check   → Finished 0 errors
cargo clippy  → Finished 0 warnings
cargo nextest → 5204 passed, 8 skipped, 0 failed
```

---

## Phase 5: Per-Campaign Wind Configuration

**Branch**: `pr-vegetation-updates`
**Date**: 2026-06-12

### Overview

Added a `data/wind.ron` file to the campaign format so each campaign can configure its own grass-wind parameters. Introduced domain types (`WindSystemKind`, `CampaignWindConfig`), a Bevy resource (`WindConfig`), SDK loading and validation support, and sample data files for both the tutorial campaign and the test fixture.

### New Files

| File                                | Purpose                                                                 |
| ----------------------------------- | ----------------------------------------------------------------------- |
| `src/domain/world/wind.rs`          | `WindSystemKind` enum + `CampaignWindConfig` struct with serde defaults |
| `src/game/resources/wind_config.rs` | `WindConfig(pub CampaignWindConfig)` Bevy `Resource` newtype            |
| `campaigns/tutorial/data/wind.ron`  | Tutorial campaign uses `Sine` wind (strength 0.04, frequency 0.65)      |
| `data/test_campaign/data/wind.ron`  | Test campaign uses `None` (exercises serde defaults)                    |

### Modified Files

| File                                   | Change                                                                                                                 |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `src/domain/world/mod.rs`              | `pub mod wind;` + re-export `CampaignWindConfig`, `WindSystemKind`                                                     |
| `src/domain/mod.rs`                    | Re-export wind types from domain root                                                                                  |
| `src/domain/campaign_loader.rs`        | `GameData.wind` field, `load_wind_config` method, call in `load_game_data`                                             |
| `src/game/resources/mod.rs`            | `pub mod wind_config;` + `pub use wind_config::WindConfig;`                                                            |
| `src/game/systems/campaign_loading.rs` | Insert `WindConfig` resource in all 4 Ok/Err branches                                                                  |
| `src/sdk/database.rs`                  | `DatabaseError::WindLoadError`, `ContentDatabase.wind` field, load in `load_campaign_with_skills_file` and `load_core` |
| `src/sdk/validation.rs`                | `ValidationError::WindConfigInvalid`, `validate_wind_config`, call in `validate_all`                                   |
| `src/sdk/error_formatter.rs`           | `get_suggestions` arm for `WindConfigInvalid`                                                                          |

### WindSystemKind Variants

| Variant          | Description                                                                       |
| ---------------- | --------------------------------------------------------------------------------- |
| `None` (default) | No wind animation                                                                 |
| `Sine`           | Sinusoidal sway driven by `strength` and `frequency`                              |
| `Perlin`         | Spatially coherent noise; enables `perlin_scale`, `perlin_octaves`, `perlin_seed` |

### CampaignWindConfig Fields and Defaults

| Field            | Type             | Default      | Notes                                      |
| ---------------- | ---------------- | ------------ | ------------------------------------------ |
| `wind_system`    | `WindSystemKind` | `None`       | Required only if sway is desired           |
| `strength`       | `f32`            | `0.04`       | World-unit sway amplitude (>= 0.0)         |
| `frequency`      | `f32`            | `0.65`       | Cycles per second (> 0.0)                  |
| `direction`      | `[f32; 2]`       | `[1.0, 0.0]` | Normalised XZ; must be finite and non-zero |
| `perlin_scale`   | `f32`            | `100.0`      | Noise tiling scale (Perlin only, > 0.0)    |
| `perlin_octaves` | `u32`            | `4`          | Octave count (Perlin only, 1-8)            |
| `perlin_seed`    | `u32`            | `0`          | RNG seed (Perlin only)                     |

### Key Design Decisions

- **Opt-in loading**: missing `data/wind.ron` silently returns `CampaignWindConfig::default()` (same pattern as `landscape` and `levels`).
- **Newtype resource**: `WindConfig(pub CampaignWindConfig)` keeps the Bevy layer thin — no duplication of fields.
- **SDK validation range checks**: five rules enforced at validation time, not load time, so missing fields with defaults never fail.
- **`#[derive(Default)]` on WindConfig**: satisfies `clippy::derivable_impls` because `CampaignWindConfig` already implements `Default`.

### Quality Gates

```text
cargo fmt     → clean (no output)
cargo check   → Finished 0 errors
cargo clippy  → Finished 0 warnings
cargo nextest → 5212 passed, 8 skipped, 0 failed
```

---

## Phase 6: WGSL Grass Wind Shader (Sine + Perlin)

**Branch**: `pr-vegetation-updates`
**Date**: 2026-06-13

### Overview

Implemented a custom WGSL vertex shader for grass blades that supports three wind animation modes: `None` (static), `Sine` (sinusoidal sway driven by global time), and `Perlin` (spatially coherent fBm noise). The shader uses `ExtendedMaterial<StandardMaterial, GrassWindExtension>` so the grass keeps full PBR lighting while adding per-blade sway.

### New Files

| File                        | Purpose                                                                                                              |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `assets/shaders/grass.wgsl` | WGSL vertex shader with None/Sine/Perlin wind paths; uses `@group(2) @binding(100..102)` for wind extension bindings |

### Modified Files

| File                                 | Change                                                                                                                                                                                                                                 |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Cargo.toml`                         | Added `noise = "0.9"` dependency for fBm Perlin noise texture generation                                                                                                                                                               |
| `src/game/systems/advanced_grass.rs` | New types `GrassWindUniform`, `GrassWindExtension`, `GrassMaterial`, `WindNoiseTexture`; migrated all `Handle<StandardMaterial>` to `Handle<GrassMaterial>`; added `generate_wind_noise_texture` and `setup_wind_noise_texture_system` |
| `src/game/systems/map.rs`            | Registered `MaterialPlugin::<GrassMaterial>`, added `setup_wind_noise_texture_system` to Startup, updated `spawn_map` and `spawn_map_markers` to propagate `Option<ResMut<Assets<GrassMaterial>>>`                                     |
| `benches/grass_instancing.rs`        | Updated to use `GrassMaterial` instead of `StandardMaterial`                                                                                                                                                                           |

### New Types (advanced_grass.rs)

| Type                 | Description                                                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| `GrassWindUniform`   | GPU-aligned uniform struct (`ShaderType`): strength, frequency, direction, wind_system, perlin_scale, `_pad` |
| `GrassWindExtension` | `MaterialExtension` with `#[uniform(100)]` wind + `#[texture(101)]`/`#[sampler(102)]` noise                  |
| `GrassMaterial`      | Type alias for `ExtendedMaterial<StandardMaterial, GrassWindExtension>`                                      |
| `WindNoiseTexture`   | Bevy resource wrapping `Handle<Image>` for the 512×512 fBm Perlin noise texture                              |

### WGSL Binding Layout

Bindings start at 100 to avoid collisions with `StandardMaterial`'s reserved groups:

| Binding                   | Type                       | Purpose              |
| ------------------------- | -------------------------- | -------------------- |
| `@group(2) @binding(100)` | `uniform GrassWindUniform` | Wind parameters      |
| `@group(2) @binding(101)` | `texture_2d<f32>`          | Perlin noise texture |
| `@group(2) @binding(102)` | `sampler`                  | Noise sampler        |

### Key Design Decisions

- **`textureSampleLevel` not `textureSample`**: vertex shaders in WGSL require explicit LOD; used level 0.
- **`bevy::shader::ShaderRef`** (not `bevy::render::render_resource::ShaderRef`): the type is in the `bevy_shader` crate.
- **`Option<ResMut<Assets<GrassMaterial>>>`** in `spawn_map` / `spawn_map_markers`: makes the parameter optional so tests using `MapManagerPlugin` (without `MaterialPlugin::<GrassMaterial>`) don't fail resource validation; grass is silently skipped when the resource is absent.
- **512×512 RGBA8 fBm noise texture**: generated at startup by `setup_wind_noise_texture_system`; falls back to a 1×1 white placeholder when `WindConfig` is absent or `wind_system` is not `Perlin`.
- **`GrassAssetCache.set_wind()`**: clears cached materials so they are recreated with updated wind uniforms on map transition.
- **`#[allow(clippy::too_many_arguments)]`** on `build_grass_chunks_system`: adding `Res<Assets<GrassMaterial>>` pushed arg count to 8.

### Quality Gates

```text
cargo fmt     → clean (no output)
cargo check   → Finished 0 errors
cargo clippy  → Finished 0 warnings
cargo nextest → 5212 passed, 8 skipped, 0 failed
```

---
