// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::creature_id_manager::{CreatureCategory, CreatureIdManager};
use crate::creatures_manager::CreaturesManager;
use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::types::CreatureId;
use antares::domain::visual::{
    CreatureDefinition, CreatureReference, MeshDefinition, MeshTransform,
};
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

    // Phase 1: Registry Management UI
    pub category_filter: Option<CreatureCategory>,
    pub show_registry_stats: bool,
    pub id_manager: CreatureIdManager,
    pub selected_registry_entry: Option<usize>,
    pub registry_sort_by: RegistrySortBy,
    pub show_validation_panel: bool,

    // Phase 2: Asset Editor UI
    pub show_primitive_dialog: bool,
    pub primitive_type: PrimitiveType,
    pub primitive_size: f32,
    pub primitive_segments: u32,
    pub primitive_rings: u32,
    pub primitive_use_current_color: bool,
    pub primitive_custom_color: [f32; 4],
    pub primitive_preserve_transform: bool,
    pub primitive_keep_name: bool,
    pub mesh_visibility: Vec<bool>,
    pub show_grid: bool,
    pub show_wireframe: bool,
    pub show_normals: bool,
    pub show_axes: bool,
    pub background_color: [f32; 4],
    pub camera_distance: f32,
    pub uniform_scale: bool,
}

/// Primitive type for mesh generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Cube,
    Sphere,
    Cylinder,
    Pyramid,
    Cone,
}

/// Sort order for registry list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrySortBy {
    Id,
    Name,
    Category,
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
            category_filter: None,
            show_registry_stats: true,
            id_manager: CreatureIdManager::new(),
            selected_registry_entry: None,
            registry_sort_by: RegistrySortBy::Id,
            show_validation_panel: false,
            show_primitive_dialog: false,
            primitive_type: PrimitiveType::Cube,
            primitive_size: 1.0,
            primitive_segments: 16,
            primitive_rings: 16,
            primitive_use_current_color: true,
            primitive_custom_color: [0.5, 0.5, 0.5, 1.0],
            primitive_preserve_transform: true,
            primitive_keep_name: true,
            mesh_visibility: Vec::new(),
            show_grid: true,
            show_wireframe: false,
            show_normals: false,
            show_axes: true,
            background_color: [0.2, 0.2, 0.25, 1.0],
            camera_distance: 5.0,
            uniform_scale: true,
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
                self.show_registry_mode(ui, creatures, campaign_dir, unsaved_changes)
            }
            CreaturesEditorMode::Add | CreaturesEditorMode::Edit => {
                self.show_edit_mode(ui, creatures, unsaved_changes)
            }
        }
    }

    /// Show registry management mode (Phase 1)
    fn show_registry_mode(
        &mut self,
        ui: &mut egui::Ui,
        creatures: &mut [CreatureDefinition],
        campaign_dir: &Option<PathBuf>,
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        let mut result_message: Option<String> = None;

        // Update ID manager from current creatures
        let references: Vec<CreatureReference> = creatures
            .iter()
            .map(|c| CreatureReference {
                id: c.id,
                name: c.name.clone(),
                filepath: format!(
                    "assets/creatures/{}.ron",
                    c.name.to_lowercase().replace(' ', "_")
                ),
            })
            .collect();
        self.id_manager.update_from_registry(&references);

        // Registry Overview Section
        if self.show_registry_stats {
            ui.horizontal(|ui| {
                ui.label(format!("📊 {} creatures registered", creatures.len()));

                let (monsters, npcs, templates, variants, custom) =
                    self.count_by_category(creatures);
                ui.separator();
                ui.label(format!(
                    "({} Monsters, {} NPCs, {} Templates, {} Variants, {} Custom)",
                    monsters, npcs, templates, variants, custom
                ));
            });
            ui.separator();
        }

        // Toolbar with filters
        ui.horizontal(|ui| {
            let toolbar_action = EditorToolbar::new("creatures_toolbar")
                .with_search(&mut self.search_query)
                .with_total_count(creatures.len())
                .show(ui);

            match toolbar_action {
                ToolbarAction::New => {
                    self.mode = CreaturesEditorMode::Add;
                    self.edit_buffer = Self::default_creature();
                    let suggested_category =
                        self.category_filter.unwrap_or(CreatureCategory::Monsters);
                    self.edit_buffer.id = self.id_manager.suggest_next_id(suggested_category);
                    *unsaved_changes = true;
                }
                ToolbarAction::Save
                | ToolbarAction::Load
                | ToolbarAction::Import
                | ToolbarAction::Export
                | ToolbarAction::Reload => {
                    // Handled by parent
                }
                ToolbarAction::None => {}
            }

            ui.separator();

            // Category filter dropdown
            egui::ComboBox::from_label("Category")
                .selected_text(
                    self.category_filter
                        .map(|c| c.display_name())
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category_filter, None, "All");
                    ui.selectable_value(
                        &mut self.category_filter,
                        Some(CreatureCategory::Monsters),
                        "Monsters",
                    );
                    ui.selectable_value(
                        &mut self.category_filter,
                        Some(CreatureCategory::Npcs),
                        "NPCs",
                    );
                    ui.selectable_value(
                        &mut self.category_filter,
                        Some(CreatureCategory::Templates),
                        "Templates",
                    );
                    ui.selectable_value(
                        &mut self.category_filter,
                        Some(CreatureCategory::Variants),
                        "Variants",
                    );
                    ui.selectable_value(
                        &mut self.category_filter,
                        Some(CreatureCategory::Custom),
                        "Custom",
                    );
                });

            // Sort dropdown
            egui::ComboBox::from_label("Sort")
                .selected_text(match self.registry_sort_by {
                    RegistrySortBy::Id => "By ID",
                    RegistrySortBy::Name => "By Name",
                    RegistrySortBy::Category => "By Category",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.registry_sort_by, RegistrySortBy::Id, "By ID");
                    ui.selectable_value(
                        &mut self.registry_sort_by,
                        RegistrySortBy::Name,
                        "By Name",
                    );
                    ui.selectable_value(
                        &mut self.registry_sort_by,
                        RegistrySortBy::Category,
                        "By Category",
                    );
                });

            if ui.button("🔄 Revalidate").clicked() {
                self.show_validation_panel = true;
                result_message = Some("Validation complete".to_string());
            }
        });

        ui.separator();

        // Registry List View
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Filter and sort creatures
            let mut filtered_creatures: Vec<(usize, &CreatureDefinition)> = creatures
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    // Apply category filter
                    if let Some(cat) = self.category_filter {
                        if CreatureCategory::from_id(c.id) != cat {
                            return false;
                        }
                    }

                    // Apply search filter
                    if !self.search_query.is_empty() {
                        let query = self.search_query.to_lowercase();
                        return c.name.to_lowercase().contains(&query)
                            || c.id.to_string().contains(&query);
                    }

                    true
                })
                .collect();

            // Sort creatures
            match self.registry_sort_by {
                RegistrySortBy::Id => filtered_creatures.sort_by_key(|(_, c)| c.id),
                RegistrySortBy::Name => {
                    filtered_creatures.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name))
                }
                RegistrySortBy::Category => filtered_creatures
                    .sort_by_key(|(_, c)| (CreatureCategory::from_id(c.id) as u8, c.id)),
            }

            if filtered_creatures.is_empty() {
                ui.label("No creatures found. Click 'New' to create one.");
            } else {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ID").strong());
                    ui.separator();
                    ui.label(egui::RichText::new("Name").strong());
                    ui.separator();
                    ui.label(egui::RichText::new("Status").strong());
                    ui.separator();
                    ui.label(egui::RichText::new("Category").strong());
                });
                ui.separator();

                for (idx, creature) in filtered_creatures {
                    let is_selected = self.selected_registry_entry == Some(idx);
                    let category = CreatureCategory::from_id(creature.id);
                    let color = category.color();

                    ui.horizontal(|ui| {
                        // ID badge with category color
                        let id_text = format!("{:03}", creature.id);
                        ui.colored_label(
                            egui::Color32::from_rgb(
                                (color[0] * 255.0) as u8,
                                (color[1] * 255.0) as u8,
                                (color[2] * 255.0) as u8,
                            ),
                            egui::RichText::new(id_text).strong(),
                        );

                        ui.separator();

                        // Creature name (selectable)
                        let response = ui.selectable_label(
                            is_selected,
                            format!(
                                "{} ({} mesh{})",
                                creature.name,
                                creature.meshes.len(),
                                if creature.meshes.len() == 1 { "" } else { "es" }
                            ),
                        );

                        if response.clicked() {
                            self.selected_registry_entry = Some(idx);
                        }

                        if response.double_clicked() {
                            self.mode = CreaturesEditorMode::Edit;
                            self.edit_buffer = creature.clone();
                            self.selected_mesh_index = None;
                            self.mesh_edit_buffer = None;
                            self.mesh_transform_buffer = None;
                            self.preview_dirty = true;
                        }

                        ui.separator();

                        // Status indicator
                        let validation_result = self.id_manager.validate_id(creature.id, category);
                        if validation_result.is_ok() {
                            ui.label("✓");
                        } else {
                            ui.colored_label(egui::Color32::YELLOW, "⚠");
                        }

                        ui.separator();

                        // Category badge
                        ui.label(
                            egui::RichText::new(category.display_name())
                                .small()
                                .background_color(egui::Color32::from_rgb(
                                    (color[0] * 100.0) as u8,
                                    (color[1] * 100.0) as u8,
                                    (color[2] * 100.0) as u8,
                                )),
                        );
                    });
                }
            }
        });

        // Show validation panel if requested
        if self.show_validation_panel {
            ui.separator();
            ui.collapsing("Validation Results", |ui| {
                let conflicts = self.id_manager.check_conflicts();
                if conflicts.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, "✓ No ID conflicts detected");
                } else {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("⚠ {} ID conflict(s) detected", conflicts.len()),
                    );
                    for conflict in conflicts {
                        ui.label(format!(
                            "ID {}: {} creatures ({})",
                            conflict.id,
                            conflict.creature_names.len(),
                            conflict.creature_names.join(", ")
                        ));
                    }
                }
            });
        }

        result_message
    }

    /// Count creatures by category
    fn count_by_category(
        &self,
        creatures: &[CreatureDefinition],
    ) -> (usize, usize, usize, usize, usize) {
        let mut monsters = 0;
        let mut npcs = 0;
        let mut templates = 0;
        let mut variants = 0;
        let mut custom = 0;

        for creature in creatures {
            match CreatureCategory::from_id(creature.id) {
                CreatureCategory::Monsters => monsters += 1,
                CreatureCategory::Npcs => npcs += 1,
                CreatureCategory::Templates => templates += 1,
                CreatureCategory::Variants => variants += 1,
                CreatureCategory::Custom => custom += 1,
            }
        }

        (monsters, npcs, templates, variants, custom)
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

        // Phase 2: Three-panel layout: Mesh List | 3D Preview | Mesh Properties
        egui::SidePanel::left("mesh_list_panel")
            .resizable(true)
            .default_width(250.0)
            .min_width(200.0)
            .max_width(400.0)
            .show_inside(ui, |ui| {
                self.show_mesh_list_panel(ui, unsaved_changes);
            });

        egui::SidePanel::right("mesh_properties_panel")
            .resizable(true)
            .default_width(350.0)
            .min_width(300.0)
            .max_width(500.0)
            .show_inside(ui, |ui| {
                self.show_mesh_properties_panel(ui, unsaved_changes);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.show_preview_panel(ui);
        });

        // Show primitive replacement dialog if active
        if self.show_primitive_dialog {
            self.show_primitive_replacement_dialog(ui.ctx(), unsaved_changes);
        }

        // Bottom panel for creature-level properties
        egui::TopBottomPanel::bottom("creature_properties_bottom")
            .resizable(false)
            .min_height(100.0)
            .show_inside(ui, |ui| {
                self.show_creature_level_properties(ui, unsaved_changes);
            });

        result_message
    }

    /// Show mesh list panel (left, 250px) - Phase 2.1
    fn show_mesh_list_panel(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        ui.heading("Meshes");

        // Mesh list toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ Add Primitive").clicked() {
                self.show_primitive_dialog = true;
                self.primitive_type = PrimitiveType::Cube;
                self.primitive_size = 1.0;
                self.primitive_use_current_color = true;
                self.primitive_preserve_transform = false;
                self.primitive_keep_name = false;
            }

            if let Some(mesh_idx) = self.selected_mesh_index {
                if ui.button("📋 Duplicate").clicked() {
                    if mesh_idx < self.edit_buffer.meshes.len() {
                        let mesh = self.edit_buffer.meshes[mesh_idx].clone();
                        let transform = self.edit_buffer.mesh_transforms[mesh_idx];
                        self.edit_buffer.meshes.push(mesh);
                        self.edit_buffer.mesh_transforms.push(transform);
                        self.mesh_visibility.push(true);
                        *unsaved_changes = true;
                        self.preview_dirty = true;
                    }
                }

                if ui.button("🗑 Delete").clicked() {
                    if mesh_idx < self.edit_buffer.meshes.len() {
                        self.edit_buffer.meshes.remove(mesh_idx);
                        self.edit_buffer.mesh_transforms.remove(mesh_idx);
                        if mesh_idx < self.mesh_visibility.len() {
                            self.mesh_visibility.remove(mesh_idx);
                        }
                        self.selected_mesh_index = None;
                        self.mesh_edit_buffer = None;
                        self.mesh_transform_buffer = None;
                        *unsaved_changes = true;
                        self.preview_dirty = true;
                    }
                }
            }
        });

        ui.separator();

        // Ensure mesh_visibility matches mesh count
        while self.mesh_visibility.len() < self.edit_buffer.meshes.len() {
            self.mesh_visibility.push(true);
        }
        self.mesh_visibility.truncate(self.edit_buffer.meshes.len());

        // Mesh list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.edit_buffer.meshes.is_empty() {
                ui.label("No meshes. Click 'Add Primitive' to get started.");
            } else {
                for (idx, mesh) in self.edit_buffer.meshes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        // Visibility checkbox
                        let mut visible = self.mesh_visibility.get(idx).copied().unwrap_or(true);
                        if ui.checkbox(&mut visible, "").changed() {
                            if idx < self.mesh_visibility.len() {
                                self.mesh_visibility[idx] = visible;
                            }
                            self.preview_dirty = true;
                        }

                        // Color indicator dot
                        let color = egui::Color32::from_rgba_premultiplied(
                            (mesh.color[0] * 255.0) as u8,
                            (mesh.color[1] * 255.0) as u8,
                            (mesh.color[2] * 255.0) as u8,
                            (mesh.color[3] * 255.0) as u8,
                        );
                        ui.colored_label(color, "●");

                        // Mesh name and info
                        let is_selected = self.selected_mesh_index == Some(idx);
                        let default_name = format!("unnamed_mesh_{}", idx);
                        let name = mesh.name.as_deref().unwrap_or(&default_name);
                        let label = format!("{} ({} verts)", name, mesh.vertices.len());

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_mesh_index = Some(idx);
                            self.mesh_edit_buffer = Some(mesh.clone());
                            self.mesh_transform_buffer =
                                Some(self.edit_buffer.mesh_transforms[idx]);
                        }
                    });
                }
            }
        });
    }

    /// Show 3D preview panel (center, flex) - Phase 2.2
    fn show_preview_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Preview");

        // Preview controls overlay
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.show_wireframe, "Wireframe");
            ui.checkbox(&mut self.show_normals, "Normals");
            ui.checkbox(&mut self.show_axes, "Axes");

            if ui.button("🔄 Reset Camera").clicked() {
                self.camera_distance = 5.0;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Camera Distance:");
            ui.add(
                egui::Slider::new(&mut self.camera_distance, 1.0..=10.0)
                    .text("units")
                    .show_value(true),
            );

            ui.label("Background:");
            ui.color_edit_button_rgba_unmultiplied(&mut self.background_color);
        });

        ui.separator();

        // Preview area placeholder
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ui.available_height() - 20.0),
            egui::Sense::click_and_drag(),
        );

        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_premultiplied(
                (self.background_color[0] * 255.0) as u8,
                (self.background_color[1] * 255.0) as u8,
                (self.background_color[2] * 255.0) as u8,
                (self.background_color[3] * 255.0) as u8,
            ),
        );

        // Draw simple placeholder text
        let center = rect.center();
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            "3D Preview (Bevy integration pending)",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );

        // TODO: Integrate actual preview_renderer.rs here
        // - Left-drag: Rotate camera
        // - Right-drag: Pan camera
        // - Scroll: Zoom
        // - Double-click: Focus on selected mesh
        // - Highlight selected mesh
        // - Show mesh coordinate axes when selected
        // - Display bounding box
    }

    /// Show mesh properties panel (right, 350px) - Phase 2.3
    fn show_mesh_properties_panel(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        if let Some(mesh_idx) = self.selected_mesh_index {
            if mesh_idx >= self.edit_buffer.meshes.len() {
                ui.label("Invalid mesh selection");
                return;
            }

            ui.heading(format!("Mesh {} Properties", mesh_idx));
            ui.separator();

            // Mesh Info Section
            ui.collapsing("Mesh Info", |ui| {
                egui::Grid::new("mesh_info_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        let mut name = self.edit_buffer.meshes[mesh_idx]
                            .name
                            .clone()
                            .unwrap_or_default();
                        if ui.text_edit_singleline(&mut name).changed() {
                            self.edit_buffer.meshes[mesh_idx].name =
                                if name.is_empty() { None } else { Some(name) };
                            if let Some(buffer) = &mut self.mesh_edit_buffer {
                                buffer.name = self.edit_buffer.meshes[mesh_idx].name.clone();
                            }
                            *unsaved_changes = true;
                        }
                        ui.end_row();

                        ui.label("Color:");
                        if ui
                            .color_edit_button_rgba_unmultiplied(
                                &mut self.edit_buffer.meshes[mesh_idx].color,
                            )
                            .changed()
                        {
                            if let Some(buffer) = &mut self.mesh_edit_buffer {
                                buffer.color = self.edit_buffer.meshes[mesh_idx].color;
                            }
                            *unsaved_changes = true;
                            self.preview_dirty = true;
                        }
                        ui.end_row();

                        ui.label("Vertices:");
                        ui.label(format!(
                            "{}",
                            self.edit_buffer.meshes[mesh_idx].vertices.len()
                        ));
                        ui.end_row();

                        ui.label("Triangles:");
                        ui.label(format!(
                            "{}",
                            self.edit_buffer.meshes[mesh_idx].indices.len() / 3
                        ));
                        ui.end_row();
                    });
            });

            // Transform Section
            if let Some(transform) = self.mesh_transform_buffer.as_mut() {
                ui.collapsing("Transform", |ui| {
                    egui::Grid::new("mesh_transform_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Translation:");
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[0])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[1])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut transform.translation[2])
                                                .speed(0.01)
                                                .range(-5.0..=5.0),
                                        )
                                        .changed()
                                    {
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label("Rotation (deg):");
                            ui.vertical(|ui| {
                                let mut pitch_deg = transform.rotation[0].to_degrees();
                                let mut yaw_deg = transform.rotation[1].to_degrees();
                                let mut roll_deg = transform.rotation[2].to_degrees();

                                ui.horizontal(|ui| {
                                    ui.label("Pitch:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut pitch_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[0] = pitch_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Yaw:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut yaw_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[1] = yaw_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Roll:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut roll_deg)
                                                .speed(1.0)
                                                .range(0.0..=360.0),
                                        )
                                        .changed()
                                    {
                                        transform.rotation[2] = roll_deg.to_radians();
                                        self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                        *unsaved_changes = true;
                                        self.preview_dirty = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label("Scale:");
                            ui.vertical(|ui| {
                                ui.checkbox(&mut self.uniform_scale, "Uniform scaling");

                                if self.uniform_scale {
                                    let mut uniform = transform.scale[0];
                                    ui.horizontal(|ui| {
                                        ui.label("XYZ:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut uniform)
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            transform.scale = [uniform, uniform, uniform];
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[0])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[1])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.scale[2])
                                                    .speed(0.01)
                                                    .range(0.01..=10.0),
                                            )
                                            .changed()
                                        {
                                            self.edit_buffer.mesh_transforms[mesh_idx] = *transform;
                                            *unsaved_changes = true;
                                            self.preview_dirty = true;
                                        }
                                    });
                                }
                            });
                            ui.end_row();
                        });
                });
            }

            // Geometry Section
            ui.collapsing("Geometry", |ui| {
                let mesh = &self.edit_buffer.meshes[mesh_idx];
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

                // TODO: Add View/Edit Table buttons for vertices/indices/normals
            });

            // Action Buttons
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("🔄 Replace with Primitive").clicked() {
                    self.show_primitive_dialog = true;
                    self.primitive_use_current_color = true;
                    self.primitive_preserve_transform = true;
                    self.primitive_keep_name = true;
                }

                if ui.button("🔍 Validate Mesh").clicked() {
                    // TODO: Implement mesh validation
                }

                if ui.button("↺ Reset Transform").clicked() {
                    self.edit_buffer.mesh_transforms[mesh_idx] = MeshTransform::identity();
                    self.mesh_transform_buffer = Some(MeshTransform::identity());
                    *unsaved_changes = true;
                    self.preview_dirty = true;
                }
            });
        } else {
            ui.label("Select a mesh to edit its properties");
        }
    }

    /// Show creature-level properties (bottom panel) - Phase 2.5
    fn show_creature_level_properties(&mut self, ui: &mut egui::Ui, unsaved_changes: &mut bool) {
        ui.heading("Creature Properties");

        egui::Grid::new("creature_properties_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("ID:");
                let category = CreatureCategory::from_id(self.edit_buffer.id);
                let category_label = format!("{:?}", category);
                ui.horizontal(|ui| {
                    ui.add_enabled(false, egui::DragValue::new(&mut self.edit_buffer.id));
                    ui.label(format!("({})", category_label));
                });
                ui.end_row();

                ui.label("Name:");
                if ui
                    .text_edit_singleline(&mut self.edit_buffer.name)
                    .changed()
                {
                    *unsaved_changes = true;
                    self.preview_dirty = true;
                }
                ui.end_row();

                ui.label("Scale:");
                if ui
                    .add(
                        egui::Slider::new(&mut self.edit_buffer.scale, 0.1..=5.0)
                            .text("units")
                            .logarithmic(true),
                    )
                    .changed()
                {
                    *unsaved_changes = true;
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
                        *unsaved_changes = true;
                        self.preview_dirty = true;
                    }

                    if let Some(tint) = &mut self.edit_buffer.color_tint {
                        if ui.color_edit_button_rgba_unmultiplied(tint).changed() {
                            *unsaved_changes = true;
                            self.preview_dirty = true;
                        }
                    }
                });
                ui.end_row();
            });

        // Validation and file operations
        ui.separator();
        ui.horizontal(|ui| {
            let error_count = 0; // TODO: Implement validation
            let warning_count = 0;
            ui.label(format!(
                "{} errors, {} warnings",
                error_count, warning_count
            ));

            if ui.button("Show Issues").clicked() {
                // TODO: Expand validation panel
            }
        });

        ui.horizontal(|ui| {
            if ui.button("💾 Save Asset").clicked() {
                // Handled by parent Save button
            }

            if ui.button("💾 Save As...").clicked() {
                // TODO: Implement save as dialog
            }

            if ui.button("📋 Export RON").clicked() {
                // TODO: Implement RON export to clipboard
            }

            if ui.button("↺ Revert Changes").clicked() {
                // TODO: Implement revert from file
            }
        });
    }

    /// Show primitive replacement dialog - Phase 2.4
    fn show_primitive_replacement_dialog(
        &mut self,
        ctx: &egui::Context,
        unsaved_changes: &mut bool,
    ) {
        egui::Window::new("Replace with Primitive")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Select Primitive Type");

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.primitive_type, PrimitiveType::Cube, "Cube");
                    ui.selectable_value(&mut self.primitive_type, PrimitiveType::Sphere, "Sphere");
                    ui.selectable_value(
                        &mut self.primitive_type,
                        PrimitiveType::Cylinder,
                        "Cylinder",
                    );
                    ui.selectable_value(
                        &mut self.primitive_type,
                        PrimitiveType::Pyramid,
                        "Pyramid",
                    );
                    ui.selectable_value(&mut self.primitive_type, PrimitiveType::Cone, "Cone");
                });

                ui.separator();

                // Primitive-specific settings
                match self.primitive_type {
                    PrimitiveType::Cube => {
                        ui.label("Cube Settings:");
                        ui.add(
                            egui::Slider::new(&mut self.primitive_size, 0.1..=5.0)
                                .text("Size")
                                .logarithmic(true),
                        );
                    }
                    PrimitiveType::Sphere => {
                        ui.label("Sphere Settings:");
                        ui.add(
                            egui::Slider::new(&mut self.primitive_size, 0.1..=5.0)
                                .text("Radius")
                                .logarithmic(true),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.primitive_segments, 3..=64)
                                .text("Segments"),
                        );
                        ui.add(egui::Slider::new(&mut self.primitive_rings, 2..=64).text("Rings"));
                    }
                    PrimitiveType::Cylinder => {
                        ui.label("Cylinder Settings:");
                        ui.add(
                            egui::Slider::new(&mut self.primitive_size, 0.1..=5.0)
                                .text("Radius")
                                .logarithmic(true),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.primitive_segments, 3..=64)
                                .text("Segments"),
                        );
                    }
                    PrimitiveType::Pyramid => {
                        ui.label("Pyramid Settings:");
                        ui.add(
                            egui::Slider::new(&mut self.primitive_size, 0.1..=5.0)
                                .text("Base Size")
                                .logarithmic(true),
                        );
                    }
                    PrimitiveType::Cone => {
                        ui.label("Cone Settings:");
                        ui.add(
                            egui::Slider::new(&mut self.primitive_size, 0.1..=5.0)
                                .text("Base Radius")
                                .logarithmic(true),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.primitive_segments, 3..=64)
                                .text("Segments"),
                        );
                    }
                }

                ui.separator();

                ui.label("Color:");
                ui.checkbox(
                    &mut self.primitive_use_current_color,
                    "Use current mesh color",
                );
                if !self.primitive_use_current_color {
                    ui.horizontal(|ui| {
                        ui.label("Custom:");
                        ui.color_edit_button_rgba_unmultiplied(&mut self.primitive_custom_color);
                    });
                }

                ui.separator();

                ui.label("Options:");
                ui.checkbox(&mut self.primitive_preserve_transform, "Preserve transform");
                ui.checkbox(&mut self.primitive_keep_name, "Keep mesh name");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Generate").clicked() {
                        self.apply_primitive_replacement(unsaved_changes);
                        self.show_primitive_dialog = false;
                    }

                    if ui.button("✕ Cancel").clicked() {
                        self.show_primitive_dialog = false;
                    }
                });
            });
    }

    /// Apply primitive replacement to selected mesh or create new mesh
    fn apply_primitive_replacement(&mut self, unsaved_changes: &mut bool) {
        use crate::primitive_generators::*;

        // Determine color
        let color = if self.primitive_use_current_color {
            if let Some(mesh_idx) = self.selected_mesh_index {
                if mesh_idx < self.edit_buffer.meshes.len() {
                    self.edit_buffer.meshes[mesh_idx].color
                } else {
                    self.primitive_custom_color
                }
            } else {
                self.primitive_custom_color
            }
        } else {
            self.primitive_custom_color
        };

        // Generate primitive mesh
        let mut new_mesh = match self.primitive_type {
            PrimitiveType::Cube => generate_cube(self.primitive_size, color),
            PrimitiveType::Sphere => generate_sphere(
                self.primitive_size,
                self.primitive_segments,
                self.primitive_rings,
                color,
            ),
            PrimitiveType::Cylinder => generate_cylinder(
                self.primitive_size,
                self.primitive_size * 2.0,
                self.primitive_segments,
                color,
            ),
            PrimitiveType::Pyramid => generate_pyramid(self.primitive_size, color),
            PrimitiveType::Cone => generate_cone(
                self.primitive_size,
                self.primitive_size * 2.0,
                self.primitive_segments,
                color,
            ),
        };

        // Handle name preservation
        if let Some(mesh_idx) = self.selected_mesh_index {
            if mesh_idx < self.edit_buffer.meshes.len() {
                if self.primitive_keep_name {
                    new_mesh.name = self.edit_buffer.meshes[mesh_idx].name.clone();
                }

                // Replace existing mesh
                self.edit_buffer.meshes[mesh_idx] = new_mesh.clone();

                if !self.primitive_preserve_transform {
                    self.edit_buffer.mesh_transforms[mesh_idx] = MeshTransform::identity();
                    self.mesh_transform_buffer = Some(MeshTransform::identity());
                }

                self.mesh_edit_buffer = Some(new_mesh);
            }
        } else {
            // Add as new mesh
            self.edit_buffer.meshes.push(new_mesh);
            self.edit_buffer
                .mesh_transforms
                .push(MeshTransform::identity());
            self.mesh_visibility.push(true);
        }

        *unsaved_changes = true;
        self.preview_dirty = true;
    }

    fn _legacy_show_mesh_list_and_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Meshes");

        ui.horizontal(|ui| {
            if ui.button("➕ Add Mesh").clicked() {
                self.edit_buffer.meshes.push(MeshDefinition {
                    name: None,
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
        assert_eq!(state.category_filter, None);
        assert!(state.show_registry_stats);
        assert_eq!(state.registry_sort_by, RegistrySortBy::Id);
    }

    #[test]
    fn test_default_creature_creation() {
        let creature = CreaturesEditorState::default_creature();
        assert_eq!(creature.id, 0);
        assert_eq!(creature.name, "New Creature");
        assert_eq!(creature.meshes.len(), 0);
        assert_eq!(creature.scale, 1.0);
        assert_eq!(creature.color_tint, None);
    }

    #[test]
    fn test_registry_sort_by_enum() {
        assert_eq!(RegistrySortBy::Id, RegistrySortBy::Id);
        assert_ne!(RegistrySortBy::Id, RegistrySortBy::Name);
    }

    #[test]
    fn test_count_by_category_empty() {
        let state = CreaturesEditorState::new();
        let creatures = vec![];
        let (monsters, npcs, templates, variants, custom) = state.count_by_category(&creatures);
        assert_eq!(monsters, 0);
        assert_eq!(npcs, 0);
        assert_eq!(templates, 0);
        assert_eq!(variants, 0);
        assert_eq!(custom, 0);
    }

    #[test]
    fn test_count_by_category_mixed() {
        let state = CreaturesEditorState::new();
        let creatures = vec![
            CreatureDefinition {
                id: 1,
                name: "Goblin".to_string(),
                meshes: vec![],
                mesh_transforms: vec![],
                scale: 1.0,
                color_tint: None,
            },
            CreatureDefinition {
                id: 2,
                name: "Orc".to_string(),
                meshes: vec![],
                mesh_transforms: vec![],
                scale: 1.0,
                color_tint: None,
            },
            CreatureDefinition {
                id: 51,
                name: "Villager".to_string(),
                meshes: vec![],
                mesh_transforms: vec![],
                scale: 1.0,
                color_tint: None,
            },
        ];
        let (monsters, npcs, templates, variants, custom) = state.count_by_category(&creatures);
        assert_eq!(monsters, 2);
        assert_eq!(npcs, 1);
        assert_eq!(templates, 0);
        assert_eq!(variants, 0);
        assert_eq!(custom, 0);
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
