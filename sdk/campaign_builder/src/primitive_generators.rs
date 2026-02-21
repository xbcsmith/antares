// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Primitive mesh generators for the campaign builder
//!
//! Provides functions to generate common 3D primitive shapes as MeshDefinition
//! instances for use in creature creation.

use antares::domain::visual::MeshDefinition;
use std::f32::consts::PI;

/// Generates a cube mesh with specified size and color
///
/// # Arguments
///
/// * `size` - The side length of the cube
/// * `color` - RGBA color values [r, g, b, a] in range [0.0, 1.0]
///
/// # Returns
///
/// A `MeshDefinition` representing a cube
///
/// # Examples
///
/// ```
/// use campaign_builder::primitive_generators::generate_cube;
///
/// let cube = generate_cube(1.0, [1.0, 0.0, 0.0, 1.0]);
/// assert_eq!(cube.vertices.len(), 24); // 6 faces * 4 vertices
/// ```
pub fn generate_cube(size: f32, color: [f32; 4]) -> MeshDefinition {
    let half = size / 2.0;

    // 24 vertices (4 per face, 6 faces) with proper normals
    #[rustfmt::skip]
    let vertices = vec![
        // Front face (+Z)
        [-half, -half,  half], [ half, -half,  half], [ half,  half,  half], [-half,  half,  half],
        // Back face (-Z)
        [ half, -half, -half], [-half, -half, -half], [-half,  half, -half], [ half,  half, -half],
        // Top face (+Y)
        [-half,  half,  half], [ half,  half,  half], [ half,  half, -half], [-half,  half, -half],
        // Bottom face (-Y)
        [-half, -half, -half], [ half, -half, -half], [ half, -half,  half], [-half, -half,  half],
        // Right face (+X)
        [ half, -half,  half], [ half, -half, -half], [ half,  half, -half], [ half,  half,  half],
        // Left face (-X)
        [-half, -half, -half], [-half, -half,  half], [-half,  half,  half], [-half,  half, -half],
    ];

    #[rustfmt::skip]
    let normals = vec![
        // Front
        [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0],
        // Back
        [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0],
        // Top
        [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
        // Bottom
        [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
        // Right
        [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
        // Left
        [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
    ];

    #[rustfmt::skip]
    let uvs = vec![
        // Front
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        // Back
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        // Top
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        // Bottom
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        // Right
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        // Left
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];

    #[rustfmt::skip]
    let indices = vec![
        // Front
        0, 1, 2,  2, 3, 0,
        // Back
        4, 5, 6,  6, 7, 4,
        // Top
        8, 9, 10,  10, 11, 8,
        // Bottom
        12, 13, 14,  14, 15, 12,
        // Right
        16, 17, 18,  18, 19, 16,
        // Left
        20, 21, 22,  22, 23, 20,
    ];

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: Some(normals),
        uvs: Some(uvs),
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

/// Generates a UV sphere mesh with specified subdivisions
///
/// # Arguments
///
/// * `radius` - The radius of the sphere
/// * `segments` - Number of horizontal segments (longitude)
/// * `rings` - Number of vertical rings (latitude)
/// * `color` - RGBA color values [r, g, b, a] in range [0.0, 1.0]
///
/// # Returns
///
/// A `MeshDefinition` representing a sphere
///
/// # Examples
///
/// ```
/// use campaign_builder::primitive_generators::generate_sphere;
///
/// let sphere = generate_sphere(1.0, 16, 16, [0.0, 1.0, 0.0, 1.0]);
/// assert!(sphere.vertices.len() > 0);
/// ```
pub fn generate_sphere(radius: f32, segments: u32, rings: u32, color: [f32; 4]) -> MeshDefinition {
    let segments = segments.max(3);
    let rings = rings.max(2);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for ring in 0..=rings {
        let theta = (ring as f32 / rings as f32) * PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for segment in 0..=segments {
            let phi = (segment as f32 / segments as f32) * 2.0 * PI;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;

            vertices.push([x * radius, y * radius, z * radius]);
            normals.push([x, y, z]);
            uvs.push([segment as f32 / segments as f32, ring as f32 / rings as f32]);
        }
    }

    // Generate indices
    for ring in 0..rings {
        for segment in 0..segments {
            let current = ring * (segments + 1) + segment;
            let next = current + segments + 1;

            // Two triangles per quad
            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: Some(normals),
        uvs: Some(uvs),
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

/// Generates a cylinder mesh
///
/// # Arguments
///
/// * `radius` - The radius of the cylinder base
/// * `height` - The height of the cylinder
/// * `segments` - Number of radial segments
/// * `color` - RGBA color values [r, g, b, a] in range [0.0, 1.0]
///
/// # Returns
///
/// A `MeshDefinition` representing a cylinder
///
/// # Examples
///
/// ```
/// use campaign_builder::primitive_generators::generate_cylinder;
///
/// let cylinder = generate_cylinder(0.5, 2.0, 16, [0.0, 0.0, 1.0, 1.0]);
/// assert!(cylinder.vertices.len() > 0);
/// ```
pub fn generate_cylinder(
    radius: f32,
    height: f32,
    segments: u32,
    color: [f32; 4],
) -> MeshDefinition {
    let segments = segments.max(3);
    let half_height = height / 2.0;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Generate side vertices
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();

        let u = i as f32 / segments as f32;

        // Bottom vertex
        vertices.push([x * radius, -half_height, z * radius]);
        normals.push([x, 0.0, z]);
        uvs.push([u, 0.0]);

        // Top vertex
        vertices.push([x * radius, half_height, z * radius]);
        normals.push([x, 0.0, z]);
        uvs.push([u, 1.0]);
    }

    // Generate side indices
    for i in 0..segments {
        let bottom_left = i * 2;
        let top_left = bottom_left + 1;
        let bottom_right = (i + 1) * 2;
        let top_right = bottom_right + 1;

        indices.push(bottom_left);
        indices.push(bottom_right);
        indices.push(top_left);

        indices.push(top_left);
        indices.push(bottom_right);
        indices.push(top_right);
    }

    // Add caps
    let cap_start = vertices.len() as u32;

    // Bottom cap center
    vertices.push([0.0, -half_height, 0.0]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Top cap center
    vertices.push([0.0, half_height, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Bottom cap vertices
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();

        vertices.push([x * radius, -half_height, z * radius]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + x * 0.5, 0.5 + z * 0.5]);
    }

    // Top cap vertices
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();

        vertices.push([x * radius, half_height, z * radius]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5 + x * 0.5, 0.5 - z * 0.5]);
    }

    // Bottom cap indices
    let bottom_center = cap_start;
    let bottom_ring_start = cap_start + 2;
    for i in 0..segments {
        indices.push(bottom_center);
        indices.push(bottom_ring_start + i);
        indices.push(bottom_ring_start + i + 1);
    }

    // Top cap indices
    let top_center = cap_start + 1;
    let top_ring_start = bottom_ring_start + segments + 1;
    for i in 0..segments {
        indices.push(top_center);
        indices.push(top_ring_start + i + 1);
        indices.push(top_ring_start + i);
    }

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: Some(normals),
        uvs: Some(uvs),
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

/// Generates a cone mesh
///
/// # Arguments
///
/// * `radius` - The radius of the cone base
/// * `height` - The height of the cone
/// * `segments` - Number of radial segments
/// * `color` - RGBA color values [r, g, b, a] in range [0.0, 1.0]
///
/// # Returns
///
/// A `MeshDefinition` representing a cone
///
/// # Examples
///
/// ```
/// use campaign_builder::primitive_generators::generate_cone;
///
/// let cone = generate_cone(0.5, 1.5, 16, [1.0, 1.0, 0.0, 1.0]);
/// assert!(cone.vertices.len() > 0);
/// ```
pub fn generate_cone(radius: f32, height: f32, segments: u32, color: [f32; 4]) -> MeshDefinition {
    let segments = segments.max(3);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Apex
    let apex_idx = 0;
    vertices.push([0.0, height, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 1.0]);

    // Side vertices (base ring)
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();

        vertices.push([x * radius, 0.0, z * radius]);

        // Approximate side normal
        let slope_normal_y = radius / height;
        let normal_length = (1.0 + slope_normal_y * slope_normal_y).sqrt();
        normals.push([
            x / normal_length,
            slope_normal_y / normal_length,
            z / normal_length,
        ]);

        uvs.push([i as f32 / segments as f32, 0.0]);
    }

    // Side triangle fan
    for i in 0..segments {
        indices.push(apex_idx);
        indices.push(i + 1);
        indices.push(i + 2);
    }

    // Base cap
    let base_center_idx = vertices.len() as u32;
    vertices.push([0.0, 0.0, 0.0]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Base cap vertices
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();

        vertices.push([x * radius, 0.0, z * radius]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + x * 0.5, 0.5 + z * 0.5]);
    }

    // Base cap indices
    let base_ring_start = base_center_idx + 1;
    for i in 0..segments {
        indices.push(base_center_idx);
        indices.push(base_ring_start + i + 1);
        indices.push(base_ring_start + i);
    }

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: Some(normals),
        uvs: Some(uvs),
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

/// Generates a square pyramid mesh with specified base size and color
///
/// # Arguments
///
/// * `base_size` - The side length of the square base
/// * `color` - RGBA color values [r, g, b, a] in range [0.0, 1.0]
///
/// # Returns
///
/// A `MeshDefinition` representing a pyramid
///
/// # Examples
///
/// ```
/// use campaign_builder::primitive_generators::generate_pyramid;
///
/// let pyramid = generate_pyramid(1.0, [1.0, 1.0, 0.0, 1.0]);
/// assert_eq!(pyramid.vertices.len(), 5); // 4 base + 1 apex
/// ```
pub fn generate_pyramid(base_size: f32, color: [f32; 4]) -> MeshDefinition {
    let half = base_size / 2.0;
    let height = base_size; // Height equals base size for proportional pyramid

    // 5 vertices: 4 base corners + 1 apex
    #[rustfmt::skip]
    let vertices = vec![
        // Base (bottom face, Y = 0)
        [-half, 0.0, -half],  // 0: back-left
        [ half, 0.0, -half],  // 1: back-right
        [ half, 0.0,  half],  // 2: front-right
        [-half, 0.0,  half],  // 3: front-left
        // Apex
        [0.0, height, 0.0],   // 4: top
    ];

    // Calculate face normals for each triangular face
    let base_normal = [0.0, -1.0, 0.0];

    // For sloped faces, calculate normals pointing outward
    let front_normal = [0.0, 0.5, 1.0]; // Approximate, will normalize
    let back_normal = [0.0, 0.5, -1.0];
    let left_normal = [-1.0, 0.5, 0.0];
    let right_normal = [1.0, 0.5, 0.0];

    // Normalize the sloped normals
    let normalize = |v: [f32; 3]| -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / len, v[1] / len, v[2] / len]
    };

    let front_normal = normalize(front_normal);
    let back_normal = normalize(back_normal);
    let left_normal = normalize(left_normal);
    let right_normal = normalize(right_normal);

    #[rustfmt::skip]
    let normals = vec![
        base_normal, base_normal, base_normal, base_normal,  // Base vertices
        [0.0, 1.0, 0.0],  // Apex (average of all face normals pointing up)
    ];

    #[rustfmt::skip]
    let uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],  // Base
        [0.5, 0.5],  // Apex
    ];

    // Indices for 5 faces: 1 base (2 triangles) + 4 sides (1 triangle each)
    #[rustfmt::skip]
    let indices = vec![
        // Base (looking up from below, clockwise)
        0, 2, 1,
        0, 3, 2,
        // Front face
        3, 4, 2,
        // Right face
        2, 4, 1,
        // Back face
        1, 4, 0,
        // Left face
        0, 4, 3,
    ];

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: Some(normals),
        uvs: Some(uvs),
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cube_has_correct_vertex_count() {
        let cube = generate_cube(1.0, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(cube.vertices.len(), 24); // 6 faces * 4 vertices
        assert_eq!(cube.indices.len(), 36); // 6 faces * 2 triangles * 3 indices
    }

    #[test]
    fn test_generate_cube_has_normals_and_uvs() {
        let cube = generate_cube(2.0, [0.5, 0.5, 0.5, 1.0]);
        assert!(cube.normals.is_some());
        assert!(cube.uvs.is_some());
        assert_eq!(cube.normals.as_ref().unwrap().len(), 24);
        assert_eq!(cube.uvs.as_ref().unwrap().len(), 24);
    }

    #[test]
    fn test_generate_sphere_minimum_subdivisions() {
        let sphere = generate_sphere(1.0, 3, 2, [1.0, 0.0, 0.0, 1.0]);
        assert!(!sphere.vertices.is_empty());
        assert!(!sphere.indices.is_empty());
        assert!(sphere.normals.is_some());
        assert!(sphere.uvs.is_some());
    }

    #[test]
    fn test_generate_sphere_with_subdivisions() {
        let sphere = generate_sphere(1.0, 16, 16, [0.0, 1.0, 0.0, 1.0]);
        // (rings + 1) * (segments + 1) vertices
        assert_eq!(sphere.vertices.len(), 17 * 17);
        assert!(!sphere.indices.is_empty());
    }

    #[test]
    fn test_generate_cylinder_has_caps() {
        let cylinder = generate_cylinder(0.5, 2.0, 8, [0.0, 0.0, 1.0, 1.0]);
        assert!(!cylinder.vertices.is_empty());
        assert!(!cylinder.indices.is_empty());
        assert!(cylinder.normals.is_some());
        assert!(cylinder.uvs.is_some());
    }

    #[test]
    fn test_generate_cone_has_base() {
        let cone = generate_cone(0.5, 1.5, 8, [1.0, 1.0, 0.0, 1.0]);
        assert!(!cone.vertices.is_empty());
        assert!(!cone.indices.is_empty());
        assert!(cone.normals.is_some());
        assert!(cone.uvs.is_some());
    }

    #[test]
    fn test_cube_respects_size_parameter() {
        let cube = generate_cube(4.0, [1.0, 1.0, 1.0, 1.0]);
        // Check that some vertices are at expected positions
        let max_coord = cube
            .vertices
            .iter()
            .flat_map(|v| v.iter())
            .map(|&c| c.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!((max_coord - 2.0).abs() < 0.001); // half_size = 2.0
    }

    #[test]
    fn test_sphere_respects_radius_parameter() {
        let sphere = generate_sphere(3.0, 8, 8, [1.0, 1.0, 1.0, 1.0]);
        // Check that vertex distances from origin are approximately radius
        let distances: Vec<f32> = sphere
            .vertices
            .iter()
            .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
            .collect();

        for &dist in &distances {
            assert!((dist - 3.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_primitives_respect_color_parameter() {
        let red = [1.0, 0.0, 0.0, 1.0];
        let cube = generate_cube(1.0, red);
        assert_eq!(cube.color, red);

        let green = [0.0, 1.0, 0.0, 0.5];
        let sphere = generate_sphere(1.0, 8, 8, green);
        assert_eq!(sphere.color, green);
    }

    #[test]
    fn test_cylinder_height_parameter() {
        let cylinder = generate_cylinder(1.0, 4.0, 8, [1.0, 1.0, 1.0, 1.0]);
        // Check that top and bottom vertices are at +/- height/2
        let y_coords: Vec<f32> = cylinder.vertices.iter().map(|v| v[1]).collect();
        let max_y = y_coords
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let min_y = y_coords
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        assert!((*max_y - 2.0).abs() < 0.001);
        assert!((*min_y + 2.0).abs() < 0.001);
    }

    #[test]
    fn test_cone_apex_at_top() {
        let cone = generate_cone(1.0, 2.0, 8, [1.0, 1.0, 1.0, 1.0]);
        // First vertex should be apex at (0, height, 0)
        assert_eq!(cone.vertices[0], [0.0, 2.0, 0.0]);
    }

    #[test]
    fn test_generate_pyramid_has_five_vertices() {
        let pyramid = generate_pyramid(1.0, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(pyramid.vertices.len(), 5); // 4 base + 1 apex
        assert!(!pyramid.indices.is_empty());
        assert!(pyramid.normals.is_some());
        assert!(pyramid.uvs.is_some());
    }

    #[test]
    fn test_pyramid_apex_position() {
        let pyramid = generate_pyramid(2.0, [1.0, 1.0, 1.0, 1.0]);
        // Apex should be at (0, height, 0) where height = base_size
        assert_eq!(pyramid.vertices[4], [0.0, 2.0, 0.0]);
    }

    #[test]
    fn test_pyramid_base_size() {
        let base_size = 3.0;
        let pyramid = generate_pyramid(base_size, [1.0, 1.0, 1.0, 1.0]);
        let half = base_size / 2.0;

        // Check base corners are at expected positions
        assert_eq!(pyramid.vertices[0], [-half, 0.0, -half]);
        assert_eq!(pyramid.vertices[1], [half, 0.0, -half]);
        assert_eq!(pyramid.vertices[2], [half, 0.0, half]);
        assert_eq!(pyramid.vertices[3], [-half, 0.0, half]);
    }
}
