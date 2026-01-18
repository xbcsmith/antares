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

// Dialogue background entity type removed - UI now uses screen-space Node entities (DialogueBubble.root_entity / text_entity).

/// Type alias for dialogue text entity
///
/// Used to reference the text display component of a dialogue bubble.
pub type DialogueTextEntity = Entity;

// ============================================================================
// Visual Constants
// ============================================================================

// 3D world-space positioning constants removed; dialogue visuals now use screen-space `bevy_ui` panels.

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

// Choice UI Constants

/// Height of each choice button (in world units)
pub const CHOICE_BUTTON_HEIGHT: f32 = 0.4;

/// Vertical spacing between choice buttons (in world units)
pub const CHOICE_BUTTON_SPACING: f32 = 0.1;

/// Color for selected choice button text
pub const CHOICE_SELECTED_COLOR: Color = Color::srgb(0.9, 0.8, 0.3);

/// Color for unselected choice button text
pub const CHOICE_UNSELECTED_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);

/// Background color for choice container
pub const CHOICE_BACKGROUND_COLOR: Color = Color::srgba(0.05, 0.05, 0.1, 0.95);

// Screen-Space UI Constants (bevy_ui)
/// Panel width as percentage of screen width
pub const DIALOGUE_PANEL_WIDTH: Val = Val::Percent(60.0);

/// Distance from bottom of screen
pub const DIALOGUE_PANEL_BOTTOM: Val = Val::Px(120.0);

/// Internal padding for dialogue panel
pub const DIALOGUE_PANEL_PADDING: Val = Val::Px(16.0);

/// Font size for speaker name
pub const DIALOGUE_SPEAKER_FONT_SIZE: f32 = 20.0;

/// Font size for dialogue content text
pub const DIALOGUE_CONTENT_FONT_SIZE: f32 = 18.0;

// ============================================================================
// Components
// ============================================================================

/// Marks an entity as a dialogue bubble UI element
///
/// Dialogue bubbles are screen-space UI panels that appear at the bottom-center
/// of the screen. They contain the current dialogue text and are tracked by
/// the `ActiveDialogueUI` resource.
///
/// # Fields
///
/// * `speaker_entity` - The entity that initiated this dialogue (optional)
/// * `root_entity` - Root `Node` entity of the panel hierarchy
/// * `text_entity` - Entity containing the content `Text` component
#[derive(Component, Debug, Clone)]
pub struct DialogueBubble {
    /// Entity that initiated this dialogue (optional)
    pub speaker_entity: Option<Entity>,
    /// Root UI node entity for the dialogue panel
    pub root_entity: Entity,
    /// Text entity containing the dialogue content
    pub text_entity: Entity,
}

// Billboard component removed - dialogue UI is screen-space and does not require an entity-facing-camera marker.

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

/// Marks an entity as a dialogue choice button
///
/// Each choice is a selectable option displayed to the player
/// during branching dialogue.
#[derive(Component, Debug)]
pub struct DialogueChoiceButton {
    /// Index of this choice in the choices list
    pub choice_index: usize,
    /// Whether this choice is currently selected
    pub selected: bool,
}

/// Marks the container entity holding all choice buttons
#[derive(Component, Debug)]
pub struct DialogueChoiceContainer;

/// Marks the root UI container for the dialogue panel (screen-space)
///
/// This is the top-level `Node` entity for the dialogue UI, positioned
/// at the bottom-center of the screen using bevy_ui.
#[derive(Component, Debug)]
pub struct DialoguePanelRoot;

/// Marks the speaker name text element in the dialogue panel
///
/// This component identifies the text entity displaying the speaker's name
/// (e.g., "Apprentice Zara") in the dialogue UI.
#[derive(Component, Debug)]
pub struct DialogueSpeakerText;

/// Marks the dialogue content text element
///
/// This component identifies the text entity displaying the actual dialogue
/// content. It works with `TypewriterText` for animated text reveal.
#[derive(Component, Debug)]
pub struct DialogueContentText;

/// Marks the choice button list container
///
/// This component identifies the container holding all dialogue choice buttons
/// in the screen-space UI.
#[derive(Component, Debug)]
pub struct DialogueChoiceList;

/// Marks an entity as an NPC that can initiate dialogue
///
/// NPCs with this component can be interacted with to start conversations.
/// The dialogue system uses this component to identify dialogue sources
/// and track which NPC is speaking during a conversation.
///
/// # Fields
///
/// * `dialogue_id` - Dialogue tree ID to start when interacting with this NPC
/// * `npc_name` - NPC's display name for identification
#[derive(Component, Debug, Clone)]
pub struct NpcDialogue {
    /// Dialogue tree ID to start when interacting with this NPC
    pub dialogue_id: crate::domain::dialogue::DialogueId,
    /// NPC's display name
    pub npc_name: String,
}

impl NpcDialogue {
    /// Creates a new NPC dialogue component
    ///
    /// # Arguments
    ///
    /// * `dialogue_id` - The dialogue tree ID this NPC uses
    /// * `npc_name` - The NPC's display name
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::dialogue::NpcDialogue;
    ///
    /// let npc = NpcDialogue::new(1, "Village Elder");
    /// assert_eq!(npc.dialogue_id, 1);
    /// assert_eq!(npc.npc_name, "Village Elder");
    /// ```
    pub fn new(
        dialogue_id: crate::domain::dialogue::DialogueId,
        npc_name: impl Into<String>,
    ) -> Self {
        Self {
            dialogue_id,
            npc_name: npc_name.into(),
        }
    }
}

/// Resource tracking current choice selection state
#[derive(Resource, Debug, Default)]
pub struct ChoiceSelectionState {
    /// Currently selected choice index (0-based)
    pub selected_index: usize,
    /// Total number of available choices
    pub choice_count: usize,
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

    // Billboard component removed - no unit test required.

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
    fn test_dialogue_choice_button_creation() {
        let choice = DialogueChoiceButton {
            choice_index: 0,
            selected: true,
        };
        assert_eq!(choice.choice_index, 0);
        assert!(choice.selected);
    }

    #[test]
    fn test_choice_selection_state_default() {
        let state = ChoiceSelectionState::default();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.choice_count, 0);
    }

    #[test]
    fn test_choice_ui_constants_valid() {
        // Constants are compile-time verified through type definitions
        let _ = CHOICE_BUTTON_HEIGHT;
        let _ = CHOICE_BUTTON_SPACING;
    }

    #[test]
    fn test_npc_dialogue_creation() {
        let npc = NpcDialogue::new(5, "Merchant");
        assert_eq!(npc.dialogue_id, 5);
        assert_eq!(npc.npc_name, "Merchant");
    }

    #[test]
    fn test_npc_dialogue_with_different_name() {
        let npc = NpcDialogue::new(10, "Village Elder");
        assert_eq!(npc.dialogue_id, 10);
        assert_eq!(npc.npc_name, "Village Elder");
    }
}
