// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! 3D preview renderer for creature visualization in the campaign builder
//!
//! This module provides an embedded Bevy rendering context that displays
//! procedurally-generated creature meshes in real-time within an egui panel.
//!
//! # Architecture
//!
//! The preview renderer uses a separate Bevy app instance that renders to a
//! texture, which is then displayed in an egui image widget. This provides:
//!
//! - Real-time mesh updates as the user edits creature properties
//! - Camera controls (orbit, zoom, pan)
//! - Grid and axis helpers for spatial reference
//! - Lighting to visualize mesh geometry
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::preview_renderer::PreviewRenderer;
//! use antares::domain::visual::CreatureDefinition;
//!
//! let mut renderer = PreviewRenderer::new();
//!
//! // Update the creature being previewed
//! let creature = CreatureDefinition {
//!     id: 1,
//!     name: "Test Creature".to_string(),
//!     meshes: vec![],
//!     mesh_transforms: vec![],
//!     scale: 1.0,
//!     color_tint: None,
//! };
//! renderer.update_creature(Some(creature));
//!
//! // Render preview in egui context
//! // ui.add(renderer.ui_widget());
//! ```

use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Camera control mode for the 3D preview
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraMode {
    /// Orbit around target point
    Orbit,
    /// Free-look camera (not yet implemented)
    FreeLook,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self::Orbit
    }
}

/// State for the 3D preview camera
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraState {
    /// Camera control mode
    pub mode: CameraMode,

    /// Distance from target point
    pub distance: f32,

    /// Azimuth angle (rotation around Y axis) in radians
    pub azimuth: f32,

    /// Elevation angle (up/down) in radians
    pub elevation: f32,

    /// Target point the camera looks at
    pub target: [f32; 3],

    /// Field of view in degrees
    pub fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            mode: CameraMode::Orbit,
            distance: 5.0,
            azimuth: 0.785,   // 45 degrees
            elevation: 0.524, // 30 degrees
            target: [0.0, 0.0, 0.0],
            fov: 60.0,
        }
    }
}

impl CameraState {
    /// Calculate camera position from orbital parameters
    pub fn position(&self) -> [f32; 3] {
        let x = self.target[0] + self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.target[1] + self.distance * self.elevation.sin();
        let z = self.target[2] + self.distance * self.elevation.cos() * self.azimuth.cos();
        [x, y, z]
    }

    /// Orbit the camera by delta angles
    pub fn orbit(&mut self, delta_azimuth: f32, delta_elevation: f32) {
        self.azimuth += delta_azimuth;
        self.elevation = (self.elevation + delta_elevation).clamp(-1.57, 1.57); // Clamp to ±90°
    }

    /// Zoom the camera in or out
    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance + delta).max(0.1);
    }

    /// Pan the camera (move target point)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Transform delta based on camera orientation
        let right_x = self.azimuth.cos();
        let right_z = -self.azimuth.sin();

        self.target[0] += delta_x * right_x;
        self.target[2] += delta_x * right_z;
        self.target[1] += delta_y;
    }

    /// Reset camera to default position
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Preview renderer options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewOptions {
    /// Show grid helper
    pub show_grid: bool,

    /// Show axis helpers (X=red, Y=green, Z=blue)
    pub show_axes: bool,

    /// Show wireframe overlay
    pub show_wireframe: bool,

    /// Show normals as lines
    pub show_normals: bool,

    /// Background color [r, g, b, a]
    pub background_color: [f32; 4],

    /// Enable lighting
    pub enable_lighting: bool,

    /// Render resolution (width, height)
    pub resolution: (u32, u32),
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_axes: true,
            show_wireframe: false,
            show_normals: false,
            background_color: [0.2, 0.2, 0.25, 1.0],
            enable_lighting: true,
            resolution: (512, 512),
        }
    }
}

/// 3D preview renderer for creatures
///
/// This renderer manages a separate rendering context (conceptually a Bevy app)
/// that renders creature meshes to a texture for display in egui.
///
/// # Note
///
/// For Phase 3, this is a simplified implementation that doesn't actually
/// spawn a full Bevy app (to avoid complexity with nested event loops).
/// Instead, it provides the state management and will render using egui's
/// built-in painting primitives or a simple software rasterizer.
///
/// Future phases can integrate a true offscreen Bevy renderer using
/// render-to-texture techniques.
pub struct PreviewRenderer {
    /// Current creature being previewed
    creature: Arc<Mutex<Option<CreatureDefinition>>>,

    /// Camera state
    pub camera: CameraState,

    /// Render options
    pub options: PreviewOptions,

    /// Needs redraw flag
    needs_update: bool,

    /// Last mouse position for drag interactions
    last_mouse_pos: Option<(f32, f32)>,
}

impl Default for PreviewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewRenderer {
    /// Create a new preview renderer
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::preview_renderer::PreviewRenderer;
    ///
    /// let renderer = PreviewRenderer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            creature: Arc::new(Mutex::new(None)),
            camera: CameraState::default(),
            options: PreviewOptions::default(),
            needs_update: true,
            last_mouse_pos: None,
        }
    }

    /// Update the creature being previewed
    ///
    /// This triggers a re-render on the next frame.
    ///
    /// # Arguments
    ///
    /// * `creature` - The creature definition to preview, or None to clear
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::preview_renderer::PreviewRenderer;
    /// use antares::domain::visual::CreatureDefinition;
    ///
    /// let mut renderer = PreviewRenderer::new();
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Test".to_string(),
    ///     meshes: vec![],
    ///     mesh_transforms: vec![],
    ///     scale: 1.0,
    ///     color_tint: None,
    /// };
    /// renderer.update_creature(Some(creature));
    /// ```
    pub fn update_creature(&mut self, creature: Option<CreatureDefinition>) {
        if let Ok(mut locked) = self.creature.lock() {
            *locked = creature;
            self.needs_update = true;
        }
    }

    /// Get the current creature being previewed
    pub fn get_creature(&self) -> Option<CreatureDefinition> {
        self.creature.lock().ok().and_then(|c| c.clone())
    }

    /// Reset camera to default position
    pub fn reset_camera(&mut self) {
        self.camera.reset();
        self.needs_update = true;
    }

    /// Show the preview renderer UI
    ///
    /// Renders the 3D preview and camera controls in an egui panel.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// Returns true if the preview was interacted with (camera moved, etc.)
    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        // Preview viewport
        let (response, painter) = ui.allocate_painter(
            egui::vec2(
                self.options.resolution.0 as f32,
                self.options.resolution.1 as f32,
            ),
            egui::Sense::click_and_drag(),
        );

        // Draw background
        painter.rect_filled(
            response.rect,
            0.0,
            egui::Color32::from_rgba_premultiplied(
                (self.options.background_color[0] * 255.0) as u8,
                (self.options.background_color[1] * 255.0) as u8,
                (self.options.background_color[2] * 255.0) as u8,
                (self.options.background_color[3] * 255.0) as u8,
            ),
        );

        // Handle mouse interaction for camera controls
        if response.dragged() {
            let delta = response.drag_delta();

            if ui.input(|i| i.modifiers.shift) {
                // Pan
                let pan_speed = 0.01;
                self.camera.pan(-delta.x * pan_speed, delta.y * pan_speed);
                changed = true;
                self.needs_update = true;
            } else {
                // Orbit
                let orbit_speed = 0.01;
                self.camera
                    .orbit(delta.x * orbit_speed, -delta.y * orbit_speed);
                changed = true;
                self.needs_update = true;
            }
        }

        // Handle scroll for zoom
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            self.camera.zoom(-scroll * 0.01);
            changed = true;
            self.needs_update = true;
        }

        // Render simplified 3D preview
        self.render_preview(&painter, response.rect);

        // Show preview info overlay
        let builder = egui::UiBuilder::new().max_rect(response.rect);
        ui.scope_builder(builder, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(false),
                |ui| {
                    ui.group(|ui| {
                        ui.set_width(200.0);
                        ui.label(
                            egui::RichText::new("Camera Controls")
                                .color(egui::Color32::WHITE)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("Drag: Orbit")
                                .color(egui::Color32::LIGHT_GRAY)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("Shift+Drag: Pan")
                                .color(egui::Color32::LIGHT_GRAY)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("Scroll: Zoom")
                                .color(egui::Color32::LIGHT_GRAY)
                                .small(),
                        );
                    });
                },
            );
        });

        changed
    }

    /// Render the 3D preview (simplified version for Phase 3)
    ///
    /// This renders a basic wireframe representation of the creature.
    /// Future phases will integrate full Bevy rendering.
    fn render_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let creature_guard = self.creature.lock();
        if creature_guard.is_err() {
            return;
        }

        let creature_opt = creature_guard.unwrap();
        if creature_opt.is_none() {
            // Draw "No creature" text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No creature loaded",
                egui::FontId::proportional(16.0),
                egui::Color32::GRAY,
            );
            return;
        }

        let creature = creature_opt.as_ref().unwrap();

        // Draw grid if enabled
        if self.options.show_grid {
            self.draw_grid(painter, rect);
        }

        // Draw axes if enabled
        if self.options.show_axes {
            self.draw_axes(painter, rect);
        }

        // Draw creature meshes (simplified wireframe)
        for (mesh_idx, mesh) in creature.meshes.iter().enumerate() {
            let transform = creature
                .mesh_transforms
                .get(mesh_idx)
                .cloned()
                .unwrap_or_default();

            self.draw_mesh_wireframe(painter, rect, mesh, &transform, creature.scale);
        }

        // Draw mesh count info
        let info_text = format!(
            "{} - {} meshes, {} total vertices",
            creature.name,
            creature.meshes.len(),
            creature
                .meshes
                .iter()
                .map(|m| m.vertices.len())
                .sum::<usize>()
        );

        painter.text(
            egui::pos2(rect.min.x + 10.0, rect.max.y - 20.0),
            egui::Align2::LEFT_BOTTOM,
            info_text,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }

    /// Draw grid helper
    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 128);
        let center = rect.center();
        let spacing = 30.0;

        // Horizontal lines
        for i in -5..=5 {
            let y = center.y + i as f32 * spacing;
            painter.line_segment(
                [
                    egui::pos2(rect.min.x + 20.0, y),
                    egui::pos2(rect.max.x - 20.0, y),
                ],
                egui::Stroke::new(1.0, grid_color),
            );
        }

        // Vertical lines
        for i in -5..=5 {
            let x = center.x + i as f32 * spacing;
            painter.line_segment(
                [
                    egui::pos2(x, rect.min.y + 20.0),
                    egui::pos2(x, rect.max.y - 20.0),
                ],
                egui::Stroke::new(1.0, grid_color),
            );
        }
    }

    /// Draw axis helpers (X=red, Y=green, Z=blue)
    fn draw_axes(&self, painter: &egui::Painter, rect: egui::Rect) {
        let origin = egui::pos2(rect.min.x + 50.0, rect.max.y - 50.0);
        let axis_length = 30.0;

        // Project 3D axes to 2D screen space (simplified orthographic)
        let x_end = egui::pos2(origin.x + axis_length, origin.y);
        let y_end = egui::pos2(origin.x, origin.y - axis_length);
        let z_end = egui::pos2(origin.x - axis_length * 0.7, origin.y + axis_length * 0.7);

        // Draw axes
        painter.line_segment([origin, x_end], egui::Stroke::new(2.0, egui::Color32::RED));
        painter.line_segment(
            [origin, y_end],
            egui::Stroke::new(2.0, egui::Color32::GREEN),
        );
        painter.line_segment([origin, z_end], egui::Stroke::new(2.0, egui::Color32::BLUE));

        // Draw labels
        painter.text(
            x_end + egui::vec2(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "X",
            egui::FontId::proportional(12.0),
            egui::Color32::RED,
        );
        painter.text(
            y_end + egui::vec2(0.0, -5.0),
            egui::Align2::CENTER_BOTTOM,
            "Y",
            egui::FontId::proportional(12.0),
            egui::Color32::GREEN,
        );
        painter.text(
            z_end + egui::vec2(-5.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Z",
            egui::FontId::proportional(12.0),
            egui::Color32::BLUE,
        );
    }

    /// Draw mesh as wireframe (simplified 3D projection)
    /// Draw mesh as wireframe (simplified 3D projection)
    fn draw_mesh_wireframe(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        mesh: &MeshDefinition,
        transform: &MeshTransform,
        global_scale: f32,
    ) {
        // Simple orthographic-ish projection for Phase 3
        // This is a placeholder - Phase 5 will use proper 3D rendering

        let center = rect.center();
        let scale = 50.0 * global_scale;

        // Project vertices to 2D screen space
        let projected: Vec<egui::Pos2> = mesh
            .vertices
            .iter()
            .map(|v| {
                // Apply mesh transform (simplified)
                let x = v[0] * transform.scale[0] + transform.translation[0];
                let y = v[1] * transform.scale[1] + transform.translation[1];
                let z = v[2] * transform.scale[2] + transform.translation[2];

                // Simple isometric projection
                let screen_x = center.x + (x - z * 0.5) * scale;
                let screen_y = center.y - (y + z * 0.5) * scale;

                egui::pos2(screen_x, screen_y)
            })
            .collect();

        // Draw mesh color as a filled polygon (if not too many triangles)
        let mesh_color = egui::Color32::from_rgba_premultiplied(
            (mesh.color[0] * 255.0) as u8,
            (mesh.color[1] * 255.0) as u8,
            (mesh.color[2] * 255.0) as u8,
            (mesh.color[3] * 128.0) as u8, // Semi-transparent
        );

        // Draw triangles
        for tri in mesh.indices.chunks(3) {
            if tri.len() == 3 {
                let i0 = tri[0] as usize;
                let i1 = tri[1] as usize;
                let i2 = tri[2] as usize;

                if i0 < projected.len() && i1 < projected.len() && i2 < projected.len() {
                    // Fill triangle
                    let points = vec![projected[i0], projected[i1], projected[i2]];
                    painter.add(egui::Shape::convex_polygon(
                        points.clone(),
                        mesh_color,
                        egui::Stroke::NONE,
                    ));

                    // Draw wireframe edges
                    if self.options.show_wireframe {
                        painter.line_segment(
                            [projected[i0], projected[i1]],
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );
                        painter.line_segment(
                            [projected[i1], projected[i2]],
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );
                        painter.line_segment(
                            [projected[i2], projected[i0]],
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );
                    }
                }
            }
        }
    }

    /// Show preview options UI
    ///
    /// Displays controls for toggling grid, axes, wireframe, etc.
    pub fn show_options(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.group(|ui| {
            ui.label(egui::RichText::new("Preview Options").strong());

            if ui
                .checkbox(&mut self.options.show_grid, "Show Grid")
                .changed()
            {
                changed = true;
                self.needs_update = true;
            }

            if ui
                .checkbox(&mut self.options.show_axes, "Show Axes")
                .changed()
            {
                changed = true;
                self.needs_update = true;
            }

            if ui
                .checkbox(&mut self.options.show_wireframe, "Show Wireframe")
                .changed()
            {
                changed = true;
                self.needs_update = true;
            }

            ui.separator();

            ui.label("Background Color");
            let mut bg_color = egui::Color32::from_rgba_premultiplied(
                (self.options.background_color[0] * 255.0) as u8,
                (self.options.background_color[1] * 255.0) as u8,
                (self.options.background_color[2] * 255.0) as u8,
                255,
            );

            if ui.color_edit_button_srgba(&mut bg_color).changed() {
                self.options.background_color = [
                    bg_color.r() as f32 / 255.0,
                    bg_color.g() as f32 / 255.0,
                    bg_color.b() as f32 / 255.0,
                    1.0,
                ];
                changed = true;
                self.needs_update = true;
            }

            ui.separator();

            if ui.button("Reset Camera").clicked() {
                self.reset_camera();
                changed = true;
            }
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_renderer_new() {
        let renderer = PreviewRenderer::new();
        assert!(renderer.get_creature().is_none());
        assert!(renderer.needs_update);
    }

    #[test]
    fn test_update_creature() {
        let mut renderer = PreviewRenderer::new();
        let creature = CreatureDefinition {
            id: 1,
            name: "Test Creature".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };

        renderer.update_creature(Some(creature.clone()));
        let result = renderer.get_creature();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Test Creature");
    }

    #[test]
    fn test_camera_state_position() {
        let camera = CameraState::default();
        let pos = camera.position();
        assert_eq!(pos.len(), 3);
        // Position should be non-zero given default distance
        assert!(pos[0].abs() > 0.0 || pos[1].abs() > 0.0 || pos[2].abs() > 0.0);
    }

    #[test]
    fn test_camera_orbit() {
        let mut camera = CameraState::default();
        let initial_azimuth = camera.azimuth;

        camera.orbit(0.1, 0.0);
        assert!((camera.azimuth - (initial_azimuth + 0.1)).abs() < 0.001);
    }

    #[test]
    fn test_camera_zoom() {
        let mut camera = CameraState::default();
        let initial_distance = camera.distance;

        camera.zoom(-1.0);
        assert!((camera.distance - (initial_distance - 1.0)).abs() < 0.001);

        // Test minimum zoom
        camera.distance = 0.5;
        camera.zoom(-1.0);
        assert!(camera.distance >= 0.1);
    }

    #[test]
    fn test_camera_pan() {
        let mut camera = CameraState::default();
        let initial_target = camera.target;

        camera.pan(1.0, 0.5);
        assert_ne!(camera.target, initial_target);
    }

    #[test]
    fn test_camera_reset() {
        let mut camera = CameraState::default();
        camera.orbit(1.0, 0.5);
        camera.zoom(2.0);

        camera.reset();
        assert_eq!(camera.distance, CameraState::default().distance);
        assert_eq!(camera.azimuth, CameraState::default().azimuth);
    }

    #[test]
    fn test_preview_options_default() {
        let options = PreviewOptions::default();
        assert!(options.show_grid);
        assert!(options.show_axes);
        assert!(!options.show_wireframe);
        assert!(!options.show_normals);
        assert!(options.enable_lighting);
    }

    #[test]
    fn test_camera_elevation_clamp() {
        let mut camera = CameraState::default();

        // Try to orbit beyond limits
        camera.orbit(0.0, 10.0); // Large positive elevation
        assert!(camera.elevation <= 1.57);

        camera.orbit(0.0, -20.0); // Large negative elevation
        assert!(camera.elevation >= -1.57);
    }

    #[test]
    fn test_reset_camera() {
        let mut renderer = PreviewRenderer::new();
        renderer.camera.orbit(1.0, 0.5);
        renderer.camera.zoom(2.0);

        renderer.reset_camera();
        assert_eq!(renderer.camera.distance, CameraState::default().distance);
        assert!(renderer.needs_update);
    }

    #[test]
    fn test_preview_renderer_creature_clear() {
        let mut renderer = PreviewRenderer::new();
        let creature = CreatureDefinition {
            id: 1,
            name: "Test".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };

        renderer.update_creature(Some(creature));
        assert!(renderer.get_creature().is_some());

        renderer.update_creature(None);
        assert!(renderer.get_creature().is_none());
    }
}
