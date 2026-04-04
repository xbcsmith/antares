// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh properties panel UI for the creature editor.
//!
//! Extracted from the main creatures editor module for maintainability.

use super::CreaturesEditorState;
use antares::domain::visual::MeshTransform;
use eframe::egui;

impl CreaturesEditorState {
    /// Show mesh properties panel (right, 350px)
    pub(super) fn show_mesh_properties_panel(
        &mut self,
        ui: &mut egui::Ui,
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        let mut result_message = None;
        if let Some(mesh_idx) = self.selected_mesh_index {
            if mesh_idx >= self.edit_buffer.meshes.len() {
                ui.label("Invalid mesh selection");
                return Some("Invalid mesh selection".to_string());
            }

            ui.heading(format!("Mesh {} Properties", mesh_idx));
            ui.separator();

            // Mesh Info Section
            ui.collapsing("Mesh Info", |ui| {
                egui::Grid::new("mesh_info_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        let mut name = self.edit_buffer.meshes[mesh_idx]
                            .name
                            .clone()
                            .unwrap_or_default();
                        if ui.text_edit_singleline(&mut name).changed() {
                            self.edit_buffer.meshes[mesh_idx].name =
                                if name.is_empty() { None } else { Some(name) };
                            if let Some(buffer) = &mut self.mesh_edit_buffer {
                                buffer.name = self.edit_buffer.meshes[mesh_idx].name.clone();
                            }
                            *unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Color:");
                        if ui
                            .color_edit_button_rgba_unmultiplied(
                                &mut self.edit_buffer.meshes[mesh_idx].color,
                            )
                            .changed()
                        {
                            if let Some(buffer) = &mut self.mesh_edit_buffer {
                                buffer.color = self.edit_buffer.meshes[mesh_idx].color;
                            }
                            *unsaved_changes = true;
                            self.preview_dirty = true;
                        }
                        ui.end_row();

                        ui.label("Vertices:");
                        ui.label(format!(
                            "{}",
                            self.edit_buffer.meshes[mesh_idx].vertices.len()
                        ));
                        ui.end_row();

                        ui.label("Triangles:");
                        ui.label(format!(
                            "{}",
                            self.edit_buffer.meshes[mesh_idx].indices.len() / 3
                        ));
                        ui.end_row();
                    });
            });

            // Transform Section
            if let Some(transform) = self.mesh_transform_buffer.as_mut() {
                ui.collapsing("Transform", |ui| {
                    egui::Grid::new("mesh_transform_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Translation:");
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[0])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[1])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[2])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label("Rotation (deg):");
                            ui.vertical(|ui| {
                                let mut pitch_deg = transform.rotation[0].to_degrees();
                                let mut yaw_deg = transform.rotation[1].to_degrees();
                                let mut roll_deg = transform.rotation[2].to_degrees();

                                ui.horizontal(|ui| {
                                    ui.label("Pitch:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut pitch_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[0] = pitch_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Yaw:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut yaw_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[1] = yaw_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Roll:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut roll_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[2] = roll_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label("Scale:");
                            ui.vertical(|ui| {
                                ui.checkbox(&mut self.uniform_scale, "Uniform scaling");

                                if self.uniform_scale {
                                    let mut uniform = transform.scale[0];
                                    ui.horizontal(|ui| {
                                        ui.label("XYZ:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut uniform)
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            transform.scale = [uniform, uniform, uniform];
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[0])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[1])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[2])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                }
                            });
                            ui.end_row();
                        });
                });
            }

            // Geometry Section
            ui.collapsing("Geometry", |ui| {
                let mesh = &self.edit_buffer.meshes[mesh_idx];
                ui.label(format!("Vertices: {}", mesh.vertices.len()));
                ui.label(format!("Triangles: {}", mesh.indices.len() / 3));
                ui.label(format!(
                    "Normals: {}",
                    if mesh.normals.is_some() { "Yes" } else { "No" }
                ));
                ui.label(format!(
                    "UVs: {}",
                    if mesh.uvs.is_some() { "Yes" } else { "No" }
                ));

                // TODO: Add View/Edit Table buttons for vertices/indices/normals
            });

            // Action Buttons
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("🔄 Replace with Primitive").clicked() {
                    self.show_primitive_dialog = true;
                    self.primitive_use_current_color = true;
                    self.primitive_preserve_transform = true;
                    self.primitive_keep_name = true;
                }

                if ui.button("🔍 Validate Mesh").clicked() {
                    result_message = Some(self.validate_selected_mesh(mesh_idx));
                    ui.ctx().request_repaint();
                }

                if ui.button("↺ Reset Transform").clicked() {
                    self.edit_buffer.mesh_transforms[mesh_idx] = MeshTransform::identity();
                    self.mesh_transform_buffer = Some(MeshTransform::identity());
                    *unsaved_changes = true;
                    self.preview_dirty = true;
                }
            });
        } else {
            ui.label("Select a mesh to edit its properties");
        }

        result_message
    }
}
