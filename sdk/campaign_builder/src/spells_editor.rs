// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

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

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                self.selected_spell = None;
            }
            ui.separator();

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

            if ui.button("➕ Add Spell").clicked() {
                self.mode = SpellsEditorMode::Add;
                self.edit_buffer = Self::default_spell();
                let next_id = spells.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
            }

            if ui.button("🔄 Reload").clicked() {
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
                    }
                }
            }

            if ui.button("📥 Import").clicked() {
                self.show_import_dialog = true;
            }

            ui.separator();

            // File I/O buttons
            if ui.button("📂 Load from File").clicked() {
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

            ui.checkbox(file_load_merge_mode, "Merge");
            ui.label(if *file_load_merge_mode {
                "(adds to existing)"
            } else {
                "(replaces all)"
            });

            if ui.button("💾 Save to File").clicked() {
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

            ui.separator();
            ui.label(format!("Total: {}", spells.len()));

            ui.checkbox(&mut self.show_preview, "Preview");
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
        let mut filtered_spells: Vec<(usize, String)> = spells
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
                )
            })
            .collect();

        filtered_spells.sort_by_key(|(idx, _)| spells[*idx].id);

        let selected = self.selected_spell;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(crate::ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH);
                ui.set_min_height(panel_height);

                ui.heading("Spells");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("spells_list_scroll")
                    .auto_shrink([false, false])
                    .max_height(panel_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for (idx, label) in &filtered_spells {
                            let is_selected = selected == Some(*idx);
                            if ui.selectable_label(is_selected, label).clicked() {
                                new_selection = Some(*idx);
                            }
                        }

                        if filtered_spells.is_empty() {
                            ui.label("No spells found");
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_min_height(panel_height);
                ui.set_min_width(ui.available_width());
                if let Some(idx) = selected {
                    if idx < spells.len() {
                        let spell = spells[idx].clone();

                        ui.heading(&spell.name);
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("✏️ Edit").clicked() {
                                action = Some((idx, "edit"));
                            }
                            if ui.button("🗑️ Delete").clicked() {
                                action = Some((idx, "delete"));
                            }
                            if ui.button("📋 Duplicate").clicked() {
                                action = Some((idx, "duplicate"));
                            }
                            if ui.button("📤 Export").clicked() {
                                action = Some((idx, "export"));
                            }
                        });

                        ui.separator();

                        if self.show_preview {
                            self.show_preview(ui, &spell);
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(panel_height)
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
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a spell to view details");
                    });
                }
            });
        });

        self.selected_spell = new_selection;

        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.mode = SpellsEditorMode::Edit;
                    self.edit_buffer = spells[idx].clone();
                }
                "delete" => {
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
                "duplicate" => {
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
                "export" => {
                    if let Ok(ron) = ron::to_string(&spells[idx]) {
                        self.import_export_buffer = ron;
                        *status_message = "Spell exported to buffer".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    fn show_preview(&self, ui: &mut egui::Ui, spell: &Spell) {
        egui::ScrollArea::vertical().show(ui, |ui| {
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
                    ui.label(format!("Damage: {:?} + {}", damage, damage.bonus));
                }

                if spell.duration > 0 {
                    ui.label(format!("Duration: {} rounds", spell.duration));
                }

                if spell.saving_throw {
                    ui.label("Saving Throw: Allowed");
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
                    ui.radio_value(&mut self.edit_buffer.school, SpellSchool::Cleric, "Cleric");
                    ui.radio_value(
                        &mut self.edit_buffer.school,
                        SpellSchool::Sorcerer,
                        "Sorcerer",
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Level:");
                    ui.add(egui::Slider::new(&mut self.edit_buffer.level, 1..=7));
                });

                ui.horizontal(|ui| {
                    ui.label("SP Cost:");
                    ui.add(egui::DragValue::new(&mut self.edit_buffer.sp_cost).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Gem Cost:");
                    ui.add(egui::DragValue::new(&mut self.edit_buffer.gem_cost).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Context:");
                    egui::ComboBox::from_id_salt("spell_context")
                        .selected_text(format!("{:?}", self.edit_buffer.context))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::Anytime,
                                "Anytime",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::CombatOnly,
                                "CombatOnly",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::NonCombatOnly,
                                "NonCombatOnly",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::OutdoorOnly,
                                "OutdoorOnly",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::IndoorOnly,
                                "IndoorOnly",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.context,
                                SpellContext::OutdoorCombat,
                                "OutdoorCombat",
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
                                SpellTarget::Self_,
                                "Self",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::SingleCharacter,
                                "SingleCharacter",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::AllCharacters,
                                "AllCharacters",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::SingleMonster,
                                "SingleMonster",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::MonsterGroup,
                                "MonsterGroup",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::AllMonsters,
                                "AllMonsters",
                            );
                            ui.selectable_value(
                                &mut self.edit_buffer.target,
                                SpellTarget::SpecificMonsters,
                                "SpecificMonsters",
                            );
                        });
                });

                ui.separator();
                ui.label("Effects:");

                ui.horizontal(|ui| {
                    ui.label("Damage:");
                    let mut has_damage = self.edit_buffer.damage.is_some();
                    if ui.checkbox(&mut has_damage, "Has Damage").changed() {
                        if has_damage {
                            self.edit_buffer.damage = Some(DiceRoll::new(1, 6, 0));
                        } else {
                            self.edit_buffer.damage = None;
                        }
                    }

                    if let Some(damage) = &mut self.edit_buffer.damage {
                        ui.label("Count:");
                        ui.add(
                            egui::DragValue::new(&mut damage.count)
                                .speed(1.0)
                                .range(1..=100),
                        );
                        ui.label("d");
                        ui.add(
                            egui::DragValue::new(&mut damage.sides)
                                .speed(1.0)
                                .range(2..=100),
                        );
                        ui.label("+");
                        ui.add(egui::DragValue::new(&mut damage.bonus).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Duration (rounds):");
                    ui.add(egui::DragValue::new(&mut self.edit_buffer.duration).speed(1.0));
                    ui.label("(0 = Instant)");
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.edit_buffer.saving_throw, "Saving Throw Allowed");
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                });
                ui.text_edit_multiline(&mut self.edit_buffer.description);

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
        spells: &Vec<Spell>,
        campaign_dir: Option<&PathBuf>,
        spells_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let spells_path = dir.join(spells_file);

            if let Some(parent) = spells_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            match ron::ser::to_string_pretty(spells, ron_config) {
                Ok(contents) => match std::fs::write(&spells_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                    }
                    Err(e) => {
                        *status_message = format!("Failed to write spells file: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize spells: {}", e);
                }
            }
        }
    }
}
