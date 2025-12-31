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

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::dialogue::{DialogueId, DialogueTree};
use antares::domain::quest::{Quest, QuestId};
use antares::domain::world::npc::{NpcDefinition, NpcId};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    ///
    /// # Returns
    ///
    /// Returns `true` if changes were made requiring save
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &[DialogueTree],
        quests: &[Quest],
    ) -> bool {
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

        // Mode-specific UI
        match self.mode {
            NpcEditorMode::List => {
                needs_save |= self.show_list_view(ui);
            }
            NpcEditorMode::Add | NpcEditorMode::Edit => {
                needs_save |= self.show_edit_view(ui);
            }
        }

        // Import dialog
        if self.show_import_dialog {
            self.show_import_dialog_window(ui.ctx());
        }

        needs_save
    }

    /// Shows the list view with all NPCs
    fn show_list_view(&mut self, ui: &mut egui::Ui) -> bool {
        let mut needs_save = false;

        // Search and filters
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.search_filter);

            ui.separator();

            ui.checkbox(&mut self.filter_merchants, "Merchants");
            ui.checkbox(&mut self.filter_innkeepers, "Innkeepers");
            ui.checkbox(&mut self.filter_quest_givers, "Quest Givers");

            if ui.button("Clear Filters").clicked() {
                self.search_filter.clear();
                self.filter_merchants = false;
                self.filter_innkeepers = false;
                self.filter_quest_givers = false;
            }
        });

        ui.separator();

        // NPC count
        let filtered_npcs: Vec<(usize, &NpcDefinition)> = self
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, npc)| self.matches_filters(npc))
            .collect();

        ui.label(format!(
            "📊 NPCs: {} / {}",
            filtered_npcs.len(),
            self.npcs.len()
        ));

        ui.separator();

        // Track actions to perform after iteration
        let mut index_to_delete: Option<usize> = None;
        let mut index_to_edit: Option<usize> = None;

        // NPC list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, npc) in &filtered_npcs {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(&npc.name);
                            ui.label(format!("ID: {}", npc.id));

                            if !npc.description.is_empty() {
                                ui.label(&npc.description);
                            }

                            // Tags
                            ui.horizontal(|ui| {
                                if npc.is_merchant {
                                    ui.label("🏪 Merchant");
                                }
                                if npc.is_innkeeper {
                                    ui.label("🛏️ Innkeeper");
                                }
                                if !npc.quest_ids.is_empty() {
                                    ui.label(format!("📜 {} Quests", npc.quest_ids.len()));
                                }
                                if npc.dialogue_id.is_some() {
                                    ui.label("💬 Has Dialogue");
                                }
                                if let Some(faction) = &npc.faction {
                                    ui.label(format!("⚔️ {}", faction));
                                }
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑️ Delete").clicked() {
                                index_to_delete = Some(*index);
                            }
                            if ui.button("✏️ Edit").clicked() {
                                index_to_edit = Some(*index);
                            }
                        });
                    });
                });
            }

            if filtered_npcs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No NPCs found. Click 'Add NPC' to create one.");
                });
            }
        });

        // Apply actions after iteration
        if let Some(index) = index_to_delete {
            self.npcs.remove(index);
            needs_save = true;
        }
        if let Some(index) = index_to_edit {
            self.start_edit_npc(index);
        }

        needs_save
    }

    /// Shows the edit view for adding or editing an NPC
    fn show_edit_view(&mut self, ui: &mut egui::Ui) -> bool {
        let mut needs_save = false;

        // Validate current buffer
        self.validate_edit_buffer();

        let title = match self.mode {
            NpcEditorMode::Add => "Add New NPC",
            NpcEditorMode::Edit => "Edit NPC",
            _ => "NPC Editor",
        };

        let preview_buffer = self.edit_buffer.clone();
        TwoColumnLayout::new(title)
            .with_left_width(300.0)
            .show_split(
                ui,
                |left_ui| {
                    // Left column: Form fields
                    egui::ScrollArea::vertical().show(left_ui, |ui| {
                        ui.heading("Basic Information");

                        ui.horizontal(|ui| {
                            ui.label("ID:");
                            ui.text_edit_singleline(&mut self.edit_buffer.id);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.edit_buffer.name);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Description:");
                        });
                        ui.text_edit_multiline(&mut self.edit_buffer.description);

                        ui.separator();

                        ui.heading("Appearance");

                        ui.horizontal(|ui| {
                            ui.label("Portrait ID:");
                            ui.text_edit_singleline(&mut self.edit_buffer.portrait_id);
                        });
                        ui.label("📁 Relative to campaign assets directory");

                        ui.separator();

                        ui.heading("Dialogue & Quests");

                        // Dialogue ID autocomplete
                        ui.horizontal(|ui| {
                            ui.label("Dialogue ID:");
                        });

                        let dialogue_options: Vec<String> = self
                            .available_dialogues
                            .iter()
                            .map(|d| format!("{} - {}", d.id, d.name))
                            .collect();

                        egui::ComboBox::from_id_salt("npc_dialogue_select")
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

                        ui.separator();

                        // Quest IDs multi-select
                        ui.label("Associated Quests:");

                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for quest in &self.available_quests {
                                    let quest_id_str = quest.id.to_string();
                                    let mut is_selected =
                                        self.edit_buffer.quest_ids.contains(&quest_id_str);

                                    if ui
                                        .checkbox(
                                            &mut is_selected,
                                            format!("{} - {}", quest.id, quest.name),
                                        )
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
                                }
                            });

                        ui.separator();

                        ui.heading("Faction & Roles");

                        ui.horizontal(|ui| {
                            ui.label("Faction:");
                            ui.text_edit_singleline(&mut self.edit_buffer.faction);
                        });

                        ui.checkbox(&mut self.edit_buffer.is_merchant, "🏪 Is Merchant");
                        ui.checkbox(&mut self.edit_buffer.is_innkeeper, "🛏️ Is Innkeeper");

                        ui.separator();

                        // Validation errors
                        if !self.validation_errors.is_empty() {
                            ui.group(|ui| {
                                ui.heading("⚠️ Validation Errors");
                                for error in &self.validation_errors {
                                    ui.label(error);
                                }
                            });
                        }
                    });
                },
                |right_ui| {
                    // Right column: Preview and actions
                    egui::ScrollArea::vertical().show(right_ui, |ui| {
                        ui.heading("Preview");

                        ui.group(|ui| {
                            ui.label(format!("ID: {}", preview_buffer.id));
                            ui.label(format!("Name: {}", preview_buffer.name));

                            if !preview_buffer.description.is_empty() {
                                ui.separator();
                                ui.label(&preview_buffer.description);
                            }

                            ui.separator();

                            ui.label(format!("Portrait: {}", preview_buffer.portrait_id));

                            if !preview_buffer.dialogue_id.is_empty() {
                                ui.label(format!("💬 Dialogue: {}", preview_buffer.dialogue_id));
                            }

                            if !preview_buffer.quest_ids.is_empty() {
                                ui.label(format!("📜 Quests: {}", preview_buffer.quest_ids.len()));
                                for quest_id in &preview_buffer.quest_ids {
                                    ui.label(format!("  - {}", quest_id));
                                }
                            }

                            if !preview_buffer.faction.is_empty() {
                                ui.label(format!("⚔️ Faction: {}", preview_buffer.faction));
                            }

                            ui.separator();

                            if preview_buffer.is_merchant {
                                ui.label("🏪 Merchant");
                            }
                            if preview_buffer.is_innkeeper {
                                ui.label("🛏️ Innkeeper");
                            }
                        });
                    });
                },
            );

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.validation_errors.is_empty(), egui::Button::new("Save"))
                .clicked()
            {
                if self.save_npc() {
                    needs_save = true;
                    self.mode = NpcEditorMode::List;
                }
            }
            if ui.button("Cancel").clicked() {
                self.mode = NpcEditorMode::List;
                self.edit_buffer = NpcEditBuffer::default();
            }
        });

        needs_save
    }

    /// Checks if an NPC matches current filters
    fn matches_filters(&self, npc: &NpcDefinition) -> bool {
        // Search filter
        if !self.search_filter.is_empty() {
            let search_lower = self.search_filter.to_lowercase();
            let matches_search = npc.name.to_lowercase().contains(&search_lower)
                || npc.id.to_lowercase().contains(&search_lower)
                || npc.description.to_lowercase().contains(&search_lower);

            if !matches_search {
                return false;
            }
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

    /// Starts adding a new NPC
    fn start_add_npc(&mut self) {
        self.mode = NpcEditorMode::Add;
        self.edit_buffer = NpcEditBuffer::default();
        self.validation_errors.clear();
    }

    /// Starts editing an existing NPC
    fn start_edit_npc(&mut self, index: usize) {
        if let Some(npc) = self.npcs.get(index) {
            self.mode = NpcEditorMode::Edit;
            self.selected_npc = Some(index);
            self.edit_buffer = NpcEditBuffer {
                id: npc.id.clone(),
                name: npc.name.clone(),
                description: npc.description.clone(),
                portrait_id: npc.portrait_id.clone(),
                dialogue_id: npc.dialogue_id.map(|id| id.to_string()).unwrap_or_default(),
                quest_ids: npc.quest_ids.iter().map(|id| id.to_string()).collect(),
                faction: npc.faction.clone().unwrap_or_default(),
                is_merchant: npc.is_merchant,
                is_innkeeper: npc.is_innkeeper,
            };
            self.validation_errors.clear();
        }
    }

    /// Validates the edit buffer
    fn validate_edit_buffer(&mut self) {
        self.validation_errors.clear();

        // ID validation
        if self.edit_buffer.id.trim().is_empty() {
            self.validation_errors
                .push("ID cannot be empty".to_string());
        } else if !self.is_valid_id(&self.edit_buffer.id) {
            self.validation_errors
                .push("ID can only contain letters, numbers, underscores, and hyphens".to_string());
        } else {
            // Check for duplicate IDs (only when adding or changing ID)
            let is_duplicate = match self.mode {
                NpcEditorMode::Add => self.npcs.iter().any(|npc| npc.id == self.edit_buffer.id),
                NpcEditorMode::Edit => {
                    if let Some(selected) = self.selected_npc {
                        self.npcs
                            .iter()
                            .enumerate()
                            .any(|(idx, npc)| idx != selected && npc.id == self.edit_buffer.id)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if is_duplicate {
                self.validation_errors
                    .push(format!("NPC ID '{}' already exists", self.edit_buffer.id));
            }
        }

        // Name validation
        if self.edit_buffer.name.trim().is_empty() {
            self.validation_errors
                .push("Name cannot be empty".to_string());
        }

        // Portrait path validation
        if self.edit_buffer.portrait_id.trim().is_empty() {
            self.validation_errors
                .push("Portrait ID cannot be empty".to_string());
        }

        // Dialogue ID validation
        if !self.edit_buffer.dialogue_id.is_empty() {
            if let Ok(dialogue_id) = self.edit_buffer.dialogue_id.parse::<DialogueId>() {
                if !self.available_dialogues.iter().any(|d| d.id == dialogue_id) {
                    self.validation_errors.push(format!(
                        "Dialogue ID {} does not exist",
                        self.edit_buffer.dialogue_id
                    ));
                }
            } else {
                self.validation_errors
                    .push("Invalid dialogue ID format (must be a number)".to_string());
            }
        }

        // Quest IDs validation
        for quest_id_str in &self.edit_buffer.quest_ids {
            if let Ok(quest_id) = quest_id_str.parse::<QuestId>() {
                if !self.available_quests.iter().any(|q| q.id == quest_id) {
                    self.validation_errors
                        .push(format!("Quest ID {} does not exist", quest_id_str));
                }
            } else {
                self.validation_errors
                    .push(format!("Invalid quest ID format: {}", quest_id_str));
            }
        }
    }

    /// Checks if an ID is valid (alphanumeric, underscore, hyphen)
    fn is_valid_id(&self, id: &str) -> bool {
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Saves the current NPC being edited
    fn save_npc(&mut self) -> bool {
        // Parse dialogue ID
        let dialogue_id = if self.edit_buffer.dialogue_id.is_empty() {
            None
        } else {
            self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
        };

        // Parse quest IDs
        let quest_ids: Vec<QuestId> = self
            .edit_buffer
            .quest_ids
            .iter()
            .filter_map(|s| s.parse::<QuestId>().ok())
            .collect();

        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id,
            quest_ids,
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
                    if let Some(existing) = self.npcs.get_mut(index) {
                        *existing = npc;
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

    /// Exports all NPCs to clipboard
    fn export_all_npcs(&self) -> bool {
        if let Ok(ron_string) = ron::ser::to_string_pretty(&self.npcs, Default::default()) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Err(e) =
                    arboard::Clipboard::new().and_then(|mut clip| clip.set_text(&ron_string))
                {
                    eprintln!("Failed to copy to clipboard: {}", e);
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Shows the import dialog window
    fn show_import_dialog_window(&mut self, ctx: &egui::Context) {
        let mut open = true;

        egui::Window::new("Import NPCs from RON")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Paste RON-formatted NPC data:");

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.text_edit_multiline(&mut self.import_buffer);
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        if let Ok(imported_npcs) =
                            ron::from_str::<Vec<NpcDefinition>>(&self.import_buffer)
                        {
                            self.npcs.extend(imported_npcs);
                            self.import_buffer.clear();
                            self.show_import_dialog = false;
                        } else {
                            self.validation_errors
                                .push("Failed to parse RON data".to_string());
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_import_dialog = false;
                        self.import_buffer.clear();
                    }
                });
            });

        if !open {
            self.show_import_dialog = false;
        }
    }

    /// Returns the next available NPC ID suggestion
    pub fn next_npc_id(&self) -> String {
        let mut counter = 1;
        loop {
            let id = format!("npc_{}", counter);
            if !self.npcs.iter().any(|n| n.id == id) {
                return id;
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
        assert!(state.selected_npc.is_none());
    }

    #[test]
    fn test_start_add_npc() {
        let mut state = NpcEditorState::new();
        state.start_add_npc();
        assert_eq!(state.mode, NpcEditorMode::Add);
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_validate_edit_buffer_empty_id() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.portrait_id = "test.png".to_string();
        state.validate_edit_buffer();
        assert!(!state.validation_errors.is_empty());
        assert!(state
            .validation_errors
            .iter()
            .any(|e| e.contains("ID cannot be empty")));
    }

    #[test]
    fn test_validate_edit_buffer_invalid_id() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "invalid id!".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.portrait_id = "test.png".to_string();
        state.validate_edit_buffer();
        assert!(!state.validation_errors.is_empty());
    }

    #[test]
    fn test_validate_edit_buffer_valid() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "test_npc_1".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.portrait_id = "test.png".to_string();
        state.mode = NpcEditorMode::Add;
        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_save_npc_add_mode() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer = NpcEditBuffer {
            id: "merchant_1".to_string(),
            name: "Bob the Merchant".to_string(),
            description: "A friendly merchant".to_string(),
            portrait_id: "portraits/merchant.png".to_string(),
            dialogue_id: String::new(),
            quest_ids: Vec::new(),
            faction: "Merchants".to_string(),
            is_merchant: true,
            is_innkeeper: false,
        };

        assert!(state.save_npc());
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].id, "merchant_1");
        assert!(state.npcs[0].is_merchant);
    }

    #[test]
    fn test_save_npc_edit_mode() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "npc_1".to_string(),
            name: "Old Name".to_string(),
            description: String::new(),
            portrait_id: "old.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        state.mode = NpcEditorMode::Edit;
        state.selected_npc = Some(0);
        state.edit_buffer = NpcEditBuffer {
            id: "npc_1".to_string(),
            name: "New Name".to_string(),
            description: String::new(),
            portrait_id: "old.png".to_string(),
            dialogue_id: String::new(),
            quest_ids: Vec::new(),
            faction: String::new(),
            is_merchant: false,
            is_innkeeper: false,
        };

        assert!(state.save_npc());
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].name, "New Name");
    }

    #[test]
    fn test_matches_filters_no_filters() {
        let state = NpcEditorState::new();
        let npc = NpcDefinition {
            id: "npc_1".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
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
            id: "merchant_1".to_string(),
            name: "Bob the Merchant".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };

        assert!(state.matches_filters(&npc));
    }

    #[test]
    fn test_matches_filters_merchant_filter() {
        let mut state = NpcEditorState::new();
        state.filter_merchants = true;

        let merchant = NpcDefinition {
            id: "merchant_1".to_string(),
            name: "Merchant".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };

        let non_merchant = NpcDefinition {
            id: "guard_1".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };

        assert!(state.matches_filters(&merchant));
        assert!(!state.matches_filters(&non_merchant));
    }

    #[test]
    fn test_next_npc_id() {
        let mut state = NpcEditorState::new();
        assert_eq!(state.next_npc_id(), "npc_1");

        state.npcs.push(NpcDefinition {
            id: "npc_1".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        assert_eq!(state.next_npc_id(), "npc_2");
    }

    #[test]
    fn test_is_valid_id() {
        let state = NpcEditorState::new();
        assert!(state.is_valid_id("valid_id"));
        assert!(state.is_valid_id("valid-id"));
        assert!(state.is_valid_id("valid123"));
        assert!(!state.is_valid_id("invalid id"));
        assert!(!state.is_valid_id("invalid!id"));
        assert!(!state.is_valid_id(""));
    }

    #[test]
    fn test_validate_duplicate_id_add_mode() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "existing".to_string(),
            name: "Existing".to_string(),
            description: String::new(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "existing".to_string();
        state.edit_buffer.name = "New".to_string();
        state.edit_buffer.portrait_id = "test.png".to_string();

        state.validate_edit_buffer();
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
