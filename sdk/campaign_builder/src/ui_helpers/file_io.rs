// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! CSV helpers, import/export dialog, and file load/save/reload functions.
//!
//! Contains [`CsvParseError`], [`parse_id_csv_to_vec`], [`format_vec_to_csv`],
//! [`ImportExportResult`], [`ImportExportDialogState`], [`ImportExportDialog`],
//! [`load_ron_file`], [`save_ron_file`], [`handle_file_load`],
//! [`handle_file_save`], and [`handle_reload`].

use crate::editor_context::EditorContext;
use crate::ui_helpers::layout::ToolbarAction;
use eframe::egui;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

/// Errors when parsing CSV-like ID lists
#[derive(Debug, Error)]
pub enum CsvParseError {
    #[error("Invalid token '{token}': {error}")]
    InvalidToken { token: String, error: String },
}

/// Parses a comma-separated list of IDs into a Vec<T>.
///
/// - Trims whitespace around elements
/// - Ignores empty tokens
/// - Returns a `CsvParseError` if any token fails to parse
///
/// # Examples
///
/// ```
/// # use campaign_builder::ui_helpers::parse_id_csv_to_vec;
/// let parsed = parse_id_csv_to_vec::<u8>("1, 2, 3").unwrap();
/// assert_eq!(parsed, vec![1, 2, 3u8]);
/// ```
pub fn parse_id_csv_to_vec<T>(csv: &str) -> Result<Vec<T>, CsvParseError>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    if csv.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut ids = Vec::new();
    let tokens = csv.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()); // Legitimate: CSV helper
    for token in tokens {
        match token.parse::<T>() {
            Ok(v) => ids.push(v),
            Err(e) => {
                return Err(CsvParseError::InvalidToken {
                    token: token.to_string(),
                    error: e.to_string(),
                })
            }
        }
    }
    Ok(ids)
}

/// Formats a Vec<T> into a user-friendly CSV string using `", "` separators.
///
/// # Examples
///
/// ```
/// # use campaign_builder::ui_helpers::format_vec_to_csv;
/// let out = format_vec_to_csv(&[1u8, 2u8, 3u8]);
/// assert_eq!(out, "1, 2, 3");
/// ```
pub fn format_vec_to_csv<T>(values: &[T]) -> String
where
    T: Display,
{
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

// =============================================================================
// Import/Export Dialog Component
// =============================================================================

/// Result of import/export dialog interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportExportResult {
    /// User confirmed import with the provided RON data
    Import(String),
    /// User cancelled the dialog
    Cancel,
    /// Dialog is still open, no action taken
    Open,
}

/// State for the import/export dialog.
#[derive(Debug, Default)]
pub struct ImportExportDialogState {
    /// The RON text buffer
    pub buffer: String,
    /// Whether the dialog is currently open
    pub is_open: bool,
    /// Error message to display, if any
    pub error_message: Option<String>,
    /// Dialog mode (true = export/read-only, false = import/editable)
    pub export_mode: bool,
}

impl ImportExportDialogState {
    /// Creates a new dialog state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the dialog in import mode (editable).
    pub fn open_import(&mut self) {
        self.is_open = true;
        self.buffer.clear();
        self.error_message = None;
        self.export_mode = false;
    }

    /// Opens the dialog in export mode (read-only) with the provided content.
    ///
    /// # Arguments
    ///
    /// * `content` - The RON content to display
    pub fn open_export(&mut self, content: String) {
        self.is_open = true;
        self.buffer = content;
        self.error_message = None;
        self.export_mode = true;
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.is_open = false;
        self.buffer.clear();
        self.error_message = None;
    }

    /// Sets an error message to display.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }
}

/// A dialog for importing and exporting RON data.
///
/// This component provides a modal window with a text area for RON data and
/// appropriate action buttons for import/export operations.
pub struct ImportExportDialog<'a> {
    /// Dialog title
    title: &'a str,
    /// Dialog state
    state: &'a mut ImportExportDialogState,
    /// Width of the dialog window
    width: f32,
    /// Height of the dialog window
    height: f32,
}

impl<'a> ImportExportDialog<'a> {
    /// Creates a new import/export dialog.
    ///
    /// # Arguments
    ///
    /// * `title` - Title for the dialog window
    /// * `state` - Mutable reference to the dialog state
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::{ImportExportDialog, ImportExportDialogState, ImportExportResult};
    ///
    /// fn example(ctx: &egui::Context, state: &mut ImportExportDialogState) {
    ///     let result = ImportExportDialog::new("Import Item", state)
    ///         .show(ctx);
    ///
    ///     match result {
    ///         ImportExportResult::Import(ron_data) => {
    ///             // Parse and import the data
    ///         }
    ///         ImportExportResult::Cancel => {
    ///             // User cancelled
    ///         }
    ///         ImportExportResult::Open => {
    ///             // Dialog still open
    ///         }
    ///     }
    /// }
    /// ```
    pub fn new(title: &'a str, state: &'a mut ImportExportDialogState) -> Self {
        Self {
            title,
            state,
            width: 500.0,
            height: 400.0,
        }
    }

    /// Sets a custom width for the dialog.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets a custom height for the dialog.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Shows the dialog and returns the result.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context
    ///
    /// # Returns
    ///
    /// - `ImportExportResult::Import(data)` when user confirms import
    /// - `ImportExportResult::Cancel` when user cancels or closes dialog
    /// - `ImportExportResult::Open` when dialog is still open
    pub fn show(self, ctx: &egui::Context) -> ImportExportResult {
        if !self.state.is_open {
            return ImportExportResult::Cancel;
        }

        let mut result = ImportExportResult::Open;

        egui::Window::new(self.title)
            .collapsible(false)
            .resizable(true)
            .default_width(self.width)
            .default_height(self.height)
            .show(ctx, |ui| {
                // Error message
                if let Some(ref error) = self.state.error_message {
                    ui.colored_label(egui::Color32::RED, error);
                    ui.separator();
                }

                // Instructions
                if self.state.export_mode {
                    ui.label("Copy the RON data below:");
                } else {
                    ui.label("Paste RON data below:");
                }

                // Text area
                egui::ScrollArea::vertical()
                    .max_height(self.height - 100.0)
                    .show(ui, |ui| {
                        if self.state.export_mode {
                            // Read-only for export
                            let mut readonly_buffer = self.state.buffer.clone();
                            ui.add(
                                egui::TextEdit::multiline(&mut readonly_buffer)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .interactive(true), // Allow selection for copying
                            );
                        } else {
                            // Editable for import
                            ui.add(
                                egui::TextEdit::multiline(&mut self.state.buffer)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        }
                    });

                ui.separator();

                // Action buttons
                ui.horizontal(|ui| {
                    if self.state.export_mode {
                        if ui.button("📋 Copy to Clipboard").clicked() {
                            ui.ctx().copy_text(self.state.buffer.clone());
                        }
                        if ui.button("Close").clicked() {
                            self.state.close();
                            result = ImportExportResult::Cancel;
                        }
                    } else {
                        if ui.button("📥 Import").clicked() {
                            result = ImportExportResult::Import(self.state.buffer.clone());
                            self.state.close();
                        }
                        if ui.button("Cancel").clicked() {
                            self.state.close();
                            result = ImportExportResult::Cancel;
                        }
                    }
                });
            });

        result
    }
}

// =============================================================================
// File I/O Helper Functions
// =============================================================================

/// Loads data from a RON file with error handling.
///
/// # Type Parameters
///
/// * `T` - The type to deserialize, must implement `serde::de::DeserializeOwned`
///
/// # Arguments
///
/// * `path` - Path to the RON file
///
/// # Returns
///
/// `Ok(T)` on success, `Err(String)` with error message on failure.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::Item;
/// use campaign_builder::ui_helpers::load_ron_file;
/// use std::path::Path;
///
/// let items: Result<Vec<Item>, String> = load_ron_file(Path::new("data/items.ron"));
/// ```
pub fn load_ron_file<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    ron::from_str(&contents)
        .map_err(|e| format!("Failed to parse RON from {}: {}", path.display(), e))
}

/// Errors that can occur during RON file I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum FileIoError {
    /// An OS-level I/O error (file not found, permission denied, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The data could not be serialised to RON.
    #[error("RON serialisation error: {0}")]
    Serialization(String),
}

/// Saves data to a RON file with pretty formatting.
///
/// # Type Parameters
///
/// * `T` - The type to serialize, must implement `serde::Serialize`
///
/// # Arguments
///
/// * `data` - Reference to the data to serialize
/// * `path` - Path to write the RON file
///
/// # Returns
///
/// `Ok(())` on success, `Err(FileIoError)` on failure.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::Item;
/// use campaign_builder::ui_helpers::save_ron_file;
/// use std::path::Path;
///
/// let items: Vec<Item> = vec![];
/// save_ron_file(&items, Path::new("data/items.ron")).expect("Failed to save");
/// ```
pub fn save_ron_file<T: serde::Serialize>(
    data: &T,
    path: &std::path::Path,
) -> Result<(), FileIoError> {
    let contents = ron::ser::to_string_pretty(data, Default::default())
        .map_err(|e| FileIoError::Serialization(e.to_string()))?;

    std::fs::write(path, contents)?;
    Ok(())
}

/// Handles file load action for an editor.
///
/// This function opens a file dialog, loads RON data, and either merges or replaces
/// the existing data based on the merge mode flag.
///
/// # Type Parameters
///
/// * `T` - The entity type, must implement Clone, DeserializeOwned, and have an `id` field
///
/// # Arguments
///
/// * `data` - Mutable reference to the data vector
/// * `merge_mode` - Whether to merge with existing data or replace
/// * `id_getter` - Function to get the ID from an entity
/// * `status_message` - Mutable reference to update with status
/// * `unsaved_changes` - Mutable flag to mark unsaved changes
///
/// # Returns
///
/// `true` if data was loaded, `false` otherwise.
pub fn handle_file_load<T, K, F>(
    data: &mut Vec<T>,
    merge_mode: bool,
    id_getter: F,
    status_message: &mut String,
    unsaved_changes: &mut bool,
) -> bool
where
    T: Clone + serde::de::DeserializeOwned,
    K: PartialEq + Clone,
    F: Fn(&T) -> K,
{
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("RON", &["ron"])
        .pick_file()
    {
        match load_ron_file::<Vec<T>>(&path) {
            Ok(loaded_data) => {
                if merge_mode {
                    // Merge: update existing, add new
                    for item in loaded_data {
                        let item_key = id_getter(&item);
                        if let Some(existing) = data.iter_mut().find(|d| id_getter(d) == item_key) {
                            *existing = item;
                        } else {
                            data.push(item);
                        }
                    }
                } else {
                    // Replace: clear and load
                    *data = loaded_data;
                }
                *unsaved_changes = true;
                *status_message = format!("Loaded from: {}", path.display());
                return true;
            }
            Err(e) => {
                *status_message = e;
            }
        }
    }
    false
}

/// Handles file save action for an editor.
///
/// This function opens a save file dialog and writes the data as pretty-formatted RON.
///
/// # Type Parameters
///
/// * `T` - The entity type, must implement Serialize
///
/// # Arguments
///
/// * `data` - Reference to the data to save
/// * `default_filename` - Default filename to suggest
/// * `status_message` - Mutable reference to update with status
///
/// # Returns
///
/// `true` if data was saved, `false` otherwise.
pub fn handle_file_save<T: serde::Serialize>(
    data: &[T],
    default_filename: &str,
    status_message: &mut String,
) -> bool {
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(default_filename)
        .add_filter("RON", &["ron"])
        .save_file()
    {
        match save_ron_file(&data, &path) {
            Ok(()) => {
                *status_message = format!("Saved to: {}", path.display());
                return true;
            }
            Err(e) => {
                *status_message = e.to_string();
            }
        }
    }
    false
}

/// Handles reload action for an editor.
///
/// This function reloads data from the campaign directory.
///
/// # Type Parameters
///
/// * `T` - The entity type
///
/// # Arguments
///
/// * `data` - Mutable reference to the data vector
/// * `campaign_dir` - Optional campaign directory
/// * `filename` - Filename within the campaign directory
/// * `status_message` - Mutable reference to update with status
///
/// # Returns
///
/// `true` if data was reloaded, `false` otherwise.
pub fn handle_reload<T: serde::de::DeserializeOwned>(
    data: &mut Vec<T>,
    campaign_dir: Option<&PathBuf>,
    filename: &str,
    status_message: &mut String,
) -> bool {
    if let Some(dir) = campaign_dir {
        let path = dir.join(filename);
        if path.exists() {
            match load_ron_file::<Vec<T>>(&path) {
                Ok(loaded_data) => {
                    *data = loaded_data;
                    *status_message = format!("Reloaded from: {}", path.display());
                    return true;
                }
                Err(e) => {
                    *status_message = e;
                }
            }
        } else {
            *status_message = format!("File not found: {}", path.display());
        }
    } else {
        *status_message = "No campaign directory set".to_string();
    }
    false
}

/// Dispatches the common toolbar actions (`Save`, `Load`, `Export`, `Reload`,
/// `None`) for any list-based editor that holds a `Vec<T>`.
///
/// The `New` and `Import` arms are intentionally **not** handled here because
/// they differ between editors.  In the calling `match` block, pattern those
/// arms first and pass everything else through a catch-all arm:
///
/// ```no_run
/// # use campaign_builder::ui_helpers::{ToolbarAction, handle_toolbar_action};
/// # use campaign_builder::editor_context::EditorContext;
/// # use std::path::PathBuf;
/// # let action = ToolbarAction::None;
/// # let mut data: Vec<u32> = vec![];
/// # let mut editor_unsaved = false;
/// # let dir = PathBuf::from("/tmp");
/// # let mut unsaved = false;
/// # let mut status = String::new();
/// # let mut merge = false;
/// # let mut ctx = EditorContext::new(Some(&dir), "data.ron", &mut unsaved, &mut status, &mut merge);
/// match action {
///     ToolbarAction::New    => { /* editor-specific */ }
///     ToolbarAction::Import => { /* editor-specific */ }
///     other => handle_toolbar_action(
///         other,
///         &mut data,
///         |x: &u32| *x,
///         &mut editor_unsaved,
///         &mut ctx,
///         "data.ron",
///         "items",
///     ),
/// }
/// ```
///
/// # Type Parameters
///
/// * `T` - Entity type.  Must be cloneable and round-trip through RON.
/// * `K` - The ID key type used by `id_getter` for merge-load deduplication.
/// * `F` - Closure that extracts the deduplication key from an entity.
///
/// # Arguments
///
/// * `action`          – The toolbar action returned by [`EditorToolbar::show`].
/// * `data`            – Mutable reference to the editor's data vector.
/// * `id_getter`       – Extracts the key used to deduplicate on merge-load.
/// * `editor_unsaved`  – Editor-level dirty flag; set to `false` on `Reload`.
/// * `ctx`             – Shared editor context (campaign dir, data file, …).
/// * `export_filename` – Default filename suggested to the user by Export dialog.
/// * `noun`            – Human-readable plural noun for status messages (e.g. `"classes"`).
pub fn handle_toolbar_action<T, K, F>(
    action: ToolbarAction,
    data: &mut Vec<T>,
    id_getter: F,
    editor_unsaved: &mut bool,
    ctx: &mut EditorContext<'_>,
    export_filename: &str,
    noun: &str,
) where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    K: PartialEq + Clone,
    F: Fn(&T) -> K,
{
    match action {
        ToolbarAction::Save => {
            if let Some(dir) = ctx.campaign_dir {
                let path = dir.join(ctx.data_file);
                if let Some(parent) = path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        *ctx.status_message = format!("Failed to save {}: {}", noun, e);
                        return;
                    }
                }
                match save_ron_file(data, &path) {
                    Ok(()) => {
                        *ctx.status_message = format!("Saved {} {}", data.len(), noun);
                    }
                    Err(e) => {
                        *ctx.status_message = format!("Failed to save {}: {}", noun, e);
                    }
                }
            }
        }
        ToolbarAction::Load => {
            handle_file_load(
                data,
                *ctx.file_load_merge_mode,
                id_getter,
                ctx.status_message,
                ctx.unsaved_changes,
            );
        }
        ToolbarAction::Export => {
            handle_file_save(data, export_filename, ctx.status_message);
        }
        ToolbarAction::Reload => {
            if let Some(dir) = ctx.campaign_dir {
                let path = dir.join(ctx.data_file);
                if path.exists() {
                    match load_ron_file::<Vec<T>>(&path) {
                        Ok(loaded_data) => {
                            let count = loaded_data.len();
                            *data = loaded_data;
                            *editor_unsaved = false;
                            *ctx.status_message = format!("Loaded {} {}", count, noun);
                        }
                        Err(e) => {
                            *ctx.status_message = format!("Failed to load {}: {}", noun, e);
                        }
                    }
                } else {
                    *ctx.status_message = format!("{} file does not exist", noun);
                }
            }
        }
        // New and Import are editor-specific; the calling match block handles them.
        ToolbarAction::New | ToolbarAction::Import | ToolbarAction::None => {}
    }
}

#[cfg(test)]
mod toolbar_action_tests {
    use super::*;
    use crate::editor_context::EditorContext;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// Minimal test entity that round-trips through RON.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestItem {
        id: u32,
        name: String,
    }

    fn make_items() -> Vec<TestItem> {
        vec![
            TestItem {
                id: 1,
                name: "alpha".into(),
            },
            TestItem {
                id: 2,
                name: "beta".into(),
            },
        ]
    }

    fn make_ctx<'a>(
        dir: Option<&'a PathBuf>,
        data_file: &'a str,
        unsaved: &'a mut bool,
        status: &'a mut String,
        merge: &'a mut bool,
    ) -> EditorContext<'a> {
        EditorContext::new(dir, data_file, unsaved, status, merge)
    }

    // ── None ────────────────────────────────────────────────────────────────

    #[test]
    fn test_toolbar_action_none_is_no_op() {
        let mut data = make_items();
        let snapshot = data.clone();
        let mut editor_unsaved = false;
        let mut unsaved = false;
        let mut status = String::from("before");
        let mut merge = false;

        let mut ctx = make_ctx(None, "items.ron", &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::None,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "items.ron",
            "items",
        );

        assert_eq!(data, snapshot, "None must not modify data");
        assert_eq!(status, "before", "None must not touch status");
        assert!(!editor_unsaved, "None must not set editor_unsaved");
    }

    // ── Save ────────────────────────────────────────────────────────────────

    #[test]
    fn test_toolbar_action_save_writes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let file = "items.ron";
        let path = dir.join(file);

        let mut data = make_items();
        let mut editor_unsaved = true;
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = false;

        let mut ctx = make_ctx(Some(&dir), file, &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::Save,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "items.ron",
            "items",
        );

        assert!(path.exists(), "Save must create the file");
        assert!(
            status.contains("Saved 2 items"),
            "status should report count: {status}"
        );
        // editor_unsaved is unchanged by Save (was true, stays true)
        assert!(editor_unsaved);
    }

    #[test]
    fn test_toolbar_action_save_no_campaign_dir_is_no_op() {
        let mut data = make_items();
        let snapshot = data.clone();
        let mut editor_unsaved = false;
        let mut unsaved = false;
        let mut status = String::from("initial");
        let mut merge = false;

        let mut ctx = make_ctx(None, "items.ron", &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::Save,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "items.ron",
            "items",
        );

        assert_eq!(data, snapshot, "no campaign dir — data must be unchanged");
        assert_eq!(
            status, "initial",
            "no campaign dir — status must be unchanged"
        );
    }

    // ── Reload ──────────────────────────────────────────────────────────────

    #[test]
    fn test_toolbar_action_reload_replaces_data() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let file = "items.ron";

        // Write a file with different data to reload from.
        let on_disk = vec![TestItem {
            id: 99,
            name: "disk".into(),
        }];
        let contents = ron::ser::to_string_pretty(&on_disk, Default::default()).unwrap();
        std::fs::write(dir.join(file), contents).unwrap();

        let mut data = make_items(); // different from on_disk
        let mut editor_unsaved = true;
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = false;

        let mut ctx = make_ctx(Some(&dir), file, &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::Reload,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "items.ron",
            "items",
        );

        assert_eq!(data, on_disk, "Reload must replace data with file contents");
        assert!(!editor_unsaved, "Reload must clear editor_unsaved flag");
        assert!(
            status.contains("Loaded 1 items"),
            "status should report count: {status}"
        );
    }

    #[test]
    fn test_toolbar_action_reload_missing_file_sets_status() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let file = "missing.ron";

        let mut data = make_items();
        let snapshot = data.clone();
        let mut editor_unsaved = true;
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = false;

        let mut ctx = make_ctx(Some(&dir), file, &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::Reload,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "missing.ron",
            "items",
        );

        assert_eq!(data, snapshot, "missing file — data must be unchanged");
        assert!(
            status.contains("does not exist"),
            "status should indicate missing file: {status}"
        );
        // editor_unsaved was true and stays true since no reload happened
        assert!(editor_unsaved);
    }

    #[test]
    fn test_toolbar_action_reload_no_campaign_dir_is_no_op() {
        let mut data = make_items();
        let snapshot = data.clone();
        let mut editor_unsaved = true;
        let mut unsaved = false;
        let mut status = String::from("before");
        let mut merge = false;

        let mut ctx = make_ctx(None, "items.ron", &mut unsaved, &mut status, &mut merge);
        handle_toolbar_action(
            ToolbarAction::Reload,
            &mut data,
            |i: &TestItem| i.id,
            &mut editor_unsaved,
            &mut ctx,
            "items.ron",
            "items",
        );

        assert_eq!(data, snapshot);
        assert_eq!(status, "before");
        assert!(editor_unsaved, "editor_unsaved unchanged when dir is None");
    }
}
