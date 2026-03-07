// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Bevy ECS components for game entities
//!
//! This module provides reusable components for game entities:
//! - `billboard` - Camera-facing sprite components
//! - `sprite` - Tile and actor sprite components
//! - `dialogue` - Dialogue system components
//! - `inventory` - Inventory ECS identity components
//! - `menu` - Menu system components

pub mod billboard;
pub mod combat;
pub mod creature;
pub mod dialogue;
pub mod dropped_item;
pub mod furniture;
pub mod inventory;
pub mod menu;
pub mod performance;
pub mod sprite;

// Re-export commonly used types
pub use billboard::Billboard;
pub use combat::{CombatHudRoot, CombatantMarker, TargetSelector, TurnIndicator};
pub use creature::{
    CreatureAnimationState, CreatureVisual, FacingComponent, MeshPart, SpawnCreatureRequest,
};
pub use dialogue::{
    ActiveDialogueUI, DialogueBubble, DialogueBubbleEntity, DialogueTextEntity, TypewriterText,
    DIALOGUE_BACKGROUND_COLOR, DIALOGUE_BUBBLE_HEIGHT, DIALOGUE_BUBBLE_PADDING,
    DIALOGUE_BUBBLE_WIDTH, DIALOGUE_CHOICE_COLOR, DIALOGUE_TEXT_COLOR, DIALOGUE_TEXT_SIZE,
    DIALOGUE_TYPEWRITER_SPEED,
};
pub use dropped_item::DroppedItem;
pub use furniture::{FurnitureEntity, Interactable, InteractionType};
pub use inventory::{CharacterEntity, PartyEntities};
pub use menu::*;
pub use performance::{
    DistanceCulling, InstanceData, InstancedCreature, LodState, MeshStreaming, PerformanceCategory,
    PerformanceMarker,
};
pub use sprite::{ActorSprite, ActorType, AnimatedSprite, TileSprite};
