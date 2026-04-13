// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Per-frame decoded input intent helpers.
//!
//! This module separates raw Bevy button polling from higher-level input
//! execution by decoding all relevant inputs once into a small value object.
//! Later input-system phases can operate on `FrameInputIntent` instead of
//! repeatedly querying the active key map and window state inline.

use crate::game::systems::input::world_click::mouse_center_interact_pressed;
use crate::game::systems::input::{GameAction, KeyMap};
use bevy::prelude::*;
use bevy::window::Window;

/// Decoded player input for a single frame.
///
/// This intent is intentionally small and behavior-oriented: it captures the
/// actions the input system cares about after raw keyboard and mouse state have
/// been interpreted through the active key bindings and the exploration
/// centre-screen click heuristic.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::frame_input::FrameInputIntent;
///
/// let intent = FrameInputIntent {
///     menu_toggle: true,
///     ..FrameInputIntent::default()
/// };
///
/// assert!(intent.menu_toggle);
/// assert!(!intent.move_forward);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInputIntent {
    /// Whether the menu toggle action was requested this frame.
    pub menu_toggle: bool,
    /// Whether the inventory toggle action was requested this frame.
    pub inventory_toggle: bool,
    /// Whether the automap toggle action was requested this frame.
    pub automap_toggle: bool,
    /// Whether the rest action was requested this frame.
    pub rest: bool,
    /// Whether forward movement is currently being attempted.
    pub move_forward: bool,
    /// Whether backward movement is currently being attempted.
    pub move_back: bool,
    /// Whether turning left is currently being attempted.
    pub turn_left: bool,
    /// Whether turning right is currently being attempted.
    pub turn_right: bool,
    /// Whether keyboard interaction is currently being attempted.
    pub interact: bool,
    /// Whether the centre-screen mouse interaction heuristic fired this frame.
    pub mouse_center_interact: bool,
    /// Whether the full-screen game log toggle was requested this frame.
    pub game_log_toggle: bool,
    /// Whether the exploration spell-casting menu was requested this frame.
    pub cast: bool,
    /// Whether the Spell Book management screen was requested this frame.
    pub spell_book_toggle: bool,
    /// Whether the character sheet toggle was requested this frame.
    pub character_sheet_toggle: bool,
}

impl FrameInputIntent {
    /// Returns whether any movement-oriented action is being attempted.
    ///
    /// This mirrors the existing movement cooldown gate semantics in the input
    /// system, which consider forward, backward, and turning actions as the
    /// set of movement-throttled inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::frame_input::FrameInputIntent;
    ///
    /// let intent = FrameInputIntent {
    ///     turn_left: true,
    ///     ..FrameInputIntent::default()
    /// };
    ///
    /// assert!(intent.is_movement_attempt());
    /// ```
    pub fn is_movement_attempt(self) -> bool {
        self.move_forward || self.move_back || self.turn_left || self.turn_right
    }

    /// Returns whether any interaction action is being attempted.
    ///
    /// This combines keyboard interaction with the existing centre-screen mouse
    /// interaction fallback so later phases can treat both routes uniformly.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::frame_input::FrameInputIntent;
    ///
    /// let intent = FrameInputIntent {
    ///     mouse_center_interact: true,
    ///     ..FrameInputIntent::default()
    /// };
    ///
    /// assert!(intent.is_interact_attempt());
    /// ```
    pub fn is_interact_attempt(self) -> bool {
        self.interact || self.mouse_center_interact
    }
}

/// Decodes all relevant per-frame input state into a `FrameInputIntent`.
///
/// The decoder applies the configured `KeyMap` to keyboard input and delegates
/// exploration mouse fallback decoding to the dedicated world-click helper.
///
/// Toggle-style actions use `just_pressed` semantics:
///
/// - menu
/// - inventory
/// - automap
/// - rest
///
/// Continuous actions use `pressed` semantics:
///
/// - movement
/// - interact
///
/// Mouse centre interaction is resolved through the shared world-click helper so
/// the frame decoder does not own primary-window lookup policy or the
/// centre-third heuristic directly.
///
/// # Arguments
///
/// * `key_map` - Compiled control bindings for action decoding
/// * `keyboard_input` - Current keyboard button state
/// * `mouse_buttons` - Current mouse button state
/// * `primary_window` - Optional primary window used for centre-click decoding
///
/// # Returns
///
/// A fully-decoded `FrameInputIntent` for the current frame.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::{FrameInputIntent, KeyMap};
/// use antares::game::systems::input::frame_input::decode_frame_input;
/// use antares::sdk::game_config::ControlsConfig;
/// use bevy::prelude::{ButtonInput, KeyCode, MouseButton};
///
/// let key_map = KeyMap::from_controls_config(&ControlsConfig::default());
/// let mut keyboard = ButtonInput::<KeyCode>::default();
/// let mouse = ButtonInput::<MouseButton>::default();
///
/// keyboard.press(KeyCode::Escape);
///
/// let intent = decode_frame_input(&key_map, &keyboard, &mouse, None);
///
/// assert!(intent.menu_toggle);
/// assert!(!intent.move_forward);
/// ```
pub fn decode_frame_input(
    key_map: &KeyMap,
    keyboard_input: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    primary_window: Option<&Window>,
) -> FrameInputIntent {
    FrameInputIntent {
        menu_toggle: key_map.is_action_just_pressed(GameAction::Menu, keyboard_input),
        inventory_toggle: key_map.is_action_just_pressed(GameAction::Inventory, keyboard_input),
        automap_toggle: key_map.is_action_just_pressed(GameAction::Automap, keyboard_input),
        rest: key_map.is_action_just_pressed(GameAction::Rest, keyboard_input),
        move_forward: key_map.is_action_pressed(GameAction::MoveForward, keyboard_input),
        move_back: key_map.is_action_pressed(GameAction::MoveBack, keyboard_input),
        turn_left: key_map.is_action_pressed(GameAction::TurnLeft, keyboard_input),
        turn_right: key_map.is_action_pressed(GameAction::TurnRight, keyboard_input),
        interact: key_map.is_action_pressed(GameAction::Interact, keyboard_input),
        mouse_center_interact: mouse_center_interact_pressed(mouse_buttons, primary_window),
        game_log_toggle: key_map.is_action_just_pressed(GameAction::GameLog, keyboard_input),
        cast: key_map.is_action_just_pressed(GameAction::Cast, keyboard_input),
        spell_book_toggle: key_map
            .is_action_just_pressed(GameAction::OpenSpellBook, keyboard_input),
        character_sheet_toggle: key_map
            .is_action_just_pressed(GameAction::CharacterSheet, keyboard_input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk::game_config::ControlsConfig;
    use bevy::prelude::Vec2;

    fn default_key_map() -> KeyMap {
        KeyMap::from_controls_config(&ControlsConfig::default())
    }

    fn window_with_cursor(width: u32, height: u32, cursor: Option<Vec2>) -> Window {
        let mut window = Window::default();
        window.resolution.set_physical_resolution(width, height);
        window.set_cursor_position(cursor);
        window
    }

    #[test]
    fn test_frame_input_intent_default_has_no_actions() {
        let intent = FrameInputIntent::default();

        assert!(!intent.menu_toggle);
        assert!(!intent.inventory_toggle);
        assert!(!intent.automap_toggle);
        assert!(!intent.rest);
        assert!(!intent.move_forward);
        assert!(!intent.move_back);
        assert!(!intent.turn_left);
        assert!(!intent.turn_right);
        assert!(!intent.interact);
        assert!(!intent.mouse_center_interact);
        assert!(!intent.game_log_toggle);
        assert!(!intent.cast);
        assert!(!intent.spell_book_toggle);
        assert!(!intent.character_sheet_toggle);
    }

    #[test]
    fn test_frame_input_intent_default_has_no_character_sheet_toggle() {
        let intent = FrameInputIntent::default();
        assert!(!intent.character_sheet_toggle);
    }

    #[test]
    fn test_frame_input_intent_is_movement_attempt_with_turn() {
        let intent = FrameInputIntent {
            turn_right: true,
            ..FrameInputIntent::default()
        };

        assert!(intent.is_movement_attempt());
    }

    #[test]
    fn test_frame_input_intent_is_movement_attempt_false_when_idle() {
        let intent = FrameInputIntent::default();

        assert!(!intent.is_movement_attempt());
    }

    #[test]
    fn test_frame_input_intent_is_interact_attempt_with_keyboard_interact() {
        let intent = FrameInputIntent {
            interact: true,
            ..FrameInputIntent::default()
        };

        assert!(intent.is_interact_attempt());
    }

    #[test]
    fn test_frame_input_intent_is_interact_attempt_with_mouse_center_interact() {
        let intent = FrameInputIntent {
            mouse_center_interact: true,
            ..FrameInputIntent::default()
        };

        assert!(intent.is_interact_attempt());
    }

    #[test]
    fn test_decode_frame_input_sets_toggle_actions_from_just_pressed_keys() {
        let key_map = default_key_map();
        let mut keyboard = ButtonInput::<KeyCode>::default();
        let mouse = ButtonInput::<MouseButton>::default();

        keyboard.press(KeyCode::Escape);
        keyboard.press(KeyCode::KeyI);
        keyboard.press(KeyCode::KeyM);
        keyboard.press(KeyCode::KeyR);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, None);

        assert!(intent.menu_toggle);
        assert!(intent.inventory_toggle);
        assert!(intent.automap_toggle);
        assert!(intent.rest);
    }

    #[test]
    fn test_decode_frame_input_sets_continuous_movement_and_interact_actions() {
        let key_map = default_key_map();
        let mut keyboard = ButtonInput::<KeyCode>::default();
        let mouse = ButtonInput::<MouseButton>::default();

        keyboard.press(KeyCode::KeyW);
        keyboard.press(KeyCode::KeyS);
        keyboard.press(KeyCode::KeyA);
        keyboard.press(KeyCode::KeyD);
        keyboard.press(KeyCode::KeyE);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, None);

        assert!(intent.move_forward);
        assert!(intent.move_back);
        assert!(intent.turn_left);
        assert!(intent.turn_right);
        assert!(intent.interact);
    }

    #[test]
    fn test_decode_frame_input_uses_custom_key_bindings() {
        let config = ControlsConfig {
            move_forward: vec!["I".to_string()],
            menu: vec!["P".to_string()],
            inventory: vec!["F".to_string()],
            rest: vec!["G".to_string()],
            automap: vec!["U".to_string()],
            interact: vec!["O".to_string()],
            game_log: vec!["H".to_string()],
            character_sheet: vec!["F4".to_string()],
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&config);
        let mut keyboard = ButtonInput::<KeyCode>::default();
        let mouse = ButtonInput::<MouseButton>::default();

        keyboard.press(KeyCode::KeyI);
        keyboard.press(KeyCode::KeyP);
        keyboard.press(KeyCode::KeyF);
        keyboard.press(KeyCode::KeyG);
        keyboard.press(KeyCode::KeyU);
        keyboard.press(KeyCode::KeyO);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, None);

        assert!(intent.move_forward);
        assert!(intent.menu_toggle);
        assert!(intent.inventory_toggle);
        assert!(intent.rest);
        assert!(intent.automap_toggle);
        assert!(intent.interact);
    }

    #[test]
    fn test_decode_frame_input_marks_mouse_center_interact_when_cursor_is_centered() {
        let key_map = default_key_map();
        let keyboard = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(450.0, 300.0)));

        mouse.press(MouseButton::Left);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, Some(&window));

        assert!(intent.mouse_center_interact);
        assert!(intent.is_interact_attempt());
    }

    #[test]
    fn test_decode_frame_input_does_not_mark_mouse_center_interact_outside_center_third() {
        let key_map = default_key_map();
        let keyboard = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(100.0, 100.0)));

        mouse.press(MouseButton::Left);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, Some(&window));

        assert!(!intent.mouse_center_interact);
    }

    #[test]
    fn test_decode_frame_input_does_not_mark_mouse_center_interact_without_window() {
        let key_map = default_key_map();
        let keyboard = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();

        mouse.press(MouseButton::Left);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, None);

        assert!(!intent.mouse_center_interact);
    }

    #[test]
    fn test_decode_frame_input_does_not_mark_mouse_center_interact_without_cursor() {
        let key_map = default_key_map();
        let keyboard = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, None);

        mouse.press(MouseButton::Left);

        let intent = decode_frame_input(&key_map, &keyboard, &mouse, Some(&window));

        assert!(!intent.mouse_center_interact);
    }
}
