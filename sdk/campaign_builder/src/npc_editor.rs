// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC Editor for Campaign Builder
//!
//! This module provides a visual NPC editor for creating and managing
//! NPC definitions that can be placed in maps throughout the campaign.
//!
//! The `show()` method provides full UI rendering following the standard editor pattern.
//! Uses shared UI components for consistent layout.
//!
//! # Features
//!
//! - NPC definition list view with search and filtering
//! - Add/Edit/Delete functionality
//! - Fields for all `NpcDefinition` properties
//! - Autocomplete for dialogue_id (from loaded dialogues)
//! - Multi-select for quest_ids (from loaded quests)
//! - Portrait path validation
//! - Import/export RON support
//!
//! # Architecture
//!
//! Follows standard SDK editor pattern:
//! - `NpcEditorState`: Main editor state with `show()` method
//! - `NpcEditorMode`: List/Add/Edit modes
//! - `NpcEditBuffer`: Form field buffer for editing
//! - Standard UI components: EditorToolbar, TwoColumnLayout, ActionButtons

use crate::ui_helpers::{
    autocomplete_portrait_selector, extract_portrait_candidates, ActionButtons, EditorToolbar,
    ItemAction, ToolbarAction, TwoColumnLayout,
};
use antares::domain::dialogue::{DialogueId, DialogueTree};
use antares::domain::quest::{Quest, QuestId};
use antares::domain::world::npc::{NpcDefinition, NpcId};
use antares::sdk::tool_config::DisplayConfig;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Editor state for NPC editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcEditorState {
    /// All NPC definitions being edited
    pub npcs: Vec<NpcDefinition>,

    /// Currently selected NPC index
    pub selected_npc: Option<usize>,

    /// NPC editor mode
    pub mode: NpcEditorMode,

    /// Edit buffer for NPC form fields
    pub edit_buffer: NpcEditBuffer,

    /// NPC search/filter string
    pub search_filter: String,

    /// Unsaved changes flag
    pub has_unsaved_changes: bool,

    /// Validation errors for current NPC
    pub validation_errors: Vec<String>,

    /// Available dialogue trees (for dialogue_id autocomplete)
    pub available_dialogues: Vec<DialogueTree>,

    /// Available quests (for quest_ids multi-select)
    pub available_quests: Vec<Quest>,

    /// Whether to show dialogue preview in edit view
    pub show_dialogue_preview: bool,

    /// Whether to show import dialog
    pub show_import_dialog: bool,

    /// Import buffer for RON text
    pub import_buffer: String,

    /// Filter: Show only merchants
    pub filter_merchants: bool,

    /// Filter: Show only innkeepers
    pub filter_innkeepers: bool,

    /// Filter: Show only NPCs with quests
    pub filter_quest_givers: bool,

    /// Available portrait IDs (cached from directory scan)
    #[serde(skip)]
    pub available_portraits: Vec<String>,
}

/// NPC editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcEditorMode {
    /// Viewing list of NPCs
    List,
    /// Creating new NPC
    Add,
    /// Editing existing NPC
    Edit,
}

/// Buffer for NPC form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcEditBuffer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub portrait_id: String,
    pub dialogue_id: String,
    pub quest_ids: Vec<String>,
    pub faction: String,
    pub is_merchant: bool,
    pub is_innkeeper: bool,
}

impl Default for NpcEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: String::new(),
            quest_ids: Vec::new(),
            faction: String::new(),
            is_merchant: false,
            is_innkeeper: false,
        }
    }
}

impl Default for NpcEditorState {
    fn default() -> Self {
        Self {
            npcs: Vec::new(),
            selected_npc: None,
            mode: NpcEditorMode::List,
            edit_buffer: NpcEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
            available_dialogues: Vec::new(),
            available_quests: Vec::new(),
            show_dialogue_preview: false,
            show_import_dialog: false,
            import_buffer: String::new(),
            filter_merchants: false,
            filter_innkeepers: false,
            filter_quest_givers: false,
            available_portraits: Vec::new(),
        }
    }
}

impl NpcEditorState {
    /// Creates a new NPC editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows the NPC editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `dialogues` - Available dialogue trees for autocomplete
    /// * `quests` - Available quests for multi-select
    /// * `campaign_dir` - Optional campaign directory for portrait loading
    /// * `display_config` - Display configuration for layout
    ///
    /// # Returns
    ///
    /// Returns `true` if changes were made requiring save
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &[DialogueTree],
        quests: &[Quest],
        campaign_dir: Option<&PathBuf>,
        display_config: &DisplayConfig,
    ) -> bool {
        // Update portrait candidates if campaign directory changed
        self.available_portraits = extract_portrait_candidates(campaign_dir);

        // Update available references
        self.available_dialogues = dialogues.to_vec();
        self.available_quests = quests.to_vec();

        let mut needs_save = false;

        // Toolbar
        let toolbar_action = EditorToolbar::new("NPCs").show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => self.start_add_npc(),
            ToolbarAction::Import => self.show_import_dialog = true,
            ToolbarAction::Export => {
                needs_save |= self.export_all_npcs();
            }
            _ => {}
        }

        ui.separator();

        // Filters (only shown in list mode)
        if self.mode == NpcEditorMode::List {
            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                ui.text_edit_singleline(&mut self.search_filter);

                ui.separator();

                if ui
                    .selectable_label(self.filter_merchants, "🏪 Merchants")
                    .clicked()
                {
                    self.filter_merchants = !self.filter_merchants;
                }

                if ui
                    .selectable_label(self.filter_innkeepers, "🛏️ Innkeepers")
                    .clicked()
                {
                    self.filter_innkeepers = !self.filter_innkeepers;
                }

                if ui
                    .selectable_label(self.filter_quest_givers, "📜 Quest Givers")
                    .clicked()
                {
                    self.filter_quest_givers = !self.filter_quest_givers;
                }

                ui.separator();

                if ui.button("🔄 Clear Filters").clicked() {
                    self.search_filter.clear();
                    self.filter_merchants = false;
                    self.filter_innkeepers = false;
                    self.filter_quest_givers = false;
                }
            });

            ui.separator();
        }

        // Mode-specific UI
        match self.mode {
            NpcEditorMode::List => {
                needs_save |= self.show_list_view(ui, display_config);
            }
            NpcEditorMode::Add | NpcEditorMode::Edit => {
                needs_save |= self.show_edit_view(ui, campaign_dir);
            }
        }

        // Import dialog
        if self.show_import_dialog {
            self.show_import_dialog_window(ui.ctx());
        }

        needs_save
    }

    fn show_list_view(&mut self, ui: &mut egui::Ui, display_config: &DisplayConfig) -> bool {
        let mut needs_save = false;

        let search_lower = self.search_filter.to_lowercase();

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_npcs: Vec<(usize, String, NpcDefinition)> = self
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, npc)| {
                // Search filter
                if !search_lower.is_empty()
                    && !npc.name.to_lowercase().contains(&search_lower)
                    && !npc.id.to_lowercase().contains(&search_lower)
                {
                    return false;
                }

                // Merchant filter
                if self.filter_merchants && !npc.is_merchant {
                    return false;
                }

                // Innkeeper filter
                if self.filter_innkeepers && !npc.is_innkeeper {
                    return false;
                }

                // Quest giver filter
                if self.filter_quest_givers && npc.quest_ids.is_empty() {
                    return false;
                }

                true
            })
            .map(|(idx, npc)| {
                let mut label = format!("{}: {}", npc.id, npc.name);
                if npc.is_merchant {
                    label.push_str(" 🏪");
                }
                if npc.is_innkeeper {
                    label.push_str(" 🛏️");
                }
                if !npc.quest_ids.is_empty() {
                    label.push_str(" 📜");
                }
                (idx, label, npc.clone())
            })
            .collect();

        // Sort by ID
        let mut sorted_npcs = filtered_npcs;
        sorted_npcs.sort_by(|(_, _, a), (_, _, b)| a.id.cmp(&b.id));

        let selected = self.selected_npc;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        ui.separator();

        let total_width = ui.available_width();
        let inspector_min_width = display_config
            .inspector_min_width
            .max(crate::ui_helpers::DEFAULT_INSPECTOR_MIN_WIDTH);
        let sep_margin = 12.0;
        let requested_left = total_width - inspector_min_width - sep_margin;
        let left_width = crate::ui_helpers::compute_left_column_width(
            total_width,
            requested_left,
            inspector_min_width,
            sep_margin,
            crate::ui_helpers::MIN_SAFE_LEFT_COLUMN_WIDTH,
            display_config.left_column_max_ratio,
        );

        TwoColumnLayout::new("npcs")
            .with_left_width(left_width)
            .show_split(
                ui,
                |left_ui| {
                    // Left panel: NPC list
                    left_ui.heading("NPCs");
                    left_ui.separator();

                    for (idx, label, _) in &sorted_npcs {
                        let is_selected = selected == Some(*idx);
                        if left_ui.selectable_label(is_selected, label).clicked() {
                            new_selection = Some(*idx);
                        }
                    }

                    if sorted_npcs.is_empty() {
                        left_ui.label("No NPCs found");
                    }
                },
                |right_ui| {
                    // Right panel: Detail view
                    if let Some(idx) = selected {
                        if let Some((_, _, npc)) = sorted_npcs.iter().find(|(i, _, _)| *i == idx) {
                            right_ui.heading(&npc.name);
                            right_ui.separator();

                            // Use shared ActionButtons component
                            let action = ActionButtons::new().enabled(true).show(right_ui);
                            if action != ItemAction::None {
                                action_requested = Some(action);
                            }

                            right_ui.separator();
                            Self::show_preview_static(right_ui, npc);
                        } else {
                            right_ui.vertical_centered(|ui| {
                                ui.add_space(100.0);
                                ui.label("Select an NPC to view details");
                            });
                        }
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select an NPC to view details");
                        });
                    }
                },
            );

        // Apply selection change after closures
        self.selected_npc = new_selection;

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_npc {
                        if idx < self.npcs.len() {
                            self.start_edit_npc(idx);
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_npc {
                        if idx < self.npcs.len() {
                            self.npcs.remove(idx);
                            self.selected_npc = None;
                            needs_save = true;
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_npc {
                        if idx < self.npcs.len() {
                            let mut new_npc = self.npcs[idx].clone();
                            let next_id = self.next_npc_id();
                            new_npc.id = next_id;
                            new_npc.name = format!("{} (Copy)", new_npc.name);
                            self.npcs.push(new_npc);
                            needs_save = true;
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_npc {
                        if idx < self.npcs.len() {
                            if let Ok(ron_str) = ron::ser::to_string_pretty(
                                &self.npcs[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                self.import_buffer = ron_str;
                                self.show_import_dialog = true;
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }

        needs_save
    }

    /// Static preview method that doesn't require self
    fn show_preview_static(ui: &mut egui::Ui, npc: &NpcDefinition) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Info");
                    ui.label(format!("ID: {}", npc.id));
                    ui.label(format!("Name: {}", npc.name));

                    if !npc.description.is_empty() {
                        ui.separator();
                        ui.label(&npc.description);
                    }
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.heading("Appearance");
                    if !npc.portrait_id.is_empty() {
                        ui.label(format!("Portrait: {}", npc.portrait_id));
                    } else {
                        ui.label("No portrait");
                    }
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.heading("Interactions");

                    if let Some(dialogue_id) = &npc.dialogue_id {
                        ui.label(format!("💬 Dialogue: {}", dialogue_id));
                    } else {
                        ui.label("No dialogue");
                    }

                    if !npc.quest_ids.is_empty() {
                        ui.label(format!("📜 Quests: {}", npc.quest_ids.len()));
                        for quest_id in &npc.quest_ids {
                            ui.label(format!("  • {}", quest_id));
                        }
                    } else {
                        ui.label("No quests");
                    }
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.heading("Roles & Faction");

                    let mut roles = Vec::new();
                    if npc.is_merchant {
                        roles.push("🏪 Merchant");
                    }
                    if npc.is_innkeeper {
                        roles.push("🛏️ Innkeeper");
                    }

                    if !roles.is_empty() {
                        for role in roles {
                            ui.label(role);
                        }
                    } else {
                        ui.label("No special roles");
                    }

                    if let Some(faction) = &npc.faction {
                        ui.label(format!("⚔️ Faction: {}", faction));
                    }
                });
            });
    }

    fn show_edit_view(&mut self, ui: &mut egui::Ui, campaign_dir: Option<&PathBuf>) -> bool {
        let mut needs_save = false;
        let is_add = self.mode == NpcEditorMode::Add;

        ui.heading(if is_add { "Add New NPC" } else { "Edit NPC" });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Information");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.id)
                                .id_salt("npc_edit_id"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.name)
                                .id_salt("npc_edit_name"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.edit_buffer.description)
                            .desired_rows(3)
                            .id_salt("npc_edit_description"),
                    );
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Appearance");

                    ui.horizontal(|ui| {
                        ui.label("Portrait ID:");
                        autocomplete_portrait_selector(
                            ui,
                            "npc_edit_portrait",
                            "",
                            &mut self.edit_buffer.portrait_id,
                            &self.available_portraits,
                            campaign_dir,
                        );
                    });
                    ui.label(
                        "📁 Relative to campaign assets directory (e.g., '0', '1', 'warrior')",
                    );
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Dialogue & Quests");

                    ui.horizontal(|ui| {
                        ui.label("Dialogue ID:");
                        egui::ComboBox::from_id_salt("npc_edit_dialogue_select")
                            .selected_text(if self.edit_buffer.dialogue_id.is_empty() {
                                "None".to_string()
                            } else {
                                self.edit_buffer.dialogue_id.clone()
                            })
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.edit_buffer.dialogue_id.is_empty(),
                                        "None",
                                    )
                                    .clicked()
                                {
                                    self.edit_buffer.dialogue_id.clear();
                                }

                                for dialogue in &self.available_dialogues {
                                    let label = format!("{} - {}", dialogue.id, dialogue.name);
                                    if ui
                                        .selectable_label(
                                            self.edit_buffer.dialogue_id == dialogue.id.to_string(),
                                            &label,
                                        )
                                        .clicked()
                                    {
                                        self.edit_buffer.dialogue_id = dialogue.id.to_string();
                                    }
                                }
                            });
                    });

                    ui.separator();

                    ui.label("Associated Quests:");
                    egui::ScrollArea::vertical()
                        .id_salt("npc_edit_quests_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for (quest_idx, quest) in self.available_quests.iter().enumerate() {
                                ui.push_id(format!("npc_edit_quest_{}", quest_idx), |ui| {
                                    let quest_id_str = quest.id.to_string();
                                    let mut is_selected =
                                        self.edit_buffer.quest_ids.contains(&quest_id_str);

                                    if ui
                                        .checkbox(
                                            &mut is_selected,
                                            format!("{} - {}", quest.id, quest.name),
                                        )
                                        .on_hover_text(format!("Quest ID: {}", quest.id))
                                        .clicked()
                                    {
                                        if is_selected {
                                            if !self.edit_buffer.quest_ids.contains(&quest_id_str) {
                                                self.edit_buffer.quest_ids.push(quest_id_str);
                                            }
                                        } else {
                                            self.edit_buffer
                                                .quest_ids
                                                .retain(|id| id != &quest_id_str);
                                        }
                                    }
                                });
                            }
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Faction & Roles");

                    ui.horizontal(|ui| {
                        ui.label("Faction:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.faction)
                                .id_salt("npc_edit_faction"),
                        );
                    });

                    ui.checkbox(&mut self.edit_buffer.is_merchant, "🏪 Is Merchant");
                    ui.checkbox(&mut self.edit_buffer.is_innkeeper, "🛏️ Is Innkeeper");
                });

                ui.add_space(10.0);

                // Validation errors
                if !self.validation_errors.is_empty() {
                    ui.group(|ui| {
                        ui.heading("⚠️ Validation Errors");
                        for error in &self.validation_errors {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                    });
                    ui.add_space(10.0);
                }

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = NpcEditorMode::List;
                    }

                    if ui.button("💾 Save").clicked() {
                        self.validate_edit_buffer();
                        if self.validation_errors.is_empty() {
                            if self.save_npc() {
                                needs_save = true;
                                self.mode = NpcEditorMode::List;
                            }
                        }
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = NpcEditorMode::List;
                        self.edit_buffer = NpcEditBuffer::default();
                    }
                });
            });

        needs_save
    }

    fn matches_filters(&self, npc: &NpcDefinition) -> bool {
        // Search filter
        let search_lower = self.search_filter.to_lowercase();
        if !search_lower.is_empty()
            && !npc.name.to_lowercase().contains(&search_lower)
            && !npc.id.to_lowercase().contains(&search_lower)
        {
            return false;
        }

        // Merchant filter
        if self.filter_merchants && !npc.is_merchant {
            return false;
        }

        // Innkeeper filter
        if self.filter_innkeepers && !npc.is_innkeeper {
            return false;
        }

        // Quest giver filter
        if self.filter_quest_givers && npc.quest_ids.is_empty() {
            return false;
        }

        true
    }

    fn start_add_npc(&mut self) {
        self.mode = NpcEditorMode::Add;
        self.edit_buffer = NpcEditBuffer::default();
        self.edit_buffer.id = self.next_npc_id();
    }

    fn start_edit_npc(&mut self, index: usize) {
        if let Some(npc) = self.npcs.get(index) {
            self.mode = NpcEditorMode::Edit;
            self.selected_npc = Some(index);
            self.edit_buffer = NpcEditBuffer {
                id: npc.id.clone(),
                name: npc.name.clone(),
                description: npc.description.clone(),
                portrait_id: npc.portrait_id.clone(),
                dialogue_id: npc
                    .dialogue_id
                    .as_ref()
                    .map_or(String::new(), |id| id.to_string()),
                quest_ids: npc.quest_ids.iter().map(|id| id.to_string()).collect(),
                faction: npc.faction.as_ref().unwrap_or(&String::new()).clone(),
                is_merchant: npc.is_merchant,
                is_innkeeper: npc.is_innkeeper,
            };
        }
    }

    fn validate_edit_buffer(&mut self) {
        self.validation_errors.clear();

        // Validate ID
        if self.edit_buffer.id.is_empty() {
            self.validation_errors
                .push("ID cannot be empty".to_string());
        } else if !self.is_valid_id(&self.edit_buffer.id) {
            self.validation_errors.push(
                "ID must start with a letter and contain only alphanumeric characters and underscores"
                    .to_string(),
            );
        } else {
            // Check for duplicate IDs
            match self.mode {
                NpcEditorMode::Add => {
                    if self.npcs.iter().any(|n| n.id == self.edit_buffer.id) {
                        self.validation_errors
                            .push(format!("ID '{}' already exists", self.edit_buffer.id));
                    }
                }
                NpcEditorMode::Edit => {
                    if let Some(selected_idx) = self.selected_npc {
                        if self
                            .npcs
                            .iter()
                            .enumerate()
                            .any(|(idx, n)| idx != selected_idx && n.id == self.edit_buffer.id)
                        {
                            self.validation_errors
                                .push(format!("ID '{}' already exists", self.edit_buffer.id));
                        }
                    }
                }
                _ => {}
            }
        }

        // Validate name
        if self.edit_buffer.name.is_empty() {
            self.validation_errors
                .push("Name cannot be empty".to_string());
        }

        // Validate dialogue ID if provided
        if !self.edit_buffer.dialogue_id.is_empty() {
            let dialogue_exists = self
                .available_dialogues
                .iter()
                .any(|d| d.id.to_string() == self.edit_buffer.dialogue_id);
            if !dialogue_exists {
                self.validation_errors.push(format!(
                    "Dialogue ID '{}' does not exist",
                    self.edit_buffer.dialogue_id
                ));
            }
        }

        // Validate quest IDs
        for quest_id_str in &self.edit_buffer.quest_ids {
            let quest_exists = self
                .available_quests
                .iter()
                .any(|q| q.id.to_string() == *quest_id_str);
            if !quest_exists {
                self.validation_errors
                    .push(format!("Quest ID '{}' does not exist", quest_id_str));
            }
        }
    }

    fn is_valid_id(&self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        let first_char = id.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return false;
        }
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn save_npc(&mut self) -> bool {
        self.validate_edit_buffer();
        if !self.validation_errors.is_empty() {
            return false;
        }

        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
        };

        match self.mode {
            NpcEditorMode::Add => {
                self.npcs.push(npc);
                true
            }
            NpcEditorMode::Edit => {
                if let Some(index) = self.selected_npc {
                    if index < self.npcs.len() {
                        self.npcs[index] = npc;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn export_all_npcs(&mut self) -> bool {
        if let Ok(ron_str) =
            ron::ser::to_string_pretty(&self.npcs, ron::ser::PrettyConfig::default())
        {
            self.import_buffer = ron_str;
            self.show_import_dialog = true;
            false // Export doesn't require save
        } else {
            false
        }
    }

    fn show_import_dialog_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import/Export NPC")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("NPC RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_buffer)
                        .desired_rows(15)
                        .code_editor()
                        .id_salt("npc_import_buffer"),
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<NpcDefinition>(&self.import_buffer) {
                            Ok(mut npc) => {
                                let next_id = self.next_npc_id();
                                npc.id = next_id;
                                self.npcs.push(npc);
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                // Show error in validation
                                self.validation_errors.push(format!("Import failed: {}", e));
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_buffer.clone());
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });

        self.show_import_dialog = open;
    }

    /// Generate next available NPC ID
    pub fn next_npc_id(&self) -> String {
        let existing_ids: HashSet<String> = self.npcs.iter().map(|n| n.id.clone()).collect();

        let mut counter = 1;
        loop {
            let candidate = format!("npc_{}", counter);
            if !existing_ids.contains(&candidate) {
                return candidate;
            }
            counter += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_editor_state_new() {
        let state = NpcEditorState::new();
        assert_eq!(state.npcs.len(), 0);
        assert_eq!(state.mode, NpcEditorMode::List);
    }

    #[test]
    fn test_start_add_npc() {
        let mut state = NpcEditorState::new();
        state.start_add_npc();
        assert_eq!(state.mode, NpcEditorMode::Add);
        assert!(!state.edit_buffer.id.is_empty());
    }

    #[test]
    fn test_validate_edit_buffer_empty_id() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = String::new();
        state.edit_buffer.name = "Test".to_string();
        state.validate_edit_buffer();
        assert!(!state.validation_errors.is_empty());
        assert!(state.validation_errors[0].contains("ID cannot be empty"));
    }

    #[test]
    fn test_validate_edit_buffer_invalid_id() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "123invalid".to_string();
        state.validate_edit_buffer();
        assert!(state
            .validation_errors
            .iter()
            .any(|e| e.contains("must start")));
    }

    #[test]
    fn test_validate_edit_buffer_valid() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "valid_id".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_save_npc_add_mode() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "test_npc".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.description = "Test description".to_string();

        let result = state.save_npc();
        assert!(result);
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].id, "test_npc");
        assert_eq!(state.npcs[0].name, "Test NPC");
    }

    #[test]
    fn test_save_npc_edit_mode() {
        let mut state = NpcEditorState::new();

        // Add an NPC first
        state.npcs.push(NpcDefinition {
            id: "npc1".to_string(),
            name: "Original".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        // Edit it
        state.mode = NpcEditorMode::Edit;
        state.selected_npc = Some(0);
        state.edit_buffer.id = "npc1".to_string();
        state.edit_buffer.name = "Modified".to_string();

        let result = state.save_npc();
        assert!(result);
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].name, "Modified");
    }

    #[test]
    fn test_matches_filters_no_filters() {
        let state = NpcEditorState::new();
        let npc = NpcDefinition {
            id: "test".to_string(),
            name: "Test NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };
        assert!(state.matches_filters(&npc));
    }

    #[test]
    fn test_matches_filters_search() {
        let mut state = NpcEditorState::new();
        state.search_filter = "merchant".to_string();

        let npc = NpcDefinition {
            id: "test".to_string(),
            name: "Merchant Bob".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };
        assert!(state.matches_filters(&npc));

        let npc2 = NpcDefinition {
            id: "test2".to_string(),
            name: "Innkeeper Jane".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
        };
        assert!(!state.matches_filters(&npc2));
    }

    #[test]
    fn test_matches_filters_merchant_filter() {
        let mut state = NpcEditorState::new();
        state.filter_merchants = true;

        let merchant = NpcDefinition {
            id: "merchant".to_string(),
            name: "Bob".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };
        assert!(state.matches_filters(&merchant));

        let non_merchant = NpcDefinition {
            id: "guard".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };
        assert!(!state.matches_filters(&non_merchant));
    }

    #[test]
    fn test_next_npc_id() {
        let mut state = NpcEditorState::new();

        let id1 = state.next_npc_id();
        assert_eq!(id1, "npc_1");

        state.npcs.push(NpcDefinition {
            id: "npc_1".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        let id2 = state.next_npc_id();
        assert_eq!(id2, "npc_2");
    }

    #[test]
    fn test_is_valid_id() {
        let state = NpcEditorState::new();
        assert!(state.is_valid_id("valid_id"));
        assert!(state.is_valid_id("_valid"));
        assert!(state.is_valid_id("Valid123"));
        assert!(!state.is_valid_id("123invalid"));
        assert!(!state.is_valid_id("invalid-id"));
        assert!(!state.is_valid_id(""));
    }

    #[test]
    fn test_validate_duplicate_id_add_mode() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "existing".to_string(),
            name: "Existing NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "existing".to_string();
        state.edit_buffer.name = "New NPC".to_string();
        state.validate_edit_buffer();

        assert!(!state.validation_errors.is_empty());
        assert!(state
            .validation_errors
            .iter()
            .any(|e| e.contains("already exists")));
    }

    #[test]
    fn test_npc_editor_mode_equality() {
        assert_eq!(NpcEditorMode::List, NpcEditorMode::List);
        assert_ne!(NpcEditorMode::List, NpcEditorMode::Add);
    }
}
