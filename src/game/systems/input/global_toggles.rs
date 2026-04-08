// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Global input-toggle handling helpers.
//!
//! This module owns the top-of-frame global mode toggles that should be handled
//! before exploration interaction and movement behavior:
//!
//! - menu toggle
//! - automap toggle
//! - inventory toggle
//! - rest toggle
//!
//! The helper returns whether the current frame was consumed so the caller can
//! exit early and avoid falling through into exploration-specific behavior.

use crate::application::resources::GameContent;
use crate::application::{GameMode, GameState};

use crate::game::systems::input::{toggle_menu_state, FrameInputIntent};

/// Handles global mode-toggle input for a single frame.
///
/// This helper is intentionally limited to non-exploration-specific top-level
/// toggles that should be processed before movement cooldown checks or
/// exploration interaction routing.
///
/// Processing order is preserved exactly:
///
/// 1. automap toggle
/// 2. menu toggle
/// 3. inventory toggle
/// 4. rest toggle
///
/// # Arguments
///
/// * `game_state` - Mutable game state to update
/// * `frame_input` - Decoded frame input intent
/// * `game_content` - Optional content database used for merchant dialogue checks
///
/// # Returns
///
/// Returns `true` when one of the global toggle branches consumed the frame.
/// Returns `false` when no global toggle applied and the caller should continue
/// with non-global input handling.
///
/// # Examples
///
/// ```
/// use antares::application::{GameMode, GameState};
/// use antares::game::systems::input::{handle_global_mode_toggles, FrameInputIntent};
///
/// let mut state = GameState::new();
/// let consumed = handle_global_mode_toggles(
///     &mut state,
///     FrameInputIntent {
///         menu_toggle: true,
///         ..FrameInputIntent::default()
///     },
///     None,
/// );
///
/// assert!(consumed);
/// assert!(matches!(state.mode, GameMode::Menu(_)));
/// ```
pub fn handle_global_mode_toggles(
    game_state: &mut GameState,
    frame_input: FrameInputIntent,
    game_content: Option<&GameContent>,
) -> bool {
    if frame_input.automap_toggle {
        match game_state.mode {
            GameMode::Exploration => {
                game_state.mode = GameMode::Automap;
                bevy::prelude::info!("Automap opened: new_mode = {:?}", game_state.mode);
            }
            GameMode::Automap => {
                game_state.mode = GameMode::Exploration;
                bevy::prelude::info!(
                    "Automap closed via automap key: new_mode = {:?}",
                    game_state.mode
                );
            }
            _ => {}
        }
        return true;
    }

    if frame_input.menu_toggle {
        match &game_state.mode {
            GameMode::Automap => {
                game_state.mode = GameMode::Exploration;
                bevy::prelude::info!(
                    "Automap closed via menu key: new_mode = {:?}",
                    game_state.mode
                );
            }
            GameMode::MerchantInventory(merchant_state) => {
                let resume = merchant_state.get_resume_mode();
                bevy::prelude::info!(
                    "Merchant inventory closed via menu key: restored mode = {:?}",
                    resume
                );
                game_state.mode = resume;
            }
            GameMode::ContainerInventory(container_state) => {
                let resume = container_state.get_resume_mode();
                bevy::prelude::info!(
                    "Container inventory closed via menu key: restored mode = {:?}",
                    resume
                );
                game_state.mode = resume;
            }
            GameMode::TempleService(_) => {
                game_state.mode = GameMode::Exploration;
                bevy::prelude::info!(
                    "Temple service closed via menu key: new_mode = {:?}",
                    game_state.mode
                );
            }
            GameMode::GameLog => {
                game_state.mode = GameMode::Exploration;
                bevy::prelude::info!(
                    "Game log closed via menu key: new_mode = {:?}",
                    game_state.mode
                );
            }
            GameMode::SpellCasting(_) => {
                game_state.exit_spell_casting();
                bevy::prelude::info!(
                    "Spell casting cancelled via menu key: new_mode = {:?}",
                    game_state.mode
                );
            }
            _ => {
                toggle_menu_state(game_state);
                bevy::prelude::info!("Menu toggled: new_mode = {:?}", game_state.mode);
            }
        }
        return true;
    }

    if frame_input.inventory_toggle {
        let dialogue_npc_id = if let GameMode::Dialogue(ref ds) = game_state.mode {
            ds.speaker_npc_id.clone()
        } else {
            None
        };

        match &game_state.mode {
            GameMode::Inventory(inv_state) => {
                let resume = inv_state.get_resume_mode();
                bevy::prelude::info!("Inventory closed: restored mode = {:?}", resume);
                game_state.mode = resume;
            }
            GameMode::Dialogue(_) => {
                if let Some(npc_id) = dialogue_npc_id {
                    if let Some(content) = game_content {
                        if let Some(npc_def) = content.db().npcs.get_npc(&npc_id) {
                            if npc_def.is_merchant {
                                game_state.ensure_npc_runtime_initialized(content.db());
                                let npc_name = npc_def.name.clone();
                                bevy::prelude::info!(
                                    "I key in Dialogue: opening merchant inventory for '{}'",
                                    npc_id
                                );
                                game_state.enter_merchant_inventory(npc_id, npc_name);
                            } else {
                                bevy::prelude::info!(
                                    "I key in Dialogue: NPC '{}' is not a merchant, ignoring",
                                    npc_id
                                );
                            }
                        }
                    }
                }
            }
            GameMode::Menu(_) | GameMode::Combat(_) => {
                // Inventory toggle is ignored in menu and combat, but still
                // consumes the frame to preserve top-of-function behavior.
            }
            _ => {
                game_state.enter_inventory();
                bevy::prelude::info!("Inventory opened: mode = {:?}", game_state.mode);
            }
        }
        return true;
    }

    if frame_input.rest {
        if matches!(game_state.mode, GameMode::Exploration) {
            bevy::prelude::info!("Rest key pressed: opening rest menu");
            game_state.enter_rest_menu();
        } else {
            bevy::prelude::info!(
                "Rest key pressed but mode is {:?} — ignoring",
                game_state.mode
            );
        }
        return true;
    }

    if frame_input.cast {
        if matches!(game_state.mode, GameMode::Exploration) {
            bevy::prelude::info!("Cast key pressed: opening spell casting menu");
            game_state.enter_spell_casting_with_caster_select();
        } else {
            bevy::prelude::info!(
                "Cast key pressed but mode is {:?} — ignoring",
                game_state.mode
            );
        }
        return true;
    }

    if frame_input.spell_book_toggle {
        if matches!(game_state.mode, GameMode::Exploration) {
            bevy::prelude::info!("Spell Book key pressed: opening spell book");
            game_state.enter_spellbook_with_caster_select();
        } else {
            bevy::prelude::info!(
                "Spell Book key pressed but mode is {:?} — ignoring",
                game_state.mode
            );
        }
        return true;
    }

    if frame_input.game_log_toggle {
        match game_state.mode {
            GameMode::Exploration => {
                game_state.mode = GameMode::GameLog;
                bevy::prelude::info!("Fullscreen game log opened");
            }
            GameMode::GameLog => {
                game_state.mode = GameMode::Exploration;
                bevy::prelude::info!("Fullscreen game log closed via toggle key");
            }
            _ => {
                bevy::prelude::info!(
                    "Game log toggle pressed but mode is {:?} — ignoring",
                    game_state.mode
                );
            }
        }
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dialogue::DialogueState;
    use crate::application::resources::GameContent;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::npc::NpcDefinition;
    use crate::sdk::database::ContentDatabase;

    fn merchant_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let merchant = NpcDefinition::merchant("merchant_tom", "Tom the Merchant", "tom.png");
        db.npcs.add_npc(merchant).unwrap();
        db
    }

    fn non_merchant_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let elder = NpcDefinition::new("elder_bob", "Elder Bob", "bob.png");
        db.npcs.add_npc(elder).unwrap();
        db
    }

    fn dialogue_state_for(npc_id: &str) -> DialogueState {
        DialogueState::start(1, 1, None, Some(npc_id.to_string()))
    }

    fn inventory_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            inventory_toggle: true,
            ..FrameInputIntent::default()
        }
    }

    fn menu_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            menu_toggle: true,
            ..FrameInputIntent::default()
        }
    }

    fn game_log_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            game_log_toggle: true,
            ..FrameInputIntent::default()
        }
    }

    fn automap_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            automap_toggle: true,
            ..FrameInputIntent::default()
        }
    }

    fn rest_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            rest: true,
            ..FrameInputIntent::default()
        }
    }

    fn make_combat_state() -> GameState {
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
        state
    }

    #[test]
    fn test_handle_global_mode_toggles_returns_false_when_no_toggle_requested() {
        let mut state = GameState::new();

        let consumed = handle_global_mode_toggles(&mut state, FrameInputIntent::default(), None);

        assert!(!consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_menu_opens_and_closes() {
        let mut state = GameState::new();

        let consumed_open = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);
        assert!(consumed_open);
        assert!(matches!(state.mode, GameMode::Menu(_)));

        let consumed_close = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);
        assert!(consumed_close);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_automap_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::Automap;

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_merchant_inventory_to_dialogue() {
        let mut state = GameState::new();
        state.mode = GameMode::MerchantInventory(
            crate::application::merchant_inventory_state::MerchantInventoryState::new(
                "merchant_tom".to_string(),
                "Tom the Merchant".to_string(),
                0,
                GameMode::Dialogue(dialogue_state_for("merchant_tom")),
            ),
        );

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Dialogue(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_merchant_inventory_not_menu() {
        let mut state = GameState::new();
        state.mode = GameMode::MerchantInventory(
            crate::application::merchant_inventory_state::MerchantInventoryState::new(
                "merchant_tom".to_string(),
                "Tom the Merchant".to_string(),
                0,
                GameMode::Dialogue(dialogue_state_for("merchant_tom")),
            ),
        );

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(
            !matches!(state.mode, GameMode::Menu(_)),
            "Escape in MerchantInventory must close the merchant screen instead of opening the game menu"
        );
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_container_inventory_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::ContainerInventory(
            crate::application::container_inventory_state::ContainerInventoryState::new(
                "crate_01".to_string(),
                "Wooden Crate".to_string(),
                vec![],
                0,
                GameMode::Exploration,
            ),
        );

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_container_inventory_not_menu() {
        let mut state = GameState::new();
        state.mode = GameMode::ContainerInventory(
            crate::application::container_inventory_state::ContainerInventoryState::new(
                "crate_01".to_string(),
                "Wooden Crate".to_string(),
                vec![],
                0,
                GameMode::Exploration,
            ),
        );

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(
            !matches!(state.mode, GameMode::Menu(_)),
            "Escape in ContainerInventory must close the container screen instead of opening the game menu"
        );
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_temple_service_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::TempleService(crate::application::TempleServiceState::new(
            "temple_priest".to_string(),
        ));

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_temple_service_not_menu() {
        let mut state = GameState::new();
        state.mode = GameMode::TempleService(crate::application::TempleServiceState::new(
            "temple_priest".to_string(),
        ));

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(
            !matches!(state.mode, GameMode::Menu(_)),
            "Escape in TempleService must close the temple screen instead of opening the game menu"
        );
    }

    #[test]
    fn test_handle_global_mode_toggles_automap_opens_from_exploration() {
        let mut state = GameState::new();

        let consumed = handle_global_mode_toggles(&mut state, automap_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Automap));
    }

    #[test]
    fn test_handle_global_mode_toggles_automap_closes_back_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::Automap;

        let consumed = handle_global_mode_toggles(&mut state, automap_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_opens_in_exploration() {
        let mut state = GameState::new();

        let consumed = handle_global_mode_toggles(&mut state, inventory_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Inventory(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_closes_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_inventory();

        let consumed = handle_global_mode_toggles(&mut state, inventory_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_ignored_in_menu_mode() {
        let mut state = GameState::new();
        state.enter_menu();

        let consumed = handle_global_mode_toggles(&mut state, inventory_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Menu(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_ignored_in_combat_mode() {
        let mut state = make_combat_state();

        let consumed = handle_global_mode_toggles(&mut state, inventory_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Combat(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_opens_merchant_inventory_in_dialogue() {
        let db = merchant_db();
        let content = GameContent::new(db);
        let mut state = GameState::new();
        state.mode = GameMode::Dialogue(dialogue_state_for("merchant_tom"));

        let consumed =
            handle_global_mode_toggles(&mut state, inventory_toggle_intent(), Some(&content));

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::MerchantInventory(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_ignored_for_non_merchant_dialogue() {
        let db = non_merchant_db();
        let content = GameContent::new(db);
        let mut state = GameState::new();
        state.mode = GameMode::Dialogue(dialogue_state_for("elder_bob"));

        let consumed =
            handle_global_mode_toggles(&mut state, inventory_toggle_intent(), Some(&content));

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Dialogue(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_ignored_for_dialogue_without_npc_id() {
        let mut state = GameState::new();
        state.mode = GameMode::Dialogue(DialogueState::start(1, 1, None, None));

        let consumed = handle_global_mode_toggles(&mut state, inventory_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Dialogue(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_rest_opens_rest_menu_in_exploration() {
        let mut state = GameState::new();

        let consumed = handle_global_mode_toggles(&mut state, rest_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::RestMenu));
    }

    #[test]
    fn test_handle_global_mode_toggles_rest_ignored_in_menu_mode() {
        let mut state = GameState::new();
        state.enter_menu();

        let consumed = handle_global_mode_toggles(&mut state, rest_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Menu(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_rest_ignored_in_inventory_mode() {
        let mut state = GameState::new();
        state.enter_inventory();

        let consumed = handle_global_mode_toggles(&mut state, rest_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Inventory(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_rest_ignored_in_combat_mode() {
        let mut state = make_combat_state();

        let consumed = handle_global_mode_toggles(&mut state, rest_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Combat(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_automap_has_priority_over_menu() {
        let mut state = GameState::new();
        let intent = FrameInputIntent {
            automap_toggle: true,
            menu_toggle: true,
            ..FrameInputIntent::default()
        };

        let consumed = handle_global_mode_toggles(&mut state, intent, None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Automap));
    }

    #[test]
    fn test_handle_global_mode_toggles_menu_has_priority_over_inventory() {
        let mut state = GameState::new();
        let intent = FrameInputIntent {
            menu_toggle: true,
            inventory_toggle: true,
            ..FrameInputIntent::default()
        };

        let consumed = handle_global_mode_toggles(&mut state, intent, None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Menu(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_game_log_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::GameLog;

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_escape_closes_game_log_not_menu() {
        let mut state = GameState::new();
        state.mode = GameMode::GameLog;

        let consumed = handle_global_mode_toggles(&mut state, menu_toggle_intent(), None);

        assert!(consumed);
        assert!(
            !matches!(state.mode, GameMode::Menu(_)),
            "ESC from GameLog must return to Exploration, not open Menu"
        );
    }

    #[test]
    fn test_handle_global_mode_toggles_game_log_opens_from_exploration() {
        let mut state = GameState::new();

        let consumed = handle_global_mode_toggles(&mut state, game_log_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::GameLog));
    }

    #[test]
    fn test_handle_global_mode_toggles_game_log_closes_back_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::GameLog;

        let consumed = handle_global_mode_toggles(&mut state, game_log_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_handle_global_mode_toggles_game_log_ignored_in_combat() {
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

        let consumed = handle_global_mode_toggles(&mut state, game_log_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Combat(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_inventory_has_priority_over_rest() {
        let mut state = GameState::new();
        let intent = FrameInputIntent {
            inventory_toggle: true,
            rest: true,
            ..FrameInputIntent::default()
        };

        let consumed = handle_global_mode_toggles(&mut state, intent, None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Inventory(_)));
    }

    fn spell_book_toggle_intent() -> FrameInputIntent {
        FrameInputIntent {
            spell_book_toggle: true,
            ..FrameInputIntent::default()
        }
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_opens_from_exploration() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));

        let consumed = handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_ignored_in_menu_mode() {
        let mut state = GameState::new();
        state.enter_menu();
        assert!(matches!(state.mode, GameMode::Menu(_)));

        let consumed = handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        assert!(consumed);
        // menu mode is preserved — spell book open is ignored outside Exploration
        assert!(matches!(state.mode, GameMode::Menu(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_ignored_in_inventory_mode() {
        let mut state = GameState::new();
        state.enter_inventory();
        assert!(matches!(state.mode, GameMode::Inventory(_)));

        let consumed = handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        assert!(consumed);
        // inventory mode is preserved — spell book open is ignored outside Exploration
        assert!(matches!(state.mode, GameMode::Inventory(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_ignored_in_combat_mode() {
        use crate::domain::character::{Alignment, Character, Sex};
        let mut state = GameState::new();
        let hero = Character::new(
            "Spell Guard".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();
        state.enter_combat();

        let consumed = handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        assert!(consumed);
        assert!(matches!(state.mode, GameMode::Combat(_)));
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_stores_previous_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));

        handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        if let GameMode::SpellBook(ref sb) = state.mode {
            assert!(
                matches!(sb.get_resume_mode(), GameMode::Exploration),
                "SpellBook must store Exploration as the previous mode"
            );
        } else {
            panic!("expected SpellBook mode after spell_book_toggle in Exploration");
        }
    }

    #[test]
    fn test_handle_global_mode_toggles_spell_book_character_index_is_zero() {
        let mut state = GameState::new();
        handle_global_mode_toggles(&mut state, spell_book_toggle_intent(), None);

        if let GameMode::SpellBook(ref sb) = state.mode {
            assert_eq!(
                sb.character_index, 0,
                "enter_spellbook_with_caster_select must open at character index 0"
            );
        } else {
            panic!("expected SpellBook mode");
        }
    }
}
