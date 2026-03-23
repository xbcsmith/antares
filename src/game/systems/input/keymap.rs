// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Config-driven input key mapping helpers.
//!
//! This module owns the pure key-decoding and key-to-action mapping logic used
//! by the exploration input system. Keeping these helpers separate from the
//! Bevy system reduces the amount of non-system logic in `input.rs` and keeps
//! the direct unit tests close to the helpers they validate.

use crate::sdk::game_config::ControlsConfig;
use bevy::prelude::*;
use std::collections::HashMap;

/// Game actions that can be triggered by input.
///
/// These actions are intentionally limited to the low-level decoded actions
/// consumed by the input system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    /// Move forward in the current facing direction.
    MoveForward,
    /// Move backward relative to the current facing direction.
    MoveBack,
    /// Rotate counterclockwise.
    TurnLeft,
    /// Rotate clockwise.
    TurnRight,
    /// Interact with objects or events in the world.
    Interact,
    /// Open or close the game menu.
    Menu,
    /// Open or close the inventory screen.
    Inventory,
    /// Begin a party rest sequence.
    Rest,
    /// Open or close the automap overlay.
    Automap,
}

/// Key mapping structure for efficient input lookups.
///
/// `KeyMap` compiles string-based control bindings into `KeyCode` lookups so
/// the frame-by-frame input system can work with fast enum comparisons instead
/// of repeatedly parsing strings.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::KeyMap;
/// use antares::sdk::game_config::ControlsConfig;
///
/// let config = ControlsConfig::default();
/// let key_map = KeyMap::from_controls_config(&config);
///
/// assert!(key_map.get_action(bevy::prelude::KeyCode::KeyW).is_some());
/// ```
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Map from `KeyCode` to the action it triggers.
    bindings: HashMap<KeyCode, GameAction>,
}

impl KeyMap {
    /// Creates a compiled key map from a controls configuration.
    ///
    /// Invalid key names are ignored and logged as warnings.
    ///
    /// # Arguments
    ///
    /// * `config` - Controls configuration with string-based bindings
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::{GameAction, KeyMap};
    /// use antares::sdk::game_config::ControlsConfig;
    ///
    /// let config = ControlsConfig::default();
    /// let key_map = KeyMap::from_controls_config(&config);
    ///
    /// assert_eq!(
    ///     key_map.get_action(bevy::prelude::KeyCode::Escape),
    ///     Some(GameAction::Menu)
    /// );
    /// ```
    pub fn from_controls_config(config: &ControlsConfig) -> Self {
        let mut bindings = HashMap::new();

        insert_action_bindings(&mut bindings, &config.move_forward, GameAction::MoveForward);
        insert_action_bindings(&mut bindings, &config.move_back, GameAction::MoveBack);
        insert_action_bindings(&mut bindings, &config.turn_left, GameAction::TurnLeft);
        insert_action_bindings(&mut bindings, &config.turn_right, GameAction::TurnRight);
        insert_action_bindings(&mut bindings, &config.interact, GameAction::Interact);
        insert_action_bindings(&mut bindings, &config.menu, GameAction::Menu);
        insert_action_bindings(&mut bindings, &config.inventory, GameAction::Inventory);
        insert_action_bindings(&mut bindings, &config.rest, GameAction::Rest);
        insert_action_bindings(&mut bindings, &config.automap, GameAction::Automap);

        Self { bindings }
    }

    /// Returns the action bound to a specific key code.
    ///
    /// # Arguments
    ///
    /// * `key_code` - The key code to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(GameAction)` when a binding exists, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::{GameAction, KeyMap};
    /// use antares::sdk::game_config::ControlsConfig;
    /// use bevy::prelude::KeyCode;
    ///
    /// let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
    ///
    /// assert_eq!(key_map.get_action(KeyCode::KeyR), Some(GameAction::Rest));
    /// ```
    pub fn get_action(&self, key_code: KeyCode) -> Option<GameAction> {
        self.bindings.get(&key_code).copied()
    }

    /// Returns whether any key bound to the given action is currently pressed.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to query
    /// * `keyboard_input` - Current Bevy keyboard input state
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::{GameAction, KeyMap};
    /// use antares::sdk::game_config::ControlsConfig;
    /// use bevy::prelude::{ButtonInput, KeyCode};
    ///
    /// let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
    /// let mut input = ButtonInput::<KeyCode>::default();
    /// input.press(KeyCode::KeyW);
    ///
    /// assert!(key_map.is_action_pressed(GameAction::MoveForward, &input));
    /// ```
    pub fn is_action_pressed(
        &self,
        action: GameAction,
        keyboard_input: &ButtonInput<KeyCode>,
    ) -> bool {
        self.bindings.iter().any(|(key_code, bound_action)| {
            *bound_action == action && keyboard_input.pressed(*key_code)
        })
    }

    /// Returns whether any key bound to the given action was just pressed this frame.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to query
    /// * `keyboard_input` - Current Bevy keyboard input state
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::{GameAction, KeyMap};
    /// use antares::sdk::game_config::ControlsConfig;
    /// use bevy::prelude::{ButtonInput, KeyCode};
    ///
    /// let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
    /// let mut input = ButtonInput::<KeyCode>::default();
    /// input.press(KeyCode::Escape);
    ///
    /// assert!(key_map.is_action_just_pressed(GameAction::Menu, &input));
    /// ```
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

/// Parses a string key name into Bevy's `KeyCode`.
///
/// The parser accepts the canonical key names used in `ControlsConfig` plus a
/// set of compatibility aliases such as `"Up"` for `"ArrowUp"` and `"Esc"` for
/// `"Escape"`.
///
/// # Arguments
///
/// * `key_str` - String representation of the key
///
/// # Returns
///
/// Returns `Some(KeyCode)` if the key string is recognized, otherwise `None`.
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

        "ArrowUp" | "Up" => Some(KeyCode::ArrowUp),
        "ArrowDown" | "Down" => Some(KeyCode::ArrowDown),
        "ArrowLeft" | "Left" => Some(KeyCode::ArrowLeft),
        "ArrowRight" | "Right" => Some(KeyCode::ArrowRight),

        "Space" | "Spacebar" => Some(KeyCode::Space),
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),

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

        "Shift" | "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "Control" | "Ctrl" | "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" | "CtrlRight" => Some(KeyCode::ControlRight),
        "Alt" | "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),

        _ => {
            let lowercase = key_str.to_lowercase();
            if lowercase != key_str {
                return parse_key_code(&lowercase);
            }
            None
        }
    }
}

/// Inserts all valid key bindings for a single action into the compiled map.
///
/// Invalid key strings are ignored and logged so control parsing remains
/// tolerant of bad data while still surfacing configuration issues.
fn insert_action_bindings(
    bindings: &mut HashMap<KeyCode, GameAction>,
    key_strings: &[String],
    action: GameAction,
) {
    for key_str in key_strings {
        if let Some(key_code) = parse_key_code(key_str) {
            bindings.insert(key_code, action);
        } else {
            warn!("Invalid key code for {:?}: {}", action, key_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_map_rest_action() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);

        assert_eq!(
            key_map.get_action(KeyCode::KeyR),
            Some(GameAction::Rest),
            "KeyCode::KeyR must map to GameAction::Rest with default config"
        );
    }

    #[test]
    fn test_custom_rest_key() {
        let config = ControlsConfig {
            rest: vec!["F5".to_string()],
            ..Default::default()
        };
        let key_map = KeyMap::from_controls_config(&config);

        assert_eq!(
            key_map.get_action(KeyCode::F5),
            Some(GameAction::Rest),
            "F5 must map to GameAction::Rest when configured as rest key"
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyR),
            None,
            "KeyR must not be mapped when rest is overridden to F5"
        );
    }

    #[test]
    fn test_key_map_inventory_action() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);

        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::Inventory),
            "KeyCode::KeyI must map to GameAction::Inventory with default config"
        );
    }

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
            inventory: vec!["F".to_string()],
            rest: vec!["G".to_string()],
            automap: vec!["M".to_string()],
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
            inventory: vec!["F".to_string()],
            rest: vec!["R".to_string()],
            automap: vec!["M".to_string()],
            movement_cooldown: 0.2,
        };

        let key_map = KeyMap::from_controls_config(&config);

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
    fn test_is_action_pressed_detects_bound_key() {
        let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
        let mut keyboard_input = ButtonInput::<KeyCode>::default();
        keyboard_input.press(KeyCode::KeyW);

        assert!(key_map.is_action_pressed(GameAction::MoveForward, &keyboard_input));
        assert!(!key_map.is_action_pressed(GameAction::TurnLeft, &keyboard_input));
    }

    #[test]
    fn test_is_action_just_pressed_detects_bound_key() {
        let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
        let mut keyboard_input = ButtonInput::<KeyCode>::default();
        keyboard_input.press(KeyCode::Escape);

        assert!(key_map.is_action_just_pressed(GameAction::Menu, &keyboard_input));
        assert!(!key_map.is_action_just_pressed(GameAction::Inventory, &keyboard_input));
    }
}
