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
use bevy::prelude::*;
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

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(AntaresPlugin { campaign })
        .add_plugins(MapRenderingPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(HudPlugin)
        .add_plugins(antares::game::systems::input::InputPlugin)
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
