// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
//! Bevy ECS components and UI constants for the in-game menu system
//!
//! This module contains lightweight marker components used to tag menu-related
//! UI entities as well as enums representing button and slider types. It also
//! exposes visual constants used when spawning menu UI panels and controls.
//!
//! # Examples
//!
//! ```no_run
//! use crate::game::components::menu::{MenuButton, MenuRoot, MENU_WIDTH};
//!
//! // Marker component can be instantiated
//! let _root = MenuRoot;
//!
//! // Enum variants are simple value types
//! let btn = MenuButton::Resume;
//! assert!(matches!(btn, MenuButton::Resume));
//!
//! // Constants are available for layout
//! assert_eq!(MENU_WIDTH, 500.0);
//! ```
use bevy::prelude::*;

/// Marker component for the top-level menu UI entity
///
/// Attach this to the root `Node` entity that contains the entire menu panel
/// hierarchy so cleanup systems can query and despawn the menu in one go.
#[derive(Component, Debug)]
pub struct MenuRoot;

/// Marker component for the main menu panel
///
/// This identifies the primary action panel (Resume / Save / Load / Settings / Quit).
#[derive(Component, Debug)]
pub struct MainMenuPanel;

/// Marker component for the save/load panel
///
/// Attached to the save/load UI container shown when the `SaveLoad` submenu is
/// active.
#[derive(Component, Debug)]
pub struct SaveLoadPanel;

/// Marker component for the settings panel
///
/// Attached to the settings UI container (audio sliders, toggles, etc.).
#[derive(Component, Debug)]
pub struct SettingsPanel;

/// Menu button types.
///
/// Attach a `MenuButton` directly to a UI `Button` entity to identify its
/// semantic role. `SelectSave(index)` carries an index into the save list.
///
/// # Examples
///
/// ```
/// use crate::game::components::menu::MenuButton;
///
/// let resume = MenuButton::Resume;
/// let slot = MenuButton::SelectSave(2);
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuButton {
    /// Resume gameplay / close menu
    Resume,
    /// Open the Save Game submenu
    SaveGame,
    /// Open the Load Game submenu
    LoadGame,
    /// Open the Settings submenu
    Settings,
    /// Quit to the system or main menu
    Quit,
    /// Back action (return to previous submenu)
    Back,
    /// Confirm action (used in dialogs)
    Confirm,
    /// Cancel action
    Cancel,
    /// Select a save slot by index
    SelectSave(usize),
}

/// Volume slider identifiers.
///
/// These identify which audio channel a slider is bound to in the settings UI.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeSlider {
    /// Master volume (affects all channels)
    Master,
    /// Music volume
    Music,
    /// SFX volume
    Sfx,
    /// Ambient sound volume
    Ambient,
}

// ============================================================================
// UI Constants
// ============================================================================

/// Background color for menu panels (semi-transparent dark)
pub const MENU_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 0.95);

/// Menu panel width in pixels
pub const MENU_WIDTH: f32 = 500.0;

/// Menu panel height in pixels
pub const MENU_HEIGHT: f32 = 600.0;

/// Standard button width in pixels
pub const BUTTON_WIDTH: f32 = 400.0;

/// Standard button height in pixels
pub const BUTTON_HEIGHT: f32 = 50.0;

/// Vertical spacing between buttons in pixels
pub const BUTTON_SPACING: f32 = 15.0;

/// Font size for menu buttons (points / px)
pub const BUTTON_FONT_SIZE: f32 = 24.0;

/// Font size for menu titles (points / px)
pub const TITLE_FONT_SIZE: f32 = 36.0;

/// Button background color when idle
pub const BUTTON_NORMAL_COLOR: Color = Color::srgb(0.25, 0.25, 0.35);

/// Button background color when hovered
pub const BUTTON_HOVER_COLOR: Color = Color::srgb(0.35, 0.35, 0.55);

/// Button background color when pressed
pub const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.15, 0.15, 0.25);

/// Text color for button labels
pub const BUTTON_TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_button_variants() {
        let resume = MenuButton::Resume;
        let save = MenuButton::SaveGame;
        let load = MenuButton::LoadGame;
        let settings = MenuButton::Settings;
        let quit = MenuButton::Quit;
        let back = MenuButton::Back;
        let confirm = MenuButton::Confirm;
        let cancel = MenuButton::Cancel;
        let select = MenuButton::SelectSave(3);

        assert!(matches!(resume, MenuButton::Resume));
        assert!(matches!(save, MenuButton::SaveGame));
        assert!(matches!(load, MenuButton::LoadGame));
        assert!(matches!(settings, MenuButton::Settings));
        assert!(matches!(quit, MenuButton::Quit));
        assert!(matches!(back, MenuButton::Back));
        assert!(matches!(confirm, MenuButton::Confirm));
        assert!(matches!(cancel, MenuButton::Cancel));

        if let MenuButton::SelectSave(i) = select {
            assert_eq!(i, 3);
        } else {
            panic!("expected SelectSave variant");
        }
    }

    #[test]
    fn test_volume_slider_variants() {
        assert!(matches!(VolumeSlider::Master, VolumeSlider::Master));
        assert!(matches!(VolumeSlider::Music, VolumeSlider::Music));
        assert!(matches!(VolumeSlider::Sfx, VolumeSlider::Sfx));
        assert!(matches!(VolumeSlider::Ambient, VolumeSlider::Ambient));
    }

    #[test]
    fn test_menu_constants_defined() {
        assert_eq!(MENU_WIDTH, 500.0);
        assert_eq!(MENU_HEIGHT, 600.0);
        assert_eq!(BUTTON_WIDTH, 400.0);
        assert_eq!(BUTTON_HEIGHT, 50.0);
        assert_eq!(BUTTON_SPACING, 15.0);
        assert_eq!(BUTTON_FONT_SIZE, 24.0);
        assert_eq!(TITLE_FONT_SIZE, 36.0);
        assert_eq!(BUTTON_NORMAL_COLOR, Color::srgb(0.25, 0.25, 0.35));
        assert_eq!(BUTTON_HOVER_COLOR, Color::srgb(0.35, 0.35, 0.55));
        assert_eq!(BUTTON_PRESSED_COLOR, Color::srgb(0.15, 0.15, 0.25));
        assert_eq!(BUTTON_TEXT_COLOR, Color::srgb(0.9, 0.9, 0.9));
        assert_eq!(MENU_BACKGROUND_COLOR, Color::srgba(0.1, 0.1, 0.15, 0.95));
    }

    #[test]
    fn test_menu_root_component() {
        let root = MenuRoot;
        let main = MainMenuPanel;
        let save = SaveLoadPanel;
        let settings = SettingsPanel;

        // Ensure components are zero-sized marker types and can be debug-formatted
        assert_eq!(std::mem::size_of::<MenuRoot>(), 0);
        assert_eq!(std::mem::size_of::<MainMenuPanel>(), 0);
        assert_eq!(std::mem::size_of::<SaveLoadPanel>(), 0);
        assert_eq!(std::mem::size_of::<SettingsPanel>(), 0);

        // Debug formatting should not panic
        let _ = format!("{:?}", root);
        let _ = format!("{:?}", main);
        let _ = format!("{:?}", save);
        let _ = format!("{:?}", settings);
    }
}
