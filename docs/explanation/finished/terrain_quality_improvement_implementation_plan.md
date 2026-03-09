# Terrain Quality Improvement Implementation Plan

<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

## Overview

Improve the visual quality of terrain, grass, and tree procedural meshes by
introducing PBR texture support for ground tiles and upgrading grass/tree
geometry to use textured, alpha-masked materials. All improvements must remain
compatible with the existing `TileVisualMetadata`-driven configuration system
so that campaign authors can continue to control grass density, tree types,
color tinting, and scale from map `.ron` files.

The plan is divided into three self-contained phases. Each phase compiles, passes
all tests, and ships a working improvement independently.

---

## Current State Analysis

### Existing Infrastructure

| Component                | File                                                                                                                  | Current Behaviour                                                                                              |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Ground tile materials    | `src/game/systems/map.rs` → `fn spawn_map` (L584–1418)                                                                | Flat `Color::srgb(...)` values per `TerrainType` variant, no textures                                          |
| Grass blades             | `src/game/systems/advanced_grass.rs` → `fn create_grass_blade_mesh` (L328–399)                                        | Bezier-curved quad strips with `AlphaMode::Opaque`, solid per-blade colour                                     |
| Grass material           | `src/game/systems/advanced_grass.rs` → `fn spawn_grass_cluster` (L404–489)                                            | `StandardMaterial { base_color, perceptual_roughness: 0.7, double_sided: true, cull_mode: None }` — no texture |
| Tree trunks              | `src/game/systems/procedural_meshes.rs` → `fn spawn_foliage_clusters` (L803–881) and `fn spawn_tree` (L884–991)       | Tapered cylinders + spheres coloured by `TREE_TRUNK_COLOR` and `TREE_FOLIAGE_COLOR` constants                  |
| Tree mesh cache          | `src/game/systems/procedural_meshes.rs` → `ProceduralMeshCache.tree_meshes` (L52) — `HashMap<TreeType, Handle<Mesh>>` | Caches mesh handles; no material handles cached                                                                |
| Terrain visual config    | `src/game/systems/advanced_trees.rs` → `struct TerrainVisualConfig` (L196–215)                                        | `scale`, `height_multiplier`, `color_tint`, `rotation_y` — passed through correctly                            |
| Domain tile metadata     | `src/domain/world/types.rs` → `struct TileVisualMetadata` (L420–493)                                                  | `height`, `width_x`, `width_z`, `grass_density`, `tree_type`, `color_tint`, `grass_blade_config`               |
| Asset loading pattern    | `src/game/resources/sprite_assets.rs` → `fn get_or_load_material` (L267–276)                                          | `asset_server.load::<Image>(path)` → `StandardMaterial { base_color_texture: Some(handle), .. }`               |
| Quality settings         | `src/game/resources/grass_quality_settings.rs` → `struct GrassQualitySettings` (L38–41)                               | `GrassPerformanceLevel` enum (`Low`, `Medium`, `High`) already plumbed into grass spawning                     |
| Texture assets directory | `assets/`                                                                                                             | No `assets/textures/` subdirectory exists; only `assets/sprites/`, `assets/portraits/`, `assets/icons/`        |

### Identified Issues

1. **No terrain textures**: `spawn_map` in `src/game/systems/map.rs` (L605–620)
   defines nine `*_rgb` tuple constants (`floor_rgb`, `wall_base_rgb`, `door_rgb`,
   `water_rgb`, `mountain_rgb`, `forest_rgb`, `grass_rgb`, `stone_rgb`, `dirt_rgb`)
   and builds `StandardMaterial { base_color: Color::srgb(...) }` instances —
   there is no `base_color_texture` ever assigned to ground or wall tiles.

2. **No texture asset directory**: `assets/textures/` does not exist. Any
   texture-loading path requires creating this directory and placing procedurally
   generated or artist-supplied PNG/KTX2 files there before Bevy's `AssetServer`
   can load them.

3. **Grass uses opaque solid colour**: `spawn_grass_cluster` (L461) sets
   `alpha_mode: AlphaMode::Opaque`. Realistic grass requires `AlphaMode::Mask`
   (alpha-cutout) with a `base_color_texture` that has a transparent background,
   otherwise the blade shape is invisible and all that is shown is the solid
   bounding rectangle.

4. **No `TerrainMaterialCache` resource**: The material/texture loading in
   `spawn_map` happens inline on every map load with no caching. The `ProceduralMeshCache`
   struct (`src/game/systems/procedural_meshes.rs` L46–126) caches mesh handles
   but not material or texture handles. There is no equivalent for terrain
   materials, causing redundant allocations per tile when the map is re-spawned.

5. **Tree foliage uses spheres with opaque solid colour**: `spawn_foliage_clusters`
   (L825–829) creates `StandardMaterial { base_color: foliage_color, perceptual_roughness: 0.7 }`.
   Spheres with opaque green material do not resemble leaves. Realistic foliage
   requires plane-quad billboards with an alpha-masked leaf texture (or a leaf
   sprite atlas), not spheres.

6. **Tree bark uses plain brown tapered cylinders**: `TREE_TRUNK_COLOR` (L652)
   is `Color::srgb(0.4, 0.25, 0.15)` — a flat brown. No normal map or roughness
   texture is applied, so trunks look plastic.

7. **`Palm` tree type falls back to `Oak`**: The match arm in
   `src/game/systems/map.rs` (L762–765) maps `domain::world::TreeType::Palm` to
   `advanced_trees::TreeType::Oak` with a comment "Fallback for Palm". A dedicated
   `Palm` variant should be added to the rendering `TreeType` enum.

---

## Implementation Phases

---

### Phase 1: Terrain Texture Foundation

Introduce the `assets/textures/terrain/` directory, a `TerrainMaterialCache`
Bevy resource, and wire it into `spawn_map` so that all nine terrain types render
with a `base_color_texture` instead of flat colour. The `color_tint` from
`TileVisualMetadata` is preserved by multiplying it into `base_color`.

#### 1.1 Foundation Work — Asset Directory and Placeholder Textures

**Goal**: Establish the asset path structure that Bevy's `AssetServer` will
serve. Because no artist textures exist yet, create a set of 8 procedurally
generated 64×64 PNG placeholder files (solid colour with a noise overlay) using
a new binary at `src/bin/generate_terrain_textures.rs`.

**Files to create**:

| Asset path                                 | Content           |
| ------------------------------------------ | ----------------- |
| `assets/textures/terrain/ground.png`       | Grey 64×64        |
| `assets/textures/terrain/grass.png`        | Green 64×64       |
| `assets/textures/terrain/stone.png`        | Light grey 64×64  |
| `assets/textures/terrain/mountain.png`     | Dark grey 64×64   |
| `assets/textures/terrain/dirt.png`         | Brown 64×64       |
| `assets/textures/terrain/water.png`        | Blue 64×64        |
| `assets/textures/terrain/lava.png`         | Red-orange 64×64  |
| `assets/textures/terrain/swamp.png`        | Olive-green 64×64 |
| `assets/textures/terrain/forest_floor.png` | Dark green 64×64  |

**File to create**: `src/bin/generate_terrain_textures.rs`

- Add SPDX header.
- Use the `image` crate (already in `Cargo.toml`) to write 64×64 RGBA PNGs.
- Each texture is a solid base colour (values below) with `±10` random noise per
  channel, seeded with a deterministic `u64` so the output is reproducible.
- The binary writes to `assets/textures/terrain/` relative to `CARGO_MANIFEST_DIR`.
- Add the binary to `Cargo.toml` under `[[bin]]` with `name = "generate_terrain_textures"`.

**Base colours for generator** (RGBA u8):

| Filename         | R   | G   | B   | A   |
| ---------------- | --- | --- | --- | --- |
| ground.png       | 100 | 95  | 85  | 255 |
| grass.png        | 65  | 120 | 50  | 255 |
| stone.png        | 130 | 130 | 135 | 255 |
| mountain.png     | 90  | 88  | 90  | 255 |
| dirt.png         | 110 | 80  | 55  | 255 |
| water.png        | 55  | 105 | 200 | 255 |
| lava.png         | 210 | 75  | 50  | 255 |
| swamp.png        | 88  | 100 | 55  | 255 |
| forest_floor.png | 50  | 95  | 40  | 255 |

#### 1.2 Add `TerrainMaterialCache` Resource

**File to create**: `src/game/resources/terrain_material_cache.rs`

Add SPDX header.

Define the following public resource struct:

```rust
// src/game/resources/terrain_material_cache.rs

pub struct TerrainMaterialCache {
    pub ground:       Option<Handle<StandardMaterial>>,
    pub grass:        Option<Handle<StandardMaterial>>,
    pub stone:        Option<Handle<StandardMaterial>>,
    pub mountain:     Option<Handle<StandardMaterial>>,
    pub dirt:         Option<Handle<StandardMaterial>>,
    pub water:        Option<Handle<StandardMaterial>>,
    pub lava:         Option<Handle<StandardMaterial>>,
    pub swamp:        Option<Handle<StandardMaterial>>,
    pub forest_floor: Option<Handle<StandardMaterial>>,
}
```

- Derive `Resource`, `Default` (all fields `None`).
- Add `/// doc` comments on every field.

Implement:

```rust
impl TerrainMaterialCache {
    /// Returns the cached handle for `terrain`, or `None` if not yet loaded.
    pub fn get(&self, terrain: TerrainType) -> Option<&Handle<StandardMaterial>>;

    /// Inserts or replaces the handle for `terrain`.
    pub fn set(&mut self, terrain: TerrainType, handle: Handle<StandardMaterial>);

    /// Returns `true` when all nine terrain variants have a cached handle.
    pub fn is_fully_loaded(&self) -> bool;
}
```

**File to modify**: `src/game/resources/mod.rs`

- Add `mod terrain_material_cache;`
- Add `pub use terrain_material_cache::TerrainMaterialCache;` to the `pub use` block.

#### 1.3 Add Terrain Material Loading System

**File to create**: `src/game/systems/terrain_materials.rs`

Add SPDX header.

Implement a Bevy startup system:

```rust
pub fn load_terrain_materials_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Build a TerrainMaterialCache by loading each terrain texture,
    // constructing a StandardMaterial, and storing its handle.
    // ...
    commands.insert_resource(cache);
}
```

Texture path constants — define these as `const` strings at module level:

| Constant name          | Value                                 |
| ---------------------- | ------------------------------------- |
| `TEXTURE_GROUND`       | `"textures/terrain/ground.png"`       |
| `TEXTURE_GRASS`        | `"textures/terrain/grass.png"`        |
| `TEXTURE_STONE`        | `"textures/terrain/stone.png"`        |
| `TEXTURE_MOUNTAIN`     | `"textures/terrain/mountain.png"`     |
| `TEXTURE_DIRT`         | `"textures/terrain/dirt.png"`         |
| `TEXTURE_WATER`        | `"textures/terrain/water.png"`        |
| `TEXTURE_LAVA`         | `"textures/terrain/lava.png"`         |
| `TEXTURE_SWAMP`        | `"textures/terrain/swamp.png"`        |
| `TEXTURE_FOREST_FLOOR` | `"textures/terrain/forest_floor.png"` |

For each terrain type, call `asset_server.load::<Image>(TEXTURE_*)`, then create:

```rust
StandardMaterial {
    base_color_texture: Some(texture_handle),
    perceptual_roughness: <per_terrain_value>,
    ..default()
}
```

Roughness values per terrain type:

| TerrainType    | `perceptual_roughness` |
| -------------- | ---------------------- |
| Ground         | 0.95                   |
| Grass          | 0.90                   |
| Stone          | 0.75                   |
| Mountain       | 0.85                   |
| Dirt           | 0.92                   |
| Water          | 0.10                   |
| Lava           | 0.60                   |
| Swamp          | 0.88                   |
| Forest (floor) | 0.90                   |

**File to modify**: `src/game/systems/map.rs`

In `impl Plugin for MapRenderingPlugin` → `fn build` (L200–228):

- Add `.add_systems(Startup, terrain_materials::load_terrain_materials_system)`.
- Add `use super::terrain_materials;` at the top of the file.

In `fn spawn_map` (L584):

- Add `terrain_cache: Res<TerrainMaterialCache>` as a new system parameter.
- Replace each inline `materials.add(StandardMaterial { base_color: ..._color, .. })`
  block for floor/wall tiles with a lookup into `terrain_cache`:

  ```rust
  // Before (example for Grass):
  let grass_material = materials.add(StandardMaterial {
      base_color: grass_color,
      perceptual_roughness: 0.9,
      ..default()
  });

  // After:
  let grass_material = terrain_cache
      .get(TerrainType::Grass)
      .cloned()
      .unwrap_or_else(|| {
          materials.add(StandardMaterial {
              base_color: grass_color,
              perceptual_roughness: 0.9,
              ..default()
          })
      });
  ```

- Preserve the existing `color_tint` logic: when `tile.visual.color_tint` is set,
  create a one-off tinted material by cloning the base material and setting
  `base_color` to `Color::srgb(base_r * r, base_g * g, base_b * b)` as already
  done for Mountain (L695–706) and Normal walls (L875–888). Do **not** overwrite
  the cached handle.

**File to modify**: `src/game/systems/mod.rs` (or wherever systems are declared)

- Add `pub mod terrain_materials;`

#### 1.4 Testing Requirements

Run the full quality gate sequence after every code change:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All four commands must produce zero errors and zero warnings.

Unit tests to add in `src/game/resources/terrain_material_cache.rs` inside
`mod tests`:

| Test name                                                       | What it verifies                                                                         |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `test_terrain_material_cache_default_all_none`                  | `TerrainMaterialCache::default()` has `None` for all nine fields                         |
| `test_terrain_material_cache_is_fully_loaded_false_when_empty`  | `is_fully_loaded()` returns `false` on default                                           |
| `test_terrain_material_cache_set_get_roundtrip`                 | `set(TerrainType::Grass, handle)` then `get(TerrainType::Grass)` returns the same handle |
| `test_terrain_material_cache_is_fully_loaded_true_when_all_set` | After nine `set()` calls (one per terrain), `is_fully_loaded()` returns `true`           |

Unit tests to add in `src/game/systems/terrain_materials.rs` inside `mod tests`:

| Test name                                                   | What it verifies                                                                                                                                                                               |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_texture_path_constants_non_empty`                     | All nine `TEXTURE_*` constants are non-empty strings starting with `"textures/terrain/"`                                                                                                       |
| `test_load_terrain_materials_system_inserts_cache_resource` | Build a minimal `App`, add `AssetPlugin` + `RenderPlugin` stubs or use `MinimalPlugins`, run the startup system, assert `TerrainMaterialCache` resource exists and `is_fully_loaded()` is true |

#### 1.5 Deliverables

- [ ] `src/bin/generate_terrain_textures.rs` — binary that generates placeholder terrain PNGs
- [ ] `assets/textures/terrain/*.png` — nine placeholder texture files (committed to the repo)
- [ ] `src/game/resources/terrain_material_cache.rs` — `TerrainMaterialCache` resource + impl
- [ ] `src/game/resources/mod.rs` — exports `TerrainMaterialCache`
- [ ] `src/game/systems/terrain_materials.rs` — `load_terrain_materials_system` startup system
- [ ] `src/game/systems/map.rs` — `spawn_map` reads from `TerrainMaterialCache`; `MapRenderingPlugin::build` registers the startup system
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All new unit tests pass

#### 1.6 Success Criteria

- `cargo nextest run --all-features` reports zero failures.
- `TerrainMaterialCache::is_fully_loaded()` returns `true` after the startup
  system runs in a test `App`.
- Every `TerrainType` variant in `spawn_map` resolves to a `StandardMaterial`
  that has a `base_color_texture: Some(_)` (verified by the cache test).
- The existing `color_tint` behaviour is preserved: a tile with
  `color_tint: Some((r, g, b))` still gets a tinted one-off material, not the
  cached handle.

---

### Phase 2: High-Quality Grass

Replace the opaque solid-colour grass blade material with an `AlphaMode::Mask`
textured material, add a grass blade texture asset, and ensure that all
`GrassBladeConfig` / `color_tint` parameters from `TileVisualMetadata` continue
to work by multiplying the tint into `base_color` on the `StandardMaterial`.

#### 2.1 Feature Work — Grass Blade Texture

**File to create**: `assets/textures/grass/grass_blade.png`

- Dimensions: 32×128 RGBA PNG.
- Content: A single vertical grass blade with transparent background.
  - The blade occupies roughly the centre 16 pixels wide.
  - It fades from opaque (alpha = 255) at the base to semi-transparent (alpha ≈ 64)
    at the tip.
  - Base colour: RGBA (60, 130, 50, 255).
- Generate this file with the existing `src/bin/generate_terrain_textures.rs`
  binary (extend it with a `generate_grass_blade_texture()` function) OR create
  a separate binary `src/bin/generate_grass_textures.rs` using the same `image`
  crate pattern.

**Constant to add** in `src/game/systems/advanced_grass.rs` at the module-level
constants block (after L31):

```rust
/// Path to the grass blade alpha-cutout texture
const GRASS_BLADE_TEXTURE: &str = "textures/grass/grass_blade.png";

/// Alpha threshold for grass blade mask cutout
const GRASS_ALPHA_CUTOFF: f32 = 0.3;
```

#### 2.2 Integrate Feature — Update `spawn_grass_cluster`

**File to modify**: `src/game/systems/advanced_grass.rs`

Function signature of `fn spawn_grass_cluster` (L404):

```rust
fn spawn_grass_cluster(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,   // ADD THIS PARAMETER
    cluster_center: Vec2,
    blade_height: f32,
    blade_config: &BladeConfig,
    color_scheme: &GrassColorScheme,
    parent_entity: Entity,
)
```

Inside `spawn_grass_cluster`, replace the existing `blade_material` construction
(L456–464 — `StandardMaterial { base_color: blade_color, perceptual_roughness: 0.7, double_sided: true, cull_mode: None, alpha_mode: AlphaMode::Opaque, ..default() }`)
with:

```rust
let texture_handle: Handle<Image> = asset_server.load(GRASS_BLADE_TEXTURE);
let blade_material = materials.add(StandardMaterial {
    base_color: blade_color,          // still tinted by GrassColorScheme
    base_color_texture: Some(texture_handle),
    alpha_mode: AlphaMode::Mask(GRASS_ALPHA_CUTOFF),
    double_sided: true,
    cull_mode: None,
    perceptual_roughness: 0.7,
    ..default()
});
```

Update the call-site of `spawn_grass_cluster` inside `fn spawn_grass` (L537–648)
to pass `asset_server`. The `fn spawn_grass` signature must also receive
`asset_server: &Res<AssetServer>` and forward it.

Update the call-site of `spawn_grass` in `src/game/systems/map.rs` → `fn spawn_map`
to pass `&asset_server`. The `asset_server: Res<AssetServer>` parameter is
already in `spawn_map`'s signature (L588).

#### 2.3 Configuration Updates

`GrassBladeConfig.color_variation` (field in `src/domain/world/types.rs` L370–401)
already drives `GrassColorScheme::sample_blade_color` (L292–313 in
`src/game/systems/advanced_grass.rs`), which returns a per-blade `Color`. This
`Color` is assigned to `base_color` in the material. Because `base_color` is
multiplied with the texture colour by Bevy's PBR pipeline at render time, tinting
continues to work without code changes — no domain struct modifications needed.

Verify that the following `TileVisualMetadata` fields continue to affect grass:

| Field                                  | How it reaches the material                                                                                                                                                                                                         |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `grass_density`                        | Consumed by `GrassQualitySettings::blade_count_range_for_content` in `spawn_grass`; controls blade count                                                                                                                            |
| `grass_blade_config.length` / `.width` | Read by `BladeConfig::from(&GrassBladeConfig)` (L243–252); affects mesh geometry in `create_grass_blade_mesh`                                                                                                                       |
| `grass_blade_config.color_variation`   | Feeds `GrassColorScheme`; sets `base_color` which tints the texture                                                                                                                                                                 |
| `color_tint` in `TileVisualMetadata`   | **Currently NOT forwarded to grass.** Add this in Phase 2: read `tile.visual.color_tint` in `spawn_grass` and pass it as an `Option<(f32,f32,f32)>` to the cluster. Multiply it into `GrassColorScheme.base_color` before sampling. |

**`color_tint` forwarding — specific change**:

In `fn spawn_grass` (L537), add an `Option<(f32, f32, f32)>` `tile_tint`
parameter. At the construction of `color_scheme` (approximately L580–590),
apply the tint:

```rust
if let Some((tr, tg, tb)) = tile_tint {
    color_scheme.base_color = Color::srgb(
        color_scheme.base_color.to_srgba().red   * tr,
        color_scheme.base_color.to_srgba().green * tg,
        color_scheme.base_color.to_srgba().blue  * tb,
    );
}
```

Pass `tile.visual.color_tint` from the `spawn_map` call site.

#### 2.4 Testing Requirements

Run the full quality gate sequence:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Unit tests to add in `src/game/systems/advanced_grass.rs` inside `mod tests`:

| Test name                                         | What it verifies                                                                                                                                                           |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_grass_blade_texture_path_constant`          | `GRASS_BLADE_TEXTURE` starts with `"textures/grass/"` and ends with `".png"`                                                                                               |
| `test_grass_alpha_cutoff_in_valid_range`          | `GRASS_ALPHA_CUTOFF` is `> 0.0` and `< 1.0`                                                                                                                                |
| `test_grass_material_uses_alpha_mask`             | Build a minimal `App`, spawn grass with `GrassDensity::Low`, inspect spawned `MeshMaterial3d` components, assert `alpha_mode == AlphaMode::Mask(_)`                        |
| `test_grass_color_tint_forwarded_to_color_scheme` | Call `spawn_grass` with a tile that has `color_tint: Some((0.5, 0.5, 0.5))` and `GrassDensity::Low`, assert resulting `GrassColorScheme.base_color` is darker than default |

Update the existing test `test_create_grass_blade_mesh_has_uvs` (L1149–1152) to
assert that UV coordinates span the full `[0.0, 1.0]` range on the V axis
(confirming texture mapping is correct end-to-end).

#### 2.5 Deliverables

- [ ] `assets/textures/grass/grass_blade.png` — 32×128 alpha-masked blade texture
- [ ] `src/game/systems/advanced_grass.rs` — `GRASS_BLADE_TEXTURE` and `GRASS_ALPHA_CUTOFF` constants added
- [ ] `src/game/systems/advanced_grass.rs` — `spawn_grass_cluster` and `spawn_grass` accept `asset_server` and use `AlphaMode::Mask`
- [ ] `src/game/systems/advanced_grass.rs` — `spawn_grass` accepts and applies `tile_tint: Option<(f32,f32,f32)>`
- [ ] `src/game/systems/map.rs` — `spawn_map` passes `&asset_server` and `tile.visual.color_tint` to `spawn_grass`
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All new and updated unit tests pass

#### 2.6 Success Criteria

- `cargo nextest run --all-features` reports zero failures.
- Spawned grass blade entities have a `MeshMaterial3d` whose `StandardMaterial`
  satisfies `alpha_mode == AlphaMode::Mask(_)` and
  `base_color_texture.is_some() == true` (verified by new unit test).
- `tile.visual.color_tint` set to `(0.8, 0.6, 0.3)` produces a yellowed grass
  tint without breaking blade count or mesh geometry.
- `GrassBladeConfig` fields (`length`, `width`, `tilt`, `curve`, `color_variation`)
  continue to affect geometry and material as before.

---

### Phase 3: High-Quality Tree Models

Add alpha-masked foliage plane-quads to replace sphere-based foliage, add a
bark texture to trunk cylinders, add a dedicated `Palm` `TreeType` variant to
the rendering enum, and cache material handles in `ProceduralMeshCache` alongside
existing mesh handles.

#### 3.1 Foundation Work — Tree Texture Assets

**Files to create** under `assets/textures/trees/`:

| Asset path                                 | Dimensions   | Content                                            |
| ------------------------------------------ | ------------ | -------------------------------------------------- |
| `assets/textures/trees/bark.png`           | 64×128 RGBA  | Brown vertical-grain pattern, fully opaque         |
| `assets/textures/trees/foliage_oak.png`    | 128×128 RGBA | Rounded leaf cluster, transparent background       |
| `assets/textures/trees/foliage_pine.png`   | 64×128 RGBA  | Vertical needle cluster, transparent background    |
| `assets/textures/trees/foliage_birch.png`  | 128×128 RGBA | Small round leaves, transparent background         |
| `assets/textures/trees/foliage_willow.png` | 128×128 RGBA | Drooping curtain of leaves, transparent background |
| `assets/textures/trees/foliage_palm.png`   | 128×128 RGBA | Fan-shaped palm fronds, transparent background     |
| `assets/textures/trees/foliage_shrub.png`  | 64×64 RGBA   | Dense bush silhouette, transparent background      |

Generate these with `src/bin/generate_terrain_textures.rs` (extend the existing
binary with a `generate_tree_textures()` function). Foliage textures use a rough
circular/leaf-shaped alpha mask over a base leaf colour; bark textures are fully
opaque.

#### 3.2 Feature Work — Extend `TreeType` Enum and Material Cache

**File to modify**: `src/game/systems/advanced_trees.rs`

Add `Palm` variant to `pub enum TreeType` (L270–294):

```rust
pub enum TreeType {
    Oak,
    Pine,
    Birch,
    Willow,
    Dead,
    Shrub,
    Palm,   // ADD: tropical palm tree
}
```

Add `Palm` arm to `impl TreeType`:

- `config()` (L310–361): Add `TreeType::Palm => TreeConfig { trunk_radius: 0.18, height: 5.5, branch_angle_range: (70.0, 85.0), depth: 2, foliage_density: 0.7, foliage_color: (0.4, 0.7, 0.2) }`
- `name()` (L373–382): Add `TreeType::Palm => "Palm"`
- `all()` (L395–404): Add `TreeType::Palm` to the returned slice

Update the fallback in `src/game/systems/map.rs` (L762–765) from:

```rust
crate::domain::world::TreeType::Palm => {
    crate::game::systems::advanced_trees::TreeType::Oak
} // Fallback for Palm
```

to:

```rust
crate::domain::world::TreeType::Palm => {
    crate::game::systems::advanced_trees::TreeType::Palm
}
```

**File to modify**: `src/game/systems/procedural_meshes.rs`

Extend `ProceduralMeshCache` (L46–126) with material handle fields:

```rust
// Bark material handle (shared across all non-Dead tree types)
tree_bark_material: Option<Handle<StandardMaterial>>,
// Foliage material handles keyed by TreeType
tree_foliage_materials: HashMap<TreeType, Handle<StandardMaterial>>,
```

Add methods to `impl ProceduralMeshCache` (L128–214):

```rust
/// Gets or creates the bark `StandardMaterial` handle.
pub fn get_or_create_bark_material(
    &mut self,
    asset_server: &AssetServer,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Handle<StandardMaterial>;

/// Gets or creates the foliage `StandardMaterial` handle for `tree_type`.
pub fn get_or_create_foliage_material(
    &mut self,
    tree_type: TreeType,
    asset_server: &AssetServer,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Handle<StandardMaterial>;
```

**Texture path constants** — add at the module-level constant block (after L596):

| Constant                      | Value                                 |
| ----------------------------- | ------------------------------------- |
| `TREE_BARK_TEXTURE`           | `"textures/trees/bark.png"`           |
| `TREE_FOLIAGE_TEXTURE_OAK`    | `"textures/trees/foliage_oak.png"`    |
| `TREE_FOLIAGE_TEXTURE_PINE`   | `"textures/trees/foliage_pine.png"`   |
| `TREE_FOLIAGE_TEXTURE_BIRCH`  | `"textures/trees/foliage_birch.png"`  |
| `TREE_FOLIAGE_TEXTURE_WILLOW` | `"textures/trees/foliage_willow.png"` |
| `TREE_FOLIAGE_TEXTURE_PALM`   | `"textures/trees/foliage_palm.png"`   |
| `TREE_FOLIAGE_TEXTURE_SHRUB`  | `"textures/trees/foliage_shrub.png"`  |
| `TREE_FOLIAGE_ALPHA_CUTOFF`   | `0.35_f32`                            |

Foliage texture selection helper (add as private fn in `procedural_meshes.rs`):

```rust
fn foliage_texture_path(tree_type: TreeType) -> &'static str {
    match tree_type {
        TreeType::Oak    => TREE_FOLIAGE_TEXTURE_OAK,
        TreeType::Pine   => TREE_FOLIAGE_TEXTURE_PINE,
        TreeType::Birch  => TREE_FOLIAGE_TEXTURE_BIRCH,
        TreeType::Willow => TREE_FOLIAGE_TEXTURE_WILLOW,
        TreeType::Palm   => TREE_FOLIAGE_TEXTURE_PALM,
        TreeType::Dead   => TREE_FOLIAGE_TEXTURE_OAK,  // unused; Dead has density 0
        TreeType::Shrub  => TREE_FOLIAGE_TEXTURE_SHRUB,
    }
}
```

**`get_or_create_bark_material` implementation**:

```rust
StandardMaterial {
    base_color_texture: Some(asset_server.load(TREE_BARK_TEXTURE)),
    base_color: TREE_TRUNK_COLOR,   // existing constant, now used as tint
    perceptual_roughness: 0.9,
    ..default()
}
```

**`get_or_create_foliage_material` implementation**:

```rust
StandardMaterial {
    base_color_texture: Some(asset_server.load(foliage_texture_path(tree_type))),
    base_color: Color::WHITE,          // tint is applied by spawn_tree caller
    alpha_mode: AlphaMode::Mask(TREE_FOLIAGE_ALPHA_CUTOFF),
    double_sided: true,
    cull_mode: None,
    perceptual_roughness: 0.8,
    ..default()
}
```

#### 3.3 Feature Work — Replace Foliage Spheres with Plane Quads

**File to modify**: `src/game/systems/procedural_meshes.rs`

Function `fn spawn_foliage_clusters` (L803–881):

1. Add `asset_server: &AssetServer` parameter.
2. Replace the foliage sphere `Mesh` (currently a `Sphere`) with a double-sided
   plane quad using `create_billboard_mesh` (L2632–2638, already exists in the
   file) or `Plane3d::default().mesh().size(foliage_size, foliage_size)`.
   - Size formula: `foliage_size = config.foliage_density * TREE_FOLIAGE_RADIUS`
     where `TREE_FOLIAGE_RADIUS` is the existing constant (L587).
3. Replace the inline `StandardMaterial` construction (L825–829) with a call
   to `cache.get_or_create_foliage_material(tree_type, asset_server, materials)`.
4. Apply `foliage_color` tint (already computed in `spawn_tree` L927–944) by
   cloning the cached material and setting `base_color` to the computed tint
   colour, only when `color_tint` is `Some(...)`. When `color_tint` is `None`,
   use the cached handle directly to avoid allocating a new material.

Add `tree_type: TreeType` and `asset_server: &AssetServer` parameters to
`spawn_foliage_clusters`.

Update the call-site of `spawn_foliage_clusters` inside `spawn_tree` (L977–988)
to pass the resolved `tree_type_resolved` and `&asset_server`.

Update `pub fn spawn_tree` signature (L884) to add `asset_server: &AssetServer`.

Update the call-site of `spawn_tree` in `src/game/systems/map.rs` → `spawn_map`
to pass `&asset_server`.

Update `impl ProceduralMeshCache` → `fn get_or_create_tree_mesh` (L139–154) to
also update the bark material: after inserting the mesh handle, call
`self.get_or_create_bark_material(asset_server, materials)` and store the result.
Add `asset_server: &AssetServer` and `materials: &mut ResMut<Assets<StandardMaterial>>`
parameters to `get_or_create_tree_mesh`.

Update trunk spawn inside `fn spawn_tree` (L950–975, approximate) to use
`cache.get_or_create_bark_material(asset_server, materials)` instead of the
inline `StandardMaterial { base_color: TREE_TRUNK_COLOR, .. }`.

#### 3.4 Configuration Updates

`TerrainVisualConfig` (defined in `src/game/systems/advanced_trees.rs` L196–215)
provides `scale`, `height_multiplier`, `color_tint`, and `rotation_y`. These are
consumed by `spawn_tree` already; no domain struct changes are required.

Verify the following continue to work end-to-end after Phase 3 changes:

| `TileVisualMetadata` field  | Expected post-Phase-3 behaviour                                                   |
| --------------------------- | --------------------------------------------------------------------------------- |
| `tree_type: Some(Pine)`     | Selects `foliage_pine.png` foliage texture                                        |
| `tree_type: Some(Palm)`     | Selects `foliage_palm.png` foliage texture (no longer falls back to Oak)          |
| `color_tint: Some((r,g,b))` | Applied to foliage `base_color`; bark uses fixed `TREE_TRUNK_COLOR`               |
| `visual.scale`              | Still applied via `TerrainVisualConfig` → `Transform::with_scale` in `spawn_tree` |
| `visual.rotation_y`         | Still applied via `Transform::with_rotation` in `spawn_tree`                      |

#### 3.5 Testing Requirements

Run the full quality gate sequence:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Unit tests to add in `src/game/systems/advanced_trees.rs` inside `mod tests`:

| Test name                                               | What it verifies                                                                                                |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `test_tree_type_palm_config_returns_correct_parameters` | `TreeType::Palm.config()` returns `trunk_radius == 0.18`, `height == 5.5`                                       |
| `test_tree_type_palm_name`                              | `TreeType::Palm.name() == "Palm"`                                                                               |
| `test_tree_type_all_includes_palm`                      | `TreeType::all().contains(&TreeType::Palm)` — also assert total length is now 7                                 |
| `test_all_tree_types_generate_without_panic`            | Existing test — verify it still passes and now covers `Palm` (update the test if it iterates `TreeType::all()`) |

Unit tests to add in `src/game/systems/procedural_meshes.rs` inside `mod tests`:

| Test name                                         | What it verifies                                                                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `test_foliage_texture_path_all_variants`          | `foliage_texture_path(t)` returns a non-empty string ending in `.png` for all seven `TreeType` variants |
| `test_tree_foliage_alpha_cutoff_valid`            | `TREE_FOLIAGE_ALPHA_CUTOFF` is `> 0.0` and `< 1.0`                                                      |
| `test_cache_tree_foliage_materials_default_empty` | `ProceduralMeshCache::default().tree_foliage_materials` is an empty `HashMap`                           |
| `test_cache_tree_bark_material_default_none`      | `ProceduralMeshCache::default().tree_bark_material` is `None`                                           |
| `test_cache_clear_all_clears_foliage_materials`   | After inserting a foliage material and calling `clear_all()`, `tree_foliage_materials` is empty         |

Update the existing test `test_all_tree_types_generate_without_panic` (L1537–1547)
in `advanced_trees.rs` to iterate `TreeType::all()` (which now includes `Palm`)
and assert no panics occur.

#### 3.6 Deliverables

- [ ] `assets/textures/trees/bark.png` — bark texture (64×128, opaque)
- [ ] `assets/textures/trees/foliage_oak.png` — Oak foliage alpha-mask texture
- [ ] `assets/textures/trees/foliage_pine.png` — Pine foliage alpha-mask texture
- [ ] `assets/textures/trees/foliage_birch.png` — Birch foliage alpha-mask texture
- [ ] `assets/textures/trees/foliage_willow.png` — Willow foliage alpha-mask texture
- [ ] `assets/textures/trees/foliage_palm.png` — Palm foliage alpha-mask texture
- [ ] `assets/textures/trees/foliage_shrub.png` — Shrub foliage alpha-mask texture
- [ ] `src/game/systems/advanced_trees.rs` — `Palm` variant added to `TreeType` enum, `config()`, `name()`, `all()`
- [ ] `src/game/systems/map.rs` — `Palm` domain type maps to `advanced_trees::TreeType::Palm` (fallback removed)
- [ ] `src/game/systems/procedural_meshes.rs` — `tree_bark_material` and `tree_foliage_materials` fields added to `ProceduralMeshCache`
- [ ] `src/game/systems/procedural_meshes.rs` — `get_or_create_bark_material` and `get_or_create_foliage_material` methods implemented
- [ ] `src/game/systems/procedural_meshes.rs` — `spawn_foliage_clusters` uses plane quads with `AlphaMode::Mask` foliage material
- [ ] `src/game/systems/procedural_meshes.rs` — `spawn_tree` uses bark material on trunk cylinders
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All new and updated unit tests pass

#### 3.7 Success Criteria

- `cargo nextest run --all-features` reports zero failures.
- `TreeType::all().len() == 7` (Oak, Pine, Birch, Willow, Dead, Shrub, Palm).
- A tile with `tree_type: Some(Palm)` spawns a tree entity whose trunk material
  has `base_color_texture == Some(_)` (bark texture) and whose foliage entities
  have `alpha_mode == AlphaMode::Mask(_)` and `base_color_texture == Some(_)`
  (verified in unit test or integration test).
- `ProceduralMeshCache::default().tree_foliage_materials.is_empty() == true`.
- The existing `TerrainVisualConfig` fields (`scale`, `rotation_y`, `color_tint`)
  continue to affect spawned tree entities without modification to the domain layer.

---

## Documentation

After all three phases pass quality gates, update
`docs/explanation/implementations.md` by prepending a new section:

```
## Terrain Quality Improvement

### Phase 1: Terrain Texture Foundation
### Phase 2: High-Quality Grass
### Phase 3: High-Quality Tree Models
```

Each section must list:

- Files created / modified
- Deliverables checklist (copied and checked from above)
- A one-paragraph summary of what changed and why
