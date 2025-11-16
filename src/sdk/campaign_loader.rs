// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign loader module
//!
//! This module provides structures and functions for loading, validating,
//! and managing game campaigns. Campaigns are self-contained packages of
//! game content including maps, items, quests, dialogues, and configuration.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 6 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::campaign_loader::CampaignLoader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = CampaignLoader::new("campaigns");
//!
//! // List available campaigns
//! let campaigns = loader.list_campaigns()?;
//! for info in campaigns {
//!     println!("Campaign: {} v{}", info.name, info.version);
//! }
//!
//! // Load a campaign
//! let campaign = loader.load_campaign("example")?;
//! println!("Loaded: {}", campaign.name);
//! # Ok(())
//! # }
//! ```

use crate::domain::types::{Direction, Position};
use crate::sdk::database::ContentDatabase;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with campaigns
#[derive(Error, Debug)]
pub enum CampaignError {
    #[error("Campaign not found: {0}")]
    NotFound(String),

    #[error("Invalid campaign structure: {0}")]
    InvalidStructure(String),

    #[error("Failed to load campaign metadata: {0}")]
    MetadataError(String),

    #[error("Campaign validation failed: {0}")]
    ValidationError(String),

    #[error("Incompatible engine version: campaign requires {required}, found {current}")]
    IncompatibleVersion { required: String, current: String },

    #[error("Missing required feature: {0}")]
    MissingFeature(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("RON parsing error: {0}")]
    RonError(#[from] ron::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

// ===== Campaign Structures =====

/// Campaign identifier (directory name)
pub type CampaignId = String;

/// Complete campaign with metadata and loaded content
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::campaign_loader::Campaign;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let campaign = Campaign::load("campaigns/example")?;
/// println!("Campaign: {} by {}", campaign.name, campaign.author);
/// println!("Starting map: {}", campaign.config.starting_map);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    /// Unique campaign identifier (directory name)
    pub id: CampaignId,

    /// Display name
    pub name: String,

    /// Version string (e.g., "1.0.0")
    pub version: String,

    /// Author name
    pub author: String,

    /// Campaign description
    pub description: String,

    /// Required game engine version
    pub engine_version: String,

    /// Required engine features
    pub required_features: Vec<String>,

    /// Campaign configuration
    pub config: CampaignConfig,

    /// Data file paths (relative to campaign directory)
    pub data: CampaignData,

    /// Asset paths
    pub assets: CampaignAssets,

    /// Root path of campaign directory
    #[serde(skip)]
    pub root_path: PathBuf,
}

/// Campaign configuration (gameplay settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    /// Starting map ID
    pub starting_map: u16,

    /// Starting position on map
    pub starting_position: Position,

    /// Starting facing direction
    pub starting_direction: Direction,

    /// Starting gold amount
    pub starting_gold: u32,

    /// Starting food units
    pub starting_food: u32,

    /// Maximum party size (default: 6)
    #[serde(default = "default_max_party_size")]
    pub max_party_size: usize,

    /// Maximum roster size (default: 20)
    #[serde(default = "default_max_roster_size")]
    pub max_roster_size: usize,

    /// Difficulty setting
    #[serde(default)]
    pub difficulty: Difficulty,

    /// Enable permadeath
    #[serde(default)]
    pub permadeath: bool,

    /// Allow multiclassing
    #[serde(default)]
    pub allow_multiclassing: bool,

    /// Starting character level
    #[serde(default = "default_starting_level")]
    pub starting_level: u8,

    /// Maximum character level
    #[serde(default = "default_max_level")]
    pub max_level: u8,
}

fn default_max_party_size() -> usize {
    6
}

fn default_max_roster_size() -> usize {
    20
}

fn default_starting_level() -> u8 {
    1
}

fn default_max_level() -> u8 {
    20
}

/// Difficulty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
    Brutal,
}

/// Data file paths within campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignData {
    /// Items data file
    #[serde(default = "default_items_path")]
    pub items: String,

    /// Spells data file
    #[serde(default = "default_spells_path")]
    pub spells: String,

    /// Monsters data file
    #[serde(default = "default_monsters_path")]
    pub monsters: String,

    /// Classes data file
    #[serde(default = "default_classes_path")]
    pub classes: String,

    /// Races data file
    #[serde(default = "default_races_path")]
    pub races: String,

    /// Maps directory
    #[serde(default = "default_maps_path")]
    pub maps: String,

    /// Quests data file
    #[serde(default = "default_quests_path")]
    pub quests: String,

    /// Dialogues data file
    #[serde(default = "default_dialogues_path")]
    pub dialogues: String,
}

fn default_items_path() -> String {
    "data/items.ron".to_string()
}

fn default_spells_path() -> String {
    "data/spells.ron".to_string()
}

fn default_monsters_path() -> String {
    "data/monsters.ron".to_string()
}

fn default_classes_path() -> String {
    "data/classes.ron".to_string()
}

fn default_races_path() -> String {
    "data/races.ron".to_string()
}

fn default_maps_path() -> String {
    "data/maps".to_string()
}

fn default_quests_path() -> String {
    "data/quests.ron".to_string()
}

fn default_dialogues_path() -> String {
    "data/dialogues.ron".to_string()
}

/// Asset paths within campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignAssets {
    /// Tilesets directory
    #[serde(default = "default_tilesets_path")]
    pub tilesets: String,

    /// Music directory
    #[serde(default = "default_music_path")]
    pub music: String,

    /// Sound effects directory
    #[serde(default = "default_sounds_path")]
    pub sounds: String,

    /// Images directory
    #[serde(default = "default_images_path")]
    pub images: String,
}

fn default_tilesets_path() -> String {
    "assets/tilesets".to_string()
}

fn default_music_path() -> String {
    "assets/music".to_string()
}

fn default_sounds_path() -> String {
    "assets/sounds".to_string()
}

fn default_images_path() -> String {
    "assets/images".to_string()
}

impl Campaign {
    /// Load a campaign from a directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::campaign_loader::Campaign;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let campaign = Campaign::load("campaigns/example")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, CampaignError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(CampaignError::NotFound(path.display().to_string()));
        }

        // Load campaign.ron
        let campaign_file = path.join("campaign.ron");
        if !campaign_file.exists() {
            return Err(CampaignError::InvalidStructure(
                "campaign.ron not found".to_string(),
            ));
        }

        let contents = std::fs::read_to_string(&campaign_file)?;
        let mut campaign: Campaign =
            ron::from_str(&contents).map_err(|e| CampaignError::MetadataError(e.to_string()))?;

        // Set root path
        campaign.root_path = path.to_path_buf();

        // Extract ID from directory name
        if let Some(dir_name) = path.file_name() {
            campaign.id = dir_name.to_string_lossy().to_string();
        }

        Ok(campaign)
    }

    /// Load campaign content into ContentDatabase
    pub fn load_content(&self) -> Result<ContentDatabase, CampaignError> {
        ContentDatabase::load_campaign(&self.root_path)
            .map_err(|e| CampaignError::DatabaseError(e.to_string()))
    }

    /// Validate campaign structure and metadata
    pub fn validate_structure(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check required directories exist
        let data_dir = self.root_path.join("data");
        if !data_dir.exists() {
            errors.push("Missing 'data' directory".to_string());
        }

        // Check README exists
        let readme = self.root_path.join("README.md");
        if !readme.exists() {
            errors.push("Missing README.md".to_string());
        }

        // Validate config
        if self.config.starting_level > self.config.max_level {
            errors.push(format!(
                "starting_level ({}) > max_level ({})",
                self.config.starting_level, self.config.max_level
            ));
        }

        if self.config.max_party_size == 0 {
            errors.push("max_party_size cannot be 0".to_string());
        }

        if self.config.max_roster_size < self.config.max_party_size {
            errors.push("max_roster_size must be >= max_party_size".to_string());
        }

        errors
    }
}

// ===== Campaign Loader =====

/// Campaign loader for discovering and loading campaigns
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::campaign_loader::CampaignLoader;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let loader = CampaignLoader::new("campaigns");
/// let campaigns = loader.list_campaigns()?;
///
/// for info in campaigns {
///     println!("{}: {}", info.id, info.name);
/// }
/// # Ok(())
/// # }
/// ```
pub struct CampaignLoader {
    campaigns_dir: PathBuf,
}

impl CampaignLoader {
    /// Create a new campaign loader
    pub fn new<P: AsRef<Path>>(campaigns_dir: P) -> Self {
        Self {
            campaigns_dir: campaigns_dir.as_ref().to_path_buf(),
        }
    }

    /// List all available campaigns
    pub fn list_campaigns(&self) -> Result<Vec<CampaignInfo>, CampaignError> {
        if !self.campaigns_dir.exists() {
            return Ok(Vec::new());
        }

        let mut campaigns = Vec::new();

        for entry in std::fs::read_dir(&self.campaigns_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Ok(campaign) = Campaign::load(&path) {
                    let errors = campaign.validate_structure();
                    campaigns.push(CampaignInfo {
                        id: campaign.id.clone(),
                        name: campaign.name.clone(),
                        version: campaign.version.clone(),
                        author: campaign.author.clone(),
                        description: campaign.description.clone(),
                        is_valid: errors.is_empty(),
                        path: path.clone(),
                    });
                }
            }
        }

        Ok(campaigns)
    }

    /// Load a campaign by ID
    pub fn load_campaign(&self, id: &str) -> Result<Campaign, CampaignError> {
        let path = self.campaigns_dir.join(id);
        Campaign::load(path)
    }

    /// Validate a campaign
    pub fn validate_campaign(&self, id: &str) -> Result<ValidationReport, CampaignError> {
        let campaign = self.load_campaign(id)?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Structure validation
        errors.extend(campaign.validate_structure());

        // Try to load content
        match campaign.load_content() {
            Ok(db) => {
                // Content loaded successfully
                let stats = db.stats();
                if stats.class_count == 0 {
                    warnings.push("No classes defined".to_string());
                }
                if stats.item_count == 0 {
                    warnings.push("No items defined".to_string());
                }
                if stats.map_count == 0 {
                    errors.push("No maps defined - campaign cannot be played".to_string());
                }
            }
            Err(e) => {
                errors.push(format!("Failed to load content: {}", e));
            }
        }

        Ok(ValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }
}

/// Campaign information (lightweight metadata)
#[derive(Debug, Clone)]
pub struct CampaignInfo {
    /// Campaign ID
    pub id: CampaignId,

    /// Campaign name
    pub name: String,

    /// Version string
    pub version: String,

    /// Author name
    pub author: String,

    /// Description
    pub description: String,

    /// Whether campaign passes basic validation
    pub is_valid: bool,

    /// Path to campaign directory
    pub path: PathBuf,
}

/// Campaign validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Whether campaign is valid
    pub is_valid: bool,

    /// Validation errors (must be fixed)
    pub errors: Vec<String>,

    /// Warnings (should be addressed)
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// Returns true if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns true if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns total issue count
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_config_defaults() {
        let config = CampaignConfig {
            starting_map: 1,
            starting_position: Position::new(10, 10),
            starting_direction: Direction::North,
            starting_gold: 100,
            starting_food: 50,
            max_party_size: default_max_party_size(),
            max_roster_size: default_max_roster_size(),
            difficulty: Difficulty::default(),
            permadeath: false,
            allow_multiclassing: false,
            starting_level: default_starting_level(),
            max_level: default_max_level(),
        };

        assert_eq!(config.max_party_size, 6);
        assert_eq!(config.max_roster_size, 20);
        assert_eq!(config.starting_level, 1);
        assert_eq!(config.max_level, 20);
        assert_eq!(config.difficulty, Difficulty::Normal);
    }

    #[test]
    fn test_difficulty_default() {
        assert_eq!(Difficulty::default(), Difficulty::Normal);
    }

    #[test]
    fn test_campaign_data_defaults() {
        let data = CampaignData {
            items: default_items_path(),
            spells: default_spells_path(),
            monsters: default_monsters_path(),
            classes: default_classes_path(),
            races: default_races_path(),
            maps: default_maps_path(),
            quests: default_quests_path(),
            dialogues: default_dialogues_path(),
        };

        assert_eq!(data.items, "data/items.ron");
        assert_eq!(data.maps, "data/maps");
        assert_eq!(data.quests, "data/quests.ron");
    }

    #[test]
    fn test_validation_report_checks() {
        let report = ValidationReport {
            is_valid: false,
            errors: vec!["Error 1".to_string()],
            warnings: vec!["Warning 1".to_string(), "Warning 2".to_string()],
        };

        assert!(report.has_errors());
        assert!(report.has_warnings());
        assert_eq!(report.issue_count(), 3);
    }

    #[test]
    fn test_validation_report_no_issues() {
        let report = ValidationReport {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        assert!(!report.has_errors());
        assert!(!report.has_warnings());
        assert_eq!(report.issue_count(), 0);
    }
}
