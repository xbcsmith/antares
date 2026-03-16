// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Furniture definitions editor for the Campaign Builder SDK.
//!
//! Mirrors the [`items_editor`](crate::items_editor) pattern:
//! List / Add / Edit modes, two-column layout with search and category/type
//! filters, toolbar actions (New, Save, Load, Import, Export, Reload,
//! Duplicate, Delete), and full RON import/export.
//!
//! # Usage
//!
//! ```no_run
//! use campaign_builder::furniture_editor::FurnitureEditorState;
//!
//! let mut state = FurnitureEditorState::new();
//! assert_eq!(state.mode, campaign_builder::furniture_editor::FurnitureEditorMode::List);
//! ```

use crate::ui_helpers::{
    handle_reload, show_standard_list_item, EditorToolbar, ItemAction, MetadataBadge,
    StandardListItemConfig, ToolbarAction, TwoColumnLayout,
};
use antares::domain::types::FurnitureId;
use antares::domain::world::furniture::FurnitureDefinition;
use antares::domain::world::{FurnitureCategory, FurnitureFlags, FurnitureMaterial, FurnitureType};
use eframe::egui;
use std::path::PathBuf;

// =============================================================================
// Editor Mode
// =============================================================================

/// Operating mode for the furniture editor panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnitureEditorMode {
    /// Shows the list / detail split view
    List,
    /// Shows the add-new form (write to a fresh buffer then append)
    Add,
    /// Shows the edit form for an existing definition
    Edit,
}

// =============================================================================
// Filters
// =============================================================================

/// Category filter – mirrors [`FurnitureCategory`] but is `Option`-wrapped
/// in the editor state so `None` means "show all".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnitureCategoryFilter {
    /// Seating category filter
    Seating,
    /// Storage category filter
    Storage,
    /// Decoration category filter
    Decoration,
    /// Lighting category filter
    Lighting,
    /// Utility category filter
    Utility,
    /// Passage category filter
    Passage,
}

impl FurnitureCategoryFilter {
    /// Returns `true` if `def` belongs to the filtered category.
    pub fn matches(self, def: &FurnitureDefinition) -> bool {
        matches!(
            (self, def.category),
            (FurnitureCategoryFilter::Seating, FurnitureCategory::Seating)
                | (FurnitureCategoryFilter::Storage, FurnitureCategory::Storage)
                | (
                    FurnitureCategoryFilter::Decoration,
                    FurnitureCategory::Decoration
                )
                | (
                    FurnitureCategoryFilter::Lighting,
                    FurnitureCategory::Lighting
                )
                | (FurnitureCategoryFilter::Utility, FurnitureCategory::Utility)
                | (FurnitureCategoryFilter::Passage, FurnitureCategory::Passage)
        )
    }

    /// Human-readable label used in the ComboBox
    pub fn as_str(self) -> &'static str {
        match self {
            FurnitureCategoryFilter::Seating => "Seating",
            FurnitureCategoryFilter::Storage => "Storage",
            FurnitureCategoryFilter::Decoration => "Decoration",
            FurnitureCategoryFilter::Lighting => "Lighting",
            FurnitureCategoryFilter::Utility => "Utility",
            FurnitureCategoryFilter::Passage => "Passage",
        }
    }

    /// All filter variants in display order
    pub fn all() -> [FurnitureCategoryFilter; 6] {
        [
            FurnitureCategoryFilter::Seating,
            FurnitureCategoryFilter::Storage,
            FurnitureCategoryFilter::Decoration,
            FurnitureCategoryFilter::Lighting,
            FurnitureCategoryFilter::Utility,
            FurnitureCategoryFilter::Passage,
        ]
    }
}

// =============================================================================
// Editor State
// =============================================================================

/// Signal emitted by the furniture editor when it wants the host app to switch
/// to another tab or tool.
///
/// This mirrors the cross-tab navigation pattern already used by other editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnitureEditorSignal {
    /// Ask the host app to open the OBJ importer and prepare it for furniture mesh work.
    OpenInObjImporter,
}

/// All mutable UI state owned by the furniture definitions editor.
///
/// Holds the current mode, search / filter inputs, the in-flight edit buffer,
/// and the import-export dialog state.
pub struct FurnitureEditorState {
    /// Current editor mode (List / Add / Edit)
    pub mode: FurnitureEditorMode,
    /// Live search query matched against definition names
    pub search_query: String,
    /// Index into the canonical `Vec<FurnitureDefinition>` that is selected
    pub selected_furniture: Option<usize>,
    /// Working copy of the definition being added or edited
    pub edit_buffer: FurnitureDefinition,
    /// Whether the import / export dialog is currently visible
    pub show_import_dialog: bool,
    /// Textarea contents for the RON import / export dialog
    pub import_export_buffer: String,
    /// Optional category filter; `None` = show all categories
    pub filter_category: Option<FurnitureCategoryFilter>,
    /// Optional base-type filter; `None` = show all types
    pub filter_base_type: Option<FurnitureType>,
    /// When `Some`, requests host-app navigation to the OBJ importer.
    pub requested_signal: Option<FurnitureEditorSignal>,
}

impl Default for FurnitureEditorState {
    fn default() -> Self {
        Self {
            mode: FurnitureEditorMode::List,
            search_query: String::new(),
            selected_furniture: None,
            edit_buffer: Self::default_furniture(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            filter_category: None,
            filter_base_type: None,
            requested_signal: None,
        }
    }
}

impl FurnitureEditorState {
    /// Creates a new furniture editor state with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::furniture_editor::FurnitureEditorState;
    ///
    /// let state = FurnitureEditorState::new();
    /// assert_eq!(state.mode, campaign_builder::furniture_editor::FurnitureEditorMode::List);
    /// assert!(state.selected_furniture.is_none());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a sensible default [`FurnitureDefinition`] for the "New" toolbar
    /// button.  The id is set to `0` and must be replaced with the next
    /// available campaign ID before inserting.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::furniture_editor::FurnitureEditorState;
    ///
    /// let def = FurnitureEditorState::default_furniture();
    /// assert_eq!(def.name, "New Furniture");
    /// assert_eq!(def.scale, 1.0);
    /// ```
    pub fn default_furniture() -> FurnitureDefinition {
        FurnitureDefinition {
            id: 0,
            name: "New Furniture".to_string(),
            category: FurnitureCategory::Seating,
            base_type: FurnitureType::Throne,
            material: FurnitureMaterial::Wood,
            scale: 1.0,
            color_tint: None,
            flags: FurnitureFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }
    }

    // =========================================================================
    // Top-level show
    // =========================================================================

    /// Renders the full furniture editor panel.
    ///
    /// Must be called once per frame from the central panel match arm for the
    /// `EditorTab::Furniture` variant.
    ///
    /// # Arguments
    ///
    /// * `ui` – the current egui UI context
    /// * `defs` – mutable reference to the campaign's furniture definition list
    /// * `campaign_dir` – optional path to the campaign root directory
    /// * `furniture_file` – relative path of the furniture RON file (e.g.
    ///   `"data/furniture.ron"`)
    /// * `unsaved_changes` – set to `true` whenever the list is mutated
    /// * `status_message` – updated with human-readable feedback
    /// * `file_load_merge_mode` – when `true` a Load operation merges by ID
    ///   rather than replacing the full list
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<FurnitureDefinition>,
        campaign_dir: Option<&PathBuf>,
        furniture_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
        available_mesh_ids: &[u32],
    ) {
        ui.heading("🪑 Furniture Editor");
        ui.add_space(5.0);

        // ----- Toolbar -------------------------------------------------------
        let toolbar_action = EditorToolbar::new("Furniture")
            .with_search(&mut self.search_query)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(defs.len())
            .with_id_salt("furniture_toolbar")
            .show(ui);

        match toolbar_action {
            ToolbarAction::New => {
                self.mode = FurnitureEditorMode::Add;
                self.edit_buffer = Self::default_furniture();
                let next_id = next_available_id(defs);
                self.edit_buffer.id = next_id;
                *unsaved_changes = true;
                ui.ctx().request_repaint();
            }
            ToolbarAction::Save => {
                self.save_furniture(
                    defs,
                    campaign_dir,
                    furniture_file,
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
                        ron::from_str::<Vec<FurnitureDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });
                    match load_result {
                        Ok(loaded) => {
                            if *file_load_merge_mode {
                                for def in loaded {
                                    if let Some(existing) = defs.iter_mut().find(|d| d.id == def.id)
                                    {
                                        *existing = def;
                                    } else {
                                        defs.push(def);
                                    }
                                }
                            } else {
                                *defs = loaded;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded furniture from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load furniture: {}", e);
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
                    .set_file_name("furniture.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(defs, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message =
                                    format!("Exported furniture to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to export furniture: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize furniture: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                handle_reload(defs, campaign_dir, furniture_file, status_message);
            }
            ToolbarAction::None => {}
        }

        // ----- Filter row (SDK Rule 12: use horizontal_wrapped) --------------
        ui.horizontal_wrapped(|ui| {
            ui.label("Category:");
            // SDK Rule 3: ComboBox must use from_id_salt
            egui::ComboBox::from_id_salt("furniture_category_filter")
                .selected_text(
                    self.filter_category
                        .map(|f| f.as_str())
                        .unwrap_or("All Categories"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_category.is_none(), "All Categories")
                        .clicked()
                    {
                        self.filter_category = None;
                    }
                    for f in FurnitureCategoryFilter::all() {
                        if ui
                            .selectable_value(&mut self.filter_category, Some(f), f.as_str())
                            .clicked()
                        {}
                    }
                });

            ui.separator();

            ui.label("Base Type:");
            egui::ComboBox::from_id_salt("furniture_base_type_filter")
                .selected_text(
                    self.filter_base_type
                        .map(FurnitureType::name)
                        .unwrap_or("All Types"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_base_type.is_none(), "All Types")
                        .clicked()
                    {
                        self.filter_base_type = None;
                    }
                    for t in FurnitureType::all() {
                        if ui
                            .selectable_value(&mut self.filter_base_type, Some(*t), t.name())
                            .clicked()
                        {}
                    }
                });

            ui.separator();

            if ui.button("🔄 Clear Filters").clicked() {
                self.filter_category = None;
                self.filter_base_type = None;
            }
        });

        ui.separator();

        // ----- Main content --------------------------------------------------
        match self.mode {
            FurnitureEditorMode::List => self.show_list(
                ui,
                defs,
                unsaved_changes,
                status_message,
                campaign_dir,
                furniture_file,
            ),
            FurnitureEditorMode::Add | FurnitureEditorMode::Edit => self.show_form(
                ui,
                defs,
                unsaved_changes,
                status_message,
                campaign_dir,
                furniture_file,
            ),
        }

        // ----- Import / export dialog ----------------------------------------
        if self.show_import_dialog {
            self.show_import_dialog(
                ui.ctx(),
                defs,
                unsaved_changes,
                status_message,
                campaign_dir,
                furniture_file,
            );
        }
    }

    // =========================================================================
    // List view
    // =========================================================================

    #[allow(clippy::too_many_arguments)]
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<FurnitureDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        furniture_file: &str,
    ) {
        let search_lower = self.search_query.to_lowercase();

        // SDK Rule 10: pre-compute shared state before multi-closure calls
        let filtered: Vec<(usize, FurnitureDefinition)> = defs
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                if !search_lower.is_empty() && !d.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                if let Some(cat) = self.filter_category {
                    if !cat.matches(d) {
                        return false;
                    }
                }
                if let Some(bt) = self.filter_base_type {
                    if d.base_type != bt {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, d)| (idx, d.clone()))
            .collect();

        let mut sorted = filtered;
        sorted.sort_by_key(|(idx, _)| defs[*idx].id);

        let selected = self.selected_furniture;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        ui.separator();

        // SDK Rule 9: always use TwoColumnLayout for list / detail splits
        TwoColumnLayout::new("furniture").show_split(
            ui,
            |left_ui| {
                left_ui.heading("Definitions");
                left_ui.separator();

                for (idx, def) in &sorted {
                    // SDK Rule 1: every loop body must use push_id
                    left_ui.push_id(*idx, |left_ui| {
                        let mut badges = Vec::new();

                        let cat_badge = match def.category {
                            FurnitureCategory::Seating => MetadataBadge::new("Seating")
                                .with_color(egui::Color32::from_rgb(100, 180, 100)),
                            FurnitureCategory::Storage => MetadataBadge::new("Storage")
                                .with_color(egui::Color32::from_rgb(180, 120, 60)),
                            FurnitureCategory::Decoration => MetadataBadge::new("Decoration")
                                .with_color(egui::Color32::from_rgb(200, 100, 200)),
                            FurnitureCategory::Lighting => MetadataBadge::new("Lighting")
                                .with_color(egui::Color32::from_rgb(220, 180, 50)),
                            FurnitureCategory::Utility => MetadataBadge::new("Utility")
                                .with_color(egui::Color32::from_rgb(100, 150, 200)),
                            FurnitureCategory::Passage => MetadataBadge::new("Passage")
                                .with_color(egui::Color32::from_rgb(150, 100, 50)),
                        };
                        badges.push(cat_badge);

                        if def.has_custom_mesh() {
                            badges.push(
                                MetadataBadge::new("Custom Mesh")
                                    .with_color(egui::Color32::from_rgb(50, 150, 220))
                                    .with_tooltip("Uses a custom OBJ-imported mesh"),
                            );
                        }

                        // SDK Rule 15: use show_standard_list_item for left-panel lists
                        let config = StandardListItemConfig::new(&def.name)
                            .with_badges(badges)
                            .with_id(def.id)
                            .with_icon(def.display_icon())
                            .selected(selected == Some(*idx));

                        let (clicked, ctx_action) = show_standard_list_item(left_ui, config);

                        if clicked {
                            new_selection = Some(*idx);
                        }
                        if ctx_action != ItemAction::None {
                            action_requested = Some(ctx_action);
                        }
                    });
                }

                if sorted.is_empty() {
                    left_ui.label("No furniture definitions found.");
                }
            },
            |right_ui| {
                if let Some(idx) = selected {
                    if let Some((_, def)) = sorted.iter().find(|(i, _)| *i == idx) {
                        right_ui.heading(&def.name);
                        right_ui.separator();
                        Self::show_preview_static(right_ui, def);
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select a definition to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a definition to view details");
                    });
                }
            },
        );

        // Apply selection change after closures (avoids borrow conflict)
        self.selected_furniture = new_selection;

        // Handle context-menu actions after the TwoColumnLayout closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_furniture {
                        if idx < defs.len() {
                            self.mode = FurnitureEditorMode::Edit;
                            self.edit_buffer = defs[idx].clone();
                            ui.ctx().request_repaint();
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_furniture {
                        if idx < defs.len() {
                            defs.remove(idx);
                            self.selected_furniture = None;
                            self.save_furniture(
                                defs,
                                campaign_dir,
                                furniture_file,
                                unsaved_changes,
                                status_message,
                            );
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_furniture {
                        if idx < defs.len() {
                            let mut new_def = defs[idx].clone();
                            new_def.id = next_available_id(defs);
                            new_def.name = format!("{} (Copy)", new_def.name);
                            defs.push(new_def);
                            self.save_furniture(
                                defs,
                                campaign_dir,
                                furniture_file,
                                unsaved_changes,
                                status_message,
                            );
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_furniture {
                        if idx < defs.len() {
                            match ron::ser::to_string_pretty(
                                &defs[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                Ok(ron_str) => {
                                    self.import_export_buffer = ron_str;
                                    self.show_import_dialog = true;
                                    *status_message = "Definition exported to dialog".to_string();
                                }
                                Err(_) => {
                                    *status_message = "Failed to serialize definition".to_string();
                                }
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    // =========================================================================
    // Preview panel (right column, read-only)
    // =========================================================================

    /// Renders a read-only summary of `def` in the right detail panel.
    fn show_preview_static(ui: &mut egui::Ui, def: &FurnitureDefinition) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        // SDK Rule 2: ScrollArea must have a distinct id_salt
        egui::ScrollArea::vertical()
            .id_salt("furniture_preview_scroll")
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.strong("Basic Info");
                    ui.label(format!("ID: {}", def.id));
                    ui.label(format!("Category: {}", def.category.name()));
                    ui.label(format!("Base Type: {}", def.base_type.name()));
                    ui.label(format!("Material: {}", def.material.name()));
                    ui.label(format!("Scale: {:.2}×", def.scale));
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.strong("Flags");
                    ui.label(if def.flags.lit {
                        "🔥 Lit"
                    } else {
                        "   Not lit"
                    });
                    ui.label(if def.flags.locked {
                        "🔒 Locked"
                    } else {
                        "   Unlocked"
                    });
                    ui.label(if def.flags.blocking {
                        "🚫 Blocks movement"
                    } else {
                        "   Passable"
                    });
                });

                if let Some(tint) = def.color_tint {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.strong("Color Tint");
                        let r = (tint[0] * 255.0).clamp(0.0, 255.0) as u8;
                        let g = (tint[1] * 255.0).clamp(0.0, 255.0) as u8;
                        let b = (tint[2] * 255.0).clamp(0.0, 255.0) as u8;
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(r, g, b), "████");
                            ui.label(format!(
                                "R:{:.2}  G:{:.2}  B:{:.2}",
                                tint[0], tint[1], tint[2]
                            ));
                        });
                    });
                }

                if !def.tags.is_empty() {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.strong("Tags");
                        ui.label(def.tags.join(", "));
                    });
                }

                if let Some(desc) = &def.description {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.strong("Description");
                        ui.label(desc);
                    });
                }

                if let Some(mesh_id) = def.mesh_id {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.strong("Custom Mesh");
                        ui.label(format!("Mesh ID: {}", mesh_id));
                    });
                }
            });
    }

    // =========================================================================
    // Import / export dialog
    // =========================================================================

    #[allow(clippy::too_many_arguments)]
    fn show_import_dialog(
        &mut self,
        ctx: &egui::Context,
        defs: &mut Vec<FurnitureDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        furniture_file: &str,
    ) {
        let mut open = self.show_import_dialog;

        // SDK Rule 8: Window title must be unique across the whole frame
        egui::Window::new("Import/Export Furniture Definition")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.strong("Furniture RON Data");
                ui.separator();
                ui.label("Paste RON to import, or copy the exported text below:");

                let text_edit = egui::TextEdit::multiline(&mut self.import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                // SDK Rule 12: use horizontal_wrapped for button rows
                ui.horizontal_wrapped(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<FurnitureDefinition>(&self.import_export_buffer) {
                            Ok(mut def) => {
                                def.id = next_available_id(defs);
                                defs.push(def);
                                self.save_furniture(
                                    defs,
                                    campaign_dir,
                                    furniture_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                *status_message =
                                    "Furniture definition imported successfully".to_string();
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

    // =========================================================================
    // Add / Edit form
    // =========================================================================

    #[allow(clippy::too_many_arguments)]
    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<FurnitureDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        furniture_file: &str,
    ) {
        let is_add = self.mode == FurnitureEditorMode::Add;
        ui.heading(if is_add {
            "Add New Furniture"
        } else {
            "Edit Furniture"
        });
        ui.separator();

        // SDK Rule 2: distinct id_salt
        egui::ScrollArea::vertical()
            .id_salt("furniture_form_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ---- Basic Properties ----------------------------------------
                ui.group(|ui| {
                    ui.strong("Basic Properties");
                    ui.add_space(4.0);

                    // ID (read-only display)
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

                    // Icon override
                    ui.horizontal(|ui| {
                        ui.label("Icon (emoji override):");
                        let mut icon_buf = self.edit_buffer.icon.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut icon_buf).changed() {
                            self.edit_buffer.icon = if icon_buf.is_empty() {
                                None
                            } else {
                                Some(icon_buf)
                            };
                        }
                    });

                    // Category
                    ui.horizontal(|ui| {
                        ui.label("Category:");
                        // SDK Rule 3
                        egui::ComboBox::from_id_salt("furniture_form_category")
                            .selected_text(self.edit_buffer.category.name())
                            .show_ui(ui, |ui| {
                                for cat in FurnitureCategory::all() {
                                    ui.selectable_value(
                                        &mut self.edit_buffer.category,
                                        *cat,
                                        cat.name(),
                                    );
                                }
                            });
                    });

                    // Base type
                    ui.horizontal(|ui| {
                        ui.label("Base Type:");
                        egui::ComboBox::from_id_salt("furniture_form_base_type")
                            .selected_text(self.edit_buffer.base_type.name())
                            .show_ui(ui, |ui| {
                                for t in FurnitureType::all() {
                                    ui.selectable_value(
                                        &mut self.edit_buffer.base_type,
                                        *t,
                                        t.name(),
                                    );
                                }
                            });
                    });

                    // Material
                    ui.horizontal(|ui| {
                        ui.label("Material:");
                        egui::ComboBox::from_id_salt("furniture_form_material")
                            .selected_text(self.edit_buffer.material.name())
                            .show_ui(ui, |ui| {
                                for m in FurnitureMaterial::all() {
                                    ui.selectable_value(
                                        &mut self.edit_buffer.material,
                                        *m,
                                        m.name(),
                                    );
                                }
                            });
                    });

                    // Scale
                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.add(
                            egui::Slider::new(&mut self.edit_buffer.scale, 0.1..=5.0)
                                .step_by(0.05)
                                .text("×"),
                        );
                    });
                });

                ui.add_space(8.0);

                // ---- Color Tint ---------------------------------------------
                ui.group(|ui| {
                    ui.strong("Color Tint");
                    ui.add_space(4.0);

                    let has_tint = self.edit_buffer.color_tint.is_some();
                    let mut enable_tint = has_tint;

                    if ui
                        .checkbox(&mut enable_tint, "Enable custom color tint")
                        .changed()
                    {
                        if enable_tint {
                            // Seed from material base color
                            self.edit_buffer.color_tint =
                                Some(self.edit_buffer.material.base_color());
                        } else {
                            self.edit_buffer.color_tint = None;
                        }
                        ui.ctx().request_repaint();
                    }

                    if let Some(ref mut tint) = self.edit_buffer.color_tint {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("R:");
                            ui.add(egui::Slider::new(&mut tint[0], 0.0..=1.0).step_by(0.01));
                        });
                        ui.horizontal(|ui| {
                            ui.label("G:");
                            ui.add(egui::Slider::new(&mut tint[1], 0.0..=1.0).step_by(0.01));
                        });
                        ui.horizontal(|ui| {
                            ui.label("B:");
                            ui.add(egui::Slider::new(&mut tint[2], 0.0..=1.0).step_by(0.01));
                        });
                        // Swatch preview
                        let r = (tint[0] * 255.0).clamp(0.0, 255.0) as u8;
                        let g = (tint[1] * 255.0).clamp(0.0, 255.0) as u8;
                        let b = (tint[2] * 255.0).clamp(0.0, 255.0) as u8;
                        ui.horizontal(|ui| {
                            ui.label("Preview:");
                            ui.colored_label(egui::Color32::from_rgb(r, g, b), "████████");
                        });
                    }
                });

                ui.add_space(8.0);

                // ---- Flags --------------------------------------------------
                ui.group(|ui| {
                    ui.strong("Flags");
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.edit_buffer.flags.lit, "🔥 Lit (emits light)");
                    ui.checkbox(&mut self.edit_buffer.flags.locked, "🔒 Locked");
                    ui.checkbox(&mut self.edit_buffer.flags.blocking, "🚫 Blocks movement");
                });

                ui.add_space(8.0);

                // ---- Custom Mesh ID -----------------------------------------
                ui.group(|ui| {
                    ui.strong("Custom Mesh");
                    ui.add_space(4.0);

                    let has_mesh = self.edit_buffer.mesh_id.is_some();
                    let mut enable_mesh = has_mesh;

                    if ui
                        .checkbox(&mut enable_mesh, "Use custom OBJ-imported mesh")
                        .changed()
                    {
                        if enable_mesh {
                            self.edit_buffer.mesh_id =
                                available_mesh_ids.first().copied().or(Some(10001));
                        } else {
                            self.edit_buffer.mesh_id = None;
                        }
                        ui.ctx().request_repaint();
                    }

                    if let Some(ref mut mesh_id) = self.edit_buffer.mesh_id {
                        ui.horizontal(|ui| {
                            ui.label("Mesh ID:");

                            egui::ComboBox::from_id_salt("furniture_mesh_id_selector")
                                .selected_text(mesh_id.to_string())
                                .show_ui(ui, |ui| {
                                    for id in available_mesh_ids {
                                        ui.selectable_value(mesh_id, *id, id.to_string());
                                    }
                                });
                        });

                        ui.small("Mesh IDs come from furniture_mesh_registry.ron");

                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Open in OBJ Importer").clicked() {
                                self.requested_signal =
                                    Some(FurnitureEditorSignal::OpenInObjImporter);
                                *status_message =
                                    "Opening OBJ Importer for furniture mesh work".to_string();
                                ui.ctx().request_repaint();
                            }
                        });
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Open in OBJ Importer").clicked() {
                                self.requested_signal =
                                    Some(FurnitureEditorSignal::OpenInObjImporter);
                                *status_message =
                                    "Opening OBJ Importer for furniture mesh work".to_string();
                                ui.ctx().request_repaint();
                            }
                        });
                    }
                });

                ui.add_space(8.0);

                // ---- Tags ---------------------------------------------------
                ui.group(|ui| {
                    ui.strong("Tags");
                    ui.add_space(4.0);

                    let mut tags_csv = self.edit_buffer.tags.join(", ");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut tags_csv)
                                .hint_text("dungeon, boss, key …"),
                        )
                        .changed()
                    {
                        self.edit_buffer.tags = tags_csv
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    ui.small("Comma-separated tags for editor filtering");
                });

                ui.add_space(8.0);

                // ---- Description --------------------------------------------
                ui.group(|ui| {
                    ui.strong("Description (optional)");
                    ui.add_space(4.0);

                    let has_desc = self.edit_buffer.description.is_some();
                    let mut enable_desc = has_desc;

                    if ui
                        .checkbox(&mut enable_desc, "Include flavor text")
                        .changed()
                    {
                        self.edit_buffer.description = if enable_desc {
                            Some(String::new())
                        } else {
                            None
                        };
                        ui.ctx().request_repaint();
                    }

                    if let Some(ref mut desc) = self.edit_buffer.description {
                        ui.add(egui::TextEdit::multiline(desc).desired_rows(4).hint_text(
                            "Flavor text shown in the editor and any in-game inspect UI…",
                        ));
                    }
                });

                ui.add_space(12.0);
                ui.separator();

                // ---- Form action buttons ------------------------------------
                // SDK Rule 12: use horizontal_wrapped for button rows
                ui.horizontal_wrapped(|ui| {
                    if ui.button("✅ Save Definition").clicked() {
                        if is_add {
                            defs.push(self.edit_buffer.clone());
                        } else if let Some(idx) = self.selected_furniture {
                            if idx < defs.len() {
                                defs[idx] = self.edit_buffer.clone();
                            }
                        }
                        self.save_furniture(
                            defs,
                            campaign_dir,
                            furniture_file,
                            unsaved_changes,
                            status_message,
                        );
                        self.mode = FurnitureEditorMode::List;
                        ui.ctx().request_repaint();
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = FurnitureEditorMode::List;
                        ui.ctx().request_repaint();
                    }
                });
            });
    }

    // =========================================================================
    // Persistence
    // =========================================================================

    /// Serialises `defs` to RON and writes to `campaign_dir/furniture_file`.
    ///
    /// Sets `*unsaved_changes = true` on success and updates `status_message`
    /// in both success and failure cases.
    fn save_furniture(
        &self,
        defs: &[FurnitureDefinition],
        campaign_dir: Option<&PathBuf>,
        furniture_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let path = dir.join(furniture_file);

            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    *status_message = format!("Failed to create directory: {}", e);
                    return;
                }
            }

            match ron::ser::to_string_pretty(defs, Default::default()) {
                Ok(contents) => match std::fs::write(&path, &contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        *status_message = format!("Auto-saved furniture to: {}", path.display());
                    }
                    Err(e) => {
                        *status_message = format!("Failed to save furniture: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize furniture: {}", e);
                }
            }
        }
    }
}

// =============================================================================
// Private helpers
// =============================================================================

/// Returns the next available [`FurnitureId`] by taking max current + 1.
/// Returns 1 when the list is empty.
fn next_available_id(defs: &[FurnitureDefinition]) -> FurnitureId {
    defs.iter()
        .map(|d| d.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::world::furniture::FurnitureDefinition;
    use antares::domain::world::{
        FurnitureCategory, FurnitureFlags, FurnitureMaterial, FurnitureType,
    };

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_def(id: FurnitureId, name: &str) -> FurnitureDefinition {
        FurnitureDefinition {
            id,
            name: name.to_string(),
            category: FurnitureCategory::Seating,
            base_type: FurnitureType::Throne,
            material: FurnitureMaterial::Wood,
            scale: 1.0,
            color_tint: None,
            flags: FurnitureFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }
    }

    // ── FurnitureEditorMode ───────────────────────────────────────────────────

    #[test]
    fn test_editor_mode_variants_are_distinct() {
        assert_ne!(FurnitureEditorMode::List, FurnitureEditorMode::Add);
        assert_ne!(FurnitureEditorMode::Add, FurnitureEditorMode::Edit);
        assert_ne!(FurnitureEditorMode::List, FurnitureEditorMode::Edit);
    }

    // ── FurnitureCategoryFilter ───────────────────────────────────────────────

    #[test]
    fn test_category_filter_all_returns_six_variants() {
        assert_eq!(FurnitureCategoryFilter::all().len(), 6);
    }

    #[test]
    fn test_category_filter_as_str_non_empty() {
        for f in FurnitureCategoryFilter::all() {
            assert!(!f.as_str().is_empty());
        }
    }

    #[test]
    fn test_category_filter_matches_seating() {
        let def = make_def(1, "Bench");
        assert!(FurnitureCategoryFilter::Seating.matches(&def));
        assert!(!FurnitureCategoryFilter::Storage.matches(&def));
    }

    #[test]
    fn test_category_filter_matches_storage() {
        let mut def = make_def(2, "Chest");
        def.category = FurnitureCategory::Storage;
        assert!(FurnitureCategoryFilter::Storage.matches(&def));
        assert!(!FurnitureCategoryFilter::Lighting.matches(&def));
    }

    #[test]
    fn test_category_filter_matches_lighting() {
        let mut def = make_def(3, "Torch");
        def.category = FurnitureCategory::Lighting;
        assert!(FurnitureCategoryFilter::Lighting.matches(&def));
        assert!(!FurnitureCategoryFilter::Decoration.matches(&def));
    }

    #[test]
    fn test_category_filter_matches_passage() {
        let mut def = make_def(4, "Door");
        def.category = FurnitureCategory::Passage;
        assert!(FurnitureCategoryFilter::Passage.matches(&def));
        assert!(!FurnitureCategoryFilter::Utility.matches(&def));
    }

    #[test]
    fn test_category_filter_matches_decoration() {
        let mut def = make_def(5, "Statue");
        def.category = FurnitureCategory::Decoration;
        assert!(FurnitureCategoryFilter::Decoration.matches(&def));
    }

    #[test]
    fn test_category_filter_matches_utility() {
        let mut def = make_def(6, "Table");
        def.category = FurnitureCategory::Utility;
        assert!(FurnitureCategoryFilter::Utility.matches(&def));
    }

    // ── FurnitureEditorState ──────────────────────────────────────────────────

    #[test]
    fn test_state_new_returns_list_mode() {
        let state = FurnitureEditorState::new();
        assert_eq!(state.mode, FurnitureEditorMode::List);
    }

    #[test]
    fn test_state_default_selected_is_none() {
        let state = FurnitureEditorState::default();
        assert!(state.selected_furniture.is_none());
    }

    #[test]
    fn test_state_default_filters_are_none() {
        let state = FurnitureEditorState::default();
        assert!(state.filter_category.is_none());
        assert!(state.filter_base_type.is_none());
    }

    #[test]
    fn test_state_default_import_dialog_closed() {
        let state = FurnitureEditorState::default();
        assert!(!state.show_import_dialog);
        assert!(state.import_export_buffer.is_empty());
    }

    // ── default_furniture ────────────────────────────────────────────────────

    #[test]
    fn test_default_furniture_name() {
        let def = FurnitureEditorState::default_furniture();
        assert_eq!(def.name, "New Furniture");
    }

    #[test]
    fn test_default_furniture_id_is_zero() {
        let def = FurnitureEditorState::default_furniture();
        assert_eq!(def.id, 0);
    }

    #[test]
    fn test_default_furniture_scale_is_one() {
        let def = FurnitureEditorState::default_furniture();
        assert!((def.scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_furniture_no_color_tint() {
        let def = FurnitureEditorState::default_furniture();
        assert!(def.color_tint.is_none());
    }

    #[test]
    fn test_default_furniture_no_mesh_id() {
        let def = FurnitureEditorState::default_furniture();
        assert!(def.mesh_id.is_none());
    }

    #[test]
    fn test_default_furniture_empty_tags() {
        let def = FurnitureEditorState::default_furniture();
        assert!(def.tags.is_empty());
    }

    #[test]
    fn test_default_furniture_flags_all_false() {
        let def = FurnitureEditorState::default_furniture();
        assert!(!def.flags.lit);
        assert!(!def.flags.locked);
        assert!(!def.flags.blocking);
    }

    // ── next_available_id helper ──────────────────────────────────────────────

    #[test]
    fn test_next_available_id_empty_list_returns_one() {
        let defs: Vec<FurnitureDefinition> = vec![];
        assert_eq!(next_available_id(&defs), 1);
    }

    #[test]
    fn test_next_available_id_increments_max() {
        let defs = vec![make_def(1, "A"), make_def(5, "B"), make_def(3, "C")];
        assert_eq!(next_available_id(&defs), 6);
    }

    #[test]
    fn test_next_available_id_with_gaps() {
        let defs = vec![make_def(10, "X")];
        assert_eq!(next_available_id(&defs), 11);
    }

    // ── Add / Edit / Delete logic ─────────────────────────────────────────────

    #[test]
    fn test_add_definition_to_list() {
        let mut defs: Vec<FurnitureDefinition> = vec![];
        let new_def = make_def(1, "Throne");
        defs.push(new_def.clone());
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "Throne");
    }

    #[test]
    fn test_edit_definition_in_list() {
        let mut defs = vec![make_def(1, "Old Name")];
        defs[0].name = "New Name".to_string();
        assert_eq!(defs[0].name, "New Name");
    }

    #[test]
    fn test_delete_definition_from_list() {
        let mut defs = vec![make_def(1, "A"), make_def(2, "B"), make_def(3, "C")];
        defs.remove(1);
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].id, 1);
        assert_eq!(defs[1].id, 3);
    }

    #[test]
    fn test_duplicate_definition_assigns_new_id() {
        let mut defs = vec![make_def(1, "Throne")];
        let mut copy = defs[0].clone();
        copy.id = next_available_id(&defs);
        copy.name = format!("{} (Copy)", copy.name);
        defs.push(copy);

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[1].id, 2);
        assert_eq!(defs[1].name, "Throne (Copy)");
    }

    // ── RON round-trip ────────────────────────────────────────────────────────

    #[test]
    fn test_single_definition_ron_roundtrip() {
        let def = make_def(42, "Stone Bench");
        let ron_str = ron::ser::to_string_pretty(&def, ron::ser::PrettyConfig::default())
            .expect("Serialize must succeed");
        let restored: FurnitureDefinition =
            ron::from_str(&ron_str).expect("Deserialize must succeed");
        assert_eq!(restored.id, 42);
        assert_eq!(restored.name, "Stone Bench");
    }

    #[test]
    fn test_definition_list_ron_roundtrip() {
        let defs = vec![
            make_def(1, "Throne"),
            make_def(2, "Bench"),
            make_def(3, "Torch"),
        ];
        let ron_str = ron::ser::to_string_pretty(&defs, ron::ser::PrettyConfig::default())
            .expect("Serialize must succeed");
        let restored: Vec<FurnitureDefinition> =
            ron::from_str(&ron_str).expect("Deserialize must succeed");
        assert_eq!(restored.len(), 3);
        assert_eq!(restored[2].name, "Torch");
    }

    #[test]
    fn test_definition_with_color_tint_roundtrip() {
        let mut def = make_def(7, "Lit Torch");
        def.color_tint = Some([1.0, 0.6, 0.2]);
        def.flags.lit = true;

        let ron_str =
            ron::ser::to_string_pretty(&def, Default::default()).expect("Serialize must succeed");
        let restored: FurnitureDefinition =
            ron::from_str(&ron_str).expect("Deserialize must succeed");

        assert_eq!(restored.color_tint, Some([1.0, 0.6, 0.2]));
        assert!(restored.flags.lit);
    }

    #[test]
    fn test_definition_with_mesh_id_roundtrip() {
        let mut def = make_def(10, "Custom Table");
        def.mesh_id = Some(10001);

        let ron_str =
            ron::ser::to_string_pretty(&def, Default::default()).expect("Serialize must succeed");
        let restored: FurnitureDefinition =
            ron::from_str(&ron_str).expect("Deserialize must succeed");

        assert_eq!(restored.mesh_id, Some(10001));
        assert!(restored.has_custom_mesh());
    }

    #[test]
    fn test_definition_with_tags_roundtrip() {
        let mut def = make_def(11, "Tagged Barrel");
        def.tags = vec!["dungeon".to_string(), "storage".to_string()];

        let ron_str =
            ron::ser::to_string_pretty(&def, Default::default()).expect("Serialize must succeed");
        let restored: FurnitureDefinition =
            ron::from_str(&ron_str).expect("Deserialize must succeed");

        assert_eq!(restored.tags, vec!["dungeon", "storage"]);
    }

    #[test]
    fn test_definition_with_description_roundtrip() {
        let mut def = make_def(12, "Fancy Chest");
        def.description = Some("An ornate chest bound in iron.".to_string());

        let ron_str =
            ron::ser::to_string_pretty(&def, Default::default()).expect("Serialize must succeed");
        let restored: FurnitureDefinition =
            ron::from_str(&ron_str).expect("Deserialize must succeed");

        assert_eq!(
            restored.description,
            Some("An ornate chest bound in iron.".to_string())
        );
    }

    // ── Filter interaction ────────────────────────────────────────────────────

    #[test]
    fn test_filter_by_category_seating() {
        let defs = vec![
            make_def(1, "Throne"), // Seating
            {
                let mut d = make_def(2, "Torch");
                d.category = FurnitureCategory::Lighting;
                d
            },
            {
                let mut d = make_def(3, "Bench");
                d.category = FurnitureCategory::Seating;
                d
            },
        ];

        let filtered: Vec<_> = defs
            .iter()
            .filter(|d| FurnitureCategoryFilter::Seating.matches(d))
            .collect();

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_base_type_torch() {
        let defs = vec![
            make_def(1, "Throne"),
            {
                let mut d = make_def(2, "Torch A");
                d.base_type = FurnitureType::Torch;
                d
            },
            {
                let mut d = make_def(3, "Torch B");
                d.base_type = FurnitureType::Torch;
                d
            },
        ];

        let filtered: Vec<_> = defs
            .iter()
            .filter(|d| d.base_type == FurnitureType::Torch)
            .collect();

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_name_search() {
        let defs = vec![
            make_def(1, "Iron Throne"),
            make_def(2, "Wooden Bench"),
            make_def(3, "Iron Chest"),
        ];
        let query = "iron";
        let filtered: Vec<_> = defs
            .iter()
            .filter(|d| d.name.to_lowercase().contains(query))
            .collect();
        assert_eq!(filtered.len(), 2);
    }

    // ── Save furniture to tmp dir ─────────────────────────────────────────────

    #[test]
    fn test_save_furniture_creates_file() {
        let tmp = tempfile::TempDir::new().expect("tmp dir");
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("data")).expect("create data dir");

        let defs = vec![make_def(1, "Throne"), make_def(2, "Bench")];
        let state = FurnitureEditorState::new();
        let mut unsaved = false;
        let mut status = String::new();

        state.save_furniture(
            &defs,
            Some(&dir),
            "data/furniture.ron",
            &mut unsaved,
            &mut status,
        );

        assert!(unsaved, "unsaved_changes should be true after save");
        assert!(dir.join("data/furniture.ron").exists(), "file must exist");

        // Verify the written file round-trips
        let contents = std::fs::read_to_string(dir.join("data/furniture.ron")).expect("read");
        let restored: Vec<FurnitureDefinition> = ron::from_str(&contents).expect("parse");
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].name, "Throne");
    }

    #[test]
    fn test_save_furniture_no_campaign_dir_is_noop() {
        let defs = vec![make_def(1, "Throne")];
        let state = FurnitureEditorState::new();
        let mut unsaved = false;
        let mut status = String::new();

        state.save_furniture(&defs, None, "data/furniture.ron", &mut unsaved, &mut status);

        // No crash, no changes, no status update from the Ok path
        assert!(!unsaved);
    }

    #[test]
    fn test_save_furniture_creates_missing_parent_directories() {
        let tmp = tempfile::TempDir::new().expect("tmp dir");
        let dir = tmp.path().to_path_buf();
        // data/ subdir does NOT exist yet

        let defs = vec![make_def(1, "Bench")];
        let state = FurnitureEditorState::new();
        let mut unsaved = false;
        let mut status = String::new();

        state.save_furniture(
            &defs,
            Some(&dir),
            "data/furniture.ron",
            &mut unsaved,
            &mut status,
        );

        assert!(dir.join("data/furniture.ron").exists());
        assert!(unsaved);
    }

    // ── Mode transitions ──────────────────────────────────────────────────────

    #[test]
    fn test_mode_starts_as_list() {
        let state = FurnitureEditorState::new();
        assert_eq!(state.mode, FurnitureEditorMode::List);
    }

    #[test]
    fn test_mode_add_is_not_edit() {
        assert_ne!(FurnitureEditorMode::Add, FurnitureEditorMode::Edit);
    }

    // ── Edit buffer manipulation ──────────────────────────────────────────────

    #[test]
    fn test_edit_buffer_mutation_does_not_affect_list() {
        let original = make_def(1, "Original");
        let mut state = FurnitureEditorState::new();
        state.edit_buffer = original.clone();
        state.edit_buffer.name = "Changed".to_string();

        // Original is unchanged (it's a clone)
        assert_eq!(original.name, "Original");
        assert_eq!(state.edit_buffer.name, "Changed");
    }

    #[test]
    fn test_tags_csv_round_trip_logic() {
        let csv = "dungeon, boss, key";
        let tags: Vec<String> = csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(tags, vec!["dungeon", "boss", "key"]);
    }

    #[test]
    fn test_tags_csv_empty_string_produces_empty_vec() {
        let csv = "";
        let tags: Vec<String> = csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(tags.is_empty());
    }

    // ── has_custom_mesh delegation ────────────────────────────────────────────

    #[test]
    fn test_has_custom_mesh_true_when_mesh_id_set() {
        let mut def = FurnitureEditorState::default_furniture();
        def.mesh_id = Some(10001);
        assert!(def.has_custom_mesh());
    }

    #[test]
    fn test_has_custom_mesh_false_when_no_mesh_id() {
        let def = FurnitureEditorState::default_furniture();
        assert!(!def.has_custom_mesh());
    }

    // ── display_icon ──────────────────────────────────────────────────────────

    #[test]
    fn test_display_icon_uses_override_when_set() {
        let mut def = FurnitureEditorState::default_furniture();
        def.icon = Some("🏰".to_string());
        assert_eq!(def.display_icon(), "🏰");
    }

    #[test]
    fn test_display_icon_falls_back_to_base_type() {
        let def = FurnitureEditorState::default_furniture();
        // default is Throne → 👑
        assert_eq!(def.display_icon(), FurnitureType::Throne.icon());
    }
}
