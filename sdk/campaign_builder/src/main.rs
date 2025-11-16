// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder - Prototype UI for Antares SDK
//!
//! This prototype validates egui as the UI framework for the Antares Campaign Builder.
//! It demonstrates key UI patterns needed for the full SDK:
//! - Menu bar with file operations
//! - Tabbed interface for different editors
//! - Form inputs for metadata editing
//! - File browser integration
//! - Real-time validation feedback
//! - Status bar with messages

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    // Logging can be enabled in the future with env_logger

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Antares Campaign Builder - Prototype"),

        // Let eframe choose best available backend (works without GPU!)
        renderer: eframe::Renderer::default(),

        ..Default::default()
    };

    eframe::run_native(
        "Antares Campaign Builder",
        options,
        Box::new(|_cc| {
            // Use default fonts and style
            Ok(Box::<CampaignBuilderApp>::default())
        }),
    )
}

/// Campaign metadata structure (simplified for prototype)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignMetadata {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    engine_version: String,
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
    Validation,
}

impl EditorTab {
    fn name(&self) -> &str {
        match self {
            EditorTab::Metadata => "📋 Metadata",
            EditorTab::Items => "⚔️ Items",
            EditorTab::Spells => "✨ Spells",
            EditorTab::Monsters => "👹 Monsters",
            EditorTab::Maps => "🗺️ Maps",
            EditorTab::Quests => "📜 Quests",
            EditorTab::Validation => "✅ Validation",
        }
    }
}

/// Main application state
struct CampaignBuilderApp {
    campaign: CampaignMetadata,
    active_tab: EditorTab,
    campaign_path: Option<PathBuf>,
    status_message: String,
    unsaved_changes: bool,
    validation_errors: Vec<String>,
    show_about_dialog: bool,
}

impl Default for CampaignBuilderApp {
    fn default() -> Self {
        Self {
            campaign: CampaignMetadata::default(),
            active_tab: EditorTab::Metadata,
            campaign_path: None,
            status_message: "Ready. Create a new campaign or open an existing one.".to_string(),
            unsaved_changes: false,
            validation_errors: Vec::new(),
            show_about_dialog: false,
        }
    }
}

impl CampaignBuilderApp {
    fn validate_campaign(&mut self) {
        self.validation_errors.clear();

        if self.campaign.id.is_empty() {
            self.validation_errors
                .push("Campaign ID is required".to_string());
        }

        if self.campaign.name.is_empty() {
            self.validation_errors
                .push("Campaign name is required".to_string());
        }

        if self.campaign.author.is_empty() {
            self.validation_errors
                .push("Author name is required".to_string());
        }

        if !self.campaign.version.contains('.') {
            self.validation_errors
                .push("Version should follow semantic versioning (e.g., 1.0.0)".to_string());
        }

        if self.validation_errors.is_empty() {
            self.status_message = "✅ Validation passed!".to_string();
        } else {
            self.status_message = format!(
                "❌ {} validation error(s) found",
                self.validation_errors.len()
            );
        }
    }

    fn new_campaign(&mut self) {
        self.campaign = CampaignMetadata::default();
        self.campaign_path = None;
        self.unsaved_changes = false;
        self.validation_errors.clear();
        self.status_message = "New campaign created.".to_string();
    }

    fn save_campaign(&mut self) {
        // In real implementation, this would save to campaign.ron
        if let Some(path) = &self.campaign_path {
            self.status_message = format!("Campaign saved to: {}", path.display());
            self.unsaved_changes = false;
        } else {
            self.status_message = "No file path set. Use 'Save As' first.".to_string();
        }
    }

    fn open_campaign(&mut self) {
        // In real implementation, this would use file dialog and load campaign.ron
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .pick_file()
        {
            self.campaign_path = Some(path.clone());
            self.status_message = format!("Opened campaign from: {}", path.display());
            // Would load actual data here
            self.unsaved_changes = false;
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
                        self.save_campaign();
                        ui.close_menu();
                    }
                    if ui.button("💾 Save As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RON Files", &["ron"])
                            .save_file()
                        {
                            self.campaign_path = Some(path);
                            self.save_campaign();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🚪 Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("✅ Validate Campaign").clicked() {
                        self.validate_campaign();
                        self.active_tab = EditorTab::Validation;
                        ui.close_menu();
                    }
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
                        ui.label("● Unsaved changes");
                    } else {
                        ui.label("✓ Saved");
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
                    EditorTab::Items,
                    EditorTab::Spells,
                    EditorTab::Monsters,
                    EditorTab::Maps,
                    EditorTab::Quests,
                    EditorTab::Validation,
                ];

                for tab in &tabs {
                    let is_selected = self.active_tab == *tab;
                    if ui.selectable_label(is_selected, tab.name()).clicked() {
                        self.active_tab = *tab;
                    }
                }

                ui.separator();
                ui.label("Prototype UI");
                ui.label("Powered by egui");
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(&self.status_message);
            });
        });

        // Central panel with editor content
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            EditorTab::Metadata => self.show_metadata_editor(ui),
            EditorTab::Items => self.show_placeholder_editor(ui, "Items"),
            EditorTab::Spells => self.show_placeholder_editor(ui, "Spells"),
            EditorTab::Monsters => self.show_placeholder_editor(ui, "Monsters"),
            EditorTab::Maps => self.show_placeholder_editor(ui, "Maps"),
            EditorTab::Quests => self.show_placeholder_editor(ui, "Quests"),
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
                        ui.label("Prototype v0.1.0");
                        ui.separator();
                        ui.label("A visual editor for creating custom");
                        ui.label("campaigns for the Antares RPG engine.");
                        ui.separator();
                        ui.label("Built with egui - works without GPU!");
                        ui.separator();
                        if ui.button("Close").clicked() {
                            self.show_about_dialog = false;
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
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("metadata_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
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

                    // Description (multiline)
                    ui.label("Description:");
                    if ui
                        .text_edit_multiline(&mut self.campaign.description)
                        .changed()
                    {
                        self.unsaved_changes = true;
                    }
                    ui.end_row();
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("💾 Save Campaign").clicked() {
                    self.save_campaign();
                }

                if ui.button("✅ Validate").clicked() {
                    self.validate_campaign();
                    self.active_tab = EditorTab::Validation;
                }
            });

            ui.separator();

            ui.group(|ui| {
                ui.heading("Preview");
                ui.separator();
                ui.label(format!("ID: {}", self.campaign.id));
                ui.label(format!("Name: {}", self.campaign.name));
                ui.label(format!("Version: {}", self.campaign.version));
                ui.label(format!("Author: {}", self.campaign.author));
                ui.label(format!("Engine: {}", self.campaign.engine_version));
                ui.label(format!("Description: {}", self.campaign.description));
            });
        });
    }

    /// Show placeholder for other editors (to be implemented)
    fn show_placeholder_editor(&self, ui: &mut egui::Ui, editor_name: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading(format!("{} Editor", editor_name));
            ui.separator();
            ui.label(format!(
                "The {} editor will be implemented in the full SDK.",
                editor_name
            ));
            ui.label("This prototype demonstrates the UI framework choice.");
            ui.add_space(20.0);
            ui.label("Expected features:");
            ui.label(format!("• List view of all {}", editor_name.to_lowercase()));
            ui.label("• Add/Edit/Delete operations");
            ui.label("• Real-time validation");
            ui.label("• Search and filtering");
            ui.label("• Import/Export capabilities");
        });
    }

    /// Show validation results panel
    fn show_validation_panel(&self, ui: &mut egui::Ui) {
        ui.heading("Campaign Validation");
        ui.separator();

        if self.validation_errors.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.heading("✅ All Checks Passed!");
                ui.label("Your campaign metadata is valid.");
            });
        } else {
            ui.label(format!(
                "Found {} validation error(s):",
                self.validation_errors.len()
            ));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for error in &self.validation_errors {
                    ui.horizontal(|ui| {
                        ui.label("❌");
                        ui.label(error);
                    });
                }
            });

            ui.separator();
            ui.label("💡 Tip: Go to the Metadata tab to fix these issues.");
        }
    }
}
