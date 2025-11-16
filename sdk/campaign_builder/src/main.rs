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

use eframe::egui;
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
    Config,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
    Files,
    Validation,
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
            status_message: "Ready. Create a new campaign or open an existing one.".to_string(),
            unsaved_changes: false,
            validation_errors: Vec::new(),
            show_about_dialog: false,
            show_unsaved_warning: false,
            pending_action: None,
            file_tree: Vec::new(),
        }
    }
}

impl CampaignBuilderApp {
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

    /// Show items editor (placeholder with list view)
    fn show_items_editor(&self, ui: &mut egui::Ui) {
        ui.heading("⚔️ Items Editor");
        ui.add_space(5.0);
        ui.label("Manage weapons, armor, and consumable items");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut String::new());
            ui.separator();
            if ui.button("➕ Add Item").clicked() {
                // Will be implemented in Phase 3
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("📋 Item List (Placeholder)");
                ui.separator();
                ui.label("No items loaded. Items will be loaded from:");
                ui.monospace(&self.campaign.items_file);
                ui.add_space(10.0);
                ui.label("Phase 3 will add:");
                ui.label("  • Load items from .ron files");
                ui.label("  • Add/Edit/Delete operations");
                ui.label("  • Item type filtering (Weapon/Armor/Consumable)");
                ui.label("  • Class restriction editor");
                ui.label("  • Real-time validation");
            });
        });
    }

    /// Show spells editor (placeholder with list view)
    fn show_spells_editor(&self, ui: &mut egui::Ui) {
        ui.heading("✨ Spells Editor");
        ui.add_space(5.0);
        ui.label("Manage cleric and sorcerer spells");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut String::new());
            ui.separator();
            if ui.button("➕ Add Spell").clicked() {
                // Will be implemented in Phase 3
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("📋 Spell List (Placeholder)");
                ui.separator();
                ui.label("No spells loaded. Spells will be loaded from:");
                ui.monospace(&self.campaign.spells_file);
                ui.add_space(10.0);
                ui.label("Phase 3 will add:");
                ui.label("  • Load spells from .ron files");
                ui.label("  • Add/Edit/Delete operations");
                ui.label("  • School filtering (Cleric/Sorcerer)");
                ui.label("  • Level-based organization");
                ui.label("  • SP and gem cost editor");
                ui.label("  • Target and context editor");
            });
        });
    }

    /// Show monsters editor (placeholder with list view)
    fn show_monsters_editor(&self, ui: &mut egui::Ui) {
        ui.heading("👹 Monsters Editor");
        ui.add_space(5.0);
        ui.label("Manage enemy creatures and encounters");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut String::new());
            ui.separator();
            if ui.button("➕ Add Monster").clicked() {
                // Will be implemented in Phase 3
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("📋 Monster List (Placeholder)");
                ui.separator();
                ui.label("No monsters loaded. Monsters will be loaded from:");
                ui.monospace(&self.campaign.monsters_file);
                ui.add_space(10.0);
                ui.label("Phase 3 will add:");
                ui.label("  • Load monsters from .ron files");
                ui.label("  • Add/Edit/Delete operations");
                ui.label("  • Stats and abilities editor");
                ui.label("  • Loot table editor");
                ui.label("  • Special attack configuration");
                ui.label("  • Group encounter builder");
            });
        });
    }

    /// Show maps editor (placeholder with list view)
    fn show_maps_editor(&self, ui: &mut egui::Ui) {
        ui.heading("🗺️ Maps Editor");
        ui.add_space(5.0);
        ui.label("Manage world maps and dungeons");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut String::new());
            ui.separator();
            if ui.button("➕ Add Map").clicked() {
                // Will be implemented in Phase 4
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("📋 Map List (Placeholder)");
                ui.separator();
                ui.label("No maps loaded. Maps will be loaded from:");
                ui.monospace(&self.campaign.maps_dir);
                ui.add_space(10.0);
                ui.label("Phase 4 will integrate:");
                ui.label("  • Launch map_builder tool");
                ui.label("  • Load existing maps");
                ui.label("  • Map preview");
                ui.label("  • Event editor integration");
                ui.label("  • Interconnection management");
            });
        });
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
}
