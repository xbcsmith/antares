// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Metadata Editor
//!
//! Phase 1 - Foundation: extraction & module setup for a focused Campaign
//! metadata editor. This module contains the `CampaignMetadataEditorState`
//! which is a lightweight editor state and file operations for campaign
//! metadata. UI rendering is provided by `show()` which mirrors the basic
//! layout previously implemented in `main.rs` and exposes an editing flow
//! compatible with other editor modules in the SDK.

use crate::ui_helpers::{EditorToolbar, ToolbarAction};
use eframe::egui;
use ron;
use serde::{Deserialize, Serialize};
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
    Overview,
    Files,
    Gameplay,
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
/// let mut state = campaign_editor::CampaignMetadataEditorState::new();
/// assert_eq!(state.mode, campaign_editor::CampaignEditorMode::List);
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
}

impl Default for CampaignMetadataEditorState {
    fn default() -> Self {
        let meta = crate::CampaignMetadata::default();
        Self {
            mode: CampaignEditorMode::List,
            metadata: meta.clone(),
            buffer: CampaignMetadataEditBuffer::from_metadata(&meta),
            search_filter: String::new(),
            selected_section: Some(CampaignSection::Overview),
            has_unsaved_changes: false,
            show_import_dialog: false,
            import_export_buffer: String::new(),
        }
    }
}

impl CampaignMetadataEditorState {
    /// Create a new campaign metadata editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin editing the currently loaded metadata
    pub fn start_edit(&mut self) {
        self.mode = CampaignEditorMode::Editing;
        self.buffer = CampaignMetadataEditBuffer::from_metadata(&self.metadata);
        self.has_unsaved_changes = false;
    }

    /// Cancel the current edit, restoring the buffer to the loaded metadata.
    pub fn cancel_edit(&mut self) {
        self.buffer = CampaignMetadataEditBuffer::from_metadata(&self.metadata);
        self.has_unsaved_changes = false;
        self.mode = CampaignEditorMode::List;
    }

    /// Apply the buffer to the authoritative metadata and mark unsaved.
    pub fn apply_buffer_to_metadata(&mut self) {
        self.buffer.apply_to(&mut self.metadata);
        self.has_unsaved_changes = true;
    }

    /// Save the current authoritative metadata to the given path using RON.
    ///
    /// Returns `crate::CampaignError` on error.
    pub fn save_to_file(&self, path: &Path) -> Result<(), crate::CampaignError> {
        let s = ron::ser::to_string_pretty(&self.metadata, ron::ser::PrettyConfig::default())?;
        fs::write(path, s)?;
        Ok(())
    }

    /// Load campaign metadata from a RON file and replace the current authoritative metadata.
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

        // Toolbar (basic)
        let toolbar_action = EditorToolbar::new("Campaign")
            .with_total_count(1)
            .with_search(&mut self.search_filter)
            .show(ui);

        match toolbar_action {
            ToolbarAction::Save => {
                // Apply buffer to metadata, then persist to disk / campaign path
                self.apply_buffer_to_metadata();
                if let Some(path) = campaign_path.as_ref() {
                    if let Err(e) = self.save_to_file(path.as_path()) {
                        *status_message = format!("Save failed: {}", e);
                    } else {
                        *unsaved_changes = false;
                        *status_message = format!("Saved campaign to: {}", path.display());
                    }
                } else {
                    // If no existing path, prompt for save as
                    if let Some(path) = rfd::FileDialog::new()
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
                        }
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
                            // Keep the selected campaign path in the app if present
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load campaign: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("metadata_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    // Campaign ID
                    ui.label("Campaign ID:");
                    if ui.text_edit_singleline(&mut self.buffer.id).changed() {
                        self.has_unsaved_changes = true;
                        *unsaved_changes = true;
                    }
                    ui.end_row();

                    // Campaign Name
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut self.buffer.name).changed() {
                        self.has_unsaved_changes = true;
                        *unsaved_changes = true;
                    }
                    ui.end_row();

                    // Version
                    ui.label("Version:");
                    if ui.text_edit_singleline(&mut self.buffer.version).changed() {
                        self.has_unsaved_changes = true;
                        *unsaved_changes = true;
                    }
                    ui.end_row();

                    // Author
                    ui.label("Author:");
                    if ui.text_edit_singleline(&mut self.buffer.author).changed() {
                        self.has_unsaved_changes = true;
                        *unsaved_changes = true;
                    }
                    ui.end_row();

                    // Engine Version
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

            ui.add_space(10.0);
            ui.label("Description:");
            let response =
                ui.add(egui::TextEdit::multiline(&mut self.buffer.description).desired_rows(6));
            if response.changed() {
                self.has_unsaved_changes = true;
                *unsaved_changes = true;
            }

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("⬅ Back to List").clicked() {
                    self.mode = CampaignEditorMode::List;
                }

                if ui.button("💾 Save Campaign").clicked() {
                    // Duplicate save flow to support the non-toolbar button
                    self.apply_buffer_to_metadata();
                    if let Some(path) = campaign_path.as_ref() {
                        match self.save_to_file(path.as_path()) {
                            Ok(_) => {
                                *unsaved_changes = false;
                                *status_message = format!("Saved campaign to {}", path.display());
                                // propagate metadata changes back to the provided metadata reference
                                *metadata = self.metadata.clone();
                            }
                            Err(e) => *status_message = format!("Save failed: {}", e),
                        }
                    } else {
                        if let Some(path) = rfd::FileDialog::new()
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
                                }
                                Err(e) => *status_message = format!("Save failed: {}", e),
                            }
                        }
                    }
                }

                if ui.button("✅ Validate").clicked() {
                    // Validation is performed at the app-level via `validate_campaign`.
                    // For now we set a helpful status message and the app can be
                    // wired to perform validation after calling this function.
                    *status_message =
                        "Validation requested from Campaign metadata editor".to_string();
                }
            });
        });
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
}
