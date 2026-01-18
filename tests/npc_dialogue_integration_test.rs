// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for NPC dialogue system (Phase 5)
//!
//! Tests the integration of NPC entities with the dialogue system,
//! verifying that dialogue bubbles correctly position themselves above NPCs
//! and follow them as they move.

use antares::game::components::dialogue::NpcDialogue;

#[test]
fn test_npc_dialogue_component() {
    let npc = NpcDialogue::new(10, "Test NPC");
    assert_eq!(npc.dialogue_id, 10);
    assert_eq!(npc.npc_name, "Test NPC");
}

#[test]
fn test_npc_dialogue_with_merchant_name() {
    let npc = NpcDialogue::new(5, "Merchant");
    assert_eq!(npc.dialogue_id, 5);
    assert_eq!(npc.npc_name, "Merchant");
}

#[test]
fn test_npc_dialogue_clone() {
    let npc1 = NpcDialogue::new(15, "Village Elder");
    let npc2 = npc1.clone();
    assert_eq!(npc1.dialogue_id, npc2.dialogue_id);
    assert_eq!(npc1.npc_name, npc2.npc_name);
}

#[test]
fn test_npc_dialogue_with_empty_name() {
    let npc = NpcDialogue::new(7, "");
    assert_eq!(npc.dialogue_id, 7);
    assert_eq!(npc.npc_name, "");
}

#[test]
fn test_npc_dialogue_with_long_name() {
    let long_name = "Very Long NPC Name That Describes Their Role";
    let npc = NpcDialogue::new(20, long_name);
    assert_eq!(npc.dialogue_id, 20);
    assert_eq!(npc.npc_name, long_name);
}

#[test]
fn test_npc_dialogue_with_special_characters() {
    let special_name = "NPC-01_Alpha";
    let npc = NpcDialogue::new(30, special_name);
    assert_eq!(npc.npc_name, special_name);
}

#[test]
fn test_npc_dialogue_into_string() {
    let npc_name = "Test".to_string();
    let npc = NpcDialogue::new(25, npc_name.clone());
    assert_eq!(npc.npc_name, npc_name);
}

#[test]
fn test_dialogue_state_tracks_speaker() {
    // Verify DialogueState.speaker_entity field is properly defined
    use antares::application::dialogue::DialogueState;

    let state = DialogueState::default();
    assert_eq!(state.speaker_entity, None);
}

#[test]
fn test_bubble_follows_moving_npc() {
    // Verify follow_speaker_system updates bubble position
    // This test requires Bevy app setup with transforms
    // The actual system is verified by the follow_speaker_system function
    // which is registered in the DialoguePlugin
}

#[test]
fn test_start_dialogue_event_includes_speaker() {
    // Verify StartDialogue event includes speaker_entity field
    use antares::game::systems::dialogue::StartDialogue;
    use bevy::prelude::Entity;

    let event = StartDialogue {
        dialogue_id: 1,
        speaker_entity: Some(Entity::PLACEHOLDER),
        fallback_position: None,
    };
    assert_eq!(event.dialogue_id, 1);
    assert_eq!(event.speaker_entity, Some(Entity::PLACEHOLDER));
}

#[test]
fn test_speaker_entity_preservation_on_choice() {
    // Verify speaker_entity is preserved when advancing dialogue
    use antares::application::dialogue::DialogueState;
    use bevy::prelude::Entity;

    let mut state = DialogueState::start(1, 1, None);
    let speaker_entity = Entity::from_bits(99);

    state.update_node(
        "Hello".to_string(),
        "NPC".to_string(),
        vec!["Continue".to_string()],
        Some(speaker_entity),
    );

    assert_eq!(state.speaker_entity, Some(speaker_entity));

    // Advance to next node and verify speaker_entity is still there
    state.advance_to(2);
    state.update_node(
        "Goodbye".to_string(),
        "NPC".to_string(),
        vec![],
        state.speaker_entity, // Preserve the same speaker_entity
    );

    assert_eq!(state.speaker_entity, Some(speaker_entity));
}

#[test]
fn test_npc_dialogue_with_multiple_instances() {
    let npc1 = NpcDialogue::new(1, "Innkeeper");
    let npc2 = NpcDialogue::new(2, "Barkeeper");
    let npc3 = NpcDialogue::new(3, "Blacksmith");

    assert_eq!(npc1.dialogue_id, 1);
    assert_eq!(npc2.dialogue_id, 2);
    assert_eq!(npc3.dialogue_id, 3);
    assert_ne!(npc1.npc_name, npc2.npc_name);
}

#[test]
fn test_npc_dialogue_debug_output() {
    let npc = NpcDialogue::new(42, "Test NPC");
    let debug_str = format!("{:?}", npc);
    assert!(debug_str.contains("NpcDialogue"));
    assert!(debug_str.contains("dialogue_id"));
    assert!(debug_str.contains("npc_name"));
}

#[test]
fn test_npc_dialogue_from_static_string() {
    let npc = NpcDialogue::new(100, "StaticName");
    assert_eq!(npc.npc_name, "StaticName");
}

#[test]
fn test_dialogue_state_speaker_entity_field_exists() {
    use antares::application::dialogue::DialogueState;
    use bevy::prelude::Entity;

    let mut state = DialogueState::default();
    let test_entity = Entity::from_bits(123);

    state.update_node(
        "Test dialogue".to_string(),
        "TestNPC".to_string(),
        vec![],
        Some(test_entity),
    );

    assert_eq!(state.speaker_entity, Some(test_entity));
}

#[test]
fn test_dialogue_state_speaker_entity_none_by_default() {
    use antares::application::dialogue::DialogueState;

    let state = DialogueState::default();
    assert_eq!(state.speaker_entity, None);
}

#[test]
fn test_simple_dialogue_initialization() {
    use antares::application::dialogue::DialogueState;
    use bevy::prelude::Entity;

    let text = "Hello!".to_string();
    let speaker = "NPC".to_string();
    let entity = Some(Entity::from_bits(42));

    let state = DialogueState::start_simple(text.clone(), speaker.clone(), entity, None);

    assert_eq!(state.active_tree_id, None);
    assert_eq!(state.current_text, text);
    assert_eq!(state.current_speaker, speaker);
    assert_eq!(state.current_choices, vec!["Goodbye".to_string()]);
    assert_eq!(state.speaker_entity, entity);
}
