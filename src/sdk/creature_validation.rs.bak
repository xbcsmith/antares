// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature mesh topology validation
//!
//! This module provides topology validation for creature meshes, checking for
//! common mesh issues like degenerate triangles, inconsistent winding order,
//! non-manifold edges, and isolated vertices.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::MeshDefinition;
//! use antares::sdk::creature_validation::validate_mesh_topology;
//!
//! let mesh = MeshDefinition {
//!     vertices: vec![
//!         [0.0, 0.0, 0.0],
//!         [1.0, 0.0, 0.0],
//!         [0.5, 1.0, 0.0],
//!     ],
//!     indices: vec![0, 1, 2],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//! };
//!
//! let result = validate_mesh_topology(&mesh);
//! assert!(result.is_ok());
//! ```

use crate::domain::visual::{CreatureDefinition, MeshDefinition};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Topology validation errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TopologyError {
    /// Mesh contains degenerate triangles (zero or near-zero area)
    #[error("Mesh contains {count} degenerate triangle(s) with area < {threshold}")]
    DegenerateTriangles { count: usize, threshold: f32 },

    /// Mesh has inconsistent triangle winding order
    #[error("Mesh has inconsistent winding order: {ccw_count} CCW, {cw_count} CW triangles")]
    InconsistentWinding { ccw_count: usize, cw_count: usize },

    /// Mesh contains non-manifold edges (shared by more than 2 triangles)
    #[error("Mesh contains {count} non-manifold edge(s)")]
    NonManifoldEdges { count: usize },

    /// Mesh has invalid indices (out of bounds)
    #[error(
        "Mesh has invalid indices: index {index} out of bounds (vertex count: {vertex_count})"
    )]
    InvalidIndices { index: u32, vertex_count: usize },

    /// Triangle count is not a multiple of 3
    #[error("Index count {count} is not a multiple of 3")]
    InvalidIndexCount { count: usize },
}

/// Topology validation warnings
#[derive(Debug, Clone, PartialEq)]
pub enum TopologyWarning {
    /// Mesh contains isolated vertices (not referenced by any triangle)
    IsolatedVertices { count: usize },

    /// Mesh has very small triangles (but not degenerate)
    SmallTriangles { count: usize, threshold: f32 },

    /// Mesh normals don't match calculated normals
    NormalMismatch { vertex_index: usize },

    /// UV coordinates out of typical 0-1 range
    UVOutOfRange { vertex_index: usize },
}

/// Validation result containing errors and warnings
#[derive(Debug, Clone, Default)]
pub struct TopologyValidation {
    /// Critical errors that prevent mesh use
    pub errors: Vec<TopologyError>,

    /// Non-critical warnings about mesh quality
    pub warnings: Vec<TopologyWarning>,
}

impl TopologyValidation {
    /// Creates a new empty validation result
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns true if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns true if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns total issue count (errors + warnings)
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

/// Validates mesh topology and returns detailed results
///
/// This function performs comprehensive topology validation including:
/// - Degenerate triangle detection
/// - Winding order consistency
/// - Manifold edge validation
/// - Index bounds checking
/// - Isolated vertex detection
///
/// # Arguments
///
/// * `mesh` - The mesh definition to validate
///
/// # Returns
///
/// Returns `Ok(TopologyValidation)` with any warnings, or `Err(TopologyError)` for critical errors
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshDefinition;
/// use antares::sdk::creature_validation::validate_mesh_topology;
///
/// let mesh = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
/// };
///
/// let validation = validate_mesh_topology(&mesh).unwrap();
/// assert!(validation.is_valid());
/// ```
pub fn validate_mesh_topology(mesh: &MeshDefinition) -> Result<TopologyValidation, TopologyError> {
    let mut validation = TopologyValidation::new();

    // Check index count is multiple of 3
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(TopologyError::InvalidIndexCount {
            count: mesh.indices.len(),
        });
    }

    // Check all indices are in bounds
    let vertex_count = mesh.vertices.len();
    for &index in &mesh.indices {
        if (index as usize) >= vertex_count {
            return Err(TopologyError::InvalidIndices {
                index,
                vertex_count,
            });
        }
    }

    // Check for degenerate triangles
    let degenerate_count = count_degenerate_triangles(mesh, 0.0001);
    if degenerate_count > 0 {
        return Err(TopologyError::DegenerateTriangles {
            count: degenerate_count,
            threshold: 0.0001,
        });
    }

    // Check winding order consistency
    let (ccw_count, cw_count) = count_winding_orders(mesh);
    if ccw_count > 0 && cw_count > 0 {
        // Mixed winding order is an error
        return Err(TopologyError::InconsistentWinding {
            ccw_count,
            cw_count,
        });
    }

    // Check for non-manifold edges
    let non_manifold_count = count_non_manifold_edges(mesh);
    if non_manifold_count > 0 {
        return Err(TopologyError::NonManifoldEdges {
            count: non_manifold_count,
        });
    }

    // Check for isolated vertices (warning only)
    let isolated_count = count_isolated_vertices(mesh);
    if isolated_count > 0 {
        validation.warnings.push(TopologyWarning::IsolatedVertices {
            count: isolated_count,
        });
    }

    // Check for small triangles (warning only)
    let small_triangle_count = count_small_triangles(mesh, 0.001);
    if small_triangle_count > 0 {
        validation.warnings.push(TopologyWarning::SmallTriangles {
            count: small_triangle_count,
            threshold: 0.001,
        });
    }

    // Check UV coordinates if present
    if let Some(ref uvs) = mesh.uvs {
        for (i, uv) in uvs.iter().enumerate() {
            if uv[0] < 0.0 || uv[0] > 1.0 || uv[1] < 0.0 || uv[1] > 1.0 {
                validation
                    .warnings
                    .push(TopologyWarning::UVOutOfRange { vertex_index: i });
                break; // Only report first occurrence
            }
        }
    }

    Ok(validation)
}

/// Validates a complete creature definition
///
/// # Arguments
///
/// * `creature` - The creature definition to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, or `Err(TopologyError)` for the first error found
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
/// use antares::sdk::creature_validation::validate_creature_topology;
///
/// let mesh = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
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
/// assert!(validate_creature_topology(&creature).is_ok());
/// ```
pub fn validate_creature_topology(creature: &CreatureDefinition) -> Result<(), TopologyError> {
    for mesh in creature.meshes.iter() {
        validate_mesh_topology(mesh).map_err(|e| {
            // Add mesh index context to error
            match e {
                TopologyError::DegenerateTriangles { count, threshold } => {
                    TopologyError::DegenerateTriangles { count, threshold }
                }
                TopologyError::InconsistentWinding {
                    ccw_count,
                    cw_count,
                } => TopologyError::InconsistentWinding {
                    ccw_count,
                    cw_count,
                },
                TopologyError::NonManifoldEdges { count } => {
                    TopologyError::NonManifoldEdges { count }
                }
                TopologyError::InvalidIndices {
                    index,
                    vertex_count,
                } => TopologyError::InvalidIndices {
                    index,
                    vertex_count,
                },
                TopologyError::InvalidIndexCount { count } => {
                    TopologyError::InvalidIndexCount { count }
                }
            }
        })?;
    }
    Ok(())
}

/// Counts degenerate triangles (zero or near-zero area)
fn count_degenerate_triangles(mesh: &MeshDefinition, threshold: f32) -> usize {
    let mut count = 0;

    for triangle in mesh.indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let v0 = mesh.vertices[triangle[0] as usize];
        let v1 = mesh.vertices[triangle[1] as usize];
        let v2 = mesh.vertices[triangle[2] as usize];

        let area = triangle_area(v0, v1, v2);
        if area < threshold {
            count += 1;
        }
    }

    count
}

/// Counts small triangles (above degenerate threshold but still small)
fn count_small_triangles(mesh: &MeshDefinition, threshold: f32) -> usize {
    let mut count = 0;

    for triangle in mesh.indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let v0 = mesh.vertices[triangle[0] as usize];
        let v1 = mesh.vertices[triangle[1] as usize];
        let v2 = mesh.vertices[triangle[2] as usize];

        let area = triangle_area(v0, v1, v2);
        if area >= 0.0001 && area < threshold {
            count += 1;
        }
    }

    count
}

/// Counts winding orders (CCW and CW)
fn count_winding_orders(mesh: &MeshDefinition) -> (usize, usize) {
    let mut ccw_count = 0;
    let mut cw_count = 0;

    for triangle in mesh.indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let v0 = mesh.vertices[triangle[0] as usize];
        let v1 = mesh.vertices[triangle[1] as usize];
        let v2 = mesh.vertices[triangle[2] as usize];

        let normal = triangle_normal(v0, v1, v2);

        // Assume camera looking down -Z axis (right-handed)
        // CCW winding produces positive Z normal
        if normal[2] > 0.0 {
            ccw_count += 1;
        } else if normal[2] < 0.0 {
            cw_count += 1;
        }
        // Ignore if normal[2] == 0.0 (triangle parallel to view plane)
    }

    (ccw_count, cw_count)
}

/// Counts non-manifold edges (edges shared by more than 2 triangles)
fn count_non_manifold_edges(mesh: &MeshDefinition) -> usize {
    let mut edge_counts: HashMap<(u32, u32), usize> = HashMap::new();

    for triangle in mesh.indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        // Add all three edges (normalized so smaller index first)
        for i in 0..3 {
            let v1 = triangle[i];
            let v2 = triangle[(i + 1) % 3];

            let edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
            *edge_counts.entry(edge).or_insert(0) += 1;
        }
    }

    // Count edges with more than 2 triangles
    edge_counts.values().filter(|&&count| count > 2).count()
}

/// Counts isolated vertices (not referenced by any triangle)
fn count_isolated_vertices(mesh: &MeshDefinition) -> usize {
    let mut used_vertices = HashSet::new();

    for &index in &mesh.indices {
        used_vertices.insert(index as usize);
    }

    mesh.vertices.len() - used_vertices.len()
}

/// Calculates triangle area using cross product
fn triangle_area(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> f32 {
    let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let cross = cross_product(edge1, edge2);
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();

    length * 0.5
}

/// Calculates triangle normal using cross product
fn triangle_normal(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    cross_product(edge1, edge2)
}

/// Calculates cross product of two vectors
fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::MeshTransform;

    fn create_valid_triangle_mesh() -> MeshDefinition {
        MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn test_validate_mesh_topology_valid_triangle() {
        let mesh = create_valid_triangle_mesh();
        let result = validate_mesh_topology(&mesh);

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.is_valid());
        assert!(!validation.has_errors());
    }

    #[test]
    fn test_validate_mesh_topology_degenerate_triangle() {
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TopologyError::DegenerateTriangles { .. }
        ));
    }

    #[test]
    fn test_validate_mesh_topology_invalid_index_count() {
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            indices: vec![0, 1], // Only 2 indices
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TopologyError::InvalidIndexCount { .. }
        ));
    }

    #[test]
    fn test_validate_mesh_topology_out_of_bounds_index() {
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 5], // Index 5 out of bounds
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TopologyError::InvalidIndices { .. }
        ));
    }

    #[test]
    fn test_validate_mesh_topology_isolated_vertices() {
        let mesh = MeshDefinition {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
                [2.0, 2.0, 0.0], // Isolated vertex
            ],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.has_warnings());
        assert!(matches!(
            validation.warnings[0],
            TopologyWarning::IsolatedVertices { count: 1 }
        ));
    }

    #[test]
    fn test_validate_mesh_topology_inconsistent_winding() {
        let mesh = MeshDefinition {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.5, 1.0, 0.0],
            ],
            indices: vec![
                0, 1, 2, // CCW
                3, 1, 4, // CW (reversed)
            ],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TopologyError::InconsistentWinding { .. }
        ));
    }

    #[test]
    fn test_validate_mesh_topology_uv_out_of_range() {
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.5]]), // Last UV out of range
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let result = validate_mesh_topology(&mesh);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.has_warnings());
        assert!(matches!(
            validation.warnings[0],
            TopologyWarning::UVOutOfRange { .. }
        ));
    }

    #[test]
    fn test_validate_creature_topology_valid() {
        let mesh = create_valid_triangle_mesh();
        let creature = CreatureDefinition {
            id: 1,
            name: "Test Creature".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        let result = validate_creature_topology(&creature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_creature_topology_invalid_mesh() {
        let bad_mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0]],
            indices: vec![0, 1, 2], // Indices out of bounds
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let creature = CreatureDefinition {
            id: 1,
            name: "Bad Creature".to_string(),
            meshes: vec![bad_mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        let result = validate_creature_topology(&creature);
        assert!(result.is_err());
    }

    #[test]
    fn test_triangle_area_calculation() {
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];

        let area = triangle_area(v0, v1, v2);
        assert!((area - 0.5).abs() < 0.001); // Area of right triangle with legs 1,1
    }

    #[test]
    fn test_triangle_area_degenerate() {
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [0.0, 0.0, 0.0];
        let v2 = [0.0, 0.0, 0.0];

        let area = triangle_area(v0, v1, v2);
        assert!(area < 0.0001); // Near-zero area
    }

    #[test]
    fn test_cross_product() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let result = cross_product(a, b);

        assert_eq!(result, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_count_isolated_vertices_none() {
        let mesh = create_valid_triangle_mesh();
        let count = count_isolated_vertices(&mesh);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_isolated_vertices_some() {
        let mesh = MeshDefinition {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
            ],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let count = count_isolated_vertices(&mesh);
        assert_eq!(count, 2); // Vertices 3 and 4 are isolated
    }

    #[test]
    fn test_topology_validation_has_errors() {
        let mut validation = TopologyValidation::new();
        assert!(!validation.has_errors());

        validation
            .errors
            .push(TopologyError::InvalidIndexCount { count: 2 });
        assert!(validation.has_errors());
    }

    #[test]
    fn test_topology_validation_has_warnings() {
        let mut validation = TopologyValidation::new();
        assert!(!validation.has_warnings());

        validation
            .warnings
            .push(TopologyWarning::IsolatedVertices { count: 1 });
        assert!(validation.has_warnings());
    }

    #[test]
    fn test_topology_validation_issue_count() {
        let mut validation = TopologyValidation::new();
        assert_eq!(validation.issue_count(), 0);

        validation
            .errors
            .push(TopologyError::InvalidIndexCount { count: 2 });
        validation
            .warnings
            .push(TopologyWarning::IsolatedVertices { count: 1 });

        assert_eq!(validation.issue_count(), 2);
    }

    #[test]
    fn test_count_non_manifold_edges_valid() {
        let mesh = create_valid_triangle_mesh();
        let count = count_non_manifold_edges(&mesh);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_non_manifold_edges_cube() {
        // Simple cube - all edges should be manifold (shared by exactly 2 triangles)
        let mesh = MeshDefinition {
            vertices: vec![
                [-1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
            indices: vec![
                0, 1, 2, 2, 3, 0, // Front
                4, 5, 6, 6, 7, 4, // Back
                0, 4, 7, 7, 3, 0, // Left
                1, 5, 6, 6, 2, 1, // Right
                0, 1, 5, 5, 4, 0, // Bottom
                3, 2, 6, 6, 7, 3, // Top
            ],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let count = count_non_manifold_edges(&mesh);
        assert_eq!(count, 0); // All edges properly manifold
    }
}
