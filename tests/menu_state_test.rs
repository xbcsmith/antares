// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
//! Integration tests for MenuState types.
//!
//! This file duplicates the unit tests placed under `tests/unit/` to ensure
//! Cargo's integration test discovery runs the MenuState tests as part of the
//! standard `cargo test` workflow.

use antares::application::menu::{MenuState, MenuType, SaveGameInfo};
use antares::application::GameMode;

#[test]
fn test_menu_state_new_stores_previous_mode() {
    let menu_state = MenuState::new(GameMode::Exploration);

    assert!(matches!(*menu_state.previous_mode, GameMode::Exploration));
    assert_eq!(menu_state.current_submenu, MenuType::Main);
    assert_eq!(menu_state.selected_index, 0);
}

#[test]
fn test_menu_state_get_resume_mode_returns_previous() {
    let combat_state = antares::domain::combat::engine::CombatState::new(
        antares::domain::combat::types::Handicap::Even,
    );
    let menu_state = MenuState::new(GameMode::Combat(combat_state));
    let resumed = menu_state.get_resume_mode();

    assert!(matches!(resumed, GameMode::Combat(_)));
}

#[test]
fn test_menu_state_set_submenu_resets_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);
    menu_state.selected_index = 3;
    menu_state.set_submenu(MenuType::Settings);

    assert_eq!(menu_state.current_submenu, MenuType::Settings);
    assert_eq!(menu_state.selected_index, 0);
}

#[test]
fn test_menu_state_select_next_wraps_around() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // 5 items: increase selection and wrap
    menu_state.select_next(5);
    assert_eq!(menu_state.selected_index, 1);

    for _ in 0..4 {
        menu_state.select_next(5);
    }

    assert_eq!(menu_state.selected_index, 0);
}

#[test]
fn test_menu_state_select_previous_wraps_around() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // 5 items: previous from 0 should wrap to 4
    menu_state.select_previous(5);
    assert_eq!(menu_state.selected_index, 4);
}

#[test]
fn test_menu_state_serialization() {
    let menu_state = MenuState::new(GameMode::Exploration);
    let ron = ron::to_string(&menu_state).expect("Serialization should succeed");
    let de: MenuState = ron::from_str(&ron).expect("Deserialization should succeed");

    assert!(matches!(*de.previous_mode, GameMode::Exploration));
    assert_eq!(de.current_submenu, MenuType::Main);
    assert_eq!(de.selected_index, 0);
}

#[test]
fn test_menu_type_variants() {
    assert_eq!(MenuType::Main, MenuType::Main);
    assert_eq!(MenuType::SaveLoad, MenuType::SaveLoad);
    assert_eq!(MenuType::Settings, MenuType::Settings);
}

#[test]
fn test_save_game_info_creation() {
    let info = SaveGameInfo {
        filename: "save1.sav".to_string(),
        timestamp: "2025-01-01T12:00:00Z".to_string(),
        character_names: vec!["Alice".to_string(), "Bob".to_string()],
        location: "Town Square".to_string(),
        game_version: "0.1.0".to_string(),
    };

    assert_eq!(info.filename, "save1.sav");
    assert_eq!(info.character_names.len(), 2);
}
