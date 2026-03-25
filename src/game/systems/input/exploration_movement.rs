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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::Map;
    use crate::game::components::furniture::{DoorState, FurnitureEntity};
    use crate::game::systems::map::TileCoord;
    use bevy::prelude::{App, ButtonInput, KeyCode, Transform, Update};

    fn default_input_resource() -> crate::game::systems::input::InputConfigResource {
        let controls = crate::sdk::game_config::ControlsConfig {
            movement_cooldown: 0.2,
            ..crate::sdk::game_config::ControlsConfig::default()
        };
        let key_map = crate::game::systems::input::KeyMap::from_controls_config(&controls);
        crate::game::systems::input::InputConfigResource { controls, key_map }
    }

    fn build_movement_test_app() -> App {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(default_input_resource());
        app.insert_resource(crate::game::resources::GlobalState(
            crate::application::GameState::new(),
        ));
        app.insert_resource::<Time>(Time::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.add_systems(Update, movement_test_system);

        app
    }

    fn build_world_movement_test_app() -> App {
        let mut app = build_movement_test_app();

        {
            let mut gs = app
                .world_mut()
                .resource_mut::<crate::game::resources::GlobalState>();
            let map = Map::new(1, "MovementTestMap".to_string(), "Test".to_string(), 10, 10);
            gs.0.world.add_map(map);
            gs.0.world.set_current_map(1);
            gs.0.world
                .set_party_position(crate::domain::types::Position::new(5, 5));
        }

        app
    }

    fn spawn_door_entity(
        app: &mut App,
        position: crate::domain::types::Position,
        is_locked: bool,
        is_open: bool,
    ) {
        let mut door_state = DoorState::new(is_locked, 0.0);
        door_state.is_open = is_open;
        app.world_mut().spawn((
            FurnitureEntity::new(
                crate::domain::world::FurnitureType::Door,
                !door_state.is_open,
            ),
            door_state,
            Transform::default(),
            TileCoord(position),
        ));
    }

    fn move_forward_intent() -> crate::game::systems::input::FrameInputIntent {
        crate::game::systems::input::FrameInputIntent {
            move_forward: true,
            ..crate::game::systems::input::FrameInputIntent::default()
        }
    }

    fn move_back_intent() -> crate::game::systems::input::FrameInputIntent {
        crate::game::systems::input::FrameInputIntent {
            move_back: true,
            ..crate::game::systems::input::FrameInputIntent::default()
        }
    }

    fn turn_left_intent() -> crate::game::systems::input::FrameInputIntent {
        crate::game::systems::input::FrameInputIntent {
            turn_left: true,
            ..crate::game::systems::input::FrameInputIntent::default()
        }
    }

    fn turn_right_intent() -> crate::game::systems::input::FrameInputIntent {
        crate::game::systems::input::FrameInputIntent {
            turn_right: true,
            ..crate::game::systems::input::FrameInputIntent::default()
        }
    }

    fn no_input_intent() -> crate::game::systems::input::FrameInputIntent {
        crate::game::systems::input::FrameInputIntent::default()
    }

    fn movement_test_system(
        mut commands: Commands,
        input_config: Res<crate::game::systems::input::InputConfigResource>,
        mut global_state: ResMut<crate::game::resources::GlobalState>,
        time: Res<Time>,
        mut last_move_time: Local<f32>,
        victory_roots: Query<Entity, With<VictorySummaryRoot>>,
        door_entity_query: Query<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>,
        mut game_log: ResMut<GameLog>,
    ) {
        let current_time = time.elapsed_secs();
        let frame_input = decode_frame_input(
            &input_config.key_map,
            &ButtonInput::<KeyCode>::default(),
            &ButtonInput::<MouseButton>::default(),
            None,
        );

        let _ = handle_exploration_movement(
            &mut commands,
            frame_input,
            &mut global_state.0,
            None,
            &door_entity_query,
            &mut game_log,
            current_time,
            &mut last_move_time,
            &victory_roots,
        );
    }

    #[test]
    fn test_is_movement_attempt_forward_true() {
        assert!(is_movement_attempt(move_forward_intent()));
    }

    #[test]
    fn test_is_movement_attempt_backward_true() {
        assert!(is_movement_attempt(move_back_intent()));
    }

    #[test]
    fn test_is_movement_attempt_turn_left_true() {
        assert!(is_movement_attempt(turn_left_intent()));
    }

    #[test]
    fn test_is_movement_attempt_turn_right_true() {
        assert!(is_movement_attempt(turn_right_intent()));
    }

    #[test]
    fn test_is_movement_attempt_false_when_idle() {
        assert!(!is_movement_attempt(no_input_intent()));
    }

    #[test]
    fn test_movement_blocked_by_cooldown_true_with_recent_move() {
        let blocked = movement_blocked_by_cooldown(move_forward_intent(), 1.0, 0.9, 0.2);
        assert!(blocked);
    }

    #[test]
    fn test_movement_blocked_by_cooldown_false_after_window_expires() {
        let blocked = movement_blocked_by_cooldown(move_forward_intent(), 1.2, 0.9, 0.2);
        assert!(!blocked);
    }

    #[test]
    fn test_movement_blocked_by_cooldown_false_without_movement_attempt() {
        let blocked = movement_blocked_by_cooldown(no_input_intent(), 1.0, 0.9, 0.2);
        assert!(!blocked);
    }

    #[test]
    fn test_log_locked_door_adds_message() {
        let mut log = GameLog::default();

        log_locked_door(&mut log);

        assert!(
            log.messages
                .iter()
                .any(|message| message == "The door is locked."),
            "Expected the locked-door movement message to be recorded"
        );
    }

    #[test]
    fn test_handle_exploration_movement_returns_false_when_no_movement_requested() {
        let mut app = build_movement_test_app();

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            no_input_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            1.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(!moved);
        assert_eq!(last_move_time, 0.0);
    }

    #[test]
    fn test_handle_exploration_movement_turn_left_updates_facing_and_time() {
        let mut app = build_world_movement_test_app();

        let original_facing = {
            let gs = app
                .world()
                .resource::<crate::game::resources::GlobalState>();
            gs.0.world.party_facing
        };

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            turn_left_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            1.5,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        assert_ne!(gs.world.party_facing, original_facing);
        assert_eq!(last_move_time, 1.5);
    }

    #[test]
    fn test_handle_exploration_movement_turn_right_updates_facing_and_time() {
        let mut app = build_world_movement_test_app();

        let original_facing = {
            let gs = app
                .world()
                .resource::<crate::game::resources::GlobalState>();
            gs.0.world.party_facing
        };

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            turn_right_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            2.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        assert_ne!(gs.world.party_facing, original_facing);
        assert_eq!(last_move_time, 2.0);
    }

    #[test]
    fn test_handle_exploration_movement_forward_moves_party_when_tile_is_open() {
        let mut app = build_world_movement_test_app();

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let mut map = Map::new(1, "Movement".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            move_forward_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            3.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        assert_eq!(
            gs.world.party_position,
            crate::domain::types::Position::new(5, 4)
        );
        assert_eq!(last_move_time, 3.0);
    }

    #[test]
    fn test_handle_exploration_movement_backward_moves_party_when_tile_is_open() {
        let mut app = build_world_movement_test_app();

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let map = Map::new(1, "Movement".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            move_back_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            4.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        assert_eq!(
            gs.world.party_position,
            crate::domain::types::Position::new(5, 6)
        );
        assert_eq!(last_move_time, 4.0);
    }

    #[test]
    fn test_handle_exploration_movement_forward_blocked_by_locked_furniture_door_logs_message() {
        let mut app = build_world_movement_test_app();
        spawn_door_entity(
            &mut app,
            crate::domain::types::Position::new(5, 4),
            true,
            false,
        );

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let map = Map::new(1, "Movement".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            move_forward_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            5.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(!moved);
        assert!(
            game_log
                .messages
                .iter()
                .any(|message| message == "The door is locked."),
            "Expected locked-door feedback when forward movement is blocked"
        );
        assert_eq!(last_move_time, 0.0);
    }

    #[test]
    fn test_handle_exploration_movement_dialogue_move_cancels_dialogue() {
        let mut app = build_world_movement_test_app();

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let map = Map::new(1, "Movement".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));
        gs.mode = crate::application::GameMode::Dialogue(
            crate::application::dialogue::DialogueState::start(1, 1, None, None),
        );

        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            turn_left_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            6.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        assert!(matches!(gs.mode, crate::application::GameMode::Exploration));
        assert_eq!(last_move_time, 6.0);
    }

    #[test]
    fn test_handle_exploration_movement_victory_overlay_cleanup_despawns_roots() {
        let mut app = build_world_movement_test_app();
        app.world_mut().spawn(VictorySummaryRoot);

        let mut commands = app.world_mut().commands();
        let mut gs = crate::application::GameState::new();
        let map = Map::new(1, "Movement".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        let mut game_log = GameLog::default();
        let door_query = app.world().query::<(
            &mut FurnitureEntity,
            &mut DoorState,
            &mut Transform,
            &TileCoord,
        )>();
        let victory_query = app
            .world()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        let mut last_move_time = 0.0;

        let moved = handle_exploration_movement(
            &mut commands,
            turn_left_intent(),
            &mut gs,
            None,
            &door_query,
            &mut game_log,
            7.0,
            &mut last_move_time,
            &victory_query,
        );

        assert!(moved);
        let remaining = app
            .world_mut()
            .query_filtered::<Entity, With<VictorySummaryRoot>>()
            .iter(app.world())
            .count();
        assert_eq!(remaining, 0);
    }
}
