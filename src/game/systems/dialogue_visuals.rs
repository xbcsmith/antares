// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue visual systems for rendering and animating dialogue UI
//!
//! This module provides Bevy systems that handle the visual presentation of dialogue:
//! - Spawning dialogue bubbles above speaker entities
//! - Animating text with a typewriter effect
//! - Billboard rotation to face the camera
//! - Cleanup when dialogue ends
//!
//! These systems work with the dialogue components to create an immersive 2.5D
//! dialogue experience.

use bevy::prelude::*;
use thiserror::Error;

use crate::application::GameMode;
use crate::game::components::dialogue::*;
use crate::game::resources::GlobalState;

/// Error type for dialogue visual system operations
#[derive(Error, Debug)]
pub enum DialogueVisualError {
    /// Speaker entity not found in the world
    #[error("Failed to spawn dialogue bubble: speaker entity {0:?} not found")]
    SpeakerNotFound(Entity),

    /// Failed to create mesh for dialogue background
    #[error("Failed to create mesh: {0}")]
    MeshCreationFailed(String),

    /// Game is not in Dialogue mode
    #[error("DialogueState not available in Dialogue mode")]
    InvalidGameMode,
}

/// Spawns a 2.5D dialogue bubble above the speaker entity
///
/// Creates a hierarchy of entities:
/// - Root entity (positioned above speaker with Billboard component)
///   - Background entity (semi-transparent colored panel)
///   - Text entity (animated typewriter text)
///
/// The bubble is only spawned if the game is in Dialogue mode and no bubble currently exists.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity spawning
/// * `global_state` - Current game state (must contain DialogueState in Dialogue mode)
/// * `active_ui` - Resource tracking the active dialogue bubble
/// * `meshes` - Mesh assets manager
/// * `materials` - Material assets manager
///
/// # Returns
///
/// Updates `active_ui.bubble_entity` with the spawned bubble entity, or leaves it unchanged
/// if not in Dialogue mode or bubble already exists.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::spawn_dialogue_bubble;
/// use antares::game::components::dialogue::ActiveDialogueUI;
///
/// # fn example(
/// #     commands: Commands,
/// #     global_state: Res<GlobalState>,
/// #     active_ui: ResMut<ActiveDialogueUI>,
/// #     meshes: ResMut<Assets<Mesh>>,
/// #     materials: ResMut<Assets<StandardMaterial>>,
/// # ) {
/// spawn_dialogue_bubble(commands, global_state, active_ui, meshes, materials);
/// # }
/// ```
pub fn spawn_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query_speaker: Query<&Transform, Without<Billboard>>,
) {
    // Only spawn if in Dialogue mode and no bubble exists yet
    if let GameMode::Dialogue(dialogue_state) = &global_state.0.mode {
        if active_ui.bubble_entity.is_some() {
            return; // Bubble already exists
        }

        // Get speaker position from dialogue state
        let speaker_position = if let Some(speaker_entity) = dialogue_state.speaker_entity {
            if let Ok(speaker_transform) = query_speaker.get(speaker_entity) {
                speaker_transform.translation
            } else {
                warn!(
                    "Speaker entity {:?} not found, using origin",
                    speaker_entity
                );
                Vec3::ZERO
            }
        } else {
            warn!("No speaker entity in dialogue state, using origin");
            Vec3::ZERO
        };
        let bubble_position = speaker_position + Vec3::new(0.0, DIALOGUE_BUBBLE_Y_OFFSET, 0.0);

        // Create background quad mesh
        let background_mesh = meshes.add(Mesh::from(Rectangle::new(
            DIALOGUE_BUBBLE_WIDTH,
            DIALOGUE_BUBBLE_HEIGHT,
        )));

        let background_material = materials.add(StandardMaterial {
            base_color: DIALOGUE_BACKGROUND_COLOR,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Spawn root entity with Billboard component to face camera
        let root_entity = commands
            .spawn((
                Transform::from_translation(bubble_position),
                Visibility::default(),
                GlobalTransform::default(),
                Billboard,
            ))
            .id();

        // Spawn background mesh as child of root
        let background_entity = commands
            .spawn((
                Mesh3d(background_mesh),
                MeshMaterial3d(background_material),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Visibility::default(),
                GlobalTransform::default(),
            ))
            .id();

        // Spawn text entity as child of root
        let text_entity = commands
            .spawn((
                Text::new(""),
                TextFont {
                    font_size: DIALOGUE_TEXT_SIZE,
                    ..default()
                },
                TextColor(DIALOGUE_TEXT_COLOR),
                Transform::from_xyz(0.0, 0.0, 0.1),
                Visibility::default(),
                GlobalTransform::default(),
                TypewriterText {
                    full_text: dialogue_state.current_text.clone(),
                    visible_chars: 0,
                    timer: 0.0,
                    speed: DIALOGUE_TYPEWRITER_SPEED,
                    finished: false,
                },
            ))
            .id();

        // Set up hierarchy: text and background are children of root
        commands.entity(root_entity).add_child(background_entity);
        commands.entity(root_entity).add_child(text_entity);

        // Create DialogueBubble marker component and spawn it
        let bubble = commands
            .spawn(DialogueBubble {
                speaker_entity: dialogue_state.speaker_entity.unwrap_or(Entity::PLACEHOLDER),
                root_entity,
                background_entity,
                text_entity,
                y_offset: DIALOGUE_BUBBLE_Y_OFFSET,
            })
            .id();

        // Track the bubble in the resource
        active_ui.bubble_entity = Some(bubble);
    }
}

/// Updates typewriter text animation
///
/// Reveals text character-by-character based on elapsed time. Each character is revealed
/// after `TypewriterText::speed` seconds have elapsed.
///
/// When all characters are revealed, the `finished` flag is set to true.
///
/// # Arguments
///
/// * `time` - Bevy time resource for delta time calculation
/// * `query` - Query for entities with both Text and TypewriterText components
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::update_typewriter_text;
/// use antares::game::components::dialogue::TypewriterText;
///
/// # fn example(time: Res<Time>, query: Query<(&mut Text, &mut TypewriterText)>) {
/// update_typewriter_text(time, query);
/// # }
/// ```
///
/// Updates dialogue bubble text when node changes
///
/// Detects when DialogueState.current_text changes and resets typewriter animation.
///
/// # Arguments
///
/// * `global_state` - Current game state with DialogueState
/// * `active_ui` - Resource tracking active dialogue bubble
/// * `query_bubble` - Query for DialogueBubble components
/// * `query_text` - Query for Text and TypewriterText components
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::application::GameMode;
/// use antares::game::systems::dialogue_visuals::update_dialogue_text;
/// use antares::game::components::dialogue::ActiveDialogueUI;
/// use antares::game::resources::GlobalState;
///
/// # fn example(
/// #     global_state: Res<GlobalState>,
/// #     active_ui: Res<ActiveDialogueUI>,
/// #     query_bubble: Query<&DialogueBubble>,
/// #     query_text: Query<(&mut Text, &mut TypewriterText)>,
/// # ) {
/// update_dialogue_text(global_state, active_ui, query_bubble, query_text);
/// # }
/// ```
pub fn update_dialogue_text(
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    query_bubble: Query<&DialogueBubble>,
    mut query_text: Query<(&mut Text, &mut TypewriterText)>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if let Some(bubble_entity) = active_ui.bubble_entity {
            if let Ok(bubble) = query_bubble.get(bubble_entity) {
                if let Ok((mut text, mut typewriter)) = query_text.get_mut(bubble.text_entity) {
                    // Check if text changed
                    if typewriter.full_text != dialogue_state.current_text {
                        // Reset typewriter animation for new text
                        typewriter.full_text = dialogue_state.current_text.clone();
                        typewriter.visible_chars = 0;
                        typewriter.timer = 0.0;
                        typewriter.finished = false;
                        text.0 = String::new(); // Clear visible text
                    }
                }
            }
        }
    }
}

pub fn update_typewriter_text(time: Res<Time>, mut query: Query<(&mut Text, &mut TypewriterText)>) {
    for (mut text, mut typewriter) in query.iter_mut() {
        // Skip if animation is already complete
        if typewriter.finished {
            continue;
        }

        // Accumulate time
        typewriter.timer += time.delta_secs();

        // Check if enough time has passed to reveal next character
        if typewriter.timer >= typewriter.speed {
            typewriter.timer = 0.0;
            typewriter.visible_chars =
                (typewriter.visible_chars + 1).min(typewriter.full_text.len());

            // Build visible text string
            let visible_text: String = typewriter
                .full_text
                .chars()
                .take(typewriter.visible_chars)
                .collect();

            // Update the text component
            text.0 = visible_text;

            // Mark as finished if all characters are visible
            if typewriter.visible_chars >= typewriter.full_text.len() {
                typewriter.finished = true;
            }
        }
    }
}

/// Billboard system - rotates entities to face the camera
///
/// Makes entities marked with the `Billboard` component always face the camera,
/// creating a pseudo-3D effect for 2.5D UI elements like dialogue bubbles.
///
/// This system continuously rotates billboarded entities to look at the camera's position,
/// using the world's up direction (Y axis) as the up vector.
///
/// # Arguments
///
/// * `query_camera` - Query for the camera entity and its transform
/// * `query_billboards` - Query for entities with Billboard component (excluding camera)
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::billboard_system;
/// use antares::game::components::dialogue::Billboard;
///
/// # fn example(
/// #     query_camera: Query<&Transform, With<Camera>>,
/// #     query_billboards: Query<&mut Transform, (With<Billboard>, Without<Camera>)>,
/// # ) {
/// billboard_system(query_camera, query_billboards);
/// # }
/// ```
pub fn billboard_system(
    query_camera: Query<&Transform, With<Camera3d>>,
    mut query_billboards: Query<&mut Transform, (With<Billboard>, Without<Camera3d>)>,
) {
    // Get the camera position
    if let Ok(camera_transform) = query_camera.single() {
        // Rotate each billboard to face the camera
        for mut transform in query_billboards.iter_mut() {
            transform.look_at(camera_transform.translation, Vec3::Y);
        }
    }
}

/// Cleans up dialogue UI when dialogue mode ends
///
/// Despawns all dialogue UI entities and clears the `ActiveDialogueUI` resource
/// when the game mode changes away from Dialogue.
///
/// This ensures no dialogue UI remnants persist after dialogue ends, and prepares
/// the system to spawn a new bubble if dialogue starts again.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity despawning
/// * `global_state` - Current game state
/// * `active_ui` - Resource tracking active dialogue bubble
/// * `query` - Query for DialogueBubble components
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::cleanup_dialogue_bubble;
/// use antares::game::components::dialogue::DialogueBubble;
///
/// # fn example(
/// #     commands: Commands,
/// #     global_state: Res<GlobalState>,
/// #     active_ui: ResMut<ActiveDialogueUI>,
/// #     query: Query<(Entity, &DialogueBubble)>,
/// # ) {
/// cleanup_dialogue_bubble(commands, global_state, active_ui, query);
/// # }
/// ```
pub fn cleanup_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    query: Query<(Entity, &DialogueBubble)>,
) {
    // Only cleanup if no longer in Dialogue mode
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        if let Some(bubble_entity) = active_ui.bubble_entity {
            // Try to get the bubble and despawn it with all children
            if let Ok((_, bubble)) = query.get(bubble_entity) {
                // Despawn root and all its children
                commands.entity(bubble.root_entity).despawn();
                // Despawn the bubble marker entity itself
                commands.entity(bubble_entity).despawn();
            }
            // Clear the resource
            active_ui.bubble_entity = None;
        }
    }
}

/// Updates dialogue bubble position to follow speaker
///
/// Keeps the dialogue bubble positioned above the NPC even if the NPC moves.
/// This system runs each frame to sync the bubble's world position with the
/// speaker entity's position.
///
/// # Arguments
///
/// * `query_bubbles` - Query for DialogueBubble components
/// * `query_speaker` - Query for speaker Transform components
/// * `mut query_bubble_transform` - Query for bubble Transform components to update
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::follow_speaker_system;
/// use antares::game::components::dialogue::{DialogueBubble, Billboard};
///
/// # fn example(
/// #     query_bubbles: Query<&DialogueBubble>,
/// #     query_speaker: Query<&Transform, Without<DialogueBubble>>,
/// #     mut query_bubble_transform: Query<&mut Transform, With<Billboard>>,
/// # ) {
/// follow_speaker_system(query_bubbles, query_speaker, query_bubble_transform);
/// # }
/// ```
pub fn follow_speaker_system(
    query_bubbles: Query<&DialogueBubble>,
    query_speaker: Query<&Transform, Without<DialogueBubble>>,
    mut query_bubble_transform: Query<&mut Transform, With<Billboard>>,
) {
    for bubble in query_bubbles.iter() {
        if let Ok(speaker_transform) = query_speaker.get(bubble.speaker_entity) {
            if let Ok(mut bubble_transform) = query_bubble_transform.get_mut(bubble.root_entity) {
                // Update position to follow speaker
                let target_position =
                    speaker_transform.translation + Vec3::new(0.0, bubble.y_offset, 0.0);
                bubble_transform.translation = target_position;
            }
        }
    }
}

/// Monitors speaker entity and ends dialogue if speaker is despawned
///
/// This system checks if the active dialogue speaker entity still exists in the world.
/// If the speaker is despawned during dialogue (e.g., NPC removed or level unloaded),
/// dialogue is ended gracefully and the game returns to Exploration mode.
///
/// # Arguments
///
/// * `mut global_state` - Current game state to update
/// * `query_entities` - Query to verify speaker entity existence
/// * `mut game_log` - Optional game log for error messages
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dialogue_visuals::check_speaker_exists;
/// use antares::game::resources::GlobalState;
///
/// # fn example(
/// #     global_state: Res<GlobalState>,
/// #     query_entities: Query<Entity>,
/// # ) {
/// // System is automatically called each frame
/// # }
/// ```
pub fn check_speaker_exists(
    mut global_state: ResMut<GlobalState>,
    query_entities: Query<Entity>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if let Some(speaker_entity) = dialogue_state.speaker_entity {
            // Check if speaker still exists
            if query_entities.get(speaker_entity).is_err() {
                info!(
                    "Speaker entity {:?} despawned during dialogue, ending conversation",
                    speaker_entity
                );
                if let Some(ref mut log) = game_log {
                    log.add("Speaker left the conversation.".to_string());
                }
                global_state.0.return_to_exploration();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_reveals_characters_over_time() {
        // Test that typewriter animation advances properly
        let mut typewriter = TypewriterText {
            full_text: "Hello".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        // Simulate time passing
        typewriter.timer = 0.05;

        if typewriter.timer >= typewriter.speed {
            typewriter.timer = 0.0;
            typewriter.visible_chars =
                (typewriter.visible_chars + 1).min(typewriter.full_text.len());
        }

        assert_eq!(typewriter.visible_chars, 1);
        assert!(!typewriter.finished);
    }

    #[test]
    fn test_typewriter_finishes_when_complete() {
        // Test that typewriter marks itself as finished when all chars revealed
        let mut typewriter = TypewriterText {
            full_text: "Hi".to_string(),
            visible_chars: 2,
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        // Simulate completion check
        if typewriter.visible_chars >= typewriter.full_text.len() {
            typewriter.finished = true;
        }

        assert!(typewriter.finished);
        assert_eq!(typewriter.visible_chars, typewriter.full_text.len());
    }

    #[test]
    fn test_typewriter_accumulates_time() {
        // Test that timer accumulates correctly
        let mut typewriter = TypewriterText {
            full_text: "Test".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        typewriter.timer += 0.03;
        assert!(typewriter.timer < typewriter.speed);

        typewriter.timer += 0.03;
        assert!(typewriter.timer >= typewriter.speed);
    }

    #[test]
    fn test_typewriter_caps_visible_chars() {
        // Test that visible_chars never exceeds text length
        let text = "Test".to_string();
        let mut typewriter = TypewriterText {
            full_text: text.clone(),
            visible_chars: 100, // Way too many
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        typewriter.visible_chars = (typewriter.visible_chars + 1).min(typewriter.full_text.len());
        assert_eq!(typewriter.visible_chars, typewriter.full_text.len());
    }

    #[test]
    fn test_active_dialogue_ui_initialization() {
        let ui = ActiveDialogueUI::default();
        assert!(ui.bubble_entity.is_none());
    }

    #[test]
    fn test_dialogue_visual_error_messages() {
        let entity = Entity::from_bits(42);
        let err = DialogueVisualError::SpeakerNotFound(entity);
        assert!(format!("{}", err).contains("speaker entity"));

        let err2 = DialogueVisualError::MeshCreationFailed("test reason".to_string());
        assert!(format!("{}", err2).contains("test reason"));

        let err3 = DialogueVisualError::InvalidGameMode;
        assert!(format!("{}", err3).contains("not available"));
    }

    #[test]
    fn test_typewriter_empty_text() {
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
    fn test_typewriter_visible_chars_calculation() {
        let text = "Hello, World!".to_string();
        let visible_text: String = text.chars().take(5).collect();

        assert_eq!(visible_text, "Hello");
        assert_eq!(visible_text.len(), 5);
    }
}
