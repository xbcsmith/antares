// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::creature_assets::CreatureAssetManager;
use crate::ui_helpers::{
    show_standard_list_item, AttributePair16Input, AttributePairInput, EditorToolbar, ItemAction,
    MetadataBadge, StandardListItemConfig, ToolbarAction, TwoColumnLayout,
};
use antares::domain::character::{AttributePair, AttributePair16, Stats};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
use antares::domain::types::{CreatureId, DiceRoll};
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

    // Autocomplete input buffers
    pub monster_name_input_buffer: String,

    // Creature asset picker
    pub creature_picker_open: bool,
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
            monster_name_input_buffer: String::new(),
            creature_picker_open: false,
        }
    }
}

impl MonstersEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the creature ID on the edit buffer and closes the picker.
    pub fn apply_selected_creature_id(&mut self, id: Option<CreatureId>) {
        self.edit_buffer.creature_id = id;
        self.creature_picker_open = false;
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
                is_ranged: false,
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
            creature_id: None,
        }
    }

    /// Shows the monsters editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `monsters` - Mutable reference to the monsters list for editing
    /// * `campaign_dir` - Optional campaign directory path for file operations
    /// * `monsters_file` - Name of the monsters file
    /// * `unsaved_changes` - Flag to track if there are unsaved changes
    /// * `status_message` - Status message to display to user
    /// * `file_load_merge_mode` - Whether to merge or replace when loading files
    /// * `creature_manager` - Optional creature asset manager for visual asset binding
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
        creature_manager: Option<&CreatureAssetManager>,
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
            MonstersEditorMode::Add | MonstersEditorMode::Edit => {
                // Initialize autocomplete buffer with current monster name when entering edit mode
                if self.monster_name_input_buffer.is_empty() && !self.edit_buffer.name.is_empty() {
                    self.monster_name_input_buffer = self.edit_buffer.name.clone();
                }

                self.show_form(
                    ui,
                    monsters,
                    unsaved_changes,
                    status_message,
                    campaign_dir,
                    monsters_file,
                    creature_manager,
                )
            }
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
        let filtered_monsters: Vec<(usize, MonsterDefinition)> = monsters
            .iter()
            .enumerate()
            .filter(|(_, monster)| {
                search_lower.is_empty() || monster.name.to_lowercase().contains(&search_lower)
            })
            .map(|(idx, monster)| (idx, monster.clone()))
            .collect();

        // Sort by ID
        let mut sorted_monsters = filtered_monsters;
        sorted_monsters.sort_by_key(|(idx, _)| monsters[*idx].id);

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

                for (idx, monster) in &sorted_monsters {
                    let mut badges = Vec::new();

                    // HP badge
                    badges.push(
                        MetadataBadge::new(format!("HP:{}", monster.hp.base))
                            .with_color(egui::Color32::from_rgb(200, 100, 100))
                            .with_tooltip("Hit Points"),
                    );

                    // AC badge
                    badges.push(
                        MetadataBadge::new(format!("AC:{}", monster.ac.base))
                            .with_color(egui::Color32::from_rgb(100, 100, 200))
                            .with_tooltip("Armor Class"),
                    );

                    // Undead badge
                    if monster.is_undead {
                        badges.push(
                            MetadataBadge::new("Undead")
                                .with_color(egui::Color32::from_rgb(139, 0, 139))
                                .with_tooltip("Undead creature"),
                        );
                    }

                    // Attacks badge
                    if !monster.attacks.is_empty() {
                        badges.push(
                            MetadataBadge::new(format!("Attacks:{}", monster.attacks.len()))
                                .with_color(egui::Color32::from_rgb(255, 165, 0))
                                .with_tooltip("Number of attacks"),
                        );
                    }

                    let icon = if monster.is_undead { "💀" } else { "👹" };

                    let config = StandardListItemConfig::new(&monster.name)
                        .with_badges(badges)
                        .with_id(monster.id)
                        .selected(selected == Some(*idx))
                        .with_icon(icon);

                    let (clicked, ctx_action) = show_standard_list_item(left_ui, config);

                    if clicked {
                        new_selection = Some(*idx);
                    }

                    if ctx_action != ItemAction::None {
                        action_requested = Some(ctx_action);
                    }
                }

                if sorted_monsters.is_empty() {
                    left_ui.label("No monsters found");
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, monster)) = sorted_monsters.iter().find(|(i, _)| *i == idx) {
                        if show_preview {
                            // show_preview_static renders its own heading
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
                    ui.heading(&monster.name);
                    ui.separator();
                    ui.label(format!("ID: {}", monster.id));
                    ui.label(format!("HP: {}", monster.hp.base));
                    ui.label(format!("AC: {}", monster.ac.base));
                    ui.label(format!("Attacks: {}", monster.attacks.len()));
                    ui.label(format!("Undead: {}", monster.is_undead));
                    ui.label(format!("Can Regenerate: {}", monster.can_regenerate));
                    ui.label(format!("Can Advance: {}", monster.can_advance));
                    ui.label(format!("Magic Resistance: {}%", monster.magic_resistance));
                    if let Some(id) = monster.creature_id {
                        ui.label(format!("Creature ID: {}", id));
                    }
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
                    if let Some(id) = monster.creature_id {
                        ui.label(format!("🦎 Creature: {}", id));
                    } else {
                        ui.label("🦎 Creature: No creature asset");
                    }

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

                    // Detailed attacks listing
                    for (i, attack) in monster.attacks.iter().enumerate() {
                        // Show damage roll and attack type
                        ui.label(format!(
                            "  Attack {}: {}d{}+{} ({:?})",
                            i + 1,
                            attack.damage.count,
                            attack.damage.sides,
                            attack.damage.bonus,
                            attack.attack_type
                        ));

                        // Show special effect if present
                        if let Some(ref effect) = attack.special_effect {
                            ui.label(format!("    Special: {:?}", effect));
                        }
                    }

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

                    // Resistances display: show the current values for all resistances
                    ui.label("🛡️ Resistances:");
                    ui.label(format!(
                        "  Physical: {}",
                        if monster.resistances.physical {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Fire: {}",
                        if monster.resistances.fire {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Cold: {}",
                        if monster.resistances.cold {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Electricity: {}",
                        if monster.resistances.electricity {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Energy: {}",
                        if monster.resistances.energy {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Paralysis: {}",
                        if monster.resistances.paralysis {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Fear: {}",
                        if monster.resistances.fear {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    ui.label(format!(
                        "  Sleep: {}",
                        if monster.resistances.sleep {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));

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

    /// Shows the monster creation/edit form
    ///
    /// Displays fields for editing monster properties including name (with autocomplete),
    /// stats, abilities, attacks, and loot tables.
    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        monsters: &mut Vec<MonsterDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        monsters_file: &str,
        creature_manager: Option<&CreatureAssetManager>,
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

                    // Use autocomplete for monster name to help with consistency
                    use crate::ui_helpers::autocomplete_monster_selector;

                    if autocomplete_monster_selector(
                        ui,
                        "monster_name_autocomplete",
                        "Name:",
                        &mut self.monster_name_input_buffer,
                        monsters,
                    ) {
                        // Update edit buffer when selection changes
                        self.edit_buffer.name = self.monster_name_input_buffer.clone();
                        *unsaved_changes = true;
                    }

                    // Also allow direct text editing for new names
                    ui.horizontal(|ui| {
                        ui.label("Custom Name:");
                        if ui
                            .text_edit_singleline(&mut self.edit_buffer.name)
                            .changed()
                        {
                            self.monster_name_input_buffer = self.edit_buffer.name.clone();
                            *unsaved_changes = true;
                        }
                    });

                    ui.checkbox(&mut self.edit_buffer.is_undead, "💀 Undead");
                });

                ui.add_space(10.0);

                // Visual Asset section
                ui.group(|ui| {
                    ui.heading("Visual Asset");

                    let creature_id_label = match self.edit_buffer.creature_id {
                        Some(id) => id.to_string(),
                        None => "None".to_string(),
                    };

                    ui.horizontal(|ui| {
                        ui.label("Creature ID:");
                        ui.label(&creature_id_label);
                        if ui
                            .button("Browse…")
                            .on_hover_text("Select a creature asset")
                            .clicked()
                        {
                            self.creature_picker_open = true;
                        }
                        if ui.button("Clear").clicked() {
                            self.apply_selected_creature_id(None);
                        }
                        ui.label("ℹ").on_hover_text(
                            "Links this monster to a procedural mesh creature definition. When set, \
                             the monster spawns as a 3-D creature mesh on the map instead of a \
                             sprite placeholder.",
                        );
                    });

                    // Show resolved creature name when manager is available
                    if let (Some(id), Some(manager)) =
                        (self.edit_buffer.creature_id, creature_manager)
                    {
                        if let Ok(creature) = manager.load_creature(id) {
                            ui.label(
                                egui::RichText::new(format!("Asset: \"{}\"", creature.name))
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    }
                });

                // Creature picker modal
                if self.creature_picker_open {
                    if let Some(manager) = creature_manager {
                        let creatures = manager.load_all_creatures().unwrap_or_default();
                        let mut picked_id: Option<CreatureId> = None;
                        let mut should_close = false;
                        egui::Window::new("Select Creature")
                            .id(egui::Id::new("monster_creature_picker"))
                            .resizable(true)
                            .show(ui.ctx(), |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("monster_creature_picker_scroll")
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        for creature in &creatures {
                                            ui.push_id(creature.id, |ui| {
                                                if ui
                                                    .selectable_label(
                                                        self.edit_buffer.creature_id
                                                            == Some(creature.id),
                                                        format!(
                                                            "{} — {}",
                                                            creature.id, creature.name
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    picked_id = Some(creature.id);
                                                }
                                            });
                                        }
                                    });
                                if ui.button("Close").clicked() {
                                    should_close = true;
                                }
                            });
                        if let Some(id) = picked_id {
                            self.apply_selected_creature_id(Some(id));
                        } else if should_close {
                            self.creature_picker_open = false;
                        }
                    } else {
                        // No manager available; close picker
                        self.creature_picker_open = false;
                    }
                }

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
                    ui.heading("Resistances");

                    // Provide checkboxes for all boolean resistance flags on the monster.
                    // Arrange into rows so the UI remains compact.
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.edit_buffer.resistances.physical, "Physical");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.edit_buffer.resistances.fire, "Fire");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.edit_buffer.resistances.cold, "Cold");
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.edit_buffer.resistances.electricity, "Electricity");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.edit_buffer.resistances.energy, "Energy");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.edit_buffer.resistances.paralysis, "Paralysis");
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.edit_buffer.resistances.fear, "Fear");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.edit_buffer.resistances.sleep, "Sleep");
                    });
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
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = MonstersEditorMode::List;
                    }

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
                        self.monster_name_input_buffer.clear();
                        *status_message = "Monster saved".to_string();
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = MonstersEditorMode::List;
                        self.monster_name_input_buffer.clear();
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
                                ui.selectable_value(effect, SpecialEffect::Disease, "Disease");
                                ui.selectable_value(effect, SpecialEffect::Paralysis, "Paralysis");
                                ui.selectable_value(effect, SpecialEffect::Sleep, "Sleep");
                                ui.selectable_value(effect, SpecialEffect::Drain, "Drain");
                                ui.selectable_value(effect, SpecialEffect::Stone, "Stone");
                                ui.selectable_value(effect, SpecialEffect::Death, "Death");
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
                is_ranged: false,
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
        for (i, (chance, item_id)) in self.edit_buffer.loot.items.iter_mut().enumerate() {
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
            self.edit_buffer.loot.items.remove(idx);
        }

        if ui.button("➕ Add Item Drop").clicked() {
            self.edit_buffer.loot.items.push((0.1, 0));
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

    /// Calculate experience points for a monster based on its stats and abilities.
    ///
    /// The formula considers:
    /// - Base HP (HP * 10)
    /// - Armor Class bonus ((10 - AC) * 50, if AC < 10)
    /// - Number of attacks (attacks * 20)
    /// - Average damage per attack (avg_damage * 5)
    /// - Special effects (+50 per attack with special)
    /// - Regeneration ability (+100)
    /// - Undead status (+50)
    /// - Magic resistance (resistance * 2)
    pub fn calculate_monster_xp(&self, monster: &MonsterDefinition) -> u32 {
        let mut xp: u32 = 0;

        // Base XP from HP
        xp += monster.hp.current as u32 * 10;

        // AC bonus (lower AC = harder to hit = more XP)
        if monster.ac.current < 10 {
            xp += (10 - monster.ac.current) as u32 * 50;
        }

        // Attack bonuses
        xp += monster.attacks.len() as u32 * 20;

        // Damage contribution
        for attack in &monster.attacks {
            // Average damage = (count * (sides + 1) / 2) + bonus
            let avg_damage = (attack.damage.count as f32 * (attack.damage.sides as f32 + 1.0)
                / 2.0)
                + attack.damage.bonus as f32;
            xp += (avg_damage * 5.0) as u32;

            // Special effect bonus
            if attack.special_effect.is_some() {
                xp += 50;
            }
        }

        // Ability bonuses
        if monster.can_regenerate {
            xp += 100;
        }
        if monster.is_undead {
            xp += 50;
        }

        // Magic resistance bonus
        xp += monster.magic_resistance as u32 * 2;

        xp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Creature Asset Binding Tests
    // =========================================================================

    #[test]
    fn test_monsters_editor_creature_id_roundtrips_through_form() {
        let mut state = MonstersEditorState::default();
        state.apply_selected_creature_id(Some(42));
        assert_eq!(state.edit_buffer.creature_id, Some(42));
    }

    #[test]
    fn test_monsters_editor_clear_creature_id() {
        let mut state = MonstersEditorState::default();
        state.edit_buffer.creature_id = Some(42);
        state.apply_selected_creature_id(None);
        assert_eq!(state.edit_buffer.creature_id, None);
    }

    #[test]
    fn test_monsters_editor_default_monster_creature_id_is_none() {
        let monster = MonstersEditorState::default_monster();
        assert_eq!(monster.creature_id, None);
    }

    // =========================================================================
    // MonstersEditorState Tests
    // =========================================================================

    #[test]
    fn test_monsters_editor_state_new() {
        let state = MonstersEditorState::new();
        assert_eq!(state.mode, MonstersEditorMode::List);
        assert!(state.search_query.is_empty());
        assert!(state.selected_monster.is_none());
        assert!(!state.show_import_dialog);
        assert!(state.import_export_buffer.is_empty());
        assert!(!state.show_preview);
    }

    #[test]
    fn test_monsters_editor_state_default() {
        let state = MonstersEditorState::default();
        assert_eq!(state.mode, MonstersEditorMode::List);
        assert!(!state.show_stats_editor);
        assert!(!state.show_attacks_editor);
        assert!(!state.show_loot_editor);
    }

    #[test]
    fn test_default_monster_creation() {
        let monster = MonstersEditorState::default_monster();
        assert_eq!(monster.id, 0);
        assert_eq!(monster.name, "New Monster");
        assert_eq!(monster.hp.base, 10);
        assert_eq!(monster.hp.current, 10);
        assert_eq!(monster.ac.base, 10);
        assert_eq!(monster.ac.current, 10);
        assert!(!monster.is_undead);
        assert!(!monster.can_regenerate);
        assert!(!monster.can_advance);
        assert_eq!(monster.magic_resistance, 0);
        assert_eq!(monster.flee_threshold, 0);
        assert_eq!(monster.attacks.len(), 1);
    }

    // =========================================================================
    // MonstersEditorMode Tests
    // =========================================================================

    #[test]
    fn test_monsters_editor_mode_variants() {
        assert_eq!(MonstersEditorMode::List, MonstersEditorMode::List);
        assert_eq!(MonstersEditorMode::Add, MonstersEditorMode::Add);
        assert_eq!(MonstersEditorMode::Edit, MonstersEditorMode::Edit);
        assert_ne!(MonstersEditorMode::List, MonstersEditorMode::Add);
    }

    // =========================================================================
    // Editor State Transitions Tests
    // =========================================================================

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = MonstersEditorState::new();
        assert_eq!(state.mode, MonstersEditorMode::List);

        state.mode = MonstersEditorMode::Add;
        assert_eq!(state.mode, MonstersEditorMode::Add);

        state.mode = MonstersEditorMode::Edit;
        assert_eq!(state.mode, MonstersEditorMode::Edit);

        state.mode = MonstersEditorMode::List;
        assert_eq!(state.mode, MonstersEditorMode::List);
    }

    #[test]
    fn test_selected_monster_handling() {
        let mut state = MonstersEditorState::new();
        assert!(state.selected_monster.is_none());

        state.selected_monster = Some(0);
        assert_eq!(state.selected_monster, Some(0));

        state.selected_monster = Some(5);
        assert_eq!(state.selected_monster, Some(5));

        state.selected_monster = None;
        assert!(state.selected_monster.is_none());
    }

    #[test]
    fn test_editor_toggle_states() {
        let mut state = MonstersEditorState::new();

        // Toggle stats editor
        state.show_stats_editor = true;
        assert!(state.show_stats_editor);

        // Toggle attacks editor
        state.show_attacks_editor = true;
        assert!(state.show_attacks_editor);

        // Toggle loot editor
        state.show_loot_editor = true;
        assert!(state.show_loot_editor);
    }

    // =========================================================================
    // Monster XP Calculation Tests
    // =========================================================================

    #[test]
    fn test_calculate_monster_xp_basic() {
        let state = MonstersEditorState::new();
        let monster = MonstersEditorState::default_monster();
        let xp = state.calculate_monster_xp(&monster);
        // Base XP for a level 1 monster with default stats
        assert!(xp > 0);
    }

    #[test]
    fn test_calculate_monster_xp_with_abilities() {
        let state = MonstersEditorState::new();
        let mut monster = MonstersEditorState::default_monster();
        let base_xp = state.calculate_monster_xp(&monster);

        // Add regeneration
        monster.can_regenerate = true;
        let regen_xp = state.calculate_monster_xp(&monster);
        assert!(regen_xp > base_xp, "Regeneration should increase XP");

        // Add undead
        monster.is_undead = true;
        let undead_xp = state.calculate_monster_xp(&monster);
        assert!(undead_xp > regen_xp, "Undead should increase XP");
    }

    #[test]
    fn test_calculate_monster_xp_with_magic_resistance() {
        let state = MonstersEditorState::new();
        let mut monster = MonstersEditorState::default_monster();
        let base_xp = state.calculate_monster_xp(&monster);

        monster.magic_resistance = 50;
        let resistant_xp = state.calculate_monster_xp(&monster);
        assert!(
            resistant_xp > base_xp,
            "Magic resistance should increase XP"
        );
    }

    #[test]
    fn test_edit_buffer_modification() {
        let mut state = MonstersEditorState::new();

        // Modify the edit buffer
        state.edit_buffer.name = "Dragon".to_string();
        state.edit_buffer.hp.base = 100;
        state.edit_buffer.hp.current = 100;
        state.edit_buffer.ac.base = 20;
        state.edit_buffer.is_undead = false;
        state.edit_buffer.can_regenerate = true;

        assert_eq!(state.edit_buffer.name, "Dragon");
        assert_eq!(state.edit_buffer.hp.base, 100);
        assert_eq!(state.edit_buffer.ac.base, 20);
        assert!(state.edit_buffer.can_regenerate);
    }

    #[test]
    fn test_monster_stats_initialization() {
        let monster = MonstersEditorState::default_monster();

        // Check all stats are initialized
        assert_eq!(monster.stats.might.base, 10);
        assert_eq!(monster.stats.intellect.base, 10);
        assert_eq!(monster.stats.personality.base, 10);
        assert_eq!(monster.stats.endurance.base, 10);
        assert_eq!(monster.stats.speed.base, 10);
        assert_eq!(monster.stats.accuracy.base, 10);
        assert_eq!(monster.stats.luck.base, 10);
    }

    #[test]
    fn test_preview_toggle() {
        let mut state = MonstersEditorState::new();
        assert!(!state.show_preview);

        state.show_preview = true;
        assert!(state.show_preview);

        state.show_preview = false;
        assert!(!state.show_preview);
    }

    // =========================================================================
    // Autocomplete Buffer Tests
    // =========================================================================

    #[test]
    fn test_monster_name_input_buffer_initialization() {
        let state = MonstersEditorState::new();
        assert!(
            state.monster_name_input_buffer.is_empty(),
            "Monster name input buffer should be empty on initialization"
        );
    }

    #[test]
    fn test_monster_name_input_buffer_default() {
        let state = MonstersEditorState::default();
        assert!(
            state.monster_name_input_buffer.is_empty(),
            "Monster name input buffer should be empty by default"
        );
    }

    #[test]
    fn test_autocomplete_buffer_synchronization() {
        let mut state = MonstersEditorState::new();
        let test_name = "Goblin Warrior";

        // Simulate setting the buffer (as autocomplete would)
        state.monster_name_input_buffer = test_name.to_string();
        assert_eq!(state.monster_name_input_buffer, test_name);

        // Simulate syncing to edit buffer
        state.edit_buffer.name = state.monster_name_input_buffer.clone();
        assert_eq!(state.edit_buffer.name, test_name);
        assert_eq!(state.edit_buffer.name, state.monster_name_input_buffer);
    }

    #[test]
    fn test_autocomplete_buffer_cleared_on_mode_transition() {
        let mut state = MonstersEditorState::new();
        state.mode = MonstersEditorMode::Edit;
        state.monster_name_input_buffer = "Test Monster".to_string();

        // Simulate clearing buffer when returning to list mode
        state.mode = MonstersEditorMode::List;
        state.monster_name_input_buffer.clear();

        assert!(state.monster_name_input_buffer.is_empty());
        assert_eq!(state.mode, MonstersEditorMode::List);
    }

    #[test]
    fn test_monster_name_persistence_between_buffers() {
        let mut state = MonstersEditorState::new();
        let original_name = "Dragon";

        // Set edit buffer name
        state.edit_buffer.name = original_name.to_string();

        // Simulate initializing autocomplete buffer from edit buffer
        state.monster_name_input_buffer = state.edit_buffer.name.clone();

        assert_eq!(state.monster_name_input_buffer, original_name);
        assert_eq!(state.edit_buffer.name, original_name);

        // Modify via autocomplete buffer
        state.monster_name_input_buffer = "Ancient Dragon".to_string();
        state.edit_buffer.name = state.monster_name_input_buffer.clone();

        assert_eq!(state.edit_buffer.name, "Ancient Dragon");
        assert_eq!(state.monster_name_input_buffer, "Ancient Dragon");
    }
}
