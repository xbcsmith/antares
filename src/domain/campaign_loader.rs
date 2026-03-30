// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign loader for loading game data with override support
//!
//! This module provides campaign loading functionality with support for
//! content overrides and validation.
//!
//! # Examples
//!
//! ```no_run
//! use antares::domain::campaign_loader::{CampaignLoader, GameData};
//! use std::path::PathBuf;
//!
//! let base_path = PathBuf::from("data");
//! let campaign_path = PathBuf::from("data/test_campaign");
//! let mut loader = CampaignLoader::new(base_path, campaign_path);
//!
//! // Load game data
//! // let game_data = loader.load_game_data()?;
//! ```

use std::path::PathBuf;

use thiserror::Error;

use crate::domain::items::database::ItemMeshDatabase;
use crate::domain::visual::creature_database::CreatureDatabase;
use crate::domain::world::furniture::{FurnitureDatabase, FurnitureMeshDatabase};

/// Campaign validation errors
#[derive(Debug, Error)]
pub enum CampaignError {
    /// Campaign not found
    #[error("Campaign not found: {0}")]
    NotFound(String),

    /// Invalid campaign format
    #[error("Invalid campaign format: {0}")]
    InvalidFormat(String),

    /// Missing dependency
    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    /// Data version mismatch
    #[error("Data version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// Expected version
        expected: String,
        /// Found version
        found: String,
    },

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// File read error
    #[error("Failed to read file: {0}")]
    ReadError(String),

    /// Parse error
    #[error("Failed to parse data: {0}")]
    ParseError(String),
}

/// Complete game data for a campaign
///
/// Contains all loaded game content including creatures, items, spells, etc.
///
/// # Examples
///
/// ```
/// use antares::domain::campaign_loader::GameData;
/// use antares::domain::visual::creature_database::CreatureDatabase;
/// use antares::domain::items::database::ItemMeshDatabase;
/// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureMeshDatabase};
///
/// let game_data = GameData {
///     creatures: CreatureDatabase::new(),
///     item_meshes: ItemMeshDatabase::new(),
///     furniture: FurnitureDatabase::new(),
///     furniture_meshes: FurnitureMeshDatabase::new(),
/// };
///
/// assert!(game_data.creatures.is_empty());
/// assert!(game_data.item_meshes.is_empty());
/// assert!(game_data.furniture.is_empty());
/// assert!(game_data.furniture_meshes.is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct GameData {
    /// Creature visual database
    pub creatures: CreatureDatabase,
    /// Item mesh database — visual definitions for dropped items
    pub item_meshes: ItemMeshDatabase,
    /// Furniture definition database — named, reusable furniture templates
    pub furniture: FurnitureDatabase,
    /// Furniture mesh database — custom OBJ-imported mesh definitions
    pub furniture_meshes: FurnitureMeshDatabase,
}

impl GameData {
    /// Creates a new empty GameData
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::campaign_loader::GameData;
    ///
    /// let data = GameData::new();
    /// assert!(data.creatures.is_empty());
    /// assert!(data.item_meshes.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            creatures: CreatureDatabase::new(),
            item_meshes: ItemMeshDatabase::new(),
            furniture: FurnitureDatabase::new(),
            furniture_meshes: FurnitureMeshDatabase::new(),
        }
    }

    /// Validates all game data cross-references
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::ValidationFailed` if validation fails
    pub fn validate(&self) -> Result<(), CampaignError> {
        // Validate creature database
        self.creatures
            .validate()
            .map_err(|e| CampaignError::ValidationFailed(format!("Creature validation: {}", e)))?;

        // Validate item mesh database
        self.item_meshes
            .validate()
            .map_err(|e| CampaignError::ValidationFailed(format!("Item mesh validation: {}", e)))?;

        // Furniture database has no external references to validate;
        // an empty database is always valid.

        // Validate furniture mesh database
        self.furniture_meshes.validate().map_err(|e| {
            CampaignError::ValidationFailed(format!("Furniture mesh validation: {}", e))
        })?;

        Ok(())
    }
}

/// Campaign loader with override support
///
/// Loads game data from base and campaign-specific paths with override support.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::campaign_loader::CampaignLoader;
/// use std::path::PathBuf;
///
/// let base_path = PathBuf::from("data");
/// let campaign_path = PathBuf::from("data/test_campaign");
/// let loader = CampaignLoader::new(base_path, campaign_path);
/// ```
#[derive(Debug, Clone)]
pub struct CampaignLoader {
    base_data_path: PathBuf,
    campaign_path: PathBuf,
}

impl CampaignLoader {
    /// Creates a new campaign loader
    ///
    /// # Arguments
    ///
    /// * `base_data_path` - Path to base game data directory
    /// * `campaign_path` - Path to campaign directory
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::campaign_loader::CampaignLoader;
    /// use std::path::PathBuf;
    ///
    /// let loader = CampaignLoader::new(
    ///     PathBuf::from("data"),
    ///     PathBuf::from("data/test_campaign")
    /// );
    /// ```
    pub fn new(base_data_path: PathBuf, campaign_path: PathBuf) -> Self {
        Self {
            base_data_path,
            campaign_path,
        }
    }

    /// Loads complete game data for the campaign
    ///
    /// # Errors
    ///
    /// Returns `CampaignError` if loading or validation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::campaign_loader::CampaignLoader;
    /// use std::path::PathBuf;
    ///
    /// let mut loader = CampaignLoader::new(
    ///     PathBuf::from("data"),
    ///     PathBuf::from("data/test_campaign")
    /// );
    ///
    /// // let game_data = loader.load_game_data()?;
    /// ```
    pub fn load_game_data(&mut self) -> Result<GameData, CampaignError> {
        let mut game_data = GameData::new();

        // Load creatures from campaign path
        game_data.creatures = self.load_creatures()?;

        // Load item mesh registry (opt-in per campaign; missing file is OK)
        game_data.item_meshes = self.load_item_meshes()?;

        // Load furniture definitions (opt-in per campaign; missing file is OK)
        game_data.furniture = self.load_furniture()?;

        // Load furniture mesh registry (opt-in per campaign; missing file is OK)
        game_data.furniture_meshes = self.load_furniture_meshes()?;

        // Validate all loaded data
        game_data.validate()?;

        Ok(game_data)
    }

    /// Loads creature database from campaign
    ///
    /// Attempts to load from campaign path first, falls back to base path.
    /// Uses registry-based loading (CreatureReference) which loads individual
    /// creature files from the registry.
    ///
    /// # Errors
    ///
    /// Returns `CampaignError` if loading fails
    fn load_creatures(&self) -> Result<CreatureDatabase, CampaignError> {
        // Try campaign-specific creatures registry file first
        let campaign_creatures_path = self.campaign_path.join("data/creatures.ron");

        if campaign_creatures_path.exists() {
            // Use load_from_registry for the new registry format
            CreatureDatabase::load_from_registry(&campaign_creatures_path, &self.campaign_path)
                .map_err(|e| CampaignError::ReadError(format!("Campaign creatures: {}", e)))
        } else {
            // Fall back to base data path
            let base_creatures_path = self.base_data_path.join("creatures.ron");

            if base_creatures_path.exists() {
                // Try registry format first
                match CreatureDatabase::load_from_registry(
                    &base_creatures_path,
                    &self.base_data_path,
                ) {
                    Ok(db) => Ok(db),
                    Err(_) => {
                        // Fall back to direct loading if not registry format
                        let creatures = CreatureDatabase::load_from_file(&base_creatures_path)
                            .map_err(|e| {
                                CampaignError::ReadError(format!("Base creatures: {}", e))
                            })?;

                        // Convert Vec<CreatureDefinition> to CreatureDatabase
                        let mut db = CreatureDatabase::new();
                        for creature in creatures {
                            db.add_creature(creature).map_err(|e| {
                                CampaignError::ValidationFailed(format!(
                                    "Failed to add creature: {}",
                                    e
                                ))
                            })?;
                        }
                        Ok(db)
                    }
                }
            } else {
                // Return empty database if no files found
                Ok(CreatureDatabase::new())
            }
        }
    }

    /// Loads furniture definitions from the campaign.
    ///
    /// Looks for `data/furniture.ron` inside the campaign directory.
    /// If the file does not exist the function returns an empty
    /// [`FurnitureDatabase`] without error — furniture definition support is
    /// opt-in per campaign.
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::ReadError` if the file exists but cannot be
    /// read, or `CampaignError::ParseError` if it cannot be parsed.
    fn load_furniture(&self) -> Result<FurnitureDatabase, CampaignError> {
        let furniture_path = self.campaign_path.join("data/furniture.ron");

        if !furniture_path.exists() {
            // Missing furniture.ron is not an error — campaign simply has no furniture definitions
            return Ok(FurnitureDatabase::new());
        }

        FurnitureDatabase::load_from_file(&furniture_path).map_err(|e| {
            CampaignError::ReadError(format!(
                "furniture.ron '{}': {}",
                furniture_path.display(),
                e
            ))
        })
    }

    /// Loads item mesh database from campaign.
    ///
    /// Looks for `data/item_mesh_registry.ron` inside the campaign directory.
    /// If the file does not exist the function returns an empty
    /// [`ItemMeshDatabase`] without error — item mesh support is opt-in per
    /// campaign.
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::ReadError` if the registry file exists but
    /// cannot be read, or `CampaignError::ParseError` if it cannot be parsed.
    fn load_item_meshes(&self) -> Result<ItemMeshDatabase, CampaignError> {
        let registry_path = self.campaign_path.join("data/item_mesh_registry.ron");

        if !registry_path.exists() {
            // Missing registry is not an error — campaign simply has no item meshes
            return Ok(ItemMeshDatabase::new());
        }

        ItemMeshDatabase::load_from_registry(&registry_path, &self.campaign_path).map_err(|e| {
            CampaignError::ReadError(format!(
                "Item mesh registry '{}': {}",
                registry_path.display(),
                e
            ))
        })
    }

    /// Loads furniture mesh database from campaign.
    ///
    /// Looks for `data/furniture_mesh_registry.ron` inside the campaign
    /// directory. If the file does not exist the function returns an empty
    /// [`FurnitureMeshDatabase`] without error — furniture mesh support is
    /// opt-in per campaign.
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::ReadError` if the registry file exists but
    /// cannot be read, or `CampaignError::ParseError` if it cannot be parsed.
    fn load_furniture_meshes(&self) -> Result<FurnitureMeshDatabase, CampaignError> {
        let registry_path = self.campaign_path.join("data/furniture_mesh_registry.ron");

        if !registry_path.exists() {
            // Missing registry is not an error — campaign simply has no furniture meshes
            return Ok(FurnitureMeshDatabase::new());
        }

        FurnitureMeshDatabase::load_from_registry(&registry_path, &self.campaign_path).map_err(
            |e| {
                CampaignError::ReadError(format!(
                    "Furniture mesh registry '{}': {}",
                    registry_path.display(),
                    e
                ))
            },
        )
    }

    /// Gets the campaign path
    pub fn campaign_path(&self) -> &PathBuf {
        &self.campaign_path
    }

    /// Gets the base data path
    pub fn base_data_path(&self) -> &PathBuf {
        &self.base_data_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_data_new() {
        let data = GameData::new();
        assert!(data.creatures.is_empty());
        assert!(data.item_meshes.is_empty());
        assert!(data.furniture.is_empty());
    }

    #[test]
    fn test_game_data_default() {
        let data = GameData::default();
        assert!(data.creatures.is_empty());
        assert!(data.item_meshes.is_empty());
        assert!(data.furniture.is_empty());
    }

    #[test]
    fn test_game_data_validate_empty() {
        let data = GameData::new();
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_campaign_loader_new() {
        let loader =
            CampaignLoader::new(PathBuf::from("data"), PathBuf::from("data/test_campaign"));

        assert_eq!(loader.base_data_path(), &PathBuf::from("data"));
        assert_eq!(loader.campaign_path(), &PathBuf::from("data/test_campaign"));
    }

    #[test]
    fn test_load_creatures_no_files() {
        let loader = CampaignLoader::new(
            PathBuf::from("nonexistent_data"),
            PathBuf::from("nonexistent_campaign"),
        );

        let result = loader.load_creatures();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// §3.5 — loads `data/test_campaign` and asserts `item_meshes` has ≥ 2 entries.
    #[test]
    fn test_campaign_loader_loads_item_mesh_registry() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base = std::path::PathBuf::from(manifest_dir).join("data");
        let campaign = base.join("test_campaign");

        let mut loader = CampaignLoader::new(base, campaign);
        let result = loader.load_game_data();
        assert!(result.is_ok(), "load_game_data failed: {:?}", result.err());

        let game_data = result.unwrap();
        assert!(
            game_data.item_meshes.count() >= 2,
            "Expected ≥ 2 item mesh entries, got {}",
            game_data.item_meshes.count()
        );
        assert!(
            game_data.item_meshes.has_mesh(9001),
            "Expected item mesh id 9001 (sword) to be present"
        );
        assert!(
            game_data.item_meshes.has_mesh(9201),
            "Expected item mesh id 9201 (potion) to be present"
        );
    }

    /// §3.5 — a campaign without `item_mesh_registry.ron` loads without error.
    #[test]
    fn test_item_mesh_registry_missing_is_ok() {
        let loader = CampaignLoader::new(
            PathBuf::from("nonexistent_data"),
            PathBuf::from("nonexistent_campaign"),
        );

        let result = loader.load_item_meshes();
        assert!(
            result.is_ok(),
            "Expected empty ItemMeshDatabase when registry is absent"
        );
        assert!(result.unwrap().is_empty());
    }

    /// §1.5 — loads `data/test_campaign` and asserts furniture database has ≥ 11 entries.
    #[test]
    fn test_campaign_loader_loads_furniture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base = std::path::PathBuf::from(manifest_dir).join("data");
        let campaign = base.join("test_campaign");

        let mut loader = CampaignLoader::new(base, campaign);
        let result = loader.load_game_data();
        assert!(result.is_ok(), "load_game_data failed: {:?}", result.err());

        let game_data = result.unwrap();
        assert!(
            game_data.furniture.len() >= 11,
            "Expected ≥ 11 furniture definitions, got {}",
            game_data.furniture.len()
        );
    }

    /// §1.5 — a campaign without `furniture.ron` loads without error and returns empty database.
    #[test]
    fn test_furniture_missing_is_ok() {
        let loader = CampaignLoader::new(
            PathBuf::from("nonexistent_data"),
            PathBuf::from("nonexistent_campaign"),
        );

        let result = loader.load_furniture();
        assert!(
            result.is_ok(),
            "Expected empty FurnitureDatabase when furniture.ron is absent"
        );
        assert!(result.unwrap().is_empty());
    }
}
