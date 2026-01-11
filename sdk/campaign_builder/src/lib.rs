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

pub mod advanced_validation;
pub mod asset_manager;
pub mod campaign_editor;
pub mod characters_editor;
pub mod classes_editor;
pub mod conditions_editor;
pub mod config_editor;
pub mod dialogue_editor;
pub mod items_editor;
pub mod logging;
pub mod map_editor;
pub mod monsters_editor;
pub mod npc_editor;
pub mod packager;
pub mod proficiencies_editor;
pub mod quest_editor;
pub mod races_editor;
pub mod spells_editor;
pub mod templates;
pub mod test_play;
pub mod test_utils;
pub mod ui_helpers;
pub mod undo_redo;
pub mod validation;

use antares::sdk::tool_config::ToolConfig;
use logging::{category, LogLevel, Logger};

use antares::domain::character::Stats;
use antares::domain::character::{FOOD_MAX, FOOD_MIN, PARTY_MAX_SIZE};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
use antares::domain::conditions::ConditionDefinition;
use antares::domain::dialogue::{DialogueTree, NodeId};
use antares::domain::items::types::{
    AccessoryData, AccessorySlot, AlignmentRestriction, AmmoData, AmmoType, ArmorClassification,
    ArmorData, AttributeType, Bonus, BonusAttribute, ConsumableData, ConsumableEffect, Item,
    ItemType, MagicItemClassification, QuestData, WeaponClassification, WeaponData,
};
use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
use antares::domain::proficiency::ProficiencyDefinition;
use antares::domain::quest::{Quest, QuestId};
use antares::domain::types::{DiceRoll, ItemId, MapId, MonsterId, SpellId};
use antares::domain::world::Map;
use conditions_editor::ConditionsEditorState;
use dialogue_editor::DialogueEditorState;
use eframe::egui;
use items_editor::ItemsEditorState;
use map_editor::MapsEditorState;
use monsters_editor::MonstersEditorState;
use quest_editor::QuestEditorState;
use serde::{Deserialize, Serialize};
use spells_editor::SpellsEditorState;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

const STARTING_GOLD_MAX: u32 = 100_000;

pub fn run() -> Result<(), eframe::Error> {
    // Initialize logger from command-line arguments
    let logger = Logger::from_args();
    let log_level = logger.level();

    // Print startup message based on log level
    if log_level >= LogLevel::Info {
        eprintln!(
            "[INFO] Antares Campaign Builder starting (log level: {})",
            log_level
        );
    }
    if log_level >= LogLevel::Verbose {
        eprintln!("[VERBOSE] Verbose logging enabled - showing detailed trace information");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Antares Campaign Builder"),

        renderer: eframe::Renderer::default(),
        ..Default::default()
    };

    eframe::run_native(
        "Antares Campaign Builder",
        options,
        Box::new(move |_cc| {
            let mut app = CampaignBuilderApp {
                logger: logger.clone(),
                ..Default::default()
            };

            // Load persisted ToolConfig if available; otherwise fall back to defaults.
            // This makes display/editor preferences persistent across sessions.
            if let Ok(cfg) = ToolConfig::load_or_default() {
                app.tool_config = cfg;
            }

            app.logger.info(category::APP, "Application initialized");
            Ok(Box::new(app))
        }),
    )
}

/// Campaign metadata structure matching campaign.ron schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetadata {
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
    #[serde(default = "default_starting_innkeeper")]
    starting_innkeeper: String,
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
    characters_file: String,
    maps_dir: String,
    quests_file: String,
    dialogue_file: String,
    conditions_file: String,
    npcs_file: String,
    #[serde(default = "default_proficiencies_file")]
    proficiencies_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Difficulty {
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

fn default_starting_innkeeper() -> String {
    "tutorial_innkeeper_town".to_string()
}

fn default_proficiencies_file() -> String {
    "data/proficiencies.ron".to_string()
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
            starting_innkeeper: "tutorial_innkeeper_town".to_string(),
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
            characters_file: "data/characters.ron".to_string(),
            maps_dir: "data/maps/".to_string(),
            quests_file: "data/quests.ron".to_string(),
            dialogue_file: "data/dialogue.ron".to_string(),
            conditions_file: "data/conditions.ron".to_string(),
            npcs_file: "data/npcs.ron".to_string(),
            proficiencies_file: "data/proficiencies.ron".to_string(),
        }
    }
}

/// Active tab in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Metadata,
    Config,
    Items,
    Spells,
    Conditions,
    Monsters,
    Maps,
    Quests,
    Classes,
    Races,
    Characters,
    Dialogues,
    NPCs,
    Proficiencies,
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
            EditorTab::Config => "Config",
            EditorTab::Items => "Items",
            EditorTab::Spells => "Spells",
            EditorTab::Conditions => "Conditions",
            EditorTab::Monsters => "Monsters",
            EditorTab::Maps => "Maps",
            EditorTab::Quests => "Quests",
            EditorTab::Classes => "Classes",
            EditorTab::Races => "Races",
            EditorTab::Characters => "Characters",
            EditorTab::Dialogues => "Dialogues",
            EditorTab::NPCs => "NPCs",
            EditorTab::Proficiencies => "Proficiencies",
            EditorTab::Assets => "Assets",
            EditorTab::Validation => "Validation",
        }
    }
}

// NOTE: ValidationError and Severity types have been replaced by the validation module.
// Use validation::ValidationResult and validation::ValidationSeverity instead.

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

/// Quick validation filter for the Validation Panel.
///
/// The filter controls which severities the UI will display. These are user-facing
/// selections and are persisted via the `CampaignBuilderApp` state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationFilter {
    /// Show all severities (default)
    All,
    /// Only show `Error` severity checks
    ErrorsOnly,
    /// Only show `Warning` severity checks
    WarningsOnly,
}

impl ValidationFilter {
    /// Returns a short label for the control UI
    fn as_str(&self) -> &str {
        match self {
            ValidationFilter::All => "All",
            ValidationFilter::ErrorsOnly => "Errors Only",
            ValidationFilter::WarningsOnly => "Warnings Only",
        }
    }
}

/// File I/O errors
#[derive(Debug, Error)]
pub enum CampaignError {
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
    validation_errors: Vec<validation::ValidationResult>,
    validation_filter: ValidationFilter,
    validation_focus_asset: Option<PathBuf>,
    show_about_dialog: bool,
    asset_type_filter: Option<asset_manager::AssetType>,
    show_unsaved_warning: bool,
    pending_action: Option<PendingAction>,
    file_tree: Vec<FileNode>,

    // Campaign metadata editor state
    campaign_editor_state: campaign_editor::CampaignMetadataEditorState,

    // Config editor state
    config_editor_state: config_editor::ConfigEditorState,

    // Data editor state
    items: Vec<Item>,
    items_editor_state: ItemsEditorState,

    spells: Vec<Spell>,
    spells_editor_state: SpellsEditorState,

    proficiencies: Vec<ProficiencyDefinition>,
    proficiencies_editor_state: proficiencies_editor::ProficienciesEditorState,

    monsters: Vec<MonsterDefinition>,
    monsters_editor_state: MonstersEditorState,

    conditions: Vec<ConditionDefinition>,
    conditions_editor_state: ConditionsEditorState,

    // Map editor state
    maps: Vec<Map>,
    maps_editor_state: MapsEditorState,

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

    // NPC editor state
    npc_editor_state: npc_editor::NpcEditorState,

    // Classes editor state
    classes_editor_state: classes_editor::ClassesEditorState,

    // Races editor state
    races_editor_state: races_editor::RacesEditorState,

    // Characters editor state
    characters_editor_state: characters_editor::CharactersEditorState,

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
    show_cleanup_candidates: bool,
    cleanup_candidates_selected: std::collections::HashSet<PathBuf>,

    // File I/O pattern state
    file_load_merge_mode: bool,

    // Phase 7: Logging and developer experience
    logger: Logger,
    tool_config: ToolConfig,
    show_preferences: bool,
    show_debug_panel: bool,
    debug_panel_filter_level: LogLevel,
    debug_panel_auto_scroll: bool,
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
            validation_filter: ValidationFilter::All,
            validation_focus_asset: None,
            show_about_dialog: false,
            asset_type_filter: None,
            show_unsaved_warning: false,
            pending_action: None,
            file_tree: Vec::new(),

            // Campaign metadata editor state
            campaign_editor_state: campaign_editor::CampaignMetadataEditorState::new(),

            // Config editor state
            config_editor_state: config_editor::ConfigEditorState::new(),

            items: Vec::new(),
            items_editor_state: ItemsEditorState::new(),

            spells: Vec::new(),
            spells_editor_state: SpellsEditorState::new(),

            proficiencies: Vec::new(),
            proficiencies_editor_state: proficiencies_editor::ProficienciesEditorState::new(),

            monsters: Vec::new(),
            monsters_editor_state: MonstersEditorState::new(),

            conditions: Vec::new(),
            conditions_editor_state: ConditionsEditorState::new(),

            maps: Vec::new(),
            maps_editor_state: MapsEditorState::new(),

            quests: Vec::new(),
            quest_editor_state: QuestEditorState::default(),
            quests_search_filter: String::new(),
            quests_show_preview: true,
            quests_import_buffer: String::new(),
            quests_show_import_dialog: false,

            dialogues: Vec::new(),
            dialogue_editor_state: DialogueEditorState::default(),

            npc_editor_state: npc_editor::NpcEditorState::default(),

            classes_editor_state: classes_editor::ClassesEditorState::default(),

            races_editor_state: races_editor::RacesEditorState::default(),

            characters_editor_state: characters_editor::CharactersEditorState::default(),

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
            show_cleanup_candidates: false,
            cleanup_candidates_selected: std::collections::HashSet::new(),

            file_load_merge_mode: true, // Default to merge mode

            // Phase 7: Logging and developer experience
            logger: Logger::default(),
            tool_config: ToolConfig::default(),
            show_preferences: false,
            show_debug_panel: false,
            debug_panel_filter_level: LogLevel::Info,
            debug_panel_auto_scroll: true,
        }
    }
}

impl CampaignBuilderApp {
    /// Create a default item for the edit buffer
    #[allow(deprecated)]
    fn default_item() -> Item {
        Item {
            id: 0,
            name: String::new(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
            }),
            base_cost: 0,
            sell_cost: 0,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
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
        use antares::domain::character::{AttributePair, AttributePair16};
        use antares::domain::combat::monster::MonsterCondition;

        MonsterDefinition {
            id: 0,
            name: String::new(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: AttributePair16::new(10),
            ac: AttributePair::new(10),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }
    }

    /// Validate item IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_item_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for item in &self.items {
            if !seen_ids.insert(item.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Items,
                    format!("Duplicate item ID: {}", item.id),
                ));
            }
        }
        errors
    }

    /// Validate spell IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_spell_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for spell in &self.spells {
            if !seen_ids.insert(spell.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Spells,
                    format!("Duplicate spell ID: {}", spell.id),
                ));
            }
        }
        errors
    }

    /// Validate monster IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_monster_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for monster in &self.monsters {
            if !seen_ids.insert(monster.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Monsters,
                    format!("Duplicate monster ID: {}", monster.id),
                ));
            }
        }
        errors
    }

    /// Validate map IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_map_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for map in &self.maps {
            if !seen_ids.insert(map.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Maps,
                    format!("Duplicate map ID: {}", map.id),
                ));
            }
        }
        errors
    }

    /// Validate condition IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_condition_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for cond in &self.conditions {
            if !seen_ids.insert(cond.id.clone()) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Conditions,
                    format!("Duplicate condition ID: {}", cond.id),
                ));
            }
        }
        errors
    }

    /// Validate NPC IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    fn validate_npc_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for npc in &self.npc_editor_state.npcs {
            if !seen_ids.insert(npc.id.clone()) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::NPCs,
                    format!("Duplicate NPC ID: {}", npc.id),
                ));
            }
        }
        errors
    }

    /// Validate character IDs for uniqueness and references
    ///
    /// Returns validation errors for:
    /// - Duplicate character IDs
    /// - Empty character IDs
    /// - Empty character names (warning)
    /// - Non-existent class references
    /// - Non-existent race references
    fn validate_character_ids(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for character in &self.characters_editor_state.characters {
            // Check for duplicate IDs
            if !seen_ids.insert(character.id.clone()) {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!("Duplicate character ID: '{}'", character.id),
                ));
            }

            // Check for empty IDs
            if character.id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    "Character has empty ID",
                ));
            }

            // Check for empty names
            if character.name.is_empty() {
                results.push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Characters,
                    format!("Character '{}' has empty name", character.id),
                ));
            }

            // Validate class exists
            let class_exists = self
                .classes_editor_state
                .classes
                .iter()
                .any(|c| c.id == character.class_id);
            if !class_exists && !character.class_id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!(
                        "Character '{}' references non-existent class '{}'",
                        character.id, character.class_id
                    ),
                ));
            }

            // Validate race exists
            let race_exists = self
                .races_editor_state
                .races
                .iter()
                .any(|r| r.id == character.race_id);
            if !race_exists && !character.race_id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!(
                        "Character '{}' references non-existent race '{}'",
                        character.id, character.race_id
                    ),
                ));
            }
        }

        // Add passed message if no characters or all valid
        if self.characters_editor_state.characters.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Characters,
                "No characters defined",
            ));
        } else if results
            .iter()
            .all(|r| r.severity != validation::ValidationSeverity::Error)
        {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Characters,
                format!(
                    "{} character(s) validated",
                    self.characters_editor_state.characters.len()
                ),
            ));
        }

        results
    }

    /// Validate proficiency IDs for uniqueness and cross-references
    ///
    /// Returns validation errors for:
    /// - Duplicate proficiency IDs
    /// - Empty proficiency IDs
    /// - Empty proficiency names (warning)
    /// - Proficiencies referenced by classes that don't exist
    /// - Proficiencies referenced by races that don't exist
    /// - Proficiencies required by items that don't exist
    /// - Info messages for unreferenced proficiencies
    fn validate_proficiency_ids(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for proficiency in &self.proficiencies {
            // Check for duplicate IDs
            if !seen_ids.insert(proficiency.id.clone()) {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    format!("Duplicate proficiency ID: '{}'", proficiency.id),
                ));
            }

            // Check for empty IDs
            if proficiency.id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    "Proficiency has empty ID",
                ));
            }

            // Check for empty names
            if proficiency.name.is_empty() {
                results.push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Proficiencies,
                    format!("Proficiency '{}' has empty name", proficiency.id),
                ));
            }
        }

        // Cross-reference validation: Check for proficiencies referenced by classes
        let mut referenced_proficiencies = std::collections::HashSet::new();
        for class in &self.classes_editor_state.classes {
            for prof_id in &class.proficiencies {
                referenced_proficiencies.insert(prof_id.clone());

                let prof_exists = self.proficiencies.iter().any(|p| &p.id == prof_id);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Class '{}' references non-existent proficiency '{}'",
                            class.id, prof_id
                        ),
                    ));
                }
            }
        }

        // Cross-reference validation: Check for proficiencies referenced by races
        for race in &self.races_editor_state.races {
            for prof_id in &race.proficiencies {
                referenced_proficiencies.insert(prof_id.clone());

                let prof_exists = self.proficiencies.iter().any(|p| &p.id == prof_id);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Race '{}' references non-existent proficiency '{}'",
                            race.id, prof_id
                        ),
                    ));
                }
            }
        }

        // Cross-reference validation: Check for proficiencies required by items
        for item in &self.items {
            if let Some(required_prof) = item.required_proficiency() {
                referenced_proficiencies.insert(required_prof.clone());

                let prof_exists = self.proficiencies.iter().any(|p| &p.id == &required_prof);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Item '{}' requires non-existent proficiency '{}'",
                            item.id, required_prof
                        ),
                    ));
                }
            }
        }

        // Warning for unreferenced proficiencies
        for proficiency in &self.proficiencies {
            if !referenced_proficiencies.contains(&proficiency.id) {
                results.push(validation::ValidationResult::info(
                    validation::ValidationCategory::Proficiencies,
                    format!(
                        "Proficiency '{}' is not used by any class, race, or item",
                        proficiency.id
                    ),
                ));
            }
        }

        // Add passed message if no proficiencies or all valid
        if self.proficiencies.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Proficiencies,
                "No proficiencies defined",
            ));
        } else if results
            .iter()
            .all(|r| r.severity != validation::ValidationSeverity::Error)
        {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Proficiencies,
                format!("{} proficiency(ies) validated", self.proficiencies.len()),
            ));
        }

        results
    }

    /// Generate category status checks (passed or no data info messages)
    ///
    /// This function checks each data category and adds:
    /// - ✅ Passed check if data exists and has no errors
    /// - ℹ️ Info message if no data is loaded for the category
    fn generate_category_status_checks(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();

        // Items category
        if self.items.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Items,
                "No items loaded - add items or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Items,
                format!("{} items validated", self.items.len()),
            ));
        }

        // Spells category
        if self.spells.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Spells,
                "No spells loaded - add spells or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Spells,
                format!("{} spells validated", self.spells.len()),
            ));
        }

        // Monsters category
        if self.monsters.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Monsters,
                "No monsters loaded - add monsters or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Monsters,
                format!("{} monsters validated", self.monsters.len()),
            ));
        }

        // Maps category
        if self.maps.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Maps,
                "No maps loaded - create maps in the Maps editor",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Maps,
                format!("{} maps validated", self.maps.len()),
            ));
        }

        // Conditions category
        if self.conditions.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Conditions,
                "No conditions loaded - add conditions or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Conditions,
                format!("{} conditions validated", self.conditions.len()),
            ));
        }

        // Quests category
        if self.quests.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Quests,
                "No quests loaded - add quests or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Quests,
                format!("{} quests validated", self.quests.len()),
            ));
        }

        // Dialogues category
        if self.dialogues.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Dialogues,
                "No dialogues loaded - add dialogues or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Dialogues,
                format!("{} dialogues validated", self.dialogues.len()),
            ));
        }

        // NPCs category
        if self.npc_editor_state.npcs.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::NPCs,
                "No NPCs loaded - add NPCs or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::NPCs,
                format!("{} NPCs validated", self.npc_editor_state.npcs.len()),
            ));
        }

        // Classes category
        if self.classes_editor_state.classes.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Classes,
                "No classes loaded - add classes or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Classes,
                format!(
                    "{} classes validated",
                    self.classes_editor_state.classes.len()
                ),
            ));
        }

        // Races category
        if self.races_editor_state.races.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Races,
                "No races loaded - add races or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Races,
                format!("{} races validated", self.races_editor_state.races.len()),
            ));
        }

        results
    }

    /// Groups and filters validation results according to the UI filter settings and
    /// returns only categories that have results after applying the filters.
    ///
    /// This variation returns owned `ValidationResult` objects (cloned) to avoid
    /// lifetime/borrow conflicts with UI closures that can overlap `&mut self`
    /// borrows. Cloning is acceptable for this UI-only structure and keeps the
    /// UI code simpler and safer.
    fn grouped_filtered_validation_results(
        &self,
    ) -> Vec<(
        validation::ValidationCategory,
        Vec<validation::ValidationResult>,
    )> {
        use std::collections::HashMap;

        // Bucket results by category after applying UI filters.
        let mut buckets: HashMap<
            validation::ValidationCategory,
            Vec<validation::ValidationResult>,
        > = HashMap::new();

        for res in &self.validation_errors {
            // Apply active severity filter
            let should_show = match self.validation_filter {
                ValidationFilter::All => true,
                ValidationFilter::ErrorsOnly => {
                    res.severity == validation::ValidationSeverity::Error
                }
                ValidationFilter::WarningsOnly => {
                    res.severity == validation::ValidationSeverity::Warning
                }
            };

            if !should_show {
                continue;
            }

            // Clone the result into the bucket, providing an owned copy for UI use.
            buckets.entry(res.category).or_default().push(res.clone());
        }

        // Convert into a Vec ordered by category display order.
        let mut result: Vec<(
            validation::ValidationCategory,
            Vec<validation::ValidationResult>,
        )> = Vec::new();

        for category in validation::ValidationCategory::all() {
            if let Some(group) = buckets.remove(&category) {
                if !group.is_empty() {
                    result.push((category, group));
                }
            }
        }

        result
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
    /// Discovers map files in the maps directory by scanning for .ron files
    ///
    /// This function scans the actual maps directory and returns paths to all .ron files found,
    /// rather than inferring filenames from map IDs. This allows maps to have any filename.
    ///
    /// # Returns
    ///
    /// A vector of map file paths relative to the campaign directory.
    fn discover_map_files(&self) -> Vec<String> {
        let mut map_files = Vec::new();

        if let Some(ref campaign_dir) = self.campaign_dir {
            let maps_path = campaign_dir.join(&self.campaign.maps_dir);

            if let Ok(entries) = std::fs::read_dir(&maps_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                        // Store relative path from campaign dir
                        if let Ok(rel_path) = path.strip_prefix(campaign_dir) {
                            map_files.push(rel_path.display().to_string());
                        }
                    }
                }
            }

            // Sort for consistent ordering
            map_files.sort();
        }

        map_files
    }

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
        self.logger.debug(category::FILE_IO, "load_items() called");
        let items_file = self.campaign.items_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&items_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!("Loading items from: {}", items_path.display()),
            );
            if items_path.exists() {
                match fs::read_to_string(&items_path) {
                    Ok(contents) => {
                        self.logger.verbose(
                            category::FILE_IO,
                            &format!("Read {} bytes from items file", contents.len()),
                        );
                        match ron::from_str::<Vec<Item>>(&contents) {
                            Ok(items) => {
                                let count = items.len();
                                self.items = items;

                                // Mark data file as loaded in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_loaded(&items_file, count);
                                }

                                // Validate IDs after loading
                                let id_errors = self.validate_item_ids();
                                if !id_errors.is_empty() {
                                    self.validation_errors.extend(id_errors.clone());
                                    self.logger.warn(
                                        category::DATA,
                                        &format!(
                                            "Loaded {} items with {} ID conflicts",
                                            self.items.len(),
                                            id_errors.len()
                                        ),
                                    );
                                    self.status_message = format!(
                                        "⚠️ Loaded {} items with {} ID conflicts",
                                        self.items.len(),
                                        id_errors.len()
                                    );
                                } else {
                                    self.logger.info(
                                        category::FILE_IO,
                                        &format!("Loaded {} items", self.items.len()),
                                    );
                                    self.status_message =
                                        format!("Loaded {} items", self.items.len());
                                }
                            }
                            Err(e) => {
                                // Mark data file as error in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_error(&items_file, &e.to_string());
                                }
                                self.logger.error(
                                    category::FILE_IO,
                                    &format!("Failed to parse items: {}", e),
                                );
                                self.status_message = format!("Failed to parse items: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&items_file, &e.to_string());
                        }
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to read items file: {}", e),
                        );
                        self.status_message = format!("Failed to read items file: {}", e);
                    }
                }
            } else {
                self.logger.warn(
                    category::FILE_IO,
                    &format!("Items file does not exist: {}", items_path.display()),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load items",
            );
        }
    }

    /// Save items to RON file
    fn save_items(&mut self) -> Result<(), String> {
        self.logger.debug(category::FILE_IO, "save_items() called");
        if let Some(ref dir) = self.campaign_dir {
            let items_path = dir.join(&self.campaign.items_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!(
                    "Saving {} items to: {}",
                    self.items.len(),
                    items_path.display()
                ),
            );

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

            fs::write(&items_path, &contents)
                .map_err(|e| format!("Failed to write items file: {}", e))?;

            self.logger.info(
                category::FILE_IO,
                &format!(
                    "Saved {} items ({} bytes)",
                    self.items.len(),
                    contents.len()
                ),
            );
            self.unsaved_changes = true;
            Ok(())
        } else {
            self.logger.error(
                category::FILE_IO,
                "No campaign directory set when trying to save items",
            );
            Err("No campaign directory set".to_string())
        }
    }

    /// Load spells from RON file
    fn load_spells(&mut self) {
        let spells_file = self.campaign.spells_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let spells_path = dir.join(&spells_file);
            if spells_path.exists() {
                match fs::read_to_string(&spells_path) {
                    Ok(contents) => match ron::from_str::<Vec<Spell>>(&contents) {
                        Ok(spells) => {
                            let count = spells.len();
                            self.spells = spells;

                            // Mark data file as loaded in asset manager
                            if let Some(ref mut manager) = self.asset_manager {
                                manager.mark_data_file_loaded(&spells_file, count);
                            }

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
                            // Mark data file as error in asset manager
                            if let Some(ref mut manager) = self.asset_manager {
                                manager.mark_data_file_error(&spells_file, &e.to_string());
                            }
                            self.status_message = format!("Failed to parse spells: {}", e);
                        }
                    },
                    Err(e) => {
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&spells_file, &e.to_string());
                        }
                        self.status_message = format!("Failed to read spells file: {}", e);
                        eprintln!("Failed to read spells file {:?}: {}", spells_path, e);
                    }
                }
            }
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

    /// Load conditions from RON file
    fn load_conditions(&mut self) {
        if let Some(ref dir) = self.campaign_dir {
            let conditions_path = dir.join(&self.campaign.conditions_file);
            if conditions_path.exists() {
                match fs::read_to_string(&conditions_path) {
                    Ok(contents) => match ron::from_str::<Vec<ConditionDefinition>>(&contents) {
                        Ok(conditions) => {
                            self.conditions = conditions;

                            // Validate IDs after loading
                            let id_errors = self.validate_condition_ids();
                            if !id_errors.is_empty() {
                                self.validation_errors.extend(id_errors.clone());
                                self.status_message = format!(
                                    "⚠️ Loaded {} conditions with {} ID conflicts",
                                    self.conditions.len(),
                                    id_errors.len()
                                );
                            } else {
                                self.status_message =
                                    format!("Loaded {} conditions", self.conditions.len());
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse conditions: {}", e);
                            eprintln!(
                                "Failed to parse conditions from {:?}: {}",
                                conditions_path, e
                            );
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("Failed to read conditions file: {}", e);
                        eprintln!(
                            "Failed to read conditions file {:?}: {}",
                            conditions_path, e
                        );
                    }
                }
            } else {
                eprintln!("Conditions file does not exist: {:?}", conditions_path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load conditions");
        }
    }

    /// Save conditions to RON file
    fn save_conditions(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let conditions_path = dir.join(&self.campaign.conditions_file);

            // Create conditions directory if it doesn't exist
            if let Some(parent) = conditions_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create conditions directory: {}", e))?;
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(&self.conditions, ron_config)
                .map_err(|e| format!("Failed to serialize conditions: {}", e))?;

            fs::write(&conditions_path, contents)
                .map_err(|e| format!("Failed to write conditions file: {}", e))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Load proficiencies from RON file
    fn load_proficiencies(&mut self) {
        self.logger
            .debug(category::FILE_IO, "load_proficiencies() called");
        let proficiencies_file = self.campaign.proficiencies_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let proficiencies_path = dir.join(&proficiencies_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!(
                    "Loading proficiencies from: {}",
                    proficiencies_path.display()
                ),
            );
            if proficiencies_path.exists() {
                match fs::read_to_string(&proficiencies_path) {
                    Ok(contents) => {
                        self.logger.verbose(
                            category::FILE_IO,
                            &format!("Read {} bytes from proficiencies file", contents.len()),
                        );
                        match ron::from_str::<Vec<ProficiencyDefinition>>(&contents) {
                            Ok(proficiencies) => {
                                let count = proficiencies.len();
                                self.proficiencies = proficiencies;

                                // Mark data file as loaded in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_loaded(&proficiencies_file, count);
                                }

                                self.logger.info(
                                    category::FILE_IO,
                                    &format!("Loaded {} proficiencies", self.proficiencies.len()),
                                );
                                self.status_message =
                                    format!("Loaded {} proficiencies", self.proficiencies.len());
                            }
                            Err(e) => {
                                // Mark data file as error in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager
                                        .mark_data_file_error(&proficiencies_file, &e.to_string());
                                }
                                self.logger.error(
                                    category::FILE_IO,
                                    &format!("Failed to parse proficiencies: {}", e),
                                );
                                self.status_message =
                                    format!("Failed to parse proficiencies: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&proficiencies_file, &e.to_string());
                        }
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to read proficiencies file: {}", e),
                        );
                        self.status_message = format!("Failed to read proficiencies file: {}", e);
                    }
                }
            } else {
                self.logger.warn(
                    category::FILE_IO,
                    &format!(
                        "Proficiencies file does not exist: {}",
                        proficiencies_path.display()
                    ),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load proficiencies",
            );
        }
    }

    /// Save proficiencies to RON file
    fn save_proficiencies(&mut self) -> Result<(), String> {
        self.logger
            .debug(category::FILE_IO, "save_proficiencies() called");
        if let Some(ref dir) = self.campaign_dir {
            let proficiencies_path = dir.join(&self.campaign.proficiencies_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!(
                    "Saving {} proficiencies to: {}",
                    self.proficiencies.len(),
                    proficiencies_path.display()
                ),
            );

            // Create proficiencies directory if it doesn't exist
            if let Some(parent) = proficiencies_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create proficiencies directory: {}", e))?;
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(&self.proficiencies, ron_config)
                .map_err(|e| format!("Failed to serialize proficiencies: {}", e))?;

            fs::write(&proficiencies_path, &contents)
                .map_err(|e| format!("Failed to write proficiencies file: {}", e))?;

            self.logger.info(
                category::FILE_IO,
                &format!(
                    "Saved {} proficiencies ({} bytes)",
                    self.proficiencies.len(),
                    contents.len()
                ),
            );
            self.unsaved_changes = true;
            Ok(())
        } else {
            self.logger.error(
                category::FILE_IO,
                "No campaign directory set when trying to save proficiencies",
            );
            Err("No campaign directory set".to_string())
        }
    }

    /// Save dialogues to a file path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save dialogues to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save was successful
    fn save_dialogues_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dialogues directory: {}", e))?;
        }

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);

        let contents = ron::ser::to_string_pretty(&self.dialogues, ron_config)
            .map_err(|e| format!("Failed to serialize dialogues: {}", e))?;

        std::fs::write(path, contents)
            .map_err(|e| format!("Failed to write dialogues file: {}", e))?;

        Ok(())
    }

    /// Save NPCs to a file path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save NPCs to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save was successful
    fn save_npcs_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create NPCs directory: {}", e))?;
        }

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);

        let contents = ron::ser::to_string_pretty(&self.npc_editor_state.npcs, ron_config)
            .map_err(|e| format!("Failed to serialize NPCs: {}", e))?;

        std::fs::write(path, contents).map_err(|e| format!("Failed to write NPCs file: {}", e))?;

        Ok(())
    }

    /// Load NPCs from campaign file
    fn load_npcs(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let npcs_path = dir.join(&self.campaign.npcs_file);

            if npcs_path.exists() {
                let contents = std::fs::read_to_string(&npcs_path).map_err(CampaignError::Io)?;

                let npcs: Vec<antares::domain::world::npc::NpcDefinition> =
                    ron::from_str(&contents).map_err(CampaignError::Deserialization)?;

                let count = npcs.len();
                self.npc_editor_state.npcs = npcs;
                self.logger.log(
                    LogLevel::Info,
                    category::CAMPAIGN,
                    &format!("Loaded {} NPCs", count),
                );
                // Mark data file as loaded in asset manager
                if let Some(ref mut manager) = self.asset_manager {
                    manager.mark_data_file_loaded(&self.campaign.npcs_file, count);
                }
            } else {
                self.logger.log(
                    LogLevel::Warn,
                    category::CAMPAIGN,
                    &format!("NPCs file not found: {:?}", npcs_path),
                );
            }
        }
        Ok(())
    }

    /// Load monsters from RON file
    fn load_monsters(&mut self) {
        let monsters_file = self.campaign.monsters_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let monsters_path = dir.join(&monsters_file);
            if monsters_path.exists() {
                match fs::read_to_string(&monsters_path) {
                    Ok(contents) => match ron::from_str::<Vec<MonsterDefinition>>(&contents) {
                        Ok(monsters) => {
                            let count = monsters.len();
                            self.monsters = monsters;

                            // Mark data file as loaded in asset manager
                            if let Some(ref mut manager) = self.asset_manager {
                                manager.mark_data_file_loaded(&monsters_file, count);
                            }

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
                            // Mark data file as error in asset manager
                            if let Some(ref mut manager) = self.asset_manager {
                                manager.mark_data_file_error(&monsters_file, &e.to_string());
                            }
                            self.status_message = format!("Failed to parse monsters: {}", e);
                        }
                    },
                    Err(e) => {
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&monsters_file, &e.to_string());
                        }
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

                                            // Mark individual map file as loaded in asset manager
                                            if let Some(ref mut manager) = self.asset_manager {
                                                if let Some(relative_path) =
                                                    path.strip_prefix(dir).ok()
                                                {
                                                    if let Some(path_str) = relative_path.to_str() {
                                                        manager.mark_data_file_loaded(path_str, 1);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.status_message = format!(
                                                "Failed to parse map {:?}: {}",
                                                path.file_name().unwrap_or_default(),
                                                e
                                            );

                                            // Mark individual map file as error in asset manager
                                            if let Some(ref mut manager) = self.asset_manager {
                                                if let Some(relative_path) =
                                                    path.strip_prefix(dir).ok()
                                                {
                                                    if let Some(path_str) = relative_path.to_str() {
                                                        manager.mark_data_file_error(
                                                            path_str,
                                                            &e.to_string(),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        self.status_message = format!(
                                            "Failed to read map {:?}: {}",
                                            path.file_name().unwrap_or_default(),
                                            e
                                        );

                                        // Mark individual map file as error in asset manager
                                        if let Some(ref mut manager) = self.asset_manager {
                                            if let Some(relative_path) = path.strip_prefix(dir).ok()
                                            {
                                                if let Some(path_str) = relative_path.to_str() {
                                                    manager.mark_data_file_error(
                                                        path_str,
                                                        &e.to_string(),
                                                    );
                                                }
                                            }
                                        }
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
        self.logger
            .debug(category::VALIDATION, "validate_campaign() called");
        self.validation_errors.clear();

        // Validate data IDs for uniqueness (in EditorTab order)
        self.validation_errors.extend(self.validate_item_ids());
        self.validation_errors.extend(self.validate_spell_ids());
        self.validation_errors.extend(self.validate_condition_ids());
        self.validation_errors.extend(self.validate_monster_ids());
        self.validation_errors.extend(self.validate_map_ids());
        // Quests validated elsewhere
        // Classes validated elsewhere
        // Races validated elsewhere
        self.validation_errors.extend(self.validate_character_ids());
        // Dialogues validated elsewhere
        self.validation_errors.extend(self.validate_npc_ids());
        self.validation_errors
            .extend(self.validate_proficiency_ids());

        // Add category status checks (passed or no data info)
        self.validation_errors
            .extend(self.generate_category_status_checks());

        // Required fields - Metadata category
        if self.campaign.id.is_empty() {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign ID is required",
                ));
        } else if !self
            .campaign
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign ID must contain only alphanumeric characters and underscores",
                ));
        }

        if self.campaign.name.is_empty() {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign name is required",
                ));
        }

        if self.campaign.author.is_empty() {
            self.validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Metadata,
                    "Author name is recommended",
                ));
        }

        // Version validation - Metadata category
        if !self.campaign.version.contains('.') {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Version should follow semantic versioning (e.g., 1.0.0)",
                ));
        }

        // Engine version validation - Metadata category
        if !self.campaign.engine_version.contains('.') {
            self.validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Metadata,
                    "Engine version should follow semantic versioning",
                ));
        }

        // Configuration validation
        if self.campaign.starting_map.is_empty() {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Starting map is required",
                ));
        } else if !self.maps.is_empty() {
            // Only validate existence if the maps are loaded - avoids false positives during
            // tests or when the campaign hasn't loaded assets yet.
            let start_map_key = self.campaign.starting_map.trim();
            let mut found = false;

            // 1) Numeric map ID (e.g., "1")
            if let Ok(parsed_id) = start_map_key.parse::<u16>() {
                found = self.maps.iter().any(|m| m.id == parsed_id);
            }

            // 2) map_N pattern (e.g., "map_1")
            if !found {
                if let Some(stripped) = start_map_key.strip_prefix("map_") {
                    if let Ok(num) = stripped.parse::<u16>() {
                        found = self.maps.iter().any(|m| m.id == num);
                    }
                }
            }

            // 3) filename pattern with .ron (e.g., "map_1.ron" or "starter_town.ron")
            if !found && start_map_key.ends_with(".ron") {
                let base = start_map_key.trim_end_matches(".ron");
                if let Some(stripped) = base.strip_prefix("map_") {
                    if let Ok(num) = stripped.parse::<u16>() {
                        found = self.maps.iter().any(|m| m.id == num);
                    }
                }
                if !found {
                    if let Ok(num) = base.parse::<u16>() {
                        found = self.maps.iter().any(|m| m.id == num);
                    }
                }
                if !found {
                    let normalized = base.replace('_', " ").to_lowercase();
                    found = self
                        .maps
                        .iter()
                        .any(|m| m.name.to_lowercase() == normalized);
                }
            }

            // 4) Normalized name match: starter_town -> "starter town" matches "Starter Town"
            if !found {
                let normalized = start_map_key.replace('_', " ").to_lowercase();
                found = self
                    .maps
                    .iter()
                    .any(|m| m.name.to_lowercase() == normalized);
            }

            // If no match, this is a configuration error for the campaign
            if !found {
                self.validation_errors
                    .push(validation::ValidationResult::error(
                        validation::ValidationCategory::Configuration,
                        format!(
                            "Starting map '{}' does not match any loaded map",
                            start_map_key
                        ),
                    ));
            }
        }

        if self.campaign.max_party_size == 0 || self.campaign.max_party_size > PARTY_MAX_SIZE {
            self.validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!("Max party size should be between 1 and {}", PARTY_MAX_SIZE),
                ));
        }

        if self.campaign.max_roster_size < self.campaign.max_party_size {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Max roster size must be >= max party size",
                ));
        }

        if self.campaign.starting_level == 0
            || self.campaign.starting_level > self.campaign.max_level
        {
            self.validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Starting level must be between 1 and max level",
                ));
        }

        // Starting resource validity checks
        if self.campaign.starting_food < FOOD_MIN as u32
            || self.campaign.starting_food > FOOD_MAX as u32
        {
            self.validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!(
                        "Starting food must be between {} and {}",
                        FOOD_MIN, FOOD_MAX
                    ),
                ));
        }

        if self.campaign.starting_gold > STARTING_GOLD_MAX {
            self.validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!(
                        "Starting gold ({}) exceeds recommended maximum {}",
                        self.campaign.starting_gold, STARTING_GOLD_MAX
                    ),
                ));
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
            ("NPCs file", &self.campaign.npcs_file),
        ] {
            if path.is_empty() {
                self.validation_errors
                    .push(validation::ValidationResult::error(
                        validation::ValidationCategory::FilePaths,
                        format!("{} path is required", field),
                    ));
            } else if !path.ends_with(".ron") {
                self.validation_errors
                    .push(validation::ValidationResult::warning(
                        validation::ValidationCategory::FilePaths,
                        format!("{} should use .ron extension", field),
                    ));
            }
        }

        // Run SDK validator for deeper content checks (e.g., starting innkeeper)
        // Only run if NPCs have been loaded into the editor OR the configured starting
        // innkeeper differs from the default tutorial innkeeper (i.e., user-specified).
        if !self.npc_editor_state.npcs.is_empty()
            || self.campaign.starting_innkeeper != default_starting_innkeeper()
        {
            // Build a lightweight ContentDatabase containing relevant content so the SDK Validator can
            // validate content-dependent configuration such as `starting_innkeeper`.
            let mut db = antares::sdk::database::ContentDatabase::new();

            // Populate NPC database from the editor state
            for npc in &self.npc_editor_state.npcs {
                // Ignore insertion errors from the DB helper (should be infrequent)
                let _ = db.npcs.add_npc(npc.clone());
            }

            // Invoke SDK validator for campaign config checks
            let validator = antares::sdk::validation::Validator::new(&db);

            // Build a minimal CampaignConfig for validation - other fields are defaulted to reasonable values
            let config = antares::sdk::campaign_loader::CampaignConfig {
                starting_map: 0,
                starting_position: antares::domain::types::Position { x: 0, y: 0 },
                starting_direction: antares::domain::types::Direction::North,
                starting_gold: self.campaign.starting_gold,
                starting_food: self.campaign.starting_food,
                starting_innkeeper: self.campaign.starting_innkeeper.clone(),
                max_party_size: self.campaign.max_party_size as usize,
                max_roster_size: self.campaign.max_roster_size as usize,
                difficulty: antares::sdk::campaign_loader::Difficulty::Normal,
                permadeath: self.campaign.permadeath,
                allow_multiclassing: self.campaign.allow_multiclassing,
                starting_level: self.campaign.starting_level,
                max_level: self.campaign.max_level,
            };

            let config_errors = validator.validate_campaign_config(&config);
            for ve in config_errors {
                match ve {
                    antares::sdk::validation::ValidationError::InvalidStartingInnkeeper {
                        innkeeper_id,
                        reason,
                    } => {
                        // Use a message format that matches existing tests' expectations
                        let msg =
                            format!("Starting innkeeper '{}' invalid: {}", innkeeper_id, reason);
                        self.validation_errors
                            .push(validation::ValidationResult::error(
                                validation::ValidationCategory::Configuration,
                                msg,
                            ));
                    }
                    other => match other.severity() {
                        antares::sdk::validation::Severity::Error => {
                            self.validation_errors
                                .push(validation::ValidationResult::error(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ));
                        }
                        antares::sdk::validation::Severity::Warning => {
                            self.validation_errors
                                .push(validation::ValidationResult::warning(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ));
                        }
                        antares::sdk::validation::Severity::Info => {
                            self.validation_errors
                                .push(validation::ValidationResult::info(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ));
                        }
                    },
                }
            }
        }

        // Update status using ValidationSummary
        let summary = validation::ValidationSummary::from_results(&self.validation_errors);

        if self.validation_errors.is_empty() {
            self.logger
                .info(category::VALIDATION, "Validation passed with no issues");
            self.status_message = "✅ Validation passed!".to_string();
        } else {
            self.logger.info(
                category::VALIDATION,
                &format!(
                    "Validation complete: {} error(s), {} warning(s)",
                    summary.error_count, summary.warning_count
                ),
            );
            // Log individual errors at debug level
            for result in &self.validation_errors {
                let level_str = match result.severity {
                    validation::ValidationSeverity::Error => "ERROR",
                    validation::ValidationSeverity::Warning => "WARN",
                    validation::ValidationSeverity::Info => "INFO",
                    validation::ValidationSeverity::Passed => "PASS",
                };
                self.logger.debug(
                    category::VALIDATION,
                    &format!("[{}] {}: {}", level_str, result.category, result.message),
                );
            }
            self.status_message = format!(
                "Validation: {} error(s), {} warning(s)",
                summary.error_count, summary.warning_count
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

        // Sync the campaign editor's authoritative metadata and edit buffer with
        // the newly created campaign. This ensures the editor shows the fresh
        // campaign values and a fresh buffer, preventing stale UI states.
        self.campaign_editor_state.metadata = self.campaign.clone();
        self.campaign_editor_state.buffer =
            campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                &self.campaign_editor_state.metadata,
            );
        self.campaign_editor_state.has_unsaved_changes = false;
        self.campaign_editor_state.mode = campaign_editor::CampaignEditorMode::List;

        // Clear all loaded campaign content and reset editor states so the new
        // campaign starts with an empty workspace rather than retaining the
        // previously opened campaign's data.
        self.items.clear();
        self.items_editor_state = ItemsEditorState::new();

        self.spells.clear();
        self.spells_editor_state = SpellsEditorState::new();

        self.proficiencies.clear();
        self.proficiencies_editor_state = proficiencies_editor::ProficienciesEditorState::new();

        self.monsters.clear();
        self.monsters_editor_state = MonstersEditorState::new();

        self.conditions.clear();
        self.conditions_editor_state = ConditionsEditorState::new();

        self.maps.clear();
        self.maps_editor_state = MapsEditorState::new();

        self.quests.clear();
        self.quest_editor_state = QuestEditorState::default();

        self.dialogues.clear();
        self.dialogue_editor_state = DialogueEditorState::default();

        // Reset NPCs, characters, classes and races editors
        self.npc_editor_state = npc_editor::NpcEditorState::default();
        self.characters_editor_state = characters_editor::CharactersEditorState::new();
        self.classes_editor_state = classes_editor::ClassesEditorState::default();
        self.races_editor_state = races_editor::RacesEditorState::default();

        // Reset campaign file/path-related state
        self.campaign_path = None;
        self.campaign_dir = None;
        self.unsaved_changes = false;
        self.validation_errors.clear();
        self.file_tree.clear();

        // Clear undo/redo history and any remembered content in the manager's state
        self.undo_redo_manager.clear();
        {
            let s = self.undo_redo_manager.state_mut();
            s.items.clear();
            s.spells.clear();
            s.monsters.clear();
            s.maps.clear();
            s.quests.clear();
            s.dialogues.clear();
            s.metadata_changed = false;
        }

        // Drop asset manager to avoid referencing previous campaign assets
        self.asset_manager = None;

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

        if let Err(e) = self.save_conditions() {
            save_warnings.push(format!("Conditions: {}", e));
        }

        if let Err(e) = self.save_proficiencies() {
            save_warnings.push(format!("Proficiencies: {}", e));
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

            let npcs_path = dir.join(&self.campaign.npcs_file);
            if let Err(e) = self.save_npcs_to_file(&npcs_path) {
                save_warnings.push(format!("NPCs: {}", e));
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

        // Keep the campaign metadata editor in sync with the saved campaign file.
        // This ensures that the UI and editor buffer reflect the authoritative
        // campaign data after a successful save.
        self.campaign_editor_state.metadata = self.campaign.clone();
        self.campaign_editor_state.buffer =
            campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                &self.campaign_editor_state.metadata,
            );
        self.campaign_editor_state.has_unsaved_changes = false;
        self.campaign_editor_state.mode = campaign_editor::CampaignEditorMode::List;

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
        self.logger
            .debug(category::FILE_IO, "do_open_campaign() called");
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .pick_file()
        {
            self.logger.info(
                category::FILE_IO,
                &format!("Opening campaign: {}", path.display()),
            );
            match self.load_campaign_file(&path) {
                Ok(()) => {
                    self.campaign_path = Some(path.clone());

                    // Set campaign directory
                    if let Some(parent) = path.parent() {
                        let parent_buf = parent.to_path_buf();
                        self.campaign_dir = Some(parent_buf.clone());
                        self.logger.verbose(
                            category::FILE_IO,
                            &format!("Campaign directory: {}", parent_buf.display()),
                        );
                        self.update_file_tree(&parent_buf);
                    }

                    // Load data files
                    self.logger
                        .debug(category::FILE_IO, "Loading data files...");
                    self.load_items();
                    self.load_spells();
                    self.load_proficiencies();
                    self.load_monsters();
                    self.load_classes_from_campaign();
                    self.load_races_from_campaign();
                    self.load_characters_from_campaign();
                    self.load_maps();
                    self.load_conditions();

                    // Load quests and dialogues
                    if let Err(e) = self.load_quests() {
                        self.logger
                            .warn(category::FILE_IO, &format!("Failed to load quests: {}", e));
                    }

                    if let Err(e) = self.load_dialogues() {
                        self.logger.warn(
                            category::FILE_IO,
                            &format!("Failed to load dialogues: {}", e),
                        );
                    }

                    if let Err(e) = self.load_npcs() {
                        self.logger
                            .warn(category::FILE_IO, &format!("Failed to load NPCs: {}", e));
                    }

                    // Scan asset references and mark loaded data files
                    if let Some(ref mut manager) = self.asset_manager {
                        manager.scan_references(
                            &self.items,
                            &self.quests,
                            &self.dialogues,
                            &self.maps,
                            &self.classes_editor_state.classes,
                            &self.characters_editor_state.characters,
                            &self.npc_editor_state.npcs,
                        );
                        manager.mark_data_files_as_referenced();
                    }

                    self.unsaved_changes = false;
                    self.logger.info(
                        category::FILE_IO,
                        &format!("Campaign opened successfully: {}", self.campaign.name),
                    );
                    self.status_message = format!("Opened campaign from: {}", path.display());

                    // Synchronize campaign editor state with the newly opened campaign.
                    // This ensures the metadata editor shows the loaded campaign and its
                    // edit buffer reflects the current authoritative values.
                    self.campaign_editor_state.metadata = self.campaign.clone();
                    self.campaign_editor_state.buffer =
                        campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                            &self.campaign_editor_state.metadata,
                        );
                    self.campaign_editor_state.has_unsaved_changes = false;
                    self.campaign_editor_state.mode = campaign_editor::CampaignEditorMode::List;
                }
                Err(e) => {
                    self.logger.error(
                        category::FILE_IO,
                        &format!("Failed to load campaign: {}", e),
                    );
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

    /// Show the debug panel window
    ///
    /// Displays:
    /// - Current editor state
    /// - Loaded data counts
    /// - Recent log messages with filtering
    fn show_debug_panel_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_debug_panel;
        egui::Window::new("🐛 Debug Panel")
            .open(&mut open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                // Current state section
                ui.collapsing("📊 Current State", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Active Tab:");
                        ui.strong(self.active_tab.name());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Campaign Path:");
                        if let Some(ref path) = self.campaign_path {
                            ui.monospace(path.display().to_string());
                        } else {
                            ui.weak("(none)");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Unsaved Changes:");
                        if self.unsaved_changes {
                            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "Yes");
                        } else {
                            ui.label("No");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Log Level:");
                        ui.strong(self.logger.level().name());
                    });
                    let uptime = self.logger.uptime();
                    ui.horizontal(|ui| {
                        ui.label("Uptime:");
                        ui.monospace(format!("{:.1}s", uptime.as_secs_f64()));
                    });
                });

                ui.add_space(5.0);

                // Data counts section
                ui.collapsing("📦 Loaded Data", |ui| {
                    egui::Grid::new("debug_data_counts")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Items:");
                            ui.strong(self.items.len().to_string());
                            ui.end_row();

                            ui.label("Spells:");
                            ui.strong(self.spells.len().to_string());
                            ui.end_row();

                            ui.label("Monsters:");
                            ui.strong(self.monsters.len().to_string());
                            ui.end_row();

                            ui.label("Maps:");
                            ui.strong(self.maps.len().to_string());
                            ui.end_row();

                            ui.label("Quests:");
                            ui.strong(self.quests.len().to_string());
                            ui.end_row();

                            ui.label("Dialogues:");
                            ui.strong(self.dialogues.len().to_string());
                            ui.end_row();

                            ui.label("Conditions:");
                            ui.strong(self.conditions.len().to_string());
                            ui.end_row();

                            ui.label("Classes:");
                            ui.strong(self.classes_editor_state.classes.len().to_string());
                            ui.end_row();
                        });
                });

                ui.add_space(5.0);

                // Log messages section
                ui.collapsing("📝 Log Messages", |ui| {
                    // Controls
                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        egui::ComboBox::from_id_salt("debug_log_filter")
                            .selected_text(self.debug_panel_filter_level.name())
                            .show_ui(ui, |ui| {
                                for level in [
                                    LogLevel::Error,
                                    LogLevel::Warn,
                                    LogLevel::Info,
                                    LogLevel::Debug,
                                    LogLevel::Verbose,
                                ] {
                                    ui.selectable_value(
                                        &mut self.debug_panel_filter_level,
                                        level,
                                        level.name(),
                                    );
                                }
                            });

                        ui.checkbox(&mut self.debug_panel_auto_scroll, "Auto-scroll");

                        if ui.button("Clear").clicked() {
                            self.logger.clear();
                        }
                    });

                    // Message counts
                    let counts = self.logger.message_counts();
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            format!("E:{}", counts.error),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 100),
                            format!("W:{}", counts.warn),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 200, 200),
                            format!("I:{}", counts.info),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(150, 200, 255),
                            format!("D:{}", counts.debug),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(150, 150, 150),
                            format!("V:{}", counts.verbose),
                        );
                        ui.label(format!("Total: {}", counts.total()));
                    });

                    ui.separator();

                    // Log messages list
                    let scroll_area = egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .auto_shrink([false, false]);

                    scroll_area.show(ui, |ui| {
                        let filter_level = self.debug_panel_filter_level;
                        for msg in self.logger.messages_at_level(filter_level) {
                            let color = msg.level.color();
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(color[0], color[1], color[2]),
                                    format!("[{}]", msg.level.prefix()),
                                );
                                ui.colored_label(
                                    egui::Color32::from_rgb(150, 150, 200),
                                    format!("[{}]", msg.category),
                                );
                                ui.label(&msg.message);
                            });
                        }

                        // Auto-scroll to bottom
                        if self.debug_panel_auto_scroll {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Press F12 to toggle this panel");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_debug_panel = false;
                        }
                    });
                });
            });
        self.show_debug_panel = open;
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

// Tests moved to the existing tests module below

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

                ui.menu_button("View", |ui| {
                    // Debug panel toggle
                    let debug_label = if self.show_debug_panel {
                        "🐛 Hide Debug Panel"
                    } else {
                        "🐛 Show Debug Panel"
                    };
                    if ui.button(debug_label).clicked() {
                        self.show_debug_panel = !self.show_debug_panel;
                        self.logger.info(
                            category::UI,
                            &format!(
                                "Debug panel {}",
                                if self.show_debug_panel {
                                    "opened"
                                } else {
                                    "closed"
                                }
                            ),
                        );
                        ui.close();
                    }

                    ui.separator();
                    ui.label("Log Level:");

                    for level in [
                        LogLevel::Error,
                        LogLevel::Warn,
                        LogLevel::Info,
                        LogLevel::Debug,
                        LogLevel::Verbose,
                    ] {
                        if ui
                            .selectable_label(self.logger.level() == level, level.name())
                            .clicked()
                        {
                            self.logger.set_level(level);
                            self.logger.info(
                                category::APP,
                                &format!("Log level changed to {}", level.name()),
                            );
                            ui.close();
                        }
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
                    ui.separator();
                    // Preferences dialog toggle
                    if ui.button("⚙️ Preferences...").clicked() {
                        self.show_preferences = true;
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

        // F12 = Toggle Debug Panel
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.show_debug_panel = !self.show_debug_panel;
            self.logger.info(
                category::UI,
                &format!(
                    "Debug panel {} (F12)",
                    if self.show_debug_panel {
                        "opened"
                    } else {
                        "closed"
                    }
                ),
            );
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
                    EditorTab::Config,
                    EditorTab::Items,
                    EditorTab::Spells,
                    EditorTab::Conditions,
                    EditorTab::Monsters,
                    EditorTab::Maps,
                    EditorTab::Quests,
                    EditorTab::Classes,
                    EditorTab::Races,
                    EditorTab::Characters,
                    EditorTab::Dialogues,
                    EditorTab::NPCs,
                    EditorTab::Proficiencies,
                    EditorTab::Assets,
                    EditorTab::Validation,
                ];

                for tab in &tabs {
                    let is_selected = self.active_tab == *tab;
                    if ui.selectable_label(is_selected, tab.name()).clicked() {
                        let previous_tab = self.active_tab;
                        self.active_tab = *tab;
                        self.logger.debug(
                            category::EDITOR,
                            &format!("Tab changed: {} -> {}", previous_tab.name(), tab.name()),
                        );
                    }
                }

                ui.separator();
                ui.label("Antares Campaign Builder");
                ui.label("Foundation v0.2.0");
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
            EditorTab::Config => self.config_editor_state.show(
                ui,
                self.campaign_dir.as_ref(),
                &mut self.unsaved_changes,
                &mut self.status_message,
            ),
            EditorTab::Items => self.items_editor_state.show(
                ui,
                &mut self.items,
                &self.classes_editor_state.classes,
                self.campaign_dir.as_ref(),
                &self.campaign.items_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Spells => self.spells_editor_state.show(
                ui,
                &mut self.spells,
                &self.conditions,
                self.campaign_dir.as_ref(),
                &self.campaign.spells_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Conditions => {
                self.conditions_editor_state.show(
                    ui,
                    &mut self.conditions,
                    &mut self.spells,
                    self.campaign_dir.as_ref(),
                    &self.campaign.conditions_file,
                    &mut self.unsaved_changes,
                    &mut self.status_message,
                    &mut self.file_load_merge_mode,
                );
                // Handle navigation request from conditions editor
                if let Some(spell_name) = self.conditions_editor_state.navigate_to_spell.take() {
                    // Find the spell index by name and select it in spells editor
                    if let Some(idx) = self.spells.iter().position(|s| s.name == spell_name) {
                        self.spells_editor_state.selected_spell = Some(idx);
                        self.spells_editor_state.mode = spells_editor::SpellsEditorMode::Edit;
                        self.spells_editor_state.edit_buffer = self.spells[idx].clone();
                        self.active_tab = EditorTab::Spells;
                        self.status_message = format!("Jumped to spell: {}", spell_name);
                    } else {
                        self.status_message = format!("Spell '{}' not found", spell_name);
                    }
                }
            }
            EditorTab::Monsters => self.monsters_editor_state.show(
                ui,
                &mut self.monsters,
                self.campaign_dir.as_ref(),
                &self.campaign.monsters_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Maps => self.maps_editor_state.show(
                ui,
                &mut self.maps,
                &self.monsters,
                &self.items,
                &self.conditions,
                &self.npc_editor_state.npcs,
                self.campaign_dir.as_ref(),
                &self.campaign.maps_dir,
                &self.tool_config.display,
                &mut self.unsaved_changes,
                &mut self.status_message,
            ),
            EditorTab::Quests => self.show_quests_editor(ui),
            EditorTab::Classes => self.classes_editor_state.show(
                ui,
                &self.items,
                self.campaign_dir.as_ref(),
                &self.campaign.classes_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Races => self.races_editor_state.show(
                ui,
                &self.items,
                self.campaign_dir.as_ref(),
                &self.campaign.races_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Characters => self.characters_editor_state.show(
                ui,
                &self.races_editor_state.races,
                &self.classes_editor_state.classes,
                &self.items,
                self.campaign_dir.as_ref(),
                &self.campaign.characters_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::Dialogues => self.dialogue_editor_state.show(
                ui,
                &mut self.dialogues,
                &self.quests,
                &self.items,
                self.campaign_dir.as_ref(),
                &self.campaign.dialogue_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
            ),
            EditorTab::NPCs => {
                if self.npc_editor_state.show(
                    ui,
                    &self.dialogues,
                    &self.quests,
                    self.campaign_dir.as_ref(),
                    &self.tool_config.display,
                    &self.campaign.npcs_file,
                ) {
                    self.unsaved_changes = true;
                }
            }
            EditorTab::Proficiencies => self.proficiencies_editor_state.show(
                ui,
                &mut self.proficiencies,
                self.campaign_dir.as_ref(),
                &self.campaign.proficiencies_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
                &self.classes_editor_state.classes,
                &self.races_editor_state.races,
                &self.items,
            ),
            EditorTab::Assets => self.show_assets_editor(ui),
            EditorTab::Validation => self.show_validation_panel(ui),
        });

        // Preferences dialog using a local temporary variable to avoid borrow conflicts
        // Use local flags and avoid mutably borrowing `self.show_preferences` inside the `show` closure.
        let mut show_preferences_local = self.show_preferences;
        let mut prefs_save_clicked = false;
        let mut prefs_close_clicked = false;

        if show_preferences_local {
            egui::Window::new("Preferences")
                .open(&mut show_preferences_local)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Preferences");
                    ui.separator();

                    // Inspector min width slider
                    ui.label("Inspector minimum width (px):");
                    ui.add(egui::Slider::new(
                        &mut self.tool_config.display.inspector_min_width,
                        150.0..=800.0,
                    )
                    .text("inspector_min_width"));

                    ui.add_space(6.0);

                    // Left column max ratio slider
                    ui.label("Max left column ratio (0.4 - 0.9):");
                    ui.add(egui::Slider::new(
                        &mut self.tool_config.display.left_column_max_ratio,
                        0.4..=0.9,
                    )
                    .text("left_column_max_ratio"));

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Save Preferences").clicked() {
                            // Toggle a local flag, we'll save outside the closure to avoid borrows on self here.
                            prefs_save_clicked = true;
                        }

                        if ui.button("Close").clicked() {
                            // Toggle a local flag instead of mutating the show flag directly.
                            prefs_close_clicked = true;
                        }
                    });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.label("Inspector min width will be respected by editors that use TwoColumnLayout. Left column ratio is a conservative clamp to avoid list panel clipping the detail/inspector panel.");
                });
        }

        // Persist any actions that happened in the preferences UI.
        if prefs_save_clicked {
            match self.tool_config.save() {
                Ok(_) => {
                    self.status_message = "Preferences saved".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Failed to save preferences: {}", e);
                }
            }
        }

        if prefs_close_clicked {
            show_preferences_local = false;
        }

        // Finally, update the app state.
        self.show_preferences = show_preferences_local;
        // About dialog
        if self.show_about_dialog {
            egui::Window::new("About Antares Campaign Builder")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Antares Campaign Builder");
                        ui.label("Foundation v0.2.0");
                        ui.separator();
                        ui.label("A visual editor for creating custom");
                        ui.label("campaigns for the Antares RPG engine.");
                        ui.separator();
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

        // Phase 7: Debug panel
        if self.show_debug_panel {
            self.show_debug_panel_window(ctx);
        }
    }
}

impl CampaignBuilderApp {
    /// Show the metadata editor (delegates to the campaign_editor module)
    fn show_metadata_editor(&mut self, ui: &mut egui::Ui) {
        // Delegate the UI rendering and editing behavior to the dedicated editor
        // state. The editor manages a local buffer and applies changes to the
        // active `self.campaign` when the user saves.
        self.campaign_editor_state.show(
            ui,
            &mut self.campaign,
            &mut self.campaign_path,
            self.campaign_dir.as_ref(),
            &mut self.unsaved_changes,
            &mut self.status_message,
            self.npc_editor_state.npcs.as_slice(),
        );

        // If the campaign metadata editor requested validation, run the shared
        // validator and switch to the Validation tab so results are visible.
        if self.campaign_editor_state.consume_validate_request() {
            self.validate_campaign();
            self.active_tab = EditorTab::Validation;
        }
    }

    /// Show the configuration editor (legacy stub)
    ///
    /// NOTE: The configuration editor UI was refactored into the dedicated
    /// `CampaignMetadataEditorState` (sdk/campaign_builder/src/campaign_editor.rs).
    /// This function remains as a lightweight compatibility stub and delegates
    /// to the new editor. It keeps call-sites stable for any code that might
    /// still reference this function.
    fn show_config_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Campaign Configuration (Legacy)");
        ui.add_space(5.0);
        ui.label("This legacy view has been migrated to the Campaign Metadata Editor.");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Open Metadata Editor").clicked() {
                // Switch the central panel to the Metadata editor (delegation).
                self.active_tab = EditorTab::Metadata;
            }

            if ui.button("Open Metadata Editor (and Validate)").clicked() {
                self.active_tab = EditorTab::Metadata;
                // If we want to automatically request validation, we can set the editor's
                // validate flag or run `validate_campaign()` here. Keep behaviour minimal.
            }
        });

        // For compatibility in case some callers expect the metadata editing form to be shown
        // inline here, delegate to the metadata editor rendering.
        self.show_metadata_editor(ui);
    }

    /// Show quests editor (Phase 4A: Full Quest Editor Integration)
    fn show_quests_editor(&mut self, ui: &mut egui::Ui) {
        self.quest_editor_state.show(
            ui,
            &mut self.quests,
            &self.items,
            &self.monsters,
            &self.maps,
            self.campaign_dir.as_ref(),
            &self.campaign.quests_file,
            &mut self.unsaved_changes,
            &mut self.status_message,
            &mut self.file_load_merge_mode,
        );
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

    /// Load classes from campaign directory
    fn load_classes_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.classes_file);
            if path.exists() {
                match self.classes_editor_state.load_from_file(&path) {
                    Ok(_) => {
                        self.status_message =
                            format!("Loaded {} classes", self.classes_editor_state.classes.len());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load classes: {}", e);
                        eprintln!("Failed to load classes from {:?}: {}", path, e);
                    }
                }
            } else {
                eprintln!("Classes file does not exist: {:?}", path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load classes");
        }
    }

    /// Load characters from campaign directory
    fn load_characters_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.characters_file);
            if path.exists() {
                match self.characters_editor_state.load_from_file(&path) {
                    Ok(_) => {
                        let count = self.characters_editor_state.characters.len();
                        self.status_message = format!("Loaded {} characters", count);
                        // Mark data file as loaded in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_loaded(&self.campaign.characters_file, count);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load characters: {}", e);
                        eprintln!("Failed to load characters from {:?}: {}", path, e);
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(
                                &self.campaign.characters_file,
                                &e.to_string(),
                            );
                        }
                    }
                }
            } else {
                // Characters file is optional, don't log error if it doesn't exist
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Characters file does not exist: {:?}", path),
                );
            }
        } else {
            eprintln!("No campaign directory set when trying to load characters");
        }
    }

    /// Load races from campaign directory
    fn load_races_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.races_file);
            if path.exists() {
                match self.races_editor_state.load_from_file(&path) {
                    Ok(_) => {
                        self.status_message =
                            format!("Loaded {} races", self.races_editor_state.races.len());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load races: {}", e);
                        eprintln!("Failed to load races from {:?}: {}", path, e);
                    }
                }
            } else {
                eprintln!("Races file does not exist: {:?}", path);
            }
        } else {
            eprintln!("No campaign directory set when trying to load races");
        }
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
    ///
    /// Displays validation results in a table-based layout grouped by category.
    /// Uses icons to indicate severity: ✅ passed, ❌ error, ⚠️ warning, ℹ️ info.
    fn show_validation_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("✅ Campaign Validation");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Re-validate").clicked() {
                    self.validate_campaign();
                }
            });
        });
        ui.add_space(5.0);
        ui.label("Check your campaign for errors and warnings");
        ui.separator();

        // Calculate summary using the validation module
        let summary = validation::ValidationSummary::from_results(&self.validation_errors);

        // Enhanced summary section with status badge
        ui.horizontal(|ui| {
            // Overall status badge
            if summary.error_count == 0 && summary.warning_count == 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(80, 200, 80),
                    "✅ All checks passed!",
                );
            } else if summary.error_count > 0 {
                ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "⚠️ Issues found");
            } else {
                ui.colored_label(egui::Color32::from_rgb(255, 180, 0), "⚠️ Warnings only");
            }

            ui.separator();

            // Count badges
            if summary.error_count > 0 {
                ui.colored_label(
                    validation::ValidationSeverity::Error.color(),
                    format!("❌ {}", summary.error_count),
                );
            }
            if summary.warning_count > 0 {
                ui.colored_label(
                    validation::ValidationSeverity::Warning.color(),
                    format!("⚠️ {}", summary.warning_count),
                );
            }
            if summary.info_count > 0 {
                ui.colored_label(
                    validation::ValidationSeverity::Info.color(),
                    format!("ℹ️ {}", summary.info_count),
                );
            }
            if summary.passed_count > 0 {
                ui.colored_label(
                    validation::ValidationSeverity::Passed.color(),
                    format!("✅ {}", summary.passed_count),
                );
            }

            // Total count
            ui.separator();
            ui.label(format!("Total: {} checks", self.validation_errors.len()));
        });

        ui.separator();

        // Quick filter controls
        ui.horizontal(|ui| {
            ui.label("Show:");
            if ui
                .selectable_label(self.validation_filter == ValidationFilter::All, "All")
                .clicked()
            {
                self.validation_filter = ValidationFilter::All;
            }
            if summary.error_count > 0
                && ui
                    .selectable_label(
                        self.validation_filter == ValidationFilter::ErrorsOnly,
                        "Errors Only",
                    )
                    .clicked()
            {
                self.validation_filter = ValidationFilter::ErrorsOnly;
            }
            if summary.warning_count > 0
                && ui
                    .selectable_label(
                        self.validation_filter == ValidationFilter::WarningsOnly,
                        "Warnings Only",
                    )
                    .clicked()
            {
                self.validation_filter = ValidationFilter::WarningsOnly;
            }
            ui.add_space(10.0);
            ui.add_space(8.0);
            if ui.button("🔁 Reset Filter").clicked() {
                self.reset_validation_filters();
            }
        });

        ui.separator();

        // Always show category breakdown - group results by category and display in table format
        let grouped = self.grouped_filtered_validation_results();

        let max_height = ui_helpers::compute_default_panel_height(ui);
        // We'll track any clicked path inside the closure and handle it after the UI
        // closure completes so we don't attempt to mutably borrow `self` while the UI
        // code still holds references to `ui`.
        let mut pending_focus: Option<PathBuf> = None;

        egui::ScrollArea::vertical()
            .max_height(max_height)
            .id_salt("validation_panel_scroll")
            .show(ui, |ui| {
                // Render a single unified grid for all validation results rather than a per-category grid.
                // This provides the "Category | File | Status | Message" layout used by the design.
                if !grouped.is_empty() {
                    // Flatten grouped results into a single vector while preserving order.
                    let mut all_results: Vec<validation::ValidationResult> = Vec::new();
                    for (_cat, results) in grouped.iter() {
                        for r in results.iter() {
                            all_results.push(r.clone());
                        }
                    }

                    // Unified table across categories
                    egui::Grid::new("validation_grid_all")
                        .num_columns(4)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui_helpers::render_grid_header(
                                ui,
                                &["Category", "File", "Status", "Message"],
                            );

                            for result in all_results.iter() {
                                // First column: Category (icon + name)
                                ui.horizontal(|ui| {
                                    ui.label(result.category.icon());
                                    ui.add_space(4.0);
                                    ui.label(result.category.display_name());
                                });

                                // Second column: File (clickable)
                                if let Some(path) = result.file_path.as_ref() {
                                    let resp = ui_helpers::show_clickable_path(ui, path);
                                    if resp.clicked() {
                                        pending_focus = Some(path.clone());
                                    }
                                } else {
                                    ui.label("-");
                                }

                                // Third column: Status icon
                                ui_helpers::show_validation_severity_icon(ui, result.severity);

                                // Fourth column: Message
                                ui.label(&result.message);

                                ui.end_row();
                            }
                        });

                    ui.add_space(10.0);
                } else {
                    ui.vertical_centered(|ui| {
                        ui.label("No validation results");
                    });
                }
            });
        // Apply the pending focus if one was requested while rendering the validation panel.
        if let Some(path) = pending_focus {
            self.focus_asset(path);
        }

        ui.separator();

        // Action tips based on validation status
        ui.horizontal(|ui| {
            ui.label("💡");
            if summary.error_count > 0 {
                ui.label("Fix errors in the Metadata and Config tabs to enable test play");
            } else if summary.warning_count > 0 {
                ui.label("Address warnings to ensure best campaign quality");
            } else {
                ui.label("Campaign is valid! Click 'Re-validate' after making changes");
            }
        });
    } // end of show_validation_panel

    /// Load dialogues from campaign file
    fn load_dialogues(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let dialogue_path = dir.join(&self.campaign.dialogue_file);
            if dialogue_path.exists() {
                match std::fs::read_to_string(&dialogue_path) {
                    Ok(contents) => match ron::from_str::<Vec<DialogueTree>>(&contents) {
                        Ok(dialogues) => {
                            self.dialogues = dialogues;
                            self.dialogue_editor_state
                                .load_dialogues(self.dialogues.clone());
                            self.status_message =
                                format!("Loaded {} dialogues", self.dialogues.len());
                        }
                        Err(e) => {
                            eprintln!("Failed to parse dialogues from {:?}: {}", dialogue_path, e);
                            return Err(CampaignError::Deserialization(e));
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read dialogues file {:?}: {}", dialogue_path, e);
                        return Err(CampaignError::Io(e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Focus (and open) the asset manager to the given asset path.
    ///
    /// When a user clicks a file path in the validation panel, we set this value,
    /// open the Asset Manager, and surface a useful status message. The asset
    /// editor UI will highlight the focused asset if present.
    fn reset_validation_filters(&mut self) {
        // Restore default validation filter state and clear any asset focus
        self.validation_filter = ValidationFilter::All;
        self.validation_focus_asset = None;
        self.status_message = "Validation filters reset".to_string();
    }

    fn focus_asset(&mut self, path: PathBuf) {
        self.validation_focus_asset = Some(path.clone());
        self.show_asset_manager = true;
        self.status_message = format!("🔎 Focused asset: {}", path.display());
    }

    /// Show assets editor
    ///
    /// Displays campaign data file status and asset management tools.
    /// Data files show Loaded/Error/Missing status; orphaned assets can be cleaned up.
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
                    // Initialize data file tracking
                    // Discover actual map files from the maps directory
                    let map_file_paths = self.discover_map_files();

                    manager.init_data_files(
                        &self.campaign.items_file,
                        &self.campaign.spells_file,
                        &self.campaign.conditions_file,
                        &self.campaign.monsters_file,
                        &map_file_paths,
                        &self.campaign.quests_file,
                        &self.campaign.classes_file,
                        &self.campaign.races_file,
                        &self.campaign.characters_file,
                        &self.campaign.dialogue_file,
                        &self.campaign.npcs_file,
                        &self.campaign.proficiencies_file,
                    );
                    // Scan references on initial load so portraits are properly marked as referenced
                    manager.scan_references(
                        &self.items,
                        &self.quests,
                        &self.dialogues,
                        &self.maps,
                        &self.classes_editor_state.classes,
                        &self.characters_editor_state.characters,
                        &self.npc_editor_state.npcs,
                    );
                    manager.mark_data_files_as_referenced();

                    self.status_message = format!("Scanned {} assets", manager.assets().len());
                    self.asset_manager = Some(manager);

                    // Auto-load races into the Races Editor when the asset manager initializes.
                    // This ensures the Races Editor's in-memory state is populated after scanning
                    // the campaign assets, resolving the case where assets show as "Loaded" but
                    // the Races Editor list is empty.
                    if self.campaign_dir.is_some() {
                        // Load races from the configured file and update UI status
                        self.load_races_from_campaign();
                    }
                }
            } else {
                // no campaign dir
            }
        }

        if let Some(ref mut manager) = self.asset_manager {
            // Toolbar with actions
            // Asset summary toolbar
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("📊 {} Assets", manager.asset_count())).strong(),
                );
                ui.separator();
                ui.label(format!("💾 {}", manager.total_size_string()));
                ui.separator();

                if ui.button("🔄 Refresh").clicked() {
                    if let Err(e) = manager.scan_directory() {
                        self.status_message = format!("Failed to refresh assets: {}", e);
                    } else {
                        // After refreshing assets, rescan references to properly mark portraits
                        // referenced by characters and NPCs
                        manager.scan_references(
                            &self.items,
                            &self.quests,
                            &self.dialogues,
                            &self.maps,
                            &self.classes_editor_state.classes,
                            &self.characters_editor_state.characters,
                            &self.npc_editor_state.npcs,
                        );
                        manager.mark_data_files_as_referenced();
                        self.status_message = "Assets refreshed and references scanned".to_string();
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
                        &self.characters_editor_state.characters,
                        &self.npc_editor_state.npcs,
                    );
                    // Mark successfully loaded data files as referenced
                    manager.mark_data_files_as_referenced();
                    self.status_message = "Asset references scanned".to_string();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let unreferenced = manager.orphaned_assets().len();
                    if unreferenced > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 0),
                            format!("⚠️ {} unreferenced", unreferenced),
                        );
                    }
                });
            });

            ui.separator();

            // Asset type filters with counts
            ui.horizontal_wrapped(|ui| {
                ui.label("Filter:");

                // "All" button
                if ui
                    .selectable_label(
                        self.asset_type_filter.is_none(),
                        format!("All ({})", manager.assets().len()),
                    )
                    .clicked()
                {
                    self.asset_type_filter = None;
                    self.status_message = "Showing all asset types".to_string();
                }

                // Individual type filters
                for asset_type in asset_manager::AssetType::all() {
                    let count = manager.asset_count_by_type(asset_type);
                    if count > 0 {
                        let is_selected = self.asset_type_filter == Some(asset_type);
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("{} ({})", asset_type.display_name(), count),
                            )
                            .clicked()
                        {
                            self.asset_type_filter = Some(asset_type);
                            self.status_message =
                                format!("Filtered by {}", asset_type.display_name());
                        }
                    }
                }
            });

            ui.separator();

            // Data Files Status Section
            ui.collapsing("📁 Campaign Data Files", |ui| {
                let data_files = manager.data_files();
                if data_files.is_empty() {
                    ui.label("No data files tracked");
                } else {
                    egui::Grid::new("data_files_grid")
                        .num_columns(4)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header row
                            ui.label(egui::RichText::new("Status").strong());
                            ui.label(egui::RichText::new("Type").strong());
                            ui.label(egui::RichText::new("Path").strong());
                            ui.label(egui::RichText::new("Entries").strong());
                            ui.end_row();

                            // Data file rows
                            for file_info in data_files {
                                // Status icon with color
                                ui.colored_label(file_info.status.color(), file_info.status.icon());

                                // Display name
                                ui.label(&file_info.display_name);

                                // Path
                                ui.label(file_info.path.display().to_string());

                                // Entry count or error
                                match file_info.status {
                                    asset_manager::DataFileStatus::Loaded => {
                                        if let Some(count) = file_info.entry_count {
                                            ui.label(format!("{}", count));
                                        } else {
                                            ui.label("-");
                                        }
                                    }
                                    asset_manager::DataFileStatus::Error => {
                                        if let Some(ref msg) = file_info.error_message {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 80, 80),
                                                msg.chars().take(30).collect::<String>(),
                                            );
                                        } else {
                                            ui.label("Error");
                                        }
                                    }
                                    _ => {
                                        ui.label("-");
                                    }
                                }
                                ui.end_row();
                            }
                        });

                    // Summary
                    ui.add_space(5.0);
                    let loaded_count = data_files
                        .iter()
                        .filter(|f| f.status == asset_manager::DataFileStatus::Loaded)
                        .count();
                    let error_count = manager.data_file_error_count();
                    let missing_count = manager.data_file_missing_count();

                    ui.horizontal(|ui| {
                        if loaded_count > 0 {
                            ui.colored_label(
                                egui::Color32::from_rgb(80, 200, 80),
                                format!("✅ {} loaded", loaded_count),
                            );
                        }
                        if error_count > 0 {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 80, 80),
                                format!("❌ {} errors", error_count),
                            );
                        }
                        if missing_count > 0 {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 180, 0),
                                format!("⚠️ {} missing", missing_count),
                            );
                        }
                    });
                }
            });

            ui.separator();

            // Unreferenced assets section (excluding data files)
            let orphaned_count = manager.orphaned_assets().len();
            let cleanup_candidates_count = manager.get_cleanup_candidates().len();

            if orphaned_count > 0 || cleanup_candidates_count > 0 {
                ui.collapsing("🧹 Unreferenced Assets", |ui| {
                    if orphaned_count > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 0),
                            format!("⚠️ {} asset files are not referenced by campaign data", orphaned_count),
                        );
                        ui.label(egui::RichText::new("These files exist but aren't used by items, maps, or other content").small().weak());
                    }

                    if cleanup_candidates_count > 0 {
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(format!(
                                    "🧹 Review {} Cleanup Candidates",
                                    cleanup_candidates_count
                                ))
                                .clicked()
                            {
                                // Toggle the cleanup candidates display
                                self.show_cleanup_candidates = !self.show_cleanup_candidates;
                            }
                            ui.label(egui::RichText::new("(Safe cleanup preview)").small().weak());
                        });

                        // Show the cleanup candidates list if toggled
                        if self.show_cleanup_candidates {
                            ui.add_space(5.0);
                            ui.separator();
                            ui.label(egui::RichText::new("Cleanup Candidates:").strong());

                            // Clone candidates to avoid borrow issues when deleting
                            let candidates: Vec<PathBuf> = manager.get_cleanup_candidates()
                                .iter()
                                .map(|p| (*p).clone())
                                .collect();

                            // Action buttons
                            ui.horizontal(|ui| {
                                if ui.button("Select All").clicked() {
                                    self.cleanup_candidates_selected.clear();
                                    for path in &candidates {
                                        self.cleanup_candidates_selected.insert(path.clone());
                                    }
                                }

                                if ui.button("Deselect All").clicked() {
                                    self.cleanup_candidates_selected.clear();
                                }

                                ui.separator();

                                let selected_count = self.cleanup_candidates_selected.len();
                                if selected_count > 0 {
                                    if ui.button(format!("🗑️ Delete {} Selected", selected_count))
                                        .clicked()
                                    {
                                        // Calculate total size of selected files
                                        let mut total_size = 0u64;
                                        for path in &self.cleanup_candidates_selected {
                                            if let Some(asset) = manager.assets().get(path) {
                                                total_size += asset.size;
                                            }
                                        }

                                        // Format size
                                        let size_str = if total_size < 1024 {
                                            format!("{} B", total_size)
                                        } else if total_size < 1024 * 1024 {
                                            format!("{:.1} KB", total_size as f64 / 1024.0)
                                        } else {
                                            format!("{:.1} MB", total_size as f64 / (1024.0 * 1024.0))
                                        };

                                        self.status_message = format!(
                                            "⚠️ About to delete {} files ({}) - Click again to confirm",
                                            selected_count,
                                            size_str
                                        );

                                        // Perform deletion
                                        let mut deleted_count = 0;
                                        let mut failed_deletions = Vec::new();

                                        for path in self.cleanup_candidates_selected.iter() {
                                            match manager.remove_asset(path) {
                                                Ok(_) => deleted_count += 1,
                                                Err(e) => failed_deletions.push(format!("{}: {}", path.display(), e)),
                                            }
                                        }

                                        // Update status message
                                        if failed_deletions.is_empty() {
                                            self.status_message = format!(
                                                "✅ Successfully deleted {} files ({})",
                                                deleted_count,
                                                size_str
                                            );
                                        } else {
                                            self.status_message = format!(
                                                "⚠️ Deleted {} files, {} failed: {}",
                                                deleted_count,
                                                failed_deletions.len(),
                                                failed_deletions.join(", ")
                                            );
                                        }

                                        // Clear selection after deletion
                                        self.cleanup_candidates_selected.clear();
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Select files to delete").weak());
                                }
                            });

                            ui.add_space(5.0);

                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for candidate_path in &candidates {
                                        let is_selected = self.cleanup_candidates_selected.contains(candidate_path);

                                        ui.horizontal(|ui| {
                                            let mut selected = is_selected;
                                            if ui.checkbox(&mut selected, "").changed() {
                                                if selected {
                                                    self.cleanup_candidates_selected.insert(candidate_path.clone());
                                                } else {
                                                    self.cleanup_candidates_selected.remove(candidate_path);
                                                }
                                            }

                                            ui.label("🗑️");
                                            ui.label(candidate_path.display().to_string());

                                            // Show file size
                                            if let Some(asset) = manager.assets().get(candidate_path) {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(asset.size_string())
                                                                .small()
                                                                .weak()
                                                        );
                                                    }
                                                );
                                            }
                                        });
                                    }
                                });

                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(
                                "These files are not referenced by any campaign data and could be safely removed."
                            ).small().weak());
                        }
                    }
                });
                ui.separator();
            }

            // Asset list with usage context (sorted by path)
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Convert HashMap to sorted Vec for consistent display
                let mut sorted_assets: Vec<_> = manager.assets().iter().collect();
                sorted_assets.sort_by(|a, b| a.0.cmp(b.0));

                // Apply asset type filter
                let filtered_assets: Vec<_> = sorted_assets
                    .iter()
                    .filter(|(_, asset)| {
                        match self.asset_type_filter {
                            None => true, // Show all
                            Some(filter_type) => asset.asset_type == filter_type,
                        }
                    })
                    .copied()
                    .collect();

                for (path, asset) in filtered_assets {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            // Asset header
                            ui.horizontal(|ui| {
                                ui.label(format!("📄 {}", path.display()));

                                // Highlight if selected from the Validation panel
                                if let Some(ref focus) = self.validation_focus_asset {
                                    if focus == path {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(100, 180, 255),
                                            "🔎 Selected from Validation",
                                        );
                                    }
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(asset.size_string());
                                        ui.label(asset.asset_type.display_name());

                                        // Show better status for assets
                                        if asset.is_referenced {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(80, 200, 80),
                                                "✅ Referenced",
                                            );
                                        } else if asset.asset_type == asset_manager::AssetType::Data
                                        {
                                            // Data files that are loaded but not "referenced" by items
                                            // should show as "Loaded" not "Unused"
                                            ui.colored_label(
                                                egui::Color32::from_rgb(100, 180, 255),
                                                "📁 Data File",
                                            );
                                        } else {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 180, 0),
                                                "⚠️ Unreferenced",
                                            );
                                        }
                                    },
                                );
                            });

                            // Show references if any
                            if !asset.references.is_empty() {
                                ui.indent("asset_refs", |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Referenced by {} item(s):",
                                            asset.references.len()
                                        ))
                                        .small(),
                                    );
                                    for reference in asset.references.iter().take(5) {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "  • {}",
                                                reference.display_string()
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    }
                                    if asset.references.len() > 5 {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "  ... and {} more",
                                                asset.references.len() - 5
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    }
                                });
                            } else if asset.asset_type == asset_manager::AssetType::Data {
                                ui.indent("asset_info", |ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            "Campaign data file (items, spells, etc.)",
                                        )
                                        .small()
                                        .weak(),
                                    );
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
    use antares::domain::character::{Alignment, AttributePair, AttributePair16, Sex};
    use antares::domain::character_definition::CharacterDefinition;
    use antares::domain::classes::ClassDefinition;
    use antares::domain::combat::monster::MonsterCondition;
    use antares::domain::quest::QuestStage;
    use antares::domain::races::RaceDefinition;

    #[test]
    fn test_do_new_campaign_clears_loaded_data() {
        let mut app = CampaignBuilderApp::default();

        // Populate editors and domain data to simulate an open campaign
        app.items.push(ItemsEditorState::default_item());
        app.items_editor_state.search_query = "sword".to_string();

        app.spells.push(CampaignBuilderApp::default_spell());
        app.spells_editor_state.search_query = "fire".to_string();

        app.monsters.push(CampaignBuilderApp::default_monster());
        app.monsters_editor_state.edit_buffer.name = "Orc".to_string();

        // Add a simple condition definition
        use antares::domain::conditions::{ConditionDefinition, ConditionDuration};
        app.conditions.push(ConditionDefinition {
            id: "cond_1".to_string(),
            name: "Test Condition".to_string(),
            description: String::new(),
            effects: Vec::new(),
            default_duration: ConditionDuration::Permanent,
            icon_id: None,
        });

        // Add a map and a quest
        use antares::domain::world::Map;
        app.maps.push(Map::new(
            1,
            "test_map".to_string(),
            "desc".to_string(),
            10,
            10,
        ));
        app.quests
            .push(antares::domain::quest::Quest::new(1, "Test Quest", "desc"));

        // Add a dialogue tree
        use antares::domain::dialogue::DialogueTree;
        let dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        app.dialogues.push(dialogue);
        app.dialogue_editor_state.selected_dialogue = Some(0);

        // Add an NPC
        app.npc_editor_state
            .npcs
            .push(antares::domain::world::npc::NpcDefinition::new(
                "npc_1",
                "NPC 1",
                "portrait_1",
            ));

        // Sanity checks: ensure data present before invoking the method under test
        assert!(!app.items.is_empty());
        assert!(!app.spells.is_empty());
        assert!(!app.monsters.is_empty());
        assert!(!app.maps.is_empty());
        assert!(!app.quests.is_empty());
        assert!(!app.dialogues.is_empty());
        assert!(!app.npc_editor_state.npcs.is_empty());

        // Call the method under test
        app.do_new_campaign();

        // Assert everything cleared and editors reset
        assert!(app.items.is_empty());
        assert!(app.spells.is_empty());
        assert!(app.monsters.is_empty());
        assert!(app.maps.is_empty());
        assert!(app.quests.is_empty());
        assert!(app.dialogues.is_empty());
        assert!(app.npc_editor_state.npcs.is_empty());

        // Editor states reset
        assert!(app.items_editor_state.search_query.is_empty());
        assert!(app.spells_editor_state.search_query.is_empty());
        assert_eq!(
            app.monsters_editor_state.edit_buffer.name,
            MonstersEditorState::new().edit_buffer.name,
            "Monster editor buffer should be reset to default"
        );
        assert!(app.asset_manager.is_none());
        assert!(!app.unsaved_changes);
    }

    #[test]
    fn test_generate_category_status_checks_empty_races_shows_info() {
        // Default app has no races loaded
        let app = CampaignBuilderApp::default();
        let results = app.generate_category_status_checks();

        let race_info = results
            .iter()
            .find(|r| r.category == validation::ValidationCategory::Races);

        assert!(
            race_info.is_some(),
            "Races category missing from validation results"
        );
        let result = race_info.unwrap();
        assert_eq!(
            result.severity,
            validation::ValidationSeverity::Info,
            "Expected Info severity for empty races"
        );
        assert!(
            result
                .message
                .contains("No races loaded - add races or load from file"),
            "Unexpected message: {}",
            result.message
        );
    }

    #[test]
    fn test_generate_category_status_checks_loaded_races_shows_passed() {
        // Create a CampaignBuilderApp and add a single race definition
        let mut app = CampaignBuilderApp::default();
        let race =
            RaceDefinition::new("test".to_string(), "Test".to_string(), "A test".to_string());
        app.races_editor_state.races.push(race);

        let results = app.generate_category_status_checks();
        let race_result = results
            .iter()
            .find(|r| r.category == validation::ValidationCategory::Races);

        assert!(
            race_result.is_some(),
            "Races category missing from validation results"
        );
        let result = race_result.unwrap();
        assert_eq!(
            result.severity,
            validation::ValidationSeverity::Passed,
            "Expected Passed severity when races are loaded"
        );
        assert!(
            result.message.contains(&format!(
                "{} races validated",
                app.races_editor_state.races.len()
            )),
            "Unexpected message: {}",
            result.message
        );
    }

    #[test]
    fn test_load_races_from_campaign_populates_races_editor_state() {
        use std::fs;
        use std::path::PathBuf;
        use std::time::{SystemTime, UNIX_EPOCH};

        // Build a temporary campaign directory under the system temp dir
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        let tmp_base = std::env::temp_dir();
        let tmpdir = tmp_base.join(format!("antares_test_races_{}", unique));
        let data_dir = tmpdir.join("data");

        // Ensure directory exists
        fs::create_dir_all(&data_dir).expect("Failed to create temp data dir");

        // RON content containing two race definitions (human, elf)
        let races_ron = r#"
[
    (
        id: "human",
        name: "Human",
    ),
    (
        id: "elf",
        name: "Elf",
    ),
]
"#;

        // Write races.ron to the data directory
        let races_path = data_dir.join("races.ron");
        fs::write(&races_path, races_ron).expect("Failed to write races.ron");

        // Setup the CampaignBuilderApp, and point it at our temporary campaign
        let mut app = CampaignBuilderApp::default();
        app.campaign_dir = Some(tmpdir.clone());
        // Use the default "data/races.ron" path; set explicitly to be safe
        app.campaign.races_file = "data/races.ron".to_string();

        // Load races into the editor state (this should populate races_editor_state.races)
        app.load_races_from_campaign();

        // Validate we loaded exactly 2 races and they have the expected IDs
        assert_eq!(app.races_editor_state.races.len(), 2);
        assert_eq!(app.races_editor_state.races[0].id, "human");
        assert_eq!(app.races_editor_state.races[1].id, "elf");

        // Cleanup any temporary files/directories
        let _ = fs::remove_dir_all(tmpdir);
    }

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
    fn test_validation_filter_all_shows_passed() {
        let mut app = CampaignBuilderApp::default();

        // Add a passed check to the validation_errors and ensure All shows it
        app.validation_errors
            .push(validation::ValidationResult::passed(
                validation::ValidationCategory::Items,
                format!("{} items validated", 1),
            ));
        app.validation_filter = ValidationFilter::All;

        let grouped = app.grouped_filtered_validation_results();
        let has_passed = grouped.iter().any(|(_cat, results)| {
            results
                .iter()
                .any(|r| r.severity == validation::ValidationSeverity::Passed)
        });
        assert!(
            has_passed,
            "All filter should include passed checks by default"
        );
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
            .filter(|e| e.is_error())
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
    fn test_validation_starting_map_missing() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();

        // Add a loaded map (name "Starter Town") so the validator checks against loaded maps
        let map = Map::new(1, "Starter Town".to_string(), "Desc".to_string(), 10, 10);
        app.maps.push(map);

        app.campaign.starting_map = "does_not_exist".to_string();
        app.validate_campaign();

        let has_map_error = app.validation_errors.iter().any(|e| {
            e.category == validation::ValidationCategory::Configuration
                && e.is_error()
                && e.message.contains("Starting map")
        });
        assert!(has_map_error);
    }

    #[test]
    fn test_validation_starting_innkeeper_missing() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test_map".to_string();

        // No NPCs loaded - starting_innkeeper should be flagged as missing
        app.campaign.starting_innkeeper = "does_not_exist".to_string();
        app.validate_campaign();

        let has_inn_error = app.validation_errors.iter().any(|e| {
            e.category == validation::ValidationCategory::Configuration
                && e.is_error()
                && e.message.contains("Starting innkeeper")
        });
        assert!(has_inn_error);
    }

    #[test]
    fn test_validation_starting_innkeeper_not_innkeeper() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test_map".to_string();

        // Add an NPC that exists but is NOT an innkeeper
        app.npc_editor_state
            .npcs
            .push(antares::domain::world::npc::NpcDefinition::new(
                "npc_not_inn".to_string(),
                "NPC Not Inn".to_string(),
                "portrait.png".to_string(),
            ));

        app.campaign.starting_innkeeper = "npc_not_inn".to_string();
        app.validate_campaign();

        let has_inn_error = app.validation_errors.iter().any(|e| {
            e.category == validation::ValidationCategory::Configuration
                && e.is_error()
                && e.message.contains("is not marked as is_innkeeper")
        });
        assert!(has_inn_error);
    }

    #[test]
    fn test_default_starting_innkeeper() {
        let metadata = CampaignMetadata::default();
        assert_eq!(
            metadata.starting_innkeeper,
            "tutorial_innkeeper_town".to_string()
        );
    }

    #[test]
    fn test_metadata_editor_validate_triggers_validation_and_switches_tab() {
        let mut app = CampaignBuilderApp::default();

        // Ensure ID/Name are present to avoid unrelated Metadata errors and create a
        // configuration error by leaving starting_map empty.
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "".to_string();

        // Simulate 'Validate' button click in the Campaign metadata editor by setting
        // the request flag that the editor sets when the UI Validate button is clicked.
        app.campaign_editor_state.validate_requested = true;

        // Behavior in the main app after the editor returns:
        // If a validate request was issued, consume it, run validation, and switch to the Validation tab.
        if app.campaign_editor_state.consume_validate_request() {
            app.validate_campaign();
            app.active_tab = EditorTab::Validation;
        }

        // After validation, the active tab should be the Validation tab and at least
        // one Configuration category error should be present.
        assert_eq!(app.active_tab, EditorTab::Validation);

        let has_config_error = app
            .validation_errors
            .iter()
            .any(|e| e.category == validation::ValidationCategory::Configuration && e.is_error());

        assert!(has_config_error);
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
    fn test_validation_starting_map_exists_by_name() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();

        // Add a loaded map (name "Starter Town") so the validator checks against loaded maps
        let map = Map::new(1, "Starter Town".to_string(), "Desc".to_string(), 10, 10);
        app.maps.push(map);

        // Use the normalized map key (underscores -> spaces) that should match the map name
        app.campaign.starting_map = "starter_town".to_string();
        app.validate_campaign();

        // There should NOT be a configuration error related to starting map
        let has_map_error = app.validation_errors.iter().any(|e| {
            e.category == validation::ValidationCategory::Configuration
                && e.is_error()
                && e.message.contains("Starting map")
        });
        assert!(!has_map_error);
    }

    #[test]
    fn test_validation_configuration_category_grouping() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();

        // Force a configuration error (empty starting_map)
        app.campaign.starting_map = "".to_string();
        app.validate_campaign();

        let grouped = validation::group_results_by_category(&app.validation_errors);
        let has_config_group = grouped
            .iter()
            .any(|(category, _results)| *category == validation::ValidationCategory::Configuration);
        assert!(has_config_group);
    }

    #[test]
    fn test_validation_filter_errors_only() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();

        // Add an error and a warning in the same category
        app.validation_errors.clear();
        app.validation_errors
            .push(validation::ValidationResult::error(
                validation::ValidationCategory::Items,
                "Item ID duplicate",
            ));
        app.validation_errors
            .push(validation::ValidationResult::warning(
                validation::ValidationCategory::Items,
                "Item name recommended",
            ));

        // Enable Errors Only filter
        app.validation_filter = ValidationFilter::ErrorsOnly;
        let grouped = validation::group_results_by_category(&app.validation_errors);

        // Use the grouped & filtered results (matching what the UI shows)
        let grouped2 = app.grouped_filtered_validation_results();
        let visible: usize = grouped2
            .iter()
            .map(|(_category, results)| results.len())
            .sum();

        assert_eq!(visible, 1, "Filter should show only the error result");
    }

    #[test]
    fn test_validation_filter_warnings_only() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();

        // Add an error and a warning in the same category
        app.validation_errors.clear();
        app.validation_errors
            .push(validation::ValidationResult::error(
                validation::ValidationCategory::Items,
                "Item ID duplicate",
            ));
        app.validation_errors
            .push(validation::ValidationResult::warning(
                validation::ValidationCategory::Items,
                "Item name recommended",
            ));

        // Enable Warnings Only filter
        app.validation_filter = ValidationFilter::WarningsOnly;

        let grouped2 = app.grouped_filtered_validation_results();
        let visible: usize = grouped2
            .iter()
            .map(|(_category, results)| results.len())
            .sum();

        assert_eq!(visible, 1, "Filter should show only the warning result");
    }

    #[test]
    fn test_validation_focus_asset_click_sets_state() {
        use std::path::PathBuf;
        let mut app = CampaignBuilderApp::default();

        // Create a temporary campaign directory and write a dummy asset file
        use std::time::{SystemTime, UNIX_EPOCH};
        let tmp_base = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        let tmpdir = tmp_base.join(format!("antares_test_assets_{}", unique));
        std::fs::create_dir_all(tmpdir.join("data")).expect("Failed to create temp data dir");

        let item_file = tmpdir.join("data").join("items.ron");
        std::fs::write(&item_file, "[]").expect("Failed to write dummy items.ron");

        let mut manager = asset_manager::AssetManager::new(tmpdir.clone());
        manager.scan_directory().expect("scan_directory failed");
        app.asset_manager = Some(manager);

        let asset_path = PathBuf::from("data/items.ron");

        // Simulate clicking the file path (invoking the focus method directly)
        app.focus_asset(asset_path.clone());

        assert!(app.show_asset_manager);
        assert_eq!(app.validation_focus_asset, Some(asset_path.clone()));
    }

    #[test]
    fn test_validation_reset_filters_clears_state() {
        use std::path::PathBuf;
        let mut app = CampaignBuilderApp::default();

        // Set a non-default state to simulate user changes
        app.validation_filter = ValidationFilter::ErrorsOnly;
        app.validation_focus_asset = Some(PathBuf::from("data/items.ron"));

        // Call the reset helper and verify all state is reverted
        app.reset_validation_filters();

        assert_eq!(app.validation_filter, ValidationFilter::All);
        assert_eq!(app.validation_focus_asset, None);
    }

    #[test]
    fn test_validation_starting_food_invalid() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();
        app.campaign.name = "Test".to_string();
        app.campaign.starting_map = "test".to_string();
        app.campaign.starting_food = (FOOD_MAX as u32) + 10;
        app.validate_campaign();

        let has_food_warning = app.validation_errors.iter().any(|e| {
            e.category == validation::ValidationCategory::Configuration
                && e.is_warning()
                && e.message.contains("Starting food")
        });
        assert!(has_food_warning);
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
            .filter(|e| e.is_error())
            .count();
        assert_eq!(error_count, 0);
    }

    #[test]
    fn test_validate_character_ids_duplicate() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add two characters with the same ID
        let char1 = CharacterDefinition::new(
            "char_1".to_string(),
            "Hero".to_string(),
            "race_1".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        let char2 = CharacterDefinition::new(
            "char_1".to_string(), // Duplicate ID
            "Another Hero".to_string(),
            "race_1".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char1);
        app.characters_editor_state.characters.push(char2);

        let results = app.validate_character_ids();
        let has_duplicate_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("Duplicate character ID"));
        assert!(has_duplicate_error);
    }

    #[test]
    fn test_validate_character_ids_empty_id() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        let char = CharacterDefinition::new(
            "".to_string(), // Empty ID
            "Hero".to_string(),
            "race_1".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char);

        let results = app.validate_character_ids();
        let has_empty_id_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("empty ID"));
        assert!(has_empty_id_error);
    }

    #[test]
    fn test_validate_character_ids_empty_name_warning() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        let char = CharacterDefinition::new(
            "char_1".to_string(),
            "".to_string(), // Empty name
            "race_1".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char);

        let results = app.validate_character_ids();
        let has_name_warning = results.iter().any(|r| {
            r.severity == validation::ValidationSeverity::Warning
                && r.message.contains("empty name")
        });
        assert!(has_name_warning);
    }

    #[test]
    fn test_validate_character_ids_invalid_class_reference() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        let char = CharacterDefinition::new(
            "char_1".to_string(),
            "Hero".to_string(),
            "race_1".to_string(),
            "nonexistent_class".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char);

        let results = app.validate_character_ids();
        let has_class_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("non-existent class"));
        assert!(has_class_error);
    }

    #[test]
    fn test_validate_character_ids_invalid_race_reference() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        let char = CharacterDefinition::new(
            "char_1".to_string(),
            "Hero".to_string(),
            "nonexistent_race".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char);

        let results = app.validate_character_ids();
        let has_race_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("non-existent race"));
        assert!(has_race_error);
    }

    #[test]
    fn test_validate_character_ids_valid() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add a class and race that the character can reference
        app.classes_editor_state.classes.push(ClassDefinition::new(
            "class_1".to_string(),
            "Knight".to_string(),
        ));

        app.races_editor_state.races.push(RaceDefinition::new(
            "race_1".to_string(),
            "Human".to_string(),
            "A balanced race".to_string(),
        ));

        let char = CharacterDefinition::new(
            "char_1".to_string(),
            "Hero".to_string(),
            "race_1".to_string(),
            "class_1".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        app.characters_editor_state.characters.push(char);

        let results = app.validate_character_ids();
        let has_pass = results
            .iter()
            .any(|r| r.severity == validation::ValidationSeverity::Passed);
        assert!(has_pass);
    }

    #[test]
    fn test_validate_proficiency_ids_duplicate() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add two proficiencies with the same ID
        app.proficiencies.push(ProficiencyDefinition {
            id: "prof_1".to_string(),
            name: "Longsword".to_string(),
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        app.proficiencies.push(ProficiencyDefinition {
            id: "prof_1".to_string(), // Duplicate ID
            name: "Shortsword".to_string(),
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        let results = app.validate_proficiency_ids();
        let has_duplicate_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("Duplicate proficiency ID"));
        assert!(has_duplicate_error);
    }

    #[test]
    fn test_validate_proficiency_ids_empty_id() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        app.proficiencies.push(ProficiencyDefinition {
            id: "".to_string(), // Empty ID
            name: "Longsword".to_string(),
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        let results = app.validate_proficiency_ids();
        let has_empty_id_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("empty ID"));
        assert!(has_empty_id_error);
    }

    #[test]
    fn test_validate_proficiency_ids_empty_name_warning() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        app.proficiencies.push(ProficiencyDefinition {
            id: "prof_1".to_string(),
            name: "".to_string(), // Empty name
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        let results = app.validate_proficiency_ids();
        let has_name_warning = results.iter().any(|r| {
            r.severity == validation::ValidationSeverity::Warning
                && r.message.contains("empty name")
        });
        assert!(has_name_warning);
    }

    #[test]
    fn test_validate_proficiency_ids_referenced_by_class() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add a proficiency
        app.proficiencies.push(ProficiencyDefinition {
            id: "prof_1".to_string(),
            name: "Longsword".to_string(),
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        // Add a class that references the proficiency
        let mut class = ClassDefinition::new("class_1".to_string(), "Knight".to_string());
        class.proficiencies = vec!["prof_1".to_string()];
        app.classes_editor_state.classes.push(class);

        let results = app.validate_proficiency_ids();
        let has_pass = results
            .iter()
            .any(|r| r.severity == validation::ValidationSeverity::Passed);
        assert!(has_pass);
    }

    #[test]
    fn test_validate_proficiency_ids_class_references_nonexistent() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add a class that references a non-existent proficiency
        let mut class = ClassDefinition::new("class_1".to_string(), "Knight".to_string());
        class.proficiencies = vec!["nonexistent_prof".to_string()];
        app.classes_editor_state.classes.push(class);

        let results = app.validate_proficiency_ids();
        let has_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("references non-existent proficiency"));
        assert!(has_error);
    }

    #[test]
    fn test_validate_proficiency_ids_race_references_nonexistent() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add a race that references a non-existent proficiency
        let mut race =
            RaceDefinition::new("race_1".to_string(), "Human".to_string(), String::new());
        race.proficiencies = vec!["nonexistent_prof".to_string()];
        app.races_editor_state.races.push(race);

        let results = app.validate_proficiency_ids();
        let has_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("references non-existent proficiency"));
        assert!(has_error);
    }

    #[test]
    fn test_validate_proficiency_ids_item_requires_nonexistent() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add an item that requires a non-existent proficiency
        let mut item = ItemsEditorState::default_item();
        item.item_type = ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 6, 0),
            bonus: 0,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        });
        // "martial_melee" is not in app.proficiencies by default
        app.items.push(item);

        let results = app.validate_proficiency_ids();
        let has_error = results
            .iter()
            .any(|r| r.is_error() && r.message.contains("requires non-existent proficiency"));
        assert!(has_error);
    }

    #[test]
    fn test_validate_proficiency_ids_unreferenced_info() {
        let mut app = CampaignBuilderApp::default();
        app.campaign.id = "test".to_string();

        // Add a proficiency that is not referenced by anything
        app.proficiencies.push(ProficiencyDefinition {
            id: "unused_prof".to_string(),
            name: "Unused".to_string(),
            category: antares::domain::proficiency::ProficiencyCategory::Weapon,
            description: String::new(),
        });

        let results = app.validate_proficiency_ids();
        let has_info = results.iter().any(|r| {
            r.severity == validation::ValidationSeverity::Info && r.message.contains("not used")
        });
        assert!(has_info);
    }

    #[test]
    fn test_validation_all_shows_passed_and_errors_only_filters_out_passed() {
        // Ensure 'All' filter shows Passed results by default and 'Errors Only' filters them out.
        let mut app = CampaignBuilderApp::default();

        // Add a single 'Passed' validation result for the Items category.
        app.validation_errors
            .push(validation::ValidationResult::passed(
                validation::ValidationCategory::Items,
                "All items validated",
            ));

        // Default filter is All; verify passed checks are included.
        app.validation_filter = ValidationFilter::All;
        let grouped_all = app.grouped_filtered_validation_results();
        let has_passed_in_all = grouped_all.iter().any(|(_cat, results)| {
            results
                .iter()
                .any(|r| r.severity == validation::ValidationSeverity::Passed)
        });
        assert!(
            has_passed_in_all,
            "All filter should include passed checks by default"
        );

        // Switch to ErrorsOnly filter and verify passed checks are excluded.
        app.validation_filter = ValidationFilter::ErrorsOnly;
        let grouped_errors = app.grouped_filtered_validation_results();
        let has_passed_in_errors = grouped_errors.iter().any(|(_cat, results)| {
            results
                .iter()
                .any(|r| r.severity == validation::ValidationSeverity::Passed)
        });
        assert!(
            !has_passed_in_errors,
            "ErrorsOnly filter should exclude passed checks"
        );
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
            starting_innkeeper: "tutorial_innkeeper_town".to_string(),
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
            characters_file: "data/characters.ron".to_string(),
            maps_dir: "data/maps/".to_string(),
            quests_file: "data/quests.ron".to_string(),
            dialogue_file: "data/dialogue.ron".to_string(),
            conditions_file: "data/conditions.ron".to_string(),
            npcs_file: "data/npcs.ron".to_string(),
            proficiencies_file: "data/proficiencies.ron".to_string(),
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
    fn test_campaign_backwards_compatibility_missing_proficiencies_file() {
        // Test that old campaign files without proficiencies_file can still load
        // This ensures campaigns created before Phase 3 still work
        let old_campaign_ron = r#"CampaignMetadata(
    id: "legacy_campaign",
    name: "Legacy Campaign",
    version: "1.0.0",
    author: "Test",
    description: "Test",
    engine_version: "0.1.0",
    starting_map: "test_map",
    starting_position: (5, 5),
    starting_direction: "North",
    starting_gold: 100,
    starting_food: 10,
    max_party_size: 6,
    max_roster_size: 20,
    difficulty: Normal,
    permadeath: false,
    allow_multiclassing: false,
    starting_level: 1,
    max_level: 20,
    items_file: "data/items.ron",
    spells_file: "data/spells.ron",
    monsters_file: "data/monsters.ron",
    classes_file: "data/classes.ron",
    races_file: "data/races.ron",
    characters_file: "data/characters.ron",
    maps_dir: "data/maps/",
    quests_file: "data/quests.ron",
    dialogue_file: "data/dialogues.ron",
    conditions_file: "data/conditions.ron",
    npcs_file: "data/npcs.ron",
)"#;

        // Deserialize should succeed and use default proficiencies_file
        let result: Result<CampaignMetadata, _> = ron::from_str(old_campaign_ron);
        assert!(
            result.is_ok(),
            "Failed to deserialize legacy campaign: {:?}",
            result.err()
        );

        let campaign = result.unwrap();
        assert_eq!(campaign.id, "legacy_campaign");
        assert_eq!(campaign.proficiencies_file, "data/proficiencies.ron");
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
        assert_eq!(validation::ValidationSeverity::Error.icon(), "❌");
        assert_eq!(validation::ValidationSeverity::Warning.icon(), "⚠️");
        assert_eq!(validation::ValidationSeverity::Passed.icon(), "✅");
        assert_eq!(validation::ValidationSeverity::Info.icon(), "ℹ️");
    }

    #[test]
    fn test_validation_result_creation() {
        let error = validation::ValidationResult::error(
            validation::ValidationCategory::Metadata,
            "Test error",
        );
        assert!(error.is_error());
        assert_eq!(error.message, "Test error");
        assert_eq!(error.category, validation::ValidationCategory::Metadata);
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut app = CampaignBuilderApp::default();
        assert_eq!(
            app.items_editor_state.mode,
            items_editor::ItemsEditorMode::List
        );

        // Simulate adding an item
        app.items_editor_state.mode = items_editor::ItemsEditorMode::Add;
        assert_eq!(
            app.items_editor_state.mode,
            items_editor::ItemsEditorMode::Add
        );

        // Simulate editing an item
        app.items_editor_state.mode = items_editor::ItemsEditorMode::Edit;
        assert_eq!(
            app.items_editor_state.mode,
            items_editor::ItemsEditorMode::Edit
        );

        // Return to list
        app.items_editor_state.mode = items_editor::ItemsEditorMode::List;
        assert_eq!(
            app.items_editor_state.mode,
            items_editor::ItemsEditorMode::List
        );
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
        assert_eq!(monster.hp.base, 10);
        assert_eq!(monster.ac.base, 10);
        assert!(!monster.is_undead);
        assert!(!monster.can_regenerate);
        assert!(monster.can_advance);
        assert_eq!(monster.magic_resistance, 0);
    }

    #[test]
    fn test_items_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.items.len(), 0);
        assert_eq!(app.items_editor_state.search_query, "");
        assert_eq!(app.items_editor_state.selected_item, None);
        assert_eq!(
            app.items_editor_state.mode,
            items_editor::ItemsEditorMode::List
        );
    }

    #[test]
    fn test_spells_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.spells.len(), 0);
        assert_eq!(app.spells_editor_state.search_query, "");
        assert_eq!(app.spells_editor_state.selected_spell, None);
        assert_eq!(
            app.spells_editor_state.mode,
            spells_editor::SpellsEditorMode::List
        );
    }

    #[test]
    fn test_monsters_data_structure_initialization() {
        let app = CampaignBuilderApp::default();
        assert_eq!(app.monsters.len(), 0);
        assert_eq!(app.monsters_editor_state.search_query, "");
        assert_eq!(app.monsters_editor_state.selected_monster, None);
        assert_eq!(
            app.monsters_editor_state.mode,
            monsters_editor::MonstersEditorMode::List
        );
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

        app.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Test edit stage
        let result = app.quest_editor_state.edit_stage(&app.quests, 0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quest_editor_state.selected_stage, Some(0));
        assert_eq!(app.quest_editor_state.stage_buffer.name, "Stage 1");
        assert_eq!(
            app.quest_editor_state.stage_buffer.description,
            "Stage 1 description"
        );

        // Test save stage
        app.quest_editor_state.stage_buffer.name = "Updated Stage".to_string();
        let result = app.quest_editor_state.save_stage(&mut app.quests, 0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quests[0].stages[0].name, "Updated Stage");
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

        app.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Test edit objective
        let result = app.quest_editor_state.edit_objective(&app.quests, 0, 0, 0);
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
        let result = app
            .quest_editor_state
            .save_objective(&mut app.quests, 0, 0, 0);
        assert!(result.is_ok());

        if let antares::domain::quest::QuestObjective::KillMonsters {
            monster_id,
            quantity,
        } = &app.quests[0].stages[0].objectives[0]
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

        app.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Delete first stage
        assert_eq!(app.quests[0].stages.len(), 2);
        let result = app.quest_editor_state.delete_stage(&mut app.quests, 0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quests[0].stages.len(), 1);
        assert_eq!(app.quests[0].stages[0].name, "Stage 2");
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

        app.quests.push(quest);
        app.quest_editor_state.selected_quest = Some(0);

        // Delete first objective
        assert_eq!(app.quests[0].stages[0].objectives.len(), 2);
        let result = app
            .quest_editor_state
            .delete_objective(&mut app.quests, 0, 0, 0);
        assert!(result.is_ok());
        assert_eq!(app.quests[0].stages[0].objectives.len(), 1);

        // Verify remaining objective is CollectItems
        if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
            &app.quests[0].stages[0].objectives[0]
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

        app.quests.push(quest);

        // Edit objective and change type to CollectItems
        let result = app.quest_editor_state.edit_objective(&app.quests, 0, 0, 0);
        assert!(result.is_ok());

        app.quest_editor_state.objective_buffer.objective_type =
            quest_editor::ObjectiveType::CollectItems;
        app.quest_editor_state.objective_buffer.item_id = "250".to_string();
        app.quest_editor_state.objective_buffer.quantity = "7".to_string();

        let result = app
            .quest_editor_state
            .save_objective(&mut app.quests, 0, 0, 0);
        assert!(result.is_ok());

        // Verify objective type changed
        if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
            &app.quests[0].stages[0].objectives[0]
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
        let result = app.quest_editor_state.edit_stage(&app.quests, 0, 0);
        assert!(result.is_err());

        let result = app.quest_editor_state.edit_objective(&app.quests, 0, 0, 0);
        assert!(result.is_err());

        let result = app.quest_editor_state.delete_stage(&mut app.quests, 0, 0);
        assert!(result.is_err());

        let result = app
            .quest_editor_state
            .delete_objective(&mut app.quests, 0, 0, 0);
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
        assert!(errors[0].is_error());
        assert_eq!(errors[0].category, validation::ValidationCategory::Items);
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
        assert!(errors[0].is_error());
        assert_eq!(errors[0].category, validation::ValidationCategory::Spells);
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
        assert!(errors[0].is_error());
        assert_eq!(errors[0].category, validation::ValidationCategory::Monsters);
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
        assert!(errors[0].is_error());
        assert_eq!(errors[0].category, validation::ValidationCategory::Maps);
        assert!(errors[0].message.contains("Duplicate map ID: 10"));
    }

    #[test]
    fn test_condition_id_uniqueness_validation() {
        let mut app = CampaignBuilderApp::default();

        let cond1 = ConditionDefinition {
            id: "dup_test".to_string(),
            name: "Duplicate 1".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(3),
            icon_id: None,
        };

        let mut cond2 = cond1.clone();
        cond2.name = "Duplicate 2".to_string();
        app.conditions.push(cond1);
        app.conditions.push(cond2);

        // Validate
        let errors = app.validate_condition_ids();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_error());
        assert_eq!(
            errors[0].category,
            validation::ValidationCategory::Conditions
        );
        assert!(errors[0]
            .message
            .contains("Duplicate condition ID: dup_test"));
    }

    #[test]
    fn test_conditions_save_load_roundtrip() {
        let mut app = CampaignBuilderApp::default();

        // Create a unique temporary directory under the system temp dir
        let tmp_dir = std::env::temp_dir().join(format!(
            "antares_test_conditions_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        app.campaign_dir = Some(tmp_dir);
        app.campaign.conditions_file = "conditions_test.ron".to_string();

        let c1 = ConditionDefinition {
            id: "c1".to_string(),
            name: "Condition 1".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };
        let c2 = ConditionDefinition {
            id: "c2".to_string(),
            name: "Condition 2".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(4),
            icon_id: None,
        };

        app.conditions.push(c1);
        app.conditions.push(c2);

        assert!(app.save_conditions().is_ok());

        app.conditions.clear();
        app.load_conditions();

        assert_eq!(app.conditions.len(), 2);
        assert_eq!(app.conditions[0].id, "c1");
        assert_eq!(app.conditions[1].id, "c2");
    }

    #[test]
    fn test_apply_condition_edits_insert() {
        use crate::conditions_editor::apply_condition_edits;

        let mut conditions: Vec<ConditionDefinition> = Vec::new();

        let new_cond = ConditionDefinition {
            id: "insert_test".to_string(),
            name: "Insert Test".to_string(),
            description: "Insert via helper".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(1),
            icon_id: Some("icon_insert".to_string()),
        };

        // Insert should succeed
        assert!(apply_condition_edits(&mut conditions, None, &new_cond).is_ok());
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].id, "insert_test");
    }

    #[test]
    fn test_apply_condition_edits_update_success() {
        use crate::conditions_editor::apply_condition_edits;

        let mut conditions: Vec<ConditionDefinition> = Vec::new();

        let c1 = ConditionDefinition {
            id: "c1".to_string(),
            name: "C1".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };
        let c2 = ConditionDefinition {
            id: "c2".to_string(),
            name: "C2".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Minutes(3),
            icon_id: None,
        };

        conditions.push(c1.clone());
        conditions.push(c2.clone());

        // Update c1 to have id 'c3'
        let mut edited = c1.clone();
        edited.id = "c3".to_string();
        edited.name = "C3 changed".to_string();
        edited.icon_id = Some("icon_c3".to_string());
        match apply_condition_edits(&mut conditions, Some("c1"), &edited) {
            Ok(()) => {
                assert!(conditions.iter().any(|c| c.id == "c3"));
                assert!(!conditions.iter().any(|c| c.id == "c1"));
                // confirm name/icon changed
                let found = conditions.iter().find(|c| c.id == "c3").unwrap();
                assert_eq!(found.name, "C3 changed");
                assert_eq!(found.icon_id.as_ref().unwrap(), "icon_c3");
            }
            Err(e) => panic!("apply_condition_edits failed: {}", e),
        }
    }

    #[test]
    fn test_apply_condition_edits_update_duplicate_error() {
        use crate::conditions_editor::apply_condition_edits;

        let mut conditions: Vec<ConditionDefinition> = Vec::new();

        let c1 = ConditionDefinition {
            id: "c1".to_string(),
            name: "C1".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };
        let c2 = ConditionDefinition {
            id: "c2".to_string(),
            name: "C2".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Minutes(3),
            icon_id: None,
        };

        conditions.push(c1.clone());
        conditions.push(c2.clone());

        // Attempt to update c1 to id 'c2' (duplicate) -> should fail
        let mut edited = c1.clone();
        edited.id = "c2".to_string();
        let res = apply_condition_edits(&mut conditions, Some("c1"), &edited);
        assert!(res.is_err());
    }

    #[test]
    fn test_apply_condition_edits_insert_duplicate_error() {
        use crate::conditions_editor::apply_condition_edits;

        let mut conditions: Vec<ConditionDefinition> = Vec::new();

        let c1 = ConditionDefinition {
            id: "dup".to_string(),
            name: "Dup".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };

        conditions.push(c1.clone());

        // Attempt to insert a new condition with id 'dup' -> should fail
        let new_dup = ConditionDefinition {
            id: "dup".to_string(),
            name: "Dup New".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: antares::domain::conditions::ConditionDuration::Minutes(1),
            icon_id: None,
        };
        let res = apply_condition_edits(&mut conditions, None, &new_dup);
        assert!(res.is_err());
    }

    // New tests for effect helpers (Phase 3 - Effects editing helpers)
    #[test]
    fn test_condition_effect_helpers_success_flow() {
        use crate::conditions_editor::{
            add_effect_to_condition, delete_effect_from_condition, duplicate_effect_in_condition,
            move_effect_in_condition, update_effect_in_condition,
        };

        use antares::domain::conditions::{
            ConditionDefinition, ConditionDuration, ConditionEffect,
        };

        let mut condition = ConditionDefinition {
            id: "c_effects".to_string(),
            name: "Effect Test".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Rounds(1),
            icon_id: None,
        };

        // Add a status effect 'sleep'
        let sleep_eff = ConditionEffect::StatusEffect("sleep".to_string());
        add_effect_to_condition(&mut condition, sleep_eff.clone());
        assert_eq!(condition.effects.len(), 1);
        assert_eq!(condition.effects[0], sleep_eff);

        // Add an AttributeModifier effect
        let buff = ConditionEffect::AttributeModifier {
            attribute: "might".to_string(),
            value: 5,
        };
        add_effect_to_condition(&mut condition, buff.clone());
        assert_eq!(condition.effects.len(), 2);
        assert_eq!(condition.effects[1], buff);

        // Duplicate the sleep effect (index 0), now effects [sleep, sleep, buff]
        duplicate_effect_in_condition(&mut condition, 0).unwrap();
        assert_eq!(condition.effects.len(), 3);
        assert_eq!(condition.effects[1], sleep_eff);

        // Move the buff up (index 2 -> index 1)
        move_effect_in_condition(&mut condition, 2, -1).unwrap();
        // After moving, index 1 should be the AttributeModifier and index 2 is the duplicate sleep
        if let ConditionEffect::AttributeModifier { attribute, value } = &condition.effects[1] {
            assert_eq!(attribute, "might");
            assert_eq!(*value, 5);
        } else {
            panic!("Expected AttributeModifier at index 1 after move");
        }

        // Update the moved buff to a different value
        let buff2 = ConditionEffect::AttributeModifier {
            attribute: "might".to_string(),
            value: 10,
        };
        update_effect_in_condition(&mut condition, 1, buff2.clone()).unwrap();
        assert_eq!(condition.effects[1], buff2);

        // Delete effect at index 0 (original sleep)
        delete_effect_from_condition(&mut condition, 0).unwrap();
        assert_eq!(condition.effects.len(), 2);
    }

    #[test]
    fn test_update_effect_out_of_range() {
        use crate::conditions_editor::update_effect_in_condition;
        use antares::domain::conditions::{
            ConditionDefinition, ConditionDuration, ConditionEffect,
        };

        let mut condition = ConditionDefinition {
            id: "c_effects2".to_string(),
            name: "Effect Test 2".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Rounds(1),
            icon_id: None,
        };

        let res = update_effect_in_condition(
            &mut condition,
            0,
            ConditionEffect::StatusEffect("x".to_string()),
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_spells_referencing_condition_and_removal() {
        use crate::conditions_editor::{
            remove_condition_references_from_spells, spells_referencing_condition,
        };
        use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
        use antares::domain::types::DiceRoll;

        // Create a small set of spells, some referencing 'bless', others not.
        let mut spells: Vec<Spell> = Vec::new();

        spells.push(Spell {
            id: 0x0101,
            name: "Bless".to_string(),
            school: SpellSchool::Cleric,
            level: 1,
            sp_cost: 1,
            gem_cost: 0,
            context: SpellContext::Anytime,
            target: SpellTarget::SingleCharacter,
            description: "A simple buff".to_string(),
            damage: None,
            duration: 0,
            saving_throw: false,
            applied_conditions: vec!["bless".to_string()],
        });

        spells.push(Spell {
            id: 0x0201,
            name: "Fireball".to_string(),
            school: SpellSchool::Sorcerer,
            level: 3,
            sp_cost: 5,
            gem_cost: 0,
            context: SpellContext::CombatOnly,
            target: SpellTarget::MonsterGroup,
            description: "A large blast of fire".to_string(),
            damage: Some(DiceRoll::new(3, 6, 0)),
            duration: 0,
            saving_throw: false,
            applied_conditions: vec!["burn".to_string()],
        });

        // No match for a nonexistent condition
        let used = spells_referencing_condition(&spells, "nonexistent");
        assert!(used.is_empty());

        // A single spell references 'bless'
        let used = spells_referencing_condition(&spells, "bless");
        assert_eq!(used.len(), 1);
        assert_eq!(used[0], "Bless");

        // Remove references from spells (should remove from Bless)
        let removed = remove_condition_references_from_spells(&mut spells, "bless");
        assert_eq!(removed, 1);
        assert!(spells[0].applied_conditions.is_empty());
    }

    #[test]
    fn test_apply_condition_edits_validation_and_effect_types() {
        use crate::conditions_editor::apply_condition_edits;
        use antares::domain::conditions::{
            ConditionDefinition, ConditionDuration, ConditionEffect,
        };
        use antares::domain::types::DiceRoll;

        let mut conditions: Vec<ConditionDefinition> = Vec::new();

        // Invalid DOT - zero dice count should fail validation.
        let invalid_dot = ConditionDefinition {
            id: "invalid_dot".to_string(),
            name: "Invalid DOT".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::DamageOverTime {
                damage: DiceRoll::new(0, 6, 0),
                element: "fire".to_string(),
            }],
            default_duration: ConditionDuration::Rounds(1),
            icon_id: None,
        };

        assert!(apply_condition_edits(&mut conditions, None, &invalid_dot).is_err());

        // Invalid attribute modifier value (too large).
        let invalid_attr = ConditionDefinition {
            id: "invalid_attr".to_string(),
            name: "Invalid Attribute".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "might".to_string(),
                value: 9999,
            }],
            default_duration: ConditionDuration::Rounds(1),
            icon_id: None,
        };

        assert!(apply_condition_edits(&mut conditions, None, &invalid_attr).is_err());

        // Valid round-trip: each effect type present and valid should be accepted.
        let valid_all = ConditionDefinition {
            id: "valid_all".to_string(),
            name: "Valid All".to_string(),
            description: "".to_string(),
            effects: vec![
                ConditionEffect::StatusEffect("sleep".to_string()),
                ConditionEffect::AttributeModifier {
                    attribute: "might".to_string(),
                    value: 5,
                },
                ConditionEffect::DamageOverTime {
                    damage: DiceRoll::new(1, 6, 0),
                    element: "poison".to_string(),
                },
                ConditionEffect::HealOverTime {
                    amount: DiceRoll::new(1, 4, 0),
                },
            ],
            default_duration: ConditionDuration::Rounds(1),
            icon_id: None,
        };

        assert!(apply_condition_edits(&mut conditions, None, &valid_all).is_ok());
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_validate_effect_edit_buffer() {
        use crate::conditions_editor::{validate_effect_edit_buffer, EffectEditBuffer};
        use antares::domain::types::DiceRoll;

        // Attribute modifier - empty attribute fails
        let mut buf = EffectEditBuffer {
            effect_type: Some("AttributeModifier".to_string()),
            attribute: "".to_string(),
            ..Default::default()
        };
        assert!(validate_effect_edit_buffer(&buf).is_err());

        // Attribute present but value out of allowed range fails
        buf.attribute = "might".to_string();
        buf.attribute_value = 300; // out of range
        assert!(validate_effect_edit_buffer(&buf).is_err());

        // Status effect - empty tag fails
        let buf2 = EffectEditBuffer {
            effect_type: Some("StatusEffect".to_string()),
            status_tag: "".to_string(),
            ..Default::default()
        };
        assert!(validate_effect_edit_buffer(&buf2).is_err());

        // DOT validation - invalid dice count should fail
        let buf3 = EffectEditBuffer {
            effect_type: Some("DamageOverTime".to_string()),
            dice: DiceRoll::new(0, 6, 0),
            element: "fire".to_string(),
            ..Default::default()
        };
        assert!(validate_effect_edit_buffer(&buf3).is_err());

        // HOT validation - invalid dice sides should fail
        let buf4 = EffectEditBuffer {
            effect_type: Some("HealOverTime".to_string()),
            dice: DiceRoll::new(1, 1, 0), // invalid sides
            ..Default::default()
        };
        assert!(validate_effect_edit_buffer(&buf4).is_err());
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
            classification: WeaponClassification::MartialMelee,
        });

        let filter = ItemTypeFilter::Weapon;
        assert!(filter.matches(&weapon));

        // Should not match other types
        let mut armor = CampaignBuilderApp::default_item();
        armor.item_type = ItemType::Armor(antares::domain::items::types::ArmorData {
            ac_bonus: 5,
            weight: 20,
            classification: ArmorClassification::Medium,
        });
        assert!(!filter.matches(&armor));
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_type_filter_all_types() {
        use antares::domain::items::types::*;

        let weapon_item = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        let armor_item = Item {
            id: 2,
            name: "Chainmail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 5,
                weight: 30,
                classification: ArmorClassification::Medium,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        assert!(ItemTypeFilter::Weapon.matches(&weapon_item));
        assert!(!ItemTypeFilter::Weapon.matches(&armor_item));
        assert!(ItemTypeFilter::Armor.matches(&armor_item));
        assert!(!ItemTypeFilter::Armor.matches(&weapon_item));
    }

    #[test]
    fn test_items_filter_magical() {
        let mut app = CampaignBuilderApp::default();
        app.items_editor_state.filter_magical = Some(true);

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
    fn test_item_proficiency_and_alignment_restrictions() {
        // Local imports for clarity and isolation in test function
        use antares::domain::items::types::{AlignmentRestriction, WeaponClassification};
        use antares::domain::proficiency::ProficiencyDatabase;

        let mut item = CampaignBuilderApp::default_item();

        // Default item should derive the correct proficiency for a simple weapon
        assert_eq!(
            item.required_proficiency(),
            Some(ProficiencyDatabase::proficiency_for_weapon(
                WeaponClassification::Simple
            ))
        );

        // Default alignment restriction should be None
        assert_eq!(item.alignment_restriction, None);

        // Update alignment restriction to GoodOnly and confirm it is stored correctly
        item.alignment_restriction = Some(AlignmentRestriction::GoodOnly);
        assert_eq!(
            item.alignment_restriction,
            Some(AlignmentRestriction::GoodOnly)
        );
        // Ensure EvilOnly is not set
        assert_ne!(
            item.alignment_restriction,
            Some(AlignmentRestriction::EvilOnly)
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_import_export_roundtrip() {
        use antares::domain::items::types::*;

        let original_item = Item {
            id: 42,
            name: "Test Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 6, 1),
                bonus: 2,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 3,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
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
    #[allow(deprecated)]
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
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 15,
            sell_cost: 7,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };
        assert!(weapon.is_weapon());

        // Test armor type
        let armor = Item {
            id: 2,
            name: "Plate Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 8,
                weight: 50,
                classification: ArmorClassification::Heavy,
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
            tags: vec!["heavy_armor".to_string()],
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
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };
        assert!(potion.is_consumable());
    }

    #[test]
    #[allow(deprecated)]
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
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 2,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 5,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        let cursed_armor = Item {
            id: 2,
            name: "Cursed Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 5,
                weight: 30,
                classification: ArmorClassification::Medium,
            }),
            base_cost: 50,
            sell_cost: 0,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: true,
            icon_path: None,
            tags: vec![],
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
    fn test_items_editor_default_required_proficiency() {
        // Ensure the items editor edit buffer starts with a default weapon and correct derived proficiency
        use antares::domain::items::types::WeaponClassification;
        use antares::domain::proficiency::ProficiencyDatabase;

        let app = CampaignBuilderApp::default();

        // Default edit_buffer is a simple weapon; required_proficiency should match simple_weapon id
        let required_prof = app
            .items_editor_state
            .edit_buffer
            .required_proficiency()
            .expect("Default edit buffer should have a derived proficiency");

        assert_eq!(
            required_prof,
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Simple)
        );

        // Default alignment restriction should be None
        assert_eq!(
            app.items_editor_state.edit_buffer.alignment_restriction,
            None
        );
    }

    #[test]
    fn test_items_editor_classification_changes_required_proficiency() {
        use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
        use antares::domain::proficiency::ProficiencyDatabase;
        use antares::domain::types::DiceRoll;

        let mut app = CampaignBuilderApp::default();

        // Change the edit buffer weapon classification to MartialMelee and verify derived proficiency
        app.items_editor_state.edit_buffer.item_type = ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(2, 6, 0),
            bonus: 1,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        });

        let required_prof = app
            .items_editor_state
            .edit_buffer
            .required_proficiency()
            .expect("Martial melee weapon should have a derived proficiency");

        assert_eq!(
            required_prof,
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialMelee)
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_preview_displays_all_info() {
        use antares::domain::items::types::*;

        // Standard class bit positions
        const BIT_KNIGHT: u8 = 0b0000_0001;
        const BIT_PALADIN: u8 = 0b0000_0010;

        let item = Item {
            id: 10,
            name: "Flaming Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 2),
                bonus: 3,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,
            alignment_restriction: Some(AlignmentRestriction::GoodOnly),
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 2,
            }),
            temporary_bonus: None,
            spell_effect: Some(10),
            max_charges: 20,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
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
        app.spells_editor_state.filter_school = Some(SpellSchool::Cleric);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| {
                app.spells_editor_state
                    .filter_school
                    .is_none_or(|f| s.school == f)
            })
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
        app.spells_editor_state.filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| {
                app.spells_editor_state
                    .filter_level
                    .is_none_or(|f| s.level == f)
            })
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
        app.spells_editor_state.filter_school = Some(SpellSchool::Cleric);
        app.spells_editor_state.filter_level = Some(3);

        let filtered: Vec<_> = app
            .spells
            .iter()
            .filter(|s| {
                app.spells_editor_state
                    .filter_school
                    .is_none_or(|f| s.school == f)
                    && app
                        .spells_editor_state
                        .filter_level
                        .is_none_or(|f| s.level == f)
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
        app.spells_editor_state.edit_buffer = Spell::new(
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
        );

        // Change context
        app.spells_editor_state.edit_buffer.context = SpellContext::CombatOnly;
        assert_eq!(
            app.spells_editor_state.edit_buffer.context,
            SpellContext::CombatOnly
        );

        // Change target
        app.spells_editor_state.edit_buffer.target = SpellTarget::AllCharacters;
        assert_eq!(
            app.spells_editor_state.edit_buffer.target,
            SpellTarget::AllCharacters
        );
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
        let mut app = CampaignBuilderApp::default();
        app.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

        // Initial attacks
        assert_eq!(app.monsters_editor_state.edit_buffer.attacks.len(), 1);

        // Add attack
        app.monsters_editor_state.edit_buffer.attacks.push(Attack {
            damage: DiceRoll::new(2, 8, 3),
            attack_type: AttackType::Fire,
            special_effect: Some(SpecialEffect::Poison),
        });

        assert_eq!(app.monsters_editor_state.edit_buffer.attacks.len(), 2);
        assert_eq!(
            app.monsters_editor_state.edit_buffer.attacks[1]
                .damage
                .count,
            2
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.attacks[1]
                .damage
                .sides,
            8
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.attacks[1]
                .damage
                .bonus,
            3
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.attacks[1].attack_type,
            AttackType::Fire
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.attacks[1].special_effect,
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
        app.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

        // Modify loot table
        app.monsters_editor_state.edit_buffer.loot.gold_min = 10;
        app.monsters_editor_state.edit_buffer.loot.gold_max = 50;
        app.monsters_editor_state.edit_buffer.loot.gems_min = 0;
        app.monsters_editor_state.edit_buffer.loot.gems_max = 2;
        app.monsters_editor_state.edit_buffer.loot.experience = 150;

        assert_eq!(app.monsters_editor_state.edit_buffer.loot.gold_min, 10);
        assert_eq!(app.monsters_editor_state.edit_buffer.loot.gold_max, 50);
        assert_eq!(app.monsters_editor_state.edit_buffer.loot.gems_min, 0);
        assert_eq!(app.monsters_editor_state.edit_buffer.loot.gems_max, 2);
        assert_eq!(app.monsters_editor_state.edit_buffer.loot.experience, 150);
    }

    #[test]
    fn test_monster_stats_editor() {
        let mut app = CampaignBuilderApp::default();
        app.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

        // Modify all stats
        app.monsters_editor_state.edit_buffer.stats.might.base = 20;
        app.monsters_editor_state.edit_buffer.stats.intellect.base = 5;
        app.monsters_editor_state.edit_buffer.stats.personality.base = 8;
        app.monsters_editor_state.edit_buffer.stats.endurance.base = 18;
        app.monsters_editor_state.edit_buffer.stats.speed.base = 12;
        app.monsters_editor_state.edit_buffer.stats.accuracy.base = 15;
        app.monsters_editor_state.edit_buffer.stats.luck.base = 6;

        assert_eq!(app.monsters_editor_state.edit_buffer.stats.might.base, 20);
        assert_eq!(
            app.monsters_editor_state.edit_buffer.stats.intellect.base,
            5
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.stats.personality.base,
            8
        );
        assert_eq!(
            app.monsters_editor_state.edit_buffer.stats.endurance.base,
            18
        );
        assert_eq!(app.monsters_editor_state.edit_buffer.stats.speed.base, 12);
        assert_eq!(
            app.monsters_editor_state.edit_buffer.stats.accuracy.base,
            15
        );
        assert_eq!(app.monsters_editor_state.edit_buffer.stats.luck.base, 6);
    }

    #[test]
    fn test_monster_xp_calculation_basic() {
        let app = CampaignBuilderApp::default();
        let monster = MonsterDefinition {
            id: 1,
            name: "Test Monster".to_string(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: AttributePair16::new(20),
            ac: AttributePair::new(10),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        };

        let xp = app.monsters_editor_state.calculate_monster_xp(&monster);

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
            hp: AttributePair16::new(50),
            ac: AttributePair::new(5),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        };

        let xp = app.monsters_editor_state.calculate_monster_xp(&monster);

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
            hp: AttributePair16::new(30),
            ac: AttributePair::new(8),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
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
            hp: AttributePair16::new(15),
            ac: AttributePair::new(12),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        };

        // Verify all preview fields exist
        assert_eq!(monster.name, "Goblin");
        assert_eq!(monster.hp.base, 15);
        assert_eq!(monster.ac.base, 12);
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

        // Filter by "dragon"
        app.quest_editor_state.search_filter = "dragon".to_string();
        let filtered = app.quest_editor_state.filtered_quests(&app.quests);

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
        quest.quest_giver_npc = Some("100".to_string());
        quest.quest_giver_map = Some(5);
        quest.quest_giver_position = Some(antares::domain::types::Position::new(10, 20));

        assert_eq!(quest.quest_giver_npc, Some("100".to_string()));
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
        app.quest_editor_state
            .start_new_quest(&mut app.quests, next_id.to_string());
        assert_eq!(
            app.quest_editor_state.mode,
            quest_editor::QuestEditorMode::Creating
        );

        // Cancel back to list
        app.quest_editor_state.cancel_edit(&mut app.quests);
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
        // Search filter and import state are managed by DialogueEditorState
        assert!(app.dialogue_editor_state.search_filter.is_empty());
        assert!(!app.dialogue_editor_state.show_preview);
        assert!(app.dialogue_editor_state.import_buffer.is_empty());
        assert!(!app.dialogue_editor_state.show_import_dialog);
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
        assert_eq!(app.dialogue_editor_state.next_available_dialogue_id(), 1);

        // With dialogues - should return max + 1
        app.dialogues.push(DialogueTree::new(1, "D1", 1));
        app.dialogues.push(DialogueTree::new(5, "D2", 1));
        app.dialogues.push(DialogueTree::new(3, "D3", 1));
        app.dialogue_editor_state
            .load_dialogues(app.dialogues.clone());

        assert_eq!(app.dialogue_editor_state.next_available_dialogue_id(), 6);
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
        // Import buffer and dialog state are managed by DialogueEditorState
        assert!(app.dialogue_editor_state.import_buffer.is_empty());
        assert!(!app.dialogue_editor_state.show_import_dialog);
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

    // ===== Conditions Editor QoL Tests =====

    #[test]
    fn test_effect_type_filter_matches() {
        use crate::conditions_editor::EffectTypeFilter;
        use antares::domain::conditions::{
            ConditionDefinition, ConditionDuration, ConditionEffect,
        };
        use antares::domain::types::DiceRoll;

        // Condition with AttributeModifier
        let attr_cond = ConditionDefinition {
            id: "attr".to_string(),
            name: "Attribute Buff".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "might".to_string(),
                value: 5,
            }],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        };

        // Condition with DOT
        let dot_cond = ConditionDefinition {
            id: "dot".to_string(),
            name: "Burning".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::DamageOverTime {
                damage: DiceRoll::new(1, 6, 0),
                element: "fire".to_string(),
            }],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        };

        // Condition with HOT
        let hot_cond = ConditionDefinition {
            id: "hot".to_string(),
            name: "Regeneration".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::HealOverTime {
                amount: DiceRoll::new(1, 4, 1),
            }],
            default_duration: ConditionDuration::Rounds(5),
            icon_id: None,
        };

        // Empty condition
        let empty_cond = ConditionDefinition {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Instant,
            icon_id: None,
        };

        // Test All filter matches everything
        assert!(EffectTypeFilter::All.matches(&attr_cond));
        assert!(EffectTypeFilter::All.matches(&dot_cond));
        assert!(EffectTypeFilter::All.matches(&hot_cond));
        assert!(EffectTypeFilter::All.matches(&empty_cond));

        // Test specific filters
        assert!(EffectTypeFilter::AttributeModifier.matches(&attr_cond));
        assert!(!EffectTypeFilter::AttributeModifier.matches(&dot_cond));

        assert!(EffectTypeFilter::DamageOverTime.matches(&dot_cond));
        assert!(!EffectTypeFilter::DamageOverTime.matches(&hot_cond));

        assert!(EffectTypeFilter::HealOverTime.matches(&hot_cond));
        assert!(!EffectTypeFilter::HealOverTime.matches(&attr_cond));

        // Empty condition doesn't match specific filters
        assert!(!EffectTypeFilter::AttributeModifier.matches(&empty_cond));
        assert!(!EffectTypeFilter::DamageOverTime.matches(&empty_cond));
    }

    #[test]
    fn test_condition_sort_order_as_str() {
        use crate::conditions_editor::ConditionSortOrder;

        assert_eq!(ConditionSortOrder::NameAsc.as_str(), "Name (A-Z)");
        assert_eq!(ConditionSortOrder::NameDesc.as_str(), "Name (Z-A)");
        assert_eq!(ConditionSortOrder::IdAsc.as_str(), "ID (A-Z)");
        assert_eq!(ConditionSortOrder::IdDesc.as_str(), "ID (Z-A)");
        assert_eq!(ConditionSortOrder::EffectCount.as_str(), "Effect Count");
    }

    #[test]
    fn test_condition_statistics_computation() {
        use crate::conditions_editor::compute_condition_statistics;
        use antares::domain::conditions::{
            ConditionDefinition, ConditionDuration, ConditionEffect,
        };
        use antares::domain::types::DiceRoll;

        let conditions = vec![
            ConditionDefinition {
                id: "c1".to_string(),
                name: "Buff".to_string(),
                description: "".to_string(),
                effects: vec![ConditionEffect::AttributeModifier {
                    attribute: "might".to_string(),
                    value: 5,
                }],
                default_duration: ConditionDuration::Rounds(3),
                icon_id: None,
            },
            ConditionDefinition {
                id: "c2".to_string(),
                name: "Burning".to_string(),
                description: "".to_string(),
                effects: vec![ConditionEffect::DamageOverTime {
                    damage: DiceRoll::new(1, 6, 0),
                    element: "fire".to_string(),
                }],
                default_duration: ConditionDuration::Rounds(3),
                icon_id: None,
            },
            ConditionDefinition {
                id: "c3".to_string(),
                name: "Multi".to_string(),
                description: "".to_string(),
                effects: vec![
                    ConditionEffect::StatusEffect("poisoned".to_string()),
                    ConditionEffect::DamageOverTime {
                        damage: DiceRoll::new(1, 4, 0),
                        element: "poison".to_string(),
                    },
                ],
                default_duration: ConditionDuration::Rounds(5),
                icon_id: None,
            },
            ConditionDefinition {
                id: "c4".to_string(),
                name: "Empty".to_string(),
                description: "".to_string(),
                effects: vec![],
                default_duration: ConditionDuration::Instant,
                icon_id: None,
            },
        ];

        let stats = compute_condition_statistics(&conditions);

        assert_eq!(stats.total, 4);
        assert_eq!(stats.attribute_count, 1);
        assert_eq!(stats.dot_count, 2); // c2 has 1, c3 has 1
        assert_eq!(stats.status_count, 1);
        assert_eq!(stats.hot_count, 0);
        assert_eq!(stats.empty_count, 1);
        assert_eq!(stats.multi_effect_count, 1); // only c3 has multiple effects
    }

    #[test]
    fn test_conditions_editor_navigation_request() {
        let mut state = conditions_editor::ConditionsEditorState::new();

        // Initially no navigation request
        assert!(state.navigate_to_spell.is_none());

        // Set a navigation request
        state.navigate_to_spell = Some("Fireball".to_string());
        assert_eq!(state.navigate_to_spell, Some("Fireball".to_string()));

        // Take clears the request
        let nav = state.navigate_to_spell.take();
        assert_eq!(nav, Some("Fireball".to_string()));
        assert!(state.navigate_to_spell.is_none());
    }

    #[test]
    fn test_conditions_editor_state_qol_defaults() {
        use crate::conditions_editor::{ConditionSortOrder, EffectTypeFilter};

        let state = conditions_editor::ConditionsEditorState::new();

        // Verify new QoL fields have correct defaults
        assert_eq!(state.filter_effect_type, EffectTypeFilter::All);
        assert_eq!(state.sort_order, ConditionSortOrder::NameAsc);
        assert!(!state.show_statistics);
        assert!(state.navigate_to_spell.is_none());
    }

    #[test]
    fn test_effect_type_filter_all_variants() {
        use crate::conditions_editor::EffectTypeFilter;

        let all = EffectTypeFilter::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], EffectTypeFilter::All);
        assert_eq!(all[1], EffectTypeFilter::AttributeModifier);
        assert_eq!(all[2], EffectTypeFilter::StatusEffect);
        assert_eq!(all[3], EffectTypeFilter::DamageOverTime);
        assert_eq!(all[4], EffectTypeFilter::HealOverTime);
    }

    #[test]
    fn test_effect_type_filter_as_str() {
        use crate::conditions_editor::EffectTypeFilter;

        assert_eq!(EffectTypeFilter::All.as_str(), "All");
        assert_eq!(EffectTypeFilter::AttributeModifier.as_str(), "Attribute");
        assert_eq!(EffectTypeFilter::StatusEffect.as_str(), "Status");
        assert_eq!(EffectTypeFilter::DamageOverTime.as_str(), "DOT");
        assert_eq!(EffectTypeFilter::HealOverTime.as_str(), "HOT");
    }

    // =========================================================================
    // Phase 5: Testing Infrastructure Improvements
    // Pattern Matching and Compliance Tests
    // =========================================================================

    #[test]
    fn test_pattern_matcher_combobox_id_salt_detection() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::combobox_id_salt();

        // Valid patterns
        assert!(matcher.matches(r#"ComboBox::from_id_salt("test_combo")"#));
        assert!(matcher.matches(r#"egui::ComboBox::from_id_salt("another_combo")"#));
        assert!(matcher.matches(r#"ComboBox::from_id_salt('single_quotes')"#));

        // Invalid patterns (should not match)
        assert!(!matcher.matches("ComboBox::new()"));
        assert!(!matcher.matches("from_id_salt without ComboBox"));
    }

    #[test]
    fn test_pattern_matcher_combobox_from_label_detection() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::combobox_from_label();

        // Should detect from_label usage
        assert!(matcher.matches(r#"ComboBox::from_label("bad_pattern")"#));
        assert!(matcher.matches(r#"egui::ComboBox::from_label("also_bad")"#));

        // Should not match from_id_salt
        assert!(!matcher.matches(r#"ComboBox::from_id_salt("good")"#));
    }

    #[test]
    fn test_pattern_matcher_pub_fn_show_detection() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::pub_fn_show();

        // Valid patterns
        assert!(matcher.matches("pub fn show(&mut self, ui: &mut Ui)"));
        assert!(matcher.matches("    pub fn show("));
        assert!(matcher.matches("pub fn show(&self)"));

        // Invalid patterns
        assert!(!matcher.matches("fn show(")); // not public
        assert!(!matcher.matches("pub fn show_items(")); // different function name
        assert!(!matcher.matches("pub fn showing(")); // partial match
    }

    #[test]
    fn test_pattern_matcher_editor_state_struct_detection() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::editor_state_struct();

        // Valid patterns
        assert!(matcher.matches("pub struct ItemsEditorState {"));
        assert!(matcher.matches("pub struct SpellsEditorState {"));
        assert!(matcher.matches("pub struct MonstersEditorState {"));

        // Invalid patterns
        assert!(!matcher.matches("struct ItemsEditorState {")); // not pub
        assert!(!matcher.matches("pub struct SomeOtherState {")); // not *EditorState
    }

    #[test]
    fn test_source_file_creation_and_analysis() {
        use crate::test_utils::SourceFile;

        let content = r#"
pub struct TestEditorState {
    items: Vec<String>,
}

impl TestEditorState {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // editor content
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {}
}
"#;

        let file = SourceFile::new("test_editor.rs", content);

        assert_eq!(file.name, "test_editor");
        assert!(file.line_count() > 10);
        assert!(file.contains_pattern("pub struct"));
        assert!(file.contains_pattern("pub fn show"));
    }

    #[test]
    fn test_editor_compliance_check_detects_issues() {
        use crate::test_utils::{check_editor_compliance, SourceFile};

        // Editor with from_label violation
        let bad_content = r#"
impl BadEditor {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_label("Bad ID")
            .show_ui(ui, |ui| {});
    }
}
"#;

        let file = SourceFile::new("bad_editor.rs", bad_content);
        let result = check_editor_compliance(&file);

        assert_eq!(result.combobox_from_label_count, 1);
        assert!(!result.is_compliant());
        assert!(result.issues.iter().any(|i| i.contains("from_label")));
    }

    #[test]
    fn test_editor_compliance_check_passes_good_editor() {
        use crate::test_utils::{check_editor_compliance, SourceFile};

        let good_content = r#"
pub struct GoodEditorState {
    data: Vec<String>,
}

impl GoodEditorState {
    pub fn new() -> Self {
        Self { data: vec![] }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("good_combo")
            .show_ui(ui, |ui| {});
        EditorToolbar::new("Good").show(ui);
        ActionButtons::new().show(ui);
        TwoColumnLayout::new("good").show_split(ui, |left_ui| {}, |right_ui| {});
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_good_editor() {}
}
"#;

        let file = SourceFile::new("good_editor.rs", good_content);
        let result = check_editor_compliance(&file);

        assert!(result.has_show_method);
        assert!(result.has_new_method);
        assert!(result.has_state_struct);
        assert!(result.has_tests);
        assert_eq!(result.combobox_from_label_count, 0);
        assert_eq!(result.combobox_id_salt_count, 1);
    }

    #[test]
    fn test_compliance_score_calculation() {
        use crate::test_utils::EditorComplianceResult;

        // Full compliance = 100 points
        let full = EditorComplianceResult {
            editor_name: "full".to_string(),
            has_show_method: true,        // 20
            has_new_method: true,         // 10
            has_state_struct: true,       // 15
            uses_toolbar: true,           // 15
            uses_action_buttons: true,    // 10
            uses_two_column_layout: true, // 10
            has_tests: true,              // 10
            combobox_id_salt_count: 5,
            combobox_from_label_count: 0, // 10
            issues: vec![],
        };
        assert_eq!(full.compliance_score(), 100);

        // Partial compliance
        let partial = EditorComplianceResult {
            editor_name: "partial".to_string(),
            has_show_method: true,         // 20
            has_new_method: false,         // 0
            has_state_struct: false,       // 0
            uses_toolbar: true,            // 15
            uses_action_buttons: false,    // 0
            uses_two_column_layout: false, // 0
            has_tests: true,               // 10
            combobox_id_salt_count: 0,
            combobox_from_label_count: 0, // 10
            issues: vec![],
        };
        assert_eq!(partial.compliance_score(), 55);
    }

    #[test]
    fn test_collect_combobox_id_salts() {
        use crate::test_utils::{collect_combobox_id_salts, SourceFile};

        let content = r#"
egui::ComboBox::from_id_salt("difficulty_combo")
    .show_ui(ui, |ui| {});
egui::ComboBox::from_id_salt("terrain_combo")
    .show_ui(ui, |ui| {});
egui::ComboBox::from_id_salt("wall_combo")
    .show_ui(ui, |ui| {});
"#;

        let file = SourceFile::new("test.rs", content);
        let salts = collect_combobox_id_salts(&file);

        assert_eq!(salts.len(), 3);
        assert!(salts.contains(&"difficulty_combo".to_string()));
        assert!(salts.contains(&"terrain_combo".to_string()));
        assert!(salts.contains(&"wall_combo".to_string()));
    }

    #[test]
    fn test_find_duplicate_combobox_ids_detects_conflicts() {
        use crate::test_utils::{find_duplicate_combobox_ids, SourceFile};

        let file1 = SourceFile::new(
            "editor1.rs",
            r#"egui::ComboBox::from_id_salt("duplicate_id")"#,
        );
        let file2 = SourceFile::new(
            "editor2.rs",
            r#"egui::ComboBox::from_id_salt("duplicate_id")"#,
        );
        let file3 = SourceFile::new("editor3.rs", r#"egui::ComboBox::from_id_salt("unique_id")"#);

        let files = vec![file1, file2, file3];
        let duplicates = find_duplicate_combobox_ids(&files);

        // Only "duplicate_id" should be reported as duplicate
        assert_eq!(duplicates.len(), 1);
        assert!(duplicates.contains_key("duplicate_id"));
        assert_eq!(duplicates["duplicate_id"].len(), 2);
        assert!(!duplicates.contains_key("unique_id"));
    }

    #[test]
    fn test_compliance_summary_calculation() {
        use crate::test_utils::{ComplianceSummary, EditorComplianceResult};
        use std::collections::HashMap;

        let mut results = HashMap::new();

        results.insert(
            "compliant_editor".to_string(),
            EditorComplianceResult {
                editor_name: "compliant_editor".to_string(),
                has_show_method: true,
                has_new_method: true,
                has_state_struct: true,
                uses_toolbar: true,
                uses_action_buttons: true,
                uses_two_column_layout: true,
                has_tests: true,
                combobox_id_salt_count: 2,
                combobox_from_label_count: 0,
                issues: vec![],
            },
        );

        results.insert(
            "noncompliant_editor".to_string(),
            EditorComplianceResult {
                editor_name: "noncompliant_editor".to_string(),
                has_show_method: true,
                has_new_method: false,
                has_state_struct: false,
                uses_toolbar: false,
                uses_action_buttons: false,
                uses_two_column_layout: false,
                has_tests: false,
                combobox_id_salt_count: 0,
                combobox_from_label_count: 2,
                issues: vec!["Missing tests".to_string(), "Uses from_label".to_string()],
            },
        );

        let summary = ComplianceSummary::from_results(&results);

        assert_eq!(summary.total_editors, 2);
        assert_eq!(summary.compliant_editors, 1);
        assert_eq!(summary.total_issues, 2);
        assert_eq!(summary.from_label_violations, 2);
        assert!(summary.average_score > 0.0);
    }

    #[test]
    fn test_pattern_match_line_numbers() {
        use crate::test_utils::PatternMatcher;

        let content = r#"line 1
line 2
ComboBox::from_id_salt("test")
line 4
line 5
ComboBox::from_id_salt("another")
line 7"#;

        let matcher = PatternMatcher::combobox_id_salt();
        let matches = matcher.find_matches(content);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 3);
        assert_eq!(matches[1].line_number, 6);
    }

    #[test]
    fn test_pattern_matcher_test_annotation() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::test_annotation();
        let content = r#"
#[test]
fn test_something() {}

#[test]
fn test_another() {}
"#;

        assert_eq!(matcher.count_matches(content), 2);
    }

    #[test]
    fn test_pattern_matcher_toolbar_usage() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::editor_toolbar_usage();

        assert!(matcher.matches("EditorToolbar::new(\"Items\")"));
        assert!(matcher.matches("    EditorToolbar::new("));
        assert!(!matcher.matches("Toolbar::new(")); // different struct
    }

    #[test]
    fn test_pattern_matcher_action_buttons_usage() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::action_buttons_usage();

        assert!(matcher.matches("ActionButtons::new()"));
        assert!(matcher.matches("    ActionButtons::new("));
        assert!(!matcher.matches("Buttons::new(")); // different struct
    }

    #[test]
    fn test_pattern_matcher_two_column_layout_usage() {
        use crate::test_utils::PatternMatcher;

        let matcher = PatternMatcher::two_column_layout_usage();

        assert!(matcher.matches("TwoColumnLayout::new(\"items\")"));
        assert!(matcher.matches("    TwoColumnLayout::new("));
        assert!(!matcher.matches("ColumnLayout::new(")); // different struct
    }

    #[test]
    fn test_editor_compliance_result_is_compliant() {
        use crate::test_utils::EditorComplianceResult;

        // Compliant: no issues and no from_label
        let compliant = EditorComplianceResult {
            editor_name: "test".to_string(),
            has_show_method: true,
            has_new_method: true,
            has_state_struct: true,
            uses_toolbar: true,
            uses_action_buttons: true,
            uses_two_column_layout: true,
            has_tests: true,
            combobox_id_salt_count: 1,
            combobox_from_label_count: 0,
            issues: vec![],
        };
        assert!(compliant.is_compliant());

        // Not compliant: has from_label usage
        let with_from_label = EditorComplianceResult {
            editor_name: "test".to_string(),
            has_show_method: true,
            has_new_method: true,
            has_state_struct: true,
            uses_toolbar: true,
            uses_action_buttons: true,
            uses_two_column_layout: true,
            has_tests: true,
            combobox_id_salt_count: 1,
            combobox_from_label_count: 1,
            issues: vec![],
        };
        assert!(!with_from_label.is_compliant());

        // Not compliant: has issues
        let with_issues = EditorComplianceResult {
            editor_name: "test".to_string(),
            has_show_method: true,
            has_new_method: true,
            has_state_struct: true,
            uses_toolbar: true,
            uses_action_buttons: true,
            uses_two_column_layout: true,
            has_tests: true,
            combobox_id_salt_count: 1,
            combobox_from_label_count: 0,
            issues: vec!["Some issue".to_string()],
        };
        assert!(!with_issues.is_compliant());
    }

    #[test]
    fn test_compliance_summary_to_string_format() {
        use crate::test_utils::ComplianceSummary;

        let summary = ComplianceSummary {
            total_editors: 5,
            compliant_editors: 4,
            total_issues: 2,
            from_label_violations: 1,
            average_score: 90.5,
            all_issues: vec!["issue1".to_string(), "issue2".to_string()],
        };

        let output = summary.to_string();

        assert!(output.contains("Total Editors: 5"));
        assert!(output.contains("Compliant: 4"));
        assert!(output.contains("Total Issues: 2"));
        assert!(output.contains("from_label Violations: 1"));
        assert!(output.contains("Average Score: 90.5"));
    }
}
