// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::character::Stats;
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
use antares::domain::types::DiceRoll;
use eframe::egui;
use std::path::PathBuf;

/// Editor mode for monsters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonstersEditorMode {
    List,
    Add,
    Edit,
}

/// State for the monsters editor
pub struct MonstersEditorState {
    pub mode: MonstersEditorMode,
    pub search_query: String,
    pub selected_monster: Option<usize>,
    pub edit_buffer: MonsterDefinition,
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
    pub show_preview: bool,

    // Editor toggles
    pub show_stats_editor: bool,
    pub show_attacks_editor: bool,
    pub show_loot_editor: bool,
}

impl Default for MonstersEditorState {
    fn default() -> Self {
        Self {
            mode: MonstersEditorMode::List,
            search_query: String::new(),
            selected_monster: None,
            edit_buffer: Self::default_monster(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            show_preview: false,
            show_stats_editor: false,
            show_attacks_editor: false,
            show_loot_editor: false,
        }
    }
}

impl MonstersEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_monster() -> MonsterDefinition {
        MonsterDefinition {
            id: 0,
            name: "New Monster".to_string(),
            hp: 10,
            ac: 10,
            attacks: vec![Attack {
                damage: DiceRoll::new(1, 6, 0),
                attack_type: AttackType::Physical,
                special_effect: None,
            }],
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            is_undead: false,
            can_regenerate: false,
            can_advance: false,
            magic_resistance: 0,
            resistances: MonsterResistances::default(),
            loot: LootTable::default(),
            flee_threshold: 0,
            special_attack_threshold: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        monsters: &mut Vec<MonsterDefinition>,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        ui.heading("👹 Monsters Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                self.selected_monster = None;
            }
            ui.separator();

            if ui.button("➕ Add Monster").clicked() {
                self.mode = MonstersEditorMode::Add;
                self.edit_buffer = Self::default_monster();
                let next_id = monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
            }

            if ui.button("🔄 Reload").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(monsters_file);
                    if path.exists() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => {
                                match ron::from_str::<Vec<MonsterDefinition>>(&contents) {
                                    Ok(loaded_monsters) => {
                                        *monsters = loaded_monsters;
                                        *status_message =
                                            format!("Loaded monsters from: {}", path.display());
                                    }
                                    Err(e) => {
                                        *status_message = format!("Failed to parse monsters: {}", e)
                                    }
                                }
                            }
                            Err(e) => *status_message = format!("Failed to read monsters: {}", e),
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
                        ron::from_str::<Vec<MonsterDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_monsters) => {
                            if *file_load_merge_mode {
                                for monster in loaded_monsters {
                                    if let Some(existing) =
                                        monsters.iter_mut().find(|m| m.id == monster.id)
                                    {
                                        *existing = monster;
                                    } else {
                                        monsters.push(monster);
                                    }
                                }
                            } else {
                                *monsters = loaded_monsters;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded monsters from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load monsters: {}", e);
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
                    .set_file_name("monsters.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(monsters, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved monsters to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save monsters: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize monsters: {}", e);
                        }
                    }
                }
            }

            ui.separator();
            ui.label(format!("Total: {}", monsters.len()));

            ui.checkbox(&mut self.show_preview, "Preview");
        });

        ui.separator();

        if self.show_import_dialog {
            self.show_import_dialog(
                ui.ctx(),
                monsters,
                unsaved_changes,
                status_message,
                campaign_dir,
                monsters_file,
            );
        }

        match self.mode {
            MonstersEditorMode::List => self.show_list(
                ui,
                monsters,
                unsaved_changes,
                status_message,
                campaign_dir,
                monsters_file,
            ),
            MonstersEditorMode::Add | MonstersEditorMode::Edit => self.show_form(
                ui,
                monsters,
                unsaved_changes,
                status_message,
                campaign_dir,
                monsters_file,
            ),
        }
    }

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        monsters: &mut Vec<MonsterDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
    ) {
        let search_lower = self.search_query.to_lowercase();
        let mut filtered_monsters: Vec<(usize, String)> = monsters
            .iter()
            .enumerate()
            .filter(|(_, monster)| {
                search_lower.is_empty() || monster.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, monster)| {
                let undead_icon = if monster.is_undead { "💀" } else { "👹" };
                (
                    idx,
                    format!("{} {} (HP:{})", undead_icon, monster.name, monster.hp),
                )
            })
            .collect();

        filtered_monsters.sort_by_key(|(idx, _)| monsters[*idx].id);

        let selected = self.selected_monster;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        // Compute panel height using shared helper to keep consistent across editors.
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(crate::ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH);
                ui.set_min_height(panel_height);

                ui.heading("Monsters");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("monsters_list_scroll")
                    .auto_shrink([false, false])
                    .max_height(panel_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for (idx, label) in &filtered_monsters {
                            let is_selected = selected == Some(*idx);
                            if ui.selectable_label(is_selected, label).clicked() {
                                new_selection = Some(*idx);
                            }
                        }

                        if filtered_monsters.is_empty() {
                            ui.label("No monsters found");
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_min_height(panel_height);
                ui.set_min_width(ui.available_width());
                if let Some(idx) = selected {
                    if idx < monsters.len() {
                        let monster = monsters[idx].clone();

                        ui.heading(&monster.name);
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
                            self.show_preview(ui, &monster);
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("monster_details_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.group(|ui| {
                                        ui.label(format!("ID: {}", monster.id));
                                        ui.label(format!("HP: {}", monster.hp));
                                        ui.label(format!("AC: {}", monster.ac));
                                        ui.label(format!("Attacks: {}", monster.attacks.len()));
                                        ui.label(format!("Undead: {}", monster.is_undead));
                                        ui.label(format!(
                                            "Can Regenerate: {}!",
                                            monster.can_regenerate
                                        ));
                                        ui.label(format!("Can Advance: {}!", monster.can_advance));
                                        ui.label(format!(
                                            "Magic Resistance: {}%!",
                                            monster.magic_resistance,
                                        ));
                                        ui.separator();
                                        ui.label("Loot:");
                                        ui.label(format!(
                                            "  Gold: {}-{} gp",
                                            monster.loot.gold_min, monster.loot.gold_max
                                        ));
                                        ui.label(format!(
                                            "  Gems: {}-{}",
                                            monster.loot.gems_min, monster.loot.gems_max
                                        ));
                                        ui.label(format!(
                                            "  Experience: {} XP",
                                            monster.loot.experience
                                        ));
                                    });
                                });
                        }
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a monster to view details");
                    });
                }
            });
        });

        self.selected_monster = new_selection;

        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.mode = MonstersEditorMode::Edit;
                    self.edit_buffer = monsters[idx].clone();
                }
                "delete" => {
                    monsters.remove(idx);
                    self.selected_monster = None;
                    self.save_monsters(
                        monsters,
                        campaign_dir,
                        monsters_file,
                        unsaved_changes,
                        status_message,
                    );
                }
                "duplicate" => {
                    let mut new_monster = monsters[idx].clone();
                    let next_id = monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                    new_monster.id = next_id;
                    new_monster.name = format!("{} (Copy)", new_monster.name);
                    monsters.push(new_monster);
                    self.save_monsters(
                        monsters,
                        campaign_dir,
                        monsters_file,
                        unsaved_changes,
                        status_message,
                    );
                }
                "export" => {
                    if let Ok(ron) = ron::to_string(&monsters[idx]) {
                        self.import_export_buffer = ron;
                        *status_message = "Monster exported to buffer".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    fn show_preview(&self, ui: &mut egui::Ui, monster: &MonsterDefinition) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading(&monster.name);
                    ui.separator();

                    ui.label(format!("🆔 ID: {}", monster.id));

                    let type_icon = if monster.is_undead { "💀" } else { "👹" };
                    ui.label(format!(
                        "{} Type: {}",
                        type_icon,
                        if monster.is_undead {
                            "Undead"
                        } else {
                            "Living"
                        }
                    ));

                    ui.separator();

                    ui.label("⚔️ Combat Stats:");
                    ui.label(format!("  ❤️ HP: {}", monster.hp));
                    ui.label(format!("  🛡️ AC: {}", monster.ac));
                    ui.label(format!("  ⚡ Attacks: {}", monster.attacks.len()));

                    if monster.magic_resistance > 0 {
                        ui.label(format!(
                            "  🔮 Magic Resistance: {}%",
                            monster.magic_resistance
                        ));
                    }

                    ui.separator();

                    ui.label("🎲 Attributes:");
                    ui.label(format!("  Might: {}", monster.stats.might.base));
                    ui.label(format!("  Intellect: {}", monster.stats.intellect.base));
                    ui.label(format!("  Personality: {}", monster.stats.personality.base));
                    ui.label(format!("  Endurance: {}", monster.stats.endurance.base));
                    ui.label(format!("  Speed: {}", monster.stats.speed.base));
                    ui.label(format!("  Accuracy: {}", monster.stats.accuracy.base));
                    ui.label(format!("  Luck: {}", monster.stats.luck.base));

                    ui.separator();

                    ui.label("⚙️ Special Abilities:");
                    if monster.can_regenerate {
                        ui.label("  ♻️ Can Regenerate");
                    }
                    if monster.can_advance {
                        ui.label("  🏃 Can Advance");
                    }

                    ui.separator();

                    ui.label("💰 Loot:");
                    if monster.loot.gold_max > 0 {
                        ui.label(format!(
                            "  Gold: {}-{} gp",
                            monster.loot.gold_min, monster.loot.gold_max
                        ));
                    }
                    if monster.loot.gems_max > 0 {
                        ui.label(format!(
                            "  Gems: {}-{}",
                            monster.loot.gems_min, monster.loot.gems_max
                        ));
                    }
                    ui.label(format!("  Experience: {} XP", monster.loot.experience));
                });
            });
    }

    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        monsters: &mut Vec<MonsterDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
    ) {
        let is_add = self.mode == MonstersEditorMode::Add;
        ui.heading(if is_add {
            "Add New Monster"
        } else {
            "Edit Monster"
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
                    ui.label("HP:");
                    ui.add(egui::DragValue::new(&mut self.edit_buffer.hp).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("AC:");
                    ui.add(egui::DragValue::new(&mut self.edit_buffer.ac).speed(1.0));
                });

                ui.checkbox(&mut self.edit_buffer.is_undead, "Undead");
                ui.checkbox(&mut self.edit_buffer.can_advance, "Can Advance");

                ui.horizontal(|ui| {
                    ui.label("Magic Resistance:");
                    ui.add(egui::Slider::new(
                        &mut self.edit_buffer.magic_resistance,
                        0..=100,
                    ));
                });

                ui.separator();
                if ui
                    .button(if self.show_stats_editor {
                        "▼ Stats"
                    } else {
                        "▶ Stats"
                    })
                    .clicked()
                {
                    self.show_stats_editor = !self.show_stats_editor;
                }

                if self.show_stats_editor {
                    ui.group(|ui| {
                        self.show_stats_editor(ui);
                    });
                }

                ui.separator();
                if ui
                    .button(if self.show_attacks_editor {
                        "▼ Attacks"
                    } else {
                        "▶ Attacks"
                    })
                    .clicked()
                {
                    self.show_attacks_editor = !self.show_attacks_editor;
                }

                if self.show_attacks_editor {
                    ui.group(|ui| {
                        self.show_attacks_editor(ui);
                    });
                }

                ui.separator();
                if ui
                    .button(if self.show_loot_editor {
                        "▼ Loot Table"
                    } else {
                        "▶ Loot Table"
                    })
                    .clicked()
                {
                    self.show_loot_editor = !self.show_loot_editor;
                }

                if self.show_loot_editor {
                    ui.group(|ui| {
                        self.show_loot_editor(ui);
                    });
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        if is_add {
                            monsters.push(self.edit_buffer.clone());
                        } else if let Some(idx) = self.selected_monster {
                            if idx < monsters.len() {
                                monsters[idx] = self.edit_buffer.clone();
                            }
                        }
                        self.save_monsters(
                            monsters,
                            campaign_dir,
                            monsters_file,
                            unsaved_changes,
                            status_message,
                        );
                        self.mode = MonstersEditorMode::List;
                        *status_message = "Monster saved".to_string();
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = MonstersEditorMode::List;
                    }
                });
            });
    }

    fn show_stats_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Attributes:");

        ui.horizontal(|ui| {
            ui.label("Might:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.might.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Intellect:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.intellect.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Personality:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.personality.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Endurance:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.endurance.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Speed:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.speed.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Accuracy:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.accuracy.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Luck:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.stats.luck.base)
                    .speed(1.0)
                    .range(0..=255),
            );
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Flee Threshold:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.flee_threshold).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Special Attack %:");
            ui.add(egui::Slider::new(
                &mut self.edit_buffer.special_attack_threshold,
                0..=100,
            ));
        });
    }

    fn show_attacks_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Attacks ({})", self.edit_buffer.attacks.len()));

        if ui.button("➕ Add Attack").clicked() {
            self.edit_buffer.attacks.push(Attack {
                damage: DiceRoll::new(1, 6, 0),
                attack_type: AttackType::Physical,
                special_effect: None,
            });
        }

        ui.separator();

        let mut to_remove: Option<usize> = None;

        for (idx, attack) in self.edit_buffer.attacks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Attack {}", idx + 1));
                    if ui.button("🗑️").clicked() {
                        to_remove = Some(idx);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Damage:");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.count)
                            .speed(1.0)
                            .range(1..=10)
                            .prefix("d"),
                    );
                    ui.label("d");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.sides)
                            .speed(1.0)
                            .range(2..=100),
                    );
                    ui.label("+");
                    ui.add(
                        egui::DragValue::new(&mut attack.damage.bonus)
                            .speed(1.0)
                            .range(-10..=100),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt(format!("attack_type_{}", idx))
                        .selected_text(format!("{:?}", attack.attack_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Physical,
                                "Physical",
                            );
                            ui.selectable_value(&mut attack.attack_type, AttackType::Fire, "Fire");
                            ui.selectable_value(&mut attack.attack_type, AttackType::Cold, "Cold");
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Electricity,
                                "Electricity",
                            );
                            ui.selectable_value(&mut attack.attack_type, AttackType::Acid, "Acid");
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Poison,
                                "Poison",
                            );
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Energy,
                                "Energy",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Special Effect:");
                    egui::ComboBox::from_id_salt(format!("special_effect_{}", idx))
                        .selected_text(match attack.special_effect {
                            None => "None",
                            Some(SpecialEffect::Poison) => "Poison",
                            Some(SpecialEffect::Disease) => "Disease",
                            Some(SpecialEffect::Paralysis) => "Paralysis",
                            Some(SpecialEffect::Sleep) => "Sleep",
                            Some(SpecialEffect::Drain) => "Drain",
                            Some(SpecialEffect::Stone) => "Stone",
                            Some(SpecialEffect::Death) => "Death",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut attack.special_effect, None, "None");
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Poison),
                                "Poison",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Disease),
                                "Disease",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Paralysis),
                                "Paralysis",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Sleep),
                                "Sleep",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Drain),
                                "Drain",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Stone),
                                "Stone",
                            );
                            ui.selectable_value(
                                &mut attack.special_effect,
                                Some(SpecialEffect::Death),
                                "Death",
                            );
                        });
                });
            });
        }

        if let Some(idx) = to_remove {
            self.edit_buffer.attacks.remove(idx);
        }
    }

    fn show_loot_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Loot Table:");

        ui.horizontal(|ui| {
            ui.label("Gold Min:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.gold_min).speed(1.0));
            ui.label("Max:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.gold_max).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Gems Min:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.gems_min).speed(1.0));
            ui.label("Max:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.gems_max).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Experience:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.experience).speed(10.0));
        });

        ui.separator();

        let calculated_xp = self.calculate_monster_xp(&self.edit_buffer);
        ui.label(format!(
            "💡 Suggested XP: {} (based on stats)",
            calculated_xp
        ));

        if ui.button("Use Suggested XP").clicked() {
            self.edit_buffer.loot.experience = calculated_xp;
        }
    }

    pub fn calculate_monster_xp(&self, monster: &MonsterDefinition) -> u32 {
        let mut xp = monster.hp as u32 * 10;

        if monster.ac < 10 {
            xp += (10 - monster.ac as u32) * 50;
        }

        xp += monster.attacks.len() as u32 * 20;

        for attack in &monster.attacks {
            let avg_damage = (attack.damage.count as f32 * (attack.damage.sides as f32 / 2.0))
                + attack.damage.bonus as f32;
            xp += (avg_damage * 5.0) as u32;

            if attack.special_effect.is_some() {
                xp += 50;
            }
        }

        if monster.can_regenerate {
            xp += 100;
        }
        if monster.is_undead {
            xp += 50;
        }
        if monster.magic_resistance > 0 {
            xp += monster.magic_resistance as u32 * 2;
        }

        xp
    }

    fn show_import_dialog(
        &mut self,
        ctx: &egui::Context,
        monsters: &mut Vec<MonsterDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
    ) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import Monster")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Monster RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                let text_edit = egui::TextEdit::multiline(&mut self.import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<MonsterDefinition>(&self.import_export_buffer) {
                            Ok(mut monster) => {
                                let next_id = monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                                monster.id = next_id;
                                monsters.push(monster);
                                self.save_monsters(
                                    monsters,
                                    campaign_dir,
                                    monsters_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                *status_message = "Monster imported successfully".to_string();
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

    fn save_monsters(
        &self,
        monsters: &Vec<MonsterDefinition>,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let monsters_path = dir.join(monsters_file);

            if let Some(parent) = monsters_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            match ron::ser::to_string_pretty(monsters, ron_config) {
                Ok(contents) => match std::fs::write(&monsters_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                    }
                    Err(e) => {
                        *status_message = format!("Failed to write monsters file: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize monsters: {}", e);
                }
            }
        }
    }
}
