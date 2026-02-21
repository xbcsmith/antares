// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh normal editor for creature editor
//!
//! Provides UI and functionality for editing mesh normals, including automatic
//! calculation, smoothing, and manual manipulation.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::MeshDefinition;
//! use campaign_builder::mesh_normal_editor::MeshNormalEditor;
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
//! let mut editor = MeshNormalEditor::new(mesh);
//! editor.calculate_flat_normals();
//! assert!(editor.mesh().normals.is_some());
//! ```

use antares::domain::visual::MeshDefinition;
use std::collections::HashMap;

/// Normal editor for mesh normal manipulation
#[derive(Debug, Clone)]
pub struct MeshNormalEditor {
    /// The mesh being edited
    mesh: MeshDefinition,

    /// Whether to auto-normalize normals after edits
    auto_normalize: bool,
}

/// Normal calculation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalMode {
    /// Flat shading - one normal per triangle face
    Flat,

    /// Smooth shading - averaged normals across shared vertices
    Smooth,

    /// Weighted smooth - normals weighted by triangle area
    WeightedSmooth,
}

impl MeshNormalEditor {
    /// Creates a new normal editor for the given mesh
    ///
    /// # Arguments
    ///
    /// * `mesh` - The mesh to edit
    ///
    /// # Returns
    ///
    /// A new `MeshNormalEditor`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_normal_editor::MeshNormalEditor;
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
    /// let editor = MeshNormalEditor::new(mesh);
    /// assert!(editor.mesh().normals.is_none());
    /// ```
    pub fn new(mesh: MeshDefinition) -> Self {
        Self {
            mesh,
            auto_normalize: true,
        }
    }

    /// Returns a reference to the mesh being edited
    pub fn mesh(&self) -> &MeshDefinition {
        &self.mesh
    }

    /// Returns a mutable reference to the mesh being edited
    pub fn mesh_mut(&mut self) -> &mut MeshDefinition {
        &mut self.mesh
    }

    /// Consumes the editor and returns the edited mesh
    pub fn into_mesh(self) -> MeshDefinition {
        self.mesh
    }

    /// Returns whether auto-normalize is enabled
    pub fn auto_normalize(&self) -> bool {
        self.auto_normalize
    }

    /// Sets whether to auto-normalize normals after edits
    pub fn set_auto_normalize(&mut self, enabled: bool) {
        self.auto_normalize = enabled;
    }

    /// Calculates flat normals (one per triangle face)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_normal_editor::MeshNormalEditor;
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
    /// let mut editor = MeshNormalEditor::new(mesh);
    /// editor.calculate_flat_normals();
    /// assert!(editor.mesh().normals.is_some());
    /// ```
    pub fn calculate_flat_normals(&mut self) {
        let vertex_count = self.mesh.vertices.len();
        let mut normals = vec![[0.0, 0.0, 0.0]; vertex_count];

        // Calculate normal for each triangle and assign to vertices
        for triangle_idx in 0..(self.mesh.indices.len() / 3) {
            let i0 = self.mesh.indices[triangle_idx * 3] as usize;
            let i1 = self.mesh.indices[triangle_idx * 3 + 1] as usize;
            let i2 = self.mesh.indices[triangle_idx * 3 + 2] as usize;

            let v0 = self.mesh.vertices[i0];
            let v1 = self.mesh.vertices[i1];
            let v2 = self.mesh.vertices[i2];

            let normal = Self::calculate_triangle_normal(v0, v1, v2);

            // Assign the same normal to all three vertices of this triangle
            normals[i0] = normal;
            normals[i1] = normal;
            normals[i2] = normal;
        }

        self.mesh.normals = Some(normals);
    }

    /// Calculates smooth normals (averaged across shared vertices)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_normal_editor::MeshNormalEditor;
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
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
    /// let mut editor = MeshNormalEditor::new(mesh);
    /// editor.calculate_smooth_normals();
    /// assert!(editor.mesh().normals.is_some());
    /// ```
    pub fn calculate_smooth_normals(&mut self) {
        let vertex_count = self.mesh.vertices.len();
        let mut normal_accumulators = vec![[0.0, 0.0, 0.0]; vertex_count];
        let mut normal_counts = vec![0; vertex_count];

        // Accumulate normals for each triangle
        for triangle_idx in 0..(self.mesh.indices.len() / 3) {
            let i0 = self.mesh.indices[triangle_idx * 3] as usize;
            let i1 = self.mesh.indices[triangle_idx * 3 + 1] as usize;
            let i2 = self.mesh.indices[triangle_idx * 3 + 2] as usize;

            let v0 = self.mesh.vertices[i0];
            let v1 = self.mesh.vertices[i1];
            let v2 = self.mesh.vertices[i2];

            let normal = Self::calculate_triangle_normal(v0, v1, v2);

            // Add to each vertex's accumulator
            for &idx in &[i0, i1, i2] {
                normal_accumulators[idx][0] += normal[0];
                normal_accumulators[idx][1] += normal[1];
                normal_accumulators[idx][2] += normal[2];
                normal_counts[idx] += 1;
            }
        }

        // Average and normalize
        let mut normals = vec![[0.0, 0.0, 0.0]; vertex_count];
        for i in 0..vertex_count {
            if normal_counts[i] > 0 {
                let count = normal_counts[i] as f32;
                normals[i] = [
                    normal_accumulators[i][0] / count,
                    normal_accumulators[i][1] / count,
                    normal_accumulators[i][2] / count,
                ];
                normals[i] = Self::normalize(normals[i]);
            } else {
                // Default up normal for unused vertices
                normals[i] = [0.0, 1.0, 0.0];
            }
        }

        self.mesh.normals = Some(normals);
    }

    /// Calculates weighted smooth normals (normals weighted by triangle area)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_normal_editor::MeshNormalEditor;
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
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
    /// let mut editor = MeshNormalEditor::new(mesh);
    /// editor.calculate_weighted_smooth_normals();
    /// assert!(editor.mesh().normals.is_some());
    /// ```
    pub fn calculate_weighted_smooth_normals(&mut self) {
        let vertex_count = self.mesh.vertices.len();
        let mut normal_accumulators = vec![[0.0, 0.0, 0.0]; vertex_count];

        // Accumulate area-weighted normals for each triangle
        for triangle_idx in 0..(self.mesh.indices.len() / 3) {
            let i0 = self.mesh.indices[triangle_idx * 3] as usize;
            let i1 = self.mesh.indices[triangle_idx * 3 + 1] as usize;
            let i2 = self.mesh.indices[triangle_idx * 3 + 2] as usize;

            let v0 = self.mesh.vertices[i0];
            let v1 = self.mesh.vertices[i1];
            let v2 = self.mesh.vertices[i2];

            let (normal, area) = Self::calculate_triangle_normal_and_area(v0, v1, v2);

            // Weight by area
            let weighted_normal = [normal[0] * area, normal[1] * area, normal[2] * area];

            // Add to each vertex's accumulator
            for &idx in &[i0, i1, i2] {
                normal_accumulators[idx][0] += weighted_normal[0];
                normal_accumulators[idx][1] += weighted_normal[1];
                normal_accumulators[idx][2] += weighted_normal[2];
            }
        }

        // Normalize
        let mut normals = vec![[0.0, 0.0, 0.0]; vertex_count];
        for i in 0..vertex_count {
            normals[i] = Self::normalize(normal_accumulators[i]);

            // If normalization failed (zero vector), use default up normal
            if normals[i][0].is_nan() || normals[i][1].is_nan() || normals[i][2].is_nan() {
                normals[i] = [0.0, 1.0, 0.0];
            }
        }

        self.mesh.normals = Some(normals);
    }

    /// Calculates normals using the specified mode
    ///
    /// # Arguments
    ///
    /// * `mode` - The normal calculation mode
    pub fn calculate_normals(&mut self, mode: NormalMode) {
        match mode {
            NormalMode::Flat => self.calculate_flat_normals(),
            NormalMode::Smooth => self.calculate_smooth_normals(),
            NormalMode::WeightedSmooth => self.calculate_weighted_smooth_normals(),
        }
    }

    /// Normalizes all normals in the mesh to unit length
    pub fn normalize_all(&mut self) {
        if let Some(ref mut normals) = self.mesh.normals {
            for normal in normals.iter_mut() {
                *normal = Self::normalize(*normal);
            }
        }
    }

    /// Sets the normal for a specific vertex
    ///
    /// # Arguments
    ///
    /// * `vertex_idx` - The vertex index
    /// * `normal` - The new normal vector
    pub fn set_normal(&mut self, vertex_idx: usize, normal: [f32; 3]) {
        // Ensure normals exist
        if self.mesh.normals.is_none() {
            self.mesh.normals = Some(vec![[0.0, 1.0, 0.0]; self.mesh.vertices.len()]);
        }

        if let Some(ref mut normals) = self.mesh.normals {
            if vertex_idx < normals.len() {
                normals[vertex_idx] = if self.auto_normalize {
                    Self::normalize(normal)
                } else {
                    normal
                };
            }
        }
    }

    /// Gets the normal for a specific vertex
    ///
    /// # Arguments
    ///
    /// * `vertex_idx` - The vertex index
    ///
    /// # Returns
    ///
    /// The normal vector, or None if normals don't exist or index is out of bounds
    pub fn get_normal(&self, vertex_idx: usize) -> Option<[f32; 3]> {
        self.mesh.normals.as_ref()?.get(vertex_idx).copied()
    }

    /// Flips all normals (reverses direction)
    pub fn flip_all_normals(&mut self) {
        if let Some(ref mut normals) = self.mesh.normals {
            for normal in normals.iter_mut() {
                normal[0] = -normal[0];
                normal[1] = -normal[1];
                normal[2] = -normal[2];
            }
        }
    }

    /// Flips normals for specific vertices
    ///
    /// # Arguments
    ///
    /// * `vertex_indices` - Iterator of vertex indices whose normals to flip
    pub fn flip_normals<I>(&mut self, vertex_indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        if let Some(ref mut normals) = self.mesh.normals {
            for idx in vertex_indices {
                if idx < normals.len() {
                    normals[idx][0] = -normals[idx][0];
                    normals[idx][1] = -normals[idx][1];
                    normals[idx][2] = -normals[idx][2];
                }
            }
        }
    }

    /// Removes normals from the mesh
    pub fn remove_normals(&mut self) {
        self.mesh.normals = None;
    }

    /// Smooths normals in a specific region
    ///
    /// # Arguments
    ///
    /// * `vertex_indices` - Vertices to smooth
    /// * `iterations` - Number of smoothing iterations
    pub fn smooth_region(&mut self, vertex_indices: &[usize], iterations: usize) {
        if self.mesh.normals.is_none() {
            self.calculate_smooth_normals();
        }

        // Build adjacency map before mutable borrow
        let adjacency = self.build_vertex_adjacency();

        if let Some(ref mut normals) = self.mesh.normals {
            for _ in 0..iterations {
                let mut new_normals = normals.clone();

                for &idx in vertex_indices {
                    if let Some(neighbors) = adjacency.get(&idx) {
                        let mut avg_normal = [0.0, 0.0, 0.0];
                        let mut count = 1.0;

                        // Include self
                        avg_normal[0] += normals[idx][0];
                        avg_normal[1] += normals[idx][1];
                        avg_normal[2] += normals[idx][2];

                        // Add neighbors
                        for &neighbor_idx in neighbors {
                            avg_normal[0] += normals[neighbor_idx][0];
                            avg_normal[1] += normals[neighbor_idx][1];
                            avg_normal[2] += normals[neighbor_idx][2];
                            count += 1.0;
                        }

                        // Average
                        avg_normal[0] /= count;
                        avg_normal[1] /= count;
                        avg_normal[2] /= count;

                        new_normals[idx] = Self::normalize(avg_normal);
                    }
                }

                *normals = new_normals;
            }
        }
    }

    /// Builds a map of vertex adjacency (which vertices share edges)
    fn build_vertex_adjacency(&self) -> HashMap<usize, Vec<usize>> {
        let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();

        for triangle_idx in 0..(self.mesh.indices.len() / 3) {
            let i0 = self.mesh.indices[triangle_idx * 3] as usize;
            let i1 = self.mesh.indices[triangle_idx * 3 + 1] as usize;
            let i2 = self.mesh.indices[triangle_idx * 3 + 2] as usize;

            // Add bidirectional edges
            adjacency.entry(i0).or_default().push(i1);
            adjacency.entry(i0).or_default().push(i2);
            adjacency.entry(i1).or_default().push(i0);
            adjacency.entry(i1).or_default().push(i2);
            adjacency.entry(i2).or_default().push(i0);
            adjacency.entry(i2).or_default().push(i1);
        }

        // Remove duplicates
        for neighbors in adjacency.values_mut() {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        adjacency
    }

    /// Calculates the normal for a triangle
    fn calculate_triangle_normal(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let cross = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        Self::normalize(cross)
    }

    /// Calculates the normal and area for a triangle
    fn calculate_triangle_normal_and_area(
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
    ) -> ([f32; 3], f32) {
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let cross = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        let area = 0.5 * length;

        let normal = if length > 1e-6 {
            [cross[0] / length, cross[1] / length, cross[2] / length]
        } else {
            [0.0, 1.0, 0.0]
        };

        (normal, area)
    }

    /// Normalizes a vector to unit length
    fn normalize(v: [f32; 3]) -> [f32; 3] {
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if length > 1e-6 {
            [v[0] / length, v[1] / length, v[2] / length]
        } else {
            [0.0, 1.0, 0.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> MeshDefinition {
        MeshDefinition {
            name: Some("test".to_string()),
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        }
    }

    fn approximately_equal(a: [f32; 3], b: [f32; 3], epsilon: f32) -> bool {
        (a[0] - b[0]).abs() < epsilon
            && (a[1] - b[1]).abs() < epsilon
            && (a[2] - b[2]).abs() < epsilon
    }

    #[test]
    fn test_new_editor() {
        let mesh = create_test_mesh();
        let editor = MeshNormalEditor::new(mesh);
        assert!(editor.mesh().normals.is_none());
        assert!(editor.auto_normalize());
    }

    #[test]
    fn test_calculate_flat_normals() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);
        editor.calculate_flat_normals();

        assert!(editor.mesh().normals.is_some());
        let normals = editor.mesh().normals.as_ref().unwrap();
        assert_eq!(normals.len(), 4);

        // All normals should point in +Z direction (0, 0, 1) for a flat quad
        for normal in normals {
            assert!(approximately_equal(*normal, [0.0, 0.0, 1.0], 0.01));
        }
    }

    #[test]
    fn test_calculate_smooth_normals() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);
        editor.calculate_smooth_normals();

        assert!(editor.mesh().normals.is_some());
        let normals = editor.mesh().normals.as_ref().unwrap();
        assert_eq!(normals.len(), 4);
    }

    #[test]
    fn test_calculate_weighted_smooth_normals() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);
        editor.calculate_weighted_smooth_normals();

        assert!(editor.mesh().normals.is_some());
        let normals = editor.mesh().normals.as_ref().unwrap();
        assert_eq!(normals.len(), 4);
    }

    #[test]
    fn test_calculate_normals_by_mode() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);

        editor.calculate_normals(NormalMode::Flat);
        assert!(editor.mesh().normals.is_some());

        editor.calculate_normals(NormalMode::Smooth);
        assert!(editor.mesh().normals.is_some());

        editor.calculate_normals(NormalMode::WeightedSmooth);
        assert!(editor.mesh().normals.is_some());
    }

    #[test]
    fn test_normalize_all() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 4.0],
            [1.0, 1.0, 1.0],
        ]);
        let mut editor = MeshNormalEditor::new(mesh);

        editor.normalize_all();

        let normals = editor.mesh().normals.as_ref().unwrap();
        for normal in normals {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!((length - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_set_get_normal() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);

        editor.set_normal(0, [1.0, 0.0, 0.0]);
        let normal = editor.get_normal(0).unwrap();
        assert!(approximately_equal(normal, [1.0, 0.0, 0.0], 0.01));
    }

    #[test]
    fn test_flip_all_normals() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]);
        let mut editor = MeshNormalEditor::new(mesh);

        editor.flip_all_normals();

        let normals = editor.mesh().normals.as_ref().unwrap();
        for normal in normals {
            assert!(approximately_equal(*normal, [0.0, 0.0, -1.0], 0.01));
        }
    }

    #[test]
    fn test_flip_specific_normals() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]);
        let mut editor = MeshNormalEditor::new(mesh);

        editor.flip_normals(vec![0, 2]);

        let normals = editor.mesh().normals.as_ref().unwrap();
        assert!(approximately_equal(normals[0], [0.0, 0.0, -1.0], 0.01));
        assert!(approximately_equal(normals[1], [0.0, 0.0, 1.0], 0.01));
        assert!(approximately_equal(normals[2], [0.0, 0.0, -1.0], 0.01));
        assert!(approximately_equal(normals[3], [0.0, 0.0, 1.0], 0.01));
    }

    #[test]
    fn test_remove_normals() {
        let mut mesh = create_test_mesh();
        mesh.normals = Some(vec![[0.0, 1.0, 0.0]; 4]);
        let mut editor = MeshNormalEditor::new(mesh);

        assert!(editor.mesh().normals.is_some());
        editor.remove_normals();
        assert!(editor.mesh().normals.is_none());
    }

    #[test]
    fn test_auto_normalize() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);

        editor.set_auto_normalize(true);
        editor.set_normal(0, [2.0, 0.0, 0.0]);

        let normal = editor.get_normal(0).unwrap();
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        assert!((length - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_smooth_region() {
        let mesh = create_test_mesh();
        let mut editor = MeshNormalEditor::new(mesh);
        editor.calculate_flat_normals();

        // Manually perturb one normal
        editor.set_normal(0, [0.5, 0.5, 0.0]);

        // Smooth around that vertex
        editor.smooth_region(&[0], 5);

        let normal = editor.get_normal(0).unwrap();
        // After smoothing, should be more aligned with neighbors
        assert!(normal[2] > 0.1);
    }

    #[test]
    fn test_build_vertex_adjacency() {
        let mesh = create_test_mesh();
        let editor = MeshNormalEditor::new(mesh);

        let adjacency = editor.build_vertex_adjacency();

        // Vertex 0 should be adjacent to vertices 1, 2, 3
        let neighbors = adjacency.get(&0).unwrap();
        assert!(neighbors.contains(&1));
        assert!(neighbors.contains(&2));
        assert!(neighbors.contains(&3));
    }

    #[test]
    fn test_normalize_zero_vector() {
        let normalized = MeshNormalEditor::normalize([0.0, 0.0, 0.0]);
        assert!(approximately_equal(normalized, [0.0, 1.0, 0.0], 0.01));
    }

    #[test]
    fn test_triangle_normal_calculation() {
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];

        let normal = MeshNormalEditor::calculate_triangle_normal(v0, v1, v2);

        // Should point in +Z direction
        assert!(approximately_equal(normal, [0.0, 0.0, 1.0], 0.01));
    }

    #[test]
    fn test_triangle_normal_and_area() {
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];

        let (normal, area) = MeshNormalEditor::calculate_triangle_normal_and_area(v0, v1, v2);

        assert!(approximately_equal(normal, [0.0, 0.0, 1.0], 0.01));
        assert!((area - 0.5).abs() < 0.01); // Area of right triangle with legs 1,1
    }
}
