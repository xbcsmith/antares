// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Configuration Editor for Campaign Builder
//!
//! Provides a visual editor for editing `config.ron` files following the
//! existing editor patterns (SpellsEditorState, ItemsEditorState, etc.).
//!
//! # Features
//!
//! - Edit game configuration with visual UI
//! - Section-based layout (Graphics, Audio, Controls, Camera)
//! - Validation error display
//! - Save/load/reload configuration files
//!
//! # Examples
//!
//! ```ignore
//! use campaign_builder::config_editor::ConfigEditorState;
//!
//! let mut editor_state = ConfigEditorState::new();
//! // Use in ui code:
//! // editor_state.show(&mut ui, campaign_dir, unsaved_changes, status_message);
//! ```

use crate::ui_helpers::{EditorToolbar, ToolbarAction};
use antares::sdk::game_config::{CameraMode, GameConfig, ShadowQuality};
use eframe::egui;
use std::path::PathBuf;

/// State for the configuration editor
///
/// Manages the UI state and edit buffer for the game configuration editor.
/// Follows the same pattern as SpellsEditorState and ItemsEditorState.
#[derive(Debug, Clone)]
pub struct ConfigEditorState {
    /// Current game configuration being edited
    pub game_config: GameConfig,

    /// Track if configuration was successfully loaded
    pub has_loaded: bool,

    /// Section expansion state
    pub graphics_expanded: bool,
    pub audio_expanded: bool,
    pub controls_expanded: bool,
    pub camera_expanded: bool,

    /// Edit buffers for key bindings
    pub controls_move_forward_buffer: String,
    pub controls_move_back_buffer: String,
    pub controls_turn_left_buffer: String,
    pub controls_turn_right_buffer: String,
    pub controls_interact_buffer: String,
    pub controls_menu_buffer: String,
}

impl Default for ConfigEditorState {
    fn default() -> Self {
        Self {
            game_config: GameConfig::default(),
            has_loaded: false,
            graphics_expanded: true,
            audio_expanded: false,
            controls_expanded: false,
            camera_expanded: false,
            controls_move_forward_buffer: String::new(),
            controls_move_back_buffer: String::new(),
            controls_turn_left_buffer: String::new(),
            controls_turn_right_buffer: String::new(),
            controls_interact_buffer: String::new(),
            controls_menu_buffer: String::new(),
        }
    }
}

impl ConfigEditorState {
    /// Create a new ConfigEditorState with default values
    ///
    /// # Returns
    ///
    /// A new ConfigEditorState with default GameConfig and section visibility
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let editor = ConfigEditorState::new();
    /// assert!(!editor.has_loaded);
    /// assert_eq!(editor.game_config, GameConfig::default());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the configuration editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `campaign_dir` - Optional path to the campaign directory
    /// * `unsaved_changes` - Mutable reference to track unsaved changes
    /// * `status_message` - Mutable reference to display status messages
    ///
    /// # Examples
    ///
    /// ```ignore
    /// editor_state.show(
    ///     &mut ui,
    ///     campaign_dir.as_ref(),
    ///     &mut unsaved_changes,
    ///     &mut status_message,
    /// );
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        ui.heading("⚙️ Game Configuration");
        ui.add_space(5.0);

        // Toolbar for save/load/reload actions
        let toolbar_action = EditorToolbar::new("Config")
            .with_total_count(1)
            .with_id_salt("config_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::Save => {
                self.update_config_from_buffers();
                match self.save_config(campaign_dir) {
                    Ok(msg) => {
                        *status_message = msg;
                        *unsaved_changes = false;
                    }
                    Err(msg) => {
                        *status_message = msg;
                    }
                }
            }
            ToolbarAction::Load => {
                self.update_config_from_buffers();
                if self.load_config(campaign_dir) {
                    *status_message = "Config loaded successfully".to_string();
                } else {
                    *status_message = "Failed to load config".to_string();
                }
            }
            ToolbarAction::Reload => {
                if self.load_config(campaign_dir) {
                    *status_message = "Config reloaded from file".to_string();
                } else {
                    *status_message = "Failed to reload config".to_string();
                }
            }
            _ => {}
        }

        ui.add_space(10.0);

        // Display sections in a scrollable area
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.show_graphics_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_audio_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_controls_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_camera_section(ui, unsaved_changes);
            });
    }

    /// Show the graphics configuration section
    fn show_graphics_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("🖥️ Graphics Settings", |ui| {
            ui.add_space(5.0);

            // Resolution
            ui.horizontal(|ui| {
                ui.label("Resolution:");
                let mut width = self.game_config.graphics.resolution.0;
                let mut height = self.game_config.graphics.resolution.1;

                if ui
                    .add(egui::DragValue::new(&mut width).range(320..=7680))
                    .changed()
                {
                    self.game_config.graphics.resolution.0 = width;
                    *unsaved_changes = true;
                }

                ui.label("×");

                if ui
                    .add(egui::DragValue::new(&mut height).range(240..=4320))
                    .changed()
                {
                    self.game_config.graphics.resolution.1 = height;
                    *unsaved_changes = true;
                }
            });

            // Fullscreen
            if ui
                .checkbox(&mut self.game_config.graphics.fullscreen, "Fullscreen")
                .changed()
            {
                *unsaved_changes = true;
            }

            // VSync
            if ui
                .checkbox(&mut self.game_config.graphics.vsync, "VSync")
                .changed()
            {
                *unsaved_changes = true;
            }

            // MSAA Samples
            ui.horizontal(|ui| {
                ui.label("MSAA Samples:");
                let msaa_options = [1u32, 2, 4, 8, 16];
                let mut selected_index = msaa_options
                    .iter()
                    .position(|&x| x == self.game_config.graphics.msaa_samples)
                    .unwrap_or(2);

                let original_index = selected_index;
                egui::ComboBox::from_id_salt("msaa_samples")
                    .selected_text(format!("{}", msaa_options[selected_index]))
                    .show_index(ui, &mut selected_index, msaa_options.len(), |i| {
                        egui::WidgetText::from(format!("{}", msaa_options[i]))
                    });

                if selected_index != original_index {
                    self.game_config.graphics.msaa_samples = msaa_options[selected_index];
                    *unsaved_changes = true;
                }
            });

            // Shadow Quality
            ui.horizontal(|ui| {
                ui.label("Shadow Quality:");
                let quality_options = [
                    ShadowQuality::Low,
                    ShadowQuality::Medium,
                    ShadowQuality::High,
                    ShadowQuality::Ultra,
                ];
                let quality_names = ["Low", "Medium", "High", "Ultra"];
                let mut selected_index = quality_options
                    .iter()
                    .position(|&x| x == self.game_config.graphics.shadow_quality)
                    .unwrap_or(1);

                let original_index = selected_index;
                egui::ComboBox::from_id_salt("shadow_quality")
                    .selected_text(quality_names[selected_index])
                    .show_index(ui, &mut selected_index, quality_names.len(), |i| {
                        egui::WidgetText::from(quality_names[i])
                    });

                if selected_index != original_index {
                    self.game_config.graphics.shadow_quality = quality_options[selected_index];
                    *unsaved_changes = true;
                }
            });

            ui.add_space(5.0);
        });

        self.graphics_expanded = section_open.openness > 0.0;
    }

    /// Show the audio configuration section
    fn show_audio_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("🔊 Audio Settings", |ui| {
            ui.add_space(5.0);

            // Master Volume
            ui.label("Master Volume:");
            if ui
                .add(
                    egui::Slider::new(&mut self.game_config.audio.master_volume, 0.0..=1.0)
                        .step_by(0.05),
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            // Music Volume
            ui.label("Music Volume:");
            if ui
                .add(
                    egui::Slider::new(&mut self.game_config.audio.music_volume, 0.0..=1.0)
                        .step_by(0.05),
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            // SFX Volume
            ui.label("SFX Volume:");
            if ui
                .add(
                    egui::Slider::new(&mut self.game_config.audio.sfx_volume, 0.0..=1.0)
                        .step_by(0.05),
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            // Ambient Volume
            ui.label("Ambient Volume:");
            if ui
                .add(
                    egui::Slider::new(&mut self.game_config.audio.ambient_volume, 0.0..=1.0)
                        .step_by(0.05),
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            // Enable Audio
            if ui
                .checkbox(&mut self.game_config.audio.enable_audio, "Enable Audio")
                .changed()
            {
                *unsaved_changes = true;
            }

            ui.add_space(5.0);
        });

        self.audio_expanded = section_open.openness > 0.0;
    }

    /// Show the controls configuration section
    fn show_controls_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("⌨️ Controls Settings", |ui| {
            ui.add_space(5.0);

            // Movement Cooldown
            ui.horizontal(|ui| {
                ui.label("Movement Cooldown (s):");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.controls.movement_cooldown)
                            .range(0.0..=1.0)
                            .speed(0.01),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            ui.add_space(10.0);
            ui.label("Key Bindings (comma-separated):");

            // Move Forward
            ui.horizontal(|ui| {
                ui.label("Move Forward:");
                if ui
                    .text_edit_singleline(&mut self.controls_move_forward_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Move Back
            ui.horizontal(|ui| {
                ui.label("Move Back:");
                if ui
                    .text_edit_singleline(&mut self.controls_move_back_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Turn Left
            ui.horizontal(|ui| {
                ui.label("Turn Left:");
                if ui
                    .text_edit_singleline(&mut self.controls_turn_left_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Turn Right
            ui.horizontal(|ui| {
                ui.label("Turn Right:");
                if ui
                    .text_edit_singleline(&mut self.controls_turn_right_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Interact
            ui.horizontal(|ui| {
                ui.label("Interact:");
                if ui
                    .text_edit_singleline(&mut self.controls_interact_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Menu
            ui.horizontal(|ui| {
                ui.label("Menu:");
                if ui
                    .text_edit_singleline(&mut self.controls_menu_buffer)
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            ui.add_space(5.0);
        });

        self.controls_expanded = section_open.openness > 0.0;
    }

    /// Show the camera configuration section
    fn show_camera_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("📷 Camera Settings", |ui| {
            ui.add_space(5.0);

            // Camera Mode
            ui.horizontal(|ui| {
                ui.label("Camera Mode:");
                let mode_options = [
                    CameraMode::FirstPerson,
                    CameraMode::Tactical,
                    CameraMode::Isometric,
                ];
                let mode_names = ["First Person", "Tactical", "Isometric"];
                let mut selected_index = mode_options
                    .iter()
                    .position(|&x| x == self.game_config.camera.mode)
                    .unwrap_or(0);

                let original_index = selected_index;
                egui::ComboBox::from_id_salt("camera_mode")
                    .selected_text(mode_names[selected_index])
                    .show_index(ui, &mut selected_index, mode_names.len(), |i| {
                        egui::WidgetText::from(mode_names[i])
                    });

                if selected_index != original_index {
                    self.game_config.camera.mode = mode_options[selected_index];
                    *unsaved_changes = true;
                }
            });

            // Eye Height
            ui.horizontal(|ui| {
                ui.label("Eye Height:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.eye_height)
                            .range(0.1..=3.0)
                            .speed(0.05),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Field of View
            ui.horizontal(|ui| {
                ui.label("FOV (degrees):");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.fov)
                            .range(30.0..=120.0)
                            .speed(1.0),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Near Clip
            ui.horizontal(|ui| {
                ui.label("Near Clip:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.near_clip)
                            .range(0.01..=10.0)
                            .speed(0.01),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Far Clip
            ui.horizontal(|ui| {
                ui.label("Far Clip:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.far_clip)
                            .range(10.0..=10000.0)
                            .speed(10.0),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Smooth Rotation
            if ui
                .checkbox(
                    &mut self.game_config.camera.smooth_rotation,
                    "Smooth Rotation",
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            // Rotation Speed
            ui.horizontal(|ui| {
                ui.label("Rotation Speed (°/s):");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.rotation_speed)
                            .range(30.0..=360.0)
                            .speed(5.0),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Light Height
            ui.horizontal(|ui| {
                ui.label("Light Height:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.light_height)
                            .range(0.1..=20.0)
                            .speed(0.1),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Light Intensity
            ui.horizontal(|ui| {
                ui.label("Light Intensity:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.light_intensity)
                            .range(100000.0..=10000000.0)
                            .speed(100000.0),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Light Range
            ui.horizontal(|ui| {
                ui.label("Light Range:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.game_config.camera.light_range)
                            .range(10.0..=200.0)
                            .speed(1.0),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
                }
            });

            // Shadows Enabled
            if ui
                .checkbox(
                    &mut self.game_config.camera.shadows_enabled,
                    "Shadows Enabled",
                )
                .changed()
            {
                *unsaved_changes = true;
            }

            ui.add_space(5.0);
        });

        self.camera_expanded = section_open.openness > 0.0;
    }

    /// Load configuration from a file
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` - The campaign directory containing config.ron
    ///
    /// # Returns
    ///
    /// Returns true if load was successful, false otherwise
    fn load_config(&mut self, campaign_dir: Option<&PathBuf>) -> bool {
        match campaign_dir {
            Some(dir) => {
                let config_path = dir.join("config.ron");
                match GameConfig::load_or_default(&config_path) {
                    Ok(config) => {
                        self.game_config = config;
                        self.has_loaded = true;
                        self.update_edit_buffers();
                        true
                    }
                    Err(_) => false,
                }
            }
            None => false,
        }
    }

    /// Save configuration to a file
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` - The campaign directory where config.ron will be saved
    ///
    /// # Returns
    ///
    /// Returns Ok(message) if save was successful, Err(message) otherwise
    fn save_config(&mut self, campaign_dir: Option<&PathBuf>) -> Result<String, String> {
        match campaign_dir {
            Some(dir) => {
                let config_path = dir.join("config.ron");

                // Validate before saving
                if let Err(e) = self.game_config.validate() {
                    return Err(format!("Validation failed: {}", e));
                }

                // Serialize to RON
                match ron::ser::to_string_pretty(&self.game_config, Default::default()) {
                    Ok(contents) => {
                        // Write to file
                        match std::fs::write(&config_path, contents) {
                            Ok(_) => Ok(format!("Config saved to: {}", config_path.display())),
                            Err(e) => Err(format!("Failed to write config file: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to serialize config: {}", e)),
                }
            }
            None => Err("No campaign directory set".to_string()),
        }
    }

    /// Update edit buffers from current config values
    fn update_edit_buffers(&mut self) {
        self.controls_move_forward_buffer = self.game_config.controls.move_forward.join(", ");
        self.controls_move_back_buffer = self.game_config.controls.move_back.join(", ");
        self.controls_turn_left_buffer = self.game_config.controls.turn_left.join(", ");
        self.controls_turn_right_buffer = self.game_config.controls.turn_right.join(", ");
        self.controls_interact_buffer = self.game_config.controls.interact.join(", ");
        self.controls_menu_buffer = self.game_config.controls.menu.join(", ");
    }

    /// Update config from edit buffers
    fn update_config_from_buffers(&mut self) {
        self.game_config.controls.move_forward = self
            .controls_move_forward_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.game_config.controls.move_back = self
            .controls_move_back_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.game_config.controls.turn_left = self
            .controls_turn_left_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.game_config.controls.turn_right = self
            .controls_turn_right_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.game_config.controls.interact = self
            .controls_interact_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.game_config.controls.menu = self
            .controls_menu_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_editor_state_new() {
        let state = ConfigEditorState::new();
        assert!(!state.has_loaded);
        assert_eq!(state.game_config.graphics.resolution, (1280, 720));
    }

    #[test]
    fn test_config_editor_state_default() {
        let state = ConfigEditorState::default();
        assert!(!state.has_loaded);
    }

    #[test]
    fn test_config_editor_update_edit_buffers() {
        let mut state = ConfigEditorState::new();
        state.game_config.controls.move_forward = vec!["W".to_string(), "ArrowUp".to_string()];
        state.update_edit_buffers();

        assert_eq!(state.controls_move_forward_buffer, "W, ArrowUp");
    }

    #[test]
    fn test_config_editor_update_config_from_buffers() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "W, ArrowUp, ArrowLeft".to_string();
        state.update_config_from_buffers();

        assert_eq!(
            state.game_config.controls.move_forward,
            vec![
                "W".to_string(),
                "ArrowUp".to_string(),
                "ArrowLeft".to_string()
            ]
        );
    }

    #[test]
    fn test_config_editor_update_config_from_buffers_with_spaces() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "W  ,  ArrowUp  ,  X".to_string();
        state.update_config_from_buffers();

        assert_eq!(
            state.game_config.controls.move_forward,
            vec!["W".to_string(), "ArrowUp".to_string(), "X".to_string()]
        );
    }

    #[test]
    fn test_config_editor_save_config_no_directory() {
        let mut state = ConfigEditorState::new();
        let result = state.save_config(None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No campaign directory set");
    }

    #[test]
    fn test_config_editor_load_config_no_directory() {
        let mut state = ConfigEditorState::new();
        let success = state.load_config(None);
        assert!(!success);
    }

    #[test]
    fn test_config_editor_graphics_validation() {
        let mut state = ConfigEditorState::new();
        state.game_config.graphics.resolution = (0, 720);
        let result = state.save_config(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_editor_audio_validation() {
        let mut state = ConfigEditorState::new();
        state.game_config.audio.master_volume = 1.5;
        let result = state.save_config(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_editor_controls_validation() {
        let mut state = ConfigEditorState::new();
        state.game_config.controls.movement_cooldown = -1.0;
        let result = state.save_config(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_editor_camera_validation() {
        let mut state = ConfigEditorState::new();
        state.game_config.camera.eye_height = -1.0;
        let result = state.save_config(None);
        assert!(result.is_err());
    }
}
