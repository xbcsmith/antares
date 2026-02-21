// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh validation system for creature editor
//!
//! Provides comprehensive validation of mesh data structures, detecting errors,
//! warnings, and providing informational reports about mesh quality.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::MeshDefinition;
//! use campaign_builder::mesh_validation::validate_mesh;
//!
//! let mesh = MeshDefinition {
//!     name: Some("test_mesh".to_string()),
//!     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
//!     indices: vec![0, 1, 2],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//!     lod_levels: None,
//!     lod_distances: None,
//!     material: None,
//!     texture_path: None,
//! };
//!
//! let report = validate_mesh(&mesh);
//! assert!(report.is_valid());
//! ```

use antares::domain::visual::MeshDefinition;
use std::collections::{HashMap, HashSet};

/// Comprehensive validation report for a mesh
#[derive(Debug, Clone, Default)]
pub struct MeshValidationReport {
    /// Critical errors that make the mesh invalid/unusable
    pub errors: Vec<MeshError>,

    /// Non-critical issues that may cause rendering problems
    pub warnings: Vec<MeshWarning>,

    /// Informational statistics about the mesh
    pub info: Vec<MeshInfo>,
}

impl MeshValidationReport {
    /// Returns true if the mesh has no errors (warnings are acceptable)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns true if the mesh has no errors or warnings
    pub fn is_perfect(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    /// Returns a human-readable summary of the report
    pub fn summary(&self) -> String {
        format!(
            "Errors: {}, Warnings: {}, Info: {}",
            self.errors.len(),
            self.warnings.len(),
            self.info.len()
        )
    }

    /// Returns all errors as formatted strings
    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.to_string()).collect()
    }

    /// Returns all warnings as formatted strings
    pub fn warning_messages(&self) -> Vec<String> {
        self.warnings.iter().map(|w| w.to_string()).collect()
    }

    /// Returns all info messages as formatted strings
    pub fn info_messages(&self) -> Vec<String> {
        self.info.iter().map(|i| i.to_string()).collect()
    }
}

/// Critical mesh errors that prevent valid rendering
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    /// Mesh has no vertices
    NoVertices,

    /// Mesh has no indices
    NoIndices,

    /// Triangle index references non-existent vertex
    InvalidIndex { index: u32, max: usize },

    /// Triangle has zero area (degenerate)
    DegenerateTriangle { triangle_idx: usize },

    /// Edge is shared by more than 2 triangles (non-manifold geometry)
    NonManifoldEdge { vertex_a: u32, vertex_b: u32 },

    /// Normal count doesn't match vertex count
    MismatchedNormalCount { vertices: usize, normals: usize },

    /// UV count doesn't match vertex count
    MismatchedUvCount { vertices: usize, uvs: usize },

    /// Indices list length is not a multiple of 3
    IndicesNotTriangles { count: usize },
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::NoVertices => write!(f, "Mesh has no vertices"),
            MeshError::NoIndices => write!(f, "Mesh has no indices"),
            MeshError::InvalidIndex { index, max } => {
                write!(f, "Invalid index {} (max vertex index: {})", index, max - 1)
            }
            MeshError::DegenerateTriangle { triangle_idx } => {
                write!(f, "Degenerate triangle at index {}", triangle_idx)
            }
            MeshError::NonManifoldEdge { vertex_a, vertex_b } => {
                write!(
                    f,
                    "Non-manifold edge between vertices {} and {}",
                    vertex_a, vertex_b
                )
            }
            MeshError::MismatchedNormalCount { vertices, normals } => {
                write!(
                    f,
                    "Normal count ({}) doesn't match vertex count ({})",
                    normals, vertices
                )
            }
            MeshError::MismatchedUvCount { vertices, uvs } => {
                write!(
                    f,
                    "UV count ({}) doesn't match vertex count ({})",
                    uvs, vertices
                )
            }
            MeshError::IndicesNotTriangles { count } => {
                write!(f, "Index count ({}) is not a multiple of 3", count)
            }
        }
    }
}

/// Non-critical mesh warnings that may indicate issues
#[derive(Debug, Clone, PartialEq)]
pub enum MeshWarning {
    /// Normal vector is not unit length
    UnnormalizedNormal { index: usize, length: f32 },

    /// Two vertices are at the same position
    DuplicateVertex { index_a: usize, index_b: usize },

    /// Vertex is not referenced by any triangle
    UnusedVertex { index: usize },

    /// Triangle has unusually large area
    LargeTriangle { triangle_idx: usize, area: f32 },

    /// Triangle has unusually small area (but not degenerate)
    SmallTriangle { triangle_idx: usize, area: f32 },

    /// Vertex is outside typical coordinate range
    ExtremVertex { index: usize, position: [f32; 3] },
}

impl std::fmt::Display for MeshWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshWarning::UnnormalizedNormal { index, length } => {
                write!(
                    f,
                    "Normal at index {} is not normalized (length: {:.3})",
                    index, length
                )
            }
            MeshWarning::DuplicateVertex { index_a, index_b } => {
                write!(
                    f,
                    "Duplicate vertices at indices {} and {}",
                    index_a, index_b
                )
            }
            MeshWarning::UnusedVertex { index } => {
                write!(f, "Vertex at index {} is not used by any triangle", index)
            }
            MeshWarning::LargeTriangle { triangle_idx, area } => {
                write!(f, "Triangle {} has large area: {:.2}", triangle_idx, area)
            }
            MeshWarning::SmallTriangle { triangle_idx, area } => {
                write!(f, "Triangle {} has small area: {:.6}", triangle_idx, area)
            }
            MeshWarning::ExtremVertex { index, position } => {
                write!(
                    f,
                    "Vertex {} is far from origin: [{:.2}, {:.2}, {:.2}]",
                    index, position[0], position[1], position[2]
                )
            }
        }
    }
}

/// Informational mesh statistics
#[derive(Debug, Clone, PartialEq)]
pub enum MeshInfo {
    /// Total number of vertices
    VertexCount(usize),

    /// Total number of triangles
    TriangleCount(usize),

    /// Axis-aligned bounding box: (min, max)
    BoundingBox { min: [f32; 3], max: [f32; 3] },

    /// Total surface area
    SurfaceArea(f32),

    /// Average triangle area
    AverageTriangleArea(f32),

    /// Whether mesh has normals
    HasNormals(bool),

    /// Whether mesh has UVs
    HasUvs(bool),
}

impl std::fmt::Display for MeshInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshInfo::VertexCount(count) => write!(f, "Vertices: {}", count),
            MeshInfo::TriangleCount(count) => write!(f, "Triangles: {}", count),
            MeshInfo::BoundingBox { min, max } => {
                write!(
                    f,
                    "Bounds: [{:.2}, {:.2}, {:.2}] to [{:.2}, {:.2}, {:.2}]",
                    min[0], min[1], min[2], max[0], max[1], max[2]
                )
            }
            MeshInfo::SurfaceArea(area) => write!(f, "Surface area: {:.2}", area),
            MeshInfo::AverageTriangleArea(area) => write!(f, "Avg triangle area: {:.4}", area),
            MeshInfo::HasNormals(has) => write!(f, "Has normals: {}", has),
            MeshInfo::HasUvs(has) => write!(f, "Has UVs: {}", has),
        }
    }
}

/// Validates a mesh and returns a comprehensive report
///
/// # Arguments
///
/// * `mesh` - The mesh to validate
///
/// # Returns
///
/// A `MeshValidationReport` containing errors, warnings, and info
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshDefinition;
/// use campaign_builder::mesh_validation::validate_mesh;
///
/// let mesh = MeshDefinition {
///     name: Some("cube".to_string()),
///     vertices: vec![
///         [0.0, 0.0, 0.0], [1.0, 0.0, 0.0],
///         [1.0, 1.0, 0.0], [0.0, 1.0, 0.0],
///     ],
///     indices: vec![0, 1, 2, 0, 2, 3],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
///
/// let report = validate_mesh(&mesh);
/// assert!(report.is_valid());
/// ```
pub fn validate_mesh(mesh: &MeshDefinition) -> MeshValidationReport {
    let mut report = MeshValidationReport::default();

    // Basic checks
    if mesh.vertices.is_empty() {
        report.errors.push(MeshError::NoVertices);
        return report; // Can't proceed without vertices
    }

    if mesh.indices.is_empty() {
        report.errors.push(MeshError::NoIndices);
        return report; // Can't proceed without indices
    }

    // Check indices are multiples of 3
    if !mesh.indices.len().is_multiple_of(3) {
        report.errors.push(MeshError::IndicesNotTriangles {
            count: mesh.indices.len(),
        });
        return report;
    }

    // Info: basic stats
    report.info.push(MeshInfo::VertexCount(mesh.vertices.len()));
    report
        .info
        .push(MeshInfo::TriangleCount(mesh.indices.len() / 3));
    report
        .info
        .push(MeshInfo::HasNormals(mesh.normals.is_some()));
    report.info.push(MeshInfo::HasUvs(mesh.uvs.is_some()));

    // Validate indices
    let max_vertex_index = mesh.vertices.len();
    for (i, &index) in mesh.indices.iter().enumerate() {
        if index as usize >= max_vertex_index {
            report.errors.push(MeshError::InvalidIndex {
                index,
                max: max_vertex_index,
            });
        }
    }

    // If we have invalid indices, can't continue with topology checks
    if !report.errors.is_empty() {
        return report;
    }

    // Validate normals if present
    if let Some(ref normals) = mesh.normals {
        if normals.len() != mesh.vertices.len() {
            report.errors.push(MeshError::MismatchedNormalCount {
                vertices: mesh.vertices.len(),
                normals: normals.len(),
            });
        } else {
            // Check normal lengths
            for (i, normal) in normals.iter().enumerate() {
                let length =
                    (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
                if (length - 1.0).abs() > 0.01 {
                    report
                        .warnings
                        .push(MeshWarning::UnnormalizedNormal { index: i, length });
                }
            }
        }
    }

    // Validate UVs if present
    if let Some(ref uvs) = mesh.uvs {
        if uvs.len() != mesh.vertices.len() {
            report.errors.push(MeshError::MismatchedUvCount {
                vertices: mesh.vertices.len(),
                uvs: uvs.len(),
            });
        }
    }

    // Calculate bounding box
    let mut min = mesh.vertices[0];
    let mut max = mesh.vertices[0];
    for vertex in &mesh.vertices {
        for i in 0..3 {
            min[i] = min[i].min(vertex[i]);
            max[i] = max[i].max(vertex[i]);
        }
    }
    report.info.push(MeshInfo::BoundingBox { min, max });

    // Check for extreme vertices
    for (i, vertex) in mesh.vertices.iter().enumerate() {
        let dist_sq = vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2];
        if dist_sq > 10000.0 {
            report.warnings.push(MeshWarning::ExtremVertex {
                index: i,
                position: *vertex,
            });
        }
    }

    // Check for duplicate vertices
    let mut vertex_map: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
    for (i, vertex) in mesh.vertices.iter().enumerate() {
        let key = [
            (vertex[0] * 1000.0) as i32,
            (vertex[1] * 1000.0) as i32,
            (vertex[2] * 1000.0) as i32,
        ];
        vertex_map.entry(key).or_default().push(i);
    }
    for indices in vertex_map.values() {
        if indices.len() > 1 {
            for window in indices.windows(2) {
                report.warnings.push(MeshWarning::DuplicateVertex {
                    index_a: window[0],
                    index_b: window[1],
                });
            }
        }
    }

    // Track used vertices
    let mut used_vertices = HashSet::new();
    for &index in &mesh.indices {
        used_vertices.insert(index as usize);
    }

    // Check for unused vertices
    for i in 0..mesh.vertices.len() {
        if !used_vertices.contains(&i) {
            report.warnings.push(MeshWarning::UnusedVertex { index: i });
        }
    }

    // Validate triangles and calculate surface area
    let mut total_area = 0.0;
    let mut triangle_areas = Vec::new();

    for triangle_idx in 0..(mesh.indices.len() / 3) {
        let i0 = mesh.indices[triangle_idx * 3] as usize;
        let i1 = mesh.indices[triangle_idx * 3 + 1] as usize;
        let i2 = mesh.indices[triangle_idx * 3 + 2] as usize;

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        // Calculate triangle area using cross product
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let cross = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        let area = 0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        triangle_areas.push(area);
        total_area += area;

        // Check for degenerate triangles
        if area < 1e-6 {
            report
                .errors
                .push(MeshError::DegenerateTriangle { triangle_idx });
        } else if area < 0.001 {
            report
                .warnings
                .push(MeshWarning::SmallTriangle { triangle_idx, area });
        } else if area > 100.0 {
            report
                .warnings
                .push(MeshWarning::LargeTriangle { triangle_idx, area });
        }
    }

    report.info.push(MeshInfo::SurfaceArea(total_area));
    if !triangle_areas.is_empty() {
        let avg_area = total_area / triangle_areas.len() as f32;
        report.info.push(MeshInfo::AverageTriangleArea(avg_area));
    }

    // Check for non-manifold edges
    let mut edge_count: HashMap<(u32, u32), usize> = HashMap::new();
    for triangle_idx in 0..(mesh.indices.len() / 3) {
        let i0 = mesh.indices[triangle_idx * 3];
        let i1 = mesh.indices[triangle_idx * 3 + 1];
        let i2 = mesh.indices[triangle_idx * 3 + 2];

        // Add edges (normalized so smaller index comes first)
        for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
            let edge = if a < b { (a, b) } else { (b, a) };
            *edge_count.entry(edge).or_insert(0) += 1;
        }
    }

    for ((a, b), count) in edge_count.iter() {
        if *count > 2 {
            report.errors.push(MeshError::NonManifoldEdge {
                vertex_a: *a,
                vertex_b: *b,
            });
        }
    }

    report
}

/// Quick validation check - returns true if mesh has no errors
///
/// # Arguments
///
/// * `mesh` - The mesh to validate
///
/// # Returns
///
/// `true` if mesh is valid, `false` if it has errors
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshDefinition;
/// use campaign_builder::mesh_validation::is_valid_mesh;
///
/// let mesh = MeshDefinition {
///     name: Some("test".to_string()),
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
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
/// assert!(is_valid_mesh(&mesh));
/// ```
pub fn is_valid_mesh(mesh: &MeshDefinition) -> bool {
    validate_mesh(mesh).is_valid()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> MeshDefinition {
        MeshDefinition {
            name: Some("test".to_string()),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
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
    fn test_valid_mesh_passes() {
        let mesh = create_test_mesh();
        let report = validate_mesh(&mesh);
        assert!(report.is_valid());
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_empty_vertices_fails() {
        let mut mesh = create_test_mesh();
        mesh.vertices.clear();
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(report.errors[0], MeshError::NoVertices));
    }

    #[test]
    fn test_empty_indices_fails() {
        let mut mesh = create_test_mesh();
        mesh.indices.clear();
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(report.errors[0], MeshError::NoIndices));
    }

    #[test]
    fn test_invalid_index_fails() {
        let mut mesh = create_test_mesh();
        mesh.indices = vec![0, 1, 5]; // 5 is out of range
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(report.errors[0], MeshError::InvalidIndex { .. }));
    }

    #[test]
    fn test_non_triangle_indices_fails() {
        let mut mesh = create_test_mesh();
        mesh.indices = vec![0, 1]; // Not a multiple of 3
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(
            report.errors[0],
            MeshError::IndicesNotTriangles { .. }
        ));
    }

    #[test]
    fn test_degenerate_triangle_fails() {
        let mut mesh = create_test_mesh();
        mesh.vertices = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(
            report.errors[0],
            MeshError::DegenerateTriangle { .. }
        ));
    }

    #[test]
    fn test_mismatched_normal_count_fails() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![[0.0, 1.0, 0.0]]); // Only 1 normal for 3 vertices
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(
            report.errors[0],
            MeshError::MismatchedNormalCount { .. }
        ));
    }

    #[test]
    fn test_mismatched_uv_count_fails() {
        let mut mesh = create_test_mesh();
        mesh.uvs = Some(vec![[0.0, 0.0]]); // Only 1 UV for 3 vertices
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(matches!(
            report.errors[0],
            MeshError::MismatchedUvCount { .. }
        ));
    }

    #[test]
    fn test_unnormalized_normal_warns() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![
            [0.0, 2.0, 0.0], // Length 2.0, not normalized
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        let report = validate_mesh(&mesh);
        assert!(report.is_valid()); // Still valid, just a warning
        assert!(!report.warnings.is_empty());
        assert!(matches!(
            report.warnings[0],
            MeshWarning::UnnormalizedNormal { .. }
        ));
    }

    #[test]
    fn test_duplicate_vertices_warns() {
        let mut mesh = create_test_mesh();
        mesh.vertices.push([0.0, 0.0, 0.0]); // Duplicate of first vertex
        mesh.indices = vec![0, 1, 2]; // Don't use the duplicate
        let report = validate_mesh(&mesh);
        assert!(report.is_valid());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn test_unused_vertex_warns() {
        let mut mesh = create_test_mesh();
        mesh.vertices.push([5.0, 5.0, 5.0]); // Extra vertex not referenced
        let report = validate_mesh(&mesh);
        assert!(report.is_valid());
        assert!(report
            .warnings
            .iter()
            .any(|w| matches!(w, MeshWarning::UnusedVertex { .. })));
    }

    #[test]
    fn test_info_fields_populated() {
        let mesh = create_test_mesh();
        let report = validate_mesh(&mesh);
        assert!(report
            .info
            .iter()
            .any(|i| matches!(i, MeshInfo::VertexCount(_))));
        assert!(report
            .info
            .iter()
            .any(|i| matches!(i, MeshInfo::TriangleCount(_))));
        assert!(report
            .info
            .iter()
            .any(|i| matches!(i, MeshInfo::BoundingBox { .. })));
        assert!(report
            .info
            .iter()
            .any(|i| matches!(i, MeshInfo::SurfaceArea(_))));
    }

    #[test]
    fn test_is_valid_mesh_helper() {
        let mesh = create_test_mesh();
        assert!(is_valid_mesh(&mesh));

        let mut invalid_mesh = create_test_mesh();
        invalid_mesh.vertices.clear();
        assert!(!is_valid_mesh(&invalid_mesh));
    }

    #[test]
    fn test_non_manifold_edge_detection() {
        // Create a mesh where an edge is shared by 3 triangles
        let mesh = MeshDefinition {
            name: Some("non_manifold".to_string()),
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
                [0.5, -1.0, 0.0],
                [0.5, 0.0, 1.0],
            ],
            // Three triangles sharing edge 0-1
            indices: vec![0, 1, 2, 0, 1, 3, 0, 1, 4],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let report = validate_mesh(&mesh);
        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, MeshError::NonManifoldEdge { .. })));
    }

    #[test]
    fn test_extreme_vertex_warning() {
        let mut mesh = create_test_mesh();
        mesh.vertices.push([1000.0, 1000.0, 1000.0]);
        mesh.indices = vec![0, 1, 2];
        let report = validate_mesh(&mesh);
        assert!(report.is_valid());
        assert!(report
            .warnings
            .iter()
            .any(|w| matches!(w, MeshWarning::ExtremVertex { .. })));
    }

    #[test]
    fn test_report_summary() {
        let mesh = create_test_mesh();
        let report = validate_mesh(&mesh);
        let summary = report.summary();
        assert!(summary.contains("Errors:"));
        assert!(summary.contains("Warnings:"));
        assert!(summary.contains("Info:"));
    }

    #[test]
    fn test_report_messages() {
        let mut mesh = create_test_mesh();
        mesh.vertices.clear();
        let report = validate_mesh(&mesh);

        let error_msgs = report.error_messages();
        assert!(!error_msgs.is_empty());
        assert!(error_msgs[0].contains("no vertices"));
    }
}
