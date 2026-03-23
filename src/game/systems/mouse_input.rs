// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared mouse activation helpers for Bevy UI interactions.
//!
//! This module centralizes the canonical "activate on pressed-or-hovered click"
//! behavior used by Antares UI systems. It exists so combat, menu, and dialogue
//! systems can share identical mouse semantics instead of duplicating ad-hoc
//! checks.
//!
//! The activation model is:
//!
//! - `Interaction::Pressed` only activates when the interaction changed this
//!   frame.
//! - `Interaction::Hovered` activates when the left mouse button was just
//!   pressed this frame.
//! - `Interaction::None` never activates.
//!
//! # Mouse Input Model
//!
//! Antares uses a dual-path mouse activation model for Bevy UI widgets.
//!
//! In Bevy 0.17, relying only on `Interaction::Pressed` is not always robust
//! enough for every platform and UI timing edge case. Some widgets reliably
//! report the left mouse button press while hovered, but may not always produce
//! a fresh `Interaction::Pressed` transition in the exact frame a system checks
//! it. To keep mouse behavior stable across combat, menu, and dialogue screens,
//! Antares treats activation as either:
//!
//! - a changed `Interaction::Pressed`, or
//! - a left-click that happened while the widget was currently `Hovered`.
//!
//! This module is the single shared definition of that rule.
//!
//! ## egui screens
//!
//! For egui-managed screens, use egui's own click model:
//!
//! - use `response.clicked()`
//! - ensure the widget was created with `Sense::click()` when appropriate
//! - do not add Bevy `Button` / `Interaction` components to egui nodes
//!
//! egui owns those widgets and already provides the correct event model.
//!
//! ## Bevy UI screens
//!
//! For Bevy-UI-button screens, follow the shared helper path:
//!
//! 1. Spawn the widget with `Button` and an `Interaction` component
//!    initialized to `Interaction::None`.
//! 2. Add `Option<Res<ButtonInput<MouseButton>>>` to the system signature.
//! 3. Call `mouse_input::mouse_just_pressed(...)` and
//!    `mouse_input::is_activated(...)` instead of inlining ad-hoc click logic.
//!
//! This keeps mouse activation semantics identical across systems and avoids
//! drift between individual screens.
//!
//! ## Adding mouse support to a new Bevy UI system
//!
//! 1. Accept optional mouse-button input in the system signature.
//! 2. Compute `mouse_just_pressed(...)` once per frame.
//! 3. Route widget activation checks through `is_activated(...)`.
//!
//! Any new Bevy UI screen should prefer this module over custom
//! `just_pressed(MouseButton::Left)` patterns embedded directly in the system.
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::mouse_input::mouse_just_pressed;
//! use bevy::input::ButtonInput;
//! use bevy::prelude::MouseButton;
//!
//! let buttons: Option<&ButtonInput<MouseButton>> = None;
//! assert!(!mouse_just_pressed(buttons));
//! ```
//!
//! ```
//! use antares::game::systems::mouse_input::is_activated;
//! use bevy::prelude::Interaction;
//!
//! let interaction = Interaction::Hovered;
//! let hovered_click = is_activated(&interaction, false, true);
//!
//! assert!(hovered_click);
//! ```

use bevy::input::ButtonInput;
use bevy::prelude::{Interaction, MouseButton};

/// Returns `true` when the supplied Bevy UI interaction should count as an
/// activation this frame.
///
/// The activation rule is intentionally shared across systems:
///
/// - `Interaction::Pressed` activates only when the interaction changed this frame
/// - `Interaction::Hovered` activates when the left mouse button was just
///   pressed this frame
/// - `Interaction::None` never activates
///
/// The `interaction_changed` parameter should be computed from Bevy's
/// `Ref<Interaction>::is_changed()` at the call site.
///
/// # Arguments
///
/// * `interaction` - Current Bevy UI interaction state for the widget
/// * `interaction_changed` - Whether the interaction component changed this frame
/// * `mouse_just_pressed` - Whether the left mouse button was just pressed this frame
///
/// # Returns
///
/// Returns `true` if the widget should be treated as activated.
///
/// # Examples
///
/// ```
/// use antares::game::systems::mouse_input::is_activated;
/// use bevy::prelude::Interaction;
///
/// assert!(is_activated(&Interaction::Pressed, true, false));
/// assert!(is_activated(&Interaction::Hovered, false, true));
/// assert!(!is_activated(&Interaction::Pressed, false, false));
/// assert!(!is_activated(&Interaction::None, true, true));
/// ```
#[inline]
pub fn is_activated(
    interaction: &Interaction,
    interaction_changed: bool,
    mouse_just_pressed: bool,
) -> bool {
    (*interaction == Interaction::Pressed && interaction_changed)
        || (mouse_just_pressed && *interaction == Interaction::Hovered)
}

/// Returns `true` when the left mouse button was just pressed this frame.
///
/// This wrapper removes repeated `Option` handling from UI systems that receive
/// an optional mouse-button resource.
///
/// # Arguments
///
/// * `mouse_buttons` - Optional Bevy mouse button input resource reference
///
/// # Returns
///
/// Returns `true` when the resource exists and the left mouse button was just
/// pressed this frame; otherwise returns `false`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::mouse_input::mouse_just_pressed;
/// use bevy::input::ButtonInput;
/// use bevy::prelude::MouseButton;
///
/// let buttons = ButtonInput::<MouseButton>::default();
/// assert!(!mouse_just_pressed(Some(&buttons)));
/// ```
///
/// ```
/// use antares::game::systems::mouse_input::mouse_just_pressed;
/// use bevy::input::ButtonInput;
/// use bevy::prelude::MouseButton;
///
/// let buttons: Option<&ButtonInput<MouseButton>> = None;
/// assert!(!mouse_just_pressed(buttons));
/// ```
#[inline]
pub fn mouse_just_pressed(mouse_buttons: Option<&ButtonInput<MouseButton>>) -> bool {
    mouse_buttons.is_some_and(|buttons| buttons.just_pressed(MouseButton::Left))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_activated_pressed_changed() {
        assert!(is_activated(&Interaction::Pressed, true, false));
    }

    #[test]
    fn test_is_activated_pressed_unchanged() {
        assert!(!is_activated(&Interaction::Pressed, false, false));
    }

    #[test]
    fn test_is_activated_hovered_with_mouse_press() {
        assert!(is_activated(&Interaction::Hovered, false, true));
    }

    #[test]
    fn test_is_activated_hovered_without_mouse_press() {
        assert!(!is_activated(&Interaction::Hovered, false, false));
    }

    #[test]
    fn test_is_activated_none() {
        assert!(!is_activated(&Interaction::None, false, false));
        assert!(!is_activated(&Interaction::None, false, true));
    }

    #[test]
    fn test_mouse_just_pressed_none_resource() {
        let mouse_buttons: Option<&ButtonInput<MouseButton>> = None;
        assert!(!mouse_just_pressed(mouse_buttons));
    }

    #[test]
    fn test_mouse_just_pressed_left_button() {
        let mut mouse_buttons = ButtonInput::<MouseButton>::default();
        mouse_buttons.press(MouseButton::Left);
        assert!(mouse_just_pressed(Some(&mouse_buttons)));
    }

    #[test]
    fn test_mouse_just_pressed_other_button_only() {
        let mut mouse_buttons = ButtonInput::<MouseButton>::default();
        mouse_buttons.press(MouseButton::Right);
        assert!(!mouse_just_pressed(Some(&mouse_buttons)));
    }
}
