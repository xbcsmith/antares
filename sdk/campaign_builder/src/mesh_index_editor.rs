// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh index editor for creature editor
//!
//! Provides UI and functionality for editing mesh triangle indices, allowing
//! manipulation of mesh topology and face connectivity.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::MeshDefinition;
//! use campaign_builder::mesh_index_editor::MeshIndexEditor;
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
//! let mut editor = MeshIndexEditor::new(mesh);
//! assert_eq!(editor.triangle_count(), 1);
//! ```

use antares::domain::visual::MeshDefinition;
use std::collections::HashSet;

/// Index editor for mesh triangle manipulation
#[derive(Debug, Clone)]
pub struct MeshIndexEditor {
    /// The mesh being edited
    mesh: MeshDefinition,

    /// Currently selected triangle indices
    selected_triangles: HashSet<usize>,

    /// Operation history for undo
    history: Vec<IndexOperation>,

    /// Current position in history (for undo/redo)
    history_position: usize,
}

/// Index operation for undo/redo
#[derive(Debug, Clone)]
pub struct IndexOperation {
    /// Description of the operation
    pub description: String,

    /// Old indices before operation
    pub old_indices: Vec<u32>,

    /// New indices after operation
    pub new_indices: Vec<u32>,
}

/// Triangle representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle {
    /// Indices into the vertex array
    pub indices: [u32; 3],
}

impl Triangle {
    /// Creates a new triangle from three vertex indices
    pub fn new(i0: u32, i1: u32, i2: u32) -> Self {
        Self {
            indices: [i0, i1, i2],
        }
    }

    /// Returns the vertex indices
    pub fn vertices(&self) -> [u32; 3] {
        self.indices
    }

    /// Reverses the winding order of the triangle
    pub fn flip(&mut self) {
        self.indices.swap(1, 2);
    }

    /// Returns a new triangle with reversed winding order
    pub fn flipped(&self) -> Self {
        Self {
            indices: [self.indices[0], self.indices[2], self.indices[1]],
        }
    }
}

impl MeshIndexEditor {
    /// Creates a new index editor for the given mesh
    ///
    /// # Arguments
    ///
    /// * `mesh` - The mesh to edit
    ///
    /// # Returns
    ///
    /// A new `MeshIndexEditor`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_index_editor::MeshIndexEditor;
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
    /// let editor = MeshIndexEditor::new(mesh);
    /// assert_eq!(editor.triangle_count(), 1);
    /// ```
    pub fn new(mesh: MeshDefinition) -> Self {
        Self {
            mesh,
            selected_triangles: HashSet::new(),
            history: Vec::new(),
            history_position: 0,
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

    /// Returns the number of triangles in the mesh
    pub fn triangle_count(&self) -> usize {
        self.mesh.indices.len() / 3
    }

    /// Returns the currently selected triangle indices
    pub fn selected_triangles(&self) -> &HashSet<usize> {
        &self.selected_triangles
    }

    /// Selects a triangle
    ///
    /// # Arguments
    ///
    /// * `triangle_idx` - The triangle index to select
    pub fn select_triangle(&mut self, triangle_idx: usize) {
        if triangle_idx < self.triangle_count() {
            self.selected_triangles.insert(triangle_idx);
        }
    }

    /// Deselects a triangle
    ///
    /// # Arguments
    ///
    /// * `triangle_idx` - The triangle index to deselect
    pub fn deselect_triangle(&mut self, triangle_idx: usize) {
        self.selected_triangles.remove(&triangle_idx);
    }

    /// Clears the current triangle selection
    pub fn clear_selection(&mut self) {
        self.selected_triangles.clear();
    }

    /// Selects all triangles
    pub fn select_all(&mut self) {
        self.selected_triangles.clear();
        for i in 0..self.triangle_count() {
            self.selected_triangles.insert(i);
        }
    }

    /// Returns whether a triangle is selected
    ///
    /// # Arguments
    ///
    /// * `triangle_idx` - The triangle index to check
    ///
    /// # Returns
    ///
    /// `true` if the triangle is selected, `false` otherwise
    pub fn is_triangle_selected(&self, triangle_idx: usize) -> bool {
        self.selected_triangles.contains(&triangle_idx)
    }

    /// Gets a triangle by index
    ///
    /// # Arguments
    ///
    /// * `triangle_idx` - The triangle index
    ///
    /// # Returns
    ///
    /// The triangle, or `None` if index is out of bounds
    pub fn get_triangle(&self, triangle_idx: usize) -> Option<Triangle> {
        if triangle_idx >= self.triangle_count() {
            return None;
        }

        let base = triangle_idx * 3;
        Some(Triangle::new(
            self.mesh.indices[base],
            self.mesh.indices[base + 1],
            self.mesh.indices[base + 2],
        ))
    }

    /// Sets a triangle's indices
    ///
    /// # Arguments
    ///
    /// * `triangle_idx` - The triangle index
    /// * `triangle` - The new triangle
    ///
    /// # Returns
    ///
    /// `true` if successful, `false` if index out of bounds
    pub fn set_triangle(&mut self, triangle_idx: usize, triangle: Triangle) -> bool {
        if triangle_idx >= self.triangle_count() {
            return false;
        }

        let old_indices = self.mesh.indices.clone();

        let base = triangle_idx * 3;
        self.mesh.indices[base] = triangle.indices[0];
        self.mesh.indices[base + 1] = triangle.indices[1];
        self.mesh.indices[base + 2] = triangle.indices[2];

        let operation = IndexOperation {
            description: format!("Modify triangle {}", triangle_idx),
            old_indices,
            new_indices: self.mesh.indices.clone(),
        };
        self.add_to_history(operation);

        true
    }

    /// Adds a new triangle to the mesh
    ///
    /// # Arguments
    ///
    /// * `triangle` - The triangle to add
    ///
    /// # Returns
    ///
    /// The index of the newly added triangle
    pub fn add_triangle(&mut self, triangle: Triangle) -> usize {
        let old_indices = self.mesh.indices.clone();

        self.mesh.indices.push(triangle.indices[0]);
        self.mesh.indices.push(triangle.indices[1]);
        self.mesh.indices.push(triangle.indices[2]);

        let operation = IndexOperation {
            description: "Add triangle".to_string(),
            old_indices,
            new_indices: self.mesh.indices.clone(),
        };
        self.add_to_history(operation);

        self.triangle_count() - 1
    }

    /// Deletes selected triangles
    pub fn delete_selected(&mut self) {
        if self.selected_triangles.is_empty() {
            return;
        }

        let old_indices = self.mesh.indices.clone();

        let mut new_indices = Vec::new();
        for triangle_idx in 0..self.triangle_count() {
            if !self.selected_triangles.contains(&triangle_idx) {
                let base = triangle_idx * 3;
                new_indices.push(old_indices[base]);
                new_indices.push(old_indices[base + 1]);
                new_indices.push(old_indices[base + 2]);
            }
        }

        self.mesh.indices = new_indices;

        let operation = IndexOperation {
            description: "Delete triangles".to_string(),
            old_indices,
            new_indices: self.mesh.indices.clone(),
        };
        self.add_to_history(operation);

        self.selected_triangles.clear();
    }

    /// Flips the winding order of selected triangles
    pub fn flip_selected(&mut self) {
        if self.selected_triangles.is_empty() {
            return;
        }

        let old_indices = self.mesh.indices.clone();

        for &triangle_idx in &self.selected_triangles {
            let base = triangle_idx * 3;
            self.mesh.indices.swap(base + 1, base + 2);
        }

        let operation = IndexOperation {
            description: "Flip triangles".to_string(),
            old_indices,
            new_indices: self.mesh.indices.clone(),
        };
        self.add_to_history(operation);
    }

    /// Flips all triangle winding orders in the mesh
    pub fn flip_all(&mut self) {
        self.select_all();
        self.flip_selected();
    }

    /// Removes degenerate triangles (triangles with duplicate vertices)
    ///
    /// # Returns
    ///
    /// Number of degenerate triangles removed
    pub fn remove_degenerate_triangles(&mut self) -> usize {
        let old_indices = self.mesh.indices.clone();
        let mut removed = 0;

        let mut new_indices = Vec::new();
        for triangle_idx in 0..self.triangle_count() {
            let base = triangle_idx * 3;
            let i0 = old_indices[base];
            let i1 = old_indices[base + 1];
            let i2 = old_indices[base + 2];

            // Keep triangle only if all vertices are different
            if i0 != i1 && i1 != i2 && i2 != i0 {
                new_indices.push(i0);
                new_indices.push(i1);
                new_indices.push(i2);
            } else {
                removed += 1;
            }
        }

        if removed > 0 {
            self.mesh.indices = new_indices;

            let operation = IndexOperation {
                description: format!("Remove {} degenerate triangles", removed),
                old_indices,
                new_indices: self.mesh.indices.clone(),
            };
            self.add_to_history(operation);
        }

        removed
    }

    /// Validates that all indices reference valid vertices
    ///
    /// # Returns
    ///
    /// List of invalid index positions
    pub fn validate_indices(&self) -> Vec<usize> {
        let max_vertex_index = self.mesh.vertices.len();
        let mut invalid = Vec::new();

        for (i, &index) in self.mesh.indices.iter().enumerate() {
            if index as usize >= max_vertex_index {
                invalid.push(i);
            }
        }

        invalid
    }

    /// Finds triangles that use a specific vertex
    ///
    /// # Arguments
    ///
    /// * `vertex_idx` - The vertex index to search for
    ///
    /// # Returns
    ///
    /// List of triangle indices that reference the vertex
    pub fn find_triangles_using_vertex(&self, vertex_idx: u32) -> Vec<usize> {
        let mut triangles = Vec::new();

        for triangle_idx in 0..self.triangle_count() {
            let base = triangle_idx * 3;
            if self.mesh.indices[base] == vertex_idx
                || self.mesh.indices[base + 1] == vertex_idx
                || self.mesh.indices[base + 2] == vertex_idx
            {
                triangles.push(triangle_idx);
            }
        }

        triangles
    }

    /// Finds edges shared between triangles
    ///
    /// # Returns
    ///
    /// List of (triangle_a, triangle_b) pairs that share an edge
    pub fn find_adjacent_triangles(&self) -> Vec<(usize, usize)> {
        use std::collections::HashMap;

        // Map edges to triangles that use them
        let mut edge_map: HashMap<(u32, u32), Vec<usize>> = HashMap::new();

        for triangle_idx in 0..self.triangle_count() {
            let triangle = self.get_triangle(triangle_idx).unwrap();
            let v = triangle.vertices();

            // Add edges (normalized so smaller index comes first)
            for &(a, b) in &[(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                let edge = if a < b { (a, b) } else { (b, a) };
                edge_map.entry(edge).or_default().push(triangle_idx);
            }
        }

        // Find adjacent pairs
        let mut adjacent = Vec::new();
        for triangles in edge_map.values() {
            if triangles.len() >= 2 {
                for i in 0..triangles.len() {
                    for j in (i + 1)..triangles.len() {
                        adjacent.push((triangles[i], triangles[j]));
                    }
                }
            }
        }

        adjacent
    }

    /// Selects triangles connected to currently selected triangles
    ///
    /// # Arguments
    ///
    /// * `depth` - How many levels of adjacency to select (1 = direct neighbors)
    pub fn grow_selection(&mut self, depth: usize) {
        if self.selected_triangles.is_empty() || depth == 0 {
            return;
        }

        let adjacent = self.find_adjacent_triangles();
        let mut current_selection: HashSet<usize> = self.selected_triangles.clone();

        for _ in 0..depth {
            let mut new_selection = current_selection.clone();

            for &(a, b) in &adjacent {
                if current_selection.contains(&a) {
                    new_selection.insert(b);
                }
                if current_selection.contains(&b) {
                    new_selection.insert(a);
                }
            }

            current_selection = new_selection;
        }

        self.selected_triangles = current_selection;
    }

    /// Adds an operation to the history
    fn add_to_history(&mut self, operation: IndexOperation) {
        // Remove any operations after current position
        self.history.truncate(self.history_position);

        // Add new operation
        self.history.push(operation);
        self.history_position = self.history.len();

        // Limit history size
        const MAX_HISTORY: usize = 100;
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
            self.history_position -= 1;
        }
    }

    /// Undoes the last operation
    ///
    /// # Returns
    ///
    /// `true` if an operation was undone, `false` if there's nothing to undo
    pub fn undo(&mut self) -> bool {
        if self.history_position == 0 {
            return false;
        }

        self.history_position -= 1;
        let operation = &self.history[self.history_position];
        self.mesh.indices = operation.old_indices.clone();

        true
    }

    /// Redoes the last undone operation
    ///
    /// # Returns
    ///
    /// `true` if an operation was redone, `false` if there's nothing to redo
    pub fn redo(&mut self) -> bool {
        if self.history_position >= self.history.len() {
            return false;
        }

        let operation = &self.history[self.history_position];
        self.mesh.indices = operation.new_indices.clone();
        self.history_position += 1;

        true
    }

    /// Returns whether undo is available
    pub fn can_undo(&self) -> bool {
        self.history_position > 0
    }

    /// Returns whether redo is available
    pub fn can_redo(&self) -> bool {
        self.history_position < self.history.len()
    }

    /// Clears the operation history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_position = 0;
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

    #[test]
    fn test_new_editor() {
        let mesh = create_test_mesh();
        let editor = MeshIndexEditor::new(mesh);
        assert_eq!(editor.triangle_count(), 2);
        assert_eq!(editor.selected_triangles().len(), 0);
    }

    #[test]
    fn test_select_triangle() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_triangle(0);
        assert!(editor.is_triangle_selected(0));
        assert_eq!(editor.selected_triangles().len(), 1);
    }

    #[test]
    fn test_deselect_triangle() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_triangle(0);
        editor.deselect_triangle(0);
        assert!(!editor.is_triangle_selected(0));
        assert_eq!(editor.selected_triangles().len(), 0);
    }

    #[test]
    fn test_clear_selection() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_all();
        assert_eq!(editor.selected_triangles().len(), 2);

        editor.clear_selection();
        assert_eq!(editor.selected_triangles().len(), 0);
    }

    #[test]
    fn test_select_all() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_all();
        assert_eq!(editor.selected_triangles().len(), 2);
    }

    #[test]
    fn test_get_triangle() {
        let mesh = create_test_mesh();
        let editor = MeshIndexEditor::new(mesh);

        let triangle = editor.get_triangle(0).unwrap();
        assert_eq!(triangle.vertices(), [0, 1, 2]);

        let triangle = editor.get_triangle(1).unwrap();
        assert_eq!(triangle.vertices(), [0, 2, 3]);
    }

    #[test]
    fn test_set_triangle() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        let new_triangle = Triangle::new(1, 2, 3);
        assert!(editor.set_triangle(0, new_triangle));

        let triangle = editor.get_triangle(0).unwrap();
        assert_eq!(triangle.vertices(), [1, 2, 3]);
    }

    #[test]
    fn test_add_triangle() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        let triangle = Triangle::new(0, 1, 3);
        let idx = editor.add_triangle(triangle);

        assert_eq!(idx, 2);
        assert_eq!(editor.triangle_count(), 3);
    }

    #[test]
    fn test_delete_selected() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_triangle(0);
        editor.delete_selected();

        assert_eq!(editor.triangle_count(), 1);
        assert_eq!(editor.selected_triangles().len(), 0);
    }

    #[test]
    fn test_flip_triangle() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        let original = editor.get_triangle(0).unwrap();
        assert_eq!(original.vertices(), [0, 1, 2]);

        editor.select_triangle(0);
        editor.flip_selected();

        let flipped = editor.get_triangle(0).unwrap();
        assert_eq!(flipped.vertices(), [0, 2, 1]);
    }

    #[test]
    fn test_flip_all() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.flip_all();

        let t0 = editor.get_triangle(0).unwrap();
        let t1 = editor.get_triangle(1).unwrap();
        assert_eq!(t0.vertices(), [0, 2, 1]);
        assert_eq!(t1.vertices(), [0, 3, 2]);
    }

    #[test]
    fn test_remove_degenerate_triangles() {
        let mut mesh = create_test_mesh();
        mesh.indices.extend_from_slice(&[0, 0, 1]); // Degenerate
        let mut editor = MeshIndexEditor::new(mesh);

        assert_eq!(editor.triangle_count(), 3);

        let removed = editor.remove_degenerate_triangles();
        assert_eq!(removed, 1);
        assert_eq!(editor.triangle_count(), 2);
    }

    #[test]
    fn test_validate_indices() {
        let mut mesh = create_test_mesh();
        mesh.indices.push(99); // Invalid index
        mesh.indices.push(100);
        mesh.indices.push(101);
        let editor = MeshIndexEditor::new(mesh);

        let invalid = editor.validate_indices();
        assert_eq!(invalid.len(), 3);
    }

    #[test]
    fn test_find_triangles_using_vertex() {
        let mesh = create_test_mesh();
        let editor = MeshIndexEditor::new(mesh);

        let triangles = editor.find_triangles_using_vertex(0);
        assert_eq!(triangles.len(), 2); // Vertex 0 is used by both triangles

        let triangles = editor.find_triangles_using_vertex(1);
        assert_eq!(triangles.len(), 1); // Vertex 1 is used by first triangle
    }

    #[test]
    fn test_find_adjacent_triangles() {
        let mesh = create_test_mesh();
        let editor = MeshIndexEditor::new(mesh);

        let adjacent = editor.find_adjacent_triangles();
        assert!(!adjacent.is_empty());
        assert!(adjacent.contains(&(0, 1)) || adjacent.contains(&(1, 0)));
    }

    #[test]
    fn test_grow_selection() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        editor.select_triangle(0);
        editor.grow_selection(1);

        // After growing, both triangles should be selected (they're adjacent)
        assert_eq!(editor.selected_triangles().len(), 2);
    }

    #[test]
    fn test_triangle_flip() {
        let mut triangle = Triangle::new(0, 1, 2);
        triangle.flip();
        assert_eq!(triangle.vertices(), [0, 2, 1]);

        let triangle = Triangle::new(0, 1, 2);
        let flipped = triangle.flipped();
        assert_eq!(flipped.vertices(), [0, 2, 1]);
        assert_eq!(triangle.vertices(), [0, 1, 2]); // Original unchanged
    }

    #[test]
    fn test_undo_redo() {
        let mesh = create_test_mesh();
        let mut editor = MeshIndexEditor::new(mesh);

        let original_count = editor.triangle_count();

        let triangle = Triangle::new(1, 2, 3);
        editor.add_triangle(triangle);
        assert_eq!(editor.triangle_count(), original_count + 1);
        assert!(editor.can_undo());

        editor.undo();
        assert_eq!(editor.triangle_count(), original_count);
        assert!(editor.can_redo());

        editor.redo();
        assert_eq!(editor.triangle_count(), original_count + 1);
    }
}
