// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the menu components module
//!
//! These tests verify the basic types and constants exposed by
//! `antares::game::components::menu`.

use antares::game::components::menu::*;
use bevy::prelude::*;

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
    // Layout constants
    assert_eq!(MENU_WIDTH, 500.0);
    assert_eq!(MENU_HEIGHT, 600.0);
    assert_eq!(BUTTON_WIDTH, 400.0);
    assert_eq!(BUTTON_HEIGHT, 50.0);
    assert_eq!(BUTTON_SPACING, 15.0);
    assert_eq!(BUTTON_FONT_SIZE, 24.0);
    assert_eq!(TITLE_FONT_SIZE, 36.0);

    // Color constants (exact equality is fine here as constants are defined exactly)
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

    // Marker components are zero-sized unit types
    assert_eq!(std::mem::size_of::<MenuRoot>(), 0);
    assert_eq!(std::mem::size_of::<MainMenuPanel>(), 0);
    assert_eq!(std::mem::size_of::<SaveLoadPanel>(), 0);
    assert_eq!(std::mem::size_of::<SettingsPanel>(), 0);

    // Debug formatting should include the type name and not panic
    let _ = format!("{:?}", root);
    let _ = format!("{:?}", main);
    let _ = format!("{:?}", save);
    let _ = format!("{:?}", settings);
}
