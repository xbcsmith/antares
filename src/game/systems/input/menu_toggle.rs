// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Pure helper for toggling the in-game menu state.

use crate::application::menu::MenuState;
use crate::application::{GameMode, GameState};
use bevy::prelude::info;

/// Toggle the in-game menu.
///
/// If the current mode is `GameMode::Menu`, this closes the menu and restores
/// the previously suspended mode. Otherwise, this opens the menu and records
/// the current mode as the resume target.
///
/// This helper intentionally ignores movement cooldown and input-routing rules
/// so callers can prioritize menu toggling independently of movement handling.
///
/// # Arguments
///
/// * `game_state` - Mutable game state whose mode will be toggled
///
/// # Examples
///
/// ```
/// use antares::application::{GameMode, GameState};
/// use antares::game::systems::input::toggle_menu_state;
///
/// let mut state = GameState::new();
/// assert!(matches!(state.mode, GameMode::Exploration));
///
/// toggle_menu_state(&mut state);
/// assert!(matches!(state.mode, GameMode::Menu(_)));
///
/// toggle_menu_state(&mut state);
/// assert!(matches!(state.mode, GameMode::Exploration));
/// ```
pub fn toggle_menu_state(game_state: &mut GameState) {
    match &game_state.mode {
        GameMode::Menu(menu_state) => {
            let resume_mode = menu_state.get_resume_mode();
            info!("Closing menu, resuming to: {:?}", resume_mode);
            game_state.mode = resume_mode;
        }
        current_mode => {
            info!("Opening menu from: {:?}", current_mode);
            let menu_state = MenuState::new(current_mode.clone());
            game_state.mode = GameMode::Menu(menu_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_menu_state_from_exploration_and_back() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));

        toggle_menu_state(&mut state);
        assert!(matches!(state.mode, GameMode::Menu(_)));

        toggle_menu_state(&mut state);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_toggle_menu_state_preserves_previous_mode() {
        let mut state = GameState::new();

        toggle_menu_state(&mut state);

        if let GameMode::Menu(menu_state) = &state.mode {
            assert!(matches!(
                menu_state.get_resume_mode(),
                GameMode::Exploration
            ));
        } else {
            panic!("Expected to be in Menu mode after toggle");
        }
    }

    #[test]
    fn test_toggle_menu_state_from_automap_and_back() {
        let mut state = GameState::new();
        state.mode = GameMode::Automap;

        toggle_menu_state(&mut state);

        if let GameMode::Menu(menu_state) = &state.mode {
            assert!(matches!(menu_state.get_resume_mode(), GameMode::Automap));
        } else {
            panic!("Expected to be in Menu mode after opening from Automap");
        }

        toggle_menu_state(&mut state);
        assert!(matches!(state.mode, GameMode::Automap));
    }
}
