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
mod dialogue_editor;
mod map_editor;
mod packager;
mod quest_editor;
mod templates;
mod test_play;
mod undo_redo;

use antares::domain::character::Stats;
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
use antares::domain::dialogue::DialogueTree;
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
use map_editor::{MapEditorState, MapEditorWidget};
use quest_editor::QuestEditorState;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Metadata,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
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
    items_search: String,
    items_selected: Option<usize>,
    items_editor_mode: EditorMode,
    items_edit_buffer: Item,
    // Phase 3B: Items editor enhancements
    items_filter_type: Option<ItemTypeFilter>,
    items_filter_magical: Option<bool>,
    items_filter_cursed: Option<bool>,
    items_filter_quest: Option<bool>,
    items_show_preview: bool,
    items_import_export_buffer: String,
    items_show_import_dialog: bool,

    spells: Vec<Spell>,
    spells_search: String,
    spells_selected: Option<usize>,
    spells_editor_mode: EditorMode,
    spells_edit_buffer: Spell,
    // Phase 3C: Spell filtering
    spells_filter_school: Option<SpellSchool>,
    spells_filter_level: Option<u8>,
    spells_show_preview: bool,
    spells_import_export_buffer: String,
    spells_show_import_dialog: bool,

    monsters: Vec<MonsterDefinition>,
    monsters_search: String,
    monsters_selected: Option<usize>,
    monsters_editor_mode: EditorMode,
    monsters_edit_buffer: MonsterDefinition,
    // Phase 3C: Monster editing enhancements
    monsters_show_preview: bool,
    monsters_show_attacks_editor: bool,
    monsters_show_loot_editor: bool,
    monsters_show_stats_editor: bool,
    monsters_import_export_buffer: String,
    monsters_show_import_dialog: bool,

    // Map editor state
    maps: Vec<Map>,
    maps_search: String,
    maps_selected: Option<usize>,
    maps_editor_mode: EditorMode,
    map_editor_state: Option<MapEditorState>,

    // Quest editor state
    quests: Vec<Quest>,
    quest_editor_state: QuestEditorState,

    // Dialogue editor state
    dialogues: Vec<DialogueTree>,
    dialogue_editor_state: DialogueEditorState,

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
            items_search: String::new(),
            items_selected: None,
            items_editor_mode: EditorMode::List,
            items_edit_buffer: Self::default_item(),
            items_filter_type: None,
            items_filter_magical: None,
            items_filter_cursed: None,
            items_filter_quest: None,
            items_show_preview: true,
            items_import_export_buffer: String::new(),
            items_show_import_dialog: false,

            spells: Vec::new(),
            spells_search: String::new(),
            spells_selected: None,
            spells_editor_mode: EditorMode::List,
            spells_edit_buffer: Self::default_spell(),
            spells_filter_school: None,
            spells_filter_level: None,
            spells_show_preview: true,
            spells_import_export_buffer: String::new(),
            spells_show_import_dialog: false,

            monsters: Vec::new(),
            monsters_search: String::new(),
            monsters_selected: None,
            monsters_editor_mode: EditorMode::List,
            monsters_edit_buffer: Self::default_monster(),
            monsters_show_preview: true,
            monsters_show_attacks_editor: false,
            monsters_show_loot_editor: false,
            monsters_show_stats_editor: false,
            monsters_import_export_buffer: String::new(),
            monsters_show_import_dialog: false,

            maps: Vec::new(),
            maps_search: String::new(),
            maps_selected: None,
            maps_editor_mode: EditorMode::List,
            map_editor_state: None,

            quests: Vec::new(),
            quest_editor_state: QuestEditorState::default(),

            dialogues: Vec::new(),
            dialogue_editor_state: DialogueEditorState::default(),

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
            .unwrap_or(0)
            .saturating_add(1)
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
                                            self.items_edit_buffer = item;
                                            self.items_editor_mode = EditorMode::Add;
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
                                            self.monsters_edit_buffer = monster;
                                            self.monsters_editor_mode = EditorMode::Add;
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
                        ui.close_menu();
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
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("📋 Template Browser...").clicked() {
                        self.show_template_browser = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("✅ Validate Campaign").clicked() {
                        self.validate_campaign();
                        self.active_tab = EditorTab::Validation;
                        ui.close_menu();
                    }
                    if ui.button("📊 Advanced Validation Report...").clicked() {
                        self.run_advanced_validation();
                        self.show_validation_report = true;
                        ui.close_menu();
                    }
                    if ui.button("⚖️ Balance Statistics...").clicked() {
                        self.show_balance_stats = true;
                        ui.close_menu();
                    }
                    ui.separator();
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
            EditorTab::Items => self.show_items_editor(ui),
            EditorTab::Spells => self.show_spells_editor(ui),
            EditorTab::Monsters => self.show_monsters_editor(ui),
            EditorTab::Maps => self.show_maps_editor(ui),
            EditorTab::Quests => self.show_quests_editor(ui),
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
                self.items_edit_buffer.id = self.next_available_item_id();
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_items();
            }

            if ui.button("📥 Import").clicked() {
                self.items_show_import_dialog = true;
                self.items_import_export_buffer.clear();
            }

            ui.separator();
            ui.label(format!("Total: {}", self.items.len()));
        });

        // Phase 3B: Filter toolbar
        ui.horizontal(|ui| {
            ui.label("Filters:");

            egui::ComboBox::from_id_salt("item_type_filter")
                .selected_text(
                    self.items_filter_type
                        .map(|f| f.as_str().to_string())
                        .unwrap_or_else(|| "All Types".to_string()),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.items_filter_type.is_none(), "All Types")
                        .clicked()
                    {
                        self.items_filter_type = None;
                    }
                    for filter in ItemTypeFilter::all() {
                        if ui
                            .selectable_value(
                                &mut self.items_filter_type,
                                Some(filter),
                                filter.as_str(),
                            )
                            .clicked()
                        {}
                    }
                });

            ui.separator();

            if ui
                .selectable_label(self.items_filter_magical == Some(true), "✨ Magical")
                .clicked()
            {
                self.items_filter_magical = if self.items_filter_magical == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            if ui
                .selectable_label(self.items_filter_cursed == Some(true), "💀 Cursed")
                .clicked()
            {
                self.items_filter_cursed = if self.items_filter_cursed == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            if ui
                .selectable_label(self.items_filter_quest == Some(true), "📜 Quest")
                .clicked()
            {
                self.items_filter_quest = if self.items_filter_quest == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            ui.separator();

            if ui.button("🔄 Clear Filters").clicked() {
                self.items_filter_type = None;
                self.items_filter_magical = None;
                self.items_filter_cursed = None;
                self.items_filter_quest = None;
            }
        });

        ui.separator();

        match self.items_editor_mode {
            EditorMode::List => self.show_items_list(ui),
            EditorMode::Add | EditorMode::Edit => self.show_items_form(ui),
        }

        // Phase 3B: Import dialog
        if self.items_show_import_dialog {
            self.show_item_import_dialog(ui.ctx());
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
                // Text search
                if !search_lower.is_empty() && !item.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                // Type filter
                if let Some(type_filter) = self.items_filter_type {
                    if !type_filter.matches(item) {
                        return false;
                    }
                }
                // Magical filter
                if let Some(magical) = self.items_filter_magical {
                    if item.is_magical() != magical {
                        return false;
                    }
                }
                // Cursed filter
                if let Some(cursed) = self.items_filter_cursed {
                    if item.is_cursed != cursed {
                        return false;
                    }
                }
                // Quest filter
                if let Some(quest) = self.items_filter_quest {
                    if item.is_quest_item() != quest {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, item)| {
                let mut label = format!("{}: {}", item.id, item.name);
                if item.is_magical() {
                    label.push_str(" ✨");
                }
                if item.is_cursed {
                    label.push_str(" 💀");
                }
                if item.is_quest_item() {
                    label.push_str(" 📜");
                }
                (idx, label)
            })
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
                            if ui.button("📤 Export").clicked() {
                                action = Some((idx, "export"));
                            }
                        });

                        ui.separator();

                        // Phase 3B: Enhanced preview panel
                        self.show_item_preview(ui, &item);
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
                    new_item.id = self.next_available_item_id();
                    new_item.name = format!("{} (Copy)", new_item.name);
                    self.items.push(new_item);
                    let _ = self.save_items();
                }
                "export" => {
                    // Export item to RON
                    if let Ok(ron_str) = ron::ser::to_string_pretty(
                        &self.items[idx],
                        ron::ser::PrettyConfig::default(),
                    ) {
                        self.items_import_export_buffer = ron_str;
                        self.items_show_import_dialog = true;
                        self.status_message = "Item exported to clipboard dialog".to_string();
                    } else {
                        self.status_message = "Failed to export item".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    /// Phase 3B: Show enhanced item preview panel
    ///
    /// Displays formatted item information with type-specific stats,
    /// class restrictions, bonuses, and effects.
    fn show_item_preview(&self, ui: &mut egui::Ui, item: &Item) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("Basic Info");
                ui.label(format!("ID: {}", item.id));
                ui.label(format!("Base Cost: {} gold", item.base_cost));
                ui.label(format!("Sell Cost: {} gold", item.sell_cost));

                let mut flags = Vec::new();
                if item.is_magical() {
                    flags.push("✨ Magical");
                }
                if item.is_cursed {
                    flags.push("💀 Cursed");
                }
                if item.is_quest_item() {
                    flags.push("📜 Quest Item");
                }
                if !flags.is_empty() {
                    ui.label(flags.join(" "));
                }
            });

            ui.add_space(5.0);

            // Type-specific display
            ui.group(|ui| {
                ui.heading("Item Type");
                match &item.item_type {
                    ItemType::Weapon(data) => {
                        ui.label("⚔️ Weapon");
                        ui.label(format!("  Damage: {:?}", data.damage));
                        ui.label(format!("  Bonus: {}", data.bonus));
                        ui.label(format!("  Hands: {}", data.hands_required));
                    }
                    ItemType::Armor(data) => {
                        ui.label("🛡️ Armor");
                        ui.label(format!("  AC Bonus: +{}", data.ac_bonus));
                        ui.label(format!("  Weight: {} lbs", data.weight));
                    }
                    ItemType::Accessory(data) => {
                        ui.label("💍 Accessory");
                        ui.label(format!("  Slot: {:?}", data.slot));
                    }
                    ItemType::Consumable(data) => {
                        ui.label("🧪 Consumable");
                        ui.label(format!("  Effect: {:?}", data.effect));
                        ui.label(format!("  Combat Use: {}", data.is_combat_usable));
                    }
                    ItemType::Ammo(data) => {
                        ui.label("🏹 Ammunition");
                        ui.label(format!("  Type: {:?}", data.ammo_type));
                        ui.label(format!("  Quantity: {}", data.quantity));
                    }
                    ItemType::Quest(data) => {
                        ui.label("📜 Quest Item");
                        ui.label(format!("  Quest: {}", data.quest_id));
                        ui.label(format!("  Key Item: {}", data.is_key_item));
                    }
                }
            });

            ui.add_space(5.0);

            // Disablements (class restrictions)
            ui.group(|ui| {
                ui.heading("Class Restrictions");
                self.show_disablement_display(ui, item.disablements);
            });

            // Bonuses and effects
            if item.constant_bonus.is_some()
                || item.temporary_bonus.is_some()
                || item.spell_effect.is_some()
            {
                ui.add_space(5.0);
                ui.group(|ui| {
                    ui.heading("Magical Effects");

                    if let Some(bonus) = item.constant_bonus {
                        ui.label(format!("Constant: {:?} {:+}", bonus.attribute, bonus.value));
                    }

                    if let Some(bonus) = item.temporary_bonus {
                        ui.label(format!(
                            "Temporary: {:?} {:+}",
                            bonus.attribute, bonus.value
                        ));
                    }

                    if let Some(spell_id) = item.spell_effect {
                        ui.label(format!("Spell Effect: ID {}", spell_id));
                    }

                    if item.max_charges > 0 {
                        ui.label(format!("Max Charges: {}", item.max_charges));
                    }
                });
            }
        });
    }

    /// Phase 3B: Display disablement flags (class restrictions)
    ///
    /// Shows which classes and alignments can use the item.
    fn show_disablement_display(&self, ui: &mut egui::Ui, disablement: Disablement) {
        ui.horizontal_wrapped(|ui| {
            let classes = [
                (Disablement::KNIGHT, "Knight"),
                (Disablement::PALADIN, "Paladin"),
                (Disablement::ARCHER, "Archer"),
                (Disablement::CLERIC, "Cleric"),
                (Disablement::SORCERER, "Sorcerer"),
                (Disablement::ROBBER, "Robber"),
            ];

            for (flag, name) in &classes {
                let can_use = disablement.can_use_class(*flag);
                if can_use {
                    ui.label(format!("✓ {}", name));
                } else {
                    ui.label(format!("✗ {}", name));
                }
            }
        });

        ui.horizontal(|ui| {
            if disablement.good_only() {
                ui.label("☀️ Good Only");
            }
            if disablement.evil_only() {
                ui.label("🌙 Evil Only");
            }
            if !disablement.good_only() && !disablement.evil_only() {
                ui.label("⚖️ Any Alignment");
            }
        });
    }

    /// Phase 3B: Import/export dialog
    fn show_item_import_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.items_show_import_dialog;

        egui::Window::new("Import/Export Item")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Item RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                let text_edit = egui::TextEdit::multiline(&mut self.items_import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Item>(&self.items_import_export_buffer) {
                            Ok(mut item) => {
                                item.id = self.next_available_item_id();
                                self.items.push(item);
                                let _ = self.save_items();
                                self.status_message = "Item imported successfully".to_string();
                                self.items_show_import_dialog = false;
                            }
                            Err(e) => {
                                self.status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.output_mut(|o| o.copied_text = self.items_import_export_buffer.clone());
                        self.status_message = "Copied to clipboard".to_string();
                    }

                    if ui.button("❌ Close").clicked() {
                        self.items_show_import_dialog = false;
                    }
                });
            });

        self.items_show_import_dialog = open;
    }

    /// Show items edit/add form
    fn show_items_form(&mut self, ui: &mut egui::Ui) {
        let is_add = self.items_editor_mode == EditorMode::Add;
        ui.heading(if is_add { "Add New Item" } else { "Edit Item" });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("Basic Properties");

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
                    ui.add(
                        egui::DragValue::new(&mut self.items_edit_buffer.max_charges).speed(1.0),
                    );
                });
            });

            ui.add_space(10.0);

            // Phase 3B: Item type selector and type-specific editor
            ui.group(|ui| {
                ui.heading("Item Type");

                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.items_edit_buffer.is_weapon(), "⚔️ Weapon")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type = ItemType::Weapon(WeaponData {
                            damage: DiceRoll::new(1, 6, 0),
                            bonus: 0,
                            hands_required: 1,
                        });
                    }
                    if ui
                        .selectable_label(self.items_edit_buffer.is_armor(), "🛡️ Armor")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type =
                            ItemType::Armor(antares::domain::items::types::ArmorData {
                                ac_bonus: 0,
                                weight: 0,
                            });
                    }
                    if ui
                        .selectable_label(self.items_edit_buffer.is_accessory(), "💍 Accessory")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type =
                            ItemType::Accessory(antares::domain::items::types::AccessoryData {
                                slot: antares::domain::items::types::AccessorySlot::Ring,
                            });
                    }
                    if ui
                        .selectable_label(self.items_edit_buffer.is_consumable(), "🧪 Consumable")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type =
                            ItemType::Consumable(antares::domain::items::types::ConsumableData {
                                effect: antares::domain::items::types::ConsumableEffect::HealHp(10),
                                is_combat_usable: true,
                            });
                    }
                    if ui
                        .selectable_label(self.items_edit_buffer.is_ammo(), "🏹 Ammo")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type =
                            ItemType::Ammo(antares::domain::items::types::AmmoData {
                                ammo_type: antares::domain::items::types::AmmoType::Arrow,
                                quantity: 20,
                            });
                    }
                    if ui
                        .selectable_label(self.items_edit_buffer.is_quest_item(), "📜 Quest")
                        .clicked()
                    {
                        self.items_edit_buffer.item_type =
                            ItemType::Quest(antares::domain::items::types::QuestData {
                                quest_id: String::new(),
                                is_key_item: false,
                            });
                    }
                });

                ui.separator();

                // Phase 3B: Type-specific editors
                self.show_item_type_editor(ui);
            });

            ui.add_space(10.0);

            // Phase 3B: Disablement editor
            ui.group(|ui| {
                ui.heading("Class Restrictions");
                self.show_disablement_editor(ui);
            });

            ui.add_space(10.0);

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

    /// Phase 3B: Type-specific item editor
    ///
    /// Displays appropriate editor fields based on the current item type.
    fn show_item_type_editor(&mut self, ui: &mut egui::Ui) {
        match &mut self.items_edit_buffer.item_type {
            ItemType::Weapon(data) => {
                ui.label("⚔️ Weapon Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Damage Dice:");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.count)
                            .range(1..=10)
                            .prefix("Count: "),
                    );
                    ui.label("d");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.sides)
                            .range(1..=100)
                            .prefix("Sides: "),
                    );
                    ui.label("+");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.bonus)
                            .range(-100..=100)
                            .prefix("Bonus: "),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("To-Hit/Damage Bonus:");
                    ui.add(egui::DragValue::new(&mut data.bonus).range(-10..=10));
                });

                ui.horizontal(|ui| {
                    ui.label("Hands Required:");
                    ui.add(egui::DragValue::new(&mut data.hands_required).range(1..=2));
                });
            }
            ItemType::Armor(data) => {
                ui.label("🛡️ Armor Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("AC Bonus:");
                    ui.add(egui::DragValue::new(&mut data.ac_bonus).range(0..=20));
                });

                ui.horizontal(|ui| {
                    ui.label("Weight (lbs):");
                    ui.add(egui::DragValue::new(&mut data.weight).range(0..=255));
                });
            }
            ItemType::Accessory(data) => {
                ui.label("💍 Accessory Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Equipment Slot:");
                    egui::ComboBox::from_id_salt("accessory_slot")
                        .selected_text(format!("{:?}", data.slot))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut data.slot, AccessorySlot::Ring, "Ring");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Amulet, "Amulet");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Belt, "Belt");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Cloak, "Cloak");
                        });
                });
            }
            ItemType::Consumable(data) => {
                ui.label("🧪 Consumable Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Effect Type:");

                    let effect_str = match &data.effect {
                        ConsumableEffect::HealHp(_) => "Heal HP",
                        ConsumableEffect::RestoreSp(_) => "Restore SP",
                        ConsumableEffect::CureCondition(_) => "Cure Condition",
                        ConsumableEffect::BoostAttribute(_, _) => "Boost Attribute",
                    };

                    egui::ComboBox::from_id_salt("consumable_effect")
                        .selected_text(effect_str)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::HealHp(_)),
                                    "Heal HP",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::HealHp(10);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::RestoreSp(_)),
                                    "Restore SP",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::RestoreSp(10);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::CureCondition(_)),
                                    "Cure Condition",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::CureCondition(0);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::BoostAttribute(_, _)),
                                    "Boost Attribute",
                                )
                                .clicked()
                            {
                                data.effect =
                                    ConsumableEffect::BoostAttribute(AttributeType::Might, 1);
                            }
                        });
                });

                // Edit effect value
                match &mut data.effect {
                    ConsumableEffect::HealHp(amount) | ConsumableEffect::RestoreSp(amount) => {
                        ui.horizontal(|ui| {
                            ui.label("Amount:");
                            ui.add(egui::DragValue::new(amount).range(1..=1000));
                        });
                    }
                    ConsumableEffect::CureCondition(flags) => {
                        ui.horizontal(|ui| {
                            ui.label("Condition Flags:");
                            ui.add(egui::DragValue::new(flags).range(0..=255));
                        });
                    }
                    ConsumableEffect::BoostAttribute(attr_type, value) => {
                        ui.horizontal(|ui| {
                            ui.label("Attribute:");
                            egui::ComboBox::from_id_salt("boost_attribute")
                                .selected_text(format!("{:?}", attr_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(attr_type, AttributeType::Might, "Might");
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Intellect,
                                        "Intellect",
                                    );
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Personality,
                                        "Personality",
                                    );
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Endurance,
                                        "Endurance",
                                    );
                                    ui.selectable_value(attr_type, AttributeType::Speed, "Speed");
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Accuracy,
                                        "Accuracy",
                                    );
                                    ui.selectable_value(attr_type, AttributeType::Luck, "Luck");
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Boost Amount:");
                            ui.add(egui::DragValue::new(value).range(-10..=10));
                        });
                    }
                }

                ui.checkbox(&mut data.is_combat_usable, "Usable in Combat");
            }
            ItemType::Ammo(data) => {
                ui.label("🏹 Ammunition Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Ammo Type:");
                    egui::ComboBox::from_id_salt("ammo_type")
                        .selected_text(format!("{:?}", data.ammo_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Arrow, "Arrow");
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Bolt, "Bolt");
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Stone, "Stone");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Quantity:");
                    ui.add(egui::DragValue::new(&mut data.quantity).range(1..=1000));
                });
            }
            ItemType::Quest(data) => {
                ui.label("📜 Quest Item Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Quest ID:");
                    ui.text_edit_singleline(&mut data.quest_id);
                });

                ui.checkbox(&mut data.is_key_item, "Key Item (Cannot drop/sell)");
            }
        }
    }

    /// Phase 3B: Disablement editor
    ///
    /// Provides checkboxes for editing class and alignment restrictions.
    fn show_disablement_editor(&mut self, ui: &mut egui::Ui) {
        let mut flags = self.items_edit_buffer.disablements.0;

        ui.label("Usable by:");
        ui.horizontal_wrapped(|ui| {
            let classes = [
                (Disablement::KNIGHT, "Knight"),
                (Disablement::PALADIN, "Paladin"),
                (Disablement::ARCHER, "Archer"),
                (Disablement::CLERIC, "Cleric"),
                (Disablement::SORCERER, "Sorcerer"),
                (Disablement::ROBBER, "Robber"),
            ];

            for (flag, name) in &classes {
                let mut enabled = (flags & flag) != 0;
                if ui.checkbox(&mut enabled, *name).changed() {
                    if enabled {
                        flags |= flag;
                    } else {
                        flags &= !flag;
                    }
                }
            }
        });

        ui.separator();
        ui.label("Alignment:");
        ui.horizontal(|ui| {
            let mut good = (flags & Disablement::GOOD) != 0;
            let mut evil = (flags & Disablement::EVIL) != 0;

            if ui.checkbox(&mut good, "☀️ Good Only").changed() {
                if good {
                    flags |= Disablement::GOOD;
                    flags &= !Disablement::EVIL; // Can't be both
                } else {
                    flags &= !Disablement::GOOD;
                }
            }

            if ui.checkbox(&mut evil, "🌙 Evil Only").changed() {
                if evil {
                    flags |= Disablement::EVIL;
                    flags &= !Disablement::GOOD; // Can't be both
                } else {
                    flags &= !Disablement::EVIL;
                }
            }
        });

        self.items_edit_buffer.disablements.0 = flags;

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("✓ All Classes").clicked() {
                self.items_edit_buffer.disablements.0 = 0b0011_1111; // All classes, no alignment restriction
            }
            if ui.button("✗ None").clicked() {
                self.items_edit_buffer.disablements.0 = 0;
            }
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

            // Phase 3C: School filter
            ui.label("School:");
            if ui
                .button(match self.spells_filter_school {
                    None => "All",
                    Some(SpellSchool::Cleric) => "Cleric",
                    Some(SpellSchool::Sorcerer) => "Sorcerer",
                })
                .clicked()
            {
                self.spells_filter_school = match self.spells_filter_school {
                    None => Some(SpellSchool::Cleric),
                    Some(SpellSchool::Cleric) => Some(SpellSchool::Sorcerer),
                    Some(SpellSchool::Sorcerer) => None,
                };
                self.spells_selected = None;
            }

            // Phase 3C: Level filter
            ui.label("Level:");
            egui::ComboBox::from_id_salt("spell_level_filter")
                .selected_text(match self.spells_filter_level {
                    None => "All".to_string(),
                    Some(lvl) => format!("{}", lvl),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.spells_filter_level.is_none(), "All")
                        .clicked()
                    {
                        self.spells_filter_level = None;
                        self.spells_selected = None;
                    }
                    for level in 1..=7 {
                        if ui
                            .selectable_label(
                                self.spells_filter_level == Some(level),
                                format!("{}", level),
                            )
                            .clicked()
                        {
                            self.spells_filter_level = Some(level);
                            self.spells_selected = None;
                        }
                    }
                });

            ui.separator();

            if ui.button("➕ Add Spell").clicked() {
                self.spells_editor_mode = EditorMode::Add;
                self.spells_edit_buffer = Self::default_spell();
                self.spells_edit_buffer.id = self.next_available_spell_id();
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_spells();
            }

            if ui.button("📥 Import").clicked() {
                self.spells_show_import_dialog = true;
            }

            ui.separator();
            ui.label(format!("Total: {}", self.spells.len()));

            ui.checkbox(&mut self.spells_show_preview, "Preview");
        });

        ui.separator();

        // Phase 3C: Import dialog
        if self.spells_show_import_dialog {
            self.show_spell_import_dialog(ui);
        }

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
                // Text search
                if !search_lower.is_empty() && !spell.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                // Phase 3C: School filter
                if let Some(school) = self.spells_filter_school {
                    if spell.school != school {
                        return false;
                    }
                }
                // Phase 3C: Level filter
                if let Some(level) = self.spells_filter_level {
                    if spell.level != level {
                        return false;
                    }
                }
                true
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
                            if ui.button("📤 Export").clicked() {
                                action = Some((idx, "export"));
                            }
                        });

                        ui.separator();

                        // Phase 3C: Spell preview
                        if self.spells_show_preview {
                            self.show_spell_preview(ui, &spell);
                        } else {
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
                    new_spell.id = self.next_available_spell_id();
                    new_spell.name = format!("{} (Copy)", new_spell.name);
                    self.spells.push(new_spell);
                    let _ = self.save_spells();
                }
                "export" => {
                    if let Ok(ron) = ron::to_string(&self.spells[idx]) {
                        self.spells_import_export_buffer = ron;
                        self.status_message = "Spell exported to buffer".to_string();
                    }
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

            // Phase 3C: Context editor
            ui.horizontal(|ui| {
                ui.label("Context:");
                egui::ComboBox::from_id_salt("spell_context")
                    .selected_text(format!("{:?}", self.spells_edit_buffer.context))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::Anytime,
                            "Anytime",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::CombatOnly,
                            "CombatOnly",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::NonCombatOnly,
                            "NonCombatOnly",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::OutdoorOnly,
                            "OutdoorOnly",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::IndoorOnly,
                            "IndoorOnly",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.context,
                            SpellContext::OutdoorCombat,
                            "OutdoorCombat",
                        );
                    });
            });

            // Phase 3C: Target editor
            ui.horizontal(|ui| {
                ui.label("Target:");
                egui::ComboBox::from_id_salt("spell_target")
                    .selected_text(format!("{:?}", self.spells_edit_buffer.target))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::Self_,
                            "Self",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::SingleCharacter,
                            "SingleCharacter",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::AllCharacters,
                            "AllCharacters",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::SingleMonster,
                            "SingleMonster",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::MonsterGroup,
                            "MonsterGroup",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::AllMonsters,
                            "AllMonsters",
                        );
                        ui.selectable_value(
                            &mut self.spells_edit_buffer.target,
                            SpellTarget::SpecificMonsters,
                            "SpecificMonsters",
                        );
                    });
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
                self.monsters_edit_buffer.id = self.next_available_monster_id();
            }

            if ui.button("🔄 Reload").clicked() {
                self.load_monsters();
            }

            if ui.button("📥 Import").clicked() {
                self.monsters_show_import_dialog = true;
            }

            ui.separator();
            ui.label(format!("Total: {}", self.monsters.len()));

            ui.checkbox(&mut self.monsters_show_preview, "Preview");
        });

        ui.separator();

        // Phase 3C: Import dialog
        if self.monsters_show_import_dialog {
            self.show_monster_import_dialog(ui);
        }

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
                            if ui.button("📤 Export").clicked() {
                                action = Some((idx, "export"));
                            }
                        });

                        ui.separator();

                        // Phase 3C: Monster preview
                        if self.monsters_show_preview {
                            self.show_monster_preview(ui, &monster);
                        } else {
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
                                    ui.label(format!(
                                        "  Experience: {} XP",
                                        monster.loot.experience
                                    ));
                                });
                            });
                        }
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
                    new_monster.id = self.next_available_monster_id();
                    new_monster.name = format!("{} (Copy)", new_monster.name);
                    self.monsters.push(new_monster);
                    let _ = self.save_monsters();
                }
                "export" => {
                    if let Ok(ron) = ron::to_string(&self.monsters[idx]) {
                        self.monsters_import_export_buffer = ron;
                        self.status_message = "Monster exported to buffer".to_string();
                    }
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
            ui.checkbox(&mut self.monsters_edit_buffer.can_advance, "Can Advance");

            ui.horizontal(|ui| {
                ui.label("Magic Resistance:");
                ui.add(egui::Slider::new(
                    &mut self.monsters_edit_buffer.magic_resistance,
                    0..=100,
                ));
            });

            // Phase 3C: Stats editor toggle
            ui.separator();
            if ui
                .button(if self.monsters_show_stats_editor {
                    "▼ Stats"
                } else {
                    "▶ Stats"
                })
                .clicked()
            {
                self.monsters_show_stats_editor = !self.monsters_show_stats_editor;
            }

            if self.monsters_show_stats_editor {
                ui.group(|ui| {
                    self.show_monster_stats_editor(ui);
                });
            }

            // Phase 3C: Attacks editor toggle
            ui.separator();
            if ui
                .button(if self.monsters_show_attacks_editor {
                    "▼ Attacks"
                } else {
                    "▶ Attacks"
                })
                .clicked()
            {
                self.monsters_show_attacks_editor = !self.monsters_show_attacks_editor;
            }

            if self.monsters_show_attacks_editor {
                ui.group(|ui| {
                    self.show_monster_attacks_editor(ui);
                });
            }

            // Phase 3C: Loot editor toggle
            ui.separator();
            if ui
                .button(if self.monsters_show_loot_editor {
                    "▼ Loot Table"
                } else {
                    "▶ Loot Table"
                })
                .clicked()
            {
                self.monsters_show_loot_editor = !self.monsters_show_loot_editor;
            }

            if self.monsters_show_loot_editor {
                ui.group(|ui| {
                    self.show_monster_loot_editor(ui);
                });
            }

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

    /// Phase 3C: Show spell preview panel with formatted information
    fn show_spell_preview(&self, ui: &mut egui::Ui, spell: &Spell) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading(&spell.name);
                ui.separator();

                ui.label(format!("🆔 ID: {}", spell.id));

                let school_icon = match spell.school {
                    SpellSchool::Cleric => "✝️",
                    SpellSchool::Sorcerer => "🔮",
                };
                ui.label(format!("{} School: {:?}", school_icon, spell.school));
                ui.label(format!("📊 Level: {}", spell.level));

                ui.separator();

                ui.label(format!("💎 SP Cost: {}", spell.sp_cost));
                if spell.gem_cost > 0 {
                    ui.label(format!("💎 Gem Cost: {}", spell.gem_cost));
                }

                ui.separator();

                ui.label(format!("🎯 Target: {:?}", spell.target));
                ui.label(format!("📍 Context: {:?}", spell.context));

                ui.separator();

                ui.label("📝 Description:");
                ui.label(&spell.description);
            });
        });
    }

    /// Phase 3C: Show spell import dialog
    fn show_spell_import_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Import Spell")
            .default_width(600.0)
            .show(ui.ctx(), |ui| {
                ui.label("Paste RON spell data:");
                ui.text_edit_multiline(&mut self.spells_import_export_buffer);

                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        match ron::from_str::<Spell>(&self.spells_import_export_buffer) {
                            Ok(mut spell) => {
                                spell.id = self.next_available_spell_id();
                                self.spells.push(spell);
                                let _ = self.save_spells();
                                self.spells_import_export_buffer.clear();
                                self.spells_show_import_dialog = false;
                                self.status_message = "Spell imported successfully".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.spells_show_import_dialog = false;
                    }
                });
            });
    }

    /// Phase 3C: Show monster preview panel with formatted information
    fn show_monster_preview(&self, ui: &mut egui::Ui, monster: &MonsterDefinition) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading(&monster.name);
                ui.separator();

                ui.label(format!("🆔 ID: {}", monster.id));

                let type_icon = if monster.is_undead { "💀" } else { "👹" };
                ui.label(format!(
                    "{} Type: {}",
                    type_icon,
                    if monster.is_undead {
                        "Undead"
                    } else {
                        "Living"
                    }
                ));

                ui.separator();

                ui.label("⚔️ Combat Stats:");
                ui.label(format!("  ❤️ HP: {}", monster.hp));
                ui.label(format!("  🛡️ AC: {}", monster.ac));
                ui.label(format!("  ⚡ Attacks: {}", monster.attacks.len()));

                if monster.magic_resistance > 0 {
                    ui.label(format!(
                        "  🔮 Magic Resistance: {}%",
                        monster.magic_resistance
                    ));
                }

                ui.separator();

                ui.label("🎲 Attributes:");
                ui.label(format!("  Might: {}", monster.stats.might.base));
                ui.label(format!("  Intellect: {}", monster.stats.intellect.base));
                ui.label(format!("  Personality: {}", monster.stats.personality.base));
                ui.label(format!("  Endurance: {}", monster.stats.endurance.base));
                ui.label(format!("  Speed: {}", monster.stats.speed.base));
                ui.label(format!("  Accuracy: {}", monster.stats.accuracy.base));
                ui.label(format!("  Luck: {}", monster.stats.luck.base));

                ui.separator();

                ui.label("⚙️ Special Abilities:");
                if monster.can_regenerate {
                    ui.label("  ♻️ Can Regenerate");
                }
                if monster.can_advance {
                    ui.label("  🏃 Can Advance");
                }

                ui.separator();

                ui.label("💰 Loot:");
                if monster.loot.gold_max > 0 {
                    ui.label(format!(
                        "  Gold: {}-{} gp",
                        monster.loot.gold_min, monster.loot.gold_max
                    ));
                }
                if monster.loot.gems_max > 0 {
                    ui.label(format!(
                        "  Gems: {}-{}",
                        monster.loot.gems_min, monster.loot.gems_max
                    ));
                }
                ui.label(format!("  Experience: {} XP", monster.loot.experience));
            });
        });
    }

    /// Phase 3C: Show monster import dialog
    fn show_monster_import_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Import Monster")
            .default_width(600.0)
            .show(ui.ctx(), |ui| {
                ui.label("Paste RON monster data:");
                ui.text_edit_multiline(&mut self.monsters_import_export_buffer);

                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        match ron::from_str::<MonsterDefinition>(
                            &self.monsters_import_export_buffer,
                        ) {
                            Ok(mut monster) => {
                                monster.id = self.next_available_monster_id();
                                self.monsters.push(monster);
                                let _ = self.save_monsters();
                                self.monsters_import_export_buffer.clear();
                                self.monsters_show_import_dialog = false;
                                self.status_message = "Monster imported successfully".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.monsters_show_import_dialog = false;
                    }
                });
            });
    }

    /// Phase 3C: Show monster stats editor
    fn show_monster_stats_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Attributes:");

        ui.horizontal(|ui| {
            ui.label("Might:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.might.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Intellect:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.intellect.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Personality:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.personality.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Endurance:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.endurance.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Speed:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.speed.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Accuracy:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.accuracy.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Luck:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.stats.luck.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Flee Threshold:");
            ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.flee_threshold).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Special Attack %:");
            ui.add(egui::Slider::new(
                &mut self.monsters_edit_buffer.special_attack_threshold,
                0..=100,
            ));
        });
    }

    /// Phase 3C: Show monster attacks editor
    fn show_monster_attacks_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Attacks ({})",
            self.monsters_edit_buffer.attacks.len()
        ));

        if ui.button("➕ Add Attack").clicked() {
            self.monsters_edit_buffer.attacks.push(Attack {
                damage: DiceRoll::new(1, 6, 0),
                attack_type: AttackType::Physical,
                special_effect: None,
            });
        }

        ui.separator();

        let mut to_remove: Option<usize> = None;

        for (idx, attack) in self.monsters_edit_buffer.attacks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Attack {}", idx + 1));
                    if ui.button("🗑️").clicked() {
                        to_remove = Some(idx);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Damage:");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.count)
                            .speed(1.0)
                            .range(1..=10)
                            .prefix("d"),
                    );
                    ui.label("d");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.sides)
                            .speed(1.0)
                            .range(2..=100),
                    );
                    ui.label("+");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.bonus)
                            .speed(1.0)
                            .range(-10..=100),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt(format!("attack_type_{}", idx))
                        .selected_text(format!("{:?}", attack.attack_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Physical,
                                "Physical",
                            );
                            ui.selectable_value(&mut attack.attack_type, AttackType::Fire, "Fire");
                            ui.selectable_value(&mut attack.attack_type, AttackType::Cold, "Cold");
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Electricity,
                                "Electricity",
                            );
                            ui.selectable_value(&mut attack.attack_type, AttackType::Acid, "Acid");
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Poison,
                                "Poison",
                            );
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Energy,
                                "Energy",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Special Effect:");
                    egui::ComboBox::from_id_salt(format!("special_effect_{}", idx))
                        .selected_text(match attack.special_effect {
                            None => "None",
                            Some(SpecialEffect::Poison) => "Poison",
                            Some(SpecialEffect::Disease) => "Disease",
                            Some(SpecialEffect::Paralysis) => "Paralysis",
                            Some(SpecialEffect::Sleep) => "Sleep",
                            Some(SpecialEffect::Drain) => "Drain",
                            Some(SpecialEffect::Stone) => "Stone",
                            Some(SpecialEffect::Death) => "Death",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut attack.special_effect, None, "None");
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Poison),
                                "Poison",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Disease),
                                "Disease",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Paralysis),
                                "Paralysis",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Sleep),
                                "Sleep",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Drain),
                                "Drain",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Stone),
                                "Stone",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Death),
                                "Death",
                            );
                        });
                });
            });
        }

        if let Some(idx) = to_remove {
            self.monsters_edit_buffer.attacks.remove(idx);
        }
    }

    /// Phase 3C: Show monster loot editor
    fn show_monster_loot_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Loot Table:");

        ui.horizontal(|ui| {
            ui.label("Gold Min:");
            ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gold_min).speed(1.0));
            ui.label("Max:");
            ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gold_max).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Gems Min:");
            ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gems_min).speed(1.0));
            ui.label("Max:");
            ui.add(egui::DragValue::new(&mut self.monsters_edit_buffer.loot.gems_max).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Experience:");
            ui.add(
                egui::DragValue::new(&mut self.monsters_edit_buffer.loot.experience).speed(10.0),
            );
        });

        ui.separator();

        // Phase 3C: Calculate recommended XP based on monster stats
        let calculated_xp = self.calculate_monster_xp(&self.monsters_edit_buffer);
        ui.label(format!(
            "💡 Suggested XP: {} (based on stats)",
            calculated_xp
        ));

        if ui.button("Use Suggested XP").clicked() {
            self.monsters_edit_buffer.loot.experience = calculated_xp;
        }
    }

    /// Phase 3C: Calculate recommended XP for a monster based on stats
    fn calculate_monster_xp(&self, monster: &MonsterDefinition) -> u32 {
        let mut xp = monster.hp as u32 * 10;

        // Factor in AC (higher AC = harder to hit)
        if monster.ac < 10 {
            xp += (10 - monster.ac as u32) * 50;
        }

        // Factor in number of attacks
        xp += monster.attacks.len() as u32 * 20;

        // Factor in attack damage
        for attack in &monster.attacks {
            let avg_damage = (attack.damage.count as f32 * (attack.damage.sides as f32 / 2.0))
                + attack.damage.bonus as f32;
            xp += (avg_damage * 5.0) as u32;

            // Bonus for special effects
            if attack.special_effect.is_some() {
                xp += 50;
            }
        }

        // Factor in special abilities
        if monster.can_regenerate {
            xp += 100;
        }
        if monster.is_undead {
            xp += 50;
        }
        if monster.magic_resistance > 0 {
            xp += monster.magic_resistance as u32 * 2;
        }

        xp
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

    /// Show dialogues editor
    fn show_dialogues_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("💬 Dialogues Editor");
        ui.add_space(5.0);
        ui.label("Manage NPC dialogues and conversation trees");
        ui.separator();

        ui.label("Dialogue editor integration in progress");
        ui.separator();

        // Display dialogue count
        ui.label(format!("Dialogues loaded: {}", self.dialogues.len()));

        // TODO: Integrate DialogueEditorWidget when UI components are ready
        // For now, show basic list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, dialogue) in self.dialogues.iter().enumerate() {
                ui.group(|ui| {
                    ui.label(format!(
                        "Dialogue {}: {} ({} nodes)",
                        idx,
                        dialogue.id,
                        dialogue.nodes.len()
                    ));
                });
            }
        });
    }

    /// Save dialogues to file
    fn save_dialogues_to_file(&self, path: &std::path::Path) -> Result<(), CampaignError> {
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
            // Asset statistics
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

            // Asset list
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (path, asset) in manager.assets() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("📄 {}", path.display()));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(asset.size_string());
                                    ui.label(asset.asset_type.display_name());
                                    if !asset.is_referenced {
                                        ui.colored_label(egui::Color32::YELLOW, "⚠ Unused");
                                    }
                                },
                            );
                        });
                    });
                }
            });

            ui.separator();

            // Unreferenced assets warning
            let unreferenced = manager.unreferenced_assets();
            if !unreferenced.is_empty() {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("⚠ {} unreferenced assets found", unreferenced.len()),
                );
            }
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
        let map1 = Map::new(10, 20, 20);
        app.maps.push(map1);

        let map2 = Map::new(10, 30, 30); // Duplicate ID
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

        let map1 = Map::new(5, 20, 20);
        app.maps.push(map1);

        let map2 = Map::new(8, 30, 30);
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
        };

        assert!(ItemTypeFilter::Weapon.matches(&weapon_item));
        assert!(!ItemTypeFilter::Weapon.matches(&armor_item));
        assert!(ItemTypeFilter::Armor.matches(&armor_item));
        assert!(!ItemTypeFilter::Armor.matches(&weapon_item));
    }

    #[test]
    fn test_items_filter_magical() {
        let mut app = CampaignBuilderApp::default();
        app.items_filter_magical = Some(true);

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
        ));

        // Apply Cleric filter
        app.spells_filter_school = Some(SpellSchool::Cleric);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| app.spells_filter_school.map_or(true, |f| s.school == f))
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
        ));

        // Filter level 3 spells
        app.spells_filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| app.spells_filter_level.map_or(true, |f| s.level == f))
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
        ));

        // Filter: Cleric + Level 3
        app.spells_filter_school = Some(SpellSchool::Cleric);
        app.spells_filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| {
                app.spells_filter_school.map_or(true, |f| s.school == f)
                    && app.spells_filter_level.map_or(true, |f| s.level == f)
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Cure Disease");
        assert_eq!(filtered[0].school, SpellSchool::Cleric);
        assert_eq!(filtered[0].level, 3);
    }

    #[test]
    fn test_spell_context_target_editing() {
        let mut app = CampaignBuilderApp::default();
        app.spells_edit_buffer = Spell::new(
            1,
            "Test",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
        );

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
        let mut app = CampaignBuilderApp::default();
        app.monsters_edit_buffer = CampaignBuilderApp::default_monster();

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
        let mut app = CampaignBuilderApp::default();
        app.monsters_edit_buffer = CampaignBuilderApp::default_monster();

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
        let mut app = CampaignBuilderApp::default();
        app.monsters_edit_buffer = CampaignBuilderApp::default_monster();

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
            );
            assert_eq!(spell.target, target);
        }
    }
}
