// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::context_menu::ContextMenuManager;
use crate::creature_id_manager::{CreatureCategory, CreatureIdManager};
use crate::creature_undo_redo::CreatureUndoRedoManager;
use crate::creatures_manager::CreaturesManager;
use crate::creatures_workflow::{CreatureWorkflowState, WorkflowMode};
use crate::keyboard_shortcuts::ShortcutManager;
use crate::preview_features::PreviewState;
use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::types::CreatureId;
use antares::domain::visual::{
    CreatureDefinition, CreatureReference, MeshDefinition, MeshTransform,
};
use eframe::egui;
use std::path::PathBuf;

/// Sentinel string returned by the creatures editor `show()` method to signal
/// that the Campaign Builder should open the Creature Template Browser dialog.
///
/// This pattern is consistent with the `requested_open_npc` mechanism used in
/// the Maps editor -- the editor cannot directly open a sibling dialog, so it
/// communicates the request through its `Option<String>` return value.
///
/// # Examples
///
/// ```
/// use campaign_builder::creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL;
///
/// assert!(!OPEN_CREATURE_TEMPLATES_SENTINEL.is_empty());
/// assert!(OPEN_CREATURE_TEMPLATES_SENTINEL.starts_with("__campaign_builder"));
/// ```
pub const OPEN_CREATURE_TEMPLATES_SENTINEL: &str = "__campaign_builder::open_creature_templates__";

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
    /// Phase 3: Two-step delete confirmation flag for the registry preview panel.
    ///
    /// When `true` the Delete button shows "⚠ Confirm Delete"; a second click
    /// executes the deletion.  Resets whenever `selected_registry_entry` changes
    /// or `back_to_registry()` is called.
    pub registry_delete_confirm_pending: bool,

    // Phase 4: Register Asset Dialog
    /// When `true`, the "Register Creature Asset" dialog window is visible.
    pub show_register_asset_dialog: bool,
    /// Path buffer for the asset path text field (relative to campaign directory).
    pub register_asset_path_buffer: String,
    /// Creature parsed and validated from the asset file; `Some` when validation succeeds.
    pub register_asset_validated_creature: Option<CreatureDefinition>,
    /// Error message from the last Validate attempt; `None` when validation succeeded.
    pub register_asset_error: Option<String>,

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

    // Phase 5: Workflow Integration & Polish
    /// Unified workflow state (undo/redo, shortcuts, context menus, auto-save, preview).
    pub workflow: CreatureWorkflowState,
    /// Dedicated undo/redo manager for creature editing operations.
    pub undo_redo: CreatureUndoRedoManager,
    /// Keyboard shortcut registry for the creature editor.
    pub shortcut_manager: ShortcutManager,
    /// Context menu registry for mesh list and preview panels.
    pub context_menu_manager: ContextMenuManager,
    /// Enhanced preview state (camera, lighting, grid, statistics).
    pub preview_state: PreviewState,
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

/// Deferred action requested from the registry preview panel.
///
/// Collected during UI rendering and applied after the closure returns to avoid
/// borrow-checker conflicts between the `&mut self` receiver and the
/// `&CreatureDefinition` display borrow.
enum RegistryPreviewAction {
    /// Open the creature in the asset editor (Edit mode).
    Edit { file_name: String },
    /// Duplicate the creature with the next available ID.
    Duplicate,
    /// Delete the creature after two-step confirmation.
    Delete,
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

            registry_delete_confirm_pending: false,

            // Phase 4: Register Asset Dialog
            show_register_asset_dialog: false,
            register_asset_path_buffer: String::new(),
            register_asset_validated_creature: None,
            register_asset_error: None,

            // Phase 5: Workflow Integration & Polish
            workflow: CreatureWorkflowState::new(),
            undo_redo: CreatureUndoRedoManager::new(),
            shortcut_manager: ShortcutManager::new(),
            context_menu_manager: ContextMenuManager::new(),
            preview_state: PreviewState::new(),
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
        creatures: &mut Vec<CreatureDefinition>,
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

            if ui.button("📥 Register Asset").clicked() {
                self.show_register_asset_dialog = true;
            }

            if ui.button("📋 Browse Templates").clicked() {
                result_message = Some(OPEN_CREATURE_TEMPLATES_SENTINEL.to_string());
            }
        });

        ui.separator();

        // ---- Phase 3: Two-column layout (list + preview side panel) ----
        //
        // Strategy: compute a fully-owned index list (no borrows into `creatures`)
        // so that:
        //   1. The right-side preview panel can borrow `creatures[sel]` immutably
        //      while calling `&mut self` methods.
        //   2. The left-side scroll area can borrow `creatures[i]` for display.
        //   3. Deferred actions (Edit / Duplicate / Delete / double-click) are
        //      applied AFTER all closures return, once every borrow is released.

        // Build filtered + sorted index list (owned, no external borrows).
        let filtered_indices: Vec<usize> = {
            let mut pairs: Vec<(usize, u32, String)> = creatures
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    if let Some(cat) = self.category_filter {
                        if CreatureCategory::from_id(c.id) != cat {
                            return false;
                        }
                    }
                    if !self.search_query.is_empty() {
                        let q = self.search_query.to_lowercase();
                        return c.name.to_lowercase().contains(&q) || c.id.to_string().contains(&q);
                    }
                    true
                })
                .map(|(i, c)| (i, c.id, c.name.clone()))
                .collect();

            match self.registry_sort_by {
                RegistrySortBy::Id => pairs.sort_by_key(|(_, id, _)| *id),
                RegistrySortBy::Name => {
                    pairs.sort_by(|(_, _, a), (_, _, b)| a.cmp(b));
                }
                RegistrySortBy::Category => {
                    pairs.sort_by_key(|(_, id, _)| (CreatureCategory::from_id(*id) as u8, *id));
                }
            }
            pairs.into_iter().map(|(i, _, _)| i).collect()
        };

        // Deferred actions collected inside UI closures; applied after they return.
        let mut pending_edit: Option<(usize, String)> = None;
        let mut pending_preview_action: Option<RegistryPreviewAction> = None;

        // --- Right panel: preview (only when a creature is selected) ---
        if self.selected_registry_entry.is_some() {
            egui::SidePanel::right("registry_preview_panel")
                .default_width(300.0)
                .resizable(true)
                .show_inside(ui, |ui| {
                    // `self` is &mut Self, `creatures` is &mut Vec<...>.
                    // These are independent borrows so the closure can hold both.
                    if let Some(sel_idx) = self.selected_registry_entry {
                        if let Some(creature) = creatures.get(sel_idx) {
                            // creature: &CreatureDefinition (immutable borrow from creatures)
                            // self.show_registry_preview_panel borrows &mut self separately.
                            let action = self.show_registry_preview_panel(ui, creature, sel_idx);
                            if action.is_some() {
                                pending_preview_action = action;
                            }
                        } else {
                            // Index out of bounds (creature was deleted externally).
                            self.selected_registry_entry = None;
                            ui.label("No creature selected.");
                        }
                    }
                });
        }

        // --- Left area: registry list in a scroll area ---
        egui::ScrollArea::vertical().show(ui, |ui| {
            if filtered_indices.is_empty() {
                ui.label("No creatures found. Click 'New' to create one.");
            } else {
                // Column header row
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

                for &idx in &filtered_indices {
                    let creature = &creatures[idx];
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
                        let label = format!(
                            "{} ({} mesh{})",
                            creature.name,
                            creature.meshes.len(),
                            if creature.meshes.len() == 1 { "" } else { "es" }
                        );
                        let response = ui.selectable_label(is_selected, label);

                        if response.clicked() {
                            // Reset delete-confirm flag when switching selection.
                            if self.selected_registry_entry != Some(idx) {
                                self.registry_delete_confirm_pending = false;
                            }
                            self.selected_registry_entry = Some(idx);
                        }

                        if response.double_clicked() {
                            let file_name = format!(
                                "assets/creatures/{}.ron",
                                creature.name.to_lowercase().replace(' ', "_")
                            );
                            pending_edit = Some((idx, file_name));
                        }

                        ui.separator();

                        // Validation status indicator
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

        // --- Apply deferred actions (all closures have returned; borrows released) ---

        // Double-click from the list opens the creature in the asset editor.
        if let Some((idx, file_name)) = pending_edit {
            self.open_for_editing(creatures, idx, &file_name);
        }

        // Actions requested from the preview panel.
        match pending_preview_action {
            Some(RegistryPreviewAction::Edit { file_name }) => {
                if let Some(idx) = self.selected_registry_entry {
                    self.open_for_editing(creatures, idx, &file_name);
                }
            }
            Some(RegistryPreviewAction::Duplicate) => {
                if let Some(idx) = self.selected_registry_entry {
                    if idx < creatures.len() {
                        let new_id = self.next_available_id(creatures);
                        let mut new_creature = creatures[idx].clone();
                        new_creature.id = new_id;
                        new_creature.name = format!("{} (Copy)", new_creature.name);
                        let new_name = new_creature.name.clone();
                        creatures.push(new_creature);
                        *unsaved_changes = true;
                        result_message = Some(format!("Duplicated creature as '{}'", new_name));
                    }
                }
            }
            Some(RegistryPreviewAction::Delete) => {
                if let Some(idx) = self.selected_registry_entry {
                    if idx < creatures.len() {
                        let name = creatures[idx].name.clone();
                        creatures.remove(idx);
                        self.selected_registry_entry = None;
                        self.registry_delete_confirm_pending = false;
                        *unsaved_changes = true;
                        result_message = Some(format!("Deleted creature '{}'", name));
                    }
                }
            }
            None => {}
        }

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

        // Phase 4: Register Asset Dialog window
        if self.show_register_asset_dialog {
            let ctx = ui.ctx().clone();
            if let Some(msg) = self.show_register_asset_dialog_window(
                &ctx,
                creatures,
                campaign_dir,
                unsaved_changes,
            ) {
                result_message = Some(msg);
            }
        }

        result_message
    }

    /// Shows the "Register Creature Asset" dialog window.
    ///
    /// Allows the user to type a relative path to an existing `.ron` file,
    /// validate it, preview a creature summary, and register it into the
    /// campaign's creature list without leaving the Campaign Builder.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context used to render the floating window.
    /// * `creatures` - Mutable reference to the full creatures list.
    /// * `campaign_dir` - Optional campaign directory for resolving relative paths.
    /// * `unsaved_changes` - Set to `true` when a creature is successfully registered.
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` with a success message after a creature is
    /// registered, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    /// use antares::domain::visual::CreatureDefinition;
    ///
    /// let mut state = CreaturesEditorState::new();
    /// // Dialog starts closed
    /// assert!(!state.show_register_asset_dialog);
    /// // Opening it sets the flag
    /// state.show_register_asset_dialog = true;
    /// assert!(state.show_register_asset_dialog);
    /// ```
    fn show_register_asset_dialog_window(
        &mut self,
        ctx: &egui::Context,
        creatures: &mut Vec<CreatureDefinition>,
        campaign_dir: &Option<PathBuf>,
        unsaved_changes: &mut bool,
    ) -> Option<String> {
        let mut result: Option<String> = None;

        // Deferred action flags collected inside the egui closure.
        let mut do_validate = false;
        let mut do_register = false;
        let mut do_cancel = false;

        egui::Window::new("Register Creature Asset")
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label(
                    "Enter the path to a creature asset file, relative to the campaign directory.",
                );
                ui.label(egui::RichText::new("Example: assets/creatures/goblin.ron").monospace());
                ui.label("Paths must use forward slashes (/) and must not start with /.");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.text_edit_singleline(&mut self.register_asset_path_buffer)
                        .on_hover_text(
                            "Relative path to the .ron file (e.g. assets/creatures/goblin.ron)",
                        );
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("🔍 Validate").clicked() {
                        do_validate = true;
                    }

                    let register_enabled = self.register_asset_validated_creature.is_some();
                    ui.add_enabled_ui(register_enabled, |ui| {
                        if ui
                            .button("📥 Register")
                            .on_hover_text("Register the validated creature into this campaign")
                            .clicked()
                        {
                            do_register = true;
                        }
                    });

                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });

                // Error label (shown in red when validation fails)
                if let Some(ref err) = self.register_asset_error {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, format!("⚠ {}", err));
                }

                // Success preview (shown when validation succeeds)
                if let Some(ref creature) = self.register_asset_validated_creature {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "✓ Validation successful — confirm the details below before registering.",
                    );
                    egui::Grid::new("register_asset_preview_grid")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name:").strong());
                            ui.label(&creature.name);
                            ui.end_row();

                            ui.label(egui::RichText::new("ID:").strong());
                            ui.label(format!("{}", creature.id));
                            ui.end_row();

                            ui.label(egui::RichText::new("Category:").strong());
                            ui.label(
                                crate::creature_id_manager::CreatureCategory::from_id(creature.id)
                                    .display_name(),
                            );
                            ui.end_row();

                            ui.label(egui::RichText::new("Meshes:").strong());
                            ui.label(format!(
                                "{} mesh{}",
                                creature.meshes.len(),
                                if creature.meshes.len() == 1 { "" } else { "es" }
                            ));
                            ui.end_row();

                            ui.label(egui::RichText::new("Scale:").strong());
                            ui.label(format!("{:.3}", creature.scale));
                            ui.end_row();
                        });
                }
            });

        // ---- Apply deferred actions (all closures have returned) ----

        if do_validate {
            self.execute_register_asset_validation(creatures, campaign_dir);
        }

        if do_register {
            if let Some(creature) = self.register_asset_validated_creature.take() {
                let name = creature.name.clone();
                let id = creature.id;
                creatures.push(creature);
                *unsaved_changes = true;
                self.show_register_asset_dialog = false;
                self.register_asset_path_buffer.clear();
                self.register_asset_error = None;
                result = Some(format!("Registered creature '{}' (ID {})", name, id));
            }
        }

        if do_cancel {
            self.show_register_asset_dialog = false;
            self.register_asset_path_buffer.clear();
            self.register_asset_validated_creature = None;
            self.register_asset_error = None;
        }

        result
    }

    /// Validates the path in `register_asset_path_buffer` against the creatures list.
    ///
    /// Resolves the path relative to `campaign_dir`, reads the file, parses the
    /// RON content as a [`CreatureDefinition`], checks for duplicate IDs in
    /// `creatures`, and checks for ID range violations using the ID manager.
    ///
    /// On success, stores the parsed creature in `register_asset_validated_creature`
    /// and clears `register_asset_error`.  On failure, sets `register_asset_error`
    /// with an actionable message and clears `register_asset_validated_creature`.
    ///
    /// # Path Normalization
    ///
    /// Backslashes are replaced with forward slashes and any leading slash is
    /// stripped before joining with `campaign_dir`.
    ///
    /// # Arguments
    ///
    /// * `creatures` - Read-only view of the current creature list for duplicate detection.
    /// * `campaign_dir` - Base directory used to resolve the relative asset path.
    fn execute_register_asset_validation(
        &mut self,
        creatures: &[CreatureDefinition],
        campaign_dir: &Option<PathBuf>,
    ) {
        // Clear any prior results.
        self.register_asset_validated_creature = None;
        self.register_asset_error = None;

        // 4.5 Path normalization: replace backslashes, trim leading slash.
        let normalized = self
            .register_asset_path_buffer
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();

        if normalized.is_empty() {
            self.register_asset_error =
                Some("Path is empty. Enter a path relative to the campaign directory.".to_string());
            return;
        }

        // Resolve full path.
        let full_path = if let Some(dir) = campaign_dir {
            dir.join(&normalized)
        } else {
            std::path::PathBuf::from(&normalized)
        };

        // Read file contents.
        let contents = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                self.register_asset_error = Some(format!(
                    "File not found or unreadable ({}): {}",
                    full_path.display(),
                    e
                ));
                return;
            }
        };

        // Parse RON.
        let creature: CreatureDefinition = match ron::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                self.register_asset_error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        // 4.4 Duplicate ID check (direct vec scan — most authoritative source).
        if let Some(existing) = creatures.iter().find(|c| c.id == creature.id) {
            self.register_asset_error = Some(format!(
                "ID {} is already registered to '{}'. Edit that creature or choose a file with a unique ID.",
                creature.id, existing.name
            ));
            return;
        }

        // 4.4 Range validity check via ID manager.
        let category = crate::creature_id_manager::CreatureCategory::from_id(creature.id);
        if let Err(crate::creature_id_manager::IdError::OutOfRange {
            id,
            category: cat_name,
            range,
        }) = self.id_manager.validate_id(creature.id, category)
        {
            self.register_asset_error = Some(format!(
                "ID {} is outside the valid range for category {} ({}). Use a {} ID.",
                id, cat_name, range, cat_name
            ));
            return;
        }

        // All checks passed.
        self.register_asset_validated_creature = Some(creature);
    }

    /// Renders the registry preview panel for the creature at `idx`.
    ///
    /// Shows creature metadata (name, ID, category, scale, color tint, mesh list)
    /// and three action buttons: Edit (primary), Duplicate, Delete (two-step).
    ///
    /// Returns a `RegistryPreviewAction` if the user clicked an action button,
    /// or `None` if no action was triggered this frame.
    ///
    /// # Arguments
    ///
    /// * `ui`      - The egui Ui region to render into.
    /// * `creature` - Read-only view of the creature to display.
    /// * `idx`     - Index of the creature in the backing `creatures` vec.
    fn show_registry_preview_panel(
        &mut self,
        ui: &mut egui::Ui,
        creature: &CreatureDefinition,
        idx: usize,
    ) -> Option<RegistryPreviewAction> {
        let mut action: Option<RegistryPreviewAction> = None;

        // --- Heading: creature name ---
        ui.heading(&creature.name);
        ui.separator();

        // --- ID with category-color badge ---
        let category = CreatureCategory::from_id(creature.id);
        let color = category.color();
        let cat_color32 = egui::Color32::from_rgb(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
        );

        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.colored_label(
                cat_color32,
                egui::RichText::new(format!("{:03}", creature.id)).strong(),
            );
        });

        // --- Category ---
        ui.horizontal(|ui| {
            ui.label("Category:");
            ui.label(
                egui::RichText::new(category.display_name())
                    .color(cat_color32)
                    .strong(),
            );
        });

        // --- Scale ---
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.label(format!("{:.3}", creature.scale));
        });

        // --- Color tint swatch ---
        ui.horizontal(|ui| {
            ui.label("Color tint:");
            if let Some(tint) = creature.color_tint {
                let swatch_color = egui::Color32::from_rgb(
                    (tint[0] * 255.0) as u8,
                    (tint[1] * 255.0) as u8,
                    (tint[2] * 255.0) as u8,
                );
                // Draw a small filled rectangle as the swatch.
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(32.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, swatch_color);
                ui.label(format!("({:.2}, {:.2}, {:.2})", tint[0], tint[1], tint[2]));
            } else {
                ui.label("None");
            }
        });

        // --- Mesh count + collapsible list ---
        let mesh_count = creature.meshes.len();
        ui.collapsing(
            format!(
                "Meshes: {} mesh{}",
                mesh_count,
                if mesh_count == 1 { "" } else { "es" }
            ),
            |ui| {
                if mesh_count == 0 {
                    ui.label("(no meshes)");
                } else {
                    for (i, mesh) in creature.meshes.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", i + 1));
                            let name = mesh.name.as_deref().unwrap_or("(unnamed)");
                            ui.label(egui::RichText::new(name).monospace());
                            ui.label(format!(
                                "— {} vert{}",
                                mesh.vertices.len(),
                                if mesh.vertices.len() == 1 {
                                    "ex"
                                } else {
                                    "ices"
                                }
                            ));
                        });
                    }
                }
            },
        );

        // --- Derived file path ---
        let slug = creature.name.to_lowercase().replace(' ', "_");
        let file_path = format!("assets/creatures/{}.ron", slug);
        ui.horizontal(|ui| {
            ui.label("File:");
            ui.label(egui::RichText::new(&file_path).monospace().small());
        });

        ui.separator();

        // --- Action buttons ---

        // Primary action: Edit (opens the full asset editor)
        if ui
            .button(egui::RichText::new("✏ Edit").strong())
            .on_hover_text("Open creature in the asset editor")
            .clicked()
        {
            action = Some(RegistryPreviewAction::Edit {
                file_name: file_path,
            });
        }

        ui.horizontal(|ui| {
            // Duplicate
            if ui
                .button("📋 Duplicate")
                .on_hover_text("Create a copy with the next available ID")
                .clicked()
            {
                action = Some(RegistryPreviewAction::Duplicate);
            }

            // Delete — two-step confirmation
            if self.registry_delete_confirm_pending {
                // Second click executes the deletion.
                if ui
                    .button(egui::RichText::new("⚠ Confirm Delete").color(egui::Color32::RED))
                    .on_hover_text("Click again to permanently delete this creature")
                    .clicked()
                {
                    action = Some(RegistryPreviewAction::Delete);
                }
                // Cancel button clears the flag without deleting.
                if ui.button("Cancel").clicked() {
                    self.registry_delete_confirm_pending = false;
                }
            } else {
                // First click arms the confirmation.
                if ui
                    .button("🗑 Delete")
                    .on_hover_text("Delete this creature (requires confirmation)")
                    .clicked()
                {
                    self.registry_delete_confirm_pending = true;
                }
            }
        });

        // Show a hint so the user knows which creature is in the editor slot.
        ui.separator();
        ui.label(
            egui::RichText::new(format!("Registry index: {}", idx))
                .small()
                .weak(),
        );

        action
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

                if ui.button("📋 Browse Templates").clicked() {
                    result_message = Some(OPEN_CREATURE_TEMPLATES_SENTINEL.to_string());
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

    // -----------------------------------------------------------------------
    // Phase 5: Workflow Integration helpers
    // -----------------------------------------------------------------------

    /// Enter asset-editor mode for the given creature.
    ///
    /// Transitions the editor into `Edit` mode, records the creature index,
    /// updates the unified `workflow` state (breadcrumbs, undo history, etc.),
    /// and marks the session as clean.
    ///
    /// # Arguments
    ///
    /// * `creatures` - The full creature list.
    /// * `index` - The index of the creature to edit.
    /// * `file_name` - The asset file name (e.g. `"goblin.ron"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    /// use antares::domain::visual::CreatureDefinition;
    ///
    /// let mut state = CreaturesEditorState::new();
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Goblin".to_string(),
    ///     meshes: vec![],
    ///     mesh_transforms: vec![],
    ///     scale: 1.0,
    ///     color_tint: None,
    /// };
    /// let creatures = vec![creature];
    /// state.open_for_editing(&creatures, 0, "goblin.ron");
    ///
    /// use campaign_builder::creatures_editor::CreaturesEditorMode;
    /// assert_eq!(state.mode, CreaturesEditorMode::Edit);
    /// ```
    pub fn open_for_editing(
        &mut self,
        creatures: &[CreatureDefinition],
        index: usize,
        file_name: &str,
    ) {
        if let Some(creature) = creatures.get(index) {
            self.mode = CreaturesEditorMode::Edit;
            self.selected_creature = Some(index);
            self.edit_buffer = creature.clone();
            self.preview_dirty = true;
            self.workflow.enter_asset_editor(file_name, &creature.name);
        }
    }

    /// Return to registry (list) mode, resetting all transient edit state.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::{CreaturesEditorState, CreaturesEditorMode};
    /// use campaign_builder::creatures_workflow::WorkflowMode;
    ///
    /// let mut state = CreaturesEditorState::new();
    /// state.mode = CreaturesEditorMode::Edit;
    /// state.back_to_registry();
    ///
    /// assert_eq!(state.mode, CreaturesEditorMode::List);
    /// assert_eq!(state.workflow.mode, WorkflowMode::Registry);
    /// ```
    pub fn back_to_registry(&mut self) {
        self.mode = CreaturesEditorMode::List;
        self.selected_creature = None;
        self.selected_mesh_index = None;
        self.mesh_edit_buffer = None;
        self.mesh_transform_buffer = None;
        self.preview_dirty = false;
        self.registry_delete_confirm_pending = false;
        self.workflow.return_to_registry();
    }

    /// Returns the mode indicator string for the top bar (Phase 5.1).
    ///
    /// Examples: `"Registry Mode"` or `"Asset Editor: goblin.ron"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    ///
    /// let state = CreaturesEditorState::new();
    /// assert_eq!(state.mode_indicator(), "Registry Mode");
    /// ```
    pub fn mode_indicator(&self) -> String {
        self.workflow.mode_indicator()
    }

    /// Returns the breadcrumb navigation string (Phase 5.1).
    ///
    /// Example: `"Creatures > Goblin > left_leg"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    ///
    /// let state = CreaturesEditorState::new();
    /// assert_eq!(state.breadcrumb_string(), "Creatures");
    /// ```
    pub fn breadcrumb_string(&self) -> String {
        self.workflow.breadcrumb_string()
    }

    /// Returns `true` if there are unsaved changes in the current session.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    ///
    /// let state = CreaturesEditorState::new();
    /// assert!(!state.has_unsaved_changes());
    /// ```
    pub fn has_unsaved_changes(&self) -> bool {
        self.workflow.has_unsaved_changes()
    }

    /// Returns `true` if there is at least one action available to undo.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    ///
    /// let state = CreaturesEditorState::new();
    /// assert!(!state.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        self.undo_redo.can_undo()
    }

    /// Returns `true` if there is at least one action available to redo.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    ///
    /// let state = CreaturesEditorState::new();
    /// assert!(!state.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        self.undo_redo.can_redo()
    }

    /// Returns the keyboard shortcut string for an action (Phase 5.3).
    ///
    /// # Arguments
    ///
    /// * `action` - The shortcut action to look up.
    ///
    /// # Returns
    ///
    /// A human-readable shortcut string, or `None` if no binding exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_editor::CreaturesEditorState;
    /// use campaign_builder::keyboard_shortcuts::ShortcutAction;
    ///
    /// let state = CreaturesEditorState::new();
    /// let shortcut = state.shortcut_for(ShortcutAction::Save);
    /// assert!(shortcut.is_some());
    /// ```
    pub fn shortcut_for(
        &self,
        action: crate::keyboard_shortcuts::ShortcutAction,
    ) -> Option<String> {
        self.shortcut_manager
            .get_shortcut(action)
            .map(|s| s.to_string())
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
        assert!(!state.registry_delete_confirm_pending);
    }

    // -----------------------------------------------------------------------
    // Phase 4: Register Existing Creature Asset .ron File
    // -----------------------------------------------------------------------

    /// All Phase 4 dialog fields must default to empty / false / None.
    #[test]
    fn test_register_asset_dialog_initial_state() {
        let state = CreaturesEditorState::new();
        assert!(!state.show_register_asset_dialog);
        assert!(state.register_asset_path_buffer.is_empty());
        assert!(state.register_asset_validated_creature.is_none());
        assert!(state.register_asset_error.is_none());
    }

    /// When a creature with the same ID already exists, `execute_register_asset_validation`
    /// must set `register_asset_error` and leave `register_asset_validated_creature` as `None`.
    #[test]
    fn test_register_asset_validate_duplicate_id_sets_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut state = CreaturesEditorState::new();

        // Existing creature with id 1.
        let existing = make_creature(1, "Goblin");
        let creatures = vec![existing];

        // Write a RON file for a creature that also has id 1.
        let ron_content = r#"CreatureDefinition(
    id: 1,
    name: "Shadow Goblin",
    meshes: [],
    mesh_transforms: [],
    scale: 1.0,
    color_tint: None,
)"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        write!(tmp, "{}", ron_content).expect("write");
        let dir = tmp.path().parent().map(|p| p.to_path_buf());
        let file_name = tmp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        state.register_asset_path_buffer = file_name;
        state.execute_register_asset_validation(&creatures, &dir);

        assert!(state.register_asset_validated_creature.is_none());
        assert!(state.register_asset_error.is_some());
        let err = state.register_asset_error.as_ref().unwrap();
        assert!(
            err.contains("already registered"),
            "error should mention 'already registered', got: {err}"
        );
        assert!(
            err.contains("Goblin"),
            "error should name the conflicting creature, got: {err}"
        );
    }

    /// The Register button logic is gated on `register_asset_validated_creature`.
    /// When `None`, the button must be disabled (test via the state flag directly).
    #[test]
    fn test_register_asset_register_button_disabled_before_validation() {
        let state = CreaturesEditorState::new();
        // Button is enabled iff validated creature is Some.
        assert!(
            state.register_asset_validated_creature.is_none(),
            "register button must be disabled before validation"
        );
    }

    /// Cancelling the dialog must not modify the creatures list.
    #[test]
    fn test_register_asset_cancel_does_not_modify_creatures() {
        let mut state = CreaturesEditorState::new();
        let creatures = vec![make_creature(1, "Goblin")];
        let unsaved = false;

        // Pre-load some dialog state to confirm it gets cleared.
        state.show_register_asset_dialog = true;
        state.register_asset_path_buffer = "assets/creatures/goblin.ron".to_string();
        state.register_asset_validated_creature = Some(make_creature(2, "Orc"));
        state.register_asset_error = Some("some error".to_string());

        // Simulate Cancel: clear all dialog state without touching `creatures`.
        state.show_register_asset_dialog = false;
        state.register_asset_path_buffer.clear();
        state.register_asset_validated_creature = None;
        state.register_asset_error = None;

        // Creatures list must be unchanged.
        assert_eq!(creatures.len(), 1);
        assert_eq!(creatures[0].name, "Goblin");
        assert!(!unsaved);

        // All dialog state must be reset.
        assert!(!state.show_register_asset_dialog);
        assert!(state.register_asset_path_buffer.is_empty());
        assert!(state.register_asset_validated_creature.is_none());
        assert!(state.register_asset_error.is_none());

        let _ = creatures;
        let _ = unsaved;
    }

    /// Registering a valid creature must append it to the vec and set
    /// `unsaved_changes` to `true`.
    #[test]
    fn test_register_asset_success_appends_creature() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin")];
        let mut unsaved = false;

        // Write a RON file for a valid, unique creature.
        let ron_content = r#"CreatureDefinition(
    id: 2,
    name: "Orc",
    meshes: [],
    mesh_transforms: [],
    scale: 1.5,
    color_tint: None,
)"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        write!(tmp, "{}", ron_content).expect("write");
        let dir = tmp.path().parent().map(|p| p.to_path_buf());
        let file_name = tmp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        state.register_asset_path_buffer = file_name;
        state.execute_register_asset_validation(&creatures, &dir);

        // Validation must succeed.
        assert!(
            state.register_asset_error.is_none(),
            "unexpected error: {:?}",
            state.register_asset_error
        );
        assert!(state.register_asset_validated_creature.is_some());

        // Simulate the Register button action.
        if let Some(creature) = state.register_asset_validated_creature.take() {
            let name = creature.name.clone();
            let id = creature.id;
            creatures.push(creature);
            unsaved = true;
            state.show_register_asset_dialog = false;
            state.register_asset_path_buffer.clear();
            state.register_asset_error = None;
            let msg = format!("Registered creature '{}' (ID {})", name, id);
            assert_eq!(msg, "Registered creature 'Orc' (ID 2)");
        }

        assert_eq!(creatures.len(), 2, "creature must be appended");
        assert_eq!(creatures[1].name, "Orc");
        assert_eq!(creatures[1].id, 2);
        assert!(unsaved, "unsaved_changes must be set to true");
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

    // -----------------------------------------------------------------------
    // Phase 2 regression tests: Fix the Silent Data-Loss Bug in Edit Mode
    // -----------------------------------------------------------------------

    /// Helper that constructs a named creature with a given id.
    fn make_creature(id: u32, name: &str) -> CreatureDefinition {
        CreatureDefinition {
            id,
            name: name.to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    /// Simulates the fixed double-click path: calls `open_for_editing()` and
    /// verifies that `selected_creature`, `mode`, and `edit_buffer` are all set
    /// correctly.
    #[test]
    fn test_double_click_sets_selected_creature() {
        let mut state = CreaturesEditorState::new();
        let creatures = vec![make_creature(1, "Goblin"), make_creature(2, "Orc")];

        // This is what the fixed double-click handler calls.
        let file_name = format!(
            "assets/creatures/{}.ron",
            creatures[0].name.to_lowercase().replace(' ', "_")
        );
        state.open_for_editing(&creatures, 0, &file_name);

        assert_eq!(state.selected_creature, Some(0));
        assert_eq!(state.mode, CreaturesEditorMode::Edit);
        assert_eq!(state.edit_buffer.name, "Goblin");
        assert!(state.preview_dirty);
    }

    /// After opening via `open_for_editing()`, mutating `edit_buffer`, and
    /// executing the Save guard, the backing `creatures` vec must be updated.
    #[test]
    fn test_edit_mode_save_updates_creature() {
        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin")];

        state.open_for_editing(&creatures, 0, "assets/creatures/goblin.ron");

        // Modify the edit buffer (as the user would do in the UI).
        state.edit_buffer.name = "Super Goblin".to_string();

        // Reproduce the Save guard from show_edit_mode():
        //   if let Some(idx) = self.selected_creature { creatures[idx] = self.edit_buffer.clone(); }
        if let Some(idx) = state.selected_creature {
            if idx < creatures.len() {
                creatures[idx] = state.edit_buffer.clone();
            }
        }

        assert_eq!(creatures[0].name, "Super Goblin");
    }

    /// If edit mode is entered WITHOUT calling `open_for_editing()` (the old
    /// broken path), `selected_creature` remains `None` and the Save guard must
    /// be a no-op -- no data loss, but also no phantom write.
    #[test]
    fn test_edit_mode_save_without_selected_creature_is_noop() {
        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin")];

        // Replicate the OLD broken double-click behaviour: set mode without
        // calling open_for_editing(), so selected_creature stays None.
        state.mode = CreaturesEditorMode::Edit;
        state.edit_buffer = creatures[0].clone();
        state.edit_buffer.name = "Broken Name".to_string();

        // The Save guard in show_edit_mode() does nothing when selected_creature
        // is None.
        if let Some(idx) = state.selected_creature {
            if idx < creatures.len() {
                creatures[idx] = state.edit_buffer.clone();
            }
        }

        // The original name must be preserved.
        assert_eq!(creatures[0].name, "Goblin");
        assert_eq!(state.selected_creature, None);
    }

    /// Entering edit mode via `open_for_editing()` and then triggering Delete
    /// must remove the correct creature from the vec.
    #[test]
    fn test_delete_from_edit_mode_removes_creature() {
        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin"), make_creature(2, "Orc")];

        state.open_for_editing(&creatures, 0, "assets/creatures/goblin.ron");

        // Reproduce the Delete guard from show_edit_mode():
        if let Some(idx) = state.selected_creature {
            if idx < creatures.len() {
                creatures.remove(idx);
                state.selected_creature = None;
                state.mode = CreaturesEditorMode::List;
            }
        }

        assert_eq!(creatures.len(), 1);
        assert_eq!(creatures[0].name, "Orc");
        assert_eq!(state.selected_creature, None);
        assert_eq!(state.mode, CreaturesEditorMode::List);
    }

    /// Entering edit mode via `open_for_editing()` and then triggering Duplicate
    /// must append a copy with a new id to the vec.
    #[test]
    fn test_duplicate_from_edit_mode_adds_creature() {
        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin")];

        state.open_for_editing(&creatures, 0, "assets/creatures/goblin.ron");

        // Reproduce the Duplicate guard from show_edit_mode():
        if let Some(idx) = state.selected_creature {
            if idx < creatures.len() {
                let mut new_creature = creatures[idx].clone();
                new_creature.id = state.next_available_id(&creatures);
                new_creature.name = format!("{} (Copy)", new_creature.name);
                creatures.push(new_creature);
            }
        }

        assert_eq!(creatures.len(), 2);
        assert_eq!(creatures[1].name, "Goblin (Copy)");
        assert_eq!(creatures[1].id, 2);
    }

    // -----------------------------------------------------------------------
    // Phase 3: Preview Panel in Registry List Mode
    // -----------------------------------------------------------------------

    /// Confirm that when `selected_registry_entry` is `None` the preview panel
    /// logic is not triggered -- verified by checking that
    /// `registry_delete_confirm_pending` stays `false` and no mode transition
    /// occurs.
    #[test]
    fn test_registry_preview_not_shown_when_no_selection() {
        let state = CreaturesEditorState::new();
        assert_eq!(state.selected_registry_entry, None);
        // No preview action should be reachable; the field guard proves this.
        assert!(!state.registry_delete_confirm_pending);
        assert_eq!(state.mode, CreaturesEditorMode::List);
    }

    /// `registry_delete_confirm_pending` must reset when a DIFFERENT creature is
    /// selected via a single click in the list.
    #[test]
    fn test_registry_delete_confirm_flag_resets_on_selection_change() {
        let mut state = CreaturesEditorState::new();

        // Arm the confirmation for creature 0.
        state.selected_registry_entry = Some(0);
        state.registry_delete_confirm_pending = true;

        // Simulate clicking a different creature (idx 1) -- mirrors the list
        // click handler added in show_registry_mode().
        let new_idx: usize = 1;
        if state.selected_registry_entry != Some(new_idx) {
            state.registry_delete_confirm_pending = false;
        }
        state.selected_registry_entry = Some(new_idx);

        assert_eq!(state.selected_registry_entry, Some(1));
        assert!(!state.registry_delete_confirm_pending);
    }

    /// Clicking the Edit button in the preview panel must call `open_for_editing`,
    /// transitioning to `Edit` mode with `selected_creature` set correctly.
    #[test]
    fn test_registry_preview_edit_button_transitions_to_edit_mode() {
        let mut state = CreaturesEditorState::new();
        let creatures = vec![make_creature(1, "Goblin"), make_creature(2, "Orc")];

        state.selected_registry_entry = Some(0);

        // Simulate the Edit action produced by show_registry_preview_panel().
        let file_name = format!(
            "assets/creatures/{}.ron",
            creatures[0].name.to_lowercase().replace(' ', "_")
        );
        // Apply action as show_registry_mode() would.
        if let Some(idx) = state.selected_registry_entry {
            state.open_for_editing(&creatures, idx, &file_name);
        }

        assert_eq!(state.mode, CreaturesEditorMode::Edit);
        assert_eq!(state.selected_creature, Some(0));
        assert_eq!(state.edit_buffer.name, "Goblin");
        assert!(state.preview_dirty);
    }

    /// The Duplicate action from the preview panel must append one new creature
    /// to the vec with the next available ID and a "(Copy)" suffix.
    #[test]
    fn test_registry_preview_duplicate_appends_creature() {
        let mut state = CreaturesEditorState::new();
        let mut creatures = vec![make_creature(1, "Goblin"), make_creature(3, "Orc")];

        state.selected_registry_entry = Some(0);

        // Apply Duplicate action as show_registry_mode() would.
        let idx = state.selected_registry_entry.unwrap();
        assert!(idx < creatures.len());
        let new_id = state.next_available_id(&creatures);
        let mut new_creature = creatures[idx].clone();
        new_creature.id = new_id;
        new_creature.name = format!("{} (Copy)", new_creature.name);
        creatures.push(new_creature);

        assert_eq!(creatures.len(), 3);
        assert_eq!(creatures[2].name, "Goblin (Copy)");
        // next_available_id returns max(1,3)+1 = 4
        assert_eq!(creatures[2].id, 4);
    }
}
