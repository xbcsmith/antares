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
        .add_plugins(EguiPlugin)
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
        // Initialize game state with a test map
        let mut game_state = GameState::new();
        let mut map = antares::domain::world::Map::new(1, 10, 10);

        // Add some walls
        for x in 0..10 {
            map.get_tile_mut(antares::domain::types::Position::new(x, 0))
                .unwrap()
                .wall_type = antares::domain::world::WallType::Normal;
            map.get_tile_mut(antares::domain::types::Position::new(x, 9))
                .unwrap()
                .wall_type = antares::domain::world::WallType::Normal;
            map.get_tile_mut(antares::domain::types::Position::new(0, x))
                .unwrap()
                .wall_type = antares::domain::world::WallType::Normal;
            map.get_tile_mut(antares::domain::types::Position::new(9, x))
                .unwrap()
                .wall_type = antares::domain::world::WallType::Normal;
        }

        // Add a pillar
        map.get_tile_mut(antares::domain::types::Position::new(5, 5))
            .unwrap()
            .wall_type = antares::domain::world::WallType::Normal;

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state
            .world
            .set_party_position(antares::domain::types::Position::new(2, 2));

        app.insert_resource(GlobalState(game_state));
    }
}
