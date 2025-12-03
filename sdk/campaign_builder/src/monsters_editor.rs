// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::ui_helpers::{
    ActionButtons, AttributePair16Input, AttributePairInput, EditorToolbar, ItemAction,
    ToolbarAction, TwoColumnLayout,
};
use antares::domain::character::{AttributePair, AttributePair16, Stats};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
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
            hp: AttributePair16::new(10),
            ac: AttributePair::new(10),
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
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
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

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Monsters")
            .with_search(&mut self.search_query)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(monsters.len())
            .with_id_salt("monsters_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.mode = MonstersEditorMode::Add;
                self.edit_buffer = Self::default_monster();
                let next_id = monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                self.save_monsters(
                    monsters,
                    campaign_dir,
                    monsters_file,
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
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }
            ToolbarAction::Export => {
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
            ToolbarAction::Reload => {
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
                    } else {
                        *status_message = "Monsters file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        // Filter toolbar
        ui.horizontal(|ui| {
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

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_monsters: Vec<(usize, String, MonsterDefinition)> = monsters
            .iter()
            .enumerate()
            .filter(|(_, monster)| {
                search_lower.is_empty() || monster.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, monster)| {
                let undead_icon = if monster.is_undead { "💀" } else { "👹" };
                (
                    idx,
                    format!("{} {} (HP:{})", undead_icon, monster.name, monster.hp.base),
                    monster.clone(),
                )
            })
            .collect();

        // Sort by ID
        let mut sorted_monsters = filtered_monsters;
        sorted_monsters.sort_by_key(|(idx, _, _)| monsters[*idx].id);

        let selected = self.selected_monster;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;
        let show_preview = self.show_preview;

        // Use shared TwoColumnLayout component
        TwoColumnLayout::new("monsters").show_split(
            ui,
            |left_ui| {
                // Left panel: Monsters list
                left_ui.heading("Monsters");
                left_ui.separator();

                for (idx, label, _) in &sorted_monsters {
                    let is_selected = selected == Some(*idx);
                    if left_ui.selectable_label(is_selected, label).clicked() {
                        new_selection = Some(*idx);
                    }
                }

                if sorted_monsters.is_empty() {
                    left_ui.label("No monsters found");
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, _, monster)) =
                        sorted_monsters.iter().find(|(i, _, _)| *i == idx)
                    {
                        right_ui.heading(&monster.name);
                        right_ui.separator();

                        // Use shared ActionButtons component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();

                        if show_preview {
                            Self::show_preview_static(right_ui, monster);
                        } else {
                            Self::show_monster_details(right_ui, monster);
                        }
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select a monster to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a monster to view details");
                    });
                }
            },
        );

        // Apply selection change after closures
        self.selected_monster = new_selection;

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_monster {
                        if idx < monsters.len() {
                            self.mode = MonstersEditorMode::Edit;
                            self.edit_buffer = monsters[idx].clone();
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_monster {
                        if idx < monsters.len() {
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
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_monster {
                        if idx < monsters.len() {
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
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_monster {
                        if idx < monsters.len() {
                            if let Ok(ron_str) = ron::ser::to_string_pretty(
                                &monsters[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                self.import_export_buffer = ron_str;
                                self.show_import_dialog = true;
                                *status_message =
                                    "Monster exported to clipboard dialog".to_string();
                            } else {
                                *status_message = "Failed to export monster".to_string();
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Static monster details view that doesn't require self
    fn show_monster_details(ui: &mut egui::Ui, monster: &MonsterDefinition) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .id_salt("monster_details_scroll")
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label(format!("ID: {}", monster.id));
                    ui.label(format!("HP: {}", monster.hp.base));
                    ui.label(format!("AC: {}", monster.ac.base));
                    ui.label(format!("Attacks: {}", monster.attacks.len()));
                    ui.label(format!("Undead: {}", monster.is_undead));
                    ui.label(format!("Can Regenerate: {}", monster.can_regenerate));
                    ui.label(format!("Can Advance: {}", monster.can_advance));
                    ui.label(format!("Magic Resistance: {}%", monster.magic_resistance));
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
                    ui.label(format!("  Experience: {} XP", monster.loot.experience));
                });
            });
    }

    /// Static preview method that doesn't require self
    fn show_preview_static(ui: &mut egui::Ui, monster: &MonsterDefinition) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .max_height(panel_height)
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
                    ui.label(format!("  ❤️ HP: {}", monster.hp.base));
                    ui.label(format!("  🛡️ AC: {}", monster.ac.base));
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
                    if !monster.can_regenerate && !monster.can_advance {
                        ui.label("  None");
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
                    if monster.loot.experience > 0 {
                        ui.label(format!("  Experience: {} XP", monster.loot.experience));
                    }
                });
            });
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

        egui::Window::new("Import/Export Monster")
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

                    ui.checkbox(&mut self.edit_buffer.is_undead, "💀 Undead");
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Combat Stats");

                    // Use AttributePair widgets for HP and AC
                    AttributePair16Input::new("HP", &mut self.edit_buffer.hp)
                        .with_id_salt("monster_hp")
                        .with_reset_button(true)
                        .show(ui);

                    AttributePairInput::new("AC", &mut self.edit_buffer.ac)
                        .with_id_salt("monster_ac")
                        .with_reset_button(true)
                        .show(ui);

                    ui.horizontal(|ui| {
                        ui.label("Magic Resistance:");
                        ui.add(
                            egui::DragValue::new(&mut self.edit_buffer.magic_resistance)
                                .range(0..=100)
                                .suffix("%"),
                        );
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Attributes");

                    // Use AttributePair widgets for all stats
                    AttributePairInput::new("Might", &mut self.edit_buffer.stats.might)
                        .with_id_salt("monster_might")
                        .show(ui);

                    AttributePairInput::new("Intellect", &mut self.edit_buffer.stats.intellect)
                        .with_id_salt("monster_intellect")
                        .show(ui);

                    AttributePairInput::new("Personality", &mut self.edit_buffer.stats.personality)
                        .with_id_salt("monster_personality")
                        .show(ui);

                    AttributePairInput::new("Endurance", &mut self.edit_buffer.stats.endurance)
                        .with_id_salt("monster_endurance")
                        .show(ui);

                    AttributePairInput::new("Speed", &mut self.edit_buffer.stats.speed)
                        .with_id_salt("monster_speed")
                        .show(ui);

                    AttributePairInput::new("Accuracy", &mut self.edit_buffer.stats.accuracy)
                        .with_id_salt("monster_accuracy")
                        .show(ui);

                    AttributePairInput::new("Luck", &mut self.edit_buffer.stats.luck)
                        .with_id_salt("monster_luck")
                        .show(ui);
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Special Abilities");

                    ui.checkbox(&mut self.edit_buffer.can_regenerate, "♻️ Can Regenerate");
                    ui.checkbox(&mut self.edit_buffer.can_advance, "🏃 Can Advance");

                    ui.horizontal(|ui| {
                        ui.label("Flee Threshold:");
                        ui.add(
                            egui::DragValue::new(&mut self.edit_buffer.flee_threshold)
                                .range(0..=100)
                                .suffix("%"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Special Attack Threshold:");
                        ui.add(
                            egui::DragValue::new(&mut self.edit_buffer.special_attack_threshold)
                                .range(0..=100)
                                .suffix("%"),
                        );
                    });
                });

                ui.add_space(10.0);

                // Attacks editor
                ui.collapsing("⚔️ Attacks", |ui| {
                    self.show_attacks_editor(ui);
                });

                ui.add_space(10.0);

                // Loot editor
                ui.collapsing("💰 Loot", |ui| {
                    self.show_loot_editor(ui);
                });

                ui.add_space(10.0);
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

    fn show_attacks_editor(&mut self, ui: &mut egui::Ui) {
        let mut attacks_to_remove: Vec<usize> = Vec::new();

        for (i, attack) in self.edit_buffer.attacks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Attack {}:", i + 1));
                    if ui.button("🗑️").clicked() {
                        attacks_to_remove.push(i);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Damage:");
                    ui.add(egui::DragValue::new(&mut attack.damage.count).range(1..=10));
                    ui.label("d");
                    ui.add(egui::DragValue::new(&mut attack.damage.sides).range(1..=20));
                    ui.label("+");
                    ui.add(egui::DragValue::new(&mut attack.damage.bonus).range(-10..=20));
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt(format!("attack_type_{}", i))
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
                                AttackType::Electric,
                                "Electric",
                            );
                            ui.selectable_value(&mut attack.attack_type, AttackType::Acid, "Acid");
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Poison,
                                "Poison",
                            );
                            ui.selectable_value(
                                &mut attack.attack_type,
                                AttackType::Magic,
                                "Magic",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    let mut has_special = attack.special_effect.is_some();
                    if ui.checkbox(&mut has_special, "Special Effect").changed() {
                        if has_special {
                            attack.special_effect = Some(SpecialEffect::Poison);
                        } else {
                            attack.special_effect = None;
                        }
                    }

                    if let Some(ref mut effect) = attack.special_effect {
                        egui::ComboBox::from_id_salt(format!("special_effect_{}", i))
                            .selected_text(format!("{:?}", effect))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(effect, SpecialEffect::Poison, "Poison");
                                ui.selectable_value(effect, SpecialEffect::Paralyze, "Paralyze");
                                ui.selectable_value(effect, SpecialEffect::Sleep, "Sleep");
                                ui.selectable_value(effect, SpecialEffect::Stone, "Stone");
                                ui.selectable_value(
                                    effect,
                                    SpecialEffect::DrainLevel,
                                    "Drain Level",
                                );
                                ui.selectable_value(effect, SpecialEffect::DrainStat, "Drain Stat");
                            });
                    }
                });
            });
        }

        // Remove marked attacks
        for idx in attacks_to_remove.into_iter().rev() {
            self.edit_buffer.attacks.remove(idx);
        }

        if ui.button("➕ Add Attack").clicked() {
            self.edit_buffer.attacks.push(Attack {
                damage: DiceRoll::new(1, 6, 0),
                attack_type: AttackType::Physical,
                special_effect: None,
            });
        }
    }

    fn show_loot_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Gold:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.loot.gold_min)
                    .range(0..=65535)
                    .prefix("Min: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.loot.gold_max)
                    .range(0..=65535)
                    .prefix("Max: "),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Gems:");
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.loot.gems_min)
                    .range(0..=255)
                    .prefix("Min: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.edit_buffer.loot.gems_max)
                    .range(0..=255)
                    .prefix("Max: "),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Experience:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.loot.experience).range(0..=65535));
        });

        // Item drops
        ui.separator();
        ui.label("Item Drops:");

        let mut items_to_remove: Vec<usize> = Vec::new();
        for (i, (item_id, chance)) in self.edit_buffer.loot.item_drops.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("Item {}:", i + 1));
                ui.add(egui::DragValue::new(item_id).prefix("ID: "));
                ui.add(
                    egui::DragValue::new(chance)
                        .range(0..=100)
                        .suffix("%")
                        .prefix("Chance: "),
                );
                if ui.button("🗑️").clicked() {
                    items_to_remove.push(i);
                }
            });
        }

        for idx in items_to_remove.into_iter().rev() {
            self.edit_buffer.loot.item_drops.remove(idx);
        }

        if ui.button("➕ Add Item Drop").clicked() {
            self.edit_buffer.loot.item_drops.push((0, 10));
        }
    }

    fn save_monsters(
        &self,
        monsters: &[MonsterDefinition],
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let monsters_path = dir.join(monsters_file);

            // Create parent directories if necessary
            if let Some(parent) = monsters_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    *status_message = format!("Failed to create directory: {}", e);
                    return;
                }
            }

            match ron::ser::to_string_pretty(monsters, Default::default()) {
                Ok(contents) => match std::fs::write(&monsters_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        *status_message =
                            format!("Auto-saved monsters to: {}", monsters_path.display());
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
}
