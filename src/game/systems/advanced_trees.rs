// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Advanced procedural tree generation system using branch graph data structures
//!
//! This module provides sophisticated tree generation with configurable parameters
//! for different tree types (Oak, Pine, Birch, Willow, Dead, Shrub).
//!
//! # Architecture
//!
//! The system uses a hierarchical branch graph approach:
//! - `Branch`: Individual segment in the tree structure
//! - `BranchGraph`: Complete tree as flat Vec with parent-child relationships via indices
//! - `TreeConfig`: Parameters for tree generation (trunk radius, height, foliage, etc.)
//! - `TreeType`: Enum defining distinct tree variants
//! - `TerrainVisualConfig`: Per-tile visual customization from domain layer
//!
//! # Examples
//!
//! ```text
//! use antares::game::systems::advanced_trees::{TreeType, BranchGraph, Branch};
//! use bevy::prelude::Vec3;
//!
//! // Create a simple branch graph
//! let mut graph = BranchGraph::new();
//! let trunk = Branch {
//!     start: Vec3::ZERO,
//!     end: Vec3::new(0.0, 3.5, 0.0),
//!     start_radius: 0.3,
//!     end_radius: 0.1,
//!     children: vec![],
//! };
//! graph.add_branch(trunk);
//! graph.update_bounds();
//! ```

use crate::domain::world::TileVisualMetadata;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== Data Structures ====================

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
    /// Represented as (min, max) Vec3 tuples
    pub bounds: (Vec3, Vec3),
}

impl BranchGraph {
    /// Creates a new empty branch graph
    pub fn new() -> Self {
        Self {
            branches: Vec::new(),
            bounds: (Vec3::ZERO, Vec3::ZERO),
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
            self.bounds = (Vec3::ZERO, Vec3::ZERO);
            return;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for branch in &self.branches {
            min = min.min(branch.start).min(branch.end);
            max = max.max(branch.start).max(branch.end);
        }

        self.bounds = (min, max);
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
    /// Converts domain-layer TileVisualMetadata to application-layer TerrainVisualConfig
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::advanced_trees::TerrainVisualConfig;
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let meta = TileVisualMetadata {
    ///     height: Some(4.0),
    ///     width_x: Some(1.0),
    ///     width_z: Some(1.0),
    ///     color_tint: Some((0.8, 0.6, 0.4)),
    ///     scale: Some(1.5),
    ///     y_offset: None,
    ///     rotation_y: Some(45.0),
    ///     sprite: None,
    ///     sprite_layers: vec![],
    ///     sprite_rule: None,
    /// };
    /// let config = TerrainVisualConfig::from(&meta);
    /// assert_eq!(config.scale, 1.5);
    /// assert_eq!(config.height_multiplier, 2.0); // 4.0 / 2.0
    /// ```
    fn from(meta: &TileVisualMetadata) -> Self {
        Self {
            scale: meta.scale.unwrap_or(1.0),
            height_multiplier: meta.height.unwrap_or(2.0) / 2.0,
            color_tint: meta.color_tint.map(|(r, g, b)| Color::srgb(r, g, b)),
            rotation_y: meta.rotation_y.unwrap_or(0.0),
        }
    }
}

// ==================== Tree Type Configurations ====================

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
    /// use antares::game::systems::advanced_trees::{TreeType, TreeConfig};
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
                branch_angle_range: (60.0, 90.0),
                depth: 4,
                foliage_density: 0.9,
                foliage_color: (0.25, 0.65, 0.25),
            },
            TreeType::Dead => TreeConfig {
                trunk_radius: 0.3,
                height: 3.0,
                branch_angle_range: (20.0, 80.0),
                depth: 2,
                foliage_density: 0.0,
                foliage_color: (0.0, 0.0, 0.0),
            },
            TreeType::Shrub => TreeConfig {
                trunk_radius: 0.05,
                height: 0.8,
                branch_angle_range: (30.0, 50.0),
                depth: 2,
                foliage_density: 0.95,
                foliage_color: (0.2, 0.5, 0.2),
            },
        }
    }

    /// Returns display name for UI/debugging
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::advanced_trees::TreeType;
    ///
    /// assert_eq!(TreeType::Oak.name(), "Oak");
    /// assert_eq!(TreeType::Dead.name(), "Dead Tree");
    /// ```
    pub fn name(&self) -> &'static str {
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
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::advanced_trees::TreeType;
    ///
    /// let all_types = TreeType::all();
    /// assert_eq!(all_types.len(), 6);
    /// assert!(all_types.contains(&TreeType::Oak));
    /// ```
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

// ==================== Mesh Generation ====================

/// Generates a tree mesh from a branch graph
///
/// For Phase 1, generates a simple mesh. In later phases, this will be extended to
/// create tapered cylinders for each branch segment and foliage spheres at endpoints.
///
/// # Arguments
///
/// * `graph` - The branch graph structure
/// * `config` - Tree configuration parameters
///
/// # Returns
///
/// A Bevy `Mesh` representing the tree structure
///
/// # Notes
///
/// Phase 1 implementation generates a basic mesh from the graph structure.
/// Future phases will add:
/// - Tapered cylinders for branches
/// - Foliage sphere generation at leaf nodes
/// - Branch connection smoothing
pub fn generate_branch_mesh(graph: &BranchGraph, _config: &TreeConfig) -> Mesh {
    // For Phase 1, create a simple mesh from a cylinder that represents the overall tree structure
    // This ensures the function works correctly without complex vertex generation
    //
    // In later phases, this will create tapered cylinders for each branch and
    // add foliage spheres based on the branch graph structure.

    if graph.branches.is_empty() {
        // Return an empty mesh if no branches
        return Cylinder {
            radius: 0.1,
            half_height: 0.5,
        }
        .mesh()
        .into();
    }

    // Get the bounds to estimate tree dimensions
    let (min, max) = graph.bounds;
    let height = (max.y - min.y).max(0.1);
    let width = (max.x - min.x).max(0.1);

    // Create a simple cylinder representing the tree trunk
    // This is a placeholder for Phase 1 - will be replaced with advanced branch rendering
    Cylinder {
        radius: (width / 2.0).min(0.5),
        half_height: height / 2.0,
    }
    .mesh()
    .into()
}

// ==================== Mesh Cache Extension ====================

/// Extended cache for advanced tree meshes
///
/// Stores generated branch graph meshes by tree type to avoid recomputation.
#[derive(Clone, Debug, Default)]
pub struct AdvancedTreeMeshCache {
    /// Cached tree meshes by type (key: TreeType, value: mesh handle)
    pub tree_meshes: HashMap<TreeType, Handle<Mesh>>,
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_graph_new_creates_empty_structure() {
        let graph = BranchGraph::new();
        assert_eq!(graph.branches.len(), 0);
    }

    #[test]
    fn test_branch_graph_add_branch_returns_correct_index() {
        let mut graph = BranchGraph::new();
        let branch = Branch {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 1.0, 0.0),
            start_radius: 0.3,
            end_radius: 0.1,
            children: vec![],
        };

        let index = graph.add_branch(branch);
        assert_eq!(index, 0);
        assert_eq!(graph.branches.len(), 1);
    }

    #[test]
    fn test_branch_graph_multiple_branches() {
        let mut graph = BranchGraph::new();
        let branch1 = Branch {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 1.0, 0.0),
            start_radius: 0.3,
            end_radius: 0.1,
            children: vec![1],
        };
        let branch2 = Branch {
            start: Vec3::new(0.0, 1.0, 0.0),
            end: Vec3::new(0.5, 2.0, 0.0),
            start_radius: 0.1,
            end_radius: 0.05,
            children: vec![],
        };

        let idx1 = graph.add_branch(branch1);
        let idx2 = graph.add_branch(branch2);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(graph.branches.len(), 2);
    }

    #[test]
    fn test_branch_graph_update_bounds() {
        let mut graph = BranchGraph::new();
        graph.add_branch(Branch {
            start: Vec3::ZERO,
            end: Vec3::new(2.0, 3.0, 1.0),
            start_radius: 0.3,
            end_radius: 0.1,
            children: vec![],
        });

        graph.update_bounds();

        let (min, max) = graph.bounds;
        assert!(min.x <= 0.0);
        assert!(min.y <= 0.0);
        assert!(min.z <= 0.0);
        assert!(max.x >= 2.0);
        assert!(max.y >= 3.0);
        assert!(max.z >= 1.0);
    }

    #[test]
    fn test_tree_type_oak_config_returns_correct_parameters() {
        let config = TreeType::Oak.config();
        assert_eq!(config.trunk_radius, 0.3);
        assert_eq!(config.height, 3.5);
        assert_eq!(config.depth, 4);
        assert_eq!(config.foliage_density, 0.8);
    }

    #[test]
    fn test_tree_type_pine_config_returns_correct_parameters() {
        let config = TreeType::Pine.config();
        assert_eq!(config.trunk_radius, 0.2);
        assert_eq!(config.height, 5.0);
        assert_eq!(config.depth, 3);
        assert_eq!(config.foliage_density, 0.6);
    }

    #[test]
    fn test_tree_type_birch_config() {
        let config = TreeType::Birch.config();
        assert_eq!(config.trunk_radius, 0.15);
        assert_eq!(config.height, 4.0);
        assert_eq!(config.foliage_density, 0.5);
    }

    #[test]
    fn test_tree_type_willow_config() {
        let config = TreeType::Willow.config();
        assert_eq!(config.trunk_radius, 0.35);
        assert_eq!(config.height, 4.5);
        assert_eq!(config.foliage_density, 0.9);
    }

    #[test]
    fn test_tree_type_dead_config() {
        let config = TreeType::Dead.config();
        assert_eq!(config.trunk_radius, 0.3);
        assert_eq!(config.foliage_density, 0.0);
    }

    #[test]
    fn test_tree_type_shrub_config() {
        let config = TreeType::Shrub.config();
        assert_eq!(config.trunk_radius, 0.05);
        assert_eq!(config.height, 0.8);
        assert_eq!(config.foliage_density, 0.95);
    }

    #[test]
    fn test_tree_type_all_variants_present() {
        let all_types = TreeType::all();
        assert_eq!(all_types.len(), 6);
        assert!(all_types.contains(&TreeType::Oak));
        assert!(all_types.contains(&TreeType::Pine));
        assert!(all_types.contains(&TreeType::Birch));
        assert!(all_types.contains(&TreeType::Willow));
        assert!(all_types.contains(&TreeType::Dead));
        assert!(all_types.contains(&TreeType::Shrub));
    }

    #[test]
    fn test_tree_type_names() {
        assert_eq!(TreeType::Oak.name(), "Oak");
        assert_eq!(TreeType::Pine.name(), "Pine");
        assert_eq!(TreeType::Birch.name(), "Birch");
        assert_eq!(TreeType::Willow.name(), "Willow");
        assert_eq!(TreeType::Dead.name(), "Dead Tree");
        assert_eq!(TreeType::Shrub.name(), "Shrub");
    }

    #[test]
    fn test_tree_config_default() {
        let config = TreeConfig::default();
        assert_eq!(config.trunk_radius, 0.3);
        assert_eq!(config.height, 3.5);
        assert_eq!(config.depth, 3);
        assert_eq!(config.foliage_density, 0.7);
        assert_eq!(config.foliage_color, (0.2, 0.6, 0.2));
    }

    #[test]
    fn test_terrain_visual_config_default() {
        let config = TerrainVisualConfig::default();
        assert_eq!(config.scale, 1.0);
        assert_eq!(config.height_multiplier, 1.0);
        assert_eq!(config.rotation_y, 0.0);
        assert!(config.color_tint.is_none());
    }

    #[test]
    fn test_advanced_tree_mesh_cache_default() {
        let cache = AdvancedTreeMeshCache::default();
        assert!(cache.tree_meshes.is_empty());
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
        graph.update_bounds();

        let config = TreeConfig::default();
        let mesh = generate_branch_mesh(&graph, &config);

        // Verify mesh has vertices
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Mesh should have position attribute"
        );

        // Verify mesh has normals
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Mesh should have normals"
        );
    }

    #[test]
    fn test_generate_branch_mesh_with_multiple_branches() {
        let mut graph = BranchGraph::new();
        graph.add_branch(Branch {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 2.0, 0.0),
            start_radius: 0.3,
            end_radius: 0.15,
            children: vec![1],
        });
        graph.add_branch(Branch {
            start: Vec3::new(0.0, 2.0, 0.0),
            end: Vec3::new(1.0, 3.0, 0.0),
            start_radius: 0.15,
            end_radius: 0.05,
            children: vec![],
        });
        graph.update_bounds();

        let config = TreeConfig::default();
        let mesh = generate_branch_mesh(&graph, &config);

        // Verify mesh has vertices for branches
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Mesh should have vertices for multiple branches"
        );
    }

    #[test]
    fn test_generate_branch_mesh_empty_graph() {
        let graph = BranchGraph::new();
        let config = TreeConfig::default();
        let mesh = generate_branch_mesh(&graph, &config);

        // Empty graph should still produce a valid mesh (though it may be minimal)
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Empty graph mesh should have position attribute"
        );
    }
}
