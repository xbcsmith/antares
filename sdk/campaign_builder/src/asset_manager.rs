//! Asset Manager Module
//!
//! This module provides functionality for managing campaign assets including
//! images, sounds, music, tilesets, and other external files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Asset type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    /// Tileset images for maps
    Tileset,
    /// Character/NPC portrait images
    Portrait,
    /// Background music files
    Music,
    /// Sound effect files
    Sound,
    /// Documentation files
    Documentation,
    /// Custom data files
    Data,
    /// Other/unknown asset type
    Other,
}

impl AssetType {
    /// Gets the display name for the asset type
    pub fn display_name(&self) -> &str {
        match self {
            AssetType::Tileset => "Tileset",
            AssetType::Portrait => "Portrait",
            AssetType::Music => "Music",
            AssetType::Sound => "Sound Effect",
            AssetType::Documentation => "Documentation",
            AssetType::Data => "Data File",
            AssetType::Other => "Other",
        }
    }

    /// Gets the suggested subdirectory for this asset type
    pub fn subdirectory(&self) -> &str {
        match self {
            AssetType::Tileset => "assets/tilesets",
            AssetType::Portrait => "assets/portraits",
            AssetType::Music => "assets/music",
            AssetType::Sound => "assets/sounds",
            AssetType::Documentation => "docs",
            AssetType::Data => "data",
            AssetType::Other => "assets",
        }
    }

    /// Determines asset type from file extension
    ///
    /// # Arguments
    ///
    /// * `path` - File path to analyze
    ///
    /// # Returns
    ///
    /// Returns the detected asset type
    pub fn from_path(path: &Path) -> Self {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "png" | "jpg" | "jpeg" | "bmp" | "gif" => {
                    // Check parent directory for hints
                    if let Some(parent) = path.parent() {
                        if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                            if parent_name.contains("tileset") {
                                return AssetType::Tileset;
                            } else if parent_name.contains("portrait") {
                                return AssetType::Portrait;
                            }
                        }
                    }
                    AssetType::Tileset // Default for images
                }
                "mp3" | "ogg" | "wav" | "flac" | "midi" | "mid" => {
                    if let Some(parent) = path.parent() {
                        if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                            if parent_name.contains("sound") || parent_name.contains("sfx") {
                                return AssetType::Sound;
                            }
                        }
                    }
                    AssetType::Music // Default for audio
                }
                "md" | "txt" | "pdf" => AssetType::Documentation,
                "ron" | "yaml" | "yml" | "json" | "toml" => AssetType::Data,
                _ => AssetType::Other,
            }
        } else {
            AssetType::Other
        }
    }

    /// Gets all asset types as a list
    pub fn all() -> Vec<AssetType> {
        vec![
            AssetType::Tileset,
            AssetType::Portrait,
            AssetType::Music,
            AssetType::Sound,
            AssetType::Documentation,
            AssetType::Data,
            AssetType::Other,
        ]
    }
}

/// Represents a single asset file
#[derive(Debug, Clone)]
pub struct Asset {
    /// Relative path within campaign directory
    pub path: PathBuf,
    /// Asset type classification
    pub asset_type: AssetType,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: Option<std::time::SystemTime>,
    /// Whether asset is referenced by campaign data
    pub is_referenced: bool,
}

impl Asset {
    /// Creates a new asset from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path within campaign directory
    ///
    /// # Returns
    ///
    /// Returns a new Asset instance
    pub fn new(path: PathBuf) -> Self {
        let asset_type = AssetType::from_path(&path);
        Self {
            path,
            asset_type,
            size: 0,
            modified: None,
            is_referenced: false,
        }
    }

    /// Updates asset metadata from filesystem
    ///
    /// # Arguments
    ///
    /// * `full_path` - Absolute path to the asset file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful
    pub fn update_metadata(&mut self, full_path: &Path) -> Result<(), std::io::Error> {
        let metadata = std::fs::metadata(full_path)?;
        self.size = metadata.len();
        self.modified = metadata.modified().ok();
        Ok(())
    }

    /// Gets a human-readable size string
    pub fn size_string(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Asset manager for organizing and tracking campaign assets
#[derive(Debug, Clone)]
pub struct AssetManager {
    /// Campaign directory root
    campaign_dir: PathBuf,
    /// Map of asset paths to asset information
    assets: HashMap<PathBuf, Asset>,
    /// Total size of all assets
    total_size: u64,
}

impl AssetManager {
    /// Creates a new asset manager
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` - Root directory of the campaign
    pub fn new(campaign_dir: PathBuf) -> Self {
        Self {
            campaign_dir,
            assets: HashMap::new(),
            total_size: 0,
        }
    }

    /// Scans the campaign directory for assets
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if scan completed successfully
    pub fn scan_directory(&mut self) -> Result<(), std::io::Error> {
        self.assets.clear();
        self.total_size = 0;

        self.scan_recursive(&self.campaign_dir.clone(), &self.campaign_dir.clone())?;

        Ok(())
    }

    /// Recursively scans a directory for assets
    fn scan_recursive(
        &mut self,
        base_dir: &Path,
        current_dir: &Path,
    ) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Get relative path
                if let Ok(relative) = path.strip_prefix(base_dir) {
                    let mut asset = Asset::new(relative.to_path_buf());
                    asset.update_metadata(&path)?;
                    self.total_size += asset.size;
                    self.assets.insert(relative.to_path_buf(), asset);
                }
            } else if path.is_dir() {
                // Skip hidden directories and common ignores
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.') && name != "target" && name != "node_modules" {
                        self.scan_recursive(base_dir, &path)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Adds a new asset by copying a file into the campaign directory
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to the file to copy
    /// * `dest_subdir` - Subdirectory within campaign to place file
    ///
    /// # Returns
    ///
    /// Returns relative path of the added asset
    pub fn add_asset(
        &mut self,
        source_path: &Path,
        dest_subdir: &str,
    ) -> Result<PathBuf, std::io::Error> {
        // Ensure destination directory exists
        let dest_dir = self.campaign_dir.join(dest_subdir);
        std::fs::create_dir_all(&dest_dir)?;

        // Copy file
        let file_name = source_path.file_name().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file name")
        })?;
        let dest_path = dest_dir.join(file_name);
        std::fs::copy(source_path, &dest_path)?;

        // Add to assets
        let relative = dest_path
            .strip_prefix(&self.campaign_dir)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))?
            .to_path_buf();

        let mut asset = Asset::new(relative.clone());
        asset.update_metadata(&dest_path)?;
        self.total_size += asset.size;
        self.assets.insert(relative.clone(), asset);

        Ok(relative)
    }

    /// Removes an asset from the campaign
    ///
    /// # Arguments
    ///
    /// * `asset_path` - Relative path of the asset to remove
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if removed successfully
    pub fn remove_asset(&mut self, asset_path: &Path) -> Result<(), std::io::Error> {
        let full_path = self.campaign_dir.join(asset_path);
        std::fs::remove_file(full_path)?;

        if let Some(asset) = self.assets.remove(asset_path) {
            self.total_size = self.total_size.saturating_sub(asset.size);
        }

        Ok(())
    }

    /// Moves an asset to a different subdirectory
    ///
    /// # Arguments
    ///
    /// * `asset_path` - Current relative path of the asset
    /// * `dest_subdir` - Destination subdirectory
    ///
    /// # Returns
    ///
    /// Returns new relative path
    pub fn move_asset(
        &mut self,
        asset_path: &Path,
        dest_subdir: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let source_full = self.campaign_dir.join(asset_path);
        let dest_dir = self.campaign_dir.join(dest_subdir);
        std::fs::create_dir_all(&dest_dir)?;

        let file_name = asset_path.file_name().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file name")
        })?;
        let dest_full = dest_dir.join(file_name);

        std::fs::rename(&source_full, &dest_full)?;

        // Update assets map
        if let Some(mut asset) = self.assets.remove(asset_path) {
            let new_relative = dest_full
                .strip_prefix(&self.campaign_dir)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))?
                .to_path_buf();

            asset.path = new_relative.clone();
            asset.update_metadata(&dest_full)?;
            self.assets.insert(new_relative.clone(), asset);

            Ok(new_relative)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Asset not found",
            ))
        }
    }

    /// Gets all assets
    pub fn assets(&self) -> &HashMap<PathBuf, Asset> {
        &self.assets
    }

    /// Gets assets of a specific type
    pub fn assets_by_type(&self, asset_type: AssetType) -> Vec<&Asset> {
        self.assets
            .values()
            .filter(|a| a.asset_type == asset_type)
            .collect()
    }

    /// Gets total size of all assets
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Gets total size as human-readable string
    pub fn total_size_string(&self) -> String {
        if self.total_size < 1024 {
            format!("{} B", self.total_size)
        } else if self.total_size < 1024 * 1024 {
            format!("{:.1} KB", self.total_size as f64 / 1024.0)
        } else if self.total_size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.total_size as f64 / (1024.0 * 1024.0))
        } else {
            format!(
                "{:.1} GB",
                self.total_size as f64 / (1024.0 * 1024.0 * 1024.0)
            )
        }
    }

    /// Gets unreferenced assets (assets not used by campaign data)
    pub fn unreferenced_assets(&self) -> Vec<&Asset> {
        self.assets.values().filter(|a| !a.is_referenced).collect()
    }

    /// Marks an asset as referenced
    pub fn mark_referenced(&mut self, asset_path: &Path, referenced: bool) {
        if let Some(asset) = self.assets.get_mut(asset_path) {
            asset.is_referenced = referenced;
        }
    }

    /// Gets asset count
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Gets asset count by type
    pub fn asset_count_by_type(&self, asset_type: AssetType) -> usize {
        self.assets
            .values()
            .filter(|a| a.asset_type == asset_type)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_display_names() {
        assert_eq!(AssetType::Tileset.display_name(), "Tileset");
        assert_eq!(AssetType::Portrait.display_name(), "Portrait");
        assert_eq!(AssetType::Music.display_name(), "Music");
        assert_eq!(AssetType::Sound.display_name(), "Sound Effect");
    }

    #[test]
    fn test_asset_type_subdirectories() {
        assert_eq!(AssetType::Tileset.subdirectory(), "assets/tilesets");
        assert_eq!(AssetType::Portrait.subdirectory(), "assets/portraits");
        assert_eq!(AssetType::Music.subdirectory(), "assets/music");
        assert_eq!(AssetType::Sound.subdirectory(), "assets/sounds");
        assert_eq!(AssetType::Documentation.subdirectory(), "docs");
        assert_eq!(AssetType::Data.subdirectory(), "data");
    }

    #[test]
    fn test_asset_type_from_path_images() {
        assert_eq!(
            AssetType::from_path(Path::new("tilesets/dungeon.png")),
            AssetType::Tileset
        );
        assert_eq!(
            AssetType::from_path(Path::new("portraits/hero.jpg")),
            AssetType::Portrait
        );
        assert_eq!(
            AssetType::from_path(Path::new("image.bmp")),
            AssetType::Tileset
        );
    }

    #[test]
    fn test_asset_type_from_path_audio() {
        assert_eq!(
            AssetType::from_path(Path::new("music/battle.mp3")),
            AssetType::Music
        );
        assert_eq!(
            AssetType::from_path(Path::new("sounds/hit.wav")),
            AssetType::Sound
        );
        assert_eq!(
            AssetType::from_path(Path::new("sfx/explosion.ogg")),
            AssetType::Sound
        );
    }

    #[test]
    fn test_asset_type_from_path_data() {
        assert_eq!(
            AssetType::from_path(Path::new("data/items.ron")),
            AssetType::Data
        );
        assert_eq!(
            AssetType::from_path(Path::new("config.yaml")),
            AssetType::Data
        );
        assert_eq!(
            AssetType::from_path(Path::new("settings.json")),
            AssetType::Data
        );
    }

    #[test]
    fn test_asset_type_from_path_docs() {
        assert_eq!(
            AssetType::from_path(Path::new("README.md")),
            AssetType::Documentation
        );
        assert_eq!(
            AssetType::from_path(Path::new("guide.txt")),
            AssetType::Documentation
        );
    }

    #[test]
    fn test_asset_type_from_path_other() {
        assert_eq!(
            AssetType::from_path(Path::new("unknown.xyz")),
            AssetType::Other
        );
        assert_eq!(
            AssetType::from_path(Path::new("noextension")),
            AssetType::Other
        );
    }

    #[test]
    fn test_asset_type_all() {
        let types = AssetType::all();
        assert_eq!(types.len(), 7);
        assert!(types.contains(&AssetType::Tileset));
        assert!(types.contains(&AssetType::Music));
    }

    #[test]
    fn test_asset_creation() {
        let asset = Asset::new(PathBuf::from("test.png"));
        assert_eq!(asset.path, PathBuf::from("test.png"));
        assert_eq!(asset.asset_type, AssetType::Tileset);
        assert_eq!(asset.size, 0);
        assert!(!asset.is_referenced);
    }

    #[test]
    fn test_asset_size_string() {
        let mut asset = Asset::new(PathBuf::from("test.png"));

        asset.size = 500;
        assert_eq!(asset.size_string(), "500 B");

        asset.size = 1536; // 1.5 KB
        assert_eq!(asset.size_string(), "1.5 KB");

        asset.size = 1024 * 1024 * 2; // 2 MB
        assert_eq!(asset.size_string(), "2.0 MB");

        asset.size = 1024 * 1024 * 1024 * 3; // 3 GB
        assert_eq!(asset.size_string(), "3.0 GB");
    }

    #[test]
    fn test_asset_manager_creation() {
        let manager = AssetManager::new(PathBuf::from("/tmp/campaign"));
        assert_eq!(manager.asset_count(), 0);
        assert_eq!(manager.total_size(), 0);
    }

    #[test]
    fn test_asset_manager_total_size_string() {
        let mut manager = AssetManager::new(PathBuf::from("/tmp/campaign"));
        manager.total_size = 1024 * 500; // 500 KB
        assert!(manager.total_size_string().contains("KB"));
    }

    #[test]
    fn test_asset_manager_mark_referenced() {
        let mut manager = AssetManager::new(PathBuf::from("/tmp/campaign"));
        let path = PathBuf::from("test.png");
        let asset = Asset::new(path.clone());
        manager.assets.insert(path.clone(), asset);

        assert!(!manager.assets.get(&path).unwrap().is_referenced);

        manager.mark_referenced(&path, true);
        assert!(manager.assets.get(&path).unwrap().is_referenced);

        manager.mark_referenced(&path, false);
        assert!(!manager.assets.get(&path).unwrap().is_referenced);
    }

    #[test]
    fn test_asset_manager_unreferenced_assets() {
        let mut manager = AssetManager::new(PathBuf::from("/tmp/campaign"));

        let path1 = PathBuf::from("test1.png");
        let mut asset1 = Asset::new(path1.clone());
        asset1.is_referenced = true;
        manager.assets.insert(path1, asset1);

        let path2 = PathBuf::from("test2.png");
        let asset2 = Asset::new(path2.clone());
        manager.assets.insert(path2, asset2);

        let unreferenced = manager.unreferenced_assets();
        assert_eq!(unreferenced.len(), 1);
        assert_eq!(unreferenced[0].path, PathBuf::from("test2.png"));
    }

    #[test]
    fn test_asset_manager_assets_by_type() {
        let mut manager = AssetManager::new(PathBuf::from("/tmp/campaign"));

        manager.assets.insert(
            PathBuf::from("tile1.png"),
            Asset::new(PathBuf::from("tile1.png")),
        );
        manager.assets.insert(
            PathBuf::from("music.mp3"),
            Asset::new(PathBuf::from("music.mp3")),
        );
        manager.assets.insert(
            PathBuf::from("tile2.png"),
            Asset::new(PathBuf::from("tile2.png")),
        );

        let tilesets = manager.assets_by_type(AssetType::Tileset);
        assert_eq!(tilesets.len(), 2);

        let music = manager.assets_by_type(AssetType::Music);
        assert_eq!(music.len(), 1);
    }

    #[test]
    fn test_asset_manager_asset_count_by_type() {
        let mut manager = AssetManager::new(PathBuf::from("/tmp/campaign"));

        manager.assets.insert(
            PathBuf::from("tile.png"),
            Asset::new(PathBuf::from("tile.png")),
        );
        manager.assets.insert(
            PathBuf::from("sound.wav"),
            Asset::new(PathBuf::from("sounds/sound.wav")),
        );

        assert_eq!(manager.asset_count_by_type(AssetType::Tileset), 1);
        assert_eq!(manager.asset_count_by_type(AssetType::Sound), 1);
        assert_eq!(manager.asset_count_by_type(AssetType::Music), 0);
    }

    #[test]
    fn test_asset_type_equality() {
        assert_eq!(AssetType::Tileset, AssetType::Tileset);
        assert_ne!(AssetType::Tileset, AssetType::Music);
    }
}
