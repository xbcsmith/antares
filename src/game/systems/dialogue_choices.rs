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

use bevy::prelude::*;

use crate::application::GameMode;
use crate::game::components::dialogue::*;
use crate::game::resources::GlobalState;
use crate::game::systems::dialogue::SelectDialogueChoice;

/// Spawns dialogue choice UI when choices become available
///
/// Creates a vertical list of choice buttons positioned below the dialogue bubble.
/// Each choice is a selectable button with text showing the choice text.
///
/// The system only spawns choices when:
/// - Game is in Dialogue mode
/// - Choices are available in the current dialogue state
/// - No choice container already exists
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity spawning
/// * `global_state` - Current game state (must be in Dialogue mode)
/// * `active_ui` - Resource tracking active dialogue UI
/// * `mut choice_state` - Resource tracking choice selection
pub fn spawn_choice_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    mut choice_state: ResMut<ChoiceSelectionState>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        // Only spawn choices when they are available
        if dialogue_state.current_choices.is_empty() {
            return;
        }

        // Check if there's already a choice container to prevent re-spawning
        if choice_state.choice_count > 0 {
            return; // Already spawned
        }

        // Check that dialogue bubble exists
        if active_ui.bubble_entity.is_none() {
            return;
        }

        // Spawn choice container
        let container = commands
            .spawn((
                Transform::from_xyz(0.0, CHOICE_CONTAINER_Y_OFFSET, 0.0),
                Visibility::default(),
                GlobalTransform::default(),
                DialogueChoiceContainer,
                Billboard,
            ))
            .id();

        // Spawn individual choice buttons
        let choice_count = dialogue_state.current_choices.len();
        for (index, choice_text) in dialogue_state.current_choices.iter().enumerate() {
            let y_offset = -(index as f32) * (CHOICE_BUTTON_HEIGHT + CHOICE_BUTTON_SPACING);
            let selected = index == 0; // First choice selected by default

            let choice_color = if selected {
                CHOICE_SELECTED_COLOR
            } else {
                CHOICE_UNSELECTED_COLOR
            };

            let choice_button = commands
                .spawn((
                    Text::new(format!("{}. {}", index + 1, choice_text)),
                    TextFont {
                        font_size: DIALOGUE_TEXT_SIZE * 0.8,
                        ..default()
                    },
                    TextColor(choice_color),
                    Transform::from_xyz(0.0, y_offset, 0.1),
                    Visibility::default(),
                    GlobalTransform::default(),
                    DialogueChoiceButton {
                        choice_index: index,
                        selected,
                    },
                ))
                .id();

            commands.entity(container).add_child(choice_button);
        }

        // Update choice selection state
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
    mut query: Query<(&DialogueChoiceButton, &mut TextColor)>,
) {
    if !choice_state.is_changed() {
        return;
    }

    for (button, mut text_color) in query.iter_mut() {
        let selected = button.choice_index == choice_state.selected_index;
        text_color.0 = if selected {
            CHOICE_SELECTED_COLOR
        } else {
            CHOICE_UNSELECTED_COLOR
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
    global_state: Res<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    mut ev_select: MessageWriter<SelectDialogueChoice>,
) {
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        return;
    }

    if choice_state.choice_count == 0 {
        return; // No choices available
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

    // Direct number selection (1-9)
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
            choice_state.selected_index = index;
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
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    query: Query<Entity, With<DialogueChoiceContainer>>,
) {
    // Cleanup if no longer in Dialogue mode
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
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
    fn test_choice_container_component_marker() {
        // Verify component is simple marker
        let _container = DialogueChoiceContainer;
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
}
