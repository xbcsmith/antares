// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature Editing Undo/Redo Commands - Phase 5.5
//!
//! Command pattern implementation for reversible creature editing operations.
//! Supports undo/redo for:
//! - Adding/removing meshes
//! - Modifying mesh transforms
//! - Editing mesh geometry (vertices, indices, normals)
//! - Modifying creature properties
//! - Reordering meshes (move up/down)
//! - Template applications

use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
use serde::{Deserialize, Serialize};

/// Command trait for reversible creature editing operations
pub trait CreatureCommand: std::fmt::Debug {
    /// Execute the command on a creature
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String>;

    /// Undo the command on a creature
    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String>;

    /// Get a human-readable description of this command
    fn description(&self) -> String;
}

/// Add a mesh to a creature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMeshCommand {
    mesh: MeshDefinition,
    transform: MeshTransform,
}

impl AddMeshCommand {
    /// Create a new add mesh command
    pub fn new(mesh: MeshDefinition, transform: MeshTransform) -> Self {
        Self { mesh, transform }
    }
}

impl CreatureCommand for AddMeshCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        creature.meshes.push(self.mesh.clone());
        creature.mesh_transforms.push(self.transform);
        Ok(())
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if creature.meshes.is_empty() {
            return Err("No meshes to remove".to_string());
        }
        creature.meshes.pop();
        creature.mesh_transforms.pop();
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Add mesh '{}'",
            self.mesh.name.as_deref().unwrap_or("unnamed")
        )
    }
}

/// Remove a mesh from a creature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveMeshCommand {
    index: usize,
    mesh: MeshDefinition,
    transform: MeshTransform,
}

impl RemoveMeshCommand {
    /// Create a new remove mesh command
    pub fn new(index: usize, mesh: MeshDefinition, transform: MeshTransform) -> Self {
        Self {
            index,
            mesh,
            transform,
        }
    }
}

impl CreatureCommand for RemoveMeshCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index >= creature.meshes.len() {
            return Err(format!("Invalid mesh index: {}", self.index));
        }
        creature.meshes.remove(self.index);
        creature.mesh_transforms.remove(self.index);
        Ok(())
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index > creature.meshes.len() {
            return Err(format!("Invalid mesh index: {}", self.index));
        }
        creature.meshes.insert(self.index, self.mesh.clone());
        creature.mesh_transforms.insert(self.index, self.transform);
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Remove mesh '{}'",
            self.mesh.name.as_deref().unwrap_or("unnamed")
        )
    }
}

/// Modify a mesh transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyTransformCommand {
    index: usize,
    old_transform: MeshTransform,
    new_transform: MeshTransform,
}

impl ModifyTransformCommand {
    /// Create a new modify transform command
    pub fn new(index: usize, old_transform: MeshTransform, new_transform: MeshTransform) -> Self {
        Self {
            index,
            old_transform,
            new_transform,
        }
    }
}

impl CreatureCommand for ModifyTransformCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index >= creature.mesh_transforms.len() {
            return Err(format!("Invalid transform index: {}", self.index));
        }
        creature.mesh_transforms[self.index] = self.new_transform;
        Ok(())
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index >= creature.mesh_transforms.len() {
            return Err(format!("Invalid transform index: {}", self.index));
        }
        creature.mesh_transforms[self.index] = self.old_transform;
        Ok(())
    }

    fn description(&self) -> String {
        format!("Modify transform for mesh {}", self.index)
    }
}

/// Modify a mesh geometry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyMeshCommand {
    index: usize,
    old_mesh: MeshDefinition,
    new_mesh: MeshDefinition,
}

impl ModifyMeshCommand {
    /// Create a new modify mesh command
    pub fn new(index: usize, old_mesh: MeshDefinition, new_mesh: MeshDefinition) -> Self {
        Self {
            index,
            old_mesh,
            new_mesh,
        }
    }
}

impl CreatureCommand for ModifyMeshCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index >= creature.meshes.len() {
            return Err(format!("Invalid mesh index: {}", self.index));
        }
        creature.meshes[self.index] = self.new_mesh.clone();
        Ok(())
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        if self.index >= creature.meshes.len() {
            return Err(format!("Invalid mesh index: {}", self.index));
        }
        creature.meshes[self.index] = self.old_mesh.clone();
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Modify mesh '{}'",
            self.new_mesh.name.as_deref().unwrap_or("unnamed")
        )
    }
}

/// Modify creature properties (name, description, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyCreaturePropertiesCommand {
    old_name: String,
    new_name: String,
}

impl ModifyCreaturePropertiesCommand {
    /// Create a new modify creature properties command
    pub fn new(old_name: String, new_name: String) -> Self {
        Self { old_name, new_name }
    }
}

impl CreatureCommand for ModifyCreaturePropertiesCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        creature.name = self.new_name.clone();
        Ok(())
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        creature.name = self.old_name.clone();
        Ok(())
    }

    fn description(&self) -> String {
        format!("Rename creature '{}' to '{}'", self.old_name, self.new_name)
    }
}

/// Reorder a mesh within the creature's mesh list (move up or down).
///
/// Swaps the mesh at `index` with its neighbour at `index + offset`, where
/// `offset` is `+1` (move down) or `-1` (move up).  Both the `meshes` and
/// `mesh_transforms` slices are kept in sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderMeshCommand {
    /// Index of the mesh to move.
    index: usize,
    /// Target index after the move (index ± 1).
    target: usize,
}

impl ReorderMeshCommand {
    /// Create a command that moves the mesh at `index` to `target`.
    ///
    /// # Errors
    ///
    /// [`CreatureCommand::execute`] / [`CreatureCommand::undo`] will return
    /// `Err` if either index is out of bounds at execution time.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creature_undo_redo::ReorderMeshCommand;
    ///
    /// // Move mesh 2 up one position (to index 1)
    /// let cmd = ReorderMeshCommand::move_up(2);
    /// assert_eq!(cmd.index(), 2);
    /// assert_eq!(cmd.target(), 1);
    /// ```
    pub fn new(index: usize, target: usize) -> Self {
        Self { index, target }
    }

    /// Create a command that moves the mesh at `index` one position toward the
    /// front of the list (index − 1).
    pub fn move_up(index: usize) -> Self {
        Self {
            index,
            target: index.saturating_sub(1),
        }
    }

    /// Create a command that moves the mesh at `index` one position toward the
    /// end of the list (index + 1).
    pub fn move_down(index: usize) -> Self {
        Self {
            index,
            target: index + 1,
        }
    }

    /// Returns the source index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the destination index.
    pub fn target(&self) -> usize {
        self.target
    }

    /// Swap two positions in both parallel slices.
    fn swap(creature: &mut CreatureDefinition, a: usize, b: usize) -> Result<(), String> {
        let len = creature.meshes.len();
        if a >= len || b >= len {
            return Err(format!(
                "ReorderMesh: index out of bounds (a={}, b={}, len={})",
                a, b, len
            ));
        }
        creature.meshes.swap(a, b);
        creature.mesh_transforms.swap(a, b);
        Ok(())
    }
}

impl CreatureCommand for ReorderMeshCommand {
    fn execute(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        Self::swap(creature, self.index, self.target)
    }

    fn undo(&self, creature: &mut CreatureDefinition) -> Result<(), String> {
        // Swapping is its own inverse.
        Self::swap(creature, self.target, self.index)
    }

    fn description(&self) -> String {
        if self.target < self.index {
            format!("Move mesh {} up", self.index)
        } else {
            format!("Move mesh {} down", self.index)
        }
    }
}

/// Creature undo/redo manager
#[derive(Debug, Default)]
pub struct CreatureUndoRedoManager {
    undo_stack: Vec<Box<dyn CreatureCommand>>,
    redo_stack: Vec<Box<dyn CreatureCommand>>,
    max_history: usize,
}

impl CreatureUndoRedoManager {
    /// Maximum default history size
    pub const DEFAULT_MAX_HISTORY: usize = 50;

    /// Create a new creature undo/redo manager
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: Self::DEFAULT_MAX_HISTORY,
        }
    }

    /// Create with custom max history size
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Execute a command and add it to the undo stack
    pub fn execute(
        &mut self,
        command: Box<dyn CreatureCommand>,
        creature: &mut CreatureDefinition,
    ) -> Result<(), String> {
        command.execute(creature)?;

        // Clear redo stack when new command is executed
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(command);

        // Limit stack size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    /// Undo the last command
    pub fn undo(&mut self, creature: &mut CreatureDefinition) -> Result<String, String> {
        if let Some(command) = self.undo_stack.pop() {
            let description = command.description();
            command.undo(creature)?;
            self.redo_stack.push(command);
            Ok(description)
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self, creature: &mut CreatureDefinition) -> Result<String, String> {
        if let Some(command) = self.redo_stack.pop() {
            let description = command.description();
            command.execute(creature)?;
            self.undo_stack.push(command);
            Ok(description)
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of actions in the undo stack
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of actions in the redo stack
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get the description of the next undo action
    pub fn next_undo_description(&self) -> Option<String> {
        self.undo_stack.last().map(|cmd| cmd.description())
    }

    /// Get the description of the next redo action
    pub fn next_redo_description(&self) -> Option<String> {
        self.redo_stack.last().map(|cmd| cmd.description())
    }

    /// Clear all undo/redo history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get all undo descriptions (newest first)
    pub fn undo_descriptions(&self) -> Vec<String> {
        self.undo_stack
            .iter()
            .rev()
            .map(|cmd| cmd.description())
            .collect()
    }

    /// Get all redo descriptions (newest first)
    pub fn redo_descriptions(&self) -> Vec<String> {
        self.redo_stack
            .iter()
            .rev()
            .map(|cmd| cmd.description())
            .collect()
    }
}

#[cfg(test)]
mod reorder_tests {
    use super::*;
    use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};

    fn make_creature_with_meshes(names: &[&str]) -> CreatureDefinition {
        let mut c = CreatureDefinition {
            id: 1,
            name: "Test".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };
        for name in names {
            c.meshes.push(MeshDefinition {
                name: Some(name.to_string()),
                vertices: vec![[0.0, 0.0, 0.0]],
                indices: vec![0],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            });
            c.mesh_transforms.push(MeshTransform {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            });
        }
        c
    }

    fn mesh_names(c: &CreatureDefinition) -> Vec<&str> {
        c.meshes
            .iter()
            .map(|m| m.name.as_deref().unwrap_or(""))
            .collect()
    }

    #[test]
    fn test_reorder_move_up_execute() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        let cmd = ReorderMeshCommand::move_up(1);
        cmd.execute(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["b", "a", "c"]);
    }

    #[test]
    fn test_reorder_move_up_undo() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        let cmd = ReorderMeshCommand::move_up(1);
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_reorder_move_down_execute() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        let cmd = ReorderMeshCommand::move_down(1);
        cmd.execute(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "c", "b"]);
    }

    #[test]
    fn test_reorder_move_down_undo() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        let cmd = ReorderMeshCommand::move_down(1);
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_reorder_transforms_kept_in_sync() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        creature.mesh_transforms[0].translation = [1.0, 0.0, 0.0];
        creature.mesh_transforms[1].translation = [2.0, 0.0, 0.0];
        let cmd = ReorderMeshCommand::move_down(0);
        cmd.execute(&mut creature).unwrap();
        // Mesh "b" is now first and its transform should have moved with it
        assert_eq!(mesh_names(&creature)[0], "b");
        assert_eq!(creature.mesh_transforms[0].translation, [2.0, 0.0, 0.0]);
    }

    #[test]
    fn test_reorder_out_of_bounds_returns_error() {
        let mut creature = make_creature_with_meshes(&["a", "b"]);
        let cmd = ReorderMeshCommand::new(0, 5);
        assert!(cmd.execute(&mut creature).is_err());
    }

    #[test]
    fn test_reorder_description_move_up() {
        let cmd = ReorderMeshCommand::move_up(2);
        assert!(cmd.description().contains("up"));
    }

    #[test]
    fn test_reorder_description_move_down() {
        let cmd = ReorderMeshCommand::move_down(1);
        assert!(cmd.description().contains("down"));
    }

    #[test]
    fn test_reorder_via_undo_redo_manager() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);

        manager
            .execute(Box::new(ReorderMeshCommand::move_up(2)), &mut creature)
            .unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "c", "b"]);

        manager.undo(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "b", "c"]);

        manager.redo(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "c", "b"]);
    }

    #[test]
    fn test_move_up_from_first_is_noop() {
        let mut creature = make_creature_with_meshes(&["a", "b", "c"]);
        // move_up(0) saturates to index 0 — swapping with self is a no-op
        let cmd = ReorderMeshCommand::move_up(0);
        cmd.execute(&mut creature).unwrap();
        assert_eq!(mesh_names(&creature), vec!["a", "b", "c"]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::visual::MeshDefinition;

    fn create_test_creature() -> CreatureDefinition {
        CreatureDefinition {
            id: 0,
            name: "TestCreature".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    fn create_test_mesh(name: &str) -> MeshDefinition {
        MeshDefinition {
            name: Some(name.to_string()),
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

    fn create_test_transform() -> MeshTransform {
        MeshTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn test_add_mesh_command() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();

        let cmd = AddMeshCommand::new(mesh.clone(), transform);
        cmd.execute(&mut creature).unwrap();

        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.mesh_transforms.len(), 1);
        assert_eq!(creature.meshes[0].name, Some("TestMesh".to_string()));
    }

    #[test]
    fn test_add_mesh_undo() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();

        let cmd = AddMeshCommand::new(mesh.clone(), transform);
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();

        assert_eq!(creature.meshes.len(), 0);
        assert_eq!(creature.mesh_transforms.len(), 0);
    }

    #[test]
    fn test_remove_mesh_command() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        creature.meshes.push(mesh.clone());
        creature.mesh_transforms.push(transform);

        let cmd = RemoveMeshCommand::new(0, mesh.clone(), transform);
        cmd.execute(&mut creature).unwrap();

        assert_eq!(creature.meshes.len(), 0);
        assert_eq!(creature.mesh_transforms.len(), 0);
    }

    #[test]
    fn test_remove_mesh_undo() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        creature.meshes.push(mesh.clone());
        creature.mesh_transforms.push(transform);

        let cmd = RemoveMeshCommand::new(0, mesh.clone(), transform);
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();

        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.mesh_transforms.len(), 1);
        assert_eq!(creature.meshes[0].name, Some("TestMesh".to_string()));
    }

    #[test]
    fn test_modify_transform_command() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let old_transform = create_test_transform();
        let mut new_transform = create_test_transform();
        new_transform.translation = [1.0, 2.0, 3.0];

        creature.meshes.push(mesh);
        creature.mesh_transforms.push(old_transform);

        let cmd = ModifyTransformCommand::new(0, old_transform, new_transform);
        cmd.execute(&mut creature).unwrap();

        assert_eq!(creature.mesh_transforms[0].translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_modify_transform_undo() {
        let mut creature = create_test_creature();
        let mesh = create_test_mesh("TestMesh");
        let old_transform = create_test_transform();
        let mut new_transform = create_test_transform();
        new_transform.translation = [1.0, 2.0, 3.0];

        creature.meshes.push(mesh);
        creature.mesh_transforms.push(old_transform);

        let cmd = ModifyTransformCommand::new(0, old_transform, new_transform);
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();

        assert_eq!(creature.mesh_transforms[0].translation, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_modify_mesh_command() {
        let mut creature = create_test_creature();
        let old_mesh = create_test_mesh("OldMesh");
        let new_mesh = create_test_mesh("NewMesh");
        let transform = create_test_transform();

        creature.meshes.push(old_mesh.clone());
        creature.mesh_transforms.push(transform);

        let cmd = ModifyMeshCommand::new(0, old_mesh.clone(), new_mesh.clone());
        cmd.execute(&mut creature).unwrap();

        assert_eq!(creature.meshes[0].name, Some("NewMesh".to_string()));
    }

    #[test]
    fn test_modify_mesh_undo() {
        let mut creature = create_test_creature();
        let old_mesh = create_test_mesh("OldMesh");
        let new_mesh = create_test_mesh("NewMesh");
        let transform = create_test_transform();

        creature.meshes.push(old_mesh.clone());
        creature.mesh_transforms.push(transform);

        let cmd = ModifyMeshCommand::new(0, old_mesh.clone(), new_mesh.clone());
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();

        assert_eq!(creature.meshes[0].name, Some("OldMesh".to_string()));
    }

    #[test]
    fn test_modify_creature_properties_command() {
        let mut creature = create_test_creature();

        let cmd =
            ModifyCreaturePropertiesCommand::new("TestCreature".to_string(), "NewName".to_string());
        cmd.execute(&mut creature).unwrap();

        assert_eq!(creature.name, "NewName");
    }

    #[test]
    fn test_modify_creature_properties_undo() {
        let mut creature = create_test_creature();

        let cmd =
            ModifyCreaturePropertiesCommand::new("TestCreature".to_string(), "NewName".to_string());
        cmd.execute(&mut creature).unwrap();
        cmd.undo(&mut creature).unwrap();

        assert_eq!(creature.name, "TestCreature");
    }

    #[test]
    fn test_creature_undo_redo_manager() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        assert!(!manager.can_undo());
        assert!(!manager.can_redo());

        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        let cmd = Box::new(AddMeshCommand::new(mesh, transform));

        manager.execute(cmd, &mut creature).unwrap();

        assert!(manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(creature.meshes.len(), 1);
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        let cmd = Box::new(AddMeshCommand::new(mesh, transform));

        manager.execute(cmd, &mut creature).unwrap();
        assert_eq!(creature.meshes.len(), 1);

        manager.undo(&mut creature).unwrap();
        assert_eq!(creature.meshes.len(), 0);
        assert!(manager.can_redo());

        manager.redo(&mut creature).unwrap();
        assert_eq!(creature.meshes.len(), 1);
    }

    #[test]
    fn test_multiple_commands() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        // Add three meshes
        for i in 0..3 {
            let mesh = create_test_mesh(&format!("Mesh{}", i));
            let transform = create_test_transform();
            let cmd = Box::new(AddMeshCommand::new(mesh, transform));
            manager.execute(cmd, &mut creature).unwrap();
        }

        assert_eq!(creature.meshes.len(), 3);
        assert_eq!(manager.undo_count(), 3);

        // Undo all
        manager.undo(&mut creature).unwrap();
        manager.undo(&mut creature).unwrap();
        manager.undo(&mut creature).unwrap();

        assert_eq!(creature.meshes.len(), 0);
        assert_eq!(manager.redo_count(), 3);
    }

    #[test]
    fn test_new_command_clears_redo() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        let mesh1 = create_test_mesh("Mesh1");
        let transform1 = create_test_transform();
        let cmd1 = Box::new(AddMeshCommand::new(mesh1, transform1));
        manager.execute(cmd1, &mut creature).unwrap();

        manager.undo(&mut creature).unwrap();
        assert!(manager.can_redo());

        // Execute new command should clear redo stack
        let mesh2 = create_test_mesh("Mesh2");
        let transform2 = create_test_transform();
        let cmd2 = Box::new(AddMeshCommand::new(mesh2, transform2));
        manager.execute(cmd2, &mut creature).unwrap();

        assert!(!manager.can_redo());
    }

    #[test]
    fn test_max_history_size() {
        let mut manager = CreatureUndoRedoManager::with_max_history(3);
        let mut creature = create_test_creature();

        // Add 5 commands
        for i in 0..5 {
            let mesh = create_test_mesh(&format!("Mesh{}", i));
            let transform = create_test_transform();
            let cmd = Box::new(AddMeshCommand::new(mesh, transform));
            manager.execute(cmd, &mut creature).unwrap();
        }

        // Should only keep 3 most recent
        assert_eq!(manager.undo_count(), 3);
        assert_eq!(creature.meshes.len(), 5);
    }

    #[test]
    fn test_descriptions() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        let cmd = Box::new(AddMeshCommand::new(mesh, transform));

        manager.execute(cmd, &mut creature).unwrap();

        assert_eq!(
            manager.next_undo_description(),
            Some("Add mesh 'TestMesh'".to_string())
        );
        assert_eq!(manager.next_redo_description(), None);

        manager.undo(&mut creature).unwrap();

        assert_eq!(manager.next_undo_description(), None);
        assert_eq!(
            manager.next_redo_description(),
            Some("Add mesh 'TestMesh'".to_string())
        );
    }

    #[test]
    fn test_clear_history() {
        let mut manager = CreatureUndoRedoManager::new();
        let mut creature = create_test_creature();

        let mesh = create_test_mesh("TestMesh");
        let transform = create_test_transform();
        let cmd = Box::new(AddMeshCommand::new(mesh, transform));
        manager.execute(cmd, &mut creature).unwrap();

        manager.undo(&mut creature).unwrap();
        assert!(manager.can_redo());

        manager.clear();

        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 0);
    }
}
