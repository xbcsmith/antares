// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Custom terrain definition editor for the Campaign Builder SDK.
//!
//! Campaign authors can define new terrain types here. Built-in terrain
//! (IDs 13 000–13 011) is displayed as a read-only reference in a
//! collapsible section. Only campaign-defined terrain (IDs ≥ 13 100) is
//! editable.
//!
//! # Usage
//!
//! ```no_run
//! use campaign_builder::terrain_editor::TerrainEditorState;
//!
//! let mut state = TerrainEditorState::new();
//! assert!(state.search_query.is_empty());
//! ```

use crate::editor_context::EditorContext;
use crate::ui_helpers::{
    handle_reload, handle_toolbar_action, show_standard_list_item, EditorToolbar, ItemAction,
    MetadataBadge, StandardListItemConfig, ToolbarAction, TwoColumnLayout,
};
use antares::domain::types::TerrainId;
use antares::domain::world::terrain::{
    builtin_terrain_definitions, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
};
use eframe::egui;
use std::collections::HashMap;

/// Campaign-defined terrain IDs start here.
///
/// IDs 13 000–13 099 are reserved for built-in and engine-defined terrain.
/// Campaign authors must allocate IDs ≥ 13 100 for their custom terrain.
const CAMPAIGN_TERRAIN_START: TerrainId = 13_100;

// ─── Editor mode ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, PartialEq)]
enum TerrainEditorMode {
    #[default]
    List,
    Edit,
}

// ─── Delete confirmation ──────────────────────────────────────────────────────

/// Holds the id and name of a terrain definition pending delete confirmation.
#[derive(Debug, Clone)]
struct PendingDeleteConfirm {
    id: TerrainId,
    name: String,
}

/// Opaque terrain texture cache.
///
/// Wraps `HashMap<String, Option<egui::TextureHandle>>` and provides a minimal
/// [`Debug`] implementation so `TerrainEditorState` can keep `#[derive(Debug)]`
/// even though `TextureHandle` is not `Debug`.
#[derive(Default)]
struct TerrainTextureCache(HashMap<String, Option<egui::TextureHandle>>);

impl std::fmt::Debug for TerrainTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TerrainTextureCache({} entries)", self.0.len())
    }
}

// ─── TerrainEditorState ───────────────────────────────────────────────────────

/// View state for the Terrain definition editor tab.
///
/// Holds all transient UI state for listing and editing campaign-defined
/// terrain definitions. The authoritative data (`Vec<TerrainDefinition>`)
/// lives in [`crate::editor_state::CampaignData`]; this struct holds only
/// the view state.
///
/// # Examples
///
/// ```
/// use campaign_builder::terrain_editor::TerrainEditorState;
///
/// let state = TerrainEditorState::new();
/// assert!(state.search_query.is_empty());
/// assert!(state.selected_terrain.is_none());
/// ```
#[derive(Debug, Default)]
pub struct TerrainEditorState {
    /// Search text used to filter terrain definitions by name.
    pub search_query: String,
    /// Index of the selected terrain definition in the campaign definitions Vec.
    pub selected_terrain: Option<usize>,

    // --- edit-mode fields ---
    mode: TerrainEditorMode,
    /// Index in the definitions Vec being edited; `None` means a new entry not
    /// yet in the vec (created by "Add Terrain" and appended on Save).
    edit_index: Option<usize>,
    /// Working copy of the definition currently displayed in the edit form.
    edit_buffer: Option<TerrainDefinition>,

    /// Pending delete awaiting user confirmation.
    pending_delete_confirm: Option<PendingDeleteConfirm>,

    /// Set to `true` when terrain definitions are successfully loaded from
    /// disk via [`crate::campaign_io::CampaignBuilderApp::load_terrain`].
    ///
    /// Guards [`crate::campaign_io::CampaignBuilderApp::save_terrain`] in
    /// `do_save_campaign` against writing an empty `[]` when terrain.ron
    /// was never loaded during this session — the same wipe-bug pattern
    /// that affected creatures, stock-templates, and levels.
    pub loaded_from_file: bool,
    /// Set to `true` after `reset_for_new_campaign()`. Cleared by `show()` on first render.
    pub needs_initial_load: bool,
    /// Merge-mode flag for the standard toolbar.
    file_load_merge_mode: bool,

    /// Texture cache: maps a `texture_path` string to a loaded egui handle (or `None` on
    /// load failure).  Populated lazily in `show_list` and `show_edit`; cleared on every
    /// campaign open/new so stale paths from a previous campaign do not bleed through.
    terrain_texture_cache: TerrainTextureCache,
}

impl TerrainEditorState {
    /// Creates a fresh terrain editor state with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::terrain_editor::TerrainEditorState;
    ///
    /// let state = TerrainEditorState::new();
    /// assert!(state.search_query.is_empty());
    /// assert_eq!(state.selected_terrain, None);
    /// ```
    pub fn new() -> Self {
        Self {
            needs_initial_load: true,
            ..Self::default()
        }
    }

    /// Resets editor state for a new or freshly-opened campaign.
    ///
    /// Clears search/selection/edit state and sets `needs_initial_load = true`
    /// so that `show()` performs an auto-load the first time the Terrain tab
    /// is rendered. Call this from both `do_new_campaign` and
    /// `do_open_campaign` — `sdk/AGENTS.md` Rule 13.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::terrain_editor::TerrainEditorState;
    ///
    /// let mut state = TerrainEditorState::default();
    /// state.needs_initial_load = false;
    /// state.reset_for_new_campaign();
    /// assert!(state.needs_initial_load);
    /// ```
    pub fn reset_for_new_campaign(&mut self) {
        self.search_query.clear();
        self.selected_terrain = None;
        self.mode = TerrainEditorMode::List;
        self.edit_index = None;
        self.edit_buffer = None;
        self.pending_delete_confirm = None;
        self.needs_initial_load = true;
        self.file_load_merge_mode = true;
        self.terrain_texture_cache.0.clear();
        self.loaded_from_file = false;
    }

    /// Loads a terrain texture from `campaign_dir / texture_path` and caches the
    /// resulting [`egui::TextureHandle`].  Returns a cloned handle on cache hit;
    /// attempts to decode on miss and caches `None` for unreadable paths so the
    /// failed lookup is not retried every frame.
    ///
    /// # Arguments
    ///
    /// * `ctx`          - The egui context used to register the texture.
    /// * `campaign_dir` - Root of the open campaign (resolves relative texture paths).
    /// * `texture_path` - Path relative to `campaign_dir` (e.g. `"assets/textures/terrain/grass.png"`).
    fn load_or_get_texture(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&std::path::PathBuf>,
        texture_path: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.terrain_texture_cache.0.get(texture_path) {
            return cached.clone();
        }
        let full_path = match campaign_dir {
            Some(dir) => dir.join(texture_path),
            None => std::path::PathBuf::from(texture_path),
        };
        let handle = (|| -> Option<egui::TextureHandle> {
            let bytes = std::fs::read(&full_path).ok()?;
            let img = image::load_from_memory(&bytes).ok()?;
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            Some(ctx.load_texture(
                format!("terrain_tex_{texture_path}"),
                color_image,
                egui::TextureOptions::LINEAR,
            ))
        })();
        self.terrain_texture_cache
            .0
            .insert(texture_path.to_string(), handle.clone());
        handle
    }

    /// Renders the terrain editor UI.
    ///
    /// Call this from the `EditorTab::Terrain` arm in the central panel match.
    /// `defs` must be the **campaign-defined** terrain only
    /// (from `campaign_data.terrain_definitions`); built-in terrain is
    /// displayed separately as a read-only reference.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # fn render(ui: &mut eframe::egui::Ui) {
    /// use campaign_builder::terrain_editor::TerrainEditorState;
    /// use campaign_builder::editor_context::EditorContext;
    ///
    /// let mut state = TerrainEditorState::new();
    /// let mut definitions = Vec::new();
    /// let mut unsaved = false;
    /// let mut status = String::new();
    /// let mut merge = true;
    /// let mut ctx = EditorContext::new(None, "data/terrain.ron", &mut unsaved, &mut status, &mut merge);
    /// state.show(ui, &mut definitions, &mut ctx);
    /// # }
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<TerrainDefinition>,
        ctx: &mut EditorContext<'_>,
    ) {
        // SDK Rule 13: auto-load guard — load once on first render after campaign open/new.
        if self.needs_initial_load {
            if let Some(dir) = ctx.campaign_dir {
                let path = dir.join(ctx.data_file);
                if path.exists() {
                    match crate::ui_helpers::load_ron_file::<Vec<TerrainDefinition>>(&path) {
                        Ok(loaded) => {
                            *defs = loaded;
                        }
                        Err(e) => {
                            *ctx.status_message = format!("Failed to auto-load terrain: {}", e);
                        }
                    }
                }
            }
            self.needs_initial_load = false;
        }

        let toolbar_action = EditorToolbar::new("Terrain")
            .with_search(&mut self.search_query)
            .with_merge_mode(&mut self.file_load_merge_mode)
            .with_total_count(defs.len())
            .with_id_salt("terrain_toolbar")
            .show(ui);

        match toolbar_action {
            ToolbarAction::New => {
                self.enter_add(defs);
                ui.ctx().request_repaint();
            }
            ToolbarAction::Reload => {
                handle_reload(defs, ctx.campaign_dir, ctx.data_file, ctx.status_message);
            }
            other => {
                let mut editor_unsaved = false;
                handle_toolbar_action(
                    other,
                    defs,
                    |d: &TerrainDefinition| d.id,
                    &mut editor_unsaved,
                    ctx,
                    "terrain.ron",
                    "terrain definitions",
                );
            }
        }

        if self.mode == TerrainEditorMode::Edit {
            self.show_edit(ui, defs, ctx);
        } else {
            self.show_list(ui, defs, ctx);
        }
    }

    // =========================================================================
    // List view
    // =========================================================================

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<TerrainDefinition>,
        ctx: &mut EditorContext<'_>,
    ) {
        // SDK Rule 10: pre-compute ALL shared state before multi-closure calls.
        let filtered_rows = self.filtered_rows(defs);
        let selected_idx = self.selected_terrain;
        let preview_snapshot: Option<TerrainDefinition> = selected_idx
            .filter(|&i| i < defs.len())
            .map(|i| defs[i].clone());

        // Load the texture for the selected terrain before entering the split
        // closures so that neither closure needs to borrow &mut self.
        let preview_texture: Option<egui::TextureHandle> =
            preview_snapshot.as_ref().and_then(|def| {
                self.load_or_get_texture(ui.ctx(), ctx.campaign_dir, &def.texture_path)
            });

        let mut pending_selection: Option<usize> = None;
        let mut pending_edit: Option<usize> = None;
        let mut pending_delete: Option<usize> = None;

        // SDK Rule 9: TwoColumnLayout for list/detail splits.
        TwoColumnLayout::new("terrain_editor").show_split(
            ui,
            |left_ui| {
                if filtered_rows.is_empty() {
                    let msg = if defs.is_empty() {
                        "No custom terrain definitions.\nClick \"+ Add Terrain\" to create one."
                    } else {
                        "No terrain matches the search filter."
                    };
                    left_ui.label(egui::RichText::new(msg).weak().italics());
                } else {
                    // SDK Rule 1: push_id in every loop body.
                    for idx in filtered_rows.iter().copied() {
                        let def = &defs[idx];
                        left_ui.push_id(def.id, |ui| {
                            let selected = selected_idx == Some(idx);
                            let (clicked, action) = show_standard_list_item(
                                ui,
                                StandardListItemConfig::new(&def.name)
                                    .selected(selected)
                                    .with_icon("🗺")
                                    .with_id(def.id)
                                    .with_badges(vec![
                                        MetadataBadge::new(mesh_style_name(def.mesh_style)),
                                        MetadataBadge::new(vegetation_name(def.vegetation)),
                                        if def.blocked {
                                            MetadataBadge::new("blocked")
                                        } else {
                                            MetadataBadge::new("passable")
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
                }

                // Read-only reference section for built-in terrain.
                left_ui.add_space(8.0);
                left_ui.collapsing("Built-in terrain (read-only)", |ui| {
                    let mut builtins = builtin_terrain_definitions();
                    builtins.sort_by_key(|d| d.id);
                    // Filter out built-ins whose IDs are already overridden by campaign entries.
                    let campaign_ids: std::collections::HashSet<TerrainId> =
                        defs.iter().map(|d| d.id).collect();
                    let unoverridden: Vec<_> = builtins
                        .iter()
                        .filter(|b| !campaign_ids.contains(&b.id))
                        .collect();
                    if unoverridden.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "All built-in terrain is overridden by this campaign.",
                            )
                            .weak()
                            .italics(),
                        );
                    } else {
                        // SDK Rule 1: push_id in every loop body.
                        for def in &unoverridden {
                            ui.push_id(def.id, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("#{} {}", def.id, def.name))
                                            .monospace(),
                                    );
                                    ui.colored_label(
                                        egui::Color32::GRAY,
                                        format!("[{}]", mesh_style_name(def.mesh_style)),
                                    );
                                    if def.blocked {
                                        ui.colored_label(egui::Color32::DARK_RED, "blocked");
                                    }
                                });
                            });
                        }
                    }
                });
            },
            |right_ui| {
                show_terrain_preview(
                    right_ui,
                    preview_snapshot.as_ref(),
                    preview_texture.as_ref(),
                );
            },
        );

        // Apply deferred mutations after show_split — no active closure borrows.
        if let Some(idx) = pending_selection {
            self.selected_terrain = Some(idx);
        }
        if let Some(idx) = pending_edit {
            self.selected_terrain = Some(idx);
            self.enter_edit(idx, defs);
        }
        if let Some(idx) = pending_delete {
            if idx < defs.len() {
                self.pending_delete_confirm = Some(PendingDeleteConfirm {
                    id: defs[idx].id,
                    name: defs[idx].name.clone(),
                });
            }
        }

        self.show_delete_confirmation(ui, defs, ctx.unsaved_changes);
    }

    /// Removes `defs[idx]` and adjusts `selected_terrain` to stay valid.
    fn remove_terrain_definition(
        &mut self,
        idx: usize,
        defs: &mut Vec<TerrainDefinition>,
        unsaved_changes: &mut bool,
    ) {
        defs.remove(idx);
        match self.selected_terrain {
            Some(sel) if sel == idx => self.selected_terrain = None,
            Some(sel) if sel > idx => self.selected_terrain = Some(sel - 1),
            _ => {}
        }
        *unsaved_changes = true;
    }

    /// Renders the delete-confirmation dialog. No-op when no delete is pending.
    fn show_delete_confirmation(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<TerrainDefinition>,
        unsaved_changes: &mut bool,
    ) {
        let Some(pending) = self.pending_delete_confirm.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new("Confirm Terrain Deletion")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |dialog_ui| {
                dialog_ui.label(format!("Delete terrain definition '{}'?", pending.name));
                dialog_ui.label("This action cannot be undone.");
                dialog_ui.separator();
                // SDK Rule 12: horizontal_wrapped for button rows.
                dialog_ui.horizontal_wrapped(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.pending_delete_confirm = None;
                    }
                    if ui.button("Delete").clicked() {
                        if let Some(idx) = defs.iter().position(|def| def.id == pending.id) {
                            self.remove_terrain_definition(idx, defs, unsaved_changes);
                        }
                        self.pending_delete_confirm = None;
                    }
                });
            });
        if !open {
            self.pending_delete_confirm = None;
        }
    }

    // =========================================================================
    // Edit view
    // =========================================================================

    fn enter_edit(&mut self, idx: usize, defs: &[TerrainDefinition]) {
        if idx >= defs.len() {
            return;
        }
        self.edit_index = Some(idx);
        self.edit_buffer = Some(defs[idx].clone());
        self.mode = TerrainEditorMode::Edit;
    }

    fn enter_add(&mut self, defs: &[TerrainDefinition]) {
        let new_id = next_available_id(defs);
        // edit_index = None signals "new entry"; apply_edit will push it.
        self.edit_index = None;
        self.edit_buffer = Some(TerrainDefinition {
            id: new_id,
            name: format!("Custom Terrain {}", new_id),
            texture_path: "assets/textures/terrain/custom.png".to_string(),
            roughness: 0.5,
            mesh_style: TerrainMeshStyle::default(),
            vegetation: TerrainVegetation::default(),
            blocked: false,
            height: 0.0,
            color: [0.5, 0.5, 0.5],
        });
        self.mode = TerrainEditorMode::Edit;
    }

    fn apply_edit(&mut self, defs: &mut Vec<TerrainDefinition>) {
        let Some(buf) = self.edit_buffer.take() else {
            return;
        };
        if let Some(idx) = self.edit_index {
            if idx < defs.len() {
                defs[idx] = buf;
            }
        } else {
            // New entry — append and select it.
            defs.push(buf);
            self.selected_terrain = Some(defs.len() - 1);
        }
        self.edit_index = None;
    }

    fn show_edit(
        &mut self,
        ui: &mut egui::Ui,
        defs: &mut Vec<TerrainDefinition>,
        ctx: &mut EditorContext<'_>,
    ) {
        // Phase 1 — brief immutable borrow to clone what we need before
        // the mutable borrow for the edit form (avoids borrow-conflict with
        // load_or_get_texture which borrows &mut self.terrain_texture_cache).
        let Some((texture_path, heading_name)) = self
            .edit_buffer
            .as_ref()
            .map(|buf| (buf.texture_path.clone(), buf.name.clone()))
        else {
            self.mode = TerrainEditorMode::List;
            return;
        };

        // Phase 2 — load texture while edit_buffer is not borrowed.
        let edit_texture: Option<egui::TextureHandle> =
            self.load_or_get_texture(ui.ctx(), ctx.campaign_dir, &texture_path);

        // Phase 3 — mutable borrow for the actual form.
        let Some(buf) = self.edit_buffer.as_mut() else {
            return;
        };

        // Capture campaign_dir as an owned value so it can be moved into the
        // egui closures below without conflicting with the mutable borrow of
        // `self` via `buf` or the `&mut EditorContext` borrow of `ctx`.
        let campaign_dir_owned: Option<std::path::PathBuf> = ctx.campaign_dir.cloned();

        ui.heading(format!("Edit: {}", heading_name));
        ui.separator();

        // Reserve ~44 px for the separator + button row below so they stay visible.
        let footer_reserved = 44.0;
        let scroll_max_height = (ui.available_height() - footer_reserved).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("terrain_editor_edit_scroll")
            .max_height(scroll_max_height)
            .show(ui, |ui| {
                egui::Grid::new("terrain_editor_edit_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("ID:");
                        ui.label(buf.id.to_string());
                        ui.end_row();

                        ui.label("Name:");
                        ui.text_edit_singleline(&mut buf.name);
                        ui.end_row();

                        ui.label("Texture Path:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut buf.texture_path);
                            if ui.small_button("Browse…").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                    .pick_file()
                                {
                                    // Store a campaign-relative path so that
                                    // terrain.ron is portable across machines.
                                    buf.texture_path =
                                        make_relative(&path, campaign_dir_owned.as_ref());
                                }
                            }
                        });
                        ui.end_row();

                        ui.label("Roughness:");
                        ui.add(egui::Slider::new(&mut buf.roughness, 0.0..=1.0).step_by(0.01));
                        ui.end_row();

                        // SDK Rule 3: ComboBox must use from_id_salt.
                        ui.label("Mesh Style:");
                        egui::ComboBox::from_id_salt("terrain_edit_mesh_style")
                            .selected_text(mesh_style_name(buf.mesh_style))
                            .show_ui(ui, |ui| {
                                // SDK Rule 1: push_id in every loop body.
                                for (i, style) in TerrainMeshStyle::all().iter().enumerate() {
                                    ui.push_id(i, |ui| {
                                        ui.selectable_value(
                                            &mut buf.mesh_style,
                                            *style,
                                            mesh_style_name(*style),
                                        );
                                    });
                                }
                            });
                        ui.end_row();

                        ui.label("Vegetation:");
                        egui::ComboBox::from_id_salt("terrain_edit_vegetation")
                            .selected_text(vegetation_name(buf.vegetation))
                            .show_ui(ui, |ui| {
                                for (i, veg) in TerrainVegetation::all().iter().enumerate() {
                                    ui.push_id(i, |ui| {
                                        ui.selectable_value(
                                            &mut buf.vegetation,
                                            *veg,
                                            vegetation_name(*veg),
                                        );
                                    });
                                }
                            });
                        ui.end_row();

                        ui.label("Blocked:");
                        ui.checkbox(&mut buf.blocked, "Blocks party movement");
                        ui.end_row();

                        ui.label("Height:");
                        ui.add(egui::Slider::new(&mut buf.height, 0.0..=5.0).step_by(0.1));
                        ui.end_row();

                        ui.label("Color R:");
                        ui.add(egui::Slider::new(&mut buf.color[0], 0.0..=1.0).step_by(0.01));
                        ui.end_row();

                        ui.label("Color G:");
                        ui.add(egui::Slider::new(&mut buf.color[1], 0.0..=1.0).step_by(0.01));
                        ui.end_row();

                        ui.label("Color B:");
                        ui.add(egui::Slider::new(&mut buf.color[2], 0.0..=1.0).step_by(0.01));
                        ui.end_row();

                        ui.label("Preview:");
                        if let Some(tex) = &edit_texture {
                            ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(64.0, 64.0)));
                        } else {
                            // Fallback color swatch when texture file is not on disk.
                            let swatch_size = egui::vec2(48.0, 48.0);
                            let (rect, _) =
                                ui.allocate_exact_size(swatch_size, egui::Sense::hover());
                            let fill = egui::Color32::from_rgb(
                                (buf.color[0] * 255.0) as u8,
                                (buf.color[1] * 255.0) as u8,
                                (buf.color[2] * 255.0) as u8,
                            );
                            ui.painter().rect_filled(rect, 2.0, fill);
                            ui.label(egui::RichText::new("(texture not found)").small().weak());
                        }
                        ui.end_row();
                    });
            });

        ui.separator();

        // SDK Rule 16: edit screens must end with Back to List / Save / Cancel.
        // SDK Rule 12: horizontal_wrapped so buttons don't clip on narrow windows.
        ui.horizontal_wrapped(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.mode = TerrainEditorMode::List;
                ui.ctx().request_repaint();
            }
            if ui.button("💾 Save").clicked() {
                // Normalize the edit buffer's texture path before committing
                // it to defs so that any absolute path (typed, pasted, or
                // loaded from an older terrain.ron) is stripped to the
                // campaign-relative form.
                if let Some(b) = self.edit_buffer.as_mut() {
                    b.texture_path = make_relative(
                        std::path::Path::new(&b.texture_path),
                        campaign_dir_owned.as_ref(),
                    );
                }
                self.apply_edit(defs);

                // Normalize ALL in-memory definitions so that entries which
                // were loaded from an older terrain.ron (e.g. Sand, Ice) and
                // never opened for editing also have relative paths before
                // the inline write below and before the next full Campaign Save.
                for def in defs.iter_mut() {
                    def.texture_path = make_relative(
                        std::path::Path::new(&def.texture_path),
                        campaign_dir_owned.as_ref(),
                    );
                }
                *ctx.unsaved_changes = true;

                // Immediate best-effort persist so changes survive a crash
                // without requiring a full Campaign Save.
                if let Some(dir) = ctx.campaign_dir {
                    // Invalidate the cached texture for this path so the
                    // detail panel reflects any newly-saved texture immediately.
                    let terrain_path = dir.join(ctx.data_file);
                    let ron_config = ron::ser::PrettyConfig::new()
                        .struct_names(false)
                        .enumerate_arrays(false);
                    if let Ok(contents) = ron::ser::to_string_pretty(&*defs, ron_config) {
                        if let Err(e) = std::fs::write(&terrain_path, contents) {
                            eprintln!("Failed to write terrain.ron: {e}");
                        }
                    }
                    // Bust the texture cache for the just-saved path so the
                    // preview reloads the new texture on the next list render.
                    self.terrain_texture_cache.0.remove(&texture_path);
                }

                self.mode = TerrainEditorMode::List;
                ui.ctx().request_repaint();
            }
            if ui.button("✕ Cancel").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.mode = TerrainEditorMode::List;
                ui.ctx().request_repaint();
            }
        });
    }

    fn filtered_rows(&self, defs: &[TerrainDefinition]) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        defs.iter()
            .enumerate()
            .filter(|(_, def)| query.is_empty() || def.name.to_lowercase().contains(&query))
            .map(|(idx, _)| idx)
            .collect()
    }
}

// ─── Preview panel ────────────────────────────────────────────────────────────

/// Renders the right-column preview for a selected terrain definition.
///
/// Displays the terrain texture (if available) followed by a property grid.
/// When no texture handle is available (missing file or no campaign open) a
/// color-tinted rectangle derived from `def.color` is shown instead.
fn show_terrain_preview(
    ui: &mut egui::Ui,
    def: Option<&TerrainDefinition>,
    texture: Option<&egui::TextureHandle>,
) {
    let Some(def) = def else {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Select a terrain definition to preview.")
                    .weak()
                    .italics(),
            );
        });
        return;
    };

    ui.heading(format!("🗺 {}", def.name));
    ui.separator();

    // ── Texture preview image (or color-swatch fallback) ──────────────────────
    let available_w = ui.available_width();
    let img_sz = available_w.min(180.0);
    if let Some(tex) = texture {
        ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(img_sz, img_sz)));
    } else {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(img_sz, img_sz), egui::Sense::hover());
        let fill = egui::Color32::from_rgb(
            (def.color[0] * 255.0) as u8,
            (def.color[1] * 255.0) as u8,
            (def.color[2] * 255.0) as u8,
        );
        ui.painter().rect_filled(rect, 4.0, fill);
        ui.label(
            egui::RichText::new("(texture not available)")
                .small()
                .weak(),
        );
    }
    ui.add_space(6.0);

    // ── Property grid ─────────────────────────────────────────────────────────
    egui::Grid::new("terrain_preview_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("ID:");
            ui.label(def.id.to_string());
            ui.end_row();

            ui.label("Texture:");
            ui.label(
                egui::RichText::new(&def.texture_path)
                    .small()
                    .monospace()
                    .color(egui::Color32::GRAY),
            );
            ui.end_row();

            ui.label("Roughness:");
            ui.label(format!("{:.2}", def.roughness));
            ui.end_row();

            ui.label("Mesh Style:");
            ui.label(mesh_style_name(def.mesh_style));
            ui.end_row();

            ui.label("Vegetation:");
            ui.label(vegetation_name(def.vegetation));
            ui.end_row();

            ui.label("Blocked:");
            ui.label(if def.blocked { "Yes" } else { "No" });
            ui.end_row();

            ui.label("Height:");
            ui.label(format!("{:.1}", def.height));
            ui.end_row();

            ui.label("Color:");
            let swatch_size = egui::vec2(40.0, 16.0);
            let (rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
            let fill = egui::Color32::from_rgb(
                (def.color[0] * 255.0) as u8,
                (def.color[1] * 255.0) as u8,
                (def.color[2] * 255.0) as u8,
            );
            ui.painter().rect_filled(rect, 2.0, fill);
            ui.end_row();
        });
}

// ─── ID assignment ────────────────────────────────────────────────────────────

/// Returns the next available campaign-defined terrain ID (≥ [`CAMPAIGN_TERRAIN_START`]).
///
/// Finds the highest existing campaign-terrain ID (those ≥ 13 100) and returns
/// that value plus one.  If no campaign-defined terrain exists yet, returns
/// exactly [`CAMPAIGN_TERRAIN_START`] (13 100).
fn next_available_id(defs: &[TerrainDefinition]) -> TerrainId {
    let max_id = defs
        .iter()
        .filter(|d| d.id >= CAMPAIGN_TERRAIN_START)
        .map(|d| d.id)
        .max()
        .unwrap_or(CAMPAIGN_TERRAIN_START - 1);
    max_id + 1
}

// ─── Display helpers ──────────────────────────────────────────────────────────

/// Returns a human-readable label for a [`TerrainMeshStyle`] variant.
fn mesh_style_name(style: TerrainMeshStyle) -> &'static str {
    match style {
        TerrainMeshStyle::Flat => "Flat",
        TerrainMeshStyle::Water => "Water",
        TerrainMeshStyle::Mountain => "Mountain",
    }
}

/// Returns a human-readable label for a [`TerrainVegetation`] variant.
fn vegetation_name(veg: TerrainVegetation) -> &'static str {
    match veg {
        TerrainVegetation::None => "None",
        TerrainVegetation::GrassCover => "Grass Cover",
        TerrainVegetation::Forest => "Forest",
    }
}

/// Converts an absolute path to one relative to `base`.
///
/// When `base` is `Some` and `abs_path` is under it, the leading `base/`
/// prefix is stripped and the remainder is returned as a `String`.  If
/// `base` is `None`, or `abs_path` is not under `base`, the original path
/// is returned unchanged (as a lossy UTF-8 string).
///
/// This keeps `texture_path` entries in `terrain.ron` portable: the editor
/// stores campaign-relative paths (e.g. `"assets/textures/terrain/snow.png"`)
/// instead of absolute machine-specific paths.
///
/// # Examples
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// let base = PathBuf::from("/home/user/campaigns/tutorial");
/// let abs  = Path::new("/home/user/campaigns/tutorial/assets/textures/terrain/snow.png");
/// // Path is under base → stripped to relative form.
/// // (In real usage this comes from rfd::FileDialog::pick_file.)
/// ```
pub(crate) fn make_relative(
    abs_path: &std::path::Path,
    base: Option<&std::path::PathBuf>,
) -> String {
    base.and_then(|b| abs_path.strip_prefix(b).ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| abs_path.to_string_lossy().into_owned())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience constructor for test terrain definitions.
    fn terrain_definition(id: TerrainId, name: &str) -> TerrainDefinition {
        TerrainDefinition {
            id,
            name: name.to_string(),
            texture_path: format!(
                "assets/textures/terrain/{}.png",
                name.to_lowercase().replace(' ', "_")
            ),
            roughness: 0.5,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.5, 0.5, 0.5],
        }
    }

    #[test]
    fn test_terrain_editor_new_defaults_empty_state() {
        let state = TerrainEditorState::new();
        assert!(state.search_query.is_empty());
        assert_eq!(state.selected_terrain, None);
        assert_eq!(state.mode, TerrainEditorMode::List);
        assert!(state.edit_buffer.is_none());
        assert!(state.edit_index.is_none());
        assert!(
            state.needs_initial_load,
            "new() must set needs_initial_load = true"
        );
    }

    #[test]
    fn test_reset_for_new_campaign_sets_needs_initial_load() {
        let mut state = TerrainEditorState::new();
        // Simulate mid-session state
        state.needs_initial_load = false;
        state.search_query = "grass".to_string();
        state.selected_terrain = Some(2);

        state.reset_for_new_campaign();

        assert!(state.needs_initial_load);
        assert!(state.search_query.is_empty());
        assert_eq!(state.selected_terrain, None);
        assert_eq!(state.mode, TerrainEditorMode::List);
        assert!(state.edit_buffer.is_none());
        assert!(state.edit_index.is_none());
    }

    #[test]
    fn test_enter_edit_populates_buffers() {
        let defs = vec![terrain_definition(13100, "Custom Grass")];
        let mut state = TerrainEditorState::new();
        state.enter_edit(0, &defs);

        assert_eq!(state.mode, TerrainEditorMode::Edit);
        assert_eq!(state.edit_index, Some(0));
        assert!(state.edit_buffer.is_some());
        assert_eq!(state.edit_buffer.as_ref().unwrap().id, 13100);
        assert_eq!(state.edit_buffer.as_ref().unwrap().name, "Custom Grass");
    }

    #[test]
    fn test_apply_edit_writes_back_to_defs() {
        let mut defs = vec![terrain_definition(13100, "Custom Grass")];
        let mut state = TerrainEditorState::new();
        state.enter_edit(0, &defs);

        // Mutate the buffer as the user would in the edit form.
        state.edit_buffer.as_mut().unwrap().name = "Lush Meadow".to_string();
        state.edit_buffer.as_mut().unwrap().blocked = true;
        state.edit_buffer.as_mut().unwrap().roughness = 0.9;

        state.apply_edit(&mut defs);

        assert_eq!(defs[0].name, "Lush Meadow");
        assert!(defs[0].blocked);
        assert!((defs[0].roughness - 0.9).abs() < f32::EPSILON);
        // apply_edit consumes the buffer.
        assert!(state.edit_buffer.is_none());
        assert!(state.edit_index.is_none());
    }

    #[test]
    fn test_delete_clears_selection_when_selected_item_removed() {
        let mut defs = vec![
            terrain_definition(13100, "Grass"),
            terrain_definition(13101, "Lava"),
            terrain_definition(13102, "Ice"),
        ];
        let mut state = TerrainEditorState::new();
        state.selected_terrain = Some(1); // Lava selected

        let mut unsaved = false;
        state.remove_terrain_definition(1, &mut defs, &mut unsaved);

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "Grass");
        assert_eq!(defs[1].name, "Ice");
        assert_eq!(state.selected_terrain, None);
        assert!(unsaved);
    }

    #[test]
    fn test_delete_shifts_selection_down_when_earlier_item_removed() {
        let mut defs = vec![
            terrain_definition(13100, "Grass"),
            terrain_definition(13101, "Lava"),
            terrain_definition(13102, "Ice"),
        ];
        let mut state = TerrainEditorState::new();
        state.selected_terrain = Some(2); // Ice selected

        let mut unsaved = false;
        state.remove_terrain_definition(0, &mut defs, &mut unsaved);

        assert_eq!(defs.len(), 2);
        // Selection shifted from 2 → 1 after removing index 0.
        assert_eq!(state.selected_terrain, Some(1));
        assert_eq!(defs[1].name, "Ice");
    }

    #[test]
    fn test_delete_leaves_selection_unchanged_when_later_item_removed() {
        let mut defs = vec![
            terrain_definition(13100, "Grass"),
            terrain_definition(13101, "Lava"),
            terrain_definition(13102, "Ice"),
        ];
        let mut state = TerrainEditorState::new();
        state.selected_terrain = Some(0); // Grass selected

        let mut unsaved = false;
        state.remove_terrain_definition(2, &mut defs, &mut unsaved);

        // Selection at 0 is unaffected by removing index 2.
        assert_eq!(defs.len(), 2);
        assert_eq!(state.selected_terrain, Some(0));
    }

    #[test]
    fn test_next_available_id_returns_13100_when_no_campaign_terrain() {
        let defs: Vec<TerrainDefinition> = Vec::new();
        assert_eq!(next_available_id(&defs), CAMPAIGN_TERRAIN_START);
    }

    #[test]
    fn test_next_available_id_increments_beyond_existing() {
        let defs = vec![
            terrain_definition(13100, "A"),
            terrain_definition(13101, "B"),
            terrain_definition(13103, "C"), // gap at 13102
        ];
        // max is 13103, so next is 13104
        assert_eq!(next_available_id(&defs), 13104);
    }

    #[test]
    fn test_make_relative_strips_campaign_dir_prefix() {
        let base = std::path::PathBuf::from("/home/user/campaigns/tutorial");
        let abs =
            std::path::Path::new("/home/user/campaigns/tutorial/assets/textures/terrain/snow.png");
        assert_eq!(
            make_relative(abs, Some(&base)),
            "assets/textures/terrain/snow.png",
        );
    }

    #[test]
    fn test_make_relative_returns_original_when_not_under_base() {
        let base = std::path::PathBuf::from("/home/user/campaigns/tutorial");
        let abs = std::path::Path::new("/tmp/other_texture.png");
        assert_eq!(make_relative(abs, Some(&base)), "/tmp/other_texture.png",);
    }

    #[test]
    fn test_make_relative_returns_original_when_no_base() {
        let abs = std::path::Path::new("/some/absolute/path.png");
        assert_eq!(make_relative(abs, None), "/some/absolute/path.png");
    }

    #[test]
    fn test_terrain_definition_roundtrip() {
        let def = TerrainDefinition {
            id: 13100,
            name: "Custom Lava".to_string(),
            texture_path: "assets/textures/terrain/custom_lava.png".to_string(),
            roughness: 0.75,
            mesh_style: TerrainMeshStyle::Water,
            vegetation: TerrainVegetation::GrassCover,
            blocked: true,
            height: 1.5,
            color: [0.8, 0.2, 0.1],
        };

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);
        let ron_str = ron::ser::to_string_pretty(&def, ron_config).unwrap();
        let parsed: TerrainDefinition = ron::from_str(&ron_str).unwrap();

        assert_eq!(parsed.id, def.id);
        assert_eq!(parsed.name, def.name);
        assert_eq!(parsed.texture_path, def.texture_path);
        assert!((parsed.roughness - def.roughness).abs() < f32::EPSILON);
        assert_eq!(parsed.mesh_style, def.mesh_style);
        assert_eq!(parsed.vegetation, def.vegetation);
        assert_eq!(parsed.blocked, def.blocked);
        assert!((parsed.height - def.height).abs() < f32::EPSILON);
        assert_eq!(parsed.color, def.color);
    }
}
