//! Asset Manager Module
//!
//! This module provides functionality for managing campaign assets including
//! images, sounds, music, tilesets, and other external files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Reference to an asset from campaign data
#[derive(Debug, Clone, PartialEq)]
pub enum AssetReference {
    /// Referenced by an item
    Item {
        id: antares::domain::types::ItemId,
        name: String,
    },
    /// Referenced by a spell
    Spell {
        id: antares::domain::types::SpellId,
        name: String,
    },
    /// Referenced by a monster
    Monster {
        id: antares::domain::types::MonsterId,
        name: String,
    },
    /// Referenced by a map
    Map {
        id: antares::domain::types::MapId,
        name: String,
    },
    /// Referenced by a quest
    Quest {
        id: antares::domain::quest::QuestId,
        name: String,
    },
    /// Referenced by a dialogue
    Dialogue {
        id: antares::domain::dialogue::DialogueId,
        name: String,
    },
}

impl AssetReference {
    /// Gets a display string for the reference
    pub fn display_string(&self) -> String {
        match self {
            AssetReference::Item { id, name } => format!("Item #{}: {}", id, name),
            AssetReference::Spell { id, name } => format!("Spell #{}: {}", id, name),
            AssetReference::Monster { id, name } => format!("Monster #{}: {}", id, name),
            AssetReference::Map { id, name } => format!("Map #{}: {}", id, name),
            AssetReference::Quest { id, name } => format!("Quest #{}: {}", id, name),
            AssetReference::Dialogue { id, name } => format!("Dialogue #{}: {}", id, name),
        }
    }

    /// Gets the category of the reference
    pub fn category(&self) -> &str {
        match self {
            AssetReference::Item { .. } => "Item",
            AssetReference::Spell { .. } => "Spell",
            AssetReference::Monster { .. } => "Monster",
            AssetReference::Map { .. } => "Map",
            AssetReference::Quest { .. } => "Quest",
            AssetReference::Dialogue { .. } => "Dialogue",
        }
    }
}

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
    pub modified: SystemTime,
    /// Whether asset is referenced by campaign data
    pub is_referenced: bool,
    /// List of references to this asset from campaign data
    pub references: Vec<AssetReference>,
}

impl Asset {
    /// Creates a new asset
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path within campaign
    ///
    /// # Returns
    ///
    /// Returns a new Asset instance
    pub fn new(path: PathBuf) -> Self {
        Self {
            asset_type: AssetType::from_path(&path),
            path,
            size: 0,
            modified: SystemTime::now(),
            is_referenced: false,
            references: Vec::new(),
        }
    }

    /// Updates asset metadata from filesystem
    ///
    /// # Arguments
    ///
    /// * `full_path` - Full filesystem path to the asset
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if metadata updated successfully
    pub fn update_metadata(&mut self, full_path: &Path) -> Result<(), std::io::Error> {
        let metadata = std::fs::metadata(full_path)?;
        self.size = metadata.len();
        self.modified = metadata.modified().unwrap_or_else(|_| SystemTime::now());
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

    /// Scans campaign data to identify which assets are referenced
    ///
    /// This method examines items, spells, monsters, maps, quests, dialogues, and classes
    /// to determine which assets are actively used in the campaign.
    ///
    /// # Arguments
    ///
    /// * `items` - List of campaign items
    /// * `quests` - List of campaign quests
    /// * `dialogues` - List of campaign dialogues
    /// * `maps` - List of campaign maps
    /// * `classes` - List of campaign classes
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::types::Item;
    /// use antares::domain::quest::Quest;
    /// use antares::domain::dialogue::DialogueTree;
    /// use antares::domain::world::Map;
    /// use antares::domain::classes::ClassDefinition;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = campaign_builder::asset_manager::AssetManager::new(PathBuf::from("/tmp/campaign"));
    /// let items = vec![];
    /// let quests = vec![];
    /// let dialogues = vec![];
    /// let maps = vec![];
    /// let classes = vec![];
    /// manager.scan_references(&items, &quests, &dialogues, &maps, &classes);
    /// ```
    pub fn scan_references(
        &mut self,
        items: &[antares::domain::items::types::Item],
        quests: &[antares::domain::quest::Quest],
        dialogues: &[antares::domain::dialogue::DialogueTree],
        maps: &[antares::domain::world::Map],
        classes: &[antares::domain::classes::ClassDefinition],
    ) {
        // Reset all reference tracking
        for asset in self.assets.values_mut() {
            asset.is_referenced = false;
            asset.references.clear();
        }

        // Scan items for asset references (e.g., item icons, graphics)
        self.scan_items_references(items);

        // Scan quests for asset references (e.g., quest icons, rewards)
        self.scan_quests_references(quests);

        // Scan dialogues for asset references (e.g., portrait images)
        self.scan_dialogues_references(dialogues);

        // Scan maps for asset references (e.g., tilesets, backgrounds)
        self.scan_maps_references(maps);

        // Scan classes for asset references (e.g., class icons)
        self.scan_classes_references(classes);
    }

    /// Scans items for asset references
    fn scan_items_references(&mut self, items: &[antares::domain::items::types::Item]) {
        for item in items {
            // Items might reference assets through their names or future icon fields
            // For now, we scan for potential asset path patterns in item names
            // In a real implementation, items would have explicit asset_path fields

            // Check explicit icon path if available
            if let Some(icon_path) = &item.icon_path {
                let path = PathBuf::from(icon_path);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    asset.references.push(AssetReference::Item {
                        id: item.id,
                        name: item.name.clone(),
                    });
                }
            }

            // Heuristic fallback: Check if item name suggests an asset file
            let potential_paths = vec![
                format!("items/{}.png", item.name.to_lowercase().replace(' ', "_")),
                format!("icons/{}.png", item.name.to_lowercase().replace(' ', "_")),
                format!(
                    "assets/items/{}.png",
                    item.name.to_lowercase().replace(' ', "_")
                ),
                format!(
                    "assets/icons/{}.png",
                    item.name.to_lowercase().replace(' ', "_")
                ),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    asset.references.push(AssetReference::Item {
                        id: item.id,
                        name: item.name.clone(),
                    });
                }
            }
        }
    }

    /// Scans quests for asset references
    fn scan_quests_references(&mut self, quests: &[antares::domain::quest::Quest]) {
        for quest in quests {
            // Quests might reference portrait images, icons, or other assets
            // Similar to items, this would check explicit asset fields in a real implementation

            let potential_paths = vec![
                format!("quests/{}.png", quest.name.to_lowercase().replace(' ', "_")),
                format!(
                    "portraits/{}.png",
                    quest.name.to_lowercase().replace(' ', "_")
                ),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    asset.references.push(AssetReference::Quest {
                        id: quest.id,
                        name: quest.name.clone(),
                    });
                }
            }
        }
    }

    /// Scans dialogues for asset references
    fn scan_dialogues_references(&mut self, dialogues: &[antares::domain::dialogue::DialogueTree]) {
        for dialogue in dialogues {
            // Dialogues typically reference portrait images for speakers
            // Check for portrait assets based on dialogue speaker or name

            let potential_paths = vec![
                format!(
                    "portraits/{}.png",
                    dialogue.name.to_lowercase().replace(' ', "_")
                ),
                format!("npc/{}.png", dialogue.name.to_lowercase().replace(' ', "_")),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    asset.references.push(AssetReference::Dialogue {
                        id: dialogue.id,
                        name: dialogue.name.clone(),
                    });
                }
            }
        }
    }

    /// Scans maps for asset references (tilesets, backgrounds, etc.)
    fn scan_maps_references(&mut self, maps: &[antares::domain::world::Map]) {
        for map in maps {
            // Maps may reference tileset images and background images
            // Check for common naming patterns based on map ID

            let potential_paths = vec![
                format!("tilesets/map_{}.png", map.id),
                format!("assets/tilesets/map_{}.png", map.id),
                format!("backgrounds/map_{}.png", map.id),
                format!("assets/backgrounds/map_{}.png", map.id),
                // Common tileset names
                format!("tilesets/dungeon.png"),
                format!("tilesets/town.png"),
                format!("tilesets/wilderness.png"),
                format!("assets/tilesets/dungeon.png"),
                format!("assets/tilesets/town.png"),
                format!("assets/tilesets/wilderness.png"),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    // Only add reference if not already added
                    if !asset
                        .references
                        .iter()
                        .any(|r| matches!(r, AssetReference::Map { id, .. } if *id == map.id))
                    {
                        asset.references.push(AssetReference::Map {
                            id: map.id,
                            name: format!("Map {}", map.id),
                        });
                    }
                }
            }
        }
    }

    /// Scans classes for asset references (class icons, etc.)
    fn scan_classes_references(&mut self, classes: &[antares::domain::classes::ClassDefinition]) {
        for class in classes {
            // Classes may reference icon images
            // Check for icon assets based on class ID and name

            let potential_paths = vec![
                format!("classes/{}.png", class.id),
                format!("icons/classes/{}.png", class.id),
                format!("assets/classes/{}.png", class.id),
                format!("assets/icons/classes/{}.png", class.id),
                format!(
                    "classes/{}.png",
                    class.name.to_lowercase().replace(' ', "_")
                ),
                format!(
                    "icons/classes/{}.png",
                    class.name.to_lowercase().replace(' ', "_")
                ),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    // Only add reference if not already added for this class
                    if !asset.references.iter().any(|r| {
                        if let AssetReference::Item { name, .. } = r {
                            name == &class.name
                        } else {
                            false
                        }
                    }) {
                        // Use Item reference as a placeholder (we could add a Class variant to AssetReference if needed)
                        asset.references.push(AssetReference::Item {
                            id: 0, // Classes don't have numeric IDs
                            name: format!("Class: {}", class.name),
                        });
                    }
                }
            }
        }
    }

    /// Gets assets that are candidates for cleanup (unreferenced and non-essential)
    ///
    /// # Returns
    ///
    /// Returns a vector of asset paths that are not referenced by any campaign data
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// let manager = campaign_builder::asset_manager::AssetManager::new(PathBuf::from("/tmp/campaign"));
    /// let candidates = manager.get_cleanup_candidates();
    /// assert_eq!(candidates.len(), 0);
    /// ```
    pub fn get_cleanup_candidates(&self) -> Vec<&PathBuf> {
        self.unreferenced_assets()
            .iter()
            .filter(|asset| {
                // Don't suggest cleanup for documentation or essential files
                !matches!(asset.asset_type, AssetType::Documentation | AssetType::Data)
            })
            .map(|asset| &asset.path)
            .collect()
    }

    /// Removes unreferenced assets from the campaign
    ///
    /// # Arguments
    ///
    /// * `dry_run` - If true, returns list of files that would be deleted without deleting them
    ///
    /// # Returns
    ///
    /// Returns the list of paths that were (or would be) deleted
    ///
    /// # Errors
    ///
    /// Returns error if file deletion fails
    pub fn cleanup_unused(&mut self, dry_run: bool) -> Result<Vec<PathBuf>, std::io::Error> {
        // Collect paths to owned PathBufs to avoid borrow issues
        let candidates: Vec<PathBuf> = self
            .get_cleanup_candidates()
            .iter()
            .map(|p| (*p).clone())
            .collect();
        let mut deleted = Vec::new();

        for path in candidates {
            if dry_run {
                deleted.push(path.clone());
            } else {
                // Actually delete the file
                self.remove_asset(&path)?;
                deleted.push(path);
            }
        }

        Ok(deleted)
    }

    /// Gets reference context for a specific asset
    ///
    /// # Arguments
    ///
    /// * `asset_path` - Path to the asset
    ///
    /// # Returns
    ///
    /// Returns the list of references to this asset
    pub fn get_asset_references(&self, asset_path: &Path) -> Vec<AssetReference> {
        self.assets
            .get(asset_path)
            .map(|asset| asset.references.clone())
            .unwrap_or_default()
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
    }

    #[test]
    fn test_asset_reference_display_string() {
        use antares::domain::types::ItemId;
        let item_ref = AssetReference::Item {
            id: 42 as ItemId,
            name: "Magic Sword".to_string(),
        };
        assert_eq!(item_ref.display_string(), "Item #42: Magic Sword");
        assert_eq!(item_ref.category(), "Item");
    }

    #[test]
    fn test_asset_reference_category() {
        use antares::domain::quest::QuestId;
        let quest_ref = AssetReference::Quest {
            id: 10 as QuestId,
            name: "Save the Kingdom".to_string(),
        };
        assert_eq!(quest_ref.category(), "Quest");
    }

    #[test]
    fn test_scan_references_marks_assets_referenced() {
        use antares::domain::items::types::{Disablement, Item, ItemType};
        use antares::domain::types::ItemId;
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add a mock asset
        let asset_path = PathBuf::from("items/longsword.png");
        manager.assets.insert(
            asset_path.clone(),
            Asset {
                path: asset_path.clone(),
                asset_type: AssetType::Portrait,
                size: 1024,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        // Create an item with a matching name pattern
        use antares::domain::types::DiceRoll;
        let item = Item {
            id: 1 as ItemId,
            name: "Longsword".to_string(),
            item_type: ItemType::Weapon(antares::domain::items::WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 100,
            sell_cost: 50,
            disablements: Disablement::NONE,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        // Scan references
        manager.scan_references(&[item], &[], &[], &[], &[]);

        // Check that the asset was marked as referenced
        let asset = manager.assets.get(&asset_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
    }

    #[test]
    fn test_get_cleanup_candidates_excludes_docs() {
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add unreferenced documentation asset
        let doc_path = PathBuf::from("README.md");
        manager.assets.insert(
            doc_path.clone(),
            Asset {
                path: doc_path.clone(),
                asset_type: AssetType::Documentation,
                size: 512,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        // Add unreferenced image asset
        let img_path = PathBuf::from("unused.png");
        manager.assets.insert(
            img_path.clone(),
            Asset {
                path: img_path.clone(),
                asset_type: AssetType::Portrait,
                size: 2048,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let candidates = manager.get_cleanup_candidates();

        // Should include image but not documentation
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], &img_path);
    }

    #[test]
    fn test_cleanup_unused_dry_run() {
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add unreferenced asset
        let img_path = PathBuf::from("unused.png");
        manager.assets.insert(
            img_path.clone(),
            Asset {
                path: img_path.clone(),
                asset_type: AssetType::Portrait,
                size: 2048,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        // Dry run should return candidates without deleting
        let deleted = manager.cleanup_unused(true).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], img_path);

        // Asset should still exist in manager
        assert!(manager.assets.contains_key(&img_path));
    }

    #[test]
    fn test_get_asset_references_empty() {
        use std::path::PathBuf;

        let manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));
        let refs = manager.get_asset_references(&PathBuf::from("nonexistent.png"));
        assert!(refs.is_empty());
    }

    #[test]
    fn test_get_asset_references_with_refs() {
        use antares::domain::dialogue::DialogueId;
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        let portrait_path = PathBuf::from("portraits/npc.png");
        let reference = AssetReference::Dialogue {
            id: 5 as DialogueId,
            name: "Merchant Chat".to_string(),
        };

        manager.assets.insert(
            portrait_path.clone(),
            Asset {
                path: portrait_path.clone(),
                asset_type: AssetType::Portrait,
                size: 4096,
                modified: SystemTime::now(),
                is_referenced: true,
                references: vec![reference.clone()],
            },
        );

        let refs = manager.get_asset_references(&portrait_path);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], reference);
    }

    #[test]
    fn test_scan_quests_references() {
        use antares::domain::quest::{Quest, QuestId, QuestStage};
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        let quest_icon_path = PathBuf::from("quests/dragon_hunt.png");
        manager.assets.insert(
            quest_icon_path.clone(),
            Asset {
                path: quest_icon_path.clone(),
                asset_type: AssetType::Portrait,
                size: 2048,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let quest = Quest {
            id: 7 as QuestId,
            name: "Dragon Hunt".to_string(),
            description: "Slay the dragon".to_string(),
            stages: vec![QuestStage {
                stage_number: 1,
                name: "Find Dragon".to_string(),
                description: "Find the dragon".to_string(),
                objectives: vec![],
                require_all_objectives: true,
            }],
            rewards: vec![],
            repeatable: false,
            min_level: Some(10),
            max_level: Some(99),
            required_quests: vec![],
            is_main_quest: false,
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        manager.scan_references(&[], &[quest], &[], &[], &[]);

        let asset = manager.assets.get(&quest_icon_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "Quest");
    }

    #[test]
    fn test_scan_dialogues_references() {
        use antares::domain::dialogue::{DialogueId, DialogueNode, DialogueTree, NodeId};
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        let portrait_path = PathBuf::from("portraits/wise_sage.png");
        manager.assets.insert(
            portrait_path.clone(),
            Asset {
                path: portrait_path.clone(),
                asset_type: AssetType::Portrait,
                size: 3072,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        use std::collections::HashMap;
        let mut nodes = HashMap::new();
        nodes.insert(
            1 as NodeId,
            DialogueNode {
                id: 1 as NodeId,
                text: "Greetings, traveler.".to_string(),
                speaker_override: Some("Sage".to_string()),
                choices: vec![],
                conditions: vec![],
                actions: vec![],
                is_terminal: true,
            },
        );

        let dialogue = DialogueTree {
            id: 3 as DialogueId,
            name: "Wise Sage".to_string(),
            speaker_name: Some("Sage".to_string()),
            root_node: 1 as NodeId,
            nodes,
            repeatable: false,
            associated_quest: None,
        };

        manager.scan_references(&[], &[], &[dialogue], &[], &[]);

        let asset = manager.assets.get(&portrait_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "Dialogue");
    }
}
