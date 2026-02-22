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
//!     lod_levels: None,
//!     lod_distances: None,
//!     material: None,
//!     texture_path: None,
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
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
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
    ///     lod_levels: None,
    ///     lod_distances: None,
    ///     material: None,
    ///     texture_path: None,
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
    ///                 vertices: [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.5, 1.0, 0.0)],
    ///                 indices: [0, 1, 2],
    ///             ),
    ///         ],
    ///         mesh_transforms: [
    ///             (
    ///                 translation: (0.0, 0.0, 0.0),
    ///                 rotation: (0.0, 0.0, 0.0),
    ///                 scale: (1.0, 1.0, 1.0),
    ///             ),
    ///         ],
    ///         scale: 1.0,
    ///         color_tint: None,
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

    /// Loads creature registry and resolves all file references eagerly
    ///
    /// Reads a lightweight registry file containing `CreatureReference` entries,
    /// then loads the full `CreatureDefinition` from each referenced file.
    /// All creatures are loaded at campaign startup for performance (eager loading).
    ///
    /// # Arguments
    ///
    /// * `registry_path` - Path to creatures.ron registry file
    /// * `campaign_root` - Campaign root directory for resolving relative paths
    ///
    /// # Returns
    ///
    /// Returns `CreatureDatabase` with all creatures loaded from individual files
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Registry file cannot be read or parsed
    /// - Any referenced creature file fails to load
    /// - Any creature definition is invalid
    /// - Duplicate creature IDs are found
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::visual::creature_database::CreatureDatabase;
    /// use std::path::Path;
    ///
    /// let campaign_root = Path::new("data/test_campaign");
    /// let registry_path = campaign_root.join("data/creatures.ron");
    ///
    /// let db = CreatureDatabase::load_from_registry(&registry_path, campaign_root)
    ///     .expect("Failed to load creature registry");
    ///
    /// assert!(db.count() > 0);
    /// ```
    pub fn load_from_registry(
        registry_path: &Path,
        campaign_root: &Path,
    ) -> Result<Self, CreatureDatabaseError> {
        use crate::domain::visual::CreatureReference;

        // 1. Load registry file as Vec<CreatureReference>
        let registry_contents = std::fs::read_to_string(registry_path)
            .map_err(|e| CreatureDatabaseError::ReadError(e.to_string()))?;

        let references: Vec<CreatureReference> = ron::from_str(&registry_contents)
            .map_err(|e| CreatureDatabaseError::ParseError(e.to_string()))?;

        // 2. Create empty database
        let mut database = Self::new();

        // 3. For each reference, resolve filepath and load creature
        for reference in references {
            // Resolve filepath relative to campaign_root
            let creature_path = campaign_root.join(&reference.filepath);

            // Load full CreatureDefinition from resolved path
            let creature_contents = std::fs::read_to_string(&creature_path).map_err(|e| {
                CreatureDatabaseError::ReadError(format!(
                    "Failed to read creature file '{}': {}",
                    reference.filepath, e
                ))
            })?;

            let mut creature: CreatureDefinition =
                ron::from_str(&creature_contents).map_err(|e| {
                    CreatureDatabaseError::ParseError(format!(
                        "Failed to parse creature file '{}': {}",
                        reference.filepath, e
                    ))
                })?;

            // Registry metadata is authoritative in registry-driven loads.
            // This allows many registry entries to share a single mesh asset file.
            creature.id = reference.id;
            creature.name = reference.name.clone();

            // Add to database (this validates and checks for duplicates)
            database.add_creature(creature)?;
        }

        Ok(database)
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

    #[test]
    fn test_template_files_exist() {
        // Verify template files exist and are readable
        let templates = [
            include_str!("../../../data/creature_templates/humanoid.ron"),
            include_str!("../../../data/creature_templates/quadruped.ron"),
            include_str!("../../../data/creature_templates/dragon.ron"),
            include_str!("../../../data/creature_templates/robot.ron"),
            include_str!("../../../data/creature_templates/undead.ron"),
            include_str!("../../../data/creature_templates/beast.ron"),
        ];

        for template_str in &templates {
            assert!(!template_str.is_empty());
            assert!(template_str.contains("CreatureDefinition"));
        }
    }

    #[test]
    fn test_template_metadata_files_exist() {
        // Verify metadata files exist and are readable
        let metadata_files = [
            include_str!("../../../data/creature_templates/humanoid.meta.ron"),
            include_str!("../../../data/creature_templates/quadruped.meta.ron"),
            include_str!("../../../data/creature_templates/dragon.meta.ron"),
            include_str!("../../../data/creature_templates/robot.meta.ron"),
            include_str!("../../../data/creature_templates/undead.meta.ron"),
            include_str!("../../../data/creature_templates/beast.meta.ron"),
        ];

        for metadata_str in &metadata_files {
            assert!(!metadata_str.is_empty());
            assert!(metadata_str.contains("TemplateMetadata"));
        }
    }

    #[test]
    fn test_template_ids_are_unique() {
        // Verify each template has a unique ID by checking the content
        let templates = [
            (
                "humanoid",
                include_str!("../../../data/creature_templates/humanoid.ron"),
            ),
            (
                "quadruped",
                include_str!("../../../data/creature_templates/quadruped.ron"),
            ),
            (
                "dragon",
                include_str!("../../../data/creature_templates/dragon.ron"),
            ),
            (
                "robot",
                include_str!("../../../data/creature_templates/robot.ron"),
            ),
            (
                "undead",
                include_str!("../../../data/creature_templates/undead.ron"),
            ),
            (
                "beast",
                include_str!("../../../data/creature_templates/beast.ron"),
            ),
        ];

        let expected_ids = [1000, 1001, 1002, 1003, 1004, 1005];

        for ((name, template_str), expected_id) in templates.iter().zip(expected_ids.iter()) {
            assert!(
                template_str.contains(&format!("id: {}", expected_id)),
                "{} template should have ID {}",
                name,
                expected_id
            );
        }
    }

    #[test]
    fn test_template_structure_validity() {
        // Verify templates have basic structural validity
        let templates = [
            (
                "humanoid",
                include_str!("../../../data/creature_templates/humanoid.ron"),
            ),
            (
                "quadruped",
                include_str!("../../../data/creature_templates/quadruped.ron"),
            ),
            (
                "dragon",
                include_str!("../../../data/creature_templates/dragon.ron"),
            ),
            (
                "robot",
                include_str!("../../../data/creature_templates/robot.ron"),
            ),
            (
                "undead",
                include_str!("../../../data/creature_templates/undead.ron"),
            ),
            (
                "beast",
                include_str!("../../../data/creature_templates/beast.ron"),
            ),
        ];

        for (name, template_str) in &templates {
            // Check for required fields
            assert!(template_str.contains("id:"), "{} missing id field", name);
            assert!(
                template_str.contains("name:"),
                "{} missing name field",
                name
            );
            assert!(
                template_str.contains("meshes:"),
                "{} missing meshes field",
                name
            );
            assert!(
                template_str.contains("mesh_transforms:"),
                "{} missing mesh_transforms field",
                name
            );
            assert!(
                template_str.contains("scale:"),
                "{} missing scale field",
                name
            );

            // Check for at least one mesh
            assert!(
                template_str.contains("MeshDefinition"),
                "{} has no mesh definitions",
                name
            );
        }
    }

    #[test]
    fn test_example_creatures_exist() {
        // Verify example creature files exist
        let examples: [&str; 3] = [
            include_str!("../../../data/creature_examples/goblin.ron"),
            include_str!("../../../data/creature_examples/skeleton.ron"),
            include_str!("../../../data/creature_examples/dragon.ron"),
        ];

        for example_str in &examples {
            assert!(!example_str.is_empty());
        }
    }

    #[test]
    fn test_load_from_registry() {
        // Load the tutorial campaign creature registry
        let campaign_root = Path::new("data/test_campaign");
        let registry_path = campaign_root.join("data/creatures.ron");

        // Skip test if campaign files don't exist (e.g., in CI without assets)
        if !registry_path.exists() {
            eprintln!("Skipping test_load_from_registry: registry file not found");
            return;
        }

        // Load creature database from registry
        let result = CreatureDatabase::load_from_registry(&registry_path, campaign_root);

        // Should load successfully
        assert!(
            result.is_ok(),
            "Failed to load creature registry: {:?}",
            result.err()
        );

        let db = result.unwrap();

        // Should have loaded creatures
        assert!(
            db.count() > 0,
            "Expected creatures to be loaded, but database is empty"
        );

        // Verify specific creature IDs are present (from registry)
        assert!(db.has_creature(1), "Expected Goblin (ID 1) to be loaded");
        assert!(db.has_creature(2), "Expected Kobold (ID 2) to be loaded");
        assert!(
            db.has_creature(51),
            "Expected VillageElder (ID 51) to be loaded"
        );

        // Verify creature names match
        let goblin = db.get_creature(1).expect("Goblin should exist");
        assert_eq!(goblin.name, "Goblin");

        // Validate all creatures in database
        assert!(
            db.validate().is_ok(),
            "Database validation failed: {:?}",
            db.validate().err()
        );
    }

    #[test]
    fn test_load_from_registry_missing_file() {
        use tempfile::TempDir;

        // Create temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let campaign_root = temp_dir.path();
        let data_dir = campaign_root.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        // Create registry with non-existent creature file
        let registry_content = r#"[
    CreatureReference(
        id: 999,
        name: "NonExistent",
        filepath: "assets/creatures/non_existent.ron",
    ),
]"#;

        let registry_path = data_dir.join("creatures.ron");
        std::fs::write(&registry_path, registry_content).unwrap();

        // Should fail to load
        let result = CreatureDatabase::load_from_registry(&registry_path, campaign_root);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureDatabaseError::ReadError(_)
        ));
    }

    #[test]
    fn test_load_from_registry_registry_id_overrides_asset_id() {
        use tempfile::TempDir;

        // Create temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let campaign_root = temp_dir.path();
        let data_dir = campaign_root.join("data");
        let assets_dir = campaign_root.join("assets/creatures");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&assets_dir).unwrap();

        // Create creature file with ID 1
        let creature_content = r#"CreatureDefinition(
    id: 1,
    name: "TestCreature",
    meshes: [
        MeshDefinition(
            vertices: [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.5, 1.0, 0.0)],
            indices: [0, 1, 2],
            color: (1.0, 1.0, 1.0, 1.0),
        ),
    ],
    mesh_transforms: [
        MeshTransform(
            translation: (0.0, 0.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            scale: (1.0, 1.0, 1.0),
        ),
    ],
    scale: 1.0,
)"#;

        let creature_path = assets_dir.join("test.ron");
        std::fs::write(&creature_path, creature_content).unwrap();

        // Create registry referencing creature with different ID (999)
        let registry_content = r#"[
    CreatureReference(
        id: 999,
        name: "AliasName",
        filepath: "assets/creatures/test.ron",
    ),
]"#;

        let registry_path = data_dir.join("creatures.ron");
        std::fs::write(&registry_path, registry_content).unwrap();

        // Should succeed and normalize to registry metadata
        let result = CreatureDatabase::load_from_registry(&registry_path, campaign_root);
        assert!(result.is_ok());

        let db = result.unwrap();
        let creature = db.get_creature(999).expect("Creature 999 should be loaded");
        assert_eq!(creature.id, 999);
        assert_eq!(creature.name, "AliasName");
    }

    #[test]
    fn test_load_from_registry_multiple_ids_can_share_one_asset_file() {
        use tempfile::TempDir;

        // Create temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let campaign_root = temp_dir.path();
        let data_dir = campaign_root.join("data");
        let assets_dir = campaign_root.join("assets/creatures");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&assets_dir).unwrap();

        // Shared creature asset file
        let creature_content = r#"CreatureDefinition(
    id: 12,
    name: "Wolf",
    meshes: [
        MeshDefinition(
            vertices: [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.5, 1.0, 0.0)],
            indices: [0, 1, 2],
            color: (1.0, 1.0, 1.0, 1.0),
        ),
    ],
    mesh_transforms: [
        MeshTransform(
            translation: (0.0, 0.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            scale: (1.0, 1.0, 1.0),
        ),
    ],
    scale: 1.0,
)"#;

        let creature_path = assets_dir.join("wolf.ron");
        std::fs::write(&creature_path, creature_content).unwrap();

        // Two IDs map to the same asset file
        let registry_content = r#"[
    CreatureReference(
        id: 4,
        name: "DireWolf",
        filepath: "assets/creatures/wolf.ron",
    ),
    CreatureReference(
        id: 12,
        name: "Wolf",
        filepath: "assets/creatures/wolf.ron",
    ),
]"#;

        let registry_path = data_dir.join("creatures.ron");
        std::fs::write(&registry_path, registry_content).unwrap();

        let db = CreatureDatabase::load_from_registry(&registry_path, campaign_root)
            .expect("Registry should support many-to-one asset mapping");

        assert!(db.has_creature(4));
        assert!(db.has_creature(12));
        assert_eq!(db.count(), 2);
        assert_eq!(db.get_creature(4).unwrap().name, "DireWolf");
        assert_eq!(db.get_creature(12).unwrap().name, "Wolf");
    }
}
