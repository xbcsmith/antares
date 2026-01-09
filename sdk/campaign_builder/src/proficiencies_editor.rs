// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Proficiencies editor for the campaign builder
//!
//! This module provides a UI editor for managing proficiency definitions.
//! Proficiencies determine which items characters can use based on their
//! class and race. The editor follows the two-column layout pattern
//! (list/detail) used by other editors.
//!
//! # Features
//!
//! - Create, edit, and delete proficiency definitions
//! - Filter proficiencies by category (Weapon, Armor, Shield, MagicItem)
//! - Search proficiencies by name or ID
//! - Import/export proficiencies as RON
//! - Auto-generated IDs based on category and name
//! - Usage tracking: shows where proficiencies are used
//! - Visual category colors and icons
//! - Warnings before deleting used proficiencies
//! - Bulk operations (export all, import multiple, reset to defaults)
//!
//! # Examples
//!
//! ```ignore
//! let mut editor_state = ProficienciesEditorState::new();
//! editor_state.show(
//!     ui,
//!     &mut proficiencies,
//!     campaign_dir,
//!     "data/proficiencies.ron",
//!     &mut unsaved_changes,
//!     &mut status_message,
//!     &mut file_load_merge_mode,
//! );
//! ```

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::classes::ClassDefinition;
use antares::domain::items::types::Item;
use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};
use antares::domain::races::RaceDefinition;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

/// Editor mode for proficiencies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficienciesEditorMode {
    /// Viewing the list of proficiencies
    List,
    /// Adding a new proficiency
    Add,
    /// Editing an existing proficiency
    Edit,
}

/// Category filter for proficiency list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyCategoryFilter {
    /// Show all proficiencies
    All,
    /// Weapon proficiencies only
    Weapon,
    /// Armor proficiencies only
    Armor,
    /// Shield proficiencies only
    Shield,
    /// Magic item proficiencies only
    MagicItem,
}

impl ProficiencyCategoryFilter {
    /// Check if a proficiency matches this filter
    pub fn matches(&self, proficiency: &ProficiencyDefinition) -> bool {
        match self {
            ProficiencyCategoryFilter::All => true,
            ProficiencyCategoryFilter::Weapon => {
                proficiency.category == ProficiencyCategory::Weapon
            }
            ProficiencyCategoryFilter::Armor => proficiency.category == ProficiencyCategory::Armor,
            ProficiencyCategoryFilter::Shield => {
                proficiency.category == ProficiencyCategory::Shield
            }
            ProficiencyCategoryFilter::MagicItem => {
                proficiency.category == ProficiencyCategory::MagicItem
            }
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            ProficiencyCategoryFilter::All => "All",
            ProficiencyCategoryFilter::Weapon => "⚔️ Weapon",
            ProficiencyCategoryFilter::Armor => "🛡️ Armor",
            ProficiencyCategoryFilter::Shield => "🛡️ Shield",
            ProficiencyCategoryFilter::MagicItem => "✨ Magic Item",
        }
    }

    /// Get color for this category
    pub fn color(&self) -> egui::Color32 {
        match self {
            ProficiencyCategoryFilter::All => egui::Color32::GRAY,
            ProficiencyCategoryFilter::Weapon => egui::Color32::from_rgb(255, 100, 0), // Orange
            ProficiencyCategoryFilter::Armor => egui::Color32::from_rgb(0, 120, 215),  // Blue
            ProficiencyCategoryFilter::Shield => egui::Color32::from_rgb(0, 180, 219), // Cyan
            ProficiencyCategoryFilter::MagicItem => egui::Color32::from_rgb(200, 100, 255), // Purple
        }
    }

    /// Get all filter options
    pub fn all() -> [ProficiencyCategoryFilter; 5] {
        [
            ProficiencyCategoryFilter::All,
            ProficiencyCategoryFilter::Weapon,
            ProficiencyCategoryFilter::Armor,
            ProficiencyCategoryFilter::Shield,
            ProficiencyCategoryFilter::MagicItem,
        ]
    }
}

/// Tracks where a proficiency is used in the campaign
#[derive(Debug, Clone, Default)]
pub struct ProficiencyUsage {
    /// Classes that grant this proficiency
    pub granted_by_classes: Vec<String>,
    /// Races that grant this proficiency
    pub granted_by_races: Vec<String>,
    /// Items that require this proficiency
    pub required_by_items: Vec<String>,
}

impl ProficiencyUsage {
    /// Check if this proficiency is used anywhere
    pub fn is_used(&self) -> bool {
        !self.granted_by_classes.is_empty()
            || !self.granted_by_races.is_empty()
            || !self.required_by_items.is_empty()
    }

    /// Get total usage count
    pub fn total_count(&self) -> usize {
        self.granted_by_classes.len() + self.granted_by_races.len() + self.required_by_items.len()
    }
}

/// State for the proficiencies editor
#[derive(Debug, Clone)]
pub struct ProficienciesEditorState {
    /// Current editor mode
    pub mode: ProficienciesEditorMode,

    /// Search query for filtering proficiencies
    pub search_query: String,

    /// Index of selected proficiency in the filtered list
    pub selected_proficiency: Option<usize>,

    /// Buffer for editing proficiency details
    pub edit_buffer: ProficiencyDefinition,

    /// Whether to show import/export dialog
    pub show_import_dialog: bool,

    /// Buffer for import/export RON data
    pub import_export_buffer: String,

    /// Category filter
    pub filter_category: ProficiencyCategoryFilter,

    /// Confirm deletion of a proficiency (stores the ID to delete)
    pub confirm_delete_id: Option<String>,

    /// Cached usage information for proficiencies
    pub usage_cache: HashMap<String, ProficiencyUsage>,
}

impl Default for ProficienciesEditorState {
    fn default() -> Self {
        Self {
            mode: ProficienciesEditorMode::List,
            search_query: String::new(),
            selected_proficiency: None,
            edit_buffer: Self::default_proficiency(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            filter_category: ProficiencyCategoryFilter::All,
            confirm_delete_id: None,
            usage_cache: HashMap::new(),
        }
    }
}

impl ProficienciesEditorState {
    /// Create a new proficiencies editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a default proficiency for new entries
    pub fn default_proficiency() -> ProficiencyDefinition {
        ProficiencyDefinition {
            id: "new_proficiency".to_string(),
            name: "New Proficiency".to_string(),
            category: ProficiencyCategory::Weapon,
            description: String::new(),
        }
    }

    /// Generate the next available proficiency ID based on category
    pub fn next_proficiency_id(
        proficiencies: &[ProficiencyDefinition],
        category: ProficiencyCategory,
    ) -> String {
        let prefix = match category {
            ProficiencyCategory::Weapon => "weapon_",
            ProficiencyCategory::Armor => "armor_",
            ProficiencyCategory::Shield => "shield_",
            ProficiencyCategory::MagicItem => "item_",
        };

        let mut counter = 1;
        loop {
            let candidate = format!("{}{}", prefix, counter);
            if !proficiencies.iter().any(|p| p.id == candidate) {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Suggest a proficiency ID based on name and category
    ///
    /// This method creates a smart ID suggestion from the proficiency name
    /// by slugifying it and combining with a category-specific prefix.
    ///
    /// # Examples
    ///
    /// - Name: "Longsword", Category: Weapon → "weapon_longsword"
    /// - Name: "Heavy Armor", Category: Armor → "armor_heavy"
    /// - Name: "Magic Wand", Category: MagicItem → "item_magic_wand"
    pub fn suggest_proficiency_id(
        name: &str,
        category: ProficiencyCategory,
        proficiencies: &[ProficiencyDefinition],
    ) -> String {
        let prefix = match category {
            ProficiencyCategory::Weapon => "weapon_",
            ProficiencyCategory::Armor => "armor_",
            ProficiencyCategory::Shield => "shield_",
            ProficiencyCategory::MagicItem => "item_",
        };

        // Convert name to slugified form (lowercase, replace spaces with underscores)
        let slugified = name
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_whitespace() {
                    '_'
                } else if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_");

        let suggested = format!("{}{}", prefix, slugified);

        // If the suggested ID already exists, append a number
        if proficiencies.iter().any(|p| p.id == suggested) {
            let mut counter = 2;
            loop {
                let candidate = format!("{}_{}", suggested, counter);
                if !proficiencies.iter().any(|p| p.id == candidate) {
                    return candidate;
                }
                counter += 1;
            }
        } else {
            suggested
        }
    }

    /// Calculate usage of proficiencies across classes, races, and items
    pub fn calculate_usage(
        proficiency_ids: Vec<&str>,
        classes: &[ClassDefinition],
        races: &[RaceDefinition],
        items: &[Item],
    ) -> HashMap<String, ProficiencyUsage> {
        let mut usage_map: HashMap<String, ProficiencyUsage> = proficiency_ids
            .iter()
            .map(|id| (id.to_string(), ProficiencyUsage::default()))
            .collect();

        // Check classes
        for class in classes {
            for prof_id in &class.proficiencies {
                if let Some(entry) = usage_map.get_mut(prof_id) {
                    entry.granted_by_classes.push(class.id.clone());
                }
            }
        }

        // Check races
        for race in races {
            for prof_id in &race.proficiencies {
                if let Some(entry) = usage_map.get_mut(prof_id) {
                    entry.granted_by_races.push(race.id.clone());
                }
            }
        }

        // Check items for proficiency requirements
        for item in items {
            if let Some(req_prof) = item.required_proficiency() {
                if let Some(entry) = usage_map.get_mut(&req_prof) {
                    entry.required_by_items.push(item.id.to_string());
                }
            }
        }

        usage_map
    }

    /// Show the proficiencies editor UI
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        proficiencies: &mut Vec<ProficiencyDefinition>,
        campaign_dir: Option<&PathBuf>,
        proficiencies_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
        classes: &[ClassDefinition],
        races: &[RaceDefinition],
        items: &[Item],
    ) {
        ui.heading("🎯 Proficiencies Editor");
        ui.add_space(5.0);

        // Update usage cache
        let prof_ids: Vec<&str> = proficiencies.iter().map(|p| p.id.as_str()).collect();
        self.usage_cache = Self::calculate_usage(prof_ids, classes, races, items);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Proficiencies")
            .with_search(&mut self.search_query)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(proficiencies.len())
            .with_id_salt("proficiencies_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.mode = ProficienciesEditorMode::Add;
                self.edit_buffer = Self::default_proficiency();
                self.edit_buffer.id =
                    Self::next_proficiency_id(proficiencies, self.edit_buffer.category);
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                self.save_proficiencies(
                    proficiencies,
                    campaign_dir,
                    proficiencies_file,
                    unsaved_changes,
                    status_message,
                );
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<ProficiencyDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_proficiencies) => {
                            if *file_load_merge_mode {
                                for proficiency in loaded_proficiencies {
                                    if let Some(existing) =
                                        proficiencies.iter_mut().find(|p| p.id == proficiency.id)
                                    {
                                        *existing = proficiency;
                                    } else {
                                        proficiencies.push(proficiency);
                                    }
                                }
                            } else {
                                *proficiencies = loaded_proficiencies;
                            }
                            *unsaved_changes = true;
                            *status_message =
                                format!("Loaded proficiencies from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load proficiencies: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("proficiencies.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(proficiencies, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message =
                                    format!("Saved proficiencies to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save proficiencies: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize proficiencies: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(proficiencies_file);
                    if path.exists() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => {
                                match ron::from_str::<Vec<ProficiencyDefinition>>(&contents) {
                                    Ok(loaded_proficiencies) => {
                                        *proficiencies = loaded_proficiencies;
                                        *status_message = format!(
                                            "Loaded proficiencies from: {}",
                                            path.display()
                                        );
                                    }
                                    Err(e) => {
                                        *status_message =
                                            format!("Failed to parse proficiencies: {}", e)
                                    }
                                }
                            }
                            Err(e) => {
                                *status_message = format!("Failed to read proficiencies: {}", e)
                            }
                        }
                    } else {
                        *status_message = "Proficiencies file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        // Category filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            egui::ComboBox::from_id_salt("proficiency_category_filter")
                .selected_text(self.filter_category.as_str())
                .show_ui(ui, |ui| {
                    for filter in ProficiencyCategoryFilter::all() {
                        ui.selectable_value(&mut self.filter_category, filter, filter.as_str());
                    }
                });
        });

        ui.separator();

        // Show import/export dialog if active
        if self.show_import_dialog {
            self.show_import_dialog_window(
                ui.ctx(),
                proficiencies,
                unsaved_changes,
                status_message,
            );
        }

        // Show list or form based on mode
        match self.mode {
            ProficienciesEditorMode::List => {
                self.show_list(ui, proficiencies, unsaved_changes, status_message)
            }
            ProficienciesEditorMode::Add | ProficienciesEditorMode::Edit => {
                self.show_form(ui, proficiencies, unsaved_changes, status_message)
            }
        }
    }

    /// Show the proficiencies list with two-column layout
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        proficiencies: &mut Vec<ProficiencyDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        // Build filtered snapshot to avoid borrow conflicts in closures
        let filtered_proficiencies: Vec<(usize, String, ProficiencyDefinition)> = proficiencies
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let matches_category = self.filter_category.matches(p);
                let matches_search = self.search_query.is_empty()
                    || p.id
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    || p.name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());
                matches_category && matches_search
            })
            .map(|(idx, prof)| {
                let emoji = match prof.category {
                    ProficiencyCategory::Weapon => "⚔️",
                    ProficiencyCategory::Armor => "🛡️",
                    ProficiencyCategory::Shield => "🛡️",
                    ProficiencyCategory::MagicItem => "✨",
                };
                let label = format!("{} {}: {}", emoji, prof.id, prof.name);
                (idx, label, prof.clone())
            })
            .collect();

        // Store selected and action outside closures
        let selected = self.selected_proficiency;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        TwoColumnLayout::new("proficiencies_list_layout")
            .with_left_width(300.0)
            .show_split(
                ui,
                |left_ui| {
                    // Left column: list
                    left_ui.label("Proficiencies:");
                    left_ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(left_ui, |left_ui| {
                            for (i, (idx, label, _prof)) in
                                filtered_proficiencies.iter().enumerate()
                            {
                                let is_selected = selected == Some(*idx);
                                if left_ui.selectable_label(is_selected, label).clicked() {
                                    new_selection = Some(*idx);
                                }
                            }

                            if filtered_proficiencies.is_empty() {
                                left_ui.label("No proficiencies found");
                            }
                        });
                },
                |right_ui| {
                    // Right column: preview and actions
                    if let Some(idx) = selected {
                        if let Some((_, _label, prof)) =
                            filtered_proficiencies.iter().find(|(i, _, _)| *i == idx)
                        {
                            right_ui.label("Details:");
                            right_ui.separator();

                            // Get usage information for this proficiency
                            let usage = self.usage_cache.get(&prof.id);

                            // Preview static display with usage
                            Self::show_preview_static(right_ui, prof, usage);

                            right_ui.separator();

                            // Action buttons
                            let action = ActionButtons::new()
                                .with_edit(true)
                                .with_delete(true)
                                .with_duplicate(true)
                                .with_export(true)
                                .show(right_ui);

                            if action != ItemAction::None {
                                action_requested = Some(action);
                            }
                        } else {
                            right_ui.label("Select a proficiency to view details");
                        }
                    } else {
                        right_ui.label("Select a proficiency to view details");
                    }
                },
            );

        // Apply selection change after closures
        self.selected_proficiency = new_selection;

        // Handle deletion confirmation window if active
        if let Some(prof_id) = &self.confirm_delete_id {
            let mut confirm_open = true;
            let prof_id_clone = prof_id.clone();

            egui::Window::new("⚠️ Confirm Deletion")
                .open(&mut confirm_open)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |dialog_ui| {
                    if let Some(usage) = self.usage_cache.get(&prof_id_clone) {
                        if usage.is_used() {
                            dialog_ui.colored_label(egui::Color32::RED,
                                format!("This proficiency is used by {} classes, {} races, and {} items.",
                                    usage.granted_by_classes.len(),
                                    usage.granted_by_races.len(),
                                    usage.required_by_items.len()));
                            dialog_ui.label("Are you sure you want to delete it?");
                        } else {
                            dialog_ui.label(format!("Delete proficiency '{}'?", prof_id_clone));
                        }
                    }

                    dialog_ui.separator();

                    dialog_ui.horizontal(|ui| {
                        if ui.button("❌ Cancel").clicked() {
                            self.confirm_delete_id = None;
                        }

                        if ui.button("🗑️ Delete").clicked() {
                            if let Some(idx) = self.selected_proficiency {
                                if idx < proficiencies.len() {
                                    proficiencies.remove(idx);
                                    self.selected_proficiency = None;
                                    *unsaved_changes = true;
                                    *status_message = "Deleted proficiency".to_string();
                                }
                            }
                            self.confirm_delete_id = None;
                        }
                    });
                });

            if !confirm_open {
                self.confirm_delete_id = None;
            }
        }

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_proficiency {
                        if idx < proficiencies.len() {
                            self.mode = ProficienciesEditorMode::Edit;
                            self.edit_buffer = proficiencies[idx].clone();
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_proficiency {
                        if idx < proficiencies.len() {
                            // Initiate delete confirmation
                            self.confirm_delete_id = Some(proficiencies[idx].id.clone());
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_proficiency {
                        if idx < proficiencies.len() {
                            let mut new_prof = proficiencies[idx].clone();
                            new_prof.id =
                                Self::next_proficiency_id(proficiencies, new_prof.category);
                            new_prof.name = format!("{} (Copy)", new_prof.name);
                            proficiencies.push(new_prof);
                            *unsaved_changes = true;
                            *status_message = "Duplicated proficiency".to_string();
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_proficiency {
                        if idx < proficiencies.len() {
                            let prof = &proficiencies[idx];
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(format!("{}.ron", prof.id))
                                .add_filter("RON", &["ron"])
                                .save_file()
                            {
                                match ron::ser::to_string_pretty(prof, Default::default()) {
                                    Ok(contents) => match std::fs::write(&path, contents) {
                                        Ok(_) => {
                                            *status_message = format!(
                                                "Exported proficiency to: {}",
                                                path.display()
                                            );
                                        }
                                        Err(e) => {
                                            *status_message =
                                                format!("Failed to save proficiency: {}", e);
                                        }
                                    },
                                    Err(e) => {
                                        *status_message =
                                            format!("Failed to serialize proficiency: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Show the form for adding/editing proficiencies
    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        proficiencies: &[ProficiencyDefinition],
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let title = match self.mode {
            ProficienciesEditorMode::Add => "Add New Proficiency",
            ProficienciesEditorMode::Edit => "Edit Proficiency",
            ProficienciesEditorMode::List => "Proficiency Editor",
        };

        ui.label(title);
        ui.separator();

        ui.label("ID:");
        let id_enabled = self.mode == ProficienciesEditorMode::Add;
        ui.add(
            egui::TextEdit::singleline(&mut self.edit_buffer.id)
                .desired_rows(1)
                .interactive(id_enabled),
        );

        if self.mode == ProficienciesEditorMode::Add {
            if ui.button("💡 Suggest ID from Name").clicked() {
                self.edit_buffer.id = Self::suggest_proficiency_id(
                    &self.edit_buffer.name,
                    self.edit_buffer.category,
                    proficiencies,
                );
            }
        }

        ui.label("Name:");
        ui.text_edit_singleline(&mut self.edit_buffer.name);

        ui.label("Category:");
        egui::ComboBox::from_id_salt("proficiency_form_category")
            .selected_text(match self.edit_buffer.category {
                ProficiencyCategory::Weapon => "⚔️ Weapon",
                ProficiencyCategory::Armor => "🛡️ Armor",
                ProficiencyCategory::Shield => "🛡️ Shield",
                ProficiencyCategory::MagicItem => "✨ Magic Item",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.edit_buffer.category,
                    ProficiencyCategory::Weapon,
                    "⚔️ Weapon",
                );
                ui.selectable_value(
                    &mut self.edit_buffer.category,
                    ProficiencyCategory::Armor,
                    "🛡️ Armor",
                );
                ui.selectable_value(
                    &mut self.edit_buffer.category,
                    ProficiencyCategory::Shield,
                    "🛡️ Shield",
                );
                ui.selectable_value(
                    &mut self.edit_buffer.category,
                    ProficiencyCategory::MagicItem,
                    "✨ Magic Item",
                );
            });

        ui.label("Description:");
        ui.text_edit_multiline(&mut self.edit_buffer.description);

        ui.separator();

        // Validation feedback
        let id_valid = !self.edit_buffer.id.is_empty();
        let id_unique = self.mode == ProficienciesEditorMode::Add
            || proficiencies.iter().all(|p| p.id != self.edit_buffer.id);
        let name_valid = !self.edit_buffer.name.is_empty();

        if !id_valid {
            ui.colored_label(egui::Color32::RED, "⚠️ ID must not be empty");
        }
        if !id_unique {
            ui.colored_label(egui::Color32::RED, "⚠️ ID must be unique");
        }
        if !name_valid {
            ui.colored_label(egui::Color32::YELLOW, "⚠️ Name should not be empty");
        }

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            let save_enabled = id_valid && id_unique && name_valid;

            if ui
                .add_enabled(save_enabled, egui::Button::new("💾 Save"))
                .clicked()
            {
                if save_enabled {
                    self.mode = ProficienciesEditorMode::List;
                    *unsaved_changes = true;
                    *status_message = format!("Saved proficiency: {}", self.edit_buffer.id);
                }
            }

            if ui.button("❌ Cancel").clicked() {
                self.mode = ProficienciesEditorMode::List;
                self.edit_buffer = Self::default_proficiency();
            }

            if self.mode == ProficienciesEditorMode::Edit {
                if ui.button("🔄 Reset").clicked() {
                    if let Some(prof) = proficiencies.iter().find(|p| p.id == self.edit_buffer.id) {
                        self.edit_buffer = prof.clone();
                        *status_message = format!("Reset: {}", prof.id);
                    }
                }
            }
        });
    }

    /// Show a static preview of proficiency details with usage information
    fn show_preview_static(
        ui: &mut egui::Ui,
        proficiency: &ProficiencyDefinition,
        usage: Option<&ProficiencyUsage>,
    ) {
        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.monospace(&proficiency.id);
        });

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.label(&proficiency.name);
        });

        ui.horizontal(|ui| {
            ui.label("Category:");
            let category_str = match proficiency.category {
                ProficiencyCategory::Weapon => "⚔️ Weapon",
                ProficiencyCategory::Armor => "🛡️ Armor",
                ProficiencyCategory::Shield => "🛡️ Shield",
                ProficiencyCategory::MagicItem => "✨ Magic Item",
            };
            let color = match proficiency.category {
                ProficiencyCategory::Weapon => egui::Color32::from_rgb(255, 100, 0),
                ProficiencyCategory::Armor => egui::Color32::from_rgb(0, 120, 215),
                ProficiencyCategory::Shield => egui::Color32::from_rgb(0, 180, 219),
                ProficiencyCategory::MagicItem => egui::Color32::from_rgb(200, 100, 255),
            };
            ui.colored_label(color, category_str);
        });

        if !proficiency.description.is_empty() {
            ui.separator();
            ui.label("Description:");
            ui.label(&proficiency.description);
        }

        // Show usage information if available
        if let Some(usage) = usage {
            if usage.is_used() {
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, "⚠️ In Use");

                if !usage.granted_by_classes.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Classes:");
                        ui.label(format!("{} class(es)", usage.granted_by_classes.len()));
                    });
                }

                if !usage.granted_by_races.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Races:");
                        ui.label(format!("{} race(s)", usage.granted_by_races.len()));
                    });
                }

                if !usage.required_by_items.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Items:");
                        ui.label(format!("{} item(s)", usage.required_by_items.len()));
                    });
                }
            } else {
                ui.separator();
                ui.colored_label(egui::Color32::GREEN, "✓ Not in use");
            }
        }
    }

    /// Show the import/export dialog window
    fn show_import_dialog_window(
        &mut self,
        ctx: &egui::Context,
        proficiencies: &mut Vec<ProficiencyDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let mut open = self.show_import_dialog;
        egui::Window::new("Import/Export Proficiencies")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📋 Copy from Data").clicked() {
                        match ron::ser::to_string_pretty(proficiencies, Default::default()) {
                            Ok(contents) => {
                                self.import_export_buffer = contents;
                                *status_message = "Copied proficiencies to clipboard".to_string();
                            }
                            Err(e) => {
                                *status_message = format!("Failed to serialize: {}", e);
                            }
                        }
                    }

                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Vec<ProficiencyDefinition>>(
                            &self.import_export_buffer,
                        ) {
                            Ok(imported) => {
                                *proficiencies = imported;
                                *unsaved_changes = true;
                                *status_message = "Imported proficiencies".to_string();
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                *status_message = format!("Failed to parse RON: {}", e);
                            }
                        }
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });

                ui.separator();

                ui.label("RON Data:");
                ui.text_edit_multiline(&mut self.import_export_buffer);
            });

        self.show_import_dialog = open;
    }

    /// Save proficiencies to file
    fn save_proficiencies(
        &self,
        proficiencies: &[ProficiencyDefinition],
        campaign_dir: Option<&PathBuf>,
        proficiencies_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let file_path = dir.join(proficiencies_file);
            // Ensure directory exists
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        *status_message = format!("Failed to create directory: {}", e);
                        return;
                    }
                }
            }

            match ron::ser::to_string_pretty(proficiencies, Default::default()) {
                Ok(contents) => match std::fs::write(&file_path, contents) {
                    Ok(_) => {
                        *status_message =
                            format!("Saved proficiencies to: {}", file_path.display());
                        *unsaved_changes = false;
                    }
                    Err(e) => {
                        *status_message = format!("Failed to write proficiencies: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize proficiencies: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proficiencies_editor_state_new() {
        let state = ProficienciesEditorState::new();
        assert_eq!(state.mode, ProficienciesEditorMode::List);
        assert_eq!(state.search_query, "");
        assert_eq!(state.selected_proficiency, None);
        assert!(!state.show_import_dialog);
    }

    #[test]
    fn test_proficiencies_editor_state_default() {
        let state = ProficienciesEditorState::default();
        assert_eq!(state.mode, ProficienciesEditorMode::List);
        assert_eq!(state.filter_category, ProficiencyCategoryFilter::All);
    }

    #[test]
    fn test_default_proficiency_creation() {
        let prof = ProficienciesEditorState::default_proficiency();
        assert_eq!(prof.id, "new_proficiency");
        assert_eq!(prof.name, "New Proficiency");
        assert_eq!(prof.category, ProficiencyCategory::Weapon);
        assert_eq!(prof.description, "");
    }

    #[test]
    fn test_proficiency_id_generation_weapon() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::next_proficiency_id(
            &proficiencies,
            ProficiencyCategory::Weapon,
        );
        assert!(id.starts_with("weapon_"));
        assert_eq!(id, "weapon_1");
    }

    #[test]
    fn test_proficiency_id_generation_armor() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::next_proficiency_id(
            &proficiencies,
            ProficiencyCategory::Armor,
        );
        assert!(id.starts_with("armor_"));
        assert_eq!(id, "armor_1");
    }

    #[test]
    fn test_proficiency_id_generation_shield() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::next_proficiency_id(
            &proficiencies,
            ProficiencyCategory::Shield,
        );
        assert!(id.starts_with("shield_"));
        assert_eq!(id, "shield_1");
    }

    #[test]
    fn test_proficiency_id_generation_magic_item() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::next_proficiency_id(
            &proficiencies,
            ProficiencyCategory::MagicItem,
        );
        assert!(id.starts_with("item_"));
        assert_eq!(id, "item_1");
    }

    #[test]
    fn test_proficiency_id_generation_with_existing() {
        let proficiencies = vec![
            ProficiencyDefinition::new(
                "weapon_1".to_string(),
                "Test".to_string(),
                ProficiencyCategory::Weapon,
            ),
            ProficiencyDefinition::new(
                "weapon_2".to_string(),
                "Test".to_string(),
                ProficiencyCategory::Weapon,
            ),
        ];
        let id = ProficienciesEditorState::next_proficiency_id(
            &proficiencies,
            ProficiencyCategory::Weapon,
        );
        assert_eq!(id, "weapon_3");
    }

    #[test]
    fn test_category_filter_all() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        );
        assert!(ProficiencyCategoryFilter::All.matches(&prof));
    }

    #[test]
    fn test_category_filter_weapon() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        );
        assert!(ProficiencyCategoryFilter::Weapon.matches(&prof));
        assert!(!ProficiencyCategoryFilter::Armor.matches(&prof));
    }

    #[test]
    fn test_category_filter_armor() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Armor,
        );
        assert!(ProficiencyCategoryFilter::Armor.matches(&prof));
        assert!(!ProficiencyCategoryFilter::Weapon.matches(&prof));
    }

    #[test]
    fn test_category_filter_shield() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Shield,
        );
        assert!(ProficiencyCategoryFilter::Shield.matches(&prof));
        assert!(!ProficiencyCategoryFilter::Weapon.matches(&prof));
    }

    // ===== Phase 3: Validation and Polish Tests =====

    #[test]
    fn test_suggest_proficiency_id_weapon() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::suggest_proficiency_id(
            "Longsword",
            ProficiencyCategory::Weapon,
            &proficiencies,
        );
        assert_eq!(id, "weapon_longsword");
    }

    #[test]
    fn test_suggest_proficiency_id_armor() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::suggest_proficiency_id(
            "Heavy Armor",
            ProficiencyCategory::Armor,
            &proficiencies,
        );
        assert_eq!(id, "armor_heavy_armor");
    }

    #[test]
    fn test_suggest_proficiency_id_magic_item() {
        let proficiencies = vec![];
        let id = ProficienciesEditorState::suggest_proficiency_id(
            "Arcane Wand",
            ProficiencyCategory::MagicItem,
            &proficiencies,
        );
        assert_eq!(id, "item_arcane_wand");
    }

    #[test]
    fn test_suggest_proficiency_id_with_conflict() {
        let proficiencies = vec![ProficiencyDefinition::new(
            "weapon_longsword".to_string(),
            "Longsword".to_string(),
            ProficiencyCategory::Weapon,
        )];
        let id = ProficienciesEditorState::suggest_proficiency_id(
            "Longsword",
            ProficiencyCategory::Weapon,
            &proficiencies,
        );
        // Should append a number when conflict
        assert!(id.starts_with("weapon_longsword"));
    }

    #[test]
    fn test_proficiency_usage_not_used() {
        let usage = ProficiencyUsage::default();
        assert!(!usage.is_used());
        assert_eq!(usage.total_count(), 0);
    }

    #[test]
    fn test_proficiency_usage_is_used() {
        let mut usage = ProficiencyUsage::default();
        usage.granted_by_classes.push("knight".to_string());
        assert!(usage.is_used());
        assert_eq!(usage.total_count(), 1);
    }

    #[test]
    fn test_proficiency_usage_total_count() {
        let mut usage = ProficiencyUsage::default();
        usage.granted_by_classes.push("knight".to_string());
        usage.granted_by_races.push("human".to_string());
        usage.required_by_items.push("sword".to_string());
        assert_eq!(usage.total_count(), 3);
    }

    #[test]
    fn test_category_filter_color() {
        let weapon = ProficiencyCategoryFilter::Weapon;
        let armor = ProficiencyCategoryFilter::Armor;
        assert_ne!(weapon.color(), armor.color());
        assert_eq!(weapon.color(), egui::Color32::from_rgb(255, 100, 0));
    }

    #[test]
    fn test_calculate_usage_no_references() {
        use antares::domain::classes::ClassDefinition;
        use antares::domain::races::RaceDefinition;

        let prof_ids = vec!["test_prof"];
        let classes = vec![];
        let races = vec![];
        let items = vec![];

        let usage = ProficienciesEditorState::calculate_usage(prof_ids, &classes, &races, &items);
        assert!(usage.contains_key("test_prof"));
        assert!(!usage["test_prof"].is_used());
    }

    #[test]
    fn test_category_filter_magic_item() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::MagicItem,
        );
        assert!(ProficiencyCategoryFilter::MagicItem.matches(&prof));
        assert!(!ProficiencyCategoryFilter::Weapon.matches(&prof));
    }

    #[test]
    fn test_category_filter_all_variants() {
        assert_eq!(ProficiencyCategoryFilter::all().len(), 5);
    }

    #[test]
    fn test_category_filter_as_str() {
        assert_eq!(ProficiencyCategoryFilter::All.as_str(), "All");
        assert_eq!(ProficiencyCategoryFilter::Weapon.as_str(), "⚔️ Weapon");
        assert_eq!(ProficiencyCategoryFilter::Armor.as_str(), "🛡️ Armor");
        assert_eq!(ProficiencyCategoryFilter::Shield.as_str(), "🛡️ Shield");
        assert_eq!(
            ProficiencyCategoryFilter::MagicItem.as_str(),
            "✨ Magic Item"
        );
    }
}
