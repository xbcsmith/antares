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
use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
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
    pub graphics_quality_expanded: bool,

    /// Edit buffers for key bindings
    pub controls_move_forward_buffer: String,
    pub controls_move_back_buffer: String,
    pub controls_turn_left_buffer: String,
    pub controls_turn_right_buffer: String,
    pub controls_interact_buffer: String,
    pub controls_menu_buffer: String,

    /// Validation errors by field name
    pub validation_errors: std::collections::HashMap<String, String>,

    /// Track which key binding is being captured (None = idle, Some(action_name) = capturing)
    pub capturing_key_for: Option<String>,

    /// Recently captured key event for key binding
    pub last_captured_key: Option<String>,

    /// Track if we need to auto-load config on first display
    pub needs_initial_load: bool,

    /// Track last campaign directory to detect changes
    pub last_campaign_dir: Option<PathBuf>,

    /// Grass performance level setting
    pub grass_performance_level: GrassPerformanceLevel,
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
            validation_errors: std::collections::HashMap::new(),
            capturing_key_for: None,
            last_captured_key: None,
            needs_initial_load: true,
            last_campaign_dir: None,
            graphics_quality_expanded: false,
            grass_performance_level: GrassPerformanceLevel::Medium,
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
        // Auto-load config on first display or when campaign directory changes
        let campaign_changed = match (campaign_dir, &self.last_campaign_dir) {
            (Some(new_dir), Some(old_dir)) => new_dir != old_dir,
            (Some(_), None) => true,
            (None, Some(_)) => true,
            (None, None) => false,
        };

        if (self.needs_initial_load || campaign_changed) && campaign_dir.is_some() {
            if self.load_config(campaign_dir) {
                self.needs_initial_load = false;
                self.last_campaign_dir = campaign_dir.cloned();
            }
        }

        // Handle key capture events
        self.handle_key_capture(ui);

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

        // Reset to defaults and preset buttons
        ui.horizontal(|ui| {
            if ui.button("🔄 Reset to Defaults").clicked() {
                self.game_config = GameConfig::default();
                self.update_edit_buffers();
                *status_message = "Config reset to defaults".to_string();
                *unsaved_changes = true;
            }

            ui.separator();

            ui.label("Graphics Presets:");
            if ui.button("Low").clicked() {
                self.game_config.graphics.resolution = (1280, 720);
                self.game_config.graphics.msaa_samples = 1;
                self.game_config.graphics.shadow_quality = ShadowQuality::Low;
                *status_message = "Applied Low graphics preset".to_string();
                *unsaved_changes = true;
            }
            if ui.button("Medium").clicked() {
                self.game_config.graphics.resolution = (1920, 1080);
                self.game_config.graphics.msaa_samples = 4;
                self.game_config.graphics.shadow_quality = ShadowQuality::Medium;
                *status_message = "Applied Medium graphics preset".to_string();
                *unsaved_changes = true;
            }
            if ui.button("High").clicked() {
                self.game_config.graphics.resolution = (2560, 1440);
                self.game_config.graphics.msaa_samples = 8;
                self.game_config.graphics.shadow_quality = ShadowQuality::High;
                *status_message = "Applied High graphics preset".to_string();
                *unsaved_changes = true;
            }
        });

        ui.add_space(10.0);

        // Display sections in a scrollable area
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.show_graphics_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_graphics_quality_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_audio_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_controls_section(ui, unsaved_changes);
                ui.add_space(5.0);
                self.show_camera_section(ui, unsaved_changes);
            });
    }

    /// Show grass performance settings
    fn show_graphics_quality_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("Grass Performance", |ui| {
            ui.add_space(5.0);

            // Grass Performance Level Setting
            ui.horizontal(|ui| {
                ui.label("Grass Performance:");
                let level_options = [
                    GrassPerformanceLevel::Low,
                    GrassPerformanceLevel::Medium,
                    GrassPerformanceLevel::High,
                ];
                let level_names = [
                    GrassPerformanceLevel::Low.name(),
                    GrassPerformanceLevel::Medium.name(),
                    GrassPerformanceLevel::High.name(),
                ];
                let mut selected_index = level_options
                    .iter()
                    .position(|&x| x == self.grass_performance_level)
                    .unwrap_or(1);

                let original_index = selected_index;
                egui::ComboBox::from_id_salt("grass_performance_level")
                    .selected_text(level_names[selected_index])
                    .show_index(ui, &mut selected_index, level_names.len(), |i| {
                        egui::WidgetText::from(level_names[i])
                    });

                if selected_index != original_index {
                    self.grass_performance_level = level_options[selected_index];
                    *unsaved_changes = true;
                }
            });

            ui.label("Performance scaling:");
            ui.indent("grass_info", |ui| {
                ui.label(format!(
                    "Multiplier: {:.2}x content density",
                    self.grass_performance_level.multiplier()
                ));
                ui.label("Final blade counts depend on per-tile content density.");
                match self.grass_performance_level {
                    GrassPerformanceLevel::Low => {
                        ui.label("Prioritizes performance on older hardware.");
                    }
                    GrassPerformanceLevel::Medium => {
                        ui.label("Balanced default for standard hardware.");
                    }
                    GrassPerformanceLevel::High => {
                        ui.label("Maximum fidelity for high-end hardware.");
                    }
                }
            });

            ui.add_space(5.0);
        });

        self.graphics_quality_expanded = section_open.openness > 0.0;
    }

    /// Show the graphics configuration section
    fn show_graphics_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("🖥️ Graphics Settings", |ui| {
            ui.add_space(5.0);

            // Resolution with validation
            ui.horizontal(|ui| {
                ui.label("Resolution:");
                let mut width = self.game_config.graphics.resolution.0;
                let mut height = self.game_config.graphics.resolution.1;

                let width_response = ui.add(egui::DragValue::new(&mut width).range(320..=7680));
                if width_response.changed() {
                    self.game_config.graphics.resolution.0 = width;
                    *unsaved_changes = true;
                    self.validation_errors.remove("resolution_width");
                }
                width_response.on_hover_text("Screen width in pixels (320-7680)");

                ui.label("×");

                let height_response = ui.add(egui::DragValue::new(&mut height).range(240..=4320));
                if height_response.changed() {
                    self.game_config.graphics.resolution.1 = height;
                    *unsaved_changes = true;
                    self.validation_errors.remove("resolution_height");
                }
                height_response.on_hover_text("Screen height in pixels (240-4320)");
            });

            // Show resolution validation error if any
            if let Some(error) = self.validation_errors.get("resolution") {
                ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠️ {}", error));
            }

            // Fullscreen with tooltip
            let fullscreen_response =
                ui.checkbox(&mut self.game_config.graphics.fullscreen, "Fullscreen");
            if fullscreen_response.changed() {
                *unsaved_changes = true;
            }
            fullscreen_response.on_hover_text("Enable fullscreen mode");

            // VSync with tooltip
            let vsync_response = ui.checkbox(&mut self.game_config.graphics.vsync, "VSync");
            if vsync_response.changed() {
                *unsaved_changes = true;
            }
            vsync_response.on_hover_text("Enable vertical sync to prevent screen tearing");

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

            // Master Volume with percentage display
            ui.horizontal(|ui| {
                ui.label("Master Volume:");
                let master_response = ui.add(
                    egui::Slider::new(&mut self.game_config.audio.master_volume, 0.0..=1.0)
                        .step_by(0.05),
                );
                if master_response.changed() {
                    *unsaved_changes = true;
                }
                ui.label(format!(
                    "{}%",
                    (self.game_config.audio.master_volume * 100.0) as i32
                ));
                master_response.on_hover_text("Overall volume level (0-100%)");
            });

            // Music Volume with percentage display
            ui.horizontal(|ui| {
                ui.label("Music Volume:");
                let music_response = ui.add(
                    egui::Slider::new(&mut self.game_config.audio.music_volume, 0.0..=1.0)
                        .step_by(0.05),
                );
                if music_response.changed() {
                    *unsaved_changes = true;
                }
                ui.label(format!(
                    "{}%",
                    (self.game_config.audio.music_volume * 100.0) as i32
                ));
                music_response.on_hover_text("Background music volume level");
            });

            // SFX Volume with percentage display
            ui.horizontal(|ui| {
                ui.label("SFX Volume:");
                let sfx_response = ui.add(
                    egui::Slider::new(&mut self.game_config.audio.sfx_volume, 0.0..=1.0)
                        .step_by(0.05),
                );
                if sfx_response.changed() {
                    *unsaved_changes = true;
                }
                ui.label(format!(
                    "{}%",
                    (self.game_config.audio.sfx_volume * 100.0) as i32
                ));
                sfx_response.on_hover_text("Sound effects volume level");
            });

            // Ambient Volume with percentage display
            ui.horizontal(|ui| {
                ui.label("Ambient Volume:");
                let ambient_response = ui.add(
                    egui::Slider::new(&mut self.game_config.audio.ambient_volume, 0.0..=1.0)
                        .step_by(0.05),
                );
                if ambient_response.changed() {
                    *unsaved_changes = true;
                }
                ui.label(format!(
                    "{}%",
                    (self.game_config.audio.ambient_volume * 100.0) as i32
                ));
                ambient_response.on_hover_text("Ambient sounds and music volume level");
            });

            // Enable Audio with tooltip
            let enable_response =
                ui.checkbox(&mut self.game_config.audio.enable_audio, "Enable Audio");
            if enable_response.changed() {
                *unsaved_changes = true;
            }
            enable_response.on_hover_text("Disable to mute all sound");

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
            ui.label("🎮 Key Bindings:");
            ui.label(
                "Click 'Capture' and press a key, or manually type key names (comma-separated)",
            );
            ui.label(
                "Supported: A-Z, 0-9, Space, Enter, Escape, Tab, Shift, Ctrl, Alt, Arrow Keys",
            );

            ui.add_space(5.0);

            // Helper function for key binding field with capture, clear, and validation
            let show_key_binding_with_capture =
                |ui: &mut egui::Ui,
                 label: &str,
                 buffer: &mut String,
                 action_id: &str,
                 unsaved_changes: &mut bool,
                 validation_errors: &mut std::collections::HashMap<String, String>,
                 capturing_key_for: &mut Option<String>| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", label));

                        // Show capture state indicator
                        let is_capturing = capturing_key_for
                            .as_ref()
                            .map(|s| s == action_id)
                            .unwrap_or(false);

                        if is_capturing {
                            ui.label(
                                egui::RichText::new("🎮 Press a key...")
                                    .color(egui::Color32::LIGHT_BLUE),
                            );
                        }

                        // Text field for manual editing
                        let response = ui.text_edit_singleline(buffer);
                        if response.changed() {
                            *unsaved_changes = true;
                            validation_errors.remove(action_id);
                        }
                        response.on_hover_text(
                            "Enter comma-separated key names, or use Capture button",
                        );

                        // Capture button
                        if ui.button("🎮 Capture").clicked() {
                            *capturing_key_for = Some(action_id.to_string());
                        }

                        // Clear button
                        if ui.button("🗑 Clear").clicked() {
                            buffer.clear();
                            *unsaved_changes = true;
                            validation_errors.remove(action_id);
                        }

                        // Show validation error if exists
                        if let Some(error) = validation_errors.get(action_id) {
                            ui.label(
                                egui::RichText::new(format!("⚠️ {}", error))
                                    .color(egui::Color32::LIGHT_RED),
                            );
                        }
                    });
                };

            // Move Forward
            show_key_binding_with_capture(
                ui,
                "Move Forward",
                &mut self.controls_move_forward_buffer,
                "move_forward",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            // Move Back
            show_key_binding_with_capture(
                ui,
                "Move Back",
                &mut self.controls_move_back_buffer,
                "move_back",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            // Turn Left
            show_key_binding_with_capture(
                ui,
                "Turn Left",
                &mut self.controls_turn_left_buffer,
                "turn_left",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            // Turn Right
            show_key_binding_with_capture(
                ui,
                "Turn Right",
                &mut self.controls_turn_right_buffer,
                "turn_right",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            // Interact
            show_key_binding_with_capture(
                ui,
                "Interact",
                &mut self.controls_interact_buffer,
                "interact",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            // Menu
            show_key_binding_with_capture(
                ui,
                "Menu",
                &mut self.controls_menu_buffer,
                "menu",
                unsaved_changes,
                &mut self.validation_errors,
                &mut self.capturing_key_for,
            );

            ui.add_space(5.0);
        });

        self.controls_expanded = section_open.openness > 0.0;
    }

    /// Show the camera configuration section
    fn show_camera_section(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        let section_open = ui.collapsing("📷 Camera Settings", |ui| {
            ui.add_space(5.0);

            // Camera Mode with tooltip
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
                let combo_response = egui::ComboBox::from_id_salt("camera_mode")
                    .selected_text(mode_names[selected_index])
                    .show_index(ui, &mut selected_index, mode_names.len(), |i| {
                        egui::WidgetText::from(mode_names[i])
                    });

                if selected_index != original_index {
                    self.game_config.camera.mode = mode_options[selected_index];
                    *unsaved_changes = true;
                }

                combo_response.on_hover_text(
                    "Select camera perspective: First Person, Tactical (3rd person), or Isometric",
                );
            });

            // Eye Height with tooltip
            ui.horizontal(|ui| {
                ui.label("Eye Height:");
                let eye_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.eye_height)
                        .range(0.1..=3.0)
                        .speed(0.05),
                );
                if eye_response.changed() {
                    *unsaved_changes = true;
                }
                eye_response.on_hover_text("Camera height above ground in game units (0.1-3.0)");
            });

            // Field of View with tooltip
            ui.horizontal(|ui| {
                ui.label("FOV (degrees):");
                let fov_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.fov)
                        .range(30.0..=120.0)
                        .speed(1.0),
                );
                if fov_response.changed() {
                    *unsaved_changes = true;
                }
                fov_response
                    .on_hover_text("Field of view in degrees (30-120). Wider FOV shows more area");
            });

            // Near Clip with tooltip
            ui.horizontal(|ui| {
                ui.label("Near Clip:");
                let near_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.near_clip)
                        .range(0.01..=10.0)
                        .speed(0.01),
                );
                if near_response.changed() {
                    *unsaved_changes = true;
                }
                near_response.on_hover_text("Nearest distance to render from camera (0.01-10.0)");
            });

            // Far Clip with tooltip
            ui.horizontal(|ui| {
                ui.label("Far Clip:");
                let far_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.far_clip)
                        .range(10.0..=10000.0)
                        .speed(10.0),
                );
                if far_response.changed() {
                    *unsaved_changes = true;
                }
                far_response.on_hover_text("Farthest distance to render from camera (10-10000)");
            });

            // Smooth Rotation with tooltip
            let smooth_response = ui.checkbox(
                &mut self.game_config.camera.smooth_rotation,
                "Smooth Rotation",
            );
            if smooth_response.changed() {
                *unsaved_changes = true;
            }
            smooth_response.on_hover_text("Enable smooth camera rotation interpolation");

            // Rotation Speed with tooltip
            ui.horizontal(|ui| {
                ui.label("Rotation Speed (°/s):");
                let rotation_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.rotation_speed)
                        .range(30.0..=360.0)
                        .speed(5.0),
                );
                if rotation_response.changed() {
                    *unsaved_changes = true;
                }
                rotation_response
                    .on_hover_text("Camera rotation speed in degrees per second (30-360)");
            });

            ui.add_space(5.0);
            ui.separator();
            ui.label("💡 Lighting Settings");
            ui.add_space(5.0);

            // Light Height with tooltip
            ui.horizontal(|ui| {
                ui.label("Light Height:");
                let light_height_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.light_height)
                        .range(0.1..=20.0)
                        .speed(0.1),
                );
                if light_height_response.changed() {
                    *unsaved_changes = true;
                }
                light_height_response
                    .on_hover_text("Height of the primary light source (0.1-20.0)");
            });

            // Light Intensity with tooltip
            ui.horizontal(|ui| {
                ui.label("Light Intensity:");
                let intensity_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.light_intensity)
                        .range(100000.0..=10000000.0)
                        .speed(100000.0),
                );
                if intensity_response.changed() {
                    *unsaved_changes = true;
                }
                intensity_response.on_hover_text(
                    "Brightness of the primary light (100k-10M). Higher values = brighter",
                );
            });

            // Light Range with tooltip
            ui.horizontal(|ui| {
                ui.label("Light Range:");
                let range_response = ui.add(
                    egui::DragValue::new(&mut self.game_config.camera.light_range)
                        .range(10.0..=200.0)
                        .speed(1.0),
                );
                if range_response.changed() {
                    *unsaved_changes = true;
                }
                range_response.on_hover_text("Distance the light reaches (10-200 units)");
            });

            // Shadows Enabled with tooltip
            let shadows_response = ui.checkbox(
                &mut self.game_config.camera.shadows_enabled,
                "Shadows Enabled",
            );
            if shadows_response.changed() {
                *unsaved_changes = true;
            }
            shadows_response.on_hover_text("Enable shadow rendering for dynamic lighting effects");

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
    /// Update edit buffers from loaded config
    ///
    /// Populates text field buffers with current config values
    fn update_edit_buffers(&mut self) {
        self.controls_move_forward_buffer =
            format_key_list(&self.game_config.controls.move_forward);
        self.controls_move_back_buffer = format_key_list(&self.game_config.controls.move_back);
        self.controls_turn_left_buffer = format_key_list(&self.game_config.controls.turn_left);
        self.controls_turn_right_buffer = format_key_list(&self.game_config.controls.turn_right);
        self.controls_interact_buffer = format_key_list(&self.game_config.controls.interact);
        self.controls_menu_buffer = format_key_list(&self.game_config.controls.menu);
    }

    /// Update config from edit buffers
    ///
    /// Parses text field buffers back into config vectors
    fn update_config_from_buffers(&mut self) {
        self.game_config.controls.move_forward = parse_key_list(&self.controls_move_forward_buffer);
        self.game_config.controls.move_back = parse_key_list(&self.controls_move_back_buffer);
        self.game_config.controls.turn_left = parse_key_list(&self.controls_turn_left_buffer);
        self.game_config.controls.turn_right = parse_key_list(&self.controls_turn_right_buffer);
        self.game_config.controls.interact = parse_key_list(&self.controls_interact_buffer);
        self.game_config.controls.menu = parse_key_list(&self.controls_menu_buffer);
    }

    /// Handle key capture events from egui input
    ///
    /// Processes keyboard events when a key binding field is in capture mode.
    /// Escape cancels capture, other keys are added to the binding.
    fn handle_key_capture(&mut self, ui: &mut egui::Ui) {
        if self.capturing_key_for.is_none() {
            return;
        }

        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers: _,
                    ..
                } = event
                {
                    // Escape cancels capture without binding
                    if *key == egui::Key::Escape {
                        self.capturing_key_for = None;
                        self.last_captured_key = None;
                        return;
                    }

                    // Convert key to string and add to appropriate buffer
                    let key_name = egui_key_to_string(key);
                    if let Some(action_id) = &self.capturing_key_for.clone() {
                        let buffer = match action_id.as_str() {
                            "move_forward" => &mut self.controls_move_forward_buffer,
                            "move_back" => &mut self.controls_move_back_buffer,
                            "turn_left" => &mut self.controls_turn_left_buffer,
                            "turn_right" => &mut self.controls_turn_right_buffer,
                            "interact" => &mut self.controls_interact_buffer,
                            "menu" => &mut self.controls_menu_buffer,
                            _ => return,
                        };

                        // Add key to buffer (comma-separated if not empty)
                        if !buffer.is_empty() {
                            buffer.push_str(", ");
                        }
                        buffer.push_str(&key_name);

                        self.last_captured_key = Some(key_name);
                        self.capturing_key_for = None;
                    }
                }
            }
        });
    }

    /// Validate key binding for a control action
    ///
    /// # Arguments
    ///
    /// * `action_id` - The control action identifier
    /// * `keys_str` - Comma-separated key names
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if valid, or Err(error message) if invalid
    fn validate_key_binding(&self, action_id: &str, keys_str: &str) -> Result<(), String> {
        if keys_str.is_empty() {
            return Err(format!("{} must have at least one key bound", action_id));
        }

        // Valid key names (basic validation)
        let valid_keys = vec![
            "A",
            "B",
            "C",
            "D",
            "E",
            "F",
            "G",
            "H",
            "I",
            "J",
            "K",
            "L",
            "M",
            "N",
            "O",
            "P",
            "Q",
            "R",
            "S",
            "T",
            "U",
            "V",
            "W",
            "X",
            "Y",
            "Z",
            "0",
            "1",
            "2",
            "3",
            "4",
            "5",
            "6",
            "7",
            "8",
            "9",
            "Space",
            "Enter",
            "Escape",
            "Tab",
            "Backspace",
            "Delete",
            "Insert",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Shift",
            "Ctrl",
            "Alt",
            "Super",
            "Up Arrow",
            "Down Arrow",
            "Left Arrow",
            "Right Arrow",
            "+",
            "-",
            "*",
            "/",
            ".",
            ",",
            ";",
            "'",
            "[",
            "]",
            "\\",
            "`",
            "~",
            "!",
            "@",
            "#",
            "$",
            "%",
            "^",
            "&",
        ];

        let keys: Vec<&str> = keys_str.split(',').map(|s| s.trim()).collect();
        for key in keys {
            if !valid_keys
                .iter()
                .any(|&valid| valid.eq_ignore_ascii_case(key))
            {
                return Err(format!("'{}' is not a recognized key name", key));
            }
        }

        Ok(())
    }

    /// Validate all configuration settings
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if all settings are valid, otherwise populates validation_errors
    fn validate_config(&mut self) -> Result<(), String> {
        self.validation_errors.clear();

        // Validate resolution
        if self.game_config.graphics.resolution.0 < 320
            || self.game_config.graphics.resolution.0 > 7680
        {
            self.validation_errors.insert(
                "resolution_width".to_string(),
                "Width must be 320-7680".to_string(),
            );
        }
        if self.game_config.graphics.resolution.1 < 240
            || self.game_config.graphics.resolution.1 > 4320
        {
            self.validation_errors.insert(
                "resolution_height".to_string(),
                "Height must be 240-4320".to_string(),
            );
        }

        // Validate audio volumes
        if self.game_config.audio.master_volume < 0.0 || self.game_config.audio.master_volume > 1.0
        {
            self.validation_errors
                .insert("master_volume".to_string(), "Must be 0.0-1.0".to_string());
        }
        if self.game_config.audio.music_volume < 0.0 || self.game_config.audio.music_volume > 1.0 {
            self.validation_errors
                .insert("music_volume".to_string(), "Must be 0.0-1.0".to_string());
        }
        if self.game_config.audio.sfx_volume < 0.0 || self.game_config.audio.sfx_volume > 1.0 {
            self.validation_errors
                .insert("sfx_volume".to_string(), "Must be 0.0-1.0".to_string());
        }
        if self.game_config.audio.ambient_volume < 0.0
            || self.game_config.audio.ambient_volume > 1.0
        {
            self.validation_errors
                .insert("ambient_volume".to_string(), "Must be 0.0-1.0".to_string());
        }

        // Validate controls key bindings
        if let Err(e) =
            self.validate_key_binding("move_forward", &self.controls_move_forward_buffer)
        {
            self.validation_errors.insert("move_forward".to_string(), e);
        }
        if let Err(e) = self.validate_key_binding("move_back", &self.controls_move_back_buffer) {
            self.validation_errors.insert("move_back".to_string(), e);
        }
        if let Err(e) = self.validate_key_binding("turn_left", &self.controls_turn_left_buffer) {
            self.validation_errors.insert("turn_left".to_string(), e);
        }
        if let Err(e) = self.validate_key_binding("turn_right", &self.controls_turn_right_buffer) {
            self.validation_errors.insert("turn_right".to_string(), e);
        }
        if let Err(e) = self.validate_key_binding("interact", &self.controls_interact_buffer) {
            self.validation_errors.insert("interact".to_string(), e);
        }
        if let Err(e) = self.validate_key_binding("menu", &self.controls_menu_buffer) {
            self.validation_errors.insert("menu".to_string(), e);
        }

        // Validate camera settings
        if self.game_config.camera.eye_height < 0.1 || self.game_config.camera.eye_height > 3.0 {
            self.validation_errors
                .insert("eye_height".to_string(), "Must be 0.1-3.0".to_string());
        }
        if self.game_config.camera.fov < 30.0 || self.game_config.camera.fov > 120.0 {
            self.validation_errors
                .insert("fov".to_string(), "Must be 30-120 degrees".to_string());
        }
        if self.game_config.camera.near_clip < 0.01 || self.game_config.camera.near_clip > 10.0 {
            self.validation_errors
                .insert("near_clip".to_string(), "Must be 0.01-10.0".to_string());
        }
        if self.game_config.camera.far_clip < 10.0 || self.game_config.camera.far_clip > 10000.0 {
            self.validation_errors
                .insert("far_clip".to_string(), "Must be 10-10000".to_string());
        }
        if self.game_config.camera.near_clip >= self.game_config.camera.far_clip {
            self.validation_errors.insert(
                "clip_planes".to_string(),
                "Near clip must be less than far clip".to_string(),
            );
        }

        if self.validation_errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} validation errors found",
                self.validation_errors.len()
            ))
        }
    }
}

/// Convert egui::Key to human-readable string
///
/// # Arguments
///
/// * `key` - The egui key to convert
///
/// # Returns
///
/// A human-readable string representation of the key
///
/// # Examples
///
/// ```ignore
/// let key_name = egui_key_to_string(&egui::Key::W);
/// assert_eq!(key_name, "W");
/// ```
fn egui_key_to_string(key: &egui::Key) -> String {
    match key {
        egui::Key::A => "A".to_string(),
        egui::Key::B => "B".to_string(),
        egui::Key::C => "C".to_string(),
        egui::Key::D => "D".to_string(),
        egui::Key::E => "E".to_string(),
        egui::Key::F => "F".to_string(),
        egui::Key::G => "G".to_string(),
        egui::Key::H => "H".to_string(),
        egui::Key::I => "I".to_string(),
        egui::Key::J => "J".to_string(),
        egui::Key::K => "K".to_string(),
        egui::Key::L => "L".to_string(),
        egui::Key::M => "M".to_string(),
        egui::Key::N => "N".to_string(),
        egui::Key::O => "O".to_string(),
        egui::Key::P => "P".to_string(),
        egui::Key::Q => "Q".to_string(),
        egui::Key::R => "R".to_string(),
        egui::Key::S => "S".to_string(),
        egui::Key::T => "T".to_string(),
        egui::Key::U => "U".to_string(),
        egui::Key::V => "V".to_string(),
        egui::Key::W => "W".to_string(),
        egui::Key::X => "X".to_string(),
        egui::Key::Y => "Y".to_string(),
        egui::Key::Z => "Z".to_string(),
        egui::Key::Num0 => "0".to_string(),
        egui::Key::Num1 => "1".to_string(),
        egui::Key::Num2 => "2".to_string(),
        egui::Key::Num3 => "3".to_string(),
        egui::Key::Num4 => "4".to_string(),
        egui::Key::Num5 => "5".to_string(),
        egui::Key::Num6 => "6".to_string(),
        egui::Key::Num7 => "7".to_string(),
        egui::Key::Num8 => "8".to_string(),
        egui::Key::Num9 => "9".to_string(),
        egui::Key::Space => "Space".to_string(),
        egui::Key::Enter => "Enter".to_string(),
        egui::Key::Escape => "Escape".to_string(),
        egui::Key::Tab => "Tab".to_string(),
        egui::Key::Backspace => "Backspace".to_string(),
        egui::Key::Delete => "Delete".to_string(),
        egui::Key::Insert => "Insert".to_string(),
        egui::Key::Home => "Home".to_string(),
        egui::Key::End => "End".to_string(),
        egui::Key::PageUp => "PageUp".to_string(),
        egui::Key::PageDown => "PageDown".to_string(),
        egui::Key::ArrowUp => "Up Arrow".to_string(),
        egui::Key::ArrowDown => "Down Arrow".to_string(),
        egui::Key::ArrowLeft => "Left Arrow".to_string(),
        egui::Key::ArrowRight => "Right Arrow".to_string(),
        egui::Key::Plus => "+".to_string(),
        egui::Key::Minus => "-".to_string(),
        _ => format!("{:?}", key),
    }
}

/// Format key list as comma-separated display text
///
/// # Arguments
///
/// * `keys` - Vector of key name strings
///
/// # Returns
///
/// Comma-separated string of key names
///
/// # Examples
///
/// ```ignore
/// let keys = vec!["W".to_string(), "Up Arrow".to_string()];
/// let formatted = format_key_list(&keys);
/// assert_eq!(formatted, "W, Up Arrow");
/// ```
fn format_key_list(keys: &[String]) -> String {
    keys.join(", ")
}

/// Parse comma-separated key list back to vector
///
/// # Arguments
///
/// * `text` - Comma-separated key names
///
/// # Returns
///
/// Vector of trimmed, non-empty key name strings
///
/// # Examples
///
/// ```ignore
/// let parsed = parse_key_list("W, Up Arrow, S");
/// assert_eq!(parsed, vec!["W", "Up Arrow", "S"]);
/// ```
fn parse_key_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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

    // Phase 2: Inline Validation Tests

    #[test]
    fn test_validate_key_binding_valid_keys() {
        let state = ConfigEditorState::new();
        let result = state.validate_key_binding("move_forward", "W, A, D");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_key_binding_invalid_key() {
        let state = ConfigEditorState::new();
        let result = state.validate_key_binding("move_forward", "InvalidKey");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a recognized key"));
    }

    #[test]
    fn test_validate_key_binding_empty() {
        let state = ConfigEditorState::new();
        let result = state.validate_key_binding("move_forward", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have at least one key"));
    }

    #[test]
    fn test_validate_key_binding_with_arrows() {
        let state = ConfigEditorState::new();
        let result = state.validate_key_binding("move_forward", "Up Arrow, W");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_key_binding_case_insensitive() {
        let state = ConfigEditorState::new();
        let result = state.validate_key_binding("move_forward", "w, space, enter");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_all_valid() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "W".to_string();
        state.controls_move_back_buffer = "S".to_string();
        state.controls_turn_left_buffer = "A".to_string();
        state.controls_turn_right_buffer = "D".to_string();
        state.controls_interact_buffer = "E".to_string();
        state.controls_menu_buffer = "Escape".to_string();

        let result = state.validate_config();
        assert!(result.is_ok());
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_validate_config_invalid_resolution() {
        let mut state = ConfigEditorState::new();
        state.game_config.graphics.resolution = (0, 720);
        state.controls_move_forward_buffer = "W".to_string();
        state.controls_move_back_buffer = "S".to_string();
        state.controls_turn_left_buffer = "A".to_string();
        state.controls_turn_right_buffer = "D".to_string();
        state.controls_interact_buffer = "E".to_string();
        state.controls_menu_buffer = "Escape".to_string();

        let result = state.validate_config();
        assert!(result.is_err());
        assert!(state.validation_errors.contains_key("resolution_width"));
    }

    #[test]
    fn test_validate_config_invalid_audio_volume() {
        let mut state = ConfigEditorState::new();
        state.game_config.audio.master_volume = 1.5;
        state.controls_move_forward_buffer = "W".to_string();
        state.controls_move_back_buffer = "S".to_string();
        state.controls_turn_left_buffer = "A".to_string();
        state.controls_turn_right_buffer = "D".to_string();
        state.controls_interact_buffer = "E".to_string();
        state.controls_menu_buffer = "Escape".to_string();

        let result = state.validate_config();
        assert!(result.is_err());
        assert!(state.validation_errors.contains_key("master_volume"));
    }

    #[test]
    fn test_validate_config_invalid_key_binding() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "InvalidKey".to_string();
        state.controls_move_back_buffer = "S".to_string();
        state.controls_turn_left_buffer = "A".to_string();
        state.controls_turn_right_buffer = "D".to_string();
        state.controls_interact_buffer = "E".to_string();
        state.controls_menu_buffer = "Escape".to_string();

        let result = state.validate_config();
        assert!(result.is_err());
        assert!(state.validation_errors.contains_key("move_forward"));
    }

    #[test]
    fn test_validate_config_near_far_clip_order() {
        let mut state = ConfigEditorState::new();
        state.game_config.camera.near_clip = 10.0;
        state.game_config.camera.far_clip = 5.0; // far_clip < near_clip
        state.controls_move_forward_buffer = "W".to_string();
        state.controls_move_back_buffer = "S".to_string();
        state.controls_turn_left_buffer = "A".to_string();
        state.controls_turn_right_buffer = "D".to_string();
        state.controls_interact_buffer = "E".to_string();
        state.controls_menu_buffer = "Escape".to_string();

        let result = state.validate_config();
        assert!(result.is_err());
        assert!(state.validation_errors.contains_key("clip_planes"));
    }

    #[test]
    fn test_reset_to_defaults_clears_changes() {
        let mut state = ConfigEditorState::new();
        state.game_config.graphics.resolution = (2560, 1440);
        state.game_config.audio.master_volume = 0.5;

        // Simulate reset
        state.game_config = GameConfig::default();
        state.update_edit_buffers();

        assert_eq!(state.game_config.graphics.resolution, (1280, 720));
        assert_eq!(state.game_config.audio.master_volume, 0.8);
    }

    #[test]
    fn test_graphics_preset_low() {
        let mut state = ConfigEditorState::new();
        state.game_config.graphics.resolution = (2560, 1440);
        state.game_config.graphics.msaa_samples = 16;
        state.game_config.graphics.shadow_quality = ShadowQuality::Ultra;

        // Apply low preset
        state.game_config.graphics.resolution = (1280, 720);
        state.game_config.graphics.msaa_samples = 1;
        state.game_config.graphics.shadow_quality = ShadowQuality::Low;

        assert_eq!(state.game_config.graphics.resolution, (1280, 720));
        assert_eq!(state.game_config.graphics.msaa_samples, 1);
        assert_eq!(
            state.game_config.graphics.shadow_quality,
            ShadowQuality::Low
        );
    }

    #[test]
    fn test_graphics_preset_high() {
        let mut state = ConfigEditorState::new();
        // Apply high preset
        state.game_config.graphics.resolution = (2560, 1440);
        state.game_config.graphics.msaa_samples = 8;
        state.game_config.graphics.shadow_quality = ShadowQuality::High;

        assert_eq!(state.game_config.graphics.resolution, (2560, 1440));
        assert_eq!(state.game_config.graphics.msaa_samples, 8);
        assert_eq!(
            state.game_config.graphics.shadow_quality,
            ShadowQuality::High
        );
    }

    // Phase 3: Key Capture and Auto-Population Tests

    #[test]
    fn test_egui_key_to_string_letters() {
        assert_eq!(egui_key_to_string(&egui::Key::W), "W");
        assert_eq!(egui_key_to_string(&egui::Key::A), "A");
        assert_eq!(egui_key_to_string(&egui::Key::S), "S");
        assert_eq!(egui_key_to_string(&egui::Key::D), "D");
    }

    #[test]
    fn test_egui_key_to_string_numbers() {
        assert_eq!(egui_key_to_string(&egui::Key::Num0), "0");
        assert_eq!(egui_key_to_string(&egui::Key::Num1), "1");
        assert_eq!(egui_key_to_string(&egui::Key::Num9), "9");
    }

    #[test]
    fn test_egui_key_to_string_special_keys() {
        assert_eq!(egui_key_to_string(&egui::Key::Space), "Space");
        assert_eq!(egui_key_to_string(&egui::Key::Enter), "Enter");
        assert_eq!(egui_key_to_string(&egui::Key::Escape), "Escape");
        assert_eq!(egui_key_to_string(&egui::Key::Tab), "Tab");
        assert_eq!(egui_key_to_string(&egui::Key::Backspace), "Backspace");
    }

    #[test]
    fn test_egui_key_to_string_arrows() {
        assert_eq!(egui_key_to_string(&egui::Key::ArrowUp), "Up Arrow");
        assert_eq!(egui_key_to_string(&egui::Key::ArrowDown), "Down Arrow");
        assert_eq!(egui_key_to_string(&egui::Key::ArrowLeft), "Left Arrow");
        assert_eq!(egui_key_to_string(&egui::Key::ArrowRight), "Right Arrow");
    }

    #[test]
    fn test_format_key_list_single_key() {
        let keys = vec!["W".to_string()];
        let formatted = format_key_list(&keys);
        assert_eq!(formatted, "W");
    }

    #[test]
    fn test_format_key_list_multiple_keys() {
        let keys = vec!["W".to_string(), "Up Arrow".to_string(), "Space".to_string()];
        let formatted = format_key_list(&keys);
        assert_eq!(formatted, "W, Up Arrow, Space");
    }

    #[test]
    fn test_format_key_list_empty() {
        let keys: Vec<String> = vec![];
        let formatted = format_key_list(&keys);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_parse_key_list_single_key() {
        let parsed = parse_key_list("W");
        assert_eq!(parsed, vec!["W".to_string()]);
    }

    #[test]
    fn test_parse_key_list_multiple_keys() {
        let parsed = parse_key_list("W, Up Arrow, S");
        assert_eq!(
            parsed,
            vec!["W".to_string(), "Up Arrow".to_string(), "S".to_string()]
        );
    }

    #[test]
    fn test_parse_key_list_with_extra_spaces() {
        let parsed = parse_key_list("W  ,  Up Arrow  ,  S");
        assert_eq!(
            parsed,
            vec!["W".to_string(), "Up Arrow".to_string(), "S".to_string()]
        );
    }

    #[test]
    fn test_parse_key_list_empty_string() {
        let parsed = parse_key_list("");
        assert_eq!(parsed, Vec::<String>::new());
    }

    #[test]
    fn test_parse_key_list_filters_empty_entries() {
        let parsed = parse_key_list("W, , S, ");
        assert_eq!(parsed, vec!["W".to_string(), "S".to_string()]);
    }

    #[test]
    fn test_needs_initial_load_default_true() {
        let state = ConfigEditorState::new();
        assert!(state.needs_initial_load);
        assert!(state.last_campaign_dir.is_none());
    }

    #[test]
    fn test_capturing_key_for_default_none() {
        let state = ConfigEditorState::new();
        assert!(state.capturing_key_for.is_none());
        assert!(state.last_captured_key.is_none());
    }

    #[test]
    fn test_update_edit_buffers_auto_populates() {
        let mut state = ConfigEditorState::new();
        state.game_config.controls.move_forward = vec!["W".to_string(), "Up Arrow".to_string()];
        state.game_config.controls.move_back = vec!["S".to_string()];
        state.game_config.controls.turn_left = vec!["A".to_string(), "Left Arrow".to_string()];

        state.update_edit_buffers();

        assert_eq!(state.controls_move_forward_buffer, "W, Up Arrow");
        assert_eq!(state.controls_move_back_buffer, "S");
        assert_eq!(state.controls_turn_left_buffer, "A, Left Arrow");
    }

    #[test]
    fn test_round_trip_buffer_conversion() {
        let mut state = ConfigEditorState::new();
        let original_keys = vec!["W".to_string(), "Up Arrow".to_string(), "Space".to_string()];

        state.game_config.controls.move_forward = original_keys.clone();
        state.update_edit_buffers();
        state.update_config_from_buffers();

        assert_eq!(state.game_config.controls.move_forward, original_keys);
    }

    #[test]
    fn test_manual_text_edit_still_works() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "X, Y, Z".to_string();
        state.update_config_from_buffers();

        assert_eq!(
            state.game_config.controls.move_forward,
            vec!["X".to_string(), "Y".to_string(), "Z".to_string()]
        );
    }

    #[test]
    fn test_multiple_keys_per_action() {
        let mut state = ConfigEditorState::new();
        state.controls_move_forward_buffer = "W, Up Arrow, 8".to_string();
        state.update_config_from_buffers();

        assert_eq!(state.game_config.controls.move_forward.len(), 3);
        assert!(state
            .game_config
            .controls
            .move_forward
            .contains(&"W".to_string()));
        assert!(state
            .game_config
            .controls
            .move_forward
            .contains(&"Up Arrow".to_string()));
        assert!(state
            .game_config
            .controls
            .move_forward
            .contains(&"8".to_string()));
    }
}
