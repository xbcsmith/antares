// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Exploration world-click fallback helpers.
//!
//! This module isolates the current mouse-based exploration interaction fallback
//! so the rest of the input system does not need to know about primary-window
//! lookup details, cursor validation, or the centre-third heuristic.
//!
//! The helper here intentionally preserves the current behavior:
//!
//! - only left-clicks are considered
//! - only the primary window is considered
//! - the cursor must be present
//! - the click must land inside the centre third of the window on both axes
//!
//! Keeping this logic in one small module gives the project a clean seam for a
//! future upgrade to mesh/world picking without reopening the whole input
//! system.

use bevy::prelude::{ButtonInput, MouseButton, Vec2, Window};

/// Returns whether the current frame contains an exploration centre-screen click.
///
/// This preserves the existing exploration fallback heuristic used by the input
/// system: a left mouse click inside the centre third of the primary window is
/// treated as a world interaction request.
///
/// # Arguments
///
/// * `mouse_buttons` - Current mouse button state
/// * `primary_window` - Optional primary window to inspect
///
/// # Returns
///
/// `true` if the frame contains a qualifying centre-screen click, otherwise
/// `false`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::world_click::mouse_center_interact_pressed;
/// use bevy::prelude::{ButtonInput, MouseButton};
///
/// let mouse = ButtonInput::<MouseButton>::default();
///
/// assert!(!mouse_center_interact_pressed(&mouse, None));
/// ```
pub fn mouse_center_interact_pressed(
    mouse_buttons: &ButtonInput<MouseButton>,
    primary_window: Option<&Window>,
) -> bool {
    let Some(window) = primary_window else {
        return false;
    };

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return false;
    }

    let Some(cursor_position) = window.cursor_position() else {
        return false;
    };

    is_cursor_in_center_third(window, cursor_position)
}

/// Returns whether the given cursor position lies within the centre third of a
/// window on both axes.
///
/// # Arguments
///
/// * `window` - Window whose visible bounds define the heuristic
/// * `cursor_position` - Cursor position to test
///
/// # Returns
///
/// `true` if the cursor lies within the centre third rectangle of the window,
/// otherwise `false`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::world_click::is_cursor_in_center_third;
/// use bevy::prelude::{Vec2, Window};
///
/// let mut window = Window::default();
/// window.resolution.set_physical_resolution(900, 600);
///
/// assert!(is_cursor_in_center_third(&window, Vec2::new(450.0, 300.0)));
/// assert!(!is_cursor_in_center_third(&window, Vec2::new(50.0, 50.0)));
/// ```
pub fn is_cursor_in_center_third(window: &Window, cursor_position: Vec2) -> bool {
    let width = window.width();
    let height = window.height();
    let center_left = width / 3.0;
    let center_right = width * (2.0 / 3.0);
    let center_top = height / 3.0;
    let center_bottom = height * (2.0 / 3.0);

    cursor_position.x >= center_left
        && cursor_position.x <= center_right
        && cursor_position.y >= center_top
        && cursor_position.y <= center_bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window_with_cursor(width: u32, height: u32, cursor: Option<Vec2>) -> Window {
        let mut window = Window::default();
        window.resolution.set_physical_resolution(width, height);
        window.set_cursor_position(cursor);
        window
    }

    #[test]
    fn test_mouse_center_interact_pressed_false_without_primary_window() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);

        assert!(!mouse_center_interact_pressed(&mouse, None));
    }

    #[test]
    fn test_mouse_center_interact_pressed_false_without_left_click() {
        let mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(450.0, 300.0)));

        assert!(!mouse_center_interact_pressed(&mouse, Some(&window)));
    }

    #[test]
    fn test_mouse_center_interact_pressed_false_without_cursor_position() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, None);

        mouse.press(MouseButton::Left);

        assert!(!mouse_center_interact_pressed(&mouse, Some(&window)));
    }

    #[test]
    fn test_mouse_center_interact_pressed_true_when_cursor_is_centered() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(450.0, 300.0)));

        mouse.press(MouseButton::Left);

        assert!(mouse_center_interact_pressed(&mouse, Some(&window)));
    }

    #[test]
    fn test_mouse_center_interact_pressed_false_outside_center_third() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(100.0, 100.0)));

        mouse.press(MouseButton::Left);

        assert!(!mouse_center_interact_pressed(&mouse, Some(&window)));
    }

    #[test]
    fn test_mouse_center_interact_pressed_true_on_center_boundary() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let window = window_with_cursor(900, 600, Some(Vec2::new(300.0, 200.0)));

        mouse.press(MouseButton::Left);

        assert!(mouse_center_interact_pressed(&mouse, Some(&window)));
    }

    #[test]
    fn test_is_cursor_in_center_third_true_for_middle_of_window() {
        let window = window_with_cursor(900, 600, None);

        assert!(is_cursor_in_center_third(&window, Vec2::new(450.0, 300.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_for_top_left_corner() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_for_bottom_right_corner() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(899.0, 599.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_true_on_all_inclusive_boundaries() {
        let window = window_with_cursor(900, 600, None);

        assert!(is_cursor_in_center_third(&window, Vec2::new(300.0, 200.0)));
        assert!(is_cursor_in_center_third(&window, Vec2::new(600.0, 400.0)));
        assert!(is_cursor_in_center_third(&window, Vec2::new(300.0, 400.0)));
        assert!(is_cursor_in_center_third(&window, Vec2::new(600.0, 200.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_just_outside_left_boundary() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(299.9, 300.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_just_outside_top_boundary() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(450.0, 199.9)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_just_outside_right_boundary() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(600.1, 300.0)));
    }

    #[test]
    fn test_is_cursor_in_center_third_false_just_outside_bottom_boundary() {
        let window = window_with_cursor(900, 600, None);

        assert!(!is_cursor_in_center_third(&window, Vec2::new(450.0, 400.1)));
    }
}
