// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature Asset Management
//!
//! This module provides functionality for managing creature asset files within
//! a campaign, including save, load, list, delete, and duplicate operations.
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::creature_assets::CreatureAssetManager;
//! use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let campaign_dir = PathBuf::from("campaigns/my_campaign");
//! let manager = CreatureAssetManager::new(campaign_dir);
//!
//! // Create a simple creature
//! let creature = CreatureDefinition {
//!     id: 1,
//!     name: "Test Creature".to_string(),
//!     meshes: vec![],
//!     mesh_transforms: vec![],
//!     scale: 1.0,
//!     color_tint: None,
//! };
//!
//! // Save creature
//! manager.save_creature(&creature)?;
//!
//! // List all creatures
//! let creatures = manager.list_creatures()?;
//! println!("Found {} creatures", creatures.len());
//! # Ok(())
//! # }
//! ```

use antares::domain::visual::CreatureDefinition;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during creature asset operations
#[derive(Error, Debug)]
pub enum CreatureAssetError {
    /// I/O error during file operations
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// RON serialization error
    #[error("RON serialization error: {0}")]
    SerializationError(#[from] ron::Error),

    /// RON deserialization error
    #[error("RON deserialization error: {0}")]
    DeserializationError(String),

    /// Creature not found
    #[error("Creature '{0}' not found")]
    CreatureNotFound(String),

    /// Campaign directory not found
    #[error("Campaign directory not found: {0:?}")]
    CampaignNotFound(PathBuf),

    /// Creature already exists
    #[error("Creature '{0}' already exists")]
    CreatureExists(String),
}

/// Manages creature asset files within a campaign
///
/// Provides high-level operations for creature file management including
/// save, load, list, delete, and duplicate functionality.
#[derive(Debug, Clone)]
pub struct CreatureAssetManager {
    /// Path to campaign directory
    campaign_dir: PathBuf,
}

impl CreatureAssetManager {
    /// Creates a new creature asset manager for the given campaign directory
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` - Path to the campaign directory
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// ```
    pub fn new(campaign_dir: PathBuf) -> Self {
        Self { campaign_dir }
    }

    /// Gets the path to the creatures data directory
    fn creatures_dir(&self) -> PathBuf {
        self.campaign_dir.join("data")
    }

    /// Gets the path to the creatures RON file
    fn creatures_file(&self) -> PathBuf {
        self.creatures_dir().join("creatures.ron")
    }

    /// Saves a creature to the campaign's creature file
    ///
    /// This appends the creature to the existing creatures or creates a new file.
    ///
    /// # Arguments
    ///
    /// * `creature` - The creature definition to save
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::IoError` for file system errors
    /// Returns `CreatureAssetError::SerializationError` for RON serialization errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use antares::domain::visual::{CreatureDefinition, MeshTransform};
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Goblin".to_string(),
    ///     meshes: vec![],
    ///     mesh_transforms: vec![],
    ///     scale: 1.0,
    ///     color_tint: None,
    /// };
    ///
    /// manager.save_creature(&creature)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_creature(&self, creature: &CreatureDefinition) -> Result<(), CreatureAssetError> {
        // Ensure data directory exists
        let data_dir = self.creatures_dir();
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        // Load existing creatures or create new list
        let mut creatures = if self.creatures_file().exists() {
            self.load_all_creatures()?
        } else {
            Vec::new()
        };

        // Update or add creature
        if let Some(existing) = creatures.iter_mut().find(|c| c.id == creature.id) {
            *existing = creature.clone();
        } else {
            creatures.push(creature.clone());
        }

        // Write back to file
        self.write_creatures_file(&creatures)?;

        Ok(())
    }

    /// Loads a creature by ID from the campaign
    ///
    /// # Arguments
    ///
    /// * `creature_id` - The ID of the creature to load
    ///
    /// # Returns
    ///
    /// Returns `Ok(CreatureDefinition)` if found
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::CreatureNotFound` if creature doesn't exist
    /// Returns `CreatureAssetError::IoError` for file system errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let creature = manager.load_creature(1)?;
    /// println!("Loaded: {}", creature.name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_creature(
        &self,
        creature_id: u32,
    ) -> Result<CreatureDefinition, CreatureAssetError> {
        let creatures = self.load_all_creatures()?;

        creatures
            .into_iter()
            .find(|c| c.id == creature_id)
            .ok_or_else(|| CreatureAssetError::CreatureNotFound(creature_id.to_string()))
    }

    /// Loads all creatures from the campaign
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<CreatureDefinition>)` with all creatures
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::IoError` for file system errors
    /// Returns `CreatureAssetError::DeserializationError` for RON parsing errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let creatures = manager.load_all_creatures()?;
    /// println!("Found {} creatures", creatures.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_all_creatures(&self) -> Result<Vec<CreatureDefinition>, CreatureAssetError> {
        let file_path = self.creatures_file();

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&file_path)?;

        ron::from_str::<Vec<CreatureDefinition>>(&contents)
            .map_err(|e| CreatureAssetError::DeserializationError(e.to_string()))
    }

    /// Lists all creature names in the campaign
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<String>)` with creature names
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::IoError` for file system errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let names = manager.list_creatures()?;
    /// for name in names {
    ///     println!("- {}", name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_creatures(&self) -> Result<Vec<String>, CreatureAssetError> {
        let creatures = self.load_all_creatures()?;
        Ok(creatures.into_iter().map(|c| c.name).collect())
    }

    /// Deletes a creature by ID from the campaign
    ///
    /// # Arguments
    ///
    /// * `creature_id` - The ID of the creature to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::CreatureNotFound` if creature doesn't exist
    /// Returns `CreatureAssetError::IoError` for file system errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// manager.delete_creature(1)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_creature(&self, creature_id: u32) -> Result<(), CreatureAssetError> {
        let mut creatures = self.load_all_creatures()?;

        let original_len = creatures.len();
        creatures.retain(|c| c.id != creature_id);

        if creatures.len() == original_len {
            return Err(CreatureAssetError::CreatureNotFound(
                creature_id.to_string(),
            ));
        }

        self.write_creatures_file(&creatures)?;
        Ok(())
    }

    /// Duplicates a creature with a new ID and name
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the creature to duplicate
    /// * `new_id` - The ID for the new creature
    /// * `new_name` - The name for the new creature
    ///
    /// # Returns
    ///
    /// Returns `Ok(CreatureDefinition)` with the duplicated creature
    ///
    /// # Errors
    ///
    /// Returns `CreatureAssetError::CreatureNotFound` if source doesn't exist
    /// Returns `CreatureAssetError::CreatureExists` if new ID already exists
    /// Returns `CreatureAssetError::IoError` for file system errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let duplicate = manager.duplicate_creature(1, 2, "Goblin Variant".to_string())?;
    /// println!("Created: {}", duplicate.name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn duplicate_creature(
        &self,
        source_id: u32,
        new_id: u32,
        new_name: String,
    ) -> Result<CreatureDefinition, CreatureAssetError> {
        let creatures = self.load_all_creatures()?;

        // Find source creature
        let source = creatures
            .iter()
            .find(|c| c.id == source_id)
            .ok_or_else(|| CreatureAssetError::CreatureNotFound(source_id.to_string()))?;

        // Check new ID doesn't exist
        if creatures.iter().any(|c| c.id == new_id) {
            return Err(CreatureAssetError::CreatureExists(new_id.to_string()));
        }

        // Create duplicate with new ID and name
        let mut duplicate = source.clone();
        duplicate.id = new_id;
        duplicate.name = new_name;

        // Save duplicate
        self.save_creature(&duplicate)?;

        Ok(duplicate)
    }

    /// Writes creatures list to the RON file
    fn write_creatures_file(
        &self,
        creatures: &[CreatureDefinition],
    ) -> Result<(), CreatureAssetError> {
        let file_path = self.creatures_file();

        let ron_string = ron::ser::to_string_pretty(creatures, Default::default())?;
        fs::write(&file_path, ron_string)?;

        Ok(())
    }

    /// Checks if a creature exists by ID
    ///
    /// # Arguments
    ///
    /// * `creature_id` - The ID to check
    ///
    /// # Returns
    ///
    /// Returns `true` if creature exists, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// if manager.has_creature(1)? {
    ///     println!("Creature exists");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn has_creature(&self, creature_id: u32) -> Result<bool, CreatureAssetError> {
        let creatures = self.load_all_creatures()?;
        Ok(creatures.iter().any(|c| c.id == creature_id))
    }

    /// Gets the next available creature ID
    ///
    /// # Returns
    ///
    /// Returns the next unused creature ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creature_assets::CreatureAssetManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreatureAssetManager::new(PathBuf::from("campaigns/test"));
    /// let next_id = manager.next_creature_id()?;
    /// println!("Next available ID: {}", next_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn next_creature_id(&self) -> Result<u32, CreatureAssetError> {
        let creatures = self.load_all_creatures()?;

        if creatures.is_empty() {
            Ok(1)
        } else {
            let max_id = creatures.iter().map(|c| c.id).max().unwrap_or(0);
            Ok(max_id + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::visual::MeshTransform;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_creature(id: u32, name: &str) -> CreatureDefinition {
        CreatureDefinition {
            id,
            name: name.to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    #[test]
    fn test_creature_asset_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        assert_eq!(manager.campaign_dir, temp_dir.path());
    }

    #[test]
    fn test_save_and_load_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let creature = create_test_creature(1, "Test Creature");
        manager.save_creature(&creature).unwrap();

        let loaded = manager.load_creature(1).unwrap();
        assert_eq!(loaded.id, 1);
        assert_eq!(loaded.name, "Test Creature");
    }

    #[test]
    fn test_load_all_creatures() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Creature 1"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(2, "Creature 2"))
            .unwrap();

        let creatures = manager.load_all_creatures().unwrap();
        assert_eq!(creatures.len(), 2);
    }

    #[test]
    fn test_list_creatures() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Goblin"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(2, "Orc"))
            .unwrap();

        let names = manager.list_creatures().unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Goblin".to_string()));
        assert!(names.contains(&"Orc".to_string()));
    }

    #[test]
    fn test_delete_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Test"))
            .unwrap();
        assert!(manager.has_creature(1).unwrap());

        manager.delete_creature(1).unwrap();
        assert!(!manager.has_creature(1).unwrap());
    }

    #[test]
    fn test_delete_nonexistent_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let result = manager.delete_creature(999);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureAssetError::CreatureNotFound(_)
        ));
    }

    #[test]
    fn test_duplicate_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Original"))
            .unwrap();

        let duplicate = manager
            .duplicate_creature(1, 2, "Copy".to_string())
            .unwrap();
        assert_eq!(duplicate.id, 2);
        assert_eq!(duplicate.name, "Copy");

        assert!(manager.has_creature(1).unwrap());
        assert!(manager.has_creature(2).unwrap());
    }

    #[test]
    fn test_duplicate_nonexistent_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let result = manager.duplicate_creature(999, 2, "Copy".to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureAssetError::CreatureNotFound(_)
        ));
    }

    #[test]
    fn test_duplicate_to_existing_id() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "First"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(2, "Second"))
            .unwrap();

        let result = manager.duplicate_creature(1, 2, "Copy".to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureAssetError::CreatureExists(_)
        ));
    }

    #[test]
    fn test_has_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        assert!(!manager.has_creature(1).unwrap());

        manager
            .save_creature(&create_test_creature(1, "Test"))
            .unwrap();
        assert!(manager.has_creature(1).unwrap());
    }

    #[test]
    fn test_next_creature_id_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        assert_eq!(manager.next_creature_id().unwrap(), 1);
    }

    #[test]
    fn test_next_creature_id_with_creatures() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "First"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(5, "Fifth"))
            .unwrap();

        assert_eq!(manager.next_creature_id().unwrap(), 6);
    }

    #[test]
    fn test_update_existing_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Original"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(1, "Updated"))
            .unwrap();

        let creatures = manager.load_all_creatures().unwrap();
        assert_eq!(creatures.len(), 1);
        assert_eq!(creatures[0].name, "Updated");
    }

    #[test]
    fn test_load_from_empty_campaign() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let creatures = manager.load_all_creatures().unwrap();
        assert_eq!(creatures.len(), 0);
    }

    #[test]
    fn test_load_nonexistent_creature() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let result = manager.load_creature(999);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CreatureAssetError::CreatureNotFound(_)
        ));
    }
}
