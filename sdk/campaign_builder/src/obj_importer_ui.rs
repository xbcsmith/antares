// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OBJ and GLB importer tab UI.
//!
//! This module renders the Campaign Builder Importer tab, lets users inspect
//! imported OBJ and GLB meshes, edit per-mesh colors, manage custom palette
//! entries, and export the result as a `CreatureDefinition` RON asset.
//!
//! # Examples
//!
//! ```ignore
//! use campaign_builder::logging::Logger;
//! use campaign_builder::obj_importer::ObjImporterState;
//! use campaign_builder::obj_importer_ui::show_obj_importer_tab;
//! use eframe::egui;
//! use std::path::PathBuf;
//!
//! let ctx = egui::Context::default();
//! let mut importer = ObjImporterState::new();
//! let mut logger = Logger::default();
//! let campaign_dir = Some(PathBuf::from("/tmp/campaign"));
//!
//! egui::CentralPanel::default().show(&ctx, |ui| {
//!     let _ = show_obj_importer_tab(ui, &mut importer, campaign_dir.as_ref(), &mut logger);
//! });
//! ```

use crate::color_palette::{palette_entries, PaletteEntry};
use crate::creature_assets::{CreatureAssetError, CreatureAssetManager};
use crate::logging::{category, Logger};
use crate::obj_importer::{
    ExportType, ImportSourceFormat, ImportedMaterialSwatch, ImportedMeshColorSource,
    ImportedMtlSourceKind, ImportedTexturePayload, ImporterMode, ObjImporterState,
};
use crate::ui_helpers::TwoColumnLayout;
use antares::domain::visual::{CreatureDefinition, MeshTransform};
use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur when importing a model file into editor state.
#[derive(Debug, thiserror::Error)]
enum ObjImportError {
    /// The file could not be loaded or parsed.
    #[error("Failed to load '{path}': {message}")]
    LoadFailed { path: String, message: String },
    /// The file extension is not a supported import format.
    #[error("Unsupported file format '.{extension}'; supported formats are .obj and .glb")]
    UnknownFormat { extension: String },
}

/// Signal emitted by the importer tab when an export completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjImporterUiSignal {
    /// A creature asset was exported and the creature registry should be reloaded.
    Creature,
    /// An item asset was exported.
    Item,
    /// A furniture mesh asset was exported.
    Furniture,
}

#[derive(Debug, Clone, PartialEq)]
struct MeshRowSnapshot {
    index: usize,
    name: String,
    vertex_count: usize,
    triangle_count: usize,
    color: [f32; 4],
    selected: bool,
    is_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct ExportOutcome {
    export_type: ExportType,
    absolute_path: PathBuf,
    status_message: String,
}

#[derive(Debug, Error)]
enum ObjImporterExportError {
    #[error("Open a campaign before exporting importer output")]
    MissingCampaignDir,

    #[error("Load an OBJ file before exporting")]
    NoMeshesLoaded,

    #[error("Enter a name before exporting")]
    MissingName,

    #[error("Failed to save creature asset: {0}")]
    CreatureAsset(#[from] CreatureAssetError),

    #[error("Failed to serialize RON asset: {0}")]
    Serialization(#[from] ron::Error),

    #[error("Texture for mesh '{mesh_name}' was not found: {path}")]
    MissingTexture { mesh_name: String, path: PathBuf },

    #[error("Texture source path has no filename: {0}")]
    TextureMissingFileName(PathBuf),

    #[error("Failed to write asset file: {0}")]
    Io(#[from] std::io::Error),
}

/// Renders the OBJ importer tab and returns a signal when export completes.
pub(crate) fn show_obj_importer_tab(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    campaign_dir: Option<&PathBuf>,
    logger: &mut Logger,
) -> Option<ObjImporterUiSignal> {
    ui.heading("Model Importer");
    ui.label("Load a Wavefront OBJ or binary glTF (GLB) model, adjust mesh colors, then export a creature, item, or furniture RON asset.");

    if !state.status_message.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(&state.status_message).italics());
    }

    if campaign_dir.is_none() {
        ui.colored_label(
            egui::Color32::from_rgb(220, 170, 90),
            "Open a campaign to enable RON export destinations.",
        );
    }

    ui.separator();

    match state.mode {
        ImporterMode::Idle => {
            render_idle_mode(ui, state, logger);
            None
        }
        ImporterMode::Loaded => render_loaded_mode(ui, state, campaign_dir, logger),
        ImporterMode::Exporting => {
            ui.add(egui::Spinner::new());
            ui.label("Exporting importer asset...");
            None
        }
    }
}

fn render_idle_mode(ui: &mut egui::Ui, state: &mut ObjImporterState, logger: &mut Logger) {
    let selected_path = state
        .source_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "No model selected".to_string());

    egui::Grid::new("obj_importer_idle_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Source File:");
            ui.horizontal(|ui| {
                ui.monospace(selected_path.as_str());
                if ui.button("Browse...").clicked() {
                    if let Some(path) = pick_model_file(state.source_path.as_deref()) {
                        state.source_path = Some(path.clone());
                        if state.creature_name.trim().is_empty() {
                            state.creature_name = display_name_from_path(&path);
                        }
                        state.status_message = format!("Selected file: {}", path.display());
                        ui.ctx().request_repaint();
                    }
                }
            });
            ui.end_row();

            ui.label("Export Type:");
            ui.horizontal(|ui| {
                if ui
                    .radio_value(&mut state.export_type, ExportType::Creature, "Creature")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
                if ui
                    .radio_value(&mut state.export_type, ExportType::Item, "Item")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
                if ui
                    .radio_value(&mut state.export_type, ExportType::Furniture, "Furniture")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
            });
            ui.end_row();

            ui.label("Category:");
            if ui.text_edit_singleline(&mut state.category).changed() {
                ui.ctx().request_repaint();
            }
            ui.end_row();

            ui.label("Scale:");
            if ui
                .add(
                    egui::DragValue::new(&mut state.scale)
                        .speed(0.001)
                        .range(0.0001..=100.0)
                        .fixed_decimals(3),
                )
                .changed()
            {
                ui.ctx().request_repaint();
            }
            ui.end_row();

            ui.label("Name:");
            if ui.text_edit_singleline(&mut state.creature_name).changed() {
                ui.ctx().request_repaint();
            }
            ui.end_row();
        });

    ui.add_space(8.0);

    if ui.button("Load Model").clicked() {
        let path = if let Some(path) = state.source_path.clone() {
            path
        } else if let Some(path) = pick_model_file(None) {
            state.source_path = Some(path.clone());
            path
        } else {
            state.status_message = "Choose a model file before loading.".to_string();
            return;
        };

        match load_model_into_state(state, &path) {
            Ok(()) => {
                logger.info(
                    category::EDITOR,
                    &format!("Loaded model importer source {}", path.display()),
                );
                ui.ctx().request_repaint();
            }
            Err(error) => {
                state.status_message = error.to_string();
                logger.error(category::FILE_IO, &state.status_message);
            }
        }
    }
}

fn render_loaded_mode(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    campaign_dir: Option<&PathBuf>,
    logger: &mut Logger,
) -> Option<ObjImporterUiSignal> {
    if state.active_mesh_index.is_none() && !state.meshes.is_empty() {
        state.set_active_mesh(Some(0));
    }

    let total_vertices: usize = state.meshes.iter().map(|mesh| mesh.vertex_count).sum();
    let total_triangles: usize = state.meshes.iter().map(|mesh| mesh.triangle_count).sum();
    let source_path_text = state
        .source_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(loaded from memory)".to_string());

    ui.horizontal_wrapped(|ui| {
        ui.label("Source:");
        ui.monospace(source_path_text);
    });

    ui.add_space(6.0);

    egui::Grid::new("obj_importer_loaded_metadata_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Format:");
            match state.source_format {
                ImportSourceFormat::Obj => {
                    ui.label("OBJ");
                }
                ImportSourceFormat::Glb => {
                    ui.label("GLB (binary glTF)");
                }
            }
            ui.end_row();

            if state.source_format == ImportSourceFormat::Obj {
                ui.label("MTL Source:");
                render_mtl_source_controls(ui, state, logger, true);
                ui.end_row();
            }

            if state.source_format == ImportSourceFormat::Glb {
                if let Some(count) = parse_glb_embedded_image_count(&state.status_message) {
                    ui.label("Embedded textures:");
                    ui.label(count.to_string());
                    ui.end_row();
                }
            }

            ui.label("Export Type:");
            ui.horizontal(|ui| {
                if ui
                    .radio_value(&mut state.export_type, ExportType::Creature, "Creature")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
                if ui
                    .radio_value(&mut state.export_type, ExportType::Item, "Item")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
                if ui
                    .radio_value(&mut state.export_type, ExportType::Furniture, "Furniture")
                    .changed()
                {
                    ui.ctx().request_repaint();
                }
            });
            ui.end_row();

            ui.label("ID:");
            match state.export_type {
                ExportType::Furniture => {
                    if ui
                        .add(egui::DragValue::new(&mut state.furniture_id).range(0..=u32::MAX))
                        .changed()
                    {
                        ui.ctx().request_repaint();
                    }
                }
                ExportType::Creature | ExportType::Item => {
                    if ui
                        .add(egui::DragValue::new(&mut state.creature_id).range(0..=u32::MAX))
                        .changed()
                    {
                        ui.ctx().request_repaint();
                    }
                }
            }
            ui.end_row();

            ui.label("Category:");
            if ui.text_edit_singleline(&mut state.category).changed() {
                ui.ctx().request_repaint();
            }
            ui.end_row();

            ui.label("Name:");
            if ui.text_edit_singleline(&mut state.creature_name).changed() {
                ui.ctx().request_repaint();
            }
            ui.end_row();

            ui.label("Import Scale:");
            if ui
                .add(
                    egui::DragValue::new(&mut state.scale)
                        .speed(0.001)
                        .range(0.0001..=100.0)
                        .fixed_decimals(3),
                )
                .changed()
            {
                ui.ctx().request_repaint();
            }
            ui.end_row();

            ui.label("Export Path:");
            ui.monospace(preview_export_relative_path(
                state.export_type,
                &state.creature_name,
                match state.export_type {
                    ExportType::Furniture => state.furniture_id,
                    ExportType::Creature | ExportType::Item => state.creature_id,
                },
                &state.category,
            ));
            ui.end_row();
        });

    if state.source_format == ImportSourceFormat::Glb
        && state.status_message.contains("[skinning ignored]")
    {
        ui.add_space(4.0);
        ui.colored_label(
            egui::Color32::from_rgb(220, 170, 90),
            "\u{26a0} Skinning/animations present but not imported",
        );
    }

    ui.add_space(6.0);
    let mut export_signal = None;
    let mut cleared_importer = false;

    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "Summary: {} meshes, {} vertices, {} triangles",
            state.meshes.len(),
            total_vertices,
            total_triangles
        ));

        if ui.button("Auto-Assign All").clicked() {
            state.auto_assign_colors();
            state.status_message = "Reapplied automatic colors to all meshes.".to_string();
            ui.ctx().request_repaint();
        }

        if ui.button("Load Another Model").clicked() {
            if let Some(path) = pick_model_file(state.source_path.as_deref()) {
                match load_model_into_state(state, &path) {
                    Ok(()) => {
                        logger.info(
                            category::EDITOR,
                            &format!("Reloaded importer model {}", path.display()),
                        );
                        ui.ctx().request_repaint();
                    }
                    Err(error) => {
                        state.status_message = error.to_string();
                        logger.error(category::FILE_IO, &state.status_message);
                    }
                }
            }
        }

        if ui.button("Back / Clear").clicked() {
            state.clear();
            state.status_message = "Importer cleared.".to_string();
            cleared_importer = true;
            ui.ctx().request_repaint();
        }

        if ui
            .checkbox(
                &mut state.open_after_export,
                "Open exported creature in editor after export",
            )
            .changed()
        {
            ui.ctx().request_repaint();
        }

        let export_enabled = campaign_dir.is_some() && !state.meshes.is_empty();
        if ui
            .add_enabled(export_enabled, egui::Button::new("Export RON"))
            .clicked()
        {
            state.mode = ImporterMode::Exporting;
            match export_state_to_campaign(state, campaign_dir.map(|path| path.as_path())) {
                Ok(outcome) => {
                    let signal = match outcome.export_type {
                        ExportType::Creature => ObjImporterUiSignal::Creature,
                        ExportType::Item => ObjImporterUiSignal::Item,
                        ExportType::Furniture => ObjImporterUiSignal::Furniture,
                    };
                    logger.info(category::FILE_IO, &outcome.status_message);
                    state.clear();
                    state.status_message = outcome.status_message;
                    ui.ctx().request_repaint();
                    export_signal = Some(signal);
                }
                Err(error) => {
                    state.mode = ImporterMode::Loaded;
                    state.status_message = error.to_string();
                    logger.error(category::FILE_IO, &state.status_message);
                }
            }
        }
    });

    if export_signal.is_some() {
        return export_signal;
    }
    if cleared_importer {
        return None;
    }

    ui.separator();

    let row_snapshots: Vec<MeshRowSnapshot> = state
        .meshes
        .iter()
        .enumerate()
        .map(|(index, mesh)| MeshRowSnapshot {
            index,
            name: mesh.name.clone(),
            vertex_count: mesh.vertex_count,
            triangle_count: mesh.triangle_count,
            color: mesh.color,
            selected: mesh.selected,
            is_active: state.active_mesh_index == Some(index),
        })
        .collect();

    let mut pending_active_mesh = state.active_mesh_index;
    let mut pending_select_updates: Vec<(usize, bool)> = Vec::new();
    let mut pending_color_updates: Vec<(usize, [f32; 4])> = Vec::new();
    let mut pending_select_all = false;
    let mut pending_select_none = false;

    TwoColumnLayout::new("obj_importer_loaded_layout")
        .with_inspector_min_width(340.0)
        .show_split(
            ui,
            |left_ui| {
                left_ui.heading("Meshes");
                left_ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        pending_select_all = true;
                        ui.ctx().request_repaint();
                    }
                    if ui.button("Clear Selection").clicked() {
                        pending_select_none = true;
                        ui.ctx().request_repaint();
                    }
                });
                left_ui.separator();

                // Compute row height for show_rows virtualization.
                // Each row is a group containing two lines of content.
                let body_h = left_ui.text_style_height(&egui::TextStyle::Body);
                let spacing = left_ui.spacing().item_spacing.y;
                let row_height = body_h * 2.0 + spacing * 2.0 + 12.0;
                let num_rows = row_snapshots.len();
                egui::ScrollArea::vertical()
                    .id_salt("importer_mesh_list_scroll")
                    .show_rows(left_ui, row_height, num_rows, |ui, row_range| {
                        for i in row_range {
                            let row = &row_snapshots[i];
                            ui.push_id(row.index, |ui| {
                                let mut selected = row.selected;
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut selected, "").changed() {
                                            pending_select_updates.push((row.index, selected));
                                            ui.ctx().request_repaint();
                                        }

                                        if ui
                                            .selectable_label(row.is_active, row.name.as_str())
                                            .clicked()
                                        {
                                            pending_active_mesh = Some(row.index);
                                            ui.ctx().request_repaint();
                                        }

                                        render_color_swatch(ui, row.color, egui::vec2(18.0, 18.0));

                                        let mut row_color = row.color;
                                        let color_response =
                                            ui.color_edit_button_rgba_unmultiplied(&mut row_color);
                                        if color_response.changed() {
                                            pending_color_updates.push((row.index, row_color));
                                            pending_active_mesh = Some(row.index);
                                            ui.ctx().request_repaint();
                                        }
                                        color_response.on_hover_text("Edit mesh color");
                                    });
                                    ui.small(format!(
                                        "{} vertices, {} triangles",
                                        row.vertex_count, row.triangle_count
                                    ));
                                });
                            });
                        }
                    });
            },
            |right_ui| {
                render_color_editor(right_ui, state, campaign_dir, logger);
            },
        );

    if pending_select_all {
        for mesh in &mut state.meshes {
            mesh.selected = true;
        }
    }
    if pending_select_none {
        for mesh in &mut state.meshes {
            mesh.selected = false;
        }
    }
    for (index, selected) in pending_select_updates {
        if let Some(mesh) = state.meshes.get_mut(index) {
            mesh.selected = selected;
        }
    }
    for (index, color) in pending_color_updates {
        if let Some(mesh) = state.meshes.get_mut(index) {
            mesh.set_color(color);
            state.status_message = format!("Updated mesh color for {}.", mesh.name);
        }
    }
    if pending_active_mesh != state.active_mesh_index {
        state.set_active_mesh(pending_active_mesh);
    }

    None
}

fn render_color_editor(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    campaign_dir: Option<&PathBuf>,
    logger: &mut Logger,
) {
    ui.heading("Color Editor");
    ui.separator();

    let Some(active_index) = state.active_mesh_index else {
        ui.label("Select a mesh to edit its color.");
        return;
    };

    if active_index >= state.meshes.len() {
        ui.label("Select a mesh to edit its color.");
        return;
    }

    ui.label(format!("Active Mesh: {}", state.meshes[active_index].name));
    ui.small(format!(
        "{} vertices, {} triangles",
        state.meshes[active_index].vertex_count, state.meshes[active_index].triangle_count
    ));
    ui.small(describe_mesh_color_source(&state.meshes[active_index]));
    if let Some(texture_path) = state.meshes[active_index].mesh_def.texture_path.as_deref() {
        ui.small(format!("Texture: {}", texture_path));
    }
    let ctx = ui.ctx().clone();

    let mut active_color = state.meshes[active_index].color;
    if ui
        .color_edit_button_rgba_unmultiplied(&mut active_color)
        .changed()
    {
        state.meshes[active_index].set_color(active_color);
        state.status_message = format!(
            "Updated mesh color for {}.",
            state.meshes[active_index].name
        );
        ctx.request_repaint();
    }

    ui.horizontal(|ui| {
        if ui.button("Apply Active Color To Selected").clicked() {
            apply_color_to_selected(state, active_color);
            state.status_message = "Applied active mesh color to selected meshes.".to_string();
            ctx.request_repaint();
        }
        if ui.button("Only Select Active Mesh").clicked() {
            for (index, mesh) in state.meshes.iter_mut().enumerate() {
                mesh.selected = index == active_index;
            }
            ctx.request_repaint();
        }
    });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Imported MTL Palette").strong());
    render_imported_palette_section(ui, state, active_index);

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Built-In Palette").strong());
    render_palette_grid(ui, palette_entries(), |color| {
        state.meshes[active_index].set_color(color);
        state.status_message = format!(
            "Applied palette color to {}.",
            state.meshes[active_index].name
        );
        ctx.request_repaint();
    });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Custom Palette").strong());
    render_custom_palette_section(ui, state, campaign_dir, logger, active_index);

    ui.add_space(8.0);
    if ui.button("Auto-Assign All").clicked() {
        state.auto_assign_colors();
        state.status_message = "Reapplied automatic colors to all meshes.".to_string();
        ctx.request_repaint();
    }
}

fn render_imported_palette_section(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    active_index: usize,
) {
    if state.imported_material_palette.is_empty() {
        match state.active_mtl_source {
            ImportedMtlSourceKind::None => {
                ui.label("No imported MTL swatches available for this session.");
            }
            ImportedMtlSourceKind::AutoDetected | ImportedMtlSourceKind::ManualOverride => {
                ui.label("The current MTL data did not include diffuse Kd colors to surface as swatches.");
            }
        }
        return;
    }

    ui.small(
        "Session-only imported swatches. Apply one to the active mesh or stage it for the existing custom-palette save flow.",
    );

    let swatches = state.imported_material_palette.clone();
    egui::ScrollArea::vertical()
        .id_salt("obj_importer_imported_palette_scroll")
        .max_height(140.0)
        .show(ui, |ui| {
            for (index, swatch) in swatches.iter().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        let apply_clicked = ui
                            .add(
                                egui::Button::new("")
                                    .fill(color32(swatch.color))
                                    .min_size(egui::vec2(24.0, 24.0)),
                            )
                            .on_hover_text(imported_swatch_hover_text(swatch))
                            .clicked();
                        if apply_clicked {
                            state.meshes[active_index].set_color(swatch.color);
                            state.status_message = format!(
                                "Applied imported material '{}' to {}.",
                                swatch.label, state.meshes[active_index].name
                            );
                            ui.ctx().request_repaint();
                        }

                        ui.vertical(|ui| {
                            ui.label(&swatch.label);
                            if let Some(texture_path) = swatch.texture_path.as_deref() {
                                ui.small(format!("Texture: {}", texture_path));
                            }
                        });

                        if ui.button("Use As Draft").clicked() {
                            stage_imported_swatch_as_custom_draft(state, swatch);
                            ui.ctx().request_repaint();
                        }
                    });
                });
            }
        });
}

fn render_palette_grid(
    ui: &mut egui::Ui,
    entries: Vec<PaletteEntry>,
    mut apply_color: impl FnMut([f32; 4]),
) {
    egui::Grid::new("obj_importer_builtin_palette_grid")
        .num_columns(6)
        .spacing([6.0, 6.0])
        .show(ui, |ui| {
            for (index, entry) in entries.iter().enumerate() {
                ui.push_id(index, |ui| {
                    let response = ui
                        .add(
                            egui::Button::new("")
                                .fill(color32(entry.color))
                                .min_size(egui::vec2(24.0, 24.0)),
                        )
                        .on_hover_text(entry.label);
                    if response.clicked() {
                        apply_color(entry.color);
                    }
                });
                if (index + 1) % 6 == 0 {
                    ui.end_row();
                }
            }
        });
}

fn render_custom_palette_section(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    campaign_dir: Option<&PathBuf>,
    logger: &mut Logger,
    active_index: usize,
) {
    if state.custom_palette.colors.is_empty() {
        ui.label("No custom colors saved for this campaign yet.");
    } else {
        let mut remove_label: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("obj_importer_custom_palette_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                for (index, (label, color)) in
                    state.custom_palette.colors.clone().iter().enumerate()
                {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            let apply_clicked = ui
                                .add(
                                    egui::Button::new("")
                                        .fill(color32(*color))
                                        .min_size(egui::vec2(24.0, 24.0)),
                                )
                                .on_hover_text(label)
                                .clicked();
                            if apply_clicked {
                                state.meshes[active_index].set_color(*color);
                                state.status_message = format!(
                                    "Applied custom color '{}' to {}.",
                                    label, state.meshes[active_index].name
                                );
                                ui.ctx().request_repaint();
                            }
                            ui.label(label);
                            if ui.button("Remove").clicked() {
                                remove_label = Some(label.clone());
                            }
                        });
                    });
                }
            });

        if let Some(label) = remove_label {
            if state.remove_custom_color(&label)
                && persist_custom_palette(state, campaign_dir, logger)
            {
                state.status_message = format!("Removed custom color '{}'.", label);
                ui.ctx().request_repaint();
            }
        }
    }

    ui.separator();
    ui.label("Add Current Draft As Custom Color");
    ui.horizontal(|ui| {
        ui.label("Label:");
        if ui
            .text_edit_singleline(&mut state.new_custom_color_label)
            .changed()
        {
            ui.ctx().request_repaint();
        }
    });
    if ui
        .color_edit_button_rgba_unmultiplied(&mut state.new_custom_color)
        .changed()
    {
        ui.ctx().request_repaint();
    }

    ui.horizontal(|ui| {
        if ui.button("Use Active Mesh Color").clicked() {
            state.new_custom_color = state.meshes[active_index].color;
            ui.ctx().request_repaint();
        }

        if ui.button("Add Custom Color").clicked() {
            let label = state.new_custom_color_label.trim().to_string();
            if label.is_empty() {
                state.status_message = "Enter a label before saving a custom color.".to_string();
            } else {
                state.add_custom_color(label, state.new_custom_color);
                state.new_custom_color_label.clear();
                if persist_custom_palette(state, campaign_dir, logger) {
                    state.status_message = "Saved custom importer color.".to_string();
                    ui.ctx().request_repaint();
                }
            }
        }
    });
}

fn persist_custom_palette(
    state: &mut ObjImporterState,
    campaign_dir: Option<&PathBuf>,
    logger: &mut Logger,
) -> bool {
    let Some(campaign_dir) = campaign_dir else {
        state.status_message =
            "Open a campaign before saving custom importer palette colors.".to_string();
        return false;
    };

    if let Err(error) = state.save_custom_palette(campaign_dir) {
        state.status_message = format!("Failed to save custom palette: {}", error);
        logger.error(category::FILE_IO, &state.status_message);
        false
    } else {
        true
    }
}

/// Loads a model file (OBJ or GLB) into importer state, dispatching on file extension.
///
/// # Errors
///
/// Returns [`ObjImportError::LoadFailed`] when parsing fails.
/// Returns [`ObjImportError::UnknownFormat`] when the extension is not `.obj` or `.glb`.
fn load_model_into_state(state: &mut ObjImporterState, path: &Path) -> Result<(), ObjImportError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    match ext.as_deref() {
        Some("obj") => {
            state
                .load_obj_file(path)
                .map_err(|error| ObjImportError::LoadFailed {
                    path: path.display().to_string(),
                    message: error.to_string(),
                })?;
            if state.creature_name.trim().is_empty() {
                state.creature_name = display_name_from_path(path);
            }
            state.set_active_mesh((!state.meshes.is_empty()).then_some(0));
            state.status_message = format!(
                "Loaded {} mesh(es) from {}",
                state.meshes.len(),
                path.display()
            );
        }
        Some("glb") => {
            state
                .load_glb_file(path)
                .map_err(|error| ObjImportError::LoadFailed {
                    path: path.display().to_string(),
                    message: error.to_string(),
                })?;
            if state.creature_name.trim().is_empty() {
                state.creature_name = display_name_from_path(path);
            }
            state.set_active_mesh((!state.meshes.is_empty()).then_some(0));
            // GLB sets its own rich status_message via load_imported_glb_scene.
        }
        _ => {
            return Err(ObjImportError::UnknownFormat {
                extension: path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    Ok(())
}

fn render_mtl_source_controls(
    ui: &mut egui::Ui,
    state: &mut ObjImporterState,
    logger: &mut Logger,
    reload_on_change: bool,
) {
    ui.vertical(|ui| {
        ui.monospace(format_mtl_source_summary(state));

        if let Some(detail) = format_mtl_source_detail(state) {
            ui.small(detail);
        }

        ui.horizontal(|ui| {
            if ui.button("Browse .mtl...").clicked() {
                if let Some(path) = pick_mtl_file(preferred_mtl_dialog_path(state)) {
                    state.manual_mtl_path = Some(path.clone());
                    if reload_on_change {
                        reload_obj_after_mtl_change(state, logger, "manual MTL override", path);
                    } else {
                        state.status_message = format!(
                            "Selected manual MTL override for next import: {}",
                            path.display()
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }

            if ui
                .add_enabled(
                    state.manual_mtl_path.is_some(),
                    egui::Button::new("Clear Override"),
                )
                .clicked()
            {
                state.manual_mtl_path = None;
                if reload_on_change {
                    if let Some(source_path) = state.source_path.clone() {
                        reload_obj_after_mtl_change(
                            state,
                            logger,
                            "auto-detected MTL",
                            source_path,
                        );
                    } else {
                        state.status_message = "Cleared manual MTL override.".to_string();
                        ui.ctx().request_repaint();
                    }
                } else {
                    state.status_message = "Cleared manual MTL override.".to_string();
                    ui.ctx().request_repaint();
                }
            }
        });
    });
}

fn preferred_mtl_dialog_path(state: &ObjImporterState) -> Option<&Path> {
    state
        .manual_mtl_path
        .as_deref()
        .or(state.source_path.as_deref())
}

fn reload_obj_after_mtl_change(
    state: &mut ObjImporterState,
    logger: &mut Logger,
    action_label: &str,
    status_path: PathBuf,
) {
    let Some(source_path) = state.source_path.clone() else {
        state.status_message = format!(
            "Selected {} at {}. Load an OBJ file to apply it.",
            action_label,
            status_path.display()
        );
        return;
    };

    match load_model_into_state(state, &source_path) {
        Ok(()) => {
            state.status_message = format!(
                "Reloaded {} using {}.",
                source_path.display(),
                short_mtl_source_label(state)
            );
            logger.info(
                category::EDITOR,
                &format!(
                    "Reloaded importer OBJ {} after updating MTL settings",
                    source_path.display()
                ),
            );
        }
        Err(error) => {
            state.status_message = error.to_string();
            logger.error(category::FILE_IO, &state.status_message);
        }
    }
}

fn format_mtl_source_summary(state: &ObjImporterState) -> String {
    match state.active_mtl_source {
        ImportedMtlSourceKind::ManualOverride => state
            .manual_mtl_path
            .as_ref()
            .map(|path| format!("Manual override: {}", path.display()))
            .unwrap_or_else(|| "Manual override selected".to_string()),
        ImportedMtlSourceKind::AutoDetected => match state.resolved_mtl_paths.as_slice() {
            [] => "Auto-detect ready on next load".to_string(),
            [path] => format!("Auto-detected: {}", path.display()),
            paths => format!("Auto-detected {} material libraries", paths.len()),
        },
        ImportedMtlSourceKind::None => state
            .manual_mtl_path
            .as_ref()
            .map(|path| format!("Manual override queued: {}", path.display()))
            .or_else(|| {
                (!state.declared_mtl_libraries.is_empty())
                    .then(|| "OBJ declares MTL libraries, but none were resolved".to_string())
            })
            .unwrap_or_else(|| "No MTL file in use".to_string()),
    }
}

fn format_mtl_source_detail(state: &ObjImporterState) -> Option<String> {
    if state.resolved_mtl_paths.len() > 1 {
        Some(
            state
                .resolved_mtl_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    } else if !state.declared_mtl_libraries.is_empty() && state.resolved_mtl_paths.is_empty() {
        Some(format!(
            "Declared by OBJ: {}",
            state.declared_mtl_libraries.join(", ")
        ))
    } else if matches!(state.active_mtl_source, ImportedMtlSourceKind::None)
        && state.source_path.is_some()
    {
        Some("Browse a .mtl file to override missing or incorrect mtllib detection.".to_string())
    } else {
        None
    }
}

fn short_mtl_source_label(state: &ObjImporterState) -> &'static str {
    match state.active_mtl_source {
        ImportedMtlSourceKind::ManualOverride => "a manual MTL override",
        ImportedMtlSourceKind::AutoDetected => "auto-detected MTL data",
        ImportedMtlSourceKind::None => "OBJ geometry without MTL data",
    }
}

fn describe_mesh_color_source(mesh: &crate::obj_importer::ImportedMesh) -> &'static str {
    match mesh.color_source {
        ImportedMeshColorSource::ImportedMaterial => {
            "Color Source: imported from MTL diffuse color (Kd)."
        }
        ImportedMeshColorSource::AutoAssigned => {
            "Color Source: built-in mesh-name fallback; imported alpha and other material metadata may still be preserved."
        }
        ImportedMeshColorSource::ManualOverride => {
            "Color Source: manual importer override from the color picker or palette."
        }
    }
}

fn imported_swatch_hover_text(swatch: &ImportedMaterialSwatch) -> String {
    swatch
        .texture_path
        .as_deref()
        .map(|texture_path| format!("{}\nTexture: {}", swatch.label, texture_path))
        .unwrap_or_else(|| swatch.label.clone())
}

fn stage_imported_swatch_as_custom_draft(
    state: &mut ObjImporterState,
    swatch: &ImportedMaterialSwatch,
) {
    state.new_custom_color_label = swatch.label.clone();
    state.new_custom_color = swatch.color;
    state.status_message = format!(
        "Staged imported material '{}' for custom palette saving.",
        swatch.label
    );
}

/// Opens a file-picker dialog that accepts both Wavefront OBJ and binary glTF (GLB) files.
fn pick_model_file(initial_path: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("Wavefront OBJ", &["obj"])
        .add_filter("Binary glTF", &["glb"]);
    if let Some(path) = initial_path {
        let directory = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };
        dialog = dialog.set_directory(directory);
    }
    dialog.pick_file()
}

fn pick_mtl_file(initial_path: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter("Wavefront MTL", &["mtl"]);
    if let Some(path) = initial_path {
        let directory = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };
        dialog = dialog.set_directory(directory);
    }
    dialog.pick_file()
}

fn display_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace(['_', '-'], " "))
        .unwrap_or_else(|| "Imported Asset".to_string())
}

fn color32(color: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (color[3].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn render_color_swatch(ui: &mut egui::Ui, color: [f32; 4], size: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color32(color));
}

fn apply_color_to_selected(state: &mut ObjImporterState, color: [f32; 4]) {
    for mesh in &mut state.meshes {
        if mesh.selected {
            mesh.set_color(color);
        }
    }
}

fn build_creature_definition(
    state: &ObjImporterState,
) -> Result<CreatureDefinition, ObjImporterExportError> {
    if state.meshes.is_empty() {
        return Err(ObjImporterExportError::NoMeshesLoaded);
    }

    let name = state.creature_name.trim();
    if name.is_empty() {
        return Err(ObjImporterExportError::MissingName);
    }

    let meshes = state
        .meshes
        .iter()
        .map(|mesh| mesh.mesh_def.clone())
        .collect::<Vec<_>>();

    Ok(CreatureDefinition {
        id: state.creature_id,
        name: name.to_string(),
        mesh_transforms: vec![MeshTransform::identity(); meshes.len()],
        meshes,
        scale: state.scale,
        color_tint: None,
    })
}

fn export_state_to_campaign(
    state: &ObjImporterState,
    campaign_dir: Option<&Path>,
) -> Result<ExportOutcome, ObjImporterExportError> {
    let campaign_dir = campaign_dir.ok_or(ObjImporterExportError::MissingCampaignDir)?;
    let mut creature = build_creature_definition(state)?;
    if matches!(state.export_type, ExportType::Furniture) {
        creature.id = state.furniture_id;
    }
    let relative_path = preview_export_relative_path(
        state.export_type,
        &creature.name,
        creature.id,
        &state.category,
    );
    copy_imported_textures_into_campaign(state, &mut creature, campaign_dir, &relative_path)?;
    let absolute_path = campaign_dir.join(&relative_path);

    match state.export_type {
        ExportType::Creature => {
            let manager = CreatureAssetManager::new(campaign_dir.to_path_buf());
            manager.save_creature_at_path(&relative_path, &creature)?;
        }
        ExportType::Item => {
            if let Some(parent) = absolute_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = ron::ser::to_string_pretty(&creature, ron::ser::PrettyConfig::new())?;
            fs::write(&absolute_path, contents)?;
        }
        ExportType::Furniture => {
            if let Some(parent) = absolute_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = ron::ser::to_string_pretty(&creature, ron::ser::PrettyConfig::new())?;
            fs::write(&absolute_path, contents)?;
        }
    }

    Ok(ExportOutcome {
        export_type: state.export_type,
        absolute_path: absolute_path.clone(),
        status_message: format!(
            "Exported {} '{}' to {}",
            export_type_label(state.export_type),
            creature.name,
            absolute_path.display()
        ),
    })
}

/// Describes how a mesh texture will be obtained during campaign export.
///
/// Returned by [`resolve_imported_texture_source`] and consumed by
/// [`copy_imported_textures_into_campaign`] to determine which write strategy
/// to apply without mixing resolution logic into the copy loop.
#[derive(Debug)]
enum ResolvedTextureSource {
    /// The texture path already points inside the campaign directory; no copy needed.
    AlreadyCampaignRelative,
    /// Copy (or write) from this filesystem path into the campaign texture directory.
    FilesystemPath(PathBuf),
    /// Write these raw bytes to a new destination file.
    EmbeddedBytes {
        /// Raw image bytes extracted from a GLB buffer view.
        bytes: Vec<u8>,
        /// Preferred export filename including extension (e.g. `"albedo_0.png"`).
        /// May be empty; [`embedded_texture_file_name`] handles the fallback chain.
        file_name_hint: String,
    },
    /// No valid source was found; export must fail with `MissingTexture`.
    Missing,
}

/// Writes campaign texture files for every mesh that declares a texture path,
/// then rewrites `mesh_def.texture_path` to the new campaign-relative destination.
///
/// Textures are deduplicated by content hash: two mesh payloads whose bytes are
/// identical will both point at the same destination file without writing it twice.
///
/// # Errors
///
/// Returns [`ObjImporterExportError::MissingTexture`] when a mesh has a texture
/// path but no usable source (no embedded bytes, no resolvable filesystem path).
/// This error fires **before** any RON write.
fn copy_imported_textures_into_campaign(
    state: &ObjImporterState,
    creature: &mut CreatureDefinition,
    campaign_dir: &Path,
    exported_asset_relative_path: &str,
) -> Result<(), ObjImporterExportError> {
    let export_stem = Path::new(exported_asset_relative_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("imported_model");
    let texture_dir = PathBuf::from("assets/textures/imported").join(export_stem);
    // Keyed on a 64-bit hash of the file/byte content to deduplicate identical payloads.
    let mut content_hash_to_dest: HashMap<u64, String> = HashMap::new();
    let mut used_destinations: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (mesh_index, mesh_def) in creature.meshes.iter_mut().enumerate() {
        let Some(texture_path) = mesh_def.texture_path.clone() else {
            continue;
        };

        let mesh_name = mesh_def
            .name
            .clone()
            .unwrap_or_else(|| format!("mesh_{}", mesh_index));

        match resolve_imported_texture_source(state, mesh_index, &texture_path, campaign_dir) {
            ResolvedTextureSource::AlreadyCampaignRelative => {
                // Reserve the existing path so no other mesh can collide with it.
                used_destinations.insert(texture_path);
            }
            ResolvedTextureSource::FilesystemPath(source_path) => {
                // Read once for both hashing and writing.
                let content = fs::read(&source_path)?;
                let hash = compute_content_hash(&content);
                if let Some(existing_dest) = content_hash_to_dest.get(&hash) {
                    mesh_def.texture_path = Some(existing_dest.clone());
                } else {
                    let dest_relative = unique_texture_destination(
                        &texture_dir,
                        &source_path,
                        &mut used_destinations,
                    )?;
                    let dest_absolute = campaign_dir.join(&dest_relative);
                    if let Some(parent) = dest_absolute.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_absolute, &content)?;
                    let dest_str = dest_relative.to_string_lossy().replace('\\', "/");
                    content_hash_to_dest.insert(hash, dest_str.clone());
                    mesh_def.texture_path = Some(dest_str);
                }
            }
            ResolvedTextureSource::EmbeddedBytes {
                bytes,
                file_name_hint,
            } => {
                let hash = compute_content_hash(&bytes);
                if let Some(existing_dest) = content_hash_to_dest.get(&hash) {
                    mesh_def.texture_path = Some(existing_dest.clone());
                } else {
                    // Use the enum-captured hint as the primary naming source;
                    // fall back to the full payload when the hint is empty
                    // (source_label + MIME extension).
                    let hint = if !file_name_hint.is_empty() {
                        file_name_hint
                    } else {
                        let payload = state
                            .meshes
                            .get(mesh_index)
                            .and_then(|m| m.texture_payload.as_ref());
                        embedded_texture_file_name(payload, mesh_index)
                    };
                    let dest_relative = unique_texture_destination_by_hint(
                        &texture_dir,
                        &hint,
                        &mut used_destinations,
                    );
                    let dest_absolute = campaign_dir.join(&dest_relative);
                    if let Some(parent) = dest_absolute.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_absolute, &bytes)?;
                    let dest_str = dest_relative.to_string_lossy().replace('\\', "/");
                    content_hash_to_dest.insert(hash, dest_str.clone());
                    mesh_def.texture_path = Some(dest_str);
                }
            }
            ResolvedTextureSource::Missing => {
                return Err(ObjImporterExportError::MissingTexture {
                    mesh_name,
                    path: PathBuf::from(&texture_path),
                });
            }
        }
    }

    Ok(())
}

/// Resolves the texture source for a single mesh during campaign export.
///
/// Checks in priority order:
/// 1. If the texture path is already campaign-relative, returns
///    [`ResolvedTextureSource::AlreadyCampaignRelative`].
/// 2. If the mesh payload carries embedded bytes, returns
///    [`ResolvedTextureSource::EmbeddedBytes`].
/// 3. If the mesh payload or fallback paths point at an existing file, returns
///    [`ResolvedTextureSource::FilesystemPath`].
/// 4. Returns [`ResolvedTextureSource::Missing`] when no usable source exists.
fn resolve_imported_texture_source(
    state: &ObjImporterState,
    mesh_index: usize,
    texture_path: &str,
    campaign_dir: &Path,
) -> ResolvedTextureSource {
    // 1. Already inside the campaign directory — nothing to copy.
    if campaign_dir.join(texture_path).exists() {
        return ResolvedTextureSource::AlreadyCampaignRelative;
    }

    // 2. Embedded GLB bytes take priority over filesystem paths.
    if let Some(payload) = state
        .meshes
        .get(mesh_index)
        .and_then(|mesh| mesh.texture_payload.as_ref())
    {
        if let Some(bytes) = payload.bytes.clone() {
            return ResolvedTextureSource::EmbeddedBytes {
                bytes,
                file_name_hint: payload.file_name_hint.clone(),
            };
        }

        // 3a. Payload carries an OBJ filesystem source path.
        if let Some(source_path) = payload.source_path.as_ref().filter(|p| p.exists()) {
            return ResolvedTextureSource::FilesystemPath(source_path.clone());
        }
    }

    // 3b. Absolute path encoded directly in the texture_path string.
    let texture_path_ref = Path::new(texture_path);
    if texture_path_ref.is_absolute() && texture_path_ref.exists() {
        return ResolvedTextureSource::FilesystemPath(texture_path_ref.to_path_buf());
    }

    // 3c. Paths relative to MTL file directories.
    for mtl_path in &state.resolved_mtl_paths {
        if let Some(parent) = mtl_path.parent() {
            let candidate = parent.join(texture_path_ref);
            if candidate.exists() {
                return ResolvedTextureSource::FilesystemPath(candidate);
            }
        }
    }

    // 3d. Path relative to the source OBJ/GLB file's parent directory.
    if let Some(parent) = state.source_path.as_ref().and_then(|p| p.parent()) {
        let candidate = parent.join(texture_path_ref);
        if candidate.exists() {
            return ResolvedTextureSource::FilesystemPath(candidate);
        }
    }

    ResolvedTextureSource::Missing
}

/// Finds a unique campaign-relative destination path for a texture file, given
/// its source path. Delegates filename derivation to
/// [`unique_texture_destination_by_hint`] after extracting the filename component.
///
/// # Errors
///
/// Returns [`ObjImporterExportError::TextureMissingFileName`] when `source_path`
/// has no filename component (e.g. a bare root path).
fn unique_texture_destination(
    texture_dir: &Path,
    source_path: &Path,
    used_destinations: &mut std::collections::HashSet<String>,
) -> Result<PathBuf, ObjImporterExportError> {
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ObjImporterExportError::TextureMissingFileName(source_path.to_path_buf()))?;
    Ok(unique_texture_destination_by_hint(
        texture_dir,
        file_name,
        used_destinations,
    ))
}

/// Finds a unique campaign-relative destination path for a texture file given a
/// filename hint string (stem + optional extension).
///
/// Sanitizes the stem with [`sanitized_texture_stem`], lowercases the extension,
/// and appends a numeric suffix (`_2`, `_3`, …) until a name not present in
/// `used_destinations` is found.
///
/// The returned path is inserted into `used_destinations` before returning.
fn unique_texture_destination_by_hint(
    texture_dir: &Path,
    file_name_hint: &str,
    used_destinations: &mut std::collections::HashSet<String>,
) -> PathBuf {
    let hint_path = Path::new(file_name_hint);
    let stem = hint_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("texture");
    let extension = hint_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    let sanitized_stem = sanitized_texture_stem(stem);

    for suffix in 0.. {
        let candidate_name = if suffix == 0 {
            format_texture_file_name(&sanitized_stem, extension.as_deref())
        } else {
            format_texture_file_name(
                &format!("{}_{}", sanitized_stem, suffix + 1),
                extension.as_deref(),
            )
        };
        let candidate = texture_dir.join(candidate_name);
        let candidate_string = candidate.to_string_lossy().replace('\\', "/");
        if used_destinations.insert(candidate_string.clone()) {
            return PathBuf::from(candidate_string);
        }
    }

    unreachable!("unbounded suffix loop must return a unique texture destination")
}

fn sanitized_texture_stem(stem: &str) -> String {
    let sanitized = stem
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let mut collapsed = sanitized;
    while collapsed.contains("__") {
        collapsed = collapsed.replace("__", "_");
    }
    let collapsed = collapsed.trim_matches('_').to_string();
    if collapsed.is_empty() {
        "texture".to_string()
    } else {
        collapsed
    }
}

fn format_texture_file_name(stem: &str, extension: Option<&str>) -> String {
    extension
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!("{}.{}", stem, extension))
        .unwrap_or_else(|| stem.to_string())
}

/// Computes a 64-bit content hash for deduplication of texture payloads.
///
/// Two payloads with identical bytes produce the same hash and will be mapped
/// to the same destination file by [`copy_imported_textures_into_campaign`].
fn compute_content_hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Maps a MIME type string to a lowercase file extension.
///
/// | MIME           | Extension |
/// |----------------|-----------|
/// | `image/png`    | `png`     |
/// | `image/jpeg`   | `jpg`     |
/// | anything else  | `bin`     |
fn mime_to_extension(mime: Option<&str>) -> &'static str {
    match mime {
        Some("image/png") => "png",
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        _ => "bin",
    }
}

/// Derives a sanitized export filename (with extension) for a GLB embedded
/// texture, applying the following priority chain:
///
/// 1. `payload.file_name_hint` if non-empty — sanitized stem, lowercased extension.
/// 2. `payload.source_label` if non-empty — sanitized stem, extension from MIME.
/// 3. Fallback: `"texture_{mesh_index}"` plus extension from MIME (or `.bin`).
fn embedded_texture_file_name(
    payload: Option<&ImportedTexturePayload>,
    mesh_index: usize,
) -> String {
    let Some(p) = payload else {
        return format!("texture_{}.bin", mesh_index);
    };

    // 1. Prefer the pre-computed file_name_hint (already includes extension).
    if !p.file_name_hint.is_empty() {
        let hint_path = Path::new(&p.file_name_hint);
        let stem = hint_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("texture");
        let ext = hint_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        return format_texture_file_name(&sanitized_texture_stem(stem), ext.as_deref());
    }

    // 2. Fall back to source_label + MIME-derived extension.
    let ext = mime_to_extension(p.mime_type.as_deref());
    if !p.source_label.is_empty() {
        return format!("{}.{}", sanitized_texture_stem(&p.source_label), ext);
    }

    // 3. Last resort: positional name.
    format!("texture_{}.{}", mesh_index, ext)
}

fn preview_export_relative_path(
    export_type: ExportType,
    name: &str,
    id: u32,
    category: &str,
) -> String {
    let file_stem = sanitized_export_stem(name, id, export_type);
    let category_path = sanitize_category_path(category);

    match export_type {
        ExportType::Creature => format!("assets/creatures/{}.ron", file_stem),
        ExportType::Item => {
            if category_path.is_empty() {
                format!("assets/items/{}.ron", file_stem)
            } else {
                format!("assets/items/{}/{}.ron", category_path, file_stem)
            }
        }
        ExportType::Furniture => {
            if category_path.is_empty() {
                format!("assets/furniture/{}.ron", file_stem)
            } else {
                format!("assets/furniture/{}/{}.ron", category_path, file_stem)
            }
        }
    }
}

fn sanitized_export_stem(name: &str, id: u32, export_type: ExportType) -> String {
    let mut sanitized = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();

    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }

    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        match export_type {
            ExportType::Creature => format!("creature_{}", id),
            ExportType::Item => format!("item_{}", id),
            ExportType::Furniture => format!("furniture_{}", id),
        }
    } else {
        sanitized
    }
}

fn sanitize_category_path(category: &str) -> String {
    category
        .split('/')
        .map(|segment| {
            let mut sanitized = segment
                .trim()
                .to_ascii_lowercase()
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect::<String>();

            while sanitized.contains("__") {
                sanitized = sanitized.replace("__", "_");
            }

            sanitized.trim_matches('_').to_string()
        })
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn export_type_label(export_type: ExportType) -> &'static str {
    match export_type {
        ExportType::Creature => "creature",
        ExportType::Item => "item",
        ExportType::Furniture => "furniture",
    }
}

/// Extracts the embedded image count from a GLB status message.
///
/// The status message format produced by `load_imported_glb_scene` is:
/// `"GLB: N mesh(es), M embedded image(s), K material(s)..."`.  This helper
/// finds the token immediately before the literal `" embedded image"` substring
/// and parses it as a [`u32`].
///
/// Returns `None` when the substring is absent or the preceding token is not a
/// valid integer.
fn parse_glb_embedded_image_count(status: &str) -> Option<u32> {
    let pos = status.find(" embedded image")?;
    let before = &status[..pos];
    before.split_whitespace().last()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{
        build_creature_definition, describe_mesh_color_source, export_state_to_campaign,
        format_mtl_source_detail, format_mtl_source_summary, load_model_into_state,
        persist_custom_palette, preview_export_relative_path, show_obj_importer_tab,
        stage_imported_swatch_as_custom_draft, ObjImportError, ObjImporterExportError,
    };
    use crate::logging::Logger;
    use crate::obj_importer::{
        ExportType, ImportSourceFormat, ImportedMaterialSwatch, ImportedMeshColorSource,
        ImportedMtlSourceKind, ImportedTexturePayload, ImporterMode, ObjImporterState,
    };
    use antares::domain::visual::{AlphaMode, CreatureDefinition, MaterialDefinition};
    use eframe::egui;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn fixture_path(relative_path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(relative_path)
    }

    fn triangle_mesh_state() -> ObjImporterState {
        let mut state = ObjImporterState::new();
        state.creature_id = 4123;
        state.creature_name = "Test Import".to_string();
        state.load_mesh_definitions(
            None,
            vec![antares::domain::visual::MeshDefinition {
                name: Some("body".to_string()),
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            }],
        );
        state
    }

    #[test]
    fn test_importer_state_loads_skeleton_fixture_into_loaded_mode() {
        let path = fixture_path("data/test_fixtures/skeleton.obj");
        let mut state = ObjImporterState::new();

        assert_eq!(state.mode, ImporterMode::Idle);
        state.load_obj_file(&path).unwrap();

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert!(!state.meshes.is_empty());
    }

    #[test]
    fn test_build_creature_definition_uses_updated_mesh_colors() {
        let mut state = triangle_mesh_state();
        state.meshes[0].set_color([0.25, 0.5, 0.75, 1.0]);

        let creature = build_creature_definition(&state).unwrap();

        assert_eq!(creature.meshes[0].color, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(creature.mesh_transforms.len(), creature.meshes.len());
    }

    #[test]
    fn test_build_creature_definition_updates_material_base_color_from_importer_state() {
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.material = Some(MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.2,
            roughness: 0.7,
            emissive: None,
            alpha_mode: AlphaMode::Opaque,
        });
        state.meshes[0].set_color([0.25, 0.5, 0.75, 0.5]);

        let creature = build_creature_definition(&state).unwrap();
        let material = creature.meshes[0].material.as_ref().unwrap();

        assert_eq!(material.base_color, [0.25, 0.5, 0.75, 0.5]);
        assert_eq!(material.alpha_mode, AlphaMode::Blend);
    }

    #[test]
    fn test_export_creature_writes_valid_ron_and_registry_entry() {
        let temp_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.meshes[0].name = "body_piece".to_string();
        state.meshes[0].mesh_def.name = Some("body_piece".to_string());
        state.meshes[0].set_color([0.9, 0.2, 0.3, 1.0]);

        let outcome = export_state_to_campaign(&state, Some(temp_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        assert_eq!(
            outcome.absolute_path,
            temp_dir.path().join("assets/creatures/test_import.ron")
        );
        assert_eq!(round_trip.name, "Test Import");
        assert_eq!(round_trip.meshes.len(), 1);
        assert_eq!(round_trip.meshes[0].name.as_deref(), Some("body_piece"));
        assert_eq!(round_trip.meshes[0].color, [0.9, 0.2, 0.3, 1.0]);
        assert!(temp_dir.path().join("data/creatures.ron").exists());
    }

    #[test]
    fn test_export_item_writes_ron_to_items_directory() {
        let temp_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.export_type = ExportType::Item;
        state.creature_name = "Bronze Dagger".to_string();

        let outcome = export_state_to_campaign(&state, Some(temp_dir.path())).unwrap();

        assert_eq!(
            outcome.absolute_path,
            temp_dir.path().join("assets/items/bronze_dagger.ron")
        );
        assert!(outcome.absolute_path.exists());
    }

    #[test]
    fn test_export_creature_copies_imported_texture_maps_and_updates_paths() {
        let campaign_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let body_texture = source_dir.path().join("Body Texture.PNG");
        let wing_texture = source_dir.path().join("wing-diffuse.jpg");
        fs::write(&body_texture, b"body texture").unwrap();
        fs::write(&wing_texture, b"wing texture").unwrap();

        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("textures/body.png".to_string());
        state.meshes[0].texture_payload = Some(ImportedTexturePayload {
            source_label: "Body Texture.PNG".to_string(),
            file_name_hint: "Body Texture.PNG".to_string(),
            bytes: None,
            source_path: Some(body_texture.clone()),
            mime_type: None,
        });
        let mut second_mesh = state.meshes[0].clone();
        second_mesh.name = "wing".to_string();
        second_mesh.mesh_def.name = Some("wing".to_string());
        second_mesh.mesh_def.texture_path = Some("textures/wing.jpg".to_string());
        second_mesh.texture_payload = Some(ImportedTexturePayload {
            source_label: "wing-diffuse.jpg".to_string(),
            file_name_hint: "wing-diffuse.jpg".to_string(),
            bytes: None,
            source_path: Some(wing_texture.clone()),
            mime_type: None,
        });
        state.meshes.push(second_mesh);

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let body_path = round_trip.meshes[0].texture_path.as_ref().unwrap();
        let wing_path = round_trip.meshes[1].texture_path.as_ref().unwrap();
        assert!(body_path.starts_with("assets/textures/imported/test_import/"));
        assert!(wing_path.starts_with("assets/textures/imported/test_import/"));
        assert_ne!(body_path, wing_path);
        assert!(campaign_dir.path().join(body_path).exists());
        assert!(campaign_dir.path().join(wing_path).exists());
    }

    #[test]
    fn test_export_creature_errors_when_imported_texture_is_missing() {
        let campaign_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("textures/missing.png".to_string());
        state.meshes[0].texture_payload = Some(ImportedTexturePayload {
            source_label: "missing.png".to_string(),
            file_name_hint: "missing.png".to_string(),
            bytes: None,
            source_path: Some(campaign_dir.path().join("missing.png")),
            mime_type: None,
        });

        let error = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap_err();
        assert!(matches!(
            error,
            ObjImporterExportError::MissingTexture { .. }
        ));
    }

    #[test]
    fn test_preview_export_relative_path_uses_expected_directories() {
        assert_eq!(
            preview_export_relative_path(ExportType::Creature, "Stone Golem", 44, ""),
            "assets/creatures/stone_golem.ron"
        );
        assert_eq!(
            preview_export_relative_path(ExportType::Item, "", 9, ""),
            "assets/items/item_9.ron"
        );
        assert_eq!(
            preview_export_relative_path(ExportType::Item, "Sword", 9, "Weapons"),
            "assets/items/weapons/sword.ron"
        );
        assert_eq!(
            preview_export_relative_path(ExportType::Furniture, "Oak Table", 10001, "Tables"),
            "assets/furniture/tables/oak_table.ron"
        );
    }

    #[test]
    fn test_format_mtl_source_summary_reports_manual_override() {
        let mut state = ObjImporterState::new();
        state.manual_mtl_path = Some(PathBuf::from("materials/hero_override.mtl"));
        state.active_mtl_source = ImportedMtlSourceKind::ManualOverride;

        assert_eq!(
            format_mtl_source_summary(&state),
            "Manual override: materials/hero_override.mtl"
        );
    }

    #[test]
    fn test_format_mtl_source_detail_reports_declared_but_missing_libraries() {
        let mut state = ObjImporterState::new();
        state.source_path = Some(PathBuf::from("models/hero.obj"));
        state.declared_mtl_libraries = vec!["hero.mtl".to_string(), "shared.mtl".to_string()];

        assert_eq!(
            format_mtl_source_detail(&state).as_deref(),
            Some("Declared by OBJ: hero.mtl, shared.mtl")
        );
    }

    #[test]
    fn test_describe_mesh_color_source_covers_phase_six_messages() {
        let mut state = triangle_mesh_state();
        state.meshes[0].color_source = ImportedMeshColorSource::ImportedMaterial;
        assert!(describe_mesh_color_source(&state.meshes[0]).contains("MTL diffuse color"));

        state.meshes[0].color_source = ImportedMeshColorSource::AutoAssigned;
        assert!(describe_mesh_color_source(&state.meshes[0]).contains("mesh-name fallback"));

        state.meshes[0].color_source = ImportedMeshColorSource::ManualOverride;
        assert!(describe_mesh_color_source(&state.meshes[0]).contains("manual importer override"));
    }

    #[test]
    fn test_stage_imported_swatch_as_custom_draft_copies_label_and_color() {
        let mut state = triangle_mesh_state();
        let swatch = ImportedMaterialSwatch {
            label: "HeroSkin".to_string(),
            color: [0.7, 0.6, 0.5, 1.0],
            texture_path: Some("textures/hero.png".to_string()),
            texture_source_path: None,
        };

        stage_imported_swatch_as_custom_draft(&mut state, &swatch);

        assert_eq!(state.new_custom_color_label, "HeroSkin");
        assert_eq!(state.new_custom_color, [0.7, 0.6, 0.5, 1.0]);
    }

    #[test]
    fn test_persist_custom_palette_writes_importer_palette_file() {
        let temp_dir = tempdir().unwrap();
        let campaign_dir = temp_dir.path().to_path_buf();
        let mut state = triangle_mesh_state();
        let mut logger = Logger::default();
        state.add_custom_color("HeroSkin", [0.7, 0.6, 0.5, 1.0]);

        assert!(persist_custom_palette(
            &mut state,
            Some(&campaign_dir),
            &mut logger,
        ));

        let palette_path = campaign_dir.join("config/importer_palette.ron");
        assert!(palette_path.exists());
        let saved = fs::read_to_string(palette_path).unwrap();
        assert!(saved.contains("HeroSkin"));
    }

    #[test]
    fn test_show_obj_importer_tab_renders_phase_six_sections_without_panicking() {
        let mut state = triangle_mesh_state();
        state.active_mtl_source = ImportedMtlSourceKind::AutoDetected;
        state.resolved_mtl_paths = vec![PathBuf::from("models/hero.mtl")];
        state.imported_material_palette = vec![ImportedMaterialSwatch {
            label: "HeroSkin".to_string(),
            color: [0.7, 0.6, 0.5, 1.0],
            texture_path: Some("textures/hero.png".to_string()),
            texture_source_path: None,
        }];
        state.meshes[0].color_source = ImportedMeshColorSource::ImportedMaterial;
        let ctx = egui::Context::default();
        let mut logger = Logger::default();

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let signal = show_obj_importer_tab(ui, &mut state, None, &mut logger);
                assert!(signal.is_none());
            });
        });

        assert_eq!(state.mode, ImporterMode::Loaded);
    }

    // ─── GLB builder helpers (local copies for UI-layer tests) ──────────────

    /// Build a minimal valid GLB binary from a JSON chunk and optional binary chunk.
    fn build_test_glb(json: &str, bin: Option<&[u8]>) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let bin_chunk_total = bin.map_or(0usize, |b| {
            let padded = (b.len() + 3) & !3;
            8 + padded
        });
        let total_len = 12 + 8 + json_bytes.len() + bin_chunk_total;
        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic "glTF"
        out.extend_from_slice(&2u32.to_le_bytes()); // version
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        out.extend_from_slice(&json_bytes);
        if let Some(bin_data) = bin {
            let padded = (bin_data.len() + 3) & !3;
            out.extend_from_slice(&(padded as u32).to_le_bytes());
            out.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            out.extend_from_slice(bin_data);
            let pad = padded - bin_data.len();
            out.resize(out.len() + pad, 0x00);
        }
        out
    }

    /// A minimal GLB with one triangle mesh and no texture.
    fn build_minimal_triangle_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for f in pos {
                bin.extend_from_slice(&f.to_le_bytes());
            }
        }
        for idx in [0u16, 1, 2] {
            bin.extend_from_slice(&idx.to_le_bytes());
        }
        build_test_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"name":"TestMesh","primitives":[{"attributes":{"POSITION":0},"indices":1,"mode":4}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],"buffers":[{"byteLength":42}]}"#,
            Some(&bin),
        )
    }

    // ─── Phase 3 tests ────────────────────────────────────────────────────────

    /// OBJ path dispatches to the OBJ loader and leaves `source_format == Obj`.
    #[test]
    fn test_load_model_into_state_dispatches_obj() {
        let path = fixture_path("data/test_fixtures/skeleton.obj");
        let mut state = ObjImporterState::new();

        let result = load_model_into_state(&mut state, &path);

        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        assert_eq!(state.source_format, ImportSourceFormat::Obj);
        assert_eq!(state.mode, ImporterMode::Loaded);
    }

    /// GLB path dispatches to the GLB loader and leaves `source_format == Glb`.
    #[test]
    fn test_load_model_into_state_dispatches_glb() {
        let temp_dir = tempdir().unwrap();
        let glb_path = temp_dir.path().join("model.glb");
        fs::write(&glb_path, build_minimal_triangle_glb()).unwrap();
        let mut state = ObjImporterState::new();

        let result = load_model_into_state(&mut state, &glb_path);

        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        assert_eq!(state.source_format, ImportSourceFormat::Glb);
        assert_eq!(state.mode, ImporterMode::Loaded);
    }

    /// An unrecognised extension returns `ObjImportError::UnknownFormat`.
    #[test]
    fn test_load_model_into_state_rejects_unknown_extension() {
        let mut state = ObjImporterState::new();
        let path = PathBuf::from("model.fbx");

        let result = load_model_into_state(&mut state, &path);

        assert!(
            matches!(result, Err(ObjImportError::UnknownFormat { ref extension }) if extension == "fbx"),
            "Expected UnknownFormat{{extension: fbx}}, got {result:?}"
        );
    }

    // ─── Phase 4 tests: Embedded Texture Export ───────────────────────────────

    /// Returns a minimal `ImportedTexturePayload` with embedded PNG bytes.
    fn glb_embedded_payload(file_name_hint: &str, bytes: Vec<u8>) -> ImportedTexturePayload {
        ImportedTexturePayload {
            source_label: "albedo".to_string(),
            file_name_hint: file_name_hint.to_string(),
            bytes: Some(bytes),
            source_path: None,
            mime_type: Some("image/png".to_string()),
        }
    }

    /// After exporting a GLB creature, the embedded texture bytes are written to
    /// `assets/textures/imported/<asset>/<hint>` with the expected content.
    #[test]
    fn test_export_glb_embedded_texture_writes_campaign_texture_file() {
        let campaign_dir = tempdir().unwrap();
        let texture_bytes = b"PNG_FAKE_BYTES_FOR_TEST".to_vec();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload =
            Some(glb_embedded_payload("texture_0.png", texture_bytes.clone()));

        let _outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();

        let expected = campaign_dir
            .path()
            .join("assets/textures/imported/test_import/texture_0.png");
        assert!(expected.exists(), "Texture file should have been written");
        assert_eq!(
            fs::read(&expected).unwrap(),
            texture_bytes,
            "Texture file content must match the embedded bytes"
        );
    }

    /// The RON creature file must reference the campaign-relative texture path
    /// produced during export, not the placeholder embedded in the GLB.
    #[test]
    fn test_export_glb_rewrites_mesh_texture_path_to_campaign_relative() {
        let campaign_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload = Some(glb_embedded_payload(
            "texture_0.png",
            b"fake_png_data".to_vec(),
        ));

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let texture_path = round_trip.meshes[0].texture_path.as_ref().unwrap();
        assert_eq!(
            texture_path,
            "assets/textures/imported/test_import/texture_0.png"
        );
    }

    /// Two meshes with distinct embedded payloads must produce two distinct
    /// texture files under the same asset directory.
    #[test]
    fn test_export_glb_multiple_embedded_textures_get_distinct_paths() {
        let campaign_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload = Some(glb_embedded_payload(
            "texture_0.png",
            b"body_bytes".to_vec(),
        ));

        let mut wing = state.meshes[0].clone();
        wing.name = "wing".to_string();
        wing.mesh_def.name = Some("wing".to_string());
        wing.mesh_def.texture_path = Some("__glb_embedded_1".to_string());
        wing.texture_payload = Some(glb_embedded_payload(
            "texture_1.png",
            b"wing_bytes".to_vec(),
        ));
        state.meshes.push(wing);

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let path0 = round_trip.meshes[0].texture_path.as_ref().unwrap();
        let path1 = round_trip.meshes[1].texture_path.as_ref().unwrap();
        assert_ne!(
            path0, path1,
            "Distinct payloads must produce distinct texture paths"
        );
        assert!(
            campaign_dir.path().join(path0).exists(),
            "First texture file must exist"
        );
        assert!(
            campaign_dir.path().join(path1).exists(),
            "Second texture file must exist"
        );
    }

    /// Two meshes whose embedded byte payloads are **identical** must both
    /// reference the same destination file; only one file should be written.
    #[test]
    fn test_export_glb_deduplicates_identical_texture_payload() {
        let campaign_dir = tempdir().unwrap();
        let shared_bytes = b"shared_texture_data".to_vec();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload =
            Some(glb_embedded_payload("texture_0.png", shared_bytes.clone()));

        let mut second = state.meshes[0].clone();
        second.name = "detail".to_string();
        second.mesh_def.name = Some("detail".to_string());
        second.mesh_def.texture_path = Some("__glb_embedded_1".to_string());
        second.texture_payload = Some(glb_embedded_payload("texture_0.png", shared_bytes));
        state.meshes.push(second);

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let path0 = round_trip.meshes[0].texture_path.as_ref().unwrap();
        let path1 = round_trip.meshes[1].texture_path.as_ref().unwrap();
        assert_eq!(
            path0, path1,
            "Identical payloads must deduplicate to the same path"
        );

        let texture_dir = campaign_dir
            .path()
            .join("assets/textures/imported/test_import");
        let file_count = fs::read_dir(&texture_dir).map(|d| d.count()).unwrap_or(0);
        assert_eq!(
            file_count, 1,
            "Only one texture file should be written for identical payloads"
        );
    }

    /// When a mesh declares a texture path but the payload has neither embedded
    /// bytes nor a valid filesystem path, export must fail with `MissingTexture`
    /// **before** any RON file is created.
    #[test]
    fn test_export_glb_missing_texture_payload_fails_before_ron_write() {
        let campaign_dir = tempdir().unwrap();
        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload = Some(ImportedTexturePayload {
            source_label: "missing".to_string(),
            file_name_hint: "texture_0.png".to_string(),
            bytes: None,       // No embedded bytes
            source_path: None, // No filesystem path
            mime_type: Some("image/png".to_string()),
        });

        let error = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap_err();

        assert!(
            matches!(error, ObjImporterExportError::MissingTexture { .. }),
            "Expected MissingTexture, got {error:?}"
        );
        let ron_path = campaign_dir.path().join("assets/creatures/test_import.ron");
        assert!(
            !ron_path.exists(),
            "RON file must not be created when texture resolution fails"
        );
    }

    /// OBJ-style texture export (filesystem `source_path`, no embedded bytes)
    /// must continue to work after the Phase 4 refactor.
    #[test]
    fn test_export_obj_texture_copy_still_passes() {
        let campaign_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let body_texture = source_dir.path().join("body.png");
        fs::write(&body_texture, b"obj_body_texture").unwrap();

        let mut state = triangle_mesh_state();
        state.meshes[0].mesh_def.texture_path = Some("textures/body.png".to_string());
        state.meshes[0].texture_payload = Some(ImportedTexturePayload {
            source_label: "body.png".to_string(),
            file_name_hint: "body.png".to_string(),
            bytes: None,                             // OBJ: no embedded bytes
            source_path: Some(body_texture.clone()), // OBJ: resolved filesystem path
            mime_type: None,
        });

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let texture_path = round_trip.meshes[0].texture_path.as_ref().unwrap();
        assert!(
            texture_path.starts_with("assets/textures/imported/test_import/"),
            "Expected campaign-relative prefix, got: {texture_path}"
        );
        assert!(
            campaign_dir.path().join(texture_path).exists(),
            "Exported texture file must exist"
        );
        assert_eq!(
            fs::read(campaign_dir.path().join(texture_path)).unwrap(),
            b"obj_body_texture",
            "Copied texture content must match the source file"
        );
    }

    // ── Phase 5 tests: Runtime and Domain Compatibility ─────────────────────

    /// A `CreatureDefinition` whose `texture_path` follows the GLB-import
    /// campaign-relative convention (`assets/textures/imported/<name>/<file>`)
    /// must survive a full RON serialization/deserialization round-trip without
    /// any path corruption (back-slash conversion, extra escaping, etc.).
    #[test]
    fn test_glb_exported_texture_path_round_trips_through_ron() {
        let original_path = "assets/textures/imported/dragon/dragon_albedo.png";

        // Pre-create the texture so resolve_imported_texture_source recognises
        // it as AlreadyCampaignRelative and leaves mesh_def.texture_path intact.
        let campaign_dir = tempdir().unwrap();
        let texture_file = campaign_dir.path().join(original_path);
        fs::create_dir_all(texture_file.parent().unwrap()).unwrap();
        fs::write(&texture_file, b"dummy").unwrap();

        let mut state = triangle_mesh_state();
        state.creature_name = "Dragon".to_string();
        state.meshes[0].mesh_def.texture_path = Some(original_path.to_string());
        state.meshes[0].texture_payload = None; // already campaign-relative, no copy needed

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        assert_eq!(
            round_trip.meshes[0].texture_path.as_deref(),
            Some(original_path),
            "texture_path must survive RON round-trip without path corruption"
        );
    }

    /// Exporting with `ExportType::Item` must preserve `texture_path` in the
    /// written RON so the runtime can locate the texture asset.
    #[test]
    fn test_glb_item_export_preserves_texture_path() {
        let campaign_dir = tempdir().unwrap();
        let texture_bytes = b"fake_sword_texture".to_vec();

        let mut state = triangle_mesh_state();
        state.export_type = ExportType::Item;
        state.creature_name = "Magic Sword".to_string();
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload =
            Some(glb_embedded_payload("sword_albedo.png", texture_bytes));

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let texture_path = round_trip.meshes[0]
            .texture_path
            .as_ref()
            .expect("Item export must preserve texture_path in RON");
        assert!(
            texture_path.starts_with("assets/textures/imported/magic_sword/"),
            "Item texture_path must be campaign-relative under \
             assets/textures/imported/, got: {texture_path}"
        );
        assert!(
            campaign_dir.path().join(texture_path).exists(),
            "Item texture file must exist at the exported campaign-relative path"
        );
    }

    /// Exporting with `ExportType::Furniture` must preserve `texture_path` in
    /// the written RON so the runtime can locate the texture asset.
    #[test]
    fn test_importer_large_mesh_list_renders_bounded_rows() {
        // Build a state with 200 meshes. The show_rows virtualization should
        // handle this without panicking and without rendering all 200 rows.
        let mut state = ObjImporterState::default();
        state.mode = crate::obj_importer::ImporterMode::Loaded;

        for i in 0..200_usize {
            use antares::domain::visual::MeshDefinition;
            let mesh_def = MeshDefinition {
                name: Some(format!("mesh_{}", i)),
                vertices: vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            };
            state
                .meshes
                .push(crate::obj_importer::ImportedMesh::from_mesh_definition(
                    mesh_def,
                ));
        }

        // Render the loaded mode in a small fixed-height egui context.
        // The test passes as long as show_rows does not panic on large lists.
        let ctx = egui::Context::default();
        let mut logger = crate::logging::Logger::default();
        let did_not_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::obj_importer_ui::show_obj_importer_tab(
                        ui,
                        &mut state,
                        None,
                        &mut logger,
                    );
                });
            });
        }));
        assert!(
            did_not_panic.is_ok(),
            "show_rows must handle 200-mesh importer state without panicking"
        );
    }

    #[test]
    fn test_glb_furniture_export_preserves_texture_path() {
        let campaign_dir = tempdir().unwrap();
        let texture_bytes = b"fake_wood_texture".to_vec();

        let mut state = triangle_mesh_state();
        state.export_type = ExportType::Furniture;
        state.creature_name = "Oak Table".to_string();
        state.furniture_id = 10042;
        state.meshes[0].mesh_def.texture_path = Some("__glb_embedded_0".to_string());
        state.meshes[0].texture_payload =
            Some(glb_embedded_payload("table_wood.png", texture_bytes));

        let outcome = export_state_to_campaign(&state, Some(campaign_dir.path())).unwrap();
        let exported = fs::read_to_string(&outcome.absolute_path).unwrap();
        let round_trip = ron::from_str::<CreatureDefinition>(&exported).unwrap();

        let texture_path = round_trip.meshes[0]
            .texture_path
            .as_ref()
            .expect("Furniture export must preserve texture_path in RON");
        assert!(
            texture_path.starts_with("assets/textures/imported/oak_table/"),
            "Furniture texture_path must be campaign-relative under \
             assets/textures/imported/, got: {texture_path}"
        );
        assert!(
            campaign_dir.path().join(texture_path).exists(),
            "Furniture texture file must exist at the exported campaign-relative path"
        );
    }
}
