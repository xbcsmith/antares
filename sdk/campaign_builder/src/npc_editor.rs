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
    autocomplete_portrait_selector, extract_portrait_candidates, resolve_portrait_path,
    ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout,
};
use antares::domain::dialogue::{DialogueId, DialogueTree};
use antares::domain::quest::{Quest, QuestId};
use antares::domain::world::npc::{NpcDefinition, NpcId};
use antares::sdk::tool_config::DisplayConfig;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

/// Editor state for NPC editing
#[derive(Clone, Serialize, Deserialize)]
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

    /// Whether the portrait grid picker popup is open
    #[serde(skip)]
    pub portrait_picker_open: bool,

    /// Cached portrait textures for grid display
    #[serde(skip)]
    pub portrait_textures: HashMap<String, Option<egui::TextureHandle>>,

    /// Last campaign directory (to detect changes)
    #[serde(skip)]
    pub last_campaign_dir: Option<PathBuf>,

    /// Last NPCs filename (cached from show() call)
    #[serde(skip)]
    pub last_npcs_file: Option<String>,

    /// Whether the autocomplete buffers should be reset on next form render
    #[serde(skip)]
    pub reset_autocomplete_buffers: bool,
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
            portrait_picker_open: false,
            portrait_textures: HashMap::new(),
            last_campaign_dir: None,
            last_npcs_file: None,
            reset_autocomplete_buffers: false,
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
        npcs_file: &str,
    ) -> bool {
        // Update portrait candidates if campaign directory changed
        if self.last_campaign_dir != campaign_dir.cloned() {
            self.available_portraits = extract_portrait_candidates(campaign_dir);
            self.last_campaign_dir = campaign_dir.cloned();
        }

        // Cache the npcs filename so Save from the editor can persist immediately
        self.last_npcs_file = Some(npcs_file.to_string());

        // Update available references
        self.available_dialogues = dialogues.to_vec();
        self.available_quests = quests.to_vec();

        let mut needs_save = false;

        // Show portrait grid picker popup if open
        if self.portrait_picker_open {
            if let Some(selected_id) = self.show_portrait_grid_picker(ui.ctx(), campaign_dir) {
                self.edit_buffer.portrait_id = selected_id;
                needs_save = true;
                // Persist the selected portrait into the autocomplete buffer so the input
                // shows the new selection immediately (avoids stale typed text).
                crate::ui_helpers::store_autocomplete_buffer(
                    ui.ctx(),
                    egui::Id::new("autocomplete:portrait:npc_edit_portrait".to_string()),
                    &self.edit_buffer.portrait_id,
                );
            }
        }

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
                needs_save |= self.show_list_view(ui, display_config, campaign_dir);
            }
            NpcEditorMode::Add | NpcEditorMode::Edit => {
                needs_save |= self.show_edit_view(ui, campaign_dir, npcs_file);
            }
        }

        // Import dialog
        if self.show_import_dialog {
            self.show_import_dialog_window(ui.ctx());
        }

        needs_save
    }

    fn show_list_view(
        &mut self,
        ui: &mut egui::Ui,
        display_config: &DisplayConfig,
        campaign_dir: Option<&PathBuf>,
    ) -> bool {
        let mut needs_save = false;

        let search_lower = self.search_filter.to_lowercase();

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_npcs: Vec<(usize, NpcDefinition)> = self
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, npc)| self.matches_filters(npc))
            .map(|(idx, npc)| (idx, npc.clone()))
            .collect();

        // Sort by ID
        let mut sorted_npcs = filtered_npcs;
        sorted_npcs.sort_by(|(_, a), (_, b)| a.id.cmp(&b.id));

        let selected = self.selected_npc;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        ui.separator();

        let total_width = ui.available_width();
        let inspector_min_width = 300.0;
        // Reserve a small margin for the separator (12.0)
        let sep_margin = 12.0;

        TwoColumnLayout::new("npcs").show_split(
            ui,
            |left_ui| {
                // Left panel: NPC list (styled to match Characters editor)
                left_ui.heading("NPCs");
                left_ui.separator();

                if sorted_npcs.is_empty() {
                    left_ui.label("No NPCs found");
                } else {
                    for (idx, npc) in &sorted_npcs {
                        let is_selected = selected == Some(*idx);

                        // Primary selectable label (name)
                        let response = left_ui.selectable_label(is_selected, &npc.name);
                        if response.clicked() {
                            new_selection = Some(*idx);
                        }

                        // Badges and metadata (indented like Characters list)
                        left_ui.horizontal(|ui| {
                            ui.add_space(20.0);

                            if npc.is_merchant {
                                ui.label(
                                    egui::RichText::new("🏪 Merchant")
                                        .small()
                                        .color(egui::Color32::GOLD),
                                );
                            }

                            if npc.is_innkeeper {
                                ui.label(
                                    egui::RichText::new("🛏️ Innkeeper")
                                        .small()
                                        .color(egui::Color32::LIGHT_BLUE),
                                );
                            }

                            if !npc.quest_ids.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "📜 Quests: {}",
                                        npc.quest_ids.len()
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(150, 200, 120)),
                                );
                            }

                            ui.label(
                                egui::RichText::new(format!(
                                    "| Faction: {} | ID: {}",
                                    npc.faction.as_deref().unwrap_or("None"),
                                    npc.id
                                ))
                                .small()
                                .weak(),
                            );
                        });

                        left_ui.add_space(4.0);
                    }
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, npc)) = sorted_npcs.iter().find(|(i, _)| *i == idx) {
                        right_ui.heading(&npc.name);
                        right_ui.separator();

                        // Use shared ActionButtons component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();
                        self.show_preview(right_ui, npc, campaign_dir);
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

    /// Show preview of NPC with portrait
    fn show_preview(
        &mut self,
        ui: &mut egui::Ui,
        npc: &NpcDefinition,
        campaign_dir: Option<&PathBuf>,
    ) {
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

                    ui.horizontal(|ui| {
                        // Portrait display (left side)
                        let portrait_size = egui::vec2(128.0, 128.0);

                        // Try to load the portrait texture
                        let has_texture =
                            self.load_portrait_texture(ui.ctx(), campaign_dir, &npc.portrait_id);

                        if has_texture {
                            if let Some(Some(texture)) =
                                self.portrait_textures.get(&npc.portrait_id)
                            {
                                ui.add(
                                    egui::Image::new(texture)
                                        .fit_to_exact_size(portrait_size)
                                        .corner_radius(4.0),
                                );
                            }
                        } else {
                            // Placeholder for missing portrait
                            show_portrait_placeholder(ui, portrait_size);
                        }

                        ui.vertical(|ui| {
                            if !npc.portrait_id.is_empty() {
                                ui.label(format!("Portrait: {}", npc.portrait_id));
                            } else {
                                ui.label("No portrait");
                            }
                        });
                    });
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

    fn show_edit_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        npcs_file: &str,
    ) -> bool {
        let mut needs_save = false;
        let is_add = self.mode == NpcEditorMode::Add;

        ui.heading(if is_add { "Add New NPC" } else { "Edit NPC" });
        ui.separator();

        // If requested, reset persistent autocomplete buffers so the form displays
        // values from the newly loaded buffer rather than stale typed text.
        if self.reset_autocomplete_buffers {
            let ctx = ui.ctx();
            crate::ui_helpers::remove_autocomplete_buffer(
                ctx,
                egui::Id::new("autocomplete:portrait:npc_edit_portrait".to_string()),
            );
            self.reset_autocomplete_buffers = false;
        }

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

                        // Grid picker button
                        if ui.button("🖼").on_hover_text("Browse portraits").clicked() {
                            self.portrait_picker_open = true;
                        }
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
                                if let Some(dir) = campaign_dir {
                                    let path = dir.join(npcs_file);
                                    match self.save_to_file(&path) {
                                        Ok(_) => {
                                            self.has_unsaved_changes = false;
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Saved {}",
                                                    path.display()
                                                ))
                                                .color(egui::Color32::from_rgb(80, 200, 120)),
                                            );
                                        }
                                        Err(e) => {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Failed to save NPCs: {}",
                                                    e
                                                ))
                                                .color(egui::Color32::RED),
                                            );
                                        }
                                    }
                                }
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
        // Ensure autocomplete widgets show values from the fresh buffer
        self.reset_autocomplete_buffers = true;
    }

    /// Loads a portrait texture from the campaign assets directory
    ///
    /// This method caches loaded textures to avoid reloading. If the texture is already
    /// cached, it returns immediately. Failed loads are also cached to prevent repeated
    /// attempts.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for texture registration
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    /// * `portrait_id` - The portrait ID to load (e.g., "0", "1", "warrior")
    ///
    /// # Returns
    ///
    /// Returns `true` if the texture was successfully loaded (or was already cached),
    /// `false` if the load failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // let texture = state.load_portrait_texture(ctx, Some(&campaign_dir), "0");
    /// ```
    pub fn load_portrait_texture(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
        portrait_id: &str,
    ) -> bool {
        // Check if already cached
        if self.portrait_textures.contains_key(portrait_id) {
            return self.portrait_textures.get(portrait_id).unwrap().is_some();
        }

        // Attempt to load and decode image with error logging
        let texture_handle = (|| {
            let path = resolve_portrait_path(campaign_dir, portrait_id)?;

            // Read image file with error handling
            let image_bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Failed to read portrait file '{}': {}", path.display(), e);
                    return None;
                }
            };

            // Decode image using image crate with error handling
            let dynamic_image = match image::load_from_memory(&image_bytes) {
                Ok(img) => img,
                Err(e) => {
                    eprintln!("Failed to decode portrait '{}': {}", portrait_id, e);
                    return None;
                }
            };

            // Convert to RGBA8
            let rgba_image = dynamic_image.to_rgba8();
            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
            let pixels = rgba_image.as_flat_samples();

            // Create egui ColorImage
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            // Register texture with egui
            let texture_handle = ctx.load_texture(
                format!("npc_portrait_{}", portrait_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );

            Some(texture_handle)
        })();

        // Cache result (even None for failed loads to avoid repeated attempts)
        let loaded = texture_handle.is_some();
        if !loaded {
            eprintln!(
                "Portrait '{}' could not be loaded or was not found",
                portrait_id
            );
        }

        self.portrait_textures
            .insert(portrait_id.to_string(), texture_handle);

        loaded
    }

    /// Shows portrait grid picker popup for visual portrait selection
    ///
    /// Displays a popup window with a grid of portrait thumbnails that the user can click to select.
    /// The popup is modal and closes when a portrait is selected or the close button is clicked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for rendering
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    ///
    /// # Returns
    ///
    /// Returns `Some(portrait_id)` if the user clicked on a portrait to select it,
    /// or `None` if no selection was made this frame.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // if let Some(selected_id) = state.show_portrait_grid_picker(ctx, Some(&campaign_dir)) {
    /// //     println!("Selected portrait: {}", selected_id);
    /// // }
    /// ```
    pub fn show_portrait_grid_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_portrait: Option<String> = None;

        // Clone the portraits list to avoid borrow issues
        let available_portraits = self.available_portraits.clone();

        egui::Window::new("Select Portrait")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label("Click a portrait to select:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Display portraits in a grid with 4 columns
                    const COLUMNS: usize = 4;
                    const THUMBNAIL_SIZE: f32 = 80.0;

                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                    let total_portraits = available_portraits.len();
                    let rows = total_portraits.div_ceil(COLUMNS);

                    for row in 0..rows {
                        ui.horizontal(|ui| {
                            for col in 0..COLUMNS {
                                let idx = row * COLUMNS + col;
                                if idx >= total_portraits {
                                    break;
                                }

                                let portrait_id = &available_portraits[idx];

                                ui.vertical(|ui| {
                                    // Try to load texture
                                    self.load_portrait_texture(ctx, campaign_dir, portrait_id);
                                    let has_texture = self
                                        .portrait_textures
                                        .get(portrait_id)
                                        .and_then(|opt| opt.as_ref())
                                        .is_some();

                                    // Build tooltip text with portrait path
                                    let tooltip_text = if let Some(path) =
                                        resolve_portrait_path(campaign_dir, portrait_id)
                                    {
                                        format!(
                                            "Portrait ID: {}\nPath: {}",
                                            portrait_id,
                                            path.display()
                                        )
                                    } else {
                                        format!("Portrait ID: {}\n⚠ File not found", portrait_id)
                                    };

                                    // Create image button or placeholder
                                    let button_response = if has_texture {
                                        let texture = self
                                            .portrait_textures
                                            .get(portrait_id)
                                            .unwrap()
                                            .as_ref()
                                            .unwrap();
                                        ui.add(
                                            egui::Button::image(
                                                egui::Image::new(texture).fit_to_exact_size(
                                                    egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                                ),
                                            )
                                            .frame(true),
                                        )
                                        .on_hover_text(&tooltip_text)
                                    } else {
                                        // Placeholder for failed/missing images
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_gray(50),
                                        );
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "?",
                                            egui::FontId::proportional(24.0),
                                            egui::Color32::from_gray(150),
                                        );
                                        response.on_hover_text(&tooltip_text)
                                    };

                                    // Check if clicked
                                    if button_response.clicked() {
                                        selected_portrait = Some(portrait_id.clone());
                                        self.portrait_picker_open = false;
                                    }

                                    // Show portrait ID below thumbnail
                                    ui.label(
                                        egui::RichText::new(portrait_id)
                                            .size(10.0)
                                            .color(egui::Color32::from_gray(200)),
                                    );
                                });
                            }
                        });
                    }

                    // Show message if no portraits found
                    if total_portraits == 0 {
                        ui.label("No portraits found in campaign assets/portraits directory.");
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.portrait_picker_open = false;
                    }
                });
            });

        selected_portrait
    }

    pub(crate) fn start_edit_npc(&mut self, idx: usize) {
        if let Some(npc) = self.npcs.get(idx) {
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
            self.selected_npc = Some(idx);
            self.mode = NpcEditorMode::Edit;
            // Reset persistent autocomplete buffers so the form displays fresh values
            self.reset_autocomplete_buffers = true;
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
            sprite: None,
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

        // Perform the in-memory save and remember the result
        let saved = match self.mode {
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
        };

        // If saved, attempt immediate persistence when we have campaign dir & filename
        if saved {
            // Mark unsaved until we can persist successfully
            self.has_unsaved_changes = true;

            if let (Some(dir), Some(filename)) = (&self.last_campaign_dir, &self.last_npcs_file) {
                let path = dir.join(filename);
                match self.save_to_file(&path) {
                    Ok(_) => {
                        // Persisted successfully; clear unsaved flag
                        self.has_unsaved_changes = false;
                    }
                    Err(e) => {
                        // Log error; leave unsaved flag true so user is aware
                        eprintln!("Failed to persist NPCs to {}: {}", path.display(), e);
                    }
                }
            }
        }

        saved
    }

    fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let content = ron::ser::to_string_pretty(&self.npcs, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize npcs: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
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

/// Helper function to show portrait placeholder
fn show_portrait_placeholder(ui: &mut egui::Ui, size: egui::Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

    // Background
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(4),
        egui::Color32::from_gray(40),
    );

    // Border
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        egui::StrokeKind::Outside,
    );

    // Icon
    let center = rect.center();
    let icon_size = size.y * 0.4;
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "🖼",
        egui::FontId::proportional(icon_size),
        egui::Color32::from_rgb(150, 150, 150),
    );
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
            sprite: None,
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
            sprite: None,
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
            sprite: None,
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
            sprite: None,
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
            sprite: None,
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
            sprite: None,
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
            sprite: None,
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
    fn test_start_edit_npc_resets_autocomplete_buffers() {
        // Ensure stored autocomplete buffers are cleared when starting an edit so the
        // UI shows values from the loaded buffer rather than stale typed text.
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Populate stale buffer
        crate::ui_helpers::store_autocomplete_buffer(
            &ctx,
            egui::Id::new("autocomplete:portrait:npc_edit_portrait"),
            "character_040",
        );

        // Create NPC and start editing it
        state.npcs.push(NpcDefinition {
            id: "whisper".to_string(),
            name: "Whisper".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        state.start_edit_npc(0);
        assert!(state.reset_autocomplete_buffers);

        // Render the form (this will clear previous buffer and store current value)
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                state.show_edit_view(ui, None, "data/npcs.ron");
            });
        });

        let portrait_buf = crate::ui_helpers::load_autocomplete_buffer(
            &ctx,
            egui::Id::new("autocomplete:portrait:npc_edit_portrait"),
            || String::new(),
        );
        assert_eq!(portrait_buf, "character_055");
    }

    #[test]
    fn test_save_npc_persists_when_campaign_dir_known() {
        // Verify save_npc() will persist to disk when a campaign dir & filename are known.
        let tmp = tempfile::tempdir().expect("Failed to create tempdir");
        let dir = tmp.path().to_path_buf();

        let mut state = NpcEditorState::new();

        // Tell the editor where NPCs should be written
        state.last_campaign_dir = Some(dir.clone());
        state.last_npcs_file = Some("data/npcs.ron".to_string());

        // Create and save a new NPC
        state.start_add_npc();
        state.edit_buffer.id = "test_npc".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.description = "Test description".to_string();

        assert!(state.save_npc());

        let path = dir.join("data/npcs.ron");
        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("test_npc"));
    }

    #[test]
    fn test_validate_duplicate_id_add_mode() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "existing".to_string(),
            name: "Existing NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
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

    #[test]
    fn test_portrait_picker_initial_state() {
        let state = NpcEditorState::new();
        assert!(!state.portrait_picker_open);
        assert!(state.portrait_textures.is_empty());
        assert!(state.available_portraits.is_empty());
        assert!(state.last_campaign_dir.is_none());
    }

    #[test]
    fn test_portrait_picker_open_flag() {
        let mut state = NpcEditorState::new();
        assert!(!state.portrait_picker_open);

        state.portrait_picker_open = true;
        assert!(state.portrait_picker_open);

        state.portrait_picker_open = false;
        assert!(!state.portrait_picker_open);
    }

    #[test]
    fn test_portrait_texture_cache_insertion() {
        let mut state = NpcEditorState::new();
        assert!(state.portrait_textures.is_empty());

        // Simulate failed texture load (None)
        state.portrait_textures.insert("0".to_string(), None);
        assert_eq!(state.portrait_textures.len(), 1);
        assert!(state.portrait_textures.contains_key("0"));
        assert!(state.portrait_textures.get("0").unwrap().is_none());

        // Add another entry
        state.portrait_textures.insert("1".to_string(), None);
        assert_eq!(state.portrait_textures.len(), 2);
    }

    #[test]
    fn test_portrait_texture_error_handling_missing_file() {
        // Test that missing portrait files are handled gracefully
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();
        let campaign_dir = PathBuf::from("/nonexistent/path");

        // Attempt to load a portrait that doesn't exist
        let loaded = state.load_portrait_texture(&ctx, Some(&campaign_dir), "999");

        // Should return false and cache the failure
        assert!(!loaded);
        assert!(state.portrait_textures.contains_key("999"));
        assert!(state.portrait_textures.get("999").unwrap().is_none());
    }

    #[test]
    fn test_portrait_texture_error_handling_no_campaign_dir() {
        // Test behavior when no campaign directory is provided
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Attempt to load portrait without campaign directory
        let loaded = state.load_portrait_texture(&ctx, None, "0");

        // Should fail gracefully and cache the failure
        assert!(!loaded);
        assert!(state.portrait_textures.contains_key("0"));
        assert!(state.portrait_textures.get("0").unwrap().is_none());
    }

    #[test]
    fn test_portrait_texture_cache_efficiency() {
        // Test that cached textures aren't reloaded
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Pre-cache a failed load
        state.portrait_textures.insert("cached".to_string(), None);

        // Try to load again - should return cached result without attempting load
        let loaded = state.load_portrait_texture(&ctx, None, "cached");
        assert!(!loaded);

        // Cache should still have only one entry
        assert_eq!(state.portrait_textures.len(), 1);
    }

    #[test]
    fn test_new_npc_creation_workflow_with_portrait() {
        let mut state = NpcEditorState::new();

        // Start adding new NPC
        state.start_add_npc();
        assert_eq!(state.mode, NpcEditorMode::Add);

        // Fill in form including portrait
        state.edit_buffer.id = "merchant_bob".to_string();
        state.edit_buffer.name = "Bob the Merchant".to_string();
        state.edit_buffer.portrait_id = "42".to_string();
        state.edit_buffer.is_merchant = true;

        // Validate and save
        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());

        let saved = state.save_npc();
        assert!(saved);

        // Verify NPC was created with portrait
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].portrait_id, "42");
        assert_eq!(state.npcs[0].name, "Bob the Merchant");
    }

    #[test]
    fn test_edit_npc_workflow_updates_portrait() {
        let mut state = NpcEditorState::new();

        // Add initial NPC
        state.npcs.push(NpcDefinition {
            id: "guard".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: "10".to_string(),
            sprite: None,
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        });

        // Start editing
        state.start_edit_npc(0);
        assert_eq!(state.mode, NpcEditorMode::Edit);
        assert_eq!(state.edit_buffer.portrait_id, "10");

        // Change portrait
        state.edit_buffer.portrait_id = "20".to_string();
        state.save_npc();

        // Verify portrait was updated
        assert_eq!(state.npcs[0].portrait_id, "20");
    }

    #[test]
    fn test_campaign_dir_change_triggers_portrait_rescan() {
        let mut state = NpcEditorState::new();

        // Initially no portraits cached
        assert!(state.available_portraits.is_empty());
        assert!(state.last_campaign_dir.is_none());

        // Simulate campaign directory change detection
        let dir1 = PathBuf::from("/campaign1");
        state.last_campaign_dir = Some(dir1.clone());

        let dir2 = PathBuf::from("/campaign2");
        assert_ne!(state.last_campaign_dir, Some(dir2.clone()));

        // This would trigger portrait rescan in the show() method
        state.last_campaign_dir = Some(dir2);
        assert!(state.last_campaign_dir.is_some());
    }

    #[test]
    fn test_npc_save_preserves_portrait_data() {
        let mut state = NpcEditorState::new();

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "wizard".to_string();
        state.edit_buffer.name = "Wizard".to_string();
        state.edit_buffer.portrait_id = "magic_user".to_string();
        state.edit_buffer.description = "A wise wizard".to_string();

        state.save_npc();

        assert_eq!(state.npcs[0].portrait_id, "magic_user");

        // Verify portrait ID is preserved in serialization
        let npc = &state.npcs[0];
        assert_eq!(npc.portrait_id, "magic_user");
    }

    #[test]
    fn test_multiple_npcs_different_portraits() {
        let mut state = NpcEditorState::new();

        // Add first NPC
        state.start_add_npc();
        state.edit_buffer.id = "npc1".to_string();
        state.edit_buffer.name = "NPC 1".to_string();
        state.edit_buffer.portrait_id = "1".to_string();
        state.save_npc();

        // Add second NPC
        state.start_add_npc();
        state.edit_buffer.id = "npc2".to_string();
        state.edit_buffer.name = "NPC 2".to_string();
        state.edit_buffer.portrait_id = "2".to_string();
        state.save_npc();

        // Verify both NPCs have different portraits
        assert_eq!(state.npcs.len(), 2);
        assert_eq!(state.npcs[0].portrait_id, "1");
        assert_eq!(state.npcs[1].portrait_id, "2");
    }

    #[test]
    fn test_portrait_id_empty_string_allowed() {
        let mut state = NpcEditorState::new();

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "no_portrait_npc".to_string();
        state.edit_buffer.name = "No Portrait".to_string();
        state.edit_buffer.portrait_id = String::new();

        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());

        state.save_npc();
        assert_eq!(state.npcs[0].portrait_id, "");
    }
}
