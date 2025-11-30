// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::conditions::{
    ConditionDefinition, ConditionDuration, ConditionEffect, ConditionId,
};
use eframe::egui;

pub struct ConditionsEditorState {
    pub search_filter: String,
    pub selected_condition_id: Option<ConditionId>,
    pub edit_buffer: Option<ConditionDefinition>,
    pub show_preview: bool,
}

impl Default for ConditionsEditorState {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_condition_id: None,
            edit_buffer: None,
            show_preview: true,
        }
    }
}

impl ConditionsEditorState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn render_conditions_editor(
    ui: &mut egui::Ui,
    state: &mut ConditionsEditorState,
    conditions: &mut Vec<ConditionDefinition>,
) {
    ui.horizontal(|ui| {
        // Left panel: List
        ui.vertical(|ui| {
            ui.set_width(250.0);
            ui.heading("Conditions");
            ui.separator();

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.search_filter);
                if ui.button("❌").clicked() {
                    state.search_filter.clear();
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let filtered: Vec<(usize, &ConditionDefinition)> = conditions
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        state.search_filter.is_empty()
                            || c.name.to_lowercase().contains(&state.search_filter.to_lowercase())
                            || c.id.to_lowercase().contains(&state.search_filter.to_lowercase())
                    })
                    .collect();

                for (idx, condition) in filtered {
                    let is_selected = state.selected_condition_id.as_ref() == Some(&condition.id);
                    if ui
                        .selectable_label(is_selected, &condition.name)
                        .clicked()
                    {
                        state.selected_condition_id = Some(condition.id.clone());
                        state.edit_buffer = Some(condition.clone());
                    }
                }
            });

            ui.separator();
            if ui.button("➕ New Condition").clicked() {
                let new_condition = ConditionDefinition {
                    id: "new_condition".to_string(),
                    name: "New Condition".to_string(),
                    description: "".to_string(),
                    effects: Vec::new(),
                    default_duration: ConditionDuration::Rounds(3),
                    icon_id: None,
                };
                state.edit_buffer = Some(new_condition);
                state.selected_condition_id = None;
            }
        });

        ui.separator();

        // Right panel: Editor
        if let Some(condition_id) = &state.selected_condition_id.clone() {
            if let Some(condition) = conditions.iter_mut().find(|c| &c.id == condition_id) {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Edit Condition");
                    ui.separator();

                    egui::Grid::new("condition_editor_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("ID:");
                            ui.label(&condition.id);
                            ui.end_row();

                            ui.label("Name:");
                            ui.text_edit_singleline(&mut condition.name);
                            ui.end_row();

                            ui.label("Description:");
                            ui.text_edit_multiline(&mut condition.description);
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("Effects (read-only for now):");
                    for (idx, effect) in condition.effects.iter().enumerate() {
                        ui.label(format!("Effect #{}: {:?}", idx + 1, effect));
                    }
                });
            }
        } else if state.edit_buffer.is_some() {
            // New condition
            if let Some(new_cond) = &mut state.edit_buffer {
                let mut should_save = false;
                let mut should_cancel = false;
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("New Condition");
                    ui.separator();

                    egui::Grid::new("condition_editor_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("ID:");
                            ui.text_edit_singleline(&mut new_cond.id);
                            ui.end_row();

                            ui.label("Name:");
                            ui.text_edit_singleline(&mut new_cond.name);
                            ui.end_row();

                            ui.label("Description:");
                            ui.text_edit_multiline(&mut new_cond.description);
                            ui.end_row();
                        });

                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("💾 Save").clicked() {
                            should_save = true;
                        }
                        if ui.button("❌ Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });
                
                if should_save {
                    conditions.push(new_cond.clone());
                    state.selected_condition_id = Some(new_cond.id.clone());
                    state.edit_buffer = None;
                }
                
                if should_cancel {
                    state.edit_buffer = None;
                }
            }
        } else {
            ui.label("Select a condition to edit or click '➕ New Condition' to create one.");
        }
    });
}
