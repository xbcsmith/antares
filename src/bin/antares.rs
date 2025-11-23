// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Antares - Main Game Binary
//!
//! Turn-based RPG inspired by Might and Magic 1.
//! Now powered by Bevy Engine.

use antares::application::GameState;
use antares::game::resources::GlobalState;
use antares::game::systems::camera::CameraPlugin;
use antares::game::systems::map::MapRenderingPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(AntaresPlugin)
        .add_plugins(MapRenderingPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(antares::game::systems::input::InputPlugin)
        .add_plugins(antares::game::systems::events::EventPlugin)
        .add_plugins(antares::game::systems::ui::UiPlugin)
        .run();
}

/// Main game plugin organizing all systems
struct AntaresPlugin;

impl Plugin for AntaresPlugin {
    fn build(&self, app: &mut App) {
        // Load campaign
        let loader = antares::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
            .expect("Failed to load tutorial campaign");

        // Load campaign content
        let content_db = campaign
            .load_content()
            .expect("Failed to load campaign content");

        // Initialize game state with campaign
        let mut game_state = GameState::new_game(campaign.clone());

        // Load all maps from campaign
        for map_id in content_db.maps.all_maps() {
            if let Some(map) = content_db.maps.get_map(map_id) {
                game_state.world.add_map(map.clone());
            }
        }

        // Set starting map
        let starting_map_id = campaign.config.starting_map;
        if game_state.world.get_map(starting_map_id).is_some() {
            game_state.world.set_current_map(starting_map_id);
        } else {
            panic!("Starting map {} not found in campaign", starting_map_id);
        }

        // Set starting position
        game_state
            .world
            .set_party_position(campaign.config.starting_position);
        game_state.world.party_facing = campaign.config.starting_direction;

        app.insert_resource(GlobalState(game_state));
    }
}
