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
use bevy::log::{BoxedFmtLayer, BoxedLayer, LogPlugin, DEFAULT_FILTER};
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use bevy_egui::EguiPlugin;
use clap::Parser;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::{filter::FilterFn, Layer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to campaign directory
    #[arg(short, long)]
    campaign: Option<String>,

    /// Path to log file to write logs to (tee)
    #[arg(long, value_name = "FILE")]
    log: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    if let Some(log_path) = &args.log {
        // Make log file path available to our LogPlugin custom layer
        std::env::set_var("ANTARES_LOG_FILE", log_path.to_string_lossy().to_string());
        eprintln!("Logging to file: {}", log_path.display());
    }

    // Let Bevy's LogPlugin initialize logging to avoid double-initialization.
    // Tracing/log setup is handled by the engine's LogPlugin now.

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
    let audio_config = campaign.game_config.audio.clone();

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

    // Set BEVY_ASSET_ROOT to the campaign directory so the AssetServer resolves relative asset paths against it.
    let campaign_root_abs = campaign
        .root_path
        .canonicalize()
        .unwrap_or_else(|_| campaign.root_path.clone());
    let campaign_root_str = campaign_root_abs.to_string_lossy().to_string();
    std::env::set_var("BEVY_ASSET_ROOT", campaign_root_str.clone());

    // Build the app and configure the AssetPlugin to use '.' as its file path so the effective asset root is the campaign root
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(window_plugin)
            .set(bevy::asset::AssetPlugin {
                file_path: ".".to_string(),
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::all()),
                    ..default()
                }),
                ..default()
            })
            .set(LogPlugin {
                // Keep bevy defaults; when `--log` is provided raise the global level so debug messages
                // are captured in the file, while the console fmt layer still mutes noisy cosmic_text debug lines.
                filter: DEFAULT_FILTER.to_string(),
                level: if args.log.is_some() {
                    Level::DEBUG
                } else {
                    Level::INFO
                },
                custom_layer: antares_file_custom_layer,
                fmt_layer: antares_console_fmt_layer,
            }),
    )
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
    .add_plugins(antares::game::systems::audio::AudioPlugin {
        config: audio_config,
    })
    .add_plugins(antares::game::systems::ui::UiPlugin);

    app.run();
}

/// Console fmt layer that suppresses noisy relayout debug messages from cosmic_text
fn antares_console_fmt_layer(_app: &mut App) -> Option<BoxedFmtLayer> {
    let fmt_layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(std::io::stderr)
        .with_filter(FilterFn::new(|meta| {
            // Mute DEBUG/TRACE level logs from cosmic_text::buffer to avoid overwhelming the console
            if meta.target() == "cosmic_text::buffer" {
                match *meta.level() {
                    tracing::Level::DEBUG | tracing::Level::TRACE => return false,
                    _ => {}
                }
            }
            true
        }));
    Some(Box::new(fmt_layer))
}

/// Simple writer that delegates to an Arc<Mutex<File>> so the MakeWriter closure can clone it
struct ArcFileWriter(Arc<Mutex<std::fs::File>>);

impl std::io::Write for ArcFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut f = self.0.lock().unwrap();
        f.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let mut f = self.0.lock().unwrap();
        f.flush()
    }
}

/// If ANTARES_LOG_FILE is set, provide an extra file fmt layer so logs are also written to that file.
fn antares_file_custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    if let Ok(path_str) = std::env::var("ANTARES_LOG_FILE") {
        let path = std::path::PathBuf::from(path_str);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                let arc = Arc::new(Mutex::new(file));
                let make_writer = {
                    let arc = arc.clone();
                    move || ArcFileWriter(arc.clone())
                };
                let file_layer = tracing_subscriber::fmt::Layer::default()
                    .with_writer(make_writer)
                    .with_ansi(false);
                // Keep the layer; the Arc ensures file stays alive
                Some(Box::new(file_layer))
            }
            Err(e) => {
                eprintln!("Failed to open log file '{}': {}", path.display(), e);
                None
            }
        }
    } else {
        None
    }
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
        app.add_plugins(antares::game::systems::inn_ui::InnUiPlugin);
        app.add_plugins(antares::game::systems::recruitment_dialog::RecruitmentDialogPlugin);
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
    use std::sync::Mutex;

    // Serialize tests that modify the process environment to avoid races when tests run in parallel.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

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
                starting_innkeeper: "tutorial_innkeeper_town".to_string(),
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

    #[test]
    fn test_console_fmt_layer_present() {
        // Ensure the console fmt layer factory is present and returns a layer
        let mut app = App::new();
        assert!(antares_console_fmt_layer(&mut app).is_some());
    }

    #[test]
    fn test_file_custom_layer_none_when_env_unset() {
        // When ANTARES_LOG_FILE is unset, the custom file layer factory returns None
        // Lock to prevent races with other tests that modify the environment.
        let _guard = ENV_MUTEX.lock().unwrap();
        let original = std::env::var_os("ANTARES_LOG_FILE");
        std::env::remove_var("ANTARES_LOG_FILE");

        let mut app = App::new();
        assert!(antares_file_custom_layer(&mut app).is_none());

        // Restore original env var state
        match original {
            Some(val) => std::env::set_var("ANTARES_LOG_FILE", val),
            None => std::env::remove_var("ANTARES_LOG_FILE"),
        }
    }

    #[test]
    fn test_file_custom_layer_some_when_env_set() {
        // When ANTARES_LOG_FILE is set to a writable path, the custom file layer factory returns Some
        // Lock to prevent races with other tests that modify the environment.
        let _guard = ENV_MUTEX.lock().unwrap();
        let original = std::env::var_os("ANTARES_LOG_FILE");

        let tmp = tempfile::NamedTempFile::new().expect("create tmp file");
        std::env::set_var("ANTARES_LOG_FILE", tmp.path().to_string_lossy().to_string());

        let mut app = App::new();
        assert!(antares_file_custom_layer(&mut app).is_some());

        // Restore original env var state
        match original {
            Some(val) => std::env::set_var("ANTARES_LOG_FILE", val),
            None => std::env::remove_var("ANTARES_LOG_FILE"),
        }
    }
}
