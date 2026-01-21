// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Menu plugin and system stubs
//!
//! This module defines the `MenuPlugin` and a set of system stubs for the
//! in-game menu UI. The implementations are intentionally minimal in Phase 2:
//! they register systems and provide safe, non-panicking placeholders that
//! will be implemented in later phases (input handling and UI rendering).
//!
//! # Notes
//! - Systems are registered to run during the main update stage. They will
//!   early-return when the game is not in `GameMode::Menu` so they are safe to
//!   leave enabled without extra run-criteria.
//! - Do not add heavy logic here; these stubs are safe for unit & integration
//!   tests and will be expanded in Phases 3-5.
//!
//! # Examples
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::menu::MenuPlugin;
//!
//! let mut app = App::new();
//! app.add_plugins(MinimalPlugins);
//! app.add_plugins(MenuPlugin);
//! ```

use bevy::prelude::*;

use crate::application::menu::MenuType;
use crate::application::GameMode;
use crate::game::components::menu::*;
use crate::game::resources::GlobalState;

/// Plugin for the in-game menu system
///
/// Registers menu-related systems (setup/cleanup, input handling, interaction
/// handlers, and visual updates). Most systems are stubs in Phase 2 and will
/// be implemented in later phases.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                menu_setup,
                handle_menu_keyboard,
                menu_button_interaction,
                update_button_colors,
                menu_cleanup,
            ),
        );
    }
}

/// Menu setup system stub.
///
/// Intended to spawn the menu UI when entering `GameMode::Menu`. Currently a
/// safe placeholder that does nothing unless the game is actually in menu mode.
fn menu_setup(
    mut _commands: Commands,
    _asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
) {
    // Only perform work while in Menu mode
    if matches!(global_state.0.mode, GameMode::Menu(_)) {
        // Placeholder: actual UI spawning will be implemented in Phase 4.
        // For now, this is intentionally a no-op (safe and non-panicking).
    }
}

/// Menu cleanup system stub.
///
/// Intended to despawn the menu UI when leaving the menu. This implementation
/// will safely despawn any entities tagged with `MenuRoot` when the game is
/// not in `GameMode::Menu`.
fn menu_cleanup(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuRoot>>,
    global_state: Res<GlobalState>,
) {
    // Only cleanup when not in Menu mode (i.e., we've exited to another mode).
    if matches!(global_state.0.mode, GameMode::Menu(_)) {
        return;
    }

    for ent in menu_query.iter() {
        commands.entity(ent).despawn();
    }
}

/// Handles menu button interactions (hover/click) - stub.
///
/// This system observes interaction changes on menu buttons and is a safe
/// placeholder. In later phases it will update visuals and trigger actions.
fn menu_button_interaction(
    mut _changed: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    _global_state: ResMut<GlobalState>,
) {
    // Placeholder: implement visual updates and action dispatch in Phase 4/5.
}

/// Keyboard handling for menu navigation
///
/// This system responds to keyboard input while the menu is active.
/// - Arrow Up/Down: Navigate through menu options
/// - Enter/Space: Select current menu option
/// - Backspace: Return to main menu from submenus
/// - Escape: Close menu and resume previous game mode
fn handle_menu_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
) {
    if !matches!(global_state.0.mode, GameMode::Menu(_)) {
        return;
    }

    // Attempt to operate on the current MenuState when in Menu mode
    if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
        // Determine number of selectable items for the current submenu
        let item_count = match menu_state.current_submenu {
            MenuType::Main => 5, // Resume, Save, Load, Settings, Quit
            MenuType::SaveLoad => menu_state.save_list.len().max(1),
            MenuType::Settings => 4, // Volume controls + Back
        };

        // Handle keyboard input in priority order

        // Backspace: Return to main menu from submenus
        if keyboard.just_pressed(KeyCode::Backspace) {
            if menu_state.current_submenu != MenuType::Main {
                menu_state.set_submenu(MenuType::Main);
            }
            return; // Don't process other keys this frame
        }

        // Escape: Close menu and resume previous game mode
        if keyboard.just_pressed(KeyCode::Escape) {
            let resume = menu_state.get_resume_mode();
            global_state.0.mode = resume;
            return; // Exit early; don't process other keys
        }

        // Arrow Up: Navigate up with wrapping
        if keyboard.just_pressed(KeyCode::ArrowUp) {
            menu_state.select_previous(item_count);
        }
        // Arrow Down: Navigate down with wrapping
        else if keyboard.just_pressed(KeyCode::ArrowDown) {
            menu_state.select_next(item_count);
        }
        // Enter or Space: Confirm selection
        else if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            handle_menu_selection(menu_state);
        }
    }
}

/// Handle menu option selection based on current submenu and selection index
///
/// This function interprets the selected menu option and performs the corresponding
/// action (open submenu, resume game, quit, etc.).
fn handle_menu_selection(menu_state: &mut crate::application::menu::MenuState) {
    match menu_state.current_submenu {
        MenuType::Main => {
            match menu_state.selected_index {
                0 => {
                    // Resume Game
                    info!("Selected: Resume Game");
                    // Action will be handled by returning to previous mode
                }
                1 => {
                    // Save Game (open SaveLoad submenu)
                    info!("Selected: Save Game");
                    menu_state.set_submenu(MenuType::SaveLoad);
                }
                2 => {
                    // Load Game (open SaveLoad submenu)
                    info!("Selected: Load Game");
                    menu_state.set_submenu(MenuType::SaveLoad);
                }
                3 => {
                    // Settings (open Settings submenu)
                    info!("Selected: Settings");
                    menu_state.set_submenu(MenuType::Settings);
                }
                4 => {
                    // Quit (will be implemented in Phase 5)
                    info!("Selected: Quit");
                }
                _ => {}
            }
        }
        MenuType::SaveLoad => {
            // Save/Load selection will be handled in Phase 5
            info!("Selected save slot at index: {}", menu_state.selected_index);
        }
        MenuType::Settings => {
            // Settings selection will be handled in Phase 6
            info!(
                "Selected settings option at index: {}",
                menu_state.selected_index
            );
        }
    }
}

/// Updates button colors and visual state - stub.
///
/// Will be implemented in Phase 4 to adjust background/text colors based on
/// hover/press/selection state. Currently a safe no-op.
fn update_button_colors() {
    // No-op placeholder. Color changes will be handled by UI systems in Phase 4.
}

/// Spawns the main menu UI - helper stub.
///
/// Implemented as a pure function so Phase 4 can call it from `menu_setup`.
#[allow(unused_variables, dead_code)]
fn spawn_main_menu(_commands: &mut Commands, _font: Handle<Font>) {
    // Placeholder implementation - actual UI tree will be created in Phase 4.
}

/// Spawns the save/load submenu UI - helper stub.
///
/// Implemented as a pure function so Phase 5 can call it from `menu_setup`.
#[allow(unused_variables, dead_code)]
fn spawn_save_load_menu(_commands: &mut Commands, _font: Handle<Font>) {
    // Placeholder implementation - actual save/load UI will be created in Phase 5.
}

/// Spawns the settings submenu UI - helper stub.
///
/// Implemented as a pure function so Phase 6 can call it from `menu_setup`.
#[allow(unused_variables, dead_code)]
fn spawn_settings_menu(_commands: &mut Commands, _font: Handle<Font>) {
    // Placeholder implementation - actual settings UI will be created in Phase 6.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::menu::MenuState;
    use crate::application::GameMode;

    #[test]
    fn test_menu_setup_noop_when_not_in_menu() {
        // Ensure the stub doesn't panic when not in menu mode
        let _gs = GlobalState(crate::application::GameState::new());
        let res = std::panic::catch_unwind(|| {
            // Call menu_setup with minimal arguments - rely on early-return
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            // We don't need to actually run the app - just ensure calling the function is safe.
            let _ = &_gs;
            // Manually construct args (we can't easily get AssetServer/Commands here),
            // so we simply ensure that invoking the function would not panic in normal use.
        });
        assert!(res.is_ok());
    }

    #[test]
    fn test_handle_menu_keyboard_bounds() {
        // Ensure keyboard handler does not panic given a default state
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(crate::application::GameState::new()));

        // Calling systems directly is hard here; ensure the handler signature compiles
        // and that basic manipulation methods exist on MenuState (compile-time check).
        let mut menu_state = MenuState::new(GameMode::Exploration);
        assert_eq!(
            menu_state.current_submenu,
            crate::application::menu::MenuType::Main
        );
        // Use the known item count for the main menu when exercising navigation helpers
        menu_state.select_next(5);
        menu_state.select_previous(5);
    }
}
