// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder for Antares SDK
//!
//! Features:
//! - Full metadata editor with all campaign.ron fields
//! - Real file I/O (save/load campaign.ron)
//! - Enhanced validation UI with detailed error reporting
//! - File structure browser showing campaign directory layout
//! - Data editors for all game content types
//! - Unsaved changes tracking and warnings

pub mod advanced_validation;
pub mod animation_editor;
pub mod app_dialogs;
pub mod asset_manager;
pub mod auto_save;
pub mod campaign_editor;
pub mod campaign_io;
pub mod characters_editor;
pub mod classes_editor;
pub mod color_palette;
pub mod conditions_editor;
pub mod config_editor;
pub mod context_menu;
pub mod creature_assets;
pub mod creature_id_manager;
pub mod creature_templates;
pub mod creature_undo_redo;
pub mod creatures_editor;
pub mod creatures_manager;
pub mod creatures_workflow;
pub mod dialogue_editor;
pub mod editor_context;
pub mod editor_state;
pub mod furniture_editor;
pub mod icon;
pub mod item_mesh_editor;
pub mod item_mesh_undo_redo;
pub mod item_mesh_workflow;
pub mod items_editor;
pub mod keyboard_shortcuts;
pub mod linear_history;
pub mod lod_editor;
pub mod logging;
pub mod map_editor;
pub mod material_editor;
pub mod mesh_index_editor;
pub mod mesh_normal_editor;
pub mod mesh_obj_io;
pub mod mesh_validation;
pub mod mesh_vertex_editor;
pub mod monsters_editor;
pub mod npc_editor;
pub mod obj_importer;
pub mod obj_importer_ui;
pub mod packager;
pub mod preview_features;
pub mod preview_renderer;
pub mod primitive_generators;
pub mod proficiencies_editor;
pub mod quest_editor;
pub mod races_editor;
pub mod spells_editor;
pub mod stock_templates_editor;
pub mod template_browser;
pub mod template_metadata;
pub mod templates;
pub mod test_play;
pub mod test_utils;
#[cfg(target_os = "macos")]
pub mod tray;
pub mod ui_helpers;
pub mod undo_redo;
pub mod validation;
pub mod variation_editor;

use antares::sdk::tool_config::ToolConfig;
use logging::{category, LogLevel, Logger};

use antares::domain::character::Stats;
use antares::domain::character::{FOOD_MAX, FOOD_MIN, PARTY_MAX_SIZE};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType};
use antares::domain::conditions::ConditionDefinition;
use antares::domain::dialogue::{DialogueAction, DialogueTree};
use antares::domain::items::types::Item;
use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
use antares::domain::magic::types::Spell;
use antares::domain::magic::types::{SpellContext, SpellSchool, SpellTarget};
use antares::domain::proficiency::ProficiencyDefinition;
use antares::domain::quest::Quest;
use antares::domain::quest::QuestId;
use antares::domain::types::{CreatureId, GameTime};
use antares::domain::types::{DiceRoll, ItemId, MapId, MonsterId, SpellId};
use antares::domain::visual::CreatureReference;
use antares::domain::world::npc_runtime::MerchantStockTemplate;
use antares::domain::world::Map;
use conditions_editor::ConditionsEditorState;
use dialogue_editor::DialogueEditorState;
use editor_context::EditorContext;
use eframe::egui;
use items_editor::ItemsEditorState;
use map_editor::MapsEditorState;
use monsters_editor::MonstersEditorState;
use quest_editor::QuestEditorState;
use serde::{Deserialize, Serialize};
use spells_editor::SpellsEditorState;
use std::fs;
use std::path::PathBuf;
use stock_templates_editor::StockTemplatesEditorState;
use thiserror::Error;

const STARTING_GOLD_MAX: u32 = 100_000;

pub fn run() -> Result<(), eframe::Error> {
    // Initialize logger from command-line arguments
    let mut logger = Logger::from_args();
    let log_level = logger.level();

    // Print startup message based on log level
    if log_level >= LogLevel::Info {
        logger.info(
            category::APP,
            &format!(
                "Antares Campaign Builder starting (log level: {})",
                log_level
            ),
        );
    }
    if log_level >= LogLevel::Verbose {
        logger.verbose(
            category::APP,
            "Verbose logging enabled - showing detailed trace information",
        );
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 720.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Antares Campaign Builder");

    match icon::app_icon_data() {
        Ok(icon_data) => {
            viewport = viewport.with_icon(icon_data);
        }
        Err(e) => {
            logger.warn(
                category::APP,
                &format!("Failed to decode application icon: {e}"),
            );
        }
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::default(),
        ..Default::default()
    };

    // Build the macOS menu-bar status item (NSStatusItem).  The TrayIcon binding
    // must remain live for the entire duration of `run_native`; dropping it
    // removes the icon from the menu bar.  The Receiver is moved into the app
    // inside the closure so that `update()` can drain TrayCommand values each frame.
    #[cfg(target_os = "macos")]
    let (_tray, tray_cmd_rx) = tray::build_tray_icon();

    eframe::run_native(
        "Antares Campaign Builder",
        options,
        Box::new(move |_cc| {
            let mut app = CampaignBuilderApp {
                logger: logger.clone(),
                ..Default::default()
            };

            // Wire the tray-command receiver into the app so that `update()`
            // can drain ShowWindow / HideWindow / Quit commands each frame.
            #[cfg(target_os = "macos")]
            {
                app.tray_cmd_rx = Some(tray_cmd_rx);
            }

            // Load persisted ToolConfig if available; otherwise fall back to defaults.
            // This makes display/editor preferences persistent across sessions.
            if let Ok(cfg) = ToolConfig::load_or_default() {
                app.tool_config = cfg;
            }

            // Auto-load campaign if --campaign <path> provided on CLI.
            // Behavior:
            // - If the provided path is a directory, append "campaign.ron" and attempt to load it.
            // - If the provided path is a file, attempt to load that file directly.
            // - If the resulting campaign.ron does not exist or is not a file, emit an error and continue.
            let args: Vec<String> = std::env::args().collect();
            if let Some(pos) = args.iter().position(|a| a == "--campaign") {
                if let Some(p) = args.get(pos + 1) {
                    let provided = PathBuf::from(p);

                    // Determine the actual campaign file to load.
                    // If the provided path is a directory (or ends with a slash), append campaign.ron.
                    let mut campaign_file = provided.clone();
                    if provided.exists() && provided.is_dir() {
                        campaign_file = provided.join("campaign.ron");
                        app.logger.info(
                            category::CAMPAIGN,
                            &format!("Auto-loading campaign: {}", campaign_file.display()),
                        );
                    } else {
                        // Treat provided value as a file path (even if it doesn't exist yet).
                        app.logger.info(
                            category::CAMPAIGN,
                            &format!("Auto-loading campaign: {}", campaign_file.display()),
                        );
                    }

                    // Ensure the campaign file exists and is a regular file before attempting to read it.
                    if campaign_file.exists() && campaign_file.is_file() {
                        match app.load_campaign_file(&campaign_file) {
                            Ok(()) => {
                                // Set campaign path and directory (parent of campaign.ron)
                                app.campaign_path = Some(campaign_file.clone());
                                if let Some(parent) = campaign_file.parent() {
                                    let parent_buf = parent.to_path_buf();
                                    app.campaign_dir = Some(parent_buf.clone());
                                    app.update_file_tree(&parent_buf);
                                }

                                // Load data files (same sequence as do_open_campaign)
                                app.logger
                                    .debug(category::FILE_IO, "Loading data files (auto-load)...");
                                app.load_items();
                                app.load_spells();
                                app.load_proficiencies();
                                app.load_monsters();
                                app.load_creatures();
                                app.load_classes_from_campaign();
                                app.load_races_from_campaign();
                                app.load_characters_from_campaign();
                                app.load_maps();
                                app.load_conditions();
                                app.load_furniture();

                                if let Err(e) = app.load_quests() {
                                    app.logger.warn(
                                        category::FILE_IO,
                                        &format!("Failed to load quests: {}", e),
                                    );
                                }

                                if let Err(e) = app.load_dialogues() {
                                    app.logger.warn(
                                        category::FILE_IO,
                                        &format!("Failed to load dialogues: {}", e),
                                    );
                                }

                                if let Err(e) = app.load_npcs() {
                                    app.logger.warn(
                                        category::FILE_IO,
                                        &format!("Failed to load NPCs: {}", e),
                                    );
                                }

                                app.sync_obj_importer_campaign_state();

                                // Initialize AssetManager if we have a campaign directory
                                if let Some(ref campaign_dir) = app.campaign_dir {
                                    let mut manager =
                                        asset_manager::AssetManager::new(campaign_dir.clone());
                                    if let Err(e) = manager.scan_directory() {
                                        app.logger.warn(
                                            category::FILE_IO,
                                            &format!("Failed to scan assets: {}", e),
                                        );
                                    } else {
                                        let map_file_paths = app.discover_map_files();
                                        let data_files_cfg = asset_manager::DataFilesConfig {
                                            items_file: &app.campaign.items_file,
                                            spells_file: &app.campaign.spells_file,
                                            conditions_file: &app.campaign.conditions_file,
                                            monsters_file: &app.campaign.monsters_file,
                                            quests_file: &app.campaign.quests_file,
                                            classes_file: &app.campaign.classes_file,
                                            races_file: &app.campaign.races_file,
                                            characters_file: &app.campaign.characters_file,
                                            dialogue_file: &app.campaign.dialogue_file,
                                            npcs_file: &app.campaign.npcs_file,
                                            proficiencies_file: &app.campaign.proficiencies_file,
                                        };
                                        manager.init_data_files(&data_files_cfg, &map_file_paths);

                                        let campaign_refs = asset_manager::CampaignRefs {
                                            items: &app.campaign_data.items,
                                            quests: &app.campaign_data.quests,
                                            dialogues: &app.campaign_data.dialogues,
                                            maps: &app.campaign_data.maps,
                                            classes: &app
                                                .editor_registry
                                                .classes_editor_state
                                                .classes,
                                            characters: &app
                                                .editor_registry
                                                .characters_editor_state
                                                .characters,
                                            npcs: &app.editor_registry.npc_editor_state.npcs,
                                        };
                                        manager.scan_references(&campaign_refs);
                                        manager.mark_data_files_as_referenced();

                                        app.ui_state.status_message =
                                            format!("Scanned {} assets", manager.asset_count());
                                        app.asset_manager = Some(manager);

                                        // Auto-load races after asset manager initializes (keeps UI consistent)
                                        if app.campaign_dir.is_some() {
                                            app.load_races_from_campaign();
                                        }
                                    }
                                }

                                app.unsaved_changes = false;
                                app.logger.info(
                                    category::FILE_IO,
                                    &format!("Campaign auto-loaded: {}", app.campaign.name),
                                );
                                app.ui_state.status_message =
                                    format!("Opened campaign from: {}", campaign_file.display());

                                // Synchronize campaign editor state with the loaded campaign
                                app.editor_registry.campaign_editor_state.metadata =
                                    app.campaign.clone();
                                app.editor_registry.campaign_editor_state.buffer =
                                    campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                                        &app.editor_registry.campaign_editor_state.metadata,
                                    );
                                app.editor_registry
                                    .campaign_editor_state
                                    .has_unsaved_changes = false;
                                app.editor_registry.campaign_editor_state.mode =
                                    campaign_editor::CampaignEditorMode::List;
                            }
                            Err(e) => {
                                app.logger.error(
                                    category::FILE_IO,
                                    &format!("Failed to auto-load campaign: {}", e),
                                );
                                app.ui_state.status_message =
                                    format!("Failed to auto-load campaign: {}", e);
                            }
                        }
                    } else {
                        // campaign.ron missing or not a file at the resolved location — report error.
                        app.logger.error(
                            category::FILE_IO,
                            &format!(
                                "campaign.ron not found at expected path: {}",
                                campaign_file.display()
                            ),
                        );
                        app.ui_state.status_message =
                            format!("campaign.ron not found: {}", campaign_file.display());
                    }
                } else {
                    app.logger.warn(
                        category::FILE_IO,
                        "--campaign flag provided but no path given",
                    );
                }
            }

            app.logger.info(category::APP, "Application initialized");
            Ok(Box::new(app))
        }),
    )
}

/// Campaign metadata structure matching campaign.ron schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub engine_version: String,

    // Campaign configuration
    pub starting_map: String,
    pub starting_position: (u32, u32),
    pub starting_direction: String,
    pub starting_gold: u32,
    pub starting_food: u32,
    #[serde(default = "default_starting_innkeeper")]
    pub starting_innkeeper: String,
    pub max_party_size: usize,
    pub max_roster_size: usize,
    pub difficulty: Difficulty,
    pub permadeath: bool,
    pub allow_multiclassing: bool,
    pub starting_level: u8,
    pub max_level: u8,

    // Data file paths
    pub items_file: String,
    pub spells_file: String,
    pub monsters_file: String,
    pub classes_file: String,
    pub races_file: String,
    pub characters_file: String,
    pub maps_dir: String,
    pub quests_file: String,
    pub dialogue_file: String,
    pub conditions_file: String,
    pub npcs_file: String,
    #[serde(default = "default_proficiencies_file")]
    pub proficiencies_file: String,
    #[serde(default = "default_creatures_file")]
    pub creatures_file: String,
    /// Relative path to the NPC stock templates RON file
    #[serde(default = "default_stock_templates_file")]
    pub stock_templates_file: String,

    /// Relative path to the furniture definitions RON file
    ///
    /// Furniture support is opt-in per campaign. Existing `campaign.ron`
    /// files that omit this field will default to `"data/furniture.ron"`.
    #[serde(default = "default_furniture_file")]
    pub furniture_file: String,

    /// Starting game time for a new campaign (day, hour, minute).
    ///
    /// Defaults to Day 1, 08:00 (morning) if not specified in the RON file.
    /// The `serde(default)` attribute ensures existing `campaign.ron` files that
    /// lack this field continue to deserialize correctly.
    #[serde(default = "default_starting_time")]
    pub starting_time: GameTime,
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
    pub fn as_str(&self) -> &str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::Brutal => "Brutal",
        }
    }

    pub fn all() -> [Difficulty; 4] {
        [
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hard,
            Difficulty::Brutal,
        ]
    }
}

pub fn default_starting_innkeeper() -> String {
    "tutorial_innkeeper_town".to_string()
}

fn default_proficiencies_file() -> String {
    "data/proficiencies.ron".to_string()
}

fn default_creatures_file() -> String {
    "data/creatures.ron".to_string()
}

fn default_stock_templates_file() -> String {
    "data/npc_stock_templates.ron".to_string()
}

fn default_furniture_file() -> String {
    "data/furniture.ron".to_string()
}

/// Default starting time: Day 1, 08:00 — campaign begins in the morning.
pub fn default_starting_time() -> GameTime {
    GameTime::new(1, 8, 0)
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
            creatures_file: "data/creatures.ron".to_string(),
            stock_templates_file: "data/npc_stock_templates.ron".to_string(),
            furniture_file: "data/furniture.ron".to_string(),
            starting_time: default_starting_time(),
        }
    }
}

/// Active tab in the UI
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Metadata,
    Config,
    Items,
    ItemMeshes,
    Spells,
    Conditions,
    Monsters,
    Creatures,
    Furniture,
    Importer,
    Maps,
    Quests,
    Classes,
    Races,
    Characters,
    Dialogues,
    NPCs,
    Proficiencies,
    StockTemplates,
    Assets,
    Validation,
}

impl EditorTab {
    pub fn name(&self) -> &str {
        match self {
            EditorTab::Metadata => "Metadata",
            EditorTab::Config => "Config",
            EditorTab::Items => "Items",
            EditorTab::ItemMeshes => "Item Meshes",
            EditorTab::Spells => "Spells",
            EditorTab::Conditions => "Conditions",
            EditorTab::Monsters => "Monsters",
            EditorTab::Creatures => "Creatures",
            EditorTab::Furniture => "Furniture",
            EditorTab::Importer => "Importer",
            EditorTab::Maps => "Maps",
            EditorTab::Quests => "Quests",
            EditorTab::Classes => "Classes",
            EditorTab::Races => "Races",
            EditorTab::Characters => "Characters",
            EditorTab::Dialogues => "Dialogues",
            EditorTab::NPCs => "NPCs",
            EditorTab::Proficiencies => "Proficiencies",
            EditorTab::StockTemplates => "Stock Templates",
            EditorTab::Assets => "Assets",
            EditorTab::Validation => "Validation",
        }
    }
}

// NOTE: ValidationError and Severity types have been replaced by the validation module.
// Use validation::ValidationResult and validation::ValidationSeverity instead.

/// Quick validation filter for the Validation Panel.
///
/// The filter controls which severities the UI will display. These are user-facing
/// selections and are persisted via the `CampaignBuilderApp` state.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationFilter {
    /// Show all severities (default)
    All,
    /// Only show `Error` severity checks
    ErrorsOnly,
    /// Only show `Warning` severity checks
    WarningsOnly,
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

/// Main application state — the coordinator that owns all sub-state structs.
///
/// The raw game-content data lives in [`editor_state::CampaignData`]; the
/// editor instances in [`editor_state::EditorRegistry`]; UI visibility flags
/// in [`editor_state::EditorUiState`]; and validation results in
/// [`editor_state::ValidationState`].
#[doc(hidden)]
pub struct CampaignBuilderApp {
    // ─── Campaign identity ────────────────────────────────────────────────
    pub campaign: CampaignMetadata,
    pub campaign_path: Option<PathBuf>,
    pub campaign_dir: Option<PathBuf>,
    pub unsaved_changes: bool,
    pending_action: Option<PendingAction>,

    // ─── Special editors ─────────────────────────────────────────────────
    item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState,
    obj_importer_state: obj_importer::ObjImporterState,

    // ─── Runtime services ────────────────────────────────────────────────
    undo_redo_manager: undo_redo::UndoRedoManager,
    pub asset_manager: Option<asset_manager::AssetManager>,
    template_manager: templates::TemplateManager,
    pub creature_template_registry: template_metadata::TemplateRegistry,
    creature_template_browser_state: template_browser::TemplateBrowserState,
    logger: Logger,
    tool_config: ToolConfig,
    logo_texture: Option<egui::TextureHandle>,

    // ─── Grouped state (each counts as 1 field) ───────────────────────────
    pub campaign_data: editor_state::CampaignData,
    pub editor_registry: editor_state::EditorRegistry,
    pub ui_state: editor_state::EditorUiState,
    pub validation_state: editor_state::ValidationState,

    // ─── Future / unused fields ──────────────────────────────────────────
    _export_wizard: Option<packager::ExportWizard>,
    _test_play_session: Option<test_play::TestPlaySession>,
    _test_play_config: test_play::TestPlayConfig,
    _show_export_dialog: bool,
    _show_test_play_panel: bool,

    /// Receiver for macOS menu-bar tray commands.
    #[cfg(target_os = "macos")]
    tray_cmd_rx: Option<std::sync::mpsc::Receiver<tray::TrayCommand>>,
}

#[derive(Debug, Clone)]
enum PendingAction {
    New,
    Open,
    Exit,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub is_directory: bool,
    pub _children: Vec<FileNode>,
}

impl Default for CampaignBuilderApp {
    fn default() -> Self {
        Self {
            campaign: CampaignMetadata::default(),
            campaign_path: None,
            campaign_dir: None,
            unsaved_changes: false,
            pending_action: None,
            item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState::new(),
            obj_importer_state: obj_importer::ObjImporterState::new(),
            undo_redo_manager: undo_redo::UndoRedoManager::new(),
            asset_manager: None,
            template_manager: templates::TemplateManager::new(),
            creature_template_registry: creature_templates::initialize_template_registry(),
            creature_template_browser_state: template_browser::TemplateBrowserState::new(),
            logger: Logger::default(),
            tool_config: ToolConfig::default(),
            logo_texture: None,
            campaign_data: editor_state::CampaignData::default(),
            editor_registry: editor_state::EditorRegistry::default(),
            ui_state: editor_state::EditorUiState::default(),
            validation_state: editor_state::ValidationState::default(),
            _export_wizard: None,
            _test_play_session: None,
            _test_play_config: test_play::TestPlayConfig::default(),
            _show_export_dialog: false,
            _show_test_play_panel: false,
            #[cfg(target_os = "macos")]
            tray_cmd_rx: None,
        }
    }
}

impl eframe::App for CampaignBuilderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll macOS menu-bar status item events once per frame.
        // `handle_tray_events` translates raw MenuEvent IDs into TrayCommand
        // values and sends them over the mpsc channel.  The Receiver is drained
        // immediately below.
        #[cfg(target_os = "macos")]
        tray::handle_tray_events();

        // Drain tray commands and issue the corresponding egui ViewportCommands.
        // This runs on every frame so the window responds within one paint cycle.
        #[cfg(target_os = "macos")]
        if let Some(ref rx) = self.tray_cmd_rx {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    tray::TrayCommand::ShowWindow => {
                        // Restore visibility first, then bring the window to
                        // the front so it is immediately usable.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    tray::TrayCommand::HideWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    tray::TrayCommand::Quit => {
                        // Close the egui viewport gracefully.  The tray's Quit
                        // item also calls `process::exit` directly in
                        // `handle_tray_events`, so this path is a belt-and-
                        // suspenders fallback.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

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
                                self.ui_state.status_message = format!("Save failed: {}", e);
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
                        match self.undo_redo_manager.undo(&mut self.campaign_data) {
                            Ok(desc) => {
                                self.ui_state.status_message = format!("Undid: {}", desc);
                                self.unsaved_changes = true;
                            }
                            Err(e) => self.ui_state.status_message = e,
                        }
                        ui.close();
                    }
                    if ui
                        .add_enabled(can_redo, egui::Button::new("↷ Redo"))
                        .clicked()
                    {
                        match self.undo_redo_manager.redo(&mut self.campaign_data) {
                            Ok(desc) => {
                                self.ui_state.status_message = format!("Redid: {}", desc);
                                self.unsaved_changes = true;
                            }
                            Err(e) => self.ui_state.status_message = e,
                        }
                        ui.close();
                    }
                });

                ui.menu_button("View", |ui| {
                    // Debug panel toggle
                    let debug_label = if self.ui_state.show_debug_panel {
                        "🐛 Hide Debug Panel"
                    } else {
                        "🐛 Show Debug Panel"
                    };
                    if ui.button(debug_label).clicked() {
                        self.ui_state.show_debug_panel = !self.ui_state.show_debug_panel;
                        self.logger.info(
                            category::UI,
                            &format!(
                                "Debug panel {}",
                                if self.ui_state.show_debug_panel {
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
                        self.ui_state.show_template_browser = true;
                        ui.close();
                    }
                    if ui.button("🦎 Creature Editor").clicked() {
                        self.ui_state.active_tab = EditorTab::Creatures;
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                    if ui.button("🐉 Creature Templates...").clicked() {
                        self.ui_state.show_creature_template_browser = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("✅ Validate Campaign").clicked() {
                        self.validate_campaign();
                        self.ui_state.active_tab = EditorTab::Validation;
                        ui.close();
                    }
                    if ui.button("📊 Advanced Validation Report...").clicked() {
                        self.run_advanced_validation();
                        self.validation_state.show_validation_report = true;
                        ui.close();
                    }
                    if ui.button("⚖️ Balance Statistics...").clicked() {
                        self.ui_state.show_balance_stats = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🔄 Refresh File Tree").clicked() {
                        if let Some(dir) = self.campaign_dir.clone() {
                            self.update_file_tree(&dir);
                            self.ui_state.status_message = "File tree refreshed.".to_string();
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🧪 Test Play").clicked() {
                        self.ui_state.status_message =
                            "Test play would launch the game here...".to_string();
                        ui.close();
                    }
                    if ui.button("📦 Export Campaign...").clicked() {
                        self.ui_state.status_message =
                            "Export would create .zip archive here...".to_string();
                        ui.close();
                    }
                    ui.separator();
                    // Preferences dialog toggle
                    if ui.button("⚙️ Preferences...").clicked() {
                        self.ui_state.show_preferences = true;
                        ui.close();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("📖 Documentation").clicked() {
                        self.ui_state.status_message =
                            "Would open documentation in browser...".to_string();
                        ui.close();
                    }
                    if ui.button("ℹ️ About").clicked() {
                        self.ui_state.show_about_dialog = true;
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
                    match self.undo_redo_manager.redo(&mut self.campaign_data) {
                        Ok(desc) => {
                            self.ui_state.status_message = format!("Redid: {}", desc);
                            self.unsaved_changes = true;
                        }
                        Err(e) => self.ui_state.status_message = e,
                    }
                }
            } else {
                // Ctrl+Z = Undo
                if self.undo_redo_manager.can_undo() {
                    match self.undo_redo_manager.undo(&mut self.campaign_data) {
                        Ok(desc) => {
                            self.ui_state.status_message = format!("Undid: {}", desc);
                            self.unsaved_changes = true;
                        }
                        Err(e) => self.ui_state.status_message = e,
                    }
                }
            }
        }

        // Ctrl+Y = Redo (alternative)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y))
            && self.undo_redo_manager.can_redo()
        {
            match self.undo_redo_manager.redo(&mut self.campaign_data) {
                Ok(desc) => {
                    self.ui_state.status_message = format!("Redid: {}", desc);
                    self.unsaved_changes = true;
                }
                Err(e) => self.ui_state.status_message = e,
            }
        }

        // F12 = Toggle Debug Panel
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.ui_state.show_debug_panel = !self.ui_state.show_debug_panel;
            self.logger.info(
                category::UI,
                &format!(
                    "Debug panel {} (F12)",
                    if self.ui_state.show_debug_panel {
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
                    EditorTab::ItemMeshes,
                    EditorTab::Spells,
                    EditorTab::Conditions,
                    EditorTab::Monsters,
                    EditorTab::Creatures,
                    EditorTab::Furniture,
                    EditorTab::Importer,
                    EditorTab::Maps,
                    EditorTab::Quests,
                    EditorTab::Classes,
                    EditorTab::Races,
                    EditorTab::Characters,
                    EditorTab::Dialogues,
                    EditorTab::NPCs,
                    EditorTab::Proficiencies,
                    EditorTab::StockTemplates,
                    EditorTab::Assets,
                    EditorTab::Validation,
                ];

                for tab in &tabs {
                    let is_selected = self.ui_state.active_tab == *tab;
                    if ui.selectable_label(is_selected, tab.name()).clicked() {
                        let previous_tab = self.ui_state.active_tab;
                        self.ui_state.active_tab = *tab;
                        self.logger.debug(
                            category::EDITOR,
                            &format!("Tab changed: {} -> {}", previous_tab.name(), tab.name()),
                        );
                        ui.ctx().request_repaint();
                    }
                }

                ui.separator();

                ui.label("Antares RPG");
                ui.label("Campaign Builder");
                ui.label("Foundation v0.2.0");

                // Pin the logo to the very bottom of the sidebar using a
                // bottom-up layout so it never overlaps the tab list above.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    // Lazily decode and upload the logo PNG on first render,
                    // then reuse the cached TextureHandle every subsequent frame.
                    let logo_bytes: &[u8] = include_bytes!("../assets/antares_logo.png");
                    let texture = self.logo_texture.get_or_insert_with(|| {
                        let img = image::load_from_memory(logo_bytes)
                            .expect("antares_logo.png is a valid PNG");
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let pixels: Vec<egui::Color32> = rgba
                            .pixels()
                            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        ctx.load_texture(
                            "antares_logo",
                            egui::ColorImage::new([w as usize, h as usize], pixels),
                            egui::TextureOptions::LINEAR,
                        )
                    });

                    // Fill the sidebar width; image is square so aspect ratio holds.
                    let available_width = ui.available_width();
                    let logo_size = egui::vec2(available_width, available_width);
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(texture.id(), logo_size))
                            .maintain_aspect_ratio(true),
                    );
                });
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(&self.ui_state.status_message);

                if let Some(path) = &self.campaign_path {
                    ui.separator();
                    ui.label(format!("Path: {}", path.display()));
                }
            });
        });

        self.handle_maps_open_npc_request();
        self.handle_validation_open_npc_request();

        // Central panel with editor content
        egui::CentralPanel::default().show(ctx, |ui| match self.ui_state.active_tab {
            EditorTab::Metadata => self.show_metadata_editor(ui),
            EditorTab::ItemMeshes => {
                if let Some(signal) = self
                    .item_mesh_editor_state
                    .show(ui, self.campaign_dir.as_ref())
                {
                    match signal {
                        item_mesh_editor::ItemMeshEditorSignal::OpenInItemsEditor(item_id) => {
                            if let Some(idx) = self.campaign_data.items.iter().position(|it| it.id == item_id) {
                                self.ui_state.active_tab = EditorTab::Items;
                                self.editor_registry.items_editor_state.selected_item = Some(idx);
                                self.editor_registry.items_editor_state.mode = items_editor::ItemsEditorMode::Edit;
                                self.editor_registry.items_editor_state.edit_buffer = self.campaign_data.items[idx].clone();
                                self.ui_state.status_message = format!("Opening item #{}", item_id);
                                ui.ctx().request_repaint();
                            }
                        }
                    }
                }
                // Cross-tab: items editor wants to open item mesh editor
                if let Some(item_id) = self.editor_registry.items_editor_state.requested_open_item_mesh.take() {
                    self.ui_state.active_tab = EditorTab::ItemMeshes;
                    self.ui_state.status_message = format!("Opening Item Mesh Editor for item #{}", item_id);
                    ui.ctx().request_repaint();
                }
            }
            EditorTab::Config => self.editor_registry.config_editor_state.show(
                ui,
                self.campaign_dir.as_ref(),
                &mut self.unsaved_changes,
                &mut self.ui_state.status_message,
            ),
            EditorTab::Items => {
                let mut items_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.items_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.items_editor_state.show(
                    ui,
                    &mut self.campaign_data.items,
                    &self.editor_registry.classes_editor_state.classes,
                    &self.campaign_data.spells,
                    &mut items_ctx,
                );
                // Handle cross-tab navigation: items editor wants to open the
                // Item Mesh Editor for a specific item.
                if let Some(item_id) = self.editor_registry.items_editor_state.requested_open_item_mesh.take() {
                    self.ui_state.active_tab = EditorTab::ItemMeshes;
                    self.ui_state.status_message = format!("Opening Item Mesh Editor for item #{}", item_id);
                    ui.ctx().request_repaint();
                }
            }
            EditorTab::Spells => {
                let mut spells_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.spells_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.spells_editor_state.show(
                    ui,
                    &mut self.campaign_data.spells,
                    &self.campaign_data.conditions,
                    &mut spells_ctx,
                );
            }
            EditorTab::Conditions => {
                let mut conditions_ctx = EditorContext::new(
                    self.campaign_dir.as_ref(),
                    &self.campaign.conditions_file,
                    &mut self.unsaved_changes,
                    &mut self.ui_state.status_message,
                    &mut self.ui_state.file_load_merge_mode,
                );
                self.editor_registry.conditions_editor_state.show(
                    ui,
                    &mut self.campaign_data.conditions,
                    &mut self.campaign_data.spells,
                    &mut conditions_ctx,
                );
                // Handle navigation request from conditions editor
                if let Some(spell_name) = self.editor_registry.conditions_editor_state.navigate_to_spell.take() {
                    // Find the spell index by name and select it in spells editor
                    if let Some(idx) = self.campaign_data.spells.iter().position(|s| s.name == spell_name) {
                        self.editor_registry.spells_editor_state.selected_spell = Some(idx);
                        self.editor_registry.spells_editor_state.mode = spells_editor::SpellsEditorMode::Edit;
                        self.editor_registry.spells_editor_state.edit_buffer = self.campaign_data.spells[idx].clone();
                        self.ui_state.active_tab = EditorTab::Spells;
                        self.ui_state.status_message = format!("Jumped to spell: {}", spell_name);
                    } else {
                        self.ui_state.status_message = format!("Spell '{}' not found", spell_name);
                    }
                }
            }
            EditorTab::Monsters => {
                let monster_creature_manager = self
                    .campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()));
                let mut monsters_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.monsters_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.monsters_editor_state.show(
                    ui,
                    &mut self.campaign_data.monsters,
                    monster_creature_manager.as_ref(),
                    &mut monsters_ctx,
                );
            }
            EditorTab::Creatures => {
                if let Some(msg) = self.editor_registry.creatures_editor_state.show(
                    ui,
                    &mut self.campaign_data.creatures,
                    &self.campaign_dir,
                    &self.campaign.creatures_file,
                    &mut self.unsaved_changes,
                ) {
                    if msg == creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL {
                        self.ui_state.show_creature_template_browser = true;
                    } else if msg == creatures_editor::RELOAD_CREATURES_SENTINEL {
                        // User pressed Reload in the Creatures toolbar.  The editor
                        // cannot perform the two-step registry → per-file load itself,
                        // so it returns a sentinel and we call load_creatures() here.
                        self.load_creatures();
                    } else {
                        self.ui_state.status_message = msg;
                    }
                }
            }
            EditorTab::Furniture => {
                let available_mesh_ids: Vec<u32> = if self.obj_importer_state.furniture_id >= 10001
                {
                    (10001..=self.obj_importer_state.furniture_id).collect()
                } else {
                    vec![10001]
                };

                let mut furniture_ctx = EditorContext::new(
                    self.campaign_dir.as_ref(),
                    &self.campaign.furniture_file,
                    &mut self.unsaved_changes,
                    &mut self.ui_state.status_message,
                    &mut self.ui_state.file_load_merge_mode,
                );
                self.editor_registry.furniture_editor_state.show(
                    ui,
                    &mut self.campaign_data.furniture_definitions,
                    &mut furniture_ctx,
                    &available_mesh_ids,
                );
                if let Some(furniture_editor::FurnitureEditorSignal::OpenInObjImporter) =
                    self.editor_registry.furniture_editor_state.requested_signal.take()
                {
                    self.ui_state.active_tab = EditorTab::Importer;
                    self.obj_importer_state.export_type = obj_importer::ExportType::Furniture;
                    self.ui_state.status_message =
                        "Opening OBJ Importer for furniture mesh work".to_string();
                    ui.ctx().request_repaint();
                }
            }
            EditorTab::Importer => {
                if let Some(signal) = obj_importer_ui::show_obj_importer_tab(
                    ui,
                    &mut self.obj_importer_state,
                    self.campaign_dir.as_ref(),
                    &mut self.logger,
                ) {
                    self.ui_state.status_message = self.obj_importer_state.status_message.clone();
                    match signal {
                        obj_importer_ui::ObjImporterUiSignal::Creature => {
                            self.load_creatures();
                            self.sync_obj_importer_campaign_state();
                            self.ui_state.active_tab = EditorTab::Creatures;
                            ui.ctx().request_repaint();
                        }
                        obj_importer_ui::ObjImporterUiSignal::Item => {
                            ui.ctx().request_repaint();
                        }
                        obj_importer_ui::ObjImporterUiSignal::Furniture => {
                            self.ui_state.status_message = self.obj_importer_state.status_message.clone();
                            self.ui_state.active_tab = EditorTab::Furniture;
                            ui.ctx().request_repaint();
                        }
                    }
                } else {
                    self.ui_state.status_message = self.obj_importer_state.status_message.clone();
                }
            }
            EditorTab::Maps => {
                let map_refs = map_editor::MapEditorRefs {
                    monsters: &self.campaign_data.monsters,
                    items: &self.campaign_data.items,
                    conditions: &self.campaign_data.conditions,
                    npcs: &self.editor_registry.npc_editor_state.npcs,
                    furniture_definitions: &self.campaign_data.furniture_definitions,
                    display_config: &self.tool_config.display,
                };
                let mut maps_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.maps_dir,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.maps_editor_state.show(
                    ui,
                    &mut self.campaign_data.maps,
                    &map_refs,
                    &mut maps_ctx,
                );

                // If the Maps editor requested to open the NPC editor for a placed NPC,
                // honor that request by switching to the NPCs tab and starting edit mode
                // on the requested NPC (if it exists in the loaded NPC list).
                if let Some(requested_id) = self.editor_registry.maps_editor_state.requested_open_npc.take() {
                    if let Some(idx) = self
                        .editor_registry.npc_editor_state
                        .npcs
                        .iter()
                        .position(|n| n.id == requested_id)
                    {
                        // Switch to NPC editor tab and start editing the selected NPC
                        self.ui_state.active_tab = EditorTab::NPCs;
                        self.editor_registry.npc_editor_state.start_edit_npc(idx);
                        self.ui_state.status_message = format!("Opening NPC editor for '{}'", requested_id);
                    } else {
                        // NPC not found in loaded NPC definitions
                        self.ui_state.status_message = format!("NPC '{}' not found", requested_id);
                    }
                }
            }
            EditorTab::Quests => self.show_quests_editor(ui),
            EditorTab::Classes => {
                let mut classes_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.classes_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.classes_editor_state.show(
                    ui,
                    &self.campaign_data.items,
                    &mut classes_ctx,
                );
            }
            EditorTab::Races => {
                let mut races_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.races_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.races_editor_state.show(
                    ui,
                    &self.campaign_data.items,
                    &mut races_ctx,
                );
            }
            EditorTab::Characters => {
                let char_creature_manager = self
                    .campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()));
                let mut chars_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.characters_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.characters_editor_state.show(
                    ui,
                    &self.editor_registry.races_editor_state.races,
                    &self.editor_registry.classes_editor_state.classes,
                    &self.campaign_data.items,
                    char_creature_manager.as_ref(),
                    &mut chars_ctx,
                )
            }
            EditorTab::Dialogues => {
                let mut dialogues_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.dialogue_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.dialogue_editor_state.show(
                    ui,
                    &mut self.campaign_data.dialogues,
                    &self.campaign_data.quests,
                    &self.campaign_data.items,
                    &self.campaign_data.spells,
                    &mut dialogues_ctx,
                );
            }
            EditorTab::NPCs => {
                // Always sync the stock_templates mirror from the editor state before
                // threading into the NPC editor.  The StockTemplatesEditorState::show()
                // auto-load fires the first time the StockTemplates tab is rendered, but
                // the NPC tab may be opened first — in that case the explicit
                // load_stock_templates() call in do_open_campaign() has already populated
                // stock_templates_editor_state.templates, so pulling from there guarantees
                // the NPC editor's ComboBox and validation both see the live list.
                self.campaign_data.stock_templates = self.editor_registry.stock_templates_editor_state.templates.clone();

                // Thread available stock templates into the NPC editor before rendering
                self.editor_registry.npc_editor_state.available_stock_templates = self.campaign_data.stock_templates.clone();

                let npc_creature_manager = self
                    .campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()));

                let npc_ctx = npc_editor::NpcEditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    npcs_file: &self.campaign.npcs_file,
                    display_config: &self.tool_config.display,
                    creature_manager: npc_creature_manager.as_ref(),
                };
                if self.editor_registry.npc_editor_state.show(
                    ui,
                    &self.campaign_data.dialogues,
                    &self.campaign_data.quests,
                    &npc_ctx,
                ) {
                    self.unsaved_changes = true;
                }

                let npc_dialogue_ids_before: Vec<_> =
                    self.campaign_data.dialogues.iter().map(|dialogue| dialogue.id).collect();
                let npc_available_dialogue_ids_after: Vec<_> = self
                    .editor_registry.npc_editor_state
                    .available_dialogues
                    .iter()
                    .map(|dialogue| dialogue.id)
                    .collect();

                // Forward any status message produced inside show() (e.g. Reload result)
                // to the app's global status bar.  The NPC editor returns bool rather than
                // a status string, so it uses pending_status as a side-channel.
                if let Some(status) = self.editor_registry.npc_editor_state.pending_status.take() {
                    self.ui_state.status_message = status;
                }

                if npc_available_dialogue_ids_after != npc_dialogue_ids_before
                    || self.campaign_data.dialogues.len() != self.editor_registry.npc_editor_state.available_dialogues.len()
                {
                    self.campaign_data.dialogues = self.editor_registry.npc_editor_state.available_dialogues.clone();

                    if let Some(dir) = &self.campaign_dir {
                        let dialogue_path = dir.join(&self.campaign.dialogue_file);
                        match self.save_dialogues_to_file(&dialogue_path) {
                            Ok(()) => {
                                self.unsaved_changes = true;
                                self.ui_state.status_message = format!(
                                    "Saved {} dialogues after merchant dialogue update",
                                    self.campaign_data.dialogues.len()
                                );
                            }
                            Err(error) => {
                                self.ui_state.status_message = format!(
                                    "Merchant dialogue updated in memory, but failed to persist dialogues: {}",
                                    error
                                );
                            }
                        }
                    } else {
                        self.unsaved_changes = true;
                    }
                }

                if let Some(dialogue_id) = self.editor_registry.npc_editor_state.requested_open_dialogue.take() {
                    if let Some(dialogue_idx) =
                        self.campaign_data.dialogues.iter().position(|dialogue| dialogue.id == dialogue_id)
                    {
                        self.ui_state.active_tab = EditorTab::Dialogues;
                        self.editor_registry.dialogue_editor_state.selected_dialogue = Some(dialogue_idx);
                        self.editor_registry.dialogue_editor_state.start_edit_dialogue(dialogue_idx);
                        self.ui_state.status_message =
                            format!("Opening assigned dialogue {} from NPC editor", dialogue_id);
                        ui.ctx().request_repaint();
                    } else {
                        self.ui_state.status_message = format!(
                            "Assigned dialogue {} could not be opened because it was not found",
                            dialogue_id
                        );
                    }
                }

                // If the NPC editor requested cross-tab navigation to edit a stock template,
                // switch to the StockTemplates tab and open the named template for editing.
                if let Some(tmpl_id) = self.editor_registry.npc_editor_state.requested_template_edit.take() {
                    self.ui_state.active_tab = EditorTab::StockTemplates;
                    self.editor_registry.stock_templates_editor_state
                        .open_template_for_edit(&tmpl_id);
                    ui.ctx().request_repaint();
                }
            }
            EditorTab::StockTemplates => {
                let needs_save = self.editor_registry.stock_templates_editor_state.show(
                    ui,
                    &self.campaign_data.items,
                    self.campaign_dir.as_ref(),
                    &self.campaign.stock_templates_file,
                );
                if needs_save {
                    self.campaign_data.stock_templates = self.editor_registry.stock_templates_editor_state.templates.clone();
                    self.unsaved_changes = true;
                }
            }
            EditorTab::Proficiencies => {
                let mut profs_ctx = EditorContext {
                    campaign_dir: self.campaign_dir.as_ref(),
                    data_file: &self.campaign.proficiencies_file,
                    unsaved_changes: &mut self.unsaved_changes,
                    status_message: &mut self.ui_state.status_message,
                    file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
                };
                self.editor_registry.proficiencies_editor_state.show(
                    ui,
                    &mut self.campaign_data.proficiencies,
                    &self.editor_registry.classes_editor_state.classes,
                    &self.editor_registry.races_editor_state.races,
                    &self.campaign_data.items,
                    &mut profs_ctx,
                );
            }
            EditorTab::Assets => self.show_assets_editor(ui),
            EditorTab::Validation => self.show_validation_panel(ui),
        });

        // Preferences dialog using a local temporary variable to avoid borrow conflicts
        // Use local flags and avoid mutably borrowing `self.ui_state.show_preferences` inside the `show` closure.
        let mut show_preferences_local = self.ui_state.show_preferences;
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
                    self.ui_state.status_message = "Preferences saved".to_string();
                }
                Err(e) => {
                    self.ui_state.status_message = format!("Failed to save preferences: {}", e);
                }
            }
        }

        if prefs_close_clicked {
            show_preferences_local = false;
        }

        // Finally, update the app state.
        self.ui_state.show_preferences = show_preferences_local;
        // About dialog
        if self.ui_state.show_about_dialog {
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
                            self.ui_state.show_about_dialog = false;
                        }
                    });
                });
        }

        // Unsaved changes warning
        if self.ui_state.show_unsaved_warning {
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
                                if let Err(e) = self.save_campaign() {
                                    self.ui_state.status_message =
                                        format!("Failed to save campaign: {}", e);
                                    self.logger.error(
                                        category::CAMPAIGN,
                                        &format!("Failed to save campaign: {}", e),
                                    );
                                }
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

                            self.ui_state.show_unsaved_warning = false;
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

                            self.ui_state.show_unsaved_warning = false;
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.ui_state.show_unsaved_warning = false;
                            self.pending_action = None;
                        }
                    });
                });
        }

        // Template browser dialog
        if self.ui_state.show_template_browser {
            self.show_template_browser_dialog(ctx);
        }

        // Creature Template Browser dialog
        if self.ui_state.show_creature_template_browser {
            self.show_creature_template_browser_dialog(ctx);
        }

        // Validation report dialog
        if self.validation_state.show_validation_report {
            self.show_validation_report_dialog(ctx);
        }

        // Balance statistics dialog
        if self.ui_state.show_balance_stats {
            self.show_balance_stats_dialog(ctx);
        }

        // Debug panel
        if self.ui_state.show_debug_panel {
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
        let mut campaign_ctx = EditorContext {
            campaign_dir: self.campaign_dir.as_ref(),
            data_file: "",
            unsaved_changes: &mut self.unsaved_changes,
            status_message: &mut self.ui_state.status_message,
            file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
        };
        self.editor_registry.campaign_editor_state.show(
            ui,
            &mut self.campaign,
            &mut self.campaign_path,
            self.editor_registry.npc_editor_state.npcs.as_slice(),
            &mut campaign_ctx,
        );

        // If the campaign metadata editor requested validation, run the shared
        // validator and switch to the Validation tab so results are visible.
        if self
            .editor_registry
            .campaign_editor_state
            .consume_validate_request()
        {
            self.validate_campaign();
            self.ui_state.active_tab = EditorTab::Validation;
        }
    }

    /// Show quests editor
    fn show_quests_editor(&mut self, ui: &mut egui::Ui) {
        let mut quests_ctx = EditorContext {
            campaign_dir: self.campaign_dir.as_ref(),
            data_file: &self.campaign.quests_file,
            unsaved_changes: &mut self.unsaved_changes,
            status_message: &mut self.ui_state.status_message,
            file_load_merge_mode: &mut self.ui_state.file_load_merge_mode,
        };
        self.editor_registry.quest_editor_state.show(
            ui,
            &mut self.campaign_data.quests,
            &self.campaign_data.items,
            &self.campaign_data.monsters,
            &self.campaign_data.maps,
            &self.campaign_data.spells,
            &mut quests_ctx,
        );
    }

    /// Show validation results panel
    ///
    /// Displays validation results in a table-based layout grouped by category.
    /// Uses icons to indicate severity: ✅ passed, ❌ error, ⚠️ warning, ℹ️ info.
    fn show_validation_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("✅ Campaign Validation");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🛠 Repair Merchant Dialogue").clicked() {
                    let repair_result = self.repair_merchant_dialogue_validation_issues();
                    self.ui_state.status_message = repair_result.message.clone();
                    self.validation_state.validation_errors.push(repair_result);
                    self.validate_campaign();
                }
                if ui.button("🔄 Re-validate").clicked() {
                    self.validate_campaign();
                }
            });
        });
        ui.add_space(5.0);
        ui.label("Check your campaign for errors and warnings");
        ui.separator();

        // Calculate summary using the validation module
        let summary =
            validation::ValidationSummary::from_results(&self.validation_state.validation_errors);

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
            ui.label(format!(
                "Total: {} checks",
                self.validation_state.validation_errors.len()
            ));
        });

        ui.separator();

        // Quick filter controls
        ui.horizontal(|ui| {
            ui.label("Show:");
            if ui
                .selectable_label(
                    self.validation_state.validation_filter == ValidationFilter::All,
                    "All",
                )
                .clicked()
            {
                self.validation_state.validation_filter = ValidationFilter::All;
            }
            if summary.error_count > 0
                && ui
                    .selectable_label(
                        self.validation_state.validation_filter == ValidationFilter::ErrorsOnly,
                        "Errors Only",
                    )
                    .clicked()
            {
                self.validation_state.validation_filter = ValidationFilter::ErrorsOnly;
            }
            if summary.warning_count > 0
                && ui
                    .selectable_label(
                        self.validation_state.validation_filter == ValidationFilter::WarningsOnly,
                        "Warnings Only",
                    )
                    .clicked()
            {
                self.validation_state.validation_filter = ValidationFilter::WarningsOnly;
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
                                let clicked_jump =
                                    if result.category == validation::ValidationCategory::NPCs {
                                        let maybe_npc = self
                                            .editor_registry
                                            .npc_editor_state
                                            .npcs
                                            .iter()
                                            .find(|npc| {
                                                result.message.contains(&format!("'{}'", npc.id))
                                            })
                                            .map(|npc| npc.id.clone());

                                        if let Some(npc_id) = maybe_npc {
                                            if ui.link(&result.message).clicked() {
                                                self.editor_registry
                                                    .npc_editor_state
                                                    .requested_open_npc = Some(npc_id);
                                                true
                                            } else {
                                                false
                                            }
                                        } else {
                                            ui.label(&result.message);
                                            false
                                        }
                                    } else {
                                        ui.label(&result.message);
                                        false
                                    };

                                if clicked_jump {
                                    self.ui_state.status_message =
                                        "Opening NPC editor from validation result".to_string();
                                }

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

    /// Focus (and open) the asset manager to the given asset path.
    ///
    /// When a user clicks a file path in the validation panel, we set this value,
    /// open the Asset Manager, and surface a useful status message. The asset
    /// editor UI will highlight the focused asset if present.
    pub fn reset_validation_filters(&mut self) {
        // Restore default validation filter state and clear any asset focus
        self.validation_state.validation_filter = ValidationFilter::All;
        self.validation_state.validation_focus_asset = None;
        self.ui_state.status_message = "Validation filters reset".to_string();
    }

    pub fn focus_asset(&mut self, path: PathBuf) {
        self.validation_state.validation_focus_asset = Some(path.clone());
        self.ui_state.show_asset_manager = true;
        self.ui_state.status_message = format!("🔎 Focused asset: {}", path.display());
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
                    self.ui_state.status_message = format!("Failed to scan assets: {}", e);
                } else {
                    // Initialize data file tracking
                    // Discover actual map files from the maps directory
                    let map_file_paths = self.discover_map_files();

                    let data_files_cfg = asset_manager::DataFilesConfig {
                        items_file: &self.campaign.items_file,
                        spells_file: &self.campaign.spells_file,
                        conditions_file: &self.campaign.conditions_file,
                        monsters_file: &self.campaign.monsters_file,
                        quests_file: &self.campaign.quests_file,
                        classes_file: &self.campaign.classes_file,
                        races_file: &self.campaign.races_file,
                        characters_file: &self.campaign.characters_file,
                        dialogue_file: &self.campaign.dialogue_file,
                        npcs_file: &self.campaign.npcs_file,
                        proficiencies_file: &self.campaign.proficiencies_file,
                    };
                    manager.init_data_files(&data_files_cfg, &map_file_paths);
                    // Scan references on initial load so portraits are properly marked as referenced
                    let campaign_refs = asset_manager::CampaignRefs {
                        items: &self.campaign_data.items,
                        quests: &self.campaign_data.quests,
                        dialogues: &self.campaign_data.dialogues,
                        maps: &self.campaign_data.maps,
                        classes: &self.editor_registry.classes_editor_state.classes,
                        characters: &self.editor_registry.characters_editor_state.characters,
                        npcs: &self.editor_registry.npc_editor_state.npcs,
                    };
                    manager.scan_references(&campaign_refs);
                    manager.mark_data_files_as_referenced();

                    self.ui_state.status_message =
                        format!("Scanned {} assets", manager.assets().len());
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
                        self.ui_state.status_message = format!("Failed to refresh assets: {}", e);
                    } else {
                        // After refreshing assets, rescan references to properly mark portraits
                        // referenced by characters and NPCs
                        let campaign_refs = asset_manager::CampaignRefs {
                            items: &self.campaign_data.items,
                            quests: &self.campaign_data.quests,
                            dialogues: &self.campaign_data.dialogues,
                            maps: &self.campaign_data.maps,
                            classes: &self.editor_registry.classes_editor_state.classes,
                            characters: &self.editor_registry.characters_editor_state.characters,
                            npcs: &self.editor_registry.npc_editor_state.npcs,
                        };
                        manager.scan_references(&campaign_refs);
                        manager.mark_data_files_as_referenced();
                        self.ui_state.status_message =
                            "Assets refreshed and references scanned".to_string();
                    }
                }

                if ui.button("🔍 Scan References").clicked() {
                    // Scan references across all campaign data
                    let campaign_refs = asset_manager::CampaignRefs {
                        items: &self.campaign_data.items,
                        quests: &self.campaign_data.quests,
                        dialogues: &self.campaign_data.dialogues,
                        maps: &self.campaign_data.maps,
                        classes: &self.editor_registry.classes_editor_state.classes,
                        characters: &self.editor_registry.characters_editor_state.characters,
                        npcs: &self.editor_registry.npc_editor_state.npcs,
                    };
                    manager.scan_references(&campaign_refs);
                    // Mark successfully loaded data files as referenced
                    manager.mark_data_files_as_referenced();
                    self.ui_state.status_message = "Asset references scanned".to_string();
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
                        self.ui_state.asset_type_filter.is_none(),
                        format!("All ({})", manager.assets().len()),
                    )
                    .clicked()
                {
                    self.ui_state.asset_type_filter = None;
                    self.ui_state.status_message = "Showing all asset types".to_string();
                }

                // Individual type filters
                for asset_type in asset_manager::AssetType::all() {
                    let count = manager.asset_count_by_type(asset_type);
                    if count > 0 {
                        let is_selected = self.ui_state.asset_type_filter == Some(asset_type);
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("{} ({})", asset_type.display_name(), count),
                            )
                            .clicked()
                        {
                            self.ui_state.asset_type_filter = Some(asset_type);
                            self.ui_state.status_message =
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
                                self.ui_state.show_cleanup_candidates = !self.ui_state.show_cleanup_candidates;
                            }
                            ui.label(egui::RichText::new("(Safe cleanup preview)").small().weak());
                        });

                        // Show the cleanup candidates list if toggled
                        if self.ui_state.show_cleanup_candidates {
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
                                    self.ui_state.cleanup_candidates_selected.clear();
                                    for path in &candidates {
                                        self.ui_state.cleanup_candidates_selected.insert(path.clone());
                                    }
                                }

                                if ui.button("Deselect All").clicked() {
                                    self.ui_state.cleanup_candidates_selected.clear();
                                }

                                ui.separator();

                                let selected_count = self.ui_state.cleanup_candidates_selected.len();
                                if selected_count > 0 {
                                    if ui.button(format!("🗑️ Delete {} Selected", selected_count))
                                        .clicked()
                                    {
                                        // Calculate total size of selected files
                                        let mut total_size = 0u64;
                                        for path in &self.ui_state.cleanup_candidates_selected {
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

                                        // Perform deletion
                                        let mut deleted_count = 0;
                                        let mut failed_deletions = Vec::new();

                                        for path in self.ui_state.cleanup_candidates_selected.iter() {
                                            match manager.remove_asset(path) {
                                                Ok(_) => deleted_count += 1,
                                                Err(e) => failed_deletions.push(format!("{}: {}", path.display(), e)),
                                            }
                                        }

                                        // Update status message
                                        if failed_deletions.is_empty() {
                                            self.ui_state.status_message = format!(
                                                "✅ Successfully deleted {} files ({})",
                                                deleted_count,
                                                size_str
                                            );
                                        } else {
                                            self.ui_state.status_message = format!(
                                                "⚠️ Deleted {} files, {} failed: {}",
                                                deleted_count,
                                                failed_deletions.len(),
                                                failed_deletions.join(", ")
                                            );
                                        }

                                        // Clear selection after deletion
                                        self.ui_state.cleanup_candidates_selected.clear();
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
                                        let is_selected = self.ui_state.cleanup_candidates_selected.contains(candidate_path);

                                        ui.horizontal(|ui| {
                                            let mut selected = is_selected;
                                            if ui.checkbox(&mut selected, "").changed() {
                                                if selected {
                                                    self.ui_state.cleanup_candidates_selected.insert(candidate_path.clone());
                                                } else {
                                                    self.ui_state.cleanup_candidates_selected.remove(candidate_path);
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
                        match self.ui_state.asset_type_filter {
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
                                if let Some(ref focus) =
                                    self.validation_state.validation_focus_asset
                                {
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

                                        // Hidden-file badge — shown first so it stands out
                                        if asset.is_hidden {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(220, 60, 60),
                                                "🙈 Hidden File",
                                            )
                                            .on_hover_text(
                                                "This is a hidden file (filename starts with '.'). \
                                                Hidden files such as .DS_Store and .gitkeep are \
                                                not part of the campaign and should be removed.",
                                            );
                                        }

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

/// Editor mode for data editing panels
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    List,
    Add,
    Edit,
}

/// Item type filter for search
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTypeFilter {
    Weapon,
    Armor,
}

impl ItemTypeFilter {
    pub fn matches(&self, item: &Item) -> bool {
        match self {
            ItemTypeFilter::Weapon => item.is_weapon(),
            ItemTypeFilter::Armor => item.is_armor(),
        }
    }
}

impl CampaignBuilderApp {
    /// Create a default item for the edit buffer
    pub fn default_item() -> Item {
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
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Create a default spell for the edit buffer
    pub fn default_spell() -> Spell {
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
    pub fn default_monster() -> MonsterDefinition {
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
                is_ranged: false,
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
            creature_id: None,
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }
    }

    /// Get next available item ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    pub fn next_available_item_id(&self) -> ItemId {
        self.campaign_data
            .items
            .iter()
            .map(|i| i.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available spell ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    pub fn next_available_spell_id(&self) -> SpellId {
        self.campaign_data
            .spells
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available monster ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    pub fn next_available_monster_id(&self) -> MonsterId {
        self.campaign_data
            .monsters
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Get next available map ID
    ///
    /// Returns the next unique ID by finding the maximum existing ID and adding 1.
    pub fn next_available_map_id(&self) -> MapId {
        self.campaign_data
            .maps
            .iter()
            .map(|m| m.id)
            .max()
            .map(|id| id + 1)
            .unwrap_or(1)
    }

    /// Get next available quest ID
    pub fn next_available_quest_id(&self) -> QuestId {
        self.campaign_data
            .quests
            .iter()
            .map(|q| q.id)
            .max()
            .unwrap_or(0)
            + 1
    }
}
