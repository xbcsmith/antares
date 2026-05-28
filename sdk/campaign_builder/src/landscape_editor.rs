// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Landscape definition list and palette editor for the Campaign Builder.
//!
//! The Landscape editor gives authors a focused palette of reusable static
//! environment definitions loaded from `data/landscape.ron`. Imported OBJ/GLB
//! meshes are created through the Importer tab and upserted into this list.

use crate::ui_helpers::{show_standard_list_item, MetadataBadge, StandardListItemConfig};
use antares::domain::visual::{CreatureDefinition, CreatureReference};
use antares::domain::world::landscape::{LandscapeCategory, LandscapeDefinition};
use eframe::egui;
use std::fs;
use std::path::Path;

/// Signal requested by the Landscape editor.
///
/// # Examples
///
/// ```
/// use campaign_builder::landscape_editor::LandscapeEditorSignal;
///
/// let signal = LandscapeEditorSignal::OpenInObjImporter;
/// assert_eq!(signal, LandscapeEditorSignal::OpenInObjImporter);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandscapeEditorSignal {
    /// Switch to the Importer tab and prepare it for landscape mesh export.
    OpenInObjImporter,
}

/// View state for the Landscape editor tab.
///
/// # Examples
///
/// ```
/// use campaign_builder::landscape_editor::LandscapeEditorState;
///
/// let state = LandscapeEditorState::new();
/// assert!(state.search_query.is_empty());
/// assert!(state.category_filter.is_none());
/// ```
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
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use campaign_builder::landscape_editor::LandscapeEditorState;
    ///
    /// # fn render(ui: &mut eframe::egui::Ui) {
    /// let mut state = LandscapeEditorState::new();
    /// let mut definitions = Vec::new();
    /// state.show(ui, &mut definitions, None);
    /// # }
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut [LandscapeDefinition],
        campaign_dir: Option<&Path>,
    ) {
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
                    if ui
                        .selectable_value(&mut self.category_filter, None, "All")
                        .changed()
                    {
                        ui.ctx().request_repaint();
                    }
                    for category in LandscapeCategory::all() {
                        ui.push_id(*category as u8, |ui| {
                            if ui
                                .selectable_value(
                                    &mut self.category_filter,
                                    Some(*category),
                                    category.name(),
                                )
                                .changed()
                            {
                                ui.ctx().request_repaint();
                            }
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
                        self.show_grouped_rows(ui, defs, &rows);
                    });
            });

            ui.allocate_ui(egui::vec2(right_w, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("landscape_editor_preview_scroll")
                    .auto_shrink([true, false])
                    .show(ui, |ui| self.show_preview(ui, defs, campaign_dir));
            });
        });
    }

    fn show_grouped_rows(
        &mut self,
        ui: &mut egui::Ui,
        defs: &[LandscapeDefinition],
        rows: &[usize],
    ) {
        for category in LandscapeCategory::all() {
            let category_rows: Vec<usize> = rows
                .iter()
                .copied()
                .filter(|idx| defs[*idx].category == *category)
                .collect();
            if category_rows.is_empty() {
                continue;
            }

            ui.push_id(*category as u8, |ui| {
                ui.heading(category.name());
                for idx in category_rows {
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
                ui.add_space(6.0);
            });
        }
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

    fn show_preview(
        &self,
        ui: &mut egui::Ui,
        defs: &[LandscapeDefinition],
        campaign_dir: Option<&Path>,
    ) {
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
        ui.label(format!(
            "Texture status: {}",
            landscape_texture_validation_status(definition, campaign_dir)
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

fn landscape_texture_validation_status(
    definition: &LandscapeDefinition,
    campaign_dir: Option<&Path>,
) -> String {
    let Some(mesh_id) = definition.mesh_id else {
        return "procedural fallback (no mesh)".to_string();
    };
    let Some(campaign_dir) = campaign_dir else {
        return "open a campaign to validate textures".to_string();
    };

    let registry_path = campaign_dir.join("data/landscape_mesh_registry.ron");
    let Ok(registry_contents) = fs::read_to_string(&registry_path) else {
        return "mesh registry not found".to_string();
    };
    let Ok(registry) = ron::from_str::<Vec<CreatureReference>>(&registry_contents) else {
        return "mesh registry parse error".to_string();
    };
    let Some(reference) = registry.iter().find(|entry| entry.id == mesh_id) else {
        return "mesh ID missing from registry".to_string();
    };

    let mesh_path = campaign_dir.join(&reference.filepath);
    let Ok(mesh_contents) = fs::read_to_string(&mesh_path) else {
        return "mesh asset missing".to_string();
    };
    let Ok(creature) = ron::from_str::<CreatureDefinition>(&mesh_contents) else {
        return "mesh asset parse error".to_string();
    };

    let mut checked_textures = 0usize;
    for mesh in &creature.meshes {
        if let Some(texture_path) = &mesh.texture_path {
            checked_textures += 1;
            if !texture_path.starts_with("assets/") {
                return format!("invalid texture path: {texture_path}");
            }
            if !campaign_dir.join(texture_path).exists() {
                return format!("missing texture: {texture_path}");
            }
        }
    }

    if checked_textures == 0 {
        "mesh has no texture paths".to_string()
    } else {
        format!("{checked_textures} texture path(s) valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::world::landscape::LandscapeFlags;

    fn landscape_definition(
        id: antares::domain::types::LandscapeId,
        name: &str,
        category: LandscapeCategory,
        tags: &[&str],
    ) -> LandscapeDefinition {
        LandscapeDefinition {
            id,
            name: name.to_string(),
            category,
            default_scale: 1.0,
            color_tint: None,
            flags: LandscapeFlags::default(),
            icon: None,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            mesh_id: None,
            description: None,
        }
    }

    #[test]
    fn test_landscape_editor_new_defaults_empty_state() {
        let state = LandscapeEditorState::new();
        assert!(state.search_query.is_empty());
        assert_eq!(state.category_filter, None);
        assert_eq!(state.selected_landscape, None);
        assert_eq!(state.requested_signal, None);
    }

    #[test]
    fn test_filtered_rows_matches_name_and_tags() {
        let defs = vec![
            landscape_definition(1, "Oak Tree", LandscapeCategory::Tree, &["forest"]),
            landscape_definition(2, "Granite Rock", LandscapeCategory::Rock, &["stone"]),
        ];
        let mut state = LandscapeEditorState::new();
        state.search_query = "stone".to_string();

        assert_eq!(state.filtered_rows(&defs), vec![1]);

        state.search_query = "oak".to_string();
        assert_eq!(state.filtered_rows(&defs), vec![0]);
    }

    #[test]
    fn test_filtered_rows_combines_search_and_category() {
        let defs = vec![
            landscape_definition(1, "Oak Tree", LandscapeCategory::Tree, &["forest"]),
            landscape_definition(2, "Pine Tree", LandscapeCategory::Tree, &["needle"]),
            landscape_definition(3, "Forest Rock", LandscapeCategory::Rock, &["forest"]),
        ];
        let mut state = LandscapeEditorState::new();
        state.search_query = "forest".to_string();
        state.category_filter = Some(LandscapeCategory::Tree);

        assert_eq!(state.filtered_rows(&defs), vec![0]);
    }

    #[test]
    fn test_texture_validation_reports_procedural_fallback() {
        let def = landscape_definition(1, "Grass", LandscapeCategory::Grass, &[]);

        assert_eq!(
            landscape_texture_validation_status(&def, None),
            "procedural fallback (no mesh)"
        );
    }
}
