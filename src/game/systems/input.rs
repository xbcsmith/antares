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

use crate::domain::world::WallType;
use crate::game::resources::GlobalState;
use crate::game::systems::map::DoorOpenedEvent;
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

/// Handle keyboard input and translate to game actions
///
/// This system processes keyboard input using the configured key mappings,
/// applies movement cooldown, and updates game state accordingly.
fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    input_config: Res<InputConfigResource>,
    mut global_state: ResMut<GlobalState>,
    mut door_messages: MessageWriter<DoorOpenedEvent>,
    time: Res<Time>,
    mut last_move_time: Local<f32>,
) {
    let current_time = time.elapsed_secs();
    let cooldown = input_config.controls.movement_cooldown;

    // Check cooldown for movement actions
    if current_time - *last_move_time < cooldown {
        return;
    }

    let game_state = &mut global_state.0;
    let world = &mut game_state.world;
    let mut moved = false;

    // Door interaction - check if interact action is triggered
    if input_config
        .key_map
        .is_action_just_pressed(GameAction::Interact, &keyboard_input)
    {
        let target = world.position_ahead();
        if let Some(map) = world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(target) {
                if tile.wall_type == WallType::Door {
                    // Open the door by changing it to None
                    tile.wall_type = WallType::None;
                    info!("Opened door at {:?}", target);
                    // Send event to trigger map visual refresh
                    door_messages.write(DoorOpenedEvent { position: target });
                    moved = true; // Trigger time update
                }
            }
        }
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
        // TODO: Check for events at new position (Phase 4)
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
