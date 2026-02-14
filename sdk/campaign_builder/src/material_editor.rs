// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Material and texture editor UI for the campaign builder
//!
//! This module provides UI components for editing material properties and
//! selecting textures for creature meshes. Materials define the visual
//! appearance of meshes (color, metallic, roughness, etc.).
//!
//! # Architecture
//!
//! The material editor integrates with the creatures editor to:
//! - Edit material properties (base color, metallic, roughness, emissive)
//! - Configure alpha/transparency modes
//! - Browse and select textures from the campaign assets
//! - Preview material appearance
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::material_editor::MaterialEditorState;
//! use antares::domain::visual::{MaterialDefinition, AlphaMode};
//!
//! let mut state = MaterialEditorState::new();
//! let mut material = MaterialDefinition::default();
//!
//! // In your egui UI context:
//! // state.show(ui, &mut material);
//! ```

use antares::domain::visual::{AlphaMode, MaterialDefinition};
use eframe::egui;
use std::path::PathBuf;

/// State for the material editor UI
#[derive(Debug, Clone)]
pub struct MaterialEditorState {
    /// Show texture picker dialog
    pub show_texture_picker: bool,

    /// Currently selected texture category
    pub texture_category: TextureCategory,

    /// Search query for textures
    pub texture_search: String,

    /// Available texture paths
    pub available_textures: Vec<PathBuf>,

    /// Show material preview
    pub show_preview: bool,

    /// Preview background color
    pub preview_background: [f32; 3],
}

/// State for the texture picker UI
#[derive(Debug, Clone)]
pub struct TexturePickerState {
    /// Currently selected texture index
    pub selected_texture: Option<usize>,

    /// Search query
    pub search_query: String,

    /// Category filter
    pub category_filter: TextureCategory,

    /// View mode
    pub view_mode: TextureViewMode,

    /// Grid item size
    pub grid_item_size: f32,
}

/// Texture categories for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureCategory {
    All,
    Diffuse,
    Normal,
    Metallic,
    Roughness,
    Emissive,
    Custom,
}

impl TextureCategory {
    /// Returns all categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::All,
            Self::Diffuse,
            Self::Normal,
            Self::Metallic,
            Self::Roughness,
            Self::Emissive,
            Self::Custom,
        ]
    }

    /// Returns the display name
    pub fn name(&self) -> &str {
        match self {
            Self::All => "All",
            Self::Diffuse => "Diffuse/Color",
            Self::Normal => "Normal Map",
            Self::Metallic => "Metallic",
            Self::Roughness => "Roughness",
            Self::Emissive => "Emissive",
            Self::Custom => "Custom",
        }
    }
}

/// View mode for texture picker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureViewMode {
    Grid,
    List,
}

impl Default for MaterialEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialEditorState {
    /// Creates a new material editor state
    pub fn new() -> Self {
        Self {
            show_texture_picker: false,
            texture_category: TextureCategory::Diffuse,
            texture_search: String::new(),
            available_textures: vec![],
            show_preview: true,
            preview_background: [0.2, 0.2, 0.2],
        }
    }

    /// Shows the material editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `material` - The material to edit
    ///
    /// # Returns
    ///
    /// Returns `Some(MaterialAction)` if an action should be performed
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        material: &mut MaterialDefinition,
    ) -> Option<MaterialAction> {
        let mut action = None;

        ui.heading("Material Properties");
        ui.separator();

        // Base Color
        ui.horizontal(|ui| {
            ui.label("Base Color:");
            let mut rgb = [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ];
            if ui.color_edit_button_rgb(&mut rgb).changed() {
                material.base_color[0] = rgb[0];
                material.base_color[1] = rgb[1];
                material.base_color[2] = rgb[2];
                action = Some(MaterialAction::Modified);
            }
        });

        ui.separator();

        // Metallic
        ui.horizontal(|ui| {
            ui.label("Metallic:");
            if ui
                .add(
                    egui::Slider::new(&mut material.metallic, 0.0..=1.0)
                        .step_by(0.01)
                        .show_value(true),
                )
                .changed()
            {
                action = Some(MaterialAction::Modified);
            }
        });

        // Roughness
        ui.horizontal(|ui| {
            ui.label("Roughness:");
            if ui
                .add(
                    egui::Slider::new(&mut material.roughness, 0.0..=1.0)
                        .step_by(0.01)
                        .show_value(true),
                )
                .changed()
            {
                action = Some(MaterialAction::Modified);
            }
        });

        ui.separator();

        // Emissive
        ui.horizontal(|ui| {
            ui.label("Emissive:");
            let mut emissive = material.emissive.unwrap_or([0.0, 0.0, 0.0]);
            if ui.color_edit_button_rgb(&mut emissive).changed() {
                material.emissive = Some(emissive);
                action = Some(MaterialAction::Modified);
            }
            if ui.button("Clear").clicked() {
                material.emissive = None;
                action = Some(MaterialAction::Modified);
            }
        });

        ui.separator();

        // Alpha Mode
        ui.horizontal(|ui| {
            ui.label("Alpha Mode:");
            let mut alpha_mode_changed = false;

            egui::ComboBox::from_id_salt("alpha_mode")
                .selected_text(alpha_mode_name(&material.alpha_mode))
                .show_ui(ui, |ui| {
                    for mode in &[AlphaMode::Opaque, AlphaMode::Mask, AlphaMode::Blend] {
                        if ui
                            .selectable_value(
                                &mut material.alpha_mode,
                                *mode,
                                alpha_mode_name(mode),
                            )
                            .clicked()
                        {
                            alpha_mode_changed = true;
                        }
                    }
                });

            if alpha_mode_changed {
                action = Some(MaterialAction::Modified);
            }
        });

        ui.separator();

        // Material presets
        ui.horizontal(|ui| {
            ui.label("Presets:");

            if ui.button("Metal").clicked() {
                material.base_color = [0.8, 0.8, 0.8, 1.0];
                material.metallic = 1.0;
                material.roughness = 0.2;
                action = Some(MaterialAction::Modified);
            }

            if ui.button("Plastic").clicked() {
                material.base_color = [1.0, 1.0, 1.0, 1.0];
                material.metallic = 0.0;
                material.roughness = 0.5;
                action = Some(MaterialAction::Modified);
            }

            if ui.button("Stone").clicked() {
                material.base_color = [0.5, 0.5, 0.5, 1.0];
                material.metallic = 0.0;
                material.roughness = 0.8;
                action = Some(MaterialAction::Modified);
            }

            if ui.button("Gem").clicked() {
                material.base_color = [0.3, 0.5, 1.0, 1.0];
                material.metallic = 0.0;
                material.roughness = 0.0;
                action = Some(MaterialAction::Modified);
            }
        });

        ui.separator();

        // Reset button
        if ui.button("🔄 Reset to Default").clicked() {
            material.base_color = [1.0, 1.0, 1.0, 1.0];
            material.metallic = 0.0;
            material.roughness = 0.5;
            material.emissive = None;
            material.alpha_mode = AlphaMode::Opaque;
            action = Some(MaterialAction::Reset);
        }

        action
    }

    /// Shows the texture picker UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `current_texture` - Currently selected texture path
    /// * `available_textures` - List of available texture paths
    ///
    /// # Returns
    ///
    /// Returns `Some(PathBuf)` if a texture is selected
    pub fn show_texture_picker(
        &mut self,
        ui: &mut egui::Ui,
        current_texture: &Option<String>,
        available_textures: &[PathBuf],
    ) -> Option<TextureAction> {
        let mut action = None;

        ui.heading("Texture Selector");
        ui.separator();

        // Current texture display
        ui.horizontal(|ui| {
            ui.label("Current Texture:");
            if let Some(path) = current_texture {
                ui.label(path);
            } else {
                ui.label("(None)");
            }
        });

        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            ui.label("Category:");
            egui::ComboBox::from_id_salt("texture_category")
                .selected_text(self.texture_category.name())
                .show_ui(ui, |ui| {
                    for category in TextureCategory::all() {
                        ui.selectable_value(&mut self.texture_category, category, category.name());
                    }
                });

            ui.separator();

            ui.label("🔍");
            ui.text_edit_singleline(&mut self.texture_search);

            if ui.button("🗙").on_hover_text("Clear search").clicked() {
                self.texture_search.clear();
            }
        });

        ui.separator();

        // Filter textures
        let filtered_textures: Vec<_> = available_textures
            .iter()
            .filter(|path| {
                if !self.texture_search.is_empty() {
                    let path_str = path.to_string_lossy().to_lowercase();
                    path_str.contains(&self.texture_search.to_lowercase())
                } else {
                    true
                }
            })
            .collect();

        // Texture grid
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                if filtered_textures.is_empty() {
                    ui.label("No textures found");
                } else {
                    let item_size = 100.0;
                    let available_width = ui.available_width();
                    let items_per_row = (available_width / (item_size + 10.0)).max(1.0) as usize;

                    for row in filtered_textures.chunks(items_per_row) {
                        ui.horizontal(|ui| {
                            for texture_path in row {
                                let response = ui
                                    .vertical(|ui| {
                                        // Texture thumbnail placeholder
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(item_size, item_size),
                                            egui::Sense::click(),
                                        );

                                        // Draw placeholder
                                        ui.painter().rect_filled(
                                            rect,
                                            4.0,
                                            egui::Color32::from_gray(80),
                                        );

                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "🖼",
                                            egui::FontId::proportional(32.0),
                                            egui::Color32::WHITE,
                                        );

                                        // Texture name
                                        let name = texture_path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        ui.label(name);

                                        response
                                    })
                                    .inner;

                                if response.clicked() {
                                    let path_str = texture_path.to_string_lossy().to_string();
                                    action = Some(TextureAction::Select(path_str));
                                }
                            }
                        });
                    }
                }
            });

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("🗑 Clear Texture").clicked() {
                action = Some(TextureAction::Clear);
            }

            if ui.button("📁 Browse Files").clicked() {
                action = Some(TextureAction::BrowseFiles);
            }
        });

        action
    }
}

impl Default for TexturePickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl TexturePickerState {
    /// Creates a new texture picker state
    pub fn new() -> Self {
        Self {
            selected_texture: None,
            search_query: String::new(),
            category_filter: TextureCategory::All,
            view_mode: TextureViewMode::Grid,
            grid_item_size: 100.0,
        }
    }
}

/// Returns the display name for an alpha mode
fn alpha_mode_name(mode: &AlphaMode) -> &str {
    match mode {
        AlphaMode::Opaque => "Opaque",
        AlphaMode::Mask => "Mask",
        AlphaMode::Blend => "Blend",
    }
}

/// Actions that can be performed on materials
#[derive(Debug, Clone)]
pub enum MaterialAction {
    /// Material was modified
    Modified,

    /// Material was reset to default
    Reset,
}

/// Actions that can be performed on textures
#[derive(Debug, Clone)]
pub enum TextureAction {
    /// Select a texture
    Select(String),

    /// Clear the texture
    Clear,

    /// Browse files to add a new texture
    BrowseFiles,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_editor_state_new() {
        let state = MaterialEditorState::new();
        assert!(!state.show_texture_picker);
        assert_eq!(state.texture_category, TextureCategory::Diffuse);
        assert_eq!(state.texture_search, "");
        assert!(state.show_preview);
        assert_eq!(state.preview_background, [0.2, 0.2, 0.2]);
    }

    #[test]
    fn test_texture_picker_state_new() {
        let state = TexturePickerState::new();
        assert_eq!(state.selected_texture, None);
        assert_eq!(state.search_query, "");
        assert_eq!(state.category_filter, TextureCategory::All);
        assert_eq!(state.view_mode, TextureViewMode::Grid);
        assert_eq!(state.grid_item_size, 100.0);
    }

    #[test]
    fn test_texture_category_all() {
        let categories = TextureCategory::all();
        assert_eq!(categories.len(), 7);
        assert!(categories.contains(&TextureCategory::All));
        assert!(categories.contains(&TextureCategory::Diffuse));
        assert!(categories.contains(&TextureCategory::Normal));
    }

    #[test]
    fn test_texture_category_names() {
        assert_eq!(TextureCategory::All.name(), "All");
        assert_eq!(TextureCategory::Diffuse.name(), "Diffuse/Color");
        assert_eq!(TextureCategory::Normal.name(), "Normal Map");
        assert_eq!(TextureCategory::Metallic.name(), "Metallic");
        assert_eq!(TextureCategory::Roughness.name(), "Roughness");
        assert_eq!(TextureCategory::Emissive.name(), "Emissive");
    }

    #[test]
    fn test_alpha_mode_names() {
        assert_eq!(alpha_mode_name(&AlphaMode::Opaque), "Opaque");
        assert_eq!(alpha_mode_name(&AlphaMode::Mask), "Mask");
        assert_eq!(alpha_mode_name(&AlphaMode::Blend), "Blend");
    }

    #[test]
    fn test_texture_view_mode_variants() {
        assert_eq!(TextureViewMode::Grid, TextureViewMode::Grid);
        assert_eq!(TextureViewMode::List, TextureViewMode::List);
        assert_ne!(TextureViewMode::Grid, TextureViewMode::List);
    }

    #[test]
    fn test_material_action_variants() {
        let action = MaterialAction::Modified;
        assert!(matches!(action, MaterialAction::Modified));

        let action = MaterialAction::Reset;
        assert!(matches!(action, MaterialAction::Reset));
    }

    #[test]
    fn test_texture_action_variants() {
        let action = TextureAction::Select("test.png".to_string());
        assert!(matches!(action, TextureAction::Select(_)));

        let action = TextureAction::Clear;
        assert!(matches!(action, TextureAction::Clear));

        let action = TextureAction::BrowseFiles;
        assert!(matches!(action, TextureAction::BrowseFiles));
    }

    #[test]
    fn test_material_editor_default() {
        let state = MaterialEditorState::default();
        assert!(!state.show_texture_picker);
        assert!(state.available_textures.is_empty());
    }

    #[test]
    fn test_texture_picker_default() {
        let state = TexturePickerState::default();
        assert_eq!(state.selected_texture, None);
        assert_eq!(state.category_filter, TextureCategory::All);
    }

    #[test]
    fn test_material_definition_presets() {
        // Test that we can create basic material presets
        let mut material = MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: None,
            alpha_mode: AlphaMode::Opaque,
        };

        // Metal preset
        material.base_color = [0.8, 0.8, 0.8, 1.0];
        material.metallic = 1.0;
        material.roughness = 0.2;
        assert_eq!(material.metallic, 1.0);

        // Plastic preset
        material.base_color = [1.0, 1.0, 1.0, 1.0];
        material.metallic = 0.0;
        material.roughness = 0.5;
        assert_eq!(material.metallic, 0.0);

        // Stone preset
        material.base_color = [0.5, 0.5, 0.5, 1.0];
        material.metallic = 0.0;
        material.roughness = 0.8;
        assert_eq!(material.roughness, 0.8);
    }
}
