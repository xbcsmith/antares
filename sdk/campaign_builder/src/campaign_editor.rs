// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Metadata Editor
//!
//! A dedicated metadata editor for `CampaignMetadata` with a TwoColumn UI.
//!
//! Phase 5 - Docs, Cleanup & Handoff:
//! - Finalized API and added examples, targeted unit tests, and developer guidance.
//! - Consolidated validation request flow so the app remains the single source of truth.
//! - Cleaned up UI/UX interactions and added tests for editor state transitions.
//!
//! Developer Notes:
//! - Design Pattern: The editor uses an edit-buffer (`CampaignMetadataEditBuffer`) so all
//!   changes remain transient until persisted. Call `apply_buffer_to_metadata()` to copy
//!   buffer data into the authoritative `metadata`.
//! - Validation: Editors should not directly invoke `validate_campaign()` while they
//!   hold UI-state borrows. Instead, set `validate_requested` to true (callers can
//!   use `consume_validate_request()` to check the flag), then let the main app run the
//!   centralized validator. This avoids borrow issues with egui and maintains a single
//!   validation entry point.
//! - UI ID collision avoidance:
//!   Avoid using `egui::ComboBox::from_label("")` with an empty label. Instead:
//!   - Use `egui::ComboBox::from_id_source("unique_id")` when the UI label is displayed elsewhere (e.g., grid cell `ui.label("...")`).
//!   - Or `egui::ComboBox::from_label("UniqueLabel")` with a unique label when you want the ComboBox to render its own label.
//!     This ensures unique internal control IDs and avoids ID collisions when multiple ComboBoxes coexist in the same UI.
//! - Extensibility checklist for adding a new metadata field:
//!   1. Add the field to `CampaignMetadata` in `domain`.
//!   2. Add a matching field to `CampaignMetadataEditBuffer`.
//!   3. Update `from_metadata()` and `apply_to()` so values are round-tripped.
//!   4. Add UI controls in `show()` under the correct section (Overview/Files/Gameplay).
//!   5. Add validation checks to `validate_campaign()` and unit tests to cover edge-cases.
//!
//! API Notes:
//! - Use `save_to_file()` / `load_from_file()` for RON-based persistence.
//! - `show()` implements a TwoColumn layout and uses `ui_helpers` components for
//!   consistency with other editors.

use crate::ui_helpers::{
    compute_default_panel_height, EditorToolbar, ToolbarAction, TwoColumnLayout,
};
use antares::domain::character::{FOOD_MAX, FOOD_MIN, PARTY_MAX_SIZE};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

/// Editor mode for the campaign metadata editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignEditorMode {
    /// Show the list/main entry view
    List,
    /// Creating a new campaign (rare)
    Creating,
    /// Editing an existing campaign
    Editing,
}

/// Logical sections for the two-column UI (for future use)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignSection {
    /// Overview contains general campaign metadata (id, name, description)
    Overview,
    /// Files contains paths to data files used by the campaign
    Files,
    /// Gameplay contains starting positions, levels and difficulty settings
    Gameplay,
    /// Advanced includes extra fields and RON export utilities
    Advanced,
}

/// Edit buffer for the campaign metadata form.
///
/// This mirrors `CampaignMetadata` fields for editing. Using a buffer keeps
/// changes transient until the user confirms a save action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetadataEditBuffer {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub engine_version: String,

    // Campaign configuration
    pub starting_map: String,
    pub starting_position: (u32, u32),
    pub starting_direction: String,
    pub starting_gold: u32,
    pub starting_food: u32,
    pub starting_inn: u8,
    pub max_party_size: usize,
    pub max_roster_size: usize,
    pub difficulty: crate::Difficulty,
    pub permadeath: bool,
    pub allow_multiclassing: bool,
    pub starting_level: u8,
    pub max_level: u8,

    // Data file paths
    pub items_file: String,
    pub spells_file: String,
    pub monsters_file: String,
    pub classes_file: String,
    pub races_file: String,
    pub characters_file: String,
    pub maps_dir: String,
    pub quests_file: String,
    pub dialogue_file: String,
    pub conditions_file: String,
}

impl CampaignMetadataEditBuffer {
    /// Create a buffer from an existing `CampaignMetadata` instance.
    pub fn from_metadata(m: &crate::CampaignMetadata) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            author: m.author.clone(),
            description: m.description.clone(),
            engine_version: m.engine_version.clone(),
            starting_map: m.starting_map.clone(),
            starting_position: m.starting_position,
            starting_direction: m.starting_direction.clone(),
            starting_gold: m.starting_gold,
            starting_food: m.starting_food,
            starting_inn: m.starting_inn,
            max_party_size: m.max_party_size,
            max_roster_size: m.max_roster_size,
            difficulty: m.difficulty,
            permadeath: m.permadeath,
            allow_multiclassing: m.allow_multiclassing,
            starting_level: m.starting_level,
            max_level: m.max_level,
            items_file: m.items_file.clone(),
            spells_file: m.spells_file.clone(),
            monsters_file: m.monsters_file.clone(),
            classes_file: m.classes_file.clone(),
            races_file: m.races_file.clone(),
            characters_file: m.characters_file.clone(),
            maps_dir: m.maps_dir.clone(),
            quests_file: m.quests_file.clone(),
            dialogue_file: m.dialogue_file.clone(),
            conditions_file: m.conditions_file.clone(),
        }
    }

    /// Apply buffer values into an existing `CampaignMetadata` instance.
    pub fn apply_to(&self, dest: &mut crate::CampaignMetadata) {
        dest.id = self.id.clone();
        dest.name = self.name.clone();
        dest.version = self.version.clone();
        dest.author = self.author.clone();
        dest.description = self.description.clone();
        dest.engine_version = self.engine_version.clone();
        dest.starting_map = self.starting_map.clone();
        dest.starting_position = self.starting_position;
        dest.starting_direction = self.starting_direction.clone();
        dest.starting_gold = self.starting_gold;
        dest.starting_food = self.starting_food;
        dest.starting_inn = self.starting_inn;
        dest.max_party_size = self.max_party_size;
        dest.max_roster_size = self.max_roster_size;
        dest.difficulty = self.difficulty;
        dest.permadeath = self.permadeath;
        dest.allow_multiclassing = self.allow_multiclassing;
        dest.starting_level = self.starting_level;
        dest.max_level = self.max_level;
        dest.items_file = self.items_file.clone();
        dest.spells_file = self.spells_file.clone();
        dest.monsters_file = self.monsters_file.clone();
        dest.classes_file = self.classes_file.clone();
        dest.races_file = self.races_file.clone();
        dest.characters_file = self.characters_file.clone();
        dest.maps_dir = self.maps_dir.clone();
        dest.quests_file = self.quests_file.clone();
        dest.dialogue_file = self.dialogue_file.clone();
        dest.conditions_file = self.conditions_file.clone();
    }
}

impl Default for CampaignMetadataEditBuffer {
    fn default() -> Self {
        // Use the canonical CampaignMetadata defaults to seed the editor buffer.
        CampaignMetadataEditBuffer::from_metadata(&crate::CampaignMetadata::default())
    }
}

/// Editor state for the campaign metadata editor.
///
/// Stores the in-memory metadata and an edit buffer.
///
/// # Examples
///
/// ```
/// use campaign_builder::campaign_editor::{CampaignMetadataEditorState, CampaignEditorMode};
///
/// let mut state = CampaignMetadataEditorState::new();
/// assert_eq!(state.mode, CampaignEditorMode::List);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetadataEditorState {
    /// Editor mode: List / Creating / Editing
    pub mode: CampaignEditorMode,

    /// Metadata currently loaded (authoritative)
    pub metadata: crate::CampaignMetadata,

    /// Edit buffer that mirrors `metadata` while editing
    pub buffer: CampaignMetadataEditBuffer,

    /// Search filter (future)
    pub search_filter: String,

    /// Selected left-side section
    pub selected_section: Option<CampaignSection>,

    /// Unsaved changes in the buffer
    pub has_unsaved_changes: bool,

    /// Import/export dialog and buffer (future)
    pub show_import_dialog: bool,
    pub import_export_buffer: String,

    /// Flag set when the editor requests app-level validation (e.g., user clicked Validate)
    pub validate_requested: bool,
}

impl Default for CampaignMetadataEditorState {
    fn default() -> Self {
        Self {
            mode: CampaignEditorMode::List,
            metadata: crate::CampaignMetadata::default(),
            buffer: CampaignMetadataEditBuffer::default(),
            search_filter: String::new(),
            selected_section: Some(CampaignSection::Overview),
            has_unsaved_changes: false,
            show_import_dialog: false,
            import_export_buffer: String::new(),
            validate_requested: false,
        }
    }
}

impl CampaignMetadataEditorState {
    /// Create a new campaign metadata editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin editing the currently loaded metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::{CampaignMetadataEditorState, CampaignEditorMode};
    ///
    /// let mut state = CampaignMetadataEditorState::new();
    /// state.metadata.id = "x".to_string();
    /// state.start_edit();
    /// assert_eq!(state.mode, CampaignEditorMode::Editing);
    /// ```
    pub fn start_edit(&mut self) {
        self.mode = CampaignEditorMode::Editing;
        self.buffer = CampaignMetadataEditBuffer::from_metadata(&self.metadata);
        self.has_unsaved_changes = false;
    }

    /// Cancel the current edit, restoring the buffer to the loaded metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::{CampaignMetadataEditorState, CampaignEditorMode};
    ///
    /// let mut state = CampaignMetadataEditorState::new();
    /// state.metadata.id = "orig_id".to_string();
    /// state.start_edit();
    /// state.buffer.id = "modified".to_string();
    /// state.cancel_edit();
    /// assert_eq!(state.mode, CampaignEditorMode::List);
    /// assert_eq!(state.buffer.id, state.metadata.id);
    /// ```
    pub fn cancel_edit(&mut self) {
        self.buffer = CampaignMetadataEditBuffer::from_metadata(&self.metadata);
        self.has_unsaved_changes = false;
        self.mode = CampaignEditorMode::List;
    }

    /// Apply the buffer to the authoritative metadata and mark unsaved.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::CampaignMetadataEditorState;
    ///
    /// let mut state = CampaignMetadataEditorState::new();
    /// state.start_edit();
    /// state.buffer.id = "my_campaign".to_string();
    /// state.apply_buffer_to_metadata();
    /// assert!(state.has_unsaved_changes);
    /// assert_eq!(state.metadata.id, "my_campaign");
    /// ```
    pub fn apply_buffer_to_metadata(&mut self) {
        self.buffer.apply_to(&mut self.metadata);
        self.has_unsaved_changes = true;
    }

    /// Consume the current validation request flag.
    /// Consume the current validation request flag.
    ///
    /// Returns `true` if a validation was requested since the last call, and resets the flag.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::CampaignMetadataEditorState;
    ///
    /// let mut state = CampaignMetadataEditorState::new();
    /// state.validate_requested = true;
    /// assert!(state.consume_validate_request());
    /// assert!(!state.validate_requested);
    /// ```
    pub fn consume_validate_request(&mut self) -> bool {
        let requested = self.validate_requested;
        self.validate_requested = false;
        requested
    }

    /// Save the current authoritative metadata to the given path using RON.
    ///
    /// Returns `crate::CampaignError` on error.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::CampaignMetadataEditorState;
    ///
    /// let state = CampaignMetadataEditorState::new();
    /// let tmpdir = tempfile::tempdir().unwrap();
    /// let path = tmpdir.path().join("campaign_save.ron");
    /// let _ = state.save_to_file(path.as_path());
    /// ```
    pub fn save_to_file(&self, path: &Path) -> Result<(), crate::CampaignError> {
        let s = ron::ser::to_string_pretty(&self.metadata, ron::ser::PrettyConfig::default())?;
        fs::write(path, s)?;
        Ok(())
    }

    /// Load campaign metadata from a RON file and replace the current authoritative metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::campaign_editor::CampaignMetadataEditorState;
    ///
    /// let mut state = CampaignMetadataEditorState::new();
    /// let tmpdir = tempfile::tempdir().unwrap();
    /// let path = tmpdir.path().join("campaign_load.ron");
    /// // If a valid RON file exists at `path`, the following loads the metadata.
    /// let _ = state.load_from_file(path.as_path());
    /// ```
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), crate::CampaignError> {
        let contents = fs::read_to_string(path)?;
        let parsed: crate::CampaignMetadata = ron::from_str(&contents)?;
        self.metadata = parsed;
        self.buffer = CampaignMetadataEditBuffer::from_metadata(&self.metadata);
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Show the editor UI within the main campaign builder panel.
    ///
    /// This mirrors the previous `show_metadata_editor` in `main.rs` but delegates
    /// rendering and form state to `CampaignMetadataEditorState`. The function
    /// accepts references to the running app's `campaign` and state helpers to
    /// persist and save files.
    ///
    /// Note: For Phase 1 we keep the UI minimal and rely on paths + save/load functions.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        metadata: &mut crate::CampaignMetadata,
        campaign_path: &mut Option<PathBuf>,
        campaign_dir: Option<&PathBuf>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        ui.heading("Campaign Metadata");
        ui.add_space(5.0);
        ui.label("Basic information about your campaign");
        ui.separator();

        // Toolbar (basic) - supports Search / Save / Load / Import / Export
        let toolbar_action = EditorToolbar::new("Campaign")
            .with_total_count(1)
            .with_search(&mut self.search_filter)
            .show(ui);

        match toolbar_action {
            ToolbarAction::Save => {
                self.apply_buffer_to_metadata();
                if let Some(path) = campaign_path.as_ref() {
                    if let Err(e) = self.save_to_file(path.as_path()) {
                        *status_message = format!("Save failed: {}", e);
                    } else {
                        *unsaved_changes = false;
                        *status_message = format!("Saved campaign to: {}", path.display());
                        // Apply the updated metadata back to the shared campaign metadata
                        *metadata = self.metadata.clone();
                        // Request a validation run so the Validation panel reflects the saved changes
                        self.validate_requested = true;
                    }
                } else if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("campaign.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    if let Err(e) = self.save_to_file(path.as_path()) {
                        *status_message = format!("Save failed: {}", e);
                    } else {
                        *unsaved_changes = false;
                        *status_message = format!("Saved campaign to: {}", path.display());
                        *campaign_path = Some(path);
                        // Also update the shared campaign metadata and request validation on Save As
                        *metadata = self.metadata.clone();
                        self.validate_requested = true;
                    }
                }
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    match self.load_from_file(path.as_path()) {
                        Ok(_) => {
                            *metadata = self.metadata.clone();
                            *unsaved_changes = false;
                            *status_message = format!("Loaded campaign from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load campaign: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }

        // Two-column layout: left side lists sections, right side shows the form for that section.
        // Two-column layout: sections on left, form on right.
        let new_selected = Cell::new(self.selected_section.unwrap_or(CampaignSection::Overview));
        let meta_id = self.metadata.id.clone();
        let meta_name = self.metadata.name.clone();
        let meta_version = self.metadata.version.clone();
        let meta_description = self.metadata.description.clone();

        TwoColumnLayout::new("campaign_metadata_layout")
            .with_left_width(300.0)
            .with_min_height(compute_default_panel_height(ui))
            .show_split(
                ui,
                |left_ui| {
                    left_ui.heading("Sections");
                    left_ui.separator();

                    // List of sections (mutate local `new_selected`, not `self`); use local booleans to avoid borrow conflicts
                    let is_overview = new_selected.get() == CampaignSection::Overview;
                    if left_ui.selectable_label(is_overview, "Overview").clicked() {
                        new_selected.set(CampaignSection::Overview);
                    }

                    let is_gameplay = new_selected.get() == CampaignSection::Gameplay;
                    if left_ui.selectable_label(is_gameplay, "Gameplay").clicked() {
                        new_selected.set(CampaignSection::Gameplay);
                    }

                    let is_files = new_selected.get() == CampaignSection::Files;
                    if left_ui.selectable_label(is_files, "Files").clicked() {
                        new_selected.set(CampaignSection::Files);
                    }

                    let is_advanced = new_selected.get() == CampaignSection::Advanced;
                    if left_ui.selectable_label(is_advanced, "Advanced").clicked() {
                        new_selected.set(CampaignSection::Advanced);
                    }

                    left_ui.separator();
                    left_ui.heading("Preview");
                    left_ui.add_space(6.0);
                    left_ui.label(format!("ID: {}", meta_id));
                    left_ui.label(format!("Name: {}", meta_name));
                    left_ui.label(format!("Version: {}", meta_version));
                    left_ui.add_space(4.0);
                    if !meta_description.is_empty() {
                        left_ui.label(egui::RichText::new(&meta_description).small());
                    } else {
                        left_ui.colored_label(egui::Color32::GRAY, "No description");
                    }
                },
                |right_ui| {
                    // Show the selected section's form
                    let selected = new_selected.get();
                    match selected {
                        CampaignSection::Overview => {
                            // Overview grid: ID, Name, Version, Author, Engine, Description
                            egui::Grid::new("campaign_overview_grid")
                                .num_columns(2)
                                .spacing([10.0, 8.0])
                                .striped(true)
                                .show(right_ui, |ui| {
                                    ui.label("Campaign ID:");
                                    if ui.text_edit_singleline(&mut self.buffer.id).changed() {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Name:");
                                    if ui.text_edit_singleline(&mut self.buffer.name).changed() {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Version:");
                                    if ui.text_edit_singleline(&mut self.buffer.version).changed() {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Author:");
                                    if ui.text_edit_singleline(&mut self.buffer.author).changed() {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Engine Version:");
                                    if ui
                                        .text_edit_singleline(&mut self.buffer.engine_version)
                                        .changed()
                                    {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();
                                });

                            right_ui.add_space(10.0);
                            right_ui.label("Description:");
                            let response = right_ui.add(
                                egui::TextEdit::multiline(&mut self.buffer.description)
                                    .desired_rows(6),
                            );
                            if response.changed() {
                                self.has_unsaved_changes = true;
                                *unsaved_changes = true;
                            }
                        }

                        CampaignSection::Files => {
                            // Files grid: items, spells, monsters, classes, races, characters, maps_dir, quests, dialogue, conditions
                            egui::Grid::new("campaign_files_grid")
                                .num_columns(2)
                                .spacing([10.0, 8.0])
                                .striped(true)
                                .show(right_ui, |ui| {
                                    // Helper macro-like inline: label + path + browse
                                    ui.label("Items File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.items_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.items_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Spells File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.spells_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.spells_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Monsters File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.monsters_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.monsters_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Classes File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.classes_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.classes_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Races File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.races_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.races_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Characters File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.characters_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.characters_file =
                                                    p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Maps Directory:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.maps_dir)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📂").on_hover_text("Browse Folder").clicked()
                                        {
                                            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                                self.buffer.maps_dir = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Quests File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.quests_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.quests_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Dialogue File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.dialogue_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.dialogue_file = p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Conditions File:");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .text_edit_singleline(&mut self.buffer.conditions_file)
                                            .changed()
                                        {
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.button("📁").on_hover_text("Browse").clicked() {
                                            if let Some(p) = rfd::FileDialog::new()
                                                .add_filter("RON", &["ron"])
                                                .pick_file()
                                            {
                                                self.buffer.conditions_file =
                                                    p.display().to_string();
                                                self.has_unsaved_changes = true;
                                                *unsaved_changes = true;
                                            }
                                        }
                                    });
                                    ui.end_row();
                                });
                        }

                        CampaignSection::Gameplay => {
                            // Gameplay grid: starting map/position/direction, gold/food, difficulty, flags, levels, limits
                            egui::Grid::new("campaign_gameplay_grid")
                                .num_columns(2)
                                .spacing([10.0, 8.0])
                                .striped(true)
                                .show(right_ui, |ui| {
                                    ui.label("Starting Map:");
                                    if ui
                                        .text_edit_singleline(&mut self.buffer.starting_map)
                                        .changed()
                                    {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Starting Position (X, Y):");
                                    ui.horizontal(|ui| {
                                        let mut x = self.buffer.starting_position.0 as i32;
                                        let mut y = self.buffer.starting_position.1 as i32;
                                        if ui.add(egui::DragValue::new(&mut x)).changed() {
                                            self.buffer.starting_position.0 = x.max(0) as u32;
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut y)).changed() {
                                            self.buffer.starting_position.1 = y.max(0) as u32;
                                            self.has_unsaved_changes = true;
                                            *unsaved_changes = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Starting Direction:");
                                    let mut dir = self.buffer.starting_direction.clone();
                                    // Use `from_id_source` to avoid ID collisions with other ComboBoxes in the UI
                                    egui::ComboBox::from_id_salt("campaign_starting_direction")
                                        .selected_text(dir.clone())
                                        .show_ui(ui, |ui| {
                                            for d in &["North", "East", "South", "West"] {
                                                if ui.selectable_label(dir == *d, *d).clicked() {
                                                    dir = (*d).to_string();
                                                    self.buffer.starting_direction = dir.clone();
                                                    self.has_unsaved_changes = true;
                                                    *unsaved_changes = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Starting Gold:");
                                    let mut gold = self.buffer.starting_gold as i64;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut gold)
                                                .range(0..=crate::STARTING_GOLD_MAX as i64),
                                        )
                                        .changed()
                                    {
                                        self.buffer.starting_gold = gold.max(0) as u32;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Starting Food:");
                                    let mut food = self.buffer.starting_food as i64;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut food)
                                                .range(FOOD_MIN as i64..=FOOD_MAX as i64),
                                        )
                                        .changed()
                                    {
                                        self.buffer.starting_food =
                                            (food.max(FOOD_MIN as i64)) as u32;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Starting Inn:")
                                        .on_hover_text("Default inn where non-party premade characters start (default: 1)");
                                    let mut inn = self.buffer.starting_inn as i32;
                                    if ui
                                        .add(egui::DragValue::new(&mut inn).range(1..=255))
                                        .changed()
                                    {
                                        self.buffer.starting_inn = (inn.max(1)) as u8;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Difficulty:");
                                    // Use `from_id_source` to avoid ID collisions with other ComboBoxes in the UI
                                    egui::ComboBox::from_id_salt("campaign_difficulty")
                                        .selected_text(self.buffer.difficulty.as_str())
                                        .show_ui(ui, |ui| {
                                            for &diff in &crate::Difficulty::all() {
                                                if ui
                                                    .selectable_label(
                                                        self.buffer.difficulty == diff,
                                                        diff.as_str(),
                                                    )
                                                    .clicked()
                                                {
                                                    self.buffer.difficulty = diff;
                                                    self.has_unsaved_changes = true;
                                                    *unsaved_changes = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Permadeath:");
                                    if ui.checkbox(&mut self.buffer.permadeath, "").changed() {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Allow Multiclassing:");
                                    if ui
                                        .checkbox(&mut self.buffer.allow_multiclassing, "")
                                        .changed()
                                    {
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Starting Level:");
                                    let mut start_level = self.buffer.starting_level as i32;
                                    if ui
                                        .add(egui::DragValue::new(&mut start_level).range(1..=255))
                                        .changed()
                                    {
                                        self.buffer.starting_level = (start_level.max(1)) as u8;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Max Level:");
                                    let mut max_level = self.buffer.max_level as i32;
                                    if ui
                                        .add(egui::DragValue::new(&mut max_level).range(1..=255))
                                        .changed()
                                    {
                                        self.buffer.max_level = (max_level.max(1)) as u8;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Max Party Size:");
                                    let mut max_party = self.buffer.max_party_size as i32;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut max_party)
                                                .range(1..=(PARTY_MAX_SIZE as i32)),
                                        )
                                        .changed()
                                    {
                                        self.buffer.max_party_size = max_party.max(1) as usize;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();

                                    ui.label("Max Roster Size:");
                                    let mut roster = self.buffer.max_roster_size as i32;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut roster)
                                                .range(self.buffer.max_party_size as i32..=1000),
                                        )
                                        .changed()
                                    {
                                        self.buffer.max_roster_size =
                                            roster.max(self.buffer.max_party_size as i32) as usize;
                                        self.has_unsaved_changes = true;
                                        *unsaved_changes = true;
                                    }
                                    ui.end_row();
                                });

                            // Inline feedback for common configuration mistakes
                            if self.buffer.max_roster_size < self.buffer.max_party_size {
                                right_ui.colored_label(
                                    egui::Color32::RED,
                                    "Max roster size must be >= max party size.",
                                );
                            }
                            if (self.buffer.starting_level == 0)
                                || (self.buffer.starting_level > self.buffer.max_level)
                            {
                                right_ui.colored_label(
                                    egui::Color32::RED,
                                    "Starting level must be between 1 and max level.",
                                );
                            }
                        }

                        CampaignSection::Advanced => {
                            // Advanced shows a concise representation of all fields not included above, and a RON export utility
                            right_ui.label("Advanced");
                            right_ui.add_space(6.0);
                            right_ui.label("Engine & File Metadata (short)");
                            egui::Grid::new("campaign_advanced_grid")
                                .num_columns(2)
                                .spacing([10.0, 8.0])
                                .striped(true)
                                .show(right_ui, |ui| {
                                    ui.label("Engine Version:");
                                    ui.label(&self.buffer.engine_version);
                                    ui.end_row();

                                    ui.label("Starting Map:");
                                    ui.label(&self.buffer.starting_map);
                                    ui.end_row();

                                    ui.label("Data files path preview:");
                                    ui.label(&self.buffer.items_file);
                                    ui.end_row();
                                });

                            right_ui.add_space(10.0);
                            right_ui.horizontal(|ui| {
                                if ui.button("Export RON").clicked() {
                                    // Show dialog with RON content pre-filled (uses import/export dialog in ui_helpers)
                                    self.show_import_dialog = true;
                                    if let Ok(serialized) = ron::ser::to_string_pretty(
                                        &self.buffer,
                                        ron::ser::PrettyConfig::default(),
                                    ) {
                                        self.import_export_buffer = serialized;
                                    }
                                }
                            });

                            // Render import/export dialog if requested
                            if self.show_import_dialog {
                                let mut dlg_state =
                                    crate::ui_helpers::ImportExportDialogState::new();
                                dlg_state.open_export(self.import_export_buffer.clone());
                                let result = crate::ui_helpers::ImportExportDialog::new(
                                    "Export Campaign Metadata",
                                    &mut dlg_state,
                                )
                                .show(right_ui.ctx());
                                // import/export state is ephemeral for now
                                if let crate::ui_helpers::ImportExportResult::Cancel = result {
                                    self.show_import_dialog = false;
                                }
                            }
                        }
                    }

                    // Inspector bottom actions (Back, Save, Validate)
                    right_ui.add_space(10.0);
                    right_ui.separator();
                    right_ui.add_space(6.0);
                    right_ui.horizontal(|ui| {
                        if ui.button("⬅ Back to List").clicked() {
                            self.mode = CampaignEditorMode::List;
                        }

                        if ui.button("💾 Save Campaign").clicked() {
                            self.apply_buffer_to_metadata();
                            if let Some(path) = campaign_path.as_ref() {
                                match self.save_to_file(path.as_path()) {
                                    Ok(_) => {
                                        *unsaved_changes = false;
                                        *status_message =
                                            format!("Saved campaign to {}", path.display());
                                        *metadata = self.metadata.clone();
                                        // Request a validation run after a successful save so the
                                        // Validation panel shows updated results.
                                        self.validate_requested = true;
                                    }
                                    Err(e) => *status_message = format!("Save failed: {}", e),
                                }
                            } else if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("campaign.ron")
                                .add_filter("RON", &["ron"])
                                .save_file()
                            {
                                match self.save_to_file(path.as_path()) {
                                    Ok(_) => {
                                        *campaign_path = Some(path.clone());
                                        *unsaved_changes = false;
                                        *status_message =
                                            format!("Saved campaign to {}", path.display());
                                        *metadata = self.metadata.clone();
                                        // Request a validation run after a successful Save As
                                        // so the Validation panel shows updated results.
                                        self.validate_requested = true;
                                    }
                                    Err(e) => *status_message = format!("Save failed: {}", e),
                                }
                            }
                        }

                        if ui.button("✅ Validate").clicked() {
                            // Apply pending edits in the buffer to the editor's metadata and
                            // update the shared `metadata` reference so the app-level validator
                            // validates the latest values without requiring an explicit save.
                            self.apply_buffer_to_metadata();
                            *metadata = self.metadata.clone();

                            // Signal to the app that validation was requested. The app should
                            // call `validate_campaign()` and switch to the Validation tab when
                            // this flag is set.
                            self.validate_requested = true;

                            *status_message =
                                "Validation requested from Campaign metadata editor".to_string();
                        }
                    });
                },
            );

        // Persist the potentially changed selection back into the state
        self.selected_section = Some(new_selected.get());

        // End TwoColumnLayout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    /// Test that the editor state initializes correctly
    #[test]
    fn test_campaign_metadata_editor_new() {
        let s = CampaignMetadataEditorState::new();
        assert_eq!(s.mode, CampaignEditorMode::List);
        assert!(
            s.buffer.id.is_empty(),
            "default campaign id should be empty string"
        );
    }

    /// Roundtrip save and load to/from a temporary file
    #[test]
    fn test_save_and_load_roundtrip() {
        let mut s = CampaignMetadataEditorState::new();
        s.metadata.id = "test_campaign".to_string();
        s.metadata.name = "Test Campaign".to_string();
        s.buffer = CampaignMetadataEditBuffer::from_metadata(&s.metadata);

        let dir = tempdir().expect("Failed to create tempdir");
        let path = dir.path().join("campaign_test.ron");

        // Ensure the file does not exist before save
        assert!(!path.exists());

        s.apply_buffer_to_metadata();
        s.save_to_file(path.as_path())
            .expect("Failed to save campaign");

        assert!(path.exists());

        // Load into a new state
        let mut loaded = CampaignMetadataEditorState::new();
        loaded
            .load_from_file(path.as_path())
            .expect("Failed to load campaign");
        assert_eq!(loaded.metadata.id, "test_campaign");
        assert_eq!(loaded.metadata.name, "Test Campaign");
    }

    /// Cancel operation should reset buffer to authoritative metadata
    #[test]
    fn test_cancel_edit_restores_buffer() {
        let mut s = CampaignMetadataEditorState::new();
        s.metadata.id = "orig_id".to_string();
        s.metadata.name = "Original".to_string();
        s.buffer = CampaignMetadataEditBuffer::from_metadata(&s.metadata);

        // User starts editing
        s.start_edit();
        s.buffer.id = "modified".to_string();
        s.buffer.name = "Modified".to_string();

        // Cancel should restore buffer
        s.cancel_edit();
        assert_eq!(s.buffer.id, "orig_id");
        assert_eq!(s.buffer.name, "Original");
        assert_eq!(s.mode, CampaignEditorMode::List);
    }

    /// apply_buffer_to_metadata should update authoritative metadata and mark unsaved
    #[test]
    fn test_apply_buffer_to_metadata_updates_metadata_and_unsaved() {
        let mut s = CampaignMetadataEditorState::new();
        s.buffer.id = "new_id".to_string();
        s.buffer.name = "New Name".to_string();
        s.buffer.starting_gold = 123u32;

        s.apply_buffer_to_metadata();

        assert_eq!(s.metadata.id, "new_id");
        assert_eq!(s.metadata.name, "New Name");
        assert_eq!(s.metadata.starting_gold, 123u32);
        assert!(
            s.has_unsaved_changes,
            "apply_buffer_to_metadata should set has_unsaved_changes"
        );
    }

    /// consume_validate_request should return the current value and reset the flag
    #[test]
    fn test_consume_validate_request_resets_flag() {
        let mut s = CampaignMetadataEditorState::new();

        // Initially, no validation requested
        assert!(!s.consume_validate_request());

        // Request validation and check that it is consumed
        s.validate_requested = true;
        assert!(s.consume_validate_request());

        // After consuming, it should be reset
        assert!(!s.consume_validate_request());
    }
}
