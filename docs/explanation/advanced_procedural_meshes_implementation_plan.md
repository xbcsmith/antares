# Advanced Procedural Meshes Implementation Plan

## Overview

Extend the existing procedural mesh system to generate detailed, organic-looking 3D objects using algorithmic techniques inspired by Veloren's procedural tree generation and modern Bevy mesh construction patterns. This plan replaces simple Bevy primitives (Cylinder+Sphere trees, Cuboid signs) with complex composite meshes featuring natural variation, while preserving the game's 2.5D aesthetic.

**Target Objects**: Trees (multiple types), Shrubs, Grass, Signs, Thrones, Benches, Tables, Chairs, Chests, Torches, and Structures.

**Key Techniques**:

- **Branch graph generation** (inspired by Veloren's `ProceduralTree`)
- **L-system based plant generation** for organic variation
- **Parametric furniture generation** with configurable dimensions
- **TileVisualMetadata integration** for per-tile height/scale control
- **Vertex coloring** for material variation without textures
- **Mesh caching** for performance optimization
- **Configurable quality settings** (grass density: low/medium/high) for older hardware support

> [!IMPORTANT] > **User Requirements Confirmed**:
>
> 1. All terrain objects (trees, mountains, swamps, shrubs, grass, lava) use `TileVisualMetadata` for height/scale customization
> 2. Furniture and props are event-based (matching existing Sign/Portal pattern)
> 3. Grass density must be configurable (low/medium/high) for performance on older hardware

## Current State Analysis

### Existing Infrastructure

**Procedural Mesh Module** ([procedural_meshes.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs)):

- `spawn_tree()`: Cylinder trunk + Sphere foliage (simple composite)
- `spawn_portal()`: Rectangular frame from Cuboids
- `spawn_sign()`: Cylinder post + Cuboid board
- `ProceduralMeshCache`: Caches mesh handles for reuse (6 mesh types)
- **337 lines of implementation + 300 lines of tests**

**Map Integration** ([map.rs](file:///home/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/map.rs)):

- Forest tiles call `spawn_tree()` with position and map_id
- Events call `spawn_portal()` and `spawn_sign()` for MapEvent::Teleport/Sign

**Veloren Reference** (non-blocky procedural generation):

- `ProceduralTree` with branch graph structure (recursive `add_branch()`)
- `TreeConfig` supporting 12+ tree types (oak, pine, birch, baobab, etc.)
- Painter API for rendering branches with variable radii
- L-system style growth with randomized parameters

### Identified Issues

1. **Simple Geometry**: Current trees are sphere+cylinder, lacks natural variation
2. **No Object Variety**: Single tree type, no shrubs/grass/furniture
3. **Missing Organic Shapes**: No branching, no taper, no leaf clusters
4. **No Environmental Props**: Benches, tables, thrones needed for dungeon/town scenes
5. **Limited Customization**: No per-instance variation from visual metadata

## Implementation Phases

### Phase 1: Advanced Tree Generation System

> [!IMPORTANT]
> This phase establishes the core branch graph algorithm that all organic objects will use.

**MANDATORY PREREQUISITES - READ BEFORE STARTING**:

1. **Architecture Document Review** (REQUIRED by AGENTS.md Step 2):

   - **File**: `docs/reference/architecture.md`
   - **Required Sections**: 3.2 (Module Structure), 4 (Core Data Structures), 7 (Data Management)
   - **Verify**: No conflicts with existing domain types (check `MapEvent`, `TileVisualMetadata`)
   - **Verify**: Module placement in `src/game/systems/procedural_meshes.rs` is architecturally sound
   - **Document**: Any architectural decisions in `docs/explanation/implementations.md`
   - **Rule**: If architecture.md defines it, USE IT EXACTLY AS DEFINED

2. **Existing Code Review**:

   - **File**: `src/game/systems/procedural_meshes.rs` (current: 637 lines)
   - **Current Functions**: `spawn_tree()` (Line 143), `spawn_portal()`, `spawn_sign()`
   - **Current Cache**: `ProceduralMeshCache` (Lines 40-50)
   - **Pattern**: Simple composite meshes (Cylinder + Sphere)

3. **Quality Tools Verification**:
   ```bash
   rustup component add clippy rustfmt
   cargo install nextest
   ```

**PHASE DEPENDENCIES**: None (foundational phase)

---

#### 1.1 Branch Graph Data Structure

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: Line 50 (after `ProceduralMeshCache` struct closing brace)  
**Insert Before**: Line 52 (before `impl Default for ProceduralMeshCache`)

**Add Complete Type Definitions**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::math::Vec3;
use bevy::render::primitives::Aabb;

/// Represents a single branch segment in a tree structure
///
/// Branches are connected in a parent-child hierarchy via the `children` field.
/// The root branch (trunk) is always at index 0 in the parent `BranchGraph`.
#[derive(Clone, Debug)]
pub struct Branch {
    /// Starting point of the branch in 3D space (world coordinates)
    pub start: Vec3,

    /// Ending point of the branch in 3D space (world coordinates)
    pub end: Vec3,

    /// Radius at the branch start in world units (thicker at trunk)
    /// Valid range: 0.05 - 0.5
    pub start_radius: f32,

    /// Radius at the branch end in world units (tapers to point)
    /// Valid range: 0.01 - 0.3 (must be <= start_radius)
    pub end_radius: f32,

    /// Indices of child branches in the parent BranchGraph.branches Vec
    /// Empty for leaf branches (endpoints)
    pub children: Vec<usize>,
}

/// Collection of branches forming a complete tree structure
///
/// Branches are stored in a flat Vec with parent-child relationships
/// expressed via indices in the `children` field.
///
/// # Invariants
///
/// - `branches[0]` is always the root/trunk
/// - All indices in `Branch::children` are valid (< branches.len())
/// - No cycles in parent-child relationships
#[derive(Clone, Debug)]
pub struct BranchGraph {
    /// All branches in the tree (index 0 is root/trunk)
    pub branches: Vec<Branch>,

    /// Bounding box for the entire tree structure (for culling)
    pub bounds: Aabb,
}

impl BranchGraph {
    /// Creates a new empty branch graph
    pub fn new() -> Self {
        Self {
            branches: Vec::new(),
            bounds: Aabb::default(),
        }
    }

    /// Adds a branch to the graph and returns its index
    ///
    /// # Arguments
    ///
    /// * `branch` - The branch to add
    ///
    /// # Returns
    ///
    /// Index of the added branch (for use in parent's `children` field)
    pub fn add_branch(&mut self, branch: Branch) -> usize {
        let index = self.branches.len();
        self.branches.push(branch);
        index
    }

    /// Calculates and updates the bounding box for all branches
    ///
    /// Should be called after all branches are added.
    pub fn update_bounds(&mut self) {
        if self.branches.is_empty() {
            self.bounds = Aabb::default();
            return;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for branch in &self.branches {
            min = min.min(branch.start).min(branch.end);
            max = max.max(branch.start).max(branch.end);
        }

        self.bounds = Aabb::from_min_max(min, max);
    }
}

impl Default for BranchGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration parameters for procedural tree generation
///
/// Different tree types (oak, pine, etc.) use different configs.
#[derive(Clone, Debug)]
pub struct TreeConfig {
    /// Base radius of the trunk at ground level in world units
    /// Valid range: 0.1 - 0.5
    /// Default: 0.3
    pub trunk_radius: f32,

    /// Total height of the tree from ground to top in world units
    /// Valid range: 2.0 - 6.0
    /// Default: 3.5
    pub height: f32,

    /// Range for branch angle deviation from parent (min_degrees, max_degrees)
    /// Valid range: (10.0, 90.0)
    /// Default: (30.0, 60.0)
    pub branch_angle_range: (f32, f32),

    /// Maximum recursion depth for branch generation
    /// Valid range: 1 - 5 (higher = more detailed but slower)
    /// Default: 3
    pub depth: u32,

    /// Density of foliage spheres at branch endpoints
    /// Valid range: 0.0 - 1.0 (0.0 = no foliage, 1.0 = maximum density)
    /// Default: 0.7
    pub foliage_density: f32,

    /// Color of foliage as RGB tuple
    /// Valid range: (0.0-1.0, 0.0-1.0, 0.0-1.0)
    /// Default: (0.2, 0.6, 0.2) - green
    pub foliage_color: (f32, f32, f32),
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            trunk_radius: 0.3,
            height: 3.5,
            branch_angle_range: (30.0, 60.0),
            depth: 3,
            foliage_density: 0.7,
            foliage_color: (0.2, 0.6, 0.2),
        }
    }
}

/// Per-tile visual configuration derived from TileVisualMetadata
///
/// This struct adapts domain-layer TileVisualMetadata to application-layer
/// rendering parameters.
#[derive(Clone, Debug)]
pub struct TerrainVisualConfig {
    /// Scale multiplier for tree size
    /// Valid range: 0.5 - 2.0
    /// Default: 1.0
    pub scale: f32,

    /// Height multiplier for tree height
    /// Valid range: 0.5 - 2.0
    /// Default: 1.0
    pub height_multiplier: f32,

    /// Optional color tint applied to foliage
    /// If Some, multiplies foliage_color in TreeConfig
    pub color_tint: Option<Color>,

    /// Rotation around Y-axis in degrees
    /// Valid range: 0.0 - 360.0
    /// Default: 0.0
    pub rotation_y: f32,
}

impl Default for TerrainVisualConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            height_multiplier: 1.0,
            color_tint: None,
            rotation_y: 0.0,
        }
    }
}

impl From<&TileVisualMetadata> for TerrainVisualConfig {
    fn from(meta: &TileVisualMetadata) -> Self {
        Self {
            scale: meta.scale.unwrap_or(1.0),
            height_multiplier: meta.height.unwrap_or(2.0) / 2.0, // Normalize to multiplier
            color_tint: meta.color_tint.map(|(r, g, b)| Color::srgb(r, g, b)),
            rotation_y: meta.rotation_y.unwrap_or(0.0),
        }
    }
}
```

#### 1.2 Tree Type Configurations

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: `TerrainVisualConfig` implementation (from 1.1)

**Add Complete Enum and Implementation**:

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Enumeration of all available tree types with distinct visual characteristics
///
/// Each tree type has a predefined configuration optimized for its appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreeType {
    /// Thick trunk, wide spread branches, dense spherical foliage
    /// Use for: Default forests, temperate biomes
    Oak,

    /// Tall trunk, conical shape, short upward-angled branches
    /// Use for: Mountain biomes, cold regions
    Pine,

    /// Thin trunk, graceful drooping branches, sparse foliage
    /// Use for: Decorative areas, elegant scenes
    Birch,

    /// Thick curved trunk, long drooping branches, curtain-like foliage
    /// Use for: Water areas, swamps, mystical forests
    Willow,

    /// Dark twisted branches, no foliage, decay appearance
    /// Use for: Haunted areas, decay zones, dead forests
    Dead,

    /// Multi-stem low profile, bushy appearance
    /// Use for: Undergrowth, small vegetation
    Shrub,
}

impl TreeType {
    /// Returns the TreeConfig for this tree type
    ///
    /// Configurations are optimized for visual distinctiveness.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::procedural_meshes::{TreeType, TreeConfig};
    ///
    /// let oak_config = TreeType::Oak.config();
    /// assert_eq!(oak_config.trunk_radius, 0.3);
    /// assert_eq!(oak_config.height, 3.5);
    /// ```
    pub fn config(&self) -> TreeConfig {
        match self {
            TreeType::Oak => TreeConfig {
                trunk_radius: 0.3,
                height: 3.5,
                branch_angle_range: (30.0, 60.0),
                depth: 4,
                foliage_density: 0.8,
                foliage_color: (0.2, 0.6, 0.2), // Medium green
            },
            TreeType::Pine => TreeConfig {
                trunk_radius: 0.2,
                height: 5.0,
                branch_angle_range: (20.0, 40.0),
                depth: 3,
                foliage_density: 0.6,
                foliage_color: (0.1, 0.4, 0.1), // Dark green
            },
            TreeType::Birch => TreeConfig {
                trunk_radius: 0.15,
                height: 4.0,
                branch_angle_range: (40.0, 70.0),
                depth: 3,
                foliage_density: 0.5,
                foliage_color: (0.3, 0.7, 0.3), // Light green
            },
            TreeType::Willow => TreeConfig {
                trunk_radius: 0.35,
                height: 4.5,
                branch_angle_range: (60.0, 90.0), // Drooping
                depth: 4,
                foliage_density: 0.9,
                foliage_color: (0.25, 0.65, 0.25),
            },
            TreeType::Dead => TreeConfig {
                trunk_radius: 0.3,
                height: 3.0,
                branch_angle_range: (20.0, 80.0), // Chaotic
                depth: 2,
                foliage_density: 0.0, // No leaves
                foliage_color: (0.0, 0.0, 0.0), // N/A
            },
            TreeType::Shrub => TreeConfig {
                trunk_radius: 0.05, // Very thin stems
                height: 0.8,
                branch_angle_range: (30.0, 50.0),
                depth: 2,
                foliage_density: 0.95, // Very dense
                foliage_color: (0.2, 0.5, 0.2),
            },
        }
    }

    /// Returns display name for UI/debugging
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::procedural_meshes::TreeType;
    ///
    /// assert_eq!(TreeType::Oak.name(), "Oak");
    /// assert_eq!(TreeType::Dead.name(), "Dead Tree");
    /// ```
    pub fn name(&self) -> &str {
        match self {
            TreeType::Oak => "Oak",
            TreeType::Pine => "Pine",
            TreeType::Birch => "Birch",
            TreeType::Willow => "Willow",
            TreeType::Dead => "Dead Tree",
            TreeType::Shrub => "Shrub",
        }
    }

    /// Returns all tree types for iteration
    pub fn all() -> &'static [TreeType] {
        &[
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Dead,
            TreeType::Shrub,
        ]
    }
}
````

**Tree Type Characteristics** (for reference):

| Tree Type | Trunk                       | Branches                | Foliage                   | Use Case       |
| --------- | --------------------------- | ----------------------- | ------------------------- | -------------- |
| `Oak`     | Thick (0.3), short (3.5)    | Wide spread, depth 4    | Dense (0.8), green        | Default forest |
| `Pine`    | Medium (0.2), tall (5.0)    | Conical, depth 3        | Medium (0.6), dark green  | Mountain/cold  |
| `Birch`   | Thin (0.15), medium (4.0)   | Graceful droop, depth 3 | Sparse (0.5), light green | Decorative     |
| `Willow`  | Thick (0.35), tall (4.5)    | Long drooping, depth 4  | Dense (0.9), green        | Water areas    |
| `Dead`    | Thick (0.3), short (3.0)    | Twisted, depth 2        | None (0.0)                | Haunted/decay  |
| `Shrub`   | Very thin (0.05), low (0.8) | Short bushy, depth 2    | Very dense (0.95), green  | Undergrowth    |

#### 1.3 Branch Mesh Generation

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: `TreeType` implementation (from 1.2)

**Add Function**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::render::mesh::{Mesh, Indices};
use bevy::render::render_asset::RenderAssetUsages;

/// Generates a complete tree mesh from a branch graph
///
/// # Arguments
///
/// * `graph` - The branch graph structure to convert to mesh
/// * `config` - Tree configuration (for foliage color, etc.)
///
/// # Returns
///
/// A Bevy `Mesh` with vertices, normals, and vertex colors
///
/// # Implementation Details
///
/// 1. For each branch, generate tapered cylinder vertices
/// 2. Use smooth radius transitions for natural connections
/// 3. Add vertex colors based on height (bark gradient)
/// 4. Generate foliage spheres at branch endpoints with `foliage_density > 0`
pub fn generate_branch_mesh(graph: &BranchGraph, config: &TreeConfig) -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // TODO: Implement vertex generation for each branch
    // - Generate tapered cylinder between start and end
    // - Calculate normals for lighting
    // - Apply bark color gradient based on height
    // - Add foliage spheres at leaf nodes

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
```

**Implementation Steps**:

1. For each branch, generate tapered cylinder vertices (8-16 segments around)
2. Use `line_two_radius()` pattern from Veloren for smooth connections between parent/child
3. Add vertex colors based on height (bark gradient: dark brown at base → lighter at top)
4. Generate foliage spheres at branch endpoints where `config.foliage_density > 0`

#### 1.4 Integration with Existing System

**File**: `src/game/systems/procedural_meshes.rs`

**Modification 1: Update `spawn_tree()` Function Signature**

**Location**: Line 143-153 (current function signature)  
**Change**: Add `visual_metadata` parameter and `tree_type` parameter

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Spawns a procedurally generated tree
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - World position (tile coordinates)
/// * `map_id` - Map identifier
/// * `visual_metadata` - Optional per-tile visual customization
/// * `tree_type` - Type of tree to generate (defaults to Oak if None)
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the spawned tree
pub fn spawn_tree(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    tree_type: Option<TreeType>,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Implementation will use visual_metadata to customize tree appearance
    // and tree_type to select configuration
    todo!("Update implementation to use branch graph")
}
```

**Modification 2: Extend `ProceduralMeshCache` Structure**

**Location**: Lines 40-50 (ProceduralMeshCache struct)  
**Change**: Add new HashMap fields for advanced meshes

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

/// Cache for procedurally generated meshes to improve performance
#[derive(Default)]
pub struct ProceduralMeshCache {
    // EXISTING FIELDS - DO NOT REMOVE (backward compatibility)
    /// Cached trunk mesh handle for simple trees
    tree_trunk: Option<Handle<Mesh>>,
    /// Cached foliage mesh handle for simple trees
    tree_foliage: Option<Handle<Mesh>>,
    /// Cached horizontal bar mesh handle for portals (top/bottom)
    portal_frame_horizontal: Option<Handle<Mesh>>,
    /// Cached vertical bar mesh handle for portals (left/right)
    portal_frame_vertical: Option<Handle<Mesh>>,
    /// Cached cylinder mesh handle for sign posts
    sign_post: Option<Handle<Mesh>>,
    /// Cached cuboid mesh handle for sign boards
    sign_board: Option<Handle<Mesh>>,

    // NEW FIELDS - ADVANCED PROCEDURAL MESHES
    /// Cached tree meshes by type (key: TreeType, value: mesh handle)
    tree_meshes: HashMap<TreeType, Handle<Mesh>>,
}
```

**Modification 3: Update Forest Tile Spawning**

**File**: `src/game/systems/map.rs`  
**Location**: Where forest tiles spawn trees (search for "spawn_tree" calls)  
**Change**: Pass `TileVisualMetadata` from tile to control tree variant

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Example update (exact location will vary):
if tile.terrain == TerrainType::Forest {
    procedural_meshes::spawn_tree(
        &mut commands,
        &mut materials,
        &mut meshes,
        position,
        map_id,
        tile.visual.as_ref(), // Pass visual metadata
        None, // Use default tree type (Oak)
        &mut cache,
    );
}
```

#### 1.5 Testing Requirements

**Unit Tests** (add to existing test module in `src/game/systems/procedural_meshes.rs`):

**File**: `src/game/systems/procedural_meshes.rs`  
**Location**: Bottom of file (in `#[cfg(test)] mod tests` section)

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_graph_new_creates_empty_structure() {
        let graph = BranchGraph::new();
        assert_eq!(graph.branches.len(), 0);
    }

    #[test]
    fn test_tree_type_oak_config_returns_correct_parameters() {
        let config = TreeType::Oak.config();
        assert_eq!(config.trunk_radius, 0.3);
        assert_eq!(config.height, 3.5);
        assert_eq!(config.depth, 4);
    }

    #[test]
    fn test_tree_type_pine_config_returns_correct_parameters() {
        let config = TreeType::Pine.config();
        assert_eq!(config.trunk_radius, 0.2);
        assert_eq!(config.height, 5.0);
    }

    #[test]
    fn test_branch_mesh_generation_produces_valid_mesh() {
        let mut graph = BranchGraph::new();
        graph.add_branch(Branch {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 1.0, 0.0),
            start_radius: 0.3,
            end_radius: 0.1,
            children: vec![],
        });

        let config = TreeConfig::default();
        let mesh = generate_branch_mesh(&graph, &config);

        assert!(mesh.count_vertices() > 0);
    }

    #[test]
    fn test_tree_type_all_variants_present() {
        let all_types = TreeType::all();
        assert_eq!(all_types.len(), 6);
        assert!(all_types.contains(&TreeType::Oak));
        assert!(all_types.contains(&TreeType::Pine));
        assert!(all_types.contains(&TreeType::Shrub));
    }
}
```

**Integration Test** (new file `tests/procedural_tree_test.rs`):

**File**: `tests/procedural_tree_test.rs` (CREATE NEW)

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::game::systems::procedural_meshes::{spawn_tree, TreeType, ProceduralMeshCache};
use antares::domain::world::{Position, MapId, TileVisualMetadata};
use bevy::prelude::*;

#[test]
fn test_spawn_advanced_tree_creates_entity_with_components() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let mut cache = ProceduralMeshCache::default();

    // Test that spawning a tree creates a valid entity
    // Implementation will verify entity has Mesh, Material, Transform components
}

#[test]
fn test_tree_type_visual_difference_detectable() {
    // Test that Oak and Pine trees produce different meshes
    // Compare vertex counts or mesh bounds to verify visual difference
}
```

**Quality Gates** (ALL MUST PASS - NO EXCEPTIONS):

| Command                                                    | Expected Result                       | Failure Action                     |
| ---------------------------------------------------------- | ------------------------------------- | ---------------------------------- |
| `cargo fmt --all`                                          | No output (all files formatted)       | Re-run after editing files         |
| `cargo check --all-targets --all-features`                 | "Finished" with 0 errors              | Fix compilation errors immediately |
| `cargo clippy --all-targets --all-features -- -D warnings` | "Finished" with 0 warnings            | Fix each warning, re-run           |
| `cargo nextest run --all-features`                         | "test result: ok. X passed; 0 failed" | Fix failing tests, do not skip     |

**IF ANY COMMAND FAILS, STOP AND FIX BEFORE PROCEEDING TO NEXT PHASE.**

**Manual Verification**:

1. Run `cargo run --bin antares`
2. Load map with Forest tiles
3. Verify trees show branching structure (not simple sphere+cylinder)
4. Verify variation between individual trees
5. Verify FPS stable (no regression >5%)

#### 1.6 Deliverables

Update this checklist as deliverables are completed:

- [] `Branch` struct defined with complete implementation
- [] `BranchGraph` struct defined with add_branch() and update_bounds() methods
- [] `TreeConfig` struct defined with Default implementation
- [] `TerrainVisualConfig` struct defined with From<&TileVisualMetadata> trait
- [] `TreeType` enum with all 6 variants (Oak, Pine, Birch, Willow, Dead, Shrub)
- [] TreeType::config() method implemented for all variants
- [] TreeType::name() and all() methods implemented
- [] `generate_branch_mesh()` function implemented
- [] `spawn_tree()` function signature updated with new parameters
- [] `ProceduralMeshCache` extended with tree_meshes HashMap
- [] 5+ unit tests passing (branch graph, tree configs, mesh generation)
- [] 2 integration tests passing (spawn tree, visual difference)
- [] All quality gates passing (fmt, check, clippy, nextest)
- [] Manual verification completed (branching visible, variation present)
- [] `docs/explanation/implementations.md` updated with Phase 1 summary

#### 1.7 Success Criteria

- Trees render with visible branch structure
- Multiple tree types visually distinguishable
- No performance regression >5% FPS
- All quality gates passing

---

### Phase 2: Vegetation Systems (Shrubs & Grass)

**PHASE DEPENDENCIES**:

- Phase 1 MUST be complete (requires `TreeConfig`, `BranchGraph`, `TreeType` infrastructure)
- All Phase 1 quality gates MUST pass
- Phase 1 tests MUST achieve >80% coverage
- Phase 1 deliverables checked off and verified

**CANNOT START UNTIL**: Phase 1 deliverables verified by user

---

#### 2.1 Shrub Generation

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: `spawn_tree()` function

Implement multi-stem shrub using branch graph with no central trunk:

| Parameter         | Metadata Field              | Values       | Description                 |
| ----------------- | --------------------------- | ------------ | --------------------------- |
| `stem_count`      | `scale`                     | 3-7          | Number of stems from ground |
| `stem_angle`      | Fixed                       | 20-45°       | Outward lean                |
| `height`          | `TileVisualMetadata.height` | 0.4-0.8      | Short/Medium/Tall variants  |
| `foliage_density` | `scale`                     | Low/Med/High | Dense leaf clusters         |

**Add Function**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Spawns a procedurally generated shrub
///
/// Shrubs use multi-stem branch graphs with no central trunk.
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - World position (tile coordinates)
/// * `map_id` - Map identifier
/// * `visual_metadata` - Optional per-tile visual customization (height controls size)
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the spawned shrub
pub fn spawn_shrub(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Implementation: Use TreeType::Shrub config
    // Generate 3-7 stems from ground level
    // Apply visual_metadata.height for size variation
    todo!("Implement shrub generation")
}
```

> [!NOTE]
> Shrub height controlled by `TileVisualMetadata.height` field (default: 0.6, range: 0.4-0.8)

#### 2.2 Grass Density Quality Settings

> [!IMPORTANT]
> Grass density is configurable to support older hardware. Settings stored in game configuration.

**Resource**: `GrassQualitySettings` (added to game config)

| Setting  | Blades/Tile | Billboard Count | Target Hardware                     |
| -------- | ----------- | --------------- | ----------------------------------- |
| `Low`    | 2-4         | Minimal         | Older hardware, integrated graphics |
| `Medium` | 6-10        | Moderate        | Standard desktop                    |
| `High`   | 12-20       | Dense           | Modern gaming hardware              |

**Grass Generation Parameters**:

| Parameter         | Metadata Control                | Description                |
| ----------------- | ------------------------------- | -------------------------- |
| `blade_count`     | `GrassQualitySettings`          | From config, not per-tile  |
| `blade_height`    | `TileVisualMetadata.height`     | 0.2-0.6 (short/tall grass) |
| `sway_offset`     | Random                          | Wind animation seed        |
| `color_variation` | `TileVisualMetadata.color_tint` | Per-tile grass tint        |

**Note**: Grass uses `Billboard` component for camera-facing, matching character sprite pattern.

**Step 1: Define Resource Type**

**File**: `src/game/config.rs` (or create if doesn't exist)  
**Insert After**: Existing game configuration structs

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Resource controlling grass blade density for performance tuning
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct GrassQualitySettings {
    pub density: GrassDensity,
}

impl Default for GrassQualitySettings {
    fn default() -> Self {
        Self {
            density: GrassDensity::Medium, // Default to balanced setting
        }
    }
}

/// Grass blade density levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassDensity {
    /// 2-4 blades per tile (older hardware, integrated graphics)
    Low,
    /// 6-10 blades per tile (standard desktop)
    Medium,
    /// 12-20 blades per tile (modern gaming hardware)
    High,
}

impl GrassDensity {
    /// Returns the range of grass blades to spawn per tile
    pub fn blade_count_range(&self) -> (u32, u32) {
        match self {
            Self::Low => (2, 4),
            Self::Medium => (6, 10),
            Self::High => (12, 20),
        }
    }

    /// Returns display name for UI
    pub fn name(&self) -> &str {
        match self {
            Self::Low => "Low (2-4 blades)",
            Self::Medium => "Medium (6-10 blades)",
            Self::High => "High (12-20 blades)",
        }
    }
}
```

**Step 2: Register Resource**

**File**: `src/game/plugin.rs` (or main game initialization file)  
**Modify**: Add resource initialization in plugin setup

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// In GamePlugin::build() or similar
app.init_resource::<GrassQualitySettings>();
```

**Step 3: Add spawn_grass Function**

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: `spawn_shrub()` function

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Spawns grass blades on a grass terrain tile
///
/// Grass uses billboard quads that face the camera for performance.
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world space
/// * `map_id` - Map identifier for organization
/// * `visual_metadata` - Optional per-tile visual customization (height = blade height)
/// * `quality_settings` - Grass density configuration
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the parent grass entity
pub fn spawn_grass(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    quality_settings: &GrassQualitySettings,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Implementation: Generate blade_count grass blades based on quality_settings
    // Use Billboard component for camera-facing
    // Apply visual_metadata.height for blade height (0.2-0.6)
    // Apply visual_metadata.color_tint for grass color variation
    todo!("Implement grass generation")
}
```

#### 2.3 Extended Terrain Integration

> [!NOTE]
> All terrain types support `TileVisualMetadata` for height customization.

| Terrain Type | Spawned Objects               | Metadata Fields Used                           |
| ------------ | ----------------------------- | ---------------------------------------------- |
| `Forest`     | 1 Tree + 0-2 Shrubs + Grass   | `height` (tree size), `scale` (tree type hint) |
| `Grass`      | Grass only (enhanced density) | `height` (blade height), `color_tint`          |
| `Swamp`      | Murky water + dead trees      | `height` (water level), `scale` (tree decay)   |
| `Mountain`   | Rock formations               | `height` (peak height), `rotation_y`           |
| `Lava`       | Lava pool + ember particles   | `height` (pool depth), emissive intensity      |
| `Garden`     | Shrubs + Grass + Flowers      | `height`, `color_tint`                         |

#### 2.4 Testing Requirements

**Unit Tests** (add to `src/game/systems/procedural_meshes.rs` test module):

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

#[test]
fn test_shrub_generation_stem_count_within_range() {
    // Verify shrubs generate 3-7 stems
}

#[test]
fn test_grass_blade_count_matches_quality_setting() {
    let low = GrassDensity::Low;
    let (min, max) = low.blade_count_range();
    assert_eq!(min, 2);
    assert_eq!(max, 4);
}

#[test]
fn test_grass_quality_settings_default_is_medium() {
    let settings = GrassQualitySettings::default();
    assert_eq!(settings.density, GrassDensity::Medium);
}
```

**Quality Gates** (ALL MUST PASS):

| Command                                                    | Expected Result |
| ---------------------------------------------------------- | --------------- |
| `cargo fmt --all`                                          | No output       |
| `cargo check --all-targets --all-features`                 | 0 errors        |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 warnings      |
| `cargo nextest run --all-features`                         | All tests pass  |

**Manual Verification**:

1. Walk through Forest tiles - verify mixed vegetation (trees + shrubs)
2. Observe Grass tiles - verify grass blades visible and density correct
3. Check FPS stability with dense grass areas (should be >30 FPS on target hardware)
4. Test all 3 grass quality settings (Low/Medium/High) - verify blade counts differ

#### 2.5 Deliverables

- [] `GrassQualitySettings` resource defined in `src/game/config.rs`
- [] `GrassDensity` enum with all 3 variants (Low, Medium, High)
- [] Resource registered in game plugin initialization
- [] `spawn_shrub()` function implemented with multi-stem generation
- [] `spawn_grass()` function implemented with billboard components
- [] Map spawning updated to call spawn_shrub() and spawn_grass()
- [] Unit tests passing (shrub stems, grass density, quality settings)
- [] All quality gates passing
- [] Performance verified (grass doesn't tank FPS below 30)
- [] `docs/explanation/implementations.md` updated with Phase 2 summary

#### 2.6 Success Criteria

- Shrubs render with multi-stem organic appearance
- Grass visible on appropriate terrain types
- FPS stable on maps with 100+ grass tiles

---

### Phase 3: Furniture & Props Generation

#### 3.1 Parametric Furniture System

Create configurable furniture generator for dungeon/town props:

| Object     | Composition                         | Key Parameters                    |
| ---------- | ----------------------------------- | --------------------------------- |
| **Bench**  | Plank seat + 2 leg cuboids          | `length`, `height`, `wood_color`  |
| **Table**  | Flat top + 4 legs                   | `width`, `depth`, `height`        |
| **Chair**  | Seat + back + 4 legs                | `height`, `has_armrests`          |
| **Throne** | Ornate chair + tall back + armrests | `material`, `ornamentation_level` |
| **Chest**  | Box + lid + hinges + optional lock  | `size`, `material`, `locked`      |
| **Torch**  | Cylinder handle + flame mesh        | `height`, `lit` (emissive if lit) |

#### 3.2 Function Signatures

```rust
pub fn spawn_bench(commands, materials, meshes, position, map_id, config: BenchConfig, cache) -> Entity
pub fn spawn_table(commands, materials, meshes, position, map_id, config: TableConfig, cache) -> Entity
pub fn spawn_chair(commands, materials, meshes, position, map_id, config: ChairConfig, cache) -> Entity
pub fn spawn_throne(commands, materials, meshes, position, map_id, config: ThroneConfig, cache) -> Entity
pub fn spawn_chest(commands, materials, meshes, position, map_id, config: ChestConfig, cache) -> Entity
pub fn spawn_torch(commands, materials, meshes, position, map_id, config: TorchConfig, cache) -> Entity
```

#### 3.3 Event-Based Integration

> [!IMPORTANT]
> All furniture and props are spawned via MapEvents (confirmed by user). This matches existing Sign/Portal pattern.

**CRITICAL: ARCHITECTURE VERIFICATION REQUIRED BEFORE IMPLEMENTATION**

**Step 1: Check Architecture Document**

**File to Read**: `docs/reference/architecture.md`  
**Section**: 4 (Core Data Structures) - check if `MapEvent` is defined as immutable core type  
**Location**: `src/domain/world/types.rs` Line 810 (MapEvent enum)

**Questions to Answer**:

1. Is `MapEvent` in the domain layer? (YES = requires approval to modify)
2. Are there existing event types that can be reused? (Check for `Decoration`, `Prop`, etc.)
3. Is there a pattern for extending events without modifying core enum?

**Step 2A: IF MapEvent CAN Be Extended (User Approval Obtained)**

**File**: `src/domain/world/types.rs`  
**Modify**: `MapEvent` enum (around Line 810)  
**Action**: Add new variant

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub enum MapEvent {
    // ... existing variants (Encounter, Treasure, Teleport, etc.) ...

    /// Furniture or prop placement event
    Furniture {
        /// Event name for editor display
        #[serde(default)]
        name: String,
        /// Type of furniture to spawn
        furniture_type: FurnitureType,
        /// Optional Y-axis rotation in degrees (0-360)
        #[serde(default)]
        rotation_y: Option<f32>,
    },
}

/// Types of furniture and props that can be placed
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum FurnitureType {
    Throne,
    Bench,
    Table,
    Chair,
    Torch,
    Bookshelf,
    Barrel,
    Chest,
}
```

**Step 2B: IF MapEvent CANNOT Be Extended (Alternative Pattern)**

Use existing event types with naming conventions:

| Furniture Type | Event Type to Use    | Name Pattern        | Spawning Logic                         |
| -------------- | -------------------- | ------------------- | -------------------------------------- |
| Throne         | `Sign` or `Treasure` | `name: "throne"`    | Pattern match on name in event handler |
| Bench          | `Sign`               | `name: "bench"`     | Pattern match on name                  |
| Table          | `Sign`               | `name: "table"`     | Pattern match on name                  |
| Chair          | `Sign`               | `name: "chair"`     | Pattern match on name                  |
| Torch          | `Sign`               | `name: "torch"`     | Pattern match on name                  |
| Chest          | `Treasure`           | (existing)          | Use existing Treasure event            |
| Bookshelf      | `Sign`               | `name: "bookshelf"` | Pattern match on name                  |
| Barrel         | `Sign`               | `name: "barrel"`    | Pattern match on name                  |

**Implementation Note**: Pattern matching in `spawn_map_event()` checks name prefix/contains.

**Step 3: Event Handler Integration**

**File**: `src/game/systems/map.rs`  
**Function**: `spawn_map_event()` or similar event handling function  
**Modify**: Add pattern matching for furniture events

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

match event {
    MapEvent::Furniture { name, furniture_type, rotation_y } => {
        match furniture_type {
            FurnitureType::Throne => spawn_throne(/* ... */),
            FurnitureType::Bench => spawn_bench(/* ... */),
            FurnitureType::Table => spawn_table(/* ... */),
            // ... etc
        }
    },
    // ... other event types ...
}
```

#### 3.4 Testing Requirements

**Unit Tests**:

```rust
#[test] fn test_bench_config_defaults()
#[test] fn test_throne_ornamentation_levels()
#[test] fn test_torch_emissive_when_lit()
#[test] fn test_chest_lock_component()
```

**Integration Tests**:

```rust
#[test] fn test_furniture_event_spawning()
```

#### 3.5 Deliverables

- [ ] 6 furniture spawn functions implemented
- [ ] Config structs for each furniture type
- [ ] Event pattern matching for automatic furniture spawning
- [ ] Cache entries for furniture meshes
- [ ] Unit tests passing

#### 3.6 Success Criteria

- All furniture types render correctly
- Thrones visually distinct from chairs
- Torches emit light (emissive material)
- Chests show locked/unlocked state

---

### Phase 4: Structure & Architecture Components

#### 4.1 Modular Structure Components

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: Furniture spawn functions

**Add Complete Type Definitions**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Types of architectural structure components
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureType {
    /// Vertical support column
    Column,
    /// Arched opening
    Arch,
    /// Wall segment
    WallSegment,
    /// Door frame
    DoorFrame,
    /// Safety railing
    Railing,
}

/// Column architectural styles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnStyle {
    /// Plain cylindrical column
    Plain,
    /// Classical Doric style (simple capital)
    Doric,
    /// Classical Ionic style (scroll capital)
    Ionic,
}

/// Configuration for column generation
#[derive(Clone, Debug)]
pub struct ColumnConfig {
    /// Height of the column (default: 3.0)
    pub height: f32,
    /// Radius of the column shaft (default: 0.3)
    pub radius: f32,
    /// Architectural style
    pub style: ColumnStyle,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            height: 3.0,
            radius: 0.3,
            style: ColumnStyle::Plain,
        }
    }
}

/// Configuration for arch generation
#[derive(Clone, Debug)]
pub struct ArchConfig {
    /// Width of the arch opening (default: 2.0)
    pub width: f32,
    /// Height to the top of the arch (default: 3.0)
    pub height: f32,
    /// Thickness of the arch structure (default: 0.3)
    pub thickness: f32,
}

impl Default for ArchConfig {
    fn default() -> Self {
        Self {
            width: 2.0,
            height: 3.0,
            thickness: 0.3,
        }
    }
}
```

Implement building blocks for procedural structures:

| Component        | Mesh Type                   | Parameters                                      | Config Struct       |
| ---------------- | --------------------------- | ----------------------------------------------- | ------------------- |
| **Column**       | Cylinder with cap/base      | `height`, `radius`, `style` (doric/ionic/plain) | `ColumnConfig`      |
| **Arch**         | Half-torus + supports       | `width`, `height`, `thickness`                  | `ArchConfig`        |
| **Wall Segment** | Cuboid with optional window | `length`, `height`, `has_window`                | `WallSegmentConfig` |
| **Door Frame**   | Portal-like structure       | `width`, `height`, `material`                   | `DoorFrameConfig`   |
| **Railing**      | Posts + horizontal bars     | `length`, `post_count`                          | `RailingConfig`     |

#### 4.2 Integration Points

**File**: `src/game/systems/procedural_meshes.rs`  
**Insert After**: Structure type definitions

**Add Complete Function Signatures**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Spawns a procedurally generated column
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - World position (tile coordinates)
/// * `map_id` - Map identifier
/// * `config` - Column configuration parameters
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the spawned column
pub fn spawn_column(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    config: ColumnConfig,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    todo!("Implement column generation")
}

/// Spawns a procedurally generated arch
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - World position (tile coordinates)
/// * `map_id` - Map identifier
/// * `config` - Arch configuration parameters
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the spawned arch
pub fn spawn_arch(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    config: ArchConfig,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    todo!("Implement arch generation")
}

// TODO: Add spawn_wall_segment(), spawn_door_frame(), spawn_railing()
```

#### 4.3 Testing Requirements

**Unit Tests**:

```rust
#[test] fn test_column_style_enum()
#[test] fn test_arch_vertex_generation()
```

#### 4.4 Deliverables

- [ ] 5 structure component functions implemented
- [ ] Style configurations for each component
- [ ] Unit tests passing

#### 4.5 Success Criteria

- Structure components render correctly
- Columns show style variations
- Arches connect properly to supports

---

### Phase 5: Performance & Polish

#### 5.1 Mesh Instancing

Implement GPU instancing for repeated objects:

```rust
pub struct InstancedMeshBundle {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    instances: Vec<InstanceData>,
}
```

#### 5.2 LOD System (Level of Detail)

Implement distance-based simplification:

| Distance    | Detail Level                |
| ----------- | --------------------------- |
| < 10 tiles  | Full branch graph           |
| 10-30 tiles | Simplified (fewer branches) |
| > 30 tiles  | Billboard impostor          |

#### 5.3 Async Mesh Generation

For large map areas, generate meshes on background threads:

```rust
let task = AsyncComputeTaskPool::get().spawn(async move {
    generate_complex_tree_mesh(config)
});
```

#### 5.4 Cache Expansion

**File**: `src/game/systems/procedural_meshes.rs`  
**Current Location**: Lines 40-50 (ProceduralMeshCache struct)

**Modification: Extend Cache Structure (Already Done in Phase 1.4)**

The cache was already extended in Phase 1.4 with backward compatibility. Verify the following fields exist:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub struct ProceduralMeshCache {
    // EXISTING FIELDS (from original implementation)
    tree_trunk: Option<Handle<Mesh>>,
    tree_foliage: Option<Handle<Mesh>>,
    portal_frame_horizontal: Option<Handle<Mesh>>,
    portal_frame_vertical: Option<Handle<Mesh>>,
    sign_post: Option<Handle<Mesh>>,
    sign_board: Option<Handle<Mesh>>,

    // NEW FIELDS (added in Phase 1)
    tree_meshes: HashMap<TreeType, Handle<Mesh>>,

    // PHASE 2 ADDITIONS
    shrub_mesh: Option<Handle<Mesh>>,

    // PHASE 3 ADDITIONS
    furniture: HashMap<FurnitureType, Handle<Mesh>>,

    // PHASE 4 ADDITIONS
    structures: HashMap<StructureType, Handle<Mesh>>,
}
```

**Add Helper Methods** (if not already present):

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

impl ProceduralMeshCache {
    /// Gets or creates a furniture mesh for the specified type
    pub fn get_or_create_furniture_mesh<F>(
        &mut self,
        furniture_type: FurnitureType,
        meshes: &mut ResMut<Assets<Mesh>>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        self.furniture
            .entry(furniture_type)
            .or_insert_with(|| meshes.add(creator()))
            .clone()
    }

    /// Gets or creates a structure mesh for the specified type
    pub fn get_or_create_structure_mesh<F>(
        &mut self,
        structure_type: StructureType,
        meshes: &mut ResMut<Assets<Mesh>>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        self.structures
            .entry(structure_type)
            .or_insert_with(|| meshes.add(creator()))
            .clone()
    }
}
```

**Cache Field Summary**:

| Field         | Type                                   | Added in Phase |
| ------------- | -------------------------------------- | -------------- |
| `tree_meshes` | `HashMap<TreeType, Handle<Mesh>>`      | Phase 1        |
| `shrub_mesh`  | `Option<Handle<Mesh>>`                 | Phase 2        |
| `furniture`   | `HashMap<FurnitureType, Handle<Mesh>>` | Phase 3        |
| `structures`  | `HashMap<StructureType, Handle<Mesh>>` | Phase 4        |

#### 5.5 Testing Requirements

**Performance Benchmarks** (`benches/procedural_meshes_bench.rs`):

```rust
#[bench] fn bench_tree_mesh_generation()
#[bench] fn bench_map_with_100_trees()
#[bench] fn bench_instanced_vs_individual()
```

**Quality Gates**:

```bash
cargo bench --bench procedural_meshes_bench
```

#### 5.6 Deliverables

- [ ] Instancing implemented for repeated objects
- [ ] LOD system implemented with 3 detail levels
- [ ] Async generation for complex meshes
- [ ] Cache expanded for all object types
- [ ] Performance benchmarks passing

#### 5.7 Success Criteria

- Maps with 200+ objects maintain 60 FPS
- Memory usage stable (no mesh leaks)
- Load time under 2 seconds for large maps

---

### Phase 6: Campaign Builder SDK - Terrain Visual Configuration

> [!IMPORTANT]
> This phase extends the existing `VisualPreset` system in `map_editor.rs` to support advanced procedural mesh configuration.

#### 6.1 Extended Visual Presets

**File**: `sdk/campaign_builder/src/map_editor.rs`

Add new presets for procedural terrain objects:

| Preset Category | New Presets                                       | TileVisualMetadata Fields                    |
| --------------- | ------------------------------------------------- | -------------------------------------------- |
| **Trees**       | `ShortTree`, `MediumTree`, `TallTree`, `DeadTree` | `height`: 1.0/2.0/3.0, `color_tint`          |
| **Shrubs**      | `SmallShrub`, `LargeShrub`, `FloweringShrub`      | `height`: 0.4/0.6/0.8, `scale`, `color_tint` |
| **Grass**       | `ShortGrass`, `TallGrass`, `DriedGrass`           | `height`: 0.2/0.4/0.6, `color_tint`          |
| **Mountains**   | `LowPeak`, `HighPeak`, `JaggedPeak`               | `height`: 1.5/3.0/5.0, `rotation_y`          |
| **Swamp**       | `ShallowSwamp`, `DeepSwamp`, `MurkySwamp`         | `height`: 0.1/0.3/0.5 (water level)          |
| **Lava**        | `LavaPool`, `LavaFlow`, `VolcanicVent`            | `height`, emissive intensity                 |

```rust
pub enum VisualPreset {
    // ... existing presets ...

    // Tree variants
    ShortTree,
    MediumTree,
    TallTree,
    DeadTree,

    // Shrub variants
    SmallShrub,
    LargeShrub,
    FloweringShrub,

    // Grass variants
    ShortGrass,
    TallGrass,
    DriedGrass,

    // Terrain variants
    LowPeak,
    HighPeak,
    JaggedPeak,
    ShallowSwamp,
    DeepSwamp,
    MurkySwamp,
    LavaPool,
    LavaFlow,
    VolcanicVent,
}
```

#### 6.2 Terrain Visual Inspector Panel

**File**: `sdk/campaign_builder/src/map_editor.rs`

Extend the Inspector Panel to show terrain-specific controls:

| Terrain Type | Inspector Controls                                                |
| ------------ | ----------------------------------------------------------------- |
| Forest       | Tree Type dropdown, Height slider (0.5-4.0), Foliage Color picker |
| Grass        | Height slider (0.1-0.8), Color tint picker                        |
| Mountain     | Peak height slider, Rotation control                              |
| Swamp        | Water level slider, Dead tree toggle                              |
| Lava         | Pool depth slider, Glow intensity slider                          |

#### 6.3 Quality Settings Panel

**File**: `sdk/campaign_builder/src/config_editor.rs`

Add grass quality settings to campaign configuration:

```rust
// In CampaignConfig editing UI
ui.heading("Graphics Quality");
ui.horizontal(|ui| {
    ui.label("Grass Density:");
    egui::ComboBox::from_label("")
        .selected_text(config.grass_density.name())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut config.grass_density, GrassDensity::Low, "Low (2-4 blades)");
            ui.selectable_value(&mut config.grass_density, GrassDensity::Medium, "Medium (6-10 blades)");
            ui.selectable_value(&mut config.grass_density, GrassDensity::High, "High (12-20 blades)");
        });
});
```

#### 6.4 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/visual_preset_tests.rs`):

```rust
#[test] fn test_tree_preset_metadata_values()
#[test] fn test_shrub_preset_metadata_values()
#[test] fn test_grass_preset_metadata_values()
#[test] fn test_terrain_preset_all_variants()
```

**Integration Tests**:

```rust
#[test] fn test_inspector_panel_terrain_controls()
#[test] fn test_config_editor_grass_quality()
```

#### 6.5 Deliverables

- [ ] 18 new `VisualPreset` variants added (trees, shrubs, grass, terrain)
- [ ] Terrain-specific Inspector Panel controls
- [ ] Grass quality settings in Config Editor
- [ ] Unit tests for all new presets
- [ ] Integration tests for UI components

#### 6.6 Success Criteria

- All new presets visible in Map Editor preset dropdown
- Inspector Panel shows terrain-specific controls when tile selected
- Grass density configurable in campaign settings
- Saved maps correctly serialize new metadata values

---

### Phase 7: Campaign Builder SDK - Furniture & Props Event Editor

> [!NOTE]
> This phase adds a dedicated interface for placing and configuring furniture/prop events.

#### 7.1 FurnitureType Enum Integration

**File**: `sdk/campaign_builder/src/map_editor.rs`

Add furniture type selection to event editor:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FurnitureType {
    Throne,
    Bench,
    Table,
    Chair,
    Torch,
    Bookshelf,
    Barrel,
    Chest,
    Crate,
    Altar,
    Statue,
    Fountain,
}

impl FurnitureType {
    pub fn all() -> &'static [FurnitureType] {
        &[
            Self::Throne, Self::Bench, Self::Table, Self::Chair,
            Self::Torch, Self::Bookshelf, Self::Barrel, Self::Chest,
            Self::Crate, Self::Altar, Self::Statue, Self::Fountain,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Throne => "Throne",
            Self::Bench => "Bench",
            Self::Table => "Table",
            Self::Chair => "Chair",
            Self::Torch => "Torch",
            Self::Bookshelf => "Bookshelf",
            Self::Barrel => "Barrel",
            Self::Chest => "Chest",
            Self::Crate => "Crate",
            Self::Altar => "Altar",
            Self::Statue => "Statue",
            Self::Fountain => "Fountain",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Throne => "👑",
            Self::Bench => "🪑",
            Self::Table => "🪵",
            Self::Chair => "💺",
            Self::Torch => "🔥",
            Self::Bookshelf => "📚",
            Self::Barrel => "🛢️",
            Self::Chest => "📦",
            Self::Crate => "📦",
            Self::Altar => "⛪",
            Self::Statue => "🗿",
            Self::Fountain => "⛲",
        }
    }
}
```

#### 7.2 Furniture Event Editor Panel

**File**: `sdk/campaign_builder/src/map_editor.rs`

Add furniture-specific event editing controls:

| Control             | Type             | Description                    |
| ------------------- | ---------------- | ------------------------------ |
| Furniture Type      | ComboBox         | Select from FurnitureType enum |
| Rotation            | Slider (0-360)   | Y-axis rotation in degrees     |
| Scale               | Slider (0.5-2.0) | Size multiplier                |
| Material            | ComboBox         | Wood/Stone/Metal/Gold          |
| Lit (Torch only)    | Checkbox         | Toggle emissive/unlit          |
| Locked (Chest only) | Checkbox         | Locked chest visual            |

#### 7.3 Event Palette Update

**File**: `sdk/campaign_builder/src/map_editor.rs`

Add furniture category to event palette:

```rust
// Event categories for placement tool
pub enum EventCategory {
    Navigation,   // Teleport, Sign
    Character,    // NPC, Recruitable
    Combat,       // Encounter, Trap
    Treasure,     // Treasure, Chest
    Furniture,    // NEW: Throne, Bench, Table, Chair, Torch, etc.
    Decoration,   // NEW: Statue, Fountain, etc.
}
```

#### 7.4 Props Preview System

Add 2D icon preview in map editor (before 3D mesh is rendered in game):

| Furniture | Preview Icon | Color            |
| --------- | ------------ | ---------------- |
| Throne    | 👑           | Gold (#FFD700)   |
| Bench     | 🪑           | Brown (#8B4513)  |
| Table     | 🪵            | Brown (#A0522D)  |
| Chair     | 💺           | Brown (#D2691E)  |
| Torch     | 🔥           | Orange (#FF4500) |
| Chest     | 📦           | Brown (#8B4513)  |

#### 7.5 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/furniture_editor_tests.rs`):

```rust
#[test] fn test_furniture_type_all_variants()
#[test] fn test_furniture_type_icons()
#[test] fn test_furniture_event_serialization()
#[test] fn test_furniture_rotation_range()
```

**Integration Tests**:

```rust
#[test] fn test_furniture_event_placement()
#[test] fn test_furniture_event_editing()
#[test] fn test_furniture_category_palette()
```

#### 7.6 Deliverables

- [ ] `FurnitureType` enum with 12 variants
- [ ] Furniture event editor panel with all controls
- [ ] Event palette updated with Furniture/Decoration categories
- [ ] 2D preview icons for all furniture types
- [ ] Unit tests passing
- [ ] Integration tests passing

#### 7.7 Success Criteria

- Furniture placeable via PlaceEvent tool
- All furniture types selectable from dropdown
- Rotation and scale controls functional
- Events saved correctly to map RON files
- Preview icons visible in map editor grid

---

## Verification Plan

### Automated Tests

| Test Type  | Command                                                    | Expected Result   |
| ---------- | ---------------------------------------------------------- | ----------------- |
| Unit tests | `cargo nextest run --all-features`                         | All passing       |
| Clippy     | `cargo clippy --all-targets --all-features -- -D warnings` | No warnings       |
| Format     | `cargo fmt --all -- --check`                               | No changes needed |
| Benchmarks | `cargo bench --bench procedural_meshes_bench`              | No regressions    |

### Manual Verification

1. **Visual Inspection**:

   - Load tutorial campaign
   - Navigate to Forest areas
   - Verify trees have visible branching (not sphere+cylinder)
   - Verify shrubs appear near trees
   - Verify grass visible on grass terrain

2. **Furniture Check**:

   - Create test map with furniture events
   - Verify benches, tables, chairs render correctly
   - Verify thrones are visually ornate
   - Verify torches emit light

3. **Performance Check**:
   - Load large map (50x50 with many objects)
   - Verify FPS stays above 30
   - Check memory usage in task manager

---

## Dependencies

**Add to `Cargo.toml`**:

```toml
[dependencies]
bevy = "0.15.0"  # Exact version for reproducibility
rand = "0.8.5"   # Random number generation for variation
serde = { version = "1.0", features = ["derive"] }  # Required for FurnitureType, TreeType serialization

[dependencies.noise]
version = "0.9.0"  # Optional: for organic variation in terrain
optional = true

[dev-dependencies]
criterion = "0.5"  # For benchmarking (Phase 5)
```

**Version Compatibility Notes**:

- Bevy 0.15.0 required for `Mesh::new` API and latest ECS features
- Rand 0.8.5 required for `thread_rng()` thread safety
- Noise crate optional but recommended for natural terrain variation
- Serde required for serializing enums in MapEvent

**Crate Summary**:

| Crate       | Purpose                            | Version | Required |
| ----------- | ---------------------------------- | ------- | -------- |
| `bevy`      | Core ECS and rendering             | 0.15.0  | Yes      |
| `rand`      | Random number generation           | 0.8.5   | Yes      |
| `serde`     | Serialization                      | 1.0     | Yes      |
| `noise`     | Perlin noise for organic variation | 0.9.0   | Optional |
| `criterion` | Performance benchmarking           | 0.5     | Dev only |

---

## Transformation Table

| Object | Current Implementation          | New Implementation                                  |
| ------ | ------------------------------- | --------------------------------------------------- |
| Tree   | Cylinder trunk + Sphere foliage | Branch graph with tapered cylinders + leaf clusters |
| Shrub  | N/A                             | Multi-stem branch graph, low profile                |
| Grass  | N/A                             | Billboard quads with vertex animation               |
| Sign   | Cylinder post + Cuboid board    | Enhanced with weathering/detail                     |
| Throne | N/A                             | Ornate chair with armrests + tall back              |
| Bench  | N/A                             | Parametric plank + legs                             |
| Table  | N/A                             | Flat top + 4 legs                                   |
| Chair  | N/A                             | Seat + back + legs                                  |
| Chest  | N/A                             | Box + lid + optional lock                           |
| Torch  | N/A                             | Handle + flame mesh (emissive)                      |
| Column | N/A                             | Cylinder with style variations                      |
| Arch   | N/A                             | Half-torus + supports                               |

---

## Risk Mitigation

| Risk                   | Mitigation                                 |
| ---------------------- | ------------------------------------------ |
| Performance impact     | Mesh caching, LOD system, GPU instancing   |
| Visual inconsistency   | Consistent color palette, shared materials |
| Scope creep            | Clear phase boundaries, defer enhancements |
| Breaking existing maps | Backward-compatible spawn functions        |

---

## Design Decisions (User Confirmed)

| Decision                 | Choice                          | Rationale                                                                  |
| ------------------------ | ------------------------------- | -------------------------------------------------------------------------- |
| Tree/terrain sizing      | **TileVisualMetadata**          | Allows per-tile control of short/medium/tall trees, mountain heights, etc. |
| Furniture/props spawning | **Event-based**                 | Matches existing Sign/Portal pattern, flexible placement in map editor     |
| Grass density            | **Configurable (low/med/high)** | Ensures super performant gameplay on older hardware                        |

---

## TileVisualMetadata Usage Guide

| Terrain Type | `height` Effect                    | `scale` Effect            | `color_tint` Effect |
| ------------ | ---------------------------------- | ------------------------- | ------------------- |
| Forest/Tree  | Trunk height (short=1.0, tall=3.0) | Branch density multiplier | Foliage color       |
| Shrub        | Overall height                     | Stem count multiplier     | Leaf color          |
| Grass        | Blade height                       | N/A (use config)          | Grass tint          |
| Mountain     | Peak height                        | Rock cluster size         | Rock tint           |
| Swamp        | Water surface level                | Tree decay level          | Water murk          |
| Lava         | Pool depth                         | Ember intensity           | Glow color          |
