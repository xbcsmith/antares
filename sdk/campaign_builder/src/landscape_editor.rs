// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Landscape definition list and palette editor for the Campaign Builder.
//!
//! The Landscape editor gives authors a focused palette of reusable static
//! environment definitions loaded from `data/landscape.ron`. Imported OBJ/GLB
//! meshes are created through the Importer tab and upserted into this list.

use crate::ui_helpers::{show_standard_list_item, MetadataBadge, StandardListItemConfig};
use antares::domain::world::landscape::{LandscapeCategory, LandscapeDefinition};
use eframe::egui;

/// Signal requested by the Landscape editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandscapeEditorSignal {
    /// Switch to the Importer tab and prepare it for landscape mesh export.
    OpenInObjImporter,
}

/// View state for the Landscape editor tab.
#[derive(Debug, Default)]
pub struct LandscapeEditorState {
    /// Search text used to filter landscape definitions by name or tag.
    pub search_query: String,
    /// Optional category filter.
    pub category_filter: Option<LandscapeCategory>,
    /// Selected definition index in the visible source vector.
    pub selected_landscape: Option<usize>,
    /// Deferred signal consumed by `CampaignBuilderApp`.
    pub requested_signal: Option<LandscapeEditorSignal>,
}

impl LandscapeEditorState {
    /// Creates a fresh Landscape editor state.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::landscape_editor::LandscapeEditorState;
    ///
    /// let state = LandscapeEditorState::new();
    /// assert!(state.search_query.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the landscape definition list and selected definition preview.
    pub fn show(&mut self, ui: &mut egui::Ui, defs: &mut [LandscapeDefinition]) {
        ui.horizontal(|ui| {
            ui.heading("🌳 Landscape");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Import Landscape Mesh").clicked() {
                    self.requested_signal = Some(LandscapeEditorSignal::OpenInObjImporter);
                    ui.ctx().request_repaint();
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                ui.ctx().request_repaint();
            }
            ui.label("Category:");
            egui::ComboBox::from_id_salt("landscape_editor_category_filter")
                .selected_text(
                    self.category_filter
                        .map(|category| category.name())
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category_filter, None, "All");
                    for category in LandscapeCategory::all() {
                        ui.push_id(*category as u8, |ui| {
                            ui.selectable_value(
                                &mut self.category_filter,
                                Some(*category),
                                category.name(),
                            );
                        });
                    }
                });
        });

        let available = ui.available_size();
        let col_h = available.y;
        let left_w = (available.x * 0.45).max(260.0);
        let right_w = (available.x - left_w - ui.spacing().item_spacing.x).max(260.0);

        ui.horizontal(|ui| {
            ui.allocate_ui(egui::vec2(left_w, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("landscape_editor_list_scroll")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        let rows = self.filtered_rows(defs);
                        for idx in rows {
                            let definition = &defs[idx];
                            ui.push_id(definition.id, |ui| {
                                let selected = self.selected_landscape == Some(idx);
                                let (clicked, _action) = show_standard_list_item(
                                    ui,
                                    StandardListItemConfig::new(&definition.name)
                                        .selected(selected)
                                        .with_icon(definition.display_icon())
                                        .with_id(definition.id)
                                        .with_badges(vec![
                                            MetadataBadge::new(definition.category.name()),
                                            if definition.mesh_id.is_some() {
                                                MetadataBadge::new("mesh")
                                            } else {
                                                MetadataBadge::new("procedural")
                                            },
                                        ]),
                                );
                                if clicked {
                                    self.selected_landscape = Some(idx);
                                    ui.ctx().request_repaint();
                                }
                            });
                        }
                    });
            });

            ui.allocate_ui(egui::vec2(right_w, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("landscape_editor_preview_scroll")
                    .auto_shrink([true, false])
                    .show(ui, |ui| self.show_preview(ui, defs));
            });
        });
    }

    fn filtered_rows(&self, defs: &[LandscapeDefinition]) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        defs.iter()
            .enumerate()
            .filter(|(_, definition)| {
                self.category_filter
                    .is_none_or(|category| definition.category == category)
            })
            .filter(|(_, definition)| {
                query.is_empty()
                    || definition.name.to_lowercase().contains(&query)
                    || definition
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn show_preview(&self, ui: &mut egui::Ui, defs: &[LandscapeDefinition]) {
        let Some(idx) = self.selected_landscape.filter(|idx| *idx < defs.len()) else {
            ui.label("Select a landscape definition to preview it.");
            return;
        };

        let definition = &defs[idx];
        ui.heading(format!("{} {}", definition.display_icon(), definition.name));
        ui.separator();
        ui.label(format!("ID: {}", definition.id));
        ui.label(format!("Category: {}", definition.category.name()));
        ui.label(format!("Default scale: {:.3}", definition.default_scale));
        ui.label(format!("Blocking: {}", definition.flags.blocking));
        ui.label(format!(
            "Mesh ID: {}",
            definition
                .mesh_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "None".to_string())
        ));
        if !definition.tags.is_empty() {
            ui.label(format!("Tags: {}", definition.tags.join(", ")));
        }
        if let Some(description) = &definition.description {
            ui.separator();
            ui.label(description);
        }
    }
}
