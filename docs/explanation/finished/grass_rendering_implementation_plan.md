# Grass Rendering Implementation Plan

## Overview

This plan addresses the complete refactoring and fixing of grass rendering in Antares. The current implementation in `procedural_meshes.rs` has grass-related functions but grass is not rendering in the game. We will extract grass functionality into a dedicated module (`advanced_grass.rs`) following the pattern established by `advanced_trees.rs`, and implement proper rendering based on insights from the `bevy_procedural_grass` crate while adapting it to work with our RON-based map configuration system.

## Current State Analysis

### Existing Infrastructure

**Current Grass Implementation** (`src/game/systems/procedural_meshes.rs`):

- `create_grass_blade_mesh()` (L925-996): Creates curved blade geometry using Bezier curves
- `spawn_grass_cluster()` (L1023-1090): Spawns 5-10 blades in a cluster with variations
- `spawn_grass()` (L1093-1192): Main entry point, spawns multiple clusters per tile
- Uses `Billboard` component to face camera
- Uses `GrassQualitySettings` resource for density control
- Integrates with `TileVisualMetadata.grass_density` from RON map files

**Grass Quality System** (`src/game/resources/grass_quality_settings.rs`):

- `GrassQualitySettings` resource with configurable density
- `GrassDensity` enum: Low (2-4), Medium (6-10), High (12-20) blades per tile
- **Critical Issue**: Domain layer has different `GrassDensity` in `world/types.rs`: None, Low (10-20), Medium (40-60), High (80-120), VeryHigh (150+)
- **Purpose Confusion**: These serve DIFFERENT purposes but have identical names:
  - **Domain `GrassDensity`**: Map designer's content specification ("this area should be dense grass")
  - **Game `GrassDensity`**: Player's performance setting ("my hardware can render X blades")
  - **Correct Design**: Domain density × Performance multiplier = Actual blade count

**Domain Layer** (`src/domain/world/types.rs`):

- `TileVisualMetadata.grass_density: Option<GrassDensity>` (L410-414)
- `GrassDensity` enum with 5 variants (L59-71)
- Accessor method `grass_density()` returns default Medium (L496-505)

**Integration** (`src/game/systems/map.rs`):

- Calls `spawn_grass()` for Grass and Forest terrain tiles (L645-656)
- Passes `TileVisualMetadata` and `GrassQualitySettings` to spawner

**Reference Implementation** (`bevy_procedural_grass`):

- Custom shader-based rendering with wind effects
- Instanced rendering for performance
- LOD system with cull/lod distances
- `Blade` configuration: length, width, tilt, flexibility, curve, specular
- `GrassColor` with AO and two color variants
- Chunk-based organization for large areas
- Mesh generation creates UVs for shader texturing

### Identified Issues

1. **Grass Not Rendering**: Despite code presence, grass does not appear in game
2. **Incomplete Implementation**: Current code is in "half-finished state" after crash
3. **Module Organization**: Grass code mixed with other procedural meshes (3000+ line file)
4. **Density Enum Confusion**: Two `GrassDensity` enums serve different purposes but have same name (content spec vs performance setting)
5. **Billboard Quality Unknown**: Unclear if simple billboard approach can achieve quality grass rendering
6. **Missing Performance Systems**: No LOD/culling for maps where EVERY tile could be grass (20×20 = 400 tiles = 16,000+ blades at medium density)
7. **60 FPS Requirement**: Must maintain 60fps minimum with potentially hundreds of grass tiles
8. **Limited Configurability**: Cannot tweak blade appearance per tile via RON metadata
9. **Billboard Implementation**: May not be working correctly for grass geometry
10. **Mesh Generation Gap**: Current mesh lacks UVs, normals may be incorrect
11. **Performance Critical**: With entire maps potentially grass-covered, culling/LOD/instancing are MANDATORY not optional

## Implementation Phases

### Phase 1: Core Refactoring and Diagnosis

**Goal**: Extract grass code into dedicated module, fix density enum conflicts, diagnose rendering failure

#### 1.1 Create New Grass Module

**Files to Create**:

- `src/game/systems/advanced_grass.rs` - Main grass rendering module
- Extract from `procedural_meshes.rs`:
  - Constants: `GRASS_BLADE_WIDTH`, `GRASS_BLADE_HEIGHT_BASE`, `GRASS_BLADE_DEPTH`, `GRASS_BLADE_Y_OFFSET`, `GRASS_BLADE_COLOR`
  - Functions: `create_grass_blade_mesh()`, `spawn_grass_cluster()`, `spawn_grass()`
  - Tests: All grass-related tests from `procedural_meshes.rs` test module

**Module Structure Pattern** (following `advanced_trees.rs`):

```rust
// SPDX headers
// Module documentation

// Imports
use bevy::prelude::*;
use crate::domain::world::{GrassDensity as ContentDensity, TileVisualMetadata};
use crate::domain::types;

// Constants
pub const BLADE_WIDTH: f32 = 0.05;
pub const BLADE_HEIGHT_BASE: f32 = 0.4;
// ... other constants

// Configuration structs
pub struct BladeConfig { /* ... */ }
pub struct GrassConfig {
    pub cull_distance: f32,
    pub lod_distance: f32,
    // ... culling/LOD critical for performance
}

// Mesh generation with UVs, proper normals
fn create_blade_mesh(config: &BladeConfig) -> Mesh { /* ... */ }

// Spawning functions with performance awareness
pub fn spawn_grass(...) -> Entity { /* ... */ }

// Culling/LOD systems (MANDATORY for 60fps)
fn grass_culling_system(...) { /* ... */ }
fn grass_lod_system(...) { /* ... */ }

// Tests
#[cfg(test)]
mod tests { /* ... */ }
```

#### 1.2 Resolve Density Enum Conflict

**Problem**: Two `GrassDensity` enums serve DIFFERENT purposes but have identical names:

- **Domain `GrassDensity`**: Map content specification (10-150+ blades) - "this area has dense grass"
- **Game `GrassDensity`**: Performance quality setting (2-20 blades) - "render low quality for performance"

**Why Both Exist**:

- **Content Density**: Map designers specify intended grass coverage
- **Performance Quality**: Players configure what their hardware can handle
- **Actual Blade Count**: Content × Performance multiplier = Final count

**Resolution**: Rename game layer enum to clarify purpose

**Recommended Approach**:

- Keep `domain::world::GrassDensity` (content specification) - NO CHANGES
- Rename `game::resources::GrassDensity` → `GrassPerformanceLevel`
- Implement mapping: `fn apply_performance(content: ContentDensity, quality: GrassPerformanceLevel) -> (u32, u32)`

**Files to Modify**:

- `src/game/resources/grass_quality_settings.rs` - Rename `GrassDensity` to `GrassPerformanceLevel`, update all docs
- `src/game/resources/mod.rs` - Update re-exports
- `src/game/systems/map.rs` - Update to use both content density and performance level
- `src/game/systems/advanced_grass.rs` - Implement conversion logic
- SDK campaign builder - Update UI to clarify "Performance Quality" vs "Content Density"
- All tests using the enum

**Conversion Logic**:

```rust
impl GrassPerformanceLevel {
    /// Apply performance multiplier to content density
    pub fn apply_to_content_density(&self, content: ContentDensity) -> (u32, u32) {
        let base_range = match content {
            ContentDensity::None => (0, 0),
            ContentDensity::Low => (10, 20),
            ContentDensity::Medium => (40, 60),
            ContentDensity::High => (80, 120),
            ContentDensity::VeryHigh => (150, 200),
        };

        let multiplier = match self {
            Self::Low => 0.25,    // 25% of content density
            Self::Medium => 0.5,  // 50% of content density
            Self::High => 1.0,    // 100% of content density
        };

        let min = (base_range.0 as f32 * multiplier) as u32;
        let max = (base_range.1 as f32 * multiplier) as u32;
        (min.max(1), max.max(1))
    }
}
```

**Example**: Map specifies `High` density (80-120 blades), player has `Low` performance (0.25x) → Actual: 20-30 blades

#### 1.3 Define Quality Requirements

**Question Answered**: Can billboard approach achieve quality grass?

**Quality Grass Requirements**:

1. **Visual Fidelity**:

   - Proper mesh shape with tapering blade
   - UVs for potential texturing
   - Correct normals for lighting
   - Color variation (base to tip gradient)
   - Natural randomness (height, width, rotation, curve)

2. **Billboard vs Shader Tradeoff**:
   - **Simple Billboard CAN achieve**: Good visual quality with proper mesh + materials
   - **Simple Billboard CANNOT achieve**: Wind animation, advanced lighting effects
   - **Custom Shader ADDS**: Wind effects, better performance via instancing, advanced lighting

**Recommended Approach for Quality**:

- **Phase 1-3**: Billboard with enhanced mesh (UVs, normals, color variation) - SIMPLER, FASTER TO IMPLEMENT
- **Phase 5 (Optional)**: Custom shader for wind - COMPLEX, BETTER EFFECTS
- **Verdict**: Billboard approach CAN achieve quality grass sufficient for initial release

#### 1.4 Diagnose Rendering Failure

**Diagnostic Steps**:

1. Add logging to `spawn_grass()` to verify it's being called
2. Add logging to verify entity creation with correct components
3. Check if `Billboard` component is functioning (compare with working sprites)
4. Verify mesh vertex/index data is valid
5. Check material properties (alpha, color values)
6. Verify transform/visibility hierarchy
7. Test with simplified geometry (single quad) to isolate mesh vs rendering issue

**Files to Modify**:

- `src/game/systems/advanced_grass.rs` - Add debug logging
- Create debug visualization tool for grass entities

#### 1.5 Testing Requirements

**Unit Tests**:

- `test_blade_config_defaults()`
- `test_grass_config_defaults()`
- `test_create_blade_mesh_vertex_count()`
- `test_create_blade_mesh_normals_valid()`
- `test_create_blade_mesh_indices_valid()`
- `test_content_density_to_blade_count()`
- `test_performance_level_multiplier()`
- `test_apply_performance_to_content()`

**Integration Tests**:

- `test_grass_spawns_on_grass_terrain()`
- `test_grass_respects_metadata_density()`
- `test_grass_respects_quality_settings()`

**Visual Tests**:

- Single blade visibility test
- Cluster visibility test
- Multiple tiles with grass

#### 1.6 Deliverables

- [ ] `src/game/systems/advanced_grass.rs` created with extracted code
- [ ] `src/game/systems/mod.rs` updated to include `advanced_grass`
- [ ] `src/game/systems/procedural_meshes.rs` cleaned up (grass code removed)
- [ ] `src/game/resources/grass_quality_settings.rs` renamed enum to `GrassPerformanceLevel`
- [ ] Conversion logic implemented: content density × performance level → blade count
- [ ] All imports updated across codebase
- [ ] Quality requirements documented (billboard approach validated)
- [ ] Diagnostic logging added
- [ ] Root cause of rendering failure identified and documented
- [ ] All unit tests passing
- [ ] `docs/explanation/implementations.md` updated with Phase 1 summary

#### 1.7 Success Criteria

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo nextest run --all-features` passes with >80% coverage
- [ ] Grass code fully extracted to `advanced_grass.rs`
- [ ] Density enum conflict resolved (content vs performance clarified)
- [ ] Conversion logic tested and working
- [ ] Root cause of grass not rendering identified
- [ ] Code compiles and runs without grass rendering errors (even if not visible yet)

### Phase 2: Fix Basic Rendering + Essential Performance

**CRITICAL**: Performance systems moved from Phase 4 to Phase 2 due to requirement that entire maps could be grass (60fps minimum)

**Goal**: Get grass blades rendering AND implement mandatory culling/LOD for 60fps with grass-heavy maps

**Performance Requirement**: Maps up to 20×20 tiles (400 tiles) potentially ALL grass = 16,000+ blade entities at medium density. Must maintain 60fps minimum.

#### 2.1 Fix Mesh Generation

**Based on `bevy_procedural_grass` insights**:

- Ensure mesh has valid UVs (current implementation missing UVs)
- Fix normal calculation for proper lighting
- Verify vertex positions are in expected coordinate space
- Add proper mesh bounds for frustum culling

**Files to Modify**:

- `src/game/systems/advanced_grass.rs::create_blade_mesh()`

**Changes**:

```rust
fn create_grass_blade_mesh(config: &BladeConfig) -> Mesh {
    // ... existing position generation ...

    // ADD: UV coordinates for texturing
    let mut uvs = Vec::new();
    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;
        uvs.push([0.0, t]); // Left edge
        uvs.push([1.0, t]); // Right edge
    }

    // FIX: Calculate proper normals facing camera
    // Current: all normals point +Z
    // Better: calculate based on blade orientation

    // ... existing mesh creation ...
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // ADD: Compute mesh bounds for culling
    mesh
}
```

#### 2.2 Fix Material Properties

**Problem**: `StandardMaterial` may not render thin billboarded geometry correctly

**Investigation**:

- Check if double-sided rendering needed
- Verify alpha mode settings
- Test with emissive color for debugging visibility
- Compare with working billboard sprites

**Files to Modify**:

- `src/game/systems/advanced_grass.rs::spawn_grass_cluster()`

**Changes**:

```rust
let blade_material = materials.add(StandardMaterial {
    base_color: grass_color,
    perceptual_roughness: 0.7,
    unlit: false, // or true for debugging
    double_sided: true, // ADD: render both sides
    cull_mode: None, // ADD: don't cull backfaces
    alpha_mode: AlphaMode::Opaque, // or Blend if using transparency
    ..default()
});
```

#### 2.3 Fix Billboard Component

**Problem**: Billboard component may not be functioning correctly for grass

**Investigation**:

- Verify `BillboardPlugin` is registered in app
- Check if billboard system runs before rendering
- Test without billboard first (static orientation)
- Compare with working character billboards

**Files to Check**:

- `src/game/components/billboard.rs`
- `src/game/systems/billboard.rs`
- App setup in main

**Debugging Steps**:

1. Spawn grass WITHOUT `Billboard` component temporarily
2. Set fixed rotation to face camera direction manually
3. If visible → billboard system issue; if not → mesh/material issue

#### 2.4 Verify Transform Hierarchy

**Problem**: Parent-child entity hierarchy may cause transform issues

**Investigation**:

- Log parent and blade entity IDs
- Verify `GlobalTransform` is being computed
- Check if parent transform has correct position
- Verify blade local transforms relative to parent

**Files to Modify**:

- `src/game/systems/advanced_grass.rs::spawn_grass()` and `spawn_grass_cluster()`

#### 2.5 Implement Mandatory Culling System

**Why Now**: With potentially 400 grass tiles × 40-60 blades = 16,000-24,000 entities, culling is CRITICAL for 60fps

**Files to Modify**:

- `src/game/systems/advanced_grass.rs`

**New Components**:

```rust
#[derive(Component)]
pub struct GrassCluster {
    pub cull_distance: f32,
}

#[derive(Resource)]
pub struct GrassRenderConfig {
    pub cull_distance: f32,  // Default: 50.0
    pub lod_distance: f32,   // Default: 25.0
}
```

**Culling System**:

```rust
fn grass_distance_culling_system(
    mut grass_query: Query<(&GlobalTransform, &mut Visibility, &GrassCluster)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    config: Res<GrassRenderConfig>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    for (transform, mut visibility, cluster) in grass_query.iter_mut() {
        let distance = camera_pos.distance(transform.translation());

        // Cull grass beyond cull_distance
        if distance > config.cull_distance {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}
```

#### 2.6 Implement Mandatory LOD System

**Why Now**: Reduce blade count at distance for performance

**LOD Strategy**:

- **Near** (0-25m): Full blade count from content × performance calculation
- **Far** (25-50m): 50% blade count (every other cluster hidden)
- **Very Far** (50m+): Culled completely

**Implementation**:

```rust
#[derive(Component)]
pub struct GrassBlade {
    pub lod_index: u32,  // 0-indexed blade within cluster
}

fn grass_lod_system(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    cluster_query: Query<(&GlobalTransform, &Children), With<GrassCluster>>,
    mut blade_query: Query<(&mut Visibility, &GrassBlade)>,
    config: Res<GrassRenderConfig>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    for (cluster_transform, children) in cluster_query.iter() {
        let distance = camera_pos.distance(cluster_transform.translation());

        if distance > config.cull_distance {
            continue; // Handled by culling system
        }

        let lod_ratio = if distance > config.lod_distance {
            0.5  // Far LOD: 50% of blades
        } else {
            1.0  // Near LOD: 100% of blades
        };

        for &child in children.iter() {
            if let Ok((mut visibility, blade)) = blade_query.get_mut(child) {
                // Hide every other blade in far LOD
                if lod_ratio < 1.0 && blade.lod_index % 2 == 1 {
                    *visibility = Visibility::Hidden;
                } else {
                    *visibility = Visibility::Inherited;
                }
            }
        }
    }
}
```

#### 2.7 Performance Testing Requirements

**Benchmarks** (MANDATORY before Phase 2 completion):

- [ ] 100 grass tiles render at >60fps
- [ ] 400 grass tiles render at >60fps (full map)
- [ ] Culling reduces rendered blades beyond 50m
- [ ] LOD reduces blade count at 25-50m range
- [ ] Frame time profiling shows grass systems <5ms per frame

**Performance Tests**:

- `test_culling_hides_distant_grass()`
- `test_lod_reduces_blade_count_at_distance()`
- `test_full_map_grass_performance()` - 400 tiles benchmark
- `test_camera_movement_smooth()` - No stuttering when moving through grass

#### 2.8 Visual Testing Requirements

**Visual Tests** (manual verification):

- [ ] Single grass blade visible from all angles
- [ ] Grass cluster visible
- [ ] Multiple tiles with grass
- [ ] Grass respects color tint from metadata
- [ ] Grass height variation visible

**Unit Tests**:

- `test_mesh_has_uv_coordinates()`
- `test_mesh_has_valid_bounds()`
- `test_material_is_double_sided()`
- `test_blade_entity_hierarchy()`

#### 2.9 Deliverables

- [ ] Mesh generation fixed (UVs added, normals corrected)
- [ ] Material properties configured for visibility
- [ ] Billboard or fixed orientation rendering grass
- [ ] Transform hierarchy verified and documented
- [ ] Grass visible in game (screenshot proof)
- [ ] Culling system implemented and tested
- [ ] LOD system implemented and tested
- [ ] Performance benchmarks documented (100 tiles, 400 tiles)
- [ ] `GrassRenderConfig` resource created
- [ ] Systems registered in plugin
- [ ] `docs/explanation/implementations.md` updated with Phase 2 findings and performance data

#### 2.10 Success Criteria

- [ ] All Phase 1 success criteria still met
- [ ] Grass blades render and are visible in game
- [ ] No rendering errors in console
- [ ] Grass appears on Grass/Forest terrain tiles
- [ ] At least basic grass rendering works before proceeding to Phase 3
- [ ] **CRITICAL**: 60fps maintained with 400 grass tiles (full map coverage)
- [ ] Culling system functioning correctly
- [ ] LOD system functioning correctly
- [ ] Performance profiling shows acceptable frame times

### Phase 3: Enhanced Blade Configuration

**Goal**: Add configurable blade appearance via RON metadata, matching `bevy_procedural_grass` flexibility

#### 3.1 Extend TileVisualMetadata

**Add blade configuration fields to domain layer**:

**Files to Modify**:

- `src/domain/world/types.rs::TileVisualMetadata`

**New Fields**:

```rust
pub struct TileVisualMetadata {
    // ... existing fields ...

    /// Grass blade configuration (default: standard grass)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grass_blade_config: Option<GrassBladeConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GrassBladeConfig {
    /// Blade length multiplier (0.5-2.0, default 1.0)
    pub length: f32,
    /// Blade width multiplier (0.5-2.0, default 1.0)
    pub width: f32,
    /// Blade tilt angle in radians (0.0-0.5, default 0.3)
    pub tilt: f32,
    /// Blade curvature amount (0.0-1.0, default 0.3)
    pub curve: f32,
    /// Color variation (0.0 = uniform, 1.0 = high variation)
    pub color_variation: f32,
}
```

#### 3.2 Implement Blade Configuration

**Create blade config struct in grass module**:

**Files to Modify**:

- `src/game/systems/advanced_grass.rs`

**New Structs**:

```rust
pub struct BladeConfig {
    pub length: f32,
    pub width: f32,
    pub tilt: f32,
    pub curve: f32,
    pub color_variation: f32,
}

impl Default for BladeConfig {
    fn default() -> Self {
        Self {
            length: 1.0,
            width: 1.0,
            tilt: 0.3,
            curve: 0.3,
            color_variation: 0.2,
        }
    }
}

impl From<&GrassBladeConfig> for BladeConfig {
    fn from(config: &GrassBladeConfig) -> Self {
        // Clamp values to safe ranges
    }
}
```

#### 3.3 Add Color Variation

**Implement multi-tone grass colors inspired by `bevy_procedural_grass`**:

**Files to Modify**:

- `src/game/systems/advanced_grass.rs::spawn_grass_cluster()`

**Color System**:

```rust
pub struct GrassColorScheme {
    pub base_color: Color,
    pub tip_color: Color,
    pub variation: f32,
}

impl GrassColorScheme {
    pub fn sample_blade_color(&self, rng: &mut impl Rng) -> Color {
        // Blend base/tip with random variation
    }
}
```

#### 3.4 Configuration Updates

**Add example configurations to map data**:

**Files to Modify**:

- `campaigns/tutorial/data/maps/map_1.ron` - Add grass_blade_config examples

**Example RON**:

```ron
visual: (
    grass_density: Some(Medium),
    grass_blade_config: Some((
        length: 1.2,
        width: 0.8,
        tilt: 0.4,
        curve: 0.5,
        color_variation: 0.3,
    )),
    color_tint: Some((0.3, 0.7, 0.2)),
)
```

#### 3.5 Testing Requirements

**Unit Tests**:

- `test_blade_config_from_metadata()`
- `test_blade_config_clamping()`
- `test_color_variation_range()`
- `test_grass_blade_config_serialization()`

**Visual Tests**:

- [ ] Tall grass vs short grass
- [ ] Wide blades vs narrow blades
- [ ] High tilt vs upright
- [ ] High curve vs straight
- [ ] Color variation visible

#### 3.6 Deliverables

- [ ] `GrassBladeConfig` added to domain layer
- [ ] `BladeConfig` implementation in grass module
- [ ] Color variation system implemented
- [ ] Example configurations in tutorial map
- [ ] SDK map editor supports grass blade configuration
- [ ] Documentation with configuration examples
- [ ] `docs/explanation/implementations.md` updated

#### 3.7 Success Criteria

- [ ] All previous phase criteria still met
- [ ] Blade appearance configurable via RON metadata
- [ ] Color variation creates natural-looking grass
- [ ] Different grass styles visible in different map areas
- [ ] Configuration changes reflect in game without code changes

### Phase 4: Advanced Performance (Optional)

**Note**: Basic culling/LOD moved to Phase 2. This phase covers advanced optimizations.

**Goal**: Further optimize performance beyond 60fps baseline for ultra settings

#### 4.1 Mesh Instancing

**Use Bevy's mesh instancing for better performance**:

**Performance Gain**: Reduce draw calls from 16,000+ to dozens via batching

**Files to Modify**:

- `src/game/systems/advanced_grass.rs`

**Approach**:

- Create single grass blade mesh
- Use instance data for position/rotation/scale
- Batch rendering reduces draw calls from thousands to dozens

**Reference**: `bevy_procedural_grass/src/render/instance.rs`

**Complexity**: HIGH - Requires custom render pipeline, shader changes

#### 4.2 Frustum Culling Optimization

**Ensure mesh bounds set correctly**:

- Bevy's automatic frustum culling only works if mesh bounds are valid
- Current implementation may not set bounds, causing all grass to render even if off-screen

#### 4.3 Chunk-Based Organization

**Organize grass into spatial chunks**:

- Group nearby grass tiles into chunks
- Cull entire chunks at once
- Reduces per-frame iteration cost

#### 4.4 Testing Requirements

**Performance Tests**:

- Benchmark: 1000+ grass tiles render time
- Benchmark: Draw call count reduction with instancing

**Visual Tests**:

- [ ] Frame rate maintains >100fps with 400 grass tiles (advanced optimization)
- [ ] Instancing reduces draw calls significantly
- [ ] Frustum culling working correctly

#### 4.5 Deliverables

- [ ] Instancing system implemented (optional - complex)
- [ ] Frustum culling verified
- [ ] Chunk-based organization (optional)
- [ ] Advanced performance benchmarks documented
- [ ] `docs/explanation/implementations.md` updated with optimization results

#### 4.6 Success Criteria

- [ ] All previous phase criteria still met
- [ ] Performance exceeds 60fps baseline (targeting 100+fps)
- [ ] Draw calls significantly reduced (if instancing implemented)
- [ ] No noticeable frame drops even with extreme grass coverage
- [ ] Optimization techniques documented for future reference

**Note**: This phase is OPTIONAL - only pursue if Phase 2 performance isn't sufficient



## Answers to Open Questions

### 1. Density Enum Resolution ✓ ANSWERED

**Question**: Why two `GrassDensity` enums?

**Answer**: They serve DIFFERENT purposes:

- **Domain `GrassDensity`** (world/types.rs): Map content specification - "this area should have dense grass"
- **Game `GrassDensity`** (grass_quality_settings.rs): Player performance setting - "render at low quality"

**Resolution**:

- Keep domain `GrassDensity` unchanged (content specification)
- Rename game `GrassDensity` → `GrassPerformanceLevel` (performance setting)
- Implement: Content density × Performance multiplier = Actual blade count
- Example: High content (80-120) × Low performance (0.25x) = 20-30 actual blades

### 2. Billboard vs Custom Shader ✓ ANSWERED

**Question**: Can billboard achieve quality grass, or need custom shader?

**Answer**: Billboard CAN achieve quality grass:

- **Sufficient for quality**: Proper mesh (UVs, normals), color variation, good density
- **Billboard limitations**: No wind animation, simpler lighting
- **Custom shader adds**: Wind effects, advanced lighting, better instancing performance

**Decision**:

- **Phase 1-3**: Billboard approach (SIMPLER, FASTER to implement, SUFFICIENT quality)
- **Phase 5**: Optional custom shader for wind (COMPLEX, BETTER effects, NOT REQUIRED)

### 3. Performance Targets ✓ ANSWERED

**Question**: What performance targets?

**Answer**:

- **Map size**: Up to 20×20 tiles (400 tiles), potentially ALL grass
- **Blade count**: 400 tiles × 40-60 blades = 16,000-24,000 entities at medium density
- **FPS target**: 60fps minimum (as low)

**Implications**:

- Culling/LOD are MANDATORY not optional
- Moved from Phase 4 to Phase 2 (critical path)
- Must benchmark with full 400-tile grass coverage

### 4. Remaining Open Questions

1. **Wind Priority**: Is wind animation Phase 5 necessary for initial release or defer to future update?
2. **Instancing**: Worth the shader complexity, or is basic LOD/culling sufficient for 60fps?
3. **SDK Integration**: Should SDK map editor get real-time grass preview, or configuration-only?
4. **Quality Settings UI**: Should players have runtime quality adjustment, or config-file only?

## Risk Mitigation

**Risk**: Grass still doesn't render after Phase 2

- **Mitigation**: Implement robust diagnostic logging, create minimal test scene, compare with working billboard entities

**Risk**: Performance unacceptable with 400 grass tiles

- **Mitigation**: Culling/LOD moved to Phase 2 (mandatory), performance benchmarks required before phase completion, GrassPerformanceLevel provides quality scaling

**Risk**: Custom shader complexity delays project

- **Mitigation**: Billboard approach validated as sufficient for quality, custom shader deferred to optional Phase 5

**Risk**: Density enum refactoring breaks existing saves/maps

- **Mitigation**: Implement backward-compatible deserialization, provide migration tool

## Dependencies

**Blocked By**:

- None (can start immediately)

**Blocks**:

- Foliage decoration system (may reuse grass rendering techniques)
- Particle effects (may share billboard rendering approach)

## Timeline Estimate

- **Phase 1**: 2-3 days (refactoring + diagnosis + density resolution)
- **Phase 2**: 4-6 days (fixing rendering + MANDATORY culling/LOD + performance benchmarks)
- **Phase 3**: 1-2 days (configuration system)
- **Phase 4**: 2-3 days (advanced optimizations - OPTIONAL)
- **Phase 5**: 2-3 days (wind animation - OPTIONAL)

**Total for MVP**: 7-11 days (Phases 1-3, required for 60fps functional grass)
**Total with all enhancements**: 13-17 days (includes optional Phases 4-5)

## References

- `bevy_procedural_grass` source code (shader-based approach)
- `src/game/systems/advanced_trees.rs` (module organization pattern)
- `src/domain/world/types.rs` (metadata structure)
- Bevy documentation on billboards and mesh rendering
- Bevy examples: billboard, instancing, custom materials
