// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Phase 1: Core Configuration Infrastructure
//!
//! These tests verify that campaigns can load config.ron files and
//! that the GameConfig system integrates properly with campaign loading.

use antares::sdk::campaign_loader::Campaign;
use antares::sdk::game_config::{CameraMode, GameConfig, ShadowQuality};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Create a minimal campaign directory with campaign.ron
fn create_test_campaign_dir(with_config: bool) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let campaign_dir = temp_dir.path();

    // Create data directory
    fs::create_dir(campaign_dir.join("data")).unwrap();

    // Create minimal campaign.ron
    let campaign_ron = r#"(
        id: "test_campaign",
        name: "Test Campaign",
        version: "1.0.0",
        author: "Test Author",
        description: "Test campaign for config integration",
        engine_version: "0.1.0",
        required_features: [],
        config: (
            starting_map: 1,
            starting_position: (x: 0, y: 0),
            starting_direction: North,
            starting_gold: 100,
            starting_food: 50,
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: Normal,
            permadeath: false,
            allow_multiclassing: false,
            starting_level: 1,
            max_level: 20,
        ),
        data: (
            items: "data/items.ron",
            spells: "data/spells.ron",
            monsters: "data/monsters.ron",
            classes: "data/classes.ron",
            races: "data/races.ron",
            maps: "data/maps",
            quests: "data/quests.ron",
            dialogues: "data/dialogues.ron",
            characters: "data/characters.ron",
        ),
        assets: (
            tilesets: "assets/tilesets",
            music: "assets/music",
            sounds: "assets/sounds",
            images: "assets/images",
        ),
    )"#;

    let mut campaign_file = fs::File::create(campaign_dir.join("campaign.ron")).unwrap();
    campaign_file.write_all(campaign_ron.as_bytes()).unwrap();

    // Optionally create config.ron
    if with_config {
        let config_ron = r#"(
            graphics: (
                resolution: (1920, 1080),
                fullscreen: true,
                vsync: false,
                msaa_samples: 8,
                shadow_quality: High,
            ),
            audio: (
                master_volume: 0.9,
                music_volume: 0.7,
                sfx_volume: 0.8,
                ambient_volume: 0.6,
                enable_audio: true,
            ),
            controls: (
                move_forward: ["W"],
                move_back: ["S"],
                turn_left: ["A"],
                turn_right: ["D"],
                interact: ["E"],
                menu: ["Escape"],
                movement_cooldown: 0.15,
            ),
            camera: (
                mode: FirstPerson,
                eye_height: 0.5,
                fov: 90.0,
                near_clip: 0.1,
                far_clip: 2000.0,
                smooth_rotation: true,
                rotation_speed: 90.0,
                light_height: 6.0,
                light_intensity: 3000000.0,
                light_range: 80.0,
                shadows_enabled: true,
            ),
        )"#;

        let mut config_file = fs::File::create(campaign_dir.join("config.ron")).unwrap();
        config_file.write_all(config_ron.as_bytes()).unwrap();
    }

    temp_dir
}

#[test]
fn test_campaign_loads_config_ron() {
    let temp_dir = create_test_campaign_dir(true);
    let campaign = Campaign::load(temp_dir.path()).unwrap();

    // Verify custom config was loaded
    assert_eq!(campaign.game_config.graphics.resolution, (1920, 1080));
    assert!(campaign.game_config.graphics.fullscreen);
    assert!(!campaign.game_config.graphics.vsync);
    assert_eq!(campaign.game_config.graphics.msaa_samples, 8);
    assert_eq!(
        campaign.game_config.graphics.shadow_quality,
        ShadowQuality::High
    );

    assert_eq!(campaign.game_config.audio.master_volume, 0.9);
    assert_eq!(campaign.game_config.audio.music_volume, 0.7);
    assert_eq!(campaign.game_config.audio.sfx_volume, 0.8);
    assert_eq!(campaign.game_config.audio.ambient_volume, 0.6);

    assert_eq!(campaign.game_config.controls.move_forward, vec!["W"]);
    assert_eq!(campaign.game_config.controls.move_back, vec!["S"]);
    assert_eq!(campaign.game_config.controls.movement_cooldown, 0.15);

    assert_eq!(campaign.game_config.camera.mode, CameraMode::FirstPerson);
    assert_eq!(campaign.game_config.camera.eye_height, 0.5);
    assert_eq!(campaign.game_config.camera.fov, 90.0);
    assert_eq!(campaign.game_config.camera.far_clip, 2000.0);
    assert!(campaign.game_config.camera.smooth_rotation);
    assert_eq!(campaign.game_config.camera.rotation_speed, 90.0);
}

#[test]
fn test_campaign_without_config_uses_defaults() {
    let temp_dir = create_test_campaign_dir(false);
    let campaign = Campaign::load(temp_dir.path()).unwrap();

    // Verify default config was used
    let default_config = GameConfig::default();
    assert_eq!(campaign.game_config, default_config);

    // Verify specific defaults match hardcoded values
    assert_eq!(campaign.game_config.graphics.resolution, (1280, 720));
    assert!(!campaign.game_config.graphics.fullscreen);
    assert!(campaign.game_config.graphics.vsync);
    assert_eq!(campaign.game_config.graphics.msaa_samples, 4);

    assert_eq!(campaign.game_config.audio.master_volume, 0.8);
    assert_eq!(campaign.game_config.audio.music_volume, 0.6);

    assert_eq!(campaign.game_config.camera.eye_height, 0.6);
    assert_eq!(campaign.game_config.camera.fov, 70.0);
    assert_eq!(campaign.game_config.camera.light_intensity, 2_000_000.0);
    assert_eq!(campaign.game_config.camera.light_range, 60.0);
}

#[test]
fn test_campaign_with_invalid_config_returns_error() {
    let temp_dir = create_test_campaign_dir(false);
    let campaign_dir = temp_dir.path();

    // Create invalid config.ron
    let invalid_config = r#"(this is not valid RON syntax"#;
    let mut config_file = fs::File::create(campaign_dir.join("config.ron")).unwrap();
    config_file.write_all(invalid_config.as_bytes()).unwrap();

    // Should fail to load campaign due to config parse error
    let result = Campaign::load(campaign_dir);
    assert!(result.is_err());
}

#[test]
fn test_campaign_validates_config_on_load() {
    let temp_dir = create_test_campaign_dir(false);
    let campaign_dir = temp_dir.path();

    // Create config with invalid values (zero resolution)
    let invalid_config = r#"(
        graphics: (
            resolution: (0, 0),
            fullscreen: false,
            vsync: true,
            msaa_samples: 4,
            shadow_quality: Medium,
        ),
        audio: (
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 1.0,
            ambient_volume: 0.5,
            enable_audio: true,
        ),
        controls: (
            move_forward: ["W"],
            move_back: ["S"],
            turn_left: ["A"],
            turn_right: ["D"],
            interact: ["E"],
            menu: ["Escape"],
            movement_cooldown: 0.2,
        ),
        camera: (
            mode: FirstPerson,
            eye_height: 0.6,
            fov: 70.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            smooth_rotation: false,
            rotation_speed: 180.0,
            light_height: 5.0,
            light_intensity: 2000000.0,
            light_range: 60.0,
            shadows_enabled: true,
        ),
    )"#;

    let mut config_file = fs::File::create(campaign_dir.join("config.ron")).unwrap();
    config_file.write_all(invalid_config.as_bytes()).unwrap();

    // Should fail validation
    let result = Campaign::load(campaign_dir);
    assert!(result.is_err());
}

#[test]
fn test_campaign_config_validation_passes_for_valid_custom_config() {
    let temp_dir = create_test_campaign_dir(true);
    let campaign = Campaign::load(temp_dir.path()).unwrap();

    // Validation should pass
    assert!(campaign.game_config.validate().is_ok());
}
