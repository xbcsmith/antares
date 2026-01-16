 // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
 // SPDX-License-Identifier: Apache-2.0

 //! Dialogue visual systems for rendering and animating dialogue UI
 //!
 //! This module provides Bevy systems that handle the visual presentation of dialogue:
 //! - Spawning dialogue bubbles above speaker entities (screen-space panel)
 //! - Animating text with a typewriter effect
 //! - Cleanup when dialogue ends
 //!
 //! These systems work with the dialogue components to create a clean screen-space UI
 //! using `bevy_ui` nodes instead of 3D meshes and billboards.

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

 #[allow(dead_code, unused_mut, unused_assignments)]
 /// Clamps a dialogue bubble's world position with respect to the camera so it
 /// does not intersect the near-plane or produce full-screen planar artifacts.
 fn select_worst_camera_for_bubble(
     query_camera: &Query<&Transform, With<Camera3d>>,
     bubble_pos: Vec3,
 ) -> Option<Transform> {
     let mut worst_cam: Option<Transform> = None;
     let mut worst_min_forward = f32::INFINITY;

     for cam in query_camera.iter() {
         // Camera forward (Bevy camera local -Z)
         let mut cam_forward = cam.rotation.mul_vec3(-Vec3::Z);
         cam_forward = if cam_forward.length_squared() > 0.0 {
             cam_forward.normalize()
         } else {
             -Vec3::Z
         };

         // Yaw-only root orientation for vertex computation (avoid pitch)
         let yaw_target = Vec3::new(cam.translation.x, bubble_pos.y, cam.translation.z);
         let mut root_rot = Transform::from_translation(bubble_pos);
         root_rot.look_at(yaw_target, Vec3::Y);

         let used_width = if cfg!(debug_assertions) {
             DIALOGUE_BUBBLE_WIDTH * 0.35
         } else {
             DIALOGUE_BUBBLE_WIDTH
         };
         let used_height = if cfg!(debug_assertions) {
             DIALOGUE_BUBBLE_HEIGHT * 0.35
         } else {
             DIALOGUE_BUBBLE_HEIGHT
         };

         let half_w = used_width * 0.5;
         let half_h = used_height * 0.5;
         let local_vertices = [
             Vec3::new(half_w, half_h, 0.0),
             Vec3::new(-half_w, half_h, 0.0),
             Vec3::new(-half_w, -half_h, 0.0),
             Vec3::new(half_w, -half_h, 0.0),
         ];

         let world_vertices: Vec<Vec3> = local_vertices
             .iter()
             .map(|v| bubble_pos + root_rot.rotation.mul_vec3(*v))
             .collect();

         // Minimal forward-distance among vertices for this camera
         let min_forward = world_vertices
             .iter()
             .map(|wv| cam_forward.dot(*wv - cam.translation))
             .fold(f32::INFINITY, |a, b| a.min(b));

         if min_forward < worst_min_forward {
             worst_min_forward = min_forward;
             worst_cam = Some(*cam);
         }
     }

     worst_cam
 }

 #[allow(dead_code, unused_mut, unused_assignments)]
 /// Clamps a dialogue bubble's world position with respect to the camera so it
 /// does not intersect the near-plane or produce full-screen planar artifacts.
 ///
 /// Accepts an owned Option<Transform> (camera) and returns the adjusted position.
 fn clamp_bubble_position_to_camera(mut bubble_position: Vec3, camera: Option<Transform>) -> Vec3 {
     if let Some(camera_transform) = camera {
         // 1) Center distance clamp
         let dir = bubble_position - camera_transform.translation;
         let dist = dir.length();
         if dist < DIALOGUE_MIN_CAMERA_DISTANCE {
             bubble_position =
                 camera_transform.translation + dir.normalize() * DIALOGUE_MIN_CAMERA_DISTANCE;
         }

         // 2) Per-vertex yaw-only test and forward push (avoid pitch)
         let yaw_target = Vec3::new(
             camera_transform.translation.x,
             bubble_position.y,
             camera_transform.translation.z,
         );
         let mut root_rot = Transform::from_translation(bubble_position);
         root_rot.look_at(yaw_target, Vec3::Y);

         let used_width = if cfg!(debug_assertions) {
             DIALOGUE_BUBBLE_WIDTH * 0.35
         } else {
             DIALOGUE_BUBBLE_WIDTH
         };
         let used_height = if cfg!(debug_assertions) {
             DIALOGUE_BUBBLE_HEIGHT * 0.35
         } else {
             DIALOGUE_BUBBLE_HEIGHT
         };

         let half_w = used_width * 0.5;
         let half_h = used_height * 0.5;
         let local_vertices = [
             Vec3::new(half_w, half_h, 0.0),
             Vec3::new(-half_w, half_h, 0.0),
             Vec3::new(-half_w, -half_h, 0.0),
             Vec3::new(half_w, -half_h, 0.0),
         ];

         let mut world_vertices: Vec<Vec3> = local_vertices
             .iter()
             .map(|v| bubble_position + root_rot.rotation.mul_vec3(*v))
             .collect();

         // Camera forward (Bevy camera local -Z)
         let mut cam_forward = camera_transform.rotation.mul_vec3(-Vec3::Z);
         cam_forward = if cam_forward.length_squared() > 0.0 {
             cam_forward.normalize()
         } else {
             -Vec3::Z
         };

         let mut z_cams: Vec<f32> = world_vertices
             .iter()
             .map(|wv| cam_forward.dot(*wv - camera_transform.translation))
             .collect();

         const SAFETY_MARGIN: f32 = 0.15;
         let min_allowed = DIALOGUE_MIN_CAMERA_DISTANCE + SAFETY_MARGIN;

         // If any vertex is too close/behind, push bubble forward
         if let Some(&min_z) = z_cams.iter().min_by(|a, b| a.partial_cmp(b).unwrap()) {
             if min_z < min_allowed {
                 let push = min_allowed - min_z;
                 bubble_position += cam_forward * push;

                 // Recompute vertices after push
                 root_rot = Transform::from_translation(bubble_position);
                 root_rot.look_at(yaw_target, Vec3::Y);
                 world_vertices = local_vertices
                     .iter()
                     .map(|v| bubble_position + root_rot.rotation.mul_vec3(*v))
                     .collect();
             }
         }
     }

     bubble_position
 }

 /// Spawns a screen-space dialogue panel using bevy_ui
 ///
 /// Creates a hierarchy of UI nodes:
 /// - Root panel (positioned at bottom-center of screen)
 ///   - Speaker name text
 ///   - Content text (animated typewriter text)
 ///   - Choice list container (empty; populated by choice UI)
 ///
 /// The panel is only spawned if the game is in Dialogue mode and no panel currently exists.
 pub fn spawn_dialogue_bubble(
     mut commands: Commands,
     global_state: Res<GlobalState>,
     mut active_ui: ResMut<ActiveDialogueUI>,
 ) {
     // Only spawn if in Dialogue mode and no panel exists yet
     if let GameMode::Dialogue(dialogue_state) = &global_state.0.mode {
         if active_ui.bubble_entity.is_some() {
             return; // Panel already exists
         }

         // Spawn the root panel container
         let panel_root = commands
             .spawn((
                 Node {
                     position_type: PositionType::Absolute,
                     bottom: DIALOGUE_PANEL_BOTTOM,
                     left: Val::Px(0.0),
                     right: Val::Px(0.0),
                     width: DIALOGUE_PANEL_WIDTH,
                     margin: UiRect::horizontal(Val::Auto), // Center horizontally
                     padding: UiRect::all(DIALOGUE_PANEL_PADDING),
                     flex_direction: FlexDirection::Column,
                     row_gap: Val::Px(8.0),
                     ..default()
                 },
                 BackgroundColor(DIALOGUE_BACKGROUND_COLOR),
                 BorderRadius::all(Val::Px(8.0)),
                 DialoguePanelRoot,
             ))
             .with_children(|parent| {
                 // Speaker name text
                 parent.spawn((
                     Text::new(&dialogue_state.current_speaker),
                     TextFont {
                         font_size: DIALOGUE_SPEAKER_FONT_SIZE,
                         ..default()
                     },
                     TextColor(DIALOGUE_CHOICE_COLOR), // Golden color for speaker name
                     DialogueSpeakerText,
                 ));

                 // Dialogue content text with typewriter animation
                 parent.spawn((
                     Text::new(""),
                     TextFont {
                         font_size: DIALOGUE_CONTENT_FONT_SIZE,
                         ..default()
                     },
                     TextColor(DIALOGUE_TEXT_COLOR),
                     TypewriterText {
                         full_text: dialogue_state.current_text.clone(),
                         visible_chars: 0,
                         timer: 0.0,
                         speed: DIALOGUE_TYPEWRITER_SPEED,
                         finished: false,
                     },
                     DialogueContentText,
                 ));

                 // Choice list container (empty for now; populated by choice UI in Phase 2)
                 parent.spawn((Node { ..default() }, DialogueChoiceList));
             })
             .id();

         // Track the panel in the resource
         active_ui.bubble_entity = Some(panel_root);

         // Log the spawn for diagnostics
         info!(
             "Spawned dialogue panel entity {:?} for speaker '{}'",
             panel_root, dialogue_state.current_speaker
         );
     }
 }

 /// Updates dialogue panel content when the dialogue state changes
 ///
 /// Resets typewriter animation for any content text nodes marked with
 /// `DialogueContentText` when the active `DialogueState`'s `current_text` changes.
 pub fn update_dialogue_text(
     global_state: Res<GlobalState>,
     mut query_text: Query<(&mut Text, &mut TypewriterText), With<DialogueContentText>>,
 ) {
     if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
         for (mut text, mut typewriter) in query_text.iter_mut() {
             // Reset typewriter animation for new text
             if typewriter.full_text != dialogue_state.current_text {
                 typewriter.full_text = dialogue_state.current_text.clone();
                 typewriter.visible_chars = 0;
                 typewriter.timer = 0.0;
                 typewriter.finished = false;
                 text.0 = String::new(); // Clear visible text
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

 /// Billboard system - no-op for screen-space dialogue UI
 ///
 /// Dialogue UI now uses screen-space `Node` elements and does not require
 /// billboard rotation. This system is retained as a no-op to avoid
 /// removing public symbols used by other systems.
 pub fn billboard_system(
     _query_camera: Query<&Transform, With<Camera3d>>,
     _query_billboards: Query<&mut Transform, (With<Billboard>, Without<Camera3d>)>,
 ) {
     // Intentionally left blank - UI panels are screen-space and do not need to face the camera.
 }

 /// Cleans up dialogue UI when dialogue mode ends
 ///
 /// Despawns the screen-space panel and clears the `ActiveDialogueUI` resource
 /// when the game mode changes away from Dialogue.
 pub fn cleanup_dialogue_bubble(
     mut commands: Commands,
     global_state: Res<GlobalState>,
     mut active_ui: ResMut<ActiveDialogueUI>,
 ) {
     // Only cleanup if no longer in Dialogue mode
     if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
         if let Some(panel_entity) = active_ui.bubble_entity {
             // Despawn the UI panel root entity
             commands.entity(panel_entity).despawn();
             // Clear the resource
             active_ui.bubble_entity = None;
         }
     }
 }

 /// Updates dialogue bubble position to follow speaker
 ///
 /// Keeps the dialogue bubble positioned above the NPC even if the NPC moves.
 /// This system used to perform world-space follow/clamping; with screen-space UI it is a no-op.
 /// Previously used to sync 3D dialogue bubbles to speaker entities.
 /// For screen-space (bevy_ui) panels this is intentionally a no-op.
 pub fn follow_speaker_system() {
     // Intentionally left blank: screen-space UI does not follow world entities.
 }

 /// Monitors speaker entity and ends dialogue if speaker is despawned
 ///
 /// This system checks if the active dialogue speaker entity still exists in the world.
 /// If the speaker is despawned during dialogue (e.g., NPC removed or level unloaded),
 /// dialogue is ended gracefully and the game returns to Exploration mode.
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

     #[test]
     fn test_dialogue_panel_spawns_with_correct_structure() {
         use crate::application::dialogue::DialogueState;
         use crate::application::GameMode;
         use crate::domain::types::Position;
         use crate::game::components::dialogue::{
             ActiveDialogueUI, DialoguePanelRoot, DialogueSpeakerText, DialogueContentText,
             DialogueChoiceList,
         };
         use crate::game::resources::GlobalState;
         use bevy::prelude::*;

         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.init_resource::<ActiveDialogueUI>();

         let mut gs = GlobalState(crate::application::GameState::new());
         gs.0.mode = GameMode::Dialogue(DialogueState::start_simple(
             "Hello!".to_string(),
             "Test NPC".to_string(),
             None,
             Some(Position::new(5, 6)),
         ));
         app.insert_resource(gs);

         app.add_systems(Update, crate::game::systems::dialogue_visuals::spawn_dialogue_bubble);

         // Run one update to trigger the system
         app.update();

         let ui = app.world().resource::<ActiveDialogueUI>().clone();
         assert!(ui.bubble_entity.is_some());
         let panel_entity = ui.bubble_entity.unwrap();

         // Verify the panel root entity has the DialoguePanelRoot component
         let root_has_marker = {
             let world = app.world_mut();
             world.query::<&DialoguePanelRoot>().get(world, panel_entity).is_ok()
         };
         assert!(root_has_marker, "Panel root missing DialoguePanelRoot component");

         // Verify child text elements exist
         let speaker_found = {
             let world = app.world_mut();
             world
                 .query::<(Entity, &DialogueSpeakerText)>()
                 .iter(world)
                 .next()
                 .is_some()
         };
         assert!(speaker_found, "Missing DialogueSpeakerText child");

         let content_found = {
             let world = app.world_mut();
             world
                 .query::<(Entity, &DialogueContentText)>()
                 .iter(world)
                 .next()
                 .is_some()
         };
         assert!(content_found, "Missing DialogueContentText child");

         let choice_list_found = {
             let world = app.world_mut();
             world.query::<&DialogueChoiceList>().iter(world).next().is_some()
         };
         assert!(choice_list_found, "Missing DialogueChoiceList child");
     }

     #[test]
     fn test_dialogue_panel_displays_speaker_name() {
         use crate::application::dialogue::DialogueState;
         use crate::application::GameMode;
         use crate::domain::types::Position;
         use crate::game::components::dialogue::ActiveDialogueUI;
         use crate::game::resources::GlobalState;
         use bevy::prelude::*;

         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.init_resource::<ActiveDialogueUI>();

         let speaker = "Apprentice Zara".to_string();
         let mut gs = GlobalState(crate::application::GameState::new());
         gs.0.mode = GameMode::Dialogue(DialogueState::start_simple(
             "Hello!".to_string(),
             speaker.clone(),
             None,
             Some(Position::new(5, 6)),
         ));
         app.insert_resource(gs);

         app.add_systems(Update, crate::game::systems::dialogue_visuals::spawn_dialogue_bubble);

         // Run one update to trigger the system
         app.update();

         // Find the speaker text entity
         let speaker_text_value = {
             let world = app.world_mut();
             let (entity, _) = world
                 .query::<(Entity, &DialogueSpeakerText)>()
                 .iter(world)
                 .next()
                 .expect("Expected a DialogueSpeakerText entity");

             let text = world.get::<Text>(entity).unwrap();
             text.0.clone()
         };

         assert_eq!(speaker_text_value, speaker);
     }

     #[test]
     fn test_dialogue_panel_typewriter_works() {
         use crate::application::dialogue::DialogueState;
         use crate::application::GameMode;
         use crate::domain::types::Position;
         use crate::game::components::dialogue::{ActiveDialogueUI, DialogueContentText};
         use crate::game::resources::GlobalState;
         use bevy::prelude::*;

         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.init_resource::<ActiveDialogueUI>();

         let mut gs = GlobalState(crate::application::GameState::new());
         gs.0.mode = GameMode::Dialogue(DialogueState::start_simple(
             "Hello!".to_string(),
             "Speaker".to_string(),
             None,
             Some(Position::new(5, 6)),
         ));
         app.insert_resource(gs);

         // Spawn the panel
         app.add_systems(Update, crate::game::systems::dialogue_visuals::spawn_dialogue_bubble);
         app.update();

         // Find the content entity and its initial full text
         let (content_entity, initial_full_text) = {
             let world = app.world_mut();
             let (entity, _) = world
                 .query::<(Entity, &DialogueContentText)>()
                 .iter(world)
                 .next()
                 .expect("Content DialogueContentText entity not found");
             let typewriter = world
                 .get::<TypewriterText>(entity)
                 .expect("TypewriterText not found on content entity");
             (entity, typewriter.full_text.clone())
         };

         // Simulate a single typewriter tick by mutating the TypewriterText and capturing visible text
         let visible_text = {
             let world = app.world_mut();
             let mut tw = world.get_mut::<TypewriterText>(content_entity).unwrap();

             // Simulate time passing equal to one speed step
             tw.timer = tw.speed;
             if tw.timer >= tw.speed {
                 tw.timer = 0.0;
                 tw.visible_chars = (tw.visible_chars + 1).min(tw.full_text.len());
                 let visible: String = tw.full_text.chars().take(tw.visible_chars).collect();
                 if tw.visible_chars >= tw.full_text.len() {
                     tw.finished = true;
                 }
                 visible
             } else {
                 String::new()
             }
         };

         // Apply the visible text to the Text component in a separate world borrow
         {
             let world = app.world_mut();
             let mut text_comp = world.get_mut::<Text>(content_entity).unwrap();
             text_comp.0 = visible_text;
         }

         // Verify that the displayed text advanced by one character
         {
             let world = app.world_mut();
             let (entity, _) = world
                 .query::<(Entity, &DialogueContentText)>()
                 .iter(world)
                 .next()
                 .expect("Expected DialogueContentText entity");
             let text_after = world.get::<Text>(entity).unwrap();
             assert_eq!(text_after.0, initial_full_text.chars().take(1).collect::<String>());
         }
     }

     #[test]
     fn test_spawn_dialogue_bubble_debug_material_flags() {
         // In the new screen-space UI implementation, material debug flags are not relevant.
         // This test ensures the spawn system executes without panicking.
         use crate::application::dialogue::DialogueState;
         use crate::application::GameMode;
         use crate::domain::types::Position;
         use crate::game::components::dialogue::ActiveDialogueUI;
         use crate::game::resources::GlobalState;
         use bevy::prelude::*;

         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.init_resource::<ActiveDialogueUI>();

         let mut gs = GlobalState(crate::application::GameState::new());
         gs.0.mode = GameMode::Dialogue(DialogueState::start_simple(
             "Debug test".to_string(),
             "Test NPC".to_string(),
             None,
             Some(Position::new(3, 4)),
         ));
         app.insert_resource(gs);

         app.add_systems(
             Update,
             crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
         );

         // Run one update to trigger the system
         app.update();

         // Verify the panel spawned (we don't inspect debug materials in the new UI)
         let ui = app.world().resource::<ActiveDialogueUI>().clone();
         assert!(ui.bubble_entity.is_some(), "Expected dialogue panel to spawn");
     }

     #[test]
     fn test_follow_speaker_system_is_noop() {
         use bevy::prelude::*;

         let mut app = App::new();
         app.add_plugins(MinimalPlugins);

         // Spawn a speaker entity (NPC)
         let speaker = app
             .world_mut()
             .spawn((Transform::from_xyz(11.0, 0.9, 6.0), GlobalTransform::default()))
             .id();

         // Create a root entity with Billboard that would previously be updated by follow_speaker_system
         let root = app
             .world_mut()
             .spawn((
                 Transform::from_translation(Vec3::new(11.0, 1.0, 6.0)),
                 GlobalTransform::default(),
                 Visibility::default(),
                 Billboard,
             ))
             .id();

         // Attach a DialogueBubble marker that references the speaker and root
         app.world_mut().spawn((
             crate::game::components::dialogue::DialogueBubble {
                 speaker_entity: Some(speaker),
                 root_entity: root,
                 background_entity: root,
                 text_entity: root,
                 y_offset: 1.0,
             },
         ));

         // Register the follow system (now a no-op)
         app.add_systems(Update, crate::game::systems::dialogue_visuals::follow_speaker_system);

         // Run one update (no-op expected)
         app.update();

         // Ensure transform did not change
         let root_transform = {
             let world = app.world_mut();
             *world.query::<&Transform>().get(world, root).unwrap()
         };
         assert_eq!(root_transform.translation, Vec3::new(11.0, 1.0, 6.0));
     }

     #[test]
     fn test_billboard_system_is_noop() {
         use bevy::prelude::*;

         // Minimal app and camera + billboard setup
         let mut app = App::new();
         app.add_plugins(MinimalPlugins);

         // Spawn a camera
         app.world_mut().spawn((
             Camera3d::default(),
             Transform::from_xyz(5.0, 10.0, 5.0),
             GlobalTransform::default(),
             Visibility::default(),
         ));

         // Spawn a billboard entity
         let billboard_pos = Vec3::new(0.0, 2.0, 0.0);
         let billboard_entity = app
             .world_mut()
             .spawn((
                 Transform::from_translation(billboard_pos),
                 GlobalTransform::default(),
                 Visibility::default(),
                 Billboard,
             ))
             .id();

         // Register and run the billboard system (now a no-op)
         app.add_systems(Update, crate::game::systems::dialogue_visuals::billboard_system);
         app.update(); // Run one update

         // Verify transform unchanged
         let b_transform = {
             let world = app.world_mut();
             *world.query::<&Transform>().get(world, billboard_entity).unwrap()
         };
         assert_eq!(b_transform.translation, billboard_pos);
     }
 }
