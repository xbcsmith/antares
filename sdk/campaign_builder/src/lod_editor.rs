// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! LOD (Level of Detail) editor UI for the campaign builder
//!
//! This module provides UI components for generating, configuring, and previewing
//! LOD levels for creature meshes. LOD allows optimizing rendering performance by
//! using simpler meshes at greater distances.
//!
//! # Architecture
//!
//! The LOD editor integrates with the creatures editor to:
//! - Generate LOD levels automatically from base meshes
//! - Configure distance thresholds for LOD switching
//! - Preview individual LOD levels
//! - Manually adjust LOD meshes
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::lod_editor::LodEditorState;
//! use antares::domain::visual::MeshDefinition;
//!
//! let mut state = LodEditorState::new();
//! let mut mesh = MeshDefinition::default();
//!
//! // In your egui UI context:
//! // state.show(ui, &mut mesh);
//! ```

use antares::domain::visual::lod::generate_lod_levels;
use antares::domain::visual::MeshDefinition;
use eframe::egui;

/// State for the LOD editor UI
#[derive(Debug, Clone)]
pub struct LodEditorState {
    /// Currently selected LOD level for preview
    pub selected_lod_level: Option<usize>,

    /// Number of LOD levels to generate
    pub num_lod_levels: usize,

    /// Target reduction ratios for each LOD level
    pub reduction_ratios: Vec<f32>,

    /// Distance thresholds for LOD switching
    pub lod_distances: Vec<f32>,

    /// Show LOD generation dialog
    pub show_generate_dialog: bool,

    /// Show LOD preview panel
    pub show_preview: bool,

    /// Auto-generate LOD on mesh changes
    pub auto_generate: bool,

    /// Use billboard for furthest LOD
    pub use_billboard_fallback: bool,
}

impl Default for LodEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl LodEditorState {
    /// Creates a new LOD editor state with default values
    pub fn new() -> Self {
        Self {
            selected_lod_level: None,
            num_lod_levels: 3,
            reduction_ratios: vec![0.75, 0.5, 0.25],
            lod_distances: vec![10.0, 25.0, 50.0],
            show_generate_dialog: false,
            show_preview: true,
            auto_generate: false,
            use_billboard_fallback: true,
        }
    }

    /// Shows the LOD editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `mesh` - The base mesh to generate LODs for
    ///
    /// # Returns
    ///
    /// Returns `Some(LodAction)` if an action should be performed
    pub fn show(&mut self, ui: &mut egui::Ui, mesh: &mut MeshDefinition) -> Option<LodAction> {
        let mut action = None;

        ui.heading("Level of Detail (LOD)");
        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("🔧 Generate LODs").clicked() {
                self.show_generate_dialog = true;
            }

            if ui.button("🗑 Clear LODs").clicked() {
                action = Some(LodAction::ClearLods);
            }

            ui.separator();

            ui.checkbox(&mut self.auto_generate, "Auto-generate")
                .on_hover_text("Automatically generate LODs when mesh changes");

            ui.checkbox(&mut self.show_preview, "Show Preview");
        });

        ui.separator();

        // Current LOD status
        ui.group(|ui| {
            ui.label(format!(
                "Current LOD Levels: {}",
                mesh.lod_levels.as_ref().map_or(0, |l| l.len())
            ));
            ui.label(format!(
                "Current LOD Distances: {}",
                mesh.lod_distances.as_ref().map_or(0, |d| d.len())
            ));
        });

        ui.separator();

        // Two-column layout
        ui.columns(2, |columns| {
            // Left column: LOD level list
            columns[0].vertical(|ui| {
                ui.heading("LOD Levels");
                ui.separator();

                if let Some(lod_levels) = &mesh.lod_levels {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            // Base level (LOD 0)
                            let is_selected = self.selected_lod_level == Some(0);
                            if ui.selectable_label(is_selected, "LOD 0 (Base)").clicked() {
                                self.selected_lod_level = Some(0);
                            }

                            ui.label(format!("  Vertices: {}", mesh.vertices.len()));
                            ui.label(format!("  Triangles: {}", mesh.indices.len() / 3));

                            ui.separator();

                            // Additional LOD levels
                            for (idx, lod_mesh) in lod_levels.iter().enumerate() {
                                let lod_level = idx + 1;
                                let is_selected = self.selected_lod_level == Some(lod_level);

                                if ui
                                    .selectable_label(is_selected, format!("LOD {}", lod_level))
                                    .clicked()
                                {
                                    self.selected_lod_level = Some(lod_level);
                                }

                                ui.label(format!("  Vertices: {}", lod_mesh.vertices.len()));
                                ui.label(format!("  Triangles: {}", lod_mesh.indices.len() / 3));

                                if let Some(distances) = &mesh.lod_distances {
                                    if let Some(distance) = distances.get(idx) {
                                        ui.label(format!("  Distance: {:.1}", distance));
                                    }
                                }

                                ui.separator();
                            }
                        });
                } else {
                    ui.label("No LOD levels generated");
                    ui.label("Click 'Generate LODs' to create them");
                }
            });

            // Right column: LOD configuration
            columns[1].vertical(|ui| {
                ui.heading("LOD Configuration");
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        if let Some(level) = self.selected_lod_level {
                            if level == 0 {
                                ui.label("Base mesh (LOD 0)");
                                ui.label("This is the original high-detail mesh");
                            } else if let Some(lod_levels) = &mesh.lod_levels {
                                let lod_idx = level - 1;
                                if lod_idx < lod_levels.len() {
                                    ui.label(format!("LOD Level {}", level));
                                    ui.separator();

                                    // Distance threshold editor
                                    if let Some(distances) = mesh.lod_distances.as_mut() {
                                        if lod_idx < distances.len() {
                                            ui.horizontal(|ui| {
                                                ui.label("Switch Distance:");
                                                ui.add(
                                                    egui::DragValue::new(&mut distances[lod_idx])
                                                        .speed(0.5)
                                                        .range(0.0..=1000.0)
                                                        .suffix(" units"),
                                                );
                                            });
                                        }
                                    }

                                    ui.separator();

                                    // Mesh statistics
                                    let lod_mesh = &lod_levels[lod_idx];
                                    ui.label("Mesh Statistics:");
                                    ui.label(format!("  Vertices: {}", lod_mesh.vertices.len()));
                                    ui.label(format!(
                                        "  Triangles: {}",
                                        lod_mesh.indices.len() / 3
                                    ));
                                    ui.label(format!(
                                        "  Normals: {}",
                                        lod_mesh.normals.as_ref().map(|n| n.len()).unwrap_or(0)
                                    ));
                                    ui.label(format!(
                                        "  UVs: {}",
                                        lod_mesh.uvs.as_ref().map(|u| u.len()).unwrap_or(0)
                                    ));

                                    // Reduction percentage
                                    let base_vertices = mesh.vertices.len();
                                    let lod_vertices = lod_mesh.vertices.len();
                                    let reduction_pct = if base_vertices > 0 {
                                        100.0 * (1.0 - (lod_vertices as f32 / base_vertices as f32))
                                    } else {
                                        0.0
                                    };
                                    ui.label(format!("  Reduction: {:.1}%", reduction_pct));
                                }
                            }
                        } else {
                            ui.label("Select an LOD level to view details");
                        }
                    });
            });
        });

        // Generate LOD dialog
        if self.show_generate_dialog {
            if let Some(lod_action) = self.show_generate_dialog(ui.ctx()) {
                action = Some(lod_action);
                self.show_generate_dialog = false;
            }
        }

        action
    }

    /// Shows the generate LOD dialog
    ///
    /// Returns `Some(LodAction)` if the user confirms generation
    fn show_generate_dialog(&mut self, ctx: &egui::Context) -> Option<LodAction> {
        let mut result = None;
        let mut close_dialog = false;

        egui::Window::new("Generate LOD Levels")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.label("Configure LOD generation parameters:");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Number of LOD levels:");
                    ui.add(
                        egui::DragValue::new(&mut self.num_lod_levels)
                            .speed(1)
                            .range(1..=5),
                    );
                });

                ui.separator();

                ui.label("Reduction ratios for each level:");
                ui.label("(1.0 = no reduction, 0.0 = complete reduction)");

                // Ensure we have the right number of reduction ratios
                while self.reduction_ratios.len() < self.num_lod_levels {
                    let last = *self.reduction_ratios.last().unwrap_or(&0.5);
                    self.reduction_ratios.push((last * 0.7).max(0.1));
                }
                self.reduction_ratios.truncate(self.num_lod_levels);

                for (i, ratio) in self.reduction_ratios.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("LOD {}:", i + 1));
                        ui.add(egui::DragValue::new(ratio).speed(0.01).range(0.1..=1.0));
                    });
                }

                ui.separator();

                ui.label("Distance thresholds for LOD switching:");

                // Ensure we have the right number of distances
                while self.lod_distances.len() < self.num_lod_levels {
                    let last = *self.lod_distances.last().unwrap_or(&10.0);
                    self.lod_distances.push(last * 2.0);
                }
                self.lod_distances.truncate(self.num_lod_levels);

                for (i, distance) in self.lod_distances.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("LOD {} distance:", i + 1));
                        ui.add(
                            egui::DragValue::new(distance)
                                .speed(0.5)
                                .range(0.0..=1000.0)
                                .suffix(" units"),
                        );
                    });
                }

                ui.separator();

                ui.checkbox(
                    &mut self.use_billboard_fallback,
                    "Use billboard for furthest LOD",
                )
                .on_hover_text("Replace the furthest LOD with a simple billboard quad");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Generate").clicked() {
                        result = Some(LodAction::GenerateLods {
                            num_levels: self.num_lod_levels,
                            reduction_ratios: self.reduction_ratios.clone(),
                            distances: self.lod_distances.clone(),
                            use_billboard: self.use_billboard_fallback,
                        });
                        close_dialog = true;
                    }

                    if ui.button("✗ Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if close_dialog {
            self.show_generate_dialog = false;
        }

        result
    }
}

/// Actions that can be performed on LODs
#[derive(Debug, Clone)]
pub enum LodAction {
    /// Generate LOD levels with specified parameters
    GenerateLods {
        num_levels: usize,
        reduction_ratios: Vec<f32>,
        distances: Vec<f32>,
        use_billboard: bool,
    },

    /// Clear all LOD levels
    ClearLods,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_editor_state_new() {
        let state = LodEditorState::new();
        assert_eq!(state.selected_lod_level, None);
        assert_eq!(state.num_lod_levels, 3);
        assert_eq!(state.reduction_ratios, vec![0.75, 0.5, 0.25]);
        assert_eq!(state.lod_distances, vec![10.0, 25.0, 50.0]);
        assert!(!state.show_generate_dialog);
        assert!(state.show_preview);
        assert!(!state.auto_generate);
        assert!(state.use_billboard_fallback);
    }

    #[test]
    fn test_lod_editor_state_default() {
        let state = LodEditorState::default();
        assert_eq!(state.num_lod_levels, 3);
        assert!(state.show_preview);
    }

    #[test]
    fn test_lod_action_generate() {
        let action = LodAction::GenerateLods {
            num_levels: 3,
            reduction_ratios: vec![0.75, 0.5, 0.25],
            distances: vec![10.0, 20.0, 40.0],
            use_billboard: true,
        };

        if let LodAction::GenerateLods {
            num_levels,
            reduction_ratios,
            distances,
            use_billboard,
        } = action
        {
            assert_eq!(num_levels, 3);
            assert_eq!(reduction_ratios.len(), 3);
            assert_eq!(distances.len(), 3);
            assert!(use_billboard);
        } else {
            panic!("Expected GenerateLods action");
        }
    }

    #[test]
    fn test_lod_action_clear() {
        let action = LodAction::ClearLods;
        assert!(matches!(action, LodAction::ClearLods));
    }

    #[test]
    fn test_lod_reduction_ratios_valid_range() {
        let state = LodEditorState::new();
        for ratio in &state.reduction_ratios {
            assert!(*ratio >= 0.1 && *ratio <= 1.0);
        }
    }

    #[test]
    fn test_lod_distances_positive() {
        let state = LodEditorState::new();
        for distance in &state.lod_distances {
            assert!(*distance > 0.0);
        }
    }

    #[test]
    fn test_lod_distances_increasing() {
        let state = LodEditorState::new();
        for i in 1..state.lod_distances.len() {
            assert!(state.lod_distances[i] > state.lod_distances[i - 1]);
        }
    }
}
