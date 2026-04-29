// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! 3D preview panel rendering helpers for the creature editor.
//!
//! Extracted from the main creatures editor module for maintainability.

use super::{CreatureEditorError, CreaturesEditorState};
use crate::preview_features::PreviewStatistics;
use eframe::egui;

impl CreaturesEditorState {
    /// Show 3D preview panel (center, flex)
    pub(super) fn show_preview_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Preview");

        // Preview controls overlay
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.show_grid, "Grid").changed() {
                self.preview_dirty = true;
                ui.ctx().request_repaint();
            }
            if ui.checkbox(&mut self.show_wireframe, "Wireframe").changed() {
                self.preview_dirty = true;
                ui.ctx().request_repaint();
            }
            if ui.checkbox(&mut self.show_normals, "Normals").changed() {
                self.preview_dirty = true;
                ui.ctx().request_repaint();
            }
            if ui.checkbox(&mut self.show_axes, "Axes").changed() {
                self.preview_dirty = true;
                ui.ctx().request_repaint();
            }

            if ui.button("🔄 Reset Camera").clicked() {
                if let Some(renderer) = self.preview_renderer.as_mut() {
                    renderer.reset_camera();
                    self.camera_distance = renderer.camera.distance;
                    self.preview_state.camera.position = renderer.camera.position();
                } else {
                    self.camera_distance = 5.0;
                    self.preview_state.reset_camera();
                }
                ui.ctx().request_repaint();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Camera Distance:");
            if ui
                .add(
                    egui::Slider::new(&mut self.camera_distance, 1.0..=10.0)
                        .text("units")
                        .show_value(true),
                )
                .changed()
            {
                if let Some(renderer) = self.preview_renderer.as_mut() {
                    renderer.camera.distance = self.camera_distance;
                    self.preview_state.camera.position = renderer.camera.position();
                }
                ui.ctx().request_repaint();
            }

            ui.label("Background:");
            if ui
                .color_edit_button_rgba_unmultiplied(&mut self.background_color)
                .changed()
            {
                self.preview_dirty = true;
                ui.ctx().request_repaint();
            }
        });

        if let Some(renderer) = self.preview_renderer.as_mut() {
            ui.horizontal(|ui| {
                ui.label("Triangle Budget:");
                let mut max_preview_triangles = renderer.options.max_preview_triangles as u32;
                if ui
                    .add(
                        egui::Slider::new(&mut max_preview_triangles, 1_000..=100_000)
                            .text("triangles")
                            .logarithmic(true),
                    )
                    .changed()
                {
                    renderer.options.max_preview_triangles = max_preview_triangles as usize;
                    ui.ctx().request_repaint();
                }
            });
        }

        ui.separator();

        if self.preview_dirty {
            if let Err(error) = self.sync_preview_renderer_from_edit_buffer() {
                self.preview_error = Some(error.to_string());
            }
        }

        if let Some(renderer) = self.preview_renderer.as_mut() {
            renderer.options.resolution = (
                ui.available_width().max(240.0) as u32,
                ui.available_height().max(220.0) as u32,
            );

            let interacted = renderer.show(ui);
            self.camera_distance = renderer.camera.distance;
            self.preview_state.camera.position = renderer.camera.position();

            if interacted {
                ui.ctx().request_repaint();
            }
        } else {
            self.show_preview_fallback(ui);
        }

        if let Some(error) = &self.preview_error {
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Preview fallback: {}", error),
            );
        }
    }

    /// Show fallback placeholder when the 3D preview renderer is unavailable.
    pub(super) fn show_preview_fallback(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ui.available_height().max(220.0)),
            egui::Sense::hover(),
        );

        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgba_premultiplied(
                (self.background_color[0] * 255.0) as u8,
                (self.background_color[1] * 255.0) as u8,
                (self.background_color[2] * 255.0) as u8,
                (self.background_color[3] * 255.0) as u8,
            ),
        );

        let fallback = if self.preview_error.is_some() {
            "3D preview renderer unavailable.\nDiagnostics mode is active."
        } else {
            "Preparing preview renderer..."
        };

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            fallback,
            egui::FontId::proportional(15.0),
            egui::Color32::LIGHT_GRAY,
        );
    }

    /// Synchronise the preview renderer state from the current edit buffer.
    pub(super) fn sync_preview_renderer_from_edit_buffer(
        &mut self,
    ) -> Result<(), CreatureEditorError> {
        let visible = self.current_mesh_visibility();
        let preview_creature = self.edit_buffer.clone();
        let selected_mesh_index = self.selected_mesh_index;
        let stats = self.build_preview_statistics(&visible);

        let renderer = self
            .preview_renderer
            .as_mut()
            .ok_or(CreatureEditorError::PreviewRendererUnavailable)?;

        renderer.options.show_grid = self.show_grid;
        renderer.options.show_wireframe = self.show_wireframe;
        renderer.options.show_normals = self.show_normals;
        renderer.options.show_axes = self.show_axes;
        renderer.options.background_color = self.background_color;
        renderer.camera.distance = self.camera_distance;

        renderer.set_mesh_visibility(visible);
        renderer.set_selected_mesh_index(selected_mesh_index);
        renderer.update_creature(Some(preview_creature));

        self.preview_state.options.show_grid = self.show_grid;
        self.preview_state.options.show_wireframe = self.show_wireframe;
        self.preview_state.options.show_normals = self.show_normals;
        self.preview_state.options.show_axes = self.show_axes;
        self.preview_state.options.background_color = self.background_color;

        self.preview_state.update_statistics(stats);

        self.preview_dirty = false;
        Ok(())
    }

    /// Return a per-mesh visibility vector derived from `self.mesh_visibility`.
    pub(super) fn current_mesh_visibility(&self) -> Vec<bool> {
        self.edit_buffer
            .meshes
            .iter()
            .enumerate()
            .map(|(idx, _)| self.mesh_visibility.get(idx).copied().unwrap_or(true))
            .collect()
    }

    /// Build a [`PreviewStatistics`] snapshot for the currently visible meshes.
    pub(super) fn build_preview_statistics(&self, visible: &[bool]) -> PreviewStatistics {
        let mut stats = PreviewStatistics::new();
        stats.mesh_count = self.edit_buffer.meshes.len();
        stats.selected_meshes = usize::from(self.selected_mesh_index.is_some());

        for (idx, mesh) in self.edit_buffer.meshes.iter().enumerate() {
            if visible.get(idx).copied().unwrap_or(true) {
                stats.vertex_count += mesh.vertices.len();
                stats.triangle_count += mesh.indices.len() / 3;
            }
        }

        stats
    }
}
