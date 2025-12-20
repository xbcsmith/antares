// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Antares - Main Game Binary
//!
//! Turn-based RPG inspired by Might and Magic 1.
//! Now powered by Bevy Engine.

use antares::application::GameState;
use antares::game::resources::GlobalState;
use antares::game::systems::camera::CameraPlugin;
use antares::game::systems::hud::HudPlugin;
use antares::game::systems::map::MapRenderingPlugin;
use antares::sdk::campaign_loader::{Campaign, CampaignLoader};
use antares::sdk::game_config::ShadowQuality;
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use bevy_egui::EguiPlugin;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to campaign directory
    #[arg(short, long)]
    campaign: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Load campaign
    let campaign = if let Some(path_str) = args.campaign {
        let path = PathBuf::from(path_str);
        Campaign::load(&path).unwrap_or_else(|e| {
            eprintln!("Failed to load campaign from {}: {}", path.display(), e);
            std::process::exit(1);
        })
    } else {
        let loader = CampaignLoader::new("campaigns");
        loader
            .load_campaign("tutorial")
            .expect("Failed to load tutorial campaign")
    };

    println!("Successfully loaded campaign: {}", campaign.name);

    // Extract game config before moving campaign
    let graphics_config = campaign.game_config.graphics.clone();
    let camera_config = campaign.game_config.camera.clone();
    let controls_config = campaign.game_config.controls.clone();

    // Configure window plugin from graphics config
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            resolution: graphics_config.resolution.into(),
            title: format!("Antares - {}", campaign.name),
            mode: if graphics_config.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
            } else {
                WindowMode::Windowed
            },
            present_mode: if graphics_config.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            ..default()
        }),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin).set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::all()),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GraphicsConfigResource {
            msaa_samples: graphics_config.msaa_samples,
            shadow_quality: graphics_config.shadow_quality,
        })
        .add_plugins(EguiPlugin::default())
        .add_plugins(AntaresPlugin { campaign })
        .add_plugins(MapRenderingPlugin)
        .add_plugins(CameraPlugin::new(camera_config))
        .add_plugins(HudPlugin)
        .add_plugins(antares::game::systems::input::InputPlugin::new(
            controls_config,
        ))
        .add_plugins(antares::game::systems::events::EventPlugin)
        // .add_plugins(antares::game::systems::ui::UiPlugin) // Temporarily disabled due to egui context issue
        .run();
}

/// Main game plugin organizing all systems
struct AntaresPlugin {
    campaign: Campaign,
}

impl Plugin for AntaresPlugin {
    fn build(&self, app: &mut App) {
        // Initialize game state and load campaign content (new_game returns (GameState, ContentDatabase))
        let (mut game_state, content_db) = GameState::new_game(self.campaign.clone())
            .expect("Failed to initialize game with campaign");

        // Load all maps from campaign
        for map_id in content_db.maps.all_maps() {
            if let Some(map) = content_db.maps.get_map(map_id) {
                game_state.world.add_map(map.clone());
            }
        }

        // Set starting map
        let starting_map_id = self.campaign.config.starting_map;
        if game_state.world.get_map(starting_map_id).is_some() {
            game_state.world.set_current_map(starting_map_id);
        } else {
            panic!("Starting map {} not found in campaign", starting_map_id);
        }

        // Set starting position
        game_state
            .world
            .set_party_position(self.campaign.config.starting_position);
        game_state.world.party_facing = self.campaign.config.starting_direction;

        // Insert global state and content DB as a resource
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(antares::application::resources::GameContent::new(
            content_db,
        ));

        // Register dialogue and quest plugins so their systems are available
        app.add_plugins(antares::game::systems::dialogue::DialoguePlugin);
        app.add_plugins(antares::game::systems::quest::QuestPlugin);
    }
}

/// Resource to hold graphics configuration for runtime access
///
/// This resource provides access to graphics settings that may be needed
/// by rendering systems at runtime, such as MSAA sample count and shadow quality.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::sdk::game_config::ShadowQuality;
///
/// fn example_system(graphics_config: Res<GraphicsConfigResource>) {
///     println!("MSAA samples: {}", graphics_config.msaa_samples);
///     println!("Shadow quality: {:?}", graphics_config.shadow_quality);
/// }
/// ```
#[derive(Resource, Clone, Debug)]
pub struct GraphicsConfigResource {
    /// MSAA sample count (must be power of 2: 1, 2, 4, 8)
    pub msaa_samples: u32,
    /// Shadow rendering quality level
    pub shadow_quality: ShadowQuality,
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::sdk::game_config::{CameraConfig, ControlsConfig, GameConfig, GraphicsConfig};

    /// Helper to create a test campaign with custom graphics config
    fn create_test_campaign(graphics: GraphicsConfig) -> Campaign {
        use antares::domain::types::Position;
        use antares::sdk::campaign_loader::{CampaignAssets, CampaignConfig, CampaignData};
        use std::path::PathBuf;

        Campaign {
            id: "test_campaign".to_string(),
            name: "Test Campaign".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test campaign for graphics config".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: vec![],
            config: CampaignConfig {
                starting_map: 1,
                starting_position: Position::new(0, 0),
                starting_direction: antares::domain::types::Direction::North,
                starting_gold: 100,
                starting_food: 50,
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: antares::sdk::campaign_loader::Difficulty::Normal,
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
                maps: "data/maps".to_string(),
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
            root_path: PathBuf::from("test_campaign"),
            game_config: GameConfig {
                graphics,
                audio: Default::default(),
                controls: ControlsConfig::default(),
                camera: CameraConfig::default(),
            },
        }
    }

    #[test]
    fn test_graphics_config_resource_creation() {
        let graphics = GraphicsConfig {
            resolution: (1920, 1080),
            fullscreen: true,
            vsync: false,
            msaa_samples: 8,
            shadow_quality: ShadowQuality::Ultra,
        };

        let resource = GraphicsConfigResource {
            msaa_samples: graphics.msaa_samples,
            shadow_quality: graphics.shadow_quality,
        };

        assert_eq!(resource.msaa_samples, 8);
        assert_eq!(resource.shadow_quality, ShadowQuality::Ultra);
    }

    #[test]
    fn test_window_resolution_from_config() {
        // Test that resolution configuration is correctly extracted
        let graphics = GraphicsConfig {
            resolution: (1920, 1080),
            fullscreen: false,
            vsync: true,
            msaa_samples: 4,
            shadow_quality: ShadowQuality::Medium,
        };

        let campaign = create_test_campaign(graphics.clone());

        // Verify resolution matches config
        assert_eq!(campaign.game_config.graphics.resolution, (1920, 1080));

        // Test resolution conversion to WindowResolution format
        let resolution: (u32, u32) = graphics.resolution;
        assert_eq!(resolution.0, 1920);
        assert_eq!(resolution.1, 1080);
    }

    #[test]
    fn test_fullscreen_mode_from_config() {
        // Test fullscreen enabled
        let fullscreen_graphics = GraphicsConfig {
            fullscreen: true,
            ..Default::default()
        };

        let campaign = create_test_campaign(fullscreen_graphics.clone());
        assert!(campaign.game_config.graphics.fullscreen);

        // Test windowed mode
        let windowed_graphics = GraphicsConfig {
            fullscreen: false,
            ..Default::default()
        };

        let campaign = create_test_campaign(windowed_graphics.clone());
        assert!(!campaign.game_config.graphics.fullscreen);
    }

    #[test]
    fn test_vsync_from_config() {
        // Test VSync enabled
        let vsync_graphics = GraphicsConfig {
            vsync: true,
            ..Default::default()
        };

        let campaign = create_test_campaign(vsync_graphics.clone());
        assert!(campaign.game_config.graphics.vsync);

        // Test VSync disabled
        let no_vsync_graphics = GraphicsConfig {
            vsync: false,
            ..Default::default()
        };

        let campaign = create_test_campaign(no_vsync_graphics.clone());
        assert!(!campaign.game_config.graphics.vsync);
    }

    #[test]
    fn test_msaa_samples_from_config() {
        // Test various MSAA sample counts
        for samples in [1, 2, 4, 8] {
            let graphics = GraphicsConfig {
                msaa_samples: samples,
                ..Default::default()
            };

            let campaign = create_test_campaign(graphics.clone());
            assert_eq!(campaign.game_config.graphics.msaa_samples, samples);
        }
    }

    #[test]
    fn test_shadow_quality_from_config() {
        // Test all shadow quality levels
        for quality in [
            ShadowQuality::Low,
            ShadowQuality::Medium,
            ShadowQuality::High,
            ShadowQuality::Ultra,
        ] {
            let graphics = GraphicsConfig {
                shadow_quality: quality,
                ..Default::default()
            };

            let campaign = create_test_campaign(graphics.clone());
            assert_eq!(campaign.game_config.graphics.shadow_quality, quality);
        }
    }

    #[test]
    fn test_graphics_config_defaults() {
        let default_graphics = GraphicsConfig::default();

        assert_eq!(default_graphics.resolution, (1280, 720));
        assert!(!default_graphics.fullscreen);
        assert!(default_graphics.vsync);
        assert_eq!(default_graphics.msaa_samples, 4);
        assert_eq!(default_graphics.shadow_quality, ShadowQuality::Medium);
    }

    #[test]
    fn test_window_title_includes_campaign_name() {
        let campaign = create_test_campaign(GraphicsConfig::default());
        let title = format!("Antares - {}", campaign.name);

        assert_eq!(title, "Antares - Test Campaign");
    }

    #[test]
    fn test_graphics_resource_debug_impl() {
        let resource = GraphicsConfigResource {
            msaa_samples: 4,
            shadow_quality: ShadowQuality::High,
        };

        let debug_output = format!("{:?}", resource);
        assert!(debug_output.contains("msaa_samples"));
        assert!(debug_output.contains("shadow_quality"));
    }
}
