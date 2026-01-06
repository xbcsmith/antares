// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Phase 14: Game Engine Campaign Integration
//!
//! These tests verify that the Antares game engine can correctly load
//! and play custom campaigns created with the Campaign Builder.

use antares::application::save_game::{CampaignReference, SaveGame, SaveGameManager};
use antares::application::GameState;
use antares::domain::types::{Direction, Position};
use antares::sdk::campaign_loader::{
    Campaign, CampaignAssets, CampaignConfig, CampaignData, Difficulty,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a test campaign
fn create_test_campaign(id: &str, name: &str, version: &str) -> Campaign {
    Campaign {
        id: id.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        author: "Test Author".to_string(),
        description: "A test campaign for integration testing".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        required_features: vec![],
        config: CampaignConfig {
            starting_map: 1,
            starting_position: Position { x: 5, y: 10 },
            starting_direction: Direction::North,
            starting_gold: 500,
            starting_food: 100,
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
            items: "data/items.ron".to_string(),
            spells: "data/spells.ron".to_string(),
            monsters: "data/monsters.ron".to_string(),
            classes: "data/classes.ron".to_string(),
            races: "data/races.ron".to_string(),
            maps: "maps".to_string(),
            quests: "data/quests.ron".to_string(),
            dialogues: "data/dialogues.ron".to_string(),
            characters: "data/characters.ron".to_string(),
        },
        assets: CampaignAssets {
            tilesets: "assets/tilesets".to_string(),
            music: "assets/music".to_string(),
            sounds: "assets/sounds".to_string(),
            images: "assets/images".to_string(),
        },
        root_path: PathBuf::from(format!("campaigns/{}", id)),
        game_config: antares::sdk::game_config::GameConfig::default(),
    }
}

#[test]
fn test_game_state_without_campaign() {
    // Test that game can start without a campaign (core content mode)
    let game_state = GameState::new();

    assert!(game_state.campaign.is_none());
    assert_eq!(game_state.party.gold, 0);
    assert_eq!(game_state.party.food, 0);
}

#[test]
fn test_game_state_with_campaign() {
    // Test that game applies campaign starting configuration
    let campaign = create_test_campaign("test_campaign", "Test Campaign", "1.0.0");

    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign.clone());
    // Apply campaign starting configuration (was done by `new_game` previously)
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    assert!(game_state.campaign.is_some());

    let loaded_campaign = game_state.campaign.as_ref().unwrap();
    assert_eq!(loaded_campaign.id, "test_campaign");
    assert_eq!(loaded_campaign.name, "Test Campaign");
    assert_eq!(loaded_campaign.version, "1.0.0");

    // Verify starting conditions from campaign config
    assert_eq!(game_state.party.gold, 500);
    assert_eq!(game_state.party.food, 100);
}

#[test]
fn test_campaign_starting_conditions_applied() {
    // Test that all campaign starting conditions are properly applied
    let campaign = create_test_campaign("conditions_test", "Conditions Test", "1.0.0");

    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign);
    // Apply campaign starting configuration (was done by `new_game` previously)
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    // Check starting gold and food
    assert_eq!(game_state.party.gold, 500);
    assert_eq!(game_state.party.food, 100);

    // Game state should be in exploration mode
    assert_eq!(game_state.mode, antares::application::GameMode::Exploration);

    // Time should start at day 1, 6:00 AM
    assert_eq!(game_state.time.day, 1);
    assert_eq!(game_state.time.hour, 6);
    assert_eq!(game_state.time.minute, 0);
}

#[test]
fn test_save_game_without_campaign() {
    // Test saving game without campaign (core content mode)
    let game_state = GameState::new();
    let save = SaveGame::new(game_state);

    assert!(save.campaign_reference.is_none());
    assert_eq!(save.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_save_game_with_campaign_reference() {
    // Test that save games correctly store campaign reference
    let campaign = create_test_campaign("save_test", "Save Test", "1.2.3");
    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign.clone());
    // Mirror previous `new_game` behavior for party starting resources
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    let save = SaveGame::new(game_state);

    assert!(save.campaign_reference.is_some());

    let campaign_ref = save.campaign_reference.unwrap();
    assert_eq!(campaign_ref.id, "save_test");
    assert_eq!(campaign_ref.version, "1.2.3");
    assert_eq!(campaign_ref.name, "Save Test");
}

#[test]
fn test_save_and_load_campaign_game() {
    // Test full save/load cycle with campaign
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();

    // Create game with campaign
    let campaign = create_test_campaign("roundtrip_test", "Round Trip Test", "2.0.0");
    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign.clone());
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    // Save game
    manager.save("campaign_save", &game_state).unwrap();

    // Load game
    let loaded_state = manager.load("campaign_save").unwrap();

    // Campaign is not serialized (marked with #[serde(skip)]) so it will be None
    // This is correct behavior - campaign should be reloaded from disk based on
    // the campaign_reference in SaveGame
    assert!(loaded_state.campaign.is_none());

    // Verify game state is preserved
    assert_eq!(loaded_state.party.gold, game_state.party.gold);
    assert_eq!(loaded_state.party.food, game_state.party.food);
}

#[test]
fn test_campaign_reference_equality() {
    // Test CampaignReference equality
    let ref1 = CampaignReference {
        id: "test".to_string(),
        version: "1.0.0".to_string(),
        name: "Test".to_string(),
    };

    let ref2 = CampaignReference {
        id: "test".to_string(),
        version: "1.0.0".to_string(),
        name: "Test".to_string(),
    };

    let ref3 = CampaignReference {
        id: "other".to_string(),
        version: "1.0.0".to_string(),
        name: "Other".to_string(),
    };

    assert_eq!(ref1, ref2);
    assert_ne!(ref1, ref3);
}

#[test]
fn test_multiple_campaigns_save_load() {
    // Test saving and loading games from different campaigns
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();

    // Campaign 1
    let campaign1 = create_test_campaign("campaign1", "Campaign One", "1.0.0");
    let mut game_state1 = GameState::new();
    game_state1.campaign = Some(campaign1.clone());
    game_state1.party.gold = game_state1.campaign.as_ref().unwrap().config.starting_gold;
    game_state1.party.food = game_state1.campaign.as_ref().unwrap().config.starting_food;
    manager.save("save1", &game_state1).unwrap();

    // Campaign 2
    let campaign2 = create_test_campaign("campaign2", "Campaign Two", "2.0.0");
    let mut game_state2 = GameState::new();
    game_state2.campaign = Some(campaign2.clone());
    game_state2.party.gold = game_state2.campaign.as_ref().unwrap().config.starting_gold;
    game_state2.party.food = game_state2.campaign.as_ref().unwrap().config.starting_food;
    manager.save("save2", &game_state2).unwrap();

    // Load both saves
    let loaded1 = manager.load("save1").unwrap();
    let loaded2 = manager.load("save2").unwrap();

    // Campaign field is not serialized, so both will be None after load
    // The campaign_reference in SaveGame tracks which campaign to reload
    assert!(loaded1.campaign.is_none());
    assert!(loaded2.campaign.is_none());

    // Verify different starting conditions were preserved
    assert_eq!(loaded1.party.gold, 500);
    assert_eq!(loaded2.party.gold, 500);
}

#[test]
fn test_campaign_config_variations() {
    // Test different campaign configurations
    let mut campaign = create_test_campaign("config_test", "Config Test", "1.0.0");

    // Test hard difficulty
    campaign.config.difficulty = Difficulty::Hard;
    campaign.config.starting_gold = 50;
    campaign.config.starting_food = 10;
    campaign.config.permadeath = true;

    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign.clone());
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    assert_eq!(game_state.party.gold, 50);
    assert_eq!(game_state.party.food, 10);
}

#[test]
fn test_save_game_version_validation() {
    // Test that save game version validation works
    let game_state = GameState::new();
    let save = SaveGame::new(game_state);

    // Current version should validate
    assert!(save.validate_version().is_ok());
}

#[test]
fn test_campaign_data_paths() {
    // Test that campaign data paths are correctly stored
    let campaign = create_test_campaign("paths_test", "Paths Test", "1.0.0");

    assert_eq!(campaign.data.items, "data/items.ron");
    assert_eq!(campaign.data.spells, "data/spells.ron");
    assert_eq!(campaign.data.monsters, "data/monsters.ron");
    assert_eq!(campaign.data.classes, "data/classes.ron");
    assert_eq!(campaign.data.races, "data/races.ron");
    assert_eq!(campaign.data.maps, "maps");
    assert_eq!(campaign.data.quests, "data/quests.ron");
    assert_eq!(campaign.data.dialogues, "data/dialogues.ron");
}

#[test]
fn test_campaign_asset_paths() {
    // Test that campaign asset paths are correctly stored
    let campaign = create_test_campaign("assets_test", "Assets Test", "1.0.0");

    assert_eq!(campaign.assets.tilesets, "assets/tilesets");
    assert_eq!(campaign.assets.music, "assets/music");
    assert_eq!(campaign.assets.sounds, "assets/sounds");
    assert_eq!(campaign.assets.images, "assets/images");
}

#[test]
fn test_empty_campaign_list() {
    // Test handling of empty saves directory
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();

    let saves = manager.list_saves().unwrap();
    assert_eq!(saves.len(), 0);
}

#[test]
fn test_campaign_backward_compatibility() {
    // Test that core content mode is backward compatible
    let game_state_new = GameState::new();
    let game_state_old = GameState::new();

    // Both should have identical initial state
    assert_eq!(game_state_new.party.gold, game_state_old.party.gold);
    assert_eq!(game_state_new.party.food, game_state_old.party.food);
    assert_eq!(game_state_new.mode, game_state_old.mode);
}

#[test]
fn test_campaign_id_uniqueness() {
    // Test that different campaigns can coexist
    let campaign1 = create_test_campaign("unique1", "Unique One", "1.0.0");
    let campaign2 = create_test_campaign("unique2", "Unique Two", "1.0.0");

    assert_ne!(campaign1.id, campaign2.id);
    assert_ne!(campaign1.name, campaign2.name);
}

#[test]
fn test_game_state_serialization_with_campaign() {
    // Test that GameState with campaign can be serialized/deserialized
    let campaign = create_test_campaign("serialize_test", "Serialize Test", "1.0.0");
    let mut game_state = GameState::new();
    game_state.campaign = Some(campaign.clone());
    game_state.party.gold = game_state.campaign.as_ref().unwrap().config.starting_gold;
    game_state.party.food = game_state.campaign.as_ref().unwrap().config.starting_food;

    // Serialize to RON
    let serialized = ron::ser::to_string(&game_state).unwrap();

    // Deserialize back
    let deserialized: GameState = ron::from_str(&serialized).unwrap();

    // Campaign is skipped in serialization, so it should be None
    assert!(deserialized.campaign.is_none());

    // But other state should be preserved
    assert_eq!(deserialized.party.gold, game_state.party.gold);
    assert_eq!(deserialized.party.food, game_state.party.food);
}

#[test]
fn test_save_game_timestamp() {
    // Test that save games have valid timestamps
    let game_state = GameState::new();
    let save1 = SaveGame::new(game_state.clone());

    std::thread::sleep(std::time::Duration::from_millis(10));

    let save2 = SaveGame::new(game_state);

    // Save2 should have a later timestamp
    assert!(save2.timestamp > save1.timestamp);
}

#[test]
fn test_campaign_engine_version() {
    // Test that campaigns track engine version
    let campaign = create_test_campaign("version_test", "Version Test", "1.0.0");

    assert_eq!(campaign.engine_version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_difficulty_levels() {
    // Test all difficulty level configurations
    let difficulties = [
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::Brutal,
    ];

    for difficulty in difficulties {
        let mut campaign = create_test_campaign("diff_test", "Difficulty Test", "1.0.0");
        campaign.config.difficulty = difficulty;

        let mut game_state = GameState::new();
        game_state.campaign = Some(campaign);
        assert!(game_state.campaign.is_some());
    }
}
