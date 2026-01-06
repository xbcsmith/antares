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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

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

        // For now, exact version match required
        // TODO: Implement semantic version compatibility checking
        if self.version != current_version {
            return Err(SaveGameError::VersionMismatch {
                expected: current_version.to_string(),
                found: self.version.clone(),
            });
        }

        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Character, CharacterLocation};
    use crate::domain::types::InnkeeperId;
    use tempfile::TempDir;

    // Helper to create a test character
    fn create_test_character(name: &str) -> Character {
        use crate::domain::character::{Alignment, Sex};
        Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

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
            },
            assets: CampaignAssets {
                tilesets: "tilesets".to_string(),
                music: "music".to_string(),
                sounds: "sounds".to_string(),
                images: "images".to_string(),
            },
            root_path: PathBuf::from("test_campaign"),
            game_config: crate::sdk::game_config::GameConfig::default(),
        };

        let mut game_state = GameState::new();
        game_state.campaign = Some(campaign);
        // Mirror prior `new_game` behavior: apply starting resources from campaign config
        game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
        game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;
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
        save.version = "99.99.99".to_string();

        assert!(matches!(
            save.validate_version(),
            Err(SaveGameError::VersionMismatch { .. })
        ));
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

    // ===== Phase 5: Party Management Persistence Tests =====

    #[test]
    fn test_save_party_locations() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut game_state = GameState::new();

        // Add 3 characters to party
        let char1 = create_test_character("Knight");
        let char2 = create_test_character("Archer");
        let char3 = create_test_character("Cleric");

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
        let char1 = create_test_character("InnChar1");
        let char2 = create_test_character("InnChar2");
        let char3 = create_test_character("InnChar3");

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
        let char1 = create_test_character("FormatChar");
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
            .add_character(create_test_character("OldChar"), CharacterLocation::InParty)
            .unwrap();

        // Save normally
        manager.save("migration_test", &game_state).unwrap();

        // Load the save file as RON string
        let save_path = manager.save_path("migration_test");
        let mut ron_content = std::fs::read_to_string(&save_path).unwrap();

        // Remove encountered_characters field to simulate old format
        // This simulates a save file created before encountered_characters existed
        if let Some(start) = ron_content.find("encountered_characters:") {
            if let Some(end) = ron_content[start..].find("},") {
                let full_end = start + end + 2;
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
        let recruited = create_test_character("RecruitedNPC");
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
            .add_character(
                create_test_character("PartyMember1"),
                CharacterLocation::InParty,
            )
            .unwrap();
        game_state
            .roster
            .add_character(
                create_test_character("PartyMember2"),
                CharacterLocation::InParty,
            )
            .unwrap();
        game_state
            .roster
            .add_character(
                create_test_character("InnChar1"),
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();
        game_state
            .roster
            .add_character(
                create_test_character("InnChar2"),
                CharacterLocation::AtInn("tutorial_innkeeper_town2".to_string()),
            )
            .unwrap();
        game_state
            .roster
            .add_character(
                create_test_character("MapChar"),
                CharacterLocation::OnMap(5),
            )
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
                .add_character(create_test_character(&char_name), location)
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
                .add_character(create_test_character(&format!("Char{}", i)), location)
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
}
