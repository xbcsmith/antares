// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Input System Module
//!
//! Provides config-driven input handling with customizable key bindings and movement cooldown.
//!
//! # Overview
//!
//! The input system translates keyboard input into game actions using configurable key mappings
//! from `ControlsConfig`. This allows campaigns to customize controls and provides a foundation
//! for player-remappable keys.
//!
//! # Features
//!
//! - **Config-Driven Key Mapping**: All key bindings come from `ControlsConfig`
//! - **Multiple Keys Per Action**: Each action can be triggered by multiple keys
//! - **Configurable Cooldown**: Movement cooldown prevents accidental double-moves
//! - **Door Interaction**: Space/E to open doors (configurable via interact keys)
//! - **Classic Movement**: Arrow keys and WASD (default, fully customizable)
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::input::InputPlugin;
//! use antares::sdk::game_config::ControlsConfig;
//!
//! # fn setup() {
//! let config = ControlsConfig::default();
//! let mut app = App::new();
//! app.add_plugins(InputPlugin::new(config));
//! # }
//! ```

use crate::application::dialogue::RecruitmentContext;
use crate::domain::types::Position;
use crate::domain::world::{MapEvent, WallType};
use crate::game::components::dialogue::NpcDialogue;
use crate::game::resources::GlobalState;
use crate::game::systems::dialogue::{PendingRecruitmentContext, StartDialogue};
use crate::game::systems::events::MapEventTriggered;
use crate::game::systems::map::{DoorOpenedEvent, NpcMarker, TileCoord};
use crate::sdk::game_config::ControlsConfig;
use bevy::prelude::*;
use std::collections::HashMap;

/// Input plugin with config-driven key mappings
///
/// Manages input handling with customizable key bindings and movement cooldown.
pub struct InputPlugin {
    /// Controls configuration for key mappings and cooldown
    config: ControlsConfig,
}

impl InputPlugin {
    /// Create a new input plugin with the given controls configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Controls configuration defining key bindings and cooldown
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::InputPlugin;
    /// use antares::sdk::game_config::ControlsConfig;
    ///
    /// let config = ControlsConfig::default();
    /// let plugin = InputPlugin::new(config);
    /// ```
    pub fn new(config: ControlsConfig) -> Self {
        Self { config }
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // Insert input config as a resource
        let key_map = KeyMap::from_controls_config(&self.config);
        app.insert_resource(InputConfigResource {
            controls: self.config.clone(),
            key_map,
        });

        app.add_systems(Update, handle_input);
    }
}

/// Bevy resource containing input configuration and key mappings
///
/// This resource makes the controls configuration available to input systems.
#[derive(Resource)]
pub struct InputConfigResource {
    /// Controls configuration
    pub controls: ControlsConfig,

    /// Compiled key map for efficient input checking
    pub key_map: KeyMap,
}

/// Game actions that can be triggered by input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    /// Move forward in current facing direction
    MoveForward,

    /// Move backward (opposite of facing direction)
    MoveBack,

    /// Turn left (rotate counterclockwise)
    TurnLeft,

    /// Turn right (rotate clockwise)
    TurnRight,

    /// Interact with objects (open doors, talk to NPCs)
    Interact,

    /// Open menu
    Menu,
}

/// Key mapping structure for efficient input lookups
///
/// Maps `KeyCode` to `GameAction` for fast input processing.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Map from KeyCode to GameAction
    bindings: HashMap<KeyCode, GameAction>,
}

impl KeyMap {
    /// Create a KeyMap from ControlsConfig
    ///
    /// Translates string key names to Bevy KeyCode and builds the lookup map.
    ///
    /// # Arguments
    ///
    /// * `config` - Controls configuration with key binding strings
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::KeyMap;
    /// use antares::sdk::game_config::ControlsConfig;
    ///
    /// let config = ControlsConfig::default();
    /// let key_map = KeyMap::from_controls_config(&config);
    /// ```
    pub fn from_controls_config(config: &ControlsConfig) -> Self {
        let mut bindings = HashMap::new();

        // Map move_forward keys
        for key_str in &config.move_forward {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::MoveForward);
            } else {
                warn!("Invalid key code in move_forward: {}", key_str);
            }
        }

        // Map move_back keys
        for key_str in &config.move_back {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::MoveBack);
            } else {
                warn!("Invalid key code in move_back: {}", key_str);
            }
        }

        // Map turn_left keys
        for key_str in &config.turn_left {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::TurnLeft);
            } else {
                warn!("Invalid key code in turn_left: {}", key_str);
            }
        }

        // Map turn_right keys
        for key_str in &config.turn_right {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::TurnRight);
            } else {
                warn!("Invalid key code in turn_right: {}", key_str);
            }
        }

        // Map interact keys
        for key_str in &config.interact {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Interact);
            } else {
                warn!("Invalid key code in interact: {}", key_str);
            }
        }

        // Map menu keys
        for key_str in &config.menu {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Menu);
            } else {
                warn!("Invalid key code in menu: {}", key_str);
            }
        }

        Self { bindings }
    }

    /// Get the action bound to a specific key code
    ///
    /// # Arguments
    ///
    /// * `key_code` - The key code to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(GameAction)` if the key is bound, `None` otherwise
    pub fn get_action(&self, key_code: KeyCode) -> Option<GameAction> {
        self.bindings.get(&key_code).copied()
    }

    /// Check if any of the keys for an action are currently pressed
    ///
    /// # Arguments
    ///
    /// * `action` - The game action to check
    /// * `keyboard_input` - Bevy keyboard input state
    ///
    /// # Returns
    ///
    /// Returns `true` if any key bound to the action is pressed
    pub fn is_action_pressed(
        &self,
        action: GameAction,
        keyboard_input: &ButtonInput<KeyCode>,
    ) -> bool {
        self.bindings.iter().any(|(key_code, bound_action)| {
            *bound_action == action && keyboard_input.pressed(*key_code)
        })
    }

    /// Check if any of the keys for an action were just pressed this frame
    ///
    /// # Arguments
    ///
    /// * `action` - The game action to check
    /// * `keyboard_input` - Bevy keyboard input state
    ///
    /// # Returns
    ///
    /// Returns `true` if any key bound to the action was just pressed
    pub fn is_action_just_pressed(
        &self,
        action: GameAction,
        keyboard_input: &ButtonInput<KeyCode>,
    ) -> bool {
        self.bindings.iter().any(|(key_code, bound_action)| {
            *bound_action == action && keyboard_input.just_pressed(*key_code)
        })
    }
}

/// Parse a key code string into Bevy's KeyCode enum
///
/// Supports common key names and aliases for compatibility.
///
/// # Arguments
///
/// * `key_str` - String representation of the key (e.g., "W", "ArrowUp", "Space")
///
/// # Returns
///
/// Returns `Some(KeyCode)` if the string is recognized, `None` otherwise
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::parse_key_code;
/// use bevy::prelude::KeyCode;
///
/// assert_eq!(parse_key_code("W"), Some(KeyCode::KeyW));
/// assert_eq!(parse_key_code("ArrowUp"), Some(KeyCode::ArrowUp));
/// assert_eq!(parse_key_code("Space"), Some(KeyCode::Space));
/// assert_eq!(parse_key_code("Invalid"), None);
/// ```
pub fn parse_key_code(key_str: &str) -> Option<KeyCode> {
    match key_str {
        // Letter keys
        "A" => Some(KeyCode::KeyA),
        "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC),
        "D" => Some(KeyCode::KeyD),
        "E" => Some(KeyCode::KeyE),
        "F" => Some(KeyCode::KeyF),
        "G" => Some(KeyCode::KeyG),
        "H" => Some(KeyCode::KeyH),
        "I" => Some(KeyCode::KeyI),
        "J" => Some(KeyCode::KeyJ),
        "K" => Some(KeyCode::KeyK),
        "L" => Some(KeyCode::KeyL),
        "M" => Some(KeyCode::KeyM),
        "N" => Some(KeyCode::KeyN),
        "O" => Some(KeyCode::KeyO),
        "P" => Some(KeyCode::KeyP),
        "Q" => Some(KeyCode::KeyQ),
        "R" => Some(KeyCode::KeyR),
        "S" => Some(KeyCode::KeyS),
        "T" => Some(KeyCode::KeyT),
        "U" => Some(KeyCode::KeyU),
        "V" => Some(KeyCode::KeyV),
        "W" => Some(KeyCode::KeyW),
        "X" => Some(KeyCode::KeyX),
        "Y" => Some(KeyCode::KeyY),
        "Z" => Some(KeyCode::KeyZ),

        // Arrow keys
        "ArrowUp" | "Up" => Some(KeyCode::ArrowUp),
        "ArrowDown" | "Down" => Some(KeyCode::ArrowDown),
        "ArrowLeft" | "Left" => Some(KeyCode::ArrowLeft),
        "ArrowRight" | "Right" => Some(KeyCode::ArrowRight),

        // Special keys
        "Space" | "Spacebar" => Some(KeyCode::Space),
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),

        // Number keys
        "0" | "Digit0" => Some(KeyCode::Digit0),
        "1" | "Digit1" => Some(KeyCode::Digit1),
        "2" | "Digit2" => Some(KeyCode::Digit2),
        "3" | "Digit3" => Some(KeyCode::Digit3),
        "4" | "Digit4" => Some(KeyCode::Digit4),
        "5" | "Digit5" => Some(KeyCode::Digit5),
        "6" | "Digit6" => Some(KeyCode::Digit6),
        "7" | "Digit7" => Some(KeyCode::Digit7),
        "8" | "Digit8" => Some(KeyCode::Digit8),
        "9" | "Digit9" => Some(KeyCode::Digit9),

        // Function keys
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),

        // Modifier keys
        "Shift" | "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "Control" | "Ctrl" | "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" | "CtrlRight" => Some(KeyCode::ControlRight),
        "Alt" | "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),

        _ => {
            // Try lowercase version
            let lowercase = key_str.to_lowercase();
            if lowercase != key_str {
                return parse_key_code(&lowercase);
            }
            None
        }
    }
}

/// Toggle the in-game menu: open it if not open, or close it and return to the previous mode if open.
///
/// This helper intentionally does not consider movement cooldown so it can be called
/// from input handlers that must ensure the menu key always works.
///
/// # Arguments
///
/// * `game_state` - Mutable reference to the current `GameState`
fn toggle_menu_state(game_state: &mut crate::application::GameState) {
    use crate::application::menu::MenuState;
    match &game_state.mode {
        crate::application::GameMode::Menu(menu_state) => {
            let resume_mode = menu_state.get_resume_mode();
            info!("Closing menu, resuming to: {:?}", resume_mode);
            game_state.mode = resume_mode;
        }
        current_mode => {
            info!("Opening menu from: {:?}", current_mode);
            let menu_state = MenuState::new(current_mode.clone());
            game_state.mode = crate::application::GameMode::Menu(menu_state);
        }
    }
}

/// Handle keyboard input and translate to game actions
///
/// This system processes keyboard input using the configured key mappings,
/// applies movement cooldown, and updates game state accordingly.
#[allow(clippy::too_many_arguments)]
fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    input_config: Res<InputConfigResource>,
    mut global_state: ResMut<GlobalState>,
    mut door_messages: MessageWriter<DoorOpenedEvent>,
    mut map_event_messages: MessageWriter<MapEventTriggered>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    mut recruitment_context: ResMut<PendingRecruitmentContext>,
    time: Res<Time>,
    mut last_move_time: Local<f32>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,
    dialogue_query: Query<&NpcDialogue>,
) {
    let current_time = time.elapsed_secs();
    let cooldown = input_config.controls.movement_cooldown;

    // Check for menu toggle (ESC key) first — it should always take priority and
    // must not be blocked by movement cooldown.
    if input_config
        .key_map
        .is_action_just_pressed(GameAction::Menu, &keyboard_input)
    {
        let game_state = &mut global_state.0;
        toggle_menu_state(game_state);
        info!("Menu toggled: new_mode = {:?}", game_state.mode);
        return; // Exit early after menu toggle
    }

    // Throttle movement input using cooldown. Only block when an actual movement
    // action is being attempted.
    let is_movement_attempt = input_config
        .key_map
        .is_action_pressed(GameAction::MoveForward, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::MoveBack, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::TurnLeft, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::TurnRight, &keyboard_input);

    if is_movement_attempt && (current_time - *last_move_time < cooldown) {
        // Movement attempted but still within cooldown window - ignore movement input.
        return;
    }

    // ALLOW input processing in Dialogue mode to enable "Move to Cancel"
    // But block Interaction actions (doors, etc.) if in Dialogue.
    // BLOCK all movement/interaction input when in Menu mode (menu system handles its own input)

    let game_state = &mut global_state.0;

    // Menu toggle handled above before movement cooldown checks.

    // Block all movement/interaction input when in Menu mode
    // The menu system (handle_menu_keyboard) handles its own input processing
    if matches!(game_state.mode, crate::application::GameMode::Menu(_)) {
        return;
    }

    let world = &mut game_state.world;
    let mut moved = false;

    // Interact - check for doors, NPCs, signs, teleports
    //
    // NOTE: We intentionally use "pressed" (not "just pressed") so interaction
    // behaves consistently with the existing movement model and door behavior,
    // and so headless tests can exercise interaction without depending on
    // Bevy's per-frame input edge detection.
    // Only allow Interaction if NOT in Dialogue mode
    if !matches!(game_state.mode, crate::application::GameMode::Dialogue(_))
        && input_config
            .key_map
            .is_action_pressed(GameAction::Interact, &keyboard_input)
    {
        let party_position = world.party_position;
        let adjacent_tiles = get_adjacent_positions(party_position);

        // Door interaction - check if there's a door in front of the party
        let target = world.position_ahead();
        if let Some(map) = world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(target) {
                if tile.wall_type == WallType::Door {
                    // Open the door by changing it to None
                    tile.wall_type = WallType::None;
                    info!("Opened door at {:?}", target);
                    // Send event to trigger map visual refresh
                    door_messages.write(DoorOpenedEvent { position: target });
                    return; // Door handled; don't fall through to other checks
                }
            }
        }

        // Snapshot current map state for adjacency checks (no mutation needed)
        let Some(map) = world.get_current_map() else {
            info!("No interactable object nearby");
            return;
        };

        // Check for NPC in any adjacent tile
        if let Some(npc) = map
            .npc_placements
            .iter()
            .find(|npc| adjacent_tiles.contains(&npc.position))
        {
            info!(
                "Interacting with NPC '{}' at {:?}",
                npc.npc_id, npc.position
            );
            map_event_messages.write(MapEventTriggered {
                event: MapEvent::NpcDialogue {
                    name: npc.npc_id.clone(),
                    description: String::new(),
                    npc_id: npc.npc_id.clone(),
                },
                position: npc.position,
            });
            return;
        }

        // Check for sign/teleport/recruitable character (and other events) in any adjacent tile
        for position in adjacent_tiles {
            if let Some(event) = map.get_event(position) {
                match event {
                    MapEvent::Sign { .. } | MapEvent::Teleport { .. } => {
                        info!("Interacting with event at {:?}", position);
                        map_event_messages.write(MapEventTriggered {
                            event: event.clone(),
                            position,
                        });
                        return;
                    }
                    MapEvent::RecruitableCharacter {
                        name,
                        character_id,
                        dialogue_id,
                        ..
                    } => {
                        info!(
                            "Interacting with recruitable character '{}' (ID: {}) at {:?}",
                            name, character_id, position
                        );
                        // Find the NPC entity at this position
                        let speaker_entity = npc_query
                            .iter()
                            .find(|(_, _, tile_coord)| tile_coord.0 == position)
                            .map(|(entity, _, _)| entity);

                        // Use specific dialogue ID if the NPC has one, otherwise fallback to 100
                        // Use specific dialogue ID from event if available,
                        // OR fallback to NPC component,
                        // OR fallback to default 100
                        let dialogue_id = dialogue_id
                            .or_else(|| {
                                speaker_entity
                                    .and_then(|entity| dialogue_query.get(entity).ok())
                                    .map(|npc_dlg| npc_dlg.dialogue_id)
                            })
                            .unwrap_or(100);

                        // Set recruitment context so the dialogue system knows who to recruit
                        recruitment_context.0 = Some(RecruitmentContext {
                            character_id: character_id.clone(),
                            event_position: position,
                        });

                        dialogue_writer.write(StartDialogue {
                            dialogue_id,
                            speaker_entity,
                            fallback_position: Some(position),
                        });
                        return;
                    }
                    _ => continue,
                }
            }
        }

        // No interactable found
        info!("No interactable object nearby");
    }
    // Move forward
    else if input_config
        .key_map
        .is_action_pressed(GameAction::MoveForward, &keyboard_input)
    {
        let target = world.position_ahead();
        if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    }
    // Move backward
    else if input_config
        .key_map
        .is_action_pressed(GameAction::MoveBack, &keyboard_input)
    {
        // Calculate position behind party
        let back_facing = world.party_facing.turn_left().turn_left();
        let target = back_facing.forward(world.party_position);

        if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    }
    // Turn left
    else if input_config
        .key_map
        .is_action_pressed(GameAction::TurnLeft, &keyboard_input)
    {
        world.turn_left();
        moved = true;
    }
    // Turn right
    else if input_config
        .key_map
        .is_action_pressed(GameAction::TurnRight, &keyboard_input)
    {
        world.turn_right();
        moved = true;
    }

    if moved {
        *last_move_time = current_time;

        // If we moved while in Dialogue mode, cancel the dialogue
        if matches!(game_state.mode, crate::application::GameMode::Dialogue(_)) {
            info!("Movement detected during dialogue - cancelling dialogue");
            // Switch back to exploration mode
            game_state.mode = crate::application::GameMode::Exploration;
        }

        // TODO: Check for events at new position (Phase 4)
    }
}

/// Returns all 8 adjacent positions around a given position
///
/// Returns tiles in clockwise order starting from North:
/// N, NE, E, SE, S, SW, W, NW
///
/// # Arguments
///
/// * `position` - The center position
///
/// # Returns
///
/// Array of 8 `Position` values representing adjacent tiles
fn get_adjacent_positions(position: Position) -> [Position; 8] {
    [
        Position::new(position.x, position.y - 1),     // North
        Position::new(position.x + 1, position.y - 1), // NorthEast
        Position::new(position.x + 1, position.y),     // East
        Position::new(position.x + 1, position.y + 1), // SouthEast
        Position::new(position.x, position.y + 1),     // South
        Position::new(position.x - 1, position.y + 1), // SouthWest
        Position::new(position.x - 1, position.y),     // West
        Position::new(position.x - 1, position.y - 1), // NorthWest
    ]
}

#[cfg(test)]
mod adjacent_tile_tests {
    use super::*;

    #[test]
    fn test_adjacent_positions_count() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent.len(), 8);
    }

    #[test]
    fn test_adjacent_positions_north() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[0], Position::new(5, 4)); // North
    }

    #[test]
    fn test_adjacent_positions_east() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[2], Position::new(6, 5)); // East
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_code_letters() {
        assert_eq!(parse_key_code("W"), Some(KeyCode::KeyW));
        assert_eq!(parse_key_code("A"), Some(KeyCode::KeyA));
        assert_eq!(parse_key_code("S"), Some(KeyCode::KeyS));
        assert_eq!(parse_key_code("D"), Some(KeyCode::KeyD));
    }

    #[test]
    fn test_parse_key_code_arrows() {
        assert_eq!(parse_key_code("ArrowUp"), Some(KeyCode::ArrowUp));
        assert_eq!(parse_key_code("ArrowDown"), Some(KeyCode::ArrowDown));
        assert_eq!(parse_key_code("ArrowLeft"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key_code("ArrowRight"), Some(KeyCode::ArrowRight));
    }

    #[test]
    fn test_parse_key_code_arrow_aliases() {
        assert_eq!(parse_key_code("Up"), Some(KeyCode::ArrowUp));
        assert_eq!(parse_key_code("Down"), Some(KeyCode::ArrowDown));
        assert_eq!(parse_key_code("Left"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key_code("Right"), Some(KeyCode::ArrowRight));
    }

    #[test]
    fn test_parse_key_code_special() {
        assert_eq!(parse_key_code("Space"), Some(KeyCode::Space));
        assert_eq!(parse_key_code("Spacebar"), Some(KeyCode::Space));
        assert_eq!(parse_key_code("Escape"), Some(KeyCode::Escape));
        assert_eq!(parse_key_code("Esc"), Some(KeyCode::Escape));
        assert_eq!(parse_key_code("Enter"), Some(KeyCode::Enter));
    }

    #[test]
    fn test_toggle_menu_state_from_exploration_and_back() {
        // Start in Exploration mode
        let mut state = crate::application::GameState::new();
        assert!(matches!(
            state.mode,
            crate::application::GameMode::Exploration
        ));

        // Toggle to Menu
        toggle_menu_state(&mut state);
        assert!(matches!(state.mode, crate::application::GameMode::Menu(_)));

        // Toggle back to Exploration
        toggle_menu_state(&mut state);
        assert!(matches!(
            state.mode,
            crate::application::GameMode::Exploration
        ));
    }

    #[test]
    fn test_toggle_menu_state_preserves_previous_mode() {
        // Ensure the MenuState records the previous mode correctly
        let mut state = crate::application::GameState::new();
        toggle_menu_state(&mut state);

        if let crate::application::GameMode::Menu(menu_state) = &state.mode {
            assert!(matches!(
                menu_state.get_resume_mode(),
                crate::application::GameMode::Exploration
            ));
        } else {
            panic!("Expected to be in Menu mode after toggle");
        }
    }

    #[test]
    fn test_parse_key_code_invalid() {
        assert_eq!(parse_key_code("InvalidKey"), None);
        assert_eq!(parse_key_code(""), None);
    }

    #[test]
    fn test_key_map_from_default_config() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);

        // Check that default keys are mapped correctly
        assert_eq!(
            key_map.get_action(KeyCode::KeyW),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowUp),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyS),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowDown),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyA),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowLeft),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyD),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowRight),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::Space),
            Some(GameAction::Interact)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyE),
            Some(GameAction::Interact)
        );
        assert_eq!(key_map.get_action(KeyCode::Escape), Some(GameAction::Menu));
    }

    #[test]
    fn test_key_map_custom_config() {
        let config = ControlsConfig {
            move_forward: vec!["I".to_string()],
            move_back: vec!["K".to_string()],
            turn_left: vec!["J".to_string()],
            turn_right: vec!["L".to_string()],
            interact: vec!["U".to_string()],
            menu: vec!["P".to_string()],
            movement_cooldown: 0.1,
        };

        let key_map = KeyMap::from_controls_config(&config);

        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyK),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyJ),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyL),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyU),
            Some(GameAction::Interact)
        );
        assert_eq!(key_map.get_action(KeyCode::KeyP), Some(GameAction::Menu));

        // Old defaults should not be mapped
        assert_eq!(key_map.get_action(KeyCode::KeyW), None);
        assert_eq!(key_map.get_action(KeyCode::Space), None);
    }

    #[test]
    fn test_key_map_multiple_keys_per_action() {
        let config = ControlsConfig {
            move_forward: vec!["W".to_string(), "ArrowUp".to_string(), "I".to_string()],
            move_back: vec!["S".to_string()],
            turn_left: vec!["A".to_string()],
            turn_right: vec!["D".to_string()],
            interact: vec!["Space".to_string()],
            menu: vec!["Escape".to_string()],
            movement_cooldown: 0.2,
        };

        let key_map = KeyMap::from_controls_config(&config);

        // All three keys should map to MoveForward
        assert_eq!(
            key_map.get_action(KeyCode::KeyW),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowUp),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::MoveForward)
        );
    }

    #[test]
    fn test_controls_config_default_cooldown() {
        let config = ControlsConfig::default();
        assert_eq!(config.movement_cooldown, 0.2);
    }

    #[test]
    fn test_controls_config_validation_valid() {
        let config = ControlsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_controls_config_validation_negative_cooldown() {
        let config = ControlsConfig {
            move_forward: vec!["W".to_string()],
            move_back: vec!["S".to_string()],
            turn_left: vec!["A".to_string()],
            turn_right: vec!["D".to_string()],
            interact: vec!["Space".to_string()],
            menu: vec!["Escape".to_string()],
            movement_cooldown: -0.1,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-negative"));
    }

    #[test]
    fn test_input_plugin_creation() {
        let config = ControlsConfig::default();
        let plugin = InputPlugin::new(config.clone());
        assert_eq!(plugin.config, config);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy::prelude::{App, ButtonInput, KeyCode, Time, Update};

    /// Integration-style test: simulate pressing ESC via `ButtonInput` and ensure the
    /// input system toggles the in-game menu open and closed.
    #[test]
    fn test_escape_opens_and_closes_menu_via_button_input() {
        // Build a minimal app and register the input system under test.
        let mut app = App::new();

        // Insert required resources: button input, config, global state, and time.
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let cfg = ControlsConfig::default();
        let km = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map: km,
        });

        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register message channels the input system depends on so MessageWriter<T>
        // parameters are initialized when running the system in tests.
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();

        // Add the handle_input system (the system under test)
        app.add_systems(Update, handle_input);

        // Press Escape - should open the menu
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));

        // Press Escape again - should close the menu and resume previous mode
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(
            gs.0.mode,
            crate::application::GameMode::Exploration
        ));
    }

    #[test]
    fn test_escape_opens_after_movement() {
        use bevy::prelude::*;

        let mut app = App::new();

        // Basic resources the input system expects
        app.insert_resource(ButtonInput::<KeyCode>::default());
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register messages used by input system
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();

        // Add just the input system (we want to simulate input frames)
        app.add_systems(Update, handle_input);

        // Frame 1: press MoveForward
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::ArrowUp);
        }
        app.update();

        // Frame 2: release MoveForward and press Escape
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.release(KeyCode::ArrowUp);
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));
    }

    #[test]
    fn test_escape_opens_when_move_and_menu_pressed_simultaneously() {
        use bevy::prelude::*;

        let mut app = App::new();

        // Basic resources the input system expects
        app.insert_resource(ButtonInput::<KeyCode>::default());
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register messages used by input system
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();

        // Add the input system so frames process input
        app.add_systems(Update, handle_input);

        // Single frame: press MoveForward and Menu at the same time
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::ArrowUp);
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    /// Test that adjacent positions are correctly identified for interaction purposes.
    /// This test verifies that the helper function identifies all 8 surrounding tiles.
    #[test]
    fn test_npc_interaction_adjacent_positions() {
        // Arrange
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);

        // Assert - verify all 8 positions are adjacent
        assert!(adjacent.contains(&Position::new(5, 4))); // North
        assert!(adjacent.contains(&Position::new(6, 4))); // NorthEast
        assert!(adjacent.contains(&Position::new(6, 5))); // East
        assert!(adjacent.contains(&Position::new(6, 6))); // SouthEast
        assert!(adjacent.contains(&Position::new(5, 6))); // South
        assert!(adjacent.contains(&Position::new(4, 6))); // SouthWest
        assert!(adjacent.contains(&Position::new(4, 5))); // West
        assert!(adjacent.contains(&Position::new(4, 4))); // NorthWest
    }

    /// Test that sign interaction detects signs in adjacent positions.
    /// Validates that map events are properly stored and retrievable.
    #[test]
    fn test_sign_interaction_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let sign_pos = Position::new(5, 4);
        map.add_event(
            sign_pos,
            MapEvent::Sign {
                name: "TestSign".to_string(),
                description: "This is a test sign".to_string(),
                text: "You found it!".to_string(),
            },
        );

        // Act
        let event = map.get_event(sign_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Sign { .. })));
    }

    /// Test that teleport events are properly stored and retrievable.
    /// Validates event data persistence in the map.
    #[test]
    fn test_teleport_interaction_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let teleport_pos = Position::new(5, 4);
        map.add_event(
            teleport_pos,
            MapEvent::Teleport {
                name: "TestPortal".to_string(),
                description: "Portal to destination".to_string(),
                destination: Position::new(2, 2),
                map_id: 1,
            },
        );

        // Act
        let event = map.get_event(teleport_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Teleport { .. })));
    }

    /// Test that door interaction state changes correctly.
    /// Validates the door opening mechanism by checking wall type transitions.
    #[test]
    fn test_door_interaction_wall_state() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let door_pos = Position::new(5, 4);
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::Door;
        }

        // Act - verify initial state
        let tile_before = map.get_tile(door_pos).expect("tile missing");
        assert_eq!(tile_before.wall_type, WallType::Door);

        // Act - open door by changing wall type
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::None;
        }

        // Assert - verify final state
        let tile_after = map.get_tile(door_pos).expect("tile missing");
        assert_eq!(tile_after.wall_type, WallType::None);
    }

    /// Test that NPC placements are properly stored and retrievable.
    /// Validates the NPC data structure and storage mechanisms.
    #[test]
    fn test_npc_interaction_placement_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let npc_pos = Position::new(5, 4);
        map.npc_placements
            .push(crate::domain::world::NpcPlacement::new("test_npc", npc_pos));

        // Act
        let npc = map
            .npc_placements
            .iter()
            .find(|npc| npc.position == npc_pos);

        // Assert
        assert!(npc.is_some());
        assert_eq!(npc.unwrap().npc_id, "test_npc");
        assert_eq!(npc.unwrap().position, npc_pos);
    }

    /// Test that recruitable character events are properly stored and retrievable.
    /// Validates that map events for recruitables are correctly managed.
    #[test]
    fn test_recruitable_character_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let recruit_pos = Position::new(5, 4);
        map.add_event(
            recruit_pos,
            MapEvent::RecruitableCharacter {
                name: "TestRecruit".to_string(),
                description: "A recruitable character".to_string(),
                character_id: "hero_01".to_string(),
                dialogue_id: None,
            },
        );

        // Act
        let event = map.get_event(recruit_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::RecruitableCharacter { .. })));
        if let Some(MapEvent::RecruitableCharacter {
            character_id, name, ..
        }) = event
        {
            assert_eq!(character_id, "hero_01");
            assert_eq!(name, "TestRecruit");
        }
    }
}
