# Procedural Mesh Visual Quality Guide

## Overview

This guide describes the current visual-quality expectations, runtime behavior, performance budgets, and validation process for Antares procedural vegetation and environmental mesh rendering.

It reflects the implemented vegetation visual-quality pipeline through Phase 7:

- Branch and bark geometry are separate from leaf/frond geometry.
- Tree species use render-layer presets for distinct silhouettes.
- Grass uses deterministic clumped card meshes with shared mesh/material caches.
- Vegetation placement is deterministic and avoids obvious tree/shrub/grass overlap.
- Campaign Builder vegetation metadata maps to runtime-rendered fields.
- Tree and grass LOD are active and controlled by vegetation-wide quality settings.
- Stable validation uses `data/test_campaign`, not `campaigns/tutorial`.

This document is an explanation and QA reference. Game content remains in RON files, and runtime rendering remains in the Bevy game layer.

---

## Current Vegetation Pipeline

### Runtime Source of Truth

Vegetation is authored through map tile visual metadata and rendered by game-layer systems.

Relevant source areas:

- `src/domain/world/types.rs`
  - `TileVisualMetadata`
  - `TreeType`
  - `GrassDensity`
  - `GrassBladeConfig`
- `src/game/systems/advanced_trees.rs`
  - species presets
  - branch graphs
  - branch/leaf mesh generation
  - tree LOD selection and visibility switching
- `src/game/systems/procedural_meshes.rs`
  - tree/shrub spawning
  - mesh/material cache reuse
  - quality-aware tree spawn path
- `src/game/systems/advanced_grass.rs`
  - grass clumps
  - reusable mesh/material variants
  - grass LOD and culling
- `src/game/systems/vegetation_placement.rs`
  - deterministic tree/shrub anchors
  - grass exclusion zones
- `src/game/systems/map.rs`
  - tile-to-runtime vegetation spawning
- `sdk/campaign_builder/src/map_editor.rs`
  - Campaign Builder vegetation authoring controls and presets

### Visual Metadata Fields

The vegetation renderer consumes these `TileVisualMetadata` fields:

| Field                                | Runtime Effect                                                                       |
| ------------------------------------ | ------------------------------------------------------------------------------------ |
| `height`                             | Vertically scales tree/shrub structure and grass blade height baseline               |
| `width_x`                            | Expands vegetation footprint on the X axis for placement/exclusion                   |
| `width_z`                            | Expands vegetation footprint on the Z axis for placement/exclusion                   |
| `color_tint`                         | Tints bark, foliage, grass, and terrain materials through bucketed material variants |
| `scale`                              | Scales tree/shrub/grass visuals and vegetation footprints                            |
| `y_offset`                           | Moves visual vegetation parents vertically                                           |
| `rotation_y`                         | Rotates tree/shrub visual parents and placement offsets                              |
| `tree_type`                          | Selects Oak, Pine, Birch, Willow, Dead, Shrub, or Palm render preset                 |
| `foliage_density`                    | Scales tree leaves/fronds, shrub coverage, and grass coverage                        |
| `grass_density`                      | Selects authored grass coverage before quality scaling                               |
| `grass_blade_config.length`          | Changes blade/clump height                                                           |
| `grass_blade_config.width`           | Changes blade/clump width                                                            |
| `grass_blade_config.tilt`            | Changes blade lean                                                                   |
| `grass_blade_config.curve`           | Changes blade curvature                                                              |
| `grass_blade_config.color_variation` | Changes grass material color variation bucket                                        |

---

## Tree Visual Quality Targets

### Species Silhouettes

Each tree species should be identifiable from silhouette and material treatment.

| Tree Type | Expected Appearance                                                           |
| --------- | ----------------------------------------------------------------------------- |
| Oak       | Broad canopy, thick trunk, spreading lateral branches, dense deciduous leaves |
| Pine      | Tall conical evergreen silhouette, narrow trunk, radial branch whorls         |
| Birch     | Slender pale trunk, lighter foliage, more open canopy than Oak                |
| Willow    | Drooping branch structure and curtain-like foliage                            |
| Dead      | Twisted bare branches, grey-brown bark, no leaf/frond mesh                    |
| Shrub     | Low multi-stem bush, broad low footprint, dense perimeter foliage             |
| Palm      | Tall slender trunk with crown-only radial fronds                              |

### Required Tree Behavior

- Branch/trunk meshes must not contain foliage sphere geometry.
- Branch meshes must include UV coordinates suitable for bark texture mapping.
- Leaf/frond meshes are separate from branch meshes.
- Bark and foliage use separate materials.
- Foliage materials use alpha masking and double-sided rendering.
- Dead trees must not spawn a leaf/frond mesh.
- `foliage_density = Some(0.0)` suppresses leaves for non-dead species.
- `height`, `scale`, `rotation_y`, and `color_tint` affect the whole spawned tree consistently.
- Tree LOD is visual-only and must not alter domain map data or gameplay state.

### Tree LOD Behavior

Tree LOD is controlled by `VegetationQualitySettings`.

| LOD    | Runtime Behavior                                                         |
| ------ | ------------------------------------------------------------------------ |
| LOD0   | Full branch and leaf/frond geometry                                      |
| LOD1   | Reduced branch recursion/segments and fewer leaves/fronds                |
| LOD2   | Simplified billboard/impostor silhouette with no separate leaves         |
| Culled | Tree LOD children hidden outside the configured vegetation cull distance |

Tree LOD components:

- `TreeLodGroup`
- `TreeLodVisibility`
- `TreeLodLevel`
- `tree_lod_switching_system`

The map spawn path creates LOD0, LOD1, and LOD2 child meshes up front. The LOD switching system shows only the matching child mesh group for the active camera-distance bucket.

---

## Grass Visual Quality Targets

### Grass Representation

Grass should read as natural clumps/patches, not isolated spikes or one-off blade entities.

Current grass rendering uses:

- `GrassPatch` parent entities
- `GrassClump` renderable children
- 2–4 crossed curved cards per clump
- shared mesh variants keyed by quality and blade configuration bucket
- shared material variants keyed by tint, color variation, alpha mask, and material budget
- deterministic tile-seeded placement
- grass exclusion zones for trunks and shrubs

### Grass Density Expectations

| Density    | Expected Runtime Result                   |
| ---------- | ----------------------------------------- |
| `None`     | No clumps spawned for the tile            |
| `Low`      | Sparse tufts with visible ground          |
| `Medium`   | Natural meadow coverage                   |
| `High`     | Dense coverage with strong clump presence |
| `VeryHigh` | Overgrown coverage with high clump count  |

`foliage_density` multiplies authored grass coverage and is clamped to prevent unbounded clump counts.

### Grass LOD Behavior

Grass LOD is active and visual-only.

| Tier   | Runtime Behavior                                           |
| ------ | ---------------------------------------------------------- |
| Near   | All clumps remain visible                                  |
| Mid    | Every other clump remains visible                          |
| Far    | Every fourth clump remains visible as reduced far coverage |
| Culled | All grass children hidden beyond cull distance             |

Grass LOD components and functions:

- `GrassLodTier`
- `GrassRenderConfig`
- `select_grass_lod_tier`
- `grass_lod_system`
- `grass_distance_culling_system`

The Far tier currently reduces visible clumps. It does not yet swap to a separate far-only low-card mesh. That is an acceptable known limitation as long as the visible clump count is reduced and cache growth remains bounded.

---

## Vegetation Placement Quality Targets

Vegetation placement is deterministic per map/tile and designed to avoid obvious intra-tile clipping.

### Placement Rules

| Object               | Rule                                                                                   |
| -------------------- | -------------------------------------------------------------------------------------- |
| Main tree            | Uses a deterministic tile-centered anchor unless future metadata adds explicit offsets |
| Shrubs on tree tile  | Placed outside trunk exclusion radius plus safety margin                               |
| Shrub-only tile      | May use the center anchor or multiple low offsets                                      |
| Grass near trees     | Clumps avoid trunk exclusion zones                                                     |
| Grass near shrubs    | Clumps avoid shrub stem exclusion zones                                                |
| Blocked/walled tiles | Procedural vegetation suppressed when the tile is blocked or has a wall                |
| Props/signs/doors    | Known blocking data suppresses vegetation where available                              |

### Metadata Placement Effects

| Field             | Placement Effect                                     |
| ----------------- | ---------------------------------------------------- |
| `scale`           | Expands tree/shrub/grass footprint                   |
| `width_x`         | Overrides/expands footprint width                    |
| `width_z`         | Overrides/expands footprint depth                    |
| `y_offset`        | Applies to visual parent when relevant               |
| `rotation_y`      | Rotates visual and asymmetric placement offsets      |
| `foliage_density` | Controls foliage/coverage and understory shrub count |

---

## Vegetation Quality Settings and Budgets

### Quality Levels

`VegetationQualityLevel` provides Low, Medium, and High runtime quality presets.

`VegetationQualitySettings` controls:

| Setting                              | Purpose                                     |
| ------------------------------------ | ------------------------------------------- |
| `quality_level`                      | Low/Medium/High vegetation-wide quality     |
| `tree_lod_distance_1`                | Distance for tree LOD0 to LOD1 switch       |
| `tree_lod_distance_2`                | Distance for tree LOD1 to LOD2 switch       |
| `grass_lod_distance`                 | Distance for grass Near to Mid/Far behavior |
| `vegetation_cull_distance`           | Global maximum vegetation draw distance     |
| `max_tree_mesh_variants_per_species` | Caps deterministic tree mesh variants       |
| `max_grass_material_variants`        | Caps grass material bucket growth           |

### Performance Budget Targets

These are targets for validation and tuning, not guaranteed benchmark measurements for every developer machine.

| Metric                       | Target                                            |
| ---------------------------- | ------------------------------------------------- |
| 50 visible improved trees    | Maintain at least 30 FPS on the reference machine |
| Grass-heavy town/forest view | Avoid one mesh/material per blade                 |
| Mesh generation stall        | Avoid noticeable frame hitch during map spawn     |
| Cached tree variants         | Bounded per species and quality level             |
| Texture/material variants    | Reused by species/tint/material bucket            |

### Cache Growth Expectations

Tree cache keys include:

- tree species
- foliage density bucket
- quality/LOD bucket
- bounded variant seed bucket

Grass cache keys include:

- mesh quality
- blade length/width/tilt/curve buckets
- card count bucket
- tint/color variation/material budget buckets

Repeated identical or similar map tiles must reuse existing handles rather than create unbounded assets.

---

## Campaign Builder Authoring Expectations

Campaign Builder vegetation controls should visibly correspond to runtime behavior.

### Preset Expectations

| Preset Type      | Required Runtime Metadata                                    |
| ---------------- | ------------------------------------------------------------ |
| Dead tree        | `tree_type = Dead`, dead tint, `foliage_density = 0.0`       |
| Small/large tree | explicit tree species, height/scale, foliage density         |
| Shrub            | `tree_type = Shrub`, low height/scale, shrub foliage density |
| Short grass      | low density and shorter blade config                         |
| Tall grass       | high density and taller blade config                         |
| Dried grass      | tan tint and low color variation                             |

### UI Feedback Expectations

The Campaign Builder should provide:

- vegetation summary labels
- runtime-effect hints
- visible tint preview in the map grid
- reset vegetation action
- preset application to the current tile or selection
- save/reload preservation of vegetation metadata

---

## Stable Visual Validation Fixture

Visual validation must use stable repository-managed fixture data, not the live tutorial campaign.

Primary fixture:

- `data/test_campaign/data/maps/map_7.ron`

Fixture purpose:

| Scene Area                                   | Purpose                                                                   |
| -------------------------------------------- | ------------------------------------------------------------------------- |
| Row of Oak/Pine/Birch/Willow/Dead/Shrub/Palm | Validate all tree species are represented                                 |
| Grass density strip                          | Validate `None`, `Low`, `Medium`, `High`, and `VeryHigh` coverage         |
| Willow near water                            | Validate drooping water-edge scene                                        |
| Shrub undergrowth tile                       | Validate shrubs do not require a full-size default tree                   |
| Metadata stress tiles                        | Validate height, scale, width, tint, rotation, foliage, and blade configs |

Tests should load this fixture through `data/test_campaign` paths. Do not add tests that read from or depend on `campaigns/tutorial`.

---

## Automated Validation Coverage

Current automated coverage should verify:

### Trees

- Branch meshes include UV coordinates.
- Branch meshes do not include foliage sphere geometry.
- Dead trees generate no leaf/frond mesh.
- `foliage_density = 0.0` suppresses leaf/frond spawning.
- Tree species presets exist for Oak, Pine, Birch, Willow, Dead, Shrub, and Palm.
- Palm leaves/fronds are crown-only.
- Pine bounds are taller than wide.
- Oak is broader relative to height than Pine.
- Same cache key produces stable mesh statistics.
- LOD settings reduce branch/leaf mesh vertex counts.
- Tree cache variant buckets stay bounded.

### Grass

- `GrassDensity::None` spawns no blade/clump children.
- Higher grass density produces higher clump counts.
- `foliage_density` scales grass coverage.
- `grass_blade_config` affects mesh/config buckets.
- Mesh/material handles are reused for similar clumps.
- Grass LOD reduces visible clumps at distance.
- Grass culling hides distant patches.

### Placement

- Same tile produces the same vegetation plan across runs.
- Shrub anchors avoid tree trunk exclusion radius.
- Grass clumps avoid trunk exclusion zones.
- Explicit shrub tiles do not also spawn a full-size default tree.
- Blocked and wall tiles suppress procedural vegetation.
- Placement respects scale and footprint metadata.

### Campaign Builder

- Visual presets set runtime-consumed semantic fields.
- Grass terrain metadata preserves grass fields.
- Forest terrain metadata preserves tree fields.
- Grass blade config survives save/load round-trip.
- Clearing vegetation metadata removes vegetation-specific fields.

### Fixtures

- `data/test_campaign/data/maps/map_7.ron` parses as a map.
- The validation fixture covers every tree species.
- The validation fixture covers every grass density, including `None`.
- The validation fixture contains dead trees with zero foliage.
- The validation fixture contains metadata stress tiles.

---

## Manual QA Checklist

Use the stable test-campaign vegetation validation map for manual QA.

### Tree Species

- [ ] Oak has a broad deciduous canopy.
- [ ] Pine has a tall conical evergreen silhouette.
- [ ] Birch reads as slender and lighter than Oak.
- [ ] Willow has visibly drooping structure.
- [ ] Dead trees have no foliage.
- [ ] Shrub appears low and bushy rather than full-height tree-like.
- [ ] Palm has a tall trunk and crown fronds.

### Grass

- [ ] `None` grass tile has no clumps.
- [ ] Low grass is sparse.
- [ ] Medium grass is natural coverage.
- [ ] High grass is dense.
- [ ] VeryHigh grass is overgrown.
- [ ] Dried/tinted grass is visibly distinct without turning black.
- [ ] Tall grass appears taller than short grass.

### Placement

- [ ] Shrubs do not intersect obvious tree trunks.
- [ ] Grass does not visibly grow through large trunks.
- [ ] Forest tiles look intentionally composed rather than stacked at tile center.
- [ ] Re-entering the same map shows the same vegetation arrangement.

### LOD and Performance

- [ ] Distant trees switch to simpler silhouettes without gameplay change.
- [ ] Distant grass visibly reduces coverage.
- [ ] Vegetation disappears beyond the configured cull distance.
- [ ] Dense forest views remain playable.
- [ ] Camera movement through forest scenes does not show obvious mesh-generation hitches.

### Campaign Builder

- [ ] Changing tree type in the SDK changes the runtime species.
- [ ] Dead tree preset creates a bare dead tree.
- [ ] Editing grass density changes runtime coverage.
- [ ] Tall grass preset creates taller/denser grass.
- [ ] Save/reload preserves authored vegetation metadata.
- [ ] The SDK summary/hints explain active vegetation settings.

---

## Known Limitations

The current implementation intentionally leaves these as future improvements:

1. **No wind animation yet**: Grass and foliage include wind-ready parameters, but visible wind animation is not implemented.
2. **No seasonal simulation**: Color changes are authored through metadata rather than automatic seasons.
3. **No growth stages**: Trees do not transition from saplings to mature trees.
4. **No vegetation collision**: Grass and foliage remain visual-only.
5. **Far grass LOD is reduction-based**: Far grass currently keeps every fourth clump instead of swapping to a distinct far-only patch mesh.
6. **Performance targets need machine-specific measurement**: Automated tests prove cache bounds and LOD reductions; actual FPS should be profiled on the reference machine when performance tuning.

---

## Troubleshooting

### Grass Appears Too Sparse

Check:

1. `grass_density` is not `None`.
2. `foliage_density` is above `0.0`.
3. Vegetation quality is not Low if testing maximum coverage.
4. The camera is not in Mid/Far grass LOD.
5. The tile is not blocked or walled.
6. Exclusion zones are not covering the whole tile.

### Grass Creates Too Many Variants

Check:

1. `max_grass_material_variants` for the current quality level.
2. Tint and color-variation bucket behavior.
3. Whether similar grass tiles share `GrassMaterialKey` buckets.
4. Whether blade configuration values are unnecessarily unique.

### Tree Foliage Appears Missing

Check:

1. `tree_type` is not `Dead`.
2. `foliage_density` is not `Some(0.0)`.
3. Current camera distance is not selecting LOD2.
4. Leaf/frond mesh exists for the species preset.
5. Foliage material uses masked alpha and double-sided rendering.

### Dead Trees Show Foliage

Check:

1. The tile uses `tree_type: Some(Dead)`.
2. `foliage_density` is `Some(0.0)` for authored dead-tree presets.
3. `generate_tree_meshes_for_key` returns `leaves: None` for `Dead`.
4. The spawned tree does not have leaf/frond child entities for `Dead`.

### Shrubs Clip Tree Trunks

Check:

1. The tile uses the deterministic vegetation plan.
2. Shrub anchors are generated outside tree radius plus `SHRUB_TRUNK_SAFETY_MARGIN`.
3. Tree/shrub `scale`, `width_x`, and `width_z` are not authored with extreme values.
4. Grass and shrub exclusion zones are generated from the same plan.

### Distant Tree LOD Does Not Change

Check:

1. `TreeLodGroup` is attached to the tree parent.
2. LOD child meshes have `TreeLodVisibility`.
3. `tree_lod_switching_system` is registered.
4. Camera distance crosses `tree_lod_distance_1` or `tree_lod_distance_2`.
5. `vegetation_cull_distance` is greater than the LOD distances.

---

## Validation Commands

Run these from the repository root after vegetation rendering, fixture, or documentation changes:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

Expected result:

- formatting succeeds
- compile succeeds
- clippy reports zero warnings
- nextest passes all non-skipped tests

---

## References

- Implementation summary: `docs/explanation/implementations.md`
- Implementation plan: `docs/explanation/vegetation_visual_quality_implementation_plan.md`
- Architecture reference: `docs/reference/architecture.md`
- Stable validation fixture: `data/test_campaign/data/maps/map_7.ron`
- Tree renderer: `src/game/systems/advanced_trees.rs`
- Grass renderer: `src/game/systems/advanced_grass.rs`
- Procedural mesh spawner: `src/game/systems/procedural_meshes.rs`
- Vegetation placement: `src/game/systems/vegetation_placement.rs`
- Map renderer: `src/game/systems/map.rs`
- Campaign Builder map editor: `sdk/campaign_builder/src/map_editor.rs`

---

## Status

This guide reflects the implemented vegetation visual-quality pipeline through Phase 7. It intentionally avoids live tutorial-campaign dependencies and uses stable `data/test_campaign` fixtures for validation.
