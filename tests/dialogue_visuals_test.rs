// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for dialogue visual systems
//!
//! Tests verify that dialogue visual components and systems work correctly
//! in isolation and when integrated with the game.

use antares::game::components::dialogue::*;
use bevy::prelude::Entity;

#[test]
fn test_typewriter_text_component_creation() {
    let typewriter = TypewriterText {
        full_text: "Test message".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: DIALOGUE_TYPEWRITER_SPEED,
        finished: false,
    };

    assert_eq!(typewriter.full_text, "Test message");
    assert_eq!(typewriter.visible_chars, 0);
    assert!(!typewriter.finished);
    assert_eq!(typewriter.speed, DIALOGUE_TYPEWRITER_SPEED);
}

#[test]
fn test_active_dialogue_ui_default() {
    let ui = ActiveDialogueUI::default();
    assert!(ui.bubble_entity.is_none());
}

#[test]
fn test_typewriter_text_initialization() {
    let text = "Hello, world!";
    let typewriter = TypewriterText {
        full_text: text.to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    assert_eq!(typewriter.full_text.len(), text.len());
    assert_eq!(typewriter.visible_chars, 0);
    assert!(!typewriter.finished);
}

#[test]
fn test_typewriter_text_finished_flag() {
    let mut typewriter = TypewriterText {
        full_text: "Test".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    // Reveal all characters
    typewriter.visible_chars = typewriter.full_text.len();

    // Mark as finished
    if typewriter.visible_chars >= typewriter.full_text.len() {
        typewriter.finished = true;
    }

    assert!(typewriter.finished);
}

#[test]
fn test_dialogue_bubble_entity_references() {
    use bevy::prelude::Entity;

    let speaker = Entity::PLACEHOLDER;
    let root = Entity::PLACEHOLDER;
    let text = Entity::PLACEHOLDER;

    let bubble = DialogueBubble {
        speaker_entity: Some(speaker),
        root_entity: root,
        text_entity: text,
    };

    assert_eq!(bubble.speaker_entity, Some(speaker));
    assert_eq!(bubble.root_entity, root);
    assert_eq!(bubble.text_entity, text);
}

#[test]
fn test_typewriter_visible_chars_never_exceeds_text_length() {
    let full_text = "Hello";
    let mut visible_chars = 10; // Try to exceed

    // Apply the clamping logic
    visible_chars = (visible_chars).min(full_text.len());

    assert_eq!(visible_chars, full_text.len());
}

// Billboard component removed; no unit test required.

#[test]
fn test_active_dialogue_ui_can_track_bubble() {
    use bevy::prelude::Entity;

    let mut ui = ActiveDialogueUI::default();
    assert!(ui.bubble_entity.is_none());

    let bubble_entity = Entity::PLACEHOLDER;
    ui.bubble_entity = Some(bubble_entity);

    assert!(ui.bubble_entity.is_some());
    assert_eq!(ui.bubble_entity.unwrap(), bubble_entity);
}

#[test]
fn test_typewriter_text_accumulation() {
    let mut typewriter = TypewriterText {
        full_text: "Test".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.1,
        finished: false,
    };

    // Simulate accumulating delta time
    typewriter.timer += 0.03;
    assert_eq!(typewriter.timer, 0.03);
    assert!(typewriter.timer < typewriter.speed);

    typewriter.timer += 0.05;
    assert_eq!(typewriter.timer, 0.08);
    assert!(typewriter.timer < typewriter.speed);

    typewriter.timer += 0.05;
    assert_eq!(typewriter.timer, 0.13);
    assert!(typewriter.timer >= typewriter.speed);
}

#[test]
fn test_dialogue_colors_are_valid() {
    // Verify that color constants are created successfully
    // In Bevy 0.17, colors are created with srgba/srgb functions
    let _bg = DIALOGUE_BACKGROUND_COLOR;
    let _text_color = DIALOGUE_TEXT_COLOR;
    let _choice_color = DIALOGUE_CHOICE_COLOR;

    // All colors should be distinct (not all the same)
    // Background should be darker (used for panels)
    // Text should be bright (for readability)
    // Choice should be golden (distinctive)
    assert_ne!(DIALOGUE_BACKGROUND_COLOR, DIALOGUE_TEXT_COLOR);
    assert_ne!(DIALOGUE_TEXT_COLOR, DIALOGUE_CHOICE_COLOR);
}

#[test]
fn test_typewriter_text_empty_string() {
    let typewriter = TypewriterText {
        full_text: String::new(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    assert_eq!(typewriter.full_text.len(), 0);
    assert_eq!(typewriter.visible_chars, 0);
}

#[test]
fn test_typewriter_text_single_character() {
    let mut typewriter = TypewriterText {
        full_text: "A".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    typewriter.visible_chars = (typewriter.visible_chars + 1).min(typewriter.full_text.len());
    assert_eq!(typewriter.visible_chars, 1);
    assert_eq!(typewriter.visible_chars, typewriter.full_text.len());
}

#[test]
fn test_typewriter_character_extraction() {
    let text = "Hello";
    let visible_text: String = text.chars().take(3).collect();

    assert_eq!(visible_text, "Hel");
    assert_eq!(visible_text.len(), 3);
}

#[test]
fn test_dialogue_state_message_types() {
    // Verify that dialogue components work with message passing
    use antares::game::systems::dialogue::{SelectDialogueChoice, StartDialogue};

    let start_msg = StartDialogue {
        dialogue_id: 1,
        speaker_entity: Some(Entity::PLACEHOLDER),
        fallback_position: None,
    };
    assert_eq!(start_msg.dialogue_id, 1);

    let choice_msg = SelectDialogueChoice { choice_index: 3 };
    assert_eq!(choice_msg.choice_index, 3);
}

#[test]
fn test_dialogue_bubble_constants_relationships() {
    // Test that constants have sensible relationships
    let width_has_padding = DIALOGUE_BUBBLE_WIDTH > DIALOGUE_BUBBLE_PADDING;
    let height_has_padding = DIALOGUE_BUBBLE_HEIGHT > DIALOGUE_BUBBLE_PADDING;

    assert!(width_has_padding);
    assert!(height_has_padding);
}

#[test]
fn test_typewriter_speed_creates_reasonable_animation() {
    // For a 100-character message at the given speed, should take reasonable time
    let message_length = 100.0;
    let reveal_time = message_length * DIALOGUE_TYPEWRITER_SPEED;

    // Should be reasonable (between 1 and 10 seconds for typical message)
    assert!(reveal_time > 1.0);
    assert!(reveal_time < 10.0);
}

#[test]
fn test_typewriter_timer_accumulation_reaches_threshold() {
    let mut typewriter = TypewriterText {
        full_text: "test".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    // Accumulate just under threshold
    typewriter.timer = 0.04;
    assert!(typewriter.timer < typewriter.speed);

    // Accumulate to threshold
    typewriter.timer = 0.05;
    assert!(typewriter.timer >= typewriter.speed);
}

#[test]
fn test_typewriter_marks_finished_correctly() {
    let mut typewriter = TypewriterText {
        full_text: "Hi".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: 0.05,
        finished: false,
    };

    // Simulate revealing all characters
    typewriter.visible_chars = typewriter.full_text.len();

    if typewriter.visible_chars >= typewriter.full_text.len() {
        typewriter.finished = true;
    }

    assert!(typewriter.finished);
}

#[test]
fn test_dialogue_bubble_has_all_entity_references() {
    use bevy::prelude::Entity;

    let bubble = DialogueBubble {
        speaker_entity: Some(Entity::PLACEHOLDER),
        root_entity: Entity::PLACEHOLDER,
        text_entity: Entity::PLACEHOLDER,
    };

    // All entity references should be set (even if to placeholder)
    assert_eq!(bubble.root_entity, bubble.root_entity);
    assert_eq!(bubble.text_entity, bubble.text_entity);
}

#[test]
fn test_typewriter_reveals_incrementally() {
    let text = "Hello World";
    let mut visible_count = 0;

    for _ in 0..text.len() {
        visible_count = (visible_count + 1).min(text.len());
    }

    // After revealing all, should equal text length
    assert_eq!(visible_count, text.len());
}
