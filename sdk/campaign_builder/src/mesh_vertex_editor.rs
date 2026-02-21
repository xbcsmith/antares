// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh vertex editor for creature editor
//!
//! Provides UI and functionality for editing mesh vertices with visual selection,
//! manipulation, and real-time preview.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::MeshDefinition;
//! use campaign_builder::mesh_vertex_editor::MeshVertexEditor;
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
//! let mut editor = MeshVertexEditor::new(mesh);
//! editor.select_vertex(0);
//! editor.translate_selected([1.0, 0.0, 0.0]);
//! ```

use antares::domain::visual::MeshDefinition;
use std::collections::HashSet;

/// Vertex editor for mesh manipulation
#[derive(Debug, Clone)]
pub struct MeshVertexEditor {
    /// The mesh being edited
    mesh: MeshDefinition,

    /// Currently selected vertex indices
    selected_vertices: HashSet<usize>,

    /// Whether selection mode is active
    selection_mode: SelectionMode,

    /// Manipulation gizmo mode
    gizmo_mode: GizmoMode,

    /// Snap to grid enabled
    snap_to_grid: bool,

    /// Grid snap size
    grid_snap_size: f32,

    /// Vertex manipulation history for undo
    history: Vec<VertexOperation>,

    /// Current position in history (for undo/redo)
    history_position: usize,
}

/// Selection mode for vertex picking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Replace current selection
    Replace,

    /// Add to current selection
    Add,

    /// Remove from current selection
    Subtract,

    /// Toggle selected state
    Toggle,
}

/// Gizmo manipulation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    /// Translate vertices
    Translate,

    /// Scale vertices from selection center
    Scale,

    /// Rotate vertices around selection center
    Rotate,
}

/// Vertex operation for undo/redo
#[derive(Debug, Clone)]
pub struct VertexOperation {
    /// Description of the operation
    pub description: String,

    /// Vertex indices affected
    pub affected_vertices: Vec<usize>,

    /// Original positions before operation
    pub old_positions: Vec<[f32; 3]>,

    /// New positions after operation
    pub new_positions: Vec<[f32; 3]>,
}

impl MeshVertexEditor {
    /// Creates a new vertex editor for the given mesh
    ///
    /// # Arguments
    ///
    /// * `mesh` - The mesh to edit
    ///
    /// # Returns
    ///
    /// A new `MeshVertexEditor`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_vertex_editor::MeshVertexEditor;
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
    ///     vertices: vec![[0.0, 0.0, 0.0]],
    ///     indices: vec![],
    ///     normals: None,
    ///     uvs: None,
    ///     color: [1.0, 1.0, 1.0, 1.0],
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
    /// };
    ///
    /// let editor = MeshVertexEditor::new(mesh);
    /// assert_eq!(editor.vertex_count(), 1);
    /// ```
    pub fn new(mesh: MeshDefinition) -> Self {
        Self {
            mesh,
            selected_vertices: HashSet::new(),
            selection_mode: SelectionMode::Replace,
            gizmo_mode: GizmoMode::Translate,
            snap_to_grid: false,
            grid_snap_size: 0.1,
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

    /// Returns the number of vertices in the mesh
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    /// Returns the currently selected vertex indices
    pub fn selected_vertices(&self) -> &HashSet<usize> {
        &self.selected_vertices
    }

    /// Returns the current selection mode
    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode
    }

    /// Sets the selection mode
    pub fn set_selection_mode(&mut self, mode: SelectionMode) {
        self.selection_mode = mode;
    }

    /// Returns the current gizmo mode
    pub fn gizmo_mode(&self) -> GizmoMode {
        self.gizmo_mode
    }

    /// Sets the gizmo manipulation mode
    pub fn set_gizmo_mode(&mut self, mode: GizmoMode) {
        self.gizmo_mode = mode;
    }

    /// Returns whether snap to grid is enabled
    pub fn snap_to_grid(&self) -> bool {
        self.snap_to_grid
    }

    /// Sets whether snap to grid is enabled
    pub fn set_snap_to_grid(&mut self, enabled: bool) {
        self.snap_to_grid = enabled;
    }

    /// Returns the grid snap size
    pub fn grid_snap_size(&self) -> f32 {
        self.grid_snap_size
    }

    /// Sets the grid snap size
    pub fn set_grid_snap_size(&mut self, size: f32) {
        self.grid_snap_size = size.max(0.001);
    }

    /// Selects a vertex based on the current selection mode
    ///
    /// # Arguments
    ///
    /// * `index` - The vertex index to select
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_vertex_editor::{MeshVertexEditor, SelectionMode};
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
    ///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
    ///     indices: vec![],
    ///     normals: None,
    ///     uvs: None,
    ///     color: [1.0, 1.0, 1.0, 1.0],
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
    /// };
    ///
    /// let mut editor = MeshVertexEditor::new(mesh);
    /// editor.select_vertex(0);
    /// assert!(editor.is_vertex_selected(0));
    ///
    /// editor.set_selection_mode(SelectionMode::Add);
    /// editor.select_vertex(1);
    /// assert!(editor.is_vertex_selected(0));
    /// assert!(editor.is_vertex_selected(1));
    /// ```
    pub fn select_vertex(&mut self, index: usize) {
        if index >= self.mesh.vertices.len() {
            return;
        }

        match self.selection_mode {
            SelectionMode::Replace => {
                self.selected_vertices.clear();
                self.selected_vertices.insert(index);
            }
            SelectionMode::Add => {
                self.selected_vertices.insert(index);
            }
            SelectionMode::Subtract => {
                self.selected_vertices.remove(&index);
            }
            SelectionMode::Toggle => {
                if self.selected_vertices.contains(&index) {
                    self.selected_vertices.remove(&index);
                } else {
                    self.selected_vertices.insert(index);
                }
            }
        }
    }

    /// Selects multiple vertices
    ///
    /// # Arguments
    ///
    /// * `indices` - Iterator of vertex indices to select
    pub fn select_vertices<I>(&mut self, indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        for index in indices {
            self.select_vertex(index);
        }
    }

    /// Clears the current vertex selection
    pub fn clear_selection(&mut self) {
        self.selected_vertices.clear();
    }

    /// Selects all vertices in the mesh
    pub fn select_all(&mut self) {
        self.selected_vertices.clear();
        for i in 0..self.mesh.vertices.len() {
            self.selected_vertices.insert(i);
        }
    }

    /// Inverts the current selection
    pub fn invert_selection(&mut self) {
        let mut new_selection = HashSet::new();
        for i in 0..self.mesh.vertices.len() {
            if !self.selected_vertices.contains(&i) {
                new_selection.insert(i);
            }
        }
        self.selected_vertices = new_selection;
    }

    /// Returns whether a vertex is selected
    ///
    /// # Arguments
    ///
    /// * `index` - The vertex index to check
    ///
    /// # Returns
    ///
    /// `true` if the vertex is selected, `false` otherwise
    pub fn is_vertex_selected(&self, index: usize) -> bool {
        self.selected_vertices.contains(&index)
    }

    /// Translates selected vertices by the given offset
    ///
    /// # Arguments
    ///
    /// * `offset` - The translation offset [x, y, z]
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_vertex_editor::MeshVertexEditor;
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
    ///     vertices: vec![[0.0, 0.0, 0.0]],
    ///     indices: vec![],
    ///     normals: None,
    ///     uvs: None,
    ///     color: [1.0, 1.0, 1.0, 1.0],
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
    /// };
    ///
    /// let mut editor = MeshVertexEditor::new(mesh);
    /// editor.select_vertex(0);
    /// editor.translate_selected([1.0, 2.0, 3.0]);
    ///
    /// let vertex = editor.mesh().vertices[0];
    /// assert_eq!(vertex, [1.0, 2.0, 3.0]);
    /// ```
    pub fn translate_selected(&mut self, offset: [f32; 3]) {
        if self.selected_vertices.is_empty() {
            return;
        }

        let mut operation = VertexOperation {
            description: "Translate vertices".to_string(),
            affected_vertices: self.selected_vertices.iter().copied().collect(),
            old_positions: Vec::new(),
            new_positions: Vec::new(),
        };

        for &index in &self.selected_vertices {
            operation.old_positions.push(self.mesh.vertices[index]);

            let mut new_pos = [
                self.mesh.vertices[index][0] + offset[0],
                self.mesh.vertices[index][1] + offset[1],
                self.mesh.vertices[index][2] + offset[2],
            ];

            if self.snap_to_grid {
                new_pos = self.snap_position(new_pos);
            }

            self.mesh.vertices[index] = new_pos;
            operation.new_positions.push(new_pos);
        }

        self.add_to_history(operation);
    }

    /// Scales selected vertices from the world origin by the given factor
    ///
    /// Each selected vertex has its coordinates multiplied by the scale factor,
    /// effectively scaling from the world origin (0, 0, 0).
    ///
    /// # Arguments
    ///
    /// * `scale` - The scale factor [x, y, z]
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::MeshDefinition;
    /// use campaign_builder::mesh_vertex_editor::MeshVertexEditor;
    ///
    /// let mesh = MeshDefinition {
    ///     name: Some("test".to_string()),
    ///     vertices: vec![[1.0, 2.0, 3.0]],
    ///     indices: vec![],
    ///     normals: None,
    ///     uvs: None,
    ///     color: [1.0, 1.0, 1.0, 1.0],
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
    /// };
    ///
    /// let mut editor = MeshVertexEditor::new(mesh);
    /// editor.select_vertex(0);
    /// editor.scale_selected([2.0, 2.0, 2.0]);
    ///
    /// let vertex = editor.mesh().vertices[0];
    /// assert_eq!(vertex, [2.0, 4.0, 6.0]);
    /// ```
    pub fn scale_selected(&mut self, scale: [f32; 3]) {
        if self.selected_vertices.is_empty() {
            return;
        }

        let mut operation = VertexOperation {
            description: "Scale vertices".to_string(),
            affected_vertices: self.selected_vertices.iter().copied().collect(),
            old_positions: Vec::new(),
            new_positions: Vec::new(),
        };

        for &index in &self.selected_vertices {
            operation.old_positions.push(self.mesh.vertices[index]);

            let mut new_pos = [
                self.mesh.vertices[index][0] * scale[0],
                self.mesh.vertices[index][1] * scale[1],
                self.mesh.vertices[index][2] * scale[2],
            ];

            if self.snap_to_grid {
                new_pos = self.snap_position(new_pos);
            }

            self.mesh.vertices[index] = new_pos;
            operation.new_positions.push(new_pos);
        }

        self.add_to_history(operation);
    }

    /// Sets the absolute position of a vertex
    ///
    /// # Arguments
    ///
    /// * `index` - The vertex index
    /// * `position` - The new position [x, y, z]
    pub fn set_vertex_position(&mut self, index: usize, position: [f32; 3]) {
        if index >= self.mesh.vertices.len() {
            return;
        }

        let old_position = self.mesh.vertices[index];
        let new_position = if self.snap_to_grid {
            self.snap_position(position)
        } else {
            position
        };

        self.mesh.vertices[index] = new_position;

        let operation = VertexOperation {
            description: format!("Set vertex {} position", index),
            affected_vertices: vec![index],
            old_positions: vec![old_position],
            new_positions: vec![new_position],
        };

        self.add_to_history(operation);
    }

    /// Calculates the center point of selected vertices
    ///
    /// # Returns
    ///
    /// The center position [x, y, z], or [0, 0, 0] if no vertices are selected
    pub fn calculate_selection_center(&self) -> [f32; 3] {
        if self.selected_vertices.is_empty() {
            return [0.0, 0.0, 0.0];
        }

        let mut center = [0.0, 0.0, 0.0];
        for &index in &self.selected_vertices {
            center[0] += self.mesh.vertices[index][0];
            center[1] += self.mesh.vertices[index][1];
            center[2] += self.mesh.vertices[index][2];
        }

        let count = self.selected_vertices.len() as f32;
        center[0] /= count;
        center[1] /= count;
        center[2] /= count;

        center
    }

    /// Snaps a position to the grid
    fn snap_position(&self, position: [f32; 3]) -> [f32; 3] {
        [
            (position[0] / self.grid_snap_size).round() * self.grid_snap_size,
            (position[1] / self.grid_snap_size).round() * self.grid_snap_size,
            (position[2] / self.grid_snap_size).round() * self.grid_snap_size,
        ]
    }

    /// Adds a new vertex to the mesh
    ///
    /// # Arguments
    ///
    /// * `position` - The position of the new vertex [x, y, z]
    ///
    /// # Returns
    ///
    /// The index of the newly added vertex
    pub fn add_vertex(&mut self, position: [f32; 3]) -> usize {
        let position = if self.snap_to_grid {
            self.snap_position(position)
        } else {
            position
        };

        self.mesh.vertices.push(position);

        // Add corresponding normal if mesh has normals
        if let Some(ref mut normals) = self.mesh.normals {
            normals.push([0.0, 1.0, 0.0]); // Default up normal
        }

        // Add corresponding UV if mesh has UVs
        if let Some(ref mut uvs) = self.mesh.uvs {
            uvs.push([0.0, 0.0]); // Default UV
        }

        self.mesh.vertices.len() - 1
    }

    /// Removes selected vertices from the mesh
    ///
    /// Updates indices to maintain mesh integrity.
    pub fn delete_selected(&mut self) {
        if self.selected_vertices.is_empty() {
            return;
        }

        // Create index mapping (old -> new)
        let mut index_map: Vec<Option<usize>> = vec![Some(0); self.mesh.vertices.len()];
        let mut new_index = 0;
        for (i, slot) in index_map.iter_mut().enumerate() {
            if self.selected_vertices.contains(&i) {
                *slot = None;
            } else {
                *slot = Some(new_index);
                new_index += 1;
            }
        }

        // Remove vertices
        let mut new_vertices = Vec::new();
        for (i, vertex) in self.mesh.vertices.iter().enumerate() {
            if !self.selected_vertices.contains(&i) {
                new_vertices.push(*vertex);
            }
        }
        self.mesh.vertices = new_vertices;

        // Remove corresponding normals
        if let Some(ref mut normals) = self.mesh.normals {
            let mut new_normals = Vec::new();
            for (i, normal) in normals.iter().enumerate() {
                if !self.selected_vertices.contains(&i) {
                    new_normals.push(*normal);
                }
            }
            *normals = new_normals;
        }

        // Remove corresponding UVs
        if let Some(ref mut uvs) = self.mesh.uvs {
            let mut new_uvs = Vec::new();
            for (i, uv) in uvs.iter().enumerate() {
                if !self.selected_vertices.contains(&i) {
                    new_uvs.push(*uv);
                }
            }
            *uvs = new_uvs;
        }

        // Update indices, removing triangles that reference deleted vertices
        let mut new_indices = Vec::new();
        for chunk in self.mesh.indices.chunks(3) {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if let (Some(new_i0), Some(new_i1), Some(new_i2)) =
                (index_map[i0], index_map[i1], index_map[i2])
            {
                new_indices.push(new_i0 as u32);
                new_indices.push(new_i1 as u32);
                new_indices.push(new_i2 as u32);
            }
        }
        self.mesh.indices = new_indices;

        self.selected_vertices.clear();
    }

    /// Duplicates selected vertices
    ///
    /// # Returns
    ///
    /// Indices of the newly created vertices
    pub fn duplicate_selected(&mut self) -> Vec<usize> {
        if self.selected_vertices.is_empty() {
            return Vec::new();
        }

        let mut new_indices = Vec::new();

        for &index in &self.selected_vertices {
            let new_index = self.mesh.vertices.len();
            new_indices.push(new_index);

            self.mesh.vertices.push(self.mesh.vertices[index]);

            if let Some(ref mut normals) = self.mesh.normals {
                normals.push(normals[index]);
            }

            if let Some(ref mut uvs) = self.mesh.uvs {
                uvs.push(uvs[index]);
            }
        }

        // Select the duplicated vertices
        self.selected_vertices.clear();
        for &index in &new_indices {
            self.selected_vertices.insert(index);
        }

        new_indices
    }

    /// Merges selected vertices that are within the given distance threshold
    ///
    /// # Arguments
    ///
    /// * `threshold` - Maximum distance for vertices to be considered duplicates
    pub fn merge_selected(&mut self, threshold: f32) {
        if self.selected_vertices.len() < 2 {
            return;
        }

        let selected: Vec<usize> = self.selected_vertices.iter().copied().collect();
        let threshold_sq = threshold * threshold;

        // Find merge groups
        let mut merged: HashSet<usize> = HashSet::new();
        let mut merge_map: Vec<usize> = (0..self.mesh.vertices.len()).collect();

        for i in 0..selected.len() {
            if merged.contains(&selected[i]) {
                continue;
            }

            let pos_i = self.mesh.vertices[selected[i]];

            for j in (i + 1)..selected.len() {
                if merged.contains(&selected[j]) {
                    continue;
                }

                let pos_j = self.mesh.vertices[selected[j]];
                let dx = pos_i[0] - pos_j[0];
                let dy = pos_i[1] - pos_j[1];
                let dz = pos_i[2] - pos_j[2];
                let dist_sq = dx * dx + dy * dy + dz * dz;

                if dist_sq <= threshold_sq {
                    merged.insert(selected[j]);
                    merge_map[selected[j]] = selected[i];
                }
            }
        }

        // Update indices to point to merged vertices
        for index in self.mesh.indices.iter_mut() {
            *index = merge_map[*index as usize] as u32;
        }

        // Delete merged vertices
        self.selected_vertices = merged;
        self.delete_selected();
    }

    /// Adds an operation to the history
    fn add_to_history(&mut self, operation: VertexOperation) {
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

        // Restore old positions
        for (i, &index) in operation.affected_vertices.iter().enumerate() {
            if index < self.mesh.vertices.len() {
                self.mesh.vertices[index] = operation.old_positions[i];
            }
        }

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

        // Apply new positions
        for (i, &index) in operation.affected_vertices.iter().enumerate() {
            if index < self.mesh.vertices.len() {
                self.mesh.vertices[index] = operation.new_positions[i];
            }
        }

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
        let editor = MeshVertexEditor::new(mesh);
        assert_eq!(editor.vertex_count(), 4);
        assert_eq!(editor.selected_vertices().len(), 0);
    }

    #[test]
    fn test_select_vertex_replace() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0);
        assert!(editor.is_vertex_selected(0));
        assert_eq!(editor.selected_vertices().len(), 1);

        editor.select_vertex(1);
        assert!(!editor.is_vertex_selected(0));
        assert!(editor.is_vertex_selected(1));
        assert_eq!(editor.selected_vertices().len(), 1);
    }

    #[test]
    fn test_select_vertex_add() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);
        editor.set_selection_mode(SelectionMode::Add);

        editor.select_vertex(0);
        editor.select_vertex(1);
        assert!(editor.is_vertex_selected(0));
        assert!(editor.is_vertex_selected(1));
        assert_eq!(editor.selected_vertices().len(), 2);
    }

    #[test]
    fn test_select_vertex_subtract() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_all();
        assert_eq!(editor.selected_vertices().len(), 4);

        editor.set_selection_mode(SelectionMode::Subtract);
        editor.select_vertex(1);
        assert_eq!(editor.selected_vertices().len(), 3);
        assert!(!editor.is_vertex_selected(1));
    }

    #[test]
    fn test_select_vertex_toggle() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);
        editor.set_selection_mode(SelectionMode::Toggle);

        editor.select_vertex(0);
        assert!(editor.is_vertex_selected(0));

        editor.select_vertex(0);
        assert!(!editor.is_vertex_selected(0));
    }

    #[test]
    fn test_clear_selection() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_all();
        assert_eq!(editor.selected_vertices().len(), 4);

        editor.clear_selection();
        assert_eq!(editor.selected_vertices().len(), 0);
    }

    #[test]
    fn test_select_all() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_all();
        assert_eq!(editor.selected_vertices().len(), 4);
    }

    #[test]
    fn test_invert_selection() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        // Select vertex 0, then switch to Add mode to also select vertex 1
        editor.select_vertex(0);
        editor.set_selection_mode(SelectionMode::Add);
        editor.select_vertex(1);
        editor.invert_selection();

        // After invert: 0 and 1 were selected, so they become unselected
        // 2 and 3 were not selected, so they become selected
        assert!(!editor.is_vertex_selected(0));
        assert!(!editor.is_vertex_selected(1));
        assert!(editor.is_vertex_selected(2));
        assert!(editor.is_vertex_selected(3));
    }

    #[test]
    fn test_translate_selected() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0);
        editor.translate_selected([1.0, 2.0, 3.0]);

        let vertex = editor.mesh().vertices[0];
        assert_eq!(vertex, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_scale_selected() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(1);
        editor.scale_selected([2.0, 2.0, 2.0]);

        let vertex = editor.mesh().vertices[1];
        assert_eq!(vertex, [2.0, 0.0, 0.0]);
    }

    #[test]
    fn test_set_vertex_position() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.set_vertex_position(0, [5.0, 6.0, 7.0]);
        let vertex = editor.mesh().vertices[0];
        assert_eq!(vertex, [5.0, 6.0, 7.0]);
    }

    #[test]
    fn test_calculate_selection_center() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0); // [0, 0, 0]
        editor.set_selection_mode(SelectionMode::Add);
        editor.select_vertex(2); // [1, 1, 0]

        let center = editor.calculate_selection_center();
        assert_eq!(center, [0.5, 0.5, 0.0]);
    }

    #[test]
    fn test_snap_to_grid() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.set_snap_to_grid(true);
        editor.set_grid_snap_size(0.5);
        editor.select_vertex(0);
        editor.translate_selected([0.23, 0.78, 1.49]);

        let vertex = editor.mesh().vertices[0];
        // Should snap to nearest 0.5
        assert!((vertex[0] - 0.0).abs() < 0.01);
        assert!((vertex[1] - 1.0).abs() < 0.01);
        assert!((vertex[2] - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_add_vertex() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        let index = editor.add_vertex([5.0, 5.0, 5.0]);
        assert_eq!(index, 4);
        assert_eq!(editor.vertex_count(), 5);
        assert_eq!(editor.mesh().vertices[4], [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_delete_selected() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0);
        editor.delete_selected();

        assert_eq!(editor.vertex_count(), 3);
        assert_eq!(editor.selected_vertices().len(), 0);
    }

    #[test]
    fn test_duplicate_selected() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0);
        let new_indices = editor.duplicate_selected();

        assert_eq!(new_indices.len(), 1);
        assert_eq!(editor.vertex_count(), 5);
        assert_eq!(editor.mesh().vertices[4], editor.mesh().vertices[0]);
    }

    #[test]
    fn test_merge_selected() {
        let mut mesh = create_test_mesh();
        mesh.vertices.push([0.01, 0.01, 0.01]); // Very close to vertex 0
        let mut editor = MeshVertexEditor::new(mesh);

        editor.select_vertex(0);
        editor.set_selection_mode(SelectionMode::Add);
        editor.select_vertex(4);
        editor.merge_selected(0.1);

        assert_eq!(editor.vertex_count(), 4);
    }

    #[test]
    fn test_undo_redo() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        let original_pos = editor.mesh().vertices[0];

        editor.select_vertex(0);
        editor.translate_selected([1.0, 0.0, 0.0]);

        assert_ne!(editor.mesh().vertices[0], original_pos);
        assert!(editor.can_undo());

        editor.undo();
        assert_eq!(editor.mesh().vertices[0], original_pos);
        assert!(editor.can_redo());

        editor.redo();
        assert_ne!(editor.mesh().vertices[0], original_pos);
    }

    #[test]
    fn test_gizmo_modes() {
        let mesh = create_test_mesh();
        let mut editor = MeshVertexEditor::new(mesh);

        assert_eq!(editor.gizmo_mode(), GizmoMode::Translate);

        editor.set_gizmo_mode(GizmoMode::Scale);
        assert_eq!(editor.gizmo_mode(), GizmoMode::Scale);

        editor.set_gizmo_mode(GizmoMode::Rotate);
        assert_eq!(editor.gizmo_mode(), GizmoMode::Rotate);
    }
}
