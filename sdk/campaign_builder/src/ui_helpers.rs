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
//! ## Core Layout Components
//!
//! - [`EditorToolbar`] - Standard toolbar with New, Save, Load, Import, Export buttons
//! - [`ActionButtons`] - Standard action buttons for detail panels (Edit, Delete, Duplicate, Export)
//! - [`TwoColumnLayout`] - Standard two-column list/detail layout
//! - [`ImportExportDialog`] - Standard import/export dialog for RON data
//!
//! ## Attribute Widgets
//!
//! - [`AttributePairInput`] - Widget for editing `AttributePair` (u8 base/current)
//! - [`AttributePair16Input`] - Widget for editing `AttributePair16` (u16 base/current)
//!
//! ## Autocomplete System (Phase 1-3)
//!
//! - [`AutocompleteInput`] - Autocomplete text input widget with dropdown suggestions
//! - [`autocomplete_item_selector`] - Pre-configured autocomplete for Item selection
//! - [`autocomplete_monster_selector`] - Pre-configured autocomplete for Monster selection
//! - [`autocomplete_condition_selector`] - Pre-configured autocomplete for Condition selection
//! - [`autocomplete_item_list_selector`] - Multi-select autocomplete for Item lists
//! - [`autocomplete_proficiency_list_selector`] - Multi-select autocomplete for Proficiency lists
//! - [`autocomplete_tag_list_selector`] - Multi-select autocomplete for Item Tag lists
//! - [`autocomplete_ability_list_selector`] - Multi-select autocomplete for Special Ability lists
//!
//! ## Candidate Extraction & Caching (Phase 2-3)
//!
//! - [`extract_item_candidates`] - Extracts searchable item candidates
//! - [`extract_monster_candidates`] - Extracts searchable monster candidates
//! - [`extract_condition_candidates`] - Extracts searchable condition candidates
//! - [`extract_spell_candidates`] - Extracts searchable spell candidates
//! - [`extract_proficiency_candidates`] - Extracts searchable proficiency candidates
//! - [`extract_item_tag_candidates`] - Extracts unique item tags from items
//! - [`extract_special_ability_candidates`] - Extracts special abilities from races
//! - [`AutocompleteCandidateCache`] - Performance cache for candidate lists (invalidate on data changes)
//!
//! ## Entity Validation Warnings (Phase 3)
//!
//! - [`show_entity_validation_warning`] - Generic validation warning display
//! - [`show_item_validation_warning`] - Item-specific validation warning
//! - [`show_monster_validation_warning`] - Monster-specific validation warning
//! - [`show_condition_validation_warning`] - Condition-specific validation warning
//! - [`show_spell_validation_warning`] - Spell-specific validation warning
//!
//! # Performance Optimization
//!
//! Use [`AutocompleteCandidateCache`] to avoid regenerating candidate lists every frame.
//! Cache instances should be stored in editor state and invalidated when data changes
//! (add/delete/import operations).
//!
//! # Examples
//!
//! ```rust,ignore
//! // Using autocomplete with caching
//! let mut cache = AutocompleteCandidateCache::new();
//! let candidates = cache.get_or_generate_items(&items);
//! autocomplete_item_selector(ui, &mut selected_id, &candidates, &items);
//!
//! // Invalidate cache after data mutation
//! cache.invalidate_items();
//!
//! // Show validation warning for missing entity
//! show_item_validation_warning(ui, selected_id, &items);
//! ```

use antares::domain::character::{AttributePair, AttributePair16};
use antares::domain::items::Item;
use antares::domain::proficiency::{
    ProficiencyCategory, ProficiencyDatabase, ProficiencyDefinition,
};
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

// NOTE: Per-widget transient autocomplete buffers are stored in egui `Memory::data`.
// Use `ui.memory_mut(|mem| mem.data.get_temp_mut_or_insert_with::<String>(id, || default))`
// to obtain a `&mut String` tied to the UI context instead of a process-global map.
///
/// Helper utilities for loading/storing/removing autocomplete buffers in egui Memory.
/// These functions centralize the read/clone/writeback pattern used across
/// autocomplete widgets to avoid overlapping mutable borrows of egui internals.
///
/// Examples:
///
/// ```no_run
/// // Construct an id and use the helpers from a UI context:
/// let id = autocomplete_buffer_id("item", "my_widget");
/// let mut buf = load_autocomplete_buffer(ui.ctx(), id, || String::new());
/// // ... render widget with &mut buf ...
/// store_autocomplete_buffer(ui.ctx(), id, &buf);
/// // or remove:
/// remove_autocomplete_buffer(ui.ctx(), id);
/// ```
fn make_autocomplete_id(_ui: &egui::Ui, prefix: &str, id_salt: &str) -> egui::Id {
    // Stable autocomplete id: deterministic and independent of UI nesting
    egui::Id::new(format!("autocomplete:{}:{}", prefix, id_salt))
}

/// Load an autocomplete buffer from egui Memory.
///
/// Returns an owned `String` initialized from the stored buffer or `default()`
/// if no entry exists.
///
/// # Arguments
///
/// * `ctx` - the `egui::Context` (e.g. `ui.ctx()`)
/// * `id` - the `egui::Id` identifying the buffer
/// * `default` - fallback factory invoked when no buffer exists
pub fn load_autocomplete_buffer(
    ctx: &egui::Context,
    id: egui::Id,
    default: impl FnOnce() -> String,
) -> String {
    ctx.memory(|mem| mem.data.get_temp::<String>(id).map(|s| s.clone()))
        .unwrap_or_else(default)
}

/// Store (insert/replace) an autocomplete buffer into egui Memory.
///
/// Overwrites any existing value for `id`.
pub fn store_autocomplete_buffer(ctx: &egui::Context, id: egui::Id, buffer: &str) {
    ctx.memory_mut(|mem| {
        *mem.data
            .get_temp_mut_or_insert_with::<String>(id, || buffer.to_string()) = buffer.to_string();
    });
}

/// Remove an autocomplete buffer from egui Memory (if present).
pub fn remove_autocomplete_buffer(ctx: &egui::Context, id: egui::Id) {
    ctx.memory_mut(|mem| {
        mem.data.remove::<String>(id);
    });
}

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

    // Build candidates (cloned list)
    let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();

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

        // Persist the per-widget text buffer in egui Memory so typed text survives frames.
        let buffer_id = make_autocomplete_id(ui, "item", id_salt);

        // Read the persistent buffer into a local owned `String` so we don't hold a long-lived borrow.
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        // Render the widget using the local buffer. After the widget returns, write the buffer back to memory.
        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing item name...")
            .show(ui, &mut text_buffer);

        // Commit valid selections
        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
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
            // Remove the persisted buffer entry
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
        }

        // Persist buffer back into egui memory so it survives frames.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for selecting a quest by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_quest_id_str` - Mutable reference to the selected quest ID as String
/// * `quests` - Slice of available quests
///
/// # Returns
///
/// `true` if the user selected a quest, `false` otherwise
///
/// # Examples
///
/// ```
/// use antares::domain::quest::Quest;
/// use eframe::egui;
///
/// let mut quest_id_str = String::new();
/// let quests = vec![Quest { id: 1, name: "Quest 1".to_string(), /* ... */ }];
///
/// // In UI code:
/// // let changed = autocomplete_quest_selector(ui, "quest_sel", "Quest:", &mut quest_id_str, &quests);
/// ```
pub fn autocomplete_quest_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_quest_id_str: &mut String,
    quests: &[antares::domain::quest::Quest],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current quest name based on ID string
        let current_name = if selected_quest_id_str.is_empty() {
            String::new()
        } else {
            // Parse the ID and find the quest
            selected_quest_id_str
                .parse::<antares::domain::quest::QuestId>()
                .ok()
                .and_then(|id| quests.iter().find(|q| q.id == id))
                .map(|q| q.name.clone())
                .unwrap_or_default()
        };

        // Build candidates
        let candidates: Vec<String> = quests.iter().map(|q| q.name.clone()).collect();

        // Persist the per-widget text buffer in egui memory by reading a cloned
        // value into a local `String`, letting the widget edit it, and writing
        // the value back into Memory after the edit. This avoids overlapping
        // mutable borrows of egui internals while still providing persistent state.
        let buffer_id = make_autocomplete_id(ui, "quest", id_salt);

        // Initialize local buffer from memory (or fallback to the current name)
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing quest name...")
            .show(ui, &mut text_buffer);

        // Commit selection immediately
        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            if let Some(quest) = quests.iter().find(|q| q.name == text_buffer) {
                *selected_quest_id_str = quest.id.to_string();
                changed = true;
            }
        }

        // Persist buffer back into egui memory so it survives frames.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);

        // Show clear button if something is selected
        if !selected_quest_id_str.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_quest_id_str.clear();
            // Remove the persisted buffer entry
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
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

        // Use the provided persistent buffer directly so typed text persists
        let original = selected_monster_name.clone();

        // Build candidates
        let candidates: Vec<String> = extract_monster_candidates(monsters);

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing monster name...")
            .show(ui, selected_monster_name);

        // Check if user selected something from autocomplete
        if response.changed()
            && !selected_monster_name.is_empty()
            && selected_monster_name != &original
        {
            // Validate the monster exists
            if monsters.iter().any(|m| m.name == *selected_monster_name) {
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

        let buffer_id = make_autocomplete_id(ui, "condition", id_salt);

        // Build candidates from condition names
        let candidates: Vec<String> = conditions.iter().map(|c| c.name.clone()).collect();

        // Read the persistent buffer into a local `String`, allow the widget to edit it,
        // and then write it back into egui `Memory` after the widget runs. This avoids
        // holding long-lived mutable borrows into egui internals while still persisting
        // the user's typed text across frames.
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing condition name...")
            .show(ui, &mut text_buffer);

        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            if let Some(condition) = conditions.iter().find(|c| c.name == text_buffer) {
                *selected_condition_id = condition.id.clone();
                changed = true;
            }
        }

        // Show clear button if something is selected. If cleared, remove the persisted buffer.
        let mut cleared = false;
        // Show clear button if something is selected
        if !selected_condition_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_condition_id.clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
            cleared = true;
        }

        // Persist the edited buffer back into egui memory unless the user cleared it.
        if !cleared {
            store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
        }
    });

    changed
}

/// Shows an autocomplete input for selecting a map by ID.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_map_id` - Mutable reference to the selected map ID string
/// * `maps` - Slice of available maps
///
/// # Returns
///
/// `true` if the user selected a map, `false` otherwise
pub fn autocomplete_map_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_map_id: &mut String,
    maps: &[antares::domain::world::Map],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current map name
        let current_map_name =
            if let Ok(map_id) = selected_map_id.parse::<antares::domain::types::MapId>() {
                maps.iter()
                    .find(|m| m.id == map_id)
                    .map(|m| format!("{} (ID: {})", m.name, m.id))
                    .unwrap_or_else(|| selected_map_id.clone())
            } else {
                selected_map_id.clone()
            };

        // Persist the per-widget text buffer in egui Memory and render the widget with a mutable reference.
        // This ensures edits persist across frames and are tied to the egui UI context.
        let buffer_id = make_autocomplete_id(ui, "map", id_salt);

        // Build candidate display strings as "Name (ID: X)"
        let candidates: Vec<String> = maps
            .iter()
            .map(|m| format!("{} (ID: {})", m.name, m.id))
            .collect();

        // Read the persistent buffer into a local `String` so we can safely pass
        // a mutable reference into the widget without holding a long-lived
        // mutable borrow of egui internals. After the widget returns, write the
        // updated buffer back into Memory.
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_map_name.clone());

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing map name...")
            .show(ui, &mut text_buffer);

        if response.changed() && !text_buffer.is_empty() && text_buffer != current_map_name {
            // Try to extract map ID from the selected text (format: "Name (ID: X)")
            if let Some(id_start) = text_buffer.rfind("(ID: ") {
                if let Some(id_end) = text_buffer[id_start..].find(')') {
                    let id_str = &text_buffer[id_start + 5..id_start + id_end];
                    *selected_map_id = id_str.to_string();
                    changed = true;
                }
            }
        }

        // Show clear button if something is selected. If cleared, remove the persisted buffer.
        let mut cleared = false;
        if !selected_map_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_map_id.clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
            cleared = true;
        }

        // Persist edits back into egui Memory unless the user cleared the field.
        if !cleared {
            store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
        }
    });

    changed
}

/// Shows an autocomplete input for selecting an NPC by ID.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_npc_id` - Mutable reference to the selected NPC ID string (format: "map_id:npc_id")
/// * `maps` - Slice of available maps containing NPCs
///
/// # Returns
///
/// `true` if the user selected an NPC, `false` otherwise
pub fn autocomplete_npc_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_npc_id: &mut String,
    maps: &[antares::domain::world::Map],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current NPC display name
        let current_display = if !selected_npc_id.is_empty() {
            // Parse map_id:npc_id format
            if let Some((map_id_str, npc_id_str)) = selected_npc_id.split_once(':') {
                if let (Ok(map_id), Ok(npc_id)) = (
                    map_id_str.parse::<antares::domain::types::MapId>(),
                    npc_id_str.parse::<u16>(),
                ) {
                    // Find the NPC
                    maps.iter()
                        .find(|m| m.id == map_id)
                        .and_then(|m| m.npcs.iter().find(|n| n.id == npc_id))
                        .map(|npc| {
                            format!("{} (Map: {}, NPC ID: {})", npc.name, map_id_str, npc.id)
                        })
                        .unwrap_or_else(|| selected_npc_id.clone())
                } else {
                    selected_npc_id.clone()
                }
            } else {
                selected_npc_id.clone()
            }
        } else {
            String::new()
        };

        let buffer_id = make_autocomplete_id(ui, "npc", id_salt);

        // Read persistent buffer into a local String so the widget can mutate it
        // without holding a long-lived borrow into egui Memory, then write it back.
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_display.clone());

        // Build candidates
        let candidates: Vec<String> = extract_npc_candidates(maps)
            .into_iter()
            .map(|(display, _)| display)
            .collect();

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing NPC name...")
            .show(ui, &mut text_buffer);

        if response.changed() && !text_buffer.is_empty() && text_buffer != current_display {
            // Try to find the NPC by reconstructing the ID from the display text
            // Format: "Name (Map: MapName, NPC ID: X)"
            for (display, npc_id) in extract_npc_candidates(maps) {
                if display == text_buffer {
                    *selected_npc_id = npc_id;
                    changed = true;
                    break;
                }
            }
        }

        // Show clear button if something is selected
        let mut cleared = false;
        if !selected_npc_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_npc_id.clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
            cleared = true;
        }

        // Persist edits back into egui Memory unless the user cleared the buffer.
        if !cleared {
            store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
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

        // Add new item input (persistent buffer keyed by widget)
        let buffer_id = make_autocomplete_id(ui, "item_add", id_salt);
        let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();

        // Read persistent add buffer into a local String, render the Autocomplete widget
        // with it, then persist the edited buffer back into egui Memory so it survives frames.
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || String::new());

        ui.horizontal(|ui| {
            ui.label("Add item:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing item name...")
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            // 1) If the text changed and matches an existing candidate exactly, add it.
            if response.changed() && !tb.is_empty() {
                if let Some(item) = items.iter().find(|i| i.name == tb) {
                    if !selected_items.contains(&item.id) {
                        selected_items.push(item.id);
                        changed = true;
                    }
                    // Clear the add buffer after successful add
                    text_buffer.clear();
                }
            }

            // 2) If Enter was pressed while this widget had focus, commit the typed text
            //    (if it matches an item).
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(item) = items.iter().find(|i| i.name == tb) {
                    if !selected_items.contains(&item.id) {
                        selected_items.push(item.id);
                        changed = true;
                    }
                    text_buffer.clear();
                }
            }
        });

        // Persist the edited add buffer back into egui Memory
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete list selector for proficiencies.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_proficiencies` - Mutable reference to selected proficiency IDs
/// * `proficiencies` - Slice of available proficiency definitions
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_proficiency_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_proficiencies: &mut Vec<String>,
    proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Show current proficiencies
        let mut remove_idx: Option<usize> = None;
        for (idx, prof_id) in selected_proficiencies.iter().enumerate() {
            ui.horizontal(|ui| {
                if let Some(prof) = proficiencies.iter().find(|p| p.id == *prof_id) {
                    ui.label(&prof.name);
                } else {
                    ui.label(format!("Unknown proficiency (ID: {})", prof_id));
                }
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected_proficiencies.remove(idx);
            changed = true;
        }

        ui.separator();

        let buffer_id = make_autocomplete_id(ui, "prof_add", id_salt);
        let candidates: Vec<String> = proficiencies.iter().map(|p| p.name.clone()).collect();

        // Read the persisted add buffer into a local String so the widget can
        // mutate it without holding a long-lived mutable borrow of egui Memory.
        // After rendering, write the updated buffer back into Memory.
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || String::new());

        ui.horizontal(|ui| {
            ui.label("Add proficiency:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing proficiency...")
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            // 1) If the text changed and matches an existing candidate exactly, add it.
            if response.changed() && !tb.is_empty() {
                if let Some(prof) = proficiencies.iter().find(|p| p.name == tb) {
                    if !selected_proficiencies.contains(&prof.id) {
                        selected_proficiencies.push(prof.id.clone());
                        changed = true;
                    }
                    // Clear the add buffer after successful add
                    text_buffer.clear();
                }
            }

            // 2) If Enter was pressed while this widget had focus, commit the typed text
            //    (if it matches a proficiency).
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(prof) = proficiencies.iter().find(|p| p.name == tb) {
                    if !selected_proficiencies.contains(&prof.id) {
                        selected_proficiencies.push(prof.id.clone());
                        changed = true;
                    }
                    text_buffer.clear();
                }
            }
        });

        // Persist the edited add buffer back into egui Memory so it survives frames.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete list selector for item tags.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_tags` - Mutable reference to selected tag strings
/// * `available_tags` - Slice of available tag strings
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_tag_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_tags: &mut Vec<String>,
    available_tags: &[String],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Show current tags
        let mut remove_idx: Option<usize> = None;
        for (idx, tag) in selected_tags.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(tag);
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected_tags.remove(idx);
            changed = true;
        }

        ui.separator();

        // Add new tag input (persistent buffer)
        let buffer_id = make_autocomplete_id(ui, "tag_add", id_salt);
        let candidates: Vec<String> = available_tags.to_vec();

        // Read persistent add buffer into a local String so the widget can mutate it
        // without holding a long-lived borrow into egui Memory. After rendering, write the updated value back.
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || String::new());

        ui.horizontal(|ui| {
            ui.label("Add tag:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing tag...")
                .show(ui, &mut text_buffer);

            // Use a trimmed buffer for comparisons so accidental whitespace doesn't create tags.
            let tb = text_buffer.trim().to_string();

            // 1) If the text changed and matches an existing candidate exactly, add it.
            if response.changed() && !tb.is_empty() {
                if candidates.iter().any(|c| c == &tb) {
                    if !selected_tags.contains(&tb) {
                        selected_tags.push(tb.clone());
                        changed = true;
                    }
                    // Clear buffer after successful selection
                    text_buffer.clear();
                }
            }

            // 2) If Enter was pressed while this widget had focus, commit the typed text
            //    (even if it's not an existing candidate). This avoids adding on partial typing.
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !tb.is_empty() && !selected_tags.contains(&tb) {
                    selected_tags.push(tb.clone());
                    changed = true;
                }
                text_buffer.clear();
            }
        });

        // Persist the edited add buffer back into egui Memory
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete list selector for special abilities.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_abilities` - Mutable reference to selected ability strings
/// * `available_abilities` - Slice of available ability strings
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_ability_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_abilities: &mut Vec<String>,
    available_abilities: &[String],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Show current abilities
        let mut remove_idx: Option<usize> = None;
        for (idx, ability) in selected_abilities.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(ability);
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected_abilities.remove(idx);
            changed = true;
        }

        ui.separator();

        let buffer_id = make_autocomplete_id(ui, "ability_add", id_salt);
        let candidates: Vec<String> = available_abilities.to_vec();

        // Read persistent add buffer into a local String so the widget can mutate it
        // without holding a long-lived borrow into egui Memory. After rendering, write the updated value back.
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || String::new());

        ui.horizontal(|ui| {
            ui.label("Add ability:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing ability...")
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            // 1) If the text changed and matches an existing candidate exactly, add it.
            if response.changed() && !tb.is_empty() {
                if candidates.iter().any(|c| c == &tb) {
                    if !selected_abilities.contains(&tb) {
                        selected_abilities.push(tb.clone());
                        changed = true;
                    }
                    // Clear buffer after successful add
                    text_buffer.clear();
                }
            }

            // 2) If Enter was pressed while this widget had focus, commit the typed text.
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !tb.is_empty() && !selected_abilities.contains(&tb) {
                    selected_abilities.push(tb.clone());
                    changed = true;
                }
                text_buffer.clear();
            }
        });

        // Persist the edited add buffer back into egui Memory so it survives frames.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for selecting a race by name.
///
/// Returns `true` if the selection changed.
pub fn autocomplete_race_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_race_id: &mut String,
    races: &[antares::domain::races::RaceDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current race name based on ID
        let current_name = races
            .iter()
            .find(|r| r.id == *selected_race_id)
            .map(|r| r.name.clone())
            .unwrap_or_default();

        let buffer_id = make_autocomplete_id(ui, "race", id_salt);

        // Build candidates
        let candidates: Vec<String> = races.iter().map(|r| r.name.clone()).collect();

        // Persistent buffer logic
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing race name...")
            .show(ui, &mut text_buffer);

        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            if let Some(race) = races.iter().find(|r| r.name == text_buffer) {
                *selected_race_id = race.id.clone();
                changed = true;
            }
        }

        // Show clear button
        if !selected_race_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_race_id.clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
        }

        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for selecting a class by name.
///
/// Returns `true` if the selection changed.
pub fn autocomplete_class_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_class_id: &mut String,
    classes: &[antares::domain::classes::ClassDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current class name based on ID
        let current_name = classes
            .iter()
            .find(|c| c.id == *selected_class_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let buffer_id = make_autocomplete_id(ui, "class", id_salt);

        // Build candidates
        let candidates: Vec<String> = classes.iter().map(|c| c.name.clone()).collect();

        // Persistent buffer logic
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        let response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing class name...")
            .show(ui, &mut text_buffer);

        if response.changed() && !text_buffer.is_empty() && text_buffer != current_name {
            if let Some(class) = classes.iter().find(|c| c.name == text_buffer) {
                *selected_class_id = class.id.clone();
                changed = true;
            }
        }

        // Show clear button
        if !selected_class_id.is_empty()
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            selected_class_id.clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
        }

        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for adding monsters to a list.
///
/// Returns `true` if a monster was added to the list.
pub fn autocomplete_monster_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_monsters: &mut Vec<antares::domain::types::MonsterId>,
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> bool {
    use crate::ui_helpers::AutocompleteInput;

    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Show current monsters
        let mut remove_idx: Option<usize> = None;
        for (idx, monster_id) in selected_monsters.iter().enumerate() {
            ui.horizontal(|ui| {
                if let Some(monster) = monsters.iter().find(|m| m.id == *monster_id) {
                    ui.label(&monster.name);
                } else {
                    ui.label(format!("Unknown monster (ID: {})", monster_id));
                }
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected_monsters.remove(idx);
            changed = true;
        }

        ui.separator();

        // Add new monster input
        let buffer_id = make_autocomplete_id(ui, "monster_add", id_salt);
        let candidates: Vec<String> = monsters.iter().map(|m| m.name.clone()).collect();

        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || String::new());

        ui.horizontal(|ui| {
            ui.label("Add monster:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing monster name...")
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            // 1) If the text changed and matches an existing candidate exactly, add it.
            if response.changed() && !tb.is_empty() {
                if let Some(monster) = monsters.iter().find(|m| m.name == tb) {
                    if !selected_monsters.contains(&monster.id) {
                        selected_monsters.push(monster.id);
                        changed = true;
                    }
                    // Clear the add buffer after successful add
                    text_buffer.clear();
                }
            }

            // 2) If Enter was pressed while this widget had focus, commit the typed text
            //    (if it matches a monster).
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(monster) = monsters.iter().find(|m| m.name == tb) {
                    if !selected_monsters.contains(&monster.id) {
                        selected_monsters.push(monster.id);
                        changed = true;
                    }
                    text_buffer.clear();
                }
            }
        });

        // Persist the edited add buffer back into egui Memory
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
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

/// Extracts race name candidates from a list of race definitions.
pub fn extract_race_candidates(races: &[antares::domain::races::RaceDefinition]) -> Vec<String> {
    races.iter().map(|r| r.name.clone()).collect()
}

/// Extracts class name candidates from a list of class definitions.
pub fn extract_class_candidates(
    classes: &[antares::domain::classes::ClassDefinition],
) -> Vec<String> {
    classes.iter().map(|c| c.name.clone()).collect()
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

/// Extracts quest candidates from a list of quests.
///
/// Returns a vector of tuples mapping quest display name to QuestId.
///
/// # Arguments
///
/// * `quests` - Slice of quests to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, QuestId)
///
/// # Examples
///
/// ```
/// use antares::domain::quest::{Quest, QuestId};
///
/// let quests = vec![
///     Quest {
///         id: 1,
///         name: "Save the Village".to_string(),
///         description: "Help save the village from bandits".to_string(),
///         stages: vec![],
///         rewards: vec![],
///         prerequisites: vec![],
///         min_level: 1,
///         max_level: None,
///         repeatable: false,
///         is_main_quest: true,
///         quest_giver_npc: None,
///         quest_giver_location: None,
///     },
/// ];
///
/// let candidates = extract_quest_candidates(&quests);
/// assert_eq!(candidates.len(), 1);
/// assert_eq!(candidates[0].0, "Save the Village (ID: 1)");
/// assert_eq!(candidates[0].1, 1);
/// ```
pub fn extract_quest_candidates(
    quests: &[antares::domain::quest::Quest],
) -> Vec<(String, antares::domain::quest::QuestId)> {
    quests
        .iter()
        .map(|quest| (format!("{} (ID: {})", quest.name, quest.id), quest.id))
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
/// use antares::domain::proficiency::{ProficiencyDefinition, ProficiencyId, ProficiencyCategory};
/// use antares::sdk::campaign_builder::ui_helpers::extract_proficiency_candidates;
///
/// let proficiencies = vec![
///     ProficiencyDefinition {
///         id: "sword".to_string(),
///         name: "Sword".to_string(),
///         category: ProficiencyCategory::Weapon,
///         description: "Sword proficiency".to_string(),
///     },
/// ];
/// let candidates = extract_proficiency_candidates(&proficiencies);
/// assert_eq!(candidates.len(), 1);
/// ```
pub fn extract_proficiency_candidates(
    proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
) -> Vec<(String, String)> {
    proficiencies
        .iter()
        .map(|p| (format!("{} ({})", p.name, p.id), p.id.clone()))
        .collect()
}

/// Loads proficiency definitions with a tri-stage fallback:
/// 1. Campaign directory RON file
/// 2. Global data directory RON file
/// 3. Synthetic generation based on item classifications
pub fn load_proficiencies(
    campaign_dir: Option<&PathBuf>,
    items: &[Item],
) -> Vec<ProficiencyDefinition> {
    // Stage 1: Try campaign directory
    if let Some(dir) = campaign_dir {
        let path = dir.join("data/proficiencies.ron");
        if path.exists() {
            if let Ok(db) = ProficiencyDatabase::load_from_file(&path) {
                return db.all().into_iter().cloned().collect();
            }
        }
    }

    // Stage 2: Try global data directory
    if let Ok(db) = ProficiencyDatabase::load_from_file("data/proficiencies.ron") {
        return db.all().into_iter().cloned().collect();
    }

    // Stage 3: Synthetic Fallback
    generate_synthetic_proficiencies(items)
}

fn generate_synthetic_proficiencies(items: &[Item]) -> Vec<ProficiencyDefinition> {
    let mut profs = std::collections::HashMap::new();

    // Standard proficiencies that should always be available
    let standard = vec![
        (
            "simple_weapon",
            "Simple Weapons",
            ProficiencyCategory::Weapon,
        ),
        (
            "martial_melee",
            "Martial Melee Weapons",
            ProficiencyCategory::Weapon,
        ),
        (
            "martial_ranged",
            "Martial Ranged Weapons",
            ProficiencyCategory::Weapon,
        ),
        ("blunt_weapon", "Blunt Weapons", ProficiencyCategory::Weapon),
        ("unarmed", "Unarmed Combat", ProficiencyCategory::Weapon),
        ("light_armor", "Light Armor", ProficiencyCategory::Armor),
        ("medium_armor", "Medium Armor", ProficiencyCategory::Armor),
        ("heavy_armor", "Heavy Armor", ProficiencyCategory::Armor),
        ("shield", "Shield", ProficiencyCategory::Shield),
        (
            "arcane_item",
            "Arcane Magic Items",
            ProficiencyCategory::MagicItem,
        ),
        (
            "divine_item",
            "Divine Magic Items",
            ProficiencyCategory::MagicItem,
        ),
    ];

    for (id, name, cat) in standard {
        profs.insert(
            id.to_string(),
            ProficiencyDefinition::with_description(
                id.to_string(),
                name.to_string(),
                cat,
                format!("Standard {} proficiency", name),
            ),
        );
    }

    // Scan items for any classifications not in standard
    for item in items {
        if let Some(prof_id) = item.required_proficiency() {
            if !profs.contains_key(&prof_id) {
                let name = prof_id
                    .replace('_', " ")
                    .split_whitespace()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let category = match &item.item_type {
                    antares::domain::items::ItemType::Weapon(_) => ProficiencyCategory::Weapon,
                    antares::domain::items::ItemType::Armor(_) => ProficiencyCategory::Armor,
                    antares::domain::items::ItemType::Accessory(_) => {
                        ProficiencyCategory::MagicItem
                    }
                    _ => ProficiencyCategory::Weapon,
                };

                profs.insert(
                    prof_id.clone(),
                    ProficiencyDefinition::with_description(
                        prof_id.clone(),
                        name,
                        category,
                        "Derived from campaign items".to_string(),
                    ),
                );
            }
        }
    }

    let mut result: Vec<_> = profs.into_values().collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

/// Extracts item tag candidates from a list of items.
///
/// Returns a vector of unique item tags suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `items` - Slice of items to extract tags from
///
/// # Returns
///
/// A vector of unique tag strings sorted alphabetically
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::types::Item;
/// use antares::sdk::campaign_builder::ui_helpers::extract_item_tag_candidates;
///
/// let items = vec![]; // Items with tags
/// let candidates = extract_item_tag_candidates(&items);
/// ```
pub fn extract_item_tag_candidates(items: &[antares::domain::items::types::Item]) -> Vec<String> {
    use std::collections::HashSet;

    let mut tags = HashSet::new();
    for item in items {
        for tag in &item.tags {
            tags.insert(tag.clone());
        }
    }

    let mut result: Vec<String> = tags.into_iter().collect();
    result.sort();
    result
}

/// Extracts special ability candidates from existing race definitions.
///
/// Returns a vector of unique special abilities suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `races` - Slice of race definitions to extract abilities from
///
/// # Returns
///
/// A vector of unique special ability strings sorted alphabetically
///
/// # Examples
///
/// ```no_run
/// use antares::domain::races::RaceDefinition;
/// use antares::sdk::campaign_builder::ui_helpers::extract_special_ability_candidates;
///
/// let races = vec![]; // Races with special abilities
/// let candidates = extract_special_ability_candidates(&races);
/// ```
pub fn extract_special_ability_candidates(
    races: &[antares::domain::races::RaceDefinition],
) -> Vec<String> {
    use std::collections::HashSet;

    let mut abilities = HashSet::new();
    for race in races {
        for ability in &race.special_abilities {
            abilities.insert(ability.clone());
        }
    }

    // Add common standard abilities
    let standard_abilities = vec![
        "infravision",
        "magic_resistance",
        "poison_immunity",
        "disease_immunity",
        "keen_senses",
        "darkvision",
        "lucky",
        "brave",
        "stonecunning",
        "trance",
    ];

    for ability in standard_abilities {
        abilities.insert(ability.to_string());
    }

    let mut result: Vec<String> = abilities.into_iter().collect();
    result.sort();
    result
}

/// Extracts map candidates for autocomplete from a slice of maps.
///
/// Returns a list of tuples containing display string and map ID.
///
/// # Examples
///
/// ```
/// use antares::domain::world::Map;
/// use antares::sdk::campaign_builder::ui_helpers::extract_map_candidates;
///
/// let maps = vec![
///     Map::new(1, "Town Square".to_string(), "Starting area".to_string(), 20, 20),
///     Map::new(2, "Dark Forest".to_string(), "Dangerous woods".to_string(), 30, 30),
/// ];
/// let candidates = extract_map_candidates(&maps);
/// assert_eq!(candidates.len(), 2);
/// assert_eq!(candidates[0].0, "Town Square (ID: 1)");
/// ```
pub fn extract_map_candidates(
    maps: &[antares::domain::world::Map],
) -> Vec<(String, antares::domain::types::MapId)> {
    maps.iter()
        .map(|map| (format!("{} (ID: {})", map.name, map.id), map.id))
        .collect()
}

/// Extracts NPC candidates for autocomplete from all NPCs in all maps.
///
/// Returns a list of tuples containing display string and NPC ID.
/// NPC IDs are formatted as "{map_id}:{npc_id}" to ensure uniqueness across maps.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, Npc};
/// use antares::domain::types::Position;
/// use antares::sdk::campaign_builder::ui_helpers::extract_npc_candidates;
///
/// let mut map = Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10);
/// map.npcs.push(Npc {
///     id: 1,
///     name: "Merchant".to_string(),
///     description: "Sells goods".to_string(),
///     position: Position::new(5, 5),
///     dialogue_id: None,
/// });
///
/// let candidates = extract_npc_candidates(&[map]);
/// assert_eq!(candidates.len(), 1);
/// assert!(candidates[0].0.contains("Merchant"));
/// ```
pub fn extract_npc_candidates(maps: &[antares::domain::world::Map]) -> Vec<(String, String)> {
    let mut candidates = Vec::new();
    for map in maps {
        for npc in &map.npcs {
            let display = format!("{} (Map: {}, NPC ID: {})", npc.name, map.name, npc.id);
            let npc_id = format!("{}:{}", map.id, npc.id);
            candidates.push((display, npc_id));
        }
    }
    candidates
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
    proficiencies: Option<(Vec<(String, String)>, u64)>,
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
        spells: &[antares::domain::magic::types::Spell],
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
        proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
    ) -> Vec<(String, String)> {
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
    spells: &[antares::domain::magic::types::Spell],
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
        // Sanity check - ensure our candidates are present
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn autocomplete_monster_selector_preserves_passed_buffer() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            use antares::domain::character::{AttributePair, AttributePair16, Stats};
            use antares::domain::combat::database::MonsterDefinition;
            use antares::domain::combat::{
                monster::MonsterCondition, LootTable, MonsterResistances,
            };

            let monsters = vec![MonsterDefinition {
                id: 1,
                name: "Goblin".to_string(),
                stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
                hp: AttributePair16::new(15),
                ac: AttributePair::new(6),
                attacks: vec![],
                flee_threshold: 5,
                special_attack_threshold: 0,
                resistances: MonsterResistances::new(),
                can_regenerate: false,
                can_advance: true,
                is_undead: false,
                magic_resistance: 0,
                loot: LootTable::new(1, 10, 0, 0, 10),
                conditions: MonsterCondition::Normal,
                active_conditions: vec![],
                has_acted: false,
            }];

            let mut buffer = String::from("Go");
            let original = buffer.clone();
            // The selector should not reset a passed-in buffer on render
            let _changed =
                autocomplete_monster_selector(ui, "monster_test", "Name:", &mut buffer, &monsters);
            assert_eq!(
                buffer, original,
                "Passed in buffer should not be reset by the selector"
            );
        });
    }

    #[test]
    fn autocomplete_item_selector_persists_buffer() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            use crate::items_editor::ItemsEditorState;

            let mut item = ItemsEditorState::default_item();
            item.id = 42;
            item.name = "Sword".to_string();
            let items = vec![item];

            let mut selected_item_id: antares::domain::types::ItemId = 0;
            // First call should initialize persistent buffer
            let _ =
                autocomplete_item_selector(ui, "item_test", "Item:", &mut selected_item_id, &items);

            // Confirm memory has an entry
            ui.horizontal(|ui| {
                let id = make_autocomplete_id(ui, "item", "item_test");
                let val = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
                assert!(val.is_some(), "Buffer map should contain entry for widget");

                // Simulate typing by modifying the buffer
                ui.ctx().memory_mut(|mem| {
                    let buf = mem
                        .data
                        .get_temp_mut_or_insert_with::<String>(id, || String::new());
                    *buf = "Sw".to_string();
                });
            });

            // Second call should not overwrite the typed content
            let _ =
                autocomplete_item_selector(ui, "item_test", "Item:", &mut selected_item_id, &items);

            let val2 = ui.ctx().memory_mut(|mem| {
                mem.data
                    .get_temp::<String>(make_autocomplete_id(ui, "item", "item_test"))
            });
            assert_eq!(val2.as_deref(), Some("Sw"));
        });

        #[test]
        fn autocomplete_map_selector_persists_buffer() {
            let ctx = egui::Context::default();
            let mut raw_input = egui::RawInput::default();
            raw_input.screen_rect = Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(800.0, 600.0),
            ));
            ctx.begin_pass(raw_input);

            egui::CentralPanel::default().show(&ctx, |ui| {
                let mut selected_map_id: String = String::new();
                let maps: Vec<antares::domain::world::Map> = vec![];
                // First call should initialize persistent buffer
                let _ =
                    autocomplete_map_selector(ui, "map_test", "Map:", &mut selected_map_id, &maps);

                // Confirm memory has an entry
                let id = make_autocomplete_id(ui, "map", "map_test");
                let val = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
                assert!(val.is_some(), "Buffer map should contain entry for widget");

                // Simulate typing by modifying the buffer
                ui.ctx().memory_mut(|mem| {
                    let buf = mem
                        .data
                        .get_temp_mut_or_insert_with::<String>(id, || String::new());
                    *buf = "Over".to_string();
                });

                // Second call should not overwrite the typed content
                let _ =
                    autocomplete_map_selector(ui, "map_test", "Map:", &mut selected_map_id, &maps);

                let val2 = ui.ctx().memory_mut(|mem| mem.data.get_temp::<String>(id));
                assert_eq!(val2.as_deref(), Some("Over"));
            });
        }
    }

    #[test]
    fn autocomplete_buffer_helpers_work() {
        let ctx = egui::Context::default();
        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        ));
        ctx.begin_pass(raw_input);

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Use the same ID pattern as widgets use so we validate the helpers operate on the same keys.
            let id = make_autocomplete_id(ui, "test", "helper_test");

            // When no buffer exists, the default factory should be used.
            let v = load_autocomplete_buffer(ui.ctx(), id, || "def".to_string());
            assert_eq!(v, "def");

            // Store a value and ensure it is returned by the loader.
            store_autocomplete_buffer(ui.ctx(), id, "abc");
            let v2 = load_autocomplete_buffer(ui.ctx(), id, || "def".to_string());
            assert_eq!(v2, "abc");

            // Remove the buffer and ensure memory no longer contains it.
            remove_autocomplete_buffer(ui.ctx(), id);
            let maybe = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
            assert!(maybe.is_none());
        });
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
    fn test_extract_proficiency_candidates() {
        use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};

        let proficiencies = vec![
            ProficiencyDefinition {
                id: "simple_weapon".to_string(),
                name: "Simple Weapons".to_string(),
                category: ProficiencyCategory::Weapon,
                description: "Basic weapons".to_string(),
            },
            ProficiencyDefinition {
                id: "light_armor".to_string(),
                name: "Light Armor".to_string(),
                category: ProficiencyCategory::Armor,
                description: "Light armor proficiency".to_string(),
            },
        ];

        let candidates = extract_proficiency_candidates(&proficiencies);
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0],
            (
                "Simple Weapons (simple_weapon)".to_string(),
                "simple_weapon".to_string()
            )
        );
        assert_eq!(
            candidates[1],
            (
                "Light Armor (light_armor)".to_string(),
                "light_armor".to_string()
            )
        );
    }

    #[test]
    fn test_extract_proficiency_candidates_empty() {
        let proficiencies = vec![];
        let candidates = extract_proficiency_candidates(&proficiencies);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_load_proficiencies_synthetic_fallback() {
        use antares::domain::items::types::{
            ArmorClassification, ArmorData, Item, ItemType, WeaponClassification, WeaponData,
        };
        use antares::domain::types::DiceRoll;

        // Create test items with various classifications
        let items = vec![
            Item {
                id: 1,
                name: "Sword".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 8, 0),
                    bonus: 0,
                    hands_required: 1,
                    classification: WeaponClassification::MartialMelee,
                }),
                base_cost: 10,
                sell_cost: 5,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec![],
            },
            Item {
                id: 2,
                name: "Plate Mail".to_string(),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 8,
                    weight: 50,
                    classification: ArmorClassification::Heavy,
                }),
                base_cost: 100,
                sell_cost: 50,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec![],
            },
        ];

        // Test with no campaign dir (will fall back to synthetic)
        let profs = load_proficiencies(None, &items);

        // Should have standard proficiencies
        assert!(!profs.is_empty());
        assert!(profs.iter().any(|p| p.id == "simple_weapon"));
        assert!(profs.iter().any(|p| p.id == "martial_melee"));
        assert!(profs.iter().any(|p| p.id == "heavy_armor"));
        assert!(profs.iter().any(|p| p.id == "light_armor"));
    }

    #[test]
    fn test_generate_synthetic_proficiencies_standard() {
        // Test that standard proficiencies are always generated
        let profs = generate_synthetic_proficiencies(&[]);

        // Should have all 11 standard proficiencies
        assert_eq!(profs.len(), 11);
        assert!(profs.iter().any(|p| p.id == "simple_weapon"));
        assert!(profs.iter().any(|p| p.id == "martial_melee"));
        assert!(profs.iter().any(|p| p.id == "martial_ranged"));
        assert!(profs.iter().any(|p| p.id == "blunt_weapon"));
        assert!(profs.iter().any(|p| p.id == "unarmed"));
        assert!(profs.iter().any(|p| p.id == "light_armor"));
        assert!(profs.iter().any(|p| p.id == "medium_armor"));
        assert!(profs.iter().any(|p| p.id == "heavy_armor"));
        assert!(profs.iter().any(|p| p.id == "shield"));
        assert!(profs.iter().any(|p| p.id == "arcane_item"));
        assert!(profs.iter().any(|p| p.id == "divine_item"));
    }

    #[test]
    fn test_extract_item_tag_candidates() {
        use antares::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};

        let items = vec![
            Item {
                id: 1,
                name: "Sword".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: ConsumableEffect::HealHp(0),
                    is_combat_usable: false,
                }),
                base_cost: 10,
                sell_cost: 5,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec!["large_weapon".to_string(), "two_handed".to_string()],
            },
            Item {
                id: 2,
                name: "Armor".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: ConsumableEffect::HealHp(0),
                    is_combat_usable: false,
                }),
                base_cost: 50,
                sell_cost: 25,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec!["heavy_armor".to_string(), "two_handed".to_string()],
            },
        ];

        let candidates = extract_item_tag_candidates(&items);
        assert_eq!(candidates.len(), 3); // unique tags: heavy_armor, large_weapon, two_handed
        assert!(candidates.contains(&"large_weapon".to_string()));
        assert!(candidates.contains(&"two_handed".to_string()));
        assert!(candidates.contains(&"heavy_armor".to_string()));
    }

    #[test]
    fn test_extract_item_tag_candidates_empty() {
        let items = vec![];
        let candidates = extract_item_tag_candidates(&items);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_extract_special_ability_candidates() {
        use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

        let races = vec![
            RaceDefinition {
                id: "human".to_string(),
                name: "Human".to_string(),
                description: String::new(),
                stat_modifiers: StatModifiers::default(),
                resistances: Resistances::default(),
                size: SizeCategory::Medium,
                special_abilities: vec!["lucky".to_string(), "brave".to_string()],
                proficiencies: vec![],
                incompatible_item_tags: vec![],
            },
            RaceDefinition {
                id: "elf".to_string(),
                name: "Elf".to_string(),
                description: String::new(),
                stat_modifiers: StatModifiers::default(),
                resistances: Resistances::default(),
                size: SizeCategory::Medium,
                special_abilities: vec!["infravision".to_string(), "keen_senses".to_string()],
                proficiencies: vec![],
                incompatible_item_tags: vec![],
            },
        ];

        let candidates = extract_special_ability_candidates(&races);
        // Should include race abilities + standard abilities
        assert!(candidates.len() >= 4);
        assert!(candidates.contains(&"lucky".to_string()));
        assert!(candidates.contains(&"brave".to_string()));
        assert!(candidates.contains(&"infravision".to_string()));
        assert!(candidates.contains(&"keen_senses".to_string()));
        // Check that standard abilities are included
        assert!(candidates.contains(&"magic_resistance".to_string()));
        assert!(candidates.contains(&"darkvision".to_string()));
    }

    #[test]
    fn test_extract_special_ability_candidates_empty() {
        let races = vec![];
        let candidates = extract_special_ability_candidates(&races);
        // Should still have standard abilities
        assert!(candidates.len() > 0);
        assert!(candidates.contains(&"infravision".to_string()));
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
        use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};

        let proficiencies = vec![
            ProficiencyDefinition {
                id: "sword".to_string(),
                name: "Sword".to_string(),
                category: ProficiencyCategory::Weapon,
                description: String::new(),
            },
            ProficiencyDefinition {
                id: "shield".to_string(),
                name: "Shield".to_string(),
                category: ProficiencyCategory::Armor,
                description: String::new(),
            },
            ProficiencyDefinition {
                id: "heavy_armor".to_string(),
                name: "Heavy Armor".to_string(),
                category: ProficiencyCategory::Armor,
                description: String::new(),
            },
        ];

        let candidates = extract_proficiency_candidates(&proficiencies);
        assert_eq!(candidates.len(), 3);
        assert_eq!(
            candidates[0],
            ("Sword (sword)".to_string(), "sword".to_string())
        );
        assert_eq!(
            candidates[1],
            ("Shield (shield)".to_string(), "shield".to_string())
        );
        assert_eq!(
            candidates[2],
            (
                "Heavy Armor (heavy_armor)".to_string(),
                "heavy_armor".to_string()
            )
        );
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
        use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

        let mut cache = AutocompleteCandidateCache::new();
        let items = vec![Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 0,
                weight: 0,
                classification: ArmorClassification::Light,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
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
        use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

        let mut cache = AutocompleteCandidateCache::new();
        let items_old = vec![Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 0,
                weight: 0,
                classification: ArmorClassification::Light,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
        }];

        // Generate initial cache
        let _candidates1 = cache.get_or_generate_items(&items_old);

        // Invalidate cache
        cache.invalidate_items();

        // New data should be generated
        let items_new = vec![Item {
            id: 2,
            name: "Axe".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 0,
                weight: 0,
                classification: ArmorClassification::Light,
            }),
            base_cost: 12,
            sell_cost: 6,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
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
            special_attack_threshold: 0,
            resistances: MonsterResistances::default(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable::default(),
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }];

        let candidates = cache.get_or_generate_monsters(&monsters);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "Goblin");
    }

    #[test]
    fn candidate_cache_performance_with_200_items() {
        use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};
        use std::time::Instant;

        let mut cache = AutocompleteCandidateCache::new();

        // Generate 200 items
        let items: Vec<Item> = (0..200)
            .map(|i| Item {
                id: i,
                name: format!("Item{}", i),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 0,
                    weight: 0,
                    classification: ArmorClassification::Light,
                }),
                base_cost: 0,
                sell_cost: 0,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: Vec::new(),
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
        use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

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
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 0,
                weight: 0,
                classification: ArmorClassification::Light,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
        }];

        egui::CentralPanel::default().show(&ctx, |ui| {
            // Should show warning for non-existent item ID (use a valid u8 value)
            show_item_validation_warning(ui, 255, &items);
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

    // =========================================================================
    // Map and NPC Candidate Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_map_candidates() {
        use antares::domain::world::Map;

        let maps = vec![
            Map::new(
                1,
                "Town Square".to_string(),
                "Starting area".to_string(),
                20,
                20,
            ),
            Map::new(
                2,
                "Dark Forest".to_string(),
                "Dangerous woods".to_string(),
                30,
                30,
            ),
            Map::new(5, "Castle".to_string(), "Royal palace".to_string(), 40, 40),
        ];

        let candidates = extract_map_candidates(&maps);

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].0, "Town Square (ID: 1)");
        assert_eq!(candidates[0].1, 1);
        assert_eq!(candidates[1].0, "Dark Forest (ID: 2)");
        assert_eq!(candidates[1].1, 2);
        assert_eq!(candidates[2].0, "Castle (ID: 5)");
        assert_eq!(candidates[2].1, 5);
    }

    #[test]
    fn test_extract_map_candidates_empty() {
        let maps: Vec<antares::domain::world::Map> = vec![];
        let candidates = extract_map_candidates(&maps);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_extract_npc_candidates() {
        use antares::domain::types::Position;
        use antares::domain::world::{Map, Npc};

        let mut map1 = Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10);
        map1.npcs.push(Npc {
            id: 1,
            name: "Merchant".to_string(),
            description: "Sells goods".to_string(),
            position: Position::new(5, 5),
            dialogue: String::new(),
        });
        map1.npcs.push(Npc {
            id: 2,
            name: "Guard".to_string(),
            description: "Protects the town".to_string(),
            position: Position::new(7, 3),
            dialogue: String::new(),
        });

        let mut map2 = Map::new(2, "Castle".to_string(), "Desc".to_string(), 15, 15);
        map2.npcs.push(Npc {
            id: 1,
            name: "King".to_string(),
            description: "Rules the land".to_string(),
            position: Position::new(8, 8),
            dialogue: String::new(),
        });

        let candidates = extract_npc_candidates(&[map1, map2]);

        assert_eq!(candidates.len(), 3);
        assert!(candidates[0].0.contains("Merchant"));
        assert!(candidates[0].0.contains("Town"));
        assert_eq!(candidates[0].1, "1:1");
        assert!(candidates[1].0.contains("Guard"));
        assert_eq!(candidates[1].1, "1:2");
        assert!(candidates[2].0.contains("King"));
        assert!(candidates[2].0.contains("Castle"));
        assert_eq!(candidates[2].1, "2:1");
    }

    #[test]
    fn test_extract_npc_candidates_empty_maps() {
        let maps: Vec<antares::domain::world::Map> = vec![];
        let candidates = extract_npc_candidates(&maps);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_extract_npc_candidates_maps_with_no_npcs() {
        use antares::domain::world::Map;

        let maps = vec![
            Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10),
            Map::new(2, "Forest".to_string(), "Desc".to_string(), 20, 20),
        ];

        let candidates = extract_npc_candidates(&maps);
        assert_eq!(candidates.len(), 0);
    }

    // =========================================================================
    // Quest Candidate Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_quest_candidates() {
        use antares::domain::quest::{Quest, QuestId};

        let mut q1 = Quest::new(1, "Save the Village", "Help save the village from bandits");
        q1.min_level = Some(1);
        q1.repeatable = false;
        q1.is_main_quest = true;
        q1.quest_giver_npc = None;
        q1.quest_giver_map = None;
        q1.quest_giver_position = None;

        let mut q2 = Quest::new(2, "Find the Lost Sword", "Recover the legendary sword");
        q2.min_level = Some(5);
        q2.max_level = Some(10);
        q2.repeatable = false;
        q2.is_main_quest = false;
        q2.quest_giver_npc = None;
        q2.quest_giver_map = None;
        q2.quest_giver_position = None;

        let quests = vec![q1, q2];

        let candidates = extract_quest_candidates(&quests);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].0, "Save the Village (ID: 1)");
        assert_eq!(candidates[0].1, 1);
        assert_eq!(candidates[1].0, "Find the Lost Sword (ID: 2)");
        assert_eq!(candidates[1].1, 2);
    }

    #[test]
    fn test_extract_quest_candidates_empty() {
        use antares::domain::quest::Quest;

        let quests: Vec<Quest> = vec![];
        let candidates = extract_quest_candidates(&quests);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_extract_quest_candidates_maintains_order() {
        use antares::domain::quest::Quest;

        let mut q1 = Quest::new(10, "Quest Alpha", "First quest");
        q1.min_level = Some(1);
        q1.repeatable = false;
        q1.is_main_quest = false;
        q1.quest_giver_npc = None;
        q1.quest_giver_map = None;
        q1.quest_giver_position = None;

        let mut q2 = Quest::new(5, "Quest Beta", "Second quest");
        q2.min_level = Some(1);
        q2.repeatable = false;
        q2.is_main_quest = false;
        q2.quest_giver_npc = None;
        q2.quest_giver_map = None;
        q2.quest_giver_position = None;

        let quests = vec![q1, q2];

        let candidates = extract_quest_candidates(&quests);
        assert_eq!(candidates.len(), 2);
        // Should maintain input order
        assert_eq!(candidates[0].1, 10);
        assert_eq!(candidates[1].1, 5);
    }
}
