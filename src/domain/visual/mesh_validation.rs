// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh validation functions
//!
//! This module provides validation for mesh definitions to ensure they are
//! well-formed and can be safely rendered.
//!
//! # Validation Rules
//!
//! - Meshes must have at least 3 vertices (minimum for a triangle)
//! - Index count must be divisible by 3 (triangles only)
//! - All indices must be within vertex bounds
//! - If normals provided, count must match vertex count
//! - If UVs provided, count must match vertex count
//! - No degenerate triangles (duplicate vertex indices in same triangle)
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::{MeshDefinition, mesh_validation};
//!
//! let valid_mesh = MeshDefinition {
//!     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
//!     indices: vec![0, 1, 2],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//! };
//!
//! assert!(mesh_validation::validate_mesh_definition(&valid_mesh).is_ok());
//! ```

use super::MeshDefinition;

/// Validates a mesh definition for correctness
///
/// Performs comprehensive validation to ensure the mesh can be safely rendered.
///
/// # Errors
///
/// Returns `Err(String)` with a descriptive message if validation fails.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{MeshDefinition, mesh_validation};
///
/// let mesh = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
/// };
///
/// assert!(mesh_validation::validate_mesh_definition(&mesh).is_ok());
/// ```
pub fn validate_mesh_definition(mesh: &MeshDefinition) -> Result<(), String> {
    validate_vertices(&mesh.vertices)?;
    validate_indices(&mesh.indices, mesh.vertices.len())?;

    if let Some(ref normals) = mesh.normals {
        validate_normals(normals, mesh.vertices.len())?;
    }

    if let Some(ref uvs) = mesh.uvs {
        validate_uvs(uvs, mesh.vertices.len())?;
    }

    validate_color(&mesh.color)?;

    Ok(())
}

/// Validates vertex data
///
/// Ensures there are at least 3 vertices (minimum for a triangle).
///
/// # Errors
///
/// Returns error if vertex count is less than 3.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::mesh_validation::validate_vertices;
///
/// let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
/// assert!(validate_vertices(&vertices).is_ok());
///
/// let too_few = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
/// assert!(validate_vertices(&too_few).is_err());
/// ```
pub fn validate_vertices(vertices: &[[f32; 3]]) -> Result<(), String> {
    if vertices.is_empty() {
        return Err("Mesh must have at least one vertex".to_string());
    }

    if vertices.len() < 3 {
        return Err(format!(
            "Mesh must have at least 3 vertices for a triangle, got {}",
            vertices.len()
        ));
    }

    // Check for NaN or infinite values
    for (i, vertex) in vertices.iter().enumerate() {
        for (j, &coord) in vertex.iter().enumerate() {
            if !coord.is_finite() {
                return Err(format!(
                    "Vertex {} coordinate {} is not finite: {}",
                    i, j, coord
                ));
            }
        }
    }

    Ok(())
}

/// Validates index data
///
/// Ensures indices form valid triangles and reference existing vertices.
///
/// # Errors
///
/// Returns error if:
/// - Index count is not divisible by 3
/// - Any index is out of bounds
/// - A triangle has duplicate indices (degenerate)
///
/// # Examples
///
/// ```
/// use antares::domain::visual::mesh_validation::validate_indices;
///
/// let indices = vec![0, 1, 2];
/// assert!(validate_indices(&indices, 3).is_ok());
///
/// let bad_indices = vec![0, 1, 5]; // Index 5 out of bounds for 3 vertices
/// assert!(validate_indices(&bad_indices, 3).is_err());
/// ```
pub fn validate_indices(indices: &[u32], vertex_count: usize) -> Result<(), String> {
    if indices.is_empty() {
        return Err("Mesh must have at least one triangle (3 indices)".to_string());
    }

    if !indices.len().is_multiple_of(3) {
        return Err(format!(
            "Index count must be divisible by 3 (triangles), got {}",
            indices.len()
        ));
    }

    let vertex_count_u32 = vertex_count as u32;

    // Validate each triangle
    for (tri_idx, triangle) in indices.chunks(3).enumerate() {
        let i0 = triangle[0];
        let i1 = triangle[1];
        let i2 = triangle[2];

        // Check bounds
        if i0 >= vertex_count_u32 {
            return Err(format!(
                "Triangle {} index 0 ({}) out of bounds (vertex count: {})",
                tri_idx, i0, vertex_count
            ));
        }
        if i1 >= vertex_count_u32 {
            return Err(format!(
                "Triangle {} index 1 ({}) out of bounds (vertex count: {})",
                tri_idx, i1, vertex_count
            ));
        }
        if i2 >= vertex_count_u32 {
            return Err(format!(
                "Triangle {} index 2 ({}) out of bounds (vertex count: {})",
                tri_idx, i2, vertex_count
            ));
        }

        // Check for degenerate triangles (duplicate indices)
        if i0 == i1 || i1 == i2 || i0 == i2 {
            return Err(format!(
                "Triangle {} is degenerate (duplicate indices: {}, {}, {})",
                tri_idx, i0, i1, i2
            ));
        }
    }

    Ok(())
}

/// Validates normal data
///
/// Ensures normal count matches vertex count.
///
/// # Errors
///
/// Returns error if normal count doesn't match vertex count.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::mesh_validation::validate_normals;
///
/// let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
/// assert!(validate_normals(&normals, 3).is_ok());
///
/// let wrong_count = vec![[0.0, 0.0, 1.0]];
/// assert!(validate_normals(&wrong_count, 3).is_err());
/// ```
pub fn validate_normals(normals: &[[f32; 3]], vertex_count: usize) -> Result<(), String> {
    if normals.len() != vertex_count {
        return Err(format!(
            "Normal count ({}) must match vertex count ({})",
            normals.len(),
            vertex_count
        ));
    }

    // Check for NaN or infinite values
    for (i, normal) in normals.iter().enumerate() {
        for (j, &coord) in normal.iter().enumerate() {
            if !coord.is_finite() {
                return Err(format!(
                    "Normal {} coordinate {} is not finite: {}",
                    i, j, coord
                ));
            }
        }
    }

    Ok(())
}

/// Validates UV coordinate data
///
/// Ensures UV count matches vertex count.
///
/// # Errors
///
/// Returns error if UV count doesn't match vertex count.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::mesh_validation::validate_uvs;
///
/// let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
/// assert!(validate_uvs(&uvs, 3).is_ok());
///
/// let wrong_count = vec![[0.0, 0.0]];
/// assert!(validate_uvs(&wrong_count, 3).is_err());
/// ```
pub fn validate_uvs(uvs: &[[f32; 2]], vertex_count: usize) -> Result<(), String> {
    if uvs.len() != vertex_count {
        return Err(format!(
            "UV count ({}) must match vertex count ({})",
            uvs.len(),
            vertex_count
        ));
    }

    // Check for NaN or infinite values
    for (i, uv) in uvs.iter().enumerate() {
        for (j, &coord) in uv.iter().enumerate() {
            if !coord.is_finite() {
                return Err(format!(
                    "UV {} coordinate {} is not finite: {}",
                    i, j, coord
                ));
            }
        }
    }

    Ok(())
}

/// Validates color data
///
/// Ensures color components are in valid range [0.0, 1.0].
///
/// # Errors
///
/// Returns error if any color component is out of range or not finite.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::mesh_validation::validate_color;
///
/// let valid_color = [1.0, 0.5, 0.0, 1.0];
/// assert!(validate_color(&valid_color).is_ok());
///
/// let invalid_color = [1.5, 0.0, 0.0, 1.0]; // Red > 1.0
/// assert!(validate_color(&invalid_color).is_err());
/// ```
pub fn validate_color(color: &[f32; 4]) -> Result<(), String> {
    let components = ["red", "green", "blue", "alpha"];

    for (i, &value) in color.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "Color component {} ({}) is not finite: {}",
                components[i], i, value
            ));
        }

        if !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "Color component {} ({}) must be in range [0.0, 1.0], got {}",
                components[i], i, value
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_triangle() -> MeshDefinition {
        MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn test_validate_mesh_valid_triangle() {
        let mesh = create_valid_triangle();
        assert!(validate_mesh_definition(&mesh).is_ok());
    }

    #[test]
    fn test_validate_vertices_empty() {
        let vertices: Vec<[f32; 3]> = vec![];
        assert!(validate_vertices(&vertices).is_err());
        assert!(validate_vertices(&vertices)
            .unwrap_err()
            .contains("at least one vertex"));
    }

    #[test]
    fn test_validate_vertices_too_few() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        assert!(validate_vertices(&vertices).is_err());
        assert!(validate_vertices(&vertices)
            .unwrap_err()
            .contains("at least 3 vertices"));
    }

    #[test]
    fn test_validate_vertices_valid() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        assert!(validate_vertices(&vertices).is_ok());
    }

    #[test]
    fn test_validate_vertices_nan() {
        let vertices = vec![[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        assert!(validate_vertices(&vertices).is_err());
        assert!(validate_vertices(&vertices)
            .unwrap_err()
            .contains("not finite"));
    }

    #[test]
    fn test_validate_vertices_infinite() {
        let vertices = vec![[f32::INFINITY, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        assert!(validate_vertices(&vertices).is_err());
    }

    #[test]
    fn test_validate_indices_empty() {
        let indices: Vec<u32> = vec![];
        assert!(validate_indices(&indices, 3).is_err());
        assert!(validate_indices(&indices, 3)
            .unwrap_err()
            .contains("at least one triangle"));
    }

    #[test]
    fn test_validate_indices_not_divisible_by_three() {
        let indices = vec![0, 1]; // Only 2 indices
        assert!(validate_indices(&indices, 3).is_err());
        assert!(validate_indices(&indices, 3)
            .unwrap_err()
            .contains("divisible by 3"));
    }

    #[test]
    fn test_validate_indices_out_of_bounds() {
        let indices = vec![0, 1, 5]; // Index 5 out of bounds for 3 vertices
        assert!(validate_indices(&indices, 3).is_err());
        assert!(validate_indices(&indices, 3)
            .unwrap_err()
            .contains("out of bounds"));
    }

    #[test]
    fn test_validate_indices_degenerate_triangle() {
        let indices = vec![0, 1, 1]; // Duplicate index 1
        assert!(validate_indices(&indices, 3).is_err());
        assert!(validate_indices(&indices, 3)
            .unwrap_err()
            .contains("degenerate"));
    }

    #[test]
    fn test_validate_indices_valid() {
        let indices = vec![0, 1, 2, 2, 1, 0]; // Two triangles
        assert!(validate_indices(&indices, 3).is_ok());
    }

    #[test]
    fn test_validate_normals_wrong_count() {
        let normals = vec![[0.0, 0.0, 1.0]]; // Only 1 normal for 3 vertices
        assert!(validate_normals(&normals, 3).is_err());
        assert!(validate_normals(&normals, 3)
            .unwrap_err()
            .contains("must match vertex count"));
    }

    #[test]
    fn test_validate_normals_valid() {
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        assert!(validate_normals(&normals, 3).is_ok());
    }

    #[test]
    fn test_validate_normals_nan() {
        let normals = vec![[f32::NAN, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        assert!(validate_normals(&normals, 3).is_err());
    }

    #[test]
    fn test_validate_uvs_wrong_count() {
        let uvs = vec![[0.0, 0.0]]; // Only 1 UV for 3 vertices
        assert!(validate_uvs(&uvs, 3).is_err());
        assert!(validate_uvs(&uvs, 3)
            .unwrap_err()
            .contains("must match vertex count"));
    }

    #[test]
    fn test_validate_uvs_valid() {
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
        assert!(validate_uvs(&uvs, 3).is_ok());
    }

    #[test]
    fn test_validate_uvs_nan() {
        let uvs = vec![[f32::NAN, 0.0], [1.0, 0.0], [0.5, 1.0]];
        assert!(validate_uvs(&uvs, 3).is_err());
    }

    #[test]
    fn test_validate_color_valid() {
        let color = [1.0, 0.5, 0.0, 1.0];
        assert!(validate_color(&color).is_ok());
    }

    #[test]
    fn test_validate_color_out_of_range_high() {
        let color = [1.5, 0.0, 0.0, 1.0]; // Red > 1.0
        assert!(validate_color(&color).is_err());
        assert!(validate_color(&color)
            .unwrap_err()
            .contains("must be in range"));
    }

    #[test]
    fn test_validate_color_out_of_range_low() {
        let color = [-0.1, 0.0, 0.0, 1.0]; // Red < 0.0
        assert!(validate_color(&color).is_err());
    }

    #[test]
    fn test_validate_color_nan() {
        let color = [f32::NAN, 0.0, 0.0, 1.0];
        assert!(validate_color(&color).is_err());
        assert!(validate_color(&color).unwrap_err().contains("not finite"));
    }

    #[test]
    fn test_validate_mesh_with_normals() {
        let mut mesh = create_valid_triangle();
        mesh.normals = Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);
        assert!(validate_mesh_definition(&mesh).is_ok());
    }

    #[test]
    fn test_validate_mesh_with_invalid_normals() {
        let mut mesh = create_valid_triangle();
        mesh.normals = Some(vec![[0.0, 0.0, 1.0]]); // Wrong count
        assert!(validate_mesh_definition(&mesh).is_err());
    }

    #[test]
    fn test_validate_mesh_with_uvs() {
        let mut mesh = create_valid_triangle();
        mesh.uvs = Some(vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]);
        assert!(validate_mesh_definition(&mesh).is_ok());
    }

    #[test]
    fn test_validate_mesh_with_invalid_uvs() {
        let mut mesh = create_valid_triangle();
        mesh.uvs = Some(vec![[0.0, 0.0]]); // Wrong count
        assert!(validate_mesh_definition(&mesh).is_err());
    }

    #[test]
    fn test_validate_mesh_invalid_color() {
        let mut mesh = create_valid_triangle();
        mesh.color = [2.0, 0.0, 0.0, 1.0]; // Invalid red component
        assert!(validate_mesh_definition(&mesh).is_err());
    }

    #[test]
    fn test_validate_mesh_cube() {
        let cube = MeshDefinition {
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
                3, 2, 6, 6, 7, 3, // Top
                0, 1, 5, 5, 4, 0, // Bottom
            ],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        assert!(validate_mesh_definition(&cube).is_ok());
    }
}
