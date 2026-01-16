// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for dialogue state and event-driven logic
//!
//! Tests verify that the dialogue system correctly integrates:
//! - DialogueState updates when nodes change
//! - AdvanceDialogue events are processed
//! - Visual systems react to state changes
//! - Game mode transitions work correctly

use bevy::prelude::*;

#[test]
fn test_dialogue_state_initialization() {
    // Verify DialogueState can be created and initialized properly
    use antares::application::dialogue::DialogueState;

    let state = DialogueState::new();
    assert!(!state.is_active());
    assert_eq!(state.current_text, "");
    assert_eq!(state.current_speaker, "");
    assert!(state.current_choices.is_empty());
}

#[test]
fn test_dialogue_state_start_initializes_fields() {
    // Verify DialogueState::start properly initializes the state
    use antares::application::dialogue::DialogueState;

    let state = DialogueState::start(42, 1, None);
    assert!(state.is_active());
    assert_eq!(state.active_tree_id, Some(42));
    assert_eq!(state.current_node_id, 1);
    assert_eq!(state.dialogue_history, vec![1]);
}

#[test]
fn test_dialogue_state_update_node() {
    // Verify update_node properly sets visual state
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::new();
    state.update_node(
        "Hello, traveler!".to_string(),
        "Village Elder".to_string(),
        vec!["Greetings".to_string(), "Farewell".to_string()],
        None,
    );

    assert_eq!(state.current_text, "Hello, traveler!");
    assert_eq!(state.current_speaker, "Village Elder");
    assert_eq!(state.current_choices.len(), 2);
    assert_eq!(state.current_choices[0], "Greetings");
    assert_eq!(state.current_choices[1], "Farewell");
}

#[test]
fn test_dialogue_state_advance_and_update() {
    // Verify advancing nodes and updating state works together
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::start(1, 1, None);

    // Update node 1
    state.update_node(
        "First node".to_string(),
        "Speaker A".to_string(),
        vec!["Continue".to_string()],
        None,
    );

    assert_eq!(state.current_text, "First node");
    assert_eq!(state.current_speaker, "Speaker A");

    // Advance to node 2
    state.advance_to(2);
    state.update_node(
        "Second node".to_string(),
        "Speaker B".to_string(),
        vec!["Choice 1".to_string(), "Choice 2".to_string()],
        None,
    );

    assert_eq!(state.current_node_id, 2);
    assert_eq!(state.current_text, "Second node");
    assert_eq!(state.current_speaker, "Speaker B");
    assert_eq!(state.dialogue_history, vec![1, 2]);
}

#[test]
fn test_dialogue_state_overwrites_choices() {
    // Verify that updating node with different choices overwrites previous ones
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::new();

    state.update_node(
        "Text 1".to_string(),
        "Speaker 1".to_string(),
        vec!["Choice 1".to_string(), "Choice 2".to_string()],
        None,
    );

    assert_eq!(state.current_choices.len(), 2);

    // Update with fewer choices
    state.update_node(
        "Text 2".to_string(),
        "Speaker 2".to_string(),
        vec!["Only Choice".to_string()],
        None,
    );

    assert_eq!(state.current_choices.len(), 1);
    assert_eq!(state.current_choices[0], "Only Choice");
}

#[test]
fn test_dialogue_state_end_clears_all_state() {
    // Verify that ending dialogue clears all state
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::start(5, 1, None);
    state.update_node(
        "Some text".to_string(),
        "Some speaker".to_string(),
        vec!["Choice".to_string()],
        None,
    );
    state.advance_to(2);

    state.end();

    assert!(!state.is_active());
    assert_eq!(state.active_tree_id, None);
    assert_eq!(state.current_node_id, 0);
    assert!(state.dialogue_history.is_empty());
    assert_eq!(state.current_text, "");
    assert_eq!(state.current_speaker, "");
    assert!(state.current_choices.is_empty());
}

#[test]
fn test_advance_dialogue_event_creation() {
    // Verify AdvanceDialogue event can be created and debugged
    use antares::game::systems::dialogue::AdvanceDialogue;

    let event = AdvanceDialogue;
    let debug_str = format!("{:?}", event);
    assert_eq!(debug_str, "AdvanceDialogue");
}

#[test]
fn test_dialogue_state_terminal_choice() {
    // Verify state behavior with terminal choice
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::start(1, 1, None);
    state.update_node(
        "Do you accept?".to_string(),
        "Elder".to_string(),
        vec!["Accept".to_string()],
        None,
    );

    // Terminal choice - dialogue ends
    state.end();

    assert!(!state.is_active());
    assert_eq!(state.current_text, "");
}

#[test]
fn test_dialogue_state_multiple_node_chain() {
    // Verify traversing multiple nodes maintains history
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::start(1, 10, None);

    for i in 11..15 {
        state.advance_to(i);
        state.update_node(
            format!("Node {}", i),
            "Speaker".to_string(),
            vec!["Next".to_string()],
            None,
        );
    }

    assert_eq!(state.current_node_id, 14);
    assert_eq!(state.dialogue_history, vec![10, 11, 12, 13, 14]);
}

#[test]
fn test_dialogue_state_empty_choices() {
    // Verify dialogue state handles terminal nodes with no choices
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::new();
    state.update_node(
        "This is the end.".to_string(),
        "NPC".to_string(),
        vec![], // No choices - terminal node
        None,
    );

    assert_eq!(state.current_text, "This is the end.");
    assert!(state.current_choices.is_empty());
}

#[test]
fn test_dialogue_state_long_text() {
    // Verify dialogue state handles long text correctly
    use antares::application::dialogue::DialogueState;

    let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                     Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.";

    let mut state = DialogueState::new();
    state.update_node(
        long_text.to_string(),
        "Philosopher".to_string(),
        vec!["I understand".to_string()],
        None,
    );

    assert_eq!(state.current_text, long_text);
    assert_eq!(state.current_speaker, "Philosopher");
}

#[test]
fn test_dialogue_state_special_characters_in_names() {
    // Verify dialogue state handles special characters in speaker names
    use antares::application::dialogue::DialogueState;

    let mut state = DialogueState::new();
    state.update_node(
        "Greetings!".to_string(),
        "Sir Lancelot O'Brien".to_string(),
        vec!["Hello".to_string()],
        None,
    );

    assert_eq!(state.current_speaker, "Sir Lancelot O'Brien");
}

#[test]
fn test_game_mode_dialogue_variant() {
    // Verify GameMode::Dialogue variant can be created and matched
    use antares::application::{dialogue::DialogueState, GameMode};

    let state = DialogueState::start(1, 1, None);
    let mode = GameMode::Dialogue(state);

    match mode {
        GameMode::Dialogue(ds) => {
            assert!(ds.is_active());
            assert_eq!(ds.active_tree_id, Some(1));
        }
        _ => panic!("Expected GameMode::Dialogue"),
    }
}
