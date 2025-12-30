// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
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
            eprintln!(
                "Warning: Config file not found at {:?}, using default configuration",
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
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            resolution: (1280, 720),
            fullscreen: false,
            vsync: true,
            msaa_samples: 4,
            shadow_quality: ShadowQuality::Medium,
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

    /// Movement cooldown in seconds (prevents double-moves)
    pub movement_cooldown: f32,
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
            movement_cooldown: 0.2,
        }
    }
}

impl ControlsConfig {
    /// Validate controls configuration
    ///
    /// # Errors
    ///
    /// Returns error if movement cooldown is negative
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.movement_cooldown < 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "movement_cooldown must be non-negative, got {}",
                self.movement_cooldown
            )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_game_config_default_values() {
        let config = GameConfig::default();

        // Verify graphics defaults match current hardcoded values
        assert_eq!(config.graphics.resolution, (1280, 720));
        assert!(!config.graphics.fullscreen);
        assert!(config.graphics.vsync);
        assert_eq!(config.graphics.msaa_samples, 4);
        assert_eq!(config.graphics.shadow_quality, ShadowQuality::Medium);

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
        assert_eq!(config.controls.movement_cooldown, 0.2);
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
    fn test_load_valid_config_file() {
        let ron_content = r#"(
            graphics: (
                resolution: (1920, 1080),
                fullscreen: true,
                vsync: false,
                msaa_samples: 8,
                shadow_quality: High,
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
        assert_eq!(config.audio.master_volume, 0.7);
        assert_eq!(config.camera.fov, 80.0);
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
}
