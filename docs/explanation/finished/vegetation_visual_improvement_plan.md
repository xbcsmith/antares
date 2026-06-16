# Vegetation Visual Improvement Plan


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
| Foliage texture role | **Detail over geometry** — generated leaf-card geometry remains the silhouette source; textures are tiling leaf *detail* maps applied per card. The legacy round alpha-mask silhouette textures are NOT restored (they were deliberately removed because they made every species read as a circular blob — see `procedural_meshes.rs:355-362`). Existing `foliage_*.png` files are replaced under the same filenames by procedurally generated detail textures |
| Foliage lighting | **Lit** (`unlit: false`) — detail texturing plus directional lighting is what produces realistic trees by default; flat unlit color cannot |
| Foliage detail texture source | **Procedurally generated** by an in-repo binary (`src/bin/generate_foliage_textures.rs`) using only existing dependencies — deterministic, version-controlled, re-runnable; no hand-authored art required |
| Bark normal map source | **Option A** — generate `bark_normal.png` from `bark.png` offline via an in-repo binary (`src/bin/generate_normal_map.rs`); requires switching bark to `unlit: false` or the normal map has no effect |
| Foliage texture path resolution | **Campaign-relative** — same mechanism as landscape meshes; `assets/textures/trees/foliage_oak.png` resolves from the active campaign root |
| Wind system configuration | **Per campaign, in `data/wind.ron`** — `wind_system` field accepts `None`, `Sine`, or `Perlin`; both Sine and Perlin are implemented in WGSL. SDK `ContentDatabase` gains a matching field for parity |

### Relevant Source Files

| File | Role |
|---|---|
| `src/game/systems/advanced_trees.rs` | Branch graph, leaf geometry generation, LOD switching |
| `src/game/systems/procedural_meshes.rs` | Material creation, tree entity spawning |
| `src/game/systems/advanced_grass.rs` | Blade geometry, clump spawning, wind component, chunking |
| `src/game/systems/map.rs` | `MapRenderingPlugin` (line 110) — material plugin registration point |
| `src/game/resources/game_data.rs` | `GameDataResource` — game-layer wrapper around loaded `GameData` |
| `src/domain/campaign_loader.rs` | `GameData` struct; `load_game_data` entry point (line 247) |
| `src/domain/world/` | Domain types; new `wind.rs` module added in Phase 5 |
| `src/bin/generate_foliage_textures.rs` | New asset-generator binary (Phase 1) — renders the six foliage detail textures |
| `src/bin/generate_normal_map.rs` | New asset-generator binary (Phase 3) — derives `bark_normal.png` from `bark.png` |
| `src/sdk/database.rs` | SDK `ContentDatabase` (line 1048) — mirrors campaign data files; gains `wind` field in Phase 5 |
| `src/sdk/campaign_loader.rs` | SDK-side campaign loading; gains `wind.ron` support in Phase 5 |
| `assets/shaders/grass.wgsl` | New WGSL shader (Phase 6); `assets/shaders/` directory does not exist yet and must be created |
| `assets/shaders/grass_instanced.wgsl` | New instanced-pipeline shader (Phase 7) |
| `data/wind.ron` (per campaign) | New per-campaign wind configuration (Phase 5) |
| `docs/reference/campaign_content_format.md` | Campaign RON format reference; documents `wind.ron` in Phase 8 |
| `docs/reference/architecture.md` | Architecture reference; updated in Phase 8 |

---

## Current State Analysis

### Existing Infrastructure

- **Branch graph generator** (`advanced_trees.rs`): Recursive subdivision with
  exponential taper, quaternion force-direction per species, and species-specific
  LOD levels via `TreeLodGroup` / `tree_lod_switching_system`.

- **Leaf geometry** (`advanced_trees.rs`): `append_leaf_card` (line 1358)
  dispatches to per-species functions — `append_lobed_leaf_cluster` (Oak: 4
  coplanar quads), `append_pine_needle_cluster` (Pine: single triangle),
  `append_diamond_leaf` (Birch), `append_clustered_shrub_leaf` (Shrub),
  `append_palm_frond`, `append_willow_hanging_strip`. All geometry is
  single-plane. The plane basis is computed at lines 1368–1376:
  `side = direction.cross(Vec3::Y)` (fallback `Vec3::X`) and
  `up = Vec3::Y.lerp(direction, 0.35).normalize()`.

- **Material creation** (`procedural_meshes.rs`): `get_or_create_bark_material`
  (line 329) loads `assets/textures/trees/bark.png` and sets `unlit: true`
  (line 347). `get_or_create_foliage_material` (line 381) sets
  `AlphaMode::Opaque`, `unlit: true`, a species color, and
  `base_color_texture: None`. The doc comment (lines 355–362) and inline
  comment (lines 394–398) record a **deliberate decision**: the old round
  alpha-mask textures were removed because they made every species read as a
  circular blob; geometry now supplies the silhouette. All six
  `TREE_FOLIAGE_TEXTURE_*` constants, `TREE_ALPHA_CUTOFF`, and
  `foliage_texture_path()` are behind `#[cfg(test)]` (lines 948–981, 1116).
  `get_or_create_foliage_material_variant` (line 451) force-resets
  `base_color_texture = None` and `alpha_mode = Opaque` (lines 481–482).

- **Texture loading** (`creature_meshes.rs`): `load_texture(asset_server, path)`
  (line 415) requires `path` to start with `"assets/"` and delegates to
  `asset_server.load(path)`. The Bevy asset root (`BEVY_ASSET_ROOT`) is the
  active campaign directory, so `"assets/textures/trees/bark.png"` resolves to
  `campaigns/tutorial/assets/textures/trees/bark.png` at runtime.

- **Grass blade mesh** (`advanced_grass.rs`): `create_curved_grass_card_mesh`
  (line 886) builds a quadratic Bezier blade (single quad-strip) taking
  `vertex_colors: &[Color; 2]` (line 892). `create_grass_clump_mesh` (line 970)
  is a second mesh builder with its own copy of the Bezier math.
  `GrassColorScheme` (line 644) has one `base_color`; a derived tip color is
  generated at spawn time. Grass materials are **lit** (no `unlit` flag,
  `get_or_create_grass_material` line 847) and use `GRASS_BLADE_TEXTURE` with
  `AlphaMode::Mask`.

- **Grass wind** (`advanced_grass.rs`): `GrassWindParams` component (line 378,
  strength / frequency / phase) is attached to every blade entity at spawn
  (line 1146) but **no Bevy system reads it**. Wind has never been animated.

- **Campaign data loading** (`campaign_loader.rs`): `GameData` (line 102) holds
  `creatures`, `item_meshes`, `furniture`, `furniture_meshes`, `landscape`,
  `landscape_meshes`, and `levels`. `CampaignLoader::load_game_data` (line 247)
  is the single entry point; optional files follow a load-or-default pattern.
  The game layer accesses loaded data through `GameDataResource`
  (`src/game/resources/game_data.rs`, line 40).

- **SDK parity** (`src/sdk/database.rs`): `ContentDatabase` (line 1048) mirrors
  campaign data files — `furniture`, `landscape`, `landscape_meshes` are all
  documented as "loaded from `data/X.ron` in the campaign directory; missing
  file is not an error." New campaign data files must follow this convention.

### Identified Issues

1. **Foliage cards have no surface detail.** The legacy silhouette textures
   were deliberately removed (they made trees look like circular blobs), and
   geometry now provides the silhouette — but the cards render as flat,
   *unlit*, solid species colors. The existing `foliage_*.png` files in
   `assets/textures/trees/` and `campaigns/tutorial/assets/textures/trees/`
   are the legacy round alpha masks and are **unsuitable as-is**; they are
   regenerated as tiling leaf detail textures by a new procedural generator
   binary in Phase 1.

2. **Single-plane leaf geometry.** All leaf quads are co-planar to the branch
   direction. Canopy nearly disappears when viewed from certain angles.

3. **No bark normal map, and bark is unlit.** Only `bark.png` (base color) is
   loaded, with `unlit: true` (line 347). A normal map has zero effect on an
   unlit material, so Phase 3 must also switch bark to lit shading.

4. **Grass blades use quadratic Bezier only.** Cubic Bezier gives more natural
   S-curve lean. The two-color gradient misses an AO darkening at the base.

5. **`GrassWindParams` is a dead component.** No system ever reads it. All grass
   is completely static.

6. **No wind configuration in campaign data.** Wind intensity and algorithm are
   not part of any RON file — they cannot be authored per campaign, and the SDK
   `ContentDatabase` has no wind concept.

7. **`GrassInstanceBatch` is never drawn.** The instancing pipeline is partially
   built but not connected to any render pass.

---

## Implementation Phases

---

### Phase 1: Foliage Detail Texturing + Lit Foliage

Apply tiling leaf *detail* textures over the existing geometric leaf
silhouettes and switch foliage to lit shading. The geometry remains the
silhouette source — this phase does **not** restore the legacy round
alpha-mask approach. Goal: realistic-looking trees by default.

#### 1.1 Generate Detail Textures Procedurally (Asset Gate)

The six existing `foliage_*.png` files are legacy round alpha masks. Replace
them in place (same filenames, so the texture constants are unchanged) with
procedurally generated tiling leaf detail textures.

Add a binary at `src/bin/generate_foliage_textures.rs` that:

- Uses only existing dependencies: the `image` crate (`image = "0.25"`, already
  in `Cargo.toml`) for PNG output, plus a small deterministic PRNG implemented
  inline in the binary (e.g., splitmix64) — do **not** add `rand` or `noise`
  for this (the `noise` crate is not added until Phase 6).
- Renders one 512×512 RGBA8 image per species by stamping leaf-shape primitives
  at PRNG-driven positions/rotations/scales onto a transparent canvas, with
  per-pixel luminance jitter (±0.05) and a simple darkened-edge outline per
  primitive to suggest leaf veining and separation.
- Wraps stamp coordinates modulo 512 in both axes so the result tiles
  seamlessly.
- Uses a fixed per-species seed (table below) so output is byte-stable across
  runs.
- Writes each file to `assets/textures/trees/` and copies it to
  `campaigns/tutorial/assets/textures/trees/`.
- Asserts the output spec (below) after generation and exits non-zero on
  violation, so regeneration is self-validating.

Per-species generation parameters:

| Species | Seed | Primitive shape | Stamp count | Stamp size (px) | Base RGB (sRGB) |
|---|---|---|---|---|---|
| Oak | 101 | Rounded 5-lobe leaf | 220 | 40–70 | (0.80, 0.84, 0.72) |
| Pine | 102 | Thin needle stroke, 2px wide | 1400 | 30–55 long | (0.74, 0.80, 0.74) |
| Birch | 103 | Ovate leaf with pointed tip | 300 | 28–48 | (0.84, 0.87, 0.74) |
| Willow | 104 | Long narrow lanceolate strip | 500 | 50–90 long, 6–10 wide | (0.80, 0.84, 0.70) |
| Palm | 105 | Frond rib with paired leaflets | 90 | 90–140 long | (0.78, 0.84, 0.70) |
| Shrub | 106 | Small round leaf | 420 | 18–34 | (0.80, 0.83, 0.72) |

Output specification (asserted by the binary and by a unit test):

| Property | Requirement |
|---|---|
| Filenames | `foliage_{oak,pine,birch,willow,palm,shrub}.png` |
| Format | RGBA8 PNG, 512×512 |
| Tiling | Seamless — left/right and top/bottom edges match (stamps wrap) |
| Luminance | Mean luminance of opaque pixels 0.65–0.90, low saturation, so the species `base_color` multiply supplies the color, preserving the variant-tint system |
| Alpha | Leaf cutout with ragged edges; interior mostly opaque. Opaque coverage (alpha ≥ 128) 40–85% of pixels so the silhouette remains geometry-driven, not a blob |
| Determinism | Re-running the binary produces byte-identical files |

Run via `cargo run --bin generate-foliage-textures`. Commit the generated PNGs
(both copies); the binary exists so they can be regenerated and tuned, not as a
build step.

#### 1.2 Verify Per-Card UVs

Detail textures map per leaf card, so each card's UVs must span 0..1. In
[`src/game/systems/advanced_trees.rs`](../../src/game/systems/advanced_trees.rs),
inspect the UVs emitted by `append_quad` (line 1408) and each per-species
function (`append_lobed_leaf_cluster` line 1483, `append_pine_needle_cluster`
line 1527, `append_willow_hanging_strip` line 1569, `append_palm_frond` line
1602, `append_diamond_leaf` line 1644, `append_clustered_shrub_leaf` line
1668). Fix any generator whose cards do not span 0..1 in both U and V.

#### 1.3 Un-gate Texture Constants

In [`src/game/systems/procedural_meshes.rs`](../../src/game/systems/procedural_meshes.rs):

- Remove `#[cfg(test)]` from the six `TREE_FOLIAGE_TEXTURE_*` constants (lines
  948–979) and from `TREE_ALPHA_CUTOFF` (line 981).
- Remove `#[cfg(test)]` from `foliage_texture_path()` (line 1116).

#### 1.4 Update Material Creation

In `get_or_create_foliage_material` (line 381) and
`get_or_create_foliage_material_variant` (line 451):

- Rename the ignored `_asset_server` parameter (line 384) to `asset_server` and
  call `creature_meshes::load_texture(asset_server, foliage_texture_path(tree_type))`.
- Set `base_color_texture: Some(texture_handle)`.
- Change `alpha_mode: AlphaMode::Opaque` → `alpha_mode: AlphaMode::Mask(TREE_ALPHA_CUTOFF)`.
- Change `unlit: true` → `unlit: false`; set `perceptual_roughness: 0.9`.
- Keep `double_sided: true` and `cull_mode: None`.
- Keep `base_color: species_foliage_color_for_tree_type(tree_type)` — the
  light, desaturated detail texture is tinted by this multiply (this is why
  1.1 requires neutral luminance; do NOT force `base_color = WHITE` here, the
  tint multiply is the species/variant color mechanism).
- In the variant function, delete the force-reset at lines 481–482
  (`base_color_texture = None; alpha_mode = Opaque`) and instead inherit the
  texture and mask mode from the base material while applying the variant
  `color` as `base_color`.
- Update the doc comments at lines 355–368 and the inline comments at lines
  390–398 (they currently document the texture-free decision being superseded)
  and the doc text around lines 1058–1125.

#### 1.5 Campaign-Relative Path Resolution

Foliage texture paths begin with `"assets/"` and resolve via `load_texture`
(`creature_meshes.rs:415`), which delegates to the Bevy asset server. The asset
root is the active campaign directory, so
`"assets/textures/trees/foliage_oak.png"` resolves to
`campaigns/tutorial/assets/textures/trees/foliage_oak.png` at runtime. No path
adjustment is needed — identical to bark and landscape texture resolution.

#### 1.6 Testing Requirements

- Update tests asserting `alpha_mode == AlphaMode::Opaque` on foliage materials
  (lines 4584 and 4715 in `procedural_meshes.rs`), and search the test module
  for any other assertions on foliage `base_color_texture`, `alpha_mode`, or
  `unlit` and update them to the new expectations.
- Add a unit test (in the generator binary or `tests/`) asserting each of the
  six committed PNGs meets the 1.1 output specification: 512×512 RGBA8,
  opaque-pixel mean luminance 0.65–0.90, opaque coverage 40–85%, and matching
  wrap edges.
- Run all four quality gates: `cargo fmt --all`, `cargo check --all-targets
  --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo nextest run --all-features` — full suite passes, zero failures.
- Visual: launch tutorial campaign; Oak/Pine/Birch/Willow/Shrub/Palm canopy
  shows leaf surface detail and light/shade variation across the canopy. No
  species reads as a circular blob (regression guard for the original problem
  that caused texture removal).

#### 1.7 Deliverables

- [ ] `src/bin/generate_foliage_textures.rs` binary added (deterministic, no
  new dependencies, self-validating output)
- [ ] Six generated detail textures committed to spec in both asset locations
- [ ] Per-card UVs verified/fixed to span 0..1
- [ ] `TREE_FOLIAGE_TEXTURE_*`, `TREE_ALPHA_CUTOFF`, `foliage_texture_path()`
  available in non-test builds
- [ ] `get_or_create_foliage_material` loads detail texture, uses
  `AlphaMode::Mask(TREE_ALPHA_CUTOFF)`, `unlit: false`
- [ ] `get_or_create_foliage_material_variant` updated (force-reset removed)
- [ ] Superseded doc/inline comments updated
- [ ] All four quality gates pass

#### 1.8 Success Criteria

All six species render lit, detail-textured foliage with geometry-driven
silhouettes. No solid-color canopy and no circular-blob regression in the
tutorial campaign.

---

### Phase 2: Cross-Pattern Leaf Volume

Add a perpendicular second pass of leaf geometry inside `append_leaf_card` so
canopy looks volumetric from all camera angles.

#### 2.1 Current Geometry Analysis

Each species function builds geometry in a single plane defined by the `side`
and `up` vectors computed in `append_leaf_card` (lines 1368–1376):

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
2. Calls it again with `side2 = side.cross(up).normalize_or_zero()` substituted
   for `side`, keeping `up` the same (second pass). `side2` is perpendicular to
   the first pass's plane by construction.

Do **not** use `direction` as the substitute side vector: for near-vertical
branches `up ≈ direction` (because `up = Y.lerp(direction, 0.35)`), which would
collapse the second pass to near-zero area. `side.cross(up)` is non-degenerate
whenever the first pass is.

Replace the per-species dispatch in `append_leaf_card` (line 1358) with a call
to `append_leaf_card_cross` for Oak, Birch, Willow, Palm, and Shrub.

**Pine exception**: `append_pine_needle_cluster` uses a triangle that reads well
from all angles. Skip the second pass for `TreeType::Pine` to avoid overdraw on
dense conifer canopy.

#### 2.3 Leaf Count Compensation

With two passes the polygon budget doubles per leaf card. Reduce
`LeafPreset.count` (line 449 in `advanced_trees.rs`; reached via
`TreeSpeciesPreset.leaves`, line 476) by 30–40% for the five species that gain
the cross pass. Note: the runtime foliage-density input bucketed by
`TreeMeshCacheKey.foliage_density_bucket` (line 494) is a separate knob and is
unchanged.

#### 2.4 Testing Requirements

- Verify no triangle-winding issues: `double_sided: true` from Phase 1 covers
  both passes without needing reversed indices.
- Run all four quality gates — full suite passes, zero failures.
- Visual: rotate camera 360° around an Oak tree; canopy density should remain
  roughly consistent at 0°, 45°, and 90° view angles.

#### 2.5 Deliverables

- [ ] `append_leaf_card_cross` added to `advanced_trees.rs` using
  `side.cross(up)` for the second pass
- [ ] Oak, Birch, Willow, Palm, Shrub use cross-pattern geometry
- [ ] Pine uses single-pass (unchanged)
- [ ] `LeafPreset.count` reduced 30–40% for the five cross-pass species
- [ ] All four quality gates pass

#### 2.6 Success Criteria

Canopy silhouette visible and roughly consistent when camera orbits a tree 360°.
No visible frame-time spike on tutorial campaign forest tiles.

---

### Phase 3: Bark Normal Map + Lit Bark

Generate `bark_normal.png` offline from `bark.png` and wire it into the bark
material. **Prerequisite insight**: bark materials are currently
`unlit: true` (`procedural_meshes.rs:347`, and the variant fallback at line
434) — a normal map has no effect on an unlit material, so this phase also
switches bark to lit shading. This changes the trunk's visual baseline; the
testing section validates it.

#### 3.1 Asset Generation

Add a binary at `src/bin/generate_normal_map.rs` that:

- Reads `assets/textures/trees/bark.png` via the `image` crate (already a
  dependency: `image = "0.25"` in `Cargo.toml`; no new dependency needed).
- Converts to grayscale height, applies a 3×3 Sobel kernel per pixel to compute
  the XY gradient.
- Packs as an RGB normal map: `R = dx`, `G = dy`, `B = 1.0`, normalized, then
  remapped from `[-1, 1]` to `[0, 255]`.
- Writes `assets/textures/trees/bark_normal.png`.

This keeps generation deterministic, version-controlled, and re-runnable if
`bark.png` changes. (An ImageMagick pipeline was considered and rejected: the
Sobel-to-normal-map remap is not expressible as a single unambiguous command.)

After generation, copy the file to the tutorial campaign:
`campaigns/tutorial/assets/textures/trees/bark_normal.png`

#### 3.2 Constant and Material Integration

In [`src/game/systems/procedural_meshes.rs`](../../src/game/systems/procedural_meshes.rs):

- Add `const TREE_BARK_NORMAL_TEXTURE: &str = "assets/textures/trees/bark_normal.png";`
  alongside `TREE_BARK_TEXTURE` (line 943).
- In `get_or_create_bark_material` (line 329): load the normal texture via
  `creature_meshes::load_texture(asset_server, TREE_BARK_NORMAL_TEXTURE)` and
  set `normal_map_texture: Some(normal_handle)`.
- Change `unlit: true` → `unlit: false` (line 347). Keep
  `perceptual_roughness: 0.9` and `base_color: Color::WHITE`.
- Set `flip_normal_map_y: false` (DirectX convention; flip to `true` if grooves
  appear inverted on one axis).
- Apply the same changes to `get_or_create_bark_material_variant` (line 413),
  including its inline fallback `StandardMaterial` at lines 430–436.
- Note: with Phase 1 and this phase complete, both bark and foliage are lit —
  trunk and canopy shading are consistent.

#### 3.3 Testing Requirements

- Confirm `bark_normal.png` exists in both asset locations before running
  visual tests.
- Search the test module for assertions on bark material `unlit` /
  `normal_map_texture` (e.g., near the bark texture assertion at line 4513) and
  update expectations.
- Run all four quality gates — full suite passes, zero failures.
- Visual: trunk and branches show groove shading under the map's directional
  light at close camera range; overall trunk brightness remains acceptable
  after the unlit→lit switch (compare against a pre-change screenshot).

#### 3.4 Deliverables

- [ ] `src/bin/generate_normal_map.rs` binary added
- [ ] `bark_normal.png` generated and stored at `assets/textures/trees/`
- [ ] `bark_normal.png` copied to `campaigns/tutorial/assets/textures/trees/`
- [ ] `TREE_BARK_NORMAL_TEXTURE` constant added
- [ ] Bark materials lit (`unlit: false`) with normal map applied in both
  `get_or_create_bark_material` and `get_or_create_bark_material_variant`
- [ ] All four quality gates pass

#### 3.5 Success Criteria

Bark surface shows visible groove shading under the map directional light.
Normal texture loads without asset server warnings. Trunk lighting reads
consistently with the lit foliage from Phase 1.

---

### Phase 4: Cubic Bezier Grass Blades + Three-Color Gradient

Upgrade grass blade geometry from quadratic to cubic Bezier and replace the
two-stop color gradient with a three-stop AO base / mid-green / tip highlight.

#### 4.1 Cubic Bezier Blade Geometry

In [`src/game/systems/advanced_grass.rs`](../../src/game/systems/advanced_grass.rs),
`create_curved_grass_card_mesh` (line 886):

Replace the current quadratic Bezier (three control points; coefficients at
lines 909–911, lateral curve at line 913 which uses `tilt * height * 0.25`)
with a cubic (four control points). The `height` multiplier on `tilt` **must be
preserved** so tilt semantics stay independent of blade height:

| Point | Position (lateral, height) |
|---|---|
| `p0` | `(0, 0)` — base, anchored |
| `p1` | `(tilt * height * 0.25, height * 0.33)` — first control: lower lean |
| `p2` | `(tilt * height * 0.4 + curve_amount * 0.5, height * 0.66)` — second control: mid lean |
| `p3` | `(curve_amount, height)` — tip |

Cubic formula: `B(t) = (1-t)³p0 + 3(1-t)²t·p1 + 3(1-t)t²·p2 + t³p3`

Replace `coeff0 / coeff1 / coeff2` with their four cubic equivalents.
`segment_count` and `GrassMeshKey` are unchanged — the math is internal only.

Apply the same cubic conversion to `create_grass_clump_mesh` (line 970), which
contains its own copy of the Bezier math.

#### 4.2 Three-Color Gradient

Extend `GrassColorScheme` (line 644) with two additional fields:

```
ao_color: Color       // dark base AO, default: srgb(0.08, 0.12, 0.06)
tip_color: Color      // tip highlight, default: srgb(0.72, 0.82, 0.45)
```

Rename existing `base_color` (line 646) → `mid_color` (mid-blade primary
color). Known blast radius of the rename — update each site:

- `GrassColorScheme::default()` (line 690) and the derived-color math at lines
  660–670 and 765
- `get_or_create_grass_material` / `cached_material_color` (lines 847–872)
- The spawn-time tip-color derivation (lines ~990–998)
- The visual-metadata tint multiply (line ~1299) and its doc (line 1173)
- The chunk-merge system that reads material `base_color` (line ~1889 reads
  `StandardMaterial.base_color`, which is unaffected, but the surrounding
  `GrassColorScheme` construction at line ~1893 is)
- All test construction sites (lines 2276, 2503, 2682, 3060, …)

Update `create_curved_grass_card_mesh` signature from `vertex_colors: &[Color; 2]`
(line 892) to `vertex_colors: &[Color; 3]`, and the same for
`create_grass_clump_mesh`. Update the `#[cfg(test)]` helper
`create_grass_blade_mesh` (line 875), which passes `&[Color::WHITE; 2]`.

Interpolate in two height segments:
- `t ∈ [0.0, 0.4]` → lerp `ao_color` → `mid_color` (remap t to 0..1 over 0..0.4)
- `t ∈ [0.4, 1.0]` → lerp `mid_color` → `tip_color` (remap t to 0..1 over 0.4..1.0)

Update all callers of both mesh builders and `spawn_grass_clump` (line 1076).

#### 4.3 Testing Requirements

- Update `GrassColorScheme::default()` and all construction sites to the new
  field names (compiler-driven; the rename makes missed sites a build error).
- Run all four quality gates — full suite passes, zero failures.
- Visual: grass blades show dark base → green mid → lighter tip gradient; blades
  curve with an S-shape rather than a simple arc.

#### 4.4 Deliverables

- [ ] `create_curved_grass_card_mesh` and `create_grass_clump_mesh` use cubic
  Bezier with height-scaled tilt
- [ ] `GrassColorScheme` has `ao_color`, `mid_color`, `tip_color`
- [ ] Three-stop gradient applied along blade height
- [ ] All callers and tests updated; all four quality gates pass

#### 4.5 Success Criteria

Grass blades show a natural S-curve and a visible three-tone gradient.
No regressions in grass placement, exclusion zones, or LOD behavior.

---

### Phase 5: Per-Campaign Wind Configuration

Add a `data/wind.ron` file to the campaign data format, corresponding domain
types, the game-layer resource, and SDK parity so wind algorithm and parameters
are campaign-authored rather than hard-coded.

#### 5.1 Domain Types

Create [`src/domain/world/wind.rs`](../../src/domain/world/wind.rs):

```
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Antares Contributors

pub enum WindSystemKind {
    None,    // no wind animation (must be #[default])
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
    pub perlin_seed: u32,    // RNG seed; u32 because noise::Perlin::new takes u32, default 0
}
```

Add `CampaignWindConfig` and `WindSystemKind` to re-exports in
`src/domain/world/mod.rs` and `src/domain/mod.rs`.

Derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq` on both types;
derive `Default` on `CampaignWindConfig` via manual impl (non-zero field
defaults) and on `WindSystemKind` with `#[default]` on `None`. Annotate all
`CampaignWindConfig` fields with `#[serde(default = ...)]` so a minimal RON
file containing only `wind_system: Sine` is valid. Include runnable doc
examples on all public items (AGENTS.md documentation rule).

#### 5.2 Campaign Loader

In [`src/domain/campaign_loader.rs`](../../src/domain/campaign_loader.rs):

- Add `pub wind: CampaignWindConfig` to `GameData` (line 102), defaulting to
  `CampaignWindConfig::default()` (= `None` system).
- Add `fn load_wind_config(&self) -> Result<CampaignWindConfig, CampaignError>`
  that reads `data/wind.ron` relative to `self.campaign_path`, following the
  same opt-in pattern as `load_landscape` (line 453): if the file is absent,
  return `CampaignWindConfig::default()` (no error).
- Call `load_wind_config` from `load_game_data` (line 247) and assign to
  `game_data.wind` alongside the other optional loads (lines 250–270).

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

Note: `GameContent` (`src/application/resources.rs:43`) wraps the SDK
`ContentDatabase`, **not** `GameData` — do not wire wind through it. The
game-layer path is `GameDataResource` (`src/game/resources/game_data.rs:40`),
which wraps the loaded `GameData`.

- Create `src/game/resources/wind_config.rs` (the `resources` module is a
  directory, not a single file) containing:

```
#[derive(Resource, Clone, Debug, Default)]
pub struct WindConfig(pub CampaignWindConfig);
```

- Register the module in `src/game/resources/mod.rs`.
- At the point where `GameDataResource` is constructed and inserted after
  campaign load, also insert `WindConfig(game_data.wind.clone())`.

#### 5.5 SDK Parity

New campaign data files must be mirrored in the SDK (convention: `furniture`,
`landscape`, `landscape_meshes` in `ContentDatabase`). In:

- [`src/sdk/database.rs`](../../src/sdk/database.rs): add
  `pub wind: crate::domain::world::wind::CampaignWindConfig` to
  `ContentDatabase` (line 1048) with the same "missing file is not an error"
  doc comment style as the `landscape` field.
- [`src/sdk/campaign_loader.rs`](../../src/sdk/campaign_loader.rs): load
  `data/wind.ron` into `ContentDatabase.wind` using the same opt-in pattern as
  landscape.
- [`src/sdk/validation.rs`](../../src/sdk/validation.rs): validate ranges —
  `strength ≥ 0.0`, `frequency > 0.0`, `direction` non-zero and finite,
  `perlin_scale > 0.0`, `1 ≤ perlin_octaves ≤ 8`.
- Confirm `src/sdk/campaign_packager.rs` includes `data/wind.ron` when present
  (if it packages by directory glob, no change is needed — verify and note).

#### 5.6 Test Fixtures

- Add `data/test_campaign/data/wind.ron` with `wind_system: None` so existing
  loader tests continue to pass.
- Add `campaigns/tutorial/data/wind.ron` with `wind_system: Sine` (the 5.3
  example).

#### 5.7 Testing Requirements

- Unit tests in `wind.rs`: round-trip RON serialization for `None`, `Sine`, and
  `Perlin` variants; minimal file (only `wind_system`) deserializes with
  defaults; missing file returns `default()`.
- SDK tests: `ContentDatabase` loads `wind.ron`; validation rejects
  out-of-range values listed in 5.5.
- Run all four quality gates — full suite passes, zero failures.

#### 5.8 Deliverables

- [ ] `src/domain/world/wind.rs` with `WindSystemKind` and `CampaignWindConfig`
  (re-exported from `domain::world` and `domain`)
- [ ] `GameData.wind` field added; `load_wind_config` called from
  `load_game_data`
- [ ] `src/game/resources/wind_config.rs` with `WindConfig` resource, inserted
  alongside `GameDataResource`
- [ ] SDK: `ContentDatabase.wind` field, loader support, validation rules
- [ ] `campaigns/tutorial/data/wind.ron` with `wind_system: Sine`
- [ ] `data/test_campaign/data/wind.ron` with `wind_system: None`
- [ ] RON round-trip and SDK validation tests pass; all four quality gates pass

#### 5.9 Success Criteria

`WindConfig` is available as a Bevy resource at runtime. Changing `wind_system`
in `data/wind.ron` and reloading the campaign changes which shader path runs.
Missing `wind.ron` silently disables wind. SDK loads and validates the same
file.

---

### Phase 6: WGSL Grass Wind Shader (Sine + Perlin)

Implement a custom WGSL vertex shader for grass that reads wind parameters and
branches on `wind_system` to animate blades either with a simple sine wave or
with a spatially coherent Perlin noise texture. This replaces the dead
`GrassWindParams` CPU approach.

#### 6.1 Shader Asset

Create [`assets/shaders/grass.wgsl`](../../assets/shaders/grass.wgsl) (create
the `assets/shaders/` directory — it does not exist yet).

**Imports** (the `globals` binding comes from `mesh_view_bindings`; importing
only the `GlobalsUniform` type provides no binding):

```wgsl
#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}
```

**Wind extension bind group.** With `ExtendedMaterial`, `StandardMaterial`
occupies the low binding indices of `@group(2)`; extension bindings must start
at **100** (Bevy convention, see Bevy's `extended_material` example). Bindings
0–2 would collide:

```wgsl
struct GrassWindUniform {
    strength:       f32,
    frequency:      f32,
    direction:      vec2<f32>,
    wind_system:    u32,   // 0=None, 1=Sine, 2=Perlin
    perlin_scale:   f32,
    _pad:           vec2<f32>,
}
@group(2) @binding(100) var<uniform> wind: GrassWindUniform;
@group(2) @binding(101) var wind_noise: texture_2d<f32>;
@group(2) @binding(102) var wind_sampler: sampler;
```

**Vertex entry point.** Displacing `world_position` alone does not move
geometry — the clip-space `position` must be recomputed from the displaced
world position. Structure:

1. `var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);`
2. Compute `world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));`
3. Compute `sway` per the active wind system (below) and add
   `sway * wind.direction.x` to `world_position.x`, `sway * wind.direction.y`
   to `world_position.z`.
4. `out.world_position = world_position;`
   `out.position = position_world_to_clip(world_position.xyz);`
5. Forward normals, UVs, and vertex colors as in the stock vertex shader.

**Sine path (`wind_system == 1`):**

```wgsl
let t = globals.time * wind.frequency + world_position.x * 0.17 + world_position.z * 0.13;
let sway = wind.strength * vertex.uv.y * vertex.uv.y * sin(t);
```

`vertex.uv.y` encodes blade height 0→1 (set in `create_curved_grass_card_mesh`);
squaring it keeps the base anchored while the tip sways at full amplitude.

**Perlin path (`wind_system == 2`).** Vertex stages cannot use
`textureSample` (implicit derivatives are fragment-only in WGSL); use
`textureSampleLevel` with explicit LOD 0:

```wgsl
let scrolled_uv = world_position.xz / wind.perlin_scale
                + wind.direction * globals.time * wind.frequency;
let noise = textureSampleLevel(wind_noise, wind_sampler, scrolled_uv, 0.0).r;
let sway = wind.strength * vertex.uv.y * vertex.uv.y * (noise * 2.0 - 1.0);
```

**`wind_system == 0`:** `sway = 0.0` (no displacement).

**Fragment stage:** none — omit a custom fragment entry so the
`ExtendedMaterial` falls through to standard PBR shading with the Phase 4
vertex-color gradient (grass materials are already lit).

#### 6.2 Perlin Noise Texture Generation

When `WindConfig.0.wind_system == Perlin`, generate a tiling noise texture at
campaign load time:

- Add `fn generate_wind_noise_texture(config: &CampaignWindConfig) -> Image` to
  `src/game/systems/advanced_grass.rs`.
- Generate a 512×512 RGBA8 `Image` using the `noise` crate
  (`noise::Perlin::new(config.perlin_seed)` — seed is `u32` — with
  `noise::NoiseFn::get`).
- Tile the noise using wrapping coordinates so the texture tiles seamlessly.
- Apply `perlin_octaves` passes of fBm (fractional Brownian motion) for richer
  variation.
- Pack the float noise value into the R channel; set G/B/A to 255 (unused).
- Register the image via `Assets<Image>::add` and store the handle in a
  `WindNoiseTexture(Handle<Image>)` resource.
- When `wind_system == None` or `Sine`, skip generation; bind a 1×1 white
  placeholder texture.

Add `noise = "0.9"` to `Cargo.toml` dependencies (verified not currently
present).

#### 6.3 Custom Grass Material

Define the extension in `src/game/systems/advanced_grass.rs`:

```
#[derive(Asset, AsBindGroup, TypePath, Clone)]
struct GrassWindExtension {
    #[uniform(100)]
    wind: GrassWindUniform,          // ShaderType struct matching the WGSL layout
    #[texture(101)]
    #[sampler(102)]
    noise: Option<Handle<Image>>,
}

impl MaterialExtension for GrassWindExtension {
    fn vertex_shader() -> ShaderRef { "shaders/grass.wgsl".into() }
}
```

`GrassWindUniform` is a `#[derive(ShaderType, Clone)]` struct whose field order
and padding match the WGSL struct in 6.1 exactly (including `wind_system: u32`
and the trailing `vec2` pad).

Register
`MaterialPlugin::<ExtendedMaterial<StandardMaterial, GrassWindExtension>>` in
`MapRenderingPlugin` (`src/game/systems/map.rs:110`).

**Material handle migration — full blast radius.** Define
`type GrassMaterial = ExtendedMaterial<StandardMaterial, GrassWindExtension>;`
and migrate every grass site that currently uses `Handle<StandardMaterial>`:

| Site | Location |
|---|---|
| `get_or_create_grass_material` return type + `Assets<GrassMaterial>` param | `advanced_grass.rs:847` |
| `GrassAssetCache.material_handles` value type | `advanced_grass.rs` (cache struct) |
| `GrassBladeInstance.material` | `advanced_grass.rs:442` (plus its doctest, lines 434–441) |
| `GrassInstanceBatch.material` | `advanced_grass.rs:563` (plus its doctest, lines 547–556) |
| `spawn_grass_clump` spawn bundle (`MeshMaterial3d`) | `advanced_grass.rs:1076` |
| Chunk-merge system reading `Assets<StandardMaterial>` | `advanced_grass.rs:~1880–1895` |
| `build_grass_instance_batches_system` query | `advanced_grass.rs:1657` |
| All tests constructing grass materials/handles | `advanced_grass.rs` test module |

The extension values are read from `WindConfig` at spawn time and kept constant
per map (re-spawn the map to change wind parameters).

#### 6.4 Remove Dead CPU Wind Infrastructure

- Remove the `GrassWindParams` component (`advanced_grass.rs:378`), its
  `Default` impl (line 387), its doctest (lines 372–374), and its attachment in
  `spawn_grass_clump` (line 1146).
- Remove the per-blade `phase` value if it has no remaining consumer (the
  Phase 7 instanced pipeline re-introduces phase as per-instance data; keep the
  field only if Phase 7 is planned immediately after).

#### 6.5 Testing Requirements

- WGSL shader must load without Bevy shader compilation errors at startup for
  all three `wind_system` values.
- `wind_system: None` must produce no visible blade movement.
- `wind_system: Sine` must produce visible blade sway.
- `wind_system: Perlin` must produce spatially varied gusts.
- Run all four quality gates — full suite passes, zero failures.

#### 6.6 Deliverables

- [ ] `assets/shaders/grass.wgsl` with `None` / `Sine` / `Perlin` vertex paths,
  extension bindings at `@group(2) @binding(100..102)`, clip position
  recomputed after displacement, `textureSampleLevel` in the vertex stage
- [ ] Perlin noise texture generated at campaign load when `wind_system: Perlin`;
  1×1 placeholder otherwise
- [ ] `GrassMaterial` (`ExtendedMaterial`) registered in `MapRenderingPlugin`
  and migrated across all sites in the 6.3 table
- [ ] Wind uniforms populated from `WindConfig` resource at spawn time
- [ ] `GrassWindParams` component removed (including doctests)
- [ ] `noise = "0.9"` added to `Cargo.toml`
- [ ] All four quality gates pass

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
entities containing `Vec<world::InstanceData>` (`InstanceData` is an existing
domain type in `src/domain/world/`). These entities are created but no render
command reads or draws them. After Phase 6, `GrassInstanceBatch.material` holds
an `ExtendedMaterial` handle.

#### 7.2 Interaction with Phase 6 (Read First)

A custom `SpecializedRenderPipeline` **bypasses the Material system entirely**
— the instanced draw will NOT run the Phase 6 `ExtendedMaterial` wind shader.
Phase 7 therefore *replaces* the material-based grass path rather than adding
to it. Consequences that must be designed in, not discovered:

- The wind uniform (`GrassWindUniform`), the noise texture + sampler, and the
  Phase 4 vertex-color gradient must all be bound and evaluated inside the
  instanced pipeline's own shader, `assets/shaders/grass_instanced.wgsl`. Port
  the Phase 6 vertex logic into it; the per-instance `phase` offset feeds the
  sine term.
- Lighting: the instanced shader must reproduce the PBR fragment path (or a
  deliberately simplified lit model) — document the choice.
- Phase 6's `ExtendedMaterial` remains the fallback path until 7.4 removes
  per-blade entities; both paths must not render simultaneously (gate with a
  feature flag or a `GrassRenderMode` resource during bring-up).

#### 7.3 Render Pipeline

Create a new module `src/game/systems/grass_instancing.rs` containing:

- `GrassInstancedPipeline` implementing `SpecializedRenderPipeline`:
  - **Vertex buffer 0** (`VertexStepMode::Vertex`): per-vertex blade geometry —
    position (`Vec3`), normal (`Vec3`), UV (`Vec2`), vertex color (`Vec4`)
  - **Vertex buffer 1** (`VertexStepMode::Instance`): per-instance data —
    world position (`Vec3`), surface normal (`Vec3`), phase offset (`f32`)
- Render-app wiring: extract `GrassInstanceBatch` components into the render
  world (`ExtractComponent` or an extract system); during `PrepareAssets`,
  upload `GrassInstanceBatch.instances` to a GPU `Buffer` with
  `BufferUsages::VERTEX`; queue a `DrawGrassInstanced` command into the
  `Opaque3d` phase issuing `draw_indexed(0..index_count, 0, 0..instance_count)`
  once per batch.
- Register the pipeline, extract/prepare/queue systems, and the draw command in
  `MapRenderingPlugin` (or a new `GrassInstancingPlugin` added by it).

This follows the same pattern as Bevy's `shader_instancing` example and
`bevy_procedural_grass`'s `DrawGrassInstanced` / `GrassPipeline`.

#### 7.4 Remove Per-Blade Entities

Once instancing is confirmed working visually:

- Remove the per-blade `Mesh3d` / `MeshMaterial3d` entity spawn from
  `spawn_grass_clump` (line 1076).
- Remove now-unused `GrassBlade` (line 420), `GrassClump` (line 173),
  `GrassBladeInstance` (line 442) component queries from LOD and culling
  systems.
- Manage blade lifecycle at the chunk level via `GrassChunk` entities (line
  1736) only.
- Update `grass_chunk_culling_system` (line 1919) to operate on chunk-level
  visibility.
- Remove the Phase 6 `ExtendedMaterial` grass path once the instanced path is
  the sole renderer (the extension types move into the instanced pipeline's
  uniform plumbing).

#### 7.5 Testing Requirements

- Verify `GrassInstanceBatch` entities carry the expected instance count after
  map spawn.
- Verify grass renders with visual parity to Phase 6 output, including wind
  animation for all three `wind_system` values.
- Profile frame time on a 20×20 all-Forest map: target ≥30% reduction in render
  thread grass cost vs. per-entity approach.
- Run all four quality gates — full suite passes, zero failures.

#### 7.6 Deliverables

- [ ] `src/game/systems/grass_instancing.rs` with `GrassInstancedPipeline` /
  `DrawGrassInstanced`
- [ ] `assets/shaders/grass_instanced.wgsl` reproducing wind (Sine + Perlin)
  and the three-stop vertex-color gradient
- [ ] `GrassInstanceBatch` instance data uploaded to GPU vertex buffer
- [ ] Per-blade entity spawn removed; lifecycle managed at chunk level
- [ ] `grass_chunk_culling_system` updated to chunk-level visibility
- [ ] All four quality gates pass

#### 7.7 Success Criteria

Dense grass maps spawn in under 2 seconds. Bevy entity count for grass is
reduced by ≥90% on a 20×20 all-Forest map. Frame time is equal to or better
than Phase 6 baseline. Wind behavior is visually identical to Phase 6.

---

### Phase 8: Documentation Updates

Update existing documentation for the new campaign data format, domain types,
and rendering behavior. Per AGENTS.md, update existing documents — do not
create new documentation files.

#### 8.1 Reference Documentation

- [`docs/reference/campaign_content_format.md`](../reference/campaign_content_format.md):
  add a `wind.ron` section documenting the full schema from Phase 5.3 — field
  table (name, type, default, valid range), both example files, and the
  missing-file-means-no-wind behavior. Follow the structure used for
  `landscape.ron` / `furniture.ron` in the same document.
- [`docs/reference/architecture.md`](../reference/architecture.md): add
  `domain::world::wind`, the `WindConfig` game resource, the SDK
  `ContentDatabase.wind` field, and the grass shader/instancing pipeline to the
  relevant layer sections. Note: this file has pending uncommitted edits —
  rebase the additions on its current content.
- [`docs/reference/sdk_api.md`](../reference/sdk_api.md): document the
  `ContentDatabase.wind` field and its validation rules from Phase 5.5.

#### 8.2 Changelog and Doc Comments

- Add `CHANGELOG.md` entries per phase as phases land (conventional-commit
  style consistent with recent history).
- All new public items (`wind.rs` types, `WindConfig`, `GrassWindExtension`,
  instancing types, the `generate_foliage_textures` and `generate_normal_map`
  binaries) carry doc comments with
  runnable examples, tested by `cargo nextest run --all-features` (AGENTS.md
  documentation rule).

#### 8.3 Deliverables

- [ ] `campaign_content_format.md` documents `wind.ron`
- [ ] `architecture.md` updated for wind domain/resource/SDK/render additions
- [ ] `sdk_api.md` documents `ContentDatabase.wind`
- [ ] `CHANGELOG.md` entries for each landed phase
- [ ] Doc examples compile and pass as doctests

#### 8.4 Success Criteria

A campaign author can implement `wind.ron` from the reference docs alone.
Architecture docs match the shipped module layout.

---

## Implementation Order Summary

| Phase | Scope | Effort | Visual / Perf Impact |
|---|---|---|---|
| 1 — Foliage detail textures + lit foliage | Trees + generator binary | 1–1.5 days (no hand-authored art) | High visual |
| 2 — Cross-pattern leaves | Trees | 4–8 hrs | Medium visual |
| 3 — Bark normal map + lit bark | Trees + tooling | 2–4 hrs + binary | Medium visual |
| 4 — Cubic Bezier + 3-color grass | Grass | 1 day | Medium visual |
| 5 — Per-campaign wind config | Domain + loader + SDK | 1–1.5 days | Enabler for 6 |
| 6 — WGSL wind shader (Sine + Perlin) | Grass shader | 2–3 days | High visual |
| 7 — GPU instancing | Grass render pipeline | 3–4 days | High perf |
| 8 — Documentation | Docs | 0.5 day (incremental) | Maintainability |

Phases 1–3 (trees) are independent of Phases 4–7 (grass). Within trees, do
Phase 1 before Phase 3 so the unlit→lit switch lands on foliage and bark in
order and lighting reads consistently. Phase 6 depends on Phase 5. Phase 7
depends on Phase 6 and **replaces** its material-based render path (see 7.2).
Phase 8 items should land incrementally with the phase that introduces each
change, with a final consistency pass at the end.
