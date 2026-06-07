# Vegetation Visual Improvement Plan

<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Antares Contributors -->

## Overview

Improve the visual quality of procedural trees and grass using techniques drawn
from [`bevy_procedural_tree`](https://github.com/Affinator/bevy_procedural_tree)
(Bevy 0.17 compatible) and
[`bevy_procedural_grass`](https://github.com/jadedbay/bevy_procedural_grass).
The plan is ordered from lowest-effort highest-impact to highest-effort
highest-impact so each phase delivers visible improvement independently.

### Key Decisions

| Question | Decision |
|---|---|
| Bark normal map source | **Option A** — generate `bark_normal.png` from `bark.png` offline using a Sobel/emboss filter |
| Foliage texture path resolution | **Campaign-relative** — same mechanism as landscape meshes; `assets/textures/trees/foliage_oak.png` resolves from the active campaign root |
| Wind system configuration | **Per campaign, in `data/wind.ron`** — `wind_system` field accepts `None`, `Sine`, or `Perlin`; both Sine and Perlin are implemented in WGSL |

### Relevant Source Files

| File | Role |
|---|---|
| `src/game/systems/advanced_trees.rs` | Branch graph, leaf geometry generation, LOD switching |
| `src/game/systems/procedural_meshes.rs` | Material creation, tree entity spawning |
| `src/game/systems/advanced_grass.rs` | Blade geometry, clump spawning, wind component, chunking |
| `src/domain/campaign_loader.rs` | `GameData` struct; loads campaign RON files |
| `src/domain/world/` | Domain types; new `wind.rs` module added in Phase 5 |
| `assets/shaders/grass.wgsl` | New WGSL shader (Phase 6) |
| `data/wind.ron` (per campaign) | New per-campaign wind configuration (Phase 5) |

---

## Current State Analysis

### Existing Infrastructure

- **Branch graph generator** (`advanced_trees.rs`): Recursive subdivision with
  exponential taper, quaternion force-direction per species, and species-specific
  LOD levels via `TreeLodGroup` / `tree_lod_switching_system`.

- **Leaf geometry** (`advanced_trees.rs`): `append_leaf_card` dispatches to
  per-species functions — `append_lobed_leaf_cluster` (Oak: 4 coplanar quads),
  `append_pine_needle_cluster` (Pine: single triangle), `append_diamond_leaf`
  (Birch), `append_clustered_shrub_leaf` (Shrub), `append_palm_frond`,
  `append_willow_hanging_strip`. All geometry is single-plane (axis-aligned to
  branch direction).

- **Material creation** (`procedural_meshes.rs`): `get_or_create_bark_material`
  loads `assets/textures/trees/bark.png` via `load_texture`. `get_or_create_foliage_material`
  sets `AlphaMode::Opaque` with a species color and `base_color_texture: None` — all
  six `TREE_FOLIAGE_TEXTURE_*` constants and `foliage_texture_path()` are behind
  `#[cfg(test)]` only.

- **Texture loading** (`creature_meshes.rs`): `load_texture(asset_server, path)`
  requires `path` to start with `"assets/"` and delegates to
  `asset_server.load(path)`. The Bevy asset server root is the active campaign
  directory, so `"assets/textures/trees/bark.png"` resolves to
  `campaigns/tutorial/assets/textures/trees/bark.png` at runtime.

- **Grass blade mesh** (`advanced_grass.rs`): `create_curved_grass_card_mesh`
  builds a quadratic Bezier blade (single quad-strip). `GrassColorScheme` has
  one `base_color`; a derived tip color is generated at spawn time.

- **Grass wind** (`advanced_grass.rs`): `GrassWindParams` component
  (strength / frequency / phase) is attached to every blade entity at spawn but
  **no Bevy system reads it**. Wind has never been animated.

- **Campaign data loading** (`campaign_loader.rs`): `GameData` struct holds
  `creatures`, `item_meshes`, `furniture`, `furniture_meshes`, `landscape`,
  `landscape_meshes`, and `levels`. New campaign-level config follows this same
  optional-load pattern.

### Identified Issues

1. **Foliage textures disabled.** Six `foliage_*.png` textures exist in both
   `assets/textures/trees/` and `campaigns/tutorial/assets/textures/trees/` but
   are gated behind `#[cfg(test)]`. Material uses flat opaque species colors.

2. **Single-plane leaf geometry.** All leaf quads are co-planar to the branch
   direction. Canopy nearly disappears when viewed from certain angles.

3. **No bark normal map.** Only `bark.png` (base color) is loaded. No normal map
   exists yet — one must be generated as part of Phase 3.

4. **Grass blades use quadratic Bezier only.** Cubic Bezier gives more natural
   S-curve lean. The two-color gradient misses an AO darkening at the base.

5. **`GrassWindParams` is a dead component.** No system ever reads it. All grass
   is completely static.

6. **No wind configuration in campaign data.** Wind intensity and algorithm are
   not part of any RON file — they cannot be authored per campaign.

7. **`GrassInstanceBatch` is never drawn.** The instancing pipeline is partially
   built but not connected to any render pass.

---

## Implementation Phases

---

### Phase 1: Foliage Texture Restoration

Restore foliage textures with correct alpha masking. All textures exist in the
campaign asset directory; only the loading code and material settings need
updating.

#### 1.1 Un-gate Texture Constants

In [`src/game/systems/procedural_meshes.rs`](../../src/game/systems/procedural_meshes.rs):

- Remove `#[cfg(test)]` from the six `TREE_FOLIAGE_TEXTURE_*` constants (lines
  948–979) and from `TREE_ALPHA_CUTOFF` (line 981).
- Remove `#[cfg(test)]` from `foliage_texture_path()` (line 1116).

#### 1.2 Restore Material Loading

In `get_or_create_foliage_material` (line 381) and
`get_or_create_foliage_material_variant` (line 451):

- Call `creature_meshes::load_texture(asset_server, foliage_texture_path(tree_type))`
  to get a texture handle. The `_asset_server` parameter is already passed but
  ignored — rename to `asset_server` and use it.
- Set `base_color_texture: Some(texture_handle)`.
- Change `alpha_mode: AlphaMode::Opaque` → `alpha_mode: AlphaMode::Mask(0.5)`.
- Keep `double_sided: true`, `cull_mode: None`, and `unlit: true` (already set).

#### 1.3 Campaign-Relative Path Resolution

Foliage texture paths begin with `"assets/"` and resolve via `load_texture`,
which delegates to the Bevy asset server. The asset server root is the active
campaign directory, so `"assets/textures/trees/foliage_oak.png"` resolves to
`campaigns/tutorial/assets/textures/trees/foliage_oak.png` at runtime. No path
adjustment is needed — this is identical to how bark textures and landscape
textures are resolved.

All six species-texture files already exist in both:
- `assets/textures/trees/foliage_{oak,pine,birch,willow,palm,shrub}.png` (root engine copies)
- `campaigns/tutorial/assets/textures/trees/foliage_{oak,pine,birch,willow,palm,shrub}.png` (campaign copies)

#### 1.4 Testing Requirements

- Update tests that assert `alpha_mode == AlphaMode::Opaque` on foliage
  materials (lines 4584 and 4715 in `procedural_meshes.rs`).
- Confirm `cargo nextest run --all-features` — 5188 tests, zero failures.
- Visual: launch tutorial campaign; Oak/Pine/Birch/Willow/Shrub/Palm canopy
  renders with species texture and alpha cutout silhouette, not solid color.

#### 1.5 Deliverables

- [ ] `TREE_FOLIAGE_TEXTURE_*` constants and `foliage_texture_path()` available
  in non-test builds
- [ ] `get_or_create_foliage_material` loads foliage texture and uses
  `AlphaMode::Mask(0.5)`
- [ ] `get_or_create_foliage_material_variant` updated to match
- [ ] All tests pass

#### 1.6 Success Criteria

All six species render alpha-masked foliage using campaign texture assets.
No solid-color canopy blobs remain in the tutorial campaign.

---

### Phase 2: Cross-Pattern Leaf Volume

Add a perpendicular second pass of leaf geometry inside `append_leaf_card` so
canopy looks volumetric from all camera angles.

#### 2.1 Current Geometry Analysis

Each species function builds geometry in a single plane defined by `side` and
`up` vectors:

| Species | Function | Current geometry |
|---|---|---|
| Oak | `append_lobed_leaf_cluster` | 3 diamond quads + 1 base quad |
| Pine | `append_pine_needle_cluster` | 1 triangle |
| Birch | `append_diamond_leaf` | 1 diamond quad |
| Willow | `append_willow_hanging_strip` | vertical strip quads |
| Palm | `append_palm_frond` | frond quads |
| Shrub | `append_clustered_shrub_leaf` | diamond cluster |

#### 2.2 Add Cross-Pattern Helper

In [`src/game/systems/advanced_trees.rs`](../../src/game/systems/advanced_trees.rs),
add `fn append_leaf_card_cross(...)` that:

1. Calls the existing per-species function with `(side, up)` as-is (first pass).
2. Calls it again with `cross_side = direction.normalize_or_zero()` substituted
   for `side`, keeping `up` the same — this rotates the geometry 90° around the
   branch direction axis (second pass).

Replace the per-species dispatch in `append_leaf_card` (line 1358) with a call
to `append_leaf_card_cross` for Oak, Birch, Willow, Palm, and Shrub.

**Pine exception**: `append_pine_needle_cluster` uses a triangle that reads well
from all angles. Skip the second pass for `TreeType::Pine` to avoid overdraw on
dense conifer canopy.

#### 2.3 Leaf Density Compensation

With two passes the polygon budget doubles per leaf card. Reduce per-branch leaf
density by 30–40% for the five species that gain the cross pass. The density
field is in `TreeSpeciesPreset` (line 466 in `advanced_trees.rs`).

#### 2.4 Testing Requirements

- Verify no triangle-winding issues: `double_sided: true` from Phase 1 covers
  both passes without needing reversed indices.
- Run `cargo nextest run --all-features` — zero failures.
- Visual: rotate camera 360° around an Oak tree; canopy density should remain
  roughly consistent at 0°, 45°, and 90° view angles.

#### 2.5 Deliverables

- [ ] `append_leaf_card_cross` added to `advanced_trees.rs`
- [ ] Oak, Birch, Willow, Palm, Shrub use cross-pattern geometry
- [ ] Pine uses single-pass (unchanged)
- [ ] Leaf density reduced to compensate for doubled polygon count
- [ ] All tests pass

#### 2.6 Success Criteria

Canopy silhouette visible and roughly consistent when camera orbits a tree 360°.
No visible frame-time spike on tutorial campaign forest tiles.

---

### Phase 3: Bark Normal Map

Generate `bark_normal.png` using Option A (offline tool from `bark.png`) and
wire it into the bark material for three-dimensional surface shading.

#### 3.1 Asset Generation (Option A)

Generate `assets/textures/trees/bark_normal.png` from the existing
`assets/textures/trees/bark.png` using a Sobel-filter normal-map generator.

**Tooling options (choose one):**

- **ImageMagick** (no new code):
  ```
  convert bark.png \
    -define convolve:scale=! \
    -morphology Convolve Sobel:90 \
    bark_normal.png
  ```
  Then remap the result to a proper RGB normal map using a second pass.

- **`cargo run --bin generate-normal-map`** — add a small binary at
  `src/bin/generate_normal_map.rs` that reads `bark.png` via the `image` crate,
  applies a 3×3 Sobel kernel to each pixel's grayscale value to compute XY
  gradient, packs as RGB normal map (`R=dx, G=dy, B=1.0` normalized), and writes
  `bark_normal.png`. This keeps the process reproducible and in-repo.

After generation, copy the file to the tutorial campaign:
`campaigns/tutorial/assets/textures/trees/bark_normal.png`

The binary approach is recommended because it is deterministic, version-controlled,
and can be re-run if `bark.png` changes.

#### 3.2 Constant and Material Integration

In [`src/game/systems/procedural_meshes.rs`](../../src/game/systems/procedural_meshes.rs):

- Add `const TREE_BARK_NORMAL_TEXTURE: &str = "assets/textures/trees/bark_normal.png";`
  alongside `TREE_BARK_TEXTURE` (line 943).
- In `get_or_create_bark_material` (line 329): load the normal texture via
  `creature_meshes::load_texture(asset_server, TREE_BARK_NORMAL_TEXTURE)` and
  set `normal_map_texture: Some(normal_handle)` on the `StandardMaterial`.
- Set `flip_normal_map_y: false` (DirectX-convention; flip to `true` if banding
  appears on one axis).
- Apply the same change to `get_or_create_bark_material_variant` (line 413).

#### 3.3 Testing Requirements

- Confirm `bark_normal.png` exists in both asset locations before running visual
  tests.
- Visual: trunk and branches should show groove shading under the map's
  directional light at close camera range.

#### 3.4 Deliverables

- [ ] `bark_normal.png` generated and stored at `assets/textures/trees/`
- [ ] `bark_normal.png` copied to `campaigns/tutorial/assets/textures/trees/`
- [ ] `TREE_BARK_NORMAL_TEXTURE` constant added
- [ ] `get_or_create_bark_material` and `get_or_create_bark_material_variant`
  apply the normal map
- [ ] Optional: `src/bin/generate_normal_map.rs` binary added for reproducibility
- [ ] All tests pass

#### 3.5 Success Criteria

Bark surface shows visible groove shading under the map directional light.
Normal texture loads without asset server warnings.

---

### Phase 4: Cubic Bezier Grass Blades + Three-Color Gradient

Upgrade grass blade geometry from quadratic to cubic Bezier and replace the
two-stop color gradient with a three-stop AO base / mid-green / tip highlight.

#### 4.1 Cubic Bezier Blade Geometry

In [`src/game/systems/advanced_grass.rs`](../../src/game/systems/advanced_grass.rs),
`create_curved_grass_card_mesh` (line 886):

Replace the current quadratic Bezier (three control points) with a cubic (four
control points):

| Point | Position |
|---|---|
| `p0` | `(0, 0)` — base, anchored |
| `p1` | `(tilt * 0.25, height * 0.33)` — first control: lower lean |
| `p2` | `(tilt * 0.75 + curve * 0.5, height * 0.66)` — second control: mid lean |
| `p3` | `(curve, height)` — tip |

Cubic formula: `B(t) = (1-t)³p0 + 3(1-t)²t·p1 + 3(1-t)t²·p2 + t³p3`

Replace the current `coeff0 / coeff1 / coeff2` variables with their cubic
equivalents. `segment_count` and `GrassMeshKey` are unchanged — the math is
internal only.

#### 4.2 Three-Color Gradient

Extend `GrassColorScheme` (line 644) with two additional fields:

```
ao_color: Color       // dark base AO, default: srgb(0.08, 0.12, 0.06)
tip_color: Color      // tip highlight, default: srgb(0.72, 0.82, 0.45)
```

Rename existing `base_color` → `mid_color` (mid-blade primary color).

Update `create_curved_grass_card_mesh` signature from `vertex_colors: &[Color; 2]`
to `vertex_colors: &[Color; 3]`.

Interpolate in two height segments:
- `t ∈ [0.0, 0.4]` → lerp `ao_color` → `mid_color` (remap t to 0..1 over 0..0.4)
- `t ∈ [0.4, 1.0]` → lerp `mid_color` → `tip_color` (remap t to 0..1 over 0.4..1.0)

Update all callers of `create_curved_grass_card_mesh` and `spawn_grass_clump`.

#### 4.3 Testing Requirements

- Update `GrassColorScheme::default()` and any `GrassColorScheme { base_color: ... }`
  construction sites to the new field names.
- Run `cargo nextest run --all-features` — zero failures.
- Visual: grass blades show dark base → green mid → lighter tip gradient; blades
  curve with an S-shape rather than a simple arc.

#### 4.4 Deliverables

- [ ] `create_curved_grass_card_mesh` uses cubic Bezier
- [ ] `GrassColorScheme` has `ao_color`, `mid_color`, `tip_color`
- [ ] Three-stop gradient applied along blade height
- [ ] All callers updated; all tests pass

#### 4.5 Success Criteria

Grass blades show a natural S-curve and a visible three-tone gradient.
No regressions in grass placement, exclusion zones, or LOD behavior.

---

### Phase 5: Per-Campaign Wind Configuration

Add a `data/wind.ron` file to the campaign data format and corresponding domain
types so wind algorithm and parameters are campaign-authored rather than
hard-coded.

#### 5.1 Domain Types

Create [`src/domain/world/wind.rs`](../../src/domain/world/wind.rs):

```
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Antares Contributors

pub enum WindSystemKind {
    None,    // no wind animation
    Sine,    // simple sinusoidal sway (shader)
    Perlin,  // spatially coherent Perlin noise wind (shader)
}

pub struct CampaignWindConfig {
    pub wind_system: WindSystemKind,  // default: None

    // Shared parameters
    pub strength: f32,       // world-units sway amplitude, default 0.04
    pub frequency: f32,      // cycles per second, default 0.65
    pub direction: [f32; 2], // XZ normalized, default [1.0, 0.0]

    // Perlin-specific
    pub perlin_scale: f32,   // noise tiling scale in world units, default 100.0
    pub perlin_octaves: u32, // noise octaves, default 4
    pub perlin_seed: u64,    // RNG seed for noise generation, default 0
}
```

Add `CampaignWindConfig` to re-exports in `src/domain/world/mod.rs` and
`src/domain/mod.rs`.

Derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `Default` on both types.
Annotate all fields with `#[serde(default)]` so a minimal RON file with only
`wind_system: Sine` is valid.

#### 5.2 Campaign Loader

In [`src/domain/campaign_loader.rs`](../../src/domain/campaign_loader.rs):

- Add `pub wind: CampaignWindConfig` to `GameData` with `Default` (= `None`
  system).
- Add `fn load_wind_config(&self) -> Result<CampaignWindConfig, CampaignError>`
  that reads `data/wind.ron` relative to `self.campaign_path`. If the file is
  absent, return `CampaignWindConfig::default()` (no error).
- Call `load_wind_config` from the main `load` method and assign to
  `game_data.wind`.

#### 5.3 RON File Format

Example `campaigns/tutorial/data/wind.ron`:

```ron
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Antares Contributors
(
    wind_system: Sine,
    strength: 0.04,
    frequency: 0.65,
    direction: (1.0, 0.0),
)
```

Example with Perlin:

```ron
(
    wind_system: Perlin,
    strength: 0.06,
    frequency: 0.5,
    direction: (0.71, 0.71),
    perlin_scale: 80.0,
    perlin_octaves: 4,
    perlin_seed: 12345,
)
```

#### 5.4 Bevy Resource

Add `WindConfig` to [`src/game/resources.rs`](../../src/game/resources.rs) as a
`Resource` that mirrors `CampaignWindConfig`:

```
#[derive(Resource, Clone, Debug, Default)]
pub struct WindConfig(pub CampaignWindConfig);
```

Insert `WindConfig` into the Bevy world during game-content load alongside
`GameContent`, reading from `content.0.wind`.

#### 5.5 Test Fixture

Add `data/test_campaign/data/wind.ron` with `wind_system: None` so existing
loader tests continue to pass.

#### 5.6 Testing Requirements

- Unit tests in `wind.rs`: round-trip RON serialization for `None`, `Sine`, and
  `Perlin` variants; missing file returns `default()`.
- Confirm `cargo nextest run --all-features` — zero failures.

#### 5.7 Deliverables

- [ ] `src/domain/world/wind.rs` with `WindSystemKind` and `CampaignWindConfig`
- [ ] `GameData.wind` field added
- [ ] `CampaignLoader::load_wind_config` implemented
- [ ] `WindConfig` Bevy resource inserted at game load
- [ ] `campaigns/tutorial/data/wind.ron` with `wind_system: Sine`
- [ ] `data/test_campaign/data/wind.ron` with `wind_system: None`
- [ ] RON round-trip tests pass; all other tests pass

#### 5.8 Success Criteria

`GameContent` exposes wind configuration at runtime. Changing
`wind_system` in `data/wind.ron` and reloading the campaign changes which shader
path runs. Missing `wind.ron` silently disables wind.

---

### Phase 6: WGSL Grass Wind Shader (Sine + Perlin)

Implement a custom WGSL vertex shader for grass that reads `WindConfig` and
branches on `wind_system` to animate blades either with a simple sine wave or
with a spatially coherent Perlin noise texture. This replaces the dead
`GrassWindParams` CPU approach.

#### 6.1 Shader Asset

Create [`assets/shaders/grass.wgsl`](../../assets/shaders/grass.wgsl):

**Imports:**
```wgsl
#import bevy_pbr::forward_io::VertexOutput
#import bevy_render::globals::GlobalsUniform
```

**Wind uniform bind group (group 2):**
```wgsl
struct GrassWindUniform {
    strength:       f32,
    frequency:      f32,
    direction:      vec2<f32>,
    wind_system:    u32,   // 0=None, 1=Sine, 2=Perlin
    perlin_scale:   f32,
    _pad:           vec2<f32>,
}
@group(2) @binding(0) var<uniform> wind: GrassWindUniform;
@group(2) @binding(1) var wind_noise: texture_2d<f32>;
@group(2) @binding(2) var wind_sampler: sampler;
```

**Vertex stage — Sine path (`wind_system == 1`):**
```wgsl
let t = globals.time * wind.frequency + in.position.x * 0.17 + in.position.z * 0.13;
let sway = wind.strength * in.uv.y * in.uv.y * sin(t);
out.world_position.x += sway * wind.direction.x;
out.world_position.z += sway * wind.direction.y;
```

`in.uv.y` encodes blade height 0→1; squaring it ensures the base stays fixed
while the tip sways with maximum amplitude.

**Vertex stage — Perlin path (`wind_system == 2`):**
```wgsl
let scrolled_uv = in.world_position.xz / wind.perlin_scale
                + wind.direction * globals.time * wind.frequency;
let noise = textureSample(wind_noise, wind_sampler, scrolled_uv).r;
let sway = wind.strength * in.uv.y * in.uv.y * (noise * 2.0 - 1.0);
out.world_position.x += sway * wind.direction.x;
out.world_position.z += sway * wind.direction.y;
```

**Fragment stage:** Delegate to standard PBR lighting using the existing vertex
color gradient from Phase 4. No custom lighting logic required.

#### 6.2 Perlin Noise Texture Generation

When `WindConfig.wind_system == Perlin`, generate a tiling noise texture at
campaign load time:

- Add `fn generate_wind_noise_texture(config: &CampaignWindConfig) -> Image` to
  `src/game/systems/advanced_grass.rs`.
- Generate a 512×512 RGBA8 `Image` using the `noise` crate
  (`noise::Perlin::new(seed)` with `noise::NoiseFn::get`).
- Tile the noise using wrapping coordinates so the texture tiles seamlessly.
- Apply `octaves` passes of fBm (fractional Brownian motion) for richer
  variation.
- Pack the float noise value into the R channel; set G/B/A to 255 (unused).
- Register the image as a Bevy asset (`Assets<Image>::add`) and store the handle
  in a `WindNoiseTexture(Handle<Image>)` resource.
- When `wind_system == None` or `Sine`, skip generation; bind a 1×1 white
  placeholder texture.

Add `noise = "0.9"` to `Cargo.toml` dependencies (or verify it is already
present).

#### 6.3 Custom Grass Material

Add `GrassMaterial` as
`bevy::pbr::ExtendedMaterial<StandardMaterial, GrassWindExtension>` in
`src/game/systems/advanced_grass.rs`:

```
struct GrassWindExtension {
    strength:    f32,
    frequency:   f32,
    direction:   Vec2,
    wind_system: u32,
    perlin_scale: f32,
}
impl MaterialExtension for GrassWindExtension { ... }
```

Register `MaterialPlugin::<ExtendedMaterial<StandardMaterial, GrassWindExtension>>`
in `MapRenderingPlugin`.

Replace all `Handle<StandardMaterial>` grass handles in `spawn_grass_clump`
with `Handle<ExtendedMaterial<StandardMaterial, GrassWindExtension>>` handles.

The extension values are read from `WindConfig` at spawn time and kept constant
per map (re-spawn the map to change wind parameters).

#### 6.4 Remove Dead CPU Wind Infrastructure

- Remove `GrassWindParams` component from `advanced_grass.rs`.
- Remove the `phase` field from `spawn_grass_clump` (no longer needed).
- The `GrassBaseTransform` component from Phase 5 planning is not needed (shader
  wind never was a CPU system).

#### 6.5 Testing Requirements

- WGSL shader must load without Bevy shader compilation errors at startup for all
  three `wind_system` values.
- `WindConfig { wind_system: None }` must produce no visible blade movement.
- `WindConfig { wind_system: Sine }` must produce visible blade sway.
- `WindConfig { wind_system: Perlin }` must produce spatially varied gusts.
- Run `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- Run `cargo nextest run --all-features` — zero failures.

#### 6.6 Deliverables

- [ ] `assets/shaders/grass.wgsl` with `None` / `Sine` / `Perlin` vertex paths
- [ ] Perlin noise texture generated at campaign load when `wind_system: Perlin`
- [ ] `GrassMaterial` (`ExtendedMaterial`) registered and used by all grass
- [ ] Wind uniforms populated from `WindConfig` Bevy resource at spawn time
- [ ] `GrassWindParams` component removed
- [ ] `noise` crate added to `Cargo.toml` if not already present
- [ ] All four quality gates pass (`fmt`, `check`, `clippy`, `nextest`)

#### 6.7 Success Criteria

Grass animates via GPU vertex shader with zero CPU work per frame. Tutorial
campaign uses `wind_system: Sine` and shows visible sway. A campaign with
`wind_system: Perlin` shows spatially coherent gust variation. A campaign with
no `wind.ron` or `wind_system: None` has static grass.

---

### Phase 7: GPU Instancing for Grass

Connect the existing `GrassInstanceBatch` infrastructure to an actual render
pass so all blades within a spatial chunk share a single indexed draw call,
dramatically reducing entity count and CPU overhead on grass-dense maps.

#### 7.1 Current State

`build_grass_instance_batches_system` (line 1657 in `advanced_grass.rs`) already
groups blade entities by `(mesh, material, map_id)` into `GrassInstanceBatch`
entities containing `Vec<InstanceData>`. These entities are created but no render
command reads or draws them.

#### 7.2 Render Pipeline

Add a `GrassInstancedPipeline` that specializes `SpecializedRenderPipeline` with:

- **Vertex buffer 0** (`VertexStepMode::Vertex`): per-vertex blade geometry —
  position (`Vec3`), normal (`Vec3`), UV (`Vec2`), vertex color (`Vec4`)
- **Vertex buffer 1** (`VertexStepMode::Instance`): per-instance data —
  world position (`Vec3`), surface normal (`Vec3`), phase offset (`f32`)

During `PrepareAssets`, upload `GrassInstanceBatch.instances` to a GPU
`Buffer` with `BufferUsages::VERTEX`.

In the `Opaque3d` render phase, issue:
```
draw_indexed(0..index_count, 0, 0..instance_count)
```
once per batch.

This follows the same pattern as Bevy's built-in instanced mesh rendering and
`bevy_procedural_grass`'s `DrawGrassInstanced` / `GrassPipeline`.

#### 7.3 Remove Per-Blade Entities

Once instancing is confirmed working visually:

- Remove the per-blade `Mesh3d` / `MeshMaterial3d` entity spawn from
  `spawn_grass_clump`.
- Remove now-unused `GrassBlade`, `GrassClump`, `GrassBladeInstance` component
  queries from LOD and culling systems.
- Manage blade lifecycle at the chunk level via `GrassChunk` entities only.
- Update `grass_chunk_culling_system` to operate on chunk-level visibility.

#### 7.4 Testing Requirements

- Verify `GrassInstanceBatch` entities carry the expected instance count after
  map spawn.
- Verify grass renders with visual parity to Phase 6 output.
- Profile frame time on a 20×20 all-Forest map: target ≥30% reduction in render
  thread grass cost vs. per-entity approach.

#### 7.5 Deliverables

- [ ] `GrassInstancedPipeline` / `DrawGrassInstanced` render command implemented
- [ ] `GrassInstanceBatch` instance data uploaded to GPU vertex buffer
- [ ] Per-blade entity spawn removed; lifecycle managed at chunk level
- [ ] `grass_chunk_culling_system` updated to chunk-level visibility
- [ ] All four quality gates pass

#### 7.6 Success Criteria

Dense grass maps spawn in under 2 seconds. Bevy entity count for grass is
reduced by ≥90% on a 20×20 all-Forest map. Frame time is equal to or better
than Phase 6 baseline.

---

## Implementation Order Summary

| Phase | Scope | Effort | Visual / Perf Impact |
|---|---|---|---|
| 1 — Foliage textures | Trees | 1–2 hrs | High visual |
| 2 — Cross-pattern leaves | Trees | 4–8 hrs | Medium visual |
| 3 — Bark normal map + asset gen | Trees | 2–4 hrs + tooling | Medium visual |
| 4 — Cubic Bezier + 3-color grass | Grass | 1 day | Medium visual |
| 5 — Per-campaign wind config | Domain + loader | 1 day | Enabler for 6 |
| 6 — WGSL wind shader (Sine + Perlin) | Grass shader | 2–3 days | High visual |
| 7 — GPU instancing | Grass render pipeline | 3–4 days | High perf |

Phases 1–3 (trees) are independent of Phases 4–7 (grass) and can be done in
any order or in parallel. Phase 6 depends on Phase 5. Phase 7 depends on Phase 6.
