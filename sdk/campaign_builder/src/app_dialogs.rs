// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! App-level dialog rendering methods for [`CampaignBuilderApp`].
//!
//! Contains modal/overlay window rendering: the template browser, creature
//! template browser, validation report, debug panel, and balance stats dialogs.
//! Also contains creature ID management helpers.

use super::*;

impl CampaignBuilderApp {
    /// Show template browser dialog
    pub(crate) fn show_template_browser_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.ui_state.show_template_browser;
        egui::Window::new("📋 Template Browser")
            .open(&mut open)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    for category in templates::TemplateCategory::all() {
                        if ui
                            .selectable_label(
                                self.ui_state.template_category == *category,
                                category.name(),
                            )
                            .clicked()
                        {
                            self.ui_state.template_category = *category;
                        }
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| match self.ui_state.template_category {
                    templates::TemplateCategory::Item => {
                        for template in self.template_manager.item_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(item) = self.template_manager.create_item(
                                            &template.id,
                                            self.campaign_data.items.len() as u32 + 1,
                                        ) {
                                            self.editor_registry.items_editor_state.edit_buffer =
                                                item;
                                            self.editor_registry.items_editor_state.mode =
                                                items_editor::ItemsEditorMode::Add;
                                            self.ui_state.active_tab = EditorTab::Items;
                                            self.ui_state.status_message =
                                                format!("Template '{}' loaded", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Monster => {
                        for template in self.template_manager.monster_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(monster) = self.template_manager.create_monster(
                                            &template.id,
                                            self.campaign_data.monsters.len() as u32 + 1,
                                        ) {
                                            self.editor_registry
                                                .monsters_editor_state
                                                .edit_buffer = monster;
                                            self.editor_registry.monsters_editor_state.mode =
                                                monsters_editor::MonstersEditorMode::Add;
                                            self.ui_state.active_tab = EditorTab::Monsters;
                                            self.ui_state.status_message =
                                                format!("Template '{}' loaded", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Quest => {
                        for template in self.template_manager.quest_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(quest) = self.template_manager.create_quest(
                                            &template.id,
                                            self.campaign_data.quests.len() as u32 + 1,
                                        ) {
                                            self.campaign_data.quests.push(quest);
                                            self.ui_state.active_tab = EditorTab::Quests;
                                            self.unsaved_changes = true;
                                            self.ui_state.status_message =
                                                format!("Template '{}' added", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Dialogue => {
                        for template in self.template_manager.dialogue_templates() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(&template.name);
                                    if ui.button("Use Template").clicked() {
                                        if let Some(dialogue) =
                                            self.template_manager.create_dialogue(
                                                &template.id,
                                                self.campaign_data.dialogues.len() as u32 + 1,
                                            )
                                        {
                                            self.campaign_data.dialogues.push(dialogue);
                                            self.ui_state.active_tab = EditorTab::Dialogues;
                                            self.unsaved_changes = true;
                                            self.ui_state.status_message =
                                                format!("Template '{}' added", template.name);
                                        }
                                    }
                                });
                                ui.label(&template.description);
                                ui.label(format!("Tags: {}", template.tags.join(", ")));
                            });
                            ui.add_space(5.0);
                        }
                    }
                    templates::TemplateCategory::Map => {
                        ui.label("Map templates coming soon...");
                    }
                });
            });
        self.ui_state.show_template_browser = open;
    }

    /// Build registry references from the currently loaded creature definitions.
    pub(crate) fn creature_references_from_current_registry(&self) -> Vec<CreatureReference> {
        self.campaign_data
            .creatures
            .iter()
            .map(|creature| CreatureReference {
                id: creature.id,
                name: creature.name.clone(),
                filepath: format!(
                    "assets/creatures/{}.ron",
                    creature.name.to_lowercase().replace(' ', "_")
                ),
            })
            .collect()
    }

    /// Synchronize the creature ID manager from the current in-memory registry.
    pub(crate) fn sync_creature_id_manager_from_creatures(&mut self) {
        let references = self.creature_references_from_current_registry();
        self.editor_registry
            .creatures_editor_state
            .id_manager
            .update_from_registry(&references);
    }

    /// Determine the next available creature ID in the specified category.
    ///
    /// This method always refreshes the ID manager from `self.campaign_data.creatures` first so
    /// menu-entry order does not affect ID assignment correctness.
    pub(crate) fn next_available_creature_id_for_category(
        &mut self,
        category: creature_id_manager::CreatureCategory,
    ) -> Result<CreatureId, String> {
        self.sync_creature_id_manager_from_creatures();

        let suggested = self
            .editor_registry
            .creatures_editor_state
            .id_manager
            .suggest_next_id(category);
        if !self
            .editor_registry
            .creatures_editor_state
            .id_manager
            .is_id_used(suggested)
        {
            return Ok(suggested);
        }

        let range = category.id_range();
        let range_start = range.start;
        let range_end = range.end - 1;

        for candidate in range {
            if !self
                .editor_registry
                .creatures_editor_state
                .id_manager
                .is_id_used(candidate)
            {
                return Ok(candidate);
            }
        }

        Err(format!(
            "No available IDs in {} range ({}-{}).",
            category.display_name(),
            range_start,
            range_end
        ))
    }

    /// Show the Creature Template Browser dialog window.
    ///
    /// Opens a full-featured grid/list browser with all registered creature
    /// templates.  The browser supports category filtering, complexity filtering,
    /// search, and a live preview panel.
    ///
    /// # Actions handled
    ///
    /// * `TemplateBrowserAction::CreateNew(template_id)` -- Generates a new
    ///   `CreatureDefinition` from the selected template, assigns the next
    ///   available ID in the Monsters range, pushes it onto `self.campaign_data.creatures`,
    ///   switches to the Creatures tab, and opens the editor.
    ///
    /// * `TemplateBrowserAction::ApplyToCurrent(template_id)` -- If a creature
    ///   is currently open in Edit mode, replaces its mesh data (meshes,
    ///   mesh_transforms, scale, color_tint) with data generated from the
    ///   template while preserving the creature's ID and name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // In the update() method:
    /// // if self.ui_state.show_creature_template_browser {
    /// //     self.show_creature_template_browser_dialog(ctx);
    /// // }
    /// ```
    pub(crate) fn show_creature_template_browser_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.ui_state.show_creature_template_browser;

        // Split borrows: `entries` borrows `creature_template_registry`
        // immutably while `browser_state` borrows `creature_template_browser_state`
        // mutably.  These are disjoint fields so the borrow checker allows it.
        // The inner block ensures both borrows are released before we access
        // other `self` fields below.
        let action: Option<template_browser::TemplateBrowserAction> = {
            let entries: Vec<&template_metadata::TemplateEntry> =
                self.creature_template_registry.all_templates();
            let browser_state = &mut self.creature_template_browser_state;
            egui::Window::new("🐉 Creature Template Browser")
                .open(&mut open)
                .resizable(true)
                .default_size([900.0, 600.0])
                .show(ctx, |ui| browser_state.show(ui, &entries))
                .and_then(|r| r.inner)
                .flatten()
        };

        self.ui_state.show_creature_template_browser = open;

        let Some(action) = action else {
            return;
        };

        match action {
            template_browser::TemplateBrowserAction::CreateNew(template_id) => {
                // Collect only what we need (owned) so the borrow of
                // creature_template_registry is released before we mutate self.
                let template_name: Option<String> = self
                    .creature_template_registry
                    .get(&template_id)
                    .map(|e| e.metadata.name.clone());

                let Some(template_name) = template_name else {
                    self.ui_state.status_message =
                        format!("Template '{}' not found in registry", template_id);
                    return;
                };

                // Suggest next ID in the Monsters category (1–50) using a
                // freshly synchronized ID manager so this path is correct even
                // when users open templates directly from Tools.
                let new_id = match self.next_available_creature_id_for_category(
                    creature_id_manager::CreatureCategory::Monsters,
                ) {
                    Ok(id) => id,
                    Err(error) => {
                        self.ui_state.status_message = error;
                        return;
                    }
                };

                let creature_name = format!("New {}", template_name);

                // `generate` returns an owned value; borrow of the registry ends here.
                match self
                    .creature_template_registry
                    .generate(&template_id, &creature_name, new_id)
                {
                    Ok(new_creature) => {
                        if let Some(existing) = self
                            .campaign_data
                            .creatures
                            .iter()
                            .find(|creature| creature.id == new_creature.id)
                        {
                            self.ui_state.status_message = format!(
                                "Cannot create creature from template '{}': ID {} is already registered to '{}'.",
                                template_id, new_creature.id, existing.name
                            );
                            return;
                        }

                        self.campaign_data.creatures.push(new_creature);
                        let new_idx = self.campaign_data.creatures.len() - 1;
                        let file_name = format!(
                            "assets/creatures/{}.ron",
                            self.campaign_data.creatures[new_idx]
                                .name
                                .to_lowercase()
                                .replace(' ', "_")
                        );
                        self.editor_registry
                            .creatures_editor_state
                            .open_for_editing(&self.campaign_data.creatures, new_idx, &file_name);
                        self.ui_state.active_tab = EditorTab::Creatures;
                        self.unsaved_changes = true;
                        self.ui_state.status_message = format!(
                            "Created '{}' from template '{}' -- customize and save.",
                            creature_name, template_name
                        );
                    }
                    Err(e) => {
                        self.ui_state.status_message =
                            format!("Failed to generate creature from template: {}", e);
                    }
                }
            }
            template_browser::TemplateBrowserAction::ApplyToCurrent(template_id) => {
                if self.editor_registry.creatures_editor_state.mode
                    != creatures_editor::CreaturesEditorMode::Edit
                {
                    self.ui_state.status_message =
                        "Open a creature in the editor first, then apply a template.".to_string();
                    return;
                }

                // Preserve the current creature's identity.
                let current_name = self
                    .editor_registry
                    .creatures_editor_state
                    .edit_buffer
                    .name
                    .clone();
                let current_id = self.editor_registry.creatures_editor_state.edit_buffer.id;

                // Generate from template (owned result; borrow ends immediately).
                match self.creature_template_registry.generate(
                    &template_id,
                    &current_name,
                    current_id,
                ) {
                    Ok(template_creature) => {
                        // Copy mesh data only -- keep the creature's ID and name intact.
                        self.editor_registry
                            .creatures_editor_state
                            .edit_buffer
                            .meshes = template_creature.meshes;
                        self.editor_registry
                            .creatures_editor_state
                            .edit_buffer
                            .mesh_transforms = template_creature.mesh_transforms;
                        self.editor_registry
                            .creatures_editor_state
                            .edit_buffer
                            .scale = template_creature.scale;
                        self.editor_registry
                            .creatures_editor_state
                            .edit_buffer
                            .color_tint = template_creature.color_tint;
                        self.editor_registry.creatures_editor_state.preview_dirty = true;
                        self.ui_state.status_message = format!(
                            "Applied template '{}' mesh data to '{}'.",
                            template_id, current_name
                        );
                    }
                    Err(e) => {
                        self.ui_state.status_message = format!("Failed to apply template: {}", e);
                    }
                }
            }
        }
    }

    /// Show validation report dialog
    pub(crate) fn show_validation_report_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.validation_state.show_validation_report;
        egui::Window::new("📊 Advanced Validation Report")
            .open(&mut open)
            .resizable(true)
            .default_size([700.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.monospace(&self.validation_state.validation_report);
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.validation_state.show_validation_report = false;
                    }
                    if ui.button("Run Again").clicked() {
                        self.run_advanced_validation();
                    }
                });
            });
        self.validation_state.show_validation_report = open;
    }

    /// Show the debug panel window
    ///
    /// Displays:
    /// - Current editor state
    /// - Loaded data counts
    /// - Recent log messages with filtering
    pub(crate) fn show_debug_panel_window(&mut self, ctx: &egui::Context) {
        let mut open = self.ui_state.show_debug_panel;
        egui::Window::new("🐛 Debug Panel")
            .open(&mut open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                // Current state section
                ui.collapsing("📊 Current State", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Active Tab:");
                        ui.strong(self.ui_state.active_tab.name());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Campaign Path:");
                        if let Some(ref path) = self.campaign_path {
                            ui.monospace(path.display().to_string());
                        } else {
                            ui.weak("(none)");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Unsaved Changes:");
                        if self.unsaved_changes {
                            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "Yes");
                        } else {
                            ui.label("No");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Log Level:");
                        ui.strong(self.logger.level().name());
                    });
                    let uptime = self.logger.uptime();
                    ui.horizontal(|ui| {
                        ui.label("Uptime:");
                        ui.monospace(format!("{:.1}s", uptime.as_secs_f64()));
                    });
                });

                ui.add_space(5.0);

                // Data counts section
                ui.collapsing("📦 Loaded Data", |ui| {
                    egui::Grid::new("debug_data_counts")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Items:");
                            ui.strong(self.campaign_data.items.len().to_string());
                            ui.end_row();

                            ui.label("Spells:");
                            ui.strong(self.campaign_data.spells.len().to_string());
                            ui.end_row();

                            ui.label("Monsters:");
                            ui.strong(self.campaign_data.monsters.len().to_string());
                            ui.end_row();

                            ui.label("Maps:");
                            ui.strong(self.campaign_data.maps.len().to_string());
                            ui.end_row();

                            ui.label("Quests:");
                            ui.strong(self.campaign_data.quests.len().to_string());
                            ui.end_row();

                            ui.label("Dialogues:");
                            ui.strong(self.campaign_data.dialogues.len().to_string());
                            ui.end_row();

                            ui.label("Conditions:");
                            ui.strong(self.campaign_data.conditions.len().to_string());
                            ui.end_row();

                            ui.label("Classes:");
                            ui.strong(
                                self.editor_registry
                                    .classes_editor_state
                                    .classes
                                    .len()
                                    .to_string(),
                            );
                            ui.end_row();
                        });
                });

                ui.add_space(5.0);

                // Log messages section
                ui.collapsing("📝 Log Messages", |ui| {
                    // Controls
                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        egui::ComboBox::from_id_salt("debug_log_filter")
                            .selected_text(self.ui_state.debug_panel_filter_level.name())
                            .show_ui(ui, |ui| {
                                for level in [
                                    LogLevel::Error,
                                    LogLevel::Warn,
                                    LogLevel::Info,
                                    LogLevel::Debug,
                                    LogLevel::Verbose,
                                ] {
                                    ui.selectable_value(
                                        &mut self.ui_state.debug_panel_filter_level,
                                        level,
                                        level.name(),
                                    );
                                }
                            });

                        ui.checkbox(&mut self.ui_state.debug_panel_auto_scroll, "Auto-scroll");

                        if ui.button("Clear").clicked() {
                            self.logger.clear();
                        }
                    });

                    // Message counts
                    let counts = self.logger.message_counts();
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            format!("E:{}", counts.error),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 100),
                            format!("W:{}", counts.warn),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 200, 200),
                            format!("I:{}", counts.info),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(150, 200, 255),
                            format!("D:{}", counts.debug),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(150, 150, 150),
                            format!("V:{}", counts.verbose),
                        );
                        ui.label(format!("Total: {}", counts.total()));
                    });

                    ui.separator();

                    // Log messages list
                    let scroll_area = egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .auto_shrink([false, false]);

                    scroll_area.show(ui, |ui| {
                        let filter_level = self.ui_state.debug_panel_filter_level;
                        for msg in self.logger.messages_at_level(filter_level) {
                            let color = msg.level.color();
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(color[0], color[1], color[2]),
                                    format!("[{}]", msg.level.prefix()),
                                );
                                ui.colored_label(
                                    egui::Color32::from_rgb(150, 150, 200),
                                    format!("[{}]", msg.category),
                                );
                                ui.label(&msg.message);
                            });
                        }

                        // Auto-scroll to bottom
                        if self.ui_state.debug_panel_auto_scroll {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Press F12 to toggle this panel");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.ui_state.show_debug_panel = false;
                        }
                    });
                });
            });
        self.ui_state.show_debug_panel = open;
    }

    /// Show balance statistics dialog
    pub(crate) fn show_balance_stats_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.ui_state.show_balance_stats;
        egui::Window::new("⚖️ Balance Statistics")
            .open(&mut open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                let validator = advanced_validation::AdvancedValidator::new(
                    self.campaign_data.items.clone(),
                    self.campaign_data.monsters.clone(),
                    self.campaign_data.quests.clone(),
                    self.campaign_data.maps.clone(),
                );
                let stats = validator.calculate_balance_stats();

                ui.heading("Content Overview");
                ui.label(format!("Total Items: {}", self.campaign_data.items.len()));
                ui.label(format!(
                    "Total Monsters: {}",
                    self.campaign_data.monsters.len()
                ));
                ui.label(format!("Total Quests: {}", self.campaign_data.quests.len()));
                ui.label(format!("Total Maps: {}", self.campaign_data.maps.len()));

                ui.add_space(10.0);
                ui.heading("Monster Statistics");
                ui.label(format!("Average Level: {:.1}", stats.average_monster_level));
                ui.label(format!("Average HP: {:.1}", stats.average_monster_hp));
                ui.label(format!("Average XP: {:.0}", stats.average_monster_exp));

                ui.add_space(10.0);
                ui.heading("Economy");
                ui.label(format!(
                    "Total Gold Available: {}",
                    stats.total_gold_available
                ));
                ui.label(format!("Total Items: {}", stats.total_items_available));

                ui.add_space(10.0);
                ui.heading("Level Distribution");
                let mut levels: Vec<_> = stats.monster_level_distribution.iter().collect();
                levels.sort_by_key(|(level, _)| *level);
                for (level, count) in levels {
                    ui.label(format!("Level {}: {} monsters", level, count));
                }

                ui.separator();
                if ui.button("Close").clicked() {
                    self.ui_state.show_balance_stats = false;
                }
            });
        self.ui_state.show_balance_stats = open;
    }
}
