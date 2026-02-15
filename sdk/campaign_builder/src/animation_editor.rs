// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Animation editor UI for the campaign builder
//!
//! This module provides UI components for creating, editing, and previewing
//! keyframe-based animations for creatures. Animations allow creatures to have
//! dynamic poses and transformations over time.
//!
//! # Architecture
//!
//! The animation editor integrates with the creatures editor to:
//! - Display a timeline view of animation keyframes
//! - Create and edit keyframes at specific time points
//! - Preview animation playback in real-time
//! - Configure animation properties (duration, looping)
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::animation_editor::AnimationEditorState;
//! use antares::domain::visual::{AnimationDefinition, CreatureDefinition};
//!
//! let mut state = AnimationEditorState::new();
//! let mut creature = CreatureDefinition::default();
//!
//! // In your egui UI context:
//! // state.show(ui, &mut animations);
//! ```

use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
use antares::domain::visual::MeshTransform;
use eframe::egui;

/// State for the animation editor UI
#[derive(Debug, Clone)]
pub struct AnimationEditorState {
    /// Currently selected animation index
    pub selected_animation: Option<usize>,

    /// Currently selected keyframe index
    pub selected_keyframe: Option<usize>,

    /// Show animation creation dialog
    pub show_create_dialog: bool,

    /// Show keyframe editor
    pub show_keyframe_editor: bool,

    /// Buffer for creating new animation
    pub create_buffer: AnimationCreateBuffer,

    /// Buffer for editing keyframe
    pub keyframe_buffer: KeyframeBuffer,

    /// Playback state
    pub playback_state: PlaybackState,

    /// Show preview panel
    pub show_preview: bool,

    /// Timeline zoom level (pixels per second)
    pub timeline_zoom: f32,

    /// Timeline scroll position
    pub timeline_scroll: f32,
}

/// Buffer for creating a new animation
#[derive(Debug, Clone)]
pub struct AnimationCreateBuffer {
    /// Name of the animation
    pub name: String,

    /// Duration in seconds
    pub duration: f32,

    /// Whether the animation loops
    pub looping: bool,
}

/// Buffer for editing a keyframe
#[derive(Debug, Clone)]
pub struct KeyframeBuffer {
    /// Time of the keyframe
    pub time: f32,

    /// Mesh index to transform
    pub mesh_index: usize,

    /// Transform to apply
    pub transform: MeshTransform,
}

/// Playback state for animation preview
#[derive(Debug, Clone)]
pub struct PlaybackState {
    /// Whether animation is currently playing
    pub playing: bool,

    /// Current playback time
    pub current_time: f32,

    /// Playback speed multiplier
    pub speed: f32,
}

impl Default for AnimationEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationEditorState {
    /// Creates a new animation editor state
    pub fn new() -> Self {
        Self {
            selected_animation: None,
            selected_keyframe: None,
            show_create_dialog: false,
            show_keyframe_editor: false,
            create_buffer: AnimationCreateBuffer::default(),
            keyframe_buffer: KeyframeBuffer::default(),
            playback_state: PlaybackState::default(),
            show_preview: true,
            timeline_zoom: 100.0, // 100 pixels per second
            timeline_scroll: 0.0,
        }
    }

    /// Shows the animation editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `animations` - Mutable list of animations to edit
    ///
    /// # Returns
    ///
    /// Returns `Some(AnimationAction)` if an action should be performed
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        animations: &mut [AnimationDefinition],
    ) -> Option<AnimationAction> {
        let mut action = None;

        ui.heading("Animation Editor");
        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ Create Animation").clicked() {
                self.show_create_dialog = true;
                self.create_buffer = AnimationCreateBuffer::default();
            }

            if ui.button("➕ Add Keyframe").clicked() {
                if self.selected_animation.is_some() {
                    self.show_keyframe_editor = true;
                    self.keyframe_buffer = KeyframeBuffer::new(self.playback_state.current_time, 0);
                }
            }

            ui.separator();

            // Playback controls
            let play_icon = if self.playback_state.playing {
                "⏸"
            } else {
                "▶"
            };

            if ui.button(play_icon).clicked() {
                self.playback_state.playing = !self.playback_state.playing;
            }

            if ui.button("⏹").clicked() {
                self.playback_state.playing = false;
                self.playback_state.current_time = 0.0;
            }

            ui.separator();

            ui.label("Speed:");
            ui.add(
                egui::DragValue::new(&mut self.playback_state.speed)
                    .speed(0.1)
                    .range(0.1..=5.0)
                    .suffix("x"),
            );

            ui.separator();

            ui.checkbox(&mut self.show_preview, "Show Preview");
        });

        ui.separator();

        // Animation list
        ui.group(|ui| {
            ui.label("Animations:");

            if animations.is_empty() {
                ui.label("No animations - click 'Create Animation' to add one");
            } else {
                ui.horizontal(|ui| {
                    for (idx, animation) in animations.iter().enumerate() {
                        let is_selected = self.selected_animation == Some(idx);
                        if ui.selectable_label(is_selected, &animation.name).clicked() {
                            self.selected_animation = Some(idx);
                            self.playback_state.current_time = 0.0;
                            self.playback_state.playing = false;
                        }
                    }
                });
            }
        });

        ui.separator();

        // Main editor area
        if let Some(anim_idx) = self.selected_animation {
            if let Some(animation) = animations.get_mut(anim_idx) {
                action = self.show_animation_editor(ui, animation);
            }
        } else {
            ui.label("Select an animation to edit");
        }

        // Create animation dialog
        if self.show_create_dialog {
            if let Some(new_animation) = self.show_create_dialog(ui.ctx()) {
                action = Some(AnimationAction::Create(new_animation));
                self.show_create_dialog = false;
            }
        }

        // Keyframe editor dialog
        if self.show_keyframe_editor {
            if let Some(anim_idx) = self.selected_animation {
                if let Some(keyframe_action) = self.show_keyframe_editor(ui.ctx()) {
                    match keyframe_action {
                        KeyframeAction::Add(keyframe) => {
                            action = Some(AnimationAction::AddKeyframe(anim_idx, keyframe));
                        }
                        KeyframeAction::Update(kf_idx, keyframe) => {
                            action =
                                Some(AnimationAction::UpdateKeyframe(anim_idx, kf_idx, keyframe));
                        }
                    }
                    self.show_keyframe_editor = false;
                }
            }
        }

        action
    }

    /// Shows the main animation editor panel
    fn show_animation_editor(
        &mut self,
        ui: &mut egui::Ui,
        animation: &mut AnimationDefinition,
    ) -> Option<AnimationAction> {
        let mut action = None;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut animation.name);
            });

            ui.horizontal(|ui| {
                ui.label("Duration:");
                ui.add(
                    egui::DragValue::new(&mut animation.duration)
                        .speed(0.1)
                        .range(0.1..=60.0)
                        .suffix(" seconds"),
                );
            });

            ui.checkbox(&mut animation.looping, "Loop");
        });

        ui.separator();

        // Timeline
        self.show_timeline(ui, animation);

        ui.separator();

        // Keyframe list
        ui.heading("Keyframes");

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                if animation.keyframes.is_empty() {
                    ui.label("No keyframes - click 'Add Keyframe' to create one");
                } else {
                    for (idx, keyframe) in animation.keyframes.iter().enumerate() {
                        let is_selected = self.selected_keyframe == Some(idx);

                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    is_selected,
                                    format!("Frame {} @ {:.2}s", idx, keyframe.time),
                                )
                                .clicked()
                            {
                                self.selected_keyframe = Some(idx);
                                self.playback_state.current_time = keyframe.time;
                            }

                            ui.label(format!("Mesh {}", keyframe.mesh_index));

                            if ui.button("✏").on_hover_text("Edit").clicked() {
                                self.keyframe_buffer = KeyframeBuffer::from_keyframe(keyframe);
                                self.show_keyframe_editor = true;
                            }

                            if ui.button("🗑").on_hover_text("Delete").clicked() {
                                action = Some(AnimationAction::DeleteKeyframe(idx));
                            }
                        });
                    }
                }
            });

        action
    }

    /// Shows the timeline visualization
    fn show_timeline(&mut self, ui: &mut egui::Ui, animation: &AnimationDefinition) {
        ui.heading("Timeline");

        let timeline_height = 80.0;
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), timeline_height),
            egui::Sense::click_and_drag(),
        );

        // Draw timeline background
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(30));

        // Draw time markers
        let duration = animation.duration;
        let pixels_per_second = self.timeline_zoom;
        let timeline_width = duration * pixels_per_second;

        // Draw time grid
        let num_seconds = duration.ceil() as i32;
        for i in 0..=num_seconds {
            let x = rect.min.x + (i as f32 * pixels_per_second);
            if x >= rect.min.x && x <= rect.max.x {
                ui.painter().line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                );

                ui.painter().text(
                    egui::pos2(x + 2.0, rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", i),
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_gray(180),
                );
            }
        }

        // Draw keyframes
        for keyframe in &animation.keyframes {
            let x = rect.min.x + (keyframe.time * pixels_per_second);
            if x >= rect.min.x && x <= rect.max.x {
                let y_center = rect.center().y;
                ui.painter().circle_filled(
                    egui::pos2(x, y_center),
                    5.0,
                    egui::Color32::from_rgb(100, 150, 255),
                );
            }
        }

        // Draw playhead
        let playhead_x = rect.min.x + (self.playback_state.current_time * pixels_per_second);
        if playhead_x >= rect.min.x && playhead_x <= rect.max.x {
            ui.painter().line_segment(
                [
                    egui::pos2(playhead_x, rect.min.y),
                    egui::pos2(playhead_x, rect.max.y),
                ],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 100)),
            );
        }

        // Show current time
        ui.label(format!(
            "Time: {:.2} / {:.2}s",
            self.playback_state.current_time, duration
        ));
    }

    /// Shows the create animation dialog
    fn show_create_dialog(&mut self, ctx: &egui::Context) -> Option<AnimationDefinition> {
        let mut result = None;
        let mut close_dialog = false;

        egui::Window::new("Create New Animation")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.create_buffer.name);
                });

                ui.horizontal(|ui| {
                    ui.label("Duration:");
                    ui.add(
                        egui::DragValue::new(&mut self.create_buffer.duration)
                            .speed(0.1)
                            .range(0.1..=60.0)
                            .suffix(" seconds"),
                    );
                });

                ui.checkbox(&mut self.create_buffer.looping, "Loop");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Create").clicked() {
                        if !self.create_buffer.name.is_empty() {
                            result = Some(self.create_buffer.to_animation());
                            close_dialog = true;
                        }
                    }

                    if ui.button("✗ Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if close_dialog {
            self.show_create_dialog = false;
        }

        result
    }

    /// Shows the keyframe editor dialog
    /// Shows the keyframe editor dialog
    fn show_keyframe_editor(&mut self, ctx: &egui::Context) -> Option<KeyframeAction> {
        let mut result = None;
        let mut close_dialog = false;

        egui::Window::new("Edit Keyframe")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.time)
                            .speed(0.01)
                            .range(0.0..=60.0)
                            .suffix(" seconds"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Mesh Index:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.mesh_index)
                            .speed(1)
                            .range(0..=100),
                    );
                });

                ui.separator();

                ui.label("Transform:");

                // Translation (position)
                ui.horizontal(|ui| {
                    ui.label("Translation:");
                    ui.label("X:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.translation[0])
                            .speed(0.1),
                    );
                    ui.label("Y:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.translation[1])
                            .speed(0.1),
                    );
                    ui.label("Z:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.translation[2])
                            .speed(0.1),
                    );
                });

                // Rotation
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    ui.label("X:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.rotation[0])
                            .speed(0.01),
                    );
                    ui.label("Y:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.rotation[1])
                            .speed(0.01),
                    );
                    ui.label("Z:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.rotation[2])
                            .speed(0.01),
                    );
                });

                // Scale
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.label("X:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.scale[0])
                            .speed(0.01),
                    );
                    ui.label("Y:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.scale[1])
                            .speed(0.01),
                    );
                    ui.label("Z:");
                    ui.add(
                        egui::DragValue::new(&mut self.keyframe_buffer.transform.scale[2])
                            .speed(0.01),
                    );
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Save").clicked() {
                        let keyframe = self.keyframe_buffer.to_keyframe();
                        if let Some(kf_idx) = self.selected_keyframe {
                            result = Some(KeyframeAction::Update(kf_idx, keyframe));
                        } else {
                            result = Some(KeyframeAction::Add(keyframe));
                        }
                        close_dialog = true;
                    }

                    if ui.button("✗ Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if close_dialog {
            self.show_keyframe_editor = false;
            self.selected_keyframe = None;
        }

        result
    }

    /// Updates playback state (call this every frame)
    pub fn update(&mut self, delta_time: f32, animation: &AnimationDefinition) {
        if self.playback_state.playing {
            self.playback_state.current_time += delta_time * self.playback_state.speed;

            if self.playback_state.current_time >= animation.duration {
                if animation.looping {
                    self.playback_state.current_time %= animation.duration;
                } else {
                    self.playback_state.current_time = animation.duration;
                    self.playback_state.playing = false;
                }
            }
        }
    }
}

impl Default for AnimationCreateBuffer {
    fn default() -> Self {
        Self {
            name: "New Animation".to_string(),
            duration: 1.0,
            looping: false,
        }
    }
}

impl AnimationCreateBuffer {
    /// Converts the buffer to an AnimationDefinition
    pub fn to_animation(&self) -> AnimationDefinition {
        AnimationDefinition {
            name: self.name.clone(),
            duration: self.duration,
            keyframes: vec![],
            looping: self.looping,
        }
    }
}

impl Default for KeyframeBuffer {
    fn default() -> Self {
        Self {
            time: 0.0,
            mesh_index: 0,
            transform: MeshTransform::default(),
        }
    }
}

impl KeyframeBuffer {
    /// Creates a new keyframe buffer with default transform
    pub fn new(time: f32, mesh_index: usize) -> Self {
        Self {
            time,
            mesh_index,
            transform: MeshTransform::default(),
        }
    }

    /// Creates a buffer from an existing keyframe
    pub fn from_keyframe(keyframe: &Keyframe) -> Self {
        Self {
            time: keyframe.time,
            mesh_index: keyframe.mesh_index,
            transform: keyframe.transform,
        }
    }

    /// Converts the buffer to a Keyframe
    pub fn to_keyframe(&self) -> Keyframe {
        Keyframe {
            time: self.time,
            mesh_index: self.mesh_index,
            transform: self.transform,
        }
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            current_time: 0.0,
            speed: 1.0,
        }
    }
}

/// Actions that can be performed on animations
#[derive(Debug, Clone)]
pub enum AnimationAction {
    /// Create a new animation
    Create(AnimationDefinition),

    /// Delete an animation
    Delete(usize),

    /// Add a keyframe to an animation
    AddKeyframe(usize, Keyframe),

    /// Update a keyframe
    UpdateKeyframe(usize, usize, Keyframe),

    /// Delete a keyframe
    DeleteKeyframe(usize),
}

/// Actions for keyframe editing
#[derive(Debug, Clone)]
enum KeyframeAction {
    /// Add a new keyframe
    Add(Keyframe),

    /// Update an existing keyframe
    Update(usize, Keyframe),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_editor_state_new() {
        let state = AnimationEditorState::new();
        assert_eq!(state.selected_animation, None);
        assert_eq!(state.selected_keyframe, None);
        assert!(!state.show_create_dialog);
        assert!(!state.show_keyframe_editor);
        assert!(state.show_preview);
        assert_eq!(state.timeline_zoom, 100.0);
        assert_eq!(state.timeline_scroll, 0.0);
    }

    #[test]
    fn test_animation_create_buffer_default() {
        let buffer = AnimationCreateBuffer::default();
        assert_eq!(buffer.name, "New Animation");
        assert_eq!(buffer.duration, 1.0);
        assert!(!buffer.looping);
    }

    #[test]
    fn test_animation_create_buffer_to_animation() {
        let buffer = AnimationCreateBuffer {
            name: "Test Animation".to_string(),
            duration: 2.5,
            looping: true,
        };

        let animation = buffer.to_animation();
        assert_eq!(animation.name, "Test Animation");
        assert_eq!(animation.duration, 2.5);
        assert!(animation.looping);
        assert!(animation.keyframes.is_empty());
    }

    #[test]
    fn test_keyframe_buffer_default() {
        let buffer = KeyframeBuffer::default();
        assert_eq!(buffer.time, 0.0);
        assert_eq!(buffer.mesh_index, 0);
    }

    #[test]
    fn test_keyframe_buffer_to_keyframe() {
        let buffer = KeyframeBuffer {
            time: 1.5,
            mesh_index: 2,
            transform: MeshTransform::default(),
        };

        let keyframe = buffer.to_keyframe();
        assert_eq!(keyframe.time, 1.5);
        assert_eq!(keyframe.mesh_index, 2);
    }

    #[test]
    fn test_playback_state_default() {
        let state = PlaybackState::default();
        assert!(!state.playing);
        assert_eq!(state.current_time, 0.0);
        assert_eq!(state.speed, 1.0);
    }

    #[test]
    fn test_animation_action_variants() {
        let animation = AnimationDefinition {
            name: "Test".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: false,
        };

        let action = AnimationAction::Create(animation.clone());
        assert!(matches!(action, AnimationAction::Create(_)));

        let action = AnimationAction::Delete(0);
        assert!(matches!(action, AnimationAction::Delete(0)));
    }

    #[test]
    fn test_update_playback_looping() {
        let mut state = AnimationEditorState::new();
        state.playback_state.playing = true;

        let animation = AnimationDefinition {
            name: "Test".to_string(),
            duration: 2.0,
            keyframes: vec![],
            looping: true,
        };

        state.update(1.0, &animation);
        assert_eq!(state.playback_state.current_time, 1.0);

        state.update(1.5, &animation);
        // Should wrap: 1.0 + 1.5 = 2.5, 2.5 % 2.0 = 0.5
        assert_eq!(state.playback_state.current_time, 0.5);
    }

    #[test]
    fn test_update_playback_non_looping() {
        let mut state = AnimationEditorState::new();
        state.playback_state.playing = true;

        let animation = AnimationDefinition {
            name: "Test".to_string(),
            duration: 2.0,
            keyframes: vec![],
            looping: false,
        };

        state.update(1.0, &animation);
        assert_eq!(state.playback_state.current_time, 1.0);
        assert!(state.playback_state.playing);

        state.update(1.5, &animation);
        // Should stop at duration: 1.0 + 1.5 = 2.5, clamped to 2.0
        assert_eq!(state.playback_state.current_time, 2.0);
        assert!(!state.playback_state.playing);
    }
}
