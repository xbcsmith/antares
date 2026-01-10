// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Bevy ECS components for game entities

pub mod dialogue;

pub use dialogue::{
    ActiveDialogueUI, Billboard, DialogueBackgroundEntity, DialogueBubble, DialogueBubbleEntity,
    DialogueTextEntity, TypewriterText, DIALOGUE_BACKGROUND_COLOR, DIALOGUE_BUBBLE_HEIGHT,
    DIALOGUE_BUBBLE_PADDING, DIALOGUE_BUBBLE_WIDTH, DIALOGUE_BUBBLE_Y_OFFSET,
    DIALOGUE_CHOICE_COLOR, DIALOGUE_TEXT_COLOR, DIALOGUE_TEXT_SIZE, DIALOGUE_TYPEWRITER_SPEED,
};
