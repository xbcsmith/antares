// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh generation system for converting domain MeshDefinitions to Bevy meshes
//!
//! This module provides pure functions to convert domain-level `MeshDefinition`
//! structures into Bevy's `Mesh` format for rendering. It handles vertex data,
//! normals, UVs, colors, and indices.
//!
//! # Features
//!
//! - Converts `MeshDefinition` to Bevy `Mesh`
//! - Auto-generates normals (flat or smooth) when not provided
//! - Creates materials from mesh colors
//! - Helper functions for normal calculation
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::creature_meshes::mesh_definition_to_bevy;
//! use antares::domain::visual::MeshDefinition;
//! use bevy::prelude::*;
//!
//! fn convert_mesh(mesh_def: &MeshDefinition) -> Mesh {
//!     mesh_definition_to_bevy(mesh_def)
//! }
//! ```

use crate::domain::visual::MeshDefinition;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Converts a domain MeshDefinition to a Bevy Mesh
///
/// This function performs the core conversion from the domain-level mesh
/// representation to Bevy's rendering format. It:
/// - Inserts vertex positions
/// - Generates or inserts normals
/// - Inserts UVs if provided
/// - Inserts vertex colors if provided
/// - Sets up triangle indices
///
/// # Arguments
///
/// * `mesh_def` - The domain mesh definition to convert
///
/// # Returns
///
/// A Bevy `Mesh` ready for rendering
///
/// # Panics
///
/// Panics if the mesh definition is invalid (should be validated before calling)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::mesh_definition_to_bevy;
/// use antares::domain::visual::MeshDefinition;
/// use bevy::prelude::*;
///
/// let mesh_def = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
/// };
///
/// let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
/// ```
pub fn mesh_definition_to_bevy(mesh_def: &MeshDefinition) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Insert vertex positions
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_def.vertices.clone());

    // Insert or generate normals
    let normals = if let Some(ref provided_normals) = mesh_def.normals {
        provided_normals.clone()
    } else {
        // Auto-generate flat normals
        calculate_flat_normals(&mesh_def.vertices, &mesh_def.indices)
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    // Insert UVs if provided
    if let Some(ref uvs) = mesh_def.uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());
    }

    // Insert vertex colors (replicate mesh color for all vertices)
    let vertex_colors: Vec<[f32; 4]> = vec![mesh_def.color; mesh_def.vertices.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);

    // Insert indices
    mesh.insert_indices(Indices::U32(mesh_def.indices.clone()));

    mesh
}

/// Calculates flat normals for a mesh
///
/// Flat normals give a faceted appearance where each triangle has a uniform normal.
/// The normal for each vertex of a triangle is the face normal of that triangle.
///
/// # Arguments
///
/// * `vertices` - Vertex positions
/// * `indices` - Triangle indices (must be divisible by 3)
///
/// # Returns
///
/// Vector of normals (one per vertex, matching vertex array length)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::calculate_flat_normals;
///
/// let vertices = vec![
///     [0.0, 0.0, 0.0],
///     [1.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0],
/// ];
/// let indices = vec![0, 1, 2];
///
/// let normals = calculate_flat_normals(&vertices, &indices);
/// assert_eq!(normals.len(), 3);
/// ```
pub fn calculate_flat_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 0.0, 0.0]; vertices.len()];

    // Process each triangle
    for triangle_indices in indices.chunks(3) {
        if triangle_indices.len() != 3 {
            continue;
        }

        let i0 = triangle_indices[0] as usize;
        let i1 = triangle_indices[1] as usize;
        let i2 = triangle_indices[2] as usize;

        let v0 = Vec3::from(vertices[i0]);
        let v1 = Vec3::from(vertices[i1]);
        let v2 = Vec3::from(vertices[i2]);

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        // Assign face normal to all three vertices of the triangle
        normals[i0] = face_normal.to_array();
        normals[i1] = face_normal.to_array();
        normals[i2] = face_normal.to_array();
    }

    normals
}

/// Calculates smooth normals for a mesh
///
/// Smooth normals give a rounded appearance by averaging normals of adjacent
/// triangles that share a vertex. This creates smooth shading across the surface.
///
/// # Arguments
///
/// * `vertices` - Vertex positions
/// * `indices` - Triangle indices (must be divisible by 3)
///
/// # Returns
///
/// Vector of normals (one per vertex, matching vertex array length)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::calculate_smooth_normals;
///
/// let vertices = vec![
///     [0.0, 0.0, 0.0],
///     [1.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0],
///     [1.0, 1.0, 0.0],
/// ];
/// let indices = vec![0, 1, 2, 1, 3, 2];
///
/// let normals = calculate_smooth_normals(&vertices, &indices);
/// assert_eq!(normals.len(), 4);
/// ```
pub fn calculate_smooth_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; vertices.len()];

    // Accumulate face normals for each vertex
    for triangle_indices in indices.chunks(3) {
        if triangle_indices.len() != 3 {
            continue;
        }

        let i0 = triangle_indices[0] as usize;
        let i1 = triangle_indices[1] as usize;
        let i2 = triangle_indices[2] as usize;

        let v0 = Vec3::from(vertices[i0]);
        let v1 = Vec3::from(vertices[i1]);
        let v2 = Vec3::from(vertices[i2]);

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        // Accumulate face normal to each vertex
        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // Normalize accumulated normals
    normals
        .into_iter()
        .map(|n| n.normalize_or_zero().to_array())
        .collect()
}

/// Creates a StandardMaterial from a color array
///
/// Converts a `[f32; 4]` RGBA color to a Bevy `StandardMaterial`.
/// This is used to create materials for creature meshes.
///
/// # Arguments
///
/// * `color` - RGBA color values in range [0.0, 1.0]
///
/// # Returns
///
/// A `StandardMaterial` with the specified base color
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::create_material_from_color;
///
/// let red = [1.0, 0.0, 0.0, 1.0];
/// let material = create_material_from_color(red);
/// ```
pub fn create_material_from_color(color: [f32; 4]) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
        perceptual_roughness: 0.8,
        metallic: 0.0,
        reflectance: 0.3,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_definition_to_bevy_vertices() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);

        // Verify mesh has expected attributes
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        assert!(bevy_mesh.indices().is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_normals_auto() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None, // Should auto-generate
            uvs: None,
            color: [1.0, 0.0, 0.0, 1.0],
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_normals_provided() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_uvs() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]),
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_color() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 0.0, 0.0, 1.0],
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
    }

    #[test]
    fn test_calculate_flat_normals_triangle() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0, 1, 2];

        let normals = calculate_flat_normals(&vertices, &indices);

        assert_eq!(normals.len(), 3);
        // All vertices of the triangle should have the same normal
        assert_eq!(normals[0], normals[1]);
        assert_eq!(normals[1], normals[2]);
    }

    #[test]
    fn test_calculate_flat_normals_cube() {
        // Simple cube vertices (8 corners)
        let vertices = vec![
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ];

        // Front face indices
        let indices = vec![0, 1, 2];

        let normals = calculate_flat_normals(&vertices, &indices);
        assert_eq!(normals.len(), 8);
    }

    #[test]
    fn test_calculate_smooth_normals_triangle() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0, 1, 2];

        let normals = calculate_smooth_normals(&vertices, &indices);

        assert_eq!(normals.len(), 3);
        // For a single triangle, smooth normals should equal flat normals
        assert_eq!(normals[0], normals[1]);
        assert_eq!(normals[1], normals[2]);
    }

    #[test]
    fn test_calculate_smooth_normals_shared_vertex() {
        // Two triangles sharing vertices
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let indices = vec![
            0, 1, 2, // First triangle
            1, 3, 2, // Second triangle
        ];

        let normals = calculate_smooth_normals(&vertices, &indices);
        assert_eq!(normals.len(), 4);

        // Shared vertices (1 and 2) should have averaged normals
        // (different from pure flat normals)
    }

    #[test]
    fn test_create_material_from_color_red() {
        let color = [1.0, 0.0, 0.0, 1.0];
        let material = create_material_from_color(color);

        // Material should have red base color
        assert_eq!(material.base_color.to_srgba().red, 1.0);
        assert_eq!(material.base_color.to_srgba().green, 0.0);
        assert_eq!(material.base_color.to_srgba().blue, 0.0);
    }

    #[test]
    fn test_create_material_from_color_green() {
        let color = [0.0, 1.0, 0.0, 1.0];
        let material = create_material_from_color(color);

        assert_eq!(material.base_color.to_srgba().green, 1.0);
    }

    #[test]
    fn test_create_material_from_color_alpha() {
        let color = [1.0, 1.0, 1.0, 0.5];
        let material = create_material_from_color(color);

        assert_eq!(material.base_color.to_srgba().alpha, 0.5);
    }

    #[test]
    fn test_flat_normals_empty_indices() {
        let vertices = vec![[0.0, 0.0, 0.0]];
        let indices = vec![];

        let normals = calculate_flat_normals(&vertices, &indices);
        assert_eq!(normals.len(), 1);
        assert_eq!(normals[0], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_smooth_normals_empty_indices() {
        let vertices = vec![[0.0, 0.0, 0.0]];
        let indices = vec![];

        let normals = calculate_smooth_normals(&vertices, &indices);
        assert_eq!(normals.len(), 1);
        assert_eq!(normals[0], [0.0, 0.0, 0.0]);
    }
}
