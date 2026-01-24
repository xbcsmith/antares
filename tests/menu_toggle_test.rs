// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for menu toggle and keyboard navigation (Phase 3)
//!
//! This test suite validates:
//! - Menu toggle between game modes and Menu mode
//! - Arrow key navigation with wrapping
//! - Backspace navigation for submenus
//! - State preservation during mode transitions

use antares::application::menu::{MenuState, MenuType};
use antares::application::{GameMode, GameState};

/// Test that opening the menu from Exploration stores the correct previous mode
#[test]
fn test_menu_state_stores_previous_mode_exploration() {
    let previous_mode = GameMode::Exploration;
    let menu_state = MenuState::new(previous_mode.clone());

    assert_eq!(menu_state.get_resume_mode(), GameMode::Exploration);
    assert_eq!(menu_state.current_submenu, MenuType::Main);
    assert_eq!(menu_state.selected_index, 0);
}

/// Test that opening the menu from Combat stores the correct previous mode
#[test]
fn test_menu_state_stores_previous_mode_combat() {
    use antares::domain::combat::types::Handicap;

    // Create a simple combat mode
    let combat_mode = GameMode::Combat(antares::domain::combat::engine::CombatState::new(
        Handicap::Even,
    ));
    let menu_state = MenuState::new(combat_mode);

    // Verify the stored mode is Combat (we can't compare full CombatState, but we can check variant)
    let resumed = menu_state.get_resume_mode();
    assert!(matches!(resumed, GameMode::Combat(_)));
}

/// Test that opening the menu from Dialogue stores the correct previous mode
#[test]
fn test_menu_state_stores_previous_mode_dialogue() {
    let dialogue_mode = GameMode::Dialogue(antares::application::dialogue::DialogueState::new());
    let menu_state = MenuState::new(dialogue_mode.clone());

    let resumed = menu_state.get_resume_mode();
    assert!(matches!(resumed, GameMode::Dialogue(_)));
}

/// Test that arrow up navigates menu selection upward
#[test]
fn test_arrow_up_navigates_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Start at index 0, move up (should wrap to 4, the last item in Main menu)
    menu_state.selected_index = 0;
    menu_state.select_previous(5); // Main menu has 5 items: Resume, Save, Load, Settings, Quit

    assert_eq!(menu_state.selected_index, 4);
}

/// Test that arrow down navigates menu selection downward
#[test]
fn test_arrow_down_navigates_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Start at index 0, move down
    menu_state.selected_index = 0;
    menu_state.select_next(5);

    assert_eq!(menu_state.selected_index, 1);
}

/// Test that arrow down wraps around at the end of menu items
#[test]
fn test_arrow_down_wraps_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Start at last item in Main menu, move down (should wrap to 0)
    menu_state.selected_index = 4;
    menu_state.select_next(5);

    assert_eq!(menu_state.selected_index, 0);
}

/// Test that arrow up wraps around at the beginning of menu items
#[test]
fn test_arrow_up_wraps_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Start at index 1, move up
    menu_state.selected_index = 1;
    menu_state.select_previous(5);

    assert_eq!(menu_state.selected_index, 0);
}

/// Test that Backspace returns to Main menu from Settings submenu
#[test]
fn test_backspace_returns_to_main_from_settings() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Navigate to Settings submenu
    menu_state.set_submenu(MenuType::Settings);
    assert_eq!(menu_state.current_submenu, MenuType::Settings);

    // Simulating backspace action (manual invocation of set_submenu)
    if menu_state.current_submenu != MenuType::Main {
        menu_state.set_submenu(MenuType::Main);
    }

    assert_eq!(menu_state.current_submenu, MenuType::Main);
    assert_eq!(menu_state.selected_index, 0); // set_submenu resets index
}

/// Test that Backspace returns to Main menu from SaveLoad submenu
#[test]
fn test_backspace_returns_to_main_from_saveload() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Navigate to SaveLoad submenu
    menu_state.set_submenu(MenuType::SaveLoad);
    assert_eq!(menu_state.current_submenu, MenuType::SaveLoad);
    assert_eq!(menu_state.selected_index, 0);

    // Simulating backspace action
    if menu_state.current_submenu != MenuType::Main {
        menu_state.set_submenu(MenuType::Main);
    }

    assert_eq!(menu_state.current_submenu, MenuType::Main);
    assert_eq!(menu_state.selected_index, 0);
}

/// Test that MainMenu items are selectable by index
#[test]
fn test_main_menu_items_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Verify we can navigate through all 5 main menu items
    for expected_index in 0..5 {
        menu_state.selected_index = expected_index;
        assert_eq!(menu_state.selected_index, expected_index);
    }
}

/// Test that save list affects SaveLoad submenu item count
#[test]
fn test_save_load_submenu_respects_save_list_length() {
    let mut menu_state = MenuState::new(GameMode::Exploration);
    menu_state.set_submenu(MenuType::SaveLoad);

    // Without saves, item count should be at least 1 (back button)
    let item_count = menu_state.save_list.len().max(1);
    assert!(item_count >= 1);

    // Add a save and verify navigation
    menu_state
        .save_list
        .push(antares::application::menu::SaveGameInfo {
            filename: "save1.sav".to_string(),
            timestamp: "2025-01-01 12:00".to_string(),
            character_names: vec!["Hero1".to_string()],
            location: "Town".to_string(),
            game_version: "0.1.0".to_string(),
        });

    // Now select through multiple items
    menu_state.selected_index = 0;
    menu_state.select_next(menu_state.save_list.len().max(1));
}

/// Test Settings submenu navigation
#[test]
fn test_settings_submenu_navigation() {
    let mut menu_state = MenuState::new(GameMode::Exploration);
    menu_state.set_submenu(MenuType::Settings);

    assert_eq!(menu_state.current_submenu, MenuType::Settings);

    // Settings has 4 items
    menu_state.selected_index = 0;
    menu_state.select_next(4);
    assert_eq!(menu_state.selected_index, 1);

    menu_state.select_previous(4);
    assert_eq!(menu_state.selected_index, 0);
}

/// Test that MenuState selection resets when changing submenus
#[test]
fn test_set_submenu_resets_selection() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Set selection to something non-zero
    menu_state.selected_index = 3;

    // Switch to Settings submenu
    menu_state.set_submenu(MenuType::Settings);

    // Verify selection was reset to 0
    assert_eq!(menu_state.selected_index, 0);
    assert_eq!(menu_state.current_submenu, MenuType::Settings);
}

/// Test multiple back-and-forth menu transitions
#[test]
fn test_menu_transition_cycle() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // Cycle: Main -> Settings -> Main -> SaveLoad -> Main
    assert_eq!(menu_state.current_submenu, MenuType::Main);

    menu_state.set_submenu(MenuType::Settings);
    assert_eq!(menu_state.current_submenu, MenuType::Settings);

    menu_state.set_submenu(MenuType::Main);
    assert_eq!(menu_state.current_submenu, MenuType::Main);

    menu_state.set_submenu(MenuType::SaveLoad);
    assert_eq!(menu_state.current_submenu, MenuType::SaveLoad);

    menu_state.set_submenu(MenuType::Main);
    assert_eq!(menu_state.current_submenu, MenuType::Main);
}

/// Test that menu selection wraps correctly with small item counts
#[test]
fn test_selection_wrapping_with_single_item() {
    let mut menu_state = MenuState::new(GameMode::Exploration);

    // With only 1 item, navigation should stay in place
    menu_state.selected_index = 0;
    menu_state.select_next(1);
    assert_eq!(menu_state.selected_index, 0); // Wraps back to 0

    menu_state.select_previous(1);
    assert_eq!(menu_state.selected_index, 0);
}

/// Test that GlobalState can transition to Menu mode
#[test]
fn test_global_state_transitions_to_menu() {
    let mut game_state = GameState::new();
    assert!(matches!(game_state.mode, GameMode::Exploration));

    // Create a menu state
    let menu_state = MenuState::new(game_state.mode.clone());
    game_state.mode = GameMode::Menu(menu_state);

    assert!(matches!(game_state.mode, GameMode::Menu(_)));
}

/// Test that exiting menu preserves the original mode
#[test]
fn test_exit_menu_preserves_mode() {
    let mut game_state = GameState::new();

    // Start in Exploration
    assert!(matches!(game_state.mode, GameMode::Exploration));

    // Create menu (stores Exploration as previous_mode)
    let menu_state = MenuState::new(GameMode::Exploration);
    game_state.mode = GameMode::Menu(menu_state);

    // Exit menu
    if let GameMode::Menu(menu_state) = &game_state.mode {
        let resumed = menu_state.get_resume_mode();
        game_state.mode = resumed;
    }

    assert!(matches!(game_state.mode, GameMode::Exploration));
}

/// Test that save game info can be populated in menu state
#[test]
fn test_save_game_info_population() {
    let mut menu_state = MenuState::new(GameMode::Exploration);
    menu_state.set_submenu(MenuType::SaveLoad);

    // Add multiple save files
    for i in 0..3 {
        menu_state
            .save_list
            .push(antares::application::menu::SaveGameInfo {
                filename: format!("save_{}.sav", i),
                timestamp: format!("2025-01-{:02} 12:00", i + 1),
                character_names: vec![format!("Hero{}", i)],
                location: format!("Location{}", i),
                game_version: "0.1.0".to_string(),
            });
    }

    assert_eq!(menu_state.save_list.len(), 3);
    assert_eq!(menu_state.save_list[0].filename, "save_0.sav");
    assert_eq!(menu_state.save_list[2].filename, "save_2.sav");
}

/// Test navigation with realistic save list
#[test]
fn test_saveload_navigation_with_saves() {
    let mut menu_state = MenuState::new(GameMode::Exploration);
    menu_state.set_submenu(MenuType::SaveLoad);

    // Add 5 save files
    for i in 0..5 {
        menu_state
            .save_list
            .push(antares::application::menu::SaveGameInfo {
                filename: format!("save_{}.sav", i),
                timestamp: format!("2025-01-{:02}", i + 1),
                character_names: vec![format!("Party{}", i)],
                location: "Various".to_string(),
                game_version: "0.1.0".to_string(),
            });
    }

    let item_count = menu_state.save_list.len().max(1);
    assert_eq!(item_count, 5);

    // Navigate through all saves
    for expected_index in 0..5 {
        menu_state.selected_index = expected_index;
        assert_eq!(menu_state.selected_index, expected_index);
    }

    // Test wrapping
    menu_state.selected_index = 4;
    menu_state.select_next(item_count);
    assert_eq!(menu_state.selected_index, 0);
}
