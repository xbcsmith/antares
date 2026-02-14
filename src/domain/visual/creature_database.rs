// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature database for storing and managing visual creature definitions
//!
//! This module provides a database for storing creature visual definitions,
//! supporting loading from RON files and querying by ID or name.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::creature_database::CreatureDatabase;
//! use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
//!
//! let mut db = CreatureDatabase::new();
//!
//! let mesh = MeshDefinition {
//!     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
//!     indices: vec![0, 1, 2],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//! };
//!
//! let creature = CreatureDefinition {
//!     id: 1,
//!     name: "Test Creature".to_string(),
//!     meshes: vec![mesh],
//!     mesh_transforms: vec![MeshTransform::identity()],
//!     scale: 1.0,
//!     color_tint: None,
//! };
//!
//! db.add_creature(creature).expect("Failed to add creature");
//! assert!(db.has_creature(1));
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::types::CreatureId;
use crate::domain::visual::CreatureDefinition;

/// Errors that can occur when working with the creature database
#[derive(Error, Debug)]
pub enum CreatureDatabaseError {
    /// Failed to read file
    #[error("Failed to read file: {0}")]
    ReadError(String),

    /// Failed to parse RON data
    #[error("Failed to parse RON: {0}")]
    ParseError(String),

    /// Creature not found
    #[error("Creature not found: {0}")]
    CreatureNotFound(CreatureId),

    /// Duplicate creature ID
    #[error("Duplicate creature ID: {0}")]
    DuplicateId(CreatureId),

    /// Validation error
    #[error("Validation error for creature {0}: {1}")]
    ValidationError(CreatureId, String),
}

/// Database for storing and managing creature definitions
///
/// Provides storage, retrieval, and validation of creature visual definitions.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::creature_database::CreatureDatabase;
///
/// let db = CreatureDatabase::new();
/// assert_eq!(db.count(), 0);
/// assert!(db.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureDatabase {
    creatures: HashMap<CreatureId, CreatureDefinition>,
}

impl CreatureDatabase {
    /// Creates a new empty creature database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            creatures: HashMap::new(),
        }
    }

    /// Adds a creature to the database
    ///
    /// # Errors
    ///
    /// Returns `DuplicateId` if a creature with the same ID already exists.
    /// Returns `ValidationError` if the creature definition is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    /// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
    ///
    /// let mut db = CreatureDatabase::new();
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
    /// assert!(db.add_creature(creature).is_ok());
    /// ```
    pub fn add_creature(
        &mut self,
        creature: CreatureDefinition,
    ) -> Result<CreatureId, CreatureDatabaseError> {
        // Validate before adding
        creature
            .validate()
            .map_err(|e| CreatureDatabaseError::ValidationError(creature.id, e))?;

        if self.creatures.contains_key(&creature.id) {
            return Err(CreatureDatabaseError::DuplicateId(creature.id));
        }

        let id = creature.id;
        self.creatures.insert(id, creature);
        Ok(id)
    }

    /// Gets a creature by ID
    ///
    /// Returns `None` if the creature doesn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(db.get_creature(1).is_none());
    /// ```
    pub fn get_creature(&self, id: CreatureId) -> Option<&CreatureDefinition> {
        self.creatures.get(&id)
    }

    /// Gets a mutable reference to a creature by ID
    pub fn get_creature_mut(&mut self, id: CreatureId) -> Option<&mut CreatureDefinition> {
        self.creatures.get_mut(&id)
    }

    /// Returns an iterator over all creatures
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// let count = db.all_creatures().count();
    /// assert_eq!(count, 0);
    /// ```
    pub fn all_creatures(&self) -> impl Iterator<Item = &CreatureDefinition> {
        self.creatures.values()
    }

    /// Removes a creature from the database
    ///
    /// Returns the removed creature if it existed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    /// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
    ///
    /// let mut db = CreatureDatabase::new();
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
    /// db.add_creature(creature).unwrap();
    /// assert!(db.remove_creature(1).is_some());
    /// assert!(db.remove_creature(1).is_none());
    /// ```
    pub fn remove_creature(&mut self, id: CreatureId) -> Option<CreatureDefinition> {
        self.creatures.remove(&id)
    }

    /// Loads creatures from a RON file
    ///
    /// The file should contain a list of `CreatureDefinition` objects.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    /// use std::path::Path;
    ///
    /// let creatures = CreatureDatabase::load_from_file(Path::new("creatures.ron")).unwrap();
    /// ```
    pub fn load_from_file(path: &Path) -> Result<Vec<CreatureDefinition>, CreatureDatabaseError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| CreatureDatabaseError::ReadError(e.to_string()))?;

        Self::load_from_string(&contents)
    }

    /// Loads creatures from a RON string
    ///
    /// # Errors
    ///
    /// Returns error if the RON cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let ron_data = r#"
    /// [
    ///     (
    ///         id: 1,
    ///         name: "Test Creature",
    ///         meshes: [
    ///             (
    ///                 vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
    ///                 indices: [0, 1, 2],
    ///                 color: [1.0, 1.0, 1.0, 1.0],
    ///             ),
    ///         ],
    ///         mesh_transforms: [
    ///             (
    ///                 translation: [0.0, 0.0, 0.0],
    ///                 rotation: [0.0, 0.0, 0.0],
    ///                 scale: [1.0, 1.0, 1.0],
    ///             ),
    ///         ],
    ///         scale: 1.0,
    ///     ),
    /// ]
    /// "#;
    ///
    /// let creatures = CreatureDatabase::load_from_string(ron_data).unwrap();
    /// assert_eq!(creatures.len(), 1);
    /// ```
    pub fn load_from_string(data: &str) -> Result<Vec<CreatureDefinition>, CreatureDatabaseError> {
        ron::from_str(data).map_err(|e| CreatureDatabaseError::ParseError(e.to_string()))
    }

    /// Checks if a creature with the given ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(!db.has_creature(1));
    /// ```
    pub fn has_creature(&self, id: CreatureId) -> bool {
        self.creatures.contains_key(&id)
    }

    /// Returns the number of creatures in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert_eq!(db.count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.creatures.len()
    }

    /// Returns true if the database is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.creatures.is_empty()
    }

    /// Finds a creature by name (case-sensitive)
    ///
    /// Returns the first creature with a matching name.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(db.get_creature_by_name("Dragon").is_none());
    /// ```
    pub fn get_creature_by_name(&self, name: &str) -> Option<&CreatureDefinition> {
        self.creatures.values().find(|c| c.name == name)
    }

    /// Validates all creatures in the database
    ///
    /// # Errors
    ///
    /// Returns the first validation error encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    ///
    /// let db = CreatureDatabase::new();
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CreatureDatabaseError> {
        for creature in self.creatures.values() {
            creature
                .validate()
                .map_err(|e| CreatureDatabaseError::ValidationError(creature.id, e))?;
        }
        Ok(())
    }
}

impl Default for CreatureDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::{MeshDefinition, MeshTransform};

    fn create_test_creature(id: CreatureId) -> CreatureDefinition {
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        CreatureDefinition {
            id,
            name: format!("Test Creature {}", id),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        }
    }

    #[test]
    fn test_new_database_is_empty() {
        let db = CreatureDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_add_and_retrieve_creature() {
        let mut db = CreatureDatabase::new();
        let creature = create_test_creature(1);

        let id = db.add_creature(creature.clone()).unwrap();
        assert_eq!(id, 1);
        assert_eq!(db.count(), 1);
        assert!(!db.is_empty());

        let retrieved = db.get_creature(1).unwrap();
        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.name, "Test Creature 1");
    }

    #[test]
    fn test_duplicate_id_error() {
        let mut db = CreatureDatabase::new();
        let creature1 = create_test_creature(1);
        let creature2 = create_test_creature(1);

        db.add_creature(creature1).unwrap();
        let result = db.add_creature(creature2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureDatabaseError::DuplicateId(1)
        ));
    }

    #[test]
    fn test_get_nonexistent_creature() {
        let db = CreatureDatabase::new();
        assert!(db.get_creature(999).is_none());
    }

    #[test]
    fn test_remove_creature() {
        let mut db = CreatureDatabase::new();
        let creature = create_test_creature(1);

        db.add_creature(creature).unwrap();
        assert!(db.has_creature(1));

        let removed = db.remove_creature(1);
        assert!(removed.is_some());
        assert!(!db.has_creature(1));
        assert!(db.is_empty());
    }

    #[test]
    fn test_all_creatures() {
        let mut db = CreatureDatabase::new();
        db.add_creature(create_test_creature(1)).unwrap();
        db.add_creature(create_test_creature(2)).unwrap();
        db.add_creature(create_test_creature(3)).unwrap();

        let count = db.all_creatures().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_has_creature() {
        let mut db = CreatureDatabase::new();
        assert!(!db.has_creature(1));

        db.add_creature(create_test_creature(1)).unwrap();
        assert!(db.has_creature(1));
        assert!(!db.has_creature(2));
    }

    #[test]
    fn test_get_creature_by_name() {
        let mut db = CreatureDatabase::new();
        db.add_creature(create_test_creature(1)).unwrap();

        let found = db.get_creature_by_name("Test Creature 1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 1);

        let not_found = db.get_creature_by_name("Nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_validate_empty_database() {
        let db = CreatureDatabase::new();
        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_creatures() {
        let mut db = CreatureDatabase::new();
        db.add_creature(create_test_creature(1)).unwrap();
        db.add_creature(create_test_creature(2)).unwrap();

        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_load_from_string() {
        // First, create a creature and serialize it to see the correct format
        let mesh = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let creature = CreatureDefinition {
            id: 1,
            name: "Test Creature".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };

        let creatures_vec = vec![creature];
        let serialized = ron::to_string(&creatures_vec).expect("Failed to serialize");

        // Now deserialize it back
        let creatures = CreatureDatabase::load_from_string(&serialized).unwrap();
        assert_eq!(creatures.len(), 1);
        assert_eq!(creatures[0].id, 1);
        assert_eq!(creatures[0].name, "Test Creature");
    }

    #[test]
    fn test_load_from_string_invalid_ron() {
        let invalid_ron = "this is not valid RON";
        let result = CreatureDatabase::load_from_string(invalid_ron);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureDatabaseError::ParseError(_)
        ));
    }

    #[test]
    fn test_default() {
        let db = CreatureDatabase::default();
        assert!(db.is_empty());
    }

    #[test]
    fn test_get_creature_mut() {
        let mut db = CreatureDatabase::new();
        db.add_creature(create_test_creature(1)).unwrap();

        {
            let creature = db.get_creature_mut(1).unwrap();
            creature.name = "Modified Name".to_string();
        }

        let creature = db.get_creature(1).unwrap();
        assert_eq!(creature.name, "Modified Name");
    }

    #[test]
    fn test_validation_error_on_add() {
        let mut db = CreatureDatabase::new();

        // Create invalid creature (no meshes)
        let invalid_creature = CreatureDefinition {
            id: 1,
            name: "Invalid".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };

        let result = db.add_creature(invalid_creature);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureDatabaseError::ValidationError(_, _)
        ));
    }
}
