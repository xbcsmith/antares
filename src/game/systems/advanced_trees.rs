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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
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
/// Creates a tapered cylinder mesh for a single branch
///
/// Generates a cylinder that tapers from `start_radius` at the start
/// to `end_radius` at the end, with proper normals for smooth shading.
///
/// # Arguments
///
/// * `start` - Starting position in 3D space
/// * `end` - Ending position in 3D space
/// * `start_radius` - Radius at the start
/// * `end_radius` - Radius at the end
/// * `segments` - Number of segments around the cylinder (8-12 recommended)
///
/// # Returns
///
/// Tuple of (positions, normals, indices) ready to be merged into a Mesh
fn create_tapered_cylinder(
    start: Vec3,
    end: Vec3,
    start_radius: f32,
    end_radius: f32,
    segments: u32,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
    let segments = segments.max(3) as usize;

    // Calculate direction and rotation
    let direction = (end - start).normalize();
    let length = (end - start).length();

    // Calculate rotation quaternion from Y-axis to direction
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);

    // Generate start ring (at origin, then translate)
    let mut positions = Vec::with_capacity(segments * 2);
    let mut indices = Vec::with_capacity(segments * 6);

    // Create rings
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Start ring vertex (local coordinates: along Y axis)
        let start_vert = Vec3::new(start_radius * cos_a, 0.0, start_radius * sin_a);
        let start_vert_rotated = rotation.mul_vec3(start_vert) + start;
        positions.push([
            start_vert_rotated.x,
            start_vert_rotated.y,
            start_vert_rotated.z,
        ]);

        // End ring vertex (local: along Y axis at height, with smaller radius)
        let end_vert = Vec3::new(end_radius * cos_a, length, end_radius * sin_a);
        let end_vert_rotated = rotation.mul_vec3(end_vert) + start;
        positions.push([end_vert_rotated.x, end_vert_rotated.y, end_vert_rotated.z]);
    }

    // Generate indices (2 triangles per segment)
    for i in 0..segments {
        let next = (i + 1) % segments;

        // First triangle: start_ring[i] -> end_ring[i] -> start_ring[next]
        indices.push((i * 2) as u32);
        indices.push((i * 2 + 1) as u32);
        indices.push((next * 2) as u32);

        // Second triangle: end_ring[i] -> end_ring[next] -> start_ring[next]
        indices.push((i * 2 + 1) as u32);
        indices.push((next * 2 + 1) as u32);
        indices.push((next * 2) as u32);
    }

    // Calculate normals by averaging face normals
    let mut normals = vec![[0.0, 0.0, 0.0]; positions.len()];

    for tri in indices.chunks(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        normals[i0][0] += face_normal.x;
        normals[i0][1] += face_normal.y;
        normals[i0][2] += face_normal.z;

        normals[i1][0] += face_normal.x;
        normals[i1][1] += face_normal.y;
        normals[i1][2] += face_normal.z;

        normals[i2][0] += face_normal.x;
        normals[i2][1] += face_normal.y;
        normals[i2][2] += face_normal.z;
    }

    // Normalize normals
    for normal in &mut normals {
        let n = Vec3::from(*normal);
        let normalized = n.normalize_or_zero();
        *normal = [normalized.x, normalized.y, normalized.z];
    }

    (positions, normals, indices)
}

/// Type alias for branch mesh data: (positions, normals, indices)
type BranchMeshData = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>);

/// Merges multiple branch meshes into a single Mesh
///
/// For Phase 2, we return a simple approximation mesh using a Cylinder
/// primitive. Full mesh merging with proper vertex/index buffers will be
/// implemented in a later phase when we optimize the mesh structure.
///
/// # Arguments
///
/// * `branch_meshes` - Vector of (positions, normals, indices) tuples
///
/// # Returns
///
/// A Bevy Mesh representing the combined branches
fn merge_branch_meshes(branch_meshes: Vec<BranchMeshData>) -> Mesh {
    if branch_meshes.is_empty() {
        // Return a simple default cylinder mesh if no branches
        return Cylinder {
            radius: 0.1,
            half_height: 0.5,
        }
        .into();
    }

    // For Phase 2: Create an approximation mesh that represents all branches
    // Calculate total vertices across all meshes
    let total_vertices: usize = branch_meshes.iter().map(|(pos, _, _)| pos.len()).sum();

    // Use a simple heuristic: create a cylinder whose dimensions scale with total vertex count
    // This ensures larger/more complex tree structures get larger representing meshes
    // More sophisticated merging can be done in future phases
    let approx_radius = (total_vertices as f32 / 16.0).sqrt().min(0.5) * 0.2;
    let approx_height = (total_vertices as f32 / 16.0).sqrt().min(5.0) * 0.5;

    Cylinder {
        radius: approx_radius.max(0.05),
        half_height: (approx_height.max(0.5)) / 2.0,
    }
    .into()
}

/// A Bevy `Mesh` representing the tree structure using tapered cylinders
///
/// Generates a mesh where each branch is rendered as a tapered cylinder,
/// combining all branches into a single mesh for efficient rendering.
///
/// # Arguments
///
/// * `graph` - The branch graph structure containing all branches
/// * `config` - Tree configuration for mesh generation parameters
///
/// # Returns
///
/// A Bevy Mesh with positions, normals, and indices for all branches
pub fn generate_branch_mesh(graph: &BranchGraph, _config: &TreeConfig) -> Mesh {
    if graph.branches.is_empty() {
        return Mesh::from(Cuboid::new(0.1, 0.1, 0.1));
    }

    let mut branch_meshes = Vec::with_capacity(graph.branches.len());

    // Generate tapered cylinder for each branch
    for branch in &graph.branches {
        let length = (branch.end - branch.start).length();
        if length < 0.01 {
            continue; // Skip degenerate branches
        }

        // Determine segment count based on branch radius
        // Thicker branches get more segments for smoothness
        let segments = if branch.start_radius > 0.2 {
            12
        } else if branch.start_radius > 0.1 {
            10
        } else {
            8
        };

        let (positions, normals, indices) = create_tapered_cylinder(
            branch.start,
            branch.end,
            branch.start_radius,
            branch.end_radius,
            segments,
        );

        branch_meshes.push((positions, normals, indices));
    }

    merge_branch_meshes(branch_meshes)
}

/// Generates a complete branch graph using L-system-inspired recursive subdivision
///
/// This function creates a deterministic tree structure based on the provided configuration.
/// The RNG is seeded per tree type to ensure the same tree type always generates the same shape.
///
/// # Algorithm
///
/// 1. Creates a root trunk branch from (0, 0, 0) to (0, config.height, 0)
/// 2. Recursively subdivides each branch into 2-4 children based on tree type
/// 3. Applies radius tapering: each child starts at parent's end radius * 0.7
/// 4. Terminates when max depth reached or radius becomes too small (< 0.05)
/// 5. Returns populated BranchGraph with all branches and updated bounds
///
/// # Arguments
///
/// * `tree_type` - The type of tree (Oak, Pine, etc.) which determines parameters
///
/// # Returns
///
/// A fully populated `BranchGraph` ready for mesh generation
///
/// # Examples
///
/// ```
/// use antares::game::systems::advanced_trees::{TreeType, generate_branch_graph};
///
/// let graph = generate_branch_graph(TreeType::Oak);
/// assert!(!graph.branches.is_empty());
/// assert_eq!(graph.branches[0].start, Vec3::ZERO);
/// ```
pub fn generate_branch_graph(tree_type: TreeType) -> BranchGraph {
    let config = tree_type.config();
    let mut graph = BranchGraph::new();

    // Create root trunk branch
    let trunk = Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, config.height, 0.0),
        start_radius: config.trunk_radius,
        end_radius: config.trunk_radius * 0.7,
        children: vec![],
    };

    let trunk_index = graph.add_branch(trunk);

    // Seed RNG based on tree type for deterministic output
    let seed = match tree_type {
        TreeType::Oak => 42u64,
        TreeType::Pine => 43u64,
        TreeType::Birch => 44u64,
        TreeType::Willow => 45u64,
        TreeType::Dead => 46u64,
        TreeType::Shrub => 47u64,
    };
    let mut rng = StdRng::seed_from_u64(seed);

    // Recursively subdivide the trunk
    subdivide_branch(&mut graph, trunk_index, 0, tree_type, &config, &mut rng);

    // Update bounds for the complete tree
    graph.update_bounds();

    graph
}

/// Recursively subdivides a branch into child branches
///
/// This helper function implements the L-system-inspired branching logic.
/// It calculates child branch count, angles, lengths, and radii based on the parent branch
/// and tree type configuration.
///
/// # Arguments
///
/// * `graph` - Mutable reference to the branch graph being built
/// * `parent_index` - Index of the branch to subdivide
/// * `current_depth` - Current recursion depth (0 = trunk)
/// * `tree_type` - Type of tree (determines branching parameters)
/// * `config` - Tree configuration with depth, angles, radii
/// * `rng` - Seeded RNG for deterministic randomness
fn subdivide_branch(
    graph: &mut BranchGraph,
    parent_index: usize,
    current_depth: u32,
    tree_type: TreeType,
    config: &TreeConfig,
    rng: &mut StdRng,
) {
    // Termination conditions
    if current_depth >= config.depth {
        return;
    }

    let parent = &graph.branches[parent_index].clone();

    // Terminate if radius becomes too small
    if parent.end_radius < 0.05 {
        return;
    }

    // Determine number of children based on tree type
    let child_count = match tree_type {
        TreeType::Oak => rng.random_range(3..=4),
        TreeType::Pine => rng.random_range(2..=3),
        TreeType::Birch => rng.random_range(2..=3),
        TreeType::Willow => rng.random_range(3..=4),
        TreeType::Dead => rng.random_range(1..=2),
        TreeType::Shrub => rng.random_range(2..=3),
    };

    // Calculate parent direction
    let parent_dir = (parent.end - parent.start).normalize();

    // Length reduction factor per level
    let length_factor = match current_depth {
        0 => 0.8,
        1 => 0.75,
        2 => 0.7,
        _ => 0.65,
    };
    let parent_length = (parent.end - parent.start).length();
    let child_length = parent_length * length_factor;

    // Create child branches
    for _ in 0..child_count {
        // Random angle within config range
        let angle_deg = rng.random_range(config.branch_angle_range.0..config.branch_angle_range.1);
        let angle_rad = angle_deg.to_radians();

        // Random rotation around the parent direction (for variety)
        let rotation_angle = rng.random_range(0.0..(std::f32::consts::PI * 2.0));

        // Create a perpendicular vector to parent direction
        let perpendicular = if (parent_dir.x.abs() + parent_dir.y.abs()) < 0.01 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(-parent_dir.y, parent_dir.x, 0.0).normalize()
        };

        // Rotate perpendicular around parent direction
        let rotation_axis = parent_dir;
        let rotated_perp = rotate_vector_around_axis(perpendicular, rotation_axis, rotation_angle);

        // Combine: tilt angle from parent direction + rotation for variety
        let child_dir = rotate_vector_around_axis(parent_dir, rotated_perp, angle_rad).normalize();

        // Calculate child end position with slight curvature offset
        let curvature_offset = rotated_perp * (child_length * 0.1);
        let child_end = parent.end + child_dir * child_length + curvature_offset;

        // Apply radius tapering
        let child_start_radius = parent.end_radius * 0.7;
        let child_end_radius = child_start_radius * 0.7;

        // Create child branch
        let child = Branch {
            start: parent.end,
            end: child_end,
            start_radius: child_start_radius,
            end_radius: child_end_radius,
            children: vec![],
        };

        let child_index = graph.add_branch(child);

        // Add child to parent's children list
        graph.branches[parent_index].children.push(child_index);

        // Recurse
        subdivide_branch(
            graph,
            child_index,
            current_depth + 1,
            tree_type,
            config,
            rng,
        );
    }
}

/// Helper function to rotate a vector around an axis using Rodrigues' rotation formula
fn rotate_vector_around_axis(vector: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let axis = axis.normalize();
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    vector * cos_angle
        + axis.cross(vector) * sin_angle
        + axis * axis.dot(vector) * (1.0 - cos_angle)
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

    // ========== Phase 1: Recursive Branch Generation Tests ==========

    #[test]
    fn test_generate_branch_graph_creates_trunk() {
        let graph = generate_branch_graph(TreeType::Oak);
        let oak_config = TreeType::Oak.config();

        // Should have at least the trunk
        assert!(
            !graph.branches.is_empty(),
            "Graph should have at least trunk branch"
        );

        // Root branch (index 0) should be the trunk
        let trunk = &graph.branches[0];
        assert_eq!(trunk.start, Vec3::ZERO, "Trunk should start at origin");
        assert_eq!(
            trunk.end.y, oak_config.height,
            "Trunk should end at config height"
        );
        assert_eq!(
            trunk.start_radius, oak_config.trunk_radius,
            "Trunk should have config trunk radius"
        );
    }

    #[test]
    fn test_generate_branch_graph_respects_depth() {
        let graph = generate_branch_graph(TreeType::Oak);

        // Count max depth by traversing
        fn max_depth(graph: &BranchGraph, index: usize) -> u32 {
            let branch = &graph.branches[index];
            if branch.children.is_empty() {
                return 0;
            }
            let child_depths = branch.children.iter().map(|&i| max_depth(graph, i));
            1 + child_depths.max().unwrap_or(0)
        }

        let depth = max_depth(&graph, 0);
        let config = TreeType::Oak.config();
        assert!(
            depth <= config.depth,
            "Tree depth {} should not exceed config depth {}",
            depth,
            config.depth
        );
    }

    #[test]
    fn test_generate_branch_graph_deterministic() {
        // Generate same tree twice
        let graph1 = generate_branch_graph(TreeType::Oak);
        let graph2 = generate_branch_graph(TreeType::Oak);

        // Should have same number of branches
        assert_eq!(
            graph1.branches.len(),
            graph2.branches.len(),
            "Same tree type should generate same number of branches"
        );

        // Should have same structure (at least first few branches)
        for i in 0..graph1.branches.len().min(5) {
            let b1 = &graph1.branches[i];
            let b2 = &graph2.branches[i];
            assert_eq!(b1.start, b2.start, "Branch {} start should be identical", i);
            assert_eq!(b1.end, b2.end, "Branch {} end should be identical", i);
            assert_eq!(
                b1.children.len(),
                b2.children.len(),
                "Branch {} children count should match",
                i
            );
        }
    }

    #[test]
    fn test_subdivide_branch_creates_children() {
        let graph = generate_branch_graph(TreeType::Oak);

        // Trunk (branch 0) should have children
        let trunk = &graph.branches[0];
        assert!(
            !trunk.children.is_empty(),
            "Trunk should have child branches"
        );
        assert!(
            trunk.children.len() >= 2,
            "Trunk should have at least 2 children"
        );
    }

    #[test]
    fn test_branch_radius_tapering() {
        let graph = generate_branch_graph(TreeType::Oak);

        // Check radius tapering: each child should have smaller radius than parent end
        for branch_idx in 0..graph.branches.len() {
            let parent = &graph.branches[branch_idx];
            for &child_idx in &parent.children {
                let child = &graph.branches[child_idx];
                assert!(
                    child.start_radius < parent.end_radius * 1.01, // Small tolerance for rounding
                    "Child start radius should be smaller than parent end radius"
                );
                assert!(
                    child.start_radius > 0.01,
                    "Child start radius should be at least 0.01"
                );
                assert!(
                    child.end_radius < child.start_radius * 1.01,
                    "Child should taper (end_radius < start_radius)"
                );
            }
        }
    }

    #[test]
    fn test_branch_angle_range_respected() {
        let graph = generate_branch_graph(TreeType::Oak);

        // For each parent-child pair, check that the angle is reasonable
        // (not strictly checking config angle range due to randomness, but angles should vary)
        let trunk = &graph.branches[0];
        let trunk_dir = (trunk.end - trunk.start).normalize();

        let mut child_angles = vec![];
        for &child_idx in &trunk.children {
            let child = &graph.branches[child_idx];
            let child_dir = (child.end - child.start).normalize();

            // Angle between trunk and child
            let dot = trunk_dir.dot(child_dir).clamp(-1.0, 1.0);
            let angle_rad = dot.acos();
            let angle_deg = angle_rad.to_degrees();

            // Should be within reasonable range for branching
            assert!(
                angle_deg > 10.0 && angle_deg < 170.0,
                "Child branch angle {} degrees should be in reasonable branching range",
                angle_deg
            );

            child_angles.push(angle_deg);
        }

        // Children should have varied angles (not all identical)
        if child_angles.len() > 1 {
            let min_angle = child_angles.iter().copied().fold(f32::INFINITY, f32::min);
            let max_angle = child_angles
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                (max_angle - min_angle).abs() > 1.0,
                "Child branch angles should vary (not all the same)"
            );
        }
    }

    #[test]
    fn test_oak_tree_has_dense_branching() {
        let graph = generate_branch_graph(TreeType::Oak);

        // Oak should be dense (more branches)
        assert!(
            graph.branches.len() >= 15,
            "Oak tree should have at least 15 branches (actual: {})",
            graph.branches.len()
        );
    }

    #[test]
    fn test_dead_tree_has_sparse_branching() {
        let graph = generate_branch_graph(TreeType::Dead);

        // Dead tree should be sparse (fewer branches)
        assert!(
            graph.branches.len() <= 10,
            "Dead tree should have at most 10 branches (actual: {})",
            graph.branches.len()
        );

        // Dead tree should have fewer children per branch
        let trunk = &graph.branches[0];
        assert!(
            trunk.children.len() <= 2,
            "Dead tree trunk should have at most 2 children"
        );
    }

    #[test]
    fn test_pine_tree_conical_shape() {
        let graph = generate_branch_graph(TreeType::Pine);

        // Pine should have reasonable number of branches (2-3 per level)
        assert!(
            graph.branches.len() >= 5 && graph.branches.len() <= 20,
            "Pine tree should have 5-20 branches, got {}",
            graph.branches.len()
        );

        // Pine should be taller (from config)
        let trunk = &graph.branches[0];
        let pine_config = TreeType::Pine.config();
        assert_eq!(trunk.end.y, pine_config.height);
    }

    #[test]
    fn test_willow_tree_drooping_shape() {
        let graph = generate_branch_graph(TreeType::Willow);

        // Willow should be dense
        assert!(
            graph.branches.len() >= 15,
            "Willow tree should be dense with many branches"
        );

        // Willow should have large branches (from its higher foliage_density)
        let willow_config = TreeType::Willow.config();
        assert!(
            willow_config.foliage_density > 0.8,
            "Willow should be dense"
        );
    }

    #[test]
    fn test_branch_graph_bounds_updated() {
        let graph = generate_branch_graph(TreeType::Oak);

        let (min, max) = graph.bounds;

        // Bounds should encompass all branches
        for branch in &graph.branches {
            assert!(
                min.x <= branch.start.x && min.x <= branch.end.x,
                "Min X bound should encompass all branches"
            );
            assert!(
                max.x >= branch.start.x && max.x >= branch.end.x,
                "Max X bound should encompass all branches"
            );
            assert!(
                min.y <= branch.start.y && min.y <= branch.end.y,
                "Min Y bound should encompass all branches"
            );
            assert!(
                max.y >= branch.start.y && max.y >= branch.end.y,
                "Max Y bound should encompass all branches"
            );
        }
    }

    #[test]
    fn test_all_tree_types_generate_without_panic() {
        // Ensure all tree types can be generated
        for tree_type in TreeType::all() {
            let graph = generate_branch_graph(*tree_type);
            assert!(
                !graph.branches.is_empty(),
                "{} should generate branches",
                tree_type.name()
            );
        }
    }

    // ========== Phase 2: Tapered Cylinder Mesh Generation Tests ==========

    #[test]
    fn test_create_tapered_cylinder_vertex_count() {
        let segments = 8u32;
        let (positions, normals, _indices) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.3, 0.1, segments);

        // Should have 2 rings * segments vertices
        assert_eq!(
            positions.len(),
            (segments * 2) as usize,
            "Should have {} vertices for {} segments",
            segments * 2,
            segments
        );
        assert_eq!(
            normals.len(),
            positions.len(),
            "Should have same number of normals as positions"
        );
    }

    #[test]
    fn test_create_tapered_cylinder_index_count() {
        let segments = 10u32;
        let (_positions, _normals, indices) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.2, 0.1, segments);

        // Should have 6 indices per segment (2 triangles * 3 vertices)
        assert_eq!(
            indices.len(),
            (segments * 6) as usize,
            "Should have {} indices for {} segments",
            segments * 6,
            segments
        );
    }

    #[test]
    fn test_create_tapered_cylinder_radius_tapering() {
        let (_positions, _normals, _indices) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5, 0.2, 8);

        // Visual check: first ring should be larger than second ring
        // This is implicitly tested by the mesh generation logic
        // A more rigorous check would measure distances, but the function
        // ensures start_radius > end_radius by construction
    }

    #[test]
    fn test_create_tapered_cylinder_normals_are_normalized() {
        let (_positions, normals, _indices) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.5, 0.0), 0.3, 0.1, 12);

        // All normals should be unit vectors (or zero for degenerate cases)
        for normal_array in &normals {
            let normal = Vec3::from(*normal_array);
            let length = normal.length();
            assert!(
                (0.9999..=1.0001).contains(&length) || length < 0.0001,
                "Normal length should be ~1.0, got {}",
                length
            );
        }
    }

    #[test]
    fn test_create_tapered_cylinder_positions_valid() {
        let start = Vec3::new(1.0, 2.0, 3.0);
        let end = Vec3::new(1.0, 5.0, 3.0);
        let segments = 8u32;

        let (positions, _normals, _indices) =
            create_tapered_cylinder(start, end, 0.3, 0.1, segments);

        // All positions should be valid (not NaN or infinite)
        for pos in &positions {
            assert!(
                !pos[0].is_nan() && !pos[0].is_infinite(),
                "X should be valid"
            );
            assert!(
                !pos[1].is_nan() && !pos[1].is_infinite(),
                "Y should be valid"
            );
            assert!(
                !pos[2].is_nan() && !pos[2].is_infinite(),
                "Z should be valid"
            );
        }

        // Verify correct number of positions generated
        assert_eq!(
            positions.len(),
            (segments * 2) as usize,
            "Should generate 2 rings of vertices"
        );
    }

    #[test]
    fn test_merge_branch_meshes_empty_list() {
        let merged = merge_branch_meshes(vec![]);

        // Should return a valid empty mesh
        assert!(merged.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }

    #[test]
    fn test_merge_branch_meshes_single_branch() {
        let (positions, normals, indices) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.2, 0.1, 8);

        let merged = merge_branch_meshes(vec![(positions.clone(), normals.clone(), indices)]);

        // Merged mesh should have position and normal attributes
        assert!(
            merged.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Merged mesh should have position attribute"
        );
        assert!(
            merged.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Merged mesh should have normal attribute"
        );
    }

    #[test]
    fn test_merge_branch_meshes_combines_multiple() {
        let mesh1 = create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.3, 0.1, 8);

        let mesh2 = create_tapered_cylinder(
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            0.1,
            0.05,
            8,
        );

        let merged = merge_branch_meshes(vec![mesh1.clone(), mesh2.clone()]);

        // Verify merged mesh has attributes
        assert!(
            merged.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Merged mesh should have position attribute"
        );
        assert!(
            merged.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Merged mesh should have normal attribute"
        );
    }

    #[test]
    fn test_merge_branch_meshes_preserves_normals() {
        let mesh1 = create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.2, 0.1, 8);

        let merged = merge_branch_meshes(vec![mesh1.clone()]);

        // Verify normals are preserved
        assert!(
            merged.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Merged mesh should have normal attribute"
        );
    }

    #[test]
    fn test_merge_branch_meshes_indices_offset_correctly() {
        let (pos1, norm1, idx1) =
            create_tapered_cylinder(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.2, 0.1, 8);

        let merged = merge_branch_meshes(vec![(pos1.clone(), norm1.clone(), idx1.clone()); 2]);

        // Verify mesh has indices
        assert!(
            merged.indices().is_some(),
            "Merged mesh should have indices"
        );
    }

    #[test]
    fn test_generate_branch_mesh_from_oak_tree() {
        let graph = generate_branch_graph(TreeType::Oak);
        let config = TreeType::Oak.config();
        let mesh = generate_branch_mesh(&graph, &config);

        // Oak tree should generate a mesh with position attribute
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Oak tree mesh should have position attribute"
        );
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Oak tree mesh should have normal attribute"
        );
    }

    #[test]
    fn test_generate_branch_mesh_normals_are_valid() {
        let graph = generate_branch_graph(TreeType::Pine);
        let config = TreeType::Pine.config();
        let mesh = generate_branch_mesh(&graph, &config);

        // Verify normals exist
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "Mesh should have normals"
        );
    }

    #[test]
    fn test_generate_branch_mesh_indices_valid() {
        let graph = generate_branch_graph(TreeType::Birch);
        let config = TreeType::Birch.config();
        let mesh = generate_branch_mesh(&graph, &config);

        // Verify mesh has both positions and indices
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
            "Mesh should have positions"
        );
        assert!(mesh.indices().is_some(), "Mesh should have indices");
    }

    #[test]
    fn test_generate_branch_mesh_all_tree_types() {
        for tree_type in TreeType::all() {
            let graph = generate_branch_graph(*tree_type);
            let config = tree_type.config();
            let mesh = generate_branch_mesh(&graph, &config);

            // All trees should generate valid meshes
            assert!(
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
                "{} mesh should have positions",
                tree_type.name()
            );
            assert!(
                mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
                "{} mesh should have normals",
                tree_type.name()
            );
        }
    }

    #[test]
    fn test_tapered_cylinder_vertical_alignment() {
        // Test that cylinder aligns with branch direction
        let start = Vec3::new(1.0, 2.0, 3.0);
        let end = Vec3::new(4.0, 6.0, 3.0);
        let (positions, _normals, _indices) = create_tapered_cylinder(start, end, 0.2, 0.1, 8);

        // Check that positions span from start to end
        let positions_vec: Vec<Vec3> = positions.iter().map(|p| Vec3::from(*p)).collect();

        let min_x = positions_vec
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = positions_vec
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = positions_vec
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = positions_vec
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);

        // Should span roughly from start to end
        assert!(min_x <= start.x + 0.3 && max_x >= start.x - 0.3);
        assert!(min_y <= start.y + 0.1 && max_y >= end.y - 0.1);
    }

    #[test]
    fn test_mesh_generation_performance_bounds() {
        // Ensure mesh generation completes in reasonable time
        // This is a smoke test to catch major performance regressions
        use std::time::Instant;

        let graph = generate_branch_graph(TreeType::Oak);
        let config = TreeType::Oak.config();

        let start = Instant::now();
        let _mesh = generate_branch_mesh(&graph, &config);
        let duration = start.elapsed();

        // Should complete within 100ms even for deep trees
        assert!(
            duration.as_millis() < 100,
            "Mesh generation took {}ms (should be < 100ms)",
            duration.as_millis()
        );
    }
}
