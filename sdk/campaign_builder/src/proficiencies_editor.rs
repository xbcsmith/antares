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
//! - Auto-generated IDs based on category
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
use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};
use eframe::egui;
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
    ) {
        ui.heading("🎯 Proficiencies Editor");
        ui.add_space(5.0);

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

                            // Preview static display
                            Self::show_preview_static(right_ui, prof);

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
                            proficiencies.remove(idx);
                            self.selected_proficiency = None;
                            *unsaved_changes = true;
                            *status_message = "Deleted proficiency".to_string();
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

    /// Show a static preview of proficiency details (doesn't require &self)
    fn show_preview_static(ui: &mut egui::Ui, proficiency: &ProficiencyDefinition) {
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
            ui.label(category_str);
        });

        if !proficiency.description.is_empty() {
            ui.separator();
            ui.label("Description:");
            ui.label(&proficiency.description);
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
