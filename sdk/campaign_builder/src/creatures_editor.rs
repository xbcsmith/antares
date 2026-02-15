// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::types::CreatureId;
use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
use eframe::egui;
use std::path::PathBuf;

/// Editor mode for creatures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreaturesEditorMode {
    List,
    Add,
    Edit,
}

/// State for the creatures editor
pub struct CreaturesEditorState {
    pub mode: CreaturesEditorMode,
    pub search_query: String,
    pub selected_creature: Option<usize>,
    pub edit_buffer: CreatureDefinition,
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
    pub show_preview: bool,

    // Editor toggles
    pub show_mesh_list: bool,
    pub show_mesh_editor: bool,
    pub selected_mesh_index: Option<usize>,

    // Mesh editing buffer
    pub mesh_edit_buffer: Option<MeshDefinition>,
    pub mesh_transform_buffer: Option<MeshTransform>,

    // Preview state
    pub preview_dirty: bool,
}

impl Default for CreaturesEditorState {
    fn default() -> Self {
        Self {
            mode: CreaturesEditorMode::List,
            search_query: String::new(),
            selected_creature: None,
            edit_buffer: Self::default_creature(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            show_preview: true,
            show_mesh_list: true,
            show_mesh_editor: false,
            selected_mesh_index: None,
            mesh_edit_buffer: None,
            mesh_transform_buffer: None,
            preview_dirty: false,
        }
    }
}

impl CreaturesEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_creature() -> CreatureDefinition {
        CreatureDefinition {
            id: 0,
            name: "New Creature".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    /// Shows the creatures editor UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `creatures` - Mutable reference to the creatures list for editing
    /// * `campaign_dir` - Optional campaign directory path for file operations
    /// * `creatures_file` - Name of the creatures file
    /// * `unsaved_changes` - Flag to track if there are unsaved changes
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` with a status message if an action was performed
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        creatures: &mut Vec<CreatureDefinition>,
        campaign_dir: &Option<PathBuf>,
        creatures_file: &str,
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        match self.mode {
            CreaturesEditorMode::List => {
                self.show_list_mode(ui, creatures.as_mut_slice(), unsaved_changes)
            }
            CreaturesEditorMode::Add | CreaturesEditorMode::Edit => {
                self.show_edit_mode(ui, creatures, unsaved_changes)
            }
        }
    }

    fn show_list_mode(
        &mut self,
        ui: &mut egui::Ui,
        creatures: &mut [CreatureDefinition],
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        let result_message: Option<String> = None;

        // Toolbar
        let toolbar_action = EditorToolbar::new("creatures_toolbar")
            .with_search(&mut self.search_query)
            .show(ui);

        match toolbar_action {
            ToolbarAction::New => {
                self.mode = CreaturesEditorMode::Add;
                self.edit_buffer = Self::default_creature();
                self.edit_buffer.id = self.next_available_id(creatures);
                self.selected_mesh_index = None;
                self.mesh_edit_buffer = None;
                self.mesh_transform_buffer = None;
            }
            ToolbarAction::Save => {
                // Save creatures to campaign
            }
            ToolbarAction::Load => {
                // Load creatures from file
            }
            ToolbarAction::Import => {
                // Import creatures from RON text
            }
            ToolbarAction::Export => {
                // Export creatures to file
            }
            ToolbarAction::Reload => {
                // Reload creatures from campaign
            }
            ToolbarAction::None => {
                // No action triggered
            }
        }

        // Action buttons for selected creature (separate from toolbar)

        ui.separator();

        // Creature list
        egui::ScrollArea::vertical().show(ui, |ui| {
            let filtered_creatures: Vec<(usize, &CreatureDefinition)> = creatures
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    self.search_query.is_empty()
                        || c.name
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                        || c.id.to_string().contains(&self.search_query)
                })
                .collect();

            if filtered_creatures.is_empty() {
                ui.label("No creatures found. Click 'Add' to create one.");
            } else {
                for (idx, creature) in filtered_creatures {
                    let is_selected = self.selected_creature == Some(idx);

                    let response = ui.selectable_label(
                        is_selected,
                        format!(
                            "{} (ID: {}, {} mesh{})",
                            creature.name,
                            creature.id,
                            creature.meshes.len(),
                            if creature.meshes.len() == 1 { "" } else { "es" }
                        ),
                    );

                    if response.clicked() {
                        self.selected_creature = Some(idx);
                    }

                    if response.double_clicked() {
                        self.mode = CreaturesEditorMode::Edit;
                        self.edit_buffer = creature.clone();
                        self.selected_mesh_index = None;
                        self.mesh_edit_buffer = None;
                        self.mesh_transform_buffer = None;
                        self.preview_dirty = true;
                    }
                }
            }
        });

        result_message
    }

    fn show_edit_mode(
        &mut self,
        ui: &mut egui::Ui,
        creatures: &mut Vec<CreatureDefinition>,
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        let mut result_message: Option<String> = None;

        // Action buttons
        let action = ActionButtons::new().show(ui);

        match action {
            ItemAction::Edit => {
                // Edit mode is already set
            }
            ItemAction::Delete => {
                if let Some(idx) = self.selected_creature {
                    if idx < creatures.len() {
                        let name = creatures[idx].name.clone();
                        creatures.remove(idx);
                        self.selected_creature = None;
                        self.mode = CreaturesEditorMode::List;
                        *unsaved_changes = true;
                        result_message = Some(format!("Deleted creature: {}", name));
                    }
                }
            }
            ItemAction::Duplicate => {
                if let Some(idx) = self.selected_creature {
                    if idx < creatures.len() {
                        let mut new_creature = creatures[idx].clone();
                        new_creature.id = self.next_available_id(creatures);
                        new_creature.name = format!("{} (Copy)", new_creature.name);
                        creatures.push(new_creature.clone());
                        *unsaved_changes = true;
                        result_message =
                            Some(format!("Duplicated creature: {}", new_creature.name));
                    }
                }
            }
            ItemAction::Export => {
                // Export implementation
            }
            ItemAction::None => {
                // No action triggered
            }
        }

        // Save/Cancel buttons for edit mode
        if self.mode == CreaturesEditorMode::Edit || self.mode == CreaturesEditorMode::Add {
            ui.horizontal(|ui| {
                if ui.button("✓ Save").clicked() {
                    // Validate creature
                    if let Err(e) = self.edit_buffer.validate() {
                        result_message = Some(format!("Validation error: {}", e));
                    } else {
                        match self.mode {
                            CreaturesEditorMode::Add => {
                                creatures.push(self.edit_buffer.clone());
                                *unsaved_changes = true;
                                result_message =
                                    Some(format!("Added creature: {}", self.edit_buffer.name));
                            }
                            CreaturesEditorMode::Edit => {
                                if let Some(idx) = self.selected_creature {
                                    if idx < creatures.len() {
                                        creatures[idx] = self.edit_buffer.clone();
                                        *unsaved_changes = true;
                                        result_message = Some(format!(
                                            "Updated creature: {}",
                                            self.edit_buffer.name
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }

                        self.mode = CreaturesEditorMode::List;
                        self.preview_dirty = false;
                    }
                }

                if ui.button("✕ Cancel").clicked() {
                    self.mode = CreaturesEditorMode::List;
                    self.selected_mesh_index = None;
                    self.mesh_edit_buffer = None;
                    self.mesh_transform_buffer = None;
                    self.preview_dirty = false;
                }
            });
        }

        ui.separator();

        // Two-column layout: properties on left, preview on right
        ui.columns(2, |columns| {
            self.show_creature_properties(&mut columns[0]);
            self.show_mesh_list_and_editor(&mut columns[1]);
        });

        result_message
    }

    fn show_creature_properties(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("creature_properties_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("ID:");
                ui.add_enabled(false, egui::DragValue::new(&mut self.edit_buffer.id));
                ui.end_row();

                ui.label("Name:");
                if ui
                    .text_edit_singleline(&mut self.edit_buffer.name)
                    .changed()
                {
                    self.preview_dirty = true;
                }
                ui.end_row();

                ui.label("Scale:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.edit_buffer.scale)
                            .speed(0.01)
                            .range(0.01..=100.0),
                    )
                    .changed()
                {
                    self.preview_dirty = true;
                }
                ui.end_row();

                ui.label("Color Tint:");
                ui.horizontal(|ui| {
                    let has_tint = self.edit_buffer.color_tint.is_some();
                    let mut enable_tint = has_tint;

                    if ui.checkbox(&mut enable_tint, "").changed() {
                        if enable_tint {
                            self.edit_buffer.color_tint = Some([1.0, 1.0, 1.0, 1.0]);
                        } else {
                            self.edit_buffer.color_tint = None;
                        }
                        self.preview_dirty = true;
                    }

                    if let Some(tint) = &mut self.edit_buffer.color_tint {
                        if ui.color_edit_button_rgba_unmultiplied(tint).changed() {
                            self.preview_dirty = true;
                        }
                    }
                });
                ui.end_row();
            });
    }

    fn show_mesh_list_and_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Meshes");

        ui.horizontal(|ui| {
            if ui.button("➕ Add Mesh").clicked() {
                self.edit_buffer.meshes.push(MeshDefinition {
                    vertices: vec![],
                    indices: vec![],
                    normals: None,
                    uvs: None,
                    color: [1.0, 1.0, 1.0, 1.0],
                    lod_levels: None,
                    lod_distances: None,
                    material: None,
                    texture_path: None,
                });
                self.edit_buffer
                    .mesh_transforms
                    .push(MeshTransform::identity());
                self.preview_dirty = true;
            }

            if let Some(mesh_idx) = self.selected_mesh_index {
                if ui.button("➖ Remove Mesh").clicked() {
                    if mesh_idx < self.edit_buffer.meshes.len() {
                        self.edit_buffer.meshes.remove(mesh_idx);
                        self.edit_buffer.mesh_transforms.remove(mesh_idx);
                        self.selected_mesh_index = None;
                        self.mesh_edit_buffer = None;
                        self.mesh_transform_buffer = None;
                        self.preview_dirty = true;
                    }
                }
            }
        });

        ui.separator();

        // Mesh list
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                if self.edit_buffer.meshes.is_empty() {
                    ui.label("No meshes. Add a mesh to get started.");
                } else {
                    for (idx, mesh) in self.edit_buffer.meshes.iter().enumerate() {
                        let is_selected = self.selected_mesh_index == Some(idx);
                        let label = format!(
                            "Mesh {} ({} verts, {} tris)",
                            idx,
                            mesh.vertices.len(),
                            mesh.indices.len() / 3
                        );

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_mesh_index = Some(idx);
                            self.mesh_edit_buffer = Some(mesh.clone());
                            self.mesh_transform_buffer =
                                Some(self.edit_buffer.mesh_transforms[idx]);
                        }
                    }
                }
            });

        ui.separator();

        // Mesh editor for selected mesh
        if let Some(mesh_idx) = self.selected_mesh_index {
            if mesh_idx < self.edit_buffer.meshes.len() {
                ui.heading(format!("Mesh {} Properties", mesh_idx));

                // Mesh transform
                if let Some(transform) = self.mesh_transform_buffer.as_mut() {
                    ui.collapsing("Transform", |ui| {
                        egui::Grid::new("mesh_transform_grid")
                            .num_columns(2)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Position:");
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[2])
                                                .speed(0.01),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[1])
                                                .speed(0.01),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[2])
                                                .speed(0.01),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.end_row();

                                ui.label("Rotation:");
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[0])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[1])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[2])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.end_row();

                                ui.label("Scale:");
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[0])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[1])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                    ui.label("Z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.scale[2])
                                                .speed(0.01)
                                                .range(0.01..=100.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.end_row();
                            });
                    });
                }

                // Mesh properties
                if let Some(mesh) = self.mesh_edit_buffer.as_mut() {
                    ui.collapsing("Color", |ui| {
                        if ui
                            .color_edit_button_rgba_unmultiplied(&mut mesh.color)
                            .changed()
                        {
                            self.edit_buffer.meshes[mesh_idx].color = mesh.color;
                            self.preview_dirty = true;
                        }
                    });

                    ui.collapsing("Geometry", |ui| {
                        ui.label(format!("Vertices: {}", mesh.vertices.len()));
                        ui.label(format!("Triangles: {}", mesh.indices.len() / 3));
                        ui.label(format!(
                            "Normals: {}",
                            if mesh.normals.is_some() { "Yes" } else { "No" }
                        ));
                        ui.label(format!(
                            "UVs: {}",
                            if mesh.uvs.is_some() { "Yes" } else { "No" }
                        ));
                    });
                }
            }
        }
    }

    fn next_available_id(&self, creatures: &[CreatureDefinition]) -> CreatureId {
        creatures
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creatures_editor_state_initialization() {
        let state = CreaturesEditorState::new();
        assert_eq!(state.mode, CreaturesEditorMode::List);
        assert!(state.search_query.is_empty());
        assert_eq!(state.selected_creature, None);
        assert!(state.show_preview);
    }

    #[test]
    fn test_default_creature_creation() {
        let creature = CreaturesEditorState::default_creature();
        assert_eq!(creature.id, 0);
        assert_eq!(creature.name, "New Creature");
        assert!(creature.meshes.is_empty());
        assert!(creature.mesh_transforms.is_empty());
        assert_eq!(creature.scale, 1.0);
        assert_eq!(creature.color_tint, None);
    }

    #[test]
    fn test_next_available_id_empty() {
        let state = CreaturesEditorState::new();
        let creatures = vec![];
        assert_eq!(state.next_available_id(&creatures), 1);
    }

    #[test]
    fn test_next_available_id_with_creatures() {
        let state = CreaturesEditorState::new();
        let creatures = vec![
            CreatureDefinition {
                id: 1,
                name: "Creature 1".to_string(),
                meshes: vec![],
                mesh_transforms: vec![],
                scale: 1.0,
                color_tint: None,
            },
            CreatureDefinition {
                id: 5,
                name: "Creature 5".to_string(),
                meshes: vec![],
                mesh_transforms: vec![],
                scale: 1.0,
                color_tint: None,
            },
        ];
        assert_eq!(state.next_available_id(&creatures), 6);
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = CreaturesEditorState::new();
        assert_eq!(state.mode, CreaturesEditorMode::List);

        state.mode = CreaturesEditorMode::Add;
        assert_eq!(state.mode, CreaturesEditorMode::Add);

        state.mode = CreaturesEditorMode::Edit;
        assert_eq!(state.mode, CreaturesEditorMode::Edit);

        state.mode = CreaturesEditorMode::List;
        assert_eq!(state.mode, CreaturesEditorMode::List);
    }

    #[test]
    fn test_mesh_selection_state() {
        let mut state = CreaturesEditorState::new();
        assert_eq!(state.selected_mesh_index, None);

        state.selected_mesh_index = Some(0);
        assert_eq!(state.selected_mesh_index, Some(0));

        state.selected_mesh_index = None;
        assert_eq!(state.selected_mesh_index, None);
    }

    #[test]
    fn test_preview_dirty_flag() {
        let mut state = CreaturesEditorState::new();
        assert!(!state.preview_dirty);

        state.preview_dirty = true;
        assert!(state.preview_dirty);

        state.preview_dirty = false;
        assert!(!state.preview_dirty);
    }
}
