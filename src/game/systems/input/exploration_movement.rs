// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Exploration movement helpers.
//!
//! This module isolates exploration movement and turning behavior from the
//! monolithic input system. It owns movement-attempt detection, cooldown
//! gating, forward/backward movement, turning, dialogue-cancel-on-move
//! behavior, and victory-overlay cleanup coupling that currently remains part
//! of movement semantics.

use crate::application::resources::GameContent;
use crate::application::{GameMode, GameState, MoveHandleError};
use crate::domain::world::{self, MovementError, VISIBILITY_RADIUS};
use crate::game::components::furniture::DoorState;
use crate::game::components::FurnitureEntity;
use crate::game::systems::combat::VictorySummaryRoot;
use crate::game::systems::input::FrameInputIntent;
use crate::game::systems::map::TileCoord;
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;

/// Returns whether the current frame contains any movement-oriented input.
///
/// This mirrors the existing input-system movement grouping: forward, backward,
/// turn-left, and turn-right are all considered movement attempts for cooldown
/// purposes.
///
/// # Arguments
///
/// * `frame_input` - Decoded frame input for the current frame
///
/// # Returns
///
/// `true` when any movement or turning action is active, otherwise `false`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::{is_movement_attempt, FrameInputIntent};
///
/// let intent = FrameInputIntent {
///     move_forward: true,
///     ..FrameInputIntent::default()
/// };
///
/// assert!(is_movement_attempt(intent));
/// ```
pub fn is_movement_attempt(frame_input: FrameInputIntent) -> bool {
    frame_input.is_movement_attempt()
}

/// Returns whether movement should be blocked by the current cooldown window.
///
/// # Arguments
///
/// * `frame_input` - Decoded frame input for the current frame
/// * `current_time` - Current elapsed time in seconds
/// * `last_move_time` - Time of the last successful movement or turn
/// * `cooldown` - Configured movement cooldown in seconds
///
/// # Returns
///
/// `true` if the current frame contains a movement attempt that falls within the
/// cooldown window, otherwise `false`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::{movement_blocked_by_cooldown, FrameInputIntent};
///
/// let intent = FrameInputIntent {
///     turn_left: true,
///     ..FrameInputIntent::default()
/// };
///
/// assert!(movement_blocked_by_cooldown(intent, 1.0, 0.9, 0.2));
/// assert!(!movement_blocked_by_cooldown(intent, 1.2, 0.9, 0.2));
/// ```
pub fn movement_blocked_by_cooldown(
    frame_input: FrameInputIntent,
    current_time: f32,
    last_move_time: f32,
    cooldown: f32,
) -> bool {
    is_movement_attempt(frame_input) && (current_time - last_move_time < cooldown)
}

/// Handles exploration movement and turning for the current frame.
///
/// The helper preserves the existing priority order:
///
/// 1. move forward
/// 2. move backward
/// 3. turn left
/// 4. turn right
///
/// On successful movement or turning, this helper:
///
/// - updates `last_move_time`
/// - cancels dialogue if movement occurred during dialogue mode
/// - despawns any active victory overlay roots
///
/// # Arguments
///
/// * `commands` - Commands used for overlay cleanup
/// * `frame_input` - Decoded frame input for the current frame
/// * `game_state` - Mutable game state
/// * `game_content` - Optional game content database for movement/event handling
/// * `door_entity_query` - Furniture door query used for locked-door movement checks
/// * `game_log` - Player-visible game log
/// * `current_time` - Current elapsed time in seconds
/// * `last_move_time` - Mutable last-move timestamp
/// * `victory_roots` - Victory overlay entities to clean up after movement
///
/// # Returns
///
/// Returns `true` when movement or turning was performed and consumed the frame.
/// Returns `false` when no movement action applied.
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::game::systems::input::{handle_exploration_movement, FrameInputIntent};
///
/// let _ = (handle_exploration_movement, FrameInputIntent::default(), GameState::new());
/// ```
#[allow(clippy::too_many_arguments)]
pub fn handle_exploration_movement(
    commands: &mut Commands,
    frame_input: FrameInputIntent,
    game_state: &mut GameState,
    game_content: Option<&GameContent>,
    door_entity_query: &Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    game_log: &mut GameLog,
    current_time: f32,
    last_move_time: &mut f32,
    victory_roots: &Query<Entity, With<VictorySummaryRoot>>,
) -> bool {
    let moved = if frame_input.move_forward {
        handle_move_forward(game_state, game_content, door_entity_query, game_log)
    } else if frame_input.move_back {
        handle_move_back(game_state, game_content, game_log)
    } else if frame_input.turn_left {
        handle_turn_left(game_state)
    } else if frame_input.turn_right {
        handle_turn_right(game_state)
    } else {
        false
    };

    if !moved {
        return false;
    }

    *last_move_time = current_time;

    if matches!(game_state.mode, GameMode::Dialogue(_)) {
        info!("Movement detected during dialogue - cancelling dialogue");
        game_state.mode = GameMode::Exploration;
    }

    for entity in victory_roots.iter() {
        commands.entity(entity).despawn();
    }

    true
}

/// Handles forward movement, including locked furniture-door blocking.
fn handle_move_forward(
    game_state: &mut GameState,
    game_content: Option<&GameContent>,
    door_entity_query: &Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    game_log: &mut GameLog,
) -> bool {
    let target = game_state.world.position_ahead();
    let facing = game_state.world.party_facing;

    let locked_door_ahead = door_entity_query
        .iter()
        .any(|(_, ds, _, tc)| tc.0 == target && ds.is_locked && !ds.is_open);

    if locked_door_ahead {
        log_locked_door(game_log);
        return false;
    }

    if let Some(content) = game_content {
        match game_state.move_party_and_handle_events(facing, content.db()) {
            Ok(()) => true,
            Err(MoveHandleError::Movement(MovementError::DoorLocked(_, _))) => {
                log_locked_door(game_log);
                false
            }
            Err(MoveHandleError::Movement(MovementError::Blocked(_, _))) => false,
            Err(MoveHandleError::Movement(MovementError::OutOfBounds(_, _))) => false,
            Err(err) => {
                warn!("move forward failed: {}", err);
                false
            }
        }
    } else if let Some(map) = game_state.world.get_current_map() {
        if !map.is_blocked(target) {
            game_state.world.set_party_position(target);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Handles backward movement through the same world movement path used for
/// forward movement.
fn handle_move_back(
    game_state: &mut GameState,
    game_content: Option<&GameContent>,
    game_log: &mut GameLog,
) -> bool {
    let back_facing = game_state.world.party_facing.turn_left().turn_left();

    if let Some(content) = game_content {
        match game_state.move_party_and_handle_events(back_facing, content.db()) {
            Ok(()) => true,
            Err(MoveHandleError::Movement(MovementError::DoorLocked(_, _))) => {
                log_locked_door(game_log);
                false
            }
            Err(MoveHandleError::Movement(MovementError::Blocked(_, _))) => false,
            Err(MoveHandleError::Movement(MovementError::OutOfBounds(_, _))) => false,
            Err(err) => {
                warn!("move backward failed: {}", err);
                false
            }
        }
    } else {
        let target = back_facing.forward(game_state.world.party_position);
        if let Some(map) = game_state.world.get_current_map() {
            if !map.is_blocked(target) {
                game_state.world.set_party_position(target);
                return true;
            }
        }
        false
    }
}

/// Handles left turn movement and visibility refresh.
fn handle_turn_left(game_state: &mut GameState) -> bool {
    game_state.world.turn_left();
    refresh_visibility_if_exploring(game_state);
    true
}

/// Handles right turn movement and visibility refresh.
fn handle_turn_right(game_state: &mut GameState) -> bool {
    game_state.world.turn_right();
    refresh_visibility_if_exploring(game_state);
    true
}

/// Refreshes visible-area state when the current mode is exploration.
fn refresh_visibility_if_exploring(game_state: &mut GameState) {
    if matches!(game_state.mode, GameMode::Exploration) {
        let party_position = game_state.world.party_position;
        world::mark_visible_area(&mut game_state.world, party_position, VISIBILITY_RADIUS);
    }
}

/// Logs the standard locked-door movement message.
fn log_locked_door(game_log: &mut GameLog) {
    let msg = "The door is locked.".to_string();
    info!("{}", msg);
    game_log.add(msg);
}
