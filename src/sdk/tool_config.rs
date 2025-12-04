// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared configuration system for SDK tools
//!
//! Provides consistent configuration management across all Antares SDK tools
//! (map_builder, class_editor, race_editor, item_editor, campaign_validator).
//!
//! # Configuration File
//!
//! Tools read from `~/.config/antares/tools.ron` (or platform equivalent):
//!
//! ```ron
//! ToolConfig(
//!     editor: EditorPreferences(
//!         auto_save: true,
//!         backup_count: 3,
//!         validate_on_save: true,
//!     ),
//!     paths: PathConfig(
//!         data_dir: Some("data"),
//!         campaigns_dir: Some("campaigns"),
//!         recent_files: ["campaigns/tutorial/campaign.ron"],
//!     ),
//!     display: DisplayConfig(
//!         color: true,
//!         verbose: false,
//!         page_size: 20,
//!     ),
//!     validation: ValidationConfig(
//!         strict_mode: false,
//!         check_balance: true,
//!         show_suggestions: true,
//!     ),
//! )
//! ```
//!
//! # Examples
//!
//! ```
//! use antares::sdk::tool_config::ToolConfig;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load or create default config
//! let config = ToolConfig::load_or_default()?;
//!
//! // Access settings
//! if config.editor.auto_save {
//!     println!("Auto-save is enabled");
//! }
//!
//! // Update and save
//! let mut config = config;
//! config.display.verbose = true;
//! config.save()?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with tool configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    #[error("Failed to write config file: {0}")]
    WriteError(String),

    #[error("Failed to parse config file: {0}")]
    ParseError(String),

    #[error("Config directory not found and could not be created: {0}")]
    DirectoryError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// ===== Configuration Structures =====

/// Main tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolConfig {
    /// Editor behavior preferences
    pub editor: EditorPreferences,

    /// Path configuration
    pub paths: PathConfig,

    /// Display preferences
    pub display: DisplayConfig,

    /// Validation settings
    pub validation: ValidationConfig,
}

/// Editor behavior preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    /// Automatically save on exit
    pub auto_save: bool,

    /// Number of backup files to keep
    pub backup_count: u8,

    /// Validate content on save
    pub validate_on_save: bool,

    /// Confirm before destructive operations
    pub confirm_destructive: bool,
}

/// Path configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Default data directory for core content
    pub data_dir: Option<PathBuf>,

    /// Default campaigns directory
    pub campaigns_dir: Option<PathBuf>,

    /// Recently opened files (max 10)
    pub recent_files: Vec<PathBuf>,
}

/// Display preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Use colored output
    pub color: bool,

    /// Verbose output mode
    pub verbose: bool,

    /// Items per page in lists
    pub page_size: usize,

    /// Show hints and tips
    pub show_hints: bool,

    /// Minimum width (points) for inspector (right) panels in two-column editors.
    /// This is used by editors to ensure the inspector column isn't clipped by the
    /// list/detail split layout.
    pub inspector_min_width: f32,

    /// Maximum width ratio for the left column in two-column editors. This is a
    /// fraction in the range 0.0..=1.0 (e.g., 0.72 means left column should not
    /// exceed 72% of the available width).
    pub left_column_max_ratio: f32,
}

/// Validation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Strict validation mode (warnings become errors)
    pub strict_mode: bool,

    /// Perform balance checking
    pub check_balance: bool,

    /// Show suggestions for fixes
    pub show_suggestions: bool,

    /// Maximum errors to display before truncating
    pub max_errors_displayed: usize,
}

// ===== Default Implementations =====

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            auto_save: true,
            backup_count: 3,
            validate_on_save: true,
            confirm_destructive: true,
        }
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            data_dir: Some(PathBuf::from("data")),
            campaigns_dir: Some(PathBuf::from("campaigns")),
            recent_files: Vec::new(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            color: true,
            verbose: false,
            page_size: 20,
            show_hints: true,
            inspector_min_width: 300.0,
            left_column_max_ratio: 0.72,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            check_balance: true,
            show_suggestions: true,
            max_errors_displayed: 50,
        }
    }
}

// ===== Implementation =====

impl ToolConfig {
    /// Gets the default config file path
    ///
    /// Returns `~/.config/antares/tools.ron` on Unix-like systems,
    /// `%APPDATA%\antares\tools.ron` on Windows.
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            ConfigError::DirectoryError("Could not determine config directory".to_string())
        })?;

        Ok(config_dir.join("antares").join("tools.ron"))
    }

    /// Loads configuration from the default location
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::default_path()?;
        Self::load_from_file(&path)
    }

    /// Loads configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents =
            fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::ReadError(e.to_string()))?;

        let config: Self =
            ron::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Loads configuration, or returns default if file doesn't exist
    pub fn load_or_default() -> Result<Self, ConfigError> {
        match Self::load() {
            Ok(config) => Ok(config),
            // If the file is missing or invalid/unparseable, fall back to a default
            // configuration instead of failing — this ensures older configs or
            // partial configs do not break the application at startup.
            Err(ConfigError::ReadError(_)) => Ok(Self::default()),
            Err(ConfigError::ParseError(_)) => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Saves configuration to the default location
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_path()?;
        self.save_to_file(&path)
    }

    /// Saves configuration to a specific file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::DirectoryError(e.to_string()))?;
        }

        let ron_string = ron::ser::to_string_pretty(self, Default::default())
            .map_err(|e| ConfigError::WriteError(e.to_string()))?;

        fs::write(path.as_ref(), ron_string).map_err(|e| ConfigError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Adds a file to recent files list (max 10)
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists
        self.paths.recent_files.retain(|p| p != &path);

        // Add to front
        self.paths.recent_files.insert(0, path);

        // Keep only 10 most recent
        self.paths.recent_files.truncate(10);
    }

    /// Gets the data directory path, resolving relative paths
    pub fn get_data_dir(&self) -> PathBuf {
        self.paths
            .data_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("data"))
    }

    /// Gets the campaigns directory path, resolving relative paths
    pub fn get_campaigns_dir(&self) -> PathBuf {
        self.paths
            .campaigns_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("campaigns"))
    }
}

// ===== Helper Functions =====

/// Creates a default config file if it doesn't exist
pub fn ensure_config_exists() -> Result<PathBuf, ConfigError> {
    let path = ToolConfig::default_path()?;

    if !path.exists() {
        let default_config = ToolConfig::default();
        default_config.save_to_file(&path)?;
    }

    Ok(path)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ToolConfig::default();
        assert!(config.editor.auto_save);
        assert_eq!(config.editor.backup_count, 3);
        assert!(config.display.color);
        assert!(!config.validation.strict_mode);
    }

    #[test]
    fn test_save_and_load() -> Result<(), Box<dyn std::error::Error>> {
        let temp = NamedTempFile::new()?;
        let path = temp.path().to_path_buf();

        let mut config = ToolConfig::default();
        config.display.verbose = true;
        config.validation.strict_mode = true;

        config.save_to_file(&path)?;

        let loaded = ToolConfig::load_from_file(&path)?;
        assert!(loaded.display.verbose);
        assert!(loaded.validation.strict_mode);

        Ok(())
    }

    #[test]
    fn test_recent_files() {
        let mut config = ToolConfig::default();

        // Add files
        for i in 0..12 {
            config.add_recent_file(PathBuf::from(format!("file{}.ron", i)));
        }

        // Should keep only 10 most recent
        assert_eq!(config.paths.recent_files.len(), 10);

        // Most recent should be first
        assert_eq!(config.paths.recent_files[0], PathBuf::from("file11.ron"));
    }

    #[test]
    fn test_add_recent_file_deduplicates() {
        let mut config = ToolConfig::default();
        let path = PathBuf::from("test.ron");

        config.add_recent_file(path.clone());
        config.add_recent_file(PathBuf::from("other.ron"));
        config.add_recent_file(path.clone());

        assert_eq!(config.paths.recent_files.len(), 2);
        assert_eq!(config.paths.recent_files[0], PathBuf::from("test.ron"));
    }

    #[test]
    fn test_get_data_dir() {
        let config = ToolConfig::default();
        let data_dir = config.get_data_dir();
        assert_eq!(data_dir, PathBuf::from("data"));
    }

    #[test]
    fn test_get_campaigns_dir() {
        let mut config = ToolConfig::default();
        config.paths.campaigns_dir = Some(PathBuf::from("custom_campaigns"));
        let campaigns_dir = config.get_campaigns_dir();
        assert_eq!(campaigns_dir, PathBuf::from("custom_campaigns"));
    }

    #[test]
    fn test_load_or_default() -> Result<(), Box<dyn std::error::Error>> {
        // Should return default if file doesn't exist
        let config = ToolConfig::load_or_default()?;
        assert!(config.editor.auto_save);
        Ok(())
    }

    #[test]
    fn test_invalid_ron_returns_parse_error() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "invalid ron content").unwrap();

        let result = ToolConfig::load_from_file(temp.path());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}
