// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game Configuration Module
//!
//! This module provides configuration structures for game engine settings,
//! allowing campaigns to customize graphics, audio, controls, and camera behavior.
//!
//! # Overview
//!
//! The configuration system uses a hierarchical structure with `GameConfig` as the root,
//! containing specialized configuration for different subsystems:
//!
//! - `GraphicsConfig`: Resolution, fullscreen, MSAA, shadows
//! - `AudioConfig`: Volume levels and audio enable/disable
//! - `ControlsConfig`: Key bindings and input settings
//! - `CameraConfig`: Camera mode, FOV, clipping planes, lighting
//! - `GameLogConfig`: In-game log panel visibility, toggle key, sizing, opacity,
//!   and default category filters
//!
//! # Usage
//!
//! ```no_run
//! use antares::sdk::game_config::GameConfig;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load configuration from file, falling back to defaults
//! let config = GameConfig::load_or_default(Path::new("campaigns/tutorial/config.ron"))?;
//!
//! // Use default configuration
//! let default_config = GameConfig::default();
//!
//! // Validate configuration
//! config.validate()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Data Format
//!
//! Configuration files use RON (Rusty Object Notation) format. Example:
//!
//! ```ron
//! (
//!     graphics: (
//!         resolution: (1280, 720),
//!         fullscreen: false,
//!         vsync: true,
//!         msaa_samples: 4,
//!         shadow_quality: Medium,
//!     ),
//!     audio: (
//!         master_volume: 0.8,
//!         music_volume: 0.6,
//!         sfx_volume: 1.0,
//!         ambient_volume: 0.5,
//!         enable_audio: true,
//!     ),
//!     controls: (
//!         move_forward: ["W", "ArrowUp"],
//!         move_back: ["S", "ArrowDown"],
//!         turn_left: ["A", "ArrowLeft"],
//!         turn_right: ["D", "ArrowRight"],
//!         interact: ["Space", "E"],
//!         menu: ["Escape"],
//!         automap: ["M"],
//!         movement_cooldown: 0.2,
//!     ),
//!     camera: (
//!         mode: FirstPerson,
//!         eye_height: 0.6,
//!         fov: 70.0,
//!         near_clip: 0.1,
//!         far_clip: 1000.0,
//!         smooth_rotation: false,
//!         rotation_speed: 180.0,
//!         light_height: 5.0,
//!         light_intensity: 2000000.0,
//!         light_range: 60.0,
//!         shadows_enabled: true,
//!     ),
//!     game_log: (
//!         max_entries: 200,
//!         visible_by_default: true,
//!         toggle_key: "L",
//!         show_timestamps: false,
//!         panel_width_px: 300.0,
//!         panel_height_px: 200.0,
//!         panel_opacity: 0.88,
//!         default_enabled_categories: ["Combat", "Dialogue", "Item", "Exploration", "System"],
//!     ),
//! )
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Game configuration error types
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read configuration file
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    /// Failed to parse RON configuration
    #[error("Failed to parse config file: {0}")]
    ParseError(String),

    /// Configuration validation failed
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

/// Root game configuration structure
///
/// Contains all configuration subsystems for the game engine.
///
/// # Examples
///
/// ```
/// use antares::sdk::game_config::GameConfig;
///
/// let config = GameConfig::default();
/// assert_eq!(config.graphics.resolution, (1280, 720));
/// assert_eq!(config.audio.master_volume, 0.8);
/// assert_eq!(config.rest.full_rest_hours, 12);
/// assert_eq!(config.game_log.max_entries, 200);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameConfig {
    /// Graphics and rendering configuration
    pub graphics: GraphicsConfig,

    /// Audio configuration
    pub audio: AudioConfig,

    /// Input controls configuration
    pub controls: ControlsConfig,

    /// Camera configuration
    pub camera: CameraConfig,

    /// Rest system configuration
    #[serde(default)]
    pub rest: RestConfig,

    /// Game log panel configuration
    #[serde(default)]
    pub game_log: GameLogConfig,
}

impl GameConfig {
    /// Load configuration from a RON file, or return default if file doesn't exist
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config.ron file
    ///
    /// # Returns
    ///
    /// Returns the loaded configuration, or default configuration with a warning if file not found
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ReadError` if file exists but cannot be read
    /// Returns `ConfigError::ParseError` if file content is invalid RON
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::game_config::GameConfig;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = GameConfig::load_or_default(Path::new("campaigns/tutorial/config.ron"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            tracing::warn!(
                "Config file not found at {:?}, using default configuration",
                path
            );
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(format!("{}: {}", path.display(), e)))?;

        let config: GameConfig = ron::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(format!("{}: {}", path.display(), e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate all configuration values
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all configuration is valid
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if any validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::game_config::GameConfig;
    ///
    /// let config = GameConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.graphics.validate()?;
        self.audio.validate()?;
        self.controls.validate()?;
        self.camera.validate()?;
        self.rest.validate()?;
        self.game_log.validate()?;
        Ok(())
    }
}

/// Game log panel configuration
///
/// Controls runtime behavior of the in-game game log panel, including buffer
/// sizing, default visibility, toggle key binding, panel geometry, opacity, and
/// which categories are enabled by default.
///
/// # Examples
///
/// ```
/// use antares::sdk::game_config::GameLogConfig;
///
/// let cfg = GameLogConfig::default();
/// assert_eq!(cfg.max_entries, 200);
/// assert_eq!(cfg.toggle_key, "L");
/// assert!(cfg.visible_by_default);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameLogConfig {
    /// Maximum number of entries retained in the log ring buffer.
    pub max_entries: usize,

    /// Whether the game log panel is visible when a game starts.
    pub visible_by_default: bool,

    /// Toggle key used to show or hide the game log panel.
    pub toggle_key: String,

    /// Whether timestamps should be shown alongside log entries.
    pub show_timestamps: bool,

    /// Panel width in pixels.
    pub panel_width_px: f32,

    /// Panel height in pixels.
    pub panel_height_px: f32,

    /// Panel background opacity in the range `0.0..=1.0`.
    pub panel_opacity: f32,

    /// Category names enabled by default in the filter bar.
    pub default_enabled_categories: Vec<String>,
}

impl Default for GameLogConfig {
    fn default() -> Self {
        Self {
            max_entries: 200,
            visible_by_default: true,
            toggle_key: "L".to_string(),
            show_timestamps: false,
            panel_width_px: 300.0,
            panel_height_px: 200.0,
            panel_opacity: 0.88,
            default_enabled_categories: vec![
                "Combat".to_string(),
                "Dialogue".to_string(),
                "Item".to_string(),
                "Exploration".to_string(),
                "System".to_string(),
            ],
        }
    }
}

impl GameLogConfig {
    /// Validate the game log configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if:
    /// - `max_entries` is zero
    /// - `toggle_key` is empty
    /// - `panel_width_px` or `panel_height_px` are non-positive
    /// - `panel_opacity` is outside `0.0..=1.0`
    /// - `default_enabled_categories` is empty
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_entries == 0 {
            return Err(ConfigError::ValidationError(
                "game_log.max_entries must be at least 1".to_string(),
            ));
        }

        if self.toggle_key.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "game_log.toggle_key must not be empty".to_string(),
            ));
        }

        if self.panel_width_px <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "game_log.panel_width_px must be positive, got {}",
                self.panel_width_px
            )));
        }

        if self.panel_height_px <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "game_log.panel_height_px must be positive, got {}",
                self.panel_height_px
            )));
        }

        if !(0.0..=1.0).contains(&self.panel_opacity) {
            return Err(ConfigError::ValidationError(format!(
                "game_log.panel_opacity must be in range 0.0-1.0, got {}",
                self.panel_opacity
            )));
        }

        if self.default_enabled_categories.is_empty() {
            return Err(ConfigError::ValidationError(
                "game_log.default_enabled_categories must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Graphics and rendering configuration
///
/// Controls visual quality settings including resolution, fullscreen mode,
/// anti-aliasing, and shadow quality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphicsConfig {
    /// Window resolution (width, height)
    pub resolution: (u32, u32),

    /// Fullscreen mode enabled
    pub fullscreen: bool,

    /// VSync enabled
    pub vsync: bool,

    /// MSAA sample count (must be power of 2: 1, 2, 4, 8)
    pub msaa_samples: u32,

    /// Shadow rendering quality
    pub shadow_quality: ShadowQuality,

    /// Show combat monster HP hover bars projected above monster visuals.
    #[serde(default = "default_show_combat_monster_hp_bars")]
    pub show_combat_monster_hp_bars: bool,

    /// Show the exploration mini map in the HUD.
    #[serde(default = "default_show_minimap")]
    pub show_minimap: bool,
}

fn default_show_combat_monster_hp_bars() -> bool {
    true
}

fn default_show_minimap() -> bool {
    true
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            resolution: (1280, 720),
            fullscreen: false,
            vsync: true,
            msaa_samples: 4,
            shadow_quality: ShadowQuality::Medium,
            show_combat_monster_hp_bars: true,
            show_minimap: true,
        }
    }
}

impl GraphicsConfig {
    /// Validate graphics configuration
    ///
    /// # Errors
    ///
    /// Returns error if resolution is zero or MSAA samples is not a power of 2
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.resolution.0 == 0 || self.resolution.1 == 0 {
            return Err(ConfigError::ValidationError(
                "Resolution width and height must be greater than 0".to_string(),
            ));
        }

        if self.msaa_samples > 0 && !self.msaa_samples.is_power_of_two() {
            return Err(ConfigError::ValidationError(format!(
                "MSAA samples must be a power of 2, got {}",
                self.msaa_samples
            )));
        }

        Ok(())
    }
}

/// Shadow quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShadowQuality {
    /// Low quality shadows
    Low,
    /// Medium quality shadows
    Medium,
    /// High quality shadows
    High,
    /// Ultra quality shadows
    Ultra,
}

/// Audio configuration
///
/// Controls volume levels for different audio channels and global audio enable.
/// All volume values should be in range 0.0-1.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfig {
    /// Master volume (0.0-1.0)
    pub master_volume: f32,

    /// Music volume (0.0-1.0)
    pub music_volume: f32,

    /// Sound effects volume (0.0-1.0)
    pub sfx_volume: f32,

    /// Ambient sound volume (0.0-1.0)
    pub ambient_volume: f32,

    /// Audio system enabled
    pub enable_audio: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 1.0,
            ambient_volume: 0.5,
            enable_audio: true,
        }
    }
}

impl AudioConfig {
    /// Validate audio configuration
    ///
    /// # Errors
    ///
    /// Returns error if any volume is outside 0.0-1.0 range
    pub fn validate(&self) -> Result<(), ConfigError> {
        let volumes = [
            ("master_volume", self.master_volume),
            ("music_volume", self.music_volume),
            ("sfx_volume", self.sfx_volume),
            ("ambient_volume", self.ambient_volume),
        ];

        for (name, volume) in volumes {
            if !(0.0..=1.0).contains(&volume) {
                return Err(ConfigError::ValidationError(format!(
                    "{} must be in range 0.0-1.0, got {}",
                    name, volume
                )));
            }
        }

        Ok(())
    }
}

/// Input controls configuration
///
/// Defines key bindings for game actions. Each action can have multiple keys.
/// Keys are represented as strings matching Bevy's KeyCode naming.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlsConfig {
    /// Keys for moving forward
    pub move_forward: Vec<String>,

    /// Keys for moving backward
    pub move_back: Vec<String>,

    /// Keys for turning left
    pub turn_left: Vec<String>,

    /// Keys for turning right
    pub turn_right: Vec<String>,

    /// Keys for interaction
    pub interact: Vec<String>,

    /// Keys for opening menu
    pub menu: Vec<String>,

    /// Keys for opening the inventory screen
    #[serde(default = "default_inventory_keys")]
    pub inventory: Vec<String>,

    /// Keys for initiating a party rest sequence
    #[serde(default = "default_rest_keys")]
    pub rest: Vec<String>,

    /// Keys for opening or closing the automap overlay
    #[serde(default = "default_automap_keys")]
    pub automap: Vec<String>,

    /// Movement cooldown in seconds (prevents double-moves)
    pub movement_cooldown: f32,
}

fn default_inventory_keys() -> Vec<String> {
    vec!["I".to_string()]
}

fn default_rest_keys() -> Vec<String> {
    vec!["R".to_string()]
}

fn default_automap_keys() -> Vec<String> {
    vec!["M".to_string()]
}

impl Default for ControlsConfig {
    fn default() -> Self {
        Self {
            move_forward: vec!["W".to_string(), "ArrowUp".to_string()],
            move_back: vec!["S".to_string(), "ArrowDown".to_string()],
            turn_left: vec!["A".to_string(), "ArrowLeft".to_string()],
            turn_right: vec!["D".to_string(), "ArrowRight".to_string()],
            interact: vec!["Space".to_string(), "E".to_string()],
            menu: vec!["Escape".to_string()],
            inventory: default_inventory_keys(),
            rest: default_rest_keys(),
            automap: default_automap_keys(),
            movement_cooldown: 0.2,
        }
    }
}

impl ControlsConfig {
    /// Validate controls configuration
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if movement cooldown is negative,
    /// if the inventory key list is empty, if the rest key list is empty, or if
    /// the automap key list is empty.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.movement_cooldown < 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "movement_cooldown must be non-negative, got {}",
                self.movement_cooldown
            )));
        }

        if self.inventory.is_empty() {
            return Err(ConfigError::ValidationError(
                "inventory key list must not be empty".to_string(),
            ));
        }

        if self.rest.is_empty() {
            return Err(ConfigError::ValidationError(
                "rest key list must not be empty".to_string(),
            ));
        }

        if self.automap.is_empty() {
            return Err(ConfigError::ValidationError(
                "automap key list must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Camera configuration
///
/// Controls camera behavior, field of view, clipping planes, and associated lighting.
/// Matches current hardcoded values in camera.rs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraConfig {
    /// Camera mode (first-person, tactical, isometric)
    pub mode: CameraMode,

    /// Eye height above ground in world units (1 unit = 10 feet)
    pub eye_height: f32,

    /// Field of view in degrees
    pub fov: f32,

    /// Near clipping plane distance
    pub near_clip: f32,

    /// Far clipping plane distance
    pub far_clip: f32,

    /// Enable smooth camera rotation
    pub smooth_rotation: bool,

    /// Rotation speed in degrees per second (used if smooth_rotation is true)
    pub rotation_speed: f32,

    /// Light source height above ground
    pub light_height: f32,

    /// Light intensity in lumens
    pub light_intensity: f32,

    /// Light range in world units
    pub light_range: f32,

    /// Enable shadow rendering
    pub shadows_enabled: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            mode: CameraMode::FirstPerson,
            eye_height: 0.6, // 6 feet (1 unit = 10 feet)
            fov: 70.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            smooth_rotation: false,
            rotation_speed: 180.0,
            light_height: 5.0,
            light_intensity: 2_000_000.0,
            light_range: 60.0,
            shadows_enabled: true,
        }
    }
}

impl CameraConfig {
    /// Validate camera configuration
    ///
    /// # Errors
    ///
    /// Returns error if values are out of reasonable ranges
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.eye_height <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "eye_height must be positive, got {}",
                self.eye_height
            )));
        }

        if !(30.0..=120.0).contains(&self.fov) {
            return Err(ConfigError::ValidationError(format!(
                "fov must be in range 30.0-120.0 degrees, got {}",
                self.fov
            )));
        }

        if self.near_clip <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "near_clip must be positive, got {}",
                self.near_clip
            )));
        }

        if self.far_clip <= self.near_clip {
            return Err(ConfigError::ValidationError(format!(
                "far_clip ({}) must be greater than near_clip ({})",
                self.far_clip, self.near_clip
            )));
        }

        Ok(())
    }
}

/// Camera mode selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CameraMode {
    /// First-person view (like Might and Magic 1)
    FirstPerson,
    /// Tactical overhead view
    Tactical,
    /// Isometric view
    Isometric,
}

/// Rest system configuration
///
/// Controls tunable parameters for the party rest mechanic.  Place a `rest:`
/// block inside `GameConfig` in `config.ron` to override any of these values.
/// All fields default to sensible values so the block may be omitted entirely.
///
/// # Examples
///
/// ```
/// use antares::sdk::game_config::RestConfig;
///
/// let cfg = RestConfig::default();
/// assert_eq!(cfg.full_rest_hours, 12);
/// assert!((cfg.rest_encounter_rate_multiplier - 1.0).abs() < f32::EPSILON);
/// assert!(!cfg.allow_partial_rest);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RestConfig {
    /// Full-rest duration in hours.
    ///
    /// The number of in-game hours required for a complete rest cycle that
    /// fully restores HP and SP.  Defaults to `12`.  Must be at least `1`.
    pub full_rest_hours: u32,

    /// Encounter check probability multiplier during rest.
    ///
    /// Scales the chance that each rest-hour triggers a random encounter.
    /// `0.0` disables encounters entirely (useful for town or safe-area maps).
    /// `1.0` uses the map's unmodified encounter rate.
    /// Values above `1.0` increase the encounter chance beyond normal.
    ///
    /// Must be `>= 0.0`.  Defaults to `1.0`.
    pub rest_encounter_rate_multiplier: f32,

    /// Allow partial rest (rest for fewer than `full_rest_hours`).
    ///
    /// When `true`, the player will eventually be able to choose a shorter
    /// rest duration via the UI.  Currently not fully implemented; set to
    /// `false` (default) to keep the standard full-rest behaviour.
    pub allow_partial_rest: bool,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            full_rest_hours: 12,
            rest_encounter_rate_multiplier: 1.0,
            allow_partial_rest: false,
        }
    }
}

impl RestConfig {
    /// Validate the rest configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if:
    /// - `full_rest_hours` is zero.
    /// - `rest_encounter_rate_multiplier` is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::game_config::RestConfig;
    ///
    /// assert!(RestConfig::default().validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.full_rest_hours == 0 {
            return Err(ConfigError::ValidationError(
                "full_rest_hours must be at least 1".to_string(),
            ));
        }
        if self.rest_encounter_rate_multiplier < 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "rest_encounter_rate_multiplier must be >= 0.0, got {}",
                self.rest_encounter_rate_multiplier
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── RestConfig tests ──────────────────────────────────────────────────────

    #[test]
    fn test_rest_config_default_values() {
        let config = RestConfig::default();
        assert_eq!(config.full_rest_hours, 12);
        assert!((config.rest_encounter_rate_multiplier - 1.0).abs() < f32::EPSILON);
        assert!(!config.allow_partial_rest);
    }

    #[test]
    fn test_rest_config_validation_success() {
        assert!(RestConfig::default().validate().is_ok());
    }

    #[test]
    fn test_rest_config_validation_zero_hours_fails() {
        let config = RestConfig {
            full_rest_hours: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rest_config_validation_negative_multiplier_fails() {
        let config = RestConfig {
            rest_encounter_rate_multiplier: -0.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rest_config_multiplier_zero_is_valid() {
        let config = RestConfig {
            rest_encounter_rate_multiplier: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rest_config_multiplier_above_one_is_valid() {
        let config = RestConfig {
            rest_encounter_rate_multiplier: 2.5,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rest_config_ron_roundtrip() {
        let original = RestConfig {
            full_rest_hours: 8,
            rest_encounter_rate_multiplier: 0.5,
            allow_partial_rest: true,
        };
        let ron_string = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: RestConfig =
            ron::from_str(&ron_string).expect("deserialization must succeed");
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_game_config_rest_field_default() {
        let config = GameConfig::default();
        assert_eq!(config.rest.full_rest_hours, 12);
        assert!((config.rest.rest_encounter_rate_multiplier - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_game_log_config_default_values() {
        let cfg = GameLogConfig::default();
        assert_eq!(cfg.max_entries, 200);
        assert!(cfg.visible_by_default);
        assert_eq!(cfg.toggle_key, "L");
        assert!(!cfg.show_timestamps);
        assert!((cfg.panel_width_px - 300.0).abs() < f32::EPSILON);
        assert!((cfg.panel_height_px - 200.0).abs() < f32::EPSILON);
        assert!((cfg.panel_opacity - 0.88).abs() < f32::EPSILON);
        assert_eq!(
            cfg.default_enabled_categories,
            vec![
                "Combat".to_string(),
                "Dialogue".to_string(),
                "Item".to_string(),
                "Exploration".to_string(),
                "System".to_string(),
            ]
        );
    }

    #[test]
    fn test_game_log_config_validates() {
        let mut config = GameConfig::default();
        config.game_log.max_entries = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_game_log_config_round_trip() {
        let original = GameLogConfig::default();
        let ron_string = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: GameLogConfig =
            ron::from_str(&ron_string).expect("deserialization must succeed");
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_game_log_config_validation_success() {
        assert!(GameLogConfig::default().validate().is_ok());
    }

    #[test]
    fn test_game_log_config_validation_empty_toggle_key_fails() {
        let config = GameLogConfig {
            toggle_key: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_game_log_config_validation_invalid_panel_opacity_fails() {
        let config = GameLogConfig {
            panel_opacity: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_game_log_config_validation_empty_default_categories_fails() {
        let config = GameLogConfig {
            default_enabled_categories: vec![],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_game_config_validates_rest_block() {
        let mut config = GameConfig::default();
        config.rest.full_rest_hours = 0; // invalid
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_game_config_rest_defaults_when_missing_from_ron() {
        // A RON config without a `rest:` block must deserialise with defaults.
        let ron_without_rest = r#"GameConfig(
            graphics: GraphicsConfig(
                resolution: (1280, 720),
                fullscreen: false,
                vsync: true,
                msaa_samples: 4,
                shadow_quality: Medium,
            ),
            audio: AudioConfig(
                master_volume: 0.8,
                music_volume: 0.6,
                sfx_volume: 1.0,
                ambient_volume: 0.5,
                enable_audio: true,
            ),
            controls: ControlsConfig(
                move_forward: ["W", "ArrowUp"],
                move_back: ["S", "ArrowDown"],
                turn_left: ["A", "ArrowLeft"],
                turn_right: ["D", "ArrowRight"],
                interact: ["Space", "E"],
                menu: ["Escape"],
                movement_cooldown: 0.2,
            ),
            camera: CameraConfig(
                mode: FirstPerson,
                eye_height: 0.6,
                fov: 70.0,
                near_clip: 0.1,
                far_clip: 1000.0,
                smooth_rotation: false,
                rotation_speed: 180.0,
                light_height: 5.0,
                light_intensity: 2000000.0,
                light_range: 60.0,
                shadows_enabled: true,
            ),
            game_log: GameLogConfig(
                max_entries: 200,
                visible_by_default: true,
                toggle_key: "L",
                show_timestamps: false,
                panel_width_px: 300.0,
                panel_height_px: 200.0,
                panel_opacity: 0.88,

                default_enabled_categories: ["Combat", "Dialogue", "Item", "Exploration", "System"],
            ),
        )"#;
        let config: GameConfig =
            ron::from_str(ron_without_rest).expect("deserialization must succeed");
        assert_eq!(
            config.rest.full_rest_hours, 12,
            "rest.full_rest_hours must default to 12 when absent"
        );
        assert!(
            (config.rest.rest_encounter_rate_multiplier - 1.0).abs() < f32::EPSILON,
            "rest.rest_encounter_rate_multiplier must default to 1.0 when absent"
        );
    }

    #[test]
    fn test_game_config_default_values() {
        let config = GameConfig::default();

        // Verify graphics defaults match current hardcoded values
        assert_eq!(config.graphics.resolution, (1280, 720));
        assert!(!config.graphics.fullscreen);
        assert!(config.graphics.vsync);
        assert_eq!(config.graphics.msaa_samples, 4);
        assert_eq!(config.graphics.shadow_quality, ShadowQuality::Medium);
        assert!(config.graphics.show_combat_monster_hp_bars);
        assert!(config.graphics.show_minimap);

        // Verify audio defaults
        assert_eq!(config.audio.master_volume, 0.8);
        assert_eq!(config.audio.music_volume, 0.6);
        assert_eq!(config.audio.sfx_volume, 1.0);
        assert_eq!(config.audio.ambient_volume, 0.5);
        assert!(config.audio.enable_audio);

        // Verify camera defaults match camera.rs hardcoded values
        assert_eq!(config.camera.mode, CameraMode::FirstPerson);
        assert_eq!(config.camera.eye_height, 0.6);
        assert_eq!(config.camera.fov, 70.0);
        assert_eq!(config.camera.near_clip, 0.1);
        assert_eq!(config.camera.far_clip, 1000.0);
        assert!(!config.camera.smooth_rotation);
        assert_eq!(config.camera.rotation_speed, 180.0);
        assert_eq!(config.camera.light_height, 5.0);
        assert_eq!(config.camera.light_intensity, 2_000_000.0);
        assert_eq!(config.camera.light_range, 60.0);
        assert!(config.camera.shadows_enabled);

        // Verify controls defaults
        assert_eq!(config.controls.move_forward, vec!["W", "ArrowUp"]);
        assert_eq!(config.controls.move_back, vec!["S", "ArrowDown"]);
        assert_eq!(config.controls.turn_left, vec!["A", "ArrowLeft"]);
        assert_eq!(config.controls.turn_right, vec!["D", "ArrowRight"]);
        assert_eq!(config.controls.interact, vec!["Space", "E"]);
        assert_eq!(config.controls.menu, vec!["Escape"]);
        assert_eq!(config.controls.automap, vec!["M"]);
        assert_eq!(config.controls.movement_cooldown, 0.2);

        // Verify game log defaults
        assert_eq!(config.game_log.max_entries, 200);
        assert!(config.game_log.visible_by_default);
        assert_eq!(config.game_log.toggle_key, "L");
        assert!(!config.game_log.show_timestamps);
        assert!((config.game_log.panel_width_px - 300.0).abs() < f32::EPSILON);
        assert!((config.game_log.panel_height_px - 200.0).abs() < f32::EPSILON);
        assert!((config.game_log.panel_opacity - 0.88).abs() < f32::EPSILON);
        assert_eq!(
            config.game_log.default_enabled_categories,
            vec![
                "Combat".to_string(),
                "Dialogue".to_string(),
                "Item".to_string(),
                "Exploration".to_string(),
                "System".to_string(),
            ]
        );
    }

    #[test]
    fn test_graphics_config_validation_success() {
        let config = GraphicsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_graphics_config_validation_zero_resolution() {
        let config = GraphicsConfig {
            resolution: (0, 720),
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = GraphicsConfig {
            resolution: (1280, 0),
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_graphics_config_validation_invalid_msaa() {
        let config = GraphicsConfig {
            msaa_samples: 3, // Not a power of 2
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_graphics_config_validation_valid_msaa() {
        let mut config = GraphicsConfig::default();
        for samples in [1, 2, 4, 8, 16] {
            config.msaa_samples = samples;
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_audio_config_validation_success() {
        let config = AudioConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_audio_config_validation_out_of_range() {
        let config = AudioConfig {
            master_volume: -0.1,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = AudioConfig {
            master_volume: 1.5,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = AudioConfig {
            master_volume: 0.5,
            music_volume: 2.0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_audio_config_validation_boundary() {
        let config = AudioConfig {
            master_volume: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = AudioConfig {
            master_volume: 1.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_camera_config_validation_success() {
        let config = CameraConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_camera_config_validation_invalid_eye_height() {
        let config = CameraConfig {
            eye_height: 0.0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = CameraConfig {
            eye_height: -0.5,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_camera_config_validation_fov_range() {
        let config = CameraConfig {
            fov: 29.0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = CameraConfig {
            fov: 121.0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = CameraConfig {
            fov: 30.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = CameraConfig {
            fov: 120.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_camera_config_validation_clip_planes() {
        let config = CameraConfig {
            near_clip: 0.0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));

        let config = CameraConfig {
            near_clip: 0.1,
            far_clip: 0.05, // Less than near_clip
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_controls_config_inventory_default() {
        let config = ControlsConfig::default();
        assert_eq!(config.inventory, vec!["I".to_string()]);
    }

    #[test]
    fn test_controls_config_validate_empty_inventory_keys() {
        let config = ControlsConfig {
            inventory: vec![],
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_controls_config_validate_non_empty_inventory_keys() {
        assert!(ControlsConfig::default().validate().is_ok());
    }

    #[test]
    fn test_controls_config_validation_success() {
        let config = ControlsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_controls_config_validation_negative_cooldown() {
        let config = ControlsConfig {
            movement_cooldown: -0.1,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_controls_config_rest_default() {
        let config = ControlsConfig::default();
        assert_eq!(config.rest, vec!["R".to_string()]);
    }

    #[test]
    fn test_controls_config_default_automap_key() {
        let config = ControlsConfig::default();
        assert_eq!(config.automap, vec!["M".to_string()]);
    }

    #[test]
    fn test_controls_config_validates_empty_rest_list() {
        let config = ControlsConfig {
            rest: vec![],
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_controls_config_validate_non_empty_rest_keys() {
        let config = ControlsConfig {
            rest: vec!["R".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_controls_config_validates_empty_automap_list() {
        let config = ControlsConfig {
            automap: vec![],
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_controls_config_validate_non_empty_automap_keys() {
        let config = ControlsConfig {
            automap: vec!["M".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_load_valid_config_file() {
        let ron_content = r#"(
            graphics: (
                resolution: (1920, 1080),
                fullscreen: true,
                vsync: false,
                msaa_samples: 8,
                shadow_quality: High,
                show_combat_monster_hp_bars: false,
                show_minimap: false,
            ),
            audio: (
                master_volume: 0.7,
                music_volume: 0.5,
                sfx_volume: 0.9,
                ambient_volume: 0.4,
                enable_audio: true,
            ),
            controls: (
                move_forward: ["W"],
                move_back: ["S"],
                turn_left: ["A"],
                turn_right: ["D"],
                interact: ["E"],
                menu: ["Escape"],
                movement_cooldown: 0.15,
            ),
            camera: (
                mode: FirstPerson,
                eye_height: 0.5,
                fov: 80.0,
                near_clip: 0.1,
                far_clip: 2000.0,
                smooth_rotation: true,
                rotation_speed: 90.0,
                light_height: 6.0,
                light_intensity: 3000000.0,
                light_range: 80.0,
                shadows_enabled: true,
            ),
        )"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ron_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = GameConfig::load_or_default(temp_file.path()).unwrap();

        assert_eq!(config.graphics.resolution, (1920, 1080));
        assert!(config.graphics.fullscreen);
        assert!(!config.graphics.show_combat_monster_hp_bars);
        assert!(!config.graphics.show_minimap);
        assert_eq!(config.audio.master_volume, 0.7);
        assert_eq!(config.camera.fov, 80.0);
    }

    #[test]
    fn test_load_valid_config_file_without_combat_hp_toggle_defaults_true() {
        let ron_content = r#"(
            graphics: (
                resolution: (1024, 768),
                fullscreen: false,
                vsync: true,
                msaa_samples: 4,
                shadow_quality: Medium,
            ),
            audio: (
                master_volume: 0.8,
                music_volume: 0.6,
                sfx_volume: 1.0,
                ambient_volume: 0.5,
                enable_audio: true,
            ),
            controls: (
                move_forward: ["W"],
                move_back: ["S"],
                turn_left: ["A"],
                turn_right: ["D"],
                interact: ["E"],
                menu: ["Escape"],
                movement_cooldown: 0.2,
            ),
            camera: (
                mode: FirstPerson,
                eye_height: 0.6,
                fov: 70.0,
                near_clip: 0.1,
                far_clip: 1000.0,
                smooth_rotation: false,
                rotation_speed: 180.0,
                light_height: 5.0,
                light_intensity: 2000000.0,
                light_range: 60.0,
                shadows_enabled: true,
            ),
        )"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ron_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = GameConfig::load_or_default(temp_file.path()).unwrap();
        assert!(config.graphics.show_combat_monster_hp_bars);
        assert!(config.graphics.show_minimap);
    }

    #[test]
    fn test_load_invalid_config_returns_error() {
        let invalid_ron = r#"(this is not valid RON"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_ron.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = GameConfig::load_or_default(temp_file.path());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_load_missing_config_uses_defaults() {
        let non_existent_path = Path::new("/tmp/this_file_does_not_exist_12345.ron");
        let config = GameConfig::load_or_default(non_existent_path).unwrap();

        assert_eq!(config, GameConfig::default());
    }

    #[test]
    fn test_game_config_validation_propagates_errors() {
        let config = GameConfig {
            graphics: GraphicsConfig {
                resolution: (0, 0),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_shadow_quality_variants() {
        use ShadowQuality::*;
        let qualities = vec![Low, Medium, High, Ultra];

        for quality in qualities {
            let config = GraphicsConfig {
                shadow_quality: quality,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_camera_mode_variants() {
        use CameraMode::*;
        let modes = vec![FirstPerson, Tactical, Isometric];

        for mode in modes {
            let config = CameraConfig {
                mode,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_tutorial_config_deserializes_with_inventory_key() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let config_path = std::path::Path::new(manifest_dir).join("data/test_campaign/config.ron");
        let config = GameConfig::load_or_default(&config_path).unwrap();
        assert_eq!(config.controls.inventory, vec!["I".to_string()]);
    }

    #[test]
    fn test_controls_config_ron_roundtrip_includes_inventory() {
        let original = ControlsConfig {
            inventory: vec!["I".to_string(), "F1".to_string()],
            ..Default::default()
        };
        let ron_string = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: ControlsConfig =
            ron::from_str(&ron_string).expect("deserialization must succeed");
        assert_eq!(
            deserialized.inventory,
            vec!["I".to_string(), "F1".to_string()],
            "inventory field must survive a RON round-trip"
        );
    }

    #[test]
    fn test_controls_config_ron_roundtrip_includes_rest() {
        let original = ControlsConfig {
            rest: vec!["R".to_string(), "F5".to_string()],
            ..Default::default()
        };
        let ron_string = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: ControlsConfig =
            ron::from_str(&ron_string).expect("deserialization must succeed");
        assert_eq!(
            deserialized.rest,
            vec!["R".to_string(), "F5".to_string()],
            "rest key bindings must survive RON round-trip"
        );
    }

    #[test]
    fn test_controls_config_ron_roundtrip_includes_automap() {
        let original = ControlsConfig {
            automap: vec!["M".to_string(), "Tab".to_string()],
            ..Default::default()
        };
        let ron_string = ron::to_string(&original).expect("serialization must succeed");
        let deserialized: ControlsConfig =
            ron::from_str(&ron_string).expect("deserialization must succeed");
        assert_eq!(
            deserialized.automap,
            vec!["M".to_string(), "Tab".to_string()],
            "automap key bindings must survive RON round-trip"
        );
    }

    #[test]
    fn test_controls_config_rest_defaults_when_missing_from_ron() {
        // RON that omits the `rest` field — serde default must kick in.
        let ron_without_rest = r#"(
            move_forward: ["W"],
            move_back: ["S"],
            turn_left: ["A"],
            turn_right: ["D"],
            interact: ["E"],
            menu: ["Escape"],
            inventory: ["I"],
            movement_cooldown: 0.2,
        )"#;
        let config: ControlsConfig =
            ron::from_str(ron_without_rest).expect("deserialization must succeed");
        assert_eq!(
            config.rest,
            vec!["R".to_string()],
            "missing `rest` field must default to ['R']"
        );
        assert_eq!(
            config.automap,
            vec!["M".to_string()],
            "missing `automap` field must default to ['M']"
        );
    }

    #[test]
    fn test_graphics_config_serde_show_minimap_default() {
        let ron_without_show_minimap = r#"(
            resolution: (1280, 720),
            fullscreen: false,
            vsync: true,
            msaa_samples: 4,
            shadow_quality: Medium,
            show_combat_monster_hp_bars: true,
        )"#;
        let config: GraphicsConfig =
            ron::from_str(ron_without_show_minimap).expect("deserialization must succeed");
        assert!(
            config.show_minimap,
            "missing `show_minimap` field must default to true"
        );
    }

    #[test]
    fn test_controls_config_automap_defaults_when_missing_from_ron() {
        let ron_without_automap = r#"(
            move_forward: ["W"],
            move_back: ["S"],
            turn_left: ["A"],
            turn_right: ["D"],
            interact: ["E"],
            menu: ["Escape"],
            inventory: ["I"],
            rest: ["R"],
            movement_cooldown: 0.2,
        )"#;
        let config: ControlsConfig =
            ron::from_str(ron_without_automap).expect("deserialization must succeed");
        assert_eq!(
            config.automap,
            vec!["M".to_string()],
            "missing `automap` field must default to ['M']"
        );
    }
}
