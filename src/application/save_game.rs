// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Save game system
//!
//! This module provides functionality for saving and loading game state,
//! including campaign reference tracking for campaign-based games.
//!
//! # Examples
//!
//! ```no_run
//! use antares::application::save_game::{SaveGame, SaveGameManager};
//! use antares::application::GameState;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = SaveGameManager::new("saves")?;
//! let game_state = GameState::new();
//!
//! // Save game
//! manager.save("my_save", &game_state)?;
//!
//! // Load game
//! let loaded_state = manager.load("my_save")?;
//! # Ok(())
//! # }
//! ```

use crate::application::GameState;
use bevy::prelude::Resource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Parsed semantic version for compatibility checking.
///
/// Implements the subset of semver needed to validate save game versions:
/// - Same major version → compatible (with optional migration)
/// - Different major version → incompatible
/// - Minor/patch differences → compatible with logged warnings
#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemVer {
    /// Parses a "major.minor.patch" string into a `SemVer`.
    ///
    /// Returns `None` if the string is not a valid semver triple.
    fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    /// Returns `true` if `other` is compatible with `self` (same major version).
    fn is_compatible_with(&self, other: &SemVer) -> bool {
        self.major == other.major
    }
}

/// Save game errors
#[derive(Error, Debug)]
pub enum SaveGameError {
    /// Failed to read save file
    #[error("Failed to read save file: {0}")]
    ReadError(String),

    /// Failed to write save file
    #[error("Failed to write save file: {0}")]
    WriteError(String),

    /// Failed to parse save file
    #[error("Failed to parse save file: {0}")]
    ParseError(String),

    /// Save file version mismatch
    #[error("Save file version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    /// Campaign not found
    #[error("Campaign '{campaign_id}' referenced in save file not found")]
    CampaignNotFound { campaign_id: String },

    /// Campaign version mismatch
    #[error(
        "Campaign version mismatch: save uses {save_version}, installed campaign is {current_version}"
    )]
    CampaignVersionMismatch {
        save_version: String,
        current_version: String,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Campaign reference stored in save games
///
/// This struct tracks which campaign a save game belongs to,
/// enabling validation when loading saves.
///
/// # Examples
///
/// ```
/// use antares::application::save_game::CampaignReference;
///
/// let campaign_ref = CampaignReference {
///     id: "tutorial".to_string(),
///     version: "1.0.0".to_string(),
///     name: "Tutorial Campaign".to_string(),
/// };
///
/// assert_eq!(campaign_ref.id, "tutorial");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CampaignReference {
    /// Campaign unique identifier
    pub id: String,

    /// Campaign version (for compatibility checking)
    pub version: String,

    /// Campaign display name
    pub name: String,
}

/// Save game structure
///
/// Contains all necessary information to restore a game session,
/// including campaign reference for campaign-based games.
///
/// # Examples
///
/// ```
/// use antares::application::save_game::SaveGame;
/// use antares::application::GameState;
///
/// let game_state = GameState::new();
/// let save = SaveGame::new(game_state);
///
/// assert_eq!(save.version, env!("CARGO_PKG_VERSION"));
/// assert!(save.campaign_reference.is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    /// Save format version (for backward compatibility)
    pub version: String,

    /// Timestamp when save was created
    pub timestamp: DateTime<Utc>,

    /// Campaign reference (if playing a campaign)
    pub campaign_reference: Option<CampaignReference>,

    /// The actual game state
    pub game_state: GameState,
}

impl SaveGame {
    /// Creates a new save game from current game state
    ///
    /// # Arguments
    ///
    /// * `game_state` - The game state to save
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::save_game::SaveGame;
    /// use antares::application::GameState;
    ///
    /// let game_state = GameState::new();
    /// let save = SaveGame::new(game_state);
    ///
    /// assert!(save.campaign_reference.is_none());
    /// ```
    pub fn new(game_state: GameState) -> Self {
        let campaign_reference = game_state
            .campaign
            .as_ref()
            .map(|campaign| CampaignReference {
                id: campaign.id.clone(),
                version: campaign.version.clone(),
                name: campaign.name.clone(),
            });

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
            campaign_reference,
            game_state,
        }
    }

    /// Validates save game version compatibility
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if version is compatible
    ///
    /// # Errors
    ///
    /// Returns `SaveGameError::VersionMismatch` if versions are incompatible
    pub fn validate_version(&self) -> Result<(), SaveGameError> {
        let current_version = env!("CARGO_PKG_VERSION");

        let save_ver = SemVer::parse(&self.version);
        let current_ver = SemVer::parse(current_version);

        match (save_ver, current_ver) {
            (Some(save), Some(current)) => {
                if !save.is_compatible_with(&current) {
                    return Err(SaveGameError::VersionMismatch {
                        expected: current_version.to_string(),
                        found: self.version.clone(),
                    });
                }
                // Compatible — log warnings for minor/patch differences
                if save.minor != current.minor {
                    tracing::warn!(
                        "Save game minor version differs: save={}, current={}. Loading with possible schema migration.",
                        self.version,
                        current_version
                    );
                } else if save.patch != current.patch {
                    tracing::info!(
                        "Save game patch version differs: save={}, current={}",
                        self.version,
                        current_version
                    );
                }
                Ok(())
            }
            _ => {
                // Unparseable version string — fall back to exact match
                if self.version != current_version {
                    return Err(SaveGameError::VersionMismatch {
                        expected: current_version.to_string(),
                        found: self.version.clone(),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Save game manager for file operations
///
/// Handles saving and loading game state to/from disk.
///
/// # Examples
///
/// ```no_run
/// use antares::application::save_game::SaveGameManager;
/// use antares::application::GameState;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let manager = SaveGameManager::new("saves")?;
/// let game_state = GameState::new();
///
/// // Save
/// manager.save("quicksave", &game_state)?;
///
/// // Load
/// let loaded = manager.load("quicksave")?;
/// # Ok(())
/// # }
/// ```
#[derive(Resource)]
pub struct SaveGameManager {
    /// Directory where save files are stored
    saves_dir: PathBuf,
}

impl SaveGameManager {
    /// Creates a new save game manager
    ///
    /// # Arguments
    ///
    /// * `saves_dir` - Directory path for save files
    ///
    /// # Returns
    ///
    /// Returns `Ok(SaveGameManager)` if directory is accessible
    ///
    /// # Errors
    ///
    /// Returns `SaveGameError::IoError` if directory cannot be created
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::save_game::SaveGameManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SaveGameManager::new("saves")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<P: AsRef<Path>>(saves_dir: P) -> Result<Self, SaveGameError> {
        let saves_dir = saves_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if !saves_dir.exists() {
            fs::create_dir_all(&saves_dir)?;
        }

        Ok(Self { saves_dir })
    }

    /// Saves game state to file
    ///
    /// # Arguments
    ///
    /// * `name` - Save file name (without extension)
    /// * `game_state` - The game state to save
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save succeeded
    ///
    /// # Errors
    ///
    /// Returns `SaveGameError::WriteError` if save fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::save_game::SaveGameManager;
    /// use antares::application::GameState;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SaveGameManager::new("saves")?;
    /// let game_state = GameState::new();
    /// manager.save("my_save", &game_state)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save(&self, name: &str, game_state: &GameState) -> Result<(), SaveGameError> {
        let save = SaveGame::new(game_state.clone());
        let path = self.save_path(name);

        let ron_string = ron::ser::to_string_pretty(&save, Default::default())
            .map_err(|e| SaveGameError::WriteError(e.to_string()))?;

        fs::write(&path, ron_string)
            .map_err(|e| SaveGameError::WriteError(format!("{}: {}", path.display(), e)))?;

        Ok(())
    }

    /// Loads game state from file
    ///
    /// # Arguments
    ///
    /// * `name` - Save file name (without extension)
    ///
    /// # Returns
    ///
    /// Returns `Ok(GameState)` if load succeeded
    ///
    /// # Errors
    ///
    /// Returns `SaveGameError` if load or parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::save_game::SaveGameManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SaveGameManager::new("saves")?;
    /// let game_state = manager.load("my_save")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load(&self, name: &str) -> Result<GameState, SaveGameError> {
        let path = self.save_path(name);

        let contents = fs::read_to_string(&path)
            .map_err(|e| SaveGameError::ReadError(format!("{}: {}", path.display(), e)))?;

        let save: SaveGame =
            ron::from_str(&contents).map_err(|e| SaveGameError::ParseError(e.to_string()))?;

        // Validate version
        save.validate_version()?;

        Ok(save.game_state)
    }

    /// Lists all available save files
    ///
    /// # Returns
    ///
    /// Returns list of save file names (without extensions)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::save_game::SaveGameManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SaveGameManager::new("saves")?;
    /// let saves = manager.list_saves()?;
    /// for save in saves {
    ///     println!("Save: {}", save);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_saves(&self) -> Result<Vec<String>, SaveGameError> {
        let mut saves = Vec::new();

        if !self.saves_dir.exists() {
            return Ok(saves);
        }

        for entry in fs::read_dir(&self.saves_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    saves.push(name.to_string());
                }
            }
        }

        saves.sort();
        Ok(saves)
    }

    /// Gets full path for a save file
    fn save_path(&self, name: &str) -> PathBuf {
        self.saves_dir.join(format!("{}.ron", name))
    }

    /// Deletes a save game file
    ///
    /// # Arguments
    ///
    /// * `name` - Save file name (without extension)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if deletion succeeded
    pub fn delete(&self, name: &str) -> Result<(), SaveGameError> {
        let path = self.save_path(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::CharacterLocation;
    use crate::domain::types::InnkeeperId;
    use crate::test_helpers::factories::test_character;
    use tempfile::TempDir;

    #[test]
    fn test_save_game_new() {
        let game_state = GameState::new();
        let save = SaveGame::new(game_state);

        assert_eq!(save.version, env!("CARGO_PKG_VERSION"));
        assert!(save.campaign_reference.is_none());
    }

    #[test]
    fn test_save_game_with_campaign() {
        use crate::domain::types::{Direction, Position};
        use crate::sdk::campaign_loader::{
            Campaign, CampaignAssets, CampaignConfig, CampaignData, Difficulty,
        };

        let campaign = Campaign {
            id: "test".to_string(),
            name: "Test Campaign".to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "Test Description".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: vec![],
            config: CampaignConfig {
                starting_map: 1,
                starting_position: Position { x: 0, y: 0 },
                starting_direction: Direction::North,
                starting_gold: 100,
                starting_food: 50,
                starting_innkeeper: "tutorial_innkeeper_town".to_string(),
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
                starting_time: crate::domain::types::GameTime::new(1, 8, 0),
            },
            data: CampaignData {
                items: "items.ron".to_string(),
                spells: "spells.ron".to_string(),
                monsters: "monsters.ron".to_string(),
                classes: "classes.ron".to_string(),
                races: "races.ron".to_string(),
                maps: "maps".to_string(),
                quests: "quests.ron".to_string(),
                dialogues: "dialogues.ron".to_string(),
                characters: "characters.ron".to_string(),
                creatures: "creatures.ron".to_string(),
                furniture: "data/furniture.ron".to_string(),
            },
            assets: CampaignAssets {
                tilesets: "tilesets".to_string(),
                music: "music".to_string(),
                sounds: "sounds".to_string(),
                images: "images".to_string(),
                fonts: "fonts".to_string(),
            },
            root_path: PathBuf::from("test_campaign"),
            game_config: crate::sdk::game_config::GameConfig::default(),
        };

        let mut game_state = GameState::new();
        game_state.campaign = Some(campaign);
        // Apply starting gold from campaign config.
        game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
        let save = SaveGame::new(game_state);

        assert!(save.campaign_reference.is_some());
        let campaign_ref = save.campaign_reference.unwrap();
        assert_eq!(campaign_ref.id, "test");
        assert_eq!(campaign_ref.version, "1.0.0");
        assert_eq!(campaign_ref.name, "Test Campaign");
    }

    #[test]
    fn test_save_game_validate_version() {
        let game_state = GameState::new();
        let save = SaveGame::new(game_state);

        assert!(save.validate_version().is_ok());
    }

    #[test]
    fn test_save_game_version_mismatch() {
        let game_state = GameState::new();
        let mut save = SaveGame::new(game_state);
        save.version = "9.0.0".to_string();

        assert!(matches!(
            save.validate_version(),
            Err(SaveGameError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn test_save_game_version_compatible_minor_diff() {
        // Same major, different minor — should be compatible
        let game_state = GameState::new();
        let mut save = SaveGame::new(game_state);
        // Set version to same major but different minor
        let current = env!("CARGO_PKG_VERSION");
        let current_ver = current.split('.').collect::<Vec<_>>();
        let same_major_diff_minor = format!(
            "{}.{}.{}",
            current_ver[0],
            current_ver[1].parse::<u32>().unwrap() + 1,
            current_ver[2]
        );
        save.version = same_major_diff_minor;

        // Should succeed (same major version)
        assert!(save.validate_version().is_ok());
    }

    #[test]
    fn test_save_game_version_incompatible_major_diff() {
        // Different major version — should be incompatible
        let game_state = GameState::new();
        let mut save = SaveGame::new(game_state);
        save.version = "99.0.0".to_string();

        let result = save.validate_version();
        assert!(result.is_err());
        match result.unwrap_err() {
            SaveGameError::VersionMismatch { expected, found } => {
                assert_eq!(found, "99.0.0");
                assert_eq!(expected, env!("CARGO_PKG_VERSION"));
            }
            other => panic!("Expected VersionMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn test_save_game_version_compatible_patch_diff() {
        // Same major and minor, different patch — should be compatible
        let game_state = GameState::new();
        let mut save = SaveGame::new(game_state);
        let current = env!("CARGO_PKG_VERSION");
        let current_ver = current.split('.').collect::<Vec<_>>();
        let same_major_minor_diff_patch = format!(
            "{}.{}.{}",
            current_ver[0],
            current_ver[1],
            current_ver[2].parse::<u32>().unwrap() + 5
        );
        save.version = same_major_minor_diff_patch;

        assert!(save.validate_version().is_ok());
    }

    #[test]
    fn test_save_game_version_unparseable_fallback() {
        // Unparseable version string — falls back to exact match
        let game_state = GameState::new();
        let mut save = SaveGame::new(game_state);
        save.version = "not-a-version".to_string();

        let result = save.validate_version();
        assert!(result.is_err());
    }

    #[test]
    fn test_save_game_manager_new() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        assert!(temp_dir.path().exists());
        assert_eq!(manager.saves_dir, temp_dir.path());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let game_state = GameState::new();
        manager.save("test_save", &game_state).unwrap();

        let loaded_state = manager.load("test_save").unwrap();

        assert!(matches!(
            loaded_state.mode,
            crate::application::GameMode::Exploration
        ));
        assert!(matches!(
            game_state.mode,
            crate::application::GameMode::Exploration
        ));
        assert_eq!(loaded_state.time.day, game_state.time.day);
    }

    #[test]
    fn test_list_saves() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let game_state = GameState::new();
        manager.save("save1", &game_state).unwrap();
        manager.save("save2", &game_state).unwrap();
        manager.save("save3", &game_state).unwrap();

        let saves = manager.list_saves().unwrap();

        assert_eq!(saves.len(), 3);
        assert!(saves.contains(&"save1".to_string()));
        assert!(saves.contains(&"save2".to_string()));
        assert!(saves.contains(&"save3".to_string()));
    }

    #[test]
    fn test_list_saves_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let saves = manager.list_saves().unwrap();
        assert_eq!(saves.len(), 0);
    }

    #[test]
    fn test_save_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let path = manager.save_path("test");
        assert_eq!(path, temp_dir.path().join("test.ron"));
    }

    #[test]
    fn test_campaign_reference_creation() {
        let campaign_ref = CampaignReference {
            id: "tutorial".to_string(),
            version: "1.0.0".to_string(),
            name: "Tutorial Campaign".to_string(),
        };

        assert_eq!(campaign_ref.id, "tutorial");
        assert_eq!(campaign_ref.version, "1.0.0");
        assert_eq!(campaign_ref.name, "Tutorial Campaign");
    }

    // ===== Party Management Persistence Tests =====

    #[test]
    fn test_save_party_locations() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Add 3 characters to party
        let char1 = test_character("Knight");
        let char2 = test_character("Archer");
        let char3 = test_character("Cleric");

        game_state
            .roster
            .add_character(char1, CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(char2, CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(char3, CharacterLocation::InParty)
            .unwrap();

        // Add to party
        game_state.party.members = vec![
            game_state.roster.characters[0].clone(),
            game_state.roster.characters[1].clone(),
            game_state.roster.characters[2].clone(),
        ];

        // Save and load
        manager.save("party_test", &game_state).unwrap();
        let loaded_state = manager.load("party_test").unwrap();

        // Verify party locations preserved
        assert_eq!(loaded_state.roster.character_locations.len(), 3);
        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::InParty
        );

        // Verify party members preserved
        assert_eq!(loaded_state.party.members.len(), 3);
        assert_eq!(loaded_state.party.members[0].name, "Knight");
        assert_eq!(loaded_state.party.members[1].name, "Archer");
        assert_eq!(loaded_state.party.members[2].name, "Cleric");
    }

    #[test]
    fn test_save_inn_locations() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Add characters at different inns
        let char1 = test_character("InnChar1");
        let char2 = test_character("InnChar2");
        let char3 = test_character("InnChar3");

        let inn1: InnkeeperId = "tutorial_innkeeper_town".to_string();
        let inn2: InnkeeperId = "tutorial_innkeeper_town2".to_string();
        let inn3: InnkeeperId = "tutorial_innkeeper_town3".to_string();

        game_state
            .roster
            .add_character(char1, CharacterLocation::AtInn(inn1))
            .unwrap();
        game_state
            .roster
            .add_character(char2, CharacterLocation::AtInn(inn2))
            .unwrap();
        game_state
            .roster
            .add_character(char3, CharacterLocation::AtInn(inn3))
            .unwrap();

        // Save and load
        manager.save("inn_test", &game_state).unwrap();
        let loaded_state = manager.load("inn_test").unwrap();

        // Verify inn locations preserved
        assert_eq!(loaded_state.roster.character_locations.len(), 3);
        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::AtInn("tutorial_innkeeper_town2".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::AtInn("tutorial_innkeeper_town3".to_string())
        );

        // Verify characters preserved
        assert_eq!(loaded_state.roster.characters.len(), 3);
        assert_eq!(loaded_state.roster.characters[0].name, "InnChar1");
        assert_eq!(loaded_state.roster.characters[1].name, "InnChar2");
        assert_eq!(loaded_state.roster.characters[2].name, "InnChar3");
    }

    #[test]
    fn test_save_game_format() {
        let temp_dir = TempDir::new().unwrap();
        let _manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Add a single character located at an inn so the save contains AtInn(...)
        let char1 = test_character("FormatChar");
        let inn = "tutorial_innkeeper_town".to_string();
        game_state
            .roster
            .add_character(char1, CharacterLocation::AtInn(inn.clone()))
            .unwrap();

        // Build a SaveGame and serialize it to RON
        let save = SaveGame::new(game_state);
        let ron_str = ron::ser::to_string_pretty(&save, Default::default())
            .expect("Failed to serialize save");

        // Verify the RON contains expected fields and the innkeeper ID
        assert!(ron_str.contains("version"));
        assert!(ron_str.contains("timestamp"));
        assert!(ron_str.contains("game_state"));
        assert!(ron_str.contains(&inn));
        assert!(ron_str.contains("AtInn"));
    }

    #[test]
    fn test_save_encountered_characters() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Mark some characters as encountered
        game_state
            .encountered_characters
            .insert("npc_gareth".to_string());
        game_state
            .encountered_characters
            .insert("npc_whisper".to_string());
        game_state
            .encountered_characters
            .insert("npc_zara".to_string());

        // Save and load
        manager.save("encounter_test", &game_state).unwrap();
        let loaded_state = manager.load("encounter_test").unwrap();

        // Verify encounter tracking preserved
        assert_eq!(loaded_state.encountered_characters.len(), 3);
        assert!(loaded_state.encountered_characters.contains("npc_gareth"));
        assert!(loaded_state.encountered_characters.contains("npc_whisper"));
        assert!(loaded_state.encountered_characters.contains("npc_zara"));
    }

    #[test]
    fn test_save_migration_from_old_format() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Create a save without encountered_characters (simulating old format)
        let mut game_state = GameState::new();
        game_state
            .roster
            .add_character(test_character("OldChar"), CharacterLocation::InParty)
            .unwrap();

        // Save normally
        manager.save("migration_test", &game_state).unwrap();

        // Load the save file as RON string
        let save_path = manager.save_path("migration_test");
        let mut ron_content = std::fs::read_to_string(&save_path).unwrap();

        // Remove encountered_characters field to simulate old format.
        // This simulates a save file created before encountered_characters existed.
        //
        // encountered_characters serializes as a set literal: `{}` (empty) or
        // `{"val1", "val2"}`. The field always ends with a `,` because other
        // serde(default) fields (npc_runtime, etc.) follow it.
        // We match the whole line, including the trailing newline, so we don't
        // accidentally clip into adjacent fields.
        let field_marker = "encountered_characters:";
        if let Some(start) = ron_content.find(field_marker) {
            // Find the newline that terminates this field's line.
            // The field value is always on a single line for HashSet<String>.
            if let Some(nl) = ron_content[start..].find('\n') {
                let full_end = start + nl + 1; // include the newline
                ron_content.replace_range(start..full_end, "");
            }
        }

        // Write back the modified save (old format)
        std::fs::write(&save_path, &ron_content).unwrap();

        // Load should succeed with default empty set
        let loaded_state = manager.load("migration_test").unwrap();

        // Verify encountered_characters defaults to empty
        assert_eq!(loaded_state.encountered_characters.len(), 0);

        // Verify other state preserved
        assert_eq!(loaded_state.roster.characters.len(), 1);
        assert_eq!(loaded_state.roster.characters[0].name, "OldChar");
    }

    #[test]
    fn test_save_recruited_character() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Simulate recruiting a character
        let recruited = test_character("RecruitedNPC");
        game_state
            .roster
            .add_character(recruited, CharacterLocation::InParty)
            .unwrap();
        game_state
            .party
            .members
            .push(game_state.roster.characters[0].clone());

        // Mark as encountered
        game_state
            .encountered_characters
            .insert("npc_recruited".to_string());

        // Save and load
        manager.save("recruit_test", &game_state).unwrap();
        let loaded_state = manager.load("recruit_test").unwrap();

        // Verify recruited character state
        assert_eq!(loaded_state.roster.characters.len(), 1);
        assert_eq!(loaded_state.roster.characters[0].name, "RecruitedNPC");
        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        );
        assert_eq!(loaded_state.party.members.len(), 1);
        assert_eq!(loaded_state.party.members[0].name, "RecruitedNPC");

        // Verify encounter tracking
        assert!(loaded_state
            .encountered_characters
            .contains("npc_recruited"));
    }

    #[test]
    fn test_save_full_roster_state() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Mix of party members, inn characters, and map characters
        game_state
            .roster
            .add_character(test_character("PartyMember1"), CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(test_character("PartyMember2"), CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(
                test_character("InnChar1"),
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();
        game_state
            .roster
            .add_character(
                test_character("InnChar2"),
                CharacterLocation::AtInn("tutorial_innkeeper_town2".to_string()),
            )
            .unwrap();
        game_state
            .roster
            .add_character(test_character("MapChar"), CharacterLocation::OnMap(5))
            .unwrap();

        // Add party members to party
        game_state
            .party
            .members
            .push(game_state.roster.characters[0].clone());
        game_state
            .party
            .members
            .push(game_state.roster.characters[1].clone());

        // Some encountered characters
        game_state
            .encountered_characters
            .insert("npc_1".to_string());
        game_state
            .encountered_characters
            .insert("npc_2".to_string());

        // Save and load
        manager.save("full_roster_test", &game_state).unwrap();
        let loaded_state = manager.load("full_roster_test").unwrap();

        // Verify all locations preserved
        assert_eq!(loaded_state.roster.character_locations.len(), 5);
        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[3],
            CharacterLocation::AtInn("tutorial_innkeeper_town2".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[4],
            CharacterLocation::OnMap(5)
        );

        // Verify party state
        assert_eq!(loaded_state.party.members.len(), 2);
        assert_eq!(loaded_state.party.members[0].name, "PartyMember1");
        assert_eq!(loaded_state.party.members[1].name, "PartyMember2");

        // Verify encounters
        assert_eq!(loaded_state.encountered_characters.len(), 2);
        assert!(loaded_state.encountered_characters.contains("npc_1"));
        assert!(loaded_state.encountered_characters.contains("npc_2"));
    }

    #[test]
    fn test_save_load_preserves_character_invariants() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Add characters with specific locations
        for i in 0..5 {
            let char_name = format!("Char{}", i);
            let location = if i < 2 {
                CharacterLocation::InParty
            } else if i < 4 {
                CharacterLocation::AtInn(format!("tutorial_innkeeper_{}", i - 1))
            } else {
                CharacterLocation::OnMap((i + 10) as u16)
            };

            game_state
                .roster
                .add_character(test_character(&char_name), location)
                .unwrap();
        }

        // Add party members
        game_state
            .party
            .members
            .push(game_state.roster.characters[0].clone());
        game_state
            .party
            .members
            .push(game_state.roster.characters[1].clone());

        // Save and load
        manager.save("invariant_test", &game_state).unwrap();
        let loaded_state = manager.load("invariant_test").unwrap();

        // Verify roster/location vector lengths match
        assert_eq!(
            loaded_state.roster.characters.len(),
            loaded_state.roster.character_locations.len()
        );

        // Verify no data corruption
        assert_eq!(loaded_state.roster.characters.len(), 5);
        assert_eq!(loaded_state.party.members.len(), 2);

        // Verify location types preserved
        assert!(matches!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        ));
        assert!(matches!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::AtInn(_)
        ));
        assert!(matches!(
            loaded_state.roster.character_locations[4],
            CharacterLocation::OnMap(_)
        ));
    }

    #[test]
    fn test_save_empty_encountered_characters() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let game_state = GameState::new();

        // Save with no encounters
        manager.save("empty_encounter_test", &game_state).unwrap();
        let loaded_state = manager.load("empty_encounter_test").unwrap();

        // Verify empty set preserved
        assert_eq!(loaded_state.encountered_characters.len(), 0);
    }

    #[test]
    fn test_save_multiple_party_changes() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Initial state: 2 in party, 2 at inn
        for i in 0..4 {
            let location = if i < 2 {
                CharacterLocation::InParty
            } else {
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
            };
            game_state
                .roster
                .add_character(test_character(&format!("Char{}", i)), location)
                .unwrap();
        }

        game_state
            .party
            .members
            .push(game_state.roster.characters[0].clone());
        game_state
            .party
            .members
            .push(game_state.roster.characters[1].clone());

        // Save initial state
        manager.save("multi_change_test", &game_state).unwrap();

        // Simulate swap: move char[1] to inn, char[2] to party
        game_state.roster.character_locations[1] =
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string());
        game_state.roster.character_locations[2] = CharacterLocation::InParty;
        game_state.party.members[1] = game_state.roster.characters[2].clone();

        // Save changed state
        manager.save("multi_change_test", &game_state).unwrap();

        // Load and verify swapped state
        let loaded_state = manager.load("multi_change_test").unwrap();

        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::InParty
        );
        assert_eq!(loaded_state.party.members[1].name, "Char2");
    }

    // ===== NPC Runtime Persistence Tests =====

    /// Helper that builds a `GameState` pre-populated with one merchant NPC's
    /// runtime stock so that save/load tests can verify round-trip fidelity.
    fn make_game_state_with_merchant_runtime() -> GameState {
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let mut state = GameState::new();

        // Manually construct the runtime state that would normally be seeded from
        // a template.  We do it by hand here so the test has no dependency on
        // the tutorial campaign files being present.
        let stock = MerchantStock {
            entries: vec![
                StockEntry {
                    item_id: 10,
                    quantity: 3,
                    override_price: None,
                },
                StockEntry {
                    item_id: 20,
                    quantity: 7,
                    override_price: Some(150),
                },
            ],
            restock_template: Some("weapons_basic".to_string()),
        };

        let mut runtime = NpcRuntimeState::new("merchant_alice".to_string());
        runtime.stock = Some(stock);
        runtime.services_consumed.push("heal_all".to_string());

        state.npc_runtime.insert(runtime);
        state
    }

    #[test]
    fn test_save_load_preserves_npc_runtime_stock() {
        // Arrange: game state with a merchant that has been partially bought from
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = make_game_state_with_merchant_runtime();

        // Simulate a buy: decrement item 10 from 3 → 1
        {
            let runtime = state
                .npc_runtime
                .get_mut(&"merchant_alice".to_string())
                .unwrap();
            runtime
                .stock
                .as_mut()
                .unwrap()
                .get_entry_mut(10)
                .unwrap()
                .quantity = 1;
        }

        // Act: save then load
        manager.save("npc_runtime_test", &state).unwrap();
        let loaded = manager.load("npc_runtime_test").unwrap();

        // Assert: the decremented quantity is preserved (1, not original 3)
        let runtime = loaded
            .npc_runtime
            .get(&"merchant_alice".to_string())
            .expect("merchant_alice runtime state should be present after load");

        assert!(
            runtime.stock.is_some(),
            "merchant stock should survive the round-trip"
        );
        let stock = runtime.stock.as_ref().unwrap();

        assert_eq!(
            stock.get_entry(10).unwrap().quantity,
            1,
            "decremented quantity must be preserved through save/load"
        );
        assert_eq!(
            stock.get_entry(20).unwrap().quantity,
            7,
            "untouched quantity must be preserved through save/load"
        );
        assert_eq!(
            stock.get_entry(20).unwrap().override_price,
            Some(150),
            "override_price must be preserved through save/load"
        );
        assert_eq!(
            stock.restock_template,
            Some("weapons_basic".to_string()),
            "restock_template must be preserved through save/load"
        );

        // Services consumed should also round-trip
        assert_eq!(
            runtime.services_consumed,
            vec!["heal_all".to_string()],
            "services_consumed must be preserved through save/load"
        );
    }

    #[test]
    fn test_save_load_legacy_format_empty_npc_runtime() {
        // Arrange: produce a save file, then strip the npc_runtime field to
        // simulate a save file created before npc_runtime was implemented.
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();
        // Add a character so the save has recognisable non-default content
        state
            .roster
            .add_character(
                test_character("LegacyChar"),
                crate::domain::character::CharacterLocation::InParty,
            )
            .unwrap();

        // Write a normal save first
        manager.save("legacy_test", &state).unwrap();

        // Read the raw RON, remove the npc_runtime field to simulate old format.
        // The field serialises as a single line:
        //   npc_runtime: (npcs: {}),
        // We strip the entire line so the save parser falls back to Default.
        let save_path = manager.save_path("legacy_test");
        let ron_content = std::fs::read_to_string(&save_path).unwrap();

        // Strip the npc_runtime field from the RON.  Because ron pretty-print
        // renders the NpcRuntimeStore as a multi-line struct:
        //
        //   npc_runtime: (
        //       npcs: {},
        //   ),
        //
        // we cannot simply remove the first line; we must remove everything
        // from "npc_runtime:" up to and including the closing ")," line.
        //
        // Strategy: find the marker line, then scan forward counting
        // un-matched opening parens until they all close, then consume
        // the trailing comma and newline.
        let field_marker = "npc_runtime:";
        let stripped = if let Some(start) = ron_content.find(field_marker) {
            // Find the end of the field value by counting paren depth.
            // We start scanning from the character after the marker.
            let after_marker = start + field_marker.len();
            let tail = &ron_content[after_marker..];

            let mut depth: i32 = 0;
            let mut found_open = false;
            let mut field_end = after_marker; // position just past the closing ")"

            for (offset, ch) in tail.char_indices() {
                match ch {
                    '(' => {
                        depth += 1;
                        found_open = true;
                    }
                    ')' => {
                        depth -= 1;
                        if found_open && depth == 0 {
                            // offset points at ')'; advance past it
                            field_end = after_marker + offset + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            // Consume optional trailing comma and newline after the closing ")"
            let rest = &ron_content[field_end..];
            let extra = rest
                .chars()
                .take_while(|c| *c == ',' || *c == '\n' || *c == '\r')
                .map(|c| c.len_utf8())
                .sum::<usize>();
            let end = field_end + extra;

            format!("{}{}", &ron_content[..start], &ron_content[end..])
        } else {
            // Field not found – the file already lacks it; no change needed
            ron_content.clone()
        };

        std::fs::write(&save_path, &stripped).unwrap();

        // Act: load the legacy-format save – must succeed without error
        let loaded = manager
            .load("legacy_test")
            .expect("loading a legacy save without npc_runtime field must succeed");

        // Assert: npc_runtime defaults to empty store
        assert!(
            loaded.npc_runtime.is_empty(),
            "npc_runtime should default to empty when the field is absent in the save file"
        );

        // Assert: other state is preserved correctly
        assert_eq!(
            loaded.roster.characters.len(),
            1,
            "roster should be preserved from legacy save"
        );
        assert_eq!(
            loaded.roster.characters[0].name, "LegacyChar",
            "character name should be preserved from legacy save"
        );
    }

    // ===== Buy and Sell: Tutorial Data Wiring, Save Persistence =====

    /// Verifies that buying an item from a merchant reduces the stock count and
    /// that the reduction persists across a save/load cycle.
    ///
    /// Create a GameState with a merchant having
    /// 3 units of item 1, buy 1 unit (stock → 2), serialise, deserialise, and
    /// assert the loaded state shows 2 units.
    #[test]
    fn test_save_load_preserves_merchant_stock_after_buy() {
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // 1. Create a GameState with a merchant that has 3 units of item 1.
        let mut state = GameState::new();
        let stock = MerchantStock {
            entries: vec![StockEntry {
                item_id: 1,
                quantity: 3,
                override_price: None,
            }],
            restock_template: Some("tutorial_merchant_stock".to_string()),
        };
        let mut runtime = NpcRuntimeState::new("tutorial_merchant_town".to_string());
        runtime.stock = Some(stock);
        state.npc_runtime.insert(runtime);

        // 2. Buy 1 unit — reduces stock from 3 → 2.
        {
            let runtime = state
                .npc_runtime
                .get_mut(&"tutorial_merchant_town".to_string())
                .unwrap();
            runtime
                .stock
                .as_mut()
                .unwrap()
                .get_entry_mut(1)
                .unwrap()
                .quantity = 2;
        }

        // 3. Serialise to RON with save_game.
        manager.save("buy_sell_test", &state).unwrap();

        // 4. Deserialise with load_game.
        let loaded = manager.load("buy_sell_test").unwrap();

        // 5. Assert the loaded state has 2 units of item 1 in the merchant stock.
        let loaded_runtime = loaded
            .npc_runtime
            .get(&"tutorial_merchant_town".to_string())
            .expect("tutorial_merchant_town runtime must survive the round-trip");

        assert!(
            loaded_runtime.stock.is_some(),
            "merchant stock must be present after round-trip"
        );
        let loaded_stock = loaded_runtime.stock.as_ref().unwrap();
        assert_eq!(
            loaded_stock.get_entry(1).unwrap().quantity,
            2,
            "stock quantity after buy (3→2) must be preserved through save/load"
        );
        assert_eq!(
            loaded_stock.restock_template,
            Some("tutorial_merchant_stock".to_string()),
            "restock_template must be preserved through save/load"
        );
    }

    /// Verifies that a partial container take (removing some but not all items)
    /// survives a save/load cycle within the same map.
    ///
    /// Container state is stored in `MapEvent::Container { items, .. }` which is
    /// serialised as part of `World` inside `GameState`. This test confirms the
    /// write-back path is correctly round-tripped.
    #[test]
    fn test_save_load_preserves_container_items_after_partial_take() {
        use crate::domain::character::InventorySlot;
        use crate::domain::types::Position;
        use crate::domain::world::MapEvent;
        use crate::domain::world::{Map, World};

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Build a world with one map containing a container event that starts
        // with 3 items (item_ids 10, 20, 30).
        let mut state = GameState::new();
        let mut map = Map::new(
            1,
            "Test Map".to_string(),
            "A map with a container".to_string(),
            10,
            10,
        );
        let container_pos = Position::new(5, 5);
        map.add_event(
            container_pos,
            MapEvent::Container {
                id: "chest_room1".to_string(),
                name: "Old Chest".to_string(),
                description: "A battered chest".to_string(),
                items: vec![
                    InventorySlot {
                        item_id: 10,
                        charges: 0,
                    },
                    InventorySlot {
                        item_id: 20,
                        charges: 0,
                    },
                    InventorySlot {
                        item_id: 30,
                        charges: 0,
                    },
                ],
            },
        );
        let mut world = World::new();
        world.add_map(map);
        world.set_current_map(1);
        state.world = world;

        // Simulate a partial take: player takes item_id 20, leaving 10 and 30.
        {
            let map_mut = state.world.get_current_map_mut().unwrap();
            if let Some(MapEvent::Container { items, .. }) = map_mut.events.get_mut(&container_pos)
            {
                items.retain(|slot| slot.item_id != 20);
            }
        }

        // Save then load.
        manager.save("container_test", &state).unwrap();
        let loaded = manager.load("container_test").unwrap();

        // Verify the container on the loaded map has exactly items 10 and 30.
        let loaded_map = loaded
            .world
            .get_map(1)
            .expect("map 1 must be present after round-trip");
        let loaded_event = loaded_map
            .get_event(container_pos)
            .expect("container event must be present after round-trip");

        match loaded_event {
            MapEvent::Container { items, id, .. } => {
                assert_eq!(id, "chest_room1", "container id must be preserved");
                assert_eq!(
                    items.len(),
                    2,
                    "container must have 2 items after partial take and round-trip"
                );
                assert!(
                    items.iter().any(|s| s.item_id == 10),
                    "item_id 10 must still be in the container"
                );
                assert!(
                    items.iter().any(|s| s.item_id == 30),
                    "item_id 30 must still be in the container"
                );
                assert!(
                    !items.iter().any(|s| s.item_id == 20),
                    "taken item_id 20 must NOT be in the container after round-trip"
                );
            }
            other => panic!("expected Container event, got {:?}", other),
        }
    }
    /// Verifies that `last_restock_day`, `magic_slots`, and `last_magic_refresh_day`
    /// survive a full save/load cycle.
    ///
    /// These fields are serialised as part of `NpcRuntimeState` inside
    /// `GameState::npc_runtime`.  This test ensures they are not silently dropped
    /// or reset to their default sentinel values during round-trip serialisation.
    #[test]
    fn test_save_load_preserves_restock_day_and_magic_slots() {
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Build a runtime state that has been through a restock cycle:
        //   - last_restock_day = 3  (restocked on day 3)
        //   - magic_slots = [101, 102]  (two magic items currently in stock)
        //   - last_magic_refresh_day = 1  (magic slots refreshed on day 1)
        let stock = MerchantStock {
            entries: vec![
                StockEntry {
                    item_id: 10,
                    quantity: 2,
                    override_price: None,
                },
                // Magic-slot entries mirroring magic_slots.
                StockEntry {
                    item_id: 101,
                    quantity: 1,
                    override_price: None,
                },
                StockEntry {
                    item_id: 102,
                    quantity: 1,
                    override_price: None,
                },
            ],
            restock_template: Some("tutorial_merchant_stock".to_string()),
        };

        let mut runtime = NpcRuntimeState::new("merchant_restock".to_string());
        runtime.stock = Some(stock);
        runtime.last_restock_day = 3;
        runtime.magic_slots = vec![101, 102];
        runtime.last_magic_refresh_day = 1;

        state.npc_runtime.insert(runtime);

        // Save and reload.
        manager
            .save("restock_roundtrip", &state)
            .expect("save must succeed");
        let loaded = manager
            .load("restock_roundtrip")
            .expect("load must succeed");

        let loaded_runtime = loaded
            .npc_runtime
            .get(&"merchant_restock".to_string())
            .expect("merchant_restock must be present after round-trip");

        assert_eq!(
            loaded_runtime.last_restock_day, 3,
            "last_restock_day must survive save/load"
        );
        assert_eq!(
            loaded_runtime.magic_slots,
            vec![101, 102],
            "magic_slots must survive save/load"
        );
        assert_eq!(
            loaded_runtime.last_magic_refresh_day, 1,
            "last_magic_refresh_day must survive save/load"
        );

        // Verify the magic-slot stock entries are also intact.
        let loaded_stock = loaded_runtime
            .stock
            .as_ref()
            .expect("stock must be Some after round-trip");
        assert_eq!(
            loaded_stock
                .get_entry(101)
                .expect("item 101 must exist")
                .quantity,
            1
        );
        assert_eq!(
            loaded_stock
                .get_entry(102)
                .expect("item 102 must exist")
                .quantity,
            1
        );
        assert_eq!(
            loaded_stock
                .restock_template
                .as_deref()
                .expect("restock_template must be Some"),
            "tutorial_merchant_stock"
        );
    }

    // ===== Lock State Persistence Tests =====

    /// Verifies that unlocking a door in one session and saving/loading
    /// leaves the door open in the restored session.
    ///
    /// Steps:
    /// 1. Build a `Map` with a `LockedDoor` event.
    /// 2. Call `init_lock_states()` to seed the runtime lock state.
    /// 3. Unlock the door and clear the door tile (simulating `apply_success`).
    /// 4. Save → Load.
    /// 5. Assert `lock_states["test_door"].is_locked == false`.
    /// 6. Assert the door tile has `wall_type == WallType::None`.
    #[test]
    fn test_save_load_preserves_unlocked_door_state() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, WallType};

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let lock_id = "test_door";
        let door_pos = Position::new(3, 3);

        let mut state = GameState::new();

        // Build a 10×10 map with a LockedDoor event at (3, 3).
        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            door_pos,
            MapEvent::LockedDoor {
                name: "Test Door".to_string(),
                lock_id: lock_id.to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );

        // Place a door tile so apply_success has a tile to clear.
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::Door;
            tile.blocked = true;
        }

        // Seed the runtime lock state from the map events.
        map.init_lock_states();

        // Simulate apply_success: unlock the lock state and clear the tile.
        if let Some(ls) = map.lock_states.get_mut(lock_id) {
            ls.unlock();
        }
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::None;
            tile.blocked = false;
        }

        state.world.add_map(map);
        state.world.set_current_map(1);

        // Save → Load.
        manager
            .save("lock_state_roundtrip", &state)
            .expect("save must succeed");
        let loaded = manager
            .load("lock_state_roundtrip")
            .expect("load must succeed");

        let loaded_map = loaded
            .world
            .get_current_map()
            .expect("current map must be present after load");

        // 1. Lock state must still be unlocked.
        let loaded_lock = loaded_map
            .lock_states
            .get(lock_id)
            .expect("lock_states entry must survive save/load");
        assert!(
            !loaded_lock.is_locked,
            "unlock state must survive save/load; expected is_locked=false"
        );

        // 2. Door tile must still be open.
        let tile = loaded_map
            .get_tile(door_pos)
            .expect("door tile must exist after load");
        assert_eq!(
            tile.wall_type,
            WallType::None,
            "door tile wall_type must be WallType::None after save/load"
        );
        assert!(
            !tile.blocked,
            "door tile must not be blocked after save/load"
        );
    }

    /// Verifies that a non-zero `trap_chance` on a `LockState` survives a
    /// full save/load round-trip without being reset to zero.
    #[test]
    fn test_save_load_preserves_trap_chance() {
        use crate::domain::types::Position;
        use crate::domain::world::lock::LockState;
        use crate::domain::world::{Map, MapEvent};

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let lock_id = "trap_chance_door";
        let door_pos = Position::new(5, 5);

        let mut state = GameState::new();

        let mut map = Map::new(1, "Trap Map".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            door_pos,
            MapEvent::LockedDoor {
                name: "Trap Door".to_string(),
                lock_id: lock_id.to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );

        // Manually insert a LockState with a non-zero trap_chance (simulating
        // one failed lockpick attempt).
        let mut ls = LockState::new(lock_id);
        ls.trap_chance = 30;
        map.lock_states.insert(lock_id.to_string(), ls);

        state.world.add_map(map);
        state.world.set_current_map(1);

        // Save → Load.
        manager
            .save("trap_chance_roundtrip", &state)
            .expect("save must succeed");
        let loaded = manager
            .load("trap_chance_roundtrip")
            .expect("load must succeed");

        let loaded_map = loaded
            .world
            .get_current_map()
            .expect("current map must be present after load");

        let loaded_lock = loaded_map
            .lock_states
            .get(lock_id)
            .expect("lock_states entry must survive save/load");

        assert_eq!(
            loaded_lock.trap_chance, 30,
            "trap_chance must survive save/load without being reset"
        );
        assert!(
            loaded_lock.is_locked,
            "door must still be locked (trap_chance test does not unlock)"
        );
    }
}
