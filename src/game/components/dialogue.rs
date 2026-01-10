// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Bevy ECS components for dialogue visual representation
//!
//! This module provides Bevy components for rendering dialogue bubbles,
//! text, and UI elements in the 2.5D game world. These components are used
//! by the dialogue visual systems to animate and display dialogue content.

use bevy::prelude::*;

/// Type alias for dialogue bubble root entity
///
/// Used to reference the root entity of a dialogue bubble hierarchy.
pub type DialogueBubbleEntity = Entity;

/// Type alias for dialogue background entity
///
/// Used to reference the background mesh of a dialogue bubble.
pub type DialogueBackgroundEntity = Entity;

/// Type alias for dialogue text entity
///
/// Used to reference the text display component of a dialogue bubble.
pub type DialogueTextEntity = Entity;

// ============================================================================
// Visual Constants
// ============================================================================

/// Vertical offset of dialogue bubble above the speaker (in world units)
pub const DIALOGUE_BUBBLE_Y_OFFSET: f32 = 2.5;

/// Width of dialogue bubble in world units
pub const DIALOGUE_BUBBLE_WIDTH: f32 = 4.0;

/// Height of dialogue bubble in world units
pub const DIALOGUE_BUBBLE_HEIGHT: f32 = 1.2;

/// Padding inside dialogue bubble (distance from edge to text)
pub const DIALOGUE_BUBBLE_PADDING: f32 = 0.2;

/// Font size for dialogue text in pixels
pub const DIALOGUE_TEXT_SIZE: f32 = 24.0;

/// Typewriter animation speed in seconds per character
pub const DIALOGUE_TYPEWRITER_SPEED: f32 = 0.05;

/// Background color for dialogue bubbles (semi-transparent dark)
pub const DIALOGUE_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 0.9);

/// Text color for dialogue content (off-white)
pub const DIALOGUE_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);

/// Text color for player choice options (golden)
pub const DIALOGUE_CHOICE_COLOR: Color = Color::srgb(0.8, 0.8, 0.3);

// ============================================================================
// Components
// ============================================================================

/// Marks an entity as a dialogue bubble UI element
///
/// Dialogue bubbles are 2.5D UI elements that float above NPCs during conversations.
/// They contain text content and follow the speaker entity's position.
///
/// # Fields
///
/// * `speaker_entity` - The entity that spawned this dialogue (typically an NPC)
/// * `root_entity` - Root entity of the bubble hierarchy
/// * `background_entity` - Entity containing the background mesh
/// * `text_entity` - Entity containing the text component
/// * `y_offset` - Vertical offset from speaker position
#[derive(Component, Debug, Clone)]
pub struct DialogueBubble {
    /// Entity that spawned this dialogue (typically NPC)
    pub speaker_entity: Entity,
    /// Root entity of the bubble hierarchy
    pub root_entity: Entity,
    /// Background mesh entity
    pub background_entity: Entity,
    /// Text entity
    pub text_entity: Entity,
    /// Vertical offset from speaker position
    pub y_offset: f32,
}

/// Billboard component - makes entity always face the camera
///
/// Used for dialogue bubbles and other 2.5D UI elements that should
/// remain readable regardless of camera angle.
///
/// When attached to an entity, the dialogue system will automatically
/// rotate it to face the camera each frame, creating a billboard effect.
#[derive(Component, Debug, Clone, Copy)]
pub struct Billboard;

/// Typewriter text animation state
///
/// Animates text reveal character-by-character for dialogue text.
/// Used to create a typewriter effect where text appears one character at a time.
///
/// # Fields
///
/// * `full_text` - The complete text to display
/// * `visible_chars` - Current number of characters to show
/// * `timer` - Time elapsed since last character reveal
/// * `speed` - Seconds per character
/// * `finished` - Whether animation is complete
#[derive(Component, Debug, Clone)]
pub struct TypewriterText {
    /// Full text to display
    pub full_text: String,
    /// Currently visible character count
    pub visible_chars: usize,
    /// Time since last character reveal
    pub timer: f32,
    /// Seconds per character
    pub speed: f32,
    /// Whether animation is complete
    pub finished: bool,
}

/// Resource tracking active dialogue UI entity
///
/// Allows systems to reference the currently active dialogue bubble
/// for updates and cleanup. Only one dialogue bubble should be active at a time.
#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveDialogueUI {
    /// Entity ID of the active dialogue bubble, if any
    pub bubble_entity: Option<Entity>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_text_initialization() {
        let typewriter = TypewriterText {
            full_text: "Hello, world!".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: DIALOGUE_TYPEWRITER_SPEED,
            finished: false,
        };
        assert_eq!(typewriter.visible_chars, 0);
        assert!(!typewriter.finished);
        assert_eq!(typewriter.full_text, "Hello, world!");
        assert_eq!(typewriter.speed, DIALOGUE_TYPEWRITER_SPEED);
    }

    #[test]
    fn test_typewriter_text_component() {
        let typewriter = TypewriterText {
            full_text: "Test".to_string(),
            visible_chars: 2,
            timer: 0.1,
            speed: 0.05,
            finished: false,
        };
        assert_eq!(typewriter.visible_chars, 2);
        assert_eq!(typewriter.timer, 0.1);
        assert_eq!(typewriter.full_text.len(), 4);
    }

    #[test]
    fn test_active_dialogue_ui_default_is_empty() {
        let ui = ActiveDialogueUI::default();
        assert!(ui.bubble_entity.is_none());
    }

    #[test]
    fn test_billboard_component_creation() {
        let _billboard = Billboard;
        // Billboard is a unit struct, just verify it can be created
    }

    #[test]
    fn test_typewriter_complete_text() {
        let text = "Complete message".to_string();
        let typewriter = TypewriterText {
            full_text: text.clone(),
            visible_chars: text.len(),
            timer: 0.0,
            speed: DIALOGUE_TYPEWRITER_SPEED,
            finished: true,
        };
        assert!(typewriter.finished);
        assert_eq!(typewriter.visible_chars, typewriter.full_text.len());
    }

    #[test]
    fn test_dialogue_bubble_creation() {
        let speaker = Entity::PLACEHOLDER;
        let root = Entity::PLACEHOLDER;
        let background = Entity::PLACEHOLDER;
        let text = Entity::PLACEHOLDER;

        let bubble = DialogueBubble {
            speaker_entity: speaker,
            root_entity: root,
            background_entity: background,
            text_entity: text,
            y_offset: DIALOGUE_BUBBLE_Y_OFFSET,
        };

        assert_eq!(bubble.speaker_entity, speaker);
        assert_eq!(bubble.y_offset, DIALOGUE_BUBBLE_Y_OFFSET);
    }
}
