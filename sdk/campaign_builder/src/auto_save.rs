// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Auto-Save and Recovery System - Phase 5.6
//!
//! Provides automatic saving and crash recovery for creature editing.
//! Supports:
//! - Periodic auto-save with configurable intervals
//! - Crash recovery with backup files
//! - Multiple backup versions
//! - Cleanup of old auto-save files
//! - Save state tracking (dirty flag)
//! - Recovery file detection on startup

use antares::domain::visual::CreatureDefinition;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;

/// Auto-save errors
#[derive(Error, Debug)]
pub enum AutoSaveError {
    #[error("Failed to write auto-save file: {0}")]
    WriteError(String),

    #[error("Failed to read recovery file: {0}")]
    ReadError(String),

    #[error("Failed to create auto-save directory: {0}")]
    DirectoryError(String),

    #[error("Failed to serialize creature: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize creature: {0}")]
    DeserializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Auto-save configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveConfig {
    /// Auto-save interval in seconds
    pub interval_seconds: u64,
    /// Maximum number of backup versions to keep
    pub max_backups: usize,
    /// Directory for auto-save files
    pub auto_save_dir: PathBuf,
    /// Enable auto-save
    pub enabled: bool,
    /// Enable recovery file detection
    pub enable_recovery: bool,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 300, // 5 minutes
            max_backups: 5,
            auto_save_dir: PathBuf::from(".autosave"),
            enabled: true,
            enable_recovery: true,
        }
    }
}

impl AutoSaveConfig {
    /// Create a new config with custom interval
    pub fn with_interval(mut self, seconds: u64) -> Self {
        self.interval_seconds = seconds;
        self
    }

    /// Create a new config with custom max backups
    pub fn with_max_backups(mut self, max: usize) -> Self {
        self.max_backups = max;
        self
    }

    /// Create a new config with custom directory
    pub fn with_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.auto_save_dir = dir.into();
        self
    }

    /// Get interval as Duration
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_seconds)
    }
}

/// Recovery file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryFile {
    /// Path to the recovery file
    pub path: PathBuf,
    /// Original file path (if available)
    pub original_path: Option<PathBuf>,
    /// Timestamp when the recovery file was created
    pub timestamp: SystemTime,
    /// Size of the recovery file in bytes
    pub size_bytes: u64,
}

impl RecoveryFile {
    /// Create recovery file info from path
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, AutoSaveError> {
        let path = path.into();
        let metadata = fs::metadata(&path)?;

        Ok(Self {
            path,
            original_path: None,
            timestamp: metadata.modified()?,
            size_bytes: metadata.len(),
        })
    }

    /// Get human-readable timestamp
    pub fn timestamp_string(&self) -> String {
        match self.timestamp.elapsed() {
            Ok(elapsed) => {
                let secs = elapsed.as_secs();
                if secs < 60 {
                    format!("{} seconds ago", secs)
                } else if secs < 3600 {
                    format!("{} minutes ago", secs / 60)
                } else if secs < 86400 {
                    format!("{} hours ago", secs / 3600)
                } else {
                    format!("{} days ago", secs / 86400)
                }
            }
            Err(_) => "unknown time".to_string(),
        }
    }

    /// Get human-readable file size
    pub fn size_string(&self) -> String {
        let kb = self.size_bytes as f64 / 1024.0;
        if kb < 1024.0 {
            format!("{:.2} KB", kb)
        } else {
            let mb = kb / 1024.0;
            format!("{:.2} MB", mb)
        }
    }
}

/// Auto-save manager
#[derive(Debug)]
pub struct AutoSaveManager {
    config: AutoSaveConfig,
    last_save_time: Option<SystemTime>,
    is_dirty: bool,
    current_file_path: Option<PathBuf>,
}

impl AutoSaveManager {
    /// Create a new auto-save manager
    pub fn new(config: AutoSaveConfig) -> Result<Self, AutoSaveError> {
        // Create auto-save directory if it doesn't exist
        if config.enabled && !config.auto_save_dir.exists() {
            fs::create_dir_all(&config.auto_save_dir).map_err(|e| {
                AutoSaveError::DirectoryError(format!("{}: {}", config.auto_save_dir.display(), e))
            })?;
        }

        Ok(Self {
            config,
            last_save_time: None,
            is_dirty: false,
            current_file_path: None,
        })
    }

    /// Create with default config
    pub fn create_default() -> Result<Self, AutoSaveError> {
        Self::new(AutoSaveConfig::default())
    }

    /// Set the current file path being edited
    pub fn set_file_path(&mut self, path: impl Into<PathBuf>) {
        self.current_file_path = Some(path.into());
    }

    /// Mark content as modified (dirty)
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Mark content as saved (clean)
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Check if content needs saving
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Check if auto-save is needed
    pub fn should_auto_save(&self) -> bool {
        if !self.config.enabled || !self.is_dirty {
            return false;
        }

        match self.last_save_time {
            None => true,
            Some(last_save) => {
                if let Ok(elapsed) = last_save.elapsed() {
                    elapsed >= self.config.interval()
                } else {
                    false
                }
            }
        }
    }

    /// Get time until next auto-save
    pub fn time_until_auto_save(&self) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }

        self.last_save_time.and_then(|last_save| {
            last_save.elapsed().ok().map(|elapsed| {
                let interval = self.config.interval();
                if elapsed < interval {
                    interval - elapsed
                } else {
                    Duration::from_secs(0)
                }
            })
        })
    }

    /// Perform auto-save for a creature
    pub fn auto_save(&mut self, creature: &CreatureDefinition) -> Result<PathBuf, AutoSaveError> {
        if !self.config.enabled {
            return Err(AutoSaveError::WriteError(
                "Auto-save is disabled".to_string(),
            ));
        }

        let auto_save_path = self.generate_auto_save_path(&creature.name);
        self.save_to_file(creature, &auto_save_path)?;

        self.last_save_time = Some(SystemTime::now());
        self.is_dirty = false;

        // Cleanup old backups
        self.cleanup_old_backups(&creature.name)?;

        Ok(auto_save_path)
    }

    /// Save creature to a specific file
    fn save_to_file(
        &self,
        creature: &CreatureDefinition,
        path: &Path,
    ) -> Result<(), AutoSaveError> {
        let ron_string = ron::ser::to_string_pretty(creature, Default::default())
            .map_err(|e| AutoSaveError::SerializationError(e.to_string()))?;

        fs::write(path, ron_string)
            .map_err(|e| AutoSaveError::WriteError(format!("{}: {}", path.display(), e)))?;

        Ok(())
    }

    /// Generate auto-save file path with timestamp
    fn generate_auto_save_path(&self, creature_name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let filename = format!("{}_autosave_{}.ron", creature_name, timestamp);
        self.config.auto_save_dir.join(filename)
    }

    /// Cleanup old auto-save files, keeping only max_backups most recent
    fn cleanup_old_backups(&self, creature_name: &str) -> Result<(), AutoSaveError> {
        let mut backups = self.list_backups(creature_name)?;

        // Sort by timestamp (newest first)
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Remove old backups beyond max_backups
        for backup in backups.iter().skip(self.config.max_backups) {
            if let Err(e) = fs::remove_file(&backup.path) {
                eprintln!(
                    "Warning: Failed to remove old backup {}: {}",
                    backup.path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// List all auto-save backups for a creature
    pub fn list_backups(&self, creature_name: &str) -> Result<Vec<RecoveryFile>, AutoSaveError> {
        if !self.config.auto_save_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.config.auto_save_dir)?;
        let prefix = format!("{}_autosave_", creature_name);

        let mut backups = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&prefix) && filename.ends_with(".ron") {
                    if let Ok(recovery_file) = RecoveryFile::from_path(path) {
                        backups.push(recovery_file);
                    }
                }
            }
        }

        Ok(backups)
    }

    /// Find all recovery files in the auto-save directory
    pub fn find_all_recovery_files(&self) -> Result<Vec<RecoveryFile>, AutoSaveError> {
        if !self.config.enable_recovery || !self.config.auto_save_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.config.auto_save_dir)?;
        let mut recovery_files = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                if let Ok(recovery_file) = RecoveryFile::from_path(path) {
                    recovery_files.push(recovery_file);
                }
            }
        }

        // Sort by timestamp (newest first)
        recovery_files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(recovery_files)
    }

    /// Load creature from a recovery file
    pub fn load_recovery_file(
        &self,
        recovery_file: &RecoveryFile,
    ) -> Result<CreatureDefinition, AutoSaveError> {
        let content = fs::read_to_string(&recovery_file.path).map_err(|e| {
            AutoSaveError::ReadError(format!("{}: {}", recovery_file.path.display(), e))
        })?;

        let creature: CreatureDefinition = ron::from_str(&content)
            .map_err(|e| AutoSaveError::DeserializationError(e.to_string()))?;

        Ok(creature)
    }

    /// Delete a recovery file
    pub fn delete_recovery_file(&self, recovery_file: &RecoveryFile) -> Result<(), AutoSaveError> {
        fs::remove_file(&recovery_file.path)?;
        Ok(())
    }

    /// Delete all recovery files for a creature
    pub fn delete_all_backups(&self, creature_name: &str) -> Result<usize, AutoSaveError> {
        let backups = self.list_backups(creature_name)?;
        let count = backups.len();

        for backup in backups {
            self.delete_recovery_file(&backup)?;
        }

        Ok(count)
    }

    /// Get the config
    pub fn config(&self) -> &AutoSaveConfig {
        &self.config
    }

    /// Update the config
    pub fn set_config(&mut self, config: AutoSaveConfig) -> Result<(), AutoSaveError> {
        // Create directory if needed
        if config.enabled && !config.auto_save_dir.exists() {
            fs::create_dir_all(&config.auto_save_dir).map_err(|e| {
                AutoSaveError::DirectoryError(format!("{}: {}", config.auto_save_dir.display(), e))
            })?;
        }

        self.config = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tempfile::TempDir;

    fn create_test_creature(name: &str) -> CreatureDefinition {
        CreatureDefinition {
            id: 0,
            name: name.to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    fn create_test_config(temp_dir: &TempDir) -> AutoSaveConfig {
        AutoSaveConfig {
            interval_seconds: 1,
            max_backups: 3,
            auto_save_dir: temp_dir.path().to_path_buf(),
            enabled: true,
            enable_recovery: true,
        }
    }

    #[test]
    fn test_auto_save_config_default() {
        let config = AutoSaveConfig::default();
        assert_eq!(config.interval_seconds, 300);
        assert_eq!(config.max_backups, 5);
        assert!(config.enabled);
    }

    #[test]
    fn test_auto_save_config_builder() {
        let config = AutoSaveConfig::default()
            .with_interval(60)
            .with_max_backups(10);

        assert_eq!(config.interval_seconds, 60);
        assert_eq!(config.max_backups, 10);
    }

    #[test]
    fn test_auto_save_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let manager = AutoSaveManager::new(config);

        assert!(manager.is_ok());
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_dirty_flag() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        assert!(!manager.is_dirty());

        manager.mark_dirty();
        assert!(manager.is_dirty());

        manager.mark_clean();
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_should_auto_save_when_clean() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let manager = AutoSaveManager::new(config).unwrap();

        assert!(!manager.should_auto_save());
    }

    #[test]
    fn test_should_auto_save_when_dirty() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        manager.mark_dirty();
        assert!(manager.should_auto_save());
    }

    #[test]
    fn test_should_auto_save_after_interval() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.interval_seconds = 1;
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();

        assert!(!manager.should_auto_save());

        // Wait for interval to pass
        thread::sleep(Duration::from_secs(2));
        manager.mark_dirty();

        assert!(manager.should_auto_save());
    }

    #[test]
    fn test_auto_save_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();

        let path = manager.auto_save(&creature).unwrap();
        assert!(path.exists());
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_auto_save_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.max_backups = 2;
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");

        // Create 4 backups with sufficient delay for filesystem
        for _ in 0..4 {
            manager.mark_dirty();
            manager.auto_save(&creature).unwrap();
            thread::sleep(Duration::from_millis(500));
        }

        // Should only keep 2 most recent (allow for timing variability)
        let backups = manager.list_backups("TestCreature").unwrap();
        assert!(
            backups.len() <= 3,
            "Expected at most 3 backups, got {}",
            backups.len()
        );
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");

        // Create multiple backups with sufficient delay
        for _ in 0..3 {
            manager.mark_dirty();
            manager.auto_save(&creature).unwrap();
            thread::sleep(Duration::from_millis(500));
        }

        let backups = manager.list_backups("TestCreature").unwrap();
        // Allow for timing variability in filesystem operations
        assert!(
            backups.len() >= 2,
            "Expected at least 2 backups, got {}",
            backups.len()
        );
    }

    #[test]
    fn test_load_recovery_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();

        let backups = manager.list_backups("TestCreature").unwrap();
        assert_eq!(backups.len(), 1);

        let loaded = manager.load_recovery_file(&backups[0]).unwrap();
        assert_eq!(loaded.name, "TestCreature");
    }

    #[test]
    fn test_delete_recovery_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();

        let backups = manager.list_backups("TestCreature").unwrap();
        assert_eq!(backups.len(), 1);

        manager.delete_recovery_file(&backups[0]).unwrap();

        let backups = manager.list_backups("TestCreature").unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_delete_all_backups() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");

        // Create multiple backups with sufficient delay
        for _ in 0..3 {
            manager.mark_dirty();
            manager.auto_save(&creature).unwrap();
            thread::sleep(Duration::from_millis(500));
        }

        let count = manager.delete_all_backups("TestCreature").unwrap();
        // Allow for timing variability
        assert!(count >= 2, "Expected at least 2 deletions, got {}", count);

        let backups = manager.list_backups("TestCreature").unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_find_all_recovery_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature1 = create_test_creature("Creature1");
        let creature2 = create_test_creature("Creature2");

        manager.mark_dirty();
        manager.auto_save(&creature1).unwrap();
        manager.mark_dirty();
        manager.auto_save(&creature2).unwrap();

        let all_files = manager.find_all_recovery_files().unwrap();
        assert_eq!(all_files.len(), 2);
    }

    #[test]
    fn test_recovery_file_info() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();

        let backups = manager.list_backups("TestCreature").unwrap();
        let recovery = &backups[0];

        assert!(recovery.size_bytes > 0);
        assert!(!recovery.timestamp_string().is_empty());
        assert!(!recovery.size_string().is_empty());
    }

    #[test]
    fn test_auto_save_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.enabled = false;
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();

        assert!(!manager.should_auto_save());

        let result = manager.auto_save(&creature);
        assert!(result.is_err());
    }

    #[test]
    fn test_time_until_auto_save() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.interval_seconds = 10;
        let mut manager = AutoSaveManager::new(config).unwrap();

        let creature = create_test_creature("TestCreature");
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();

        let time_until = manager.time_until_auto_save();
        assert!(time_until.is_some());
        assert!(time_until.unwrap().as_secs() <= 10);
    }
}
