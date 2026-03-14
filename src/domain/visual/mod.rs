// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Visual representation system for creatures and monsters
//!
//! This module defines the core types for procedural mesh-based visuals,
//! including mesh definitions, creature definitions, and transformations.
//!
//! # Architecture
//!
//! The visual system is separate from game logic (monsters, NPCs) and linked
//! via `CreatureId` references. This allows multiple monsters to share the
//! same visual representation and enables visual variants without duplicating
//! game data.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::{MeshDefinition, CreatureDefinition, MeshTransform};
//!
//! // Create a simple cube mesh
//! let cube = MeshDefinition {
//!     name: None,
//!     vertices: vec![
//!         [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0],
//!         [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
//!         [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0],
//!         [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
//!     ],
//!     indices: vec![
//!         0, 1, 2, 2, 3, 0, // Front
//!         4, 5, 6, 6, 7, 4, // Back
//!         // ... more faces
//!     ],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//!     lod_levels: None,
//!     lod_distances: None,
//!     material: None,
//!     texture_path: None,
//! };
//!
//! // Create a creature from the mesh
//! let creature = CreatureDefinition {
//!     id: 1,
//!     name: "Simple Cube".to_string(),
//!     meshes: vec![cube],
//!     mesh_transforms: vec![MeshTransform::identity()],
//!     scale: 1.0,
//!     color_tint: None,
//! };
//! ```

pub mod animation;
pub mod animation_state_machine;
pub mod blend_tree;
pub mod creature_database;
pub mod creature_variations;
pub mod item_mesh;
pub mod lod;

pub use item_mesh::{ItemMeshCategory, ItemMeshDescriptor, ItemMeshDescriptorOverride};
pub mod mesh_validation;
pub mod performance;
pub mod skeletal_animation;
pub mod skeleton;
pub mod template_metadata;
pub mod texture_atlas;

use serde::{Deserialize, Serialize};

use crate::domain::types::CreatureId;

/// A single mesh definition with vertices, indices, and optional attributes
///
/// Represents a 3D mesh that can be rendered. Vertices define points in 3D space,
/// indices define triangles, and optional attributes provide normals and UVs.
///
/// # Coordinate System
///
/// Uses a right-handed coordinate system:
/// - X: right
/// - Y: up
/// - Z: forward
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshDefinition;
///
/// // Triangle mesh
/// let triangle = MeshDefinition {
///     name: None,
///     vertices: vec![
///         [0.0, 1.0, 0.0],
///         [-1.0, -1.0, 0.0],
///         [1.0, -1.0, 0.0],
///     ],
///     indices: vec![0, 1, 2],
///     normals: Some(vec![
///         [0.0, 0.0, 1.0],
///         [0.0, 0.0, 1.0],
///         [0.0, 0.0, 1.0],
///     ]),
///     uvs: None,
///     color: [1.0, 0.0, 0.0, 1.0], // Red
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshDefinition {
    /// Optional name for the mesh (e.g., "left_leg", "head", "torso")
    ///
    /// Used for debugging, editor display, and mesh identification.
    #[serde(default)]
    pub name: Option<String>,

    /// Vertex positions as [x, y, z] coordinates
    pub vertices: Vec<[f32; 3]>,

    /// Triangle indices (groups of 3 form triangles)
    pub indices: Vec<u32>,

    /// Optional vertex normals as [x, y, z] unit vectors
    ///
    /// If None, flat normals will be auto-calculated per triangle.
    /// If Some, length must match vertices.len().
    #[serde(default)]
    pub normals: Option<Vec<[f32; 3]>>,

    /// Optional UV texture coordinates as [u, v]
    ///
    /// If Some, length must match vertices.len().
    #[serde(default)]
    pub uvs: Option<Vec<[f32; 2]>>,

    /// Base color as [r, g, b, a] in range 0.0-1.0
    #[serde(default = "default_color")]
    pub color: [f32; 4],

    /// Optional Level of Detail (LOD) levels
    ///
    /// Contains simplified versions of this mesh for rendering at different distances.
    /// LOD0 is the full detail mesh (this mesh), LOD1 is simplified, etc.
    #[serde(default)]
    pub lod_levels: Option<Vec<MeshDefinition>>,

    /// Optional distance thresholds for LOD switching
    ///
    /// Specifies camera distances at which to switch to each LOD level.
    /// If Some, length must match lod_levels.len().
    /// Example: [10.0, 25.0, 50.0] means switch to LOD1 at 10 units, LOD2 at 25, etc.
    #[serde(default)]
    pub lod_distances: Option<Vec<f32>>,

    /// Optional material definition
    #[serde(default)]
    pub material: Option<MaterialDefinition>,

    /// Optional texture path relative to campaign directory
    ///
    /// Example: "textures/dragon_scales.png"
    #[serde(default)]
    pub texture_path: Option<String>,
}

impl Default for MeshDefinition {
    fn default() -> Self {
        Self {
            name: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: None,
            uvs: None,
            color: default_color(),
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        }
    }
}

/// Material definition for physically-based rendering
///
/// Defines the visual properties of a mesh surface using PBR parameters.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{MaterialDefinition, AlphaMode};
///
/// // Shiny metallic material
/// let metal = MaterialDefinition {
///     base_color: [0.8, 0.8, 0.8, 1.0],
///     metallic: 1.0,
///     roughness: 0.2,
///     emissive: None,
///     alpha_mode: AlphaMode::Opaque,
/// };
///
/// // Glowing emissive material
/// let glowing = MaterialDefinition {
///     base_color: [1.0, 1.0, 1.0, 1.0],
///     metallic: 0.0,
///     roughness: 0.9,
///     emissive: Some([1.0, 0.5, 0.0]),
///     alpha_mode: AlphaMode::Opaque,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialDefinition {
    /// Base color as [r, g, b, a] in range 0.0-1.0
    pub base_color: [f32; 4],

    /// Metallic factor (0.0 = non-metal, 1.0 = metal)
    #[serde(default)]
    pub metallic: f32,

    /// Roughness factor (0.0 = smooth/shiny, vy1.0 = rough/matte)
    #[serde(default = "default_roughness")]
    pub roughness: f32,

    /// Optional emissive color as [r, g, b]
    ///
    /// Makes the material glow with the specified color.
    #[serde(default)]
    pub emissive: Option<[f32; 3]>,

    /// Alpha blending mode
    #[serde(default)]
    pub alpha_mode: AlphaMode,
}

impl Default for MaterialDefinition {
    fn default() -> Self {
        Self {
            base_color: default_color(),
            metallic: 0.0,
            roughness: default_roughness(),
            emissive: None,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

/// Alpha blending mode for materials
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlphaMode {
    /// Fully opaque, no transparency
    #[default]
    Opaque,
    /// Alpha blending based on alpha channel
    Blend,
    /// Alpha masking with cutoff threshold
    Mask,
}

/// Transformation applied to a mesh within a creature
///
/// Allows positioning, rotating, and scaling individual meshes
/// when building multi-mesh creatures.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshTransform;
///
/// // Identity transform (no change)
/// let identity = MeshTransform::identity();
///
/// // Translate up by 2 units
/// let raised = MeshTransform {
///     translation: [0.0, 2.0, 0.0],
///     rotation: [0.0, 0.0, 0.0],
///     scale: [1.0, 1.0, 1.0],
/// };
///
/// // Scale to half size
/// let small = MeshTransform {
///     translation: [0.0, 0.0, 0.0],
///     rotation: [0.0, 0.0, 0.0],
///     scale: [0.5, 0.5, 0.5],
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshTransform {
    /// Translation offset as [x, y, z]
    #[serde(default)]
    pub translation: [f32; 3],

    /// Rotation in Euler angles [pitch, yaw, roll] in radians
    #[serde(default)]
    pub rotation: [f32; 3],

    /// Scale factors as [x, y, z]
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
}

impl MeshTransform {
    /// Creates an identity transform (no transformation)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let transform = MeshTransform::identity();
    /// assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
    /// assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
    /// assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    /// ```
    pub fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Creates a translation-only transform
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [x, y, z],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Creates a scale-only transform
    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [x, y, z],
        }
    }

    /// Creates a uniform scale transform
    pub fn uniform_scale(s: f32) -> Self {
        Self::scale(s, s, s)
    }
}

impl Default for MeshTransform {
    fn default() -> Self {
        Self::identity()
    }
}

/// A complete creature visual definition with one or more meshes
///
/// Creatures are composed of one or more meshes, each with its own transform.
/// This allows building complex creatures from simple parts (e.g., body, head, limbs).
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
///
/// let body = MeshDefinition {
///     name: None,
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [0.5, 0.5, 0.5, 1.0],
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
///
/// let head = MeshDefinition {
///     name: None,
///     vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [0.7, 0.7, 0.7, 1.0],
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
///
/// let creature = CreatureDefinition {
///     id: 42,
///     name: "Simple Biped".to_string(),
///     meshes: vec![body, head],
///     mesh_transforms: vec![
///         MeshTransform::identity(),
///         MeshTransform::translation(0.0, 1.5, 0.0), // Head above body
///     ],
///     scale: 1.0,
///     color_tint: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureDefinition {
    /// Unique creature identifier
    pub id: CreatureId,

    /// Display name for editor/debugging
    pub name: String,

    /// List of meshes that compose this creature
    pub meshes: Vec<MeshDefinition>,

    /// Transform for each mesh (must have same length as meshes)
    pub mesh_transforms: Vec<MeshTransform>,

    /// Global scale multiplier applied to entire creature
    #[serde(default = "default_scale_f32")]
    pub scale: f32,

    /// Optional color tint applied to all meshes [r, g, b, a]
    ///
    /// If Some, multiplies with each mesh's base color.
    /// If None, meshes use their own colors.
    #[serde(default)]
    pub color_tint: Option<[f32; 4]>,
}

impl Default for CreatureDefinition {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Creature".to_string(),
            meshes: Vec::new(),
            mesh_transforms: Vec::new(),
            scale: 1.0,
            color_tint: None,
        }
    }
}

impl CreatureDefinition {
    /// Validates that the creature definition is well-formed
    ///
    /// Checks:
    /// - At least one mesh
    /// - mesh_transforms length matches meshes length
    /// - scale is positive
    /// - Each mesh is valid
    ///
    /// # Errors
    ///
    /// Returns `Err` if validation fails with a descriptive message.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
    ///
    /// let mesh = MeshDefinition {
    ///     name: None,
    ///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
    ///     indices: vec![0, 1, 2],
    ///     normals: None,
    ///     uvs: None,
    ///     color: [1.0, 1.0, 1.0, 1.0],
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
    /// };
    ///
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Test".to_string(),
    ///     meshes: vec![mesh],
    ///     mesh_transforms: vec![MeshTransform::identity()],
    ///     scale: 1.0,
    ///     color_tint: None,
    /// };
    ///
    /// assert!(creature.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.meshes.is_empty() {
            return Err("Creature must have at least one mesh".to_string());
        }

        if self.meshes.len() != self.mesh_transforms.len() {
            return Err(format!(
                "Mesh count ({}) must match transform count ({})",
                self.meshes.len(),
                self.mesh_transforms.len()
            ));
        }

        if self.scale <= 0.0 {
            return Err(format!("Scale must be positive, got {}", self.scale));
        }

        // Validate each mesh
        for (i, mesh) in self.meshes.iter().enumerate() {
            mesh_validation::validate_mesh_definition(mesh)
                .map_err(|e| format!("Mesh {}: {}", i, e))?;
        }

        Ok(())
    }

    /// Returns the total number of vertices across all meshes
    pub fn total_vertices(&self) -> usize {
        self.meshes.iter().map(|m| m.vertices.len()).sum()
    }

    /// Returns the total number of triangles across all meshes
    pub fn total_triangles(&self) -> usize {
        self.meshes.iter().map(|m| m.indices.len() / 3).sum()
    }

    /// Computes the Y offset needed to raise the creature so its lowest vertex
    /// sits exactly on the ground plane (world Y = 0).
    ///
    /// Creature meshes are authored with the body origin (waist) at local Y = 0
    /// and the feet extending into negative Y.  When spawned at world Y = 0 the
    /// legs would sink through the floor.  This method returns the amount to
    /// add to the spawn Y so the creature's lowest vertex touches the floor.
    ///
    /// The calculation accounts for per-mesh Y translation and Y scale stored in
    /// `mesh_transforms`, then multiplies by the global `scale` field.
    ///
    /// # Returns
    ///
    /// `(-min_local_y * self.scale).max(0.0)` — the world-space lift to apply.
    /// Returns `0.0` when all vertices are at or above the local origin.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
    ///
    /// // A creature with legs extending 0.6 units below the origin at scale 0.72
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Goblin".to_string(),
    ///     meshes: vec![MeshDefinition {
    ///         name: Some("left_leg".to_string()),
    ///         vertices: vec![
    ///             [-0.15, -0.6, -0.1],
    ///             [ 0.15, -0.6, -0.1],
    ///             [ 0.15,  0.0, -0.1],
    ///             [-0.15,  0.0, -0.1],
    ///         ],
    ///         indices: vec![0, 1, 2, 2, 3, 0],
    ///         normals: None,
    ///         uvs: None,
    ///         color: [0.36, 0.5, 0.22, 1.0],
    ///         lod_levels: None,
    ///         lod_distances: None,
    ///         material: None,
    ///         texture_path: None,
    ///     }],
    ///     mesh_transforms: vec![MeshTransform::identity()],
    ///     scale: 0.72,
    ///     color_tint: None,
    /// };
    ///
    /// // min vertex Y = -0.6; at scale 0.72 the offset is 0.432
    /// let offset = creature.foot_ground_offset();
    /// assert!((offset - 0.432).abs() < 1e-5, "offset was {}", offset);
    /// ```
    pub fn foot_ground_offset(&self) -> f32 {
        // Find the minimum Y in the creature's local mesh space, respecting
        // per-mesh Y translation and Y scale from mesh_transforms.
        // (Mesh-level rotations are not accounted for here; all shipped RON
        // assets use identity rotations for individual mesh parts.)
        let min_local_y = self
            .meshes
            .iter()
            .zip(self.mesh_transforms.iter())
            .flat_map(|(mesh, transform)| {
                let ty = transform.translation[1];
                let sy = transform.scale[1];
                mesh.vertices.iter().map(move |v| v[1] * sy + ty)
            })
            .fold(f32::INFINITY, f32::min);

        if min_local_y.is_finite() && min_local_y < 0.0 {
            -min_local_y * self.scale
        } else {
            0.0
        }
    }
}

/// Lightweight creature registry entry
///
/// Used in campaign creature registries to reference external creature mesh files
/// instead of embedding full MeshDefinition data inline.
///
/// This struct is designed for registry files (e.g., `creatures.ron`) that map
/// creature IDs to their corresponding definition files. The actual creature
/// definitions are loaded from individual files at campaign startup.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::CreatureReference;
///
/// let reference = CreatureReference {
///     id: 1,
///     name: "Goblin".to_string(),
///     filepath: "assets/creatures/goblin.ron".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureReference {
    /// Unique creature identifier for this registry entry.
    ///
    /// This ID is authoritative for registry-driven loads and may intentionally
    /// differ from the ID stored in the referenced asset file to support
    /// many-to-one mesh reuse.
    pub id: CreatureId,

    /// Display name for editor/debugging
    pub name: String,

    /// Relative path to creature definition file from campaign root
    ///
    /// Example: "assets/creatures/goblin.ron"
    pub filepath: String,
}

// ===== Default value functions for serde =====

fn default_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0] // White
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_scale_f32() -> f32 {
    1.0
}

fn default_roughness() -> f32 {
    0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_triangle_mesh() -> MeshDefinition {
        MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        }
    }

    #[test]
    fn test_mesh_definition_creation() {
        let mesh = create_test_triangle_mesh();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_mesh_transform_identity() {
        let transform = MeshTransform::identity();
        assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
        assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_mesh_transform_translation() {
        let transform = MeshTransform::translation(1.0, 2.0, 3.0);
        assert_eq!(transform.translation, [1.0, 2.0, 3.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_mesh_transform_scale() {
        let transform = MeshTransform::scale(2.0, 3.0, 4.0);
        assert_eq!(transform.scale, [2.0, 3.0, 4.0]);
        assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_mesh_transform_uniform_scale() {
        let transform = MeshTransform::uniform_scale(2.5);
        assert_eq!(transform.scale, [2.5, 2.5, 2.5]);
    }

    #[test]
    fn test_mesh_transform_default() {
        let transform = MeshTransform::default();
        assert_eq!(transform, MeshTransform::identity());
    }

    #[test]
    fn test_creature_definition_creation() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Test Creature".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        assert_eq!(creature.id, 1);
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.mesh_transforms.len(), 1);
    }

    #[test]
    fn test_creature_definition_validate_success() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Valid".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        assert!(creature.validate().is_ok());
    }

    #[test]
    fn test_creature_definition_validate_no_meshes() {
        let creature = CreatureDefinition {
            id: 1,
            name: "Empty".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };

        assert!(creature.validate().is_err());
        assert!(creature
            .validate()
            .unwrap_err()
            .contains("at least one mesh"));
    }

    #[test]
    fn test_creature_definition_validate_transform_mismatch() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Mismatch".to_string(),
            meshes: vec![mesh.clone(), mesh],
            mesh_transforms: vec![MeshTransform::identity()], // Only 1 transform for 2 meshes
            scale: 1.0,
            color_tint: None,
        };

        assert!(creature.validate().is_err());
        assert!(creature.validate().unwrap_err().contains("must match"));
    }

    #[test]
    fn test_creature_definition_validate_negative_scale() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Negative Scale".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: -1.0,
            color_tint: None,
        };

        assert!(creature.validate().is_err());
        assert!(creature
            .validate()
            .unwrap_err()
            .contains("must be positive"));
    }

    #[test]
    fn test_creature_definition_total_vertices() {
        let mesh1 = create_test_triangle_mesh(); // 3 vertices
        let mesh2 = create_test_triangle_mesh(); // 3 vertices

        let creature = CreatureDefinition {
            id: 1,
            name: "Multi-mesh".to_string(),
            meshes: vec![mesh1, mesh2],
            mesh_transforms: vec![MeshTransform::identity(), MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        assert_eq!(creature.total_vertices(), 6);
    }

    #[test]
    fn test_creature_definition_total_triangles() {
        let mesh1 = create_test_triangle_mesh(); // 1 triangle (3 indices / 3)
        let mesh2 = create_test_triangle_mesh(); // 1 triangle

        let creature = CreatureDefinition {
            id: 1,
            name: "Multi-mesh".to_string(),
            meshes: vec![mesh1, mesh2],
            mesh_transforms: vec![MeshTransform::identity(), MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        assert_eq!(creature.total_triangles(), 2);
    }

    #[test]
    fn test_creature_definition_with_color_tint() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Tinted".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: Some([0.5, 0.5, 1.0, 1.0]), // Blue tint
        };

        assert!(creature.color_tint.is_some());
        assert_eq!(creature.color_tint.unwrap(), [0.5, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn test_foot_ground_offset_legs_below_origin() {
        // Creature with legs extending 0.6 units below origin at scale 0.72
        let leg_mesh = MeshDefinition {
            name: Some("left_leg".to_string()),
            vertices: vec![
                [-0.15, -0.6, -0.1],
                [0.15, -0.6, -0.1],
                [0.15, 0.0, -0.1],
                [-0.15, 0.0, -0.1],
            ],
            indices: vec![0, 1, 2, 2, 3, 0],
            normals: None,
            uvs: None,
            color: [0.36, 0.5, 0.22, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = CreatureDefinition {
            id: 1,
            name: "Goblin-like".to_string(),
            meshes: vec![leg_mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 0.72,
            color_tint: None,
        };
        // min_y = -0.6, scale = 0.72  →  offset = 0.6 × 0.72 = 0.432
        let offset = creature.foot_ground_offset();
        assert!(
            (offset - 0.432).abs() < 1e-5,
            "expected 0.432, got {offset}"
        );
    }

    #[test]
    fn test_foot_ground_offset_all_above_origin() {
        // Creature whose geometry is entirely above Y = 0 (e.g. a hat or floating orb)
        let orb_mesh = MeshDefinition {
            name: Some("orb".to_string()),
            vertices: vec![[0.0, 0.5, 0.0], [0.5, 1.0, 0.0], [-0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = CreatureDefinition {
            id: 2,
            name: "FloatingOrb".to_string(),
            meshes: vec![orb_mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        // All vertices at Y >= 0 → no lift needed
        assert_eq!(creature.foot_ground_offset(), 0.0);
    }

    #[test]
    fn test_foot_ground_offset_exactly_at_origin() {
        // Mesh with lowest vertex exactly at Y = 0
        let mesh = MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = CreatureDefinition {
            id: 3,
            name: "FloorLevel".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 2.0,
            color_tint: None,
        };
        assert_eq!(creature.foot_ground_offset(), 0.0);
    }

    #[test]
    fn test_foot_ground_offset_bandit_scale() {
        // Simulate Bandit: legs from Y = -1.0 to 0.0 at scale = 1.0
        let leg_mesh = MeshDefinition {
            name: Some("left_leg".to_string()),
            vertices: vec![
                [-0.15, -1.0, -0.1],
                [0.15, -1.0, -0.1],
                [0.15, 0.0, -0.1],
                [-0.15, 0.0, -0.1],
            ],
            indices: vec![0, 1, 2, 2, 3, 0],
            normals: None,
            uvs: None,
            color: [0.7, 0.72, 0.76, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = CreatureDefinition {
            id: 4,
            name: "Bandit-like".to_string(),
            meshes: vec![leg_mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        // min_y = -1.0, scale = 1.0  →  offset = 1.0
        let offset = creature.foot_ground_offset();
        assert!((offset - 1.0).abs() < 1e-5, "expected 1.0, got {offset}");
    }

    #[test]
    fn test_foot_ground_offset_multi_mesh_deepest_wins() {
        // Two meshes: one with feet at -0.3, one with feet at -0.8 — the deeper one wins
        let shallow_mesh = MeshDefinition {
            name: Some("torso".to_string()),
            vertices: vec![[0.0, -0.3, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let deep_mesh = MeshDefinition {
            name: Some("left_leg".to_string()),
            vertices: vec![[0.0, -0.8, 0.0], [0.2, 0.0, 0.0], [-0.2, 0.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = CreatureDefinition {
            id: 5,
            name: "MultiMesh".to_string(),
            meshes: vec![shallow_mesh, deep_mesh],
            mesh_transforms: vec![MeshTransform::identity(), MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        // min_y = -0.8, scale = 1.0  →  offset = 0.8
        let offset = creature.foot_ground_offset();
        assert!((offset - 0.8).abs() < 1e-5, "expected 0.8, got {offset}");
    }

    #[test]
    fn test_foot_ground_offset_respects_mesh_transform_y_translation() {
        // A mesh with lowest vertex at Y = -0.2, but its mesh_transform translates it down by -0.4.
        // Effective min_y in parent space = -0.2 + (-0.4) = -0.6
        let mesh = MeshDefinition {
            name: None,
            vertices: vec![[0.0, -0.2, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let shifted_transform = MeshTransform {
            translation: [0.0, -0.4, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        };
        let creature = CreatureDefinition {
            id: 6,
            name: "ShiftedMesh".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![shifted_transform],
            scale: 1.0,
            color_tint: None,
        };
        // min_y (in parent space) = -0.2 * 1.0 + (-0.4) = -0.6; scale = 1.0  →  offset = 0.6
        let offset = creature.foot_ground_offset();
        assert!((offset - 0.6).abs() < 1e-5, "expected 0.6, got {offset}");
    }

    #[test]
    fn test_mesh_definition_serialization() {
        let mesh = create_test_triangle_mesh();
        let serialized = ron::to_string(&mesh).expect("Failed to serialize");
        let deserialized: MeshDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(mesh, deserialized);
    }

    #[test]
    fn test_creature_definition_serialization() {
        let mesh = create_test_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Test".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        let serialized = ron::to_string(&creature).expect("Failed to serialize");
        let deserialized: CreatureDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(creature.id, deserialized.id);
        assert_eq!(creature.name, deserialized.name);
    }
}
