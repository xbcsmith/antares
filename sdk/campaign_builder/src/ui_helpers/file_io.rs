// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! CSV helpers, import/export dialog, and file load/save/reload functions.
//!
//! Contains [`CsvParseError`], [`parse_id_csv_to_vec`], [`format_vec_to_csv`],
//! [`ImportExportResult`], [`ImportExportDialogState`], [`ImportExportDialog`],
//! [`load_ron_file`], [`save_ron_file`], [`handle_file_load`],
//! [`handle_file_save`], and [`handle_reload`].

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
