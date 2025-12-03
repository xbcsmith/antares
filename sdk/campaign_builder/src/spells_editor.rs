// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
use antares::domain::types::DiceRoll;
use eframe::egui;
use std::path::PathBuf;

/// Editor mode for spells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellsEditorMode {
    List,
    Add,
    Edit,
}

/// State for the spells editor
pub struct SpellsEditorState {
    pub mode: SpellsEditorMode,
    pub search_query: String,
    pub selected_spell: Option<usize>,
    pub edit_buffer: Spell,
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
    pub show_preview: bool,

    // Filters
    pub filter_school: Option<SpellSchool>,
    pub filter_level: Option<u8>,
}

impl Default for SpellsEditorState {
    fn default() -> Self {
        Self {
            mode: SpellsEditorMode::List,
            search_query: String::new(),
            selected_spell: None,
            edit_buffer: Self::default_spell(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            show_preview: false,
            filter_school: None,
            filter_level: None,
        }
    }
}

impl SpellsEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_spell() -> Spell {
        Spell {
            id: 0,
            name: "New Spell".to_string(),
            school: SpellSchool::Cleric,
            level: 1,
            sp_cost: 1,
            gem_cost: 0,
            context: SpellContext::Anytime,
            target: SpellTarget::SingleCharacter,
            damage: None,
            duration: 0,
            saving_throw: false,
            description: String::new(),
            applied_conditions: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        spells: &mut Vec<Spell>,
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        ui.heading("✨ Spells Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Spells")
            .with_search(&mut self.search_query)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(spells.len())
            .with_id_salt("spells_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.mode = SpellsEditorMode::Add;
                self.edit_buffer = Self::default_spell();
                let next_id = spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                self.save_spells(
                    spells,
                    campaign_dir,
                    spells_file,
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
                        ron::from_str::<Vec<Spell>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_spells) => {
                            if *file_load_merge_mode {
                                for spell in loaded_spells {
                                    if let Some(existing) =
                                        spells.iter_mut().find(|s| s.id == spell.id)
                                    {
                                        *existing = spell;
                                    } else {
                                        spells.push(spell);
                                    }
                                }
                            } else {
                                *spells = loaded_spells;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded spells from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load spells: {}", e);
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
                    .set_file_name("spells.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(spells, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved spells to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save spells: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize spells: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(spells_file);
                    if path.exists() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => match ron::from_str::<Vec<Spell>>(&contents) {
                                Ok(loaded_spells) => {
                                    *spells = loaded_spells;
                                    *status_message =
                                        format!("Loaded spells from: {}", path.display());
                                }
                                Err(e) => {
                                    *status_message = format!("Failed to parse spells: {}", e)
                                }
                            },
                            Err(e) => *status_message = format!("Failed to read spells: {}", e),
                        }
                    } else {
                        *status_message = "Spells file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        // Filter toolbar
        ui.horizontal(|ui| {
            ui.label("Filters:");

            // School filter
            ui.label("School:");
            if ui
                .button(match self.filter_school {
                    None => "All",
                    Some(SpellSchool::Cleric) => "Cleric",
                    Some(SpellSchool::Sorcerer) => "Sorcerer",
                })
                .clicked()
            {
                self.filter_school = match self.filter_school {
                    None => Some(SpellSchool::Cleric),
                    Some(SpellSchool::Cleric) => Some(SpellSchool::Sorcerer),
                    Some(SpellSchool::Sorcerer) => None,
                };
                self.selected_spell = None;
            }

            // Level filter
            ui.label("Level:");
            egui::ComboBox::from_id_salt("spell_level_filter")
                .selected_text(match self.filter_level {
                    None => "All".to_string(),
                    Some(lvl) => format!("{}", lvl),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_level.is_none(), "All")
                        .clicked()
                    {
                        self.filter_level = None;
                        self.selected_spell = None;
                    }
                    for level in 1..=7 {
                        if ui
                            .selectable_label(
                                self.filter_level == Some(level),
                                format!("{}", level),
                            )
                            .clicked()
                        {
                            self.filter_level = Some(level);
                            self.selected_spell = None;
                        }
                    }
                });

            ui.separator();
            ui.checkbox(&mut self.show_preview, "Preview");

            if ui.button("🔄 Clear Filters").clicked() {
                self.filter_school = None;
                self.filter_level = None;
            }
        });

        ui.separator();

        if self.show_import_dialog {
            self.show_import_dialog(
                ui.ctx(),
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                spells_file,
            );
        }

        match self.mode {
            SpellsEditorMode::List => self.show_list(
                ui,
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                spells_file,
            ),
            SpellsEditorMode::Add | SpellsEditorMode::Edit => self.show_form(
                ui,
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                spells_file,
            ),
        }
    }

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        spells: &mut Vec<Spell>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
    ) {
        let search_lower = self.search_query.to_lowercase();

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_spells: Vec<(usize, String, Spell)> = spells
            .iter()
            .enumerate()
            .filter(|(_, spell)| {
                if !search_lower.is_empty() && !spell.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                if let Some(school) = self.filter_school {
                    if spell.school != school {
                        return false;
                    }
                }
                if let Some(level) = self.filter_level {
                    if spell.level != level {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, spell)| {
                let school_icon = match spell.school {
                    SpellSchool::Cleric => "✝️",
                    SpellSchool::Sorcerer => "🔮",
                };
                (
                    idx,
                    format!("{} L{}: {}", school_icon, spell.level, spell.name),
                    spell.clone(),
                )
            })
            .collect();

        // Sort by ID
        let mut sorted_spells = filtered_spells;
        sorted_spells.sort_by_key(|(idx, _, _)| spells[*idx].id);

        let selected = self.selected_spell;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;
        let show_preview = self.show_preview;

        // Use shared TwoColumnLayout component
        TwoColumnLayout::new("spells").show_split(
            ui,
            |left_ui| {
                // Left panel: Spells list
                left_ui.heading("Spells");
                left_ui.separator();

                for (idx, label, _) in &sorted_spells {
                    let is_selected = selected == Some(*idx);
                    if left_ui.selectable_label(is_selected, label).clicked() {
                        new_selection = Some(*idx);
                    }
                }

                if sorted_spells.is_empty() {
                    left_ui.label("No spells found");
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, _, spell)) = sorted_spells.iter().find(|(i, _, _)| *i == idx) {
                        right_ui.heading(&spell.name);
                        right_ui.separator();

                        // Use shared ActionButtons component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();

                        if show_preview {
                            Self::show_preview_static(right_ui, spell);
                        } else {
                            Self::show_spell_details(right_ui, spell);
                        }
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select a spell to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a spell to view details");
                    });
                }
            },
        );

        // Apply selection change after closures
        self.selected_spell = new_selection;

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_spell {
                        if idx < spells.len() {
                            self.mode = SpellsEditorMode::Edit;
                            self.edit_buffer = spells[idx].clone();
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_spell {
                        if idx < spells.len() {
                            spells.remove(idx);
                            self.selected_spell = None;
                            self.save_spells(
                                spells,
                                campaign_dir,
                                spells_file,
                                unsaved_changes,
                                status_message,
                            );
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_spell {
                        if idx < spells.len() {
                            let mut new_spell = spells[idx].clone();
                            let next_id = spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                            new_spell.id = next_id;
                            new_spell.name = format!("{} (Copy)", new_spell.name);
                            spells.push(new_spell);
                            self.save_spells(
                                spells,
                                campaign_dir,
                                spells_file,
                                unsaved_changes,
                                status_message,
                            );
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_spell {
                        if idx < spells.len() {
                            if let Ok(ron_str) = ron::ser::to_string_pretty(
                                &spells[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                self.import_export_buffer = ron_str;
                                self.show_import_dialog = true;
                                *status_message = "Spell exported to clipboard dialog".to_string();
                            } else {
                                *status_message = "Failed to export spell".to_string();
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Static spell details view that doesn't require self
    fn show_spell_details(ui: &mut egui::Ui, spell: &Spell) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label(format!("ID: {}", spell.id));
                    ui.label(format!("School: {:?}", spell.school));
                    ui.label(format!("Level: {}", spell.level));
                    ui.label(format!("SP Cost: {}", spell.sp_cost));
                    ui.label(format!("Gem Cost: {}", spell.gem_cost));
                    ui.label(format!("Context: {:?}", spell.context));
                    ui.label(format!("Target: {:?}", spell.target));
                    ui.separator();
                    ui.label("Description:");
                    ui.label(&spell.description);
                });
            });
    }

    /// Static preview method that doesn't require self
    fn show_preview_static(ui: &mut egui::Ui, spell: &Spell) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Spell Details");
                    ui.label(format!("ID: {}", spell.id));
                    ui.label(format!("School: {:?}", spell.school));
                    ui.label(format!("Level: {}", spell.level));
                    ui.label(format!("SP Cost: {}", spell.sp_cost));
                    ui.label(format!("Gem Cost: {}", spell.gem_cost));
                    ui.label(format!("Context: {:?}", spell.context));
                    ui.label(format!("Target: {:?}", spell.target));

                    if let Some(damage) = &spell.damage {
                        ui.label(format!(
                            "Damage: {}d{}{:+}",
                            damage.count, damage.sides, damage.bonus
                        ));
                    }

                    if spell.duration > 0 {
                        ui.label(format!("Duration: {} rounds", spell.duration));
                    }

                    if spell.saving_throw {
                        ui.label("Saving Throw: Allowed");
                    }

                    if !spell.applied_conditions.is_empty() {
                        ui.separator();
                        ui.label("Applied Conditions:");
                        for cond_id in &spell.applied_conditions {
                            ui.label(format!("  - {}", cond_id));
                        }
                    }

                    ui.separator();
                    ui.label("Description:");
                    ui.label(&spell.description);
                });
            });
    }

    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        spells: &mut Vec<Spell>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
    ) {
        let is_add = self.mode == SpellsEditorMode::Add;
        ui.heading(if is_add {
            "Add New Spell"
        } else {
            "Edit Spell"
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Properties");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut self.edit_buffer.id.to_string()),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.edit_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("School:");
                        egui::ComboBox::from_id_salt("spell_school")
                            .selected_text(format!("{:?}", self.edit_buffer.school))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.edit_buffer.school,
                                    SpellSchool::Cleric,
                                    "Cleric",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.school,
                                    SpellSchool::Sorcerer,
                                    "Sorcerer",
                                );
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Level:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.level).range(1..=7));
                    });

                    ui.horizontal(|ui| {
                        ui.label("SP Cost:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.sp_cost).range(0..=100));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Gem Cost:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.gem_cost).range(0..=100));
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Targeting");

                    ui.horizontal(|ui| {
                        ui.label("Context:");
                        egui::ComboBox::from_id_salt("spell_context")
                            .selected_text(format!("{:?}", self.edit_buffer.context))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.edit_buffer.context,
                                    SpellContext::CombatOnly,
                                    "Combat Only",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.context,
                                    SpellContext::NonCombatOnly,
                                    "Non-Combat",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.context,
                                    SpellContext::Anytime,
                                    "Anytime",
                                );
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        egui::ComboBox::from_id_salt("spell_target")
                            .selected_text(format!("{:?}", self.edit_buffer.target))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.edit_buffer.target,
                                    SpellTarget::SingleCharacter,
                                    "Single Character",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.target,
                                    SpellTarget::AllCharacters,
                                    "All Characters",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.target,
                                    SpellTarget::SingleMonster,
                                    "Single Monster",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.target,
                                    SpellTarget::AllMonsters,
                                    "All Monsters",
                                );
                                ui.selectable_value(
                                    &mut self.edit_buffer.target,
                                    SpellTarget::Self_,
                                    "Self",
                                );
                            });
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Effects");

                    let mut has_damage = self.edit_buffer.damage.is_some();
                    if ui.checkbox(&mut has_damage, "Has Damage").changed() {
                        if has_damage {
                            self.edit_buffer.damage = Some(DiceRoll::new(1, 6, 0));
                        } else {
                            self.edit_buffer.damage = None;
                        }
                    }

                    if let Some(ref mut damage) = self.edit_buffer.damage {
                        ui.horizontal(|ui| {
                            ui.label("Damage Dice:");
                            ui.add(egui::DragValue::new(&mut damage.count).range(1..=10));
                            ui.label("d");
                            ui.add(egui::DragValue::new(&mut damage.sides).range(1..=20));
                            ui.label("+");
                            ui.add(egui::DragValue::new(&mut damage.bonus).range(-10..=20));
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Duration (rounds):");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.duration).range(0..=100));
                    });

                    ui.checkbox(&mut self.edit_buffer.saving_throw, "Allows Saving Throw");
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Description");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.edit_buffer.description)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.add_space(10.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        if is_add {
                            spells.push(self.edit_buffer.clone());
                        } else if let Some(idx) = self.selected_spell {
                            if idx < spells.len() {
                                spells[idx] = self.edit_buffer.clone();
                            }
                        }
                        self.save_spells(
                            spells,
                            campaign_dir,
                            spells_file,
                            unsaved_changes,
                            status_message,
                        );
                        self.mode = SpellsEditorMode::List;
                        *status_message = "Spell saved".to_string();
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = SpellsEditorMode::List;
                    }
                });
            });
    }

    fn show_import_dialog(
        &mut self,
        ctx: &egui::Context,
        spells: &mut Vec<Spell>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
    ) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import/Export Spell")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Spell RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                let text_edit = egui::TextEdit::multiline(&mut self.import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Spell>(&self.import_export_buffer) {
                            Ok(mut spell) => {
                                let next_id = spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                                spell.id = next_id;
                                spells.push(spell);
                                self.save_spells(
                                    spells,
                                    campaign_dir,
                                    spells_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                *status_message = "Spell imported successfully".to_string();
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                *status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_export_buffer.clone());
                        *status_message = "Copied to clipboard".to_string();
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });

        self.show_import_dialog = open;
    }

    fn save_spells(
        &self,
        spells: &[Spell],
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let spells_path = dir.join(spells_file);

            // Create parent directories if necessary
            if let Some(parent) = spells_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    *status_message = format!("Failed to create directory: {}", e);
                    return;
                }
            }

            match ron::ser::to_string_pretty(spells, Default::default()) {
                Ok(contents) => match std::fs::write(&spells_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        *status_message =
                            format!("Auto-saved spells to: {}", spells_path.display());
                    }
                    Err(e) => {
                        *status_message = format!("Failed to save spells: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize spells: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SpellsEditorState Tests
    // =========================================================================

    #[test]
    fn test_spells_editor_state_new() {
        let state = SpellsEditorState::new();
        assert_eq!(state.mode, SpellsEditorMode::List);
        assert!(state.search_query.is_empty());
        assert!(state.selected_spell.is_none());
        assert!(!state.show_import_dialog);
        assert!(state.import_export_buffer.is_empty());
        assert!(!state.show_preview);
    }

    #[test]
    fn test_spells_editor_state_default() {
        let state = SpellsEditorState::default();
        assert_eq!(state.mode, SpellsEditorMode::List);
        assert!(state.filter_school.is_none());
        assert!(state.filter_level.is_none());
    }

    #[test]
    fn test_default_spell_creation() {
        let spell = SpellsEditorState::default_spell();
        assert_eq!(spell.id, 0);
        assert_eq!(spell.name, "New Spell");
        assert_eq!(spell.school, SpellSchool::Cleric);
        assert_eq!(spell.level, 1);
        assert_eq!(spell.sp_cost, 1);
        assert_eq!(spell.gem_cost, 0);
        assert_eq!(spell.context, SpellContext::Anytime);
        assert_eq!(spell.target, SpellTarget::SingleCharacter);
        assert!(spell.damage.is_none());
        assert_eq!(spell.duration, 0);
        assert!(!spell.saving_throw);
        assert!(spell.description.is_empty());
        assert!(spell.applied_conditions.is_empty());
    }

    // =========================================================================
    // SpellsEditorMode Tests
    // =========================================================================

    #[test]
    fn test_spells_editor_mode_variants() {
        assert_eq!(SpellsEditorMode::List, SpellsEditorMode::List);
        assert_eq!(SpellsEditorMode::Add, SpellsEditorMode::Add);
        assert_eq!(SpellsEditorMode::Edit, SpellsEditorMode::Edit);
        assert_ne!(SpellsEditorMode::List, SpellsEditorMode::Add);
    }

    // =========================================================================
    // Editor State Transitions Tests
    // =========================================================================

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = SpellsEditorState::new();
        assert_eq!(state.mode, SpellsEditorMode::List);

        state.mode = SpellsEditorMode::Add;
        assert_eq!(state.mode, SpellsEditorMode::Add);

        state.mode = SpellsEditorMode::Edit;
        assert_eq!(state.mode, SpellsEditorMode::Edit);

        state.mode = SpellsEditorMode::List;
        assert_eq!(state.mode, SpellsEditorMode::List);
    }

    #[test]
    fn test_selected_spell_handling() {
        let mut state = SpellsEditorState::new();
        assert!(state.selected_spell.is_none());

        state.selected_spell = Some(0);
        assert_eq!(state.selected_spell, Some(0));

        state.selected_spell = Some(5);
        assert_eq!(state.selected_spell, Some(5));

        state.selected_spell = None;
        assert!(state.selected_spell.is_none());
    }

    #[test]
    fn test_filter_combinations() {
        let mut state = SpellsEditorState::new();

        // Set school filter
        state.filter_school = Some(SpellSchool::Sorcerer);
        assert_eq!(state.filter_school, Some(SpellSchool::Sorcerer));

        // Set level filter
        state.filter_level = Some(3);
        assert_eq!(state.filter_level, Some(3));

        // Clear filters
        state.filter_school = None;
        state.filter_level = None;
        assert!(state.filter_school.is_none());
        assert!(state.filter_level.is_none());
    }

    #[test]
    fn test_edit_buffer_modification() {
        let mut state = SpellsEditorState::new();

        // Modify the edit buffer
        state.edit_buffer.name = "Fireball".to_string();
        state.edit_buffer.school = SpellSchool::Sorcerer;
        state.edit_buffer.level = 3;
        state.edit_buffer.sp_cost = 5;
        state.edit_buffer.gem_cost = 1;

        assert_eq!(state.edit_buffer.name, "Fireball");
        assert_eq!(state.edit_buffer.school, SpellSchool::Sorcerer);
        assert_eq!(state.edit_buffer.level, 3);
        assert_eq!(state.edit_buffer.sp_cost, 5);
        assert_eq!(state.edit_buffer.gem_cost, 1);
    }

    #[test]
    fn test_spell_context_values() {
        // Test that SpellContext variants are correctly used
        let mut spell = SpellsEditorState::default_spell();

        spell.context = SpellContext::CombatOnly;
        assert_eq!(spell.context, SpellContext::CombatOnly);

        spell.context = SpellContext::NonCombatOnly;
        assert_eq!(spell.context, SpellContext::NonCombatOnly);

        spell.context = SpellContext::Anytime;
        assert_eq!(spell.context, SpellContext::Anytime);
    }

    #[test]
    fn test_spell_target_values() {
        // Test that SpellTarget variants are correctly used
        let mut spell = SpellsEditorState::default_spell();

        spell.target = SpellTarget::SingleCharacter;
        assert_eq!(spell.target, SpellTarget::SingleCharacter);

        spell.target = SpellTarget::AllCharacters;
        assert_eq!(spell.target, SpellTarget::AllCharacters);

        spell.target = SpellTarget::SingleMonster;
        assert_eq!(spell.target, SpellTarget::SingleMonster);

        spell.target = SpellTarget::AllMonsters;
        assert_eq!(spell.target, SpellTarget::AllMonsters);
    }

    #[test]
    fn test_preview_toggle() {
        let mut state = SpellsEditorState::new();
        assert!(!state.show_preview);

        state.show_preview = true;
        assert!(state.show_preview);

        state.show_preview = false;
        assert!(!state.show_preview);
    }
}
