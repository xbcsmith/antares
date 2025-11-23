// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::game::resources::GlobalState;
use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_input);
    }
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    time: Res<Time>,
    mut last_move_time: Local<f32>,
) {
    // Simple cooldown to prevent zooming too fast
    if time.elapsed_seconds() - *last_move_time < 0.2 {
        return;
    }

    let game_state = &mut global_state.0;
    let world = &mut game_state.world;
    let mut moved = false;

    if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        let target = world.position_ahead();
        if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    } else if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        // Move backward
        // We don't have a direct "position_behind" method, so we calculate it manually or use direction helper
        // Assuming Direction has a way to get opposite, or we just turn 180
        let back_facing = world.party_facing.turn_left().turn_left();
        let target = back_facing.forward(world.party_position);

        if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    } else if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        world.turn_left();
        moved = true;
    } else if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        world.turn_right();
        moved = true;
    }

    if moved {
        *last_move_time = time.elapsed_seconds();
        // TODO: Check for events at new position (Phase 4)
    }
}
