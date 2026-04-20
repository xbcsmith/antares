// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Levels Editor for Campaign Builder
//!
//! This module provides a visual editor for creating and managing
//! `ClassLevelThresholds` entries that are stored in `levels.ron`.
//!
//! Content authors can define per-class XP thresholds without editing RON
//! files by hand.
//!
//! # Features
//!
//! - List view with search filter and toolbar (Add / Edit / Delete / Save / Load)
//! - Edit / Add view with:
//!   1. Class selector (autocomplete)
//!   2. Fill helpers (Formula, Flat, Step) via modal dialogs
//!   3. 200-row threshold table (Level | XP Required | Delta)
//! - Load / save helpers for RON (de)serialisation
//!
//! # Architecture
//!
//! Follows the standard SDK editor pattern (mirrors `stock_templates_editor.rs`):
//! - `LevelsEditorState` — top-level state with `show()` entry point
//! - `LevelsEditorMode`  — `List` / `Add` / `Edit`
//! - `FillFlatModalState` / `FillStepModalState` — ephemeral fill dialogs
//!
//! All egui SDK rules apply:
//! - Every loop uses `push_id`
//! - Every `ScrollArea` has a unique `id_salt`
//! - Every `ComboBox` uses `from_id_salt`
//! - `TwoColumnLayout` used for list/detail split (Rule 9)
//! - All toolbar rows use `horizontal_wrapped` (Rule 12)
//! - Class ID field uses `autocomplete_class_selector` (Rule 14)
//! - Every list row uses `show_standard_list_item` (Rule 15)

use crate::ui_helpers::{
    autocomplete_class_selector, show_standard_list_item, ItemAction, MetadataBadge,
    StandardListItemConfig, TwoColumnLayout,
};
use antares::domain::classes::ClassDefinition;
use antares::domain::levels::ClassLevelThresholds;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// Error type
// ============================================================================

/// Errors that can occur in the Levels Editor.
#[derive(Debug, thiserror::Error)]
pub enum LevelsEditorError {
    /// OS-level I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The file could not be parsed as RON.
    #[error("Parse error: {0}")]
    Parse(String),
    /// The data could not be serialised to RON.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ============================================================================
// Mode
// ============================================================================

/// Editor mode for the levels editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LevelsEditorMode {
    /// Viewing the list of class level entries.
    #[default]
    List,
    /// Creating a new class level entry.
    Add,
    /// Editing an existing class level entry.
    Edit,
}

// ============================================================================
// Modal states
// ============================================================================

/// State for the "Fill Flat" modal dialog.
///
/// Fills `edit_buffer_thresholds` as `thresholds[i] = i * delta`.
#[derive(Debug, Clone)]
pub struct FillFlatModalState {
    /// String buffer for the per-level XP delta (e.g. `"5000"`).
    pub delta_buf: String,
}

impl Default for FillFlatModalState {
    fn default() -> Self {
        Self {
            delta_buf: "5000".to_string(),
        }
    }
}

/// State for the "Fill Step" modal dialog.
///
/// Fills thresholds step-wise: the per-level delta starts at `base` and
/// increases by `step` every `breakpoint` levels.
#[derive(Debug, Clone)]
pub struct FillStepModalState {
    /// String buffer for the starting delta (e.g. `"1000"`).
    pub base_buf: String,
    /// String buffer for the per-section step increase (e.g. `"500"`).
    pub step_buf: String,
    /// String buffer for the number of levels per section (e.g. `"10"`).
    pub breakpoint_buf: String,
}

impl Default for FillStepModalState {
    fn default() -> Self {
        Self {
            base_buf: "1000".to_string(),
            step_buf: "500".to_string(),
            breakpoint_buf: "10".to_string(),
        }
    }
}

// ============================================================================
// RON file wrapper
// ============================================================================

/// Internal wrapper that mirrors the on-disk RON format `(entries: […])`.
///
/// Matches the struct layout used by [`antares::domain::levels::LevelDatabase`]
/// so files written by the editor are cross-compatible with the game engine's
/// loader.
#[derive(Debug, Serialize, Deserialize)]
struct LevelDatabaseFile {
    entries: Vec<ClassLevelThresholds>,
}

// ============================================================================
// Main state struct
// ============================================================================

/// Top-level state for the Levels Editor.
///
/// Mirrors the structure of `StockTemplatesEditorState`.
#[derive(Clone, Serialize, Deserialize)]
pub struct LevelsEditorState {
    /// All class level threshold entries being managed.
    pub levels: Vec<ClassLevelThresholds>,

    /// Currently selected entry index (list view).
    pub selected_entry: Option<usize>,

    /// Current editor mode.
    pub mode: LevelsEditorMode,

    /// Class ID string being edited in the form.
    pub edit_buffer_class_id: String,

    /// The 200-element XP threshold list being edited.
    pub edit_buffer_thresholds: Vec<u64>,

    /// Text filter for the list view search box.
    pub search_filter: String,

    /// Whether there are unsaved in-memory changes.
    pub has_unsaved_changes: bool,

    /// Validation errors for the current edit buffer.
    pub validation_errors: Vec<String>,

    /// Whether the delete-confirmation dialog is open.
    pub show_delete_confirm: bool,

    /// Last campaign directory (used by load / save helpers; skipped on serde).
    #[serde(skip)]
    pub last_campaign_dir: Option<PathBuf>,

    /// Last levels filename (cached from `show()` call; skipped on serde).
    #[serde(skip)]
    pub last_levels_file: Option<String>,

    /// Whether levels should be auto-loaded on the next `show()` call.
    ///
    /// Set to `true` whenever the campaign changes (new / open) so that
    /// `show()` triggers a lazy auto-load the first time the Levels tab is
    /// rendered. Cleared once the load attempt completes (success or
    /// file-not-found — never retried every frame). Skipped on serde so it
    /// always resets to `true` on app restart.
    #[serde(skip)]
    pub needs_initial_load: bool,

    /// Whether `load_from_file` has successfully populated `levels` at least
    /// once since the last `reset_for_new_campaign` call.
    ///
    /// Used by `do_save_campaign` to guard against overwriting a valid on-disk
    /// file with the empty default `Vec` before any load has occurred.
    /// Reset to `false` by `reset_for_new_campaign`; set to `true` by a
    /// successful `load_from_file`. Skipped on serde so it always starts
    /// as `false` on app restart, forcing a reload before any save.
    #[serde(skip)]
    pub loaded_from_file: bool,

    /// State for the "Fill Flat" modal dialog. `None` = dialog closed.
    #[serde(skip)]
    pub fill_flat_modal: Option<FillFlatModalState>,

    /// State for the "Fill Step" modal dialog. `None` = dialog closed.
    #[serde(skip)]
    pub fill_step_modal: Option<FillStepModalState>,

    /// Snapshot of available classes (updated by caller on each `show()`).
    #[serde(skip)]
    pub available_classes: Vec<ClassDefinition>,

    /// XP base from `CampaignConfig` (updated by caller on each `show()`).
    #[serde(skip)]
    pub base_xp: u64,

    /// XP multiplier from `CampaignConfig` (updated by caller on each `show()`).
    #[serde(skip)]
    pub xp_multiplier: f64,
}

impl Default for LevelsEditorState {
    fn default() -> Self {
        Self {
            levels: Vec::new(),
            selected_entry: None,
            mode: LevelsEditorMode::List,
            edit_buffer_class_id: String::new(),
            edit_buffer_thresholds: vec![0u64; 200],
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
            show_delete_confirm: false,
            last_campaign_dir: None,
            last_levels_file: None,
            needs_initial_load: true,
            loaded_from_file: false,
            fill_flat_modal: None,
            fill_step_modal: None,
            available_classes: Vec::new(),
            base_xp: 1000,
            xp_multiplier: 1.5,
        }
    }
}

// ============================================================================
// impl LevelsEditorState
// ============================================================================

impl LevelsEditorState {
    /// Creates a new default levels editor state.
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------ show

    /// Render the editor panel.
    ///
    /// Returns `true` when the in-memory levels list has changed (i.e. the
    /// caller should mark unsaved changes and sync its own copy).
    ///
    /// # Arguments
    ///
    /// * `ui`               — mutable egui Ui reference
    /// * `available_classes` — current campaign class list for the autocomplete
    /// * `campaign_dir`     — campaign root directory (used for load / save)
    /// * `levels_file`      — relative path to `levels.ron`
    /// * `base_xp`          — XP base from campaign config (used by Fill Formula)
    /// * `xp_multiplier`    — XP exponent from campaign config (used by Fill Formula)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// assert!(state.needs_initial_load);
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        available_classes: &[ClassDefinition],
        campaign_dir: Option<&PathBuf>,
        levels_file: &str,
        base_xp: u64,
        xp_multiplier: f64,
    ) -> bool {
        // Refresh snapshots every frame so the editor always sees current data.
        self.available_classes = available_classes.to_vec();
        self.base_xp = base_xp;
        self.xp_multiplier = xp_multiplier;

        // Cache campaign dir / filename for use in load/save helpers.
        if let Some(dir) = campaign_dir {
            self.last_campaign_dir = Some(dir.clone());
        }
        self.last_levels_file = Some(levels_file.to_string());

        // Auto-load on first show for the current campaign.
        //
        // `needs_initial_load` is set to `true` by `reset_for_new_campaign()`
        // (called from `do_new_campaign` and `do_open_campaign`). The explicit
        // `load_levels()` call in `do_open_campaign` is the primary load path;
        // this guard is a reliable fallback for the case where the user
        // navigates to this tab before that call has run, or the file was
        // absent at open time but has since appeared on disk.
        let mut auto_loaded = false;
        if self.needs_initial_load {
            if let Some(dir) = campaign_dir {
                let path = dir.join(levels_file);
                if path.exists() {
                    match self.load_from_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                            // Signal the caller to sync its mirror so other
                            // editors see the freshly loaded data immediately.
                            auto_loaded = true;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Auto-load failed: {}", e)];
                        }
                    }
                }
                // Clear regardless — don't retry every frame.
                self.needs_initial_load = false;
            }
        }

        let inner_changed = match self.mode {
            LevelsEditorMode::List => self.show_list_view(ui, campaign_dir, levels_file),
            LevelsEditorMode::Add | LevelsEditorMode::Edit => {
                self.show_edit_view(ui, campaign_dir, levels_file)
            }
        };

        auto_loaded || inner_changed
    }

    // ------------------------------------------------------------------ reset

    /// Reset editor state for a new or freshly-opened campaign.
    ///
    /// Clears the levels list, selection, edit buffer, and all transient UI
    /// state, then sets `needs_initial_load = true` so that `show()` will
    /// perform an auto-load the first time the Levels tab is rendered.
    ///
    /// Call this from both `do_new_campaign` and at the end of
    /// `do_open_campaign` (after the explicit `load_levels()` call).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// state.needs_initial_load = false;
    /// state.reset_for_new_campaign();
    /// assert!(state.needs_initial_load);
    /// assert!(state.levels.is_empty());
    /// ```
    pub fn reset_for_new_campaign(&mut self) {
        self.levels.clear();
        self.selected_entry = None;
        self.mode = LevelsEditorMode::List;
        self.edit_buffer_class_id.clear();
        self.edit_buffer_thresholds = vec![0u64; 200];
        self.search_filter.clear();
        self.has_unsaved_changes = false;
        self.validation_errors.clear();
        self.show_delete_confirm = false;
        self.fill_flat_modal = None;
        self.fill_step_modal = None;
        self.needs_initial_load = true;
        // Clear the load-guard so a save cannot overwrite the file with an
        // empty list before the new campaign's levels have been read.
        self.loaded_from_file = false;
        // Preserve last_campaign_dir / last_levels_file / available_classes /
        // base_xp / xp_multiplier — refreshed from caller on next show().
    }

    // ------------------------------------------------------------------ fill helpers

    /// Fill `edit_buffer_thresholds` using the campaign XP formula:
    /// `thresholds[i] = base_xp * i^xp_multiplier` (rounded to nearest integer).
    ///
    /// `thresholds[0]` is always `0` (level 1 requires 0 XP).
    /// Produces exactly 200 entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// state.fill_formula(1000, 1.5);
    ///
    /// assert_eq!(state.edit_buffer_thresholds[0], 0);
    /// assert_eq!(state.edit_buffer_thresholds[1], 1000);
    /// assert_eq!(state.edit_buffer_thresholds.len(), 200);
    /// ```
    pub fn fill_formula(&mut self, base_xp: u64, xp_multiplier: f64) {
        self.edit_buffer_thresholds = (0u32..200)
            .map(|i| {
                if i == 0 {
                    0u64
                } else {
                    ((base_xp as f64) * (i as f64).powf(xp_multiplier)).round() as u64
                }
            })
            .collect();
    }

    /// Fill `edit_buffer_thresholds` as cumulative flat: `thresholds[i] = i * delta`.
    ///
    /// `thresholds[0]` is always `0`. Produces exactly 200 entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// state.fill_flat(5000);
    ///
    /// assert_eq!(&state.edit_buffer_thresholds[0..4], &[0, 5000, 10000, 15000]);
    /// assert_eq!(state.edit_buffer_thresholds.len(), 200);
    /// ```
    pub fn fill_flat(&mut self, delta: u64) {
        self.edit_buffer_thresholds = (0u64..200).map(|i| i * delta).collect();
    }

    /// Fill `edit_buffer_thresholds` step-wise.
    ///
    /// The per-level XP delta starts at `base` and increases by `step` every
    /// `breakpoint` levels. `thresholds[0]` is always `0`. Produces exactly
    /// 200 entries.
    ///
    /// # Arguments
    ///
    /// * `base`       — starting per-level delta (XP cost of the first transition)
    /// * `step`       — how much to add to the delta each `breakpoint` levels
    /// * `breakpoint` — number of levels per section (clamped to at least 1)
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// state.fill_step(1000, 500, 10);
    ///
    /// assert_eq!(state.edit_buffer_thresholds[0], 0);
    /// // First 10 transitions (i = 1..=10): delta = 1000
    /// assert_eq!(state.edit_buffer_thresholds[1], 1000);
    /// assert_eq!(state.edit_buffer_thresholds[10], 10 * 1000);
    /// // Next 10 transitions (i = 11..=20): delta = 1500
    /// assert_eq!(state.edit_buffer_thresholds[11], 10 * 1000 + 1500);
    /// assert_eq!(state.edit_buffer_thresholds.len(), 200);
    /// ```
    pub fn fill_step(&mut self, base: u64, step: u64, breakpoint: usize) {
        let bp = breakpoint.max(1);
        let mut thresholds = vec![0u64; 200];
        for i in 1..200usize {
            let section = (i - 1) / bp;
            let delta = base + step * section as u64;
            thresholds[i] = thresholds[i - 1] + delta;
        }
        self.edit_buffer_thresholds = thresholds;
    }

    // ------------------------------------------------------------------ I/O

    /// Load level thresholds from a RON file.
    ///
    /// Replaces the current `levels` list on success and sets `loaded_from_file`
    /// to `true`. Resets `selected_entry` to `None`.
    ///
    /// # Errors
    ///
    /// Returns [`LevelsEditorError::Io`] if the file cannot be read, or
    /// [`LevelsEditorError::Parse`] if the contents are not valid RON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let mut state = LevelsEditorState::default();
    /// // Fails gracefully when the file is missing.
    /// let result = state.load_from_file(std::path::Path::new("/nonexistent/levels.ron"));
    /// assert!(result.is_err());
    /// ```
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), LevelsEditorError> {
        let contents = std::fs::read_to_string(path)?;

        let file: LevelDatabaseFile =
            ron::from_str(&contents).map_err(|e| LevelsEditorError::Parse(e.to_string()))?;

        self.levels = file.entries;
        self.selected_entry = None;
        // Mark that in-memory levels are now backed by the on-disk file.
        // `do_save_campaign` uses this flag to decide whether it is safe to
        // write back — preventing an empty default Vec from overwriting a
        // valid file before any load has occurred.
        self.loaded_from_file = true;
        Ok(())
    }

    /// Serialise the current `levels` list to a RON file.
    ///
    /// Creates any missing parent directories. The output format matches the
    /// `(entries: […])` struct-wrapper format used by the game engine's
    /// [`antares::domain::levels::LevelDatabase`] loader.
    ///
    /// # Errors
    ///
    /// Returns [`LevelsEditorError::Io`] on filesystem errors or
    /// [`LevelsEditorError::Serialization`] if RON serialisation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::levels_editor::LevelsEditorState;
    ///
    /// let state = LevelsEditorState::default();
    /// let result = state.save_to_file(std::path::Path::new("/tmp/levels.ron"));
    /// // Depends on filesystem access.
    /// ```
    pub fn save_to_file(&self, path: &Path) -> Result<(), LevelsEditorError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use struct_names(false) so the output is `(entries: […])` — the
        // same format expected by LevelDatabase::load_from_string / load_from_file.
        let config = ron::ser::PrettyConfig::new()
            .enumerate_arrays(false)
            .depth_limit(4);

        let file = LevelDatabaseFile {
            entries: self.levels.clone(),
        };

        let ron_string = ron::ser::to_string_pretty(&file, config)
            .map_err(|e| LevelsEditorError::Serialization(e.to_string()))?;

        std::fs::write(path, ron_string)?;
        Ok(())
    }

    // ------------------------------------------------------------------ list view

    fn show_list_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        levels_file: &str,
    ) -> bool {
        let mut needs_save = false;

        ui.heading("📊 Levels Database");
        ui.separator();

        // --- toolbar ---
        ui.horizontal_wrapped(|ui| {
            if ui.button("➕ Add Class").clicked() {
                self.start_add_entry();
            }

            let has_selection = self.selected_entry.is_some();

            if ui
                .add_enabled(has_selection, egui::Button::new("✏ Edit"))
                .clicked()
            {
                if let Some(idx) = self.selected_entry {
                    self.start_edit_entry(idx);
                }
            }

            if ui
                .add_enabled(has_selection, egui::Button::new("🗑 Delete"))
                .clicked()
            {
                self.show_delete_confirm = true;
            }

            ui.separator();

            if ui.button("💾 Save to File").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(levels_file);
                    match self.save_to_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Save failed: {}", e)];
                        }
                    }
                }
            }

            if ui.button("📂 Load from File").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(levels_file);
                    match self.load_from_file(&path) {
                        Ok(()) => {
                            self.has_unsaved_changes = false;
                            needs_save = true;
                        }
                        Err(e) => {
                            self.validation_errors = vec![format!("Load failed: {}", e)];
                        }
                    }
                }
            }
        });

        ui.separator();

        // --- search ---
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_filter)
                    .id_salt("levels_search")
                    .hint_text("Filter by class ID…"),
            );
            if ui.button("✕").on_hover_text("Clear search").clicked() {
                self.search_filter.clear();
            }
        });

        ui.separator();

        // --- unsaved banner ---
        if self.has_unsaved_changes {
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),
                "⚠ Unsaved changes — use 'Save to File' to persist",
            );
            ui.add_space(4.0);
        }

        // --- delete confirmation dialog ---
        if self.show_delete_confirm {
            if let Some(idx) = self.selected_entry {
                let class_id = self
                    .levels
                    .get(idx)
                    .map(|e| e.class_id.clone())
                    .unwrap_or_default();

                egui::Window::new("Confirm Delete Level Entry")
                    .collapsible(false)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Delete level thresholds for '{}'?", class_id));
                        ui.label("This cannot be undone.");
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("🗑 Delete").clicked() {
                                self.levels.remove(idx);
                                self.selected_entry = None;
                                self.has_unsaved_changes = true;
                                needs_save = true;
                                self.show_delete_confirm = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirm = false;
                            }
                        });
                    });
            } else {
                self.show_delete_confirm = false;
            }
        }

        // --- two-column layout: list + preview ---
        //
        // Pre-compute everything the left closure needs so that we avoid any
        // conflicting borrow when both closures are constructed simultaneously
        // (Rule 10).
        let filter_lower = self.search_filter.to_lowercase();
        let filtered: Vec<(usize, ClassLevelThresholds)> = self
            .levels
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                filter_lower.is_empty() || e.class_id.to_lowercase().contains(&filter_lower)
            })
            .map(|(idx, e)| (idx, e.clone()))
            .collect();

        let selected = self.selected_entry;
        let levels_is_empty = self.levels.is_empty();

        // Pre-compute the right-panel snapshot so the right closure only needs
        // immutable owned data — no &self borrow inside the closure.
        let preview_snapshot: Option<ClassLevelThresholds> =
            selected.and_then(|idx| self.levels.get(idx).cloned());

        // Deferred mutations collected from the left closure and applied after
        // show_split returns (Rule 10 deferred mutation pattern).
        let mut pending_select: Option<usize> = None;
        let mut pending_action: Option<(usize, ItemAction)> = None;

        TwoColumnLayout::new("levels_editor")
            .with_inspector_min_width(320.0)
            .show_split(
                ui,
                |left_ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("levels_list_scroll")
                        .auto_shrink([false, false])
                        .show(left_ui, |ui| {
                            for (idx, entry) in &filtered {
                                ui.push_id(idx, |ui| {
                                    // Badge: threshold count + first few values as preview.
                                    let preview_str = {
                                        let t = &entry.thresholds;
                                        let shown: Vec<String> =
                                            t.iter().take(4).map(|v| v.to_string()).collect();
                                        if t.len() > 4 {
                                            format!("{} …", shown.join(" / "))
                                        } else {
                                            shown.join(" / ")
                                        }
                                    };

                                    let badges = vec![
                                        MetadataBadge::new(format!(
                                            "Levels:{}",
                                            entry.thresholds.len()
                                        ))
                                        .with_color(egui::Color32::from_rgb(120, 170, 220))
                                        .with_tooltip("Number of defined XP thresholds"),
                                        MetadataBadge::new(preview_str)
                                            .with_color(egui::Color32::from_rgb(160, 200, 140))
                                            .with_tooltip("First few XP values"),
                                    ];

                                    let config = StandardListItemConfig::new(&entry.class_id)
                                        .with_badges(badges)
                                        .selected(selected == Some(*idx));

                                    let (clicked, action) = show_standard_list_item(ui, config);
                                    if clicked {
                                        pending_select = Some(*idx);
                                        ui.ctx().request_repaint();
                                    }
                                    if action != ItemAction::None {
                                        pending_action = Some((*idx, action));
                                    }
                                });
                            }

                            if filtered.is_empty() {
                                let empty_text = if levels_is_empty {
                                    "(no entries — click ➕ Add Class to add one)"
                                } else {
                                    "(no entries match the search filter)"
                                };
                                ui.label(egui::RichText::new(empty_text).weak());
                            }
                        });
                },
                |right_ui| {
                    if let Some(entry) = &preview_snapshot {
                        show_levels_preview(right_ui, entry);
                    } else {
                        right_ui.label(
                            egui::RichText::new(
                                "Select a class from the list to preview its thresholds.",
                            )
                            .weak(),
                        );
                    }
                },
            );

        // Apply deferred mutations — no active closure borrows at this point.
        if let Some(new_idx) = pending_select {
            self.selected_entry = Some(new_idx);
        }

        if let Some((idx, action)) = pending_action {
            self.selected_entry = Some(idx);
            match action {
                ItemAction::Edit => {
                    self.start_edit_entry(idx);
                }
                ItemAction::Delete => {
                    self.show_delete_confirm = true;
                }
                ItemAction::Duplicate => {
                    if idx < self.levels.len() {
                        let mut dup = self.levels[idx].clone();
                        let new_id = self.next_duplicate_class_id(&dup.class_id);
                        dup.class_id = new_id.clone();
                        self.levels.push(dup);
                        self.selected_entry = Some(self.levels.len() - 1);
                        self.has_unsaved_changes = true;
                        needs_save = true;
                        self.validation_errors = vec![format!("Duplicated entry as '{}'", new_id)];
                    }
                }
                ItemAction::Export => {
                    if idx < self.levels.len() {
                        match ron::ser::to_string_pretty(
                            &self.levels[idx],
                            ron::ser::PrettyConfig::default(),
                        ) {
                            Ok(ron_text) => {
                                ui.ctx().copy_text(ron_text);
                                self.validation_errors =
                                    vec!["Copied level entry RON to clipboard".to_string()];
                            }
                            Err(e) => {
                                self.validation_errors = vec![format!("Export failed: {}", e)];
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }

        // Surface any persistent errors.
        if !self.validation_errors.is_empty() {
            ui.separator();
            ui.group(|ui| {
                ui.heading("⚠️ Errors");
                for e in &self.validation_errors {
                    ui.colored_label(egui::Color32::RED, e);
                }
            });
        }

        needs_save
    }

    // ------------------------------------------------------------------ edit view

    fn show_edit_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        levels_file: &str,
    ) -> bool {
        let mut needs_save = false;
        let is_add = self.mode == LevelsEditorMode::Add;

        ui.heading(if is_add {
            "➕ Add Class Level Thresholds"
        } else {
            "✏ Edit Class Level Thresholds"
        });
        ui.separator();

        // ── Fill modals — rendered to ctx() so they float above the scroll area ──
        //
        // Capture apply/close signals as local variables and apply them after
        // the modal rendering (avoids aliasing `self` inside the Window closure
        // and the outer logic simultaneously).
        let mut apply_flat: Option<u64> = None;
        let mut close_flat = false;

        if let Some(modal) = &mut self.fill_flat_modal {
            egui::Window::new("Fill Flat")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Set all thresholds as:  thresholds[i] = i × delta");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Delta (XP per level):");
                        let mut val: u64 = modal.delta_buf.parse().unwrap_or(5000);
                        if ui.add(egui::DragValue::new(&mut val)).changed() {
                            modal.delta_buf = val.to_string();
                        }
                    });

                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("✅ Apply").clicked() {
                            let val: u64 = modal.delta_buf.parse().unwrap_or(5000);
                            apply_flat = Some(val);
                            close_flat = true;
                        }
                        if ui.button("❌ Cancel").clicked() {
                            close_flat = true;
                        }
                    });
                });
        }
        if let Some(delta) = apply_flat {
            self.fill_flat(delta);
        }
        if close_flat {
            self.fill_flat_modal = None;
        }

        let mut apply_step: Option<(u64, u64, usize)> = None;
        let mut close_step = false;

        if let Some(modal) = &mut self.fill_step_modal {
            egui::Window::new("Fill Step")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        "Step fill: delta starts at 'base', increases by 'step' \
                         every 'breakpoint' transitions.",
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Base delta:");
                        let mut val: u64 = modal.base_buf.parse().unwrap_or(1000);
                        if ui.add(egui::DragValue::new(&mut val)).changed() {
                            modal.base_buf = val.to_string();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Step (increase per section):");
                        let mut val: u64 = modal.step_buf.parse().unwrap_or(500);
                        if ui.add(egui::DragValue::new(&mut val)).changed() {
                            modal.step_buf = val.to_string();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Breakpoint (transitions per section):");
                        let mut val: usize = modal.breakpoint_buf.parse().unwrap_or(10);
                        if ui
                            .add(egui::DragValue::new(&mut val).range(1..=200usize))
                            .changed()
                        {
                            modal.breakpoint_buf = val.to_string();
                        }
                    });

                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("✅ Apply").clicked() {
                            let base: u64 = modal.base_buf.parse().unwrap_or(1000);
                            let step: u64 = modal.step_buf.parse().unwrap_or(500);
                            let bp: usize = modal.breakpoint_buf.parse().unwrap_or(10);
                            apply_step = Some((base, step, bp));
                            close_step = true;
                        }
                        if ui.button("❌ Cancel").clicked() {
                            close_step = true;
                        }
                    });
                });
        }
        if let Some((base, step, bp)) = apply_step {
            self.fill_step(base, step, bp);
        }
        if close_step {
            self.fill_step_modal = None;
        }

        // ── Main edit form (scrollable) ──
        egui::ScrollArea::vertical()
            .id_salt("levels_edit_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ── Group 1: Class Identity ──
                ui.group(|ui| {
                    ui.heading("Class");

                    autocomplete_class_selector(
                        ui,
                        "levels_class_id",
                        "Class ID:",
                        &mut self.edit_buffer_class_id,
                        &self.available_classes,
                    );

                    ui.label(
                        egui::RichText::new("Must match a class ID defined in classes.ron")
                            .small()
                            .weak(),
                    );
                });

                ui.add_space(8.0);

                // ── Group 2: Fill helpers ──
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Fill Helpers").strong());
                    ui.label(
                        egui::RichText::new("Populate all 200 thresholds using a fill strategy:")
                            .small()
                            .weak(),
                    );
                    ui.add_space(4.0);

                    let base_xp = self.base_xp;
                    let xp_multiplier = self.xp_multiplier;

                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button("📐 Fill Formula")
                            .on_hover_text(format!(
                                "Fill using: base_xp({}) × (level-1)^{:.2}",
                                base_xp, xp_multiplier
                            ))
                            .clicked()
                        {
                            self.fill_formula(base_xp, xp_multiplier);
                        }

                        if ui
                            .button("📏 Fill Flat…")
                            .on_hover_text("Fill as thresholds[i] = i × delta")
                            .clicked()
                        {
                            self.fill_flat_modal = Some(FillFlatModalState::default());
                        }

                        if ui
                            .button("📈 Fill Step…")
                            .on_hover_text(
                                "Fill step-wise with increasing delta per breakpoint section",
                            )
                            .clicked()
                        {
                            self.fill_step_modal = Some(FillStepModalState::default());
                        }
                    });
                });

                ui.add_space(8.0);

                // ── Group 3: Threshold table ──
                ui.group(|ui| {
                    ui.label(egui::RichText::new("XP Thresholds (200 levels)").strong());
                    ui.label(
                        egui::RichText::new(
                            "thresholds[0] = Level 1 (always 0). \
                             Each value is the total cumulative XP required.",
                        )
                        .small()
                        .weak(),
                    );
                    ui.add_space(4.0);

                    let threshold_len = self.edit_buffer_thresholds.len();

                    egui::ScrollArea::vertical()
                        .id_salt("levels_thresholds_scroll")
                        .max_height(420.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Grid::new("levels_thresholds_grid")
                                .num_columns(3)
                                .striped(true)
                                .min_col_width(70.0)
                                .show(ui, |ui| {
                                    // Header row
                                    ui.label(egui::RichText::new("Level").strong());
                                    ui.label(egui::RichText::new("XP Required").strong());
                                    ui.label(egui::RichText::new("Delta").strong());
                                    ui.end_row();

                                    for row_idx in 0..threshold_len {
                                        // Level label (col 0)
                                        ui.label(format!("{}", row_idx + 1));

                                        // DragValue wrapped in push_id for unique
                                        // widget IDs within the loop (Rule 1).
                                        // push_id allocates the child rect as a
                                        // single Grid cell, advancing column 1.
                                        ui.push_id(row_idx, |ui| {
                                            ui.add(egui::DragValue::new(
                                                &mut self.edit_buffer_thresholds[row_idx],
                                            ));
                                        });

                                        // Delta (col 2) — computed after the
                                        // mutable borrow via DragValue is released.
                                        let delta = if row_idx == 0 {
                                            0u64
                                        } else {
                                            self.edit_buffer_thresholds[row_idx].saturating_sub(
                                                self.edit_buffer_thresholds[row_idx - 1],
                                            )
                                        };
                                        ui.label(format!("{}", delta));
                                        ui.end_row();
                                    }
                                });
                        });
                });

                ui.add_space(8.0);

                // ── Validation errors ──
                if !self.validation_errors.is_empty() {
                    ui.group(|ui| {
                        ui.heading("⚠️ Validation Errors");
                        for err in &self.validation_errors {
                            ui.colored_label(egui::Color32::RED, err);
                        }
                    });
                    ui.add_space(4.0);
                }

                // ── Action buttons ──
                ui.horizontal_wrapped(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = LevelsEditorMode::List;
                        self.validation_errors.clear();
                    }

                    if ui.button("💾 Save").clicked() {
                        match self.validate_edit_buffer() {
                            Ok(entry) => {
                                self.validation_errors.clear();

                                match self.mode {
                                    LevelsEditorMode::Add => {
                                        self.levels.push(entry);
                                        self.selected_entry =
                                            Some(self.levels.len().saturating_sub(1));
                                    }
                                    LevelsEditorMode::Edit => {
                                        if let Some(idx) = self.selected_entry {
                                            if idx < self.levels.len() {
                                                self.levels[idx] = entry;
                                            }
                                        }
                                    }
                                    LevelsEditorMode::List => {}
                                }

                                self.has_unsaved_changes = true;
                                needs_save = true;

                                // Attempt immediate file persistence.
                                if let Some(dir) = campaign_dir {
                                    let path = dir.join(levels_file);
                                    match self.save_to_file(&path) {
                                        Ok(()) => {
                                            self.has_unsaved_changes = false;
                                        }
                                        Err(e) => {
                                            self.validation_errors =
                                                vec![format!("Save failed: {}", e)];
                                        }
                                    }
                                }

                                self.mode = LevelsEditorMode::List;
                            }
                            Err(errs) => {
                                self.validation_errors = errs;
                            }
                        }
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = LevelsEditorMode::List;
                        self.edit_buffer_class_id.clear();
                        self.edit_buffer_thresholds = vec![0u64; 200];
                        self.validation_errors.clear();
                    }
                });
            });

        needs_save
    }

    // ------------------------------------------------------------------ helpers

    /// Transition into Add mode with a blank buffer.
    fn start_add_entry(&mut self) {
        self.mode = LevelsEditorMode::Add;
        self.edit_buffer_class_id.clear();
        self.edit_buffer_thresholds = vec![0u64; 200];
        self.validation_errors.clear();
    }

    /// Transition into Edit mode for the entry at `idx`.
    fn start_edit_entry(&mut self, idx: usize) {
        if let Some(entry) = self.levels.get(idx) {
            self.edit_buffer_class_id = entry.class_id.clone();
            // Copy stored thresholds into the buffer, padding to 200 with zeros.
            let mut thresholds = entry.thresholds.clone();
            thresholds.resize(200, 0);
            self.edit_buffer_thresholds = thresholds;
            self.selected_entry = Some(idx);
            self.mode = LevelsEditorMode::Edit;
            self.validation_errors.clear();
        }
    }

    /// Returns a unique class ID suitable for a duplicated entry.
    fn next_duplicate_class_id(&self, source_id: &str) -> String {
        let mut candidate = format!("{}_copy", source_id);
        let mut suffix = 2u32;
        while self.levels.iter().any(|e| e.class_id == candidate) {
            candidate = format!("{}_copy_{}", source_id, suffix);
            suffix += 1;
        }
        candidate
    }

    /// Validate the current edit buffer and build a [`ClassLevelThresholds`] on success.
    ///
    /// Trailing zero entries are trimmed from the threshold list to keep the
    /// saved RON compact (at least one entry is always retained).
    fn validate_edit_buffer(&self) -> Result<ClassLevelThresholds, Vec<String>> {
        let mut errors = Vec::new();
        let class_id = self.edit_buffer_class_id.trim().to_string();

        if class_id.is_empty() {
            errors.push("Class ID cannot be empty.".to_string());
        }

        // Duplicate check in Add mode.
        if self.mode == LevelsEditorMode::Add
            && !class_id.is_empty()
            && self.levels.iter().any(|e| e.class_id == class_id)
        {
            errors.push(format!(
                "Class ID '{}' already has a level table.",
                class_id
            ));
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Trim trailing zeros to keep the file compact; always keep ≥1 entry.
        let mut thresholds = self.edit_buffer_thresholds.clone();
        while thresholds.len() > 1 && thresholds.last() == Some(&0) {
            thresholds.pop();
        }

        Ok(ClassLevelThresholds {
            class_id,
            thresholds,
        })
    }
}

// ============================================================================
// Preview helper (free function to avoid borrow conflicts in TwoColumnLayout)
// ============================================================================

/// Render a read-only summary of a single class's level thresholds.
///
/// Shown in the right panel of the list view when an entry is selected.
fn show_levels_preview(ui: &mut egui::Ui, entry: &ClassLevelThresholds) {
    ui.heading(format!("📊 {}", entry.class_id));
    ui.separator();

    ui.label(format!(
        "Defined thresholds: {} level(s)",
        entry.thresholds.len()
    ));

    if entry.thresholds.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("(no thresholds defined)").weak());
        return;
    }

    ui.label(
        egui::RichText::new(
            "thresholds[0] = Level 1 (always 0). Each value is the total cumulative XP required.",
        )
        .weak()
        .small(),
    );

    ui.add_space(4.0);
    ui.label(egui::RichText::new(format!("Showing {} levels:", entry.thresholds.len())).strong());

    let scroll_height = ui.available_height().max(240.0);
    egui::ScrollArea::vertical()
        .id_salt(format!("levels_preview_scroll_{}", entry.class_id))
        .auto_shrink([false, false])
        .max_height(scroll_height)
        .show(ui, |ui| {
            egui::Grid::new(format!("levels_preview_grid_{}", entry.class_id))
                .num_columns(3)
                .striped(true)
                .min_col_width(60.0)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    // Header row
                    ui.label(egui::RichText::new("Level").strong());
                    ui.label(egui::RichText::new("XP Required").strong());
                    ui.label(egui::RichText::new("Delta").strong());
                    ui.end_row();

                    for i in 0..entry.thresholds.len() {
                        let xp = entry.thresholds[i];
                        let delta = if i == 0 {
                            0u64
                        } else {
                            xp.saturating_sub(entry.thresholds[i - 1])
                        };

                        // Level column (col 0)
                        ui.label(format!("{}", i + 1));

                        // XP Required column (col 1) — styled frame to match
                        // the edit view's DragValue appearance. push_id gives
                        // the Frame a unique widget ID within the loop (Rule 1).
                        // end_row() MUST be called outside push_id so it fires
                        // on the grid's Ui, not the child scope.
                        ui.push_id(i, |ui| {
                            egui::Frame::new()
                                .fill(ui.visuals().extreme_bg_color)
                                .inner_margin(egui::Margin::symmetric(4, 2))
                                .corner_radius(egui::CornerRadius::same(2))
                                .show(ui, |ui| {
                                    ui.label(format!("{}", xp));
                                });
                        });

                        // Delta column (col 2)
                        ui.label(format!("{}", delta));

                        // Advance to the next row at the grid level.
                        // Placing end_row() here (outside push_id) is critical:
                        // calling it inside push_id's closure would target the
                        // child Ui scope rather than the grid, causing every row
                        // to overflow horizontally onto a single line.
                        ui.end_row();
                    }
                });
        });
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::levels::ClassLevelThresholds;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_entry(class_id: &str) -> ClassLevelThresholds {
        ClassLevelThresholds {
            class_id: class_id.to_string(),
            thresholds: vec![0, 1200, 3000, 6000, 10000],
        }
    }

    // ── default state ─────────────────────────────────────────────────────────

    /// `LevelsEditorState::default()` initialises to a clean, loadable state.
    #[test]
    fn test_levels_editor_state_default() {
        let state = LevelsEditorState::default();

        assert!(state.levels.is_empty(), "levels should start empty");
        assert_eq!(state.mode, LevelsEditorMode::List);
        assert!(state.selected_entry.is_none());
        assert!(state.validation_errors.is_empty());
        assert!(!state.has_unsaved_changes);
        assert!(
            state.needs_initial_load,
            "needs_initial_load must be true on default"
        );
        assert!(
            !state.loaded_from_file,
            "loaded_from_file must be false on default"
        );
        assert_eq!(
            state.edit_buffer_thresholds.len(),
            200,
            "default buffer must have 200 entries"
        );
    }

    // ── fill_formula ──────────────────────────────────────────────────────────

    /// Level 1 threshold (index 0) is always 0 regardless of formula params.
    #[test]
    fn test_fill_formula_level_1_is_zero() {
        let mut state = LevelsEditorState::default();
        state.fill_formula(1000, 1.5);
        assert_eq!(state.edit_buffer_thresholds[0], 0);
    }

    /// Level 2 threshold equals `base_xp * 1^multiplier = base_xp`.
    #[test]
    fn test_fill_formula_level_2() {
        let mut state = LevelsEditorState::default();
        state.fill_formula(1000, 1.5);
        // index 1: base_xp * (1.0)^1.5 = 1000 * 1.0 = 1000
        assert_eq!(state.edit_buffer_thresholds[1], 1000);
    }

    /// `fill_formula` produces exactly 200 entries.
    #[test]
    fn test_fill_formula_200_rows() {
        let mut state = LevelsEditorState::default();
        state.fill_formula(1000, 1.5);
        assert_eq!(state.edit_buffer_thresholds.len(), 200);
    }

    // ── fill_flat ─────────────────────────────────────────────────────────────

    /// `fill_flat(5000)` gives `[0, 5000, 10000, 15000, …]`.
    #[test]
    fn test_fill_flat_delta_5000_levels_1_4() {
        let mut state = LevelsEditorState::default();
        state.fill_flat(5000);
        assert_eq!(
            &state.edit_buffer_thresholds[0..4],
            &[0u64, 5000, 10000, 15000],
        );
    }

    /// `fill_flat` produces exactly 200 entries.
    #[test]
    fn test_fill_flat_200_rows() {
        let mut state = LevelsEditorState::default();
        state.fill_flat(5000);
        assert_eq!(state.edit_buffer_thresholds.len(), 200);
    }

    // ── fill_step ─────────────────────────────────────────────────────────────

    /// With base=1000, step=500, bp=10:
    /// - transitions i=1..=10  use delta 1000
    /// - transitions i=11..=20 use delta 1500
    #[test]
    fn test_fill_step_base_1000_step_500_breakpoint_10() {
        let mut state = LevelsEditorState::default();
        state.fill_step(1000, 500, 10);

        // Index 0 is always 0.
        assert_eq!(state.edit_buffer_thresholds[0], 0);

        // First 10 transitions (i = 1..=10) — section 0, delta = 1000.
        assert_eq!(state.edit_buffer_thresholds[1], 1000);
        assert_eq!(state.edit_buffer_thresholds[10], 10u64 * 1000);

        // Next 10 transitions (i = 11..=20) — section 1, delta = 1500.
        // thresholds[11] = thresholds[10] + 1500 = 10_000 + 1500 = 11_500
        let expected_at_11 = 10u64 * 1000 + 1500;
        assert_eq!(state.edit_buffer_thresholds[11], expected_at_11);

        // thresholds[20] = thresholds[10] + 10 * 1500 = 10_000 + 15_000 = 25_000
        let expected_at_20 = 10u64 * 1000 + 10 * 1500;
        assert_eq!(state.edit_buffer_thresholds[20], expected_at_20);
    }

    /// `fill_step` produces exactly 200 entries.
    #[test]
    fn test_fill_step_200_rows() {
        let mut state = LevelsEditorState::default();
        state.fill_step(1000, 500, 10);
        assert_eq!(state.edit_buffer_thresholds.len(), 200);
    }

    // ── load / save round-trip ────────────────────────────────────────────────

    /// Writing then reading back preserves class_id and thresholds exactly.
    #[test]
    fn test_load_from_file_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("levels.ron");

        let original = make_entry("knight");

        let mut state = LevelsEditorState::default();
        state.levels.push(original.clone());
        state.save_to_file(&path).expect("save should succeed");

        let mut state2 = LevelsEditorState::default();
        state2.load_from_file(&path).expect("load should succeed");

        assert_eq!(state2.levels.len(), 1);
        assert_eq!(state2.levels[0].class_id, original.class_id);
        assert_eq!(state2.levels[0].thresholds, original.thresholds);
        assert!(
            state2.loaded_from_file,
            "loaded_from_file must be true after successful load"
        );
    }

    /// Loading a nonexistent file returns an error rather than panicking.
    #[test]
    fn test_load_from_file_missing_returns_error() {
        let mut state = LevelsEditorState::default();
        let result = state.load_from_file(std::path::Path::new("/nonexistent/no_such_file.ron"));
        assert!(result.is_err(), "expected Err for missing file");
    }

    // ── reset_for_new_campaign ────────────────────────────────────────────────

    /// `reset_for_new_campaign` clears all data and sets the re-load flag.
    #[test]
    fn test_reset_for_new_campaign_clears_data() {
        let mut state = LevelsEditorState::default();

        // Populate with some data.
        state.levels.push(make_entry("knight"));
        state.selected_entry = Some(0);
        state.mode = LevelsEditorMode::Edit;
        state.edit_buffer_class_id = "knight".to_string();
        state.has_unsaved_changes = true;
        state.loaded_from_file = true;
        state.needs_initial_load = false;
        state.search_filter = "kni".to_string();
        state.validation_errors = vec!["some error".to_string()];

        state.reset_for_new_campaign();

        assert!(state.levels.is_empty(), "levels must be cleared");
        assert!(state.selected_entry.is_none());
        assert_eq!(state.mode, LevelsEditorMode::List);
        assert!(state.edit_buffer_class_id.is_empty());
        assert!(!state.has_unsaved_changes);
        assert!(!state.loaded_from_file, "loaded_from_file must be cleared");
        assert!(
            state.needs_initial_load,
            "needs_initial_load must be set to true"
        );
        assert!(state.search_filter.is_empty());
        assert!(state.validation_errors.is_empty());
    }

    // ── start_add_entry populates 200-row buffer ──────────────────────────────

    /// After `start_add_entry`, the edit buffer has 200 threshold entries and
    /// the mode is set to `Add`.
    #[test]
    fn test_add_entry_populates_200_rows() {
        let mut state = LevelsEditorState::default();
        state.start_add_entry();

        assert_eq!(
            state.edit_buffer_thresholds.len(),
            200,
            "edit buffer must always have 200 entries in Add mode"
        );
        assert_eq!(state.mode, LevelsEditorMode::Add);
        assert!(state.edit_buffer_class_id.is_empty());
        assert!(state.validation_errors.is_empty());
    }

    // ── validate_edit_buffer ─────────────────────────────────────────────────

    /// An empty class_id produces a validation error.
    #[test]
    fn test_validate_edit_buffer_empty_class_id_returns_error() {
        let state = LevelsEditorState {
            mode: LevelsEditorMode::Add,
            ..LevelsEditorState::default()
        };

        let result = state.validate_edit_buffer();
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("Class ID")),
            "expected Class ID error, got: {:?}",
            errs
        );
    }

    /// Duplicate class_id in Add mode produces a validation error.
    #[test]
    fn test_validate_edit_buffer_duplicate_id_in_add_mode_returns_error() {
        let mut state = LevelsEditorState {
            mode: LevelsEditorMode::Add,
            edit_buffer_class_id: "knight".to_string(),
            ..LevelsEditorState::default()
        };
        state.levels.push(make_entry("knight"));

        let result = state.validate_edit_buffer();
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("already has")),
            "expected duplicate error, got: {:?}",
            errs
        );
    }

    /// A valid buffer round-trips through `validate_edit_buffer`.
    #[test]
    fn test_validate_edit_buffer_valid_returns_entry() {
        let mut state = LevelsEditorState {
            mode: LevelsEditorMode::Add,
            edit_buffer_class_id: "sorcerer".to_string(),
            ..LevelsEditorState::default()
        };
        state.edit_buffer_thresholds[0] = 0;
        state.edit_buffer_thresholds[1] = 800;
        state.edit_buffer_thresholds[2] = 2000;

        let result = state.validate_edit_buffer();
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);

        let entry = result.unwrap();
        assert_eq!(entry.class_id, "sorcerer");
        // Trailing zeros are trimmed; first three entries must survive.
        assert!(entry.thresholds.len() >= 3);
        assert_eq!(entry.thresholds[0], 0);
        assert_eq!(entry.thresholds[1], 800);
        assert_eq!(entry.thresholds[2], 2000);
    }

    // ── next_duplicate_class_id ───────────────────────────────────────────────

    /// Duplicating a class_id that does not yet exist as `_copy` gives `_copy`.
    #[test]
    fn test_next_duplicate_class_id_basic() {
        let state = LevelsEditorState::default();
        assert_eq!(state.next_duplicate_class_id("knight"), "knight_copy");
    }

    /// When `_copy` already exists, the suffix increments.
    #[test]
    fn test_next_duplicate_class_id_increments_suffix() {
        let mut state = LevelsEditorState::default();
        state.levels.push(make_entry("knight_copy"));
        assert_eq!(state.next_duplicate_class_id("knight"), "knight_copy_2");
    }

    // ── loaded_from_file flag ─────────────────────────────────────────────────

    /// `loaded_from_file` starts as `false` and becomes `true` after a successful load.
    #[test]
    fn test_loaded_from_file_flag_lifecycle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("levels.ron");

        let mut state = LevelsEditorState::default();
        assert!(!state.loaded_from_file, "should start false");

        state.levels.push(make_entry("archer"));
        state.save_to_file(&path).expect("save");

        let mut state2 = LevelsEditorState::default();
        state2.load_from_file(&path).expect("load");
        assert!(state2.loaded_from_file, "should be true after load");

        state2.reset_for_new_campaign();
        assert!(!state2.loaded_from_file, "should be false after reset");
    }

    // ── fill_formula monotonicity ─────────────────────────────────────────────

    /// The formula fill produces a monotonically non-decreasing sequence.
    #[test]
    fn test_fill_formula_is_non_decreasing() {
        let mut state = LevelsEditorState::default();
        state.fill_formula(1000, 1.5);

        for i in 1..state.edit_buffer_thresholds.len() {
            assert!(
                state.edit_buffer_thresholds[i] >= state.edit_buffer_thresholds[i - 1],
                "thresholds must be non-decreasing at index {}",
                i
            );
        }
    }

    // ── fill_step breakpoint = 1 (edge case) ─────────────────────────────────

    /// With breakpoint=1, every level is its own section: delta increases by
    /// `step` on every transition.
    #[test]
    fn test_fill_step_breakpoint_1_every_level_is_its_own_section() {
        let mut state = LevelsEditorState::default();
        // bp=1: section for transition i = (i-1)/1 = i-1
        // delta(i) = base + step * (i-1)
        state.fill_step(100, 50, 1);

        assert_eq!(state.edit_buffer_thresholds[0], 0);
        // i=1: section=0, delta=100, thresh[1]=100
        assert_eq!(state.edit_buffer_thresholds[1], 100);
        // i=2: section=1, delta=150, thresh[2]=250
        assert_eq!(state.edit_buffer_thresholds[2], 250);
        // i=3: section=2, delta=200, thresh[3]=450
        assert_eq!(state.edit_buffer_thresholds[3], 450);
    }

    // ── multiple entries survive round-trip ───────────────────────────────────

    /// Multiple class entries are all preserved through a save/load cycle.
    #[test]
    fn test_round_trip_multiple_classes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("levels.ron");

        let mut state = LevelsEditorState::default();
        state.levels.push(make_entry("knight"));
        state.levels.push(make_entry("sorcerer"));
        state.levels.push(make_entry("thief"));
        state.save_to_file(&path).expect("save");

        let mut state2 = LevelsEditorState::default();
        state2.load_from_file(&path).expect("load");

        assert_eq!(state2.levels.len(), 3);
        let ids: Vec<&str> = state2.levels.iter().map(|e| e.class_id.as_str()).collect();
        assert!(ids.contains(&"knight"));
        assert!(ids.contains(&"sorcerer"));
        assert!(ids.contains(&"thief"));
    }
}
