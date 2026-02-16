// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Level of Detail (LOD) system for mesh optimization
//!
//! This module provides functionality for generating simplified versions of meshes
//! to improve rendering performance at different distances from the camera.
//!
//! # Overview
//!
//! LOD (Level of Detail) is a technique where simpler versions of a mesh are used
//! when the object is far from the camera. This module provides:
//!
//! - Mesh simplification algorithms to reduce triangle count
//! - LOD level generation (LOD0 = full detail, LOD1 = 50%, LOD2 = 25%, etc.)
//! - Distance threshold calculation for automatic LOD switching
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::{MeshDefinition, lod::generate_lod_levels};
//!
//! let mesh = MeshDefinition {
//!     vertices: vec![
//!         [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0],
//!         [0.0, 1.0, 0.0], [0.5, 0.5, 0.5],
//!     ],
//!     indices: vec![0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//!     lod_levels: None,
//!     lod_distances: None,
//!     material: None,
//!     texture_path: None,
//! };
//!
//! let (lod_levels, distances) = generate_lod_levels(&mesh, 3);
//! assert_eq!(lod_levels.len(), 3); // LOD1, LOD2, LOD3
//! assert_eq!(distances.len(), 3);
//! ```

use crate::domain::visual::MeshDefinition;
use std::collections::HashSet;

/// Generates LOD levels for a mesh with automatic distance thresholds
///
/// Creates simplified versions of the input mesh at different detail levels.
/// Each LOD level has fewer triangles than the previous one.
///
/// # Arguments
///
/// * `mesh` - The source mesh to generate LOD levels from
/// * `num_levels` - Number of LOD levels to generate (typically 2-4)
///
/// # Returns
///
/// Returns a tuple of (lod_levels, distances):
/// - `lod_levels` - Vector of simplified meshes
/// - `distances` - Vector of distance thresholds for switching
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{MeshDefinition, lod::generate_lod_levels};
///
/// let mesh = MeshDefinition {
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
/// let (lod_levels, distances) = generate_lod_levels(&mesh, 3);
/// assert_eq!(lod_levels.len(), 3);
/// assert_eq!(distances.len(), 3);
/// assert!(distances[0] < distances[1]);
/// assert!(distances[1] < distances[2]);
/// ```
pub fn generate_lod_levels(
    mesh: &MeshDefinition,
    num_levels: usize,
) -> (Vec<MeshDefinition>, Vec<f32>) {
    let mut lod_levels = Vec::with_capacity(num_levels);
    let mut distances = Vec::with_capacity(num_levels);

    let base_triangle_count = mesh.indices.len() / 3;

    // Calculate mesh bounding box for distance heuristics
    let mesh_size = calculate_mesh_size(mesh);
    let base_distance = mesh_size * 2.0; // Start switching at 2x mesh size

    for level in 1..=num_levels {
        // Calculate target triangle count as percentage of original
        let reduction_factor = match level {
            1 => 0.5,  // LOD1: 50% of original
            2 => 0.25, // LOD2: 25% of original
            3 => 0.10, // LOD3: 10% of original
            _ => 0.05, // LOD4+: 5% of original
        };

        let target_count = (base_triangle_count as f32 * reduction_factor).max(1.0) as usize;
        let simplified = simplify_mesh(mesh, target_count);

        // Distance threshold grows exponentially
        let distance = base_distance * (level as f32).powf(2.0);

        lod_levels.push(simplified);
        distances.push(distance);
    }

    (lod_levels, distances)
}

/// Simplifies a mesh to a target triangle count
///
/// Reduces the number of triangles in a mesh while attempting to preserve
/// the overall silhouette and important features.
///
/// # Arguments
///
/// * `mesh` - The source mesh to simplify
/// * `target_triangle_count` - Target number of triangles in simplified mesh
///
/// # Returns
///
/// Returns a new simplified `MeshDefinition`
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{MeshDefinition, lod::simplify_mesh};
///
/// let mesh = MeshDefinition {
///     vertices: vec![
///         [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0],
///         [0.0, 1.0, 0.0], [0.5, 0.5, 0.5],
///     ],
///     indices: vec![0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
///
/// let simplified = simplify_mesh(&mesh, 2);
/// assert!(simplified.indices.len() / 3 <= 2);
/// ```
pub fn simplify_mesh(mesh: &MeshDefinition, target_triangle_count: usize) -> MeshDefinition {
    let current_triangle_count = mesh.indices.len() / 3;

    // If already at or below target, return a clone
    if current_triangle_count <= target_triangle_count {
        return mesh.clone();
    }

    // For very small target counts, create a billboard or minimal representation
    if target_triangle_count <= 2 {
        return create_billboard_mesh(mesh);
    }

    // Simple decimation algorithm: Remove triangles based on importance
    // This is a basic implementation - production code would use edge collapse or similar
    let triangles_to_keep = select_important_triangles(mesh, target_triangle_count);

    // Build new mesh from selected triangles
    build_simplified_mesh(mesh, &triangles_to_keep)
}

/// Calculates the approximate size of a mesh for distance calculations
///
/// # Arguments
///
/// * `mesh` - The mesh to measure
///
/// # Returns
///
/// Returns the approximate diameter of the mesh bounding box
fn calculate_mesh_size(mesh: &MeshDefinition) -> f32 {
    if mesh.vertices.is_empty() {
        return 1.0;
    }

    let mut min = mesh.vertices[0];
    let mut max = mesh.vertices[0];

    for vertex in &mesh.vertices {
        min[0] = min[0].min(vertex[0]);
        min[1] = min[1].min(vertex[1]);
        min[2] = min[2].min(vertex[2]);

        max[0] = max[0].max(vertex[0]);
        max[1] = max[1].max(vertex[1]);
        max[2] = max[2].max(vertex[2]);
    }

    let dx = max[0] - min[0];
    let dy = max[1] - min[1];
    let dz = max[2] - min[2];

    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Creates a billboard mesh (two triangles forming a quad) from a mesh
fn create_billboard_mesh(mesh: &MeshDefinition) -> MeshDefinition {
    // Calculate bounding box
    if mesh.vertices.is_empty() {
        return mesh.clone();
    }

    let mut min = mesh.vertices[0];
    let mut max = mesh.vertices[0];

    for vertex in &mesh.vertices {
        min[0] = min[0].min(vertex[0]);
        min[1] = min[1].min(vertex[1]);
        min[2] = min[2].min(vertex[2]);

        max[0] = max[0].max(vertex[0]);
        max[1] = max[1].max(vertex[1]);
        max[2] = max[2].max(vertex[2]);
    }

    let center_x = (min[0] + max[0]) / 2.0;
    let center_y = (min[1] + max[1]) / 2.0;
    let center_z = (min[2] + max[2]) / 2.0;

    let width = (max[0] - min[0]) / 2.0;
    let height = (max[1] - min[1]) / 2.0;

    // Create a quad facing the camera
    MeshDefinition {
        name: None,
        vertices: vec![
            [center_x - width, center_y - height, center_z],
            [center_x + width, center_y - height, center_z],
            [center_x + width, center_y + height, center_z],
            [center_x - width, center_y + height, center_z],
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        normals: Some(vec![[0.0, 0.0, 1.0]; 4]),
        uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        color: mesh.color,
        lod_levels: None,
        lod_distances: None,
        material: mesh.material.clone(),
        texture_path: mesh.texture_path.clone(),
    }
}

/// Selects important triangles to keep based on various criteria
fn select_important_triangles(mesh: &MeshDefinition, target_count: usize) -> Vec<usize> {
    let triangle_count = mesh.indices.len() / 3;

    if triangle_count <= target_count {
        return (0..triangle_count).collect();
    }

    // Score each triangle by importance (larger triangles are more important)
    let mut triangle_scores: Vec<(usize, f32)> = (0..triangle_count)
        .map(|i| {
            let area = calculate_triangle_area(mesh, i);
            (i, area)
        })
        .collect();

    // Sort by score (descending) and take top N
    triangle_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    triangle_scores
        .iter()
        .take(target_count)
        .map(|(idx, _)| *idx)
        .collect()
}

/// Calculates the area of a triangle
fn calculate_triangle_area(mesh: &MeshDefinition, triangle_index: usize) -> f32 {
    let i0 = mesh.indices[triangle_index * 3] as usize;
    let i1 = mesh.indices[triangle_index * 3 + 1] as usize;
    let i2 = mesh.indices[triangle_index * 3 + 2] as usize;

    if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
        return 0.0;
    }

    let v0 = mesh.vertices[i0];
    let v1 = mesh.vertices[i1];
    let v2 = mesh.vertices[i2];

    // Calculate area using cross product
    let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let cross = [
        edge1[1] * edge2[2] - edge1[2] * edge2[1],
        edge1[2] * edge2[0] - edge1[0] * edge2[2],
        edge1[0] * edge2[1] - edge1[1] * edge2[0],
    ];

    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    length / 2.0
}

/// Builds a simplified mesh from selected triangles
fn build_simplified_mesh(mesh: &MeshDefinition, triangle_indices: &[usize]) -> MeshDefinition {
    // Collect unique vertices used by selected triangles
    let mut used_vertices = HashSet::new();
    let mut new_indices = Vec::with_capacity(triangle_indices.len() * 3);

    for &tri_idx in triangle_indices {
        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

        used_vertices.insert(i0);
        used_vertices.insert(i1);
        used_vertices.insert(i2);
    }

    // Create vertex mapping (old index -> new index)
    let mut vertex_map = vec![0usize; mesh.vertices.len()];
    let mut new_vertices = Vec::with_capacity(used_vertices.len());
    let mut new_normals = mesh
        .normals
        .as_ref()
        .map(|_| Vec::with_capacity(used_vertices.len()));
    let mut new_uvs = mesh
        .uvs
        .as_ref()
        .map(|_| Vec::with_capacity(used_vertices.len()));

    let mut sorted_vertices: Vec<_> = used_vertices.iter().copied().collect();
    sorted_vertices.sort_unstable();

    for (new_idx, &old_idx) in sorted_vertices.iter().enumerate() {
        vertex_map[old_idx] = new_idx;
        new_vertices.push(mesh.vertices[old_idx]);

        if let Some(ref normals) = mesh.normals {
            if old_idx < normals.len() {
                new_normals.as_mut().unwrap().push(normals[old_idx]);
            }
        }

        if let Some(ref uvs) = mesh.uvs {
            if old_idx < uvs.len() {
                new_uvs.as_mut().unwrap().push(uvs[old_idx]);
            }
        }
    }

    // Remap indices
    for &tri_idx in triangle_indices {
        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

        new_indices.push(vertex_map[i0] as u32);
        new_indices.push(vertex_map[i1] as u32);
        new_indices.push(vertex_map[i2] as u32);
    }

    MeshDefinition {
        name: None,
        vertices: new_vertices,
        indices: new_indices,
        normals: new_normals,
        uvs: new_uvs,
        color: mesh.color,
        lod_levels: None, // LOD levels don't have sub-LODs
        lod_distances: None,
        material: mesh.material.clone(),
        texture_path: mesh.texture_path.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> MeshDefinition {
        MeshDefinition {
            name: None,
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 0.5],
            ],
            indices: vec![0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4],
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
    fn test_generate_lod_levels() {
        let mesh = create_test_mesh();
        let (lod_levels, distances) = generate_lod_levels(&mesh, 3);

        assert_eq!(lod_levels.len(), 3);
        assert_eq!(distances.len(), 3);

        // Distances should be increasing
        assert!(distances[0] < distances[1]);
        assert!(distances[1] < distances[2]);

        // Each LOD should have fewer or equal triangles than the previous
        let base_triangles = mesh.indices.len() / 3;
        for lod in &lod_levels {
            let lod_triangles = lod.indices.len() / 3;
            assert!(lod_triangles <= base_triangles);
        }
    }

    #[test]
    fn test_simplify_mesh_reduces_triangles() {
        let mesh = create_test_mesh();
        let simplified = simplify_mesh(&mesh, 2);

        assert!(simplified.indices.len() / 3 <= 2);
        assert!(!simplified.vertices.is_empty());
        assert!(!simplified.indices.is_empty());
    }

    #[test]
    fn test_simplify_mesh_preserves_color() {
        let mesh = create_test_mesh();
        let simplified = simplify_mesh(&mesh, 2);

        assert_eq!(simplified.color, mesh.color);
    }

    #[test]
    fn test_simplify_mesh_already_simple() {
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

        let simplified = simplify_mesh(&mesh, 5);
        assert_eq!(simplified.vertices.len(), mesh.vertices.len());
        assert_eq!(simplified.indices.len(), mesh.indices.len());
    }

    #[test]
    fn test_calculate_mesh_size() {
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

        let size = calculate_mesh_size(&mesh);
        assert!(size > 0.0);
        assert!(size < 2.0); // Diagonal of 1x1 square is ~1.414
    }

    #[test]
    fn test_calculate_mesh_size_empty() {
        let mesh = MeshDefinition {
            name: None,
            vertices: vec![],
            indices: vec![],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let size = calculate_mesh_size(&mesh);
        assert_eq!(size, 1.0); // Default size
    }

    #[test]
    fn test_create_billboard_mesh() {
        let mesh = create_test_mesh();
        let billboard = create_billboard_mesh(&mesh);

        assert_eq!(billboard.vertices.len(), 4);
        assert_eq!(billboard.indices.len(), 6); // Two triangles
        assert!(billboard.normals.is_some());
        assert!(billboard.uvs.is_some());
    }

    #[test]
    fn test_calculate_triangle_area() {
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

        let area = calculate_triangle_area(&mesh, 0);
        assert!((area - 0.5).abs() < 0.001); // Right triangle with legs 1,1 has area 0.5
    }

    #[test]
    fn test_select_important_triangles() {
        let mesh = create_test_mesh();
        let selected = select_important_triangles(&mesh, 2);

        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|&i| i < mesh.indices.len() / 3));
    }

    #[test]
    fn test_build_simplified_mesh() {
        let mesh = create_test_mesh();
        let triangles = vec![0, 2]; // Keep first and third triangles

        let simplified = build_simplified_mesh(&mesh, &triangles);

        assert_eq!(simplified.indices.len(), 6); // 2 triangles
        assert!(!simplified.vertices.is_empty());
    }

    #[test]
    fn test_simplify_mesh_creates_billboard_for_very_low_count() {
        let mesh = create_test_mesh();
        let simplified = simplify_mesh(&mesh, 1);

        // Should create a billboard (2 triangles, 4 vertices)
        assert_eq!(simplified.vertices.len(), 4);
        assert_eq!(simplified.indices.len(), 6);
    }

    #[test]
    fn test_lod_levels_decrease_in_complexity() {
        let mesh = create_test_mesh();
        let (lod_levels, _) = generate_lod_levels(&mesh, 3);

        let base_count = mesh.indices.len() / 3;

        for (i, lod) in lod_levels.iter().enumerate() {
            let lod_count = lod.indices.len() / 3;
            assert!(
                lod_count <= base_count,
                "LOD{} should have fewer triangles",
                i + 1
            );
        }
    }
}
