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
//! let campaign_path = PathBuf::from("campaigns/tutorial");
//! let mut loader = CampaignLoader::new(base_path, campaign_path);
//!
//! // Load game data
//! // let game_data = loader.load_game_data()?;
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::visual::creature_database::CreatureDatabase;

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
///
/// let game_data = GameData {
///     creatures: CreatureDatabase::new(),
/// };
///
/// assert!(game_data.creatures.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// Creature visual database
    pub creatures: CreatureDatabase,
    // Future: items, spells, monsters, characters, etc.
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
    /// ```
    pub fn new() -> Self {
        Self {
            creatures: CreatureDatabase::new(),
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

        Ok(())
    }
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
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
/// let campaign_path = PathBuf::from("campaigns/tutorial");
/// let loader = CampaignLoader::new(base_path, campaign_path);
/// ```
#[derive(Debug, Clone)]
pub struct CampaignLoader {
    base_data_path: PathBuf,
    campaign_path: PathBuf,
    #[allow(dead_code)]
    content_cache: HashMap<String, String>,
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
    ///     PathBuf::from("campaigns/tutorial")
    /// );
    /// ```
    pub fn new(base_data_path: PathBuf, campaign_path: PathBuf) -> Self {
        Self {
            base_data_path,
            campaign_path,
            content_cache: HashMap::new(),
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
    ///     PathBuf::from("campaigns/tutorial")
    /// );
    ///
    /// // let game_data = loader.load_game_data()?;
    /// ```
    pub fn load_game_data(&mut self) -> Result<GameData, CampaignError> {
        let mut game_data = GameData::new();

        // Load creatures from campaign path
        game_data.creatures = self.load_creatures()?;

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

    /// Loads a data file with override support
    ///
    /// Loads from campaign path if override exists, otherwise from base path.
    ///
    /// # Errors
    ///
    /// Returns `CampaignError` if loading or parsing fails
    #[allow(dead_code)]
    fn load_with_override<T>(
        &self,
        base_file: &str,
        override_file: Option<&str>,
    ) -> Result<T, CampaignError>
    where
        T: DeserializeOwned + Clone,
    {
        let file_path = if let Some(override_path) = override_file {
            self.campaign_path.join(override_path)
        } else {
            self.base_data_path.join(base_file)
        };

        let contents = std::fs::read_to_string(&file_path)
            .map_err(|e| CampaignError::ReadError(format!("{}: {}", file_path.display(), e)))?;

        ron::from_str::<T>(&contents)
            .map_err(|e| CampaignError::ParseError(format!("{}: {}", file_path.display(), e)))
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
    }

    #[test]
    fn test_game_data_default() {
        let data = GameData::default();
        assert!(data.creatures.is_empty());
    }

    #[test]
    fn test_game_data_validate_empty() {
        let data = GameData::new();
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_campaign_loader_new() {
        let loader =
            CampaignLoader::new(PathBuf::from("data"), PathBuf::from("campaigns/tutorial"));

        assert_eq!(loader.base_data_path(), &PathBuf::from("data"));
        assert_eq!(loader.campaign_path(), &PathBuf::from("campaigns/tutorial"));
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
}
