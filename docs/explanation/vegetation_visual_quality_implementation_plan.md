<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vegetation Visual Quality Implementation Plan

## Overview

This plan fixes the current tree and grass rendering problems in Antares by replacing the “procedural shapes that technically render” approach with a species-specific vegetation pipeline inspired by:

- `Affinator/bevy_procedural_tree` for recursive trunk/branch generation, bark UVs, and separate leaf meshes.
- `jadedbay/bevy_procedural_grass` for grass blade LOD, wind-ready instancing, and culling concepts.
- `kurtkuehnert/bevy_terrain` for terrain attachment concepts, chunked vegetation placement, and distance-aware rendering strategy.

The goal is not to blindly add incompatible third-party dependencies. Antares currently uses Bevy `0.17`; `bevy_procedural_tree` has a Bevy `0.17` release line, but the grass and terrain references are older Bevy versions. The safer route is to port the proven concepts into Antares’ existing `src/game/systems` rendering layer while keeping domain data in `src/domain/world/types.rs` unchanged unless a phase explicitly calls out a small extension.

The plan prioritizes visible quality first: distinct oak, pine, palm, willow, birch, dead tree, and shrub silhouettes; bark textures that actually map onto branch meshes; foliage that no longer appears as black blobs; grass that forms believable clumps; and SDK edits that visibly affect the spawned result.

---

## Current State Analysis

### Existing Infrastructure

| Area                           | Existing File(s)                                                                                | Current Capability                                                                                                                                 | Relevance                                                                |
| ------------------------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Domain vegetation metadata     | `src/domain/world/types.rs`                                                                     | `TileVisualMetadata` includes `grass_density`, `tree_type`, `foliage_density`, `grass_blade_config`, `height`, `scale`, `color_tint`, `rotation_y` | Keep these fields as the authoring source of truth                       |
| Runtime tree generation        | `src/game/systems/advanced_trees.rs`                                                            | Generates a branch graph and a single combined branch/foliage mesh                                                                                 | Needs species-specific generation and separate bark/leaf meshes          |
| Runtime tree spawning          | `src/game/systems/procedural_meshes.rs`                                                         | Spawns tree branch mesh, bark material, foliage quads, shrubs, portals, structures, props                                                          | Main integration point for improved tree visuals                         |
| Runtime grass spawning         | `src/game/systems/advanced_grass.rs`                                                            | Spawns individual curved grass blade meshes and materials under a cluster parent                                                                   | Needs batching, clumping, material reuse, and better visual distribution |
| Map vegetation placement       | `src/game/systems/map.rs`                                                                       | Spawns forest floor, default trees, explicit tree types, extra shrubs, and grass cover                                                             | Needs placement rules to prevent shrubs/foliage clipping tree trunks     |
| Campaign Builder map authoring | `sdk/campaign_builder/src/map_editor.rs`                                                        | Exposes tree type, grass density, grass blade config, foliage density, and visual presets                                                          | Needs preview feedback and metadata fields wired to runtime effects      |
| Texture assets                 | `assets/textures/trees/`, `campaigns/tutorial/assets/textures/trees/`, `assets/textures/grass/` | Bark, foliage masks, and grass blade textures exist                                                                                                | Need verification that assets are loaded and UVs exist                   |
| Runtime asset root             | `src/bin/antares.rs`                                                                            | Sets `BEVY_ASSET_ROOT` to the active campaign root and configures `AssetPlugin.file_path = "."`                                                    | Texture paths must resolve correctly under campaign roots                |
| Visual quality reference       | `docs/explanation/procedural_mesh_visual_quality.md`                                            | Defines desired tree and grass quality targets                                                                                                     | Needs correction to match implementation reality                         |
| User-reported issue tracker    | `docs/explanation/next_plans.md`                                                                | Notes trees and grass look terrible, tree types look the same, bark is unclear, bushes clip trunks, SDK edits appear ineffective                   | This plan directly addresses those notes                                 |

### Identified Issues

| ID    | Issue                                                              | Likely Root Cause                                                                                                                           | User-Visible Effect                                                         |
| ----- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| VQ-1  | Bark texture appears absent or ineffective                         | `advanced_trees.rs` branch mesh generation does not generate bark UVs/tangents suitable for `TREE_BARK_TEXTURE`                             | Trunks look flat, muddy, or identical                                       |
| VQ-2  | Black foliage blobs appear around trees                            | `generate_branch_mesh()` still adds foliage sphere geometry into the bark-material tree mesh while `spawn_tree()` also spawns foliage quads | Leaf spheres are rendered with bark/dark material and look like black blobs |
| VQ-3  | Tree foliage and branches do not align under metadata scale/height | Branch mesh child is scaled, but foliage quads are parented separately using unscaled branch graph positions                                | Foliage floats, clips, or appears detached                                  |
| VQ-4  | Oak, pine, palm, dead, willow, and shrub look too similar          | `generate_branch_graph()` uses the same recursive subdivision model with only shallow config differences                                    | Species silhouettes are not visually distinct                               |
| VQ-5  | Palm trees are not palm trees                                      | Palm uses generic recursive child branches instead of a tall trunk with crown fronds                                                        | Palm looks like another branching tree                                      |
| VQ-6  | Pine trees are not conical enough                                  | Pine uses generic branches and generic foliage clusters instead of layered conifer whorls                                                   | Pine looks like a thin oak                                                  |
| VQ-7  | Dead trees still read as normal trees                              | Dead tree geometry is not gnarled enough and material/texture does not emphasize weathering                                                 | Dead trees do not look dead                                                 |
| VQ-8  | Shrubs/bushes clip tree trunks                                     | `map.rs` can spawn an extra shrub on the same forest tile as a tree                                                                         | Bushes overlap trunks and look broken                                       |
| VQ-9  | SDK edits appear ineffective                                       | `foliage_density`, `scale`, and some grass metadata are either ignored or only partially applied by runtime systems                         | Changing sliders in Campaign Builder does not obviously alter visuals       |
| VQ-10 | Grass looks thin, spiky, and repetitive                            | Grass uses per-blade meshes/materials with small random clusters rather than chunked clumps/cross-cards/instancing                          | Grass reads as sparse noise instead of vegetation                           |
| VQ-11 | Grass performance model is not ready for high-density scenes       | Each blade creates its own mesh and material; batching resources are diagnostic/future-facing rather than the active render path            | Raising density risks draw-call and asset-count explosions                  |
| VQ-12 | Visual quality guide overstates current results                    | `procedural_mesh_visual_quality.md` claims quality targets pass, but user-visible output contradicts it                                     | Documentation does not guide real fixes                                     |

### External Reference Findings

| Reference                        | Compatible as Dependency?                                                         | Useful Concepts                                                                                                                  | Plan Decision                                                                       |
| -------------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `Affinator/bevy_procedural_tree` | Bevy `0.17` compatible release exists; latest targets Bevy `0.18`                 | `TreeMeshSettings`, separate branch and leaf meshes, branch sections/segments, bark UVs, leaf billboards, seeded reproducibility | Port or adapt concepts first; direct dependency only if version/API fit is verified |
| `jadedbay/bevy_procedural_grass` | Main public code targets older Bevy versions                                      | generated grass mesh, LOD mesh, wind/displacement concepts, GPU instancing, culling                                              | Do not depend directly; port concepts into `advanced_grass.rs`                      |
| `kurtkuehnert/bevy_terrain`      | Targets older Bevy versions and solves large terrain rendering, not tile RPG maps | chunked clipmaps, attachment data, distance-dependent LOD, terrain/vegetation data separation                                    | Use as design inspiration for chunking and data attachments only                    |

---

## Implementation Phases

---

### Phase 1: Diagnose and Correct the Existing Vegetation Pipeline

This phase fixes the most visible broken behavior without introducing a full new renderer. The key goal is to stop rendering incorrect foliage geometry, prove textures are loaded, and make metadata edits visibly affect runtime output.

#### 1.1 Foundation Work

Audit the current tree and grass render path from map data to spawned entities.

Required inspections:

| File                                     | Symbol/Area                                           | Required Finding                                                      |
| ---------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| `src/domain/world/types.rs`              | `TileVisualMetadata`                                  | Confirm field meanings and defaults remain unchanged                  |
| `src/game/systems/map.rs`                | `spawn_map` forest/grass branch                       | Document exactly when trees, shrubs, and grass spawn                  |
| `src/game/systems/advanced_trees.rs`     | `generate_branch_mesh()`                              | Confirm foliage sphere generation is still included in branch mesh    |
| `src/game/systems/procedural_meshes.rs`  | `spawn_tree()`, `spawn_foliage_clusters()`            | Confirm branch mesh and foliage quads use different transforms/scales |
| `src/game/systems/advanced_grass.rs`     | `spawn_grass()`                                       | Confirm which metadata fields are used and which are ignored          |
| `sdk/campaign_builder/src/map_editor.rs` | `TerrainEditorState::apply_to_metadata_for_terrain()` | Confirm SDK writes fields that runtime consumes                       |

#### 1.2 Correct Existing Tree Rendering Defects

Make the smallest changes needed to stop the current “black blob” and clipping failures.

Required changes:

| File                                    | Required Change                                                                                                                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/advanced_trees.rs`    | Remove foliage sphere generation from `generate_branch_mesh()` or gate it behind a test-only/debug-only path                                                        |
| `src/game/systems/advanced_trees.rs`    | Ensure the branch mesh is branch/trunk geometry only                                                                                                                |
| `src/game/systems/procedural_meshes.rs` | Spawn foliage as a child of the same scaled transform context as the tree structure, or apply the same `scale` and `height_multiplier` to foliage cluster positions |
| `src/game/systems/procedural_meshes.rs` | Apply `TileVisualMetadata::foliage_density()` to the resolved tree config before spawning foliage clusters                                                          |
| `src/game/systems/procedural_meshes.rs` | Verify `TreeType::Dead` never spawns foliage quads                                                                                                                  |
| `src/game/systems/map.rs`               | Stop placing random shrubs at the exact center of forest tiles that already contain a tree                                                                          |
| `src/game/systems/map.rs`               | If extra shrubs remain, offset them away from trunk radius using deterministic tile-seeded placement                                                                |

#### 1.3 Correct Existing Bark and Leaf Materials

Make texture use observable and testable.

Required changes:

| File                                    | Required Change                                                                                                                  |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/advanced_trees.rs`    | Add `Mesh::ATTRIBUTE_UV_0` to branch/trunk meshes                                                                                |
| `src/game/systems/advanced_trees.rs`    | Generate branch UVs so bark texture repeats along branch sections, following the `bevy_procedural_tree` section/segment UV model |
| `src/game/systems/advanced_trees.rs`    | Generate tangents if required by Bevy PBR material features                                                                      |
| `src/game/systems/procedural_meshes.rs` | Keep bark material separate from foliage material                                                                                |
| `src/game/systems/procedural_meshes.rs` | Use species-specific bark tint/material parameters for dead, birch, palm, and pine instead of one shared brown                   |
| `src/game/systems/procedural_meshes.rs` | Add debug logging when vegetation texture handles are requested, using concise path and tree type information                    |
| `src/game/systems/procedural_meshes.rs` | Ensure foliage material alpha mode remains masked and double-sided                                                               |

#### 1.4 Wire SDK Metadata to Runtime Effects

Make Campaign Builder edits change visible runtime output.

Required changes:

| Metadata Field                       | Runtime Effect                                                   |
| ------------------------------------ | ---------------------------------------------------------------- |
| `height`                             | Tree branch mesh and foliage positions scale vertically together |
| `scale`                              | Tree/shrub/grass footprint scales horizontally                   |
| `color_tint`                         | Bark and foliage tint predictably without darkening to black     |
| `rotation_y`                         | Entire tree, including foliage, rotates together                 |
| `tree_type`                          | Selects species-specific generator preset                        |
| `foliage_density`                    | Changes foliage count/coverage for trees and shrubs              |
| `grass_density`                      | Changes grass clump count                                        |
| `grass_blade_config.length`          | Changes blade height                                             |
| `grass_blade_config.width`           | Changes blade width                                              |
| `grass_blade_config.tilt`            | Changes lean                                                     |
| `grass_blade_config.curve`           | Changes blade curvature                                          |
| `grass_blade_config.color_variation` | Changes blade color variance                                     |

#### 1.5 Testing Requirements

Add or update tests without referencing `campaigns/tutorial`.

Required tests:

| File                                     | Test Requirement                                                                         |
| ---------------------------------------- | ---------------------------------------------------------------------------------------- |
| `src/game/systems/advanced_trees.rs`     | Branch mesh has UV coordinates                                                           |
| `src/game/systems/advanced_trees.rs`     | Branch mesh generated for `TreeType::Dead` contains no foliage sphere vertices           |
| `src/game/systems/procedural_meshes.rs`  | `spawn_tree()` spawns no foliage children for `TreeType::Dead`                           |
| `src/game/systems/procedural_meshes.rs`  | `foliage_density = 0.0` suppresses foliage for non-dead trees                            |
| `src/game/systems/procedural_meshes.rs`  | `height` and `scale` metadata affect both branch and foliage transforms                  |
| `src/game/systems/map.rs`                | Forest tile with explicit tree no longer spawns a centered shrub on the same trunk point |
| `sdk/campaign_builder/src/map_editor.rs` | Terrain editor metadata fields round-trip for grass and forest tiles                     |

#### 1.6 Deliverables

- [x] `advanced_trees.rs` branch mesh no longer includes leaf/foliage sphere geometry.
- [x] Bark mesh UVs exist and are suitable for texture repetition.
- [x] Tree foliage transforms align with trunk transforms.
- [x] `foliage_density` visibly affects tree foliage.
- [x] Dead trees never spawn foliage.
- [x] Shrubs no longer clip trunk centers by default.
- [x] SDK metadata edits produce visible runtime changes.
- [x] Regression tests cover the fixed behavior.

#### 1.7 Success Criteria

- The black oval/blob foliage artifacts are gone.
- Bark texture or bark patterning is visibly present on trunks.
- Dead trees are bare.
- Changing a forest tile from Oak to Pine/Palm/Dead in Campaign Builder changes the spawned tree.
- Changing foliage density in Campaign Builder visibly changes foliage amount.
- Shrubs do not spawn directly inside tree trunks.
- Existing game state/domain structures remain architecture-compliant.

---

### Phase 2: Replace Generic Trees with Species-Specific Tree Presets

This phase turns every tree type into a distinct silhouette. It ports the useful parts of `bevy_procedural_tree` into Antares’ existing rendering layer.

#### 2.1 Foundation Work

Introduce a richer internal tree preset model in `src/game/systems/advanced_trees.rs`.

Recommended internal types:

| Type                 | Purpose                                                                                 |
| -------------------- | --------------------------------------------------------------------------------------- |
| `TreeSpeciesPreset`  | Complete render-only preset for one Antares `TreeType`                                  |
| `BranchPreset`       | Branch recursion, sections, segments, taper, twist, gnarliness, force, child counts     |
| `LeafPreset`         | Leaf billboard mode, leaf size, count, start point, angle, size variance                |
| `TreeMeshPair`       | Separate `branches: Mesh` and `leaves: Mesh` result                                     |
| `TreeGenerationSeed` | Deterministic seed derived from map ID, tile position, tree type, and optional metadata |

The preset model should be render-layer only. Do not move Bevy types into the domain layer.

#### 2.2 Implement Species Presets

Create distinct presets for every existing domain `TreeType`.

| Antares Tree Type | Required Silhouette                                   | Required Mesh Behavior                                                                                    |
| ----------------- | ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `Oak`             | Broad canopy, thick trunk, spreading lateral branches | Deciduous preset, high branch count, medium-height trunk, dense double-billboard leaves                   |
| `Pine`            | Tall conical evergreen                                | Strong upward trunk, radial branch whorls, lower/mid foliage wider than top, dark green needle-like cards |
| `Birch`           | Slender pale trunk, lighter foliage                   | Thin trunk, lighter bark material/tint, medium sparse canopy                                              |
| `Willow`          | Drooping curtain canopy                               | Branch force downward, long hanging leaf cards, foliage below branch ends                                 |
| `Dead`            | Twisted bare tree                                     | High gnarliness, dark/grey bark, no leaves, irregular branch lengths                                      |
| `Shrub`           | Low multi-stem bush                                   | Multiple short stems, foliage around perimeter, no tall central trunk                                     |
| `Palm`            | Tall slender trunk with crown fronds                  | Single curved trunk, no recursive side branches, long radial frond cards at crown                         |

#### 2.3 Replace Single-Mesh Tree Cache

Update `src/game/systems/procedural_meshes.rs` cache behavior.

Required changes:

| Current Cache                                                         | Replacement                                                                     |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `tree_meshes: HashMap<TreeType, Handle<Mesh>>`                        | Cache branch and leaf mesh handles separately                                   |
| `tree_foliage: Option<Handle<Mesh>>`                                  | Replace with per-species leaf mesh or shared billboard mesh depending on preset |
| `tree_bark_material: Option<Handle<StandardMaterial>>`                | Replace or extend with per-species bark materials                               |
| `tree_foliage_materials: HashMap<TreeType, Handle<StandardMaterial>>` | Keep, but ensure foliage tint/material overrides do not mutate shared handles   |

Cache keys should include values that alter geometry, not only `TreeType`.

Minimum geometry cache key fields:

| Field                    | Reason                                              |
| ------------------------ | --------------------------------------------------- |
| `tree_type`              | Species shape                                       |
| `foliage_density_bucket` | Density can change leaf count                       |
| `quality_level`          | Future LOD compatibility                            |
| `variant_seed_bucket`    | Allows several reusable visual variants per species |

Do not include every tile position in the mesh cache key unless generating a unique tree per tile is explicitly accepted for memory/performance reasons.

#### 2.4 Add Deterministic Variation

Avoid every oak or pine being identical.

Required behavior:

| Variation                | Rule                                                               |
| ------------------------ | ------------------------------------------------------------------ |
| Seed source              | Derive from map ID, tile position, tree type, and stable constants |
| Height variance          | ±10–20 percent after metadata scaling                              |
| Trunk bend variance      | Small deterministic curve per tile                                 |
| Branch rotation variance | Deterministic per tree                                             |
| Leaf size variance       | Deterministic per mesh generation                                  |
| Color variance           | Small material tint variation per tree or per cached variant       |
| LOD safety               | Variation must not create uncontrolled mesh cache growth           |

#### 2.5 Testing Requirements

Required tests:

| File                                    | Test Requirement                                        |
| --------------------------------------- | ------------------------------------------------------- |
| `src/game/systems/advanced_trees.rs`    | Each `TreeType` generates non-empty branch mesh         |
| `src/game/systems/advanced_trees.rs`    | `Dead` generates empty or absent leaves mesh            |
| `src/game/systems/advanced_trees.rs`    | `Palm` branch graph/preset has crown-only leaves/fronds |
| `src/game/systems/advanced_trees.rs`    | `Pine` bounds are taller than wide                      |
| `src/game/systems/advanced_trees.rs`    | `Oak` bounds are wider relative to height than pine     |
| `src/game/systems/advanced_trees.rs`    | Same seed produces same mesh statistics                 |
| `src/game/systems/procedural_meshes.rs` | Mesh cache reuses handles for equivalent cache keys     |

#### 2.6 Deliverables

- [ ] Render-only `TreeSpeciesPreset` model exists.
- [ ] Tree generation returns separate branch and leaf meshes.
- [ ] Oak, Pine, Birch, Willow, Dead, Shrub, and Palm have distinct presets.
- [ ] Bark UVs and leaf UVs are generated.
- [ ] Mesh cache supports branch/leaf separation.
- [ ] Deterministic variation exists without unbounded cache growth.
- [ ] Tests verify species shape differences.

#### 2.7 Success Criteria

- A player can identify Oak, Pine, Palm, Dead, Willow, Birch, and Shrub at a glance.
- Palm trees have crown fronds, not generic recursive branches.
- Pine trees have a conical evergreen silhouette.
- Dead trees are bare and gnarled.
- Tree materials no longer make foliage look like bark or black blobs.
- Repeated trees vary enough to avoid a copy-paste forest.

---

### Phase 3: Upgrade Grass to Clumped, Batched, Wind-Ready Vegetation

This phase improves grass visual quality and performance. It ports concepts from `bevy_procedural_grass` while staying compatible with Antares’ Bevy version and tile-map structure.

#### 3.1 Foundation Work

Refactor `src/game/systems/advanced_grass.rs` around reusable meshes and materials.

Required internal concepts:

| Type/Concept         | Purpose                                                   |
| -------------------- | --------------------------------------------------------- |
| `GrassPatch`         | Parent component for one tile or tile-subpatch grass area |
| `GrassClump`         | Small group of blades/cross-cards spawned together        |
| `GrassMeshQuality`   | Low/Medium/High segment count mesh variants               |
| `GrassMaterialKey`   | Cache key for texture/tint/alpha/material settings        |
| `GrassPlacementSeed` | Deterministic seed derived from map ID and tile position  |
| `GrassWindParams`    | Render-layer wind values, even if animation comes later   |

#### 3.2 Replace Per-Blade Mesh/Material Creation

Current grass creates a unique mesh and material for each blade. Replace this with reusable assets.

Required changes:

| Current Behavior              | Replacement                                                   |
| ----------------------------- | ------------------------------------------------------------- |
| New mesh per blade            | Shared blade mesh variants by quality and blade config bucket |
| New material per blade        | Shared material per color/tint bucket                         |
| Random runtime placement      | Deterministic tile-seeded placement                           |
| Individual sparse blades only | Clumps made from several rotated blades/cards                 |
| `foliage_density` ignored     | Use as clump count multiplier or coverage multiplier          |

#### 3.3 Improve Grass Geometry

Required geometry upgrades:

| Feature           | Requirement                                                                      |
| ----------------- | -------------------------------------------------------------------------------- |
| Blade shape       | Curved tapered blade with 3–7 segments depending on quality                      |
| Cross-card clumps | 2–4 rotated blade cards per clump for volume                                     |
| Height variation  | Deterministic variation from `grass_blade_config.length`                         |
| Width variation   | Deterministic variation from `grass_blade_config.width`                          |
| Tilt and bend     | Use `grass_blade_config.tilt` and `grass_blade_config.curve` visibly             |
| Color variation   | Apply `grass_blade_config.color_variation` without creating a material per blade |
| Texture           | Keep `assets/textures/grass/grass_blade.png` alpha-masked and double-sided       |
| Ground avoidance  | Keep blades slightly above floor without z-fighting                              |

#### 3.4 Add Grass LOD and Culling

Build on existing `GrassCluster`, `GrassRenderConfig`, and `grass_lod_system`.

Required behavior:

| Distance Band        | Behavior                                                           |
| -------------------- | ------------------------------------------------------------------ |
| Near                 | Full clump count and full blade mesh                               |
| Mid                  | Reduced clump count or lower segment mesh                          |
| Far                  | Billboard/card impostor or hidden depending on `GrassRenderConfig` |
| Beyond cull distance | Hidden or not spawned                                              |

The active render path must use the optimized representation. Existing `GrassInstanceBatch` should not remain only diagnostic if high-density grass depends on it.

#### 3.5 Testing Requirements

Required tests:

| File                                 | Test Requirement                                                              |
| ------------------------------------ | ----------------------------------------------------------------------------- |
| `src/game/systems/advanced_grass.rs` | `GrassDensity::None` spawns no blade/clump children                           |
| `src/game/systems/advanced_grass.rs` | Higher `GrassDensity` produces higher clump count                             |
| `src/game/systems/advanced_grass.rs` | `foliage_density` scales grass coverage                                       |
| `src/game/systems/advanced_grass.rs` | Mesh/material handles are reused across similar blades                        |
| `src/game/systems/advanced_grass.rs` | `grass_blade_config` affects generated mesh/config buckets                    |
| `src/game/systems/advanced_grass.rs` | LOD system hides or reduces grass at distance                                 |
| `src/game/systems/map.rs`            | Grass terrain and forest terrain both spawn grass cover according to metadata |

#### 3.6 Deliverables

- [ ] Grass uses reusable mesh variants.
- [ ] Grass uses reusable material variants.
- [ ] Grass clumps replace sparse per-blade noise.
- [ ] `foliage_density` affects grass coverage.
- [ ] Grass LOD path is active.
- [ ] Grass culling remains active.
- [ ] Tests cover density, metadata, batching, and LOD behavior.

#### 3.7 Success Criteria

- Grass reads as patches/clumps instead of isolated spikes.
- Increasing grass density in the SDK visibly increases coverage.
- Dried/tinted grass looks different without becoming black.
- High-density grass does not create one mesh and one material per blade.
- Forest ground cover looks natural without swallowing tree trunks.

---

### Phase 4: Add Vegetation Placement Rules and Terrain-Aware Composition

This phase fixes environmental composition: trees, shrubs, and grass should occupy believable locations within a tile instead of all stacking at the tile center.

#### 4.1 Foundation Work

Introduce deterministic vegetation placement helpers in `src/game/systems/map.rs` or a new render-layer module such as `src/game/systems/vegetation_placement.rs`.

Required helper concepts:

| Helper                                                    | Purpose                                               |
| --------------------------------------------------------- | ----------------------------------------------------- |
| `vegetation_seed(map_id, position, salt)`                 | Stable deterministic seed                             |
| `tree_anchor_for_tile(position, metadata)`                | Main tree placement inside tile                       |
| `shrub_anchors_for_tile(position, tree_radius, metadata)` | Shrub positions outside trunk exclusion radius        |
| `grass_exclusion_zones(position, vegetation)`             | Prevent grass/clumps inside trunk and prop footprints |
| `tile_vegetation_plan(tile, map_id, position)`            | One deterministic plan for all vegetation on a tile   |

#### 4.2 Prevent Intra-Tile Clipping

Required placement rules:

| Object                    | Rule                                                                             |
| ------------------------- | -------------------------------------------------------------------------------- |
| Main tree                 | Spawn near tile center unless metadata provides offset in a future extension     |
| Shrubs on tree tile       | Must avoid trunk radius plus safety margin                                       |
| Shrubs on shrub-only tile | Can use center or multiple low offsets                                           |
| Grass near tree           | Reduce clumps inside trunk radius                                                |
| Grass near shrubs         | Reduce or lower grass near shrub stems                                           |
| Foliage                   | Must align with tree branch/leaf transform                                       |
| Signs/doors/props         | Vegetation should not spawn through known blocking props where data is available |

#### 4.3 Use Metadata Consistently

Required metadata interpretation:

| Field             | Placement Effect                                               |
| ----------------- | -------------------------------------------------------------- |
| `scale`           | Expands trunk/shrub/grass footprint                            |
| `width_x`         | Optional footprint width override                              |
| `width_z`         | Optional footprint depth override                              |
| `y_offset`        | Applies to visual parent if relevant                           |
| `rotation_y`      | Rotates tree/shrub visual and any asymmetric placement         |
| `foliage_density` | Controls foliage/coverage, not random unrelated shrub spawning |

#### 4.4 Testing Requirements

Required tests:

| File                                                   | Test Requirement                                                             |
| ------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `src/game/systems/map.rs` or `vegetation_placement.rs` | Same tile produces same vegetation plan across runs                          |
| `src/game/systems/map.rs` or `vegetation_placement.rs` | Shrub anchors avoid tree trunk exclusion radius                              |
| `src/game/systems/map.rs` or `vegetation_placement.rs` | Grass clumps avoid trunk exclusion zone                                      |
| `src/game/systems/map.rs`                              | Forest tile with default tree and extra shrubs has non-overlapping positions |
| `src/game/systems/map.rs`                              | Explicit `TreeType::Shrub` tile does not also spawn a full-size default tree |

#### 4.5 Deliverables

- [ ] Deterministic vegetation placement helper exists.
- [ ] Tree trunk exclusion radius is enforced.
- [ ] Shrubs no longer clip tree trunks.
- [ ] Grass avoids obvious trunk intersections.
- [ ] Placement respects metadata scale.
- [ ] Tests cover deterministic placement and exclusion rules.

#### 4.6 Success Criteria

- Forest scenes look intentionally composed rather than randomly stacked.
- Bushes do not intersect tree trunks.
- Grass remains present but does not visibly grow through large trunks.
- Re-entering the same map produces the same vegetation arrangement.

---

### Phase 5: Improve Campaign Builder Feedback for Vegetation Authoring

This phase makes SDK edits obviously meaningful. The current complaint that editing vegetation “does nothing” must be addressed in the authoring UI as well as runtime.

#### 5.1 Foundation Work

Audit Campaign Builder map editor controls in `sdk/campaign_builder/src/map_editor.rs`.

Required audit points:

| Area                      | Requirement                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------- |
| Terrain-specific controls | Every visible vegetation control must map to runtime behavior                            |
| Presets                   | Tree and grass presets should set tree type/density fields, not only generic height/tint |
| Dirty state               | Editing vegetation metadata must mark map data dirty                                     |
| Save path                 | Saved RON must include changed vegetation metadata                                       |
| Reload path               | Reopening the map must restore vegetation metadata into controls                         |

#### 5.2 Update Visual Presets

Current presets such as `DeadTree`, `ShortTree`, `TallGrass`, and `DriedGrass` mostly set height, scale, and tint. Make presets set semantic vegetation fields too.

Required preset behavior examples:

| Preset       | Required Metadata                                                                              |
| ------------ | ---------------------------------------------------------------------------------------------- |
| `DeadTree`   | `tree_type = Some(Dead)`, dead bark tint, suitable height/scale, `foliage_density = Some(0.0)` |
| `SmallTree`  | `tree_type = Some(Oak)` or explicit species chosen by preset, lower height/scale               |
| `LargeTree`  | `tree_type = Some(Oak)`, higher height/scale, higher foliage density                           |
| `SmallShrub` | `tree_type = Some(Shrub)`, low height/scale                                                    |
| `LargeShrub` | `tree_type = Some(Shrub)`, higher foliage density                                              |
| `ShortGrass` | `grass_density = Some(Low)`, shorter `grass_blade_config`                                      |
| `TallGrass`  | `grass_density = Some(High)`, taller `grass_blade_config`                                      |
| `DriedGrass` | tan tint, lower color variation or dry color scheme                                            |

#### 5.3 Add Authoring Feedback

Recommended UI improvements:

| UI Feature                | Purpose                                                        |
| ------------------------- | -------------------------------------------------------------- |
| Vegetation summary label  | Show resolved tree type, grass density, foliage density, scale |
| “Runtime effect” hints    | Explain what each slider changes in-game                       |
| Optional preview swatch   | Show tree/grass icon or compact textual preview                |
| Reset vegetation button   | Clears vegetation metadata for selected tile                   |
| Apply preset to selection | Makes batch editing obvious                                    |

If this touches multi-column egui screens, follow the project’s `allocate_ui` multi-column layout rule.

#### 5.4 Testing Requirements

Required tests:

| File                                     | Test Requirement                                       |
| ---------------------------------------- | ------------------------------------------------------ |
| `sdk/campaign_builder/src/map_editor.rs` | Visual presets set semantic vegetation fields          |
| `sdk/campaign_builder/src/map_editor.rs` | Applying terrain metadata for Grass keeps grass fields |
| `sdk/campaign_builder/src/map_editor.rs` | Applying terrain metadata for Forest keeps tree fields |
| `sdk/campaign_builder/src/map_editor.rs` | Grass blade config survives round-trip when enabled    |
| `sdk/campaign_builder/src/map_editor.rs` | Clearing metadata removes vegetation-specific fields   |

#### 5.5 Deliverables

- [ ] Vegetation presets set runtime-consumed fields.
- [ ] SDK terrain controls visibly correspond to runtime effects.
- [ ] Save/reload preserves vegetation metadata.
- [ ] Optional authoring summary/preview exists.
- [ ] Tests cover preset and metadata behavior.

#### 5.6 Success Criteria

- Editing tree type in Campaign Builder changes the in-game tree type.
- Editing grass density changes in-game coverage.
- Dead tree preset creates a dead tree without foliage.
- Tall grass preset creates taller/denser grass.
- Users can tell from the SDK which vegetation settings are active on a tile.

---

### Phase 6: Add Vegetation LOD, Quality Settings, and Performance Budgets

This phase ensures the improved visuals remain performant. It should happen after the visual pipeline is correct enough to be worth optimizing.

#### 6.1 Foundation Work

Extend existing quality resources rather than inventing unrelated settings.

Relevant files:

| File                                           | Current Role                               |
| ---------------------------------------------- | ------------------------------------------ |
| `src/game/resources/grass_quality_settings.rs` | Grass density scaling by performance level |
| `src/game/resources/performance.rs`            | Existing performance and LOD resources     |
| `src/game/systems/advanced_grass.rs`           | Grass culling and LOD systems              |
| `src/game/systems/procedural_meshes.rs`        | Mesh cache and tree material cache         |
| `src/game/systems/advanced_trees.rs`           | Mesh generation complexity                 |

Recommended additions:

| Setting                              | Purpose                                 |
| ------------------------------------ | --------------------------------------- |
| `VegetationQualityLevel`             | Low/Medium/High vegetation-wide quality |
| `tree_lod_distance_1`                | Switch to simplified tree mesh          |
| `tree_lod_distance_2`                | Switch to billboard/impostor or hide    |
| `grass_lod_distance`                 | Existing concept, tuned for new clumps  |
| `vegetation_cull_distance`           | Global maximum vegetation draw distance |
| `max_tree_mesh_variants_per_species` | Prevent mesh cache explosion            |
| `max_grass_material_variants`        | Prevent material cache explosion        |

#### 6.2 Tree LOD

Required tree LOD behavior:

| LOD    | Description                                       |
| ------ | ------------------------------------------------- |
| LOD0   | Full branch and leaf meshes                       |
| LOD1   | Reduced branch sections/segments and fewer leaves |
| LOD2   | Billboard/impostor or simplified silhouette       |
| Culled | Hidden outside configured distance                |

Tree LOD should not change gameplay. It is visual-only.

#### 6.3 Grass LOD

Required grass LOD behavior:

| LOD    | Description                          |
| ------ | ------------------------------------ |
| Near   | Full clumps                          |
| Mid    | Fewer clumps or lower segment blades |
| Far    | Low-card patches                     |
| Culled | Hidden                               |

#### 6.4 Performance Budgets

Initial target budgets:

| Metric                       | Target                                        |
| ---------------------------- | --------------------------------------------- |
| 50 visible improved trees    | Maintain at least 30 FPS on reference machine |
| Grass-heavy town/forest view | Avoid one mesh/material per blade             |
| Mesh generation stall        | Avoid noticeable frame hitch during map spawn |
| Cached tree variants         | Bounded per species and quality level         |
| Texture materials            | Reused by species/tint bucket                 |

#### 6.5 Testing Requirements

Required tests:

| File                                                                | Test Requirement                                                                           |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `src/game/systems/advanced_trees.rs`                                | LOD settings reduce mesh vertex/index counts                                               |
| `src/game/systems/procedural_meshes.rs`                             | Tree cache count remains bounded for repeated map spawns                                   |
| `src/game/systems/advanced_grass.rs`                                | Grass LOD reduces visible/rendered blades or clumps                                        |
| `src/game/resources/grass_quality_settings.rs` or new resource file | Quality settings map to deterministic density/LOD values                                   |
| `src/game/systems/map.rs`                                           | Map spawn does not create unbounded vegetation asset variants for repeated identical tiles |

#### 6.6 Deliverables

- [ ] Vegetation quality setting exists.
- [ ] Tree LOD meshes or impostors exist.
- [ ] Grass LOD works with new clumps.
- [ ] Mesh/material cache growth is bounded.
- [ ] Tests cover LOD and cache behavior.
- [ ] Performance targets are documented.

#### 6.7 Success Criteria

- Improved vegetation remains playable in dense forest scenes.
- Low quality mode preserves silhouette variety while reducing geometry.
- High quality mode improves visuals without unbounded asset creation.
- LOD changes do not alter map data or gameplay state.

---

### Phase 7: Documentation, Fixtures, and Visual Validation

This phase brings docs and fixtures in line with reality and creates a repeatable visual validation route.

#### 7.1 Documentation Updates

Required documentation updates:

| File                                                                | Required Update                                                                                      |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `docs/explanation/procedural_mesh_visual_quality.md`                | Replace outdated “passes” claims with the new vegetation pipeline expectations and known limitations |
| `docs/explanation/implementations.md`                               | Add summary of completed vegetation work after implementation                                        |
| `docs/explanation/vegetation_visual_quality_implementation_plan.md` | Update deliverables as phases complete                                                               |
| `docs/reference/architecture.md`                                    | Only update if new architecture-level data structures or module placement are introduced             |

#### 7.2 Test Fixtures

All tests must use stable fixtures, not `campaigns/tutorial`.

Required fixture policy:

| Requirement                    | Path                                                                         |
| ------------------------------ | ---------------------------------------------------------------------------- |
| Loader-based campaign tests    | Use `CampaignLoader::new("data")` with id `"test_campaign"`                  |
| Path-based test campaign tests | Use `data/test_campaign`                                                     |
| Any new test maps              | Add under `data/test_campaign/data/maps/`                                    |
| Any test texture assumptions   | Use repo-managed assets or add test-safe assets outside `campaigns/tutorial` |

Do not add tests that reference `campaigns/tutorial`.

#### 7.3 Visual Validation Maps

Add or update stable test-campaign maps if needed.

Recommended validation scenes:

| Scene                         | Purpose                                       |
| ----------------------------- | --------------------------------------------- |
| Oak/Pine/Birch comparison row | Verify species distinction                    |
| Palm oasis tile               | Verify palm silhouette                        |
| Dead forest tile              | Verify bare gnarled trees                     |
| Willow water edge             | Verify drooping foliage                       |
| Shrub undergrowth tile        | Verify shrubs do not clip trunks              |
| Grass density strip           | Verify None/Low/Medium/High/VeryHigh coverage |
| SDK metadata stress tile      | Verify height/scale/tint/foliage edits        |

#### 7.4 Testing Requirements

Required validation commands after implementation:

| Order | Command                                                    |
| ----- | ---------------------------------------------------------- |
| 1     | `cargo fmt --all`                                          |
| 2     | `cargo check --all-targets --all-features`                 |
| 3     | `cargo clippy --all-targets --all-features -- -D warnings` |
| 4     | `cargo nextest run --all-features`                         |

Required checks:

| Check                   | Requirement                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------- |
| Architecture compliance | Domain structures match `docs/reference/architecture.md` unless intentionally updated |
| Test fixture rule       | No new tests reference `campaigns/tutorial`                                           |
| Asset rule              | Game data remains RON; no JSON/YAML game data                                         |
| Documentation rule      | `docs/explanation/implementations.md` updated                                         |
| SDK egui rule           | Any touched multi-column egui screen follows project layout rules                     |

#### 7.5 Deliverables

- [ ] Visual quality guide reflects the new real implementation.
- [ ] Implementation summary is updated.
- [ ] Stable test-campaign vegetation fixtures exist if needed.
- [ ] Visual validation scenarios cover all tree and grass types.
- [ ] All required quality gates pass.

#### 7.6 Success Criteria

- Documentation no longer claims broken visuals are acceptable.
- A future engineer can validate vegetation quality without using live tutorial campaign data.
- The implementation has automated coverage for the original user complaints.
- Quality gates pass with zero warnings and zero errors.

---

## Recommended Implementation Order

| Order | Phase   | Reason                                                                         |
| ----- | ------- | ------------------------------------------------------------------------------ |
| 1     | Phase 1 | Removes current broken artifacts immediately and proves metadata/texture paths |
| 2     | Phase 2 | Delivers the biggest visible tree-quality improvement                          |
| 3     | Phase 4 | Prevents new higher-quality vegetation from clipping itself                    |
| 4     | Phase 3 | Improves grass once placement rules are known                                  |
| 5     | Phase 5 | Makes SDK authoring trustworthy after runtime behavior is real                 |
| 6     | Phase 6 | Optimizes the improved visuals after quality targets are met                   |
| 7     | Phase 7 | Finalizes docs, fixtures, and validation                                       |

---

## Explicit Non-Goals

| Non-Goal                                                               | Reason                                                                                 |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Replacing Antares tile maps with full `bevy_terrain` terrain rendering | Antares is tile-based; the terrain plugin solves a different large-heightfield problem |
| Adding incompatible Bevy plugins directly without version verification | The project uses Bevy `0.17`; dependencies must match                                  |
| Moving Bevy render types into `src/domain`                             | Architecture requires domain logic to stay render-independent                          |
| Replacing RON map data with JSON/YAML                                  | Project rules require RON for game data                                                |
| Adding physics collision for grass/foliage in this plan                | Visual quality is the immediate problem                                                |
| Implementing seasonal growth/weather systems now                       | These are future enhancements after baseline vegetation looks good                     |

---

## Risk Register

| Risk                                              | Impact                                    | Mitigation                                                                               |
| ------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------- |
| Direct dependency version mismatch                | Build failures or Bevy duplicate versions | Prefer internal port of concepts; verify dependency versions before adding               |
| Mesh cache explosion from per-tile variation      | Memory and load-time problems             | Bucket variations and cap variants per species                                           |
| Alpha-masked foliage sorting artifacts            | Leaves look harsh or flicker              | Use masked alpha, double-sided materials, and avoid transparent blending unless required |
| Texture path confusion under campaign asset roots | Bark/grass textures fail silently         | Add tests/logging and ensure campaign assets mirror required paths                       |
| SDK controls still not saving metadata            | User sees no runtime change               | Add round-trip tests for metadata and presets                                            |
| Over-optimization before visual correctness       | Time spent preserving bad visuals         | Follow phase order: correctness and species distinction first                            |
| Dense grass hurts frame rate                      | Playability regression                    | Replace per-blade assets with reusable meshes/materials before increasing density        |

---

## Final Acceptance Checklist

- [ ] Bark textures visibly apply to trunks.
- [ ] Oak, Pine, Palm, Dead, Willow, Birch, and Shrub are visually distinguishable.
- [ ] Dead trees have no foliage.
- [ ] Palm trees have a tall trunk and crown fronds.
- [ ] Pine trees have conical evergreen silhouettes.
- [ ] Willow trees droop.
- [ ] Shrubs do not clip tree trunks.
- [ ] Grass forms believable clumps.
- [ ] Grass density settings visibly change coverage.
- [ ] SDK vegetation edits affect runtime output.
- [ ] No new tests reference `campaigns/tutorial`.
- [ ] `docs/explanation/implementations.md` is updated after implementation.
- [ ] `cargo fmt --all` passes.
- [ ] `cargo check --all-targets --all-features` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo nextest run --all-features` passes.
