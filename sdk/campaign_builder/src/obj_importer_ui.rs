// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OBJ importer tab UI and RON export workflow.
//!
//! This module renders the Campaign Builder Importer tab, lets users inspect
//! imported OBJ meshes, edit per-mesh colors, manage custom palette entries,
//! and export the result as a `CreatureDefinition` RON asset.
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
    ExportType, ImportedMaterialSwatch, ImportedMeshColorSource, ImportedMtlSourceKind,
    ImporterMode, ObjImporterState,
};
use crate::ui_helpers::TwoColumnLayout;
use antares::domain::visual::{CreatureDefinition, MeshTransform};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Signal emitted by the importer tab when an export completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjImporterUiSignal {
    /// A creature asset was exported and the creature registry should be reloaded.
    CreatureExported,
    /// An item asset was exported.
    ItemExported,
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
    ui.heading("OBJ Importer");
    ui.label("Load a Wavefront OBJ, adjust mesh colors, then export a creature or item RON asset.");

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
        .unwrap_or_else(|| "No OBJ selected".to_string());

    egui::Grid::new("obj_importer_idle_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Source OBJ:");
            ui.horizontal(|ui| {
                ui.monospace(selected_path.as_str());
                if ui.button("Browse...").clicked() {
                    if let Some(path) = pick_obj_file(state.source_path.as_deref()) {
                        state.source_path = Some(path.clone());
                        if state.creature_name.trim().is_empty() {
                            state.creature_name = display_name_from_path(&path);
                        }
                        state.status_message = format!("Selected OBJ file: {}", path.display());
                        ui.ctx().request_repaint();
                    }
                }
            });
            ui.end_row();

            ui.label("MTL Source:");
            render_mtl_source_controls(ui, state, logger, false);
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
            });
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

    if ui.button("Load OBJ").clicked() {
        let path = if let Some(path) = state.source_path.clone() {
            path
        } else if let Some(path) = pick_obj_file(None) {
            state.source_path = Some(path.clone());
            path
        } else {
            state.status_message = "Choose an OBJ file before loading.".to_string();
            return;
        };

        match load_obj_into_state(state, &path) {
            Ok(()) => {
                logger.info(
                    category::EDITOR,
                    &format!("Loaded OBJ importer source {}", path.display()),
                );
                ui.ctx().request_repaint();
            }
            Err(error) => {
                state.status_message = error;
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
            ui.label("MTL Source:");
            render_mtl_source_controls(ui, state, logger, true);
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
            });
            ui.end_row();

            ui.label("ID:");
            if ui
                .add(egui::DragValue::new(&mut state.creature_id).range(0..=u32::MAX))
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
                state.creature_id,
            ));
            ui.end_row();
        });

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

        if ui.button("Load Another OBJ").clicked() {
            if let Some(path) = pick_obj_file(state.source_path.as_deref()) {
                match load_obj_into_state(state, &path) {
                    Ok(()) => {
                        logger.info(
                            category::EDITOR,
                            &format!("Reloaded importer OBJ {}", path.display()),
                        );
                        ui.ctx().request_repaint();
                    }
                    Err(error) => {
                        state.status_message = error;
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

        let export_enabled = campaign_dir.is_some() && !state.meshes.is_empty();
        if ui
            .add_enabled(export_enabled, egui::Button::new("Export RON"))
            .clicked()
        {
            state.mode = ImporterMode::Exporting;
            match export_state_to_campaign(state, campaign_dir.map(|path| path.as_path())) {
                Ok(outcome) => {
                    let signal = match outcome.export_type {
                        ExportType::Creature => ObjImporterUiSignal::CreatureExported,
                        ExportType::Item => ObjImporterUiSignal::ItemExported,
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

                egui::ScrollArea::vertical()
                    .id_salt("obj_importer_mesh_list_scroll")
                    .show(left_ui, |ui| {
                        for row in &row_snapshots {
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
            if state.remove_custom_color(&label) {
                if persist_custom_palette(state, campaign_dir, logger) {
                    state.status_message = format!("Removed custom color '{}'.", label);
                    ui.ctx().request_repaint();
                }
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

fn load_obj_into_state(state: &mut ObjImporterState, path: &Path) -> Result<(), String> {
    state
        .load_obj_file(path)
        .map_err(|error| format!("Failed to load OBJ '{}': {}", path.display(), error))?;

    if state.creature_name.trim().is_empty() {
        state.creature_name = display_name_from_path(path);
    }
    state.set_active_mesh((!state.meshes.is_empty()).then_some(0));
    state.status_message = format!(
        "Loaded {} mesh(es) from {}",
        state.meshes.len(),
        path.display()
    );
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

    match load_obj_into_state(state, &source_path) {
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
            state.status_message = error;
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

fn pick_obj_file(initial_path: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter("Wavefront OBJ", &["obj"]);
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
    let creature = build_creature_definition(state)?;
    let relative_path =
        preview_export_relative_path(state.export_type, &creature.name, creature.id);
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

fn preview_export_relative_path(export_type: ExportType, name: &str, id: u32) -> String {
    let file_stem = sanitized_export_stem(name, id, export_type);
    match export_type {
        ExportType::Creature => format!("assets/creatures/{}.ron", file_stem),
        ExportType::Item => format!("assets/items/{}.ron", file_stem),
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
        }
    } else {
        sanitized
    }
}

fn export_type_label(export_type: ExportType) -> &'static str {
    match export_type {
        ExportType::Creature => "creature",
        ExportType::Item => "item",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_creature_definition, describe_mesh_color_source, export_state_to_campaign,
        format_mtl_source_detail, format_mtl_source_summary, persist_custom_palette,
        preview_export_relative_path, show_obj_importer_tab, stage_imported_swatch_as_custom_draft,
    };
    use crate::logging::Logger;
    use crate::obj_importer::{
        ExportType, ImportedMaterialSwatch, ImportedMeshColorSource, ImportedMtlSourceKind,
        ImporterMode, ObjImporterState,
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
        let path = fixture_path("examples/skeleton.obj");
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
    fn test_preview_export_relative_path_uses_expected_directories() {
        assert_eq!(
            preview_export_relative_path(ExportType::Creature, "Stone Golem", 44),
            "assets/creatures/stone_golem.ron"
        );
        assert_eq!(
            preview_export_relative_path(ExportType::Item, "", 9),
            "assets/items/item_9.ron"
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
        }];
        state.meshes[0].color_source = ImportedMeshColorSource::ImportedMaterial;
        let ctx = egui::Context::default();
        let mut logger = Logger::default();

        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let signal = show_obj_importer_tab(ui, &mut state, None, &mut logger);
                assert!(signal.is_none());
            });
        });

        assert!(!output.platform_output.cursor_icon.is_resize());
        assert_eq!(state.mode, ImporterMode::Loaded);
    }
}
