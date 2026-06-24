// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Objects editor tab — registry list and edit form for `object_mesh_registry.ron`.
//!
//! Scope: this editor manages registry-level and material-level properties of
//! an Object entry — its string key, display name, scale, color tint, and
//! per-mesh PBR material values (base color, metallic, roughness, emissive).
//! It does **not** edit vertex/index/UV geometry or add/remove meshes; that
//! stays the Importer's job. The "↻ Re-import in Importer" button in the edit
//! form and the "📥 Import Object Mesh" button in the list view both hand off
//! to the Importer tab for that work.
//!
//! Modeled on [`crate::landscape_editor`] for the List/Edit mode skeleton, and
//! on [`crate::item_mesh_editor`] for the fact that an [`ObjectEntry`] *is* a
//! mesh asset directly (a [`CreatureDefinition`]), not a wrapper around one
//! with a separate category/tags/flags layer.
//!
//! Rule compliance (`sdk/AGENTS.md`): the list-row loop and the per-mesh
//! material loop in the edit form both wrap their bodies in `push_id` (Rule
//! 1); every `ScrollArea` and `Grid` has a distinct `id_salt`/name (Rule 2,
//! Rule 5); every state-changing click or toggle calls `request_repaint()`
//! (Rule 7); the list/preview split uses [`crate::ui_helpers::TwoColumnLayout`]
//! rather than a raw `SidePanel` (Rule 9); `show_list` pre-computes
//! `filtered_rows` and `preview_snapshot` before calling `show_split` (Rule
//! 10); button rows use `horizontal_wrapped` (Rule 12); `needs_initial_load`
//! drives a lazy auto-load on first render, reset via
//! [`reset_for_new_campaign`](ObjectsEditorState::reset_for_new_campaign) and
//! [`reset_selection`](ObjectsEditorState::reset_selection) (Rule 13); the
//! left-panel list uses `show_standard_list_item` with `MetadataBadge`
//! instead of `selectable_label` (Rule 15); and the edit form ends with Back
//! to List / Save / Cancel, in that order (Rule 16). The Key field is
//! deliberately exempt from Rule 14's autocomplete-selector requirement: it
//! is the primary identifier being assigned, not a reference to another
//! registry. Rules 3, 4, 6, and 8 do not apply — this file uses no
//! `ComboBox`, `CollapsingHeader`, or `egui::Window`, and registers no
//! top-level panel of its own.
//!
//! Integration: [`ObjectsEditorState`] lives at
//! `EditorRegistry::objects_editor_state` and the data it renders lives at
//! `CampaignData::objects: Vec<ObjectEntry>` (both in `editor_state.rs`); this
//! module never owns that `Vec` itself — [`ObjectsEditorState::show`] only
//! ever receives `&mut Vec<ObjectEntry>` from the caller. Loading and saving
//! `data/object_mesh_registry.ron` happens via `CampaignBuilderApp::load_objects`
//! and `save_objects` in `campaign_io.rs`, wired into the same campaign
//! new/open/save lifecycle as every other data type. `lib.rs`'s
//! `EditorTab::Objects` arm is the sole caller of `show()`; when the OBJ
//! Importer reports an `ObjectMesh` export, that same handler calls
//! `load_objects()` again and switches to `EditorTab::Objects`, so a freshly
//! imported mesh shows up here immediately, with no save-and-reopen cycle.

use crate::ui_helpers::{
    show_standard_list_item, ItemAction, MetadataBadge, StandardListItemConfig, TwoColumnLayout,
};
use antares::domain::visual::{CreatureDefinition, MaterialDefinition};
use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
use eframe::egui;
use std::path::Path;

/// Signal requested by the Objects editor.
///
/// # Examples
///
/// ```
/// use campaign_builder::objects_editor::ObjectsEditorSignal;
///
/// let signal = ObjectsEditorSignal::OpenInObjImporter;
/// assert_eq!(signal, ObjectsEditorSignal::OpenInObjImporter);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectsEditorSignal {
    /// Switch to the Importer tab and prepare it for Object Mesh export.
    OpenInObjImporter,
}

#[derive(Debug, Default, PartialEq)]
enum ObjectsEditorMode {
    #[default]
    List,
    Edit,
}

/// One row in the Objects registry — a string key paired with the
/// [`CreatureDefinition`] loaded from `file_path`.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::CreatureDefinition;
/// use campaign_builder::objects_editor::ObjectEntry;
///
/// let entry = ObjectEntry {
///     key: "barred_door".to_string(),
///     file_path: "assets/meshes/objects/barred_door.ron".to_string(),
///     definition: CreatureDefinition::default(),
/// };
/// assert_eq!(entry.key, "barred_door");
/// ```
#[derive(Debug, Clone)]
pub struct ObjectEntry {
    /// The registry key — a human-readable string identifier (e.g. `"barred_door"`).
    pub key: String,
    /// Path to the `CreatureDefinition` RON file, relative to the campaign root.
    pub file_path: String,
    /// The mesh definition loaded from (or about to be saved to) `file_path`.
    pub definition: CreatureDefinition,
}

/// View state for the Objects editor tab.
///
/// # Examples
///
/// ```
/// use campaign_builder::objects_editor::ObjectsEditorState;
///
/// let state = ObjectsEditorState::new();
/// assert!(state.search_query.is_empty());
/// assert!(state.needs_initial_load);
/// ```
#[derive(Debug, Default)]
pub struct ObjectsEditorState {
    /// Search text used to filter entries by key or resolved name.
    pub search_query: String,
    /// Selected entry index in the visible source vector.
    pub selected: Option<usize>,
    /// Deferred signal consumed by `CampaignBuilderApp`.
    pub requested_signal: Option<ObjectsEditorSignal>,
    /// Whether entries should be auto-loaded on the next `show()` call.
    ///
    /// Set to `true` whenever the campaign changes (new / open) by
    /// [`reset_for_new_campaign`](Self::reset_for_new_campaign) so that
    /// `show()` performs a lazy auto-load the first time the Objects tab is
    /// rendered. Cleared once the load attempt completes (success or
    /// file-not-found — never retried every frame). See `sdk/AGENTS.md`
    /// Rule 13.
    pub needs_initial_load: bool,

    // Edit mode fields.
    mode: ObjectsEditorMode,
    edit_index: Option<usize>,
    /// Key text being edited. This is the one field exempt from `sdk/AGENTS.md`
    /// Rule 14 — it is the primary key being assigned, not a reference *to*
    /// another registry.
    key_buffer: String,
    edit_buffer: Option<CreatureDefinition>,
    /// Mirrors whether `edit_buffer.color_tint` is `Some`. Toggling this on
    /// initializes `color_tint` to opaque white; toggling it off clears it to
    /// `None` (same empty-means-None convention as `landscape_editor.rs`'s
    /// `description_buffer`/`icon_buffer`, applied to a boolean + array
    /// instead of a string).
    color_tint_enabled: bool,
    /// Set when a rename collides with an existing key; rendered inline above
    /// the action row instead of saving.
    key_error: Option<String>,
}

impl ObjectsEditorState {
    /// Creates a fresh Objects editor state.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::objects_editor::ObjectsEditorState;
    ///
    /// let state = ObjectsEditorState::new();
    /// assert!(state.search_query.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            needs_initial_load: true,
            ..Self::default()
        }
    }

    /// Renders the Objects registry list and selected entry preview, or the
    /// edit form when in Edit mode.
    ///
    /// On the first call after a campaign change (`needs_initial_load`),
    /// rebuilds `entries` from `data/object_mesh_registry.ron` if
    /// `campaign_dir` is `Some` (`sdk/AGENTS.md` Rule 13 auto-load guard).
    /// With `campaign_dir: None` this guard is a no-op and `entries` is left
    /// untouched — this is what keeps isolated-fixture testing valid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use campaign_builder::objects_editor::ObjectsEditorState;
    ///
    /// # fn render(ui: &mut eframe::egui::Ui) {
    /// let mut state = ObjectsEditorState::new();
    /// let mut entries = Vec::new();
    /// let mut unsaved = false;
    /// state.show(ui, &mut entries, None, &mut unsaved);
    /// # }
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        entries: &mut Vec<ObjectEntry>,
        campaign_dir: Option<&Path>,
        unsaved_changes: &mut bool,
    ) {
        if self.needs_initial_load {
            if let Some(dir) = campaign_dir {
                *entries = load_object_entries_from_registry(dir);
            }
            self.needs_initial_load = false;
        }

        ui.heading("📦 Objects");
        ui.separator();

        if self.mode == ObjectsEditorMode::Edit {
            self.show_edit(ui, entries, campaign_dir, unsaved_changes);
        } else {
            self.show_list(ui, entries, campaign_dir, unsaved_changes);
        }
    }

    /// Reset editor state for a new or freshly-opened campaign.
    ///
    /// Clears search/selection/edit state and sets `needs_initial_load = true`
    /// so that `show()` performs an auto-load the first time the Objects tab
    /// is rendered. Call this from both `do_new_campaign` and
    /// `do_open_campaign` (before the explicit `load_objects()` call) —
    /// `sdk/AGENTS.md` Rule 13.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::objects_editor::ObjectsEditorState;
    ///
    /// let mut state = ObjectsEditorState::default();
    /// state.needs_initial_load = false;
    /// state.reset_for_new_campaign();
    /// assert!(state.needs_initial_load);
    /// ```
    pub fn reset_for_new_campaign(&mut self) {
        self.search_query.clear();
        self.selected = None;
        self.mode = ObjectsEditorMode::List;
        self.edit_index = None;
        self.edit_buffer = None;
        self.key_buffer.clear();
        self.color_tint_enabled = false;
        self.key_error = None;
        self.needs_initial_load = true;
    }

    /// Reset transient selection/edit state without touching
    /// `needs_initial_load`.
    ///
    /// `ObjectsEditorState` does not own `Vec<ObjectEntry>` — it receives
    /// `&mut Vec<ObjectEntry>` by reference each frame, same architecture as
    /// `LandscapeEditorState`. Call this every time the campaign-level
    /// `load_objects` rebuilds that `Vec` (including importer-triggered
    /// reloads), so a stale `edit_index` can never point at the wrong entry
    /// after the `Vec` changes out from under an in-progress edit. Unlike
    /// [`reset_for_new_campaign`](Self::reset_for_new_campaign), this must
    /// **not** flip `needs_initial_load` back to `true` — that would
    /// re-trigger an unwanted auto-load on every importer reload of an
    /// already-open campaign.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::objects_editor::ObjectsEditorState;
    ///
    /// let mut state = ObjectsEditorState::default();
    /// state.needs_initial_load = false;
    /// state.selected = Some(2);
    /// state.reset_selection();
    /// assert_eq!(state.selected, None);
    /// assert!(!state.needs_initial_load, "must not touch needs_initial_load");
    /// ```
    pub fn reset_selection(&mut self) {
        self.selected = None;
        self.mode = ObjectsEditorMode::List;
        self.edit_index = None;
        self.edit_buffer = None;
        self.key_buffer.clear();
        self.color_tint_enabled = false;
        self.key_error = None;
    }

    // =========================================================================
    // List view
    // =========================================================================

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        entries: &mut Vec<ObjectEntry>,
        campaign_dir: Option<&Path>,
        unsaved_changes: &mut bool,
    ) {
        // SDK Rule 12: more than two controls on one row must wrap.
        ui.horizontal_wrapped(|ui| {
            ui.label("Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                ui.ctx().request_repaint();
            }
            ui.separator();
            ui.label(format!("{} object(s)", entries.len()));
            ui.separator();
            if ui.button("📥 Import Object Mesh").clicked() {
                self.requested_signal = Some(ObjectsEditorSignal::OpenInObjImporter);
                ui.ctx().request_repaint();
            }
        });

        // SDK Rule 10: pre-compute shared state before multi-closure calls.
        let filtered_rows = self.filtered_rows(entries);
        let selected_idx = self.selected;
        let preview_snapshot: Option<ObjectEntry> = selected_idx
            .filter(|&i| i < entries.len())
            .map(|i| entries[i].clone());

        // Deferred mutations applied after show_split (Rule 10).
        let mut pending_selection: Option<usize> = None;
        let mut pending_edit: Option<usize> = None;
        let mut pending_delete: Option<usize> = None;

        // SDK Rule 9: always use TwoColumnLayout for list/detail splits.
        // TwoColumnLayout::show_split already wraps both columns in a
        // ScrollArea, so the closures must NOT add another ScrollArea inside.
        TwoColumnLayout::new("objects_editor").show_split(
            ui,
            |left_ui| {
                for &idx in &filtered_rows {
                    let entry = &entries[idx];
                    // SDK Rule 1: the string key is the stable unique
                    // identifier here, unlike Landscape's numeric `id`.
                    left_ui.push_id(&entry.key, |ui| {
                        let selected = selected_idx == Some(idx);
                        let (clicked, action) = show_standard_list_item(
                            ui,
                            StandardListItemConfig::new(&entry.definition.name)
                                .selected(selected)
                                .with_badges(vec![MetadataBadge::new(&entry.key)]),
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

                if filtered_rows.is_empty() {
                    left_ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("No objects found.").weak().italics());
                    });
                }
            },
            |right_ui| {
                show_object_preview(right_ui, preview_snapshot.as_ref());
            },
        );

        // Apply deferred mutations after show_split — no active closure borrows.
        if let Some(idx) = pending_selection {
            self.selected = Some(idx);
        }
        if let Some(idx) = pending_edit {
            self.selected = Some(idx);
            self.enter_edit(idx, entries);
        }
        if let Some(idx) = pending_delete {
            if idx < entries.len() {
                entries.remove(idx);
                // Adjust the stored selection so it stays valid after removal.
                // The on-disk registry is reconciled by `save_objects` (Phase
                // 3), which rewrites it wholesale from the current `entries`
                // — no immediate registry write is needed here.
                match self.selected {
                    Some(sel) if sel == idx => self.selected = None,
                    Some(sel) if sel > idx => self.selected = Some(sel - 1),
                    _ => {}
                }
                let _ = campaign_dir; // reserved for a future immediate-delete sync, if ever needed
                *unsaved_changes = true;
                ui.ctx().request_repaint();
            }
        }
    }

    // =========================================================================
    // Edit view
    // =========================================================================

    fn enter_edit(&mut self, idx: usize, entries: &[ObjectEntry]) {
        if idx >= entries.len() {
            return;
        }
        let entry = &entries[idx];
        self.edit_index = Some(idx);
        self.key_buffer = entry.key.clone();
        self.color_tint_enabled = entry.definition.color_tint.is_some();
        self.key_error = None;
        self.edit_buffer = Some(entry.definition.clone());
        self.mode = ObjectsEditorMode::Edit;
    }

    /// Validates the key and, if valid, writes `edit_buffer` back into
    /// `entries[edit_index]` (updating the key too, if it changed).
    ///
    /// Returns `true` on success (and clears `edit_index`/`edit_buffer`).
    /// Returns `false` and sets `key_error` — leaving `entries` and the edit
    /// buffer untouched — when the key is empty or collides with another
    /// entry's key.
    fn apply_edit(&mut self, entries: &mut [ObjectEntry]) -> bool {
        let Some(idx) = self.edit_index else {
            return false;
        };
        let Some(buf) = self.edit_buffer.as_ref() else {
            return false;
        };
        if idx >= entries.len() {
            return false;
        }

        let new_key = self.key_buffer.trim().to_string();
        if new_key.is_empty() {
            self.key_error = Some("Key cannot be empty.".to_string());
            return false;
        }
        let collides = entries
            .iter()
            .enumerate()
            .any(|(i, e)| i != idx && e.key == new_key);
        if collides {
            self.key_error = Some(format!("Key '{new_key}' is already in use."));
            return false;
        }

        entries[idx].key = new_key;
        entries[idx].definition = buf.clone();
        self.key_error = None;
        self.edit_index = None;
        self.edit_buffer = None;
        true
    }

    fn show_edit(
        &mut self,
        ui: &mut egui::Ui,
        entries: &mut [ObjectEntry],
        campaign_dir: Option<&Path>,
        unsaved_changes: &mut bool,
    ) {
        let Some(buf) = self.edit_buffer.as_mut() else {
            // Shouldn't happen — guard and fall back to list mode.
            self.mode = ObjectsEditorMode::List;
            return;
        };

        ui.heading(format!("Edit: {}", buf.name));
        ui.separator();

        let footer_reserved = 44.0;
        let scroll_max_height = (ui.available_height() - footer_reserved).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("objects_editor_edit_scroll")
            .max_height(scroll_max_height)
            .show(ui, |ui| {
                egui::Grid::new("objects_editor_edit_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Key:");
                        ui.text_edit_singleline(&mut self.key_buffer);
                        ui.end_row();

                        ui.label("Name:");
                        ui.text_edit_singleline(&mut buf.name);
                        ui.end_row();

                        ui.label("Scale:");
                        ui.add(
                            egui::DragValue::new(&mut buf.scale)
                                .speed(0.01)
                                .range(0.001..=100.0),
                        );
                        ui.end_row();

                        ui.label("Color tint:");
                        ui.horizontal(|ui| {
                            let mut enabled = self.color_tint_enabled;
                            if ui.checkbox(&mut enabled, "").changed() {
                                self.color_tint_enabled = enabled;
                                if enabled {
                                    buf.color_tint.get_or_insert([1.0, 1.0, 1.0, 1.0]);
                                } else {
                                    buf.color_tint = None;
                                }
                                ui.ctx().request_repaint();
                            }
                            if self.color_tint_enabled {
                                if let Some(tint) = buf.color_tint.as_mut() {
                                    ui.color_edit_button_rgba_unmultiplied(tint);
                                }
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Per-mesh material").strong());
                    if ui.button("↻ Re-import in Importer").clicked() {
                        self.requested_signal = Some(ObjectsEditorSignal::OpenInObjImporter);
                        ui.ctx().request_repaint();
                    }
                });
                ui.add_space(4.0);

                if buf.meshes.is_empty() {
                    ui.label(
                        egui::RichText::new("This object has no meshes.")
                            .weak()
                            .italics(),
                    );
                }

                // SDK Rule 1: loop body wrapped in push_id with a stable key
                // (mesh index — stable for the lifetime of this edit session
                // since meshes are never added/removed/reordered here).
                //
                // Deliberately NOT egui::Grid here, despite the plan's literal
                // wording: each row has interactive widgets (checkbox,
                // DragValue, color pickers), and push_id + Grid::end_row()
                // inside the same closure is an untested combination in this
                // codebase. obj_importer_ui.rs's per-mesh color editor solves
                // this exact problem (interactive per-mesh rows) with
                // push_id + group + horizontal_wrapped, which this mirrors.
                let mesh_count = buf.meshes.len();
                for i in 0..mesh_count {
                    ui.push_id(i, |ui| {
                        ui.group(|ui| {
                            let mesh = &mut buf.meshes[i];
                            // SDK plan note: MeshDefinition.material is
                            // Option<MaterialDefinition> — a raw OBJ import
                            // commonly leaves this None. Initialize it before
                            // binding any widgets so the grid is never left
                            // unenterable.
                            mesh.material
                                .get_or_insert_with(MaterialDefinition::default);
                            let material = mesh.material.as_mut().expect("just inserted above");

                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!(
                                    "Mesh: {}",
                                    mesh.name.as_deref().unwrap_or("(unnamed)")
                                ));
                                ui.separator();
                                ui.label("Base color:");
                                ui.color_edit_button_rgba_unmultiplied(&mut material.base_color);
                                ui.separator();
                                ui.label("Metallic:");
                                ui.add(
                                    egui::DragValue::new(&mut material.metallic)
                                        .speed(0.01)
                                        .range(0.0..=1.0),
                                );
                                ui.separator();
                                ui.label("Roughness:");
                                ui.add(
                                    egui::DragValue::new(&mut material.roughness)
                                        .speed(0.01)
                                        .range(0.0..=1.0),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                let mut emissive_enabled = material.emissive.is_some();
                                if ui.checkbox(&mut emissive_enabled, "Emissive").changed() {
                                    if emissive_enabled {
                                        material.emissive.get_or_insert([0.0, 0.0, 0.0]);
                                    } else {
                                        material.emissive = None;
                                    }
                                    ui.ctx().request_repaint();
                                }
                                if let Some(emissive) = material.emissive.as_mut() {
                                    ui.color_edit_button_rgb(emissive);
                                }
                                ui.separator();
                                ui.label(format!(
                                    "Texture: {}",
                                    mesh.texture_path.as_deref().unwrap_or("none")
                                ));
                            });
                        });
                    });
                }
            });

        if let Some(error) = &self.key_error {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        // SDK Rule 16: edit screens must end with Back to List / Save / Cancel.
        // SDK Rule 12: use horizontal_wrapped so buttons don't clip.
        ui.horizontal_wrapped(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.key_error = None;
                self.mode = ObjectsEditorMode::List;
                ui.ctx().request_repaint();
            }
            if ui.button("💾 Save").clicked() {
                let idx_before = self.edit_index;
                let old_key = idx_before
                    .and_then(|i| entries.get(i))
                    .map(|e| e.key.clone());
                let file_path = idx_before
                    .and_then(|i| entries.get(i))
                    .map(|e| e.file_path.clone());

                if self.apply_edit(entries) {
                    *unsaved_changes = true;
                    if let (Some(dir), Some(idx), Some(path)) =
                        (campaign_dir, idx_before, &file_path)
                    {
                        write_object_definition(dir, path, &entries[idx].definition);
                        sync_object_mesh_registry_entry(
                            dir,
                            old_key.as_deref(),
                            &entries[idx].key,
                            path,
                        );
                    }
                    self.mode = ObjectsEditorMode::List;
                }
                // Rule 7: repaint unconditionally — on failure this is what
                // makes the newly-set `key_error` message visible without
                // requiring an extra mouse move (the error label renders
                // earlier in this same function, before this button, so it
                // reflects the *previous* frame's state until a repaint).
                ui.ctx().request_repaint();
            }
            if ui.button("✕ Cancel").clicked() {
                self.edit_buffer = None;
                self.edit_index = None;
                self.key_error = None;
                self.mode = ObjectsEditorMode::List;
                ui.ctx().request_repaint();
            }
        });
    }

    fn filtered_rows(&self, entries: &[ObjectEntry]) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.key.to_lowercase().contains(&query)
                    || entry.definition.name.to_lowercase().contains(&query)
            })
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// Renders the read-only preview panel for a single Object entry.
///
/// Shows a placeholder when `entry` is `None`.
fn show_object_preview(ui: &mut egui::Ui, entry: Option<&ObjectEntry>) {
    let Some(entry) = entry else {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Select an object to preview it.")
                    .weak()
                    .italics(),
            );
        });
        return;
    };

    let def = &entry.definition;
    ui.heading(&def.name);
    ui.separator();
    ui.label(format!("Key: {}", entry.key));
    ui.label(format!("Mesh count: {}", def.meshes.len()));
    ui.label(format!("Scale: {:.3}", def.scale));
    ui.label(format!(
        "Color tint: {}",
        def.color_tint
            .map(|c| format!("[{:.2}, {:.2}, {:.2}, {:.2}]", c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| "None".to_string())
    ));

    if !def.meshes.is_empty() {
        ui.separator();
        // Read-only display: plain `ui.label` calls allocate no persistent
        // egui widget IDs, so no push_id is needed for this Grid's rows.
        egui::Grid::new("objects_editor_preview_mesh_grid")
            .num_columns(3)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mesh").strong());
                ui.label(egui::RichText::new("Vertices").strong());
                ui.label(egui::RichText::new("Texture").strong());
                ui.end_row();
                for mesh in &def.meshes {
                    ui.label(mesh.name.as_deref().unwrap_or("(unnamed)"));
                    ui.label(mesh.vertices.len().to_string());
                    ui.label(mesh.texture_path.as_deref().unwrap_or("—"));
                    ui.end_row();
                }
            });
    }
}

/// Reads `data/object_mesh_registry.ron` and resolves each entry to a full
/// [`ObjectEntry`], skipping (not failing on) any entry whose asset file is
/// missing or unparsable.
///
/// Returns an empty `Vec` when the registry file does not exist or cannot be
/// parsed at all — neither condition is an error from this auto-load guard's
/// perspective (no logger is available here; the App-level `load_objects`,
/// Phase 3, is the primary load path and does log these conditions).
fn load_object_entries_from_registry(campaign_dir: &Path) -> Vec<ObjectEntry> {
    let registry_path = campaign_dir.join("data/object_mesh_registry.ron");
    if !registry_path.exists() {
        return Vec::new();
    }
    let Ok(registry) = ObjectMeshRegistryFile::load(&registry_path) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for (key, path) in registry.meshes {
        let asset_path = campaign_dir.join(&path);
        let Ok(contents) = std::fs::read_to_string(&asset_path) else {
            continue;
        };
        let Ok(definition) = ron::from_str::<CreatureDefinition>(&contents) else {
            continue;
        };
        entries.push(ObjectEntry {
            key,
            file_path: path,
            definition,
        });
    }
    entries
}

/// Writes `definition` back to `campaign_dir.join(file_path)`, creating
/// parent directories if needed.
///
/// Errors are intentionally swallowed — a write failure here must never block
/// the in-memory edit from applying or crash the editor, matching the
/// established convention in `landscape_editor.rs`'s `save_mesh_scale`.
fn write_object_definition(campaign_dir: &Path, file_path: &str, definition: &CreatureDefinition) {
    let full_path = campaign_dir.join(file_path);
    if let Some(parent) = full_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(contents) = ron::ser::to_string_pretty(definition, ron::ser::PrettyConfig::new()) {
        let _ = std::fs::write(&full_path, contents);
    }
}

/// Renames (if the key changed) and/or upserts the registry entry for an
/// edited Object so the on-disk registry reflects the new key/path
/// immediately, without requiring a full "Save Campaign".
///
/// Errors are intentionally swallowed: `save_objects` (Phase 3) rewrites the
/// registry wholesale from the in-memory `Vec<ObjectEntry>` on every campaign
/// save, so a failure here is self-healing and must never block the in-memory
/// edit or crash the editor.
fn sync_object_mesh_registry_entry(
    campaign_dir: &Path,
    old_key: Option<&str>,
    new_key: &str,
    file_path: &str,
) {
    let registry_path = campaign_dir.join("data/object_mesh_registry.ron");
    let mut registry = if registry_path.exists() {
        match ObjectMeshRegistryFile::load(&registry_path) {
            Ok(registry) => registry,
            Err(_) => return,
        }
    } else {
        ObjectMeshRegistryFile::default()
    };

    if let Some(old) = old_key {
        if old != new_key {
            registry.rename(old, new_key);
        }
    }
    registry.upsert(new_key, file_path);
    let _ = registry.save(&registry_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_entry(key: &str, name: &str) -> ObjectEntry {
        ObjectEntry {
            key: key.to_string(),
            file_path: format!("assets/meshes/objects/{key}.ron"),
            definition: CreatureDefinition {
                id: 1,
                name: name.to_string(),
                meshes: Vec::new(),
                mesh_transforms: Vec::new(),
                scale: 1.0,
                color_tint: None,
            },
        }
    }

    #[test]
    fn test_objects_editor_new_defaults() {
        let state = ObjectsEditorState::new();
        assert!(state.search_query.is_empty());
        assert_eq!(state.selected, None);
        assert_eq!(state.requested_signal, None);
        assert!(state.needs_initial_load);
        assert_eq!(state.mode, ObjectsEditorMode::List);
        assert!(state.edit_buffer.is_none());
    }

    #[test]
    fn test_filtered_rows_matches_key_and_name() {
        let entries = vec![
            object_entry("barred_door", "Barred Door"),
            object_entry("treasure_chest", "Treasure Chest"),
        ];
        let mut state = ObjectsEditorState::new();

        state.search_query = "chest".to_string();
        assert_eq!(state.filtered_rows(&entries), vec![1]);

        state.search_query = "barred".to_string();
        assert_eq!(state.filtered_rows(&entries), vec![0]);

        state.search_query = String::new();
        assert_eq!(state.filtered_rows(&entries), vec![0, 1]);
    }

    #[test]
    fn test_enter_edit_populates_buffers() {
        let entries = vec![object_entry("barred_door", "Barred Door")];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);

        assert_eq!(state.mode, ObjectsEditorMode::Edit);
        assert_eq!(state.edit_index, Some(0));
        assert_eq!(state.key_buffer, "barred_door");
        assert!(state.edit_buffer.is_some());
        assert!(!state.color_tint_enabled);
        assert!(state.key_error.is_none());
    }

    #[test]
    fn test_enter_edit_detects_existing_color_tint() {
        let mut entry = object_entry("oak_tree", "Oak Tree");
        entry.definition.color_tint = Some([0.5, 0.5, 0.5, 1.0]);
        let entries = vec![entry];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);

        assert!(state.color_tint_enabled);
    }

    #[test]
    fn test_apply_edit_renames_key_and_updates_definition() {
        let mut entries = vec![object_entry("old_chest", "Old Chest")];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);

        state.key_buffer = "new_chest".to_string();
        state.edit_buffer.as_mut().unwrap().name = "New Chest".to_string();

        assert!(state.apply_edit(&mut entries));
        assert_eq!(entries[0].key, "new_chest");
        assert_eq!(entries[0].definition.name, "New Chest");
        assert!(state.edit_buffer.is_none());
        assert!(state.edit_index.is_none());
        assert!(state.key_error.is_none());
    }

    #[test]
    fn test_apply_edit_rejects_rename_to_existing_key() {
        let mut entries = vec![
            object_entry("old_chest", "Old Chest"),
            object_entry("other_key", "Other"),
        ];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);
        state.key_buffer = "other_key".to_string();

        assert!(!state.apply_edit(&mut entries));
        assert!(state.key_error.is_some());
        // Original entries must be unmodified.
        assert_eq!(entries[0].key, "old_chest");
        assert_eq!(entries[1].key, "other_key");
        // Edit session must still be active (not silently dropped).
        assert!(state.edit_buffer.is_some());
        assert_eq!(state.edit_index, Some(0));
    }

    #[test]
    fn test_apply_edit_rejects_empty_key() {
        let mut entries = vec![object_entry("old_chest", "Old Chest")];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);
        state.key_buffer = "   ".to_string();

        assert!(!state.apply_edit(&mut entries));
        assert!(state.key_error.is_some());
        assert_eq!(entries[0].key, "old_chest");
    }

    #[test]
    fn test_apply_edit_accepts_rename_to_unique_key() {
        let mut entries = vec![
            object_entry("old_chest", "Old Chest"),
            object_entry("other_key", "Other"),
        ];
        let mut state = ObjectsEditorState::new();
        state.enter_edit(0, &entries);
        state.key_buffer = "brand_new_key".to_string();

        assert!(state.apply_edit(&mut entries));
        assert_eq!(entries[0].key, "brand_new_key");
        assert_eq!(entries[1].key, "other_key");
    }

    #[test]
    fn test_delete_clears_selection_when_selected_item_removed() {
        let mut entries = vec![
            object_entry("a", "A"),
            object_entry("b", "B"),
            object_entry("c", "C"),
        ];
        let mut state = ObjectsEditorState::new();
        state.selected = Some(1);

        let idx = 1;
        entries.remove(idx);
        match state.selected {
            Some(sel) if sel == idx => state.selected = None,
            Some(sel) if sel > idx => state.selected = Some(sel - 1),
            _ => {}
        }

        assert_eq!(entries.len(), 2);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn test_delete_shifts_selection_down_when_earlier_item_removed() {
        let mut entries = vec![
            object_entry("a", "A"),
            object_entry("b", "B"),
            object_entry("c", "C"),
        ];
        let mut state = ObjectsEditorState::new();
        state.selected = Some(2);

        let idx = 0;
        entries.remove(idx);
        match state.selected {
            Some(sel) if sel == idx => state.selected = None,
            Some(sel) if sel > idx => state.selected = Some(sel - 1),
            _ => {}
        }

        assert_eq!(entries.len(), 2);
        assert_eq!(state.selected, Some(1));
    }

    #[test]
    fn test_delete_leaves_selection_unchanged_when_later_item_removed() {
        let mut entries = vec![
            object_entry("a", "A"),
            object_entry("b", "B"),
            object_entry("c", "C"),
        ];
        let mut state = ObjectsEditorState::new();
        state.selected = Some(0);

        let idx = 2;
        entries.remove(idx);
        match state.selected {
            Some(sel) if sel == idx => state.selected = None,
            Some(sel) if sel > idx => state.selected = Some(sel - 1),
            _ => {}
        }

        assert_eq!(entries.len(), 2);
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn test_reset_for_new_campaign_sets_needs_initial_load_and_clears_state() {
        let mut state = ObjectsEditorState::new();
        let entries = vec![object_entry("a", "A")];
        state.needs_initial_load = false;
        state.enter_edit(0, &entries);
        state.search_query = "something".to_string();
        state.selected = Some(0);

        state.reset_for_new_campaign();

        assert!(state.needs_initial_load);
        assert_eq!(state.selected, None);
        assert_eq!(state.mode, ObjectsEditorMode::List);
        assert!(state.edit_buffer.is_none());
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_reset_selection_clears_state_but_preserves_needs_initial_load_true() {
        let mut state = ObjectsEditorState::new();
        let entries = vec![object_entry("a", "A")];
        state.needs_initial_load = true;
        state.enter_edit(0, &entries);
        state.selected = Some(0);

        state.reset_selection();

        assert_eq!(state.selected, None);
        assert_eq!(state.mode, ObjectsEditorMode::List);
        assert!(state.edit_buffer.is_none());
        assert!(state.needs_initial_load, "must remain true, untouched");
    }

    #[test]
    fn test_reset_selection_clears_state_but_preserves_needs_initial_load_false() {
        let mut state = ObjectsEditorState::new();
        let entries = vec![object_entry("a", "A")];
        state.needs_initial_load = false;
        state.enter_edit(0, &entries);
        state.selected = Some(0);

        state.reset_selection();

        assert_eq!(state.selected, None);
        assert!(!state.needs_initial_load, "must remain false, untouched");
    }

    /// Runs `body` inside a real `egui::Ui` via the same harness used by
    /// `obj_importer_ui.rs`'s `test_show_obj_importer_tab_renders_*` smoke
    /// tests, so `show()` itself — not just its extracted logic — is
    /// exercised and proven not to panic.
    fn with_test_ui(body: impl FnOnce(&mut egui::Ui)) {
        let mut body = Some(body);
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(body) = body.take() {
                    body(ui);
                }
            });
        });
    }

    #[test]
    fn test_show_auto_load_guard_with_no_campaign_dir_leaves_entries_untouched() {
        let mut state = ObjectsEditorState::new();
        assert!(state.needs_initial_load);
        let mut entries = vec![object_entry("fixture_only", "Fixture Only")];
        let mut unsaved = false;

        with_test_ui(|ui| {
            state.show(ui, &mut entries, None, &mut unsaved);
        });

        assert!(!state.needs_initial_load);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "fixture_only");
        assert!(!unsaved);
    }

    #[test]
    fn test_show_list_mode_renders_without_panicking() {
        let mut state = ObjectsEditorState::new();
        state.needs_initial_load = false;
        let mut entries = vec![
            object_entry("barred_door", "Barred Door"),
            object_entry("treasure_chest", "Treasure Chest"),
        ];
        let mut unsaved = false;

        with_test_ui(|ui| {
            state.show(ui, &mut entries, None, &mut unsaved);
        });

        assert_eq!(entries.len(), 2);
        assert_eq!(state.mode, ObjectsEditorMode::List);
    }

    #[test]
    fn test_show_edit_mode_renders_without_panicking() {
        let mut state = ObjectsEditorState::new();
        state.needs_initial_load = false;
        let mut entry = object_entry("oak_tree", "Oak Tree");
        entry
            .definition
            .meshes
            .push(antares::domain::visual::MeshDefinition {
                name: Some("trunk".to_string()),
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            });
        entry
            .definition
            .mesh_transforms
            .push(antares::domain::visual::MeshTransform::identity());
        let mut entries = vec![entry];
        state.enter_edit(0, &entries);
        let mut unsaved = false;

        with_test_ui(|ui| {
            state.show(ui, &mut entries, None, &mut unsaved);
        });

        // No Save click occurred (this is a single render pass with no
        // input), so the edit form must not have mutated the source-of-truth
        // `entries` Vec — only `self.edit_buffer` (a clone) is touched until
        // Save is clicked. Confirms the edit form rendered (no panic) while
        // leaving the original entry alone.
        assert_eq!(state.mode, ObjectsEditorMode::Edit);
        assert_eq!(entries[0].key, "oak_tree");
        assert!(entries[0].definition.meshes[0].material.is_none());
    }

    #[test]
    fn test_load_object_entries_from_registry_missing_file_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let entries = load_object_entries_from_registry(tmp.path());
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_object_entries_from_registry_skips_unreadable_entry_but_keeps_others() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let asset_dir = tmp.path().join("assets/meshes/objects");
        std::fs::create_dir_all(&asset_dir).unwrap();
        std::fs::File::create(asset_dir.join("good.ron"))
            .unwrap()
            .write_all(
                br#"(
    id: 1,
    name: "Good Entry",
    meshes: [],
    mesh_transforms: [],
)"#,
            )
            .unwrap();

        let mut registry = ObjectMeshRegistryFile::default();
        registry.upsert("good_key", "assets/meshes/objects/good.ron");
        registry.upsert("bad_key", "assets/meshes/objects/missing.ron");
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        registry
            .save(&tmp.path().join("data/object_mesh_registry.ron"))
            .unwrap();

        let entries = load_object_entries_from_registry(tmp.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "good_key");
        assert_eq!(entries[0].definition.name, "Good Entry");
    }

    #[test]
    fn test_write_and_sync_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();

        let definition = CreatureDefinition {
            id: 1,
            name: "Renamed Chest".to_string(),
            meshes: Vec::new(),
            mesh_transforms: Vec::new(),
            scale: 2.0,
            color_tint: None,
        };
        write_object_definition(
            tmp.path(),
            "assets/meshes/objects/old_chest.ron",
            &definition,
        );
        sync_object_mesh_registry_entry(
            tmp.path(),
            Some("old_chest"),
            "new_chest",
            "assets/meshes/objects/old_chest.ron",
        );

        let registry =
            ObjectMeshRegistryFile::load(&tmp.path().join("data/object_mesh_registry.ron"))
                .unwrap();
        assert!(!registry.meshes.contains_key("old_chest"));
        assert_eq!(
            registry.meshes.get("new_chest").map(String::as_str),
            Some("assets/meshes/objects/old_chest.ron")
        );

        let written =
            std::fs::read_to_string(tmp.path().join("assets/meshes/objects/old_chest.ron"))
                .unwrap();
        let parsed: CreatureDefinition = ron::from_str(&written).unwrap();
        assert_eq!(parsed.name, "Renamed Chest");
        assert_eq!(parsed.scale, 2.0);
    }
}
