// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Preview Features - Phase 5.2
//!
//! Provides enhanced preview rendering features for the creature editor.
//! Supports:
//! - Grid rendering with configurable size and color
//! - Axis indicators (X, Y, Z)
//! - Lighting controls (ambient, directional, point lights)
//! - Wireframe overlay
//! - Bounding box display
//! - Statistics overlay (vertex count, triangle count, etc.)
//! - Camera controls and presets

use serde::{Deserialize, Serialize};

/// Preview display options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewOptions {
    /// Show grid
    pub show_grid: bool,
    /// Show axis indicators
    pub show_axes: bool,
    /// Show wireframe overlay
    pub show_wireframe: bool,
    /// Show vertex normals
    pub show_normals: bool,
    /// Show bounding box
    pub show_bounding_box: bool,
    /// Show statistics overlay
    pub show_statistics: bool,
    /// Enable lighting
    pub enable_lighting: bool,
    /// Background color [R, G, B, A]
    pub background_color: [f32; 4],
    /// Wireframe color [R, G, B, A]
    pub wireframe_color: [f32; 4],
    /// Normal vector color [R, G, B, A]
    pub normal_color: [f32; 4],
    /// Normal vector length
    pub normal_length: f32,
    /// Selected mesh highlight color [R, G, B, A]
    pub selection_color: [f32; 4],
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_axes: true,
            show_wireframe: false,
            show_normals: false,
            show_bounding_box: false,
            show_statistics: true,
            enable_lighting: true,
            background_color: [0.2, 0.2, 0.25, 1.0],
            wireframe_color: [0.0, 1.0, 0.0, 1.0],
            normal_color: [0.0, 0.5, 1.0, 1.0],
            normal_length: 0.1,
            selection_color: [1.0, 0.6, 0.0, 1.0],
        }
    }
}

impl PreviewOptions {
    /// Create new preview options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle grid display
    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    /// Toggle wireframe display
    pub fn toggle_wireframe(&mut self) {
        self.show_wireframe = !self.show_wireframe;
    }

    /// Toggle normals display
    pub fn toggle_normals(&mut self) {
        self.show_normals = !self.show_normals;
    }

    /// Toggle bounding box display
    pub fn toggle_bounding_box(&mut self) {
        self.show_bounding_box = !self.show_bounding_box;
    }

    /// Toggle statistics display
    pub fn toggle_statistics(&mut self) {
        self.show_statistics = !self.show_statistics;
    }

    /// Toggle axes display
    pub fn toggle_axes(&mut self) {
        self.show_axes = !self.show_axes;
    }

    /// Toggle lighting
    pub fn toggle_lighting(&mut self) {
        self.enable_lighting = !self.enable_lighting;
    }
}

/// Grid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Grid size (number of lines in each direction)
    pub size: u32,
    /// Spacing between grid lines
    pub spacing: f32,
    /// Grid line color [R, G, B, A]
    pub line_color: [f32; 4],
    /// Major grid line color (every N lines) [R, G, B, A]
    pub major_line_color: [f32; 4],
    /// Major line interval
    pub major_line_interval: u32,
    /// Grid plane (XY, XZ, or YZ)
    pub plane: GridPlane,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: 20,
            spacing: 1.0,
            line_color: [0.3, 0.3, 0.35, 1.0],
            major_line_color: [0.4, 0.4, 0.45, 1.0],
            major_line_interval: 5,
            plane: GridPlane::XZ,
        }
    }
}

/// Grid plane orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridPlane {
    /// XY plane (Z = 0)
    XY,
    /// XZ plane (Y = 0)
    XZ,
    /// YZ plane (X = 0)
    YZ,
}

/// Axis indicator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    /// Axis length
    pub length: f32,
    /// Axis line width
    pub line_width: f32,
    /// X axis color [R, G, B, A]
    pub x_color: [f32; 4],
    /// Y axis color [R, G, B, A]
    pub y_color: [f32; 4],
    /// Z axis color [R, G, B, A]
    pub z_color: [f32; 4],
    /// Show axis labels
    pub show_labels: bool,
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            length: 1.0,
            line_width: 2.0,
            x_color: [1.0, 0.0, 0.0, 1.0], // Red
            y_color: [0.0, 1.0, 0.0, 1.0], // Green
            z_color: [0.0, 0.0, 1.0, 1.0], // Blue
            show_labels: true,
        }
    }
}

/// Named lighting presets for the preview panel dropdown.
///
/// # Examples
///
/// ```
/// use campaign_builder::preview_features::{LightingConfig, LightingPreset};
///
/// let mut config = LightingConfig::default();
/// config.apply_preset(LightingPreset::Night);
/// assert!(config.ambient_intensity < 0.2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingPreset {
    /// Bright outdoor daylight – high ambient and strong directional sun.
    Day,
    /// Dark environment with dim ambient and a cool moonlight direction.
    Night,
    /// Underground dungeon – very low ambient, warm torch-like point light.
    Dungeon,
    /// Even studio lighting – moderate ambient, two directional fill lights.
    Studio,
}

impl LightingPreset {
    /// Human-readable label used in the UI dropdown.
    pub fn display_name(&self) -> &'static str {
        match self {
            LightingPreset::Day => "Day",
            LightingPreset::Night => "Night",
            LightingPreset::Dungeon => "Dungeon",
            LightingPreset::Studio => "Studio",
        }
    }

    /// Returns all available presets in display order.
    pub fn all() -> &'static [LightingPreset] {
        &[
            LightingPreset::Day,
            LightingPreset::Night,
            LightingPreset::Dungeon,
            LightingPreset::Studio,
        ]
    }
}

impl std::fmt::Display for LightingPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Lighting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingConfig {
    /// Ambient light color [R, G, B]
    pub ambient_color: [f32; 3],
    /// Ambient light intensity
    pub ambient_intensity: f32,
    /// Directional lights
    pub directional_lights: Vec<DirectionalLight>,
    /// Point lights
    pub point_lights: Vec<PointLight>,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_intensity: 0.3,
            directional_lights: vec![DirectionalLight {
                direction: [-0.5, -1.0, -0.5],
                color: [1.0, 1.0, 1.0],
                intensity: 0.7,
            }],
            point_lights: vec![],
        }
    }
}

impl LightingConfig {
    /// Replace this config with the values from a named [`LightingPreset`].
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::preview_features::{LightingConfig, LightingPreset};
    ///
    /// let mut config = LightingConfig::default();
    /// config.apply_preset(LightingPreset::Studio);
    /// assert_eq!(config.directional_lights.len(), 2);
    /// ```
    pub fn apply_preset(&mut self, preset: LightingPreset) {
        *self = Self::from_preset(preset);
    }

    /// Construct a [`LightingConfig`] from a named [`LightingPreset`].
    pub fn from_preset(preset: LightingPreset) -> Self {
        match preset {
            LightingPreset::Day => Self {
                ambient_color: [1.0, 1.0, 0.95],
                ambient_intensity: 0.5,
                directional_lights: vec![DirectionalLight {
                    direction: [-0.3, -1.0, -0.4],
                    color: [1.0, 0.98, 0.9],
                    intensity: 1.0,
                }],
                point_lights: vec![],
            },
            LightingPreset::Night => Self {
                ambient_color: [0.1, 0.1, 0.2],
                ambient_intensity: 0.1,
                directional_lights: vec![DirectionalLight {
                    direction: [0.2, -0.8, 0.3],
                    color: [0.4, 0.45, 0.6],
                    intensity: 0.3,
                }],
                point_lights: vec![],
            },
            LightingPreset::Dungeon => Self {
                ambient_color: [0.05, 0.03, 0.02],
                ambient_intensity: 0.05,
                directional_lights: vec![],
                point_lights: vec![PointLight {
                    position: [0.0, 3.0, 0.0],
                    color: [1.0, 0.6, 0.2],
                    intensity: 2.0,
                    constant: 1.0,
                    linear: 0.22,
                    quadratic: 0.20,
                }],
            },
            LightingPreset::Studio => Self {
                ambient_color: [1.0, 1.0, 1.0],
                ambient_intensity: 0.4,
                directional_lights: vec![
                    DirectionalLight {
                        direction: [-0.5, -1.0, -0.5],
                        color: [1.0, 1.0, 1.0],
                        intensity: 0.7,
                    },
                    DirectionalLight {
                        direction: [0.5, -0.5, 0.5],
                        color: [0.8, 0.85, 1.0],
                        intensity: 0.4,
                    },
                ],
                point_lights: vec![],
            },
        }
    }
}

/// Directional light
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalLight {
    /// Light direction [X, Y, Z]
    pub direction: [f32; 3],
    /// Light color [R, G, B]
    pub color: [f32; 3],
    /// Light intensity
    pub intensity: f32,
}

/// Point light
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointLight {
    /// Light position [X, Y, Z]
    pub position: [f32; 3],
    /// Light color [R, G, B]
    pub color: [f32; 3],
    /// Light intensity
    pub intensity: f32,
    /// Attenuation constant
    pub constant: f32,
    /// Attenuation linear factor
    pub linear: f32,
    /// Attenuation quadratic factor
    pub quadratic: f32,
}

/// Camera configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Camera position [X, Y, Z]
    pub position: [f32; 3],
    /// Camera target/look-at point [X, Y, Z]
    pub target: [f32; 3],
    /// Camera up vector [X, Y, Z]
    pub up: [f32; 3],
    /// Field of view in degrees
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Camera movement speed
    pub move_speed: f32,
    /// Camera rotation speed
    pub rotation_speed: f32,
    /// Camera zoom speed
    pub zoom_speed: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: [5.0, 5.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 45.0,
            near: 0.1,
            far: 1000.0,
            move_speed: 1.0,
            rotation_speed: 0.5,
            zoom_speed: 0.1,
        }
    }
}

impl CameraConfig {
    /// Reset camera to default position
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Focus camera on a point
    pub fn focus_on(&mut self, target: [f32; 3]) {
        self.target = target;
    }

    /// Set camera to orthographic front view
    pub fn front_view(&mut self) {
        self.position = [0.0, 0.0, 10.0];
        self.target = [0.0, 0.0, 0.0];
        self.up = [0.0, 1.0, 0.0];
    }

    /// Set camera to orthographic top view
    pub fn top_view(&mut self) {
        self.position = [0.0, 10.0, 0.0];
        self.target = [0.0, 0.0, 0.0];
        self.up = [0.0, 0.0, -1.0];
    }

    /// Set camera to orthographic right view
    pub fn right_view(&mut self) {
        self.position = [10.0, 0.0, 0.0];
        self.target = [0.0, 0.0, 0.0];
        self.up = [0.0, 1.0, 0.0];
    }

    /// Set camera to isometric view
    pub fn isometric_view(&mut self) {
        self.position = [5.0, 5.0, 5.0];
        self.target = [0.0, 0.0, 0.0];
        self.up = [0.0, 1.0, 0.0];
    }
}

/// Preview statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreviewStatistics {
    /// Total number of meshes
    pub mesh_count: usize,
    /// Total number of vertices
    pub vertex_count: usize,
    /// Total number of triangles
    pub triangle_count: usize,
    /// Number of selected meshes
    pub selected_meshes: usize,
    /// Number of selected vertices
    pub selected_vertices: usize,
    /// Bounding box min [X, Y, Z]
    pub bounds_min: [f32; 3],
    /// Bounding box max [X, Y, Z]
    pub bounds_max: [f32; 3],
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
    /// Frames per second
    pub fps: f32,
}

impl PreviewStatistics {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get bounding box size
    pub fn bounds_size(&self) -> [f32; 3] {
        [
            self.bounds_max[0] - self.bounds_min[0],
            self.bounds_max[1] - self.bounds_min[1],
            self.bounds_max[2] - self.bounds_min[2],
        ]
    }

    /// Get bounding box center
    pub fn bounds_center(&self) -> [f32; 3] {
        [
            (self.bounds_min[0] + self.bounds_max[0]) / 2.0,
            (self.bounds_min[1] + self.bounds_max[1]) / 2.0,
            (self.bounds_min[2] + self.bounds_max[2]) / 2.0,
        ]
    }

    /// Format statistics as string
    pub fn format(&self) -> String {
        format!(
            "Meshes: {} | Vertices: {} | Triangles: {} | FPS: {:.1}",
            self.mesh_count, self.vertex_count, self.triangle_count, self.fps
        )
    }
}

/// Preview state manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewState {
    /// Display options
    pub options: PreviewOptions,
    /// Grid configuration
    pub grid: GridConfig,
    /// Axis configuration
    pub axes: AxisConfig,
    /// Lighting configuration
    pub lighting: LightingConfig,
    /// Camera configuration
    pub camera: CameraConfig,
    /// Current statistics
    pub statistics: PreviewStatistics,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewState {
    /// Create new preview state with defaults
    pub fn new() -> Self {
        Self {
            options: PreviewOptions::default(),
            grid: GridConfig::default(),
            axes: AxisConfig::default(),
            lighting: LightingConfig::default(),
            camera: CameraConfig::default(),
            statistics: PreviewStatistics::default(),
        }
    }

    /// Reset all to defaults
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Reset camera only
    pub fn reset_camera(&mut self) {
        self.camera.reset();
    }

    /// Update statistics
    pub fn update_statistics(&mut self, stats: PreviewStatistics) {
        self.statistics = stats;
    }

    /// Get current frame time
    pub fn frame_time(&self) -> f32 {
        self.statistics.frame_time_ms
    }

    /// Get current FPS
    pub fn fps(&self) -> f32 {
        self.statistics.fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_options_default() {
        let options = PreviewOptions::default();
        assert!(options.show_grid);
        assert!(options.show_axes);
        assert!(!options.show_wireframe);
        assert!(options.enable_lighting);
    }

    #[test]
    fn test_preview_options_toggles() {
        let mut options = PreviewOptions::default();

        options.toggle_grid();
        assert!(!options.show_grid);

        options.toggle_wireframe();
        assert!(options.show_wireframe);

        options.toggle_normals();
        assert!(options.show_normals);
    }

    #[test]
    fn test_grid_config_default() {
        let grid = GridConfig::default();
        assert_eq!(grid.size, 20);
        assert_eq!(grid.spacing, 1.0);
        assert_eq!(grid.plane, GridPlane::XZ);
    }

    #[test]
    fn test_axis_config_default() {
        let axes = AxisConfig::default();
        assert_eq!(axes.length, 1.0);
        assert!(axes.show_labels);
        assert_eq!(axes.x_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_lighting_config_default() {
        let lighting = LightingConfig::default();
        assert_eq!(lighting.ambient_intensity, 0.3);
        assert_eq!(lighting.directional_lights.len(), 1);
        assert_eq!(lighting.point_lights.len(), 0);
    }

    #[test]
    fn test_lighting_preset_display_names() {
        assert_eq!(LightingPreset::Day.display_name(), "Day");
        assert_eq!(LightingPreset::Night.display_name(), "Night");
        assert_eq!(LightingPreset::Dungeon.display_name(), "Dungeon");
        assert_eq!(LightingPreset::Studio.display_name(), "Studio");
    }

    #[test]
    fn test_lighting_preset_all_has_four_entries() {
        assert_eq!(LightingPreset::all().len(), 4);
    }

    #[test]
    fn test_lighting_preset_day() {
        let config = LightingConfig::from_preset(LightingPreset::Day);
        assert!(config.ambient_intensity >= 0.4);
        assert_eq!(config.directional_lights.len(), 1);
        assert_eq!(config.point_lights.len(), 0);
    }

    #[test]
    fn test_lighting_preset_night() {
        let config = LightingConfig::from_preset(LightingPreset::Night);
        assert!(config.ambient_intensity < 0.2);
        assert_eq!(config.directional_lights.len(), 1);
    }

    #[test]
    fn test_lighting_preset_dungeon() {
        let config = LightingConfig::from_preset(LightingPreset::Dungeon);
        assert!(config.ambient_intensity < 0.1);
        assert_eq!(config.directional_lights.len(), 0);
        assert_eq!(config.point_lights.len(), 1);
        assert_eq!(config.point_lights[0].color, [1.0, 0.6, 0.2]);
    }

    #[test]
    fn test_lighting_preset_studio() {
        let config = LightingConfig::from_preset(LightingPreset::Studio);
        assert_eq!(config.directional_lights.len(), 2);
        assert_eq!(config.point_lights.len(), 0);
    }

    #[test]
    fn test_lighting_apply_preset_mutates_in_place() {
        let mut config = LightingConfig::default();
        config.apply_preset(LightingPreset::Dungeon);
        assert_eq!(config.point_lights.len(), 1);
        assert_eq!(config.directional_lights.len(), 0);
    }

    #[test]
    fn test_lighting_preset_display() {
        assert_eq!(format!("{}", LightingPreset::Day), "Day");
        assert_eq!(format!("{}", LightingPreset::Studio), "Studio");
    }

    #[test]
    fn test_camera_config_default() {
        let camera = CameraConfig::default();
        assert_eq!(camera.position, [5.0, 5.0, 5.0]);
        assert_eq!(camera.target, [0.0, 0.0, 0.0]);
        assert_eq!(camera.fov, 45.0);
    }

    #[test]
    fn test_camera_reset() {
        let mut camera = CameraConfig {
            position: [10.0, 10.0, 10.0],
            ..CameraConfig::default()
        };

        camera.reset();
        assert_eq!(camera.position, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_camera_views() {
        let mut camera = CameraConfig::default();

        camera.front_view();
        assert_eq!(camera.position, [0.0, 0.0, 10.0]);

        camera.top_view();
        assert_eq!(camera.position, [0.0, 10.0, 0.0]);

        camera.right_view();
        assert_eq!(camera.position, [10.0, 0.0, 0.0]);

        camera.isometric_view();
        assert_eq!(camera.position, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_camera_focus() {
        let mut camera = CameraConfig::default();
        camera.focus_on([1.0, 2.0, 3.0]);
        assert_eq!(camera.target, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_preview_statistics() {
        let mut stats = PreviewStatistics::new();
        stats.mesh_count = 5;
        stats.vertex_count = 100;
        stats.triangle_count = 50;
        stats.fps = 60.0;

        assert_eq!(stats.mesh_count, 5);
        assert!(stats.format().contains("Meshes: 5"));
    }

    #[test]
    fn test_preview_statistics_bounds() {
        let mut stats = PreviewStatistics::new();
        stats.bounds_min = [-1.0, -2.0, -3.0];
        stats.bounds_max = [1.0, 2.0, 3.0];

        let size = stats.bounds_size();
        assert_eq!(size, [2.0, 4.0, 6.0]);

        let center = stats.bounds_center();
        assert_eq!(center, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_preview_state_default() {
        let state = PreviewState::new();
        assert!(state.options.show_grid);
        assert_eq!(state.camera.position, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_preview_state_reset() {
        let mut state = PreviewState::new();
        state.options.show_grid = false;
        state.camera.position = [10.0, 10.0, 10.0];

        state.reset();
        assert!(state.options.show_grid);
        assert_eq!(state.camera.position, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_preview_state_reset_camera_only() {
        let mut state = PreviewState::new();
        state.options.show_grid = false;
        state.camera.position = [10.0, 10.0, 10.0];

        state.reset_camera();
        assert!(!state.options.show_grid); // Options unchanged
        assert_eq!(state.camera.position, [5.0, 5.0, 5.0]); // Camera reset
    }

    #[test]
    fn test_preview_state_update_statistics() {
        let mut state = PreviewState::new();
        let mut stats = PreviewStatistics::new();
        stats.mesh_count = 10;
        stats.fps = 120.0;

        state.update_statistics(stats);
        assert_eq!(state.statistics.mesh_count, 10);
        assert_eq!(state.fps(), 120.0);
    }
}
