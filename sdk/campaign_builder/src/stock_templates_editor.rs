// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Stock Templates Editor for Campaign Builder
//!
//! This module provides a visual editor for creating and managing
//! `MerchantStockTemplate` entries that are stored in `npc_stock_templates.ron`.
//!
//! Content authors can compose regular stock entries and magic-item rotation
//! pools without editing RON files by hand.
//!
//! # Features
//!
//! - List view with search filter and toolbar (New / Edit / Delete / Export)
//! - Edit / Add view with three groups:
//!   1. Identity (id, description)
//!   2. Regular stock entries (item, quantity, price override)
//!   3. Magic item rotation (slot count, refresh days, item pool)
//! - Load / save helpers for RON (de)serialisation
//! - `open_template_for_edit` helper consumed by `CampaignBuilderApp` for
//!   cross-tab navigation from the NPC editor
//!
//! # Architecture
//!
//! Follows the standard SDK editor pattern (mirrors `npc_editor.rs`):
//! - `StockTemplatesEditorState` — top-level state with `show()` entry point
//! - `StockTemplatesEditorMode` — `List` / `Add` / `Edit`
//! - `StockTemplateEditBuffer` — string-typed form fields, parsed on save
//! - `TemplateEntryBuffer` — per-entry row in the regular stock table
//!
//! All egui SDK rules apply:
//! - Every loop uses `push_id`
//! - Every `ScrollArea` has a unique `id_salt`
//! - Every `ComboBox` uses `from_id_salt`

use crate::ui_helpers::{
    show_standard_list_item, ItemAction, MetadataBadge, StandardListItemConfig,
};
use antares::domain::items::types::Item;
use antares::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Errors that can occur in the Stock Templates Editor.
#[derive(Debug, thiserror::Error)]
pub enum StockTemplatesEditorError {
    /// OS-level I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The file could not be parsed as RON.
    #[error("Parse error: {0}")]
    Parse(String),
    /// The data could not be serialised to RON.
    #[error("Serialisation error: {0}")]
    Serialization(String),
}

// ===== Mode =====

/// Editor mode for the stock templates editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StockTemplatesEditorMode {
    /// Viewing the list of templates
    #[default]
    List,
    /// Creating a new template
    Add,
    /// Editing an existing template
    Edit,
}

// ===== Entry buffer =====

/// Edit buffer for a single regular stock entry row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntryBuffer {
    /// Item ID (stringified `ItemId` / `u8`)
    pub item_id: String,
    /// Restock quantity (stringified `u8`)
    pub quantity: String,
    /// Optional price override (empty = use item `base_cost`; otherwise stringified `u32`)
    pub override_price: String,
}

impl Default for TemplateEntryBuffer {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            quantity: "1".to_string(),
            override_price: String::new(),
        }
    }
}

// ===== Main edit buffer =====

/// Form-field buffer for editing a `MerchantStockTemplate`
///
/// All fields are `String` (parsed on save) except the two `Vec` fields
/// which use structured sub-buffers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTemplateEditBuffer {
    /// Template ID (e.g. `"blacksmith_basic_stock"`)
    pub id: String,
    /// Human-readable description shown in the editor list
    pub description: String,
    /// Editable list of regular stock entries
    pub entries: Vec<TemplateEntryBuffer>,
    /// Item IDs in the magic-item rotation pool (one per ComboBox row)
    pub magic_item_pool: Vec<String>,
    /// Number of magic slots shown at once (stringified `u8`)
    pub magic_slot_count: String,
    /// Days between magic-slot refreshes (stringified `u32`)
    pub magic_refresh_days: String,
}

impl Default for StockTemplateEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            description: String::new(),
            entries: Vec::new(),
            magic_item_pool: Vec::new(),
            magic_slot_count: "0".to_string(),
            magic_refresh_days: "7".to_string(),
        }
    }
}

impl StockTemplateEditBuffer {
    /// Create a buffer pre-populated from an existing template (for editing)
    pub fn from_template(template: &MerchantStockTemplate) -> Self {
        let entries = template
            .entries
            .iter()
            .map(|e| TemplateEntryBuffer {
                item_id: e.item_id.to_string(),
                quantity: e.quantity.to_string(),
                override_price: e.override_price.map(|p| p.to_string()).unwrap_or_default(),
            })
            .collect();

        let magic_item_pool = template
            .magic_item_pool
            .iter()
            .map(|id| id.to_string())
            .collect();

        Self {
            id: template.id.clone(),
            description: template.description.clone(),
            entries,
            magic_item_pool,
            magic_slot_count: template.magic_slot_count.to_string(),
            magic_refresh_days: template.magic_refresh_days.to_string(),
        }
    }

    /// Validate the buffer and, on success, convert to a `MerchantStockTemplate`.
    ///
    /// Returns `Ok(template)` when valid; `Err(errors)` where `errors` is a
    /// non-empty list of human-readable messages.
    ///
    /// Warnings (non-fatal issues) are appended to the returned error vec but
    /// the function still returns `Ok` when only warnings are present — callers
    /// must check the `warnings` output parameter instead.
    pub fn to_template(
        &self,
        existing_ids: &[String],
        mode: StockTemplatesEditorMode,
        warnings: &mut Vec<String>,
    ) -> Result<MerchantStockTemplate, Vec<String>> {
        let mut errors: Vec<String> = Vec::new();
        warnings.clear();

        // --- id ---
        if self.id.is_empty() {
            errors.push("Template ID cannot be empty".to_string());
        } else if !self
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            errors.push("Template ID must match [a-z0-9_]+".to_string());
        } else if mode == StockTemplatesEditorMode::Add
            && existing_ids.iter().any(|e| e == &self.id)
        {
            errors.push(format!("Template ID '{}' already exists", self.id));
        }

        // --- entries ---
        let mut entries: Vec<TemplateStockEntry> = Vec::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let row = i + 1;

            let item_id = match entry.item_id.trim().parse::<u8>() {
                Ok(0) => {
                    errors.push(format!("Row {}: item_id must be > 0", row));
                    continue;
                }
                Ok(id) => id,
                Err(_) => {
                    errors.push(format!(
                        "Row {}: item_id '{}' is not a valid item ID (u8)",
                        row, entry.item_id
                    ));
                    continue;
                }
            };

            let quantity = match entry.quantity.trim().parse::<u8>() {
                Ok(0) => {
                    errors.push(format!("Row {}: quantity must be ≥ 1", row));
                    continue;
                }
                Ok(q) => q,
                Err(_) => {
                    errors.push(format!(
                        "Row {}: quantity '{}' is not a valid number (u8)",
                        row, entry.quantity
                    ));
                    continue;
                }
            };

            let override_price = if entry.override_price.trim().is_empty() {
                None
            } else {
                match entry.override_price.trim().parse::<u32>() {
                    Ok(p) => Some(p),
                    Err(_) => {
                        errors.push(format!(
                            "Row {}: price override '{}' is not a valid price (u32)",
                            row, entry.override_price
                        ));
                        continue;
                    }
                }
            };

            entries.push(TemplateStockEntry {
                item_id,
                quantity,
                override_price,
            });
        }

        // --- magic_slot_count ---
        let magic_slot_count = match self.magic_slot_count.trim().parse::<u8>() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!(
                    "Magic slot count '{}' is not a valid number (0-255)",
                    self.magic_slot_count
                ));
                0
            }
        };

        // --- magic_refresh_days ---
        let magic_refresh_days = match self.magic_refresh_days.trim().parse::<u32>() {
            Ok(0) => {
                warnings
                    .push("Magic refresh days was 0 — treated as 1 (minimum is 1 day)".to_string());
                1
            }
            Ok(v) => v,
            Err(_) => {
                errors.push(format!(
                    "Magic refresh days '{}' is not a valid number (u32, ≥ 1)",
                    self.magic_refresh_days
                ));
                7
            }
        };

        // --- magic_item_pool ---
        let mut magic_item_pool: Vec<u8> = Vec::new();
        for (i, pool_entry) in self.magic_item_pool.iter().enumerate() {
            match pool_entry.trim().parse::<u8>() {
                Ok(id) => magic_item_pool.push(id),
                Err(_) => {
                    errors.push(format!(
                        "Magic pool entry {}: '{}' is not a valid item ID (u8)",
                        i + 1,
                        pool_entry
                    ));
                }
            }
        }

        // --- warnings ---
        if magic_slot_count as usize > magic_item_pool.len() {
            warnings.push(format!(
                "magic_slot_count ({}) is greater than magic_item_pool size ({}) — some slots may be empty",
                magic_slot_count,
                magic_item_pool.len()
            ));
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(MerchantStockTemplate {
            id: self.id.clone(),
            entries,
            magic_item_pool,
            magic_slot_count,
            magic_refresh_days,
            description: self.description.clone(),
        })
    }
}

// ===== Editor state =====

/// Top-level state for the Stock Templates editor panel
///
/// Mirrors the structure of `NpcEditorState`.
#[derive(Clone, Serialize, Deserialize)]
pub struct StockTemplatesEditorState {
    /// All templates being managed
    pub templates: Vec<MerchantStockTemplate>,

    /// Currently selected template index (list view)
    pub selected_template: Option<usize>,

    /// Current editor mode
    pub mode: StockTemplatesEditorMode,

    /// Form buffer for Add / Edit operations
    pub edit_buffer: StockTemplateEditBuffer,

    /// Text filter for the list view search box
    pub search_filter: String,

    /// Whether there are unsaved in-memory changes
    pub has_unsaved_changes: bool,

    /// Validation errors for the current edit buffer
    pub validation_errors: Vec<String>,

    /// Non-fatal warnings for the current edit buffer
    pub validation_warnings: Vec<String>,

    /// Whether the delete-confirmation dialog is open
    pub show_delete_confirm: bool,

    /// Last campaign directory (used by load / save helpers; skipped on serde)
    #[serde(skip)]
    pub last_campaign_dir: Option<PathBuf>,

    /// Last templates filename (cached from `show()` call; skipped on serde)
    #[serde(skip)]
    pub last_templates_file: Option<String>,

    /// Whether the templates have been loaded for the current campaign.
    ///
    /// Set to `false` whenever the campaign changes (new / open) so that
    /// `show()` can trigger a lazy auto-load the first time the tab is
    /// rendered, providing a reliable fallback if the explicit
    /// `load_stock_templates()` call in `do_open_campaign` was unable to
    /// populate data (e.g. the file didn't exist yet at open time but was
    /// created later, or the status_message was clobbered before the user
    /// noticed).  Skipped on serde so it resets correctly on app restart.
    #[serde(skip)]
    pub needs_initial_load: bool,

    /// Whether `load_from_file` has successfully populated `templates` at
    /// least once since the last `reset_for_new_campaign` call.
    ///
    /// This flag is the primary guard in `do_save_campaign`: stock templates
    /// are only written back to disk if this flag is `true` **or** the user
    /// has made explicit in-editor changes (`has_unsaved_changes`).  Without
    /// the guard, the default empty `Vec` that exists before any file is
    /// loaded would overwrite a valid on-disk file with `[]` the first time
    /// the user triggers a campaign save — which is the root cause of the
    /// repeated stock-template wipe bug.
    ///
    /// Reset to `false` by `reset_for_new_campaign`; set to `true` by a
    /// successful `load_from_file`.  Skipped on serde so it always starts
    /// as `false` on app restart, forcing a reload before any save.
    #[serde(skip)]
    pub loaded_from_file: bool,

    /// Item database snapshot for name lookups. Refreshed from the caller on
    /// every `show()` call (skipped on serde).
    #[serde(skip)]
    pub available_items: Vec<Item>,
}

impl Default for StockTemplatesEditorState {
    fn default() -> Self {
        Self {
            templates: Vec::new(),
            selected_template: None,
            mode: StockTemplatesEditorMode::List,
            edit_buffer: StockTemplateEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            show_delete_confirm: false,
            last_campaign_dir: None,
            last_templates_file: None,
            needs_initial_load: true,
            loaded_from_file: false,
            available_items: Vec::new(),
        }
    }
}

impl StockTemplatesEditorState {
    /// Creates a new default stock templates editor state
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------ show

    /// Render the editor panel.
    ///
    /// Returns `true` when the in-memory template list has changed (i.e. the
    /// caller should mark unsaved changes and sync its own copy of the list).
    ///
    /// # Arguments
    ///
    /// * `ui` — mutable egui Ui reference
    /// * `available_items` — current campaign item list for ComboBox population
    /// * `campaign_dir` — campaign root directory (used for load / save)
    /// * `templates_file` — relative path to `npc_stock_templates.ron`
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        available_items: &[Item],
        campaign_dir: Option<&PathBuf>,
        templates_file: &str,
    ) -> bool {
        // Refresh the item snapshot every frame so the editor always sees
        // the latest items without requiring an explicit refresh call.
        self.available_items = available_items.to_vec();

        // Cache campaign dir / filename for use in load/save helpers
        if let Some(dir) = campaign_dir {
            self.last_campaign_dir = Some(dir.clone());
        }
        self.last_templates_file = Some(templates_file.to_string());

        // Auto-load on first show for the current campaign.
        //
        // `needs_initial_load` is set to `true` by `reset_for_new_campaign()`
        // (called from `do_new_campaign` and at the end of `do_open_campaign`
        // after data is populated).  The explicit `load_stock_templates()` call
        // in `do_open_campaign` is the primary load path; this guard is a
        // reliable fallback for the case where the user navigates to this tab
        // before that call has had a chance to run, or the file was absent at
        // open time but has since appeared on disk.
        let mut auto_loaded = false;
        if self.needs_initial_load {
            if let Some(dir) = campaign_dir {
                let path = dir.join(templates_file);
                if path.exists() {
                    match self.load_from_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                            // Signal the caller to sync its mirror list so other
                            // editors (e.g. NPC editor) see the freshly loaded
                            // templates immediately, even if the StockTemplates
                            // tab has never been visited.
                            auto_loaded = true;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Auto-load failed: {}", e)];
                        }
                    }
                }
                // Mark as loaded regardless of whether the file existed so we
                // don't retry every frame.
                self.needs_initial_load = false;
            }
        }

        let inner_needs_save = match self.mode {
            StockTemplatesEditorMode::List => self.show_list_view(ui, campaign_dir, templates_file),
            StockTemplatesEditorMode::Add | StockTemplatesEditorMode::Edit => {
                self.show_edit_view(ui, campaign_dir, templates_file)
            }
        };

        auto_loaded || inner_needs_save
    }

    /// Reset editor state for a new or freshly-opened campaign.
    ///
    /// Clears the template list, selection, edit buffer, and all transient UI
    /// state, then sets `needs_initial_load = true` so that `show()` will
    /// perform an auto-load the first time the Stock Templates tab is rendered.
    ///
    /// Call this from both `do_new_campaign` and at the end of
    /// `do_open_campaign` (after the explicit `load_stock_templates()` call,
    /// so the flag is only set when needed).
    pub fn reset_for_new_campaign(&mut self) {
        self.templates.clear();
        self.selected_template = None;
        self.mode = StockTemplatesEditorMode::List;
        self.edit_buffer = StockTemplateEditBuffer::default();
        self.search_filter.clear();
        self.has_unsaved_changes = false;
        self.validation_errors.clear();
        self.validation_warnings.clear();
        self.show_delete_confirm = false;
        self.needs_initial_load = true;
        // Clear the load-guard so a save cannot overwrite the file with an
        // empty list before the new campaign's templates have been read.
        self.loaded_from_file = false;
        // Preserve last_campaign_dir / last_templates_file / available_items —
        // they are refreshed from the caller on the next show() call anyway.
    }

    // ------------------------------------------------------------------ list

    fn show_list_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        templates_file: &str,
    ) -> bool {
        let mut needs_save = false;
        let mut action_requested: Option<(usize, ItemAction)> = None;

        ui.heading("📦 Stock Templates");
        ui.separator();

        // --- toolbar ---
        ui.horizontal(|ui| {
            if ui.button("➕ New").clicked() {
                self.start_add_template();
            }

            let has_selection = self.selected_template.is_some();

            if ui
                .add_enabled(has_selection, egui::Button::new("✏ Edit"))
                .clicked()
            {
                if let Some(idx) = self.selected_template {
                    self.start_edit_template(idx);
                }
            }

            if ui
                .add_enabled(has_selection, egui::Button::new("🗑 Delete"))
                .clicked()
            {
                self.show_delete_confirm = true;
            }

            ui.separator();

            if ui.button("💾 Save to File").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(templates_file);
                    match self.save_to_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Save failed: {}", e)];
                        }
                    }
                }
            }

            if ui.button("📂 Load from File").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(templates_file);
                    match self.load_from_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                            needs_save = true;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Load failed: {}", e)];
                        }
                    }
                }
            }
        });

        ui.separator();

        // --- search ---
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_filter)
                    .id_salt("stmpl_search")
                    .hint_text("Filter by ID…"),
            );
            if ui.button("✕").on_hover_text("Clear search").clicked() {
                self.search_filter.clear();
            }
        });

        ui.separator();

        // --- unsaved banner ---
        if self.has_unsaved_changes {
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),
                "⚠ Unsaved changes — use 'Save to File' to persist",
            );
            ui.add_space(4.0);
        }

        // --- delete confirmation dialog ---
        if self.show_delete_confirm {
            if let Some(idx) = self.selected_template {
                let tmpl_id = self
                    .templates
                    .get(idx)
                    .map(|t| t.id.clone())
                    .unwrap_or_default();

                egui::Window::new("Confirm Delete")
                    .collapsible(false)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Delete template '{}'?", tmpl_id));
                        ui.label("This cannot be undone.");
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("🗑 Delete").clicked() {
                                self.templates.remove(idx);
                                self.selected_template = None;
                                self.has_unsaved_changes = true;
                                needs_save = true;
                                self.show_delete_confirm = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirm = false;
                            }
                        });
                    });
            } else {
                self.show_delete_confirm = false;
            }
        }

        // --- two-column layout: list + preview ---
        let avail = ui.available_width();
        let list_width = (avail * 0.35).clamp(180.0, 300.0);

        let filter_lower = self.search_filter.to_lowercase();
        let filtered_templates: Vec<(usize, MerchantStockTemplate)> = self
            .templates
            .iter()
            .enumerate()
            .filter(|(_, tmpl)| {
                filter_lower.is_empty() || tmpl.id.to_lowercase().contains(&filter_lower)
            })
            .map(|(idx, tmpl)| (idx, tmpl.clone()))
            .collect();

        let selected = self.selected_template;
        let mut new_selection = selected;

        ui.horizontal_top(|ui| {
            // Left: scrollable list
            ui.vertical(|ui| {
                ui.set_width(list_width);

                egui::ScrollArea::vertical()
                    .id_salt("stock_tmpl_list_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, tmpl) in &filtered_templates {
                            ui.push_id(format!("stmpl_row_{}_{}", idx, tmpl.id), |ui| {
                                let badges = vec![
                                    MetadataBadge::new(format!("Entries:{}", tmpl.entries.len()))
                                        .with_color(egui::Color32::from_rgb(120, 170, 220))
                                        .with_tooltip("Number of regular stock entries"),
                                    MetadataBadge::new(format!(
                                        "Magic:{}",
                                        tmpl.magic_item_pool.len()
                                    ))
                                    .with_color(egui::Color32::from_rgb(180, 110, 210))
                                    .with_tooltip("Magic-item pool size"),
                                    MetadataBadge::new(format!(
                                        "Slots:{} @{}d",
                                        tmpl.magic_slot_count, tmpl.magic_refresh_days
                                    ))
                                    .with_color(egui::Color32::from_rgb(180, 180, 120))
                                    .with_tooltip("Magic slots and refresh cadence"),
                                ];

                                let config = StandardListItemConfig::new(&tmpl.id)
                                    .with_badges(badges)
                                    .selected(selected == Some(*idx));
                                let (clicked, action) = show_standard_list_item(ui, config);
                                if clicked {
                                    new_selection = Some(*idx);
                                    ui.ctx().request_repaint();
                                }
                                if action != ItemAction::None {
                                    action_requested = Some((*idx, action));
                                }
                            });
                        }

                        if filtered_templates.is_empty() {
                            let empty_text = if self.templates.is_empty() {
                                "(no templates — click ➕ New to add one)"
                            } else {
                                "(no templates match the search filter)"
                            };
                            ui.label(egui::RichText::new(empty_text).weak());
                        }
                    });
            });

            // Right: read-only preview of selected template
            ui.separator();
            ui.vertical(|ui| {
                if let Some(idx) = self.selected_template {
                    if let Some(tmpl) = self.templates.get(idx) {
                        self.show_preview(ui, tmpl);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Select a template on the left to preview it.").weak(),
                    );
                }
            });
        });

        self.selected_template = new_selection;

        if let Some((idx, action)) = action_requested {
            self.selected_template = Some(idx);
            match action {
                ItemAction::Edit => {
                    self.start_edit_template(idx);
                }
                ItemAction::Delete => {
                    self.show_delete_confirm = true;
                }
                ItemAction::Duplicate => {
                    if idx < self.templates.len() {
                        let mut duplicated = self.templates[idx].clone();
                        let new_id = self.next_duplicate_template_id(&duplicated.id);
                        duplicated.id = new_id.clone();
                        self.templates.push(duplicated);
                        self.selected_template = Some(self.templates.len() - 1);
                        self.has_unsaved_changes = true;
                        needs_save = true;
                        self.validation_warnings =
                            vec![format!("Duplicated template as '{}'", new_id)];
                    }
                }
                ItemAction::Export => {
                    if idx < self.templates.len() {
                        match ron::ser::to_string_pretty(
                            &self.templates[idx],
                            ron::ser::PrettyConfig::default(),
                        ) {
                            Ok(ron_text) => {
                                ui.ctx().copy_text(ron_text);
                                self.validation_warnings =
                                    vec!["Copied template RON to clipboard".to_string()];
                            }
                            Err(e) => {
                                self.validation_errors = vec![format!("Export failed: {}", e)];
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }

        // surface any persistent errors
        if !self.validation_errors.is_empty() {
            ui.separator();
            ui.group(|ui| {
                ui.heading("⚠️ Errors");
                for e in &self.validation_errors {
                    ui.colored_label(egui::Color32::RED, e);
                }
            });
        }

        needs_save
    }

    /// Render a read-only summary of a template (right panel in list view)
    fn show_preview(&self, ui: &mut egui::Ui, tmpl: &MerchantStockTemplate) {
        ui.heading(format!("📦 {}", tmpl.id));
        ui.separator();

        egui::Grid::new("stock_template_display_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Description:").strong());
                if tmpl.description.is_empty() {
                    ui.label(egui::RichText::new("No description.").italics().weak());
                } else {
                    ui.label(&tmpl.description);
                }
                ui.end_row();
            });
        ui.separator();

        ui.label(format!("Regular stock entries: {}", tmpl.entries.len()));
        ui.label(format!(
            "Magic item pool: {} items",
            tmpl.magic_item_pool.len()
        ));
        ui.label(format!("Magic slots shown: {}", tmpl.magic_slot_count));
        ui.label(format!(
            "Magic refresh every {} day(s)",
            tmpl.magic_refresh_days
        ));

        if !tmpl.entries.is_empty() {
            ui.separator();
            ui.label("Entries:");
            egui::ScrollArea::vertical()
                .id_salt("stmpl_preview_entries_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, entry) in tmpl.entries.iter().enumerate() {
                        ui.push_id(format!("stmpl_prev_entry_{}", i), |ui| {
                            let item_name = self
                                .available_items
                                .iter()
                                .find(|it| it.id == entry.item_id)
                                .map(|it| it.name.as_str())
                                .unwrap_or("(unknown)");

                            let price_str = match entry.override_price {
                                Some(p) => format!("  price: {}", p),
                                None => String::new(),
                            };

                            ui.label(format!(
                                "  {}. {} — {} × {}{}",
                                i + 1,
                                entry.item_id,
                                item_name,
                                entry.quantity,
                                price_str
                            ));
                        });
                    }
                });
        }
    }

    // ------------------------------------------------------------------ edit

    fn show_edit_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        templates_file: &str,
    ) -> bool {
        let mut needs_save = false;
        let is_add = self.mode == StockTemplatesEditorMode::Add;

        ui.heading(if is_add {
            "➕ Add New Stock Template"
        } else {
            "✏ Edit Stock Template"
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("stock_tmpl_edit_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ── Group 1: Identity ──
                ui.group(|ui| {
                    ui.heading("Identity");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.id)
                                .id_salt("stmpl_edit_id")
                                .hint_text("e.g. blacksmith_basic_stock"),
                        );
                    });
                    ui.label(egui::RichText::new("Must match [a-z0-9_]+").small().weak());

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.edit_buffer.description)
                            .desired_rows(2)
                            .id_salt("stmpl_edit_description")
                            .hint_text("Internal notes for campaign authors"),
                    );
                });

                ui.add_space(8.0);

                // ── Group 2: Regular Stock Entries ──
                ui.group(|ui| {
                    ui.heading("Regular Stock Entries");

                    // Column headers
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Item").strong());
                        ui.add_space(120.0);
                        ui.label(egui::RichText::new("Qty").strong());
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new("Price Override").strong());
                    });
                    ui.separator();

                    // Collect mutations outside the iterator to avoid borrow issues
                    let mut remove_idx: Option<usize> = None;
                    let mut swap_up: Option<usize> = None;
                    let mut swap_down: Option<usize> = None;

                    let entry_count = self.edit_buffer.entries.len();

                    egui::ScrollArea::vertical()
                        .id_salt("stmpl_entries_scroll")
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for i in 0..entry_count {
                                ui.push_id(format!("stmpl_entry_{}", i), |ui| {
                                    ui.horizontal(|ui| {
                                        // Item ComboBox
                                        let selected_text = {
                                            let raw = self.edit_buffer.entries[i].item_id.trim();
                                            if raw.is_empty() {
                                                "Select item…".to_string()
                                            } else {
                                                // Show id + name if we can resolve it
                                                match raw.parse::<u8>() {
                                                    Ok(id) => self
                                                        .available_items
                                                        .iter()
                                                        .find(|it| it.id == id)
                                                        .map(|it| {
                                                            format!("{} - {}", it.id, it.name)
                                                        })
                                                        .unwrap_or_else(|| raw.to_string()),
                                                    Err(_) => raw.to_string(),
                                                }
                                            }
                                        };

                                        egui::ComboBox::from_id_salt(format!(
                                            "stmpl_item_sel_{}",
                                            i
                                        ))
                                        .width(200.0)
                                        .selected_text(selected_text)
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for item in &self.available_items {
                                                    ui.push_id(item.id, |ui| {
                                                        let label =
                                                            format!("{} - {}", item.id, item.name);
                                                        let selected = self.edit_buffer.entries[i]
                                                            .item_id
                                                            == item.id.to_string();
                                                        if ui
                                                            .selectable_label(selected, &label)
                                                            .clicked()
                                                        {
                                                            self.edit_buffer.entries[i].item_id =
                                                                item.id.to_string();
                                                        }
                                                    });
                                                }
                                            },
                                        );

                                        // Quantity
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_buffer.entries[i].quantity,
                                            )
                                            .desired_width(40.0),
                                        );

                                        // Price override
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_buffer.entries[i].override_price,
                                            )
                                            .desired_width(70.0)
                                            .hint_text("default"),
                                        );

                                        // Reorder buttons
                                        ui.push_id(format!("stmpl_move_{}", i), |ui| {
                                            if ui
                                                .add_enabled(i > 0, egui::Button::new("↑").small())
                                                .clicked()
                                            {
                                                swap_up = Some(i);
                                            }
                                            if ui
                                                .add_enabled(
                                                    i + 1 < entry_count,
                                                    egui::Button::new("↓").small(),
                                                )
                                                .clicked()
                                            {
                                                swap_down = Some(i);
                                            }
                                        });

                                        // Remove button
                                        if ui.button("✕").on_hover_text("Remove entry").clicked()
                                        {
                                            remove_idx = Some(i);
                                        }
                                    });
                                });
                            }

                            if entry_count == 0 {
                                ui.label(
                                    egui::RichText::new("  (no entries — click 'Add Entry' below)")
                                        .weak(),
                                );
                            }
                        });

                    // Apply mutations after rendering to avoid borrow issues
                    if let Some(idx) = remove_idx {
                        self.edit_buffer.entries.remove(idx);
                    }
                    if let Some(idx) = swap_up {
                        if idx > 0 {
                            self.edit_buffer.entries.swap(idx, idx - 1);
                        }
                    }
                    if let Some(idx) = swap_down {
                        if idx + 1 < self.edit_buffer.entries.len() {
                            self.edit_buffer.entries.swap(idx, idx + 1);
                        }
                    }

                    ui.separator();
                    if ui.button("➕ Add Entry").clicked() {
                        self.edit_buffer
                            .entries
                            .push(TemplateEntryBuffer::default());
                    }
                });

                ui.add_space(8.0);

                // ── Group 3: Magic Item Rotation ──
                ui.group(|ui| {
                    ui.heading("✨ Magic Item Rotation");

                    ui.horizontal(|ui| {
                        ui.label("Magic slots shown at once:");
                        let mut count_val: u8 =
                            self.edit_buffer.magic_slot_count.parse().unwrap_or(0);
                        if ui
                            .add(egui::DragValue::new(&mut count_val).range(0..=255))
                            .changed()
                        {
                            self.edit_buffer.magic_slot_count = count_val.to_string();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Refresh every (days):");
                        let mut days_val: u32 =
                            self.edit_buffer.magic_refresh_days.parse().unwrap_or(7);
                        if ui
                            .add(egui::DragValue::new(&mut days_val).range(1..=365))
                            .changed()
                        {
                            self.edit_buffer.magic_refresh_days = days_val.to_string();
                        }
                    });

                    ui.separator();
                    ui.label("Magic Item Pool:");
                    ui.label(
                        egui::RichText::new(
                            "Items listed here are eligible to appear in the rotating slots.",
                        )
                        .small()
                        .weak(),
                    );

                    let mut remove_pool_idx: Option<usize> = None;
                    let pool_count = self.edit_buffer.magic_item_pool.len();

                    egui::ScrollArea::vertical()
                        .id_salt("stmpl_magic_pool_scroll")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for i in 0..pool_count {
                                ui.push_id(format!("stmpl_pool_{}", i), |ui| {
                                    ui.horizontal(|ui| {
                                        // Determine selected text for the pool entry ComboBox
                                        let selected_text = {
                                            let raw = self.edit_buffer.magic_item_pool[i].trim();
                                            if raw.is_empty() {
                                                "Select magic item…".to_string()
                                            } else {
                                                match raw.parse::<u8>() {
                                                    Ok(id) => self
                                                        .available_items
                                                        .iter()
                                                        .find(|it| it.id == id)
                                                        .map(|it| {
                                                            format!("{} - {}", it.id, it.name)
                                                        })
                                                        .unwrap_or_else(|| raw.to_string()),
                                                    Err(_) => raw.to_string(),
                                                }
                                            }
                                        };

                                        // Filter to magical items only
                                        let magical_items: Vec<&Item> = self
                                            .available_items
                                            .iter()
                                            .filter(|it| it.is_magical())
                                            .collect();

                                        egui::ComboBox::from_id_salt(format!(
                                            "stmpl_pool_item_{}",
                                            i
                                        ))
                                        .width(220.0)
                                        .selected_text(selected_text)
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for item in &magical_items {
                                                    ui.push_id(item.id, |ui| {
                                                        let label =
                                                            format!("{} - {}", item.id, item.name);
                                                        let selected =
                                                            self.edit_buffer.magic_item_pool[i]
                                                                == item.id.to_string();
                                                        if ui
                                                            .selectable_label(selected, &label)
                                                            .clicked()
                                                        {
                                                            self.edit_buffer.magic_item_pool[i] =
                                                                item.id.to_string();
                                                        }
                                                    });
                                                }
                                                if magical_items.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new(
                                                            "(no magical items in campaign)",
                                                        )
                                                        .weak(),
                                                    );
                                                }
                                            },
                                        );

                                        if ui
                                            .button("✕")
                                            .on_hover_text("Remove from pool")
                                            .clicked()
                                        {
                                            remove_pool_idx = Some(i);
                                        }
                                    });
                                });
                            }

                            if pool_count == 0 {
                                ui.label(
                                    egui::RichText::new(
                                        "  (empty pool — click 'Add to Pool' below)",
                                    )
                                    .weak(),
                                );
                            }
                        });

                    if let Some(idx) = remove_pool_idx {
                        self.edit_buffer.magic_item_pool.remove(idx);
                    }

                    ui.separator();
                    if ui.button("➕ Add to Pool").clicked() {
                        self.edit_buffer.magic_item_pool.push(String::new());
                    }
                });

                ui.add_space(8.0);

                // ── Validation errors / warnings ──
                if !self.validation_errors.is_empty() {
                    ui.group(|ui| {
                        ui.heading("⚠️ Validation Errors");
                        for err in &self.validation_errors {
                            ui.colored_label(egui::Color32::RED, err);
                        }
                    });
                    ui.add_space(4.0);
                }
                if !self.validation_warnings.is_empty() {
                    ui.group(|ui| {
                        ui.heading("ℹ️ Warnings");
                        for w in &self.validation_warnings {
                            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), w);
                        }
                    });
                    ui.add_space(4.0);
                }

                // ── Action buttons ──
                ui.horizontal(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = StockTemplatesEditorMode::List;
                        self.validation_errors.clear();
                        self.validation_warnings.clear();
                    }

                    if ui.button("💾 Save").clicked() {
                        let existing_ids: Vec<String> =
                            self.templates.iter().map(|t| t.id.clone()).collect();

                        let mut warnings = Vec::new();
                        match self
                            .edit_buffer
                            .to_template(&existing_ids, self.mode, &mut warnings)
                        {
                            Ok(template) => {
                                self.validation_errors.clear();
                                self.validation_warnings = warnings;

                                match self.mode {
                                    StockTemplatesEditorMode::Add => {
                                        self.templates.push(template);
                                        self.selected_template =
                                            Some(self.templates.len().saturating_sub(1));
                                    }
                                    StockTemplatesEditorMode::Edit => {
                                        if let Some(idx) = self.selected_template {
                                            if idx < self.templates.len() {
                                                self.templates[idx] = template;
                                            }
                                        }
                                    }
                                    StockTemplatesEditorMode::List => {}
                                }

                                self.has_unsaved_changes = true;
                                needs_save = true;

                                // Attempt immediate file persistence
                                if let Some(dir) = campaign_dir {
                                    let path = dir.join(templates_file);
                                    match self.save_to_file(&path) {
                                        Ok(()) => {
                                            self.has_unsaved_changes = false;
                                        }
                                        Err(e) => {
                                            self.validation_errors =
                                                vec![format!("Save failed: {}", e)];
                                        }
                                    }
                                }

                                self.mode = StockTemplatesEditorMode::List;
                            }
                            Err(errs) => {
                                self.validation_errors = errs;
                                self.validation_warnings = warnings;
                            }
                        }
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = StockTemplatesEditorMode::List;
                        self.edit_buffer = StockTemplateEditBuffer::default();
                        self.validation_errors.clear();
                        self.validation_warnings.clear();
                    }
                });
            });

        needs_save
    }

    // ------------------------------------------------------------------ helpers

    /// Transition into Add mode with a blank buffer
    fn start_add_template(&mut self) {
        self.mode = StockTemplatesEditorMode::Add;
        self.edit_buffer = StockTemplateEditBuffer::default();
        self.validation_errors.clear();
        self.validation_warnings.clear();
    }

    /// Transition into Edit mode for the template at `idx`
    fn start_edit_template(&mut self, idx: usize) {
        if let Some(tmpl) = self.templates.get(idx) {
            self.edit_buffer = StockTemplateEditBuffer::from_template(tmpl);
            self.selected_template = Some(idx);
            self.mode = StockTemplatesEditorMode::Edit;
            self.validation_errors.clear();
            self.validation_warnings.clear();
        }
    }

    /// Returns a unique template ID suitable for a duplicated template.
    fn next_duplicate_template_id(&self, source_id: &str) -> String {
        let mut candidate = format!("{}_copy", source_id);
        let mut suffix = 2u32;
        while self.templates.iter().any(|t| t.id == candidate) {
            candidate = format!("{}_copy_{}", source_id, suffix);
            suffix += 1;
        }
        candidate
    }

    /// Navigate directly to edit mode for a template identified by `id`.
    ///
    /// Called by `CampaignBuilderApp` when the user clicks "✏ Edit template"
    /// in the NPC editor.  Does nothing if the ID is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::MerchantStockTemplate;
    /// use campaign_builder::stock_templates_editor::{
    ///     StockTemplatesEditorMode, StockTemplatesEditorState,
    /// };
    ///
    /// let mut state = StockTemplatesEditorState::default();
    /// state.templates.push(MerchantStockTemplate {
    ///     id: "foo".to_string(),
    ///     entries: vec![],
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
    ///     description: String::new(),
    /// });
    ///
    /// state.open_template_for_edit("foo");
    /// assert_eq!(state.mode, StockTemplatesEditorMode::Edit);
    /// assert_eq!(state.edit_buffer.id, "foo");
    /// ```
    pub fn open_template_for_edit(&mut self, id: &str) {
        if let Some(idx) = self.templates.iter().position(|t| t.id == id) {
            self.selected_template = Some(idx);
            self.edit_buffer = StockTemplateEditBuffer::from_template(&self.templates[idx]);
            self.mode = StockTemplatesEditorMode::Edit;
            self.validation_errors.clear();
            self.validation_warnings.clear();
        }
    }

    // ------------------------------------------------------------------ I/O

    /// Load templates from a RON file.
    ///
    /// Replaces the current `templates` list on success.
    /// Returns an `Err(String)` with a human-readable message on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::stock_templates_editor::StockTemplatesEditorState;
    ///
    /// let mut state = StockTemplatesEditorState::default();
    /// // Fails gracefully when the file is missing
    /// let result = state.load_from_file(std::path::Path::new("/nonexistent/path.ron"));
    /// assert!(result.is_err());
    /// ```
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), StockTemplatesEditorError> {
        let contents = std::fs::read_to_string(path)?;

        let templates: Vec<MerchantStockTemplate> = ron::from_str(&contents)
            .map_err(|e| StockTemplatesEditorError::Parse(e.to_string()))?;

        self.templates = templates;
        self.selected_template = None;
        // Mark that in-memory templates are now backed by the on-disk file.
        // `do_save_campaign` uses this flag to decide whether it is safe to
        // write back — preventing an empty default Vec from overwriting a
        // valid file before any load has occurred.
        self.loaded_from_file = true;
        Ok(())
    }

    /// Serialise the current `templates` list to a RON file.
    ///
    /// Returns an `Err(String)` with a human-readable message on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::stock_templates_editor::StockTemplatesEditorState;
    ///
    /// let state = StockTemplatesEditorState::default();
    /// // No templates to save; creates an empty RON array.
    /// let result = state.save_to_file(std::path::Path::new("/tmp/npc_stock_templates.ron"));
    /// // Depends on filesystem access; just calling to show the API.
    /// ```
    pub fn save_to_file(&self, path: &Path) -> Result<(), StockTemplatesEditorError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(6);

        let ron_string = ron::ser::to_string_pretty(&self.templates, config)
            .map_err(|e| StockTemplatesEditorError::Serialization(e.to_string()))?;

        std::fs::write(path, ron_string)?;

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_template(id: &str) -> MerchantStockTemplate {
        MerchantStockTemplate {
            id: id.to_string(),
            entries: vec![
                TemplateStockEntry {
                    item_id: 1,
                    quantity: 3,
                    override_price: None,
                },
                TemplateStockEntry {
                    item_id: 2,
                    quantity: 1,
                    override_price: Some(500),
                },
            ],
            magic_item_pool: vec![10, 11],
            magic_slot_count: 2,
            magic_refresh_days: 7,
            description: String::new(),
        }
    }

    // ── StockTemplatesEditorState ────────────────────────────────────────────

    #[test]
    fn test_stock_templates_editor_state_default() {
        let state = StockTemplatesEditorState::default();
        assert!(state.templates.is_empty());
        assert_eq!(state.mode, StockTemplatesEditorMode::List);
        assert!(state.selected_template.is_none());
        assert!(state.validation_errors.is_empty());
        assert!(!state.has_unsaved_changes);
    }

    // ── StockTemplateEditBuffer ──────────────────────────────────────────────

    #[test]
    fn test_stock_template_edit_buffer_default() {
        let buf = StockTemplateEditBuffer::default();
        assert!(buf.id.is_empty());
        assert!(buf.description.is_empty());
        assert!(buf.entries.is_empty());
        assert!(buf.magic_item_pool.is_empty());
        assert_eq!(buf.magic_slot_count, "0");
        assert_eq!(buf.magic_refresh_days, "7");
    }

    // ── from_template / to_template round-trip ───────────────────────────────

    #[test]
    fn test_from_template_round_trips() {
        let mut original = make_template("round_trip_test");
        original.description = "Test round-trip shop".to_string();
        let buf = StockTemplateEditBuffer::from_template(&original);
        let existing: Vec<String> = vec![];
        let mut warnings = Vec::new();
        let result = buf.to_template(&existing, StockTemplatesEditorMode::Add, &mut warnings);
        assert!(
            result.is_ok(),
            "round-trip failed: {:?}",
            result.unwrap_err()
        );
        let restored = result.unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.entries, original.entries);
        assert_eq!(restored.magic_item_pool, original.magic_item_pool);
        assert_eq!(restored.magic_slot_count, original.magic_slot_count);
        assert_eq!(restored.magic_refresh_days, original.magic_refresh_days);
        assert_eq!(restored.description, original.description);
    }

    // ── validation: empty id ─────────────────────────────────────────────────

    #[test]
    fn test_to_template_validates_empty_id() {
        let buf = StockTemplateEditBuffer {
            id: String::new(),
            ..StockTemplateEditBuffer::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("ID cannot be empty")),
            "expected empty-id error, got: {:?}",
            errs
        );
    }

    // ── validation: invalid item_id in entry ─────────────────────────────────

    #[test]
    fn test_to_template_validates_invalid_item_id() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            entries: vec![TemplateEntryBuffer {
                item_id: "not_a_number".to_string(),
                quantity: "1".to_string(),
                override_price: String::new(),
            }],
            ..Default::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("item_id")),
            "expected item_id error, got: {:?}",
            errs
        );
    }

    // ── validation: zero quantity ────────────────────────────────────────────

    #[test]
    fn test_to_template_validates_zero_quantity() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            entries: vec![TemplateEntryBuffer {
                item_id: "1".to_string(),
                quantity: "0".to_string(),
                override_price: String::new(),
            }],
            ..Default::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("quantity")),
            "expected quantity error, got: {:?}",
            errs
        );
    }

    // ── validation: invalid override_price ──────────────────────────────────

    #[test]
    fn test_to_template_validates_invalid_override_price() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            entries: vec![TemplateEntryBuffer {
                item_id: "1".to_string(),
                quantity: "3".to_string(),
                override_price: "not_a_price".to_string(),
            }],
            ..Default::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("price")),
            "expected price error, got: {:?}",
            errs
        );
    }

    // ── validation: magic_slot_count > pool length (warning) ─────────────────

    #[test]
    fn test_to_template_validates_magic_slot_count_exceeds_pool() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            magic_slot_count: "5".to_string(),
            magic_refresh_days: "7".to_string(),
            magic_item_pool: vec!["1".to_string(), "2".to_string()],
            ..Default::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        // Should succeed (warning, not error)
        assert!(
            result.is_ok(),
            "expected Ok, got: {:?}",
            result.unwrap_err()
        );
        assert!(
            warnings.iter().any(|w| w.contains("magic_slot_count")),
            "expected magic_slot_count warning, got: {:?}",
            warnings
        );
    }

    // ── validation: magic_refresh_days == 0 (treated as 1 with warning) ──────

    #[test]
    fn test_to_template_validates_magic_refresh_days_zero() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            magic_refresh_days: "0".to_string(),
            ..Default::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(
            result.is_ok(),
            "expected Ok, got: {:?}",
            result.unwrap_err()
        );
        let tmpl = result.unwrap();
        assert_eq!(tmpl.magic_refresh_days, 1, "0 should be clamped to 1");
        assert!(
            warnings.iter().any(|w| w.contains("refresh days")),
            "expected refresh-days warning, got: {:?}",
            warnings
        );
    }

    // ── add entry appends to buffer ──────────────────────────────────────────

    #[test]
    fn test_add_entry_appends_to_buffer() {
        let mut buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            ..Default::default()
        };
        buf.entries.push(TemplateEntryBuffer {
            item_id: "1".to_string(),
            quantity: "5".to_string(),
            override_price: String::new(),
        });
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(
            result.is_ok(),
            "expected Ok, got: {:?}",
            result.unwrap_err()
        );
        let tmpl = result.unwrap();
        assert_eq!(tmpl.entries.len(), 1);
        assert_eq!(tmpl.entries[0].item_id, 1);
        assert_eq!(tmpl.entries[0].quantity, 5);
    }

    // ── remove entry shrinks list ────────────────────────────────────────────

    #[test]
    fn test_remove_entry_shrinks_list() {
        let mut buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            entries: vec![
                TemplateEntryBuffer {
                    item_id: "1".to_string(),
                    quantity: "1".to_string(),
                    override_price: String::new(),
                },
                TemplateEntryBuffer {
                    item_id: "2".to_string(),
                    quantity: "2".to_string(),
                    override_price: String::new(),
                },
                TemplateEntryBuffer {
                    item_id: "3".to_string(),
                    quantity: "3".to_string(),
                    override_price: String::new(),
                },
            ],
            ..Default::default()
        };
        buf.entries.remove(1);
        assert_eq!(buf.entries.len(), 2);
        assert_eq!(buf.entries[0].item_id, "1");
        assert_eq!(buf.entries[1].item_id, "3");
    }

    // ── reorder entry up ─────────────────────────────────────────────────────

    #[test]
    fn test_reorder_entry_up() {
        let mut buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            entries: vec![
                TemplateEntryBuffer {
                    item_id: "10".to_string(),
                    quantity: "1".to_string(),
                    override_price: String::new(),
                },
                TemplateEntryBuffer {
                    item_id: "20".to_string(),
                    quantity: "1".to_string(),
                    override_price: String::new(),
                },
            ],
            ..Default::default()
        };
        buf.entries.swap(1, 0);
        assert_eq!(buf.entries[0].item_id, "20");
        assert_eq!(buf.entries[1].item_id, "10");
    }

    // ── load/save round-trip ─────────────────────────────────────────────────

    #[test]
    fn test_load_from_file_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("npc_stock_templates.ron");

        let original = make_template("persist_test");

        let mut state = StockTemplatesEditorState::default();
        state.templates.push(original.clone());
        state.save_to_file(&path).expect("save should succeed");

        let mut state2 = StockTemplatesEditorState::default();
        state2.load_from_file(&path).expect("load should succeed");

        assert_eq!(state2.templates.len(), 1);
        assert_eq!(state2.templates[0].id, original.id);
        assert_eq!(state2.templates[0].entries, original.entries);
        assert_eq!(
            state2.templates[0].magic_item_pool,
            original.magic_item_pool
        );
    }

    // ── missing path returns error ───────────────────────────────────────────

    #[test]
    fn test_load_from_file_missing_path_returns_error() {
        let mut state = StockTemplatesEditorState::default();
        let result = state.load_from_file(Path::new("/nonexistent/does_not_exist.ron"));
        assert!(result.is_err(), "expected Err for missing file");
    }

    // ── open_template_for_edit sets Edit mode ────────────────────────────────

    #[test]
    fn test_open_template_for_edit_sets_edit_mode() {
        let mut state = StockTemplatesEditorState::default();
        state.templates.push(make_template("foo"));

        state.open_template_for_edit("foo");

        assert_eq!(state.mode, StockTemplatesEditorMode::Edit);
        assert_eq!(state.edit_buffer.id, "foo");
        assert_eq!(state.selected_template, Some(0));
    }

    // ── open_template_for_edit with unknown id is a no-op ───────────────────

    #[test]
    fn test_open_template_for_edit_unknown_id_noop() {
        let mut state = StockTemplatesEditorState::default();
        state.templates.push(make_template("existing"));

        // Start in List mode; call with unknown ID
        state.open_template_for_edit("nonexistent_id");

        assert_eq!(
            state.mode,
            StockTemplatesEditorMode::List,
            "mode should remain List"
        );
        assert!(
            state.selected_template.is_none(),
            "selected_template should remain None"
        );
    }

    // ── reset_for_new_campaign ───────────────────────────────────────────────

    #[test]
    fn test_reset_for_new_campaign_clears_templates() {
        let mut state = StockTemplatesEditorState::default();
        state.templates.push(make_template("tmpl_a"));
        state.templates.push(make_template("tmpl_b"));
        state.selected_template = Some(1);
        state.has_unsaved_changes = true;
        state.needs_initial_load = false;

        state.reset_for_new_campaign();

        assert!(state.templates.is_empty(), "templates should be cleared");
        assert!(
            state.selected_template.is_none(),
            "selection should be cleared"
        );
        assert!(
            !state.has_unsaved_changes,
            "unsaved_changes should be false"
        );
        assert_eq!(
            state.mode,
            StockTemplatesEditorMode::List,
            "mode should be reset to List"
        );
    }

    #[test]
    fn test_reset_for_new_campaign_sets_needs_initial_load() {
        let mut state = StockTemplatesEditorState {
            needs_initial_load: false,
            ..Default::default()
        };
        // Simulate a loaded campaign: flag would have been cleared after load
        state.templates.push(make_template("stale"));

        state.reset_for_new_campaign();

        assert!(
            state.needs_initial_load,
            "needs_initial_load must be true after reset so show() auto-loads on next render"
        );
    }

    #[test]
    fn test_reset_for_new_campaign_clears_edit_state() {
        let mut state = StockTemplatesEditorState::default();
        state.templates.push(make_template("foo"));
        state.open_template_for_edit("foo");
        // Now in Edit mode with a populated buffer
        assert_eq!(state.mode, StockTemplatesEditorMode::Edit);
        assert_eq!(state.edit_buffer.id, "foo");

        state.reset_for_new_campaign();

        assert_eq!(
            state.mode,
            StockTemplatesEditorMode::List,
            "mode should reset to List even from Edit"
        );
        assert!(
            state.edit_buffer.id.is_empty(),
            "edit buffer should be cleared"
        );
        assert!(
            state.validation_errors.is_empty(),
            "validation errors should be cleared"
        );
    }

    #[test]
    fn test_reset_for_new_campaign_clears_search_filter() {
        let mut state = StockTemplatesEditorState {
            search_filter: "blacksmith".to_string(),
            validation_warnings: vec!["some warning".to_string()],
            show_delete_confirm: true,
            ..Default::default()
        };

        state.reset_for_new_campaign();

        assert!(
            state.search_filter.is_empty(),
            "search filter should be cleared"
        );
        assert!(
            state.validation_warnings.is_empty(),
            "warnings should be cleared"
        );
        assert!(
            !state.show_delete_confirm,
            "delete confirm dialog should be dismissed"
        );
    }

    // ── needs_initial_load default ───────────────────────────────────────────

    #[test]
    fn test_needs_initial_load_is_true_on_default() {
        let state = StockTemplatesEditorState::default();
        assert!(
            state.needs_initial_load,
            "needs_initial_load should start as true so first show() auto-loads"
        );
    }

    // ── load_from_file clears needs_initial_load indirectly ──────────────────

    // ── loaded_from_file guard (wipe-prevention fix) ─────────────────────────

    #[test]
    fn test_loaded_from_file_is_false_on_default() {
        // The guard must start as false so that do_save_campaign cannot
        // overwrite an on-disk file with [] before any load has occurred.
        let state = StockTemplatesEditorState::default();
        assert!(
            !state.loaded_from_file,
            "loaded_from_file must start as false to prevent premature overwrites"
        );
    }

    #[test]
    fn test_load_from_file_sets_loaded_from_file_true() {
        // After a successful load, the guard must be true so that
        // do_save_campaign is permitted to write the file back.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("npc_stock_templates.ron");

        // Create a valid file with one template.
        let mut writer = StockTemplatesEditorState::default();
        writer.templates.push(make_template("guard_test"));
        writer.save_to_file(&path).expect("save");

        let mut reader = StockTemplatesEditorState::default();
        assert!(!reader.loaded_from_file, "starts false");
        reader.load_from_file(&path).expect("load");
        assert!(
            reader.loaded_from_file,
            "loaded_from_file must be true after a successful load_from_file call"
        );
    }

    #[test]
    fn test_reset_for_new_campaign_clears_loaded_from_file() {
        // reset_for_new_campaign must clear the guard so that a stale loaded
        // state from a previous campaign cannot allow writes for a freshly
        // opened campaign whose templates have not yet been read from disk.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("npc_stock_templates.ron");

        let mut state = StockTemplatesEditorState::default();
        state.templates.push(make_template("reset_guard_test"));
        state.save_to_file(&path).expect("save");
        state.load_from_file(&path).expect("load");
        assert!(state.loaded_from_file, "loaded after load_from_file");

        state.reset_for_new_campaign();

        assert!(
            !state.loaded_from_file,
            "loaded_from_file must be false after reset_for_new_campaign"
        );
    }

    #[test]
    fn test_load_from_file_does_not_mutate_needs_initial_load() {
        // load_from_file itself does not touch needs_initial_load — the caller
        // (lib.rs load_stock_templates) sets it to false on success.
        // This test documents that contract explicitly.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("npc_stock_templates.ron");

        let mut writer = StockTemplatesEditorState::default();
        writer.templates.push(make_template("contract_test"));
        writer.save_to_file(&path).expect("save");

        let mut reader = StockTemplatesEditorState::default();
        // needs_initial_load starts true
        assert!(reader.needs_initial_load);
        reader.load_from_file(&path).expect("load");
        // load_from_file does NOT clear the flag — caller responsibility
        assert!(
            reader.needs_initial_load,
            "load_from_file must not clear needs_initial_load; caller owns that"
        );
        assert_eq!(reader.templates.len(), 1);
    }

    // ── description: from_template reads description ─────────────────────────

    #[test]
    fn test_stock_template_description_is_persisted() {
        let mut tmpl = make_template("general_goods");
        tmpl.description = "General goods shop".to_string();

        let buf = StockTemplateEditBuffer::from_template(&tmpl);

        assert_eq!(
            buf.description, "General goods shop",
            "from_template must copy the description from the domain struct"
        );
    }

    // ── description: to_template writes description ──────────────────────────

    #[test]
    fn test_stock_template_description_to_template() {
        let buf = StockTemplateEditBuffer {
            id: "test_tmpl".to_string(),
            description: "A fine shop".to_string(),
            ..StockTemplateEditBuffer::default()
        };
        let mut warnings = Vec::new();
        let result = buf.to_template(&[], StockTemplatesEditorMode::Add, &mut warnings);
        assert!(
            result.is_ok(),
            "to_template failed: {:?}",
            result.unwrap_err()
        );
        let tmpl = result.unwrap();
        assert_eq!(
            tmpl.description, "A fine shop",
            "to_template must copy description into the returned MerchantStockTemplate"
        );
    }

    // ── description: round-trip through from→to preserves value ──────────────

    #[test]
    fn test_stock_template_description_round_trip_non_empty() {
        let mut original = make_template("shop_with_desc");
        original.description = "Weapons and armour".to_string();

        let buf = StockTemplateEditBuffer::from_template(&original);
        assert_eq!(buf.description, "Weapons and armour");

        let mut warnings = Vec::new();
        let restored = buf
            .to_template(&[], StockTemplatesEditorMode::Add, &mut warnings)
            .expect("round-trip must succeed");

        assert_eq!(restored.description, "Weapons and armour");
    }

    // ── description: empty description round-trips as empty string ────────────

    #[test]
    fn test_stock_template_description_empty_round_trip() {
        let original = make_template("no_desc_shop");
        // description is String::new() from make_template

        let buf = StockTemplateEditBuffer::from_template(&original);
        assert!(buf.description.is_empty());

        let mut warnings = Vec::new();
        let restored = buf
            .to_template(&[], StockTemplatesEditorMode::Add, &mut warnings)
            .expect("round-trip must succeed");

        assert!(restored.description.is_empty());
    }

    // ── description: show_preview renders description or placeholder ──────────

    /// Verifies that the `show_preview` display panel correctly distinguishes
    /// between a template with a non-empty description and one with an empty
    /// description.  This mirrors what the egui `show_preview` function does
    /// with its `if tmpl.description.is_empty()` branch (§2.4 SDK fix).
    ///
    /// We test the data-level precondition (the branch predicate) rather than
    /// the egui rendering output, following the same pattern used by
    /// `test_starting_spells_display_section_shows_spell_names` in
    /// `characters_editor.rs`.
    #[test]
    fn test_stock_template_display_shows_description() {
        // ── non-empty description → description label branch ──────────────────
        let mut tmpl_with_desc = make_template("shop_with_desc");
        tmpl_with_desc.description = "Fine weapons and armour".to_string();

        // The display panel shows `tmpl.description` directly when non-empty.
        assert!(
            !tmpl_with_desc.description.is_empty(),
            "template description must be non-empty so the display panel renders it"
        );
        assert_eq!(
            tmpl_with_desc.description, "Fine weapons and armour",
            "description value must match what was set"
        );

        // ── empty description → placeholder branch ────────────────────────────
        let tmpl_no_desc = make_template("no_desc_shop");
        // description defaults to String::new() in make_template

        assert!(
            tmpl_no_desc.description.is_empty(),
            "template with no description must be empty so the display panel shows the placeholder"
        );

        // ── `from_template` preserves non-empty description for display ───────
        let buf = StockTemplateEditBuffer::from_template(&tmpl_with_desc);
        assert_eq!(
            buf.description, "Fine weapons and armour",
            "from_template must carry the description into the edit buffer \
             so re-loading the edit form pre-populates the field correctly"
        );

        // ── egui smoke test: show_preview must not panic ──────────────────────
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(800.0, 600.0),
            )),
            ..Default::default()
        };
        ctx.begin_pass(raw_input);
        let state = StockTemplatesEditorState::default();
        egui::CentralPanel::default().show(&ctx, |ui| {
            // Must not panic for a template with a non-empty description.
            state.show_preview(ui, &tmpl_with_desc);
        });
        let _ = ctx.end_pass();

        let ctx2 = egui::Context::default();
        let raw_input2 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(800.0, 600.0),
            )),
            ..Default::default()
        };
        ctx2.begin_pass(raw_input2);
        egui::CentralPanel::default().show(&ctx2, |ui| {
            // Must not panic for a template with an empty description either.
            state.show_preview(ui, &tmpl_no_desc);
        });
        let _ = ctx2.end_pass();
    }
}
