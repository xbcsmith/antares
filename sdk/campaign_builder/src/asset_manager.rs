//! Asset Manager Module
//!
//! This module provides functionality for managing campaign assets including
//! images, sounds, music, tilesets, and other external files.
//!
//! # Data File Status Tracking
//!
//! The asset manager also tracks the load status of campaign data files
//! (items.ron, spells.ron, etc.) to display accurate status in the Assets panel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Status of a campaign data file (items.ron, spells.ron, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFileStatus {
    /// File not yet loaded
    NotLoaded,
    /// File loaded successfully
    Loaded,
    /// File exists but failed to load/parse
    Error,
    /// File does not exist
    Missing,
}

impl DataFileStatus {
    /// Returns the display icon for the status.
    pub fn icon(&self) -> &'static str {
        match self {
            DataFileStatus::NotLoaded => "⏳",
            DataFileStatus::Loaded => "✅",
            DataFileStatus::Error => "❌",
            DataFileStatus::Missing => "⚠️",
        }
    }

    /// Returns the display name for the status.
    pub fn display_name(&self) -> &'static str {
        match self {
            DataFileStatus::NotLoaded => "Not Loaded",
            DataFileStatus::Loaded => "Loaded",
            DataFileStatus::Error => "Load Error",
            DataFileStatus::Missing => "Missing",
        }
    }

    /// Returns the color for displaying this status.
    pub fn color(&self) -> eframe::egui::Color32 {
        match self {
            DataFileStatus::NotLoaded => eframe::egui::Color32::GRAY,
            DataFileStatus::Loaded => eframe::egui::Color32::from_rgb(80, 200, 80),
            DataFileStatus::Error => eframe::egui::Color32::from_rgb(255, 80, 80),
            DataFileStatus::Missing => eframe::egui::Color32::from_rgb(255, 180, 0),
        }
    }
}

/// Information about a campaign data file
#[derive(Debug, Clone)]
pub struct DataFileInfo {
    /// Relative path to the file
    pub path: PathBuf,
    /// Display name for the file type (e.g., "Items", "Spells")
    pub display_name: String,
    /// Current load status
    pub status: DataFileStatus,
    /// Number of entries loaded (if applicable)
    pub entry_count: Option<usize>,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

impl DataFileInfo {
    /// Creates a new data file info entry.
    pub fn new(path: impl Into<PathBuf>, display_name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            display_name: display_name.into(),
            status: DataFileStatus::NotLoaded,
            entry_count: None,
            error_message: None,
        }
    }

    /// Marks the file as loaded with a count of entries.
    pub fn mark_loaded(&mut self, count: usize) {
        self.status = DataFileStatus::Loaded;
        self.entry_count = Some(count);
        self.error_message = None;
    }

    /// Marks the file as having an error.
    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.status = DataFileStatus::Error;
        self.error_message = Some(message.into());
    }

    /// Marks the file as missing.
    pub fn mark_missing(&mut self) {
        self.status = DataFileStatus::Missing;
        self.error_message = None;
    }
}

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
    /// Referenced by a character
    Character { id: String, name: String },
    /// Referenced by an NPC
    Npc { id: String, name: String },
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
            AssetReference::Character { id, name } => format!("Character {}: {}", id, name),
            AssetReference::Npc { id, name } => format!("NPC {}: {}", id, name),
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
            AssetReference::Character { .. } => "Character",
            AssetReference::Npc { .. } => "NPC",
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
    /// Tracked data files and their load status
    data_files: Vec<DataFileInfo>,
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
            data_files: Vec::new(),
        }
    }

    /// Initializes data file tracking with the campaign's configured file paths.
    /// Initializes data file tracking for all campaign data files.
    ///
    /// # Arguments
    ///
    /// * `items_file` - Path to items data file
    /// * `spells_file` - Path to spells data file
    /// * `conditions_file` - Path to conditions data file
    /// * `monsters_file` - Path to monsters data file
    /// * `map_file_paths` - List of individual map file paths
    /// * `quests_file` - Path to quests data file
    /// * `classes_file` - Path to classes data file
    /// * `races_file` - Path to races data file
    /// * `characters_file` - Path to characters data file
    /// * `dialogue_file` - Path to dialogues data file
    /// * `npcs_file` - Path to NPCs data file
    /// * `proficiencies_file` - Path to proficiencies data file
    #[allow(clippy::too_many_arguments)]
    pub fn init_data_files(
        &mut self,
        items_file: &str,
        spells_file: &str,
        conditions_file: &str,
        monsters_file: &str,
        maps_file_list: &[String],
        quests_file: &str,
        classes_file: &str,
        races_file: &str,
        characters_file: &str,
        dialogue_file: &str,
        npcs_file: &str,
        proficiencies_file: &str,
    ) {
        self.data_files.clear();

        // Add data files in EditorTab order: Items, Spells, Conditions, Monsters, Maps, Quests, Classes, Races, Characters, Dialogues, NPCs, Proficiencies
        self.data_files.push(DataFileInfo::new(items_file, "Items"));
        self.data_files
            .push(DataFileInfo::new(spells_file, "Spells"));
        self.data_files
            .push(DataFileInfo::new(conditions_file, "Conditions"));
        self.data_files
            .push(DataFileInfo::new(monsters_file, "Monsters"));

        // Add individual map files
        for map_file in maps_file_list {
            self.data_files.push(DataFileInfo::new(map_file, "Map"));
        }

        self.data_files
            .push(DataFileInfo::new(quests_file, "Quests"));
        self.data_files
            .push(DataFileInfo::new(classes_file, "Classes"));
        self.data_files.push(DataFileInfo::new(races_file, "Races"));
        self.data_files
            .push(DataFileInfo::new(characters_file, "Characters"));
        self.data_files
            .push(DataFileInfo::new(dialogue_file, "Dialogues"));
        self.data_files.push(DataFileInfo::new(npcs_file, "NPCs"));
        self.data_files
            .push(DataFileInfo::new(proficiencies_file, "Proficiencies"));

        // Check which files exist
        for file_info in &mut self.data_files {
            let full_path = self.campaign_dir.join(&file_info.path);
            if !full_path.exists() {
                file_info.mark_missing();
            }
        }
    }

    /// Updates the status of a data file.
    ///
    /// # Arguments
    ///
    /// * `path` - The relative path of the data file
    /// * `status` - The new status
    /// * `count` - Optional entry count (for Loaded status)
    /// * `error` - Optional error message (for Error status)
    pub fn update_data_file_status(
        &mut self,
        path: &str,
        status: DataFileStatus,
        count: Option<usize>,
        error: Option<String>,
    ) {
        if let Some(file_info) = self
            .data_files
            .iter_mut()
            .find(|f| f.path.to_string_lossy() == path)
        {
            file_info.status = status;
            file_info.entry_count = count;
            file_info.error_message = error;
        }
    }

    /// Marks a data file as successfully loaded.
    ///
    /// # Arguments
    ///
    /// * `path` - The relative path of the data file
    /// * `count` - Number of entries loaded
    pub fn mark_data_file_loaded(&mut self, path: &str, count: usize) {
        if let Some(file_info) = self
            .data_files
            .iter_mut()
            .find(|f| f.path.to_string_lossy() == path)
        {
            file_info.mark_loaded(count);
        }
    }

    /// Marks a data file as having an error.
    ///
    /// # Arguments
    ///
    /// * `path` - The relative path of the data file
    /// * `error` - The error message
    pub fn mark_data_file_error(&mut self, path: &str, error: &str) {
        if let Some(file_info) = self
            .data_files
            .iter_mut()
            .find(|f| f.path.to_string_lossy() == path)
        {
            file_info.mark_error(error);
        }
    }

    /// Returns the tracked data files.
    pub fn data_files(&self) -> &[DataFileInfo] {
        &self.data_files
    }

    /// Returns true if all data files are loaded successfully.
    pub fn all_data_files_loaded(&self) -> bool {
        self.data_files
            .iter()
            .all(|f| f.status == DataFileStatus::Loaded)
    }

    /// Returns the count of data files with errors.
    pub fn data_file_error_count(&self) -> usize {
        self.data_files
            .iter()
            .filter(|f| f.status == DataFileStatus::Error)
            .count()
    }

    /// Returns the count of missing data files.
    pub fn data_file_missing_count(&self) -> usize {
        self.data_files
            .iter()
            .filter(|f| f.status == DataFileStatus::Missing)
            .count()
    }

    /// Checks if a path is a tracked data file.
    pub fn is_data_file(&self, path: &Path) -> bool {
        self.data_files.iter().any(|f| f.path == path)
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

    /// Returns truly orphaned assets (unreferenced and not data files or documentation).
    ///
    /// This excludes:
    /// - Campaign data files (items.ron, spells.ron, etc.)
    /// - Documentation files
    /// - The campaign.ron file itself
    pub fn orphaned_assets(&self) -> Vec<&Asset> {
        self.assets
            .values()
            .filter(|a| {
                // Must be unreferenced
                if a.is_referenced {
                    return false;
                }

                // Exclude data files and documentation
                if a.asset_type == AssetType::Data || a.asset_type == AssetType::Documentation {
                    return false;
                }

                // Exclude tracked data files
                if self.is_data_file(&a.path) {
                    return false;
                }

                // Exclude campaign.ron
                if a.path
                    .file_name()
                    .map(|n| n == "campaign.ron")
                    .unwrap_or(false)
                {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Marks an asset as referenced
    pub fn mark_referenced(&mut self, asset_path: &Path, referenced: bool) {
        if let Some(asset) = self.assets.get_mut(asset_path) {
            asset.is_referenced = referenced;
        }
    }

    /// Marks all successfully loaded data files as referenced in the assets list.
    ///
    /// This should be called after data files are loaded to ensure they show
    /// as "In Use" in the Assets panel rather than "Unused".
    ///
    /// Data files that are tracked and have status `Loaded` will have their
    /// corresponding asset entry (if present) marked as referenced.
    pub fn mark_data_files_as_referenced(&mut self) {
        // Collect paths of loaded data files
        let loaded_paths: Vec<PathBuf> = self
            .data_files
            .iter()
            .filter(|f| f.status == DataFileStatus::Loaded)
            .map(|f| f.path.clone())
            .collect();

        // Mark each loaded data file as referenced in the assets map
        for path in loaded_paths {
            if let Some(asset) = self.assets.get_mut(&path) {
                asset.is_referenced = true;
            }
        }

        // Also mark campaign.ron as referenced if present
        let campaign_path = PathBuf::from("campaign.ron");
        if let Some(asset) = self.assets.get_mut(&campaign_path) {
            asset.is_referenced = true;
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
    /// This method examines items, spells, monsters, maps, quests, dialogues, classes,
    /// characters, and NPCs to determine which assets are actively used in the campaign.
    ///
    /// # Arguments
    ///
    /// * `items` - List of campaign items
    /// * `quests` - List of campaign quests
    /// * `dialogues` - List of campaign dialogues
    /// * `maps` - List of campaign maps
    /// * `classes` - List of campaign classes
    /// * `characters` - List of campaign characters
    /// * `npcs` - List of campaign NPCs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::types::Item;
    /// use antares::domain::quest::Quest;
    /// use antares::domain::dialogue::DialogueTree;
    /// use antares::domain::world::Map;
    /// use antares::domain::classes::ClassDefinition;
    /// use antares::domain::character_definition::CharacterDefinition;
    /// use antares::domain::world::npc::NpcDefinition;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = campaign_builder::asset_manager::AssetManager::new(PathBuf::from("/tmp/campaign"));
    /// let items = vec![];
    /// let quests = vec![];
    /// let dialogues = vec![];
    /// let maps = vec![];
    /// let classes = vec![];
    /// let characters = vec![];
    /// let npcs = vec![];
    /// manager.scan_references(&items, &quests, &dialogues, &maps, &classes, &characters, &npcs);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn scan_references(
        &mut self,
        items: &[antares::domain::items::types::Item],
        quests: &[antares::domain::quest::Quest],
        dialogues: &[antares::domain::dialogue::DialogueTree],
        maps: &[antares::domain::world::Map],
        classes: &[antares::domain::classes::ClassDefinition],
        characters: &[antares::domain::character_definition::CharacterDefinition],
        npcs: &[antares::domain::world::npc::NpcDefinition],
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

        // Scan characters for asset references (e.g., portrait images)
        self.scan_characters_references(characters);

        // Scan NPCs for asset references (e.g., portrait images)
        self.scan_npcs_references(npcs);
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

    /// Scans characters for asset references (portrait images)
    fn scan_characters_references(
        &mut self,
        characters: &[antares::domain::character_definition::CharacterDefinition],
    ) {
        for character in characters {
            // Characters reference portrait images via portrait_id field
            let portrait_id = &character.portrait_id;

            // Skip empty portrait IDs
            if portrait_id.is_empty() {
                continue;
            }

            // Try common portrait path patterns
            let potential_paths = vec![
                format!("assets/portraits/{}.png", portrait_id),
                format!("portraits/{}.png", portrait_id),
                format!("assets/portraits/{}.jpg", portrait_id),
                format!("portraits/{}.jpg", portrait_id),
            ];

            for path_str in potential_paths {
                let path = PathBuf::from(&path_str);
                if let Some(asset) = self.assets.get_mut(&path) {
                    asset.is_referenced = true;
                    // Only add reference if not already added for this character
                    if !asset.references.iter().any(|r| {
                        if let AssetReference::Character { id, .. } = r {
                            id == &character.id
                        } else {
                            false
                        }
                    }) {
                        asset.references.push(AssetReference::Character {
                            id: character.id.clone(),
                            name: character.name.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Scans NPCs for asset references (portrait images and sprite sheets)
    fn scan_npcs_references(&mut self, npcs: &[antares::domain::world::npc::NpcDefinition]) {
        for npc in npcs {
            // NPCs reference portrait images via portrait_id field
            let portrait_id = &npc.portrait_id;

            // Try common portrait path patterns if portrait_id is present
            if !portrait_id.is_empty() {
                let potential_paths = vec![
                    format!("assets/portraits/{}.png", portrait_id),
                    format!("portraits/{}.png", portrait_id),
                    format!("assets/portraits/{}.jpg", portrait_id),
                    format!("portraits/{}.jpg", portrait_id),
                ];

                for path_str in potential_paths {
                    let path = PathBuf::from(&path_str);
                    if let Some(asset) = self.assets.get_mut(&path) {
                        asset.is_referenced = true;
                        // Only add reference if not already added for this NPC
                        if !asset.references.iter().any(|r| {
                            if let AssetReference::Npc { id, .. } = r {
                                id == &npc.id
                            } else {
                                false
                            }
                        }) {
                            asset.references.push(AssetReference::Npc {
                                id: npc.id.clone(),
                                name: npc.name.clone(),
                            });
                        }
                    }
                }
            }

            // Scan sprite sheet references if present on the NPC definition
            if let Some(sprite) = &npc.sprite {
                let sprite_path = PathBuf::from(&sprite.sheet_path);
                if let Some(asset) = self.assets.get_mut(&sprite_path) {
                    asset.is_referenced = true;
                    // Only add reference if not already added for this NPC
                    if !asset.references.iter().any(|r| {
                        if let AssetReference::Npc { id, .. } = r {
                            id == &npc.id
                        } else {
                            false
                        }
                    }) {
                        asset.references.push(AssetReference::Npc {
                            id: npc.id.clone(),
                            name: npc.name.clone(),
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
    fn test_data_file_status_icon() {
        assert_eq!(DataFileStatus::NotLoaded.icon(), "⏳");
        assert_eq!(DataFileStatus::Loaded.icon(), "✅");
        assert_eq!(DataFileStatus::Error.icon(), "❌");
        assert_eq!(DataFileStatus::Missing.icon(), "⚠️");
    }

    #[test]
    fn test_data_file_status_display_name() {
        assert_eq!(DataFileStatus::NotLoaded.display_name(), "Not Loaded");
        assert_eq!(DataFileStatus::Loaded.display_name(), "Loaded");
        assert_eq!(DataFileStatus::Error.display_name(), "Load Error");
        assert_eq!(DataFileStatus::Missing.display_name(), "Missing");
    }

    #[test]
    fn test_data_file_info_new() {
        let info = DataFileInfo::new("data/items.ron", "Items");
        assert_eq!(info.path, PathBuf::from("data/items.ron"));
        assert_eq!(info.display_name, "Items");
        assert_eq!(info.status, DataFileStatus::NotLoaded);
        assert!(info.entry_count.is_none());
        assert!(info.error_message.is_none());
    }

    #[test]
    fn test_data_file_info_mark_loaded() {
        let mut info = DataFileInfo::new("data/items.ron", "Items");
        info.mark_loaded(42);
        assert_eq!(info.status, DataFileStatus::Loaded);
        assert_eq!(info.entry_count, Some(42));
        assert!(info.error_message.is_none());
    }

    #[test]
    fn test_data_file_info_mark_error() {
        let mut info = DataFileInfo::new("data/items.ron", "Items");
        info.mark_error("Parse error at line 5");
        assert_eq!(info.status, DataFileStatus::Error);
        assert_eq!(
            info.error_message,
            Some("Parse error at line 5".to_string())
        );
    }

    #[test]
    fn test_data_file_info_mark_missing() {
        let mut info = DataFileInfo::new("data/items.ron", "Items");
        info.mark_missing();
        assert_eq!(info.status, DataFileStatus::Missing);
    }

    #[test]
    fn test_asset_manager_data_file_tracking() {
        let tmp_dir = std::env::temp_dir().join("test_asset_manager_data_files");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut manager = AssetManager::new(tmp_dir.clone());
        let map_files = vec![
            "data/maps/map_1.ron".to_string(),
            "data/maps/map_2.ron".to_string(),
        ];
        manager.init_data_files(
            "data/items.ron",
            "data/spells.ron",
            "data/conditions.ron",
            "data/monsters.ron",
            &map_files,
            "data/quests.ron",
            "data/classes.ron",
            "data/races.ron",
            "data/characters.ron",
            "data/dialogues.ron",
            "data/npcs.ron",
            "data/proficiencies.ron",
        );

        // All files should be marked as missing since they don't exist
        // Expected: Items, Spells, Conditions, Monsters, 2 Maps, Quests, Classes, Races, Characters, Dialogues, NPCs, Proficiencies = 13
        assert_eq!(manager.data_files().len(), 13);
        for file_info in manager.data_files() {
            assert_eq!(file_info.status, DataFileStatus::Missing);
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_asset_manager_mark_data_file_loaded() {
        let tmp_dir = std::env::temp_dir().join("test_asset_manager_mark_loaded");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut manager = AssetManager::new(tmp_dir.clone());
        manager.init_data_files(
            "data/items.ron",
            "data/spells.ron",
            "data/conditions.ron",
            "data/monsters.ron",
            &[],
            "data/quests.ron",
            "data/classes.ron",
            "data/races.ron",
            "data/characters.ron",
            "data/dialogues.ron",
            "data/npcs.ron",
            "data/proficiencies.ron",
        );

        manager.mark_data_file_loaded("data/items.ron", 25);

        let items_file = manager
            .data_files()
            .iter()
            .find(|f| f.display_name == "Items")
            .unwrap();
        assert_eq!(items_file.status, DataFileStatus::Loaded);
        assert_eq!(items_file.entry_count, Some(25));

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_asset_manager_all_data_files_loaded() {
        let tmp_dir = std::env::temp_dir().join("test_asset_manager_all_loaded");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut manager = AssetManager::new(tmp_dir.clone());
        manager.init_data_files(
            "data/items.ron",
            "data/spells.ron",
            "data/conditions.ron",
            "data/monsters.ron",
            &[],
            "data/quests.ron",
            "data/classes.ron",
            "data/races.ron",
            "data/characters.ron",
            "data/dialogues.ron",
            "data/npcs.ron",
            "data/proficiencies.ron",
        );

        assert!(!manager.all_data_files_loaded());

        // Mark all as loaded
        for path in [
            "data/items.ron",
            "data/spells.ron",
            "data/conditions.ron",
            "data/monsters.ron",
            "data/quests.ron",
            "data/classes.ron",
            "data/races.ron",
            "data/characters.ron",
            "data/dialogues.ron",
            "data/npcs.ron",
            "data/proficiencies.ron",
        ] {
            manager.mark_data_file_loaded(path, 1);
        }

        assert!(manager.all_data_files_loaded());

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

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
    fn test_mark_data_files_as_referenced_marks_assets() {
        // Prepare a temporary campaign directory
        let tmp_dir = std::env::temp_dir().join("test_asset_manager_mark_referenced");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut manager = AssetManager::new(tmp_dir.clone());
        manager.init_data_files(
            "data/items.ron",
            "data/spells.ron",
            "data/conditions.ron",
            "data/monsters.ron",
            &[],
            "data/quests.ron",
            "data/classes.ron",
            "data/races.ron",
            "data/characters.ron",
            "data/dialogues.ron",
            "data/npcs.ron",
            "data/proficiencies.ron",
        );

        // Insert an Asset whose path matches one of the tracked data files
        let data_path = PathBuf::from("data/items.ron");
        let asset = Asset::new(data_path.clone());
        manager.assets.insert(data_path.clone(), asset);

        // Initially it should not be referenced
        assert!(!manager.assets.get(&data_path).unwrap().is_referenced);

        // Mark the data file as loaded
        manager.mark_data_file_loaded("data/items.ron", 10);

        // Call the method that should mark data files as referenced
        manager.mark_data_files_as_referenced();

        // Now the asset should be marked as referenced
        assert!(manager.assets.get(&data_path).unwrap().is_referenced);

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_scan_references_marks_assets_referenced() {
        use antares::domain::items::types::{Item, ItemType};
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
        #[allow(deprecated)]
        let item = Item {
            id: 1 as ItemId,
            name: "Longsword".to_string(),
            item_type: ItemType::Weapon(antares::domain::items::WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: antares::domain::items::WeaponClassification::MartialMelee,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        // Scan references
        manager.scan_references(&[item], &[], &[], &[], &[], &[], &[]);

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

        manager.scan_references(&[], &[quest], &[], &[], &[], &[], &[]);

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

        manager.scan_references(&[], &[], &[dialogue], &[], &[], &[], &[]);

        let asset = manager.assets.get(&portrait_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "Dialogue");
    }

    #[test]
    fn test_scan_characters_references() {
        use antares::domain::character::Alignment;
        use antares::domain::character::Sex;
        use antares::domain::character::Stats;
        use antares::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        let portrait_path = PathBuf::from("assets/portraits/character_040.png");
        manager.assets.insert(
            portrait_path.clone(),
            Asset {
                path: portrait_path.clone(),
                asset_type: AssetType::Portrait,
                size: 4096,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let character = CharacterDefinition {
            id: "test_knight".to_string(),
            name: "Sir Lancelot".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Good,
            base_stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp_override: None,
            portrait_id: "character_040".to_string(),
            starting_gold: 100,
            starting_gems: 0,
            starting_food: 15,
            starting_items: vec![],
            starting_equipment: StartingEquipment::default(),
            description: "A brave knight".to_string(),
            is_premade: true,
            starts_in_party: true,
        };

        manager.scan_references(&[], &[], &[], &[], &[], &[character], &[]);

        let asset = manager.assets.get(&portrait_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "Character");
        match &asset.references[0] {
            AssetReference::Character { id, name } => {
                assert_eq!(id, "test_knight");
                assert_eq!(name, "Sir Lancelot");
            }
            _ => panic!("Expected Character reference"),
        }
    }

    #[test]
    fn test_scan_npcs_references() {
        use antares::domain::world::npc::NpcDefinition;
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add an asset that should match potential path patterns for portraits
        let portrait_path = PathBuf::from("assets/portraits/elder_1.png");
        manager.assets.insert(
            portrait_path.clone(),
            Asset {
                path: portrait_path.clone(),
                asset_type: AssetType::Portrait,
                size: 4096,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let npc = NpcDefinition {
            id: "village_elder".to_string(),
            name: "Village Elder".to_string(),
            description: "The wise elder of the village".to_string(),
            portrait_id: "elder_1".to_string(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: vec![],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };

        manager.scan_references(&[], &[], &[], &[], &[], &[], &[npc]);

        let asset = manager.assets.get(&portrait_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "NPC");
        match &asset.references[0] {
            AssetReference::Npc { id, name } => {
                assert_eq!(id, "village_elder");
                assert_eq!(name, "Village Elder");
            }
            _ => panic!("Expected NPC reference"),
        }
    }

    #[test]
    fn test_scan_npcs_detects_sprite_sheet_reference_in_metadata() {
        use antares::domain::world::npc::NpcDefinition;
        use antares::domain::world::SpriteReference;
        use std::path::PathBuf;
        use std::time::SystemTime;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add an asset that should match the sprite sheet path
        let sprite_path = PathBuf::from("assets/sprites/actors/test_npc.png");
        manager.assets.insert(
            sprite_path.clone(),
            Asset {
                path: sprite_path.clone(),
                asset_type: AssetType::Tileset,
                size: 1024,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let sprite = SpriteReference {
            sheet_path: "assets/sprites/actors/test_npc.png".to_string(),
            sprite_index: 5,
            animation: None,
            material_properties: None,
        };

        let npc = NpcDefinition {
            id: "test_npc".to_string(),
            name: "Test NPC".to_string(),
            description: "A test NPC".to_string(),
            portrait_id: "test_1".to_string(),
            sprite: Some(sprite),
            dialogue_id: None,
            creature_id: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };

        manager.scan_references(&[], &[], &[], &[], &[], &[], &[npc]);

        let asset = manager.assets.get(&sprite_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 1);
        assert_eq!(asset.references[0].category(), "NPC");
        match &asset.references[0] {
            AssetReference::Npc { id, name } => {
                assert_eq!(id, "test_npc");
                assert_eq!(name, "Test NPC");
            }
            _ => panic!("Expected NPC reference"),
        }
    }

    #[test]
    fn test_scan_portrait_path_matching() {
        use antares::domain::character::Alignment;
        use antares::domain::character::Sex;
        use antares::domain::character::Stats;
        use antares::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        // Add assets with the EXACT path format used in real campaigns
        let paths = vec![
            "assets/portraits/character_040.png",
            "assets/portraits/elder_1.png",
            "assets/portraits/merchant_1.png",
            "assets/portraits/npc_015.png",
        ];

        for path_str in &paths {
            let path = PathBuf::from(path_str);
            manager.assets.insert(
                path.clone(),
                Asset {
                    path: path.clone(),
                    asset_type: AssetType::Portrait,
                    size: 4096,
                    modified: SystemTime::now(),
                    is_referenced: false,
                    references: Vec::new(),
                },
            );
        }

        // Create characters matching the actual npcs.ron and characters.ron
        let characters = vec![CharacterDefinition {
            id: "tutorial_human_knight".to_string(),
            name: "Kira".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Female,
            alignment: Alignment::Good,
            base_stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp_override: None,
            portrait_id: "character_040".to_string(),
            starting_gold: 100,
            starting_gems: 0,
            starting_food: 15,
            starting_items: vec![],
            starting_equipment: StartingEquipment::default(),
            description: "A young warrior".to_string(),
            is_premade: true,
            starts_in_party: true,
        }];

        let npcs = vec![
            antares::domain::world::npc::NpcDefinition {
                id: "tutorial_elder_village".to_string(),
                name: "Village Elder".to_string(),
                description: "The wise elder".to_string(),
                portrait_id: "elder_1".to_string(),
                sprite: None,
                dialogue_id: None,
                creature_id: None,
                quest_ids: vec![5],
                faction: Some("Village".to_string()),
                is_merchant: false,
                is_innkeeper: false,
            },
            antares::domain::world::npc::NpcDefinition {
                id: "tutorial_merchant_town".to_string(),
                name: "Merchant".to_string(),
                description: "A traveling merchant".to_string(),
                portrait_id: "merchant_1".to_string(),
                sprite: None,
                dialogue_id: None,
                creature_id: None,
                quest_ids: vec![],
                faction: Some("Merchants Guild".to_string()),
                is_merchant: true,
                is_innkeeper: false,
            },
            antares::domain::world::npc::NpcDefinition {
                id: "tutorial_wizard_arcturus_brother".to_string(),
                name: "Arcturus Brother".to_string(),
                description: "A local villager".to_string(),
                portrait_id: "npc_015".to_string(),
                sprite: None,
                dialogue_id: None,
                creature_id: None,
                quest_ids: vec![1, 3],
                faction: Some("Village".to_string()),
                is_merchant: false,
                is_innkeeper: false,
            },
        ];

        // Scan references
        manager.scan_references(&[], &[], &[], &[], &[], &characters, &npcs);

        // Verify character portrait is referenced
        let character_portrait = manager
            .assets
            .get(&PathBuf::from("assets/portraits/character_040.png"))
            .unwrap();
        assert!(
            character_portrait.is_referenced,
            "Character portrait should be referenced"
        );
        assert_eq!(character_portrait.references.len(), 1);
        match &character_portrait.references[0] {
            AssetReference::Character { id, name } => {
                assert_eq!(id, "tutorial_human_knight");
                assert_eq!(name, "Kira");
            }
            _ => panic!("Expected Character reference"),
        }

        // Verify NPC portraits are referenced
        let elder_portrait = manager
            .assets
            .get(&PathBuf::from("assets/portraits/elder_1.png"))
            .unwrap();
        assert!(
            elder_portrait.is_referenced,
            "Elder portrait should be referenced"
        );
        assert_eq!(elder_portrait.references.len(), 1);
        match &elder_portrait.references[0] {
            AssetReference::Npc { id, name } => {
                assert_eq!(id, "tutorial_elder_village");
                assert_eq!(name, "Village Elder");
            }
            _ => panic!("Expected Npc reference"),
        }

        let merchant_portrait = manager
            .assets
            .get(&PathBuf::from("assets/portraits/merchant_1.png"))
            .unwrap();
        assert!(
            merchant_portrait.is_referenced,
            "Merchant portrait should be referenced"
        );

        let npc_portrait = manager
            .assets
            .get(&PathBuf::from("assets/portraits/npc_015.png"))
            .unwrap();
        assert!(
            npc_portrait.is_referenced,
            "NPC portrait should be referenced"
        );
    }

    #[test]
    fn test_scan_with_actual_tutorial_campaign_data() {
        use std::path::PathBuf;

        // This test uses the actual tutorial campaign data to verify portrait scanning works
        let campaign_dir = PathBuf::from("campaigns/tutorial");

        // Skip test if campaign directory doesn't exist (e.g., in CI)
        if !campaign_dir.exists() {
            eprintln!("Skipping test - tutorial campaign not found");
            return;
        }

        let mut manager = AssetManager::new(campaign_dir.clone());

        // Scan the campaign directory for assets
        if let Err(e) = manager.scan_directory() {
            panic!("Failed to scan campaign directory: {}", e);
        }

        // Load actual characters from characters.ron
        let characters_path = campaign_dir.join("data/characters.ron");
        let characters: Vec<antares::domain::character_definition::CharacterDefinition> =
            if characters_path.exists() {
                let contents = std::fs::read_to_string(&characters_path)
                    .expect("Failed to read characters.ron");
                ron::from_str(&contents).expect("Failed to parse characters.ron")
            } else {
                eprintln!("Skipping test - characters.ron not found");
                return;
            };

        // Load actual NPCs from npcs.ron
        let npcs_path = campaign_dir.join("data/npcs.ron");
        let npcs: Vec<antares::domain::world::npc::NpcDefinition> = if npcs_path.exists() {
            let contents = std::fs::read_to_string(&npcs_path).expect("Failed to read npcs.ron");
            ron::from_str(&contents).expect("Failed to parse npcs.ron")
        } else {
            eprintln!("Skipping test - npcs.ron not found");
            return;
        };

        eprintln!(
            "Loaded {} characters and {} NPCs",
            characters.len(),
            npcs.len()
        );
        eprintln!("Found {} total assets", manager.assets().len());

        // Count portrait assets before scanning
        let portrait_count_before = manager.assets_by_type(AssetType::Portrait).len();
        eprintln!("Found {} portrait assets", portrait_count_before);

        // Scan references
        manager.scan_references(&[], &[], &[], &[], &[], &characters, &npcs);

        // Verify that portraits referenced by characters and NPCs are marked as referenced
        let mut referenced_portraits = 0;
        let mut unreferenced_portraits = 0;

        for (path, asset) in manager.assets() {
            if asset.asset_type == AssetType::Portrait {
                if asset.is_referenced {
                    referenced_portraits += 1;
                    eprintln!(
                        "Referenced: {} by {} references",
                        path.display(),
                        asset.references.len()
                    );
                } else {
                    unreferenced_portraits += 1;
                    eprintln!("Unreferenced: {}", path.display());
                }
            }
        }

        eprintln!(
            "Summary: {} referenced portraits, {} unreferenced portraits",
            referenced_portraits, unreferenced_portraits
        );

        // We should have at least some referenced portraits from the characters and NPCs
        assert!(
            referenced_portraits > 0,
            "Expected at least some portraits to be marked as referenced. Characters: {}, NPCs: {}",
            characters.len(),
            npcs.len()
        );

        // Specifically check a few known portraits from the actual data files
        // From characters.ron: character_040, character_042, character_041
        // From npcs.ron: elder_1, merchant_1, old_wizard_1, etc.

        let test_portraits = vec![
            (
                "assets/portraits/character_040.png",
                "character from characters.ron",
            ),
            ("assets/portraits/elder_1.png", "NPC from npcs.ron"),
            ("assets/portraits/merchant_1.png", "NPC from npcs.ron"),
        ];

        for (portrait_path, description) in test_portraits {
            let path = PathBuf::from(portrait_path);
            if let Some(asset) = manager.assets().get(&path) {
                assert!(
                    asset.is_referenced,
                    "Portrait {} ({}) should be marked as referenced",
                    portrait_path, description
                );
                assert!(
                    !asset.references.is_empty(),
                    "Portrait {} should have at least one reference",
                    portrait_path
                );
            }
        }
    }

    #[test]
    fn test_scan_multiple_characters_same_portrait() {
        use antares::domain::character::Alignment;
        use antares::domain::character::Sex;
        use antares::domain::character::Stats;
        use antares::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use std::path::PathBuf;

        let mut manager = AssetManager::new(PathBuf::from("/tmp/test_campaign"));

        let portrait_path = PathBuf::from("portraits/character_046.png");
        manager.assets.insert(
            portrait_path.clone(),
            Asset {
                path: portrait_path.clone(),
                asset_type: AssetType::Portrait,
                size: 4096,
                modified: SystemTime::now(),
                is_referenced: false,
                references: Vec::new(),
            },
        );

        let character1 = CharacterDefinition {
            id: "fighter1".to_string(),
            name: "Fighter One".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Neutral,
            base_stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp_override: None,
            portrait_id: "character_046".to_string(),
            starting_gold: 50,
            starting_gems: 0,
            starting_food: 10,
            starting_items: vec![],
            starting_equipment: StartingEquipment::default(),
            description: "First fighter".to_string(),
            is_premade: false,
            starts_in_party: false,
        };

        let character2 = CharacterDefinition {
            id: "fighter2".to_string(),
            name: "Fighter Two".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Neutral,
            base_stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp_override: None,
            portrait_id: "character_046".to_string(),
            starting_gold: 50,
            starting_gems: 0,
            starting_food: 10,
            starting_items: vec![],
            starting_equipment: StartingEquipment::default(),
            description: "Second fighter".to_string(),
            is_premade: false,
            starts_in_party: false,
        };

        manager.scan_references(&[], &[], &[], &[], &[], &[character1, character2], &[]);

        let asset = manager.assets.get(&portrait_path).unwrap();
        assert!(asset.is_referenced);
        assert_eq!(asset.references.len(), 2);
        assert!(asset
            .references
            .iter()
            .any(|r| matches!(r, AssetReference::Character { id, .. } if id == "fighter1")));
        assert!(asset
            .references
            .iter()
            .any(|r| matches!(r, AssetReference::Character { id, .. } if id == "fighter2")));
    }
}
