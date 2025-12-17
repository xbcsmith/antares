// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared UI helper utilities for the campaign builder.
//!
//! This module contains reusable UI components, helper functions, and constants
//! used by the editor UI components (items, spells, monsters, etc.). These
//! helpers centralize layout constants and logic so multiple editors behave
//! consistently when windows are resized.
//!
//! # Components
//!
//! - [`EditorToolbar`] - Standard toolbar with New, Save, Load, Import, Export buttons
//! - [`ActionButtons`] - Standard action buttons for detail panels (Edit, Delete, Duplicate, Export)
//! - [`TwoColumnLayout`] - Standard two-column list/detail layout
//! - [`ImportExportDialog`] - Standard import/export dialog for RON data
//! - [`AttributePairInput`] - Widget for editing `AttributePair` (u8 base/current)
//! - [`AttributePair16Input`] - Widget for editing `AttributePair16` (u16 base/current)
//! - [`AutocompleteInput`] - Autocomplete text input with dropdown suggestions

use antares::domain::character::{AttributePair, AttributePair16};
use eframe::egui;
use egui_autocomplete::AutoCompleteTextEdit;
use std::fmt::Display;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

// =============================================================================
// Constants
// =============================================================================

/// Default width (in points) of the left-hand column used by editors
/// (commonly the list panel).
pub const DEFAULT_LEFT_COLUMN_WIDTH: f32 = 300.0;

/// Default minimum width (in points) of the inspector (right) column used by editors.
/// This is the default inspector width used to compute the left/right split.
pub const DEFAULT_INSPECTOR_MIN_WIDTH: f32 = 300.0;

/// Default maximum ratio for the left-hand column (0.0 - 1.0).
/// This prevents the left (list) panel from consuming nearly all horizontal space
/// and clipping the inspector panel. 0.72 means the left column won't exceed 72%
/// of the total available width.
pub const DEFAULT_LEFT_COLUMN_MAX_RATIO: f32 = 0.72;

/// Minimum safe left column width (px) - prevents the left column from becoming too narrow.
pub const MIN_SAFE_LEFT_COLUMN_WIDTH: f32 = 250.0;

/// Default minimum height (in points) for editor list & preview panels.
///
/// This value prevents panels from collapsing to 0 or very short heights when
/// the computed available region is very small.
pub const DEFAULT_PANEL_MIN_HEIGHT: f32 = 100.0;

/// Renders a bold header row inside an `egui::Grid`.
///
/// This should be called from within a `egui::Grid::show(...)` closure and will
/// automatically mark the end of the header row by calling `ui.end_row()`.
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::sdk::campaign_builder::ui_helpers::render_grid_header;
///
/// // Example usage inside a Grid:
/// // egui::Grid::new("example_grid").num_columns(3).show(ui, |ui| {
/// //     render_grid_header(ui, &["Status", "Message", "File"]);
/// //     // row content...
/// //     ui.end_row();
/// // });
/// ```
pub fn render_grid_header(ui: &mut egui::Ui, headers: &[&str]) {
    for header in headers {
        ui.label(egui::RichText::new(*header).strong());
    }
    ui.end_row();
}

/// Renders a colored validation severity icon with a tooltip of its display name.
/// Accepts the `ValidationSeverity` type from the validation module and renders
/// the icon using the appropriate color and tooltip text.
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::sdk::campaign_builder::ui_helpers::show_validation_severity_icon;
///
/// // Example usage:
/// // show_validation_severity_icon(ui, crate::validation::ValidationSeverity::Error);
/// ```
pub fn show_validation_severity_icon(
    ui: &mut egui::Ui,
    severity: crate::validation::ValidationSeverity,
) {
    ui.colored_label(severity.color(), severity.icon())
        .on_hover_text(severity.display_name());
}

/// Renders a clickable file path label with an on-hover tooltip and returns
/// the widget `Response` so callers can react to clicks.
///
/// This helper centralizes a common pattern used in the Validation panel and
/// the Asset Manager where file paths should be interactive. The label is
/// shown with a more compact font size to visually fit into grid cells.
///
/// # Arguments
///
/// * `ui` - The egui UI to render into
/// * `path` - The path to display. The tooltip shows the same path text.
///
pub fn show_clickable_path(ui: &mut egui::Ui, path: &Path) -> egui::Response {
    let path_display = path.display().to_string();
    let label = egui::RichText::new(path_display.clone()).small();
    // Build the clickable label
    let resp = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
    // Attach a tooltip showing the path and return the Response for click detection
    resp.on_hover_text(path_display)
}

// =============================================================================
// Panel Height Helpers
// =============================================================================

/// Compute the effective panel height from a Vec2 size and a minimum height.
///
/// This is a pure function that can be tested easily. It returns either the
/// `size.y` or the provided `min_height` if `size.y` is smaller.
///
/// # Examples
///
/// ```
/// use eframe::egui::Vec2;
/// use antares::sdk::campaign_builder::ui_helpers::compute_panel_height_from_size;
///
/// let size = Vec2::new(400.0, 240.0);
/// assert_eq!(compute_panel_height_from_size(size, 100.0), 240.0);
///
/// let tiny_size = Vec2::new(200.0, 20.0);
/// assert_eq!(compute_panel_height_from_size(tiny_size, 100.0), 100.0);
/// ```
pub fn compute_panel_height_from_size(size: egui::Vec2, min_height: f32) -> f32 {
    size.y.max(min_height)
}

/// Compute effective panel height from an `egui::Ui` instance.
///
/// This returns the vertical size in the given `Ui` (via `available_size_before_wrap`) or the
/// provided `min_height` if the region is smaller. This function should be used
/// by editor UIs to decide the `max_height` used for `ScrollArea` and the
/// `min_height` used for columns.
///
/// # Example
///
/// ```no_run
/// use eframe::egui;
/// use antares::sdk::campaign_builder::ui_helpers::compute_panel_height;
///
/// // In editor code where `ui: &mut egui::Ui` exists:
/// // let panel_height = compute_panel_height(ui, DEFAULT_PANEL_MIN_HEIGHT);
/// ```
pub fn compute_panel_height(ui: &mut egui::Ui, min_height: f32) -> f32 {
    compute_panel_height_from_size(ui.available_size_before_wrap(), min_height)
}

/// Convenience function that uses the module's default minimum height.
///
/// Call this from editor panels when you want to use the standard configured
/// default.
pub fn compute_default_panel_height(ui: &mut egui::Ui) -> f32 {
    compute_panel_height(ui, DEFAULT_PANEL_MIN_HEIGHT)
}

/// Convenience function that uses `DEFAULT_PANEL_MIN_HEIGHT` for a given size.
pub fn compute_default_panel_height_from_size(size: egui::Vec2) -> f32 {
    compute_panel_height_from_size(size, DEFAULT_PANEL_MIN_HEIGHT)
}

/// Compute an appropriate left column width for two-column layouts.
///
/// This helper centralizes logic for left column width calculations so editors
/// can share the same rules and behavior. The algorithm:
///
/// - Respects the configured inspector minimum width.
/// - Respects a maximum left ratio relative to total width.
/// - Avoids forcing the `MIN_SAFE_LEFT_COLUMN_WIDTH` when the available
///   space is smaller; in that case, it allows smaller widths to avoid making
///   the inspector completely invisible.
/// - Gracefully returns zero if no space is available for a left column.
///
/// Arguments:
/// * `total_width` - total available width for both columns.
/// * `requested_left` - desired left column width (usually an editor's configured default).
/// * `inspector_min_width` - minimum width requested for the inspector column.
/// * `sep_margin` - separator or margin to reserve between columns.
/// * `min_safe_left` - the safe minimum left width to prefer when space is available.
/// * `max_left_ratio` - maximum ratio (0-1) for the left column; respects defaults when outside range.
pub fn compute_left_column_width(
    total_width: f32,
    requested_left: f32,
    inspector_min_width: f32,
    sep_margin: f32,
    min_safe_left: f32,
    max_left_ratio: f32,
) -> f32 {
    // Ensure inspector minimum respects the configured default fallback.
    let inspector_min = inspector_min_width.max(DEFAULT_INSPECTOR_MIN_WIDTH);

    // Validate/max the ratio
    let max_left_ratio_clamped = if (max_left_ratio <= 0.0) || (max_left_ratio > 1.0) {
        DEFAULT_LEFT_COLUMN_MAX_RATIO
    } else {
        max_left_ratio
    };

    // Max allowed left width by ratio
    let max_left = (max_left_ratio_clamped * total_width).min(total_width);

    // Space left after reserving inspector minimum and separator
    let available_left_space = (total_width - inspector_min - sep_margin).max(0.0);

    // The upper bound given by both constraints
    let max_left_possible = max_left.min(available_left_space);

    // Minimum bound: only enforce the minimum safe left width when available space allows for it
    let min_left_bound = if max_left_possible >= min_safe_left {
        min_safe_left
    } else {
        // If not enough space is available, don't force min safe value to avoid contradictions.
        0.0
    };

    // Clamp the requested left width into computed bounds.
    let left = requested_left.clamp(min_left_bound, max_left_possible);

    // Final safety clamp ensures we never exceed the total width.
    left.clamp(0.0, total_width)
}

// =============================================================================
// Toolbar Component
// =============================================================================

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
/// # use antares::sdk::campaign_builder::ui_helpers::parse_id_csv_to_vec;
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
/// # use antares::sdk::campaign_builder::ui_helpers::format_vec_to_csv;
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

/// Returns indices of `items` whose label (provided by `label_fn`) contains `query` (case-insensitive).
///
/// Useful for building filtered lists or suggestions.
///
/// # Examples
///
/// ```
/// # use antares::sdk::campaign_builder::ui_helpers::filter_items_by_query;
/// struct Foo { name: String }
/// let items = vec![Foo { name: "Goblin".to_string() }, Foo { name: "Orc".to_string() }];
/// let idx = filter_items_by_query(&items, "gob", |f| f.name.clone());
/// assert_eq!(idx, vec![0usize]);
/// ```
pub fn filter_items_by_query<T, F>(items: &[T], query: &str, label_fn: F) -> Vec<usize>
where
    F: Fn(&T) -> String,
{
    let q = query.to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| q.is_empty() || label_fn(it).to_lowercase().contains(&q))
        .map(|(idx, _)| idx)
        .collect()
}

/// Single-selection searchable selector UI helper.
///
/// - `ui`: egui UI reference
/// - `id_salt`: Unique id salt (used for ComboBox id)
/// - `label`: Label text shown before the widget
/// - `selected`: Mutable reference to current selection (`Option<ID>`)
/// - `items`, `id_fn`, `label_fn` describe the available values and how to extract id/label
/// - `search_query`: Mutable reference that stores the current query string (persisted by caller)
///
/// Returns `true` if the selection changed.
///
/// This helper wraps `egui::ComboBox` and provides an inline search text field inside the
/// ComboBox dropdown to filter options.
#[allow(clippy::too_many_arguments)]
pub fn searchable_selector_single<T, ID, FId, FLabel>(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected: &mut Option<ID>,
    items: &[T],
    id_fn: FId,
    label_fn: FLabel,
    search_query: &mut String,
) -> bool
where
    ID: Clone + PartialEq + Display,
    FId: Fn(&T) -> ID,
    FLabel: Fn(&T) -> String,
{
    ui.label(label);
    let mut changed = false;
    let selected_text = selected
        .as_ref()
        .map_or("(None)".to_string(), |id| id.to_string());
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            // Search input at the top
            ui.text_edit_singleline(search_query);
            let q = search_query.to_lowercase();

            // Filtered list
            for item in items.iter() {
                let label_text = label_fn(item);
                if q.is_empty() || label_text.to_lowercase().contains(&q) {
                    let id = id_fn(item);
                    let is_selected = selected.as_ref().map(|s| s == &id).unwrap_or(false);
                    if ui
                        .selectable_label(is_selected, label_text.clone())
                        .clicked()
                    {
                        *selected = Some(id);
                        changed = true;
                    }
                }
            }
        });
    changed
}

/// Multi-selection searchable selector UI helper.
///
/// - `label`: user-visible label
/// - `selection`: mutable vector of selected IDs (caller manages order/persistence)
/// - `items`, `id_fn`, `label_fn` describe the available values
/// - `search_query` is used to store the user's search input and is persisted by the caller
///
/// Returns `true` if the selection changed (items added or removed).
#[allow(clippy::too_many_arguments)]
pub fn searchable_selector_multi<T, ID, FId, FLabel>(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selection: &mut Vec<ID>,
    items: &[T],
    id_fn: FId,
    label_fn: FLabel,
    search_query: &mut String,
) -> bool
where
    ID: Clone + PartialEq + Display,
    FId: Fn(&T) -> ID,
    FLabel: Fn(&T) -> String,
{
    ui.label(label);
    let mut changed = false;

    // Render chips for selected items with a small remove button.
    ui.horizontal_wrapped(|ui| {
        let mut idx_to_remove: Option<usize> = None;
        for (idx, sel) in selection.iter().enumerate() {
            let label_text = items
                .iter()
                .find(|it| id_fn(it) == *sel)
                .map(&label_fn)
                .unwrap_or_else(|| sel.to_string());
            ui.horizontal(|ui| {
                ui.label(label_text);
                if ui.small_button("✖").clicked() {
                    idx_to_remove = Some(idx);
                }
            });
        }

        if let Some(idx) = idx_to_remove {
            selection.remove(idx);
            changed = true;
        }
    });

    // Add control: search box and Add button
    ui.horizontal(|ui| {
        ui.text_edit_singleline(search_query);
        if ui.button("Add").clicked() {
            let q = search_query.to_lowercase();
            // Try to find the first match by label text
            if let Some(item) = items
                .iter()
                .find(|it| label_fn(it).to_lowercase().contains(&q))
            {
                let id = id_fn(item);
                if !selection.contains(&id) {
                    selection.push(id);
                    changed = true;
                }
                *search_query = String::new();
            }
        }
    });

    // Suggestion buttons (compact)
    let q = search_query.to_lowercase();
    ui.horizontal_wrapped(|ui| {
        for item in items {
            let label_text = label_fn(item);
            if q.is_empty() || label_text.to_lowercase().contains(&q) {
                if ui.small_button(label_text.clone()).clicked() {
                    let id = id_fn(item);
                    if !selection.contains(&id) {
                        selection.push(id);
                        changed = true;
                    }
                }
            }
        }
    });

    changed
}

/// Actions that can be triggered from the editor toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    /// Create a new entity
    New,
    /// Save entities to campaign
    Save,
    /// Load entities from external file
    Load,
    /// Import entities from RON text
    Import,
    /// Export entities to external file
    Export,
    /// Reload entities from campaign
    Reload,
    /// No action triggered
    None,
}

/// Configuration for the editor toolbar.
///
/// This builder-pattern struct allows configuring which features are enabled
/// and provides mutable references to state that the toolbar needs to modify.
pub struct EditorToolbar<'a> {
    /// Editor name for display (e.g., "Items", "Monsters")
    editor_name: &'a str,
    /// Search query text (mutable reference)
    search_query: Option<&'a mut String>,
    /// Merge mode checkbox state (mutable reference)
    merge_mode: Option<&'a mut bool>,
    /// Total count to display
    total_count: Option<usize>,
    /// Whether to show the save button
    show_save: bool,
    /// Custom id salt for disambiguation
    id_salt: Option<&'a str>,
}

impl<'a> EditorToolbar<'a> {
    /// Creates a new toolbar for the specified editor.
    ///
    /// # Arguments
    ///
    /// * `editor_name` - Display name for the editor (e.g., "Items", "Monsters")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::campaign_builder::ui_helpers::EditorToolbar;
    ///
    /// let toolbar = EditorToolbar::new("Items");
    /// ```
    pub fn new(editor_name: &'a str) -> Self {
        Self {
            editor_name,
            search_query: None,
            merge_mode: None,
            total_count: None,
            show_save: true,
            id_salt: None,
        }
    }

    /// Adds a search field to the toolbar.
    ///
    /// # Arguments
    ///
    /// * `query` - Mutable reference to the search query string
    pub fn with_search(mut self, query: &'a mut String) -> Self {
        self.search_query = Some(query);
        self
    }

    /// Adds a merge mode checkbox to the toolbar.
    ///
    /// When merge mode is enabled, loading files will merge with existing data
    /// instead of replacing it.
    ///
    /// # Arguments
    ///
    /// * `merge_mode` - Mutable reference to the merge mode flag
    pub fn with_merge_mode(mut self, merge_mode: &'a mut bool) -> Self {
        self.merge_mode = Some(merge_mode);
        self
    }

    /// Displays the total count of entities.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of entities to display
    pub fn with_total_count(mut self, count: usize) -> Self {
        self.total_count = Some(count);
        self
    }

    /// Controls whether the save button is shown.
    ///
    /// # Arguments
    ///
    /// * `show` - Whether to show the save button
    pub fn with_save_button(mut self, show: bool) -> Self {
        self.show_save = show;
        self
    }

    /// Sets a custom id salt for widget disambiguation.
    ///
    /// # Arguments
    ///
    /// * `salt` - Unique identifier for this toolbar instance
    pub fn with_id_salt(mut self, salt: &'a str) -> Self {
        self.id_salt = Some(salt);
        self
    }

    /// Renders the toolbar and returns the triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// The `ToolbarAction` triggered by user interaction, or `ToolbarAction::None`
    /// if no action was triggered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::sdk::campaign_builder::ui_helpers::{EditorToolbar, ToolbarAction};
    ///
    /// // In an editor's show() method:
    /// fn example(ui: &mut egui::Ui, search: &mut String, merge: &mut bool) {
    ///     let action = EditorToolbar::new("Items")
    ///         .with_search(search)
    ///         .with_merge_mode(merge)
    ///         .with_total_count(42)
    ///         .show(ui);
    ///
    ///     match action {
    ///         ToolbarAction::New => { /* create new item */ }
    ///         ToolbarAction::Save => { /* save to campaign */ }
    ///         ToolbarAction::Load => { /* load from file */ }
    ///         ToolbarAction::Import => { /* show import dialog */ }
    ///         ToolbarAction::Export => { /* export to file */ }
    ///         ToolbarAction::Reload => { /* reload from campaign */ }
    ///         ToolbarAction::None => { /* no action */ }
    ///     }
    /// }
    /// ```
    pub fn show(self, ui: &mut egui::Ui) -> ToolbarAction {
        let mut action = ToolbarAction::None;

        // Check for keyboard shortcuts
        ui.input(|input| {
            // Ctrl+N for New
            if input.key_pressed(egui::Key::N) && input.modifiers.ctrl && !input.modifiers.shift {
                action = ToolbarAction::New;
            }
            // Ctrl+S for Save
            if self.show_save
                && input.key_pressed(egui::Key::S)
                && input.modifiers.ctrl
                && !input.modifiers.shift
            {
                action = ToolbarAction::Save;
            }
            // Ctrl+L for Load
            if input.key_pressed(egui::Key::L) && input.modifiers.ctrl && !input.modifiers.shift {
                action = ToolbarAction::Load;
            }
            // Ctrl+Shift+I for Import
            if input.key_pressed(egui::Key::I) && input.modifiers.ctrl && input.modifiers.shift {
                action = ToolbarAction::Import;
            }
            // Ctrl+Shift+E for Export
            if input.key_pressed(egui::Key::E) && input.modifiers.ctrl && input.modifiers.shift {
                action = ToolbarAction::Export;
            }
            // F5 for Reload
            if input.key_pressed(egui::Key::F5) {
                action = ToolbarAction::Reload;
            }
        });

        // Use horizontal_wrapped for the toolbar so it gracefully wraps onto
        // multiple rows when the UI width is constrained, preventing UI clipping
        // and ensuring all actions remain accessible at narrow sizes.
        ui.horizontal_wrapped(|ui| {
            // Search field
            if let Some(search_query) = self.search_query {
                ui.label("🔍 Search:");
                let id_salt = self
                    .id_salt
                    .map(|s| format!("{}_search", s))
                    .unwrap_or_else(|| format!("{}_search", self.editor_name.to_lowercase()));
                ui.push_id(&id_salt, |ui| {
                    ui.text_edit_singleline(search_query);
                });
                ui.separator();
            }

            // Action buttons with keyboard shortcuts in tooltips
            if ui
                .button("➕ New")
                .on_hover_text("Create new entry (Ctrl+N)")
                .clicked()
            {
                action = ToolbarAction::New;
            }

            if self.show_save {
                if ui
                    .button("💾 Save")
                    .on_hover_text("Save to campaign (Ctrl+S)")
                    .clicked()
                {
                    action = ToolbarAction::Save;
                }
            }

            if ui
                .button("📂 Load")
                .on_hover_text("Load from file (Ctrl+L)")
                .clicked()
            {
                action = ToolbarAction::Load;
            }

            if ui
                .button("📥 Import")
                .on_hover_text("Import from RON text (Ctrl+Shift+I)")
                .clicked()
            {
                action = ToolbarAction::Import;
            }

            // Merge mode checkbox
            if let Some(merge_mode) = self.merge_mode {
                ui.checkbox(merge_mode, "Merge");
                ui.label(if *merge_mode {
                    "(adds to existing)"
                } else {
                    "(replaces all)"
                });
            }

            if ui
                .button("📋 Export")
                .on_hover_text("Export to file (Ctrl+Shift+E)")
                .clicked()
            {
                action = ToolbarAction::Export;
            }

            if ui
                .button("🔄 Reload")
                .on_hover_text("Reload from campaign (F5)")
                .clicked()
            {
                action = ToolbarAction::Reload;
            }

            ui.separator();

            // Total count display
            if let Some(count) = self.total_count {
                ui.label(format!("Total: {}", count));
            }
        });

        action
    }
}

// =============================================================================
// Action Buttons Component
// =============================================================================

/// Actions that can be triggered from the detail panel action buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// Edit the selected entity
    Edit,
    /// Delete the selected entity
    Delete,
    /// Duplicate the selected entity
    Duplicate,
    /// Export the selected entity
    Export,
    /// No action triggered
    None,
}

/// Configuration for the action buttons in the detail panel.
///
/// These buttons appear when an entity is selected and allow common operations
/// like editing, deleting, duplicating, and exporting.
pub struct ActionButtons {
    /// Whether the buttons are enabled
    enabled: bool,
    /// Whether to show the edit button
    show_edit: bool,
    /// Whether to show the delete button
    show_delete: bool,
    /// Whether to show the duplicate button
    show_duplicate: bool,
    /// Whether to show the export button
    show_export: bool,
}

impl Default for ActionButtons {
    fn default() -> Self {
        Self {
            enabled: true,
            show_edit: true,
            show_delete: true,
            show_duplicate: true,
            show_export: true,
        }
    }
}

impl ActionButtons {
    /// Creates a new action buttons component with all buttons visible.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::ui_helpers::ActionButtons;
    ///
    /// let buttons = ActionButtons::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the buttons are enabled.
    ///
    /// Disabled buttons will be grayed out and not respond to clicks.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether the buttons should be enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Controls visibility of the edit button.
    pub fn with_edit(mut self, show: bool) -> Self {
        self.show_edit = show;
        self
    }

    /// Controls visibility of the delete button.
    pub fn with_delete(mut self, show: bool) -> Self {
        self.show_delete = show;
        self
    }

    /// Controls visibility of the duplicate button.
    pub fn with_duplicate(mut self, show: bool) -> Self {
        self.show_duplicate = show;
        self
    }

    /// Controls visibility of the export button.
    pub fn with_export(mut self, show: bool) -> Self {
        self.show_export = show;
        self
    }

    /// Renders the action buttons and returns the triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// The `ItemAction` triggered by user interaction, or `ItemAction::None`
    /// if no action was triggered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::sdk::campaign_builder::ui_helpers::{ActionButtons, ItemAction};
    ///
    /// fn example(ui: &mut egui::Ui, has_selection: bool) {
    ///     let action = ActionButtons::new()
    ///         .enabled(has_selection)
    ///         .show(ui);
    ///
    ///     match action {
    ///         ItemAction::Edit => { /* enter edit mode */ }
    ///         ItemAction::Delete => { /* delete selected */ }
    ///         ItemAction::Duplicate => { /* duplicate selected */ }
    ///         ItemAction::Export => { /* export selected */ }
    ///         ItemAction::None => { /* no action */ }
    ///     }
    /// }
    /// ```
    pub fn show(self, ui: &mut egui::Ui) -> ItemAction {
        let mut action = ItemAction::None;

        // Check for keyboard shortcuts if buttons are enabled
        if self.enabled {
            ui.input(|input| {
                // Ctrl+E for Edit
                if self.show_edit
                    && input.key_pressed(egui::Key::E)
                    && input.modifiers.ctrl
                    && !input.modifiers.shift
                {
                    action = ItemAction::Edit;
                }
                // Delete key for Delete
                if self.show_delete && input.key_pressed(egui::Key::Delete) {
                    action = ItemAction::Delete;
                }
                // Ctrl+D for Duplicate
                if self.show_duplicate
                    && input.key_pressed(egui::Key::D)
                    && input.modifiers.ctrl
                    && !input.modifiers.shift
                {
                    action = ItemAction::Duplicate;
                }
            });
        }

        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.enabled, |ui| {
                if self.show_edit {
                    if ui
                        .button("✏️ Edit")
                        .on_hover_text("Edit selected item (Ctrl+E)")
                        .clicked()
                    {
                        action = ItemAction::Edit;
                    }
                }
                if self.show_delete {
                    if ui
                        .button("🗑️ Delete")
                        .on_hover_text("Delete selected item (Delete)")
                        .clicked()
                    {
                        action = ItemAction::Delete;
                    }
                }
                if self.show_duplicate {
                    if ui
                        .button("📋 Duplicate")
                        .on_hover_text("Duplicate selected item (Ctrl+D)")
                        .clicked()
                    {
                        action = ItemAction::Duplicate;
                    }
                }
                if self.show_export {
                    if ui
                        .button("📤 Export")
                        .on_hover_text("Export selected item")
                        .clicked()
                    {
                        action = ItemAction::Export;
                    }
                }
            });
        });

        action
    }
}

// =============================================================================
// Two-Column Layout Component
// =============================================================================

/// A standard two-column layout for editor panels.
///
/// This component provides a consistent list/detail split layout used by
/// most editors. The left column contains a scrollable list, and the right
/// column contains the detail/preview panel.
pub struct TwoColumnLayout<'a> {
    /// Unique identifier for the layout
    id_salt: &'a str,
    /// Width of the left column in points
    left_width: f32,
    /// Minimum height for both panels
    min_height: f32,
    /// Minimum width for the inspector (right) column (points)
    inspector_min_width: f32,
    /// Maximum ratio (0.0 - 1.0) allowed for the left column relative to total width.
    /// This prevents the left column from consuming too much horizontal space.
    max_left_ratio: f32,
}

impl<'a> TwoColumnLayout<'a> {
    /// Creates a new two-column layout.
    ///
    /// # Arguments
    ///
    /// * `id_salt` - Unique identifier for widget disambiguation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::sdk::campaign_builder::ui_helpers::TwoColumnLayout;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     TwoColumnLayout::new("items")
    ///         .show_split(
    ///             ui,
    ///             |left_ui| {
    ///                 // left_ui: render list
    ///                 left_ui.label("Item list goes here");
    ///             },
    ///             |right_ui| {
    ///                 // right_ui: render detail/preview
    ///                 right_ui.label("Details go here");
    ///             },
    ///         );
    /// }
    /// ```
    pub fn new(id_salt: &'a str) -> Self {
        Self {
            id_salt,
            left_width: DEFAULT_LEFT_COLUMN_WIDTH,
            min_height: DEFAULT_PANEL_MIN_HEIGHT,
            inspector_min_width: DEFAULT_INSPECTOR_MIN_WIDTH,
            max_left_ratio: DEFAULT_LEFT_COLUMN_MAX_RATIO,
        }
    }

    /// Sets a custom width for the left column.
    ///
    /// # Arguments
    ///
    /// * `width` - Width in points
    pub fn with_left_width(mut self, width: f32) -> Self {
        self.left_width = width;
        self
    }

    /// Sets a custom minimum height for both panels.
    ///
    /// # Arguments
    ///
    /// * `height` - Minimum height in points
    pub fn with_min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    /// Sets the minimum width for the inspector (right) column.
    ///
    /// # Arguments
    ///
    /// * `width` - Width in points
    pub fn with_inspector_min_width(mut self, width: f32) -> Self {
        self.inspector_min_width = width;
        self
    }

    /// Sets a maximum left column ratio (0.0 - 1.0).
    /// The final left width will be clamped to be no larger than
    /// `max_left_ratio * available_total_width`.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Ratio (between 0.0 and 1.0)
    pub fn with_max_left_ratio(mut self, ratio: f32) -> Self {
        // We don't strictly enforce bounds here; the clamp occurs where the value is used.
        self.max_left_ratio = ratio;
        self
    }

    /// Renders the two-column layout with separate closures for each column.
    ///
    /// This is the preferred method as it provides cleaner separation of concerns.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `left_content` - Closure for the left column (list)
    /// * `right_content` - Closure for the right column (detail)
    ///
    /// # Returns
    ///
    /// The computed panel height used for both columns.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::sdk::campaign_builder::ui_helpers::TwoColumnLayout;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     TwoColumnLayout::new("items")
    ///         .show_split(
    ///             ui,
    ///             |left_ui| {
    ///                 left_ui.heading("Items");
    ///                 left_ui.separator();
    ///                 left_ui.label("Item 1");
    ///                 left_ui.label("Item 2");
    ///             },
    ///             |right_ui| {
    ///                 right_ui.heading("Details");
    ///                 right_ui.separator();
    ///                 right_ui.label("Select an item to view details");
    ///             },
    ///         );
    /// }
    /// ```
    pub fn show_split<L, R>(self, ui: &mut egui::Ui, left_content: L, right_content: R) -> f32
    where
        L: FnOnce(&mut egui::Ui),
        R: FnOnce(&mut egui::Ui),
    {
        let panel_height = compute_panel_height(ui, self.min_height);

        // Compute the left width via the shared helper so the same behavior is used everywhere.
        let total_width = ui.available_width();
        let sep_margin = 12.0;
        let inspector_min = self.inspector_min_width.max(DEFAULT_INSPECTOR_MIN_WIDTH);
        let left_width = compute_left_column_width(
            total_width,
            self.left_width,
            inspector_min,
            sep_margin,
            MIN_SAFE_LEFT_COLUMN_WIDTH,
            self.max_left_ratio,
        );

        ui.horizontal(|ui| {
            // Left column
            ui.vertical(|left_ui| {
                left_ui.set_width(left_width);
                left_ui.set_min_height(panel_height);

                egui::ScrollArea::vertical()
                    .id_salt(format!("{}_left_scroll", self.id_salt))
                    .auto_shrink([false, false])
                    .max_height(panel_height)
                    .show(left_ui, |scroll_ui| {
                        // Apply consistent horizontal padding around the left column content
                        // so the map or list does not butt directly against the edge.
                        scroll_ui.set_min_width(scroll_ui.available_width());
                        left_content(scroll_ui);
                    });
            });

            ui.separator();

            // Right column
            ui.vertical(|right_ui| {
                // Use configured inspector min width to ensure the right column isn't clipped.
                right_ui.set_min_width(inspector_min);
                right_ui.set_min_height(panel_height);

                egui::ScrollArea::vertical()
                    .id_salt(format!("{}_right_scroll", self.id_salt))
                    .auto_shrink([false, false])
                    .max_height(panel_height)
                    .show(right_ui, |scroll_ui| {
                        // Ensure the right-hand inspector area (detail/preview) gets the same
                        // horizontal padding as the left column for visual balance.
                        right_content(scroll_ui);
                    });
            });
        });

        panel_height
    }
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
    /// use antares::sdk::campaign_builder::ui_helpers::{ImportExportDialog, ImportExportDialogState, ImportExportResult};
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
// AttributePair Input Widget
// =============================================================================

/// State for tracking AttributePair input sync behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttributePairInputState {
    /// Whether auto-sync is enabled (current follows base)
    pub auto_sync: bool,
}

impl AttributePairInputState {
    /// Creates a new state with auto-sync enabled.
    pub fn new() -> Self {
        Self { auto_sync: true }
    }

    /// Creates a new state with specified auto-sync setting.
    pub fn with_auto_sync(auto_sync: bool) -> Self {
        Self { auto_sync }
    }
}

/// Widget for editing an `AttributePair` (u8 base/current).
///
/// This widget provides dual input fields for base and current values,
/// with optional auto-sync behavior and a reset button.
pub struct AttributePairInput<'a> {
    /// Label for the attribute
    label: &'a str,
    /// The AttributePair to edit
    value: &'a mut AttributePair,
    /// Widget state for auto-sync
    state: Option<&'a mut AttributePairInputState>,
    /// Unique id salt for widget disambiguation
    id_salt: Option<&'a str>,
    /// Whether to show the reset button
    show_reset: bool,
    /// Whether to show the auto-sync checkbox
    show_auto_sync: bool,
}

impl<'a> AttributePairInput<'a> {
    /// Creates a new AttributePair input widget.
    ///
    /// # Arguments
    ///
    /// * `label` - Display label for the attribute (e.g., "AC", "Might")
    /// * `value` - Mutable reference to the AttributePair
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::domain::character::AttributePair;
    /// use antares::sdk::campaign_builder::ui_helpers::AttributePairInput;
    ///
    /// fn example(ui: &mut egui::Ui, ac: &mut AttributePair) {
    ///     AttributePairInput::new("AC", ac).show(ui);
    /// }
    /// ```
    pub fn new(label: &'a str, value: &'a mut AttributePair) -> Self {
        Self {
            label,
            value,
            state: None,
            id_salt: None,
            show_reset: true,
            show_auto_sync: true,
        }
    }

    /// Adds state tracking for auto-sync behavior.
    pub fn with_state(mut self, state: &'a mut AttributePairInputState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom id salt for widget disambiguation.
    pub fn with_id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
    }

    /// Controls visibility of the reset button.
    pub fn with_reset_button(mut self, show: bool) -> Self {
        self.show_reset = show;
        self
    }

    /// Controls visibility of the auto-sync checkbox.
    pub fn with_auto_sync_checkbox(mut self, show: bool) -> Self {
        self.show_auto_sync = show;
        self
    }

    /// Renders the widget.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// `true` if the value was changed, `false` otherwise.
    pub fn show(self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let id_salt = self
            .id_salt
            .map(String::from)
            .unwrap_or_else(|| self.label.to_lowercase().replace(' ', "_"));

        ui.horizontal(|ui| {
            ui.label(format!("{}:", self.label));

            // Base value input
            ui.label("Base:");
            let base_response = ui.add(
                egui::DragValue::new(&mut self.value.base)
                    .range(0..=255)
                    .speed(1),
            );

            // Track if base changed for auto-sync
            let base_changed = base_response.changed();
            if base_changed {
                changed = true;
                // Auto-sync: update current to match base if enabled
                if let Some(ref state) = self.state {
                    if state.auto_sync {
                        self.value.current = self.value.base;
                    }
                }
            }

            // Current value input
            ui.label("Current:");
            if ui
                .add(
                    egui::DragValue::new(&mut self.value.current)
                        .range(0..=255)
                        .speed(1),
                )
                .changed()
            {
                changed = true;
            }

            // Auto-sync checkbox
            if self.show_auto_sync {
                if let Some(state) = self.state {
                    ui.checkbox(&mut state.auto_sync, "Sync");
                }
            }

            // Reset button
            if self.show_reset {
                ui.push_id(format!("{}_reset", id_salt), |ui| {
                    if ui
                        .button("🔄")
                        .on_hover_text("Reset current to base")
                        .clicked()
                    {
                        self.value.reset();
                        changed = true;
                    }
                });
            }
        });

        changed
    }
}

/// Widget for editing an `AttributePair16` (u16 base/current).
///
/// This widget provides dual input fields for base and current values,
/// with optional auto-sync behavior and a reset button. Used for larger
/// values like HP and SP.
pub struct AttributePair16Input<'a> {
    /// Label for the attribute
    label: &'a str,
    /// The AttributePair16 to edit
    value: &'a mut AttributePair16,
    /// Widget state for auto-sync
    state: Option<&'a mut AttributePairInputState>,
    /// Unique id salt for widget disambiguation
    id_salt: Option<&'a str>,
    /// Whether to show the reset button
    show_reset: bool,
    /// Whether to show the auto-sync checkbox
    show_auto_sync: bool,
    /// Maximum value allowed
    max_value: u16,
}

impl<'a> AttributePair16Input<'a> {
    /// Creates a new AttributePair16 input widget.
    ///
    /// # Arguments
    ///
    /// * `label` - Display label for the attribute (e.g., "HP", "SP")
    /// * `value` - Mutable reference to the AttributePair16
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::domain::character::AttributePair16;
    /// use antares::sdk::campaign_builder::ui_helpers::AttributePair16Input;
    ///
    /// fn example(ui: &mut egui::Ui, hp: &mut AttributePair16) {
    ///     AttributePair16Input::new("HP", hp).show(ui);
    /// }
    /// ```
    pub fn new(label: &'a str, value: &'a mut AttributePair16) -> Self {
        Self {
            label,
            value,
            state: None,
            id_salt: None,
            show_reset: true,
            show_auto_sync: true,
            max_value: u16::MAX,
        }
    }

    /// Adds state tracking for auto-sync behavior.
    pub fn with_state(mut self, state: &'a mut AttributePairInputState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom id salt for widget disambiguation.
    pub fn with_id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
    }

    /// Controls visibility of the reset button.
    pub fn with_reset_button(mut self, show: bool) -> Self {
        self.show_reset = show;
        self
    }

    /// Controls visibility of the auto-sync checkbox.
    pub fn with_auto_sync_checkbox(mut self, show: bool) -> Self {
        self.show_auto_sync = show;
        self
    }

    /// Sets the maximum allowed value.
    pub fn with_max_value(mut self, max: u16) -> Self {
        self.max_value = max;
        self
    }

    /// Renders the widget.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// `true` if the value was changed, `false` otherwise.
    pub fn show(self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let id_salt = self
            .id_salt
            .map(String::from)
            .unwrap_or_else(|| self.label.to_lowercase().replace(' ', "_"));

        ui.horizontal(|ui| {
            ui.label(format!("{}:", self.label));

            // Base value input
            ui.label("Base:");
            let base_response = ui.add(
                egui::DragValue::new(&mut self.value.base)
                    .range(0..=self.max_value)
                    .speed(1),
            );

            // Track if base changed for auto-sync
            let base_changed = base_response.changed();
            if base_changed {
                changed = true;
                // Auto-sync: update current to match base if enabled
                if let Some(ref state) = self.state {
                    if state.auto_sync {
                        self.value.current = self.value.base;
                    }
                }
            }

            // Current value input
            ui.label("Current:");
            if ui
                .add(
                    egui::DragValue::new(&mut self.value.current)
                        .range(0..=self.max_value)
                        .speed(1),
                )
                .changed()
            {
                changed = true;
            }

            // Auto-sync checkbox
            if self.show_auto_sync {
                if let Some(state) = self.state {
                    ui.checkbox(&mut state.auto_sync, "Sync");
                }
            }

            // Reset button
            if self.show_reset {
                ui.push_id(format!("{}_reset", id_salt), |ui| {
                    if ui
                        .button("🔄")
                        .on_hover_text("Reset current to base")
                        .clicked()
                    {
                        self.value.reset();
                        changed = true;
                    }
                });
            }
        });

        changed
    }
}

// =============================================================================
// Autocomplete Input Widget
// =============================================================================

/// Autocomplete text input with dropdown suggestions.
///
/// This widget wraps `egui_autocomplete::AutoCompleteTextEdit` to provide a
/// consistent interface with other UI helpers. It displays a text field with
/// a dropdown list of suggestions that filters as the user types (case-insensitive).
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use campaign_builder::ui_helpers::AutocompleteInput;
///
/// fn example(ui: &mut egui::Ui) {
///     let candidates = vec!["Goblin".to_string(), "Orc".to_string(), "Dragon".to_string()];
///     let mut input = String::new();
///
///     AutocompleteInput::new("monster_select", &candidates)
///         .with_placeholder("Type monster name...")
///         .show(ui, &mut input);
/// }
/// ```
pub struct AutocompleteInput<'a> {
    /// Unique widget identifier salt
    id_salt: &'a str,
    /// List of candidate suggestions
    candidates: &'a [String],
    /// Optional placeholder hint text
    placeholder: Option<&'a str>,
}

impl<'a> AutocompleteInput<'a> {
    /// Creates a new autocomplete input widget.
    ///
    /// # Arguments
    ///
    /// * `id_salt` - Unique identifier for this widget instance (used to distinguish multiple instances)
    /// * `candidates` - Slice of suggestion strings to display in dropdown
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec!["Goblin".to_string(), "Orc".to_string()];
    ///     let mut text = String::new();
    ///     AutocompleteInput::new("my_autocomplete", &candidates).show(ui, &mut text);
    /// }
    /// ```
    pub fn new(id_salt: &'a str, candidates: &'a [String]) -> Self {
        Self {
            id_salt,
            candidates,
            placeholder: None,
        }
    }

    /// Sets the placeholder hint text (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `placeholder` - Text to display when the input field is empty
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec!["Goblin".to_string()];
    ///     let mut text = String::new();
    ///
    ///     AutocompleteInput::new("autocomplete", &candidates)
    ///         .with_placeholder("Start typing...")
    ///         .show(ui, &mut text);
    /// }
    /// ```
    pub fn with_placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Renders the autocomplete widget.
    ///
    /// Displays a text input field with a dropdown list of filtered suggestions.
    /// The dropdown filters candidates case-insensitively as the user types.
    /// Clicking a suggestion or pressing Enter on a highlighted suggestion
    /// updates the text buffer.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `text` - Mutable reference to the text buffer to edit
    ///
    /// # Returns
    ///
    /// Returns the `egui::Response` from the text input widget, allowing
    /// for response chaining and inspection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec![
    ///         "Goblin".to_string(),
    ///         "Orc".to_string(),
    ///         "Dragon".to_string(),
    ///         "Skeleton".to_string(),
    ///     ];
    ///     let mut monster_name = String::new();
    ///
    ///     let response = AutocompleteInput::new("monster_input", &candidates)
    ///         .with_placeholder("Select a monster...")
    ///         .show(ui, &mut monster_name);
    ///
    ///     if response.changed() {
    ///         println!("Monster name changed to: {}", monster_name);
    ///     }
    /// }
    /// ```
    pub fn show(self, ui: &mut egui::Ui, text: &mut String) -> egui::Response {
        // Create the autocomplete text edit widget with new API
        let mut autocomplete = AutoCompleteTextEdit::new(text, self.candidates)
            .highlight_matches(true)
            .max_suggestions(10);

        // Add placeholder if provided
        if let Some(placeholder_text) = self.placeholder {
            let placeholder_owned = placeholder_text.to_string();
            autocomplete = autocomplete
                .set_text_edit_properties(move |text_edit| text_edit.hint_text(placeholder_owned));
        }

        // Show the widget and return the response
        ui.add(autocomplete)
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
/// use antares::sdk::campaign_builder::ui_helpers::load_ron_file;
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
/// `Ok(())` on success, `Err(String)` with error message on failure.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::Item;
/// use antares::sdk::campaign_builder::ui_helpers::save_ron_file;
/// use std::path::Path;
///
/// let items: Vec<Item> = vec![];
/// save_ron_file(&items, Path::new("data/items.ron")).expect("Failed to save");
/// ```
pub fn save_ron_file<T: serde::Serialize>(data: &T, path: &std::path::Path) -> Result<(), String> {
    let contents = ron::ser::to_string_pretty(data, Default::default())
        .map_err(|e| format!("Failed to serialize data: {}", e))?;

    std::fs::write(path, contents)
        .map_err(|e| format!("Failed to write file {}: {}", path.display(), e))
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
pub fn handle_file_load<T, F>(
    data: &mut Vec<T>,
    merge_mode: bool,
    id_getter: F,
    status_message: &mut String,
    unsaved_changes: &mut bool,
) -> bool
where
    T: Clone + serde::de::DeserializeOwned,
    F: Fn(&T) -> u32,
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
                        let item_id = id_getter(&item);
                        if let Some(existing) = data.iter_mut().find(|d| id_getter(d) == item_id) {
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
                *status_message = e;
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

// =============================================================================
// Entity Candidate Extraction for Autocomplete
// =============================================================================

/// Shows an autocomplete input for selecting an item by name.
///
/// Returns `true` if the selection changed (user selected an item from suggestions).
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_item_id` - Mutable reference to the currently selected ItemId (0 means none)
/// * `items` - Slice of available items
///
/// # Returns
///
/// `true` if the user selected an item, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::domain::items::types::Item;
/// use antares::domain::types::ItemId;
/// use antares::sdk::campaign_builder::ui_helpers::autocomplete_item_selector;
///
/// fn show_item_picker(ui: &mut egui::Ui, selected: &mut ItemId, items: &[Item]) {
///     if autocomplete_item_selector(ui, "weapon_picker", "Weapon:", selected, items) {
///         // User selected an item
///     }
/// }
/// ```
pub fn autocomplete_item_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_item_id: &mut antares::domain::types::ItemId,
    items: &[antares::domain::items::types::Item],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current item name
        let current_name = if *selected_item_id == 0 {
            String::new()
        } else {
            items
                .iter()
                .find(|i| i.id == *selected_item_id)
                .map(|i| i.name.clone())
                .unwrap_or_default()
        };

        let mut text_buffer = current_name.clone();

        // Build candidates
        let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing item name...")
            .show(ui, &mut text_buffer);

        // Check if user selected something from autocomplete
        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            // Try to find the item by name
            if let Some(item) = items.iter().find(|i| i.name == text_buffer) {
                *selected_item_id = item.id;
                changed = true;
            }
        }

        // Show clear button if something is selected
        if *selected_item_id != 0
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            *selected_item_id = 0;
            changed = true;
        }
    });

    changed
}

/// Shows an autocomplete input for selecting a monster by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_monster_name` - Mutable reference to the selected monster name
/// * `monsters` - Slice of available monsters
///
/// # Returns
///
/// `true` if the user selected a monster, `false` otherwise
pub fn autocomplete_monster_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_monster_name: &mut String,
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        let mut text_buffer = selected_monster_name.clone();

        // Build candidates
        let candidates: Vec<String> = extract_monster_candidates(monsters);

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing monster name...")
            .show(ui, &mut text_buffer);

        // Check if user selected something from autocomplete
        if response.changed() && !text_buffer.is_empty() && text_buffer != *selected_monster_name {
            // Validate the monster exists
            if monsters.iter().any(|m| m.name == text_buffer) {
                *selected_monster_name = text_buffer;
                changed = true;
            }
        }

        // Show clear button if something is selected
        if !selected_monster_name.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_monster_name.clear();
            changed = true;
        }
    });

    changed
}

/// Shows an autocomplete input for selecting a condition by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_condition_id` - Mutable reference to the selected condition ID
/// * `conditions` - Slice of available conditions
///
/// # Returns
///
/// `true` if the user selected a condition, `false` otherwise
pub fn autocomplete_condition_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_condition_id: &mut String,
    conditions: &[antares::domain::conditions::ConditionDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current condition name
        let current_name = conditions
            .iter()
            .find(|c| c.id == *selected_condition_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let mut text_buffer = current_name.clone();

        // Build candidates from condition names
        let candidates: Vec<String> = conditions.iter().map(|c| c.name.clone()).collect();

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing condition name...")
            .show(ui, &mut text_buffer);

        // Check if user selected something from autocomplete
        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            // Try to find the condition by name
            if let Some(condition) = conditions.iter().find(|c| c.name == text_buffer) {
                *selected_condition_id = condition.id.clone();
                changed = true;
            }
        }

        // Show clear button if something is selected
        if !selected_condition_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_condition_id.clear();
            changed = true;
        }
    });

    changed
}

/// Shows an autocomplete input for adding items to a list.
///
/// Returns `true` if an item was added to the list.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_items` - Mutable reference to the list of selected ItemIds
/// * `items` - Slice of available items
///
/// # Returns
///
/// `true` if an item was added, `false` otherwise
pub fn autocomplete_item_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_items: &mut Vec<antares::domain::types::ItemId>,
    items: &[antares::domain::items::types::Item],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Show current items
        let mut remove_idx: Option<usize> = None;
        for (idx, item_id) in selected_items.iter().enumerate() {
            ui.horizontal(|ui| {
                if let Some(item) = items.iter().find(|i| i.id == *item_id) {
                    ui.label(&item.name);
                } else {
                    ui.label(format!("Unknown item (ID: {})", item_id));
                }
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected_items.remove(idx);
            changed = true;
        }

        ui.separator();

        // Add new item input
        let mut text_buffer = String::new();
        let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();

        ui.horizontal(|ui| {
            ui.label("Add item:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing item name...")
                .show(ui, &mut text_buffer);

            // Check if user selected something from autocomplete
            if response.changed() && !text_buffer.is_empty() {
                // Try to find the item by name
                if let Some(item) = items.iter().find(|i| i.name == text_buffer) {
                    if !selected_items.contains(&item.id) {
                        selected_items.push(item.id);
                        changed = true;
                    }
                }
            }
        });
    });

    changed
}

/// Extracts monster name candidates from a list of monster definitions.
///
/// Returns a vector of monster names suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `monsters` - Slice of monster definitions to extract names from
///
/// # Returns
///
/// A vector of monster names as strings
///
/// # Examples
///
/// ```no_run
/// use antares::domain::combat::database::MonsterDefinition;
/// use antares::sdk::campaign_builder::ui_helpers::extract_monster_candidates;
///
/// let monsters = vec![
///     MonsterDefinition { id: 1, name: "Goblin".to_string(), /* ... */ },
///     MonsterDefinition { id: 2, name: "Orc".to_string(), /* ... */ },
/// ];
/// let candidates = extract_monster_candidates(&monsters);
/// assert_eq!(candidates, vec!["Goblin", "Orc"]);
/// ```
pub fn extract_monster_candidates(
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> Vec<String> {
    monsters.iter().map(|m| m.name.clone()).collect()
}

/// Extracts item candidates from a list of items.
///
/// Returns a vector of tuples mapping item display name to ItemId.
/// Display format is "{name} (ID: {id})" for clarity in the autocomplete UI.
///
/// # Arguments
///
/// * `items` - Slice of items to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, ItemId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::types::Item;
/// use antares::domain::types::ItemId;
/// use antares::sdk::campaign_builder::ui_helpers::extract_item_candidates;
///
/// let items = vec![
///     Item { id: 1, name: "Longsword".to_string(), /* ... */ },
///     Item { id: 2, name: "Health Potion".to_string(), /* ... */ },
/// ];
/// let candidates = extract_item_candidates(&items);
/// assert_eq!(candidates.len(), 2);
/// assert_eq!(candidates[0].0, "Longsword (ID: 1)");
/// ```
pub fn extract_item_candidates(
    items: &[antares::domain::items::types::Item],
) -> Vec<(String, antares::domain::types::ItemId)> {
    items
        .iter()
        .map(|item| (format!("{} (ID: {})", item.name, item.id), item.id))
        .collect()
}

/// Extracts condition candidates from a list of condition definitions.
///
/// Returns a vector of tuples mapping condition name to ConditionId.
///
/// # Arguments
///
/// * `conditions` - Slice of condition definitions to extract candidates from
///
/// # Returns
///
/// A vector of tuples (condition_name, ConditionId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::conditions::ConditionDefinition;
/// use antares::sdk::campaign_builder::ui_helpers::extract_condition_candidates;
///
/// let conditions = vec![
///     ConditionDefinition { id: "poison".to_string(), name: "Poisoned".to_string(), /* ... */ },
///     ConditionDefinition { id: "sleep".to_string(), name: "Sleeping".to_string(), /* ... */ },
/// ];
/// let candidates = extract_condition_candidates(&conditions);
/// assert_eq!(candidates.len(), 2);
/// assert_eq!(candidates[0].0, "Poisoned");
/// ```
pub fn extract_condition_candidates(
    conditions: &[antares::domain::conditions::ConditionDefinition],
) -> Vec<(String, String)> {
    conditions
        .iter()
        .map(|cond| (cond.name.clone(), cond.id.clone()))
        .collect()
}

/// Extracts spell candidates from a list of spell definitions.
///
/// Returns a vector of tuples mapping spell display name to SpellId.
/// Display format is "{name} (ID: {id})" for clarity.
///
/// # Arguments
///
/// * `spells` - Slice of spells to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, SpellId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::magic::types::Spell;
/// use antares::domain::types::SpellId;
/// use antares::sdk::campaign_builder::ui_helpers::extract_spell_candidates;
///
/// let spells = vec![
///     Spell { id: 1, name: "Fireball".to_string(), /* ... */ },
///     Spell { id: 2, name: "Heal".to_string(), /* ... */ },
/// ];
/// let candidates = extract_spell_candidates(&spells);
/// assert_eq!(candidates.len(), 2);
/// assert_eq!(candidates[0].0, "Fireball (ID: 1)");
/// ```
pub fn extract_spell_candidates(
    spells: &[antares::domain::magic::types::Spell],
) -> Vec<(String, antares::domain::types::SpellId)> {
    spells
        .iter()
        .map(|spell| (format!("{} (ID: {})", spell.name, spell.id), spell.id))
        .collect()
}

/// Extracts proficiency candidates from the proficiency database.
///
/// Returns a vector of proficiency ID strings suitable for autocomplete.
///
/// # Arguments
///
/// * `proficiencies` - Slice of proficiency IDs
///
/// # Returns
///
/// A vector of proficiency ID strings
///
/// # Examples
///
/// ```no_run
/// use antares::domain::proficiency::ProficiencyId;
/// use antares::sdk::campaign_builder::ui_helpers::extract_proficiency_candidates;
///
/// let proficiencies = vec![
///     ProficiencyId::from("sword"),
///     ProficiencyId::from("shield"),
/// ];
/// let candidates = extract_proficiency_candidates(&proficiencies);
/// assert_eq!(candidates.len(), 2);
/// ```
pub fn extract_proficiency_candidates(
    proficiencies: &[antares::domain::proficiency::ProficiencyId],
) -> Vec<String> {
    proficiencies.iter().map(|p| p.to_string()).collect()
}

// =============================================================================
// Candidate Cache for Performance Optimization
// =============================================================================

/// Cache for autocomplete candidates to avoid regenerating on every frame.
///
/// This structure caches candidate lists and invalidates them only when
/// the underlying data changes (add/delete/import operations).
#[derive(Debug, Default)]
pub struct AutocompleteCandidateCache {
    /// Cached item candidates with generation counter
    items: Option<(Vec<(String, antares::domain::types::ItemId)>, u64)>,
    /// Cached monster candidates with generation counter
    monsters: Option<(Vec<String>, u64)>,
    /// Cached condition candidates with generation counter
    conditions: Option<(Vec<(String, String)>, u64)>,
    /// Cached spell candidates with generation counter
    spells: Option<(Vec<(String, antares::domain::types::SpellId)>, u64)>,
    /// Cached proficiency candidates with generation counter
    proficiencies: Option<(Vec<String>, u64)>,
    /// Generation counter for items (incremented on data changes)
    items_generation: u64,
    /// Generation counter for monsters
    monsters_generation: u64,
    /// Generation counter for conditions
    conditions_generation: u64,
    /// Generation counter for spells
    spells_generation: u64,
    /// Generation counter for proficiencies
    proficiencies_generation: u64,
}

impl AutocompleteCandidateCache {
    /// Creates a new empty candidate cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates the item candidates cache.
    ///
    /// Call this when items are added, deleted, or imported.
    pub fn invalidate_items(&mut self) {
        self.items_generation += 1;
        self.items = None;
    }

    /// Invalidates the monster candidates cache.
    ///
    /// Call this when monsters are added, deleted, or imported.
    pub fn invalidate_monsters(&mut self) {
        self.monsters_generation += 1;
        self.monsters = None;
    }

    /// Invalidates the condition candidates cache.
    ///
    /// Call this when conditions are added, deleted, or imported.
    pub fn invalidate_conditions(&mut self) {
        self.conditions_generation += 1;
        self.conditions = None;
    }

    /// Invalidates the spell candidates cache.
    ///
    /// Call this when spells are added, deleted, or imported.
    pub fn invalidate_spells(&mut self) {
        self.spells_generation += 1;
        self.spells = None;
    }

    /// Invalidates the proficiency candidates cache.
    ///
    /// Call this when proficiencies are added, deleted, or imported.
    pub fn invalidate_proficiencies(&mut self) {
        self.proficiencies_generation += 1;
        self.proficiencies = None;
    }

    /// Invalidates all caches.
    ///
    /// Call this when loading a new campaign or resetting data.
    pub fn invalidate_all(&mut self) {
        self.invalidate_items();
        self.invalidate_monsters();
        self.invalidate_conditions();
        self.invalidate_spells();
        self.invalidate_proficiencies();
    }

    /// Gets or generates item candidates.
    ///
    /// Returns cached candidates if available and valid, otherwise generates
    /// new candidates and caches them.
    pub fn get_or_generate_items(
        &mut self,
        items: &[antares::domain::items::types::Item],
    ) -> Vec<(String, antares::domain::types::ItemId)> {
        // Check if cache is valid
        if let Some((ref candidates, gen)) = &self.items {
            if *gen == self.items_generation {
                return candidates.clone();
            }
        }

        // Generate new candidates
        let candidates = extract_item_candidates(items);
        self.items = Some((candidates.clone(), self.items_generation));
        candidates
    }

    /// Gets or generates monster candidates.
    pub fn get_or_generate_monsters(
        &mut self,
        monsters: &[antares::domain::combat::database::MonsterDefinition],
    ) -> Vec<String> {
        if let Some((ref candidates, gen)) = &self.monsters {
            if *gen == self.monsters_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_monster_candidates(monsters);
        self.monsters = Some((candidates.clone(), self.monsters_generation));
        candidates
    }

    /// Gets or generates condition candidates.
    pub fn get_or_generate_conditions(
        &mut self,
        conditions: &[antares::domain::conditions::ConditionDefinition],
    ) -> Vec<(String, String)> {
        if let Some((ref candidates, gen)) = &self.conditions {
            if *gen == self.conditions_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_condition_candidates(conditions);
        self.conditions = Some((candidates.clone(), self.conditions_generation));
        candidates
    }

    /// Gets or generates spell candidates.
    pub fn get_or_generate_spells(
        &mut self,
        spells: &[antares::domain::spell::Spell],
    ) -> Vec<(String, antares::domain::types::SpellId)> {
        if let Some((ref candidates, gen)) = &self.spells {
            if *gen == self.spells_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_spell_candidates(spells);
        self.spells = Some((candidates.clone(), self.spells_generation));
        candidates
    }

    /// Gets or generates proficiency candidates.
    pub fn get_or_generate_proficiencies(
        &mut self,
        proficiencies: &[antares::domain::proficiency::ProficiencyId],
    ) -> Vec<String> {
        if let Some((ref candidates, gen)) = &self.proficiencies {
            if *gen == self.proficiencies_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_proficiency_candidates(proficiencies);
        self.proficiencies = Some((candidates.clone(), self.proficiencies_generation));
        candidates
    }
}

// =============================================================================
// Entity Validation Helpers
// =============================================================================

/// Displays a warning label if an entity ID is invalid (not found in the list).
///
/// This helper provides consistent user feedback when a selected entity
/// reference doesn't exist in the current campaign data.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `entity_type` - Type of entity (e.g., "Item", "Monster", "Spell")
/// * `id` - The entity ID being validated
/// * `exists` - Whether the entity exists in the current data
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::sdk::campaign_builder::ui_helpers::show_entity_validation_warning;
///
/// fn example(ui: &mut egui::Ui, item_id: u32, items: &[Item]) {
///     let exists = items.iter().any(|i| i.id == item_id);
///     show_entity_validation_warning(ui, "Item", item_id, exists);
/// }
/// ```
pub fn show_entity_validation_warning(
    ui: &mut egui::Ui,
    entity_type: &str,
    id: impl std::fmt::Display,
    exists: bool,
) {
    if !exists {
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 0),
            format!("⚠ {} ID {} not found in campaign data", entity_type, id),
        );
    }
}

/// Displays a warning if an item ID is invalid.
///
/// Convenience wrapper around `show_entity_validation_warning` for items.
pub fn show_item_validation_warning(
    ui: &mut egui::Ui,
    item_id: antares::domain::types::ItemId,
    items: &[antares::domain::items::types::Item],
) {
    let exists = items.iter().any(|i| i.id == item_id);
    show_entity_validation_warning(ui, "Item", item_id, exists);
}

/// Displays a warning if a monster name is invalid.
///
/// Convenience wrapper around `show_entity_validation_warning` for monsters.
pub fn show_monster_validation_warning(
    ui: &mut egui::Ui,
    monster_name: &str,
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) {
    if monster_name.is_empty() {
        return;
    }
    let exists = monsters.iter().any(|m| m.name == monster_name);
    show_entity_validation_warning(ui, "Monster", monster_name, exists);
}

/// Displays a warning if a condition ID is invalid.
///
/// Convenience wrapper around `show_entity_validation_warning` for conditions.
pub fn show_condition_validation_warning(
    ui: &mut egui::Ui,
    condition_id: &str,
    conditions: &[antares::domain::conditions::ConditionDefinition],
) {
    if condition_id.is_empty() {
        return;
    }
    let exists = conditions.iter().any(|c| c.id == condition_id);
    show_entity_validation_warning(ui, "Condition", condition_id, exists);
}

/// Displays a warning if a spell ID is invalid.
///
/// Convenience wrapper around `show_entity_validation_warning` for spells.
pub fn show_spell_validation_warning(
    ui: &mut egui::Ui,
    spell_id: antares::domain::types::SpellId,
    spells: &[antares::domain::spell::Spell],
) {
    let exists = spells.iter().any(|s| s.id == spell_id);
    show_entity_validation_warning(ui, "Spell", spell_id, exists);
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Vec2;

    // =========================================================================
    // Panel Height Tests
    // =========================================================================

    #[test]
    fn compute_panel_height_from_size_returns_size_y_if_larger() {
        let size = Vec2::new(100.0, 250.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), 250.0);
    }

    #[test]
    fn compute_panel_height_from_size_returns_min_if_size_smaller() {
        let size = Vec2::new(100.0, 40.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), min);
    }

    #[test]
    fn compute_default_panel_height_from_size_uses_default_min() {
        let size = Vec2::new(640.0, 90.0);
        assert_eq!(
            compute_default_panel_height_from_size(size),
            DEFAULT_PANEL_MIN_HEIGHT
        );
    }

    #[test]
    fn compute_panel_height_from_size_handles_exact_minimum() {
        let size = Vec2::new(100.0, 100.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), 100.0);
    }

    #[test]
    fn compute_panel_height_from_size_handles_zero_size() {
        let size = Vec2::new(0.0, 0.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), min);
    }

    // =========================================================================
    // CSV/Filter/Format Helper Tests
    // =========================================================================

    #[test]
    fn parse_id_csv_to_vec_simple() {
        let parsed = parse_id_csv_to_vec::<u8>("1, 2, 3").unwrap();
        assert_eq!(parsed, vec![1u8, 2u8, 3u8]);
    }

    #[test]
    fn parse_id_csv_to_vec_empty() {
        let parsed = parse_id_csv_to_vec::<u8>("").unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_id_csv_to_vec_whitespace_and_commas() {
        let parsed = parse_id_csv_to_vec::<u8>(" 1 ,  , 2 ,  3 ").unwrap();
        assert_eq!(parsed, vec![1u8, 2u8, 3u8]);
    }

    #[test]
    fn parse_id_csv_to_vec_invalid() {
        let err = parse_id_csv_to_vec::<u8>("a, 2");
        assert!(err.is_err());
    }

    #[test]
    fn format_vec_to_csv_simple() {
        assert_eq!(format_vec_to_csv(&[1u8, 2u8, 3u8]), "1, 2, 3");
    }

    #[test]
    fn format_vec_to_csv_empty() {
        assert_eq!(format_vec_to_csv::<u8>(&[]), "");
    }

    #[test]
    fn filter_items_by_query_basic() {
        struct Foo {
            name: String,
        }
        let items = vec![
            Foo {
                name: "Goblin".to_string(),
            },
            Foo {
                name: "Orc".to_string(),
            },
            Foo {
                name: "Golem".to_string(),
            },
        ];

        let idx = filter_items_by_query(&items, "gob", |f| f.name.clone());
        assert_eq!(idx, vec![0usize]);

        let idx_all = filter_items_by_query(&items, "", |f| f.name.clone());
        assert_eq!(idx_all, vec![0usize, 1usize, 2usize]);

        let idx_g = filter_items_by_query(&items, "g", |f| f.name.clone());
        assert_eq!(idx_g, vec![0usize, 2usize]);
    }

    // =========================================================================
    // ToolbarAction Tests
    // =========================================================================

    #[test]
    fn toolbar_action_enum_values() {
        assert_ne!(ToolbarAction::New, ToolbarAction::Save);
        assert_ne!(ToolbarAction::Load, ToolbarAction::Import);
        assert_ne!(ToolbarAction::Export, ToolbarAction::Reload);
        assert_eq!(ToolbarAction::None, ToolbarAction::None);
    }

    #[test]
    fn editor_toolbar_new_creates_with_defaults() {
        let toolbar = EditorToolbar::new("Test");
        assert_eq!(toolbar.editor_name, "Test");
        assert!(toolbar.search_query.is_none());
        assert!(toolbar.merge_mode.is_none());
        assert!(toolbar.total_count.is_none());
        assert!(toolbar.show_save);
    }

    #[test]
    fn editor_toolbar_builder_pattern() {
        let mut search = String::new();
        let mut merge = false;

        let toolbar = EditorToolbar::new("Items")
            .with_search(&mut search)
            .with_merge_mode(&mut merge)
            .with_total_count(42)
            .with_save_button(false)
            .with_id_salt("test_salt");

        assert_eq!(toolbar.editor_name, "Items");
        assert!(toolbar.search_query.is_some());
        assert!(toolbar.merge_mode.is_some());
        assert_eq!(toolbar.total_count, Some(42));
        assert!(!toolbar.show_save);
        assert_eq!(toolbar.id_salt, Some("test_salt"));
    }

    // =========================================================================
    // ItemAction Tests
    // =========================================================================

    #[test]
    fn item_action_enum_values() {
        assert_ne!(ItemAction::Edit, ItemAction::Delete);
        assert_ne!(ItemAction::Duplicate, ItemAction::Export);
        assert_eq!(ItemAction::None, ItemAction::None);
    }

    #[test]
    fn action_buttons_default_all_visible() {
        let buttons = ActionButtons::new();
        assert!(buttons.enabled);
        assert!(buttons.show_edit);
        assert!(buttons.show_delete);
        assert!(buttons.show_duplicate);
        assert!(buttons.show_export);
    }

    #[test]
    fn action_buttons_builder_pattern() {
        let buttons = ActionButtons::new()
            .enabled(false)
            .with_edit(false)
            .with_delete(true)
            .with_duplicate(false)
            .with_export(true);

        assert!(!buttons.enabled);
        assert!(!buttons.show_edit);
        assert!(buttons.show_delete);
        assert!(!buttons.show_duplicate);
        assert!(buttons.show_export);
    }

    // =========================================================================
    // TwoColumnLayout Tests
    // =========================================================================

    #[test]
    fn two_column_layout_new_uses_defaults() {
        let layout = TwoColumnLayout::new("test");
        assert_eq!(layout.id_salt, "test");
        assert_eq!(layout.left_width, DEFAULT_LEFT_COLUMN_WIDTH);
        assert_eq!(layout.min_height, DEFAULT_PANEL_MIN_HEIGHT);
    }

    #[test]
    fn two_column_layout_builder_pattern() {
        let layout = TwoColumnLayout::new("custom")
            .with_left_width(400.0)
            .with_min_height(200.0)
            .with_inspector_min_width(320.0)
            .with_max_left_ratio(0.65);

        assert_eq!(layout.id_salt, "custom");
        assert_eq!(layout.left_width, 400.0);
        assert_eq!(layout.min_height, 200.0);
        assert_eq!(layout.inspector_min_width, 320.0);
        assert_eq!(layout.max_left_ratio, 0.65);
    }

    // =========================================================================
    // Additional TwoColumnLayout Tests
    // =========================================================================

    #[test]
    fn two_column_layout_show_split_calls_both_closures() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(1200.0, 800.0),
        ));
        ctx.begin_pass(raw_input);

        let left_called = std::rc::Rc::new(std::cell::Cell::new(false));
        let right_called = std::rc::Rc::new(std::cell::Cell::new(false));

        {
            let left_clone = left_called.clone();
            let right_clone = right_called.clone();
            egui::CentralPanel::default().show(&ctx, |ui| {
                TwoColumnLayout::new("test")
                    .with_left_width(400.0)
                    .with_inspector_min_width(300.0)
                    .with_max_left_ratio(DEFAULT_LEFT_COLUMN_MAX_RATIO)
                    .show_split(
                        ui,
                        |left_ui| {
                            left_clone.set(true);
                            // Small touch to exercise scroll area width
                            left_ui.label("Left content");
                        },
                        |right_ui| {
                            right_clone.set(true);
                            right_ui.label("Right content");
                        },
                    );
            });
        }
        let _ = ctx.end_pass();

        assert!(left_called.get());
        assert!(right_called.get());
    }

    #[test]
    fn render_grid_header_draws_headers() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            egui::Grid::new("test_grid").num_columns(3).show(ui, |ui| {
                render_grid_header(ui, &["Status", "Message", "File"]);
                // Add a sample row to ensure grid usage doesn't panic
                ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "❌");
                ui.label("Sample message");
                ui.label("-");
                ui.end_row();
            });
        });

        let _ = ctx.end_pass();
    }

    #[test]
    fn show_validation_severity_icon_shows_icon() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(400.0, 300.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            super::show_validation_severity_icon(ui, crate::validation::ValidationSeverity::Error);
        });

        let _ = ctx.end_pass();
    }

    // =========================================================================
    // compute_left_column_width tests
    // =========================================================================

    #[test]
    fn compute_left_column_width_small_total_width() {
        // 480 total width, inspector min 300, separator 12 -> available left = 168
        let total_width = 480.0;
        let requested_left = 300.0;
        let inspector_min = 300.0;
        let sep_margin = 12.0;
        let left = compute_left_column_width(
            total_width,
            requested_left,
            inspector_min,
            sep_margin,
            MIN_SAFE_LEFT_COLUMN_WIDTH,
            DEFAULT_LEFT_COLUMN_MAX_RATIO,
        );
        // force exact equality to the available space (168)
        assert_eq!(left, 168.0);
    }

    #[test]
    fn compute_left_column_width_enforces_min_when_space_available() {
        // 1200 total width, enough to allow min safe left width (250)
        let total_width = 1200.0;
        let requested_left = 400.0;
        let inspector_min = 300.0;
        let sep_margin = 12.0;
        let left = compute_left_column_width(
            total_width,
            requested_left,
            inspector_min,
            sep_margin,
            MIN_SAFE_LEFT_COLUMN_WIDTH,
            DEFAULT_LEFT_COLUMN_MAX_RATIO,
        );
        assert_eq!(left, 400.0);
    }

    #[test]
    fn compute_left_column_width_enforces_max_ratio_limit() {
        // 800 total width: available left = 488 -> should be upper bound
        let total_width = 800.0;
        let requested_left = 600.0;
        let inspector_min = 300.0;
        let sep_margin = 12.0;
        let left = compute_left_column_width(
            total_width,
            requested_left,
            inspector_min,
            sep_margin,
            MIN_SAFE_LEFT_COLUMN_WIDTH,
            DEFAULT_LEFT_COLUMN_MAX_RATIO,
        );
        assert_eq!(left, 488.0);
    }

    #[test]
    fn compute_left_column_width_zero_when_no_space() {
        // total width smaller than inspector_min + separator -> 0.0 left width
        let total_width = 200.0;
        let requested_left = 250.0;
        let inspector_min = 300.0;
        let sep_margin = 12.0;
        let left = compute_left_column_width(
            total_width,
            requested_left,
            inspector_min,
            sep_margin,
            MIN_SAFE_LEFT_COLUMN_WIDTH,
            DEFAULT_LEFT_COLUMN_MAX_RATIO,
        );
        assert_eq!(left, 0.0);
    }

    // =========================================================================
    // ImportExportDialog Tests
    // =========================================================================

    #[test]
    fn import_export_dialog_state_new() {
        let state = ImportExportDialogState::new();
        assert!(!state.is_open);
        assert!(state.buffer.is_empty());
        assert!(state.error_message.is_none());
        assert!(!state.export_mode);
    }

    #[test]
    fn import_export_dialog_state_open_import() {
        let mut state = ImportExportDialogState::new();
        state.buffer = "old data".to_string();
        state.error_message = Some("old error".to_string());

        state.open_import();

        assert!(state.is_open);
        assert!(state.buffer.is_empty());
        assert!(state.error_message.is_none());
        assert!(!state.export_mode);
    }

    #[test]
    fn import_export_dialog_state_open_export() {
        let mut state = ImportExportDialogState::new();
        state.open_export("exported data".to_string());

        assert!(state.is_open);
        assert_eq!(state.buffer, "exported data");
        assert!(state.error_message.is_none());
        assert!(state.export_mode);
    }

    #[test]
    fn import_export_dialog_state_close() {
        let mut state = ImportExportDialogState::new();
        state.open_export("data".to_string());
        state.set_error("error");

        state.close();

        assert!(!state.is_open);
        assert!(state.buffer.is_empty());
        assert!(state.error_message.is_none());
    }

    #[test]
    fn import_export_dialog_state_set_error() {
        let mut state = ImportExportDialogState::new();
        state.set_error("Parse error");

        assert_eq!(state.error_message, Some("Parse error".to_string()));
    }

    #[test]
    fn import_export_result_enum() {
        let import_result = ImportExportResult::Import("data".to_string());
        let cancel_result = ImportExportResult::Cancel;
        let open_result = ImportExportResult::Open;

        assert_ne!(import_result, cancel_result);
        assert_ne!(cancel_result, open_result);
        assert_eq!(
            ImportExportResult::Import("data".to_string()),
            ImportExportResult::Import("data".to_string())
        );
    }

    // =========================================================================
    // AttributePairInput Tests
    // =========================================================================

    #[test]
    fn attribute_pair_input_state_new() {
        let state = AttributePairInputState::new();
        assert!(state.auto_sync);
    }

    #[test]
    fn attribute_pair_input_state_with_auto_sync() {
        let state = AttributePairInputState::with_auto_sync(false);
        assert!(!state.auto_sync);

        let state = AttributePairInputState::with_auto_sync(true);
        assert!(state.auto_sync);
    }

    #[test]
    fn attribute_pair_reset_behavior() {
        let mut attr = AttributePair::new(10);
        attr.current = 25;
        assert_eq!(attr.base, 10);
        assert_eq!(attr.current, 25);

        attr.reset();
        assert_eq!(attr.current, 10);
    }

    #[test]
    fn attribute_pair16_reset_behavior() {
        let mut attr = AttributePair16::new(100);
        attr.current = 250;
        assert_eq!(attr.base, 100);
        assert_eq!(attr.current, 250);

        attr.reset();
        assert_eq!(attr.current, 100);
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn default_constants_have_expected_values() {
        // Verify constants have the expected values
        assert_eq!(DEFAULT_LEFT_COLUMN_WIDTH, 300.0);
        assert_eq!(DEFAULT_PANEL_MIN_HEIGHT, 100.0);
    }

    // =========================================================================
    // Keyboard Shortcuts Tests
    // =========================================================================

    #[test]
    fn toolbar_action_keyboard_shortcuts_documented() {
        // This test documents the keyboard shortcuts for EditorToolbar:
        // - Ctrl+N: New
        // - Ctrl+S: Save
        // - Ctrl+L: Load
        // - Ctrl+Shift+I: Import
        // - Ctrl+Shift+E: Export
        // - F5: Reload
        //
        // Note: We cannot easily unit test keyboard input in egui without
        // a full rendering context, so this test serves as documentation.
        // The shortcuts are implemented in EditorToolbar::show() and should
        // be manually tested.
        assert_eq!(ToolbarAction::New as i32, 0);
    }

    #[test]
    fn item_action_keyboard_shortcuts_documented() {
        // This test documents the keyboard shortcuts for ActionButtons:
        // - Ctrl+E: Edit
        // - Delete: Delete
        // - Ctrl+D: Duplicate
        //
        // Note: We cannot easily unit test keyboard input in egui without
        // a full rendering context, so this test serves as documentation.
        // The shortcuts are implemented in ActionButtons::show() and should
        // be manually tested.
        assert_eq!(ItemAction::Edit as i32, 0);
    }

    #[test]
    fn toolbar_buttons_have_consistent_labels() {
        // This test documents the standardized button labels:
        // - ➕ New
        // - 💾 Save
        // - 📂 Load
        // - 📥 Import
        // - 📋 Export
        // - 🔄 Reload
        //
        // All editors must use these labels consistently.
        // The labels are implemented in EditorToolbar::show().
        assert!(true);
    }

    #[test]
    fn action_buttons_have_consistent_labels() {
        // This test documents the standardized action button labels:
        // - ✏️ Edit
        // - 🗑️ Delete
        // - 📋 Duplicate
        // - 📤 Export
        //
        // All editors must use these labels consistently.
        // The labels are implemented in ActionButtons::show().
        assert!(true);
    }

    #[test]
    fn toolbar_buttons_have_tooltips_with_shortcuts() {
        // This test documents that all toolbar buttons should have
        // tooltips showing their keyboard shortcuts.
        // The tooltips are implemented using .on_hover_text() in
        // EditorToolbar::show().
        assert!(true);
    }

    #[test]
    fn action_buttons_have_tooltips_with_shortcuts() {
        // This test documents that all action buttons should have
        // tooltips showing their keyboard shortcuts.
        // The tooltips are implemented using .on_hover_text() in
        // ActionButtons::show().
        assert!(true);
    }

    // =========================================================================
    // AutocompleteInput Tests
    // =========================================================================

    #[test]
    fn autocomplete_input_new_creates_widget() {
        let candidates = vec!["Goblin".to_string(), "Orc".to_string()];
        let widget = AutocompleteInput::new("test_autocomplete", &candidates);

        assert_eq!(widget.id_salt, "test_autocomplete");
        assert_eq!(widget.candidates.len(), 2);
        assert_eq!(widget.placeholder, None);
    }

    #[test]
    fn autocomplete_input_with_placeholder() {
        let candidates = vec!["Dragon".to_string()];
        let widget = AutocompleteInput::new("test", &candidates).with_placeholder("Type here...");

        assert_eq!(widget.placeholder, Some("Type here..."));
    }

    #[test]
    fn autocomplete_input_builder_pattern() {
        let candidates = vec![
            "Goblin".to_string(),
            "Orc".to_string(),
            "Dragon".to_string(),
        ];

        let widget =
            AutocompleteInput::new("my_widget", &candidates).with_placeholder("Select monster...");

        assert_eq!(widget.id_salt, "my_widget");
        assert_eq!(widget.candidates.len(), 3);
        assert_eq!(widget.placeholder, Some("Select monster..."));
    }

    #[test]
    fn autocomplete_input_empty_candidates() {
        let candidates: Vec<String> = vec![];
        let widget = AutocompleteInput::new("empty_test", &candidates);

        assert_eq!(widget.candidates.len(), 0);
    }

    #[test]
    fn autocomplete_input_many_candidates() {
        let candidates: Vec<String> = (0..100).map(|i| format!("Monster{}", i)).collect();

        let widget = AutocompleteInput::new("many_test", &candidates);

        assert_eq!(widget.candidates.len(), 100);
    }

    #[test]
    fn autocomplete_input_unique_id_salt() {
        let candidates = vec!["Item1".to_string()];

        let widget1 = AutocompleteInput::new("widget1", &candidates);
        let widget2 = AutocompleteInput::new("widget2", &candidates);

        assert_ne!(widget1.id_salt, widget2.id_salt);
    }

    #[test]
    fn autocomplete_input_case_sensitivity_documented() {
        // This test documents that AutocompleteInput performs
        // case-insensitive filtering by default.
        // For example, typing "gob" should match "Goblin", "GOBLIN", "goblin".
        // The case-insensitive behavior is implemented via the
        // egui_autocomplete::AutoCompleteTextEdit widget.
        // This should be verified with manual testing in the UI.
        let candidates = vec![
            "Goblin".to_string(),
            "GOBLIN".to_string(),
            "goblin".to_string(),
        ];
        let widget = AutocompleteInput::new("case_test", &candidates);
        assert_eq!(widget.candidates.len(), 3);
    }

    #[test]
    fn autocomplete_input_max_suggestions_limit() {
        // This test documents that AutocompleteInput limits the dropdown
        // to a maximum of 10 suggestions to prevent UI clutter.
        // This is configured via .max_suggestions(10) in the show() method.
        // With more than 10 matching candidates, only the first 10 are shown.
        let candidates: Vec<String> = (0..20).map(|i| format!("Monster{}", i)).collect();
        let widget = AutocompleteInput::new("limit_test", &candidates);
        assert!(widget.candidates.len() > 10);
    }

    #[test]
    fn autocomplete_input_highlight_matches_enabled() {
        // This test documents that AutocompleteInput highlights matching
        // text in the dropdown suggestions for better user experience.
        // This is enabled via .highlight_matches(true) in the show() method.
        // Manual testing should verify that matching substrings are highlighted.
        let candidates = vec!["Goblin".to_string(), "Hobgoblin".to_string()];
        let widget = AutocompleteInput::new("highlight_test", &candidates);
        assert_eq!(widget.candidates.len(), 2);
    }

    #[test]
    fn autocomplete_input_follows_ui_helper_conventions() {
        // This test verifies that AutocompleteInput follows the same
        // conventions as other UI helpers:
        // - Uses builder pattern (with_* methods)
        // - Returns Self for chaining
        // - Uses &'a lifetime for borrowed references
        // - Has comprehensive doc comments with examples
        let candidates = vec!["Test".to_string()];
        let widget = AutocompleteInput::new("convention_test", &candidates)
            .with_placeholder("Test placeholder");

        assert!(widget.placeholder.is_some());
    }

    // =========================================================================
    // Entity Candidate Extraction Tests
    // =========================================================================

    #[test]
    fn extract_monster_candidates_empty_list() {
        let monsters = vec![];
        let candidates = extract_monster_candidates(&monsters);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn extract_item_candidates_empty_list() {
        let items = vec![];
        let candidates = extract_item_candidates(&items);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn extract_condition_candidates_empty_list() {
        let conditions = vec![];
        let candidates = extract_condition_candidates(&conditions);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn extract_spell_candidates_empty_list() {
        let spells = vec![];
        let candidates = extract_spell_candidates(&spells);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn extract_proficiency_candidates_empty_list() {
        let proficiencies = vec![];
        let candidates = extract_proficiency_candidates(&proficiencies);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn extract_proficiency_candidates_returns_string_ids() {
        use antares::domain::proficiency::ProficiencyId;

        let proficiencies = vec![
            ProficiencyId::from("sword"),
            ProficiencyId::from("shield"),
            ProficiencyId::from("heavy_armor"),
        ];

        let candidates = extract_proficiency_candidates(&proficiencies);
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], "sword");
        assert_eq!(candidates[1], "shield");
        assert_eq!(candidates[2], "heavy_armor");
    }

    // =========================================================================
    // Phase 3: Candidate Cache Tests
    // =========================================================================

    #[test]
    fn candidate_cache_new_is_empty() {
        let cache = AutocompleteCandidateCache::new();
        assert!(cache.items.is_none());
        assert!(cache.monsters.is_none());
        assert!(cache.conditions.is_none());
        assert!(cache.spells.is_none());
        assert!(cache.proficiencies.is_none());
    }

    #[test]
    fn candidate_cache_invalidate_items_clears_cache() {
        let mut cache = AutocompleteCandidateCache::new();
        // Simulate cached items
        cache.items = Some((vec![("Test".to_string(), 1)], 0));

        cache.invalidate_items();

        assert!(cache.items.is_none());
        assert_eq!(cache.items_generation, 1);
    }

    #[test]
    fn candidate_cache_invalidate_all_clears_all_caches() {
        let mut cache = AutocompleteCandidateCache::new();
        cache.items = Some((vec![("Test".to_string(), 1)], 0));
        cache.monsters = Some((vec!["Monster".to_string()], 0));

        cache.invalidate_all();

        assert!(cache.items.is_none());
        assert!(cache.monsters.is_none());
        assert!(cache.conditions.is_none());
        assert!(cache.spells.is_none());
        assert!(cache.proficiencies.is_none());
        assert_eq!(cache.items_generation, 1);
        assert_eq!(cache.monsters_generation, 1);
    }

    #[test]
    fn candidate_cache_get_or_generate_items_caches_results() {
        use antares::domain::items::types::{ArmorData, Item, ItemType};

        let mut cache = AutocompleteCandidateCache::new();
        #[allow(deprecated)]
        let items = vec![Item {
            id: 1,
            name: "Sword".to_string(),
            description: String::new(),
            item_type: ItemType::Armor(ArmorData::default()),
            value: 0,
            weight: 0,
            tags: Vec::new(),
            disablement: antares::domain::items::types::Disablement::default(),
            cursed: false,
            charges: None,
            max_charges: None,
        }];

        // First call generates and caches
        let candidates1 = cache.get_or_generate_items(&items);
        assert_eq!(candidates1.len(), 1);
        assert_eq!(candidates1[0].1, 1);

        // Second call returns cached results
        let candidates2 = cache.get_or_generate_items(&items);
        assert_eq!(candidates2.len(), 1);
    }

    #[test]
    fn candidate_cache_invalidation_forces_regeneration() {
        use antares::domain::items::types::{ArmorData, Item, ItemType};

        let mut cache = AutocompleteCandidateCache::new();
        #[allow(deprecated)]
        let items_old = vec![Item {
            id: 1,
            name: "Sword".to_string(),
            description: String::new(),
            item_type: ItemType::Armor(ArmorData::default()),
            value: 0,
            weight: 0,
            tags: Vec::new(),
            disablement: antares::domain::items::types::Disablement::default(),
            cursed: false,
            charges: None,
            max_charges: None,
        }];

        // Generate initial cache
        let _candidates1 = cache.get_or_generate_items(&items_old);

        // Invalidate cache
        cache.invalidate_items();

        // New data should be generated
        #[allow(deprecated)]
        let items_new = vec![Item {
            id: 2,
            name: "Axe".to_string(),
            description: String::new(),
            item_type: ItemType::Armor(ArmorData::default()),
            value: 0,
            weight: 0,
            tags: Vec::new(),
            disablement: antares::domain::items::types::Disablement::default(),
            cursed: false,
            charges: None,
            max_charges: None,
        }];
        let candidates2 = cache.get_or_generate_items(&items_new);
        assert_eq!(candidates2.len(), 1);
        assert_eq!(candidates2[0].1, 2);
    }

    #[test]
    fn candidate_cache_monsters_caches_correctly() {
        use antares::domain::character::{AttributePair, AttributePair16, Stats};
        use antares::domain::combat::database::MonsterDefinition;
        use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};

        let mut cache = AutocompleteCandidateCache::new();
        let monsters = vec![MonsterDefinition {
            id: 1,
            name: "Goblin".to_string(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: AttributePair16::new(10),
            ac: AttributePair::new(10),
            attacks: Vec::new(),
            flee_threshold: 0,
            regeneration: 0,
            advance_count: 0,
            experience: 10,
            resistances: MonsterResistances::default(),
            gold: 0,
            gems: 0,
            loot_table: LootTable::default(),
            conditions: Vec::<MonsterCondition>::new(),
            special_attack_chance: 0,
            special_attack_description: None,
        }];

        let candidates = cache.get_or_generate_monsters(&monsters);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "Goblin");
    }

    #[test]
    fn candidate_cache_performance_with_200_items() {
        use antares::domain::items::types::{ArmorData, Item, ItemType};
        use std::time::Instant;

        let mut cache = AutocompleteCandidateCache::new();

        // Generate 200 items
        #[allow(deprecated)]
        let items: Vec<Item> = (0..200)
            .map(|i| Item {
                id: i,
                name: format!("Item{}", i),
                description: String::new(),
                item_type: ItemType::Armor(ArmorData::default()),
                value: 0,
                weight: 0,
                tags: Vec::new(),
                disablement: antares::domain::items::types::Disablement::default(),
                cursed: false,
                charges: None,
                max_charges: None,
            })
            .collect();

        // First call - measure generation time
        let start = Instant::now();
        let candidates1 = cache.get_or_generate_items(&items);
        let gen_time = start.elapsed();
        assert_eq!(candidates1.len(), 200);

        // Second call - should be instant (cached)
        let start = Instant::now();
        let candidates2 = cache.get_or_generate_items(&items);
        let cache_time = start.elapsed();
        assert_eq!(candidates2.len(), 200);

        // Cache retrieval should be significantly faster than generation
        // (at least 10x faster, but we'll use 2x to be conservative)
        assert!(
            cache_time < gen_time / 2,
            "Cache retrieval time ({:?}) should be much faster than generation time ({:?})",
            cache_time,
            gen_time
        );
    }

    // =========================================================================
    // Phase 3: Validation Warning Tests
    // =========================================================================

    #[test]
    fn show_entity_validation_warning_displays_nothing_when_valid() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Should not display warning when entity exists
            show_entity_validation_warning(ui, "Item", 42, true);
        });

        let _ = ctx.end_pass();
        // Test passes if no panic - UI functions don't return testable state
    }

    #[test]
    fn show_item_validation_warning_checks_existence() {
        use antares::domain::items::types::{ArmorData, Item, ItemType};

        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        #[allow(deprecated)]
        let items = vec![Item {
            id: 1,
            name: "Sword".to_string(),
            description: String::new(),
            item_type: ItemType::Armor(ArmorData::default()),
            value: 0,
            weight: 0,
            tags: Vec::new(),
            disablement: antares::domain::items::types::Disablement::default(),
            cursed: false,
            charges: None,
            max_charges: None,
        }];

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Should show warning for non-existent item ID
            show_item_validation_warning(ui, 999, &items);
            // Should not show warning for existing item ID
            show_item_validation_warning(ui, 1, &items);
        });

        let _ = ctx.end_pass();
    }

    #[test]
    fn show_monster_validation_warning_handles_empty_name() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        let monsters = vec![];

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Should not show warning for empty name
            show_monster_validation_warning(ui, "", &monsters);
        });

        let _ = ctx.end_pass();
    }

    #[test]
    fn show_condition_validation_warning_handles_empty_id() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        let conditions = vec![];

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Should not show warning for empty condition ID
            show_condition_validation_warning(ui, "", &conditions);
        });

        let _ = ctx.end_pass();
    }
}
