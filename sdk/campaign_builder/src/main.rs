// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder - Phase 2: Foundation UI for Antares SDK
//!
//! Phase 2 adds:
//! - Full metadata editor with all campaign.ron fields
//! - Real file I/O (save/load campaign.ron)
//! - Enhanced validation UI with detailed error reporting
//! - File structure browser showing campaign directory layout
//! - Placeholder list views for Items, Spells, Monsters, Maps, Quests
//! - Unsaved changes tracking and warnings

mod map_editor;

use antares::domain::character::Stats;
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType};
use antares::domain::items::types::{Disablement, Item, ItemType, WeaponData};
use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
use antares::domain::types::DiceRoll;
use antares::domain::world::Map;
use eframe::egui;
use map_editor::{MapEditorState, MapEditorWidget};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Antares Campaign Builder - Phase 2"),

        renderer: eframe::Renderer::default(),
        ..Default::default()
    };

    eframe::run_native(
        "Antares Campaign Builder",
        options,
        Box::new(|_cc| Ok(Box::<CampaignBuilderApp>::default())),
    )
}

/// Campaign metadata structure matching campaign.ron schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignMetadata {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    engine_version: String,

    // Campaign configuration
    starting_map: String,
    starting_position: (u32, u32),
    starting_direction: String,
    starting_gold: u32,
    starting_food: u32,
    max_party_size: usize,
    max_roster_size: usize,
    difficulty: Difficulty,
    permadeath: bool,
    allow_multiclassing: bool,
    starting_level: u8,
    max_level: u8,

    // Data file paths
    items_file: String,
    spells_file: String,
    monsters_file: String,
    classes_file: String,
    races_file: String,
    maps_dir: String,
    quests_file: String,
    dialogue_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
    Brutal,
}

impl Difficulty {
    fn as_str(&self) -> &str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::Brutal => "Brutal",
        }
    }

    fn all() -> [Difficulty; 4] {
        [
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hard,
            Difficulty::Brutal,
        ]
    }
}

impl Default for CampaignMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            version: "1.0.0".to_string(),
            author: String::new(),
            description: String::new(),
            engine_version: "0.1.0".to_string(),

            starting_map: "starter_town".to_string(),
            starting_position: (10, 10),
            starting_direction: "North".to_string(),
            starting_gold: 100,
            starting_food: 10,
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: Difficulty::Normal,
            permadeath: false,
            allow_multiclassing: false,
            starting_level: 1,
            max_level: 20,

            items_file: "data/items.ron".to_string(),
            spells_file: "data/spells.ron".to_string(),
            monsters_file: "data/monsters.ron".to_string(),
            classes_file: "data/classes.ron".to_string(),
            races_file: "data/races.ron".to_string(),
            maps_dir: "data/maps/".to_string(),
            quests_file: "data/quests.ron".to_string(),
            dialogue_file: "data/dialogue.ron".to_string(),
        }
    }
}

/// Active tab in the UI
#[derive(Debug, Clone, Copy, PartialEq)]
enum EditorTab {
    Metadata,
    Config,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
    Files,
    Validation,
}

/// Editor mode for data editing panels
#[derive(Debug, Clone, Copy, PartialEq)]
enum EditorMode {
    List,
    Add,
    Edit,
}

impl EditorTab {
    fn name(&self) -> &str {
        match self {
            EditorTab::Metadata => "📋 Metadata",
            EditorTab::Config => "⚙️ Config",
            EditorTab::Items => "⚔️ Items",
            EditorTab::Spells => "✨ Spells",
            EditorTab::Monsters => "👹 Monsters",
            EditorTab::Maps => "🗺️ Maps",
            EditorTab::Quests => "📜 Quests",
            EditorTab::Files => "📁 Files",
            EditorTab::Validation => "✅ Validation",
        }
    }
}

/// Validation error with severity
#[derive(Debug, Clone)]
struct ValidationError {
    severity: Severity,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn icon(&self) -> &str {
        match self {
            Severity::Error => "❌",
            Severity::Warning => "⚠️",
        }
    }
}

/// File I/O errors
#[derive(Debug, Error)]
enum CampaignError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("RON serialization error: {0}")]
    Serialization(#[from] ron::Error),

    #[error("RON deserialization error: {0}")]
    Deserialization(#[from] ron::error::SpannedError),

    #[error("No campaign path set")]
    NoPath,
}

/// Main application state
struct CampaignBuilderApp {
    campaign: CampaignMetadata,
    active_tab: EditorTab,
    campaign_path: Option<PathBuf>,
    campaign_dir: Option<PathBuf>,
    status_message: String,
    unsaved_changes: bool,
    validation_errors: Vec<ValidationError>,
    show_about_dialog: bool,
    show_unsaved_warning: bool,
    pending_action: Option<PendingAction>,
    file_tree: Vec<FileNode>,

    // Data editor state
    items: Vec<Item>,
    items_search: String,
    items_selected: Option<usize>,
    items_editor_mode: EditorMode,
    items_edit_buffer: Item,

    spells: Vec<Spell>,
    spells_search: String,
    spells_selected: Option<usize>,
    spells_editor_mode: EditorMode,
    spells_edit_buffer: Spell,

    monsters: Vec<MonsterDefinition>,
    monsters_search: String,
    monsters_selected: Option<usize>,
    monsters_editor_mode: EditorMode,
    monsters_edit_buffer: MonsterDefinition,

    // Map editor state
    maps: Vec<Map>,
    maps_search: String,
    maps_selected: Option<usize>,
    maps_editor_mode: EditorMode,
    map_editor_state: Option<MapEditorState>,
}

#[derive(Debug, Clone)]
enum PendingAction {
    New,
    Open,
    Exit,
}

#[derive(Debug, Clone)]
struct FileNode {
    name: String,
    #[allow(dead_code)]
    path: PathBuf,
    is_directory: bool,
    children: Vec<FileNode>,
}

impl Default for CampaignBuilderApp {
    fn default() -> Self {
        Self {
            campaign: CampaignMetadata::default(),
            active_tab: EditorTab::Metadata,
            campaign_path: None,
            campaign_dir: None,
            status_message: String::new(),
            unsaved_changes: false,
            validation_errors: Vec::new(),
            show_about_dialog: false,
            show_unsaved_warning: false,
            pending_action: None,
            file_tree: Vec::new(),

            // Data editor state
            items: Vec::new(),
            items_search: String::new(),
            items_selected: None,
            items_editor_mode: EditorMode::List,
            items_edit_buffer: Self::default_item(),

            spells: Vec::new(),
            spells_search: String::new(),
            spells_selected: None,
            spells_editor_mode: EditorMode::List,
            spells_edit_buffer: Self::default_spell(),

            monsters: Vec::new(),
            monsters_search: String::new(),
            monsters_selected: None,
            monsters_editor_mode: EditorMode::List,
            monsters_edit_buffer: Self::default_monster(),

            // Map editor state
            maps: Vec::new(),
            maps_search: String::new(),
            maps_selected: None,
            maps_editor_mode: EditorMode::List,
            map_editor_state: None,
        }
    }
}

impl CampaignBuilderApp {
    /// Create a default item for the edit buffer
    fn default_item() -> Item {
        Item {
            id: 0,
            name: String::new(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 0,
            sell_cost: 0,
            disablements: Disablement(255), // All classes
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
        }
    }

    /// Create a default spell for the edit buffer
    fn default_spell() -> Spell {
        Spell::new(
            0,
            "",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "",
        )
    }

    /// Create a default monster for the edit buffer
    fn default_monster() -> MonsterDefinition {
        MonsterDefinition {
            id: 0,
            name: String::new(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: 10,
            ac: 10,
            attacks: vec![Attack {
                damage: DiceRoll::new(1, 4, 0),
                attack_type: AttackType::Physical,
                special_effect: None,
            }],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: true,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable {
                gold_min: 0,
                gold_max: 0,
                gems_min: 0,
                gems_max: 0,
                items: Vec::new(),
                experience: 0,
            },
        }
    }

    /// Load items from RON file
    fn load_items(&mut self) {
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&self.campaign.items_file);
            if items_path.exists() {
                match fs::read_to_string(&items_path) {
                    Ok(contents) => match ron::from_str::<Vec<Item>>(&contents) {
                        Ok(items) => {
                            self.items = items;
                            self.status_message = format!("Loaded {} items", self.items.len());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse items: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read items file: {}", e);
                    }
                }
            }
        }
    }

    /// Save items to RON file
    fn save_items(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&self.campaign.items_file);
            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(&self.items, ron_config)
                .map_err(|e| format!("Failed to serialize items: {}", e))?;

            fs::write(&items_path, contents)
                .map_err(|e| format!("Failed to write items file: {}", e))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Load spells from RON file
    fn load_spells(&mut self) {
        if let Some(ref dir) = self.campaign_dir {
            let spells_path = dir.join(&self.campaign.spells_file);
            if spells_path.exists() {
                match fs::read_to_string(&spells_path) {
                    Ok(contents) => match ron::from_str::<Vec<Spell>>(&contents) {
                        Ok(spells) => {
                            self.spells = spells;
                            self.status_message = format!("Loaded {} spells", self.spells.len());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse spells: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read spells file: {}", e);
                    }
                }
            }
        }
    }

    /// Save spells to RON file
    fn save_spells(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let spells_path = dir.join(&self.campaign.spells_file);
            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(&self.spells, ron_config)
                .map_err(|e| format!("Failed to serialize spells: {}", e))?;

            fs::write(&spells_path, contents)
                .map_err(|e| format!("Failed to write spells file: {}", e))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Load monsters from RON file
    fn load_monsters(&mut self) {
        if let Some(ref dir) = self.campaign_dir {
            let monsters_path = dir.join(&self.campaign.monsters_file);
            if monsters_path.exists() {
                match fs::read_to_string(&monsters_path) {
                    Ok(contents) => match ron::from_str::<Vec<MonsterDefinition>>(&contents) {
                        Ok(monsters) => {
                            self.monsters = monsters;
                            self.status_message =
                                format!("Loaded {} monsters", self.monsters.len());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse monsters: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read monsters file: {}", e);
                    }
                }
            }
        }
    }

    /// Save monsters to RON file
    fn save_monsters(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let monsters_path = dir.join(&self.campaign.monsters_file);
            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(&self.monsters, ron_config)
                .map_err(|e| format!("Failed to serialize monsters: {}", e))?;

            fs::write(&monsters_path, contents)
                .map_err(|e| format!("Failed to write monsters file: {}", e))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Load maps from the maps directory
    fn load_maps(&mut self) {
        self.maps.clear();

        if let Some(ref dir) = self.campaign_dir {
            let maps_dir = dir.join(&self.campaign.maps_dir);

            if maps_dir.exists() && maps_dir.is_dir() {
                match fs::read_dir(&maps_dir) {
                    Ok(entries) => {
                        let mut loaded_count = 0;
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                                match fs::read_to_string(&path) {
                                    Ok(contents) => match ron::from_str::<Map>(&contents) {
                                        Ok(map) => {
                                            self.maps.push(map);
                                            loaded_count += 1;
                                        }
                                        Err(e) => {
                                            self.status_message = format!(
                                                "Failed to parse map {:?}: {}",
                                                path.file_name().unwrap_or_default(),
                                                e
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        self.status_message = format!(
                                            "Failed to read map {:?}: {}",
                                            path.file_name().unwrap_or_default(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        if loaded_count > 0 {
                            self.status_message = format!("Loaded {} maps", loaded_count);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to read maps directory: {}", e);
                    }
                }
            }
        }
    }

    /// Save a map to RON file
    fn save_map(&mut self, map: &Map) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let maps_dir = dir.join(&self.campaign.maps_dir);

            // Create maps directory if it doesn't exist
            fs::create_dir_all(&maps_dir)
                .map_err(|e| format!("Failed to create maps directory: {}", e))?;

            let map_filename = format!("map_{}.ron", map.id);
            let map_path = maps_dir.join(map_filename);

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(map, ron_config)
                .map_err(|e| format!("Failed to serialize map: {}", e))?;

            fs::write(&map_path, contents)
                .map_err(|e| format!("Failed to write map file: {}", e))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Validate the campaign metadata
    fn validate_campaign(&mut self) {
        self.validation_errors.clear();

        // Required fields
        if self.campaign.id.is_empty() {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Campaign ID is required".to_string(),
            });
        } else if !self
            .campaign
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Campaign ID must contain only alphanumeric characters and underscores"
                    .to_string(),
            });
        }

        if self.campaign.name.is_empty() {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Campaign name is required".to_string(),
            });
        }

        if self.campaign.author.is_empty() {
            self.validation_errors.push(ValidationError {
                severity: Severity::Warning,
                message: "Author name is recommended".to_string(),
            });
        }

        // Version validation
        if !self.campaign.version.contains('.') {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Version should follow semantic versioning (e.g., 1.0.0)".to_string(),
            });
        }

        // Engine version validation
        if !self.campaign.engine_version.contains('.') {
            self.validation_errors.push(ValidationError {
                severity: Severity::Warning,
                message: "Engine version should follow semantic versioning".to_string(),
            });
        }

        // Configuration validation
        if self.campaign.starting_map.is_empty() {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Starting map is required".to_string(),
            });
        }

        if self.campaign.max_party_size == 0 || self.campaign.max_party_size > 10 {
            self.validation_errors.push(ValidationError {
                severity: Severity::Warning,
                message: "Max party size should be between 1 and 10".to_string(),
            });
        }

        if self.campaign.max_roster_size < self.campaign.max_party_size {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Max roster size must be >= max party size".to_string(),
            });
        }

        if self.campaign.starting_level == 0
            || self.campaign.starting_level > self.campaign.max_level
        {
            self.validation_errors.push(ValidationError {
                severity: Severity::Error,
                message: "Starting level must be between 1 and max level".to_string(),
            });
        }

        // File path validation
        for (field, path) in [
            ("Items file", &self.campaign.items_file),
            ("Spells file", &self.campaign.spells_file),
            ("Monsters file", &self.campaign.monsters_file),
            ("Classes file", &self.campaign.classes_file),
            ("Races file", &self.campaign.races_file),
            ("Quests file", &self.campaign.quests_file),
            ("Dialogue file", &self.campaign.dialogue_file),
        ] {
            if path.is_empty() {
                self.validation_errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("{} path is required", field),
                });
            } else if !path.ends_with(".ron") {
                self.validation_errors.push(ValidationError {
                    severity: Severity::Warning,
                    message: format!("{} should use .ron extension", field),
                });
            }
        }

        // Update status
        let error_count = self
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count();
        let warning_count = self
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Warning)
            .count();

        if self.validation_errors.is_empty() {
            self.status_message = "✅ Validation passed!".to_string();
        } else {
            self.status_message = format!(
                "Validation: {} error(s), {} warning(s)",
                error_count, warning_count
            );
        }
    }

    /// Create a new campaign
    fn new_campaign(&mut self) {
        if self.unsaved_changes {
            self.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::New);
        } else {
            self.do_new_campaign();
        }
    }

    fn do_new_campaign(&mut self) {
        self.campaign = CampaignMetadata::default();
        self.campaign_path = None;
        self.campaign_dir = None;
        self.unsaved_changes = false;
        self.validation_errors.clear();
        self.file_tree.clear();
        self.status_message = "New campaign created.".to_string();
    }

    /// Save campaign to file
    fn save_campaign(&mut self) -> Result<(), CampaignError> {
        if self.campaign_path.is_none() {
            return Err(CampaignError::NoPath);
        }

        self.do_save_campaign()
    }

    fn do_save_campaign(&mut self) -> Result<(), CampaignError> {
        let path = self.campaign_path.as_ref().ok_or(CampaignError::NoPath)?;

        // Serialize to RON format with pretty printing
        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(4);

        let ron_string = ron::ser::to_string_pretty(&self.campaign, ron_config)?;

        // Write to file
        fs::write(path, ron_string)?;

        self.unsaved_changes = false;
        self.status_message = format!("Campaign saved to: {}", path.display());

        // Update file tree if we have a campaign directory
        if let Some(dir) = self.campaign_dir.clone() {
            self.update_file_tree(&dir);
        }

        Ok(())
    }

    /// Save campaign as (with file dialog)
    fn save_campaign_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("campaign.ron")
            .add_filter("RON Files", &["ron"])
            .save_file()
        {
            self.campaign_path = Some(path.clone());

            // Set campaign directory (parent of campaign.ron)
            if let Some(parent) = path.parent() {
                self.campaign_dir = Some(parent.to_path_buf());
            }

            match self.do_save_campaign() {
                Ok(()) => {}
                Err(e) => {
                    self.status_message = format!("Failed to save: {}", e);
                }
            }
        }
    }

    /// Open campaign from file
    fn open_campaign(&mut self) {
        if self.unsaved_changes {
            self.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::Open);
        } else {
            self.do_open_campaign();
        }
    }

    fn do_open_campaign(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .pick_file()
        {
            match self.load_campaign_file(&path) {
                Ok(()) => {
                    self.campaign_path = Some(path.clone());

                    // Set campaign directory
                    if let Some(parent) = path.parent() {
                        let parent_buf = parent.to_path_buf();
                        self.campaign_dir = Some(parent_buf.clone());
                        self.update_file_tree(&parent_buf);
                    }

                    // Load data files
                    self.load_items();
                    self.load_spells();
                    self.load_monsters();
                    self.load_maps();

                    self.unsaved_changes = false;
                    self.status_message = format!("Opened campaign from: {}", path.display());
                }
                Err(e) => {
                    self.status_message = format!("Failed to load campaign: {}", e);
                }
            }
        }
    }

    fn load_campaign_file(&mut self, path: &PathBuf) -> Result<(), CampaignError> {
        let contents = fs::read_to_string(path)?;
        self.campaign = ron::from_str(&contents)?;
        Ok(())
    }

    /// Update the file tree view
    fn update_file_tree(&mut self, dir: &PathBuf) {
        self.file_tree.clear();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    let node = FileNode {
                        name,
                        path: path.clone(),
                        is_directory: metadata.is_dir(),
                        children: if metadata.is_dir() {
                            self.read_directory(&path)
                        } else {
                            Vec::new()
                        },
                    };

                    self.file_tree.push(node);
                }
            }
        }

        // Sort: directories first, then alphabetically
        self.file_tree
            .sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
    }

    fn read_directory(&self, dir: &PathBuf) -> Vec<FileNode> {
        let mut children = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    children.push(FileNode {
                        name,
                        path,
                        is_directory: metadata.is_dir(),
                        children: Vec::new(), // Don't recurse deeper for now
                    });
                }
            }
        }

        children.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        children
    }

    /// Check for unsaved changes before action
    fn check_unsaved_and_exit(&mut self) {
        if self.unsaved_changes {
            self.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::Exit);
        } else {
            std::process::exit(0);
        }
    }
}

impl eframe::App for CampaignBuilderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("🆕 New Campaign").clicked() {
                        self.new_campaign();
                        ui.close_menu();
                    }
                    if ui.button("📂 Open Campaign...").clicked() {
                        self.open_campaign();
                        ui.close_menu();
                    }
                    if ui.button("💾 Save").clicked() {
                        if self.campaign_path.is_some() {
                            if let Err(e) = self.save_campaign() {
                                self.status_message = format!("Save failed: {}", e);
                            }
                        } else {
                            self.save_campaign_as();
                        }
                        ui.close_menu();
                    }
                    if ui.button("💾 Save As...").clicked() {
                        self.save_campaign_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🚪 Exit").clicked() {
                        self.check_unsaved_and_exit();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("✅ Validate Campaign").clicked() {
                        self.validate_campaign();
                        self.active_tab = EditorTab::Validation;
                        ui.close_menu();
                    }
                    if ui.button("🔄 Refresh File Tree").clicked() {
                        if let Some(dir) = self.campaign_dir.clone() {
                            self.update_file_tree(&dir);
                            self.status_message = "File tree refreshed.".to_string();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🧪 Test Play").clicked() {
                        self.status_message = "Test play would launch the game here...".to_string();
                        ui.close_menu();
                    }
                    if ui.button("📦 Export Campaign...").clicked() {
                        self.status_message =
                            "Export would create .zip archive here...".to_string();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("📖 Documentation").clicked() {
                        self.status_message = "Would open documentation in browser...".to_string();
                        ui.close_menu();
                    }
                    if ui.button("ℹ️ About").clicked() {
                        self.show_about_dialog = true;
                        ui.close_menu();
                    }
                });

                // Right-aligned status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.unsaved_changes {
                        ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "● Unsaved changes");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "✓ Saved");
                    }
                });
            });
        });

        // Left sidebar with tabs
        egui::SidePanel::left("tab_panel")
            .resizable(false)
            .exact_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Editors");
                ui.separator();

                let tabs = [
                    EditorTab::Metadata,
                    EditorTab::Config,
                    EditorTab::Items,
                    EditorTab::Spells,
                    EditorTab::Monsters,
                    EditorTab::Maps,
                    EditorTab::Quests,
                    EditorTab::Files,
                    EditorTab::Validation,
                ];

                for tab in &tabs {
                    let is_selected = self.active_tab == *tab;
                    if ui.selectable_label(is_selected, tab.name()).clicked() {
                        self.active_tab = *tab;
                    }
                }

                ui.separator();
                ui.label("Phase 2: Foundation");
                ui.label("Powered by egui");
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(&self.status_message);

                if let Some(path) = &self.campaign_path {
                    ui.separator();
                    ui.label(format!("Path: {}", path.display()));
                }
            });
        });

        // Central panel with editor content
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            EditorTab::Metadata => self.show_metadata_editor(ui),
            EditorTab::Config => self.show_config_editor(ui),
            EditorTab::Items => self.show_items_editor(ui),
            EditorTab::Spells => self.show_spells_editor(ui),
            EditorTab::Monsters => self.show_monsters_editor(ui),
            EditorTab::Maps => self.show_maps_editor(ui),
            EditorTab::Quests => self.show_quests_editor(ui),
            EditorTab::Files => self.show_file_browser(ui),
            EditorTab::Validation => self.show_validation_panel(ui),
        });

        // About dialog
        if self.show_about_dialog {
            egui::Window::new("About Antares Campaign Builder")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Antares Campaign Builder");
                        ui.label("Phase 2: Foundation v0.2.0");
                        ui.separator();
                        ui.label("A visual editor for creating custom");
                        ui.label("campaigns for the Antares RPG engine.");
                        ui.separator();
                        ui.label("Phase 2 Features:");
                        ui.label("✓ Full metadata editing");
                        ui.label("✓ Real file I/O (campaign.ron)");
                        ui.label("✓ Enhanced validation UI");
                        ui.label("✓ File structure browser");
                        ui.label("✓ Data editor placeholders");
                        ui.separator();
                        ui.label("Built with egui - works without GPU!");
                        ui.separator();
                        if ui.button("Close").clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
        }

        // Unsaved changes warning
        if self.show_unsaved_warning {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes.");
                    ui.label("Do you want to save before continuing?");
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("💾 Save").clicked() {
                            if self.campaign_path.is_some() {
                                let _ = self.save_campaign();
                            } else {
                                self.save_campaign_as();
                            }

                            // Execute pending action
                            if let Some(action) = self.pending_action.take() {
                                match action {
                                    PendingAction::New => self.do_new_campaign(),
                                    PendingAction::Open => self.do_open_campaign(),
                                    PendingAction::Exit => std::process::exit(0),
                                }
                            }

                            self.show_unsaved_warning = false;
                        }

                        if ui.button("🚫 Don't Save").clicked() {
                            // Execute pending action without saving
                            if let Some(action) = self.pending_action.take() {
                                match action {
                                    PendingAction::New => self.do_new_campaign(),
                                    PendingAction::Open => self.do_open_campaign(),
                                    PendingAction::Exit => std::process::exit(0),
                                }
                            }

                            self.show_unsaved_warning = false;
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.show_unsaved_warning = false;
                            self.pending_action = None;
                        }
                    });
                });
        }
    }
}

impl CampaignBuilderApp {
    /// Show the metadata editor
    fn show_metadata_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Campaign Metadata");
        ui.add_space(5.0);
        ui.label("Basic information about your campaign");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("metadata_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    // Campaign ID
                    ui.label("Campaign ID:");
                    if ui.text_edit_singleline(&mut self.campaign.id).changed() {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();

                    // Campaign Name
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut self.campaign.name).changed() {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();

                    // Version
                    ui.label("Version:");
                    if ui
                        .text_edit_singleline(&mut self.campaign.version)
                        .changed()
                    {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();

                    // Author
                    ui.label("Author:");
                    if ui.text_edit_singleline(&mut self.campaign.author).changed() {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();

                    // Engine Version
                    ui.label("Engine Version:");
                    if ui
                        .text_edit_singleline(&mut self.campaign.engine_version)
                        .changed()
                    {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.label("Description:");
            let response =
                ui.add(egui::TextEdit::multiline(&mut self.campaign.description).desired_rows(6));
            if response.changed() {
                self.unsaved_changes = true;
            }

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save Campaign").clicked() {
                    if self.campaign_path.is_some() {
                        if let Err(e) = self.save_campaign() {
                            self.status_message = format!("Save failed: {}", e);
                        }
                    } else {
                        self.save_campaign_as();
                    }
                }

                if ui.button("✅ Validate").clicked() {
                    self.validate_campaign();
                    self.active_tab = EditorTab::Validation;
                }
            });
        });
    }

    /// Show the configuration editor
    fn show_config_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Campaign Configuration");
        ui.add_space(5.0);
        ui.label("Game rules and starting conditions");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("Starting Conditions");
                // This file contains the rest of the implementation
                // Will be appended to main.rs

                egui::Grid::new("starting_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Starting Map:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.starting_map)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Starting Position (X, Y):");
                        ui.horizontal(|ui| {
                            let mut x_str = self.campaign.starting_position.0.to_string();
                            if ui.text_edit_singleline(&mut x_str).changed() {
                                if let Ok(x) = x_str.parse::<u32>() {
                                    self.campaign.starting_position.0 = x;
                                    self.unsaved_changes = true;
                                }
                            }
                            ui.label(",");
                            let mut y_str = self.campaign.starting_position.1.to_string();
                            if ui.text_edit_singleline(&mut y_str).changed() {
                                if let Ok(y) = y_str.parse::<u32>() {
                                    self.campaign.starting_position.1 = y;
                                    self.unsaved_changes = true;
                                }
                            }
                        });
                        ui.end_row();

                        ui.label("Starting Direction:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.starting_direction)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Starting Gold:");
                        let mut gold_str = self.campaign.starting_gold.to_string();
                        if ui.text_edit_singleline(&mut gold_str).changed() {
                            if let Ok(gold) = gold_str.parse::<u32>() {
                                self.campaign.starting_gold = gold;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();

                        ui.label("Starting Food:");
                        let mut food_str = self.campaign.starting_food.to_string();
                        if ui.text_edit_singleline(&mut food_str).changed() {
                            if let Ok(food) = food_str.parse::<u32>() {
                                self.campaign.starting_food = food;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Party & Roster Settings");
                egui::Grid::new("party_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Max Party Size:");
                        let mut party_str = self.campaign.max_party_size.to_string();
                        if ui.text_edit_singleline(&mut party_str).changed() {
                            if let Ok(size) = party_str.parse::<usize>() {
                                self.campaign.max_party_size = size;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();

                        ui.label("Max Roster Size:");
                        let mut roster_str = self.campaign.max_roster_size.to_string();
                        if ui.text_edit_singleline(&mut roster_str).changed() {
                            if let Ok(size) = roster_str.parse::<usize>() {
                                self.campaign.max_roster_size = size;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Difficulty & Rules");
                egui::Grid::new("rules_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Difficulty:");
                        egui::ComboBox::from_id_salt("difficulty_combo")
                            .selected_text(self.campaign.difficulty.as_str())
                            .show_ui(ui, |ui| {
                                for diff in Difficulty::all() {
                                    if ui
                                        .selectable_value(
                                            &mut self.campaign.difficulty,
                                            diff,
                                            diff.as_str(),
                                        )
                                        .clicked()
                                    {
                                        self.unsaved_changes = true;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Permadeath:");
                        if ui.checkbox(&mut self.campaign.permadeath, "").changed() {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Allow Multiclassing:");
                        if ui
                            .checkbox(&mut self.campaign.allow_multiclassing, "")
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Starting Level:");
                        let mut start_level_str = self.campaign.starting_level.to_string();
                        if ui.text_edit_singleline(&mut start_level_str).changed() {
                            if let Ok(level) = start_level_str.parse::<u8>() {
                                self.campaign.starting_level = level;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();

                        ui.label("Max Level:");
                        let mut max_level_str = self.campaign.max_level.to_string();
                        if ui.text_edit_singleline(&mut max_level_str).changed() {
                            if let Ok(level) = max_level_str.parse::<u8>() {
                                self.campaign.max_level = level;
                                self.unsaved_changes = true;
                            }
                        }
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Data File Paths");
                egui::Grid::new("paths_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Items:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.items_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Spells:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.spells_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Monsters:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.monsters_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Classes:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.classes_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Races:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.races_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Maps Directory:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.maps_dir)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Quests:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.quests_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Dialogue:");
                        if ui
                            .text_edit_singleline(&mut self.campaign.dialogue_file)
                            .changed()
                        {
                            self.unsaved_changes = true;
                        }
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save Configuration").clicked() {
                    if self.campaign_path.is_some() {
                        if let Err(e) = self.save_campaign() {
                            self.status_message = format!("Save failed: {}", e);
                        }
                    } else {
                        self.save_campaign_as();
                    }
                }

                if ui.button("✅ Validate").clicked() {
                    self.validate_campaign();
                    self.active_tab = EditorTab::Validation;
                }
            });
        });
    }

    /// Show items editor with full CRUD operations
    fn show_items_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚔️ Items Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.items_search).changed() {
                self.items_selected = None;
            }
            ui.separator();

            if ui.button("➕ Add Item").clicked() {
                self.items_editor_mode = EditorMode::Add;
                self.items_edit_buffer = Self::default_item();
                self.items_edit_buffer.id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_items();
            }

            ui.separator();
            ui.label(format!("Total: {}", self.items.len()));
        });

        ui.separator();

        match self.items_editor_mode {
            EditorMode::List => self.show_items_list(ui),
            EditorMode::Add | EditorMode::Edit => self.show_items_form(ui),
        }
    }

    /// Show items list view
    fn show_items_list(&mut self, ui: &mut egui::Ui) {
        // Clone data for display to avoid borrow issues
        let search_lower = self.items_search.to_lowercase();
        let filtered_items: Vec<(usize, String)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                search_lower.is_empty() || item.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, item)| (idx, format!("{}: {}", item.id, item.name)))
            .collect();

        let selected = self.items_selected;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        ui.horizontal(|ui| {
            // Left panel - item list
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.heading("Items");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, label) in &filtered_items {
                        let is_selected = selected == Some(*idx);
                        if ui.selectable_label(is_selected, label).clicked() {
                            new_selection = Some(*idx);
                        }
                    }

                    if filtered_items.is_empty() {
                        ui.label("No items found");
                    }
                });
            });

            ui.separator();

            // Right panel - item details
            ui.vertical(|ui| {
                if let Some(idx) = selected {
                    if idx < self.items.len() {
                        let item = self.items[idx].clone();

                        ui.heading(&item.name);
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("✏️ Edit").clicked() {
                                action = Some((idx, "edit"));
                            }
                            if ui.button("🗑️ Delete").clicked() {
                                action = Some((idx, "delete"));
                            }
                            if ui.button("📋 Duplicate").clicked() {
                                action = Some((idx, "duplicate"));
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.group(|ui| {
                                ui.label(format!("ID: {}", item.id));
                                ui.label(format!("Type: {:?}", item.item_type));
                                ui.label(format!("Base Cost: {} gold", item.base_cost));
                                ui.label(format!("Sell Cost: {} gold", item.sell_cost));
                                ui.label(format!("Cursed: {}", item.is_cursed));
                                ui.label(format!("Magical: {}", item.is_magical()));

                                if item.max_charges > 0 {
                                    ui.label(format!("Charges: {}", item.max_charges));
                                }
                            });
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select an item to view details");
                    });
                }
            });
        });

        // Apply selection change
        self.items_selected = new_selection;

        // Apply action after UI
        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.items_editor_mode = EditorMode::Edit;
                    self.items_edit_buffer = self.items[idx].clone();
                }
                "delete" => {
                    self.items.remove(idx);
                    self.items_selected = None;
                    let _ = self.save_items();
                }
                "duplicate" => {
                    let mut new_item = self.items[idx].clone();
                    new_item.id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                    new_item.name = format!("{} (Copy)", new_item.name);
                    self.items.push(new_item);
                    let _ = self.save_items();
                }
                _ => {}
            }
        }
    }

    /// Show items edit/add form
    fn show_items_form(&mut self, ui: &mut egui::Ui) {
        let is_add = self.items_editor_mode == EditorMode::Add;
        ui.heading(if is_add { "Add New Item" } else { "Edit Item" });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.items_edit_buffer.id.to_string()),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.items_edit_buffer.name);
            });

            ui.horizontal(|ui| {
                ui.label("Base Cost:");
                ui.add(egui::DragValue::new(&mut self.items_edit_buffer.base_cost).speed(1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Sell Cost:");
                ui.add(egui::DragValue::new(&mut self.items_edit_buffer.sell_cost).speed(1.0));
            });

            ui.checkbox(&mut self.items_edit_buffer.is_cursed, "Cursed");

            ui.horizontal(|ui| {
                ui.label("Max Charges:");
                ui.add(egui::DragValue::new(&mut self.items_edit_buffer.max_charges).speed(1.0));
            });

            ui.separator();

            // Type-specific editors would go here
            ui.label(format!("Type: {:?}", self.items_edit_buffer.item_type));

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    if is_add {
                        self.items.push(self.items_edit_buffer.clone());
                    } else if let Some(idx) = self.items_selected {
                        if idx < self.items.len() {
                            self.items[idx] = self.items_edit_buffer.clone();
                        }
                    }
                    let _ = self.save_items();
                    self.items_editor_mode = EditorMode::List;
                    self.status_message = "Item saved".to_string();
                }

                if ui.button("❌ Cancel").clicked() {
                    self.items_editor_mode = EditorMode::List;
                }
            });
        });
    }

    /// Show spells editor with full CRUD operations
    fn show_spells_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("✨ Spells Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.spells_search).changed() {
                self.spells_selected = None;
            }
            ui.separator();

            if ui.button("➕ Add Spell").clicked() {
                self.spells_editor_mode = EditorMode::Add;
                self.spells_edit_buffer = Self::default_spell();
                self.spells_edit_buffer.id =
                    self.spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_spells();
            }

            ui.separator();
            ui.label(format!("Total: {}", self.spells.len()));
        });

        ui.separator();

        match self.spells_editor_mode {
            EditorMode::List => self.show_spells_list(ui),
            EditorMode::Add | EditorMode::Edit => self.show_spells_form(ui),
        }
    }

    /// Show spells list view
    fn show_spells_list(&mut self, ui: &mut egui::Ui) {
        // Clone data for display to avoid borrow issues
        let search_lower = self.spells_search.to_lowercase();
        let filtered_spells: Vec<(usize, String)> = self
            .spells
            .iter()
            .enumerate()
            .filter(|(_, spell)| {
                search_lower.is_empty() || spell.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, spell)| {
                let school_icon = match spell.school {
                    SpellSchool::Cleric => "✝️",
                    SpellSchool::Sorcerer => "🔮",
                };
                (
                    idx,
                    format!("{} L{}: {}", school_icon, spell.level, spell.name),
                )
            })
            .collect();

        let selected = self.spells_selected;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        ui.horizontal(|ui| {
            // Left panel - spell list
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.heading("Spells");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, label) in &filtered_spells {
                        let is_selected = selected == Some(*idx);
                        if ui.selectable_label(is_selected, label).clicked() {
                            new_selection = Some(*idx);
                        }
                    }

                    if filtered_spells.is_empty() {
                        ui.label("No spells found");
                    }
                });
            });

            ui.separator();

            // Right panel - spell details
            ui.vertical(|ui| {
                if let Some(idx) = selected {
                    if idx < self.spells.len() {
                        let spell = self.spells[idx].clone();

                        ui.heading(&spell.name);
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("✏️ Edit").clicked() {
                                action = Some((idx, "edit"));
                            }
                            if ui.button("🗑️ Delete").clicked() {
                                action = Some((idx, "delete"));
                            }
                            if ui.button("📋 Duplicate").clicked() {
                                action = Some((idx, "duplicate"));
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.group(|ui| {
                                ui.label(format!("ID: {}", spell.id));
                                ui.label(format!("School: {:?}", spell.school));
                                ui.label(format!("Level: {}", spell.level));
                                ui.label(format!("SP Cost: {}", spell.sp_cost));
                                ui.label(format!("Gem Cost: {}", spell.gem_cost));
                                ui.label(format!("Context: {:?}", spell.context));
                                ui.label(format!("Target: {:?}", spell.target));
                                ui.separator();
                                ui.label("Description:");
                                ui.label(&spell.description);
                            });
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a spell to view details");
                    });
                }
            });
        });

        // Apply selection change
        self.spells_selected = new_selection;

        // Apply action after UI
        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.spells_editor_mode = EditorMode::Edit;
                    self.spells_edit_buffer = self.spells[idx].clone();
                }
                "delete" => {
                    self.spells.remove(idx);
                    self.spells_selected = None;
                    let _ = self.save_spells();
                }
                "duplicate" => {
                    let mut new_spell = self.spells[idx].clone();
                    new_spell.id = self.spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                    new_spell.name = format!("{} (Copy)", new_spell.name);
                    self.spells.push(new_spell);
                    let _ = self.save_spells();
                }
                _ => {}
            }
        }
    }

    /// Show spells edit/add form
    fn show_spells_form(&mut self, ui: &mut egui::Ui) {
        let is_add = self.spells_editor_mode == EditorMode::Add;
        ui.heading(if is_add {
            "Add New Spell"
        } else {
            "Edit Spell"
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.spells_edit_buffer.id.to_string()),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.spells_edit_buffer.name);
            });

            ui.horizontal(|ui| {
                ui.label("School:");
                ui.radio_value(
                    &mut self.spells_edit_buffer.school,
                    SpellSchool::Cleric,
                    "Cleric",
                );
                ui.radio_value(
                    &mut self.spells_edit_buffer.school,
                    SpellSchool::Sorcerer,
                    "Sorcerer",
                );
            });

            ui.horizontal(|ui| {
                ui.label("Level:");
                ui.add(egui::Slider::new(&mut self.spells_edit_buffer.level, 1..=7));
            });

            ui.horizontal(|ui| {
                ui.label("SP Cost:");
                ui.add(egui::DragValue::new(&mut self.spells_edit_buffer.sp_cost).speed(1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Gem Cost:");
                ui.add(egui::DragValue::new(&mut self.spells_edit_buffer.gem_cost).speed(1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Description:");
            });
            ui.text_edit_multiline(&mut self.spells_edit_buffer.description);

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    if is_add {
                        self.spells.push(self.spells_edit_buffer.clone());
                    } else if let Some(idx) = self.spells_selected {
                        if idx < self.spells.len() {
                            self.spells[idx] = self.spells_edit_buffer.clone();
                        }
                    }
                    let _ = self.save_spells();
                    self.spells_editor_mode = EditorMode::List;
                    self.status_message = "Spell saved".to_string();
                }

                if ui.button("❌ Cancel").clicked() {
                    self.spells_editor_mode = EditorMode::List;
                }
            });
        });
    }

    /// Show monsters editor with full CRUD operations
    fn show_monsters_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("👹 Monsters Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.monsters_search).changed() {
                self.monsters_selected = None;
            }
            ui.separator();

            if ui.button("➕ Add Monster").clicked() {
                self.monsters_editor_mode = EditorMode::Add;
                self.monsters_edit_buffer = Self::default_monster();
                self.monsters_edit_buffer.id =
                    self.monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_monsters();
            }

            ui.separator();
            ui.label(format!("Total: {}", self.monsters.len()));
        });

        ui.separator();

        match self.monsters_editor_mode {
            EditorMode::List => self.show_monsters_list(ui),
            EditorMode::Add | EditorMode::Edit => self.show_monsters_form(ui),
        }
    }

    /// Show monsters list view
    fn show_monsters_list(&mut self, ui: &mut egui::Ui) {
        // Clone data for display to avoid borrow issues
        let search_lower = self.monsters_search.to_lowercase();
        let filtered_monsters: Vec<(usize, String)> = self
            .monsters
            .iter()
            .enumerate()
            .filter(|(_, monster)| {
                search_lower.is_empty() || monster.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, monster)| {
                let undead_icon = if monster.is_undead { "💀" } else { "👹" };
                (
                    idx,
                    format!("{} {} (HP:{})", undead_icon, monster.name, monster.hp),
                )
            })
            .collect();

        let selected = self.monsters_selected;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        ui.horizontal(|ui| {
            // Left panel - monster list
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.heading("Monsters");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, label) in &filtered_monsters {
                        let is_selected = selected == Some(*idx);
                        if ui.selectable_label(is_selected, label).clicked() {
                            new_selection = Some(*idx);
                        }
                    }

                    if filtered_monsters.is_empty() {
                        ui.label("No monsters found");
                    }
                });
            });

            ui.separator();

            // Right panel - monster details
            ui.vertical(|ui| {
                if let Some(idx) = selected {
                    if idx < self.monsters.len() {
                        let monster = self.monsters[idx].clone();

                        ui.heading(&monster.name);
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("✏️ Edit").clicked() {
                                action = Some((idx, "edit"));
                            }
                            if ui.button("🗑️ Delete").clicked() {
                                action = Some((idx, "delete"));
                            }
                            if ui.button("📋 Duplicate").clicked() {
                                action = Some((idx, "duplicate"));
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.group(|ui| {
                                ui.label(format!("ID: {}", monster.id));
                                ui.label(format!("HP: {}", monster.hp));
                                ui.label(format!("AC: {}", monster.ac));
                                ui.label(format!("Attacks: {}", monster.attacks.len()));
                                ui.label(format!("Undead: {}", monster.is_undead));
                                ui.label(format!("Can Regenerate: {}", monster.can_regenerate));
                                ui.label(format!("Can Advance: {}", monster.can_advance));
                                ui.label(format!(
                                    "Magic Resistance: {}%",
                                    monster.magic_resistance
                                ));
                                ui.separator();
                                ui.label("Loot:");
                                ui.label(format!(
                                    "  Gold: {}-{} gp",
                                    monster.loot.gold_min, monster.loot.gold_max
                                ));
                                ui.label(format!(
                                    "  Gems: {}-{}",
                                    monster.loot.gems_min, monster.loot.gems_max
                                ));
                                ui.label(format!("  Experience: {} XP", monster.loot.experience));
                            });
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a monster to view details");
                    });
                }
            });
        });

        // Apply selection change
        self.monsters_selected = new_selection;

        // Apply action after UI
        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.monsters_editor_mode = EditorMode::Edit;
                    self.monsters_edit_buffer = self.monsters[idx].clone();
                }
                "delete" => {
                    self.monsters.remove(idx);
                    self.monsters_selected = None;
                    let _ = self.save_monsters();
                }
                "duplicate" => {
                    let mut new_monster = self.monsters[idx].clone();
                    new_monster.id = self.monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                    new_monster.name = format!("{} (Copy)", new_monster.name);
                    self.monsters.push(new_monster);
                    let _ = self.save_monsters();
                }
                _ => {}
            }
        }
    }

    /// Show monsters edit/add form
    fn show_monsters_form(&mut self, ui: &mut egui::Ui) {
        let is_add = self.monsters_editor_mode == EditorMode::Add;
        ui.heading(if is_add {
            "Add New Monster"
        } else {
            "Edit Monster"
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.monsters_edit_buffer.id.to_string()),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.monsters_edit_buffer.name);
            });

            ui.horizontal(|ui| {
                ui.label("HP:");
                ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.hp).speed(1.0));
            });

            ui.horizontal(|ui| {
                ui.label("AC:");
                ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.ac).speed(1.0));
            });

            ui.checkbox(&mut self.monsters_edit_buffer.is_undead, "Undead");
            ui.checkbox(
                &mut self.monsters_edit_buffer.can_regenerate,
                "Can Regenerate",
            );
            ui.checkbox(&mut self.monsters_edit_buffer.can_advance, "Can Advance");

            ui.horizontal(|ui| {
                ui.label("Magic Resistance:");
                ui.add(egui::Slider::new(
                    &mut self.monsters_edit_buffer.magic_resistance,
                    0..=100,
                ));
            });

            ui.separator();
            ui.label("Loot Table:");

            ui.horizontal(|ui| {
                ui.label("Gold Min:");
                ui.add(
                    egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gold_min).speed(1.0),
                );
                ui.label("Max:");
                ui.add(
                    egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gold_max).speed(1.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Gems Min:");
                ui.add(
                    egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gems_min).speed(1.0),
                );
                ui.label("Max:");
                ui.add(
                    egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gems_max).speed(1.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Experience:");
                ui.add(
                    egui::DragValue::new(&mut self.monsters_edit_buffer.loot.experience).speed(1.0),
                );
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    if is_add {
                        self.monsters.push(self.monsters_edit_buffer.clone());
                    } else if let Some(idx) = self.monsters_selected {
                        if idx < self.monsters.len() {
                            self.monsters[idx] = self.monsters_edit_buffer.clone();
                        }
                    }
                    let _ = self.save_monsters();
                    self.monsters_editor_mode = EditorMode::List;
                    self.status_message = "Monster saved".to_string();
                }

                if ui.button("❌ Cancel").clicked() {
                    self.monsters_editor_mode = EditorMode::List;
                }
            });
        });
    }

    /// Show maps editor with integrated map editor
    fn show_maps_editor(&mut self, ui: &mut egui::Ui) {
        match self.maps_editor_mode {
            EditorMode::List => self.show_maps_list(ui),
            EditorMode::Add | EditorMode::Edit => self.show_map_editor_panel(ui),
        }
    }

    /// Show maps list view
    fn show_maps_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("🗺️ Maps Editor");
        ui.add_space(5.0);
        ui.label("Manage world maps and dungeons");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.maps_search);
            ui.separator();

            if ui.button("➕ New Map").clicked() {
                // Create a new empty map
                let new_id = self.maps.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                let new_map = Map::new(new_id, 20, 20);
                self.maps.push(new_map.clone());
                self.maps_selected = Some(self.maps.len() - 1);
                self.map_editor_state = Some(MapEditorState::new(new_map));
                self.maps_editor_mode = EditorMode::Add;
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_maps();
            }
        });

        ui.separator();

        // Map list with previews
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.maps.is_empty() {
                ui.group(|ui| {
                    ui.label("No maps found");
                    ui.label("Create a new map or load maps from:");
                    ui.monospace(&self.campaign.maps_dir);
                });
            } else {
                let mut to_delete = None;
                let mut to_edit = None;

                for (idx, map) in self.maps.iter().enumerate() {
                    let filter_match = self.maps_search.is_empty()
                        || map.id.to_string().contains(&self.maps_search);

                    if !filter_match {
                        continue;
                    }

                    let is_selected = self.maps_selected == Some(idx);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_width(ui.available_width());

                            ui.vertical(|ui| {
                                ui.strong(format!("Map ID: {}", map.id));
                                ui.label(format!("Size: {}x{}", map.width, map.height));
                                ui.label(format!("Events: {}", map.events.len()));
                                ui.label(format!("NPCs: {}", map.npcs.len()));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🗑").on_hover_text("Delete map").clicked() {
                                        to_delete = Some(idx);
                                    }

                                    if ui.button("✏️").on_hover_text("Edit map").clicked() {
                                        to_edit = Some(idx);
                                    }
                                },
                            );
                        });

                        // Show mini preview
                        if is_selected {
                            ui.separator();
                            ui.label("Preview:");
                            self.show_map_preview(ui, map);
                        }
                    });

                    ui.add_space(5.0);
                }

                // Handle actions after iteration
                if let Some(idx) = to_delete {
                    self.maps.remove(idx);
                    if self.maps_selected == Some(idx) {
                        self.maps_selected = None;
                    }
                }

                if let Some(idx) = to_edit {
                    if let Some(map) = self.maps.get(idx) {
                        self.maps_selected = Some(idx);
                        self.map_editor_state = Some(MapEditorState::new(map.clone()));
                        self.maps_editor_mode = EditorMode::Edit;
                    }
                }
            }
        });
    }

    /// Show map editor panel
    fn show_map_editor_panel(&mut self, ui: &mut egui::Ui) {
        let mut back_clicked = false;
        let mut save_clicked = false;

        if let Some(ref mut editor_state) = self.map_editor_state {
            ui.horizontal(|ui| {
                if ui.button("← Back to List").clicked() {
                    back_clicked = true;
                }

                ui.separator();

                if ui
                    .button("💾 Save")
                    .on_hover_text("Save map to file")
                    .clicked()
                {
                    save_clicked = true;
                }
            });

            ui.separator();

            // Show the map editor widget
            let mut widget = MapEditorWidget::new(editor_state);
            widget.show(ui);
        } else {
            ui.label("No map editor state available");
            if ui.button("Back").clicked() {
                self.maps_editor_mode = EditorMode::List;
            }
            return;
        }

        // Handle actions after borrowing editor_state
        if back_clicked {
            // Extract data we need before any mutable borrows
            let (map_to_save, has_changes, selected_idx) =
                if let Some(ref editor_state) = self.map_editor_state {
                    (
                        Some(editor_state.map.clone()),
                        editor_state.has_changes,
                        self.maps_selected,
                    )
                } else {
                    (None, false, None)
                };

            if has_changes {
                if let Some(map) = map_to_save {
                    // Save map before going back
                    if let Err(e) = self.save_map(&map) {
                        self.status_message = format!("Failed to save map: {}", e);
                    } else {
                        // Update the map in the list
                        if let Some(idx) = selected_idx {
                            if idx < self.maps.len() {
                                self.maps[idx] = map;
                            }
                        }
                        self.status_message = "Map saved".to_string();
                    }
                }
            }

            self.maps_editor_mode = EditorMode::List;
            self.map_editor_state = None;
        }

        if save_clicked {
            // Extract data we need before any mutable borrows
            let (map_to_save, selected_idx) = if let Some(ref editor_state) = self.map_editor_state
            {
                (Some(editor_state.map.clone()), self.maps_selected)
            } else {
                (None, None)
            };

            if let Some(map) = map_to_save {
                match self.save_map(&map) {
                    Ok(_) => {
                        // Update the map in the list
                        if let Some(idx) = selected_idx {
                            if idx < self.maps.len() {
                                self.maps[idx] = map;
                            }
                        }

                        // Now we can safely borrow mutably to update the state
                        if let Some(ref mut editor_state) = self.map_editor_state {
                            editor_state.has_changes = false;
                        }

                        self.status_message = "Map saved successfully".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save map: {}", e);
                    }
                }
            }
        }
    }

    /// Show a small preview of a map
    fn show_map_preview(&self, ui: &mut egui::Ui, map: &Map) {
        let tile_size = 8.0;
        let preview_width = (map.width.min(30) as f32 * tile_size).min(240.0);
        let preview_height = (map.height.min(20) as f32 * tile_size).min(160.0);

        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(preview_width, preview_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        let scale_x = preview_width / (map.width as f32 * tile_size);
        let scale_y = preview_height / (map.height as f32 * tile_size);
        let scale = scale_x.min(scale_y);

        let actual_tile_size = tile_size * scale;

        // Draw a simplified view of the map
        for y in 0..map.height.min(20) {
            for x in 0..map.width.min(30) {
                let pos = antares::domain::types::Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    let has_event = map.events.contains_key(&pos);
                    let has_npc = map.npcs.iter().any(|npc| npc.position == pos);

                    let color = if has_event {
                        egui::Color32::RED
                    } else if has_npc {
                        egui::Color32::YELLOW
                    } else if tile.blocked {
                        egui::Color32::DARK_GRAY
                    } else {
                        egui::Color32::LIGHT_GRAY
                    };

                    let tile_rect = egui::Rect::from_min_size(
                        rect.min
                            + egui::Vec2::new(
                                x as f32 * actual_tile_size,
                                y as f32 * actual_tile_size,
                            ),
                        egui::Vec2::new(actual_tile_size, actual_tile_size),
                    );

                    painter.rect_filled(tile_rect, 0.0, color);
                }
            }
        }
    }

    /// Show quests editor (placeholder with list view)
    fn show_quests_editor(&self, ui: &mut egui::Ui) {
        ui.heading("📜 Quests Editor");
        ui.add_space(5.0);
        ui.label("Manage quest chains and objectives");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut String::new());
            ui.separator();
            if ui.button("➕ Add Quest").clicked() {
                // Will be implemented in Phase 5
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("📋 Quest List (Placeholder)");
                ui.separator();
                ui.label("No quests loaded. Quests will be loaded from:");
                ui.monospace(&self.campaign.quests_file);
                ui.add_space(10.0);
                ui.label("Phase 5 will add:");
                ui.label("  • Quest designer UI");
                ui.label("  • Objective chains");
                ui.label("  • Reward configuration");
                ui.label("  • Prerequisite system");
                ui.label("  • Quest state tracking");
            });
        });
    }

    /// Show file browser
    fn show_file_browser(&self, ui: &mut egui::Ui) {
        ui.heading("📁 Campaign File Structure");
        ui.add_space(5.0);
        ui.label("Browse files in your campaign directory");
        ui.separator();

        if let Some(dir) = &self.campaign_dir {
            ui.label(format!("Campaign Directory: {}", dir.display()));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.file_tree.is_empty() {
                    ui.label("No files loaded. Use Tools > Refresh File Tree");
                } else {
                    for node in &self.file_tree {
                        self.show_file_node(ui, node, 0);
                    }
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No campaign directory loaded");
                ui.label("Open or save a campaign to view its file structure");
            });
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn show_file_node(&self, ui: &mut egui::Ui, node: &FileNode, depth: usize) {
        let indent = depth as f32 * 20.0;
        ui.horizontal(|ui| {
            ui.add_space(indent);

            let icon = if node.is_directory { "📁" } else { "📄" };
            ui.label(format!("{} {}", icon, node.name));
        });

        if node.is_directory && !node.children.is_empty() {
            for child in &node.children {
                self.show_file_node(ui, child, depth + 1);
            }
        }
    }

    /// Show validation results panel
    fn show_validation_panel(&self, ui: &mut egui::Ui) {
        ui.heading("✅ Campaign Validation");
        ui.add_space(5.0);
        ui.label("Check your campaign for errors and warnings");
        ui.separator();

        let error_count = self
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count();
        let warning_count = self
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Warning)
            .count();

        if self.validation_errors.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.heading("✅ All Checks Passed!");
                ui.label("Your campaign configuration is valid.");
                ui.add_space(20.0);
                ui.label("You can now:");
                ui.label("• Save your campaign");
                ui.label("• Add data (items, spells, monsters)");
                ui.label("• Create maps");
                ui.label("• Test play your campaign");
            });
        } else {
            ui.horizontal(|ui| {
                if error_count > 0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 0, 0),
                        format!("❌ {} Error(s)", error_count),
                    );
                }
                if warning_count > 0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        format!("⚠️ {} Warning(s)", warning_count),
                    );
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for error in &self.validation_errors {
                    ui.horizontal(|ui| {
                        let color = match error.severity {
                            Severity::Error => egui::Color32::from_rgb(255, 0, 0),
                            Severity::Warning => egui::Color32::from_rgb(255, 165, 0),
                        };

                        ui.colored_label(color, error.severity.icon());
                        ui.label(&error.message);
                    });
                    ui.add_space(5.0);
                }
            });

            ui.separator();
            ui.label("💡 Tip: Fix errors in the Metadata and Config tabs");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_metadata_default() {
        let campaign = CampaignMetadata::default();
        assert_eq!(campaign.id, "");
        assert_eq!(campaign.name, "");
        assert_eq!(campaign.version, "1.0.0");
        assert_eq!(campaign.engine_version, "0.1.0");
        assert_eq!(campaign.starting_map, "starter_town");
        assert_eq!(campaign.starting_position, (10, 10));
        assert_eq!(campaign.max_party_size, 6);
        assert_eq!(campaign.max_roster_size, 20);
        assert_eq!(campaign.difficulty, Difficulty::Normal);
        assert!(!campaign.permadeath);
        assert!(!campaign.allow_multiclassing);
        assert_eq!(campaign.starting_level, 1);
        assert_eq!(campaign.max_level, 20);
    }

    #[test]
    fn test_difficulty_as_str() {
        assert_eq!(Difficulty::Easy.as_str(), "Easy");
        assert_eq!(Difficulty::Normal.as_str(), "Normal");
        assert_eq!(Difficulty::Hard.as_str(), "Hard");
        assert_eq!(Difficulty::Brutal.as_str(), "Brutal");
    }

    #[test]
    fn test_difficulty_default() {
        let diff: Difficulty = Default::default();
        assert_eq!(diff, Difficulty::Normal);
    }

    #[test]
    fn test_validation_empty_id() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "".to_string();
        app.validate_campaign();

        let has_id_error = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("Campaign ID is required"));
        assert!(has_id_error);
    }

    #[test]
    fn test_validation_invalid_id_characters() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "invalid-id-with-dashes".to_string();
        app.validate_campaign();

        let has_id_error = app.validation_errors.iter().any(|e| {
            e.message
                .contains("alphanumeric characters and underscores")
        });
        assert!(has_id_error);
    }

    #[test]
    fn test_validation_valid_id() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "valid_campaign_123".to_string();
        app.campaign.name = "Valid Campaign".to_string();
        app.campaign.author = "Test Author".to_string();
        app.campaign.starting_map = "test_map".to_string();
        app.validate_campaign();

        let has_id_error = app
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .any(|e| e.message.contains("Campaign ID"));
        assert!(!has_id_error);
    }

    #[test]
    fn test_validation_version_format() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.version = "invalid".to_string();
        app.validate_campaign();

        let has_version_error = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("semantic versioning"));
        assert!(has_version_error);
    }

    #[test]
    fn test_validation_roster_size_less_than_party() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test".to_string();
        app.campaign.max_party_size = 10;
        app.campaign.max_roster_size = 5;
        app.validate_campaign();

        let has_roster_error = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("roster size must be >= max party size"));
        assert!(has_roster_error);
    }

    #[test]
    fn test_validation_starting_level_invalid() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test".to_string();
        app.campaign.starting_level = 0;
        app.validate_campaign();

        let has_level_error = app.validation_errors.iter().any(|e| {
            e.message
                .contains("Starting level must be between 1 and max level")
        });
        assert!(has_level_error);
    }

    #[test]
    fn test_validation_file_paths_empty() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test".to_string();
        app.campaign.items_file = "".to_string();
        app.validate_campaign();

        let has_path_error = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("Items file path is required"));
        assert!(has_path_error);
    }

    #[test]
    fn test_validation_file_paths_wrong_extension() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test".to_string();
        app.campaign.items_file = "data/items.json".to_string();
        app.validate_campaign();

        let has_extension_warning = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("should use .ron extension"));
        assert!(has_extension_warning);
    }

    #[test]
    fn test_validation_all_pass() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test_campaign".to_string();
        app.campaign.name = "Test Campaign".to_string();
        app.campaign.author = "Test Author".to_string();
        app.campaign.version = "1.0.0".to_string();
        app.campaign.engine_version = "0.1.0".to_string();
        app.campaign.starting_map = "test_map".to_string();
        app.validate_campaign();

        let error_count = app
            .validation_errors
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count();
        assert_eq!(error_count, 0);
    }

    #[test]
    fn test_save_campaign_no_path() {
        let mut app = CampaignBuilderApp::default();
        let result = app.save_campaign();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CampaignError::NoPath));
    }

    #[test]
    fn test_ron_serialization() {
        let campaign = CampaignMetadata {
            id: "test_campaign".to_string(),
            name: "Test Campaign".to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test campaign".to_string(),
            engine_version: "0.1.0".to_string(),
            starting_map: "test_map".to_string(),
            starting_position: (5, 5),
            starting_direction: "North".to_string(),
            starting_gold: 200,
            starting_food: 20,
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: Difficulty::Hard,
            permadeath: true,
            allow_multiclassing: true,
            starting_level: 2,
            max_level: 15,
            items_file: "data/items.ron".to_string(),
            spells_file: "data/spells.ron".to_string(),
            monsters_file: "data/monsters.ron".to_string(),
            classes_file: "data/classes.ron".to_string(),
            races_file: "data/races.ron".to_string(),
            maps_dir: "data/maps/".to_string(),
            quests_file: "data/quests.ron".to_string(),
            dialogue_file: "data/dialogue.ron".to_string(),
        };

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(4);

        let serialized = ron::ser::to_string_pretty(&campaign, ron_config);
        assert!(serialized.is_ok());

        let ron_string = serialized.unwrap();
        assert!(ron_string.contains("test_campaign"));
        assert!(ron_string.contains("Test Campaign"));

        // Test deserialization
        let deserialized: Result<CampaignMetadata, _> = ron::from_str(&ron_string);
        assert!(deserialized.is_ok());

        let loaded = deserialized.unwrap();
        assert_eq!(loaded.id, campaign.id);
        assert_eq!(loaded.name, campaign.name);
        assert_eq!(loaded.difficulty, campaign.difficulty);
        assert_eq!(loaded.permadeath, campaign.permadeath);
    }

    #[test]
    fn test_unsaved_changes_tracking() {
        let mut app = CampaignBuilderApp::default();
        assert!(!app.unsaved_changes);

        // Simulate a change
        app.campaign.name = "Changed".to_string();
        app.unsaved_changes = true;
        assert!(app.unsaved_changes);
    }

    #[test]
    fn test_editor_tab_names() {
        assert_eq!(EditorTab::Metadata.name(), "📋 Metadata");
        assert_eq!(EditorTab::Config.name(), "⚙️ Config");
        assert_eq!(EditorTab::Items.name(), "⚔️ Items");
        assert_eq!(EditorTab::Spells.name(), "✨ Spells");
        assert_eq!(EditorTab::Monsters.name(), "👹 Monsters");
        assert_eq!(EditorTab::Maps.name(), "🗺️ Maps");
        assert_eq!(EditorTab::Quests.name(), "📜 Quests");
        assert_eq!(EditorTab::Files.name(), "📁 Files");
        assert_eq!(EditorTab::Validation.name(), "✅ Validation");
    }

    #[test]
    fn test_severity_icons() {
        assert_eq!(Severity::Error.icon(), "❌");
        assert_eq!(Severity::Warning.icon(), "⚠️");
    }

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError {
            severity: Severity::Error,
            message: "Test error".to_string(),
        };
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut app = CampaignBuilderApp::default();
        assert_eq!(app.items_editor_mode, EditorMode::List);

        // Simulate adding an item
        app.items_editor_mode = EditorMode::Add;
        assert_eq!(app.items_editor_mode, EditorMode::Add);

        // Simulate editing an item
        app.items_editor_mode = EditorMode::Edit;
        assert_eq!(app.items_editor_mode, EditorMode::Edit);

        // Return to list
        app.items_editor_mode = EditorMode::List;
        assert_eq!(app.items_editor_mode, EditorMode::List);
    }

    #[test]
    fn test_default_item_creation() {
        let item = CampaignBuilderApp::default_item();
        assert_eq!(item.id, 0);
        assert_eq!(item.name, "");
        assert!(matches!(item.item_type, ItemType::Weapon(_)));
        assert_eq!(item.base_cost, 0);
        assert_eq!(item.sell_cost, 0);
        assert!(!item.is_cursed);
    }

    #[test]
    fn test_default_spell_creation() {
        let spell = CampaignBuilderApp::default_spell();
        assert_eq!(spell.id, 0);
        assert_eq!(spell.name, "");
        assert_eq!(spell.school, SpellSchool::Cleric);
        assert_eq!(spell.level, 1);
        assert_eq!(spell.sp_cost, 1);
        assert_eq!(spell.gem_cost, 0);
    }

    #[test]
    fn test_default_monster_creation() {
        let monster = CampaignBuilderApp::default_monster();
        assert_eq!(monster.id, 0);
        assert_eq!(monster.name, "");
        assert_eq!(monster.hp, 10);
        assert_eq!(monster.ac, 10);
        assert!(!monster.is_undead);
        assert!(!monster.can_regenerate);
        assert!(monster.can_advance);
        assert_eq!(monster.magic_resistance, 0);
    }

    #[test]
    fn test_items_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.items.len(), 0);
        assert_eq!(app.items_search, "");
        assert_eq!(app.items_selected, None);
        assert_eq!(app.items_editor_mode, EditorMode::List);
    }

    #[test]
    fn test_spells_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.spells.len(), 0);
        assert_eq!(app.spells_search, "");
        assert_eq!(app.spells_selected, None);
        assert_eq!(app.spells_editor_mode, EditorMode::List);
    }

    #[test]
    fn test_monsters_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.monsters.len(), 0);
        assert_eq!(app.monsters_search, "");
        assert_eq!(app.monsters_selected, None);
        assert_eq!(app.monsters_editor_mode, EditorMode::List);
    }

    #[test]
    fn test_item_type_detection() {
        let item = CampaignBuilderApp::default_item();
        assert!(item.is_weapon());
        assert!(!item.is_armor());
        assert!(!item.is_accessory());
        assert!(!item.is_consumable());
        assert!(!item.is_ammo());
        assert!(!item.is_quest_item());
    }

    #[test]
    fn test_spell_school_types() {
        let mut spell = CampaignBuilderApp::default_spell();
        spell.school = SpellSchool::Cleric;
        assert_eq!(spell.school, SpellSchool::Cleric);

        spell.school = SpellSchool::Sorcerer;
        assert_eq!(spell.school, SpellSchool::Sorcerer);
    }

    #[test]
    fn test_monster_flags() {
        let mut monster = CampaignBuilderApp::default_monster();
        assert!(!monster.is_undead);
        assert!(!monster.can_regenerate);
        assert!(monster.can_advance);

        monster.is_undead = true;
        monster.can_regenerate = true;
        monster.can_advance = false;

        assert!(monster.is_undead);
        assert!(monster.can_regenerate);
        assert!(!monster.can_advance);
    }

    #[test]
    fn test_loot_table_initialization() {
        let monster = CampaignBuilderApp::default_monster();
        assert_eq!(monster.loot.gold_min, 0);
        assert_eq!(monster.loot.gold_max, 0);
        assert_eq!(monster.loot.gems_min, 0);
        assert_eq!(monster.loot.gems_max, 0);
        assert_eq!(monster.loot.experience, 0);
    }

    #[test]
    fn test_editor_tab_equality() {
        assert_eq!(EditorTab::Items, EditorTab::Items);
        assert_ne!(EditorTab::Items, EditorTab::Spells);
        assert_ne!(EditorTab::Spells, EditorTab::Monsters);
    }

    #[test]
    fn test_editor_mode_equality() {
        assert_eq!(EditorMode::List, EditorMode::List);
        assert_eq!(EditorMode::Add, EditorMode::Add);
        assert_eq!(EditorMode::Edit, EditorMode::Edit);
        assert_ne!(EditorMode::List, EditorMode::Add);
        assert_ne!(EditorMode::Add, EditorMode::Edit);
    }

    #[test]
    fn test_items_id_generation() {
        let mut app = CampaignBuilderApp::default();

        // Add first item
        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        item1.name = "Item 1".to_string();
        app.items.push(item1);

        // Next ID should be 2
        let next_id = app.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        assert_eq!(next_id, 2);

        // Add second item
        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = next_id;
        item2.name = "Item 2".to_string();
        app.items.push(item2);

        assert_eq!(app.items.len(), 2);
    }

    #[test]
    fn test_spells_level_range() {
        let spell = CampaignBuilderApp::default_spell();
        assert!(spell.level >= 1 && spell.level <= 7);
    }

    #[test]
    fn test_monster_stats_initialization() {
        let monster = CampaignBuilderApp::default_monster();
        assert_eq!(monster.stats.might.base, 10);
        assert_eq!(monster.stats.intellect.base, 10);
        assert_eq!(monster.stats.personality.base, 10);
        assert_eq!(monster.stats.endurance.base, 10);
        assert_eq!(monster.stats.speed.base, 10);
        assert_eq!(monster.stats.accuracy.base, 10);
        assert_eq!(monster.stats.luck.base, 10);
    }

    #[test]
    fn test_attack_initialization() {
        let monster = CampaignBuilderApp::default_monster();
        assert_eq!(monster.attacks.len(), 1);
        assert_eq!(monster.attacks[0].damage.count, 1);
        assert_eq!(monster.attacks[0].damage.sides, 4);
        assert!(matches!(
            monster.attacks[0].attack_type,
            AttackType::Physical
        ));
    }

    #[test]
    fn test_magic_resistance_range() {
        let monster = CampaignBuilderApp::default_monster();
        assert!(monster.magic_resistance <= 100);
    }
}
