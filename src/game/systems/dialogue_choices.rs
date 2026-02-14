// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue choice selection UI systems
//!
//! This module provides Bevy systems for displaying and managing player dialogue choices:
//! - Spawning choice buttons when choices become available
//! - Updating visual state based on selection
//! - Handling keyboard input for choice navigation
//! - Cleanup when dialogue ends
//!
//! The choice system works with the dialogue state to present branching dialogue options
//! to the player and handle their selection.

use bevy::ecs::world::World;
use bevy::prelude::*;

use crate::application::GameMode;
use crate::game::components::dialogue::*;
use crate::game::resources::GlobalState;
use crate::game::systems::dialogue::SelectDialogueChoice;

/// Spawns dialogue choice UI when choices become available
///
/// This system attaches screen-space choice entries as children of the
/// `DialogueChoiceList` node created by the dialogue panel. It uses a column
/// layout and creates a lightweight `Node` per choice that contains the
/// `Text`, `TextFont`, `TextColor`, `BackgroundColor` and `DialogueChoiceButton`
/// marker components.
///
/// Conditions for spawn:
/// - Game is in Dialogue mode
/// - Choices are available in the current dialogue state
/// - No choices have been spawned (tracked via `ChoiceSelectionState`)
/// - A `DialogueChoiceList` container exists (created by the dialogue panel)
pub fn spawn_choice_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    // Query to find the screen-space choice list container
    choice_list_query: Query<Entity, With<DialogueChoiceList>>,
    // Query to find children for cleanup
    children_query: Query<&Children>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        // Only spawn choices when they are available
        if dialogue_state.current_choices.is_empty() {
            return;
        }

        // Prevent re-spawning if logic hasn't changed.
        // We only respawn if the global state (dialogue node) changed OR if we have no choices displayed yet.
        if !global_state.is_changed() && choice_state.choice_count > 0 {
            return;
        }

        // Ensure dialogue panel exists
        if active_ui.bubble_entity.is_none() {
            return;
        }

        // Find the existing choice list container node (created by the panel)
        let container = match choice_list_query.iter().next() {
            Some(e) => e,
            None => {
                // No container to attach choices to; nothing to do
                return;
            }
        };

        // Clear any existing children to ensure we don't append to old choices
        // Manually despawn children with safety checks to avoid the entity-not-found error
        if let Ok(children) = children_query.get(container) {
            for child in children.iter() {
                // Queue despawn with proper error handling
                commands.queue(move |world: &mut World| {
                    if world.get_entity(child).is_ok() {
                        world.despawn(child);
                    }
                });
            }
        }

        let choice_count = dialogue_state.current_choices.len();

        // Spawn UI nodes for each choice
        for (index, choice_text) in dialogue_state.current_choices.iter().enumerate() {
            let selected = index == 0; // First choice selected by default

            let text_color = if selected {
                CHOICE_SELECTED_COLOR
            } else {
                CHOICE_UNSELECTED_COLOR
            };

            // Selected background intentionally slightly different so tests can detect it.
            // Using a literal here avoids needing to change components for Phase 2.
            let selected_bg = Color::srgba(0.12, 0.12, 0.15, 1.0);

            let bg_color = if selected {
                selected_bg
            } else {
                CHOICE_BACKGROUND_COLOR
            };

            // Each choice is a Node with text and selection marker
            let choice_entity = commands
                .spawn((
                    Node {
                        margin: UiRect::vertical(Val::Px(4.0)),
                        padding: UiRect::all(Val::Px(6.0)),
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    BackgroundColor(bg_color),
                    BorderRadius::all(Val::Px(4.0)),
                    // Display number + text like "1. Choice"
                    Text::new(format!("{}. {}", index + 1, choice_text)),
                    TextFont {
                        font_size: DIALOGUE_CONTENT_FONT_SIZE * 0.9,
                        ..default()
                    },
                    TextColor(text_color),
                    DialogueChoiceButton {
                        choice_index: index,
                        selected,
                    },
                ))
                .id();

            // Attach to the choice list container
            commands.entity(container).add_child(choice_entity);
        }

        // Initialize selection tracking
        choice_state.selected_index = 0;
        choice_state.choice_count = choice_count;
    }
}

/// Updates choice button visual state based on selection
///
/// Changes text color to highlight the currently selected choice.
/// Only updates when the selection state changes.
///
/// # Arguments
///
/// * `choice_state` - Current choice selection state
/// * `query` - Query for choice button entities
pub fn update_choice_visuals(
    choice_state: Res<ChoiceSelectionState>,
    mut query: Query<(
        &mut DialogueChoiceButton,
        &mut TextColor,
        &mut BackgroundColor,
    )>,
) {
    // Use a distinct highlight background for selected choice
    let highlight_bg = Color::srgba(0.12, 0.12, 0.15, 1.0);

    for (mut button, mut text_color, mut bg_color) in query.iter_mut() {
        let selected = button.choice_index == choice_state.selected_index;

        // Reflect logical selection on the component
        button.selected = selected;

        // Update text color
        text_color.0 = if selected {
            CHOICE_SELECTED_COLOR
        } else {
            CHOICE_UNSELECTED_COLOR
        };

        // Update background to highlight selection in addition to text color
        bg_color.0 = if selected {
            highlight_bg
        } else {
            CHOICE_BACKGROUND_COLOR
        };
    }
}

/// Handles keyboard input for navigating and selecting dialogue choices
///
/// Supports the following controls:
/// - Arrow Up/Down: Navigate between choices (with wrapping)
/// - Numbers 1-9: Direct selection of choice by number
/// - Enter/Space: Confirm selection and send SelectDialogueChoice event
///
/// # Arguments
///
/// * `keyboard` - Keyboard input state
/// * `global_state` - Current game state
/// * `mut choice_state` - Choice selection state to update
/// * `mut ev_select` - Message writer for SelectDialogueChoice messages
pub fn choice_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    mut ev_select: MessageWriter<SelectDialogueChoice>,
) {
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        return;
    }

    // Handle Escape to close dialogue.
    // Also handle Space/Enter if there are no choices (terminal node).
    let is_terminal = choice_state.choice_count == 0;

    if keyboard.just_pressed(KeyCode::Escape)
        || (is_terminal
            && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)))
    {
        // Exit dialogue mode
        global_state.0.mode = GameMode::Exploration;

        // Reset choice state
        choice_state.selected_index = 0;
        choice_state.choice_count = 0;
        return;
    }

    if choice_state.choice_count == 0 {
        return; // No choices available, and not closing
    }

    // Navigate up
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        if choice_state.selected_index > 0 {
            choice_state.selected_index -= 1;
        } else {
            // Wrap to bottom
            choice_state.selected_index = choice_state.choice_count - 1;
        }
    }

    // Navigate down
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        if choice_state.selected_index < choice_state.choice_count - 1 {
            choice_state.selected_index += 1;
        } else {
            // Wrap to top
            choice_state.selected_index = 0;
        }
    }

    // Direct number selection (1-9) triggers immediately
    for (key, index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ] {
        if keyboard.just_pressed(key) && index < choice_state.choice_count {
            // Select and confirm immediately
            ev_select.write(SelectDialogueChoice {
                choice_index: index,
            });
            choice_state.selected_index = 0;
            choice_state.choice_count = 0;
            return;
        }
    }

    // Confirm selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Send SelectDialogueChoice message
        ev_select.write(SelectDialogueChoice {
            choice_index: choice_state.selected_index,
        });

        // Reset choice state for next node
        choice_state.selected_index = 0;
        choice_state.choice_count = 0;
    }
}

/// Cleans up choice UI when dialogue ends or mode changes
///
/// Despawns all choice containers and resets the choice selection state
/// when the game is no longer in Dialogue mode.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity despawning
/// * `global_state` - Current game state
/// * `mut choice_state` - Choice selection state to reset
/// * `query` - Query for choice container entities
pub fn cleanup_choice_ui(
    global_state: Res<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
) {
    // Cleanup when leaving Dialogue mode
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        // Note: We do NOT need to despawn choice entities here because they are children
        // of the dialogue panel, which is despawned by cleanup_dialogue_bubble.
        // We only need to reset the selection state.

        choice_state.selected_index = 0;
        choice_state.choice_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choice_selection_state_initialization() {
        let state = ChoiceSelectionState {
            selected_index: 0,
            choice_count: 3,
        };
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.choice_count, 3);
    }

    #[test]
    fn test_choice_list_component_marker() {
        // Verify new screen-space marker exists
        let _list = DialogueChoiceList;
        // Component should compile and be usable - verified by type checking
    }

    #[test]
    fn test_choice_wrapping_logic() {
        // Test up navigation at index 0
        let mut index = 0usize;
        let count = 3usize;

        if index > 0 {
            index -= 1;
        } else {
            index = count - 1; // Wrap to bottom
        }
        assert_eq!(index, 2);
    }

    #[test]
    fn test_choice_down_wrapping() {
        let mut index = 2usize;
        let count = 3usize;

        if index < count - 1 {
            index += 1;
        } else {
            index = 0; // Wrap to top
        }
        assert_eq!(index, 0);
    }

    #[test]
    fn test_direct_number_selection() {
        // Verify that number keys 1-9 map to indices 0-8
        let number_to_index = |num: usize| num.saturating_sub(1);

        assert_eq!(number_to_index(1), 0);
        assert_eq!(number_to_index(5), 4);
        assert_eq!(number_to_index(9), 8);
    }

    #[test]
    fn test_spawn_choice_ui_populates_choice_list() {
        use crate::application::dialogue::DialogueState;
        use crate::application::GameMode;
        use crate::domain::types::Position;
        use crate::game::components::dialogue::{ActiveDialogueUI, DialogueChoiceButton};
        use crate::game::resources::GlobalState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ActiveDialogueUI>();
        app.init_resource::<ChoiceSelectionState>();

        // Prepare a DialogueState with two choices
        let mut gs = GlobalState(crate::application::GameState::new());
        let mut ds = DialogueState::start_simple(
            "Hello!".to_string(),
            "Speaker".to_string(),
            None,
            Some(Position::new(5, 6)),
        );
        ds.update_node(
            "Hello!".to_string(),
            "Speaker".to_string(),
            vec!["Opt A".to_string(), "Opt B".to_string()],
            None,
        );
        gs.0.mode = GameMode::Dialogue(ds);
        app.insert_resource(gs);

        // Spawn panel then choices
        app.add_systems(
            Update,
            crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
        );
        app.update();
        app.add_systems(
            Update,
            crate::game::systems::dialogue_choices::spawn_choice_ui,
        );
        app.update();

        let world = app.world_mut();
        // Verify both choice entities exist
        let choice_count = world.query::<&DialogueChoiceButton>().iter(world).count();
        assert_eq!(choice_count, 2);

        // Collect entries for inspection to avoid overlapping world borrows
        let entries: Vec<_> = world
            .query::<(&DialogueChoiceButton, &Text, &TextColor, &BackgroundColor)>()
            .iter(world)
            .collect();

        // Verify first choice is selected and colors/backgrounds are set
        let first = entries
            .iter()
            .find(|(b, _, _, _)| b.choice_index == 0)
            .expect("expected first choice");
        let (button, text, text_color, bg_color) = *first;
        assert!(button.selected);
        assert_eq!(text_color.0, CHOICE_SELECTED_COLOR);

        // Ensure selected background is different from the unselected one
        let unselected_bg = entries
            .iter()
            .find(|(b, _, _, _)| b.choice_index != 0)
            .map(|(_, _, _, bg)| bg.0)
            .expect("expected unselected");
        assert_ne!(bg_color.0, unselected_bg);

        // Confirm text content format is "1. <choice>"
        assert!(text.0.starts_with("1."));
    }

    #[test]
    fn test_update_choice_visuals_applies_highlight() {
        use crate::application::dialogue::DialogueState;
        use crate::application::GameMode;
        use crate::domain::types::Position;
        use crate::game::components::dialogue::{ActiveDialogueUI, DialogueChoiceButton};
        use crate::game::resources::GlobalState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ActiveDialogueUI>();
        app.init_resource::<ChoiceSelectionState>();

        // Prepare a DialogueState with two choices
        let mut gs = GlobalState(crate::application::GameState::new());
        let mut ds = DialogueState::start_simple(
            "Hello!".to_string(),
            "Speaker".to_string(),
            None,
            Some(Position::new(5, 6)),
        );
        ds.update_node(
            "Hello!".to_string(),
            "Speaker".to_string(),
            vec!["Opt A".to_string(), "Opt B".to_string()],
            None,
        );
        gs.0.mode = GameMode::Dialogue(ds);
        app.insert_resource(gs);

        // Spawn panel and choices
        app.add_systems(
            Update,
            crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
        );
        app.update();
        app.add_systems(
            Update,
            crate::game::systems::dialogue_choices::spawn_choice_ui,
        );
        app.update();

        // Change selection to the second choice
        {
            let mut cs = app.world_mut().resource_mut::<ChoiceSelectionState>();
            cs.selected_index = 1;
        }

        // Run visuals update to apply highlight
        app.add_systems(
            Update,
            crate::game::systems::dialogue_choices::update_choice_visuals,
        );
        app.update();

        // Verify second choice shows as selected
        let world = app.world_mut();
        let entries: Vec<_> = world
            .query::<(&DialogueChoiceButton, &TextColor, &BackgroundColor)>()
            .iter(world)
            .collect();

        let second = entries
            .iter()
            .find(|(b, _, _)| b.choice_index == 1)
            .expect("expected second choice");
        let (button, text_color, _bg) = *second;
        assert!(button.selected);
        assert_eq!(text_color.0, CHOICE_SELECTED_COLOR);

        // First should now be unselected
        let first = entries
            .iter()
            .find(|(b, _, _)| b.choice_index == 0)
            .expect("expected first choice");
        let (_, first_text_color, _) = *first;
        assert_eq!(first_text_color.0, CHOICE_UNSELECTED_COLOR);
    }

    #[test]
    fn test_cleanup_choice_ui_despawns_choice_nodes() {
        use crate::application::dialogue::DialogueState;
        use crate::application::GameMode;
        use crate::domain::types::Position;
        use crate::game::components::dialogue::{ActiveDialogueUI, DialogueChoiceButton};
        use crate::game::resources::GlobalState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ActiveDialogueUI>();
        app.init_resource::<ChoiceSelectionState>();

        // Prepare a DialogueState with two choices
        let mut gs = GlobalState(crate::application::GameState::new());
        let mut ds = DialogueState::start_simple(
            "Hello!".to_string(),
            "Speaker".to_string(),
            None,
            Some(Position::new(5, 6)),
        );
        ds.update_node(
            "Hello!".to_string(),
            "Speaker".to_string(),
            vec!["Opt A".to_string(), "Opt B".to_string()],
            None,
        );
        gs.0.mode = GameMode::Dialogue(ds);
        app.insert_resource(gs);

        // Spawn panel and choices
        app.add_systems(
            Update,
            crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
        );
        app.update();
        app.add_systems(
            Update,
            crate::game::systems::dialogue_choices::spawn_choice_ui,
        );
        app.update();

        // Ensure choices exist before cleanup
        {
            let world = app.world_mut();
            let count = world.query::<&DialogueChoiceButton>().iter(world).count();
            assert_eq!(count, 2, "expected two choice buttons before cleanup");
        }

        // Change mode to Exploration which triggers cleanup
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.mode = GameMode::Exploration;
        }

        // Run cleanup system
        app.add_systems(
            Update,
            (
                crate::game::systems::dialogue_choices::cleanup_choice_ui,
                crate::game::systems::dialogue_visuals::cleanup_dialogue_bubble,
            ),
        );
        app.update();

        // Verify choices were despawned
        {
            let world = app.world_mut();
            let count = world.query::<&DialogueChoiceButton>().iter(world).count();
            assert_eq!(count, 0, "expected no choice buttons after cleanup");
        }

        // Verify choice state was reset
        let choice_state = app.world().resource::<ChoiceSelectionState>();
        assert_eq!(choice_state.choice_count, 0);
        assert_eq!(choice_state.selected_index, 0);
    }
}
