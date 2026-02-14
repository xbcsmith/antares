// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Variation editor UI for the campaign builder
//!
//! This module provides UI components for browsing, applying, and previewing
//! creature variations. Variations allow creating multiple versions of a base
//! creature with different properties (size, color, mesh modifications).
//!
//! # Architecture
//!
//! The variation editor integrates with the creatures editor to:
//! - Display available variations for the selected creature
//! - Apply variations to create new creature instances
//! - Preview variation effects in real-time
//! - Create custom variations from modified creatures
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::variation_editor::VariationEditorState;
//! use antares::domain::visual::{CreatureDefinition, CreatureVariation};
//!
//! let mut state = VariationEditorState::new();
//! let mut creature = CreatureDefinition::default();
//! let variations = vec![];
//!
//! // In your egui UI context:
//! // state.show(ui, &mut creature, &variations);
//! ```

use antares::domain::visual::creature_variations::CreatureVariation;
use antares::domain::visual::CreatureDefinition;
use eframe::egui;

/// State for the variation editor UI
#[derive(Debug, Clone)]
pub struct VariationEditorState {
    /// Currently selected variation index
    pub selected_variation: Option<usize>,

    /// Search/filter query for variations
    pub search_query: String,

    /// Show variation creation dialog
    pub show_create_dialog: bool,

    /// Buffer for creating new variation
    pub create_buffer: VariationCreateBuffer,

    /// Show preview panel
    pub show_preview: bool,

    /// Preview the variation applied to the creature
    pub preview_applied: bool,
}

/// Buffer for creating a new variation
#[derive(Debug, Clone)]
pub struct VariationCreateBuffer {
    /// Name of the variation
    pub name: String,

    /// Description
    pub description: String,

    /// Scale multiplier
    pub scale_multiplier: f32,

    /// Color tint (RGBA, 0.0-1.0)
    pub color_tint: Option<[f32; 4]>,

    /// Whether to enable color tint
    pub enable_color_tint: bool,
}

impl Default for VariationEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl VariationEditorState {
    /// Creates a new variation editor state
    pub fn new() -> Self {
        Self {
            selected_variation: None,
            search_query: String::new(),
            show_create_dialog: false,
            create_buffer: VariationCreateBuffer::default(),
            show_preview: true,
            preview_applied: false,
        }
    }

    /// Shows the variation editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `creature` - The base creature to apply variations to
    /// * `variations` - List of available variations
    ///
    /// # Returns
    ///
    /// Returns `Some(CreatureVariation)` if a variation should be applied
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        creature: &CreatureDefinition,
        variations: &[CreatureVariation],
    ) -> Option<VariationAction> {
        let mut action = None;

        ui.heading("Creature Variations");
        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ Create Variation").clicked() {
                self.show_create_dialog = true;
                self.create_buffer = VariationCreateBuffer::from_creature(creature);
            }

            ui.separator();

            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            if ui.button("🗙").on_hover_text("Clear search").clicked() {
                self.search_query.clear();
            }

            ui.separator();

            ui.checkbox(&mut self.show_preview, "Show Preview");
        });

        ui.separator();

        // Two-column layout
        ui.columns(2, |columns| {
            // Left column: Variation list
            columns[0].vertical(|ui| {
                ui.heading("Available Variations");
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        let filtered_variations: Vec<_> = variations
                            .iter()
                            .enumerate()
                            .filter(|(_, v)| {
                                self.search_query.is_empty()
                                    || v.name
                                        .to_lowercase()
                                        .contains(&self.search_query.to_lowercase())
                                    || v.description
                                        .to_lowercase()
                                        .contains(&self.search_query.to_lowercase())
                            })
                            .collect();

                        if filtered_variations.is_empty() {
                            ui.label("No variations found");
                        } else {
                            for (idx, variation) in filtered_variations {
                                let is_selected = self.selected_variation == Some(idx);

                                let response = ui.selectable_label(is_selected, &variation.name);

                                if response.clicked() {
                                    self.selected_variation = Some(idx);
                                    self.preview_applied = false;
                                }

                                if response.double_clicked() {
                                    action = Some(VariationAction::Apply(variation.clone()));
                                }
                            }
                        }
                    });
            });

            // Right column: Variation details
            columns[1].vertical(|ui| {
                ui.heading("Variation Details");
                ui.separator();

                if let Some(idx) = self.selected_variation {
                    if let Some(variation) = variations.get(idx) {
                        self.show_variation_details(ui, variation, creature);

                        ui.separator();

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.button("✓ Apply Variation").clicked() {
                                action = Some(VariationAction::Apply(variation.clone()));
                            }

                            if ui.button("👁 Preview").clicked() {
                                self.preview_applied = !self.preview_applied;
                            }

                            if ui.button("📋 Duplicate").clicked() {
                                action = Some(VariationAction::Duplicate(variation.clone()));
                            }

                            if ui.button("🗑 Delete").clicked() {
                                action = Some(VariationAction::Delete(idx));
                            }
                        });
                    } else {
                        ui.label("Selected variation not found");
                    }
                } else {
                    ui.label("Select a variation to view details");
                }
            });
        });

        // Create variation dialog
        if self.show_create_dialog {
            if let Some(new_variation) = self.show_create_dialog(ui.ctx(), creature) {
                action = Some(VariationAction::Create(new_variation));
                self.show_create_dialog = false;
            }
        }

        action
    }

    /// Shows detailed information about a variation
    fn show_variation_details(
        &self,
        ui: &mut egui::Ui,
        variation: &CreatureVariation,
        _creature: &CreatureDefinition,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!("Name: {}", variation.name));
            ui.label(format!("Description: {}", variation.description));
            ui.separator();

            if let Some(scale) = variation.scale_override {
                ui.label(format!("Scale Override: {:.2}", scale));
            }

            if !variation.mesh_color_overrides.is_empty() {
                ui.label(format!(
                    "Color Overrides: {} mesh(es)",
                    variation.mesh_color_overrides.len()
                ));
            }

            if !variation.mesh_scale_overrides.is_empty() {
                ui.label(format!(
                    "Scale Overrides: {} mesh(es)",
                    variation.mesh_scale_overrides.len()
                ));
            }
        });
    }

    /// Shows the create variation dialog
    ///
    /// Returns `Some(CreatureVariation)` if the user confirms creation
    fn show_create_dialog(
        &mut self,
        ctx: &egui::Context,
        creature: &CreatureDefinition,
    ) -> Option<CreatureVariation> {
        let mut result = None;
        let mut close_dialog = false;

        egui::Window::new("Create New Variation")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.create_buffer.name);
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut self.create_buffer.description);
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Scale Multiplier:");
                    ui.add(
                        egui::DragValue::new(&mut self.create_buffer.scale_multiplier)
                            .speed(0.01)
                            .range(0.1..=10.0),
                    );
                });

                ui.separator();

                ui.checkbox(
                    &mut self.create_buffer.enable_color_tint,
                    "Enable Color Tint",
                );

                if self.create_buffer.enable_color_tint {
                    let color = self
                        .create_buffer
                        .color_tint
                        .get_or_insert([1.0, 1.0, 1.0, 1.0]);
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut rgb = [color[0], color[1], color[2]];
                        if ui.color_edit_button_rgb(&mut rgb).changed() {
                            color[0] = rgb[0];
                            color[1] = rgb[1];
                            color[2] = rgb[2];
                        }
                    });
                } else {
                    self.create_buffer.color_tint = None;
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Create").clicked() {
                        if !self.create_buffer.name.is_empty() {
                            result = Some(self.create_buffer.to_variation());
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
}

impl Default for VariationCreateBuffer {
    fn default() -> Self {
        Self {
            name: "New Variation".to_string(),
            description: String::new(),
            scale_multiplier: 1.0,
            color_tint: None,
            enable_color_tint: false,
        }
    }
}

impl VariationCreateBuffer {
    /// Creates a buffer from an existing creature (captures current state)
    pub fn from_creature(creature: &CreatureDefinition) -> Self {
        Self {
            name: format!("{} Variation", creature.name),
            description: format!("Variation of {}", creature.name),
            scale_multiplier: 1.0,
            color_tint: creature.color_tint,
            enable_color_tint: creature.color_tint.is_some(),
        }
    }

    /// Converts the buffer to a CreatureVariation
    pub fn to_variation(&self) -> CreatureVariation {
        CreatureVariation {
            base_creature_id: 0, // Will be set by caller
            name: self.name.clone(),
            scale_override: Some(self.scale_multiplier),
            mesh_color_overrides: std::collections::HashMap::new(),
            mesh_scale_overrides: std::collections::HashMap::new(),
        }
    }
}

/// Actions that can be performed on variations
#[derive(Debug, Clone)]
pub enum VariationAction {
    /// Apply a variation to the current creature
    Apply(CreatureVariation),

    /// Create a new variation
    Create(CreatureVariation),

    /// Duplicate an existing variation
    Duplicate(CreatureVariation),

    /// Delete a variation by index
    Delete(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variation_editor_state_new() {
        let state = VariationEditorState::new();
        assert_eq!(state.selected_variation, None);
        assert_eq!(state.search_query, "");
        assert!(!state.show_create_dialog);
        assert!(state.show_preview);
        assert!(!state.preview_applied);
    }

    #[test]
    fn test_variation_create_buffer_default() {
        let buffer = VariationCreateBuffer::default();
        assert_eq!(buffer.name, "New Variation");
        assert_eq!(buffer.description, "");
        assert_eq!(buffer.scale_multiplier, 1.0);
        assert_eq!(buffer.color_tint, None);
        assert!(!buffer.enable_color_tint);
    }

    #[test]
    fn test_variation_create_buffer_from_creature() {
        let creature = CreatureDefinition {
            id: 1,
            name: "Dragon".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.5,
            color_tint: Some([1.0, 0.0, 0.0, 1.0]),
        };

        let buffer = VariationCreateBuffer::from_creature(&creature);
        assert_eq!(buffer.name, "Dragon Variation");
        assert_eq!(buffer.description, "Variation of Dragon");
        assert_eq!(buffer.color_tint, Some([1.0, 0.0, 0.0]));
        assert!(buffer.enable_color_tint);
    }

    #[test]
    fn test_variation_create_buffer_to_variation() {
        let buffer = VariationCreateBuffer {
            name: "Test Variation".to_string(),
            description: "A test variation".to_string(),
            scale_multiplier: 1.5,
            color_tint: Some([0.5, 0.5, 0.5, 1.0]),
            enable_color_tint: true,
        };

        let variation = buffer.to_variation();
        assert_eq!(variation.name, "Test Variation");
        assert_eq!(variation.scale_override, Some(1.5));
    }

    #[test]
    fn test_variation_action_variants() {
        let variation = CreatureVariation {
            base_creature_id: 0,
            name: "Test".to_string(),
            scale_override: Some(1.0),
            mesh_color_overrides: std::collections::HashMap::new(),
            mesh_scale_overrides: std::collections::HashMap::new(),
        };

        let action = VariationAction::Apply(variation.clone());
        assert!(matches!(action, VariationAction::Apply(_)));

        let action = VariationAction::Create(variation.clone());
        assert!(matches!(action, VariationAction::Create(_)));

        let action = VariationAction::Duplicate(variation.clone());
        assert!(matches!(action, VariationAction::Duplicate(_)));

        let action = VariationAction::Delete(0);
        assert!(matches!(action, VariationAction::Delete(0)));
    }

    #[test]
    fn test_color_tint_enable_disable() {
        let mut buffer = VariationCreateBuffer::default();
        assert!(!buffer.enable_color_tint);
        assert_eq!(buffer.color_tint, None);

        buffer.enable_color_tint = true;
        buffer.color_tint = Some([1.0, 0.5, 0.0, 1.0]);

        let variation = buffer.to_variation();
        assert_eq!(variation.color_tint, Some([1.0, 0.5, 0.0, 1.0]));
    }
}
