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

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::useless_conversion)]

mod advanced_validation;
mod asset_manager;
mod classes_editor;
mod dialogue_editor;
mod items_editor;
mod map_editor;
mod monsters_editor;
mod packager;
mod quest_editor;
mod spells_editor;
mod templates;
mod test_play;
mod undo_redo;

use antares::domain::character::Stats;
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
use antares::domain::dialogue::{DialogueTree, NodeId};
use antares::domain::items::types::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorData, AttributeType, Bonus,
    BonusAttribute, ConsumableData, ConsumableEffect, Disablement, Item, ItemType, QuestData,
    WeaponData,
};
use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
use antares::domain::quest::{Quest, QuestId};
use antares::domain::types::{DiceRoll, ItemId, MapId, MonsterId, SpellId};
use antares::domain::world::Map;
use dialogue_editor::DialogueEditorState;
use eframe::egui;
use items_editor::ItemsEditorState;
use map_editor::{MapEditorState, MapEditorWidget};
use monsters_editor::MonstersEditorState;
use quest_editor::QuestEditorState;
use serde::{Deserialize, Serialize};
use spells_editor::SpellsEditorState;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Metadata,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
    Classes,
    Dialogues,
    Assets,
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
            EditorTab::Metadata => "Metadata",
            EditorTab::Items => "Items",
            EditorTab::Spells => "Spells",
            EditorTab::Monsters => "Monsters",
            EditorTab::Maps => "Maps",
            EditorTab::Quests => "Quests",
            EditorTab::Classes => "Classes",
            EditorTab::Dialogues => "Dialogues",
            EditorTab::Assets => "Assets",
            EditorTab::Validation => "Validation",
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

/// Item type filter for search
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemTypeFilter {
    Weapon,
    Armor,
    Accessory,
    Consumable,
    Ammo,
    Quest,
}

impl ItemTypeFilter {
    fn matches(&self, item: &Item) -> bool {
        match self {
            ItemTypeFilter::Weapon => item.is_weapon(),
            ItemTypeFilter::Armor => item.is_armor(),
            ItemTypeFilter::Accessory => item.is_accessory(),
            ItemTypeFilter::Consumable => item.is_consumable(),
            ItemTypeFilter::Ammo => item.is_ammo(),
            ItemTypeFilter::Quest => item.is_quest_item(),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            ItemTypeFilter::Weapon => "Weapon",
            ItemTypeFilter::Armor => "Armor",
            ItemTypeFilter::Accessory => "Accessory",
            ItemTypeFilter::Consumable => "Consumable",
            ItemTypeFilter::Ammo => "Ammo",
            ItemTypeFilter::Quest => "Quest",
        }
    }

    fn all() -> [ItemTypeFilter; 6] {
        [
            ItemTypeFilter::Weapon,
            ItemTypeFilter::Armor,
            ItemTypeFilter::Accessory,
            ItemTypeFilter::Consumable,
            ItemTypeFilter::Ammo,
            ItemTypeFilter::Quest,
        ]
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
    items_editor_state: ItemsEditorState,

    spells: Vec<Spell>,
    spells_editor_state: SpellsEditorState,

    monsters: Vec<MonsterDefinition>,
    monsters_editor_state: MonstersEditorState,

    // Map editor state
    maps: Vec<Map>,
    maps_search: String,
    maps_selected: Option<usize>,
    maps_editor_mode: EditorMode,
    map_editor_state: Option<MapEditorState>,

    // Quest editor state
    quests: Vec<Quest>,
    quest_editor_state: QuestEditorState,
    quests_search_filter: String,
    quests_show_preview: bool,
    quests_import_buffer: String,
    quests_show_import_dialog: bool,

    // Dialogue editor state</parameter>
    // Dialogue editor state
    dialogues: Vec<DialogueTree>,
    dialogue_editor_state: DialogueEditorState,
    dialogues_search_filter: String,
    dialogues_show_preview: bool,
    dialogues_import_buffer: String,
    dialogues_show_import_dialog: bool,

    // Classes editor state
    classes_editor_state: classes_editor::ClassesEditorState,

    // Phase 13: Distribution tools state
    export_wizard: Option<packager::ExportWizard>,
    test_play_session: Option<test_play::TestPlaySession>,
    test_play_config: test_play::TestPlayConfig,
    asset_manager: Option<asset_manager::AssetManager>,
    show_export_dialog: bool,
    show_test_play_panel: bool,
    show_asset_manager: bool,

    // Phase 15: Polish & advanced features state
    undo_redo_manager: undo_redo::UndoRedoManager,
    template_manager: templates::TemplateManager,
    show_template_browser: bool,
    template_category: templates::TemplateCategory,
    advanced_validator: Option<advanced_validation::AdvancedValidator>,
    show_validation_report: bool,
    validation_report: String,
    show_balance_stats: bool,

    // File I/O pattern state
    file_load_merge_mode: bool,
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

            items: Vec::new(),
            items_editor_state: ItemsEditorState::new(),

            spells: Vec::new(),
            spells_editor_state: SpellsEditorState::new(),

            monsters: Vec::new(),
            monsters_editor_state: MonstersEditorState::new(),

            maps: Vec::new(),
            maps_search: String::new(),
            maps_selected: None,
            maps_editor_mode: EditorMode::List,
            map_editor_state: None,

            quests: Vec::new(),
            quest_editor_state: QuestEditorState::default(),
            quests_search_filter: String::new(),
            quests_show_preview: true,
            quests_import_buffer: String::new(),
            quests_show_import_dialog: false,

            dialogues: Vec::new(),
            dialogue_editor_state: DialogueEditorState::default(),
            dialogues_search_filter: String::new(),
            dialogues_show_preview: false,
            dialogues_import_buffer: String::new(),
            dialogues_show_import_dialog: false,

            classes_editor_state: classes_editor::ClassesEditorState::default(),

            // Phase 13: Distribution tools
            export_wizard: None,
            test_play_session: None,
            test_play_config: test_play::TestPlayConfig::default(),
            asset_manager: None,
            show_export_dialog: false,
            show_test_play_panel: false,
            show_asset_manager: false,

            // Phase 15: Polish & advanced features
            undo_redo_manager: undo_redo::UndoRedoManager::new(),
            template_manager: templates::TemplateManager::new(),
            show_template_browser: false,
            template_category: templates::TemplateCategory::Item,
            advanced_validator: None,
            show_validation_report: false,
            validation_report: String::new(),
            show_balance_stats: false,

            file_load_merge_mode: true, // Default to merge mode
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
            icon_path: None,
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
            None,
            0,
            false,
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

    /// Validate item IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_item_ids(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for item in &self.items {
            if !seen_ids.insert(item.id) {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("Duplicate item ID: {}", item.id),
                });
            }
        }
        errors
    }

    /// Validate spell IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_spell_ids(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for spell in &self.spells {
            if !seen_ids.insert(spell.id) {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("Duplicate spell ID: {}", spell.id),
                });
            }
        }
        errors
    }

    /// Validate monster IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_monster_ids(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for monster in &self.monsters {
            if !seen_ids.insert(monster.id) {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("Duplicate monster ID: {}", monster.id),
                });
            }
        }
        errors
    }

    /// Validate map IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_map_ids(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for map in &self.maps {
            if !seen_ids.insert(map.id) {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("Duplicate map ID: {}", map.id),
                });
            }
        }
        errors
    }

    /// Get next available item ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    fn next_available_item_id(&self) -> ItemId {
        self.items
            .iter()
            .map(|i| i.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available spell ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    fn next_available_spell_id(&self) -> SpellId {
        self.spells
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available monster ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    fn next_available_monster_id(&self) -> MonsterId {
        self.monsters
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available map ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    fn next_available_map_id(&self) -> MapId {
        self.maps
            .iter()
            .map(|m| m.id)
            .max()
            .map(|id| id + 1)
            .unwrap_or(1)
    }

    /// Get the next available class ID (string-based, numeric format)
    fn next_available_class_id(&self) -> String {
        let max_id = self
            .classes_editor_state
            .classes
            .iter()
            .filter_map(|c| c.id.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max_id + 1).to_string()
    }

    /// Load items from RON file
    fn load_items(&mut self) {
        eprintln!("DEBUG: load_items() called");
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&self.campaign.items_file);
            if items_path.exists() {
                match fs::read_to_string(&items_path) {
                    Ok(contents) => match ron::from_str::<Vec<Item>>(&contents) {
                        Ok(items) => {
                            self.items = items;

                            // Validate IDs after loading
                            let id_errors = self.validate_item_ids();
                            if !id_errors.is_empty() {
                                self.validation_errors.extend(id_errors.clone());
                                self.status_message = format!(
                                    "⚠️ Loaded {} items with {} ID conflicts",
                                    self.items.len(),
                                    id_errors.len()
                                );
                            } else {
                                self.status_message = format!("Loaded {} items", self.items.len());
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse items: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read items file: {}", e);
                        eprintln!("Failed to read items file {:?}: {}", items_path, e);
                    }
                }
            } else {
                eprintln!("Items file does not exist: {:?}", items_path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load items");
        }
    }

    /// Save items to RON file
    fn save_items(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&self.campaign.items_file);

            // Create items directory if it doesn't exist
            if let Some(parent) = items_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create items directory: {}", e))?;
            }

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

                            // Validate IDs after loading
                            let id_errors = self.validate_spell_ids();
                            if !id_errors.is_empty() {
                                self.validation_errors.extend(id_errors.clone());
                                self.status_message = format!(
                                    "⚠️ Loaded {} spells with {} ID conflicts",
                                    self.spells.len(),
                                    id_errors.len()
                                );
                            } else {
                                self.status_message =
                                    format!("Loaded {} spells", self.spells.len());
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse spells: {}", e);
                            eprintln!("Failed to parse spells from {:?}: {}", spells_path, e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read spells file: {}", e);
                        eprintln!("Failed to read spells file {:?}: {}", spells_path, e);
                    }
                }
            } else {
                eprintln!("Spells file does not exist: {:?}", spells_path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load spells");
        }
    }

    /// Save spells to RON file
    fn save_spells(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let spells_path = dir.join(&self.campaign.spells_file);

            // Create spells directory if it doesn't exist
            if let Some(parent) = spells_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create spells directory: {}", e))?;
            }

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

                            // Validate IDs after loading
                            let id_errors = self.validate_monster_ids();
                            if !id_errors.is_empty() {
                                self.validation_errors.extend(id_errors.clone());
                                self.status_message = format!(
                                    "⚠️ Loaded {} monsters with {} ID conflicts",
                                    self.monsters.len(),
                                    id_errors.len()
                                );
                            } else {
                                self.status_message =
                                    format!("Loaded {} monsters", self.monsters.len());
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse monsters: {}", e);
                            eprintln!("Failed to parse monsters from {:?}: {}", monsters_path, e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read monsters file: {}", e);
                        eprintln!("Failed to read monsters file {:?}: {}", monsters_path, e);
                    }
                }
            } else {
                eprintln!("Monsters file does not exist: {:?}", monsters_path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load monsters");
        }
    }

    /// Save monsters to RON file
    fn save_monsters(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let monsters_path = dir.join(&self.campaign.monsters_file);

            // Create monsters directory if it doesn't exist
            if let Some(parent) = monsters_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create monsters directory: {}", e))?;
            }

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

        // Validate data IDs for uniqueness
        self.validation_errors.extend(self.validate_item_ids());
        self.validation_errors.extend(self.validate_spell_ids());
        self.validation_errors.extend(self.validate_monster_ids());
        self.validation_errors.extend(self.validate_map_ids());

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
        // Clone path early to avoid borrow checker issues with mutable save methods
        let path = self.campaign_path.clone().ok_or(CampaignError::NoPath)?;

        // CRITICAL FIX: Save all data files BEFORE saving campaign metadata
        // This ensures all content is persisted when user clicks "Save Campaign"

        // Track any save failures but continue (partial save is better than no save)
        let mut save_warnings = Vec::new();

        if let Err(e) = self.save_items() {
            save_warnings.push(format!("Items: {}", e));
        }

        if let Err(e) = self.save_spells() {
            save_warnings.push(format!("Spells: {}", e));
        }

        if let Err(e) = self.save_monsters() {
            save_warnings.push(format!("Monsters: {}", e));
        }

        // Save maps individually (they're saved per-map, not as a collection)
        // Clone maps to avoid borrow checker issues
        let maps_to_save = self.maps.clone();
        for (idx, map) in maps_to_save.iter().enumerate() {
            if let Err(e) = self.save_map(map) {
                save_warnings.push(format!("Map {}: {}", idx, e));
            }
        }

        if let Err(e) = self.save_quests() {
            save_warnings.push(format!("Quests: {}", e));
        }

        if let Some(dir) = &self.campaign_dir {
            let dialogues_path = dir.join(&self.campaign.dialogue_file);
            if let Err(e) = self.save_dialogues_to_file(&dialogues_path) {
                save_warnings.push(format!("Dialogues: {}", e));
            }
        }

        // Now save campaign metadata to RON format
        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(4);

        let ron_string = ron::ser::to_string_pretty(&self.campaign, ron_config)?;

        // Write campaign metadata file
        fs::write(&path, ron_string)?;

        self.unsaved_changes = false;

        // Update status message based on results
        if save_warnings.is_empty() {
            self.status_message = format!("✅ Campaign and all data saved to: {}", path.display());
        } else {
            self.status_message = format!(
                "⚠️ Campaign saved with warnings:\n{}",
                save_warnings.join("\n")
            );
        }

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
                    eprintln!("DEBUG: About to load data files...");
                    eprintln!("DEBUG: campaign_dir = {:?}", self.campaign_dir);
                    self.load_items();
                    self.load_spells();
                    self.load_monsters();
                    self.load_classes();
                    self.load_maps();

                    // Load quests and dialogues
                    if let Err(e) = self.load_quests() {
                        eprintln!("Warning: Failed to load quests: {}", e);
                    }

                    if let Err(e) = self.load_dialogues() {
                        eprintln!("Warning: Failed to load dialogues: {}", e);
                    }

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

    /// Sync state from undo/redo manager back to app state
    fn sync_state_from_undo_redo(&mut self) {
        let state = self.undo_redo_manager.state();
        self.items = state.items.clone();
        self.spells = state.spells.clone();
        self.monsters = state.monsters.clone();
        self.maps = state.maps.clone();
        self.quests = state.quests.clone();
        self.dialogues = state.dialogues.clone();
    }

    /// Sync state to undo/redo manager (before executing commands)
    fn sync_state_to_undo_redo(&mut self) {
        let state = self.undo_redo_manager.state_mut();
        state.items = self.items.clone();
        state.spells = self.spells.clone();
        state.monsters = self.monsters.clone();
        state.maps = self.maps.clone();
        state.quests = self.quests.clone();
        state.dialogues = self.dialogues.clone();
    }

    /// Run advanced validation and generate report
    fn run_advanced_validation(&mut self) {
        let validator = advanced_validation::AdvancedValidator::new(
            self.items.clone(),
            self.monsters.clone(),
            self.quests.clone(),
            self.maps.clone(),
        );
        self.validation_report = validator.generate_report();
        self.advanced_validator = Some(validator);
    }

    /// Show template browser dialog
    fn show_template_browser_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_template_browser;
        egui::Window::new("📋 Template Browser")
            .open(&mut open)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    for category in templates::TemplateCategory::all() {
                        if ui
                            .selectable_label(self.template_category == *category, category.name())
                            .clicked()
                        {
                            self.template_category = *category;
                        }
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| match self.template_category {
                    templates::TemplateCategory::Item => {
                        for template in self.template_manager.item_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(item) = self
                                            .template_manager
                                            .create_item(&template.id, self.items.len() as u32 + 1)
                                        {
                                            self.items_editor_state.edit_buffer = item;
                                            self.items_editor_state.mode =
                                                items_editor::ItemsEditorMode::Add;
                                            self.active_tab = EditorTab::Items;
                                            self.status_message =
                                                format!("Template '{}' loaded", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Monster => {
                        for template in self.template_manager.monster_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(monster) = self.template_manager.create_monster(
                                            &template.id,
                                            self.monsters.len() as u32 + 1,
                                        ) {
                                            self.monsters_editor_state.edit_buffer = monster;
                                            self.monsters_editor_state.mode =
                                                monsters_editor::MonstersEditorMode::Add;
                                            self.active_tab = EditorTab::Monsters;
                                            self.status_message =
                                                format!("Template '{}' loaded", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Quest => {
                        for template in self.template_manager.quest_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(quest) = self.template_manager.create_quest(
                                            &template.id,
                                            self.quests.len() as u32 + 1,
                                        ) {
                                            self.quests.push(quest);
                                            self.active_tab = EditorTab::Quests;
                                            self.unsaved_changes = true;
                                            self.status_message =
                                                format!("Template '{}' added", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Dialogue => {
                        for template in self.template_manager.dialogue_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(dialogue) =
                                            self.template_manager.create_dialogue(
                                                &template.id,
                                                self.dialogues.len() as u32 + 1,
                                            )
                                        {
                                            self.dialogues.push(dialogue);
                                            self.active_tab = EditorTab::Dialogues;
                                            self.unsaved_changes = true;
                                            self.status_message =
                                                format!("Template '{}' added", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Map => {
                        ui.label("Map templates coming soon...");
                    }
                });
            });
        self.show_template_browser = open;
    }

    /// Show validation report dialog
    fn show_validation_report_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_validation_report;
        egui::Window::new("📊 Advanced Validation Report")
            .open(&mut open)
            .resizable(true)
            .default_size([700.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.monospace(&self.validation_report);
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.show_validation_report = false;
                    }
                    if ui.button("Run Again").clicked() {
                        self.run_advanced_validation();
                    }
                });
            });
        self.show_validation_report = open;
    }

    /// Show balance statistics dialog
    fn show_balance_stats_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_balance_stats;
        egui::Window::new("⚖️ Balance Statistics")
            .open(&mut open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                let validator = advanced_validation::AdvancedValidator::new(
                    self.items.clone(),
                    self.monsters.clone(),
                    self.quests.clone(),
                    self.maps.clone(),
                );
                let stats = validator.calculate_balance_stats();

                ui.heading("Content Overview");
                ui.label(format!("Total Items: {}", self.items.len()));
                ui.label(format!("Total Monsters: {}", self.monsters.len()));
                ui.label(format!("Total Quests: {}", self.quests.len()));
                ui.label(format!("Total Maps: {}", self.maps.len()));

                ui.add_space(10.0);
                ui.heading("Monster Statistics");
                ui.label(format!("Average Level: {:.1}", stats.average_monster_level));
                ui.label(format!("Average HP: {:.1}", stats.average_monster_hp));
                ui.label(format!("Average XP: {:.0}", stats.average_monster_exp));

                ui.add_space(10.0);
                ui.heading("Economy");
                ui.label(format!(
                    "Total Gold Available: {}",
                    stats.total_gold_available
                ));
                ui.label(format!("Total Items: {}", stats.total_items_available));

                ui.add_space(10.0);
                ui.heading("Level Distribution");
                let mut levels: Vec<_> = stats.monster_level_distribution.iter().collect();
                levels.sort_by_key(|(level, _)| *level);
                for (level, count) in levels {
                    ui.label(format!("Level {}: {} monsters", level, count));
                }

                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_balance_stats = false;
                }
            });
        self.show_balance_stats = open;
    }
}

impl eframe::App for CampaignBuilderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("🆕 New Campaign").clicked() {
                        self.new_campaign();
                        ui.close();
                    }
                    if ui.button("📂 Open Campaign...").clicked() {
                        self.open_campaign();
                        ui.close();
                    }
                    if ui.button("💾 Save").clicked() {
                        if self.campaign_path.is_some() {
                            if let Err(e) = self.save_campaign() {
                                self.status_message = format!("Save failed: {}", e);
                            }
                        } else {
                            self.save_campaign_as();
                        }
                        ui.close();
                    }
                    if ui.button("💾 Save As...").clicked() {
                        self.save_campaign_as();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🚪 Exit").clicked() {
                        self.check_unsaved_and_exit();
                        ui.close();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    let can_undo = self.undo_redo_manager.can_undo();
                    let can_redo = self.undo_redo_manager.can_redo();

                    if ui
                        .add_enabled(can_undo, egui::Button::new("⎌ Undo"))
                        .clicked()
                    {
                        match self.undo_redo_manager.undo() {
                            Ok(desc) => {
                                self.sync_state_from_undo_redo();
                                self.status_message = format!("Undid: {}", desc);
                                self.unsaved_changes = true;
                            }
                            Err(e) => self.status_message = e,
                        }
                        ui.close();
                    }
                    if ui
                        .add_enabled(can_redo, egui::Button::new("↷ Redo"))
                        .clicked()
                    {
                        match self.undo_redo_manager.redo() {
                            Ok(desc) => {
                                self.sync_state_from_undo_redo();
                                self.status_message = format!("Redid: {}", desc);
                                self.unsaved_changes = true;
                            }
                            Err(e) => self.status_message = e,
                        }
                        ui.close();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("📋 Template Browser...").clicked() {
                        self.show_template_browser = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("✅ Validate Campaign").clicked() {
                        self.validate_campaign();
                        self.active_tab = EditorTab::Validation;
                        ui.close();
                    }
                    if ui.button("📊 Advanced Validation Report...").clicked() {
                        self.run_advanced_validation();
                        self.show_validation_report = true;
                        ui.close();
                    }
                    if ui.button("⚖️ Balance Statistics...").clicked() {
                        self.show_balance_stats = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🔄 Refresh File Tree").clicked() {
                        if let Some(dir) = self.campaign_dir.clone() {
                            self.update_file_tree(&dir);
                            self.status_message = "File tree refreshed.".to_string();
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🧪 Test Play").clicked() {
                        self.status_message = "Test play would launch the game here...".to_string();
                        ui.close();
                    }
                    if ui.button("📦 Export Campaign...").clicked() {
                        self.status_message =
                            "Export would create .zip archive here...".to_string();
                        ui.close();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("📖 Documentation").clicked() {
                        self.status_message = "Would open documentation in browser...".to_string();
                        ui.close();
                    }
                    if ui.button("ℹ️ About").clicked() {
                        self.show_about_dialog = true;
                        ui.close();
                    }
                });

                // Right-aligned status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.unsaved_changes {
                        ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "● Unsaved changes");
                    }

                    // Show undo/redo status
                    if self.undo_redo_manager.can_undo() {
                        ui.label(format!("↺ {}", self.undo_redo_manager.undo_count()));
                    }
                });
            });
        });

        // Handle keyboard shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            if ctx.input(|i| i.modifiers.shift) {
                // Ctrl+Shift+Z = Redo
                if self.undo_redo_manager.can_redo() {
                    match self.undo_redo_manager.redo() {
                        Ok(desc) => {
                            self.sync_state_from_undo_redo();
                            self.status_message = format!("Redid: {}", desc);
                            self.unsaved_changes = true;
                        }
                        Err(e) => self.status_message = e,
                    }
                }
            } else {
                // Ctrl+Z = Undo
                if self.undo_redo_manager.can_undo() {
                    match self.undo_redo_manager.undo() {
                        Ok(desc) => {
                            self.sync_state_from_undo_redo();
                            self.status_message = format!("Undid: {}", desc);
                            self.unsaved_changes = true;
                        }
                        Err(e) => self.status_message = e,
                    }
                }
            }
        }

        // Ctrl+Y = Redo (alternative)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
            if self.undo_redo_manager.can_redo() {
                match self.undo_redo_manager.redo() {
                    Ok(desc) => {
                        self.sync_state_from_undo_redo();
                        self.status_message = format!("Redid: {}", desc);
                        self.unsaved_changes = true;
                    }
                    Err(e) => self.status_message = e,
                }
            }
        }

        // Left sidebar with tabs
        egui::SidePanel::left("tab_panel")
            .resizable(false)
            .exact_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Editors");
                ui.separator();

                let tabs = [
                    EditorTab::Metadata,
                    EditorTab::Items,
                    EditorTab::Spells,
                    EditorTab::Monsters,
                    EditorTab::Maps,
                    EditorTab::Quests,
                    EditorTab::Classes,
                    EditorTab::Dialogues,
                    EditorTab::Assets,
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
            EditorTab::Items => self.items_editor_state.show(
                ui,
                &mut self.items,
                self.campaign_dir.as_ref(),
                &self.campaign.items_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Spells => self.spells_editor_state.show(
                ui,
                &mut self.spells,
                self.campaign_dir.as_ref(),
                &self.campaign.spells_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Monsters => self.monsters_editor_state.show(
                ui,
                &mut self.monsters,
                self.campaign_dir.as_ref(),
                &self.campaign.monsters_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Maps => self.show_maps_editor(ui),
            EditorTab::Quests => self.show_quests_editor(ui),
            EditorTab::Classes => self.show_classes_editor(ui),
            EditorTab::Dialogues => self.show_dialogues_editor(ui),
            EditorTab::Assets => self.show_assets_editor(ui),
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

        // Phase 15: Template browser dialog
        if self.show_template_browser {
            self.show_template_browser_dialog(ctx);
        }

        // Phase 15: Validation report dialog
        if self.show_validation_report {
            self.show_validation_report_dialog(ctx);
        }

        // Phase 15: Balance statistics dialog
        if self.show_balance_stats {
            self.show_balance_stats_dialog(ctx);
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
                let new_id = self.next_available_map_id();
                let new_map = Map::new(
                    new_id,
                    "New Map".to_string(),
                    "Description".to_string(),
                    20,
                    20,
                );
                self.maps.push(new_map.clone());
                self.maps_selected = Some(self.maps.len() - 1);
                self.map_editor_state = Some(MapEditorState::new(new_map));
                self.maps_editor_mode = EditorMode::Add;
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_maps();
            }
        });

        // Sort maps by ID for consistent display
        let mut sorted_maps: Vec<_> = self.maps.iter().enumerate().collect();
        sorted_maps.sort_by_key(|(_, map)| map.id);

        ui.separator();

        // Map list with previews
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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

        // Draw a detailed view of the map with terrain colors
        for y in 0..map.height.min(20) {
            for x in 0..map.width.min(30) {
                let pos = antares::domain::types::Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    use antares::domain::world::TerrainType;

                    // Base color from terrain type
                    let base_color = match tile.terrain {
                        TerrainType::Ground => egui::Color32::from_rgb(160, 140, 120),
                        TerrainType::Grass => egui::Color32::from_rgb(100, 180, 100),
                        TerrainType::Water => egui::Color32::from_rgb(80, 120, 200),
                        TerrainType::Lava => egui::Color32::from_rgb(220, 60, 30),
                        TerrainType::Swamp => egui::Color32::from_rgb(90, 100, 70),
                        TerrainType::Stone => egui::Color32::from_rgb(120, 120, 130),
                        TerrainType::Dirt => egui::Color32::from_rgb(140, 110, 80),
                        TerrainType::Forest => egui::Color32::from_rgb(60, 120, 60),
                        TerrainType::Mountain => egui::Color32::from_rgb(100, 100, 110),
                    };

                    // Darken if blocked by wall
                    let color = if tile.blocked {
                        egui::Color32::from_rgb(
                            base_color.r() / 2,
                            base_color.g() / 2,
                            base_color.b() / 2,
                        )
                    } else {
                        base_color
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

                    // Draw event marker
                    if map.events.contains_key(&pos) {
                        let center = tile_rect.center();
                        painter.circle_filled(
                            center,
                            actual_tile_size * 0.3,
                            egui::Color32::from_rgb(255, 200, 0),
                        );
                    }

                    // Draw NPC marker
                    if map.npcs.iter().any(|npc| npc.position == pos) {
                        let center = tile_rect.center();
                        painter.circle_filled(
                            center,
                            actual_tile_size * 0.25,
                            egui::Color32::from_rgb(255, 100, 255),
                        );
                    }

                    // Draw grid lines
                    painter.rect_stroke(
                        tile_rect,
                        0.0,
                        egui::Stroke::new(0.5, egui::Color32::from_gray(100)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        }

        // Draw legend
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            ui.colored_label(egui::Color32::from_rgb(255, 200, 0), "● Event");
            ui.colored_label(egui::Color32::from_rgb(255, 100, 255), "● NPC");
            ui.colored_label(egui::Color32::from_rgb(100, 100, 110), "■ Blocked");
        });
    }

    /// Show quests editor (Phase 4A: Full Quest Editor Integration)
    fn show_quests_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("📜 Quests Editor");
        ui.add_space(5.0);

        // Sync quests between app and editor state only in List mode
        // This prevents overwriting temporary quests during creation
        if self.quest_editor_state.mode == quest_editor::QuestEditorMode::List {
            self.quest_editor_state.quests = self.quests.clone();
        }

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ New Quest").clicked() {
                let next_id = self.next_available_quest_id();
                self.quest_editor_state.start_new_quest(next_id.to_string());
                self.unsaved_changes = true;
            }

            ui.separator();

            if ui.button("💾 Save Quests").clicked() {
                if let Err(e) = self.save_quests() {
                    eprintln!("Failed to save quests: {}", e);
                }
            }

            if ui.button("📂 Load Quests").clicked() {
                if let Err(e) = self.load_quests() {
                    eprintln!("Failed to load quests: {}", e);
                }
            }

            ui.separator();

            if ui.button("📥 Import Quest").clicked() {
                self.quests_show_import_dialog = true;
            }

            ui.separator();

            // File I/O buttons
            if ui.button("📂 Load from File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<Quest>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_quests) => {
                            if self.file_load_merge_mode {
                                for quest in loaded_quests {
                                    if let Some(existing) =
                                        self.quests.iter_mut().find(|q| q.id == quest.id)
                                    {
                                        *existing = quest;
                                    } else {
                                        self.quests.push(quest);
                                    }
                                }
                            } else {
                                self.quests = loaded_quests;
                            }
                            self.quest_editor_state.quests = self.quests.clone();
                            self.unsaved_changes = true;
                            self.status_message = format!("Loaded quests from: {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load quests: {}", e);
                        }
                    }
                }
            }

            ui.checkbox(&mut self.file_load_merge_mode, "Merge");
            ui.label(if self.file_load_merge_mode {
                "(adds to existing)"
            } else {
                "(replaces all)"
            });

            if ui.button("💾 Save to File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("quests.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(&self.quests, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                self.status_message =
                                    format!("Saved quests to: {}", path.display());
                            }
                            Err(e) => {
                                self.status_message = format!("Failed to save quests: {}", e);
                            }
                        },
                        Err(e) => {
                            self.status_message = format!("Failed to serialize quests: {}", e);
                        }
                    }
                }
            }

            ui.separator();

            if let Some(selected_idx) = self.quest_editor_state.selected_quest {
                if selected_idx < self.quest_editor_state.quests.len() {
                    if ui.button("📤 Export Quest").clicked() {
                        let quest = &self.quest_editor_state.quests[selected_idx];
                        match ron::ser::to_string_pretty(quest, Default::default()) {
                            Ok(ron_string) => {
                                self.quests_import_buffer = ron_string.clone();
                                ui.ctx().copy_text(ron_string);
                            }
                            Err(e) => eprintln!("Failed to export quest: {}", e),
                        }
                    }
                }
            }

            ui.separator();

            ui.checkbox(&mut self.quests_show_preview, "Show Preview");
        });

        ui.separator();

        // Search filter
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.quest_editor_state.search_filter);
        });

        ui.separator();

        // Main content - split view or form editor
        match self.quest_editor_state.mode {
            quest_editor::QuestEditorMode::List => {
                // Split view: list on left, preview on right
                egui::SidePanel::left("quest_list_panel")
                    .resizable(true)
                    .default_width(300.0)
                    .show_inside(ui, |ui| {
                        self.show_quest_list(ui);
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    if self.quests_show_preview {
                        self.show_quest_preview(ui);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a quest to view details or create a new quest");
                        });
                    }
                });
            }
            quest_editor::QuestEditorMode::Creating | quest_editor::QuestEditorMode::Editing => {
                // Full-screen quest form editor
                self.show_quest_form(ui);
            }
        }

        // Import dialog
        if self.quests_show_import_dialog {
            egui::Window::new("Import Quest from RON")
                .collapsible(false)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Paste RON-formatted quest data:");
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt("quests_list_scroll")
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.quests_import_buffer)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Import").clicked() {
                            match ron::from_str::<Quest>(&self.quests_import_buffer) {
                                Ok(mut quest) => {
                                    // Assign new ID to avoid conflicts
                                    quest.id = self.next_available_quest_id();
                                    self.quests.push(quest);
                                    self.unsaved_changes = true;
                                    self.quests_import_buffer.clear();
                                    self.quests_show_import_dialog = false;
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse quest RON: {}", e);
                                }
                            }
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.quests_show_import_dialog = false;
                            self.quests_import_buffer.clear();
                        }
                    });
                });
        }

        // Sync back any changes
        self.quests = self.quest_editor_state.quests.clone();
    }

    /// Show quest list view
    fn show_quest_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Quest List");
        ui.separator();

        // Sync quests to editor state for filtering
        self.quest_editor_state.quests = self.quests.clone();
        let filtered_quests_cloned: Vec<(usize, Quest)> = self
            .quest_editor_state
            .filtered_quests()
            .into_iter()
            .map(|(idx, q)| (idx, q.clone()))
            .collect();

        ui.label(format!("Total: {} quest(s)", filtered_quests_cloned.len()));
        ui.separator();

        // Pre-calculate next ID to avoid borrowing self in closure
        let next_id = self.next_available_quest_id();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for quest in filtered_quests_cloned.iter() {
                    let original_idx = quest.0;
                    let is_selected = self.quest_editor_state.selected_quest == Some(original_idx);

                    let response = ui.selectable_label(
                        is_selected,
                        format!(
                            "{} - {} {}",
                            quest.1.id,
                            quest.1.name,
                            if quest.1.is_main_quest { "⭐" } else { "" }
                        ),
                    );

                    if response.clicked() {
                        self.quest_editor_state.selected_quest = Some(original_idx);
                    }

                    if response.double_clicked() {
                        self.quest_editor_state.start_edit_quest(original_idx);
                    }

                    // Context menu
                    response.context_menu(|ui| {
                        if ui.button("✏️ Edit").clicked() {
                            self.quest_editor_state.start_edit_quest(original_idx);
                            ui.close();
                        }

                        if ui.button("🗑️ Delete").clicked() {
                            self.quest_editor_state.delete_quest(original_idx);
                            self.quests = self.quest_editor_state.quests.clone();
                            self.unsaved_changes = true;
                            ui.close();
                        }

                        if ui.button("📋 Duplicate").clicked() {
                            if original_idx < self.quests.len() {
                                let mut new_quest = self.quests[original_idx].clone();
                                new_quest.id = next_id;
                                new_quest.name = format!("{} (Copy)", new_quest.name);
                                self.quests.push(new_quest);
                                self.unsaved_changes = true;
                            }
                            ui.close();
                        }
                    });
                }

                if filtered_quests_cloned.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label("No quests found");
                        ui.label("Click 'New Quest' to create one");
                    });
                }
            });
    }

    /// Show quest form editor
    fn show_quest_form(&mut self, ui: &mut egui::Ui) {
        let is_creating = matches!(
            self.quest_editor_state.mode,
            quest_editor::QuestEditorMode::Creating
        );

        ui.heading(if is_creating {
            "Create New Quest"
        } else {
            "Edit Quest"
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label("Basic Information");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.text_edit_singleline(&mut self.quest_editor_state.quest_buffer.id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.quest_editor_state.quest_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(
                            &mut self.quest_editor_state.quest_buffer.description,
                        )
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                    );

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.quest_editor_state.quest_buffer.repeatable,
                            "Repeatable",
                        );
                        ui.checkbox(
                            &mut self.quest_editor_state.quest_buffer.is_main_quest,
                            "Main Quest",
                        );
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Level Requirements");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Min Level:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.min_level,
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max Level:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.max_level,
                        );
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Quest Giver");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("NPC ID:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.quest_giver_npc,
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Map ID:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.quest_giver_map,
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Position X:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.quest_giver_x,
                        );
                        ui.label("Y:");
                        ui.text_edit_singleline(
                            &mut self.quest_editor_state.quest_buffer.quest_giver_y,
                        );
                    });
                });

                ui.add_space(10.0);

                // Stages editor
                self.show_quest_stages_editor(ui);

                ui.add_space(10.0);

                // Rewards editor
                self.show_quest_rewards_editor(ui);

                ui.add_space(10.0);

                // Validation display
                self.show_quest_validation(ui);

                ui.add_space(10.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("✅ Save Quest").clicked() {
                        if self.quest_editor_state.save_quest().is_ok() {
                            self.quests = self.quest_editor_state.quests.clone();
                            self.unsaved_changes = true;
                        }
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.quest_editor_state.cancel_edit();
                    }
                });
            });
    }

    /// Show quest stages editor
    fn show_quest_stages_editor(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Quest Stages");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Add Stage").clicked() {
                        let _ = self.quest_editor_state.add_stage();
                        self.unsaved_changes = true;
                    }
                });
            });

            ui.separator();

            if let Some(selected_idx) = self.quest_editor_state.selected_quest {
                if selected_idx < self.quest_editor_state.quests.len() {
                    // Clone stages to avoid borrowing issues
                    let stages = self.quest_editor_state.quests[selected_idx].stages.clone();
                    let mut stage_to_delete: Option<usize> = None;
                    let mut stage_to_edit: Option<usize> = None;

                    for (stage_idx, stage) in stages.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let header = ui.collapsing(
                                format!("Stage {}: {}", stage.stage_number, stage.name),
                                |ui| {
                                    ui.label(&stage.description);
                                    ui.label(format!(
                                        "Require all objectives: {}",
                                        stage.require_all_objectives
                                    ));
                                    ui.separator();

                                    // Show objectives with edit/delete controls
                                    self.show_quest_objectives_editor(
                                        ui,
                                        selected_idx,
                                        stage_idx,
                                        &stage.objectives,
                                    );
                                },
                            );

                            // Track which stage is expanded for objective addition
                            // if header.header_response.clicked() || header.body_returned.is_some() {
                            //     self.quest_editor_state.selected_stage = Some(stage_idx);
                            // }

                            // Stage action buttons
                            if ui.small_button("✏️").on_hover_text("Edit Stage").clicked() {
                                stage_to_edit = Some(stage_idx);
                            }
                            if ui
                                .small_button("🗑️")
                                .on_hover_text("Delete Stage")
                                .clicked()
                            {
                                stage_to_delete = Some(stage_idx);
                            }
                        });
                    }

                    // Handle stage deletion
                    if let Some(stage_idx) = stage_to_delete {
                        if self
                            .quest_editor_state
                            .delete_stage(selected_idx, stage_idx)
                            .is_ok()
                        {
                            self.unsaved_changes = true;
                        }
                    }

                    // Handle stage editing
                    if let Some(stage_idx) = stage_to_edit {
                        if self
                            .quest_editor_state
                            .edit_stage(selected_idx, stage_idx)
                            .is_ok()
                        {
                            self.quest_editor_state.mode = quest_editor::QuestEditorMode::Editing;
                        }
                    }

                    if stages.is_empty() {
                        ui.label("No stages defined yet");
                    }
                } else {
                    ui.label("No quest selected");
                }
            } else {
                ui.label("No quest selected");
            }
        });

        // Stage editor modal
        if let Some(stage_idx) = self.quest_editor_state.selected_stage {
            egui::Window::new("Edit Stage")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Stage Number:");
                        ui.text_edit_singleline(&mut self.quest_editor_state.stage_buffer.number);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.quest_editor_state.stage_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(
                            &mut self.quest_editor_state.stage_buffer.description,
                        )
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                    );

                    ui.checkbox(
                        &mut self.quest_editor_state.stage_buffer.require_all,
                        "Require all objectives to complete",
                    );

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if let Some(selected_idx) = self.quest_editor_state.selected_quest {
                                if self
                                    .quest_editor_state
                                    .save_stage(selected_idx, stage_idx)
                                    .is_ok()
                                {
                                    self.quests = self.quest_editor_state.quests.clone();
                                    self.unsaved_changes = true;
                                }
                            }
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.quest_editor_state.selected_stage = None;
                        }
                    });
                });
        }
    }

    /// Show quest objectives editor
    fn show_quest_objectives_editor(
        &mut self,
        ui: &mut egui::Ui,
        quest_idx: usize,
        stage_idx: usize,
        objectives: &[antares::domain::quest::QuestObjective],
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Objectives ({})", objectives.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("➕")
                        .on_hover_text("Add Objective")
                        .clicked()
                    {
                        if let Ok(new_idx) =
                            self.quest_editor_state.add_default_objective(stage_idx)
                        {
                            self.unsaved_changes = true;
                            // Immediately start editing the new objective
                            let _ = self
                                .quest_editor_state
                                .edit_objective(quest_idx, stage_idx, new_idx);
                        }
                    }
                });
            });

            ui.separator();

            let mut objective_to_delete: Option<usize> = None;
            let mut objective_to_edit: Option<usize> = None;

            for (obj_idx, objective) in objectives.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", obj_idx + 1));
                    ui.label(objective.description());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("🗑️")
                            .on_hover_text("Delete Objective")
                            .clicked()
                        {
                            objective_to_delete = Some(obj_idx);
                        }
                        if ui
                            .small_button("✏️")
                            .on_hover_text("Edit Objective")
                            .clicked()
                        {
                            objective_to_edit = Some(obj_idx);
                        }
                    });
                });
            }

            // Handle objective deletion
            if let Some(obj_idx) = objective_to_delete {
                if self
                    .quest_editor_state
                    .delete_objective(quest_idx, stage_idx, obj_idx)
                    .is_ok()
                {
                    self.unsaved_changes = true;
                }
            }

            // Handle objective editing
            if let Some(obj_idx) = objective_to_edit {
                if self
                    .quest_editor_state
                    .edit_objective(quest_idx, stage_idx, obj_idx)
                    .is_ok()
                {
                    // Objective editing modal will be shown below
                }
            }

            if objectives.is_empty() {
                ui.label("No objectives defined");
            }
        });

        // Objective editor modal
        if let Some(obj_idx) = self.quest_editor_state.selected_objective {
            egui::Window::new("Edit Objective")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Objective Type:");
                        egui::ComboBox::new("objective_type_selector", "")
                            .selected_text(format!(
                                "{:?}",
                                self.quest_editor_state.objective_buffer.objective_type
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::KillMonsters,
                                    "Kill Monsters",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::CollectItems,
                                    "Collect Items",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::ReachLocation,
                                    "Reach Location",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::TalkToNpc,
                                    "Talk To NPC",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::DeliverItem,
                                    "Deliver Item",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::EscortNpc,
                                    "Escort NPC",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.objective_buffer.objective_type,
                                    quest_editor::ObjectiveType::CustomFlag,
                                    "Custom Flag",
                                );
                            });
                    });

                    ui.separator();

                    // Type-specific fields
                    match self.quest_editor_state.objective_buffer.objective_type {
                        quest_editor::ObjectiveType::KillMonsters => {
                            ui.horizontal(|ui| {
                                ui.label("Monster:");
                                egui::ComboBox::from_id_salt("monster_selector")
                                    .selected_text(
                                        self.monsters
                                            .iter()
                                            .find(|m| {
                                                m.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .monster_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .monster_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for monster in &self.monsters {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .monster_id,
                                                monster.id.to_string(),
                                                format!("{} - {}", monster.id, monster.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.quantity,
                                );
                            });
                        }
                        quest_editor::ObjectiveType::CollectItems => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("item_selector")
                                    .selected_text(
                                        self.items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .item_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in &self.items {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.quantity,
                                );
                            });
                        }
                        quest_editor::ObjectiveType::ReachLocation => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector")
                                    .selected_text(
                                        self.maps
                                            .iter()
                                            .find(|m| {
                                                m.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .map_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in &self.maps {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.location_x,
                                );
                                ui.label("Y:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.location_y,
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.location_radius,
                                );
                            });
                        }
                        quest_editor::ObjectiveType::TalkToNpc => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector_npc")
                                    .selected_text(
                                        self.maps
                                            .iter()
                                            .find(|m| {
                                                m.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .map_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in &self.maps {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });

                            // Filter NPCs based on selected map
                            let selected_map_id = self
                                .quest_editor_state
                                .objective_buffer
                                .map_id
                                .parse::<u16>()
                                .unwrap_or(0);
                            let map_npcs: Vec<_> = self
                                .maps
                                .iter()
                                .find(|m| m.id == selected_map_id)
                                .map(|m| m.npcs.clone())
                                .unwrap_or_default();

                            ui.horizontal(|ui| {
                                ui.label("NPC:");
                                egui::ComboBox::from_id_salt("npc_selector")
                                    .selected_text(
                                        map_npcs
                                            .iter()
                                            .find(|n| {
                                                n.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .npc_id
                                            })
                                            .map(|n| format!("{} - {}", n.id, n.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .npc_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for npc in &map_npcs {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .npc_id,
                                                npc.id.to_string(),
                                                format!("{} - {}", npc.id, npc.name),
                                            );
                                        }
                                    });
                            });
                        }
                        quest_editor::ObjectiveType::DeliverItem => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("item_selector_deliver")
                                    .selected_text(
                                        self.items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .item_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in &self.items {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });

                            ui.horizontal(|ui| {
                                ui.label("NPC ID:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.npc_id,
                                );
                                // Note: Could add map selector + NPC selector here too, but DeliverItem doesn't strictly require map_id in the struct,
                                // though it's helpful. The struct has npc_id but not map_id for DeliverItem (wait, let me check domain/quest.rs).
                                // QuestObjective::DeliverItem { item_id, npc_id, quantity } -> No map_id.
                                // So we can't easily filter by map unless we ask for map_id just for filtering purposes.
                                // For now, simple text edit for NPC ID is safer, or a global NPC search if we had a global NPC index.
                            });

                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.quantity,
                                );
                            });
                        }
                        quest_editor::ObjectiveType::EscortNpc => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector_escort")
                                    .selected_text(
                                        self.maps
                                            .iter()
                                            .find(|m| {
                                                m.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .map_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in &self.maps {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });

                            // Filter NPCs based on selected map
                            let selected_map_id = self
                                .quest_editor_state
                                .objective_buffer
                                .map_id
                                .parse::<u16>()
                                .unwrap_or(0);
                            let map_npcs: Vec<_> = self
                                .maps
                                .iter()
                                .find(|m| m.id == selected_map_id)
                                .map(|m| m.npcs.clone())
                                .unwrap_or_default();

                            ui.horizontal(|ui| {
                                ui.label("NPC:");
                                egui::ComboBox::from_id_salt("npc_selector_escort")
                                    .selected_text(
                                        map_npcs
                                            .iter()
                                            .find(|n| {
                                                n.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .objective_buffer
                                                        .npc_id
                                            })
                                            .map(|n| format!("{} - {}", n.id, n.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .objective_buffer
                                                    .npc_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for npc in &map_npcs {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .objective_buffer
                                                    .npc_id,
                                                npc.id.to_string(),
                                                format!("{} - {}", npc.id, npc.name),
                                            );
                                        }
                                    });
                            });

                            ui.horizontal(|ui| {
                                ui.label("Destination X:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.location_x,
                                );
                                ui.label("Y:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.location_y,
                                );
                            });
                        }
                        quest_editor::ObjectiveType::CustomFlag => {
                            ui.horizontal(|ui| {
                                ui.label("Flag Name:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.objective_buffer.flag_name,
                                );
                            });
                            ui.checkbox(
                                &mut self.quest_editor_state.objective_buffer.flag_value,
                                "Required Value",
                            );
                        }
                    }

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if self
                                .quest_editor_state
                                .save_objective(quest_idx, stage_idx, obj_idx)
                                .is_ok()
                            {
                                self.quests = self.quest_editor_state.quests.clone();
                                self.unsaved_changes = true;
                            }
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.quest_editor_state.selected_objective = None;
                        }
                    });
                });
        }
    }

    /// Show quest rewards editor
    fn show_quest_rewards_editor(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Rewards");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Add Reward").clicked() {
                        if let Ok(new_idx) = self.quest_editor_state.add_default_reward() {
                            self.unsaved_changes = true;
                            // Immediately start editing the new reward
                            let _ = self.quest_editor_state.edit_reward(
                                self.quest_editor_state.selected_quest.unwrap(),
                                new_idx,
                            );
                        }
                    }
                });
            });

            ui.separator();

            if let Some(selected_idx) = self.quest_editor_state.selected_quest {
                if selected_idx < self.quest_editor_state.quests.len() {
                    let rewards = self.quest_editor_state.quests[selected_idx].rewards.clone();
                    let mut reward_to_delete: Option<usize> = None;
                    let mut reward_to_edit: Option<usize> = None;

                    for (reward_idx, reward) in rewards.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let desc = match reward {
                                antares::domain::quest::QuestReward::Experience(xp) => {
                                    format!("{} XP", xp)
                                }
                                antares::domain::quest::QuestReward::Gold(gold) => {
                                    format!("{} Gold", gold)
                                }
                                antares::domain::quest::QuestReward::Items(items) => {
                                    let item_strs: Vec<String> = items
                                        .iter()
                                        .map(|(id, qty)| {
                                            let name = self
                                                .items
                                                .iter()
                                                .find(|i| i.id == *id)
                                                .map(|i| i.name.clone())
                                                .unwrap_or_else(|| "Unknown Item".to_string());
                                            format!("{}x {} ({})", qty, name, id)
                                        })
                                        .collect();
                                    item_strs.join(", ")
                                }
                                antares::domain::quest::QuestReward::UnlockQuest(qid) => {
                                    let name = self
                                        .quests
                                        .iter()
                                        .find(|q| q.id == *qid)
                                        .map(|q| q.name.clone())
                                        .unwrap_or_else(|| "Unknown Quest".to_string());
                                    format!("Unlock Quest: {} ({})", name, qid)
                                }
                                antares::domain::quest::QuestReward::SetFlag {
                                    flag_name,
                                    value,
                                } => {
                                    format!("Set Flag '{}' to {}", flag_name, value)
                                }
                                antares::domain::quest::QuestReward::Reputation {
                                    faction,
                                    change,
                                } => {
                                    format!("Reputation: {} ({:+})", faction, change)
                                }
                            };

                            ui.label(format!("{}.", reward_idx + 1));
                            ui.label(desc);

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button("🗑️")
                                        .on_hover_text("Delete Reward")
                                        .clicked()
                                    {
                                        reward_to_delete = Some(reward_idx);
                                    }
                                    if ui.small_button("✏️").on_hover_text("Edit Reward").clicked()
                                    {
                                        reward_to_edit = Some(reward_idx);
                                    }
                                },
                            );
                        });
                    }

                    if let Some(reward_idx) = reward_to_delete {
                        if self
                            .quest_editor_state
                            .delete_reward(selected_idx, reward_idx)
                            .is_ok()
                        {
                            self.unsaved_changes = true;
                        }
                    }

                    if let Some(reward_idx) = reward_to_edit {
                        if self
                            .quest_editor_state
                            .edit_reward(selected_idx, reward_idx)
                            .is_ok()
                        {
                            // Modal will show
                        }
                    }

                    if rewards.is_empty() {
                        ui.label("No rewards defined");
                    }
                }
            } else {
                ui.label("No quest selected");
            }
        });

        // Reward editor modal
        if let Some(reward_idx) = self.quest_editor_state.selected_reward {
            egui::Window::new("Edit Reward")
                .collapsible(false)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        egui::ComboBox::from_id_salt("reward_type_selector")
                            .selected_text(
                                self.quest_editor_state.reward_buffer.reward_type.as_str(),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::Experience,
                                    "Experience",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::Gold,
                                    "Gold",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::Items,
                                    "Items",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::UnlockQuest,
                                    "Unlock Quest",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::SetFlag,
                                    "Set Flag",
                                );
                                ui.selectable_value(
                                    &mut self.quest_editor_state.reward_buffer.reward_type,
                                    quest_editor::RewardType::Reputation,
                                    "Reputation",
                                );
                            });
                    });

                    ui.separator();

                    match self.quest_editor_state.reward_buffer.reward_type {
                        quest_editor::RewardType::Experience => {
                            ui.horizontal(|ui| {
                                ui.label("Amount:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.experience,
                                );
                            });
                        }
                        quest_editor::RewardType::Gold => {
                            ui.horizontal(|ui| {
                                ui.label("Amount:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.gold,
                                );
                            });
                        }
                        quest_editor::RewardType::Items => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("reward_item_selector")
                                    .selected_text(
                                        self.items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string()
                                                    == self.quest_editor_state.reward_buffer.item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .reward_buffer
                                                    .item_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in &self.items {
                                            ui.selectable_value(
                                                &mut self.quest_editor_state.reward_buffer.item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.item_quantity,
                                );
                            });
                        }
                        quest_editor::RewardType::UnlockQuest => {
                            ui.horizontal(|ui| {
                                ui.label("Quest:");
                                egui::ComboBox::from_id_salt("reward_quest_selector")
                                    .selected_text(
                                        self.quests
                                            .iter()
                                            .find(|q| {
                                                q.id.to_string()
                                                    == self
                                                        .quest_editor_state
                                                        .reward_buffer
                                                        .unlock_quest_id
                                            })
                                            .map(|q| format!("{} - {}", q.id, q.name))
                                            .unwrap_or_else(|| {
                                                self.quest_editor_state
                                                    .reward_buffer
                                                    .unlock_quest_id
                                                    .clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for quest in &self.quests {
                                            ui.selectable_value(
                                                &mut self
                                                    .quest_editor_state
                                                    .reward_buffer
                                                    .unlock_quest_id,
                                                quest.id.to_string(),
                                                format!("{} - {}", quest.id, quest.name),
                                            );
                                        }
                                    });
                            });
                        }
                        quest_editor::RewardType::SetFlag => {
                            ui.horizontal(|ui| {
                                ui.label("Flag Name:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.flag_name,
                                );
                            });
                            ui.checkbox(
                                &mut self.quest_editor_state.reward_buffer.flag_value,
                                "Value",
                            );
                        }
                        quest_editor::RewardType::Reputation => {
                            ui.horizontal(|ui| {
                                ui.label("Faction:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.faction_name,
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Change:");
                                ui.text_edit_singleline(
                                    &mut self.quest_editor_state.reward_buffer.reputation_change,
                                );
                            });
                        }
                    }

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if let Some(selected_idx) = self.quest_editor_state.selected_quest {
                                if self
                                    .quest_editor_state
                                    .save_reward(selected_idx, reward_idx)
                                    .is_ok()
                                {
                                    self.unsaved_changes = true;
                                }
                            }
                        }
                        if ui.button("❌ Cancel").clicked() {
                            self.quest_editor_state.selected_reward = None;
                        }
                    });
                });
        }
    }

    /// Show quest validation display
    fn show_quest_validation(&mut self, ui: &mut egui::Ui) {
        self.quest_editor_state.validate_current_quest();
        let errors = &self.quest_editor_state.validation_errors;

        if !errors.is_empty() {
            ui.group(|ui| {
                ui.label("⚠️ Validation Errors:");
                ui.separator();

                for error in errors {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 100, 100),
                        format!("• {}", error),
                    );
                }
            });
        } else if self.quest_editor_state.selected_quest.is_some() {
            ui.group(|ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✅ Quest is valid");
            });
        }
    }

    /// Show quest preview panel
    fn show_quest_preview(&self, ui: &mut egui::Ui) {
        if let Some(selected_idx) = self.quest_editor_state.selected_quest {
            if selected_idx < self.quests.len() {
                let quest = &self.quests[selected_idx];

                ui.heading(&quest.name);
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.group(|ui| {
                        ui.label("Description:");
                        ui.label(&quest.description);
                    });

                    ui.add_space(5.0);

                    ui.group(|ui| {
                        ui.label("Quest Info:");
                        ui.separator();
                        ui.label(format!("ID: {}", quest.id));
                        ui.label(format!(
                            "Type: {}",
                            if quest.is_main_quest {
                                "Main Quest ⭐"
                            } else {
                                "Side Quest"
                            }
                        ));
                        ui.label(format!("Repeatable: {}", quest.repeatable));

                        if let Some(min) = quest.min_level {
                            ui.label(format!("Min Level: {}", min));
                        }
                        if let Some(max) = quest.max_level {
                            ui.label(format!("Max Level: {}", max));
                        }
                    });

                    ui.add_space(5.0);

                    ui.group(|ui| {
                        ui.label(format!("Stages ({}):", quest.stages.len()));
                        ui.separator();
                        for stage in &quest.stages {
                            ui.collapsing(
                                format!("Stage {}: {}", stage.stage_number, stage.name),
                                |ui| {
                                    ui.label(&stage.description);
                                    ui.separator();
                                    ui.label(format!("Objectives ({})", stage.objectives.len()));
                                    for objective in &stage.objectives {
                                        ui.label(format!("  • {}", objective.description()));
                                    }
                                },
                            );
                        }
                    });

                    ui.add_space(5.0);

                    ui.group(|ui| {
                        ui.label(format!("Rewards ({}):", quest.rewards.len()));
                        ui.separator();
                        for reward in &quest.rewards {
                            match reward {
                                antares::domain::quest::QuestReward::Experience(xp) => {
                                    ui.label(format!("  • {} XP", xp))
                                }
                                antares::domain::quest::QuestReward::Gold(gold) => {
                                    ui.label(format!("  • {} Gold", gold))
                                }
                                antares::domain::quest::QuestReward::Items(items) => {
                                    for (item_id, qty) in items {
                                        ui.label(format!("  • {} x Item {}", qty, item_id));
                                    }
                                    ui.label("")
                                }
                                antares::domain::quest::QuestReward::UnlockQuest(quest_id) => {
                                    ui.label(format!("  • Unlock Quest {}", quest_id))
                                }
                                antares::domain::quest::QuestReward::SetFlag {
                                    flag_name,
                                    value,
                                } => ui.label(format!("  • Set Flag '{}' = {}", flag_name, value)),
                                antares::domain::quest::QuestReward::Reputation {
                                    faction,
                                    change,
                                } => ui.label(format!("  • {} Reputation: {:+}", faction, change)),
                            };
                        }
                    });
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Quest not found");
                });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No quest selected");
            });
        }
    }

    /// Load quests from file
    fn load_quests(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            if quests_path.exists() {
                match fs::read_to_string(&quests_path) {
                    Ok(contents) => {
                        match ron::from_str::<Vec<antares::domain::quest::Quest>>(&contents) {
                            Ok(quests) => {
                                self.quests = quests;
                                self.quest_editor_state.load_quests(self.quests.clone());
                                self.status_message =
                                    format!("Loaded {} quests", self.quests.len());
                            }
                            Err(e) => {
                                eprintln!("Failed to parse quests from {:?}: {}", quests_path, e);
                                return Err(CampaignError::Deserialization(e));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read quests file {:?}: {}", quests_path, e);
                        return Err(CampaignError::Io(e));
                    }
                }
            } else {
                eprintln!("Quests file does not exist: {:?}", quests_path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load quests");
        }
        Ok(())
    }

    /// Save quests to file
    fn save_quests(&self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            // Create quests directory if it doesn't exist
            if let Some(parent) = quests_path.parent() {
                fs::create_dir_all(parent).map_err(CampaignError::Io)?;
            }

            let contents = ron::ser::to_string_pretty(&self.quests, Default::default())?;
            fs::write(&quests_path, contents)?;
        }
        Ok(())
    }

    /// Get next available quest ID
    fn next_available_quest_id(&self) -> QuestId {
        self.quests.iter().map(|q| q.id).max().unwrap_or(0) + 1
    }

    /// Show classes editor
    fn show_classes_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("🛡️ Classes Editor");
        ui.add_space(5.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ New Class").clicked() {
                self.classes_editor_state.start_new_class();
                self.classes_editor_state.buffer.id = self.next_available_class_id();
                self.unsaved_changes = true;
            }

            ui.separator();

            // File I/O buttons
            if ui.button("📂 Load from File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<antares::domain::classes::ClassDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_classes) => {
                            if self.file_load_merge_mode {
                                for class in loaded_classes {
                                    if let Some(existing) = self
                                        .classes_editor_state
                                        .classes
                                        .iter_mut()
                                        .find(|c| c.id == class.id)
                                    {
                                        *existing = class;
                                    } else {
                                        self.classes_editor_state.classes.push(class);
                                    }
                                }
                            } else {
                                self.classes_editor_state.classes = loaded_classes;
                            }
                            self.unsaved_changes = true;
                            self.status_message =
                                format!("Loaded classes from: {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load classes: {}", e);
                        }
                    }
                }
            }

            ui.checkbox(&mut self.file_load_merge_mode, "Merge");
            ui.label(if self.file_load_merge_mode {
                "(adds to existing)"
            } else {
                "(replaces all)"
            });

            if ui.button("💾 Save to File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("classes.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(
                        &self.classes_editor_state.classes,
                        Default::default(),
                    ) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                self.status_message =
                                    format!("Saved classes to: {}", path.display());
                            }
                            Err(e) => {
                                self.status_message = format!("Failed to save classes: {}", e);
                            }
                        },
                        Err(e) => {
                            self.status_message = format!("Failed to serialize classes: {}", e);
                        }
                    }
                }
            }

            ui.separator();

            if ui.button("💾 Save Classes").clicked() {
                if let Err(e) = self.save_classes() {
                    eprintln!("Failed to save classes: {}", e);
                }
            }

            if ui.button("📂 Load Classes").clicked() {
                self.load_classes();
            }
        });

        ui.separator();

        // Search
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.classes_editor_state.search_filter);
        });

        ui.separator();

        // Main content
        match self.classes_editor_state.mode {
            classes_editor::ClassesEditorMode::List => {
                egui::SidePanel::left("classes_list_panel")
                    .resizable(true)
                    .default_width(300.0)
                    .show_inside(ui, |ui| {
                        self.show_classes_list(ui);
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a class to edit or create a new one");
                    });
                });
            }
            classes_editor::ClassesEditorMode::Creating
            | classes_editor::ClassesEditorMode::Editing => {
                self.show_class_form(ui);
            }
        }
    }

    fn show_classes_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Classes List");
        ui.separator();

        // Clone to avoid borrow checker issues
        let filtered: Vec<(usize, antares::domain::classes::ClassDefinition)> = self
            .classes_editor_state
            .filtered_classes()
            .into_iter()
            .map(|(i, c)| (i, c.clone()))
            .collect();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, class) in filtered {
                    let is_selected = self.classes_editor_state.selected_class == Some(idx);
                    if ui.selectable_label(is_selected, &class.name).clicked() {
                        self.classes_editor_state.start_edit_class(idx);
                    }

                    if is_selected {
                        ui.horizontal(|ui| {
                            if ui.button("🗑️ Delete").clicked() {
                                self.classes_editor_state.delete_class(idx);
                                self.unsaved_changes = true;
                            }
                        });
                    }
                }
            });
    }

    fn show_class_form(&mut self, ui: &mut egui::Ui) {
        let is_creating =
            self.classes_editor_state.mode == classes_editor::ClassesEditorMode::Creating;
        ui.heading(if is_creating {
            "Create New Class"
        } else {
            "Edit Class"
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            ui.group(|ui| {
                ui.label("Basic Info");
                ui.horizontal(|ui| {
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.id);
                });
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut self.classes_editor_state.buffer.description);
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Hit Points");
                ui.horizontal(|ui| {
                    ui.label("Count:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.hp_die_count);
                    ui.label("Sides:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.hp_die_sides);
                    ui.label("Bonus:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.hp_die_modifier);
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Magic");
                ui.checkbox(
                    &mut self.classes_editor_state.buffer.is_pure_caster,
                    "Pure Caster",
                );

                ui.horizontal(|ui| {
                    ui.label("Spell School:");
                    egui::ComboBox::from_id_salt("spell_school")
                        .selected_text(format!(
                            "{:?}",
                            self.classes_editor_state.buffer.spell_school
                        ))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_school,
                                None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_school,
                                Some(antares::domain::classes::SpellSchool::Cleric),
                                "Cleric",
                            );
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_school,
                                Some(antares::domain::classes::SpellSchool::Sorcerer),
                                "Sorcerer",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Spell Stat:");
                    egui::ComboBox::from_id_salt("spell_stat")
                        .selected_text(format!("{:?}", self.classes_editor_state.buffer.spell_stat))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_stat,
                                None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_stat,
                                Some(antares::domain::classes::SpellStat::Intellect),
                                "Intellect",
                            );
                            ui.selectable_value(
                                &mut self.classes_editor_state.buffer.spell_stat,
                                Some(antares::domain::classes::SpellStat::Personality),
                                "Personality",
                            );
                        });
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Item Restrictions");
                ui.horizontal(|ui| {
                    ui.label("Disablement Bit:");
                    ui.text_edit_singleline(&mut self.classes_editor_state.buffer.disablement_bit);
                    ui.label("ℹ️").on_hover_text(
                        "This bit flag (0-7) determines item restrictions.\n\
                         Items can be flagged to disable usage by specific classes.\n\
                         Bit 0 = Knight, Bit 1 = Paladin, Bit 2 = Archer, etc.\n\
                         Example: A class with bit 2 cannot use items with disablement flag bit 2 set."
                    );
                });
                if let Ok(bit) = self.classes_editor_state.buffer.disablement_bit.parse::<u8>() {
                    if bit <= 7 {
                        ui.label(format!("This class uses restriction bit position: {}", bit));
                    }
                }
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Starting Equipment");

                // Starting Weapon
                ui.horizontal(|ui| {
                    ui.label("Starting Weapon:");
                    let current_weapon = if self.classes_editor_state.buffer.starting_weapon_id.is_empty() {
                        "None".to_string()
                    } else {
                        self.items.iter()
                            .find(|item| item.id.to_string() == self.classes_editor_state.buffer.starting_weapon_id)
                            .map(|item| format!("{} (ID: {})", item.name, item.id))
                            .unwrap_or_else(|| format!("ID: {}", self.classes_editor_state.buffer.starting_weapon_id))
                    };

                    egui::ComboBox::from_id_salt("starting_weapon")
                        .selected_text(current_weapon)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.classes_editor_state.buffer.starting_weapon_id.is_empty(), "None").clicked() {
                                self.classes_editor_state.buffer.starting_weapon_id = String::new();
                            }
                            for item in &self.items {
                                if item.is_weapon() {
                                    let is_selected = item.id.to_string() == self.classes_editor_state.buffer.starting_weapon_id;
                                    if ui.selectable_label(is_selected, format!("{} (ID: {})", item.name, item.id)).clicked() {
                                        self.classes_editor_state.buffer.starting_weapon_id = item.id.to_string();
                                    }
                                }
                            }
                        });
                });

                // Starting Armor
                ui.horizontal(|ui| {
                    ui.label("Starting Armor:");
                    let current_armor = if self.classes_editor_state.buffer.starting_armor_id.is_empty() {
                        "None".to_string()
                    } else {
                        self.items.iter()
                            .find(|item| item.id.to_string() == self.classes_editor_state.buffer.starting_armor_id)
                            .map(|item| format!("{} (ID: {})", item.name, item.id))
                            .unwrap_or_else(|| format!("ID: {}", self.classes_editor_state.buffer.starting_armor_id))
                    };

                    egui::ComboBox::from_id_salt("starting_armor")
                        .selected_text(current_armor)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.classes_editor_state.buffer.starting_armor_id.is_empty(), "None").clicked() {
                                self.classes_editor_state.buffer.starting_armor_id = String::new();
                            }
                            for item in &self.items {
                                if item.is_armor() {
                                    let is_selected = item.id.to_string() == self.classes_editor_state.buffer.starting_armor_id;
                                    if ui.selectable_label(is_selected, format!("{} (ID: {})", item.name, item.id)).clicked() {
                                        self.classes_editor_state.buffer.starting_armor_id = item.id.to_string();
                                    }
                                }
                            }
                        });
                });

                // Starting Items List
                ui.label("Starting Items:");
                let mut items_to_remove = Vec::new();
                for (idx, item_id) in self.classes_editor_state.buffer.starting_items.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let item_name = self.items.iter()
                            .find(|item| item.id.to_string() == *item_id)
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("Unknown (ID: {})", item_id));
                        ui.label(item_name);
                        if ui.small_button("🗑️").clicked() {
                            items_to_remove.push(idx);
                        }
                    });
                }
                for idx in items_to_remove.into_iter().rev() {
                    self.classes_editor_state.buffer.starting_items.remove(idx);
                }

                if ui.button("➕ Add Starting Item").clicked() {
                    self.classes_editor_state.buffer.starting_items.push(String::new());
                }
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Special Abilities (comma separated):");
                ui.text_edit_multiline(&mut self.classes_editor_state.buffer.special_abilities);
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui.button("✅ Save").clicked() {
                    if let Err(e) = self.classes_editor_state.save_class() {
                        eprintln!("Error saving class: {}", e);
                    } else {
                        self.unsaved_changes = true;
                    }
                }
                if ui.button("❌ Cancel").clicked() {
                    self.classes_editor_state.cancel_edit();
                }
            });
        });
    }

    fn load_classes(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.classes_file);
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match ron::from_str::<Vec<antares::domain::classes::ClassDefinition>>(
                            &content,
                        ) {
                            Ok(classes) => {
                                self.classes_editor_state.classes = classes;
                                self.status_message = format!(
                                    "Loaded {} classes",
                                    self.classes_editor_state.classes.len()
                                );
                            }
                            Err(e) => {
                                self.status_message = format!("Failed to parse classes: {}", e);
                                eprintln!("Failed to parse classes from {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to read classes file: {}", e);
                        eprintln!("Failed to read classes file {:?}: {}", path, e);
                    }
                }
            } else {
                eprintln!("Classes file does not exist: {:?}", path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load classes");
        }
    }

    fn save_classes(&self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.classes_file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(CampaignError::Io)?;
            }
            let content =
                ron::ser::to_string_pretty(&self.classes_editor_state.classes, Default::default())?;
            fs::write(path, content).map_err(CampaignError::Io)?;
        }
        Ok(())
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
    fn show_validation_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("✅ Campaign Validation");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Run Validation").clicked() {
                    self.validate_campaign();
                }
            });
        });
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

    /// Load dialogues from campaign file
    fn load_dialogues(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let dialogue_path = dir.join(&self.campaign.dialogue_file);
            if dialogue_path.exists() {
                self.load_dialogues_from_file(&dialogue_path)?;
            }
        }
        Ok(())
    }

    /// Load dialogues from file
    fn load_dialogues_from_file(&mut self, path: &std::path::Path) -> Result<(), CampaignError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => match ron::from_str::<Vec<DialogueTree>>(&contents) {
                Ok(dialogues) => {
                    self.dialogues = dialogues;
                    // Update editor state
                    self.dialogue_editor_state
                        .load_dialogues(self.dialogues.clone());
                    self.status_message = format!("Loaded {} dialogues", self.dialogues.len());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to parse dialogues from {:?}: {}", path, e);
                    Err(CampaignError::Deserialization(e))
                }
            },
            Err(e) => {
                eprintln!("Failed to read dialogues file {:?}: {}", path, e);
                Err(CampaignError::Io(e))
            }
        }
    }

    /// Show dialogues editor
    fn show_dialogues_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("💬 Dialogues Editor");
        ui.add_space(5.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ New Dialogue").clicked() {
                self.dialogue_editor_state.start_new_dialogue();
            }

            if ui.button("💾 Save to Campaign").clicked() {
                // Sync dialogue_editor_state.dialogues to self.dialogues
                self.dialogues = self.dialogue_editor_state.dialogues.clone();
                self.status_message = "Dialogues saved to campaign".to_string();
            }

            if ui.button("📂 Load from File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    match self.load_dialogues_from_file(&path) {
                        Ok(()) => {
                            self.dialogue_editor_state
                                .load_dialogues(self.dialogues.clone());
                            self.status_message =
                                format!("Loaded {} dialogues", self.dialogues.len());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load dialogues: {:?}", e);
                        }
                    }
                }
            }

            if ui.button("📤 Import RON").clicked() {
                self.dialogues_show_import_dialog = true;
                self.dialogues_import_buffer.clear();
            }

            if ui.button("📋 Export to Clipboard").clicked() {
                match ron::ser::to_string_pretty(&self.dialogues, Default::default()) {
                    Ok(ron_str) => {
                        ui.ctx().copy_text(ron_str.clone());
                        self.status_message = "Dialogues exported to clipboard".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Export failed: {:?}", e);
                    }
                }
            }

            ui.checkbox(&mut self.dialogues_show_preview, "Show Preview");
        });

        ui.separator();

        // Import dialog (handled outside main UI to avoid borrow conflicts)
        let mut import_complete = false;
        let mut import_result: Option<Result<DialogueTree, String>> = None;
        let mut cancel_import = false;

        if self.dialogues_show_import_dialog {
            let mut show_dialog = self.dialogues_show_import_dialog;
            egui::Window::new("Import Dialogue RON")
                .open(&mut show_dialog)
                .show(ui.ctx(), |ui| {
                    ui.label("Paste RON dialogue data:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.dialogues_import_buffer)
                            .desired_width(500.0)
                            .desired_rows(15),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Import").clicked() {
                            match ron::from_str::<DialogueTree>(&self.dialogues_import_buffer) {
                                Ok(dialogue) => {
                                    import_result = Some(Ok(dialogue));
                                    import_complete = true;
                                }
                                Err(e) => {
                                    import_result = Some(Err(format!("{:?}", e)));
                                    import_complete = true;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_import = true;
                        }
                    });
                });
            self.dialogues_show_import_dialog = show_dialog;
        }

        // Process import result outside window closure
        if cancel_import {
            self.dialogues_show_import_dialog = false;
        }
        if import_complete {
            if let Some(result) = import_result {
                match result {
                    Ok(mut dialogue) => {
                        let new_id = self.next_available_dialogue_id();
                        dialogue.id = new_id;
                        self.dialogues.push(dialogue);
                        self.dialogue_editor_state
                            .load_dialogues(self.dialogues.clone());
                        self.status_message = format!("Imported dialogue {}", new_id);
                        self.dialogues_show_import_dialog = false;
                    }
                    Err(e) => {
                        self.status_message = format!("Import failed: {}", e);
                    }
                }
            }
        }

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.dialogue_editor_state.search_filter);
        });

        ui.separator();

        // Main content area
        match self.dialogue_editor_state.mode {
            dialogue_editor::DialogueEditorMode::List => {
                self.show_dialogue_list(ui);
            }
            dialogue_editor::DialogueEditorMode::Creating
            | dialogue_editor::DialogueEditorMode::Editing => {
                self.show_dialogue_form(ui);
            }
        }
    }

    /// Show dialogue list view
    fn show_dialogue_list(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Total Dialogues: {}",
            self.dialogue_editor_state.dialogues.len()
        ));

        // Collect actions to avoid borrow conflicts
        let mut delete_idx: Option<usize> = None;
        let mut edit_idx: Option<usize> = None;
        let mut duplicate_idx: Option<usize> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let filtered = self.dialogue_editor_state.filtered_dialogues();

                for (idx, dialogue) in filtered {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("ID: {}", dialogue.id));
                            ui.separator();
                            ui.strong(&dialogue.name);
                            if let Some(speaker) = &dialogue.speaker_name {
                                ui.label(format!("(Speaker: {})", speaker));
                            }
                            ui.label(format!("({} nodes)", dialogue.nodes.len()));

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🗑 Delete").clicked() {
                                        delete_idx = Some(idx);
                                    }
                                    if ui.button("✏ Edit").clicked() {
                                        edit_idx = Some(idx);
                                    }
                                    if ui.button("📋 Duplicate").clicked() {
                                        duplicate_idx = Some(idx);
                                    }
                                },
                            );
                        });

                        // Preview details
                        if self.dialogues_show_preview {
                            ui.separator();
                            ui.label(format!("Root Node: {}", dialogue.root_node));
                            ui.label(format!("Repeatable: {}", dialogue.repeatable));
                            if let Some(quest_id) = dialogue.associated_quest {
                                ui.label(format!("Associated Quest: {}", quest_id));
                            }
                        }
                    });
                }
            });

        // Process actions after scroll area
        if let Some(idx) = delete_idx {
            self.dialogue_editor_state.delete_dialogue(idx);
            self.dialogues = self.dialogue_editor_state.dialogues.clone();
        }
        if let Some(idx) = edit_idx {
            self.dialogue_editor_state.start_edit_dialogue(idx);
        }
        if let Some(idx) = duplicate_idx {
            if let Some(dialogue) = self.dialogue_editor_state.dialogues.get(idx) {
                let mut new_dialogue = dialogue.clone();
                new_dialogue.id = self.next_available_dialogue_id();
                new_dialogue.name = format!("{} (Copy)", new_dialogue.name);
                self.dialogue_editor_state.dialogues.push(new_dialogue);
                self.dialogues = self.dialogue_editor_state.dialogues.clone();
            }
        }
    }

    /// Show dialogue form editor
    fn show_dialogue_form(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.dialogue_editor_state.cancel_edit();
            }

            if ui.button("💾 Save Dialogue").clicked() {
                match self.dialogue_editor_state.save_dialogue() {
                    Ok(()) => {
                        self.dialogues = self.dialogue_editor_state.dialogues.clone();
                        self.status_message = "Dialogue saved".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Save failed: {}", e);
                    }
                }
            }
        });

        ui.separator();

        // Dialogue form
        egui::Grid::new("dialogue_form_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Dialogue ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dialogue_editor_state.dialogue_buffer.id)
                        .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self.dialogue_editor_state.dialogue_buffer.name,
                    )
                    .desired_width(300.0),
                );
                ui.end_row();

                ui.label("Speaker Name:");
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self.dialogue_editor_state.dialogue_buffer.speaker_name,
                    )
                    .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Repeatable:");
                ui.checkbox(
                    &mut self.dialogue_editor_state.dialogue_buffer.repeatable,
                    "",
                );
                ui.end_row();

                ui.label("Associated Quest ID:");
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self.dialogue_editor_state.dialogue_buffer.associated_quest,
                    )
                    .desired_width(150.0),
                );
                ui.end_row();
            });

        ui.separator();

        // Node tree editor
        if let Some(dialogue_idx) = self.dialogue_editor_state.selected_dialogue {
            if dialogue_idx < self.dialogue_editor_state.dialogues.len() {
                ui.heading("Dialogue Nodes");
                self.show_dialogue_nodes_editor(ui, dialogue_idx);
            }
        } else {
            ui.label("Save dialogue to add nodes");
        }
    }

    /// Show dialogue node tree editor
    fn show_dialogue_nodes_editor(&mut self, ui: &mut egui::Ui, dialogue_idx: usize) {
        // Add node form
        let mut add_node_clicked = false;
        ui.horizontal(|ui| {
            ui.label("Add New Node:");
            ui.add(
                egui::TextEdit::singleline(&mut self.dialogue_editor_state.node_buffer.id)
                    .hint_text("Node ID")
                    .desired_width(80.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.dialogue_editor_state.node_buffer.text)
                    .hint_text("Node text")
                    .desired_width(250.0),
            );
            ui.checkbox(
                &mut self.dialogue_editor_state.node_buffer.is_terminal,
                "Terminal",
            );

            if ui.button("➕ Add Node").clicked() {
                add_node_clicked = true;
            }
        });

        if add_node_clicked {
            match self.dialogue_editor_state.add_node() {
                Ok(()) => {
                    self.status_message = "Node added".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Failed to add node: {}", e);
                }
            }
        }

        ui.separator();

        // Clone dialogue data to avoid borrow conflicts
        let dialogue = self.dialogue_editor_state.dialogues[dialogue_idx].clone();
        let mut select_node: Option<NodeId> = None;

        // Display nodes
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for (node_id, node) in &dialogue.nodes {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(format!("Node {}", node_id));
                            if *node_id == dialogue.root_node {
                                ui.label("(ROOT)");
                            }
                            if node.is_terminal {
                                ui.label("(TERMINAL)");
                            }
                        });

                        ui.label(&node.text);

                        if let Some(speaker) = &node.speaker_override {
                            ui.label(format!("Speaker: {}", speaker));
                        }

                        // Show choices
                        if !node.choices.is_empty() {
                            ui.separator();
                            ui.label("Choices:");
                            for (choice_idx, choice) in node.choices.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("  {}. {}", choice_idx + 1, choice.text));
                                    if let Some(target) = choice.target_node {
                                        ui.label(format!("→ Node {}", target));
                                    }
                                    if choice.ends_dialogue {
                                        ui.label("(Ends)");
                                    }
                                });
                            }
                        }

                        // Show conditions
                        if !node.conditions.is_empty() {
                            ui.separator();
                            ui.label(format!("Conditions: {}", node.conditions.len()));
                        }

                        // Show actions
                        if !node.actions.is_empty() {
                            ui.separator();
                            ui.label(format!("Actions: {}", node.actions.len()));
                        }

                        // Add choice to this node
                        if ui.button("➕ Add Choice").clicked() {
                            select_node = Some(*node_id);
                        }
                    });
                }
            });

        // Process node selection outside scroll area
        if let Some(node_id) = select_node {
            self.dialogue_editor_state.selected_node = Some(node_id);
        }

        // Choice editor panel
        let mut add_choice_clicked = false;
        let mut cancel_choice_clicked = false;

        if let Some(selected_node_id) = self.dialogue_editor_state.selected_node {
            ui.separator();
            ui.heading(format!("Add Choice to Node {}", selected_node_id));

            ui.horizontal(|ui| {
                ui.label("Choice Text:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dialogue_editor_state.choice_buffer.text)
                        .desired_width(250.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Target Node ID:");
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self.dialogue_editor_state.choice_buffer.target_node,
                    )
                    .desired_width(80.0),
                );
                ui.checkbox(
                    &mut self.dialogue_editor_state.choice_buffer.ends_dialogue,
                    "Ends Dialogue",
                );
            });

            ui.horizontal(|ui| {
                if ui.button("✓ Add Choice").clicked() {
                    add_choice_clicked = true;
                }
                if ui.button("✗ Cancel").clicked() {
                    cancel_choice_clicked = true;
                }
            });
        }

        // Process choice actions outside choice panel
        if add_choice_clicked {
            match self.dialogue_editor_state.add_choice() {
                Ok(()) => {
                    self.status_message = "Choice added".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Failed to add choice: {}", e);
                }
            }
        }
        if cancel_choice_clicked {
            self.dialogue_editor_state.selected_node = None;
        }
    }

    /// Generate next available dialogue ID
    fn next_available_dialogue_id(&self) -> u16 {
        self.dialogues
            .iter()
            .map(|d| d.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Save dialogues to file
    fn save_dialogues_to_file(&self, path: &std::path::Path) -> Result<(), CampaignError> {
        // Create dialogues directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(CampaignError::Io)?;
        }

        let ron = ron::ser::to_string_pretty(&self.dialogues, Default::default())
            .map_err(CampaignError::Serialization)?;
        std::fs::write(path, ron).map_err(CampaignError::Io)?;
        Ok(())
    }

    /// Show assets editor
    fn show_assets_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("📦 Asset Manager");
        ui.add_space(5.0);
        ui.label("Manage campaign assets (images, sounds, music, tilesets)");
        ui.separator();

        // Initialize asset manager if needed
        if self.asset_manager.is_none() {
            if let Some(ref campaign_dir) = self.campaign_dir {
                let mut manager = asset_manager::AssetManager::new(campaign_dir.clone());
                if let Err(e) = manager.scan_directory() {
                    self.status_message = format!("Failed to scan assets: {}", e);
                } else {
                    self.asset_manager = Some(manager);
                }
            }
        }

        if let Some(ref mut manager) = self.asset_manager {
            // Toolbar with actions
            ui.horizontal(|ui| {
                ui.label(format!("Total Assets: {}", manager.asset_count()));
                ui.separator();
                ui.label(format!("Total Size: {}", manager.total_size_string()));
                ui.separator();

                if ui.button("🔄 Refresh").clicked() {
                    if let Err(e) = manager.scan_directory() {
                        self.status_message = format!("Failed to refresh assets: {}", e);
                    } else {
                        self.status_message = "Assets refreshed".to_string();
                    }
                }

                if ui.button("🔍 Scan References").clicked() {
                    // Scan references across all campaign data
                    manager.scan_references(
                        &self.items,
                        &self.quests,
                        &self.dialogues,
                        &self.maps,
                        &self.classes_editor_state.classes,
                    );
                    self.status_message = "Asset references scanned".to_string();
                }
            });

            ui.separator();

            // Asset type filters
            ui.horizontal(|ui| {
                ui.label("Filter by type:");
                for asset_type in asset_manager::AssetType::all() {
                    let count = manager.asset_count_by_type(asset_type);
                    if ui
                        .button(format!("{} ({})", asset_type.display_name(), count))
                        .clicked()
                    {
                        // Filter would be implemented here
                    }
                }
            });

            ui.separator();

            // Unreferenced assets warning and cleanup
            let unreferenced_count = manager.unreferenced_assets().len();
            let cleanup_candidates_count = manager.get_cleanup_candidates().len();

            if unreferenced_count > 0 {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("⚠ {} unreferenced assets found", unreferenced_count),
                    );

                    if cleanup_candidates_count > 0 {
                        if ui
                            .button(format!(
                                "🧹 Cleanup {} Unused Assets",
                                cleanup_candidates_count
                            ))
                            .clicked()
                        {
                            // Show confirmation or perform cleanup
                            match manager.cleanup_unused(true) {
                                Ok(would_delete) => {
                                    self.status_message = format!(
                                        "Would delete {} assets (dry run)",
                                        would_delete.len()
                                    );
                                }
                                Err(e) => {
                                    self.status_message = format!("Cleanup error: {}", e);
                                }
                            }
                        }
                    }
                });
                ui.separator();
            }

            // Asset list with usage context
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (path, asset) in manager.assets() {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            // Asset header
                            ui.horizontal(|ui| {
                                ui.label(format!("📄 {}", path.display()));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(asset.size_string());
                                        ui.label(asset.asset_type.display_name());
                                        if !asset.is_referenced {
                                            ui.colored_label(egui::Color32::YELLOW, "⚠ Unused");
                                        } else {
                                            ui.colored_label(egui::Color32::GREEN, "✓ In Use");
                                        }
                                    },
                                );
                            });

                            // Show references if any
                            if !asset.references.is_empty() {
                                ui.indent("asset_refs", |ui| {
                                    ui.label(format!(
                                        "Referenced by {} item(s):",
                                        asset.references.len()
                                    ));
                                    for reference in &asset.references {
                                        ui.label(format!("  • {}", reference.display_string()));
                                    }
                                });
                            }
                        });
                    });
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No campaign directory loaded");
                ui.label("Open or create a campaign to manage assets");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::quest::QuestStage;

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
        assert_eq!(EditorTab::Metadata.name(), "Metadata");
        assert_eq!(EditorTab::Items.name(), "Items");
        assert_eq!(EditorTab::Spells.name(), "Spells");
        assert_eq!(EditorTab::Monsters.name(), "Monsters");
        assert_eq!(EditorTab::Maps.name(), "Maps");
        assert_eq!(EditorTab::Quests.name(), "Quests");
        assert_eq!(EditorTab::Dialogues.name(), "Dialogues");
        assert_eq!(EditorTab::Assets.name(), "Assets");
        assert_eq!(EditorTab::Validation.name(), "Validation");
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
    }

    #[test]
    fn test_quest_objective_editor_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.quests.len(), 0);
        assert_eq!(app.quest_editor_state.selected_quest, None);
        assert_eq!(app.quest_editor_state.selected_stage, None);
        assert_eq!(app.quest_editor_state.selected_objective, None);
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::List
        );
    }

    #[test]
    fn test_quest_stage_editing_flow() {
        let mut app = CampaignBuilderApp::default();

        // Create a quest with a stage
        let quest = Quest {
            id: 1,
            name: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            is_main_quest: false,
            repeatable: false,
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            stages: vec![QuestStage {
                stage_number: 1,
                name: "Stage 1".to_string(),
                description: "Stage 1 description".to_string(),
                require_all_objectives: true,
                objectives: Vec::new(),
            }],
            rewards: Vec::new(),
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        app.quests.push(quest.clone());
        app.quest_editor_state.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Test edit stage
        let result = app.quest_editor_state.edit_stage(0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quest_editor_state.selected_stage, Some(0));
        assert_eq!(app.quest_editor_state.stage_buffer.name, "Stage 1");
        assert_eq!(
            app.quest_editor_state.stage_buffer.description,
            "Stage 1 description"
        );

        // Test save stage
        app.quest_editor_state.stage_buffer.name = "Updated Stage".to_string();
        let result = app.quest_editor_state.save_stage(0, 0);
        assert!(result.is_ok());
        assert_eq!(
            app.quest_editor_state.quests[0].stages[0].name,
            "Updated Stage"
        );
        assert_eq!(app.quest_editor_state.selected_stage, None);
        assert!(app.quest_editor_state.has_unsaved_changes);
    }

    #[test]
    fn test_quest_objective_editing_flow() {
        let mut app = CampaignBuilderApp::default();

        // Create a quest with a stage and objective
        let quest = Quest {
            id: 1,
            name: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            is_main_quest: false,
            repeatable: false,
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            stages: vec![QuestStage {
                stage_number: 1,
                name: "Stage 1".to_string(),
                description: "Stage 1 description".to_string(),
                require_all_objectives: true,
                objectives: vec![antares::domain::quest::QuestObjective::KillMonsters {
                    monster_id: 100,
                    quantity: 5,
                }],
            }],
            rewards: Vec::new(),
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        app.quests.push(quest.clone());
        app.quest_editor_state.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Test edit objective
        let result = app.quest_editor_state.edit_objective(0, 0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quest_editor_state.selected_objective, Some(0));
        assert_eq!(
            app.quest_editor_state.objective_buffer.objective_type,
            quest_editor::ObjectiveType::KillMonsters
        );
        assert_eq!(app.quest_editor_state.objective_buffer.monster_id, "100");
        assert_eq!(app.quest_editor_state.objective_buffer.quantity, "5");

        // Test save objective
        app.quest_editor_state.objective_buffer.quantity = "10".to_string();
        let result = app.quest_editor_state.save_objective(0, 0, 0);
        assert!(result.is_ok());

        if let antares::domain::quest::QuestObjective::KillMonsters {
            monster_id,
            quantity,
        } = &app.quest_editor_state.quests[0].stages[0].objectives[0]
        {
            assert_eq!(*quantity, 10);
        } else {
            panic!("Expected KillMonsters objective");
        }

        assert_eq!(app.quest_editor_state.selected_objective, None);
        assert!(app.quest_editor_state.has_unsaved_changes);
    }

    #[test]
    fn test_quest_stage_deletion() {
        let mut app = CampaignBuilderApp::default();

        // Create a quest with two stages
        let quest = Quest {
            id: 1,
            name: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            is_main_quest: false,
            repeatable: false,
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            stages: vec![
                QuestStage {
                    stage_number: 1,
                    name: "Stage 1".to_string(),
                    description: "Stage 1 description".to_string(),
                    require_all_objectives: true,
                    objectives: Vec::new(),
                },
                QuestStage {
                    stage_number: 2,
                    name: "Stage 2".to_string(),
                    description: "Stage 2 description".to_string(),
                    require_all_objectives: true,
                    objectives: Vec::new(),
                },
            ],
            rewards: Vec::new(),
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        app.quests.push(quest.clone());
        app.quest_editor_state.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Delete first stage
        assert_eq!(app.quest_editor_state.quests[0].stages.len(), 2);
        let result = app.quest_editor_state.delete_stage(0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quest_editor_state.quests[0].stages.len(), 1);
        assert_eq!(app.quest_editor_state.quests[0].stages[0].name, "Stage 2");
        assert!(app.quest_editor_state.has_unsaved_changes);
    }

    #[test]
    fn test_quest_objective_deletion() {
        let mut app = CampaignBuilderApp::default();

        // Create a quest with a stage and multiple objectives
        let quest = Quest {
            id: 1,
            name: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            is_main_quest: false,
            repeatable: false,
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            stages: vec![QuestStage {
                stage_number: 1,
                name: "Stage 1".to_string(),
                description: "Stage 1 description".to_string(),
                require_all_objectives: true,
                objectives: vec![
                    antares::domain::quest::QuestObjective::KillMonsters {
                        monster_id: 100,
                        quantity: 5,
                    },
                    antares::domain::quest::QuestObjective::CollectItems {
                        item_id: 200,
                        quantity: 3,
                    },
                ],
            }],
            rewards: Vec::new(),
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        app.quests.push(quest.clone());
        app.quest_editor_state.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Delete first objective
        assert_eq!(
            app.quest_editor_state.quests[0].stages[0].objectives.len(),
            2
        );
        let result = app.quest_editor_state.delete_objective(0, 0, 0);
        assert!(result.is_ok());
        assert_eq!(
            app.quest_editor_state.quests[0].stages[0].objectives.len(),
            1
        );

        // Verify remaining objective is CollectItems
        if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
            &app.quest_editor_state.quests[0].stages[0].objectives[0]
        {
            assert_eq!(*item_id, 200);
            assert_eq!(*quantity, 3);
        } else {
            panic!("Expected CollectItems objective");
        }

        assert!(app.quest_editor_state.has_unsaved_changes);
    }

    #[test]
    fn test_quest_objective_type_conversion() {
        let mut app = CampaignBuilderApp::default();

        // Create a quest with a KillMonsters objective
        let quest = Quest {
            id: 1,
            name: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            is_main_quest: false,
            repeatable: false,
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            stages: vec![QuestStage {
                stage_number: 1,
                name: "Stage 1".to_string(),
                description: "Stage 1 description".to_string(),
                require_all_objectives: true,
                objectives: vec![antares::domain::quest::QuestObjective::KillMonsters {
                    monster_id: 100,
                    quantity: 5,
                }],
            }],
            rewards: Vec::new(),
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        };

        app.quests.push(quest.clone());
        app.quest_editor_state.quests.push(quest);

        // Edit objective and change type to CollectItems
        let result = app.quest_editor_state.edit_objective(0, 0, 0);
        assert!(result.is_ok());

        app.quest_editor_state.objective_buffer.objective_type =
            quest_editor::ObjectiveType::CollectItems;
        app.quest_editor_state.objective_buffer.item_id = "250".to_string();
        app.quest_editor_state.objective_buffer.quantity = "7".to_string();

        let result = app.quest_editor_state.save_objective(0, 0, 0);
        assert!(result.is_ok());

        // Verify objective type changed
        if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
            &app.quest_editor_state.quests[0].stages[0].objectives[0]
        {
            assert_eq!(*item_id, 250);
            assert_eq!(*quantity, 7);
        } else {
            panic!("Expected CollectItems objective");
        }
    }

    #[test]
    fn test_quest_editor_invalid_indices() {
        let mut app = CampaignBuilderApp::default();

        // Test with no quests
        let result = app.quest_editor_state.edit_stage(0, 0);
        assert!(result.is_err());

        let result = app.quest_editor_state.edit_objective(0, 0, 0);
        assert!(result.is_err());

        let result = app.quest_editor_state.delete_stage(0, 0);
        assert!(result.is_err());

        let result = app.quest_editor_state.delete_objective(0, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_item_type_is_quest_item() {
        let item = CampaignBuilderApp::default_item();
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

    // ===== Phase 3A: ID Validation and Generation Tests =====

    #[test]
    fn test_item_id_uniqueness_validation() {
        let mut app = CampaignBuilderApp::default();

        // Add items with duplicate IDs
        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        item1.name = "Item 1".to_string();
        app.items.push(item1);

        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = 1; // Duplicate ID
        item2.name = "Item 2".to_string();
        app.items.push(item2);

        let mut item3 = CampaignBuilderApp::default_item();
        item3.id = 2;
        item3.name = "Item 3".to_string();
        app.items.push(item3);

        // Validate
        let errors = app.validate_item_ids();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("Duplicate item ID: 1"));
    }

    #[test]
    fn test_spell_id_uniqueness_validation() {
        let mut app = CampaignBuilderApp::default();

        // Add spells with duplicate IDs
        let mut spell1 = CampaignBuilderApp::default_spell();
        spell1.id = 100;
        spell1.name = "Spell 1".to_string();
        app.spells.push(spell1);

        let mut spell2 = CampaignBuilderApp::default_spell();
        spell2.id = 100; // Duplicate ID
        spell2.name = "Spell 2".to_string();
        app.spells.push(spell2);

        // Validate
        let errors = app.validate_spell_ids();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("Duplicate spell ID: 100"));
    }

    #[test]
    fn test_monster_id_uniqueness_validation() {
        let mut app = CampaignBuilderApp::default();

        // Add monsters with duplicate IDs
        let mut monster1 = CampaignBuilderApp::default_monster();
        monster1.id = 5;
        monster1.name = "Monster 1".to_string();
        app.monsters.push(monster1);

        let mut monster2 = CampaignBuilderApp::default_monster();
        monster2.id = 5; // Duplicate ID
        monster2.name = "Monster 2".to_string();
        app.monsters.push(monster2);

        // Validate
        let errors = app.validate_monster_ids();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("Duplicate monster ID: 5"));
    }

    #[test]
    fn test_map_id_uniqueness_validation() {
        let mut app = CampaignBuilderApp::default();

        // Add maps with duplicate IDs
        let map1 = Map::new(10, "Map 1".to_string(), "Desc 1".to_string(), 20, 20);
        app.maps.push(map1);

        let map2 = Map::new(10, "Map 2".to_string(), "Desc 2".to_string(), 30, 30); // Duplicate ID
        app.maps.push(map2);

        // Validate
        let errors = app.validate_map_ids();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("Duplicate map ID: 10"));
    }

    #[test]
    fn test_next_available_item_id_empty() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.next_available_item_id(), 1);
    }

    #[test]
    fn test_next_available_item_id_with_items() {
        let mut app = CampaignBuilderApp::default();

        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        app.items.push(item1);

        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = 5;
        app.items.push(item2);

        let mut item3 = CampaignBuilderApp::default_item();
        item3.id = 3;
        app.items.push(item3);

        // Should return max + 1 = 6
        assert_eq!(app.next_available_item_id(), 6);
    }

    #[test]
    fn test_next_available_spell_id() {
        let mut app = CampaignBuilderApp::default();

        let mut spell1 = CampaignBuilderApp::default_spell();
        spell1.id = 100;
        app.spells.push(spell1);

        let mut spell2 = CampaignBuilderApp::default_spell();
        spell2.id = 150;
        app.spells.push(spell2);

        assert_eq!(app.next_available_spell_id(), 151);
    }

    #[test]
    fn test_next_available_monster_id() {
        let mut app = CampaignBuilderApp::default();

        let mut monster1 = CampaignBuilderApp::default_monster();
        monster1.id = 10;
        app.monsters.push(monster1);

        assert_eq!(app.next_available_monster_id(), 11);
    }

    #[test]
    fn test_next_available_map_id() {
        let mut app = CampaignBuilderApp::default();

        let map1 = Map::new(5, "Map 1".to_string(), "Desc 1".to_string(), 20, 20);
        app.maps.push(map1);

        let map2 = Map::new(8, "Map 2".to_string(), "Desc 2".to_string(), 30, 30);
        app.maps.push(map2);

        assert_eq!(app.next_available_map_id(), 9);
    }

    #[test]
    fn test_id_generation_with_gaps() {
        let mut app = CampaignBuilderApp::default();

        // Add items with IDs: 1, 2, 5 (gap at 3, 4)
        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        app.items.push(item1);

        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = 2;
        app.items.push(item2);

        let mut item3 = CampaignBuilderApp::default_item();
        item3.id = 5;
        app.items.push(item3);

        // Should return 6 (max + 1), not fill gap
        assert_eq!(app.next_available_item_id(), 6);
    }

    #[test]
    fn test_validate_campaign_includes_id_checks() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test_campaign".to_string();
        app.campaign.name = "Test Campaign".to_string();
        app.campaign.version = "1.0.0".to_string();
        app.campaign.engine_version = "0.1.0".to_string();
        app.campaign.starting_map = "start".to_string();

        // Add duplicate item IDs
        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        app.items.push(item1);

        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = 1; // Duplicate
        app.items.push(item2);

        // Run validation
        app.validate_campaign();

        // Should have error for duplicate item ID
        let has_duplicate_error = app
            .validation_errors
            .iter()
            .any(|e| e.message.contains("Duplicate item ID"));
        assert!(has_duplicate_error);
    }

    #[test]
    fn test_no_duplicate_ids_validation_passes() {
        let mut app = CampaignBuilderApp::default();

        // Add items with unique IDs
        let mut item1 = CampaignBuilderApp::default_item();
        item1.id = 1;
        app.items.push(item1);

        let mut item2 = CampaignBuilderApp::default_item();
        item2.id = 2;
        app.items.push(item2);

        // Validate
        let errors = app.validate_item_ids();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_saturating_add_prevents_overflow() {
        let mut app = CampaignBuilderApp::default();

        // Add item with max ID for ItemId (u8)
        let mut item = CampaignBuilderApp::default_item();
        item.id = 255; // u8::MAX
        app.items.push(item);

        // Should saturate at 255, not overflow
        assert_eq!(app.next_available_item_id(), 255);
    }

    // ===== Phase 3B: Items Editor Enhancement Tests =====

    #[test]
    fn test_item_type_filter_weapon() {
        let app = CampaignBuilderApp::default();

        let mut weapon = CampaignBuilderApp::default_item();
        weapon.item_type = ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 8, 0),
            bonus: 1,
            hands_required: 1,
        });

        let filter = ItemTypeFilter::Weapon;
        assert!(filter.matches(&weapon));

        // Should not match other types
        let mut armor = CampaignBuilderApp::default_item();
        armor.item_type = ItemType::Armor(antares::domain::items::types::ArmorData {
            ac_bonus: 5,
            weight: 20,
        });
        assert!(!filter.matches(&armor));
    }

    #[test]
    fn test_item_type_filter_all_types() {
        use antares::domain::items::types::*;

        let weapon_item = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 10,
            sell_cost: 5,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        let armor_item = Item {
            id: 2,
            name: "Chainmail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 5,
                weight: 30,
            }),
            base_cost: 50,
            sell_cost: 25,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        assert!(ItemTypeFilter::Weapon.matches(&weapon_item));
        assert!(!ItemTypeFilter::Weapon.matches(&armor_item));
        assert!(ItemTypeFilter::Armor.matches(&armor_item));
        assert!(!ItemTypeFilter::Armor.matches(&weapon_item));
    }

    #[test]
    fn test_items_filter_magical() {
        let mut app = CampaignBuilderApp {
            items_filter_magical: Some(true),
            ..Default::default()
        };

        let mut magical_item = CampaignBuilderApp::default_item();
        magical_item.id = 1;
        magical_item.name = "Magic Sword".to_string();
        magical_item.max_charges = 10;
        app.items.push(magical_item.clone());

        let mut mundane_item = CampaignBuilderApp::default_item();
        mundane_item.id = 2;
        mundane_item.name = "Normal Sword".to_string();
        mundane_item.max_charges = 0;
        app.items.push(mundane_item);

        // Magical filter should only match magical items
        assert!(magical_item.is_magical());
    }

    #[test]
    fn test_items_filter_cursed() {
        let mut cursed_item = CampaignBuilderApp::default_item();
        cursed_item.is_cursed = true;

        let normal_item = CampaignBuilderApp::default_item();

        assert!(cursed_item.is_cursed);
        assert!(!normal_item.is_cursed);
    }

    #[test]
    fn test_items_filter_quest() {
        use antares::domain::items::types::QuestData;

        let mut quest_item = CampaignBuilderApp::default_item();
        quest_item.item_type = ItemType::Quest(QuestData {
            quest_id: "main_quest".to_string(),
            is_key_item: true,
        });

        let normal_item = CampaignBuilderApp::default_item();

        assert!(quest_item.is_quest_item());
        assert!(!normal_item.is_quest_item());
    }

    #[test]
    fn test_disablement_flags() {
        let mut item = CampaignBuilderApp::default_item();

        // Test all classes enabled
        item.disablements = Disablement(0xFF);
        assert!(item.disablements.can_use_class(Disablement::KNIGHT));
        assert!(item.disablements.can_use_class(Disablement::SORCERER));

        // Test specific class restriction
        item.disablements = Disablement(Disablement::KNIGHT | Disablement::PALADIN);
        assert!(item.disablements.can_use_class(Disablement::KNIGHT));
        assert!(item.disablements.can_use_class(Disablement::PALADIN));
        assert!(!item.disablements.can_use_class(Disablement::SORCERER));

        // Test alignment flags
        item.disablements = Disablement(Disablement::KNIGHT | Disablement::GOOD);
        assert!(item.disablements.good_only());
        assert!(!item.disablements.evil_only());
    }

    #[test]
    fn test_item_import_export_roundtrip() {
        use antares::domain::items::types::*;

        let original_item = Item {
            id: 42,
            name: "Test Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 6, 1),
                bonus: 2,
                hands_required: 1,
            }),
            base_cost: 100,
            sell_cost: 50,
            disablements: Disablement(0xFF),
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 3,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };

        // Export to RON
        let ron_string =
            ron::ser::to_string_pretty(&original_item, ron::ser::PrettyConfig::default())
                .expect("Failed to serialize item");

        // Import from RON
        let imported_item: Item = ron::from_str(&ron_string).expect("Failed to deserialize item");

        // Verify roundtrip
        assert_eq!(original_item.id, imported_item.id);
        assert_eq!(original_item.name, imported_item.name);
        assert_eq!(original_item.base_cost, imported_item.base_cost);
        assert_eq!(original_item.is_cursed, imported_item.is_cursed);

        // Verify weapon data
        if let (ItemType::Weapon(orig_data), ItemType::Weapon(import_data)) =
            (&original_item.item_type, &imported_item.item_type)
        {
            assert_eq!(orig_data.damage, import_data.damage);
            assert_eq!(orig_data.bonus, import_data.bonus);
            assert_eq!(orig_data.hands_required, import_data.hands_required);
        } else {
            panic!("Item type mismatch after roundtrip");
        }
    }

    #[test]
    fn test_item_type_specific_editors() {
        use antares::domain::items::types::*;

        // Test weapon type
        let weapon = Item {
            id: 1,
            name: "Longsword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 15,
            sell_cost: 7,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };
        assert!(weapon.is_weapon());

        // Test armor type
        let armor = Item {
            id: 2,
            name: "Plate Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 8,
                weight: 50,
            }),
            base_cost: 100,
            sell_cost: 50,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };
        assert!(armor.is_armor());

        // Test consumable type
        let potion = Item {
            id: 3,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
            }),
            base_cost: 10,
            sell_cost: 5,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        };
        assert!(potion.is_consumable());
    }

    #[test]
    fn test_combined_filters() {
        use antares::domain::items::types::*;

        let mut app = CampaignBuilderApp::default();

        // Add various items
        let magical_weapon = Item {
            id: 1,
            name: "Magic Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 1,
                hands_required: 1,
            }),
            base_cost: 100,
            sell_cost: 50,
            disablements: Disablement::ALL,
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 2,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 5,
            is_cursed: false,
            icon_path: None,
        };

        let cursed_armor = Item {
            id: 2,
            name: "Cursed Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 5,
                weight: 30,
            }),
            base_cost: 50,
            sell_cost: 0,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: true,
            icon_path: None,
        };

        app.items.push(magical_weapon.clone());
        app.items.push(cursed_armor.clone());

        // Test magical + weapon filters
        assert!(magical_weapon.is_magical());
        assert!(magical_weapon.is_weapon());
        assert!(!cursed_armor.is_magical());
        assert!(cursed_armor.is_cursed);
    }

    #[test]
    fn test_disablement_editor_all_classes() {
        let mut app = CampaignBuilderApp::default();

        // Set all classes enabled
        app.items_edit_buffer.disablements = Disablement(0b0011_1111);

        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::KNIGHT));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::PALADIN));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::ARCHER));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::CLERIC));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::SORCERER));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::ROBBER));
    }

    #[test]
    fn test_disablement_editor_specific_classes() {
        let mut app = CampaignBuilderApp::default();

        // Only knight and paladin
        app.items_edit_buffer.disablements =
            Disablement(Disablement::KNIGHT | Disablement::PALADIN);

        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::KNIGHT));
        assert!(app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::PALADIN));
        assert!(!app
            .items_edit_buffer
            .disablements
            .can_use_class(Disablement::SORCERER));
    }

    #[test]
    fn test_item_preview_displays_all_info() {
        use antares::domain::items::types::*;

        let item = Item {
            id: 10,
            name: "Flaming Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 2),
                bonus: 3,
                hands_required: 1,
            }),
            base_cost: 500,
            sell_cost: 250,
            disablements: Disablement(
                Disablement::KNIGHT | Disablement::PALADIN | Disablement::GOOD,
            ),
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 2,
            }),
            temporary_bonus: None,
            spell_effect: Some(10),
            max_charges: 20,
            is_cursed: false,
            icon_path: None,
        };

        // Verify item has all expected properties
        assert_eq!(item.id, 10);
        assert!(item.is_weapon());
        assert!(item.is_magical());
        assert!(!item.is_cursed);
        assert!(item.constant_bonus.is_some());
        assert!(item.spell_effect.is_some());
        assert_eq!(item.max_charges, 20);
    }

    // ===== Phase 3C Tests: Spell Editor Enhancements =====

    #[test]
    fn test_spell_school_filter_cleric() {
        let mut app = CampaignBuilderApp::default();

        // Add test spells
        app.spells.push(Spell::new(
            1,
            "Heal",
            SpellSchool::Cleric,
            1,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals wounds",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            2,
            "Fireball",
            SpellSchool::Sorcerer,
            3,
            5,
            0,
            SpellContext::CombatOnly,
            SpellTarget::MonsterGroup,
            "Fire damage",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            3,
            "Bless",
            SpellSchool::Cleric,
            2,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::AllCharacters,
            "Party buff",
            None,
            0,
            false,
        ));

        // Apply Cleric filter
        app.spells_filter_school = Some(SpellSchool::Cleric);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| app.spells_filter_school.is_none_or(|f| s.school == f))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|s| s.school == SpellSchool::Cleric));
    }

    #[test]
    fn test_spell_level_filter() {
        let mut app = CampaignBuilderApp::default();

        app.spells.push(Spell::new(
            1,
            "Heal",
            SpellSchool::Cleric,
            1,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Level 1",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            2,
            "Fireball",
            SpellSchool::Sorcerer,
            3,
            5,
            0,
            SpellContext::CombatOnly,
            SpellTarget::MonsterGroup,
            "Level 3",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            3,
            "Lightning",
            SpellSchool::Sorcerer,
            3,
            6,
            0,
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            "Level 3",
            None,
            0,
            false,
        ));

        // Filter level 3 spells
        app.spells_filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| app.spells_filter_level.is_none_or(|f| s.level == f))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|s| s.level == 3));
    }

    #[test]
    fn test_spell_combined_filters() {
        let mut app = CampaignBuilderApp::default();

        app.spells.push(Spell::new(
            1,
            "Heal",
            SpellSchool::Cleric,
            1,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Cleric L1",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            2,
            "Cure Disease",
            SpellSchool::Cleric,
            3,
            5,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Cleric L3",
            None,
            0,
            false,
        ));
        app.spells.push(Spell::new(
            3,
            "Fireball",
            SpellSchool::Sorcerer,
            3,
            5,
            0,
            SpellContext::CombatOnly,
            SpellTarget::MonsterGroup,
            "Sorcerer L3",
            None,
            0,
            false,
        ));

        // Filter: Cleric + Level 3
        app.spells_filter_school = Some(SpellSchool::Cleric);
        app.spells_filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| {
                app.spells_filter_school.is_none_or(|f| s.school == f)
                    && app.spells_filter_level.is_none_or(|f| s.level == f)
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Cure Disease");
        assert_eq!(filtered[0].school, SpellSchool::Cleric);
        assert_eq!(filtered[0].level, 3);
    }

    #[test]
    fn test_spell_context_target_editing() {
        let mut app = CampaignBuilderApp {
            spells_edit_buffer: Spell::new(
                1,
                "Test",
                SpellSchool::Cleric,
                1,
                1,
                0,
                SpellContext::Anytime,
                SpellTarget::Self_,
                "Test",
                None,
                0,
                false,
            ),
            ..Default::default()
        };

        // Change context
        app.spells_edit_buffer.context = SpellContext::CombatOnly;
        assert_eq!(app.spells_edit_buffer.context, SpellContext::CombatOnly);

        // Change target
        app.spells_edit_buffer.target = SpellTarget::AllCharacters;
        assert_eq!(app.spells_edit_buffer.target, SpellTarget::AllCharacters);
    }

    #[test]
    fn test_spell_import_export_roundtrip() {
        let original = Spell::new(
            42,
            "Test Spell",
            SpellSchool::Sorcerer,
            5,
            10,
            2,
            SpellContext::CombatOnly,
            SpellTarget::AllMonsters,
            "Test description",
            None,
            0,
            false,
        );

        // Export to RON
        let ron_data = ron::to_string(&original).unwrap();

        // Import from RON
        let imported: Spell = ron::from_str(&ron_data).unwrap();

        assert_eq!(imported.id, original.id);
        assert_eq!(imported.name, original.name);
        assert_eq!(imported.school, original.school);
        assert_eq!(imported.level, original.level);
        assert_eq!(imported.sp_cost, original.sp_cost);
        assert_eq!(imported.gem_cost, original.gem_cost);
        assert_eq!(imported.context, original.context);
        assert_eq!(imported.target, original.target);
    }

    // ===== Phase 3C Tests: Monster Editor Enhancements =====

    #[test]
    fn test_monster_attacks_editor() {
        let mut app = CampaignBuilderApp {
            monsters_edit_buffer: CampaignBuilderApp::default_monster(),
            ..Default::default()
        };

        // Initial attacks
        assert_eq!(app.monsters_edit_buffer.attacks.len(), 1);

        // Add attack
        app.monsters_edit_buffer.attacks.push(Attack {
            damage: DiceRoll::new(2, 8, 3),
            attack_type: AttackType::Fire,
            special_effect: Some(SpecialEffect::Poison),
        });

        assert_eq!(app.monsters_edit_buffer.attacks.len(), 2);
        assert_eq!(app.monsters_edit_buffer.attacks[1].damage.count, 2);
        assert_eq!(app.monsters_edit_buffer.attacks[1].damage.sides, 8);
        assert_eq!(app.monsters_edit_buffer.attacks[1].damage.bonus, 3);
        assert_eq!(
            app.monsters_edit_buffer.attacks[1].attack_type,
            AttackType::Fire
        );
        assert_eq!(
            app.monsters_edit_buffer.attacks[1].special_effect,
            Some(SpecialEffect::Poison)
        );
    }

    #[test]
    fn test_monster_attack_types() {
        let mut attack = Attack {
            damage: DiceRoll::new(1, 6, 0),
            attack_type: AttackType::Physical,
            special_effect: None,
        };

        // Test all attack types
        attack.attack_type = AttackType::Fire;
        assert_eq!(attack.attack_type, AttackType::Fire);

        attack.attack_type = AttackType::Cold;
        assert_eq!(attack.attack_type, AttackType::Cold);

        attack.attack_type = AttackType::Electricity;
        assert_eq!(attack.attack_type, AttackType::Electricity);

        attack.attack_type = AttackType::Acid;
        assert_eq!(attack.attack_type, AttackType::Acid);

        attack.attack_type = AttackType::Poison;
        assert_eq!(attack.attack_type, AttackType::Poison);

        attack.attack_type = AttackType::Energy;
        assert_eq!(attack.attack_type, AttackType::Energy);
    }

    #[test]
    fn test_monster_special_effects() {
        let mut attack = Attack {
            damage: DiceRoll::new(1, 6, 0),
            attack_type: AttackType::Physical,
            special_effect: None,
        };

        // Test all special effects
        let effects = vec![
            SpecialEffect::Poison,
            SpecialEffect::Disease,
            SpecialEffect::Paralysis,
            SpecialEffect::Sleep,
            SpecialEffect::Drain,
            SpecialEffect::Stone,
            SpecialEffect::Death,
        ];

        for effect in effects {
            attack.special_effect = Some(effect);
            assert_eq!(attack.special_effect, Some(effect));
        }

        attack.special_effect = None;
        assert!(attack.special_effect.is_none());
    }

    #[test]
    fn test_monster_loot_editor() {
        let mut app = CampaignBuilderApp {
            monsters_edit_buffer: CampaignBuilderApp::default_monster(),
            ..Default::default()
        };

        // Modify loot table
        app.monsters_edit_buffer.loot.gold_min = 10;
        app.monsters_edit_buffer.loot.gold_max = 50;
        app.monsters_edit_buffer.loot.gems_min = 0;
        app.monsters_edit_buffer.loot.gems_max = 2;
        app.monsters_edit_buffer.loot.experience = 150;

        assert_eq!(app.monsters_edit_buffer.loot.gold_min, 10);
        assert_eq!(app.monsters_edit_buffer.loot.gold_max, 50);
        assert_eq!(app.monsters_edit_buffer.loot.gems_min, 0);
        assert_eq!(app.monsters_edit_buffer.loot.gems_max, 2);
        assert_eq!(app.monsters_edit_buffer.loot.experience, 150);
    }

    #[test]
    fn test_monster_stats_editor() {
        let mut app = CampaignBuilderApp {
            monsters_edit_buffer: CampaignBuilderApp::default_monster(),
            ..Default::default()
        };

        // Modify all stats
        app.monsters_edit_buffer.stats.might.base = 20;
        app.monsters_edit_buffer.stats.intellect.base = 5;
        app.monsters_edit_buffer.stats.personality.base = 8;
        app.monsters_edit_buffer.stats.endurance.base = 18;
        app.monsters_edit_buffer.stats.speed.base = 12;
        app.monsters_edit_buffer.stats.accuracy.base = 15;
        app.monsters_edit_buffer.stats.luck.base = 6;

        assert_eq!(app.monsters_edit_buffer.stats.might.base, 20);
        assert_eq!(app.monsters_edit_buffer.stats.intellect.base, 5);
        assert_eq!(app.monsters_edit_buffer.stats.personality.base, 8);
        assert_eq!(app.monsters_edit_buffer.stats.endurance.base, 18);
        assert_eq!(app.monsters_edit_buffer.stats.speed.base, 12);
        assert_eq!(app.monsters_edit_buffer.stats.accuracy.base, 15);
        assert_eq!(app.monsters_edit_buffer.stats.luck.base, 6);
    }

    #[test]
    fn test_monster_xp_calculation_basic() {
        let app = CampaignBuilderApp::default();
        let monster = MonsterDefinition {
            id: 1,
            name: "Test Monster".to_string(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: 20,
            ac: 10,
            attacks: vec![Attack {
                damage: DiceRoll::new(1, 6, 0),
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
        };

        let xp = app.calculate_monster_xp(&monster);

        // Base: 20 HP * 10 = 200
        // + 1 attack * 20 = 20
        // + avg damage (1d6 = 3.5) * 5 = ~17
        // Total should be ~237
        assert!(xp >= 200);
        assert!(xp < 300);
    }

    #[test]
    fn test_monster_xp_calculation_with_abilities() {
        let app = CampaignBuilderApp::default();
        let monster = MonsterDefinition {
            id: 1,
            name: "Powerful Monster".to_string(),
            stats: Stats::new(20, 10, 10, 20, 15, 15, 10),
            hp: 50,
            ac: 5,
            attacks: vec![
                Attack {
                    damage: DiceRoll::new(2, 8, 5),
                    attack_type: AttackType::Fire,
                    special_effect: Some(SpecialEffect::Poison),
                },
                Attack {
                    damage: DiceRoll::new(1, 10, 3),
                    attack_type: AttackType::Physical,
                    special_effect: None,
                },
            ],
            flee_threshold: 10,
            special_attack_threshold: 25,
            resistances: MonsterResistances::new(),
            can_regenerate: true,
            can_advance: true,
            is_undead: true,
            magic_resistance: 50,
            loot: LootTable {
                gold_min: 50,
                gold_max: 200,
                gems_min: 1,
                gems_max: 5,
                items: Vec::new(),
                experience: 0,
            },
        };

        let xp = app.calculate_monster_xp(&monster);

        // Should have significant XP due to:
        // - High HP (50 * 10 = 500)
        // - Low AC bonus ((10 - 5) * 50 = 250)
        // - 2 attacks (2 * 20 = 40)
        // - High damage
        // - Special effect (+50)
        // - Regenerate (+100)
        // - Undead (+50)
        // - Magic resistance (50 * 2 = 100)
        assert!(xp >= 1000);
    }

    #[test]
    fn test_monster_import_export_roundtrip() {
        let original = MonsterDefinition {
            id: 42,
            name: "Test Monster".to_string(),
            stats: Stats::new(15, 12, 10, 14, 13, 11, 8),
            hp: 30,
            ac: 8,
            attacks: vec![Attack {
                damage: DiceRoll::new(2, 6, 2),
                attack_type: AttackType::Fire,
                special_effect: Some(SpecialEffect::Paralysis),
            }],
            flee_threshold: 5,
            special_attack_threshold: 20,
            resistances: MonsterResistances::new(),
            can_regenerate: true,
            can_advance: false,
            is_undead: true,
            magic_resistance: 25,
            loot: LootTable {
                gold_min: 10,
                gold_max: 50,
                gems_min: 0,
                gems_max: 2,
                items: Vec::new(),
                experience: 200,
            },
        };

        // Export to RON
        let ron_data = ron::to_string(&original).unwrap();

        // Import from RON
        let imported: MonsterDefinition = ron::from_str(&ron_data).unwrap();

        assert_eq!(imported.id, original.id);
        assert_eq!(imported.name, original.name);
        assert_eq!(imported.hp, original.hp);
        assert_eq!(imported.ac, original.ac);
        assert_eq!(imported.attacks.len(), original.attacks.len());
        assert_eq!(imported.can_regenerate, original.can_regenerate);
        assert_eq!(imported.can_advance, original.can_advance);
        assert_eq!(imported.is_undead, original.is_undead);
        assert_eq!(imported.magic_resistance, original.magic_resistance);
        assert_eq!(imported.loot.experience, original.loot.experience);
    }

    #[test]
    fn test_monster_preview_fields() {
        let app = CampaignBuilderApp::default();
        let monster = MonsterDefinition {
            id: 1,
            name: "Goblin".to_string(),
            stats: Stats::new(12, 8, 6, 10, 14, 10, 5),
            hp: 15,
            ac: 12,
            attacks: vec![Attack {
                damage: DiceRoll::new(1, 4, 1),
                attack_type: AttackType::Physical,
                special_effect: None,
            }],
            flee_threshold: 5,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: true,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable {
                gold_min: 1,
                gold_max: 10,
                gems_min: 0,
                gems_max: 0,
                items: Vec::new(),
                experience: 25,
            },
        };

        // Verify all preview fields exist
        assert_eq!(monster.name, "Goblin");
        assert_eq!(monster.hp, 15);
        assert_eq!(monster.ac, 12);
        assert_eq!(monster.attacks.len(), 1);
        assert!(!monster.is_undead);
        assert!(!monster.can_regenerate);
        assert!(monster.can_advance);
        assert_eq!(monster.loot.experience, 25);
    }

    #[test]
    fn test_spell_all_contexts() {
        // Test all spell contexts are available
        let contexts = vec![
            SpellContext::Anytime,
            SpellContext::CombatOnly,
            SpellContext::NonCombatOnly,
            SpellContext::OutdoorOnly,
            SpellContext::IndoorOnly,
            SpellContext::OutdoorCombat,
        ];

        for context in contexts {
            let spell = Spell::new(
                1,
                "Test",
                SpellSchool::Cleric,
                1,
                1,
                0,
                context,
                SpellTarget::Self_,
                "Test",
                None,
                0,
                false,
            );
            assert_eq!(spell.context, context);
        }
    }

    #[test]
    fn test_spell_all_targets() {
        // Test all spell targets are available
        let targets = vec![
            SpellTarget::Self_,
            SpellTarget::SingleCharacter,
            SpellTarget::AllCharacters,
            SpellTarget::SingleMonster,
            SpellTarget::MonsterGroup,
            SpellTarget::AllMonsters,
            SpellTarget::SpecificMonsters,
        ];

        for target in targets {
            let spell = Spell::new(
                1,
                "Test",
                SpellSchool::Cleric,
                1,
                1,
                0,
                SpellContext::Anytime,
                target,
                "Test",
                None,
                0,
                false,
            );
            assert_eq!(spell.target, target);
        }
    }

    // ============================================================
    // Phase 4A: Quest Editor Integration Tests
    // ============================================================

    #[test]
    fn test_quest_editor_state_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.quests.len(), 0);
        assert_eq!(app.quest_editor_state.selected_quest, None);
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::List
        );
        assert!(app.quests_show_preview);
        assert!(app.quest_editor_state.search_filter.is_empty());
    }

    #[test]
    fn test_quest_list_operations() {
        let mut app = CampaignBuilderApp::default();

        let quest1 = Quest::new(1, "Quest 1", "First quest");
        let quest2 = Quest::new(2, "Quest 2", "Second quest");
        app.quests.push(quest1);
        app.quests.push(quest2);

        assert_eq!(app.quests.len(), 2);
        assert_eq!(app.quests[0].name, "Quest 1");
        assert_eq!(app.quests[1].name, "Quest 2");
    }

    #[test]
    fn test_quest_search_filter() {
        let mut app = CampaignBuilderApp::default();

        let quest1 = Quest::new(1, "Dragon Slayer", "Kill the dragon");
        let quest2 = Quest::new(2, "Fetch Water", "Get water from the well");
        let quest3 = Quest::new(3, "Dragon Rider", "Tame a dragon");

        app.quests.push(quest1);
        app.quests.push(quest2);
        app.quests.push(quest3);
        app.quest_editor_state.quests = app.quests.clone();

        // Filter by "dragon"
        app.quest_editor_state.search_filter = "dragon".to_string();
        let filtered = app.quest_editor_state.filtered_quests();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|(_, q)| q.id == 1));
        assert!(filtered.iter().any(|(_, q)| q.id == 3));
    }

    #[test]
    fn test_next_available_quest_id() {
        let mut app = CampaignBuilderApp::default();

        assert_eq!(app.next_available_quest_id(), 1);

        app.quests.push(Quest::new(5, "Quest 5", "Description"));
        assert_eq!(app.next_available_quest_id(), 6);

        app.quests.push(Quest::new(10, "Quest 10", "Description"));
        assert_eq!(app.next_available_quest_id(), 11);
    }

    #[test]
    fn test_quest_with_stages() {
        let mut quest = Quest::new(1, "Multi-Stage Quest", "A quest with multiple stages");
        let stage1 = antares::domain::quest::QuestStage::new(1, "Stage 1");
        let stage2 = antares::domain::quest::QuestStage::new(2, "Stage 2");

        quest.add_stage(stage1);
        quest.add_stage(stage2);

        assert_eq!(quest.stages.len(), 2);
        assert_eq!(quest.stages[0].stage_number, 1);
        assert_eq!(quest.stages[1].stage_number, 2);
    }

    #[test]
    fn test_quest_with_rewards() {
        use antares::domain::quest::QuestReward;

        let mut quest = Quest::new(1, "Rewarding Quest", "A quest with rewards");
        quest.add_reward(QuestReward::Experience(1000));
        quest.add_reward(QuestReward::Gold(500));

        assert_eq!(quest.rewards.len(), 2);
        assert!(matches!(quest.rewards[0], QuestReward::Experience(1000)));
        assert!(matches!(quest.rewards[1], QuestReward::Gold(500)));
    }

    #[test]
    fn test_quest_level_requirements() {
        let mut quest = Quest::new(1, "Level Quest", "Quest with level requirements");
        quest.min_level = Some(5);
        quest.max_level = Some(15);

        assert_eq!(quest.min_level, Some(5));
        assert_eq!(quest.max_level, Some(15));
        assert!(quest.is_available_for_level(10));
        assert!(!quest.is_available_for_level(3));
        assert!(!quest.is_available_for_level(20));
    }

    #[test]
    fn test_quest_import_export_roundtrip() {
        use antares::domain::quest::QuestReward;

        let original = Quest::new(42, "Export Quest", "Quest for export testing");
        let mut quest = original.clone();
        quest.min_level = Some(10);
        quest.repeatable = true;
        quest.is_main_quest = true;
        quest.add_reward(QuestReward::Experience(2000));
        quest.add_reward(QuestReward::Gold(1000));

        // Export to RON
        let exported = ron::ser::to_string_pretty(&quest, Default::default());
        assert!(exported.is_ok());

        let ron_string = exported.unwrap();
        assert!(ron_string.contains("Export Quest"));
        assert!(ron_string.contains("repeatable: true"));

        // Import from RON
        let imported: Result<Quest, _> = ron::from_str(&ron_string);
        assert!(imported.is_ok());

        let quest_imported = imported.unwrap();
        assert_eq!(quest_imported.id, quest.id);
        assert_eq!(quest_imported.name, quest.name);
        assert_eq!(quest_imported.repeatable, quest.repeatable);
        assert_eq!(quest_imported.is_main_quest, quest.is_main_quest);
        assert_eq!(quest_imported.min_level, quest.min_level);
        assert_eq!(quest_imported.rewards.len(), 2);
    }

    #[test]
    fn test_quest_preview_toggle() {
        let mut app = CampaignBuilderApp::default();

        // Default is true
        assert!(app.quests_show_preview);

        app.quests_show_preview = false;
        assert!(!app.quests_show_preview);

        app.quests_show_preview = true;
        assert!(app.quests_show_preview);
    }

    #[test]
    fn test_quest_with_giver_location() {
        let mut quest = Quest::new(1, "NPC Quest", "Quest from an NPC");
        quest.quest_giver_npc = Some(100);
        quest.quest_giver_map = Some(5);
        quest.quest_giver_position = Some(antares::domain::types::Position::new(10, 20));

        assert_eq!(quest.quest_giver_npc, Some(100));
        assert_eq!(quest.quest_giver_map, Some(5));
        assert!(quest.quest_giver_position.is_some());
    }

    #[test]
    fn test_quest_repeatable_flag() {
        let mut quest1 = Quest::new(1, "One-Time Quest", "Can only do once");
        quest1.repeatable = false;

        let mut quest2 = Quest::new(2, "Daily Quest", "Can repeat daily");
        quest2.repeatable = true;

        assert!(!quest1.repeatable);
        assert!(quest2.repeatable);
    }

    #[test]
    fn test_quest_main_quest_flag() {
        let mut quest1 = Quest::new(1, "Main Story", "Part of main storyline");
        quest1.is_main_quest = true;

        let mut quest2 = Quest::new(2, "Side Story", "Optional side quest");
        quest2.is_main_quest = false;

        assert!(quest1.is_main_quest);
        assert!(!quest2.is_main_quest);
    }

    #[test]
    fn test_quest_import_buffer() {
        let app = CampaignBuilderApp::default();
        assert!(app.quests_import_buffer.is_empty());
        assert!(!app.quests_show_import_dialog);
    }

    #[test]
    fn test_quest_editor_mode_transitions() {
        let mut app = CampaignBuilderApp::default();

        // Start in list mode
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::List
        );

        // Transition to creating
        let next_id = app.next_available_quest_id();
        app.quest_editor_state.start_new_quest(next_id.to_string());
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::Creating
        );

        // Cancel back to list
        app.quest_editor_state.cancel_edit();
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::List
        );
    }

    // ============================================================
    // Phase 4B: Dialogue Editor Integration Tests
    // ============================================================

    #[test]
    fn test_dialogue_editor_state_initialization() {
        let app = CampaignBuilderApp::default();

        assert!(app.dialogues.is_empty());
        assert!(app.dialogue_editor_state.dialogues.is_empty());
        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::List
        );
        assert!(app.dialogues_search_filter.is_empty());
        assert!(!app.dialogues_show_preview);
        assert!(app.dialogues_import_buffer.is_empty());
        assert!(!app.dialogues_show_import_dialog);
    }

    #[test]
    fn test_dialogue_list_operations() {
        let mut app = CampaignBuilderApp::default();

        // Add dialogues
        let dialogue1 = DialogueTree::new(1, "Merchant Greeting", 1);
        let dialogue2 = DialogueTree::new(2, "Guard Warning", 1);

        app.dialogues.push(dialogue1);
        app.dialogues.push(dialogue2);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        assert_eq!(app.dialogue_editor_state.dialogues.len(), 2);
        assert_eq!(
            app.dialogue_editor_state.dialogues[0].name,
            "Merchant Greeting"
        );
        assert_eq!(app.dialogue_editor_state.dialogues[1].name, "Guard Warning");
    }

    #[test]
    fn test_dialogue_search_filter() {
        let mut app = CampaignBuilderApp::default();

        let dialogue1 = DialogueTree::new(1, "Merchant Greeting", 1);
        let dialogue2 = DialogueTree::new(2, "Guard Warning", 1);

        app.dialogues.push(dialogue1);
        app.dialogues.push(dialogue2);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        // No filter - all dialogues
        let filtered = app.dialogue_editor_state.filtered_dialogues();
        assert_eq!(filtered.len(), 2);

        // Filter by name
        app.dialogue_editor_state.search_filter = "merchant".to_string();
        let filtered = app.dialogue_editor_state.filtered_dialogues();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "Merchant Greeting");

        // Filter by ID
        app.dialogue_editor_state.search_filter = "2".to_string();
        let filtered = app.dialogue_editor_state.filtered_dialogues();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.id, 2);
    }

    #[test]
    fn test_dialogue_start_new() {
        let mut app = CampaignBuilderApp::default();

        app.dialogue_editor_state.start_new_dialogue();

        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::Creating
        );
        assert!(app.dialogue_editor_state.dialogue_buffer.id.is_empty());
        assert!(app.dialogue_editor_state.dialogue_buffer.name.is_empty());
        assert!(app.dialogue_editor_state.validation_errors.is_empty());
    }

    #[test]
    fn test_dialogue_edit_existing() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.speaker_name = Some("Merchant".to_string());
        dialogue.repeatable = true;

        app.dialogues.push(dialogue);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        app.dialogue_editor_state.start_edit_dialogue(0);

        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::Editing
        );
        assert_eq!(app.dialogue_editor_state.dialogue_buffer.id, "1");
        assert_eq!(
            app.dialogue_editor_state.dialogue_buffer.name,
            "Test Dialogue"
        );
        assert_eq!(
            app.dialogue_editor_state.dialogue_buffer.speaker_name,
            "Merchant"
        );
        assert!(app.dialogue_editor_state.dialogue_buffer.repeatable);
    }

    #[test]
    fn test_dialogue_save_new() {
        let mut app = CampaignBuilderApp::default();

        app.dialogue_editor_state.start_new_dialogue();
        app.dialogue_editor_state.dialogue_buffer.id = "10".to_string();
        app.dialogue_editor_state.dialogue_buffer.name = "New Dialogue".to_string();
        app.dialogue_editor_state.dialogue_buffer.speaker_name = "NPC".to_string();

        let result = app.dialogue_editor_state.save_dialogue();

        assert!(result.is_ok());
        assert_eq!(app.dialogue_editor_state.dialogues.len(), 1);
        assert_eq!(app.dialogue_editor_state.dialogues[0].id, 10);
        assert_eq!(app.dialogue_editor_state.dialogues[0].name, "New Dialogue");
        assert_eq!(
            app.dialogue_editor_state.dialogues[0].speaker_name,
            Some("NPC".to_string())
        );
        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::List
        );
    }

    #[test]
    fn test_dialogue_delete() {
        let mut app = CampaignBuilderApp::default();

        let dialogue1 = DialogueTree::new(1, "Dialogue 1", 1);
        let dialogue2 = DialogueTree::new(2, "Dialogue 2", 1);

        app.dialogues.push(dialogue1);
        app.dialogues.push(dialogue2);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        assert_eq!(app.dialogue_editor_state.dialogues.len(), 2);

        app.dialogue_editor_state.delete_dialogue(0);

        assert_eq!(app.dialogue_editor_state.dialogues.len(), 1);
        assert_eq!(app.dialogue_editor_state.dialogues[0].id, 2);
    }

    #[test]
    fn test_dialogue_cancel_edit() {
        let mut app = CampaignBuilderApp::default();

        app.dialogue_editor_state.start_new_dialogue();
        app.dialogue_editor_state.dialogue_buffer.name = "Test".to_string();

        app.dialogue_editor_state.cancel_edit();

        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::List
        );
        assert!(app.dialogue_editor_state.dialogue_buffer.name.is_empty());
        assert!(app.dialogue_editor_state.selected_dialogue.is_none());
    }

    #[test]
    fn test_dialogue_add_node() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let root_node = antares::domain::dialogue::DialogueNode::new(1, "Root node text");
        dialogue.add_node(root_node);

        app.dialogues.push(dialogue);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());
        app.dialogue_editor_state.start_edit_dialogue(0);

        // Add new node
        app.dialogue_editor_state.node_buffer.id = "2".to_string();
        app.dialogue_editor_state.node_buffer.text = "New node text".to_string();
        app.dialogue_editor_state.node_buffer.is_terminal = true;

        let result = app.dialogue_editor_state.add_node();

        assert!(result.is_ok());
        assert_eq!(app.dialogue_editor_state.dialogues[0].nodes.len(), 2);
        assert!(app.dialogue_editor_state.dialogues[0]
            .nodes
            .contains_key(&2));
    }

    #[test]
    fn test_dialogue_add_choice() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let root_node = antares::domain::dialogue::DialogueNode::new(1, "Root");
        let target_node = antares::domain::dialogue::DialogueNode::new(2, "Target");
        dialogue.add_node(root_node);
        dialogue.add_node(target_node);

        app.dialogues.push(dialogue);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());
        app.dialogue_editor_state.start_edit_dialogue(0);

        // Select node 1 to add choice to
        app.dialogue_editor_state.selected_node = Some(1);
        app.dialogue_editor_state.choice_buffer.text = "Go to node 2".to_string();
        app.dialogue_editor_state.choice_buffer.target_node = "2".to_string();

        let result = app.dialogue_editor_state.add_choice();

        assert!(result.is_ok());
        let node = app.dialogue_editor_state.dialogues[0]
            .nodes
            .get(&1)
            .unwrap();
        assert_eq!(node.choices.len(), 1);
        assert_eq!(node.choices[0].text, "Go to node 2");
    }

    #[test]
    fn test_dialogue_next_available_id() {
        let mut app = CampaignBuilderApp::default();

        // Empty list - should return 1
        assert_eq!(app.next_available_dialogue_id(), 1);

        // With dialogues - should return max + 1
        app.dialogues.push(DialogueTree::new(1, "D1", 1));
        app.dialogues.push(DialogueTree::new(5, "D2", 1));
        app.dialogues.push(DialogueTree::new(3, "D3", 1));

        assert_eq!(app.next_available_dialogue_id(), 6);
    }

    #[test]
    fn test_dialogue_import_export_roundtrip() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(42, "Test Export", 1);
        dialogue.speaker_name = Some("Merchant".to_string());
        dialogue.repeatable = false;

        let root_node = antares::domain::dialogue::DialogueNode::new(1, "Hello!");
        dialogue.add_node(root_node);

        app.dialogues.push(dialogue);

        // Export to RON
        let ron_str = ron::ser::to_string_pretty(&app.dialogues, Default::default()).unwrap();

        // Import back
        let imported: Vec<DialogueTree> = ron::from_str(&ron_str).unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].id, 42);
        assert_eq!(imported[0].name, "Test Export");
        assert_eq!(imported[0].speaker_name, Some("Merchant".to_string()));
        assert!(!imported[0].repeatable);
        assert_eq!(imported[0].nodes.len(), 1);
    }

    #[test]
    fn test_dialogue_editor_mode_transitions() {
        let mut app = CampaignBuilderApp::default();

        // Start in list mode
        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::List
        );

        // Transition to creating
        app.dialogue_editor_state.start_new_dialogue();
        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::Creating
        );

        // Cancel back to list
        app.dialogue_editor_state.cancel_edit();
        assert_eq!(
            app.dialogue_editor_state.mode,
            dialogue_editor::DialogueEditorMode::List
        );
    }

    #[test]
    fn test_dialogue_node_terminal_flag() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node = antares::domain::dialogue::DialogueNode::new(1, "Terminal node");
        node.is_terminal = true;
        dialogue.add_node(node);

        app.dialogues.push(dialogue);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        let node = app.dialogue_editor_state.dialogues[0]
            .nodes
            .get(&1)
            .unwrap();
        assert!(node.is_terminal);
    }

    #[test]
    fn test_dialogue_speaker_override() {
        let mut app = CampaignBuilderApp::default();

        let mut dialogue = DialogueTree::new(1, "Test", 1);
        dialogue.speaker_name = Some("Default Speaker".to_string());

        let mut node = antares::domain::dialogue::DialogueNode::new(1, "Override test");
        node.speaker_override = Some("Special Speaker".to_string());
        dialogue.add_node(node);

        app.dialogues.push(dialogue);
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        let node = app.dialogue_editor_state.dialogues[0]
            .nodes
            .get(&1)
            .unwrap();
        assert_eq!(node.speaker_override, Some("Special Speaker".to_string()));
    }

    #[test]
    fn test_dialogue_associated_quest() {
        let mut app = CampaignBuilderApp::default();

        app.dialogue_editor_state.start_new_dialogue();
        app.dialogue_editor_state.dialogue_buffer.id = "10".to_string();
        app.dialogue_editor_state.dialogue_buffer.name = "Quest Dialogue".to_string();
        app.dialogue_editor_state.dialogue_buffer.associated_quest = "5".to_string();

        let result = app.dialogue_editor_state.save_dialogue();

        assert!(result.is_ok());
        assert_eq!(
            app.dialogue_editor_state.dialogues[0].associated_quest,
            Some(5)
        );
    }

    #[test]
    fn test_dialogue_repeatable_flag() {
        let mut dialogue1 = DialogueTree::new(1, "One-time", 1);
        dialogue1.repeatable = false;

        let mut dialogue2 = DialogueTree::new(2, "Repeatable", 1);
        dialogue2.repeatable = true;

        assert!(!dialogue1.repeatable);
        assert!(dialogue2.repeatable);
    }

    #[test]
    fn test_dialogue_import_buffer() {
        let app = CampaignBuilderApp::default();
        assert!(app.dialogues_import_buffer.is_empty());
        assert!(!app.dialogues_show_import_dialog);
    }

    #[test]
    fn test_dialogue_validation_errors_cleared() {
        let mut app = CampaignBuilderApp::default();

        app.dialogue_editor_state
            .validation_errors
            .push("Test error".to_string());
        assert!(!app.dialogue_editor_state.validation_errors.is_empty());

        app.dialogue_editor_state.start_new_dialogue();
        assert!(app.dialogue_editor_state.validation_errors.is_empty());
    }
}
