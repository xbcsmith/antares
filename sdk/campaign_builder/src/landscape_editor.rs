// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Landscape definition list and palette editor for the Campaign Builder.
//!
//! The Landscape editor gives authors a focused palette of reusable static
//! environment definitions loaded from `data/landscape.ron`. Imported OBJ/GLB
//! meshes are created through the Importer tab and upserted into this list.

use crate::ui_helpers::{
    show_standard_list_item, ItemAction, MetadataBadge, StandardListItemConfig, TwoColumnLayout,
};
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

#[derive(Debug, Default, PartialEq)]
enum LandscapeEditorMode {
    #[default]
    List,
    Edit,
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

    // Edit mode fields.
    mode: LandscapeEditorMode,
    edit_index: Option<usize>,
    edit_buffer: Option<LandscapeDefinition>,
    /// Comma-separated tags string edited as a single text field.
    tags_buffer: String,
    /// Description string buffer (empty means None).
    description_buffer: String,
    /// Icon/emoji buffer (empty means None).
    icon_buffer: String,

    // Texture validation cache: (definition_id, status_string).
    // Only recomputed when the selected definition ID changes, so the
    // blocking file reads in landscape_texture_validation_status are not
    // called on every render frame (which caused the right-click "timeout").
    texture_validation_cache: Option<(u32, String)>,
    /// Cached mesh options loaded from `landscape_mesh_registry.ron` when
    /// entering edit mode. Populated once per edit session; used to drive
    /// the Mesh picker ComboBox without re-reading the registry every frame.
    available_meshes: Vec<CreatureReference>,
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
    /// let mut unsaved = false;
    /// state.show(ui, &mut definitions, None, &mut unsaved);
    /// # }
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<LandscapeDefinition>,
        campaign_dir: Option<&Path>,
        unsaved_changes: &mut bool,
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

        if self.mode == LandscapeEditorMode::Edit {
            self.show_edit(ui, defs, unsaved_changes);
        } else {
            self.show_list(ui, defs, campaign_dir, unsaved_changes);
        }
    }

    // =========================================================================
    // List view
    // =========================================================================

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<LandscapeDefinition>,
        campaign_dir: Option<&Path>,
        unsaved_changes: &mut bool,
    ) {
        // SDK Rule 12: use horizontal_wrapped for filter rows so they reflow
        // rather than clip when the window is narrow.
        ui.horizontal_wrapped(|ui| {
            ui.label("Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                ui.ctx().request_repaint();
            }
            ui.separator();
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

        // SDK Rule 10: pre-compute shared state before multi-closure calls.
        let filtered_rows = self.filtered_rows(defs);
        let selected_idx = self.selected_landscape;
        let preview_snapshot: Option<LandscapeDefinition> = selected_idx
            .filter(|&i| i < defs.len())
            .map(|i| defs[i].clone());

        // Texture validation: only do the blocking file reads when the selected
        // definition changes — not on every render frame. Without this cache the
        // two fs::read_to_string + ron::from_str calls run at 60 fps and stall the
        // frame loop long enough to time out the right-click context menu.
        if let Some(def) = &preview_snapshot {
            let cached_id = self.texture_validation_cache.as_ref().map(|(id, _)| *id);
            if cached_id != Some(def.id) {
                let status = landscape_texture_validation_status(def, campaign_dir);
                self.texture_validation_cache = Some((def.id, status));
            }
        } else {
            self.texture_validation_cache = None;
        }
        let texture_status: Option<String> = self
            .texture_validation_cache
            .as_ref()
            .map(|(_, s)| s.clone());

        // Deferred mutations applied after show_split (Rule 10).
        let mut pending_selection: Option<usize> = None;
        let mut pending_edit: Option<usize> = None;
        let mut pending_delete: Option<usize> = None;

        // SDK Rule 9: always use TwoColumnLayout for list/detail splits.
        // TwoColumnLayout::show_split already wraps both columns in a ScrollArea,
        // so the closures must NOT add another ScrollArea inside — nesting them
        // produces near-zero inner height and hides all list content.
        TwoColumnLayout::new("landscape_editor").show_split(
            ui,
            |left_ui| {
                // Group rows by category.
                for category in LandscapeCategory::all() {
                    let category_rows: Vec<usize> = filtered_rows
                        .iter()
                        .copied()
                        .filter(|&idx| defs[idx].category == *category)
                        .collect();
                    if category_rows.is_empty() {
                        continue;
                    }

                    // SDK Rule 1: wrap loop body in push_id with stable key.
                    left_ui.push_id(*category as u8, |ui| {
                        ui.heading(category.name());
                        for idx in category_rows {
                            let definition = &defs[idx];
                            ui.push_id(definition.id, |ui| {
                                let selected = selected_idx == Some(idx);
                                let (clicked, action) = show_standard_list_item(
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
                                    pending_selection = Some(idx);
                                    ui.ctx().request_repaint();
                                }
                                if action == ItemAction::Edit {
                                    pending_edit = Some(idx);
                                    ui.ctx().request_repaint();
                                }
                                if action == ItemAction::Delete {
                                    pending_delete = Some(idx);
                                    ui.ctx().request_repaint();
                                }
                            });
                        }
                        ui.add_space(6.0);
                    });
                }

                if filtered_rows.is_empty() {
                    left_ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No landscape definitions found.")
                                .weak()
                                .italics(),
                        );
                    });
                }
            },
            |right_ui| {
                show_landscape_preview(
                    right_ui,
                    preview_snapshot.as_ref(),
                    texture_status.as_deref(),
                );
            },
        );

        // Apply deferred mutations after show_split — no active closure borrows.
        if let Some(idx) = pending_selection {
            self.selected_landscape = Some(idx);
        }
        if let Some(idx) = pending_edit {
            self.selected_landscape = Some(idx);
            self.enter_edit(idx, defs, campaign_dir);
        }
        if let Some(idx) = pending_delete {
            if idx < defs.len() {
                let deleted = defs.remove(idx);
                // Adjust the stored selection so it stays valid after the removal.
                match self.selected_landscape {
                    Some(sel) if sel == idx => self.selected_landscape = None,
                    Some(sel) if sel > idx => self.selected_landscape = Some(sel - 1),
                    _ => {}
                }
                // Remove the associated mesh registry entry when the landscape
                // had a custom mesh — keeps landscape_mesh_registry.ron in sync.
                if let (Some(mesh_id), Some(dir)) = (deleted.mesh_id, campaign_dir) {
                    let _ = remove_landscape_mesh_registry_entry(dir, mesh_id);
                }
                self.texture_validation_cache = None;
                *unsaved_changes = true;
                ui.ctx().request_repaint();
            }
        }
    }

    // =========================================================================
    // Edit view
    // =========================================================================

    fn enter_edit(
        &mut self,
        idx: usize,
        defs: &[LandscapeDefinition],
        campaign_dir: Option<&Path>,
    ) {
        if idx >= defs.len() {
            return;
        }
        let def = &defs[idx];
        self.edit_index = Some(idx);
        self.tags_buffer = def.tags.join(", ");
        self.description_buffer = def.description.clone().unwrap_or_default();
        self.icon_buffer = def.icon.clone().unwrap_or_default();
        self.edit_buffer = Some(def.clone());
        self.available_meshes = load_available_meshes(campaign_dir);
        self.mode = LandscapeEditorMode::Edit;
    }

    fn apply_edit(&mut self, defs: &mut [LandscapeDefinition]) {
        let (Some(idx), Some(mut buf)) = (self.edit_index, self.edit_buffer.take()) else {
            return;
        };
        buf.tags = self
            .tags_buffer
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        buf.description = if self.description_buffer.is_empty() {
            None
        } else {
            Some(self.description_buffer.clone())
        };
        buf.icon = if self.icon_buffer.is_empty() {
            None
        } else {
            Some(self.icon_buffer.clone())
        };
        if idx < defs.len() {
            defs[idx] = buf;
        }
        self.edit_index = None;
    }

    fn show_edit(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut [LandscapeDefinition],
        unsaved_changes: &mut bool,
    ) {
        let Some(buf) = self.edit_buffer.as_mut() else {
            // Shouldn't happen — guard and fall back to list mode.
            self.mode = LandscapeEditorMode::List;
            return;
        };

        ui.heading(format!("Edit: {}", buf.name));
        ui.separator();

        // Pre-compute mesh options from the cached list before entering the
        // ScrollArea/Grid closures to avoid closure-capture conflicts between
        // the `buf` borrow (from self.edit_buffer) and self.available_meshes.
        let mesh_options_snapshot: Vec<(u32, String)> = self
            .available_meshes
            .iter()
            .map(|m| (m.id, format!("#{} \u{2013} {}", m.id, m.name)))
            .collect();

        // Reserve ~44px for the separator + button row below, so the ScrollArea
        // never consumes all available height and the action buttons stay visible.
        let footer_reserved = 44.0;
        let scroll_max_height = (ui.available_height() - footer_reserved).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("landscape_editor_edit_scroll")
            .max_height(scroll_max_height)
            .show(ui, |ui| {
                egui::Grid::new("landscape_editor_edit_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("ID:");
                        ui.label(buf.id.to_string());
                        ui.end_row();

                        ui.label("Name:");
                        ui.text_edit_singleline(&mut buf.name);
                        ui.end_row();

                        ui.label("Category:");
                        egui::ComboBox::from_id_salt("landscape_edit_category")
                            .selected_text(buf.category.name())
                            .show_ui(ui, |ui| {
                                for category in LandscapeCategory::all() {
                                    ui.selectable_value(
                                        &mut buf.category,
                                        *category,
                                        category.name(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Default scale:");
                        ui.add(
                            egui::DragValue::new(&mut buf.default_scale)
                                .speed(0.01)
                                .range(0.01..=100.0),
                        );
                        ui.end_row();

                        ui.label("Blocking:");
                        ui.checkbox(&mut buf.flags.blocking, "");
                        ui.end_row();

                        ui.label("Mesh:");
                        {
                            let mesh_options: Vec<(u32, String)> = mesh_options_snapshot
                                .iter()
                                .map(|(id, label)| (*id, label.clone()))
                                .collect();
                            if mesh_options.is_empty() {
                                ui.label(
                                    buf.mesh_id
                                        .map(|id| format!("#{id}"))
                                        .unwrap_or_else(|| "None (procedural)".to_string()),
                                )
                                .on_hover_text(
                                    "Open a campaign with imported landscape meshes to assign one here.",
                                );
                            } else {
                                let selected_text = mesh_options
                                    .iter()
                                    .find(|(id, _)| Some(*id) == buf.mesh_id)
                                    .map(|(_, label)| label.clone())
                                    .unwrap_or_else(|| "None (procedural)".to_string());
                                egui::ComboBox::from_id_salt("landscape_edit_mesh_id")
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut buf.mesh_id,
                                            None,
                                            "None (procedural)",
                                        );
                                        ui.separator();
                                        for (mesh_id, label) in &mesh_options {
                                            ui.selectable_value(
                                                &mut buf.mesh_id,
                                                Some(*mesh_id),
                                                label.as_str(),
                                            );
                                        }
                                    });
                            }
                        }
                        ui.end_row();

                        ui.label("Icon:");
                        ui.text_edit_singleline(&mut self.icon_buffer);
                        ui.end_row();

                        ui.label("Tags:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tags_buffer)
                                .hint_text("comma-separated, e.g. forest, oak"),
                        );
                        ui.end_row();

                        ui.label("Description:");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.description_buffer)
                                .desired_rows(3)
                                .hint_text("Optional flavor text"),
                        );
                        ui.end_row();
                    });
            });

        ui.separator();

        // SDK Rule 16: edit screens must end with Back to List / Save / Cancel.
        // SDK Rule 12: use horizontal_wrapped so buttons don't clip on narrow windows.
        ui.horizontal_wrapped(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.mode = LandscapeEditorMode::List;
                ui.ctx().request_repaint();
            }
            if ui.button("💾 Save").clicked() {
                self.apply_edit(defs);
                *unsaved_changes = true;
                self.mode = LandscapeEditorMode::List;
                ui.ctx().request_repaint();
            }
            if ui.button("✕ Cancel").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.mode = LandscapeEditorMode::List;
                ui.ctx().request_repaint();
            }
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
}

/// Renders the preview panel for a single landscape definition.
///
/// `texture_status` is the pre-computed result of `landscape_texture_validation_status`,
/// cached by the caller so this function never touches the filesystem.
///
/// Shows a placeholder when `definition` is `None`.
fn show_landscape_preview(
    ui: &mut egui::Ui,
    definition: Option<&LandscapeDefinition>,
    texture_status: Option<&str>,
) {
    let Some(definition) = definition else {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Select a landscape definition to preview it.")
                    .weak()
                    .italics(),
            );
        });
        return;
    };

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
        texture_status.unwrap_or("unknown")
    ));
    if !definition.tags.is_empty() {
        ui.label(format!("Tags: {}", definition.tags.join(", ")));
    }
    if let Some(description) = &definition.description {
        ui.separator();
        ui.label(description);
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

/// Reads `data/landscape_mesh_registry.ron` and returns the list of available
/// mesh entries for the Mesh picker ComboBox in the edit form.
///
/// Returns an empty `Vec` when `campaign_dir` is `None`, the file does not
/// exist, or it cannot be parsed — none of those conditions are errors from the
/// editor's perspective.
fn load_available_meshes(campaign_dir: Option<&Path>) -> Vec<CreatureReference> {
    let Some(dir) = campaign_dir else {
        return Vec::new();
    };
    let registry_path = dir.join("data/landscape_mesh_registry.ron");
    let Ok(contents) = fs::read_to_string(&registry_path) else {
        return Vec::new();
    };
    ron::from_str::<Vec<CreatureReference>>(&contents).unwrap_or_default()
}

/// Removes the `landscape_mesh_registry.ron` entry whose `id` matches `mesh_id`.
///
/// Called when a [`LandscapeDefinition`] that references a custom mesh is
/// deleted, so the registry stays in sync with the definition list.
///
/// Silently succeeds if the registry file does not exist, if the ID is not
/// present, or if any I/O error occurs — a failed registry pruning must never
/// block the delete operation in the UI.
///
/// Returns `true` when an entry was actually removed and the file rewritten.
fn remove_landscape_mesh_registry_entry(campaign_dir: &Path, mesh_id: u32) -> bool {
    let registry_path = campaign_dir.join("data/landscape_mesh_registry.ron");
    let Ok(contents) = fs::read_to_string(&registry_path) else {
        return false;
    };
    let Ok(mut refs) = ron::from_str::<Vec<CreatureReference>>(&contents) else {
        return false;
    };
    let before = refs.len();
    refs.retain(|entry| entry.id != mesh_id);
    if refs.len() == before {
        return false; // nothing to remove
    }
    let Ok(updated) = ron::ser::to_string_pretty(&refs, ron::ser::PrettyConfig::new()) else {
        return false;
    };
    fs::write(&registry_path, updated).is_ok()
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
        assert_eq!(state.mode, LandscapeEditorMode::List);
        assert!(state.edit_buffer.is_none());
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

    #[test]
    fn test_enter_edit_populates_buffers() {
        let defs = vec![landscape_definition(
            42,
            "Short Oak",
            LandscapeCategory::Tree,
            &["oak", "short"],
        )];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, None);

        assert_eq!(state.mode, LandscapeEditorMode::Edit);
        assert_eq!(state.edit_index, Some(0));
        assert!(state.edit_buffer.is_some());
        assert_eq!(state.tags_buffer, "oak, short");
        assert!(state.description_buffer.is_empty());
        assert!(state.icon_buffer.is_empty());
    }

    #[test]
    fn test_apply_edit_writes_back_to_defs() {
        let mut defs = vec![landscape_definition(
            1,
            "Oak Tree",
            LandscapeCategory::Tree,
            &[],
        )];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, None);

        // Mutate the buffer as the user would.
        state.edit_buffer.as_mut().unwrap().name = "Short Oak".to_string();
        state.tags_buffer = "oak, short".to_string();
        state.description_buffer = "A small oak.".to_string();

        state.apply_edit(&mut defs);

        assert_eq!(defs[0].name, "Short Oak");
        assert_eq!(defs[0].tags, vec!["oak", "short"]);
        assert_eq!(defs[0].description.as_deref(), Some("A small oak."));
        // apply_edit writes data back and clears the buffer; the caller
        // (show_edit) is responsible for transitioning mode back to List.
        assert!(state.edit_buffer.is_none());
    }

    #[test]
    fn test_apply_edit_empty_description_becomes_none() {
        let mut defs = vec![landscape_definition(
            1,
            "Rock",
            LandscapeCategory::Rock,
            &[],
        )];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, None);
        state.description_buffer = String::new();

        state.apply_edit(&mut defs);

        assert!(defs[0].description.is_none());
    }

    #[test]
    fn test_delete_clears_selection_when_selected_item_removed() {
        let mut defs = vec![
            landscape_definition(1, "Oak Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(2, "Pine Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(3, "Granite Rock", LandscapeCategory::Rock, &[]),
        ];
        let mut state = LandscapeEditorState::new();
        state.selected_landscape = Some(1); // Pine Tree selected

        // Simulate delete of index 1 (the selected item).
        let idx = 1;
        defs.remove(idx);
        match state.selected_landscape {
            Some(sel) if sel == idx => state.selected_landscape = None,
            Some(sel) if sel > idx => state.selected_landscape = Some(sel - 1),
            _ => {}
        }

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "Oak Tree");
        assert_eq!(defs[1].name, "Granite Rock");
        assert_eq!(state.selected_landscape, None);
    }

    #[test]
    fn test_delete_shifts_selection_down_when_earlier_item_removed() {
        let mut defs = vec![
            landscape_definition(1, "Oak Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(2, "Pine Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(3, "Granite Rock", LandscapeCategory::Rock, &[]),
        ];
        let mut state = LandscapeEditorState::new();
        state.selected_landscape = Some(2); // Granite Rock selected

        // Simulate delete of index 0 (an earlier item).
        let idx = 0;
        defs.remove(idx);
        match state.selected_landscape {
            Some(sel) if sel == idx => state.selected_landscape = None,
            Some(sel) if sel > idx => state.selected_landscape = Some(sel - 1),
            _ => {}
        }

        assert_eq!(defs.len(), 2);
        // Selection shifted from 2 → 1 after the item at 0 was removed.
        assert_eq!(state.selected_landscape, Some(1));
        assert_eq!(defs[1].name, "Granite Rock");
    }

    #[test]
    fn test_delete_leaves_selection_unchanged_when_later_item_removed() {
        let mut defs = vec![
            landscape_definition(1, "Oak Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(2, "Pine Tree", LandscapeCategory::Tree, &[]),
            landscape_definition(3, "Granite Rock", LandscapeCategory::Rock, &[]),
        ];
        let mut state = LandscapeEditorState::new();
        state.selected_landscape = Some(0); // Oak Tree selected

        // Simulate delete of index 2 (a later item).
        let idx = 2;
        defs.remove(idx);
        match state.selected_landscape {
            Some(sel) if sel == idx => state.selected_landscape = None,
            Some(sel) if sel > idx => state.selected_landscape = Some(sel - 1),
            _ => {}
        }

        // Selection at 0 is unaffected by deleting index 2.
        assert_eq!(defs.len(), 2);
        assert_eq!(state.selected_landscape, Some(0));
    }

    #[test]
    fn test_remove_landscape_mesh_registry_entry_removes_matching_id() {
        use antares::domain::visual::CreatureReference;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let registry = vec![
            CreatureReference {
                id: 11000,
                name: "Mesh A".into(),
                filepath: "assets/landscape/mesh_a.ron".into(),
            },
            CreatureReference {
                id: 11001,
                name: "Mesh B".into(),
                filepath: "assets/landscape/mesh_b.ron".into(),
            },
            CreatureReference {
                id: 11002,
                name: "Mesh C".into(),
                filepath: "assets/landscape/mesh_c.ron".into(),
            },
        ];
        let registry_path = data_dir.join("landscape_mesh_registry.ron");
        fs::write(
            &registry_path,
            ron::ser::to_string_pretty(&registry, ron::ser::PrettyConfig::new()).unwrap(),
        )
        .unwrap();

        let removed = remove_landscape_mesh_registry_entry(dir.path(), 11001);
        assert!(removed, "should report that an entry was removed");

        let contents = fs::read_to_string(&registry_path).unwrap();
        let updated: Vec<CreatureReference> = ron::from_str(&contents).unwrap();
        assert_eq!(updated.len(), 2);
        assert!(updated.iter().all(|e| e.id != 11001));
        assert!(updated.iter().any(|e| e.id == 11000));
        assert!(updated.iter().any(|e| e.id == 11002));
    }

    #[test]
    fn test_remove_landscape_mesh_registry_entry_returns_false_for_missing_id() {
        use antares::domain::visual::CreatureReference;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let registry = vec![CreatureReference {
            id: 11000,
            name: "Mesh A".into(),
            filepath: "assets/landscape/mesh_a.ron".into(),
        }];
        let registry_path = data_dir.join("landscape_mesh_registry.ron");
        fs::write(
            &registry_path,
            ron::ser::to_string_pretty(&registry, ron::ser::PrettyConfig::new()).unwrap(),
        )
        .unwrap();

        let removed = remove_landscape_mesh_registry_entry(dir.path(), 99999);
        assert!(!removed, "should return false when id is not in registry");

        // File should be unchanged.
        let contents = fs::read_to_string(&registry_path).unwrap();
        let unchanged: Vec<CreatureReference> = ron::from_str(&contents).unwrap();
        assert_eq!(unchanged.len(), 1);
    }

    #[test]
    fn test_remove_landscape_mesh_registry_entry_returns_false_when_no_file() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        // No registry file created.
        let removed = remove_landscape_mesh_registry_entry(dir.path(), 11000);
        assert!(!removed);
    }

    #[test]
    fn test_enter_edit_populates_available_meshes_from_registry() {
        use antares::domain::visual::CreatureReference;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let registry = vec![
            CreatureReference {
                id: 11000,
                name: "Oak Tree Mesh".into(),
                filepath: "assets/landscape/oak_tree.ron".into(),
            },
            CreatureReference {
                id: 11001,
                name: "Pine Tree Mesh".into(),
                filepath: "assets/landscape/pine_tree.ron".into(),
            },
        ];
        fs::write(
            data_dir.join("landscape_mesh_registry.ron"),
            ron::ser::to_string_pretty(&registry, ron::ser::PrettyConfig::new()).unwrap(),
        )
        .unwrap();

        let defs = vec![landscape_definition(
            1,
            "Oak Tree",
            LandscapeCategory::Tree,
            &[],
        )];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, Some(dir.path()));

        assert_eq!(state.available_meshes.len(), 2);
        assert_eq!(state.available_meshes[0].id, 11000);
        assert_eq!(state.available_meshes[0].name, "Oak Tree Mesh");
        assert_eq!(state.available_meshes[1].id, 11001);
    }

    #[test]
    fn test_enter_edit_with_no_campaign_dir_produces_empty_mesh_list() {
        let defs = vec![landscape_definition(
            1,
            "Rock",
            LandscapeCategory::Rock,
            &[],
        )];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, None);

        assert!(
            state.available_meshes.is_empty(),
            "no campaign dir → no available meshes"
        );
    }

    #[test]
    fn test_apply_edit_saves_mesh_id_assignment() {
        use antares::domain::world::landscape::LandscapeFlags;

        let mut defs = vec![antares::domain::world::landscape::LandscapeDefinition {
            id: 5,
            name: "Oak Tree".to_string(),
            category: LandscapeCategory::Tree,
            default_scale: 1.0,
            color_tint: None,
            flags: LandscapeFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }];
        let mut state = LandscapeEditorState::new();
        state.enter_edit(0, &defs, None);

        // Simulate the user picking mesh #11000 in the ComboBox.
        state.edit_buffer.as_mut().unwrap().mesh_id = Some(11000);

        state.apply_edit(&mut defs);

        assert_eq!(
            defs[0].mesh_id,
            Some(11000),
            "mesh_id should be saved back to the definition"
        );
    }
}
