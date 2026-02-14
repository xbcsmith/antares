// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Core Configuration Infrastructure Integration Tests
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

/// Tutorial Campaign Configuration Tests

#[test]
fn test_tutorial_campaign_loads_config() {
    // Load the actual tutorial campaign
    let tutorial_path = std::path::Path::new("campaigns/tutorial");

    // Skip if tutorial campaign doesn't exist (CI environment)
    if !tutorial_path.exists() {
        eprintln!(
            "Skipping: tutorial campaign not found at {:?}",
            tutorial_path
        );
        return;
    }

    let campaign = Campaign::load(tutorial_path).unwrap();

    // Verify config loaded successfully
    assert!(campaign.game_config.validate().is_ok());

    // Verify tutorial uses default values (as specified in config.ron)
    assert_eq!(campaign.game_config.graphics.resolution, (1280, 720));
    assert!(!campaign.game_config.graphics.fullscreen);
    assert!(campaign.game_config.graphics.vsync);
    assert_eq!(campaign.game_config.graphics.msaa_samples, 4);
    assert_eq!(
        campaign.game_config.graphics.shadow_quality,
        ShadowQuality::Medium
    );

    assert_eq!(campaign.game_config.audio.master_volume, 0.8);
    assert_eq!(campaign.game_config.audio.music_volume, 0.6);
    assert_eq!(campaign.game_config.audio.sfx_volume, 1.0);
    assert_eq!(campaign.game_config.audio.ambient_volume, 0.5);
    assert!(campaign.game_config.audio.enable_audio);

    assert_eq!(
        campaign.game_config.controls.move_forward,
        vec!["W", "ArrowUp"]
    );
    assert_eq!(
        campaign.game_config.controls.move_back,
        vec!["S", "ArrowDown"]
    );
    assert_eq!(
        campaign.game_config.controls.turn_left,
        vec!["A", "ArrowLeft"]
    );
    assert_eq!(
        campaign.game_config.controls.turn_right,
        vec!["D", "ArrowRight"]
    );
    assert_eq!(campaign.game_config.controls.interact, vec!["Space", "E"]);
    assert_eq!(campaign.game_config.controls.menu, vec!["Escape"]);
    assert_eq!(campaign.game_config.controls.movement_cooldown, 0.2);

    assert_eq!(campaign.game_config.camera.mode, CameraMode::FirstPerson);
    assert_eq!(campaign.game_config.camera.eye_height, 0.6);
    assert_eq!(campaign.game_config.camera.fov, 70.0);
    assert_eq!(campaign.game_config.camera.near_clip, 0.1);
    assert_eq!(campaign.game_config.camera.far_clip, 1000.0);
    assert!(!campaign.game_config.camera.smooth_rotation);
    assert_eq!(campaign.game_config.camera.rotation_speed, 180.0);
    assert_eq!(campaign.game_config.camera.light_height, 5.0);
    assert_eq!(campaign.game_config.camera.light_intensity, 2000000.0);
    assert_eq!(campaign.game_config.camera.light_range, 60.0);
    assert!(campaign.game_config.camera.shadows_enabled);
}

#[test]
fn test_config_template_is_valid_ron() {
    use std::path::Path;

    let template_path = Path::new("campaigns/config.template.ron");

    // Skip if template doesn't exist (CI environment)
    if !template_path.exists() {
        eprintln!("Skipping: config template not found at {:?}", template_path);
        return;
    }

    // Read template file
    let contents = fs::read_to_string(template_path).unwrap();

    // Parse as GameConfig (should succeed despite comments)
    let result: Result<GameConfig, _> = ron::from_str(&contents);

    assert!(
        result.is_ok(),
        "Template should be valid RON: {:?}",
        result.err()
    );

    // Validate the parsed config
    let config = result.unwrap();
    assert!(
        config.validate().is_ok(),
        "Template config should pass validation"
    );
}

#[test]
fn test_campaign_defaults_match_template() {
    use std::path::Path;

    let template_path = Path::new("campaigns/config.template.ron");

    // Skip if template doesn't exist (CI environment)
    if !template_path.exists() {
        eprintln!("Skipping: config template not found at {:?}", template_path);
        return;
    }

    // Load template
    let contents = fs::read_to_string(template_path).unwrap();
    let template_config: GameConfig = ron::from_str(&contents).unwrap();

    // Get defaults
    let default_config = GameConfig::default();

    // Verify template matches defaults exactly
    assert_eq!(
        template_config.graphics.resolution, default_config.graphics.resolution,
        "Template graphics.resolution should match defaults"
    );
    assert_eq!(
        template_config.graphics.fullscreen, default_config.graphics.fullscreen,
        "Template graphics.fullscreen should match defaults"
    );
    assert_eq!(
        template_config.graphics.vsync, default_config.graphics.vsync,
        "Template graphics.vsync should match defaults"
    );
    assert_eq!(
        template_config.graphics.msaa_samples, default_config.graphics.msaa_samples,
        "Template graphics.msaa_samples should match defaults"
    );
    assert_eq!(
        template_config.graphics.shadow_quality, default_config.graphics.shadow_quality,
        "Template graphics.shadow_quality should match defaults"
    );

    assert_eq!(
        template_config.audio.master_volume, default_config.audio.master_volume,
        "Template audio.master_volume should match defaults"
    );
    assert_eq!(
        template_config.audio.music_volume, default_config.audio.music_volume,
        "Template audio.music_volume should match defaults"
    );
    assert_eq!(
        template_config.audio.sfx_volume, default_config.audio.sfx_volume,
        "Template audio.sfx_volume should match defaults"
    );
    assert_eq!(
        template_config.audio.ambient_volume, default_config.audio.ambient_volume,
        "Template audio.ambient_volume should match defaults"
    );
    assert_eq!(
        template_config.audio.enable_audio, default_config.audio.enable_audio,
        "Template audio.enable_audio should match defaults"
    );

    assert_eq!(
        template_config.controls.move_forward, default_config.controls.move_forward,
        "Template controls.move_forward should match defaults"
    );
    assert_eq!(
        template_config.controls.move_back, default_config.controls.move_back,
        "Template controls.move_back should match defaults"
    );
    assert_eq!(
        template_config.controls.turn_left, default_config.controls.turn_left,
        "Template controls.turn_left should match defaults"
    );
    assert_eq!(
        template_config.controls.turn_right, default_config.controls.turn_right,
        "Template controls.turn_right should match defaults"
    );
    assert_eq!(
        template_config.controls.interact, default_config.controls.interact,
        "Template controls.interact should match defaults"
    );
    assert_eq!(
        template_config.controls.menu, default_config.controls.menu,
        "Template controls.menu should match defaults"
    );
    assert_eq!(
        template_config.controls.movement_cooldown, default_config.controls.movement_cooldown,
        "Template controls.movement_cooldown should match defaults"
    );

    assert_eq!(
        template_config.camera.mode, default_config.camera.mode,
        "Template camera.mode should match defaults"
    );
    assert_eq!(
        template_config.camera.eye_height, default_config.camera.eye_height,
        "Template camera.eye_height should match defaults"
    );
    assert_eq!(
        template_config.camera.fov, default_config.camera.fov,
        "Template camera.fov should match defaults"
    );
    assert_eq!(
        template_config.camera.near_clip, default_config.camera.near_clip,
        "Template camera.near_clip should match defaults"
    );
    assert_eq!(
        template_config.camera.far_clip, default_config.camera.far_clip,
        "Template camera.far_clip should match defaults"
    );
    assert_eq!(
        template_config.camera.smooth_rotation, default_config.camera.smooth_rotation,
        "Template camera.smooth_rotation should match defaults"
    );
    assert_eq!(
        template_config.camera.rotation_speed, default_config.camera.rotation_speed,
        "Template camera.rotation_speed should match defaults"
    );
    assert_eq!(
        template_config.camera.light_height, default_config.camera.light_height,
        "Template camera.light_height should match defaults"
    );
    assert_eq!(
        template_config.camera.light_intensity, default_config.camera.light_intensity,
        "Template camera.light_intensity should match defaults"
    );
    assert_eq!(
        template_config.camera.light_range, default_config.camera.light_range,
        "Template camera.light_range should match defaults"
    );
    assert_eq!(
        template_config.camera.shadows_enabled, default_config.camera.shadows_enabled,
        "Template camera.shadows_enabled should match defaults"
    );

    // Verify overall equality
    assert_eq!(
        template_config, default_config,
        "Template config should exactly match GameConfig::default()"
    );
}
