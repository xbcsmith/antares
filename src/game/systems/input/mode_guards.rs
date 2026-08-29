// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Input mode-guard helpers.
//!
//! This module centralizes the rules that determine whether movement and
//! interaction input should be blocked for a given [`crate::application::GameMode`].
//! Keeping these guards in one place makes the top-level input system easier to
//! read and gives later refactor phases a reusable set of explicit policies.

use crate::application::GameMode;

/// Returns whether movement input is blocked for the current game mode.
///
/// Movement is blocked in modes that own their own input flow or otherwise must
/// suspend exploration movement:
///
/// - `Menu`
/// - `Inventory`
/// - `ContainerInventory`
/// - `Automap`
/// - `Combat`
/// - `Resting`
/// - `RestMenu`
/// - `GameLog`
/// - `CharacterSheet`
/// - `GameOver`
///
/// `CharacterSheet` was added here to fix a stale-mode-guard bug: without it,
/// `handle_exploration_input_interact`/`handle_exploration_input_movement`
/// only stood down via the `egui_wants_any_pointer_input` run condition,
/// which is written in `PostUpdate` -- one frame *after* the Character Sheet
/// UI draws in `Update` -- so exploration input could leak through on the
/// frame the sheet opened.
///
/// `ContainerInventory` was missing for the same reason: pressing the interact
/// key while a chest was open would re-fire `MapEventTriggered` with a stale
/// snapshot of the map event (before any take had been synced), causing
/// `handle_events` to call `enter_container_inventory` again with the original
/// items and resetting the container state mid-session. Players could exploit
/// this to loot the same item twice.
///
/// Dialogue is intentionally **not** blocked for movement because the current
/// input flow allows "move to cancel" behavior.
///
/// # Arguments
///
/// * `mode` - The current game mode
///
/// # Returns
///
/// `true` if movement input must be ignored in this mode, otherwise `false`.
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
/// use antares::game::systems::input::movement_blocked_for_mode;
///
/// assert!(movement_blocked_for_mode(&GameMode::Automap));
/// assert!(!movement_blocked_for_mode(&GameMode::Exploration));
/// ```
pub fn movement_blocked_for_mode(mode: &GameMode) -> bool {
    matches!(
        mode,
        GameMode::Menu(_)
            | GameMode::Inventory(_)
            | GameMode::ContainerInventory(_)
            | GameMode::Automap
            | GameMode::Combat(_)
            | GameMode::Resting(_)
            | GameMode::RestMenu
            | GameMode::GameLog
            | GameMode::SpellCasting(_)
            | GameMode::TrapNotification(_)
            | GameMode::SkillTraining(_)
            | GameMode::CharacterSheet(_)
            | GameMode::GameOver
    )
}

/// Returns whether interaction input is blocked for the current game mode.
///
/// Interaction is blocked in all modes that block movement, plus `Dialogue`.
/// Dialogue remains movement-permissive so the player can cancel by moving, but
/// interaction with doors, NPCs, and map events is intentionally suspended while
/// a dialogue is already active.
///
/// # Arguments
///
/// * `mode` - The current game mode
///
/// # Returns
///
/// `true` if interaction input must be ignored in this mode, otherwise `false`.
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
/// use antares::game::systems::input::interaction_blocked_for_mode;
///
/// assert!(interaction_blocked_for_mode(&GameMode::Dialogue(
///     antares::application::dialogue::DialogueState::start(1, 1, None, None),
/// )));
/// assert!(!interaction_blocked_for_mode(&GameMode::Exploration));
/// ```
pub fn interaction_blocked_for_mode(mode: &GameMode) -> bool {
    matches!(mode, GameMode::Dialogue(_)) || movement_blocked_for_mode(mode)
}

/// Returns whether all non-global exploration-style input is blocked for the
/// current game mode.
///
/// This is a convenience helper for callers that want a single early-return
/// check after global toggles have already been processed.
///
/// # Arguments
///
/// * `mode` - The current game mode
///
/// # Returns
///
/// `true` if both movement and interaction are blocked in this mode, otherwise
/// `false`.
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
/// use antares::game::systems::input::input_blocked_for_mode;
///
/// assert!(input_blocked_for_mode(&GameMode::RestMenu));
/// assert!(!input_blocked_for_mode(&GameMode::Dialogue(
///     antares::application::dialogue::DialogueState::start(1, 1, None, None),
/// )));
/// ```
pub fn input_blocked_for_mode(mode: &GameMode) -> bool {
    movement_blocked_for_mode(mode) && interaction_blocked_for_mode(mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dialogue::DialogueState;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};

    fn make_dialogue_mode() -> GameMode {
        GameMode::Dialogue(DialogueState::start(1, 1, None, None))
    }

    fn make_character_sheet_mode() -> GameMode {
        let mut state = GameState::new();
        state.enter_character_sheet();
        state.mode
    }

    fn make_combat_mode() -> GameMode {
        let mut state = GameState::new();
        let hero = Character::new(
            "Guard Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();
        state.enter_combat();
        state.mode
    }

    #[test]
    fn test_movement_blocked_for_mode_exploration_false() {
        assert!(!movement_blocked_for_mode(&GameMode::Exploration));
    }

    #[test]
    fn test_movement_blocked_for_mode_dialogue_false() {
        let mode = make_dialogue_mode();
        assert!(
            !movement_blocked_for_mode(&mode),
            "Dialogue must allow movement so move-to-cancel behavior is preserved"
        );
    }

    #[test]
    fn test_movement_blocked_for_mode_menu_true() {
        let mut state = GameState::new();
        state.enter_menu();

        assert!(movement_blocked_for_mode(&state.mode));
    }

    #[test]
    fn test_movement_blocked_for_mode_inventory_true() {
        let mut state = GameState::new();
        state.enter_inventory();

        assert!(movement_blocked_for_mode(&state.mode));
    }

    #[test]
    fn test_movement_blocked_for_mode_automap_true() {
        assert!(movement_blocked_for_mode(&GameMode::Automap));
    }

    #[test]
    fn test_movement_blocked_for_mode_combat_true() {
        let mode = make_combat_mode();
        assert!(movement_blocked_for_mode(&mode));
    }

    #[test]
    fn test_movement_blocked_for_mode_rest_menu_true() {
        assert!(movement_blocked_for_mode(&GameMode::RestMenu));
    }

    #[test]
    fn test_movement_blocked_for_mode_game_log_true() {
        assert!(movement_blocked_for_mode(&GameMode::GameLog));
    }

    #[test]
    fn test_movement_blocked_for_skill_training_true() {
        let mut state = GameState::new();
        state.enter_skill_training("trainer", vec![], vec![]);
        assert!(
            movement_blocked_for_mode(&state.mode),
            "movement must be blocked while in SkillTraining mode"
        );
    }

    #[test]
    fn test_interaction_blocked_for_mode_exploration_false() {
        assert!(!interaction_blocked_for_mode(&GameMode::Exploration));
    }

    #[test]
    fn test_interaction_blocked_for_mode_dialogue_true() {
        let mode = make_dialogue_mode();
        assert!(interaction_blocked_for_mode(&mode));
    }

    #[test]
    fn test_interaction_blocked_for_mode_inventory_true() {
        let mut state = GameState::new();
        state.enter_inventory();

        assert!(interaction_blocked_for_mode(&state.mode));
    }

    #[test]
    fn test_interaction_blocked_for_mode_combat_true() {
        let mode = make_combat_mode();
        assert!(interaction_blocked_for_mode(&mode));
    }

    #[test]
    fn test_input_blocked_for_mode_exploration_false() {
        assert!(!input_blocked_for_mode(&GameMode::Exploration));
    }

    #[test]
    fn test_input_blocked_for_mode_dialogue_false() {
        let mode = make_dialogue_mode();
        assert!(
            !input_blocked_for_mode(&mode),
            "Combined blocking must remain false for Dialogue because movement is still allowed"
        );
    }

    #[test]
    fn test_input_blocked_for_mode_menu_true() {
        let mut state = GameState::new();
        state.enter_menu();

        assert!(input_blocked_for_mode(&state.mode));
    }

    #[test]
    fn test_input_blocked_for_mode_automap_true() {
        assert!(input_blocked_for_mode(&GameMode::Automap));
    }

    #[test]
    fn test_input_blocked_for_mode_rest_menu_true() {
        assert!(input_blocked_for_mode(&GameMode::RestMenu));
    }

    #[test]
    fn test_input_blocked_for_mode_game_log_true() {
        assert!(input_blocked_for_mode(&GameMode::GameLog));
    }

    #[test]
    fn test_movement_blocked_for_trap_notification() {
        use crate::application::TrapNotificationState;
        let mode = GameMode::TrapNotification(TrapNotificationState::new_avoided(
            "Test Trap".to_string(),
            String::new(),
        ));
        assert!(
            movement_blocked_for_mode(&mode),
            "Movement must be blocked while the trap notification is showing"
        );
    }

    #[test]
    fn test_movement_blocked_for_game_over() {
        assert!(
            movement_blocked_for_mode(&GameMode::GameOver),
            "Movement must be blocked in GameOver — the party is dead"
        );
    }

    #[test]
    fn test_interaction_blocked_for_game_over() {
        assert!(
            interaction_blocked_for_mode(&GameMode::GameOver),
            "Interaction must be blocked in GameOver — the party is dead"
        );
    }

    #[test]
    fn test_input_blocked_for_game_over() {
        assert!(
            input_blocked_for_mode(&GameMode::GameOver),
            "All exploration input must be blocked in GameOver — the party is dead"
        );
    }

    #[test]
    fn test_movement_blocked_for_container_inventory_true() {
        let mut state = GameState::new();
        state.enter_container_inventory("chest".to_string(), "Chest".to_string(), vec![], 0, 0);
        assert!(
            movement_blocked_for_mode(&state.mode),
            "Movement must be blocked while ContainerInventory is open — \
             prevents interact-key from re-firing MapEventTriggered with stale items"
        );
    }

    #[test]
    fn test_interaction_blocked_for_container_inventory_true() {
        let mut state = GameState::new();
        state.enter_container_inventory("chest".to_string(), "Chest".to_string(), vec![], 0, 0);
        assert!(
            interaction_blocked_for_mode(&state.mode),
            "Interaction must be blocked while ContainerInventory is open"
        );
    }

    #[test]
    fn test_input_blocked_for_container_inventory_true() {
        let mut state = GameState::new();
        state.enter_container_inventory("chest".to_string(), "Chest".to_string(), vec![], 0, 0);
        assert!(
            input_blocked_for_mode(&state.mode),
            "All exploration input must be blocked while ContainerInventory is open"
        );
    }

    #[test]
    fn test_movement_blocked_for_character_sheet_true() {
        let mode = make_character_sheet_mode();
        assert!(
            movement_blocked_for_mode(&mode),
            "Movement must be blocked while the Character Sheet is open, matching \
             every other modal screen"
        );
    }

    #[test]
    fn test_interaction_blocked_for_character_sheet_true() {
        let mode = make_character_sheet_mode();
        assert!(
            interaction_blocked_for_mode(&mode),
            "Interaction must be blocked while the Character Sheet is open, matching \
             every other modal screen"
        );
    }

    #[test]
    fn test_input_blocked_for_character_sheet_true() {
        let mode = make_character_sheet_mode();
        assert!(
            input_blocked_for_mode(&mode),
            "All exploration input must be blocked while the Character Sheet is open"
        );
    }
}
