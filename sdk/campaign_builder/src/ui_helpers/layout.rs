// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Layout types, toolbar, list components, and entity validation warnings.
//!
//! Contains constants, autocomplete buffer helpers, panel-height helpers,
//! filter/selector helpers, [`EditorToolbar`], [`ActionButtons`],
//! [`TwoColumnLayout`], [`MetadataBadge`], [`StandardListItemConfig`],
//! [`show_standard_list_item`], and entity validation warnings.

use eframe::egui;
use std::fmt::Display;
use std::path::Path;

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
/// // Construct an id and use the helpers with an egui::Context:
/// let ctx = egui::Context::default();
/// let id = egui::Id::new(format!("autocomplete:item:{}", "my_widget"));
/// let mut buf = campaign_builder::ui_helpers::load_autocomplete_buffer(&ctx, id, || String::new());
/// // ... render widget with &mut buf ...
/// campaign_builder::ui_helpers::store_autocomplete_buffer(&ctx, id, &buf);
/// // or remove:
/// campaign_builder::ui_helpers::remove_autocomplete_buffer(&ctx, id);
/// ```
pub(crate) fn make_autocomplete_id(_ui: &egui::Ui, prefix: &str, id_salt: &str) -> egui::Id {
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
#[allow(clippy::map_clone)]
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
/// use campaign_builder::ui_helpers::render_grid_header;
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
/// use campaign_builder::ui_helpers::show_validation_severity_icon;
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
/// use campaign_builder::ui_helpers::compute_panel_height_from_size;
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
/// use campaign_builder::ui_helpers::compute_panel_height;
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

/// Returns indices of `items` whose label (provided by `label_fn`) contains `query` (case-insensitive).
///
/// Useful for building filtered lists or suggestions.
///
/// # Examples
///
/// ```
/// # use campaign_builder::ui_helpers::filter_items_by_query;
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

/// Configuration bundle for [`searchable_selector_single`] and [`searchable_selector_multi`].
///
/// Groups the two presentation parameters (`id_salt` and `label`) into a
/// single struct so that the selector functions stay under the Clippy
/// `too_many_arguments` threshold.
///
/// The mutable search buffer and candidate data are now carried by
/// [`SearchableSelectorContext`].
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::SearchableSelectorConfig;
///
/// let cfg = SearchableSelectorConfig {
///     id_salt: "my_selector",
///     label: "Select item:",
/// };
/// ```
pub struct SearchableSelectorConfig<'a> {
    /// Unique egui ID salt for the `ComboBox` widget.  Must be unique within the frame.
    pub id_salt: &'a str,
    /// User-visible label rendered to the left of the `ComboBox`.
    pub label: &'a str,
}

/// Context bundle for [`searchable_selector_single`] and [`searchable_selector_multi`].
///
/// Bundles the candidate slice, mutable search buffer, and accessor closures so
/// call sites pass one argument instead of four.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::SearchableSelectorContext;
///
/// struct Planet { id: u32, name: String }
///
/// let planets = vec![
///     Planet { id: 1, name: "Mercury".to_string() },
///     Planet { id: 2, name: "Venus".to_string() },
/// ];
/// let mut search = String::new();
/// let ctx = SearchableSelectorContext {
///     candidates: &planets,
///     search_buf: &mut search,
///     id_fn: |p: &Planet| p.id,
///     label_fn: |p: &Planet| p.name.as_str(),
/// };
/// assert_eq!(ctx.candidates.len(), 2);
/// assert_eq!((ctx.label_fn)(&planets[0]), "Mercury");
/// ```
pub struct SearchableSelectorContext<'a, T, ID> {
    /// Full candidate list to filter and display.
    pub candidates: &'a [T],
    /// Mutable search string typed by the user, persisted by the caller across frames.
    pub search_buf: &'a mut String,
    /// Extracts the comparable ID from a candidate item.
    pub id_fn: fn(&T) -> ID,
    /// Extracts the display label from a candidate item.
    pub label_fn: fn(&T) -> &str,
}

/// Single-selection searchable selector UI helper.
///
/// - `ui`: egui UI reference
/// - `cfg`: [`SearchableSelectorConfig`] bundling `id_salt` and `label`
/// - `selected`: Mutable reference to current selection (`Option<ID>`)
/// - `ctx`: [`SearchableSelectorContext`] bundling candidates, search buffer, and accessor fns
///
/// Returns `true` if the selection changed.
///
/// This helper wraps `egui::ComboBox` and provides an inline search text field inside the
/// ComboBox dropdown to filter options.
pub fn searchable_selector_single<T, ID>(
    ui: &mut egui::Ui,
    cfg: &SearchableSelectorConfig<'_>,
    selected: &mut Option<ID>,
    ctx: SearchableSelectorContext<'_, T, ID>,
) -> bool
where
    ID: Clone + PartialEq + Display,
{
    let SearchableSelectorContext {
        candidates,
        search_buf,
        id_fn,
        label_fn,
    } = ctx;
    ui.label(cfg.label);
    let mut changed = false;
    let selected_text = selected
        .as_ref()
        .map_or("(None)".to_string(), |id| id.to_string());
    egui::ComboBox::from_id_salt(cfg.id_salt)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            // Search input at the top
            ui.text_edit_singleline(search_buf);
            let q = search_buf.to_lowercase();

            // Filtered list
            for item in candidates.iter() {
                let label_text = label_fn(item);
                if q.is_empty() || label_text.to_lowercase().contains(&q) {
                    let id = id_fn(item);
                    let is_selected = selected.as_ref().map(|s| s == &id).unwrap_or(false);
                    if ui.selectable_label(is_selected, label_text).clicked() {
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
/// - `ui`: egui UI reference
/// - `cfg`: [`SearchableSelectorConfig`] bundling `id_salt` and `label`
/// - `selection`: mutable vector of selected IDs (caller manages order/persistence)
/// - `ctx`: [`SearchableSelectorContext`] bundling candidates, search buffer, and accessor fns
///
/// Returns `true` if the selection changed (items added or removed).
pub fn searchable_selector_multi<T, ID>(
    ui: &mut egui::Ui,
    cfg: &SearchableSelectorConfig<'_>,
    selection: &mut Vec<ID>,
    ctx: SearchableSelectorContext<'_, T, ID>,
) -> bool
where
    ID: Clone + PartialEq + Display,
{
    let SearchableSelectorContext {
        candidates,
        search_buf,
        id_fn,
        label_fn,
    } = ctx;
    ui.label(cfg.label);
    let mut changed = false;

    // Render chips for selected items with a small remove button.
    ui.horizontal_wrapped(|ui| {
        let mut idx_to_remove: Option<usize> = None;
        for (idx, sel) in selection.iter().enumerate() {
            let label_text = candidates
                .iter()
                .find(|it| id_fn(it) == *sel)
                .map(|it| label_fn(it).to_owned())
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
        ui.text_edit_singleline(search_buf);
        if ui.button("Add").clicked() {
            let q = search_buf.to_lowercase();
            // Try to find the first match by label text
            if let Some(item) = candidates
                .iter()
                .find(|it| label_fn(it).to_lowercase().contains(&q))
            {
                let id = id_fn(item);
                if !selection.contains(&id) {
                    selection.push(id);
                    changed = true;
                }
                *search_buf = String::new();
            }
        }
    });

    // Suggestion buttons (compact)
    let q = search_buf.to_lowercase();
    ui.horizontal_wrapped(|ui| {
        for item in candidates {
            let label_text = label_fn(item);
            if (q.is_empty() || label_text.to_lowercase().contains(&q))
                && ui.small_button(label_text).clicked()
            {
                let id = id_fn(item);
                if !selection.contains(&id) {
                    selection.push(id);
                    changed = true;
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
    pub(crate) editor_name: &'a str,
    /// Search query text (mutable reference)
    pub(crate) search_query: Option<&'a mut String>,
    /// Merge mode checkbox state (mutable reference)
    pub(crate) merge_mode: Option<&'a mut bool>,
    /// Total count to display
    pub(crate) total_count: Option<usize>,
    /// Whether to show the save button
    pub(crate) show_save: bool,
    /// Custom id salt for disambiguation
    pub(crate) id_salt: Option<&'a str>,
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
    /// use campaign_builder::ui_helpers::EditorToolbar;
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
    /// use campaign_builder::ui_helpers::{EditorToolbar, ToolbarAction};
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

            if self.show_save
                && ui
                    .button("💾 Save")
                    .on_hover_text("Save to campaign (Ctrl+S)")
                    .clicked()
            {
                action = ToolbarAction::Save;
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
    pub(crate) enabled: bool,
    /// Whether to show the edit button
    pub(crate) show_edit: bool,
    /// Whether to show the delete button
    pub(crate) show_delete: bool,
    /// Whether to show the duplicate button
    pub(crate) show_duplicate: bool,
    /// Whether to show the export button
    pub(crate) show_export: bool,
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
    /// use campaign_builder::ui_helpers::ActionButtons;
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
    /// use campaign_builder::ui_helpers::{ActionButtons, ItemAction};
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
                if self.show_edit
                    && ui
                        .button("✏️ Edit")
                        .on_hover_text("Edit selected item (Ctrl+E)")
                        .clicked()
                {
                    action = ItemAction::Edit;
                }
                if self.show_delete
                    && ui
                        .button("🗑️ Delete")
                        .on_hover_text("Delete selected item (Delete)")
                        .clicked()
                {
                    action = ItemAction::Delete;
                }
                if self.show_duplicate
                    && ui
                        .button("📋 Duplicate")
                        .on_hover_text("Duplicate selected item (Ctrl+D)")
                        .clicked()
                {
                    action = ItemAction::Duplicate;
                }
                if self.show_export
                    && ui
                        .button("📤 Export")
                        .on_hover_text("Export selected item")
                        .clicked()
                {
                    action = ItemAction::Export;
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
    pub(crate) id_salt: &'a str,
    /// Width of the left column in points
    pub(crate) left_width: f32,
    /// Minimum height for both panels
    pub(crate) min_height: f32,
    /// Minimum width for the inspector (right) column (points)
    pub(crate) inspector_min_width: f32,
    /// Maximum ratio (0.0 - 1.0) allowed for the left column relative to total width.
    /// This prevents the left column from consuming too much horizontal space.
    pub(crate) max_left_ratio: f32,
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
    /// use campaign_builder::ui_helpers::TwoColumnLayout;
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
    /// use campaign_builder::ui_helpers::TwoColumnLayout;
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
// Standard List Item Component
// =============================================================================

/// A colored badge for displaying rich metadata in left-panel list items.
///
/// Badges appear as small colored text labels rendered below the primary item
/// label in the left panel list. They convey category, type, status, and other
/// relevant metadata without cluttering the primary label.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::MetadataBadge;
/// use eframe::egui;
///
/// let badge = MetadataBadge::new("Weapon")
///     .with_color(egui::Color32::from_rgb(200, 100, 50))
///     .with_tooltip("Melee weapon");
///
/// assert_eq!(badge.text, "Weapon");
/// assert_eq!(badge.tooltip, Some("Melee weapon".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct MetadataBadge {
    /// The text displayed on the badge
    pub text: String,
    /// The foreground color of the badge text
    pub color: egui::Color32,
    /// Optional tooltip shown when the user hovers over the badge
    pub tooltip: Option<String>,
}

impl MetadataBadge {
    /// Creates a new metadata badge with the given text using the default gray color.
    ///
    /// # Arguments
    ///
    /// * `text` - The badge label text
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::MetadataBadge;
    ///
    /// let badge = MetadataBadge::new("Armor");
    /// assert_eq!(badge.text, "Armor");
    /// assert!(badge.tooltip.is_none());
    /// ```
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: egui::Color32::GRAY,
            tooltip: None,
        }
    }

    /// Sets the foreground color of the badge text.
    ///
    /// # Arguments
    ///
    /// * `color` - The desired text color
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::MetadataBadge;
    /// use eframe::egui;
    ///
    /// let badge = MetadataBadge::new("Magic")
    ///     .with_color(egui::Color32::from_rgb(128, 0, 200));
    /// assert_eq!(badge.color, egui::Color32::from_rgb(128, 0, 200));
    /// ```
    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    /// Sets a tooltip shown when the user hovers over the badge.
    ///
    /// # Arguments
    ///
    /// * `tooltip` - The tooltip text
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::MetadataBadge;
    ///
    /// let badge = MetadataBadge::new("Magic")
    ///     .with_tooltip("Magical item");
    /// assert_eq!(badge.tooltip, Some("Magical item".to_string()));
    /// ```
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Configuration for rendering a standard left-panel list item.
///
/// Encapsulates the primary label, optional icon prefix, selection state,
/// metadata badges, and an optional display ID. Pass this to
/// [`show_standard_list_item`] to render a consistent list entry across all
/// editors.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::{MetadataBadge, StandardListItemConfig};
/// use eframe::egui;
///
/// let badges = vec![
///     MetadataBadge::new("Sword").with_color(egui::Color32::from_rgb(200, 100, 50)),
/// ];
/// let config = StandardListItemConfig::new("Iron Sword")
///     .with_badges(badges)
///     .with_id(1u32)
///     .with_icon("⚔️")
///     .selected(true);
///
/// assert_eq!(config.label, "Iron Sword");
/// assert!(config.selected);
/// assert_eq!(config.badges.len(), 1);
/// assert_eq!(config.id, Some("1".to_string()));
/// assert_eq!(config.icon, Some("⚔️"));
/// ```
pub struct StandardListItemConfig<'a> {
    /// Primary display label for the item
    pub label: String,
    /// Whether this item is currently selected (highlighted)
    pub selected: bool,
    /// Optional metadata badges shown below the primary label
    pub badges: Vec<MetadataBadge>,
    /// Optional display ID shown at the end of the metadata row in gray
    pub id: Option<String>,
    /// Optional icon or emoji prefix rendered before the primary label
    pub icon: Option<&'a str>,
    /// Whether right-click context menu actions are enabled for this item
    pub context_menu_enabled: bool,
}

impl<'a> StandardListItemConfig<'a> {
    /// Creates a new list item configuration with the given label.
    ///
    /// All optional fields default to `None` / `false` / empty.
    ///
    /// # Arguments
    ///
    /// * `label` - The primary display label
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::StandardListItemConfig;
    ///
    /// let config = StandardListItemConfig::new("Iron Sword");
    /// assert_eq!(config.label, "Iron Sword");
    /// assert!(!config.selected);
    /// assert!(config.badges.is_empty());
    /// assert!(config.id.is_none());
    /// assert!(config.icon.is_none());
    /// ```
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            badges: Vec::new(),
            id: None,
            icon: None,
            context_menu_enabled: true,
        }
    }

    /// Sets the selection state of the list item.
    ///
    /// When `selected` is `true` the item is rendered with a highlighted background.
    ///
    /// # Arguments
    ///
    /// * `selected` - Whether this item should appear selected
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::StandardListItemConfig;
    ///
    /// let config = StandardListItemConfig::new("Iron Sword").selected(true);
    /// assert!(config.selected);
    /// ```
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Attaches metadata badges that appear below the primary label.
    ///
    /// # Arguments
    ///
    /// * `badges` - A vector of [`MetadataBadge`] values to display
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::{MetadataBadge, StandardListItemConfig};
    ///
    /// let badges = vec![MetadataBadge::new("Weapon"), MetadataBadge::new("Magic")];
    /// let config = StandardListItemConfig::new("Iron Sword").with_badges(badges);
    /// assert_eq!(config.badges.len(), 2);
    /// ```
    pub fn with_badges(mut self, badges: Vec<MetadataBadge>) -> Self {
        self.badges = badges;
        self
    }

    /// Sets the display ID shown in gray at the end of the metadata row.
    ///
    /// The value is converted to a string via [`Display`][std::fmt::Display]
    /// and rendered as `#<id>` in a small, muted style.
    ///
    /// # Arguments
    ///
    /// * `id` - A value that implements `Display`
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::StandardListItemConfig;
    ///
    /// let config = StandardListItemConfig::new("Iron Sword").with_id(42u32);
    /// assert_eq!(config.id, Some("42".to_string()));
    /// ```
    pub fn with_id(mut self, id: impl std::fmt::Display) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// Sets an icon or emoji prefix rendered before the primary label text.
    ///
    /// # Arguments
    ///
    /// * `icon` - A string slice containing the icon or emoji
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::ui_helpers::StandardListItemConfig;
    ///
    /// let config = StandardListItemConfig::new("Iron Sword").with_icon("⚔️");
    /// assert_eq!(config.icon, Some("⚔️"));
    /// ```
    pub fn with_icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Enables or disables the right-click context menu for this item.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether context menu actions should be available
    pub fn with_context_menu(mut self, enabled: bool) -> Self {
        self.context_menu_enabled = enabled;
        self
    }
}

/// Shows a standard list item in the left panel with badges and a context menu.
///
/// Renders the primary label (with optional icon prefix), a row of colored
/// metadata badges below it, and wires up a right-click context menu with
/// Edit / Delete / Duplicate / Export actions.
///
/// # Arguments
///
/// * `ui`     - The egui UI context
/// * `config` - The [`StandardListItemConfig`] describing the item
///
/// # Returns
///
/// A tuple `(clicked, action)` where:
/// - `clicked` is `true` if the item was left-clicked (indicates selection intent)
/// - `action` is the [`ItemAction`] triggered from the context menu
///   (`ItemAction::None` if no action was triggered this frame)
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use campaign_builder::ui_helpers::{
///     ItemAction, MetadataBadge, StandardListItemConfig, show_standard_list_item,
/// };
///
/// fn example(ui: &mut egui::Ui, is_selected: bool) {
///     let badges = vec![
///         MetadataBadge::new("Sword").with_color(egui::Color32::from_rgb(200, 100, 50)),
///     ];
///     let config = StandardListItemConfig::new("Iron Sword")
///         .with_badges(badges)
///         .with_id(1u32)
///         .selected(is_selected);
///
///     let (clicked, action) = show_standard_list_item(ui, config);
///
///     if clicked {
///         // handle item selection
///     }
///     match action {
///         ItemAction::Edit      => { /* enter edit mode */ }
///         ItemAction::Delete    => { /* delete item */ }
///         ItemAction::Duplicate => { /* duplicate item */ }
///         ItemAction::Export    => { /* export item */ }
///         ItemAction::None      => {}
///     }
/// }
/// ```
pub fn show_standard_list_item(
    ui: &mut egui::Ui,
    config: StandardListItemConfig,
) -> (bool, ItemAction) {
    let mut action = ItemAction::None;

    // Main selectable label with optional icon
    let label_text = if let Some(icon) = config.icon {
        format!("{} {}", icon, config.label)
    } else {
        config.label.clone()
    };

    let response = ui.selectable_label(config.selected, &label_text);
    let clicked = response.clicked();

    // Context menu
    if config.context_menu_enabled {
        response.context_menu(|ui| {
            if ui.button("✏️ Edit").clicked() {
                action = ItemAction::Edit;
                ui.close();
            }
            if ui.button("🗑️ Delete").clicked() {
                action = ItemAction::Delete;
                ui.close();
            }
            if ui.button("📋 Duplicate").clicked() {
                action = ItemAction::Duplicate;
                ui.close();
            }
            if ui.button("📤 Export").clicked() {
                action = ItemAction::Export;
                ui.close();
            }
        });
    }

    // Metadata badges (indented)
    if !config.badges.is_empty() || config.id.is_some() {
        ui.horizontal(|ui| {
            ui.add_space(20.0); // Indent for hierarchy

            // Show badges
            for badge in &config.badges {
                let badge_text = egui::RichText::new(&badge.text).small().color(badge.color);
                let badge_response = ui.label(badge_text);
                if let Some(tooltip) = &badge.tooltip {
                    badge_response.on_hover_text(tooltip);
                }
            }

            // Show ID if present
            if let Some(id) = config.id {
                let id_text = egui::RichText::new(format!("#{}", id)).small().weak();
                ui.label(id_text);
            }
        });
    }

    (clicked, action)
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
/// use antares::domain::items::types::Item;
/// use campaign_builder::ui_helpers::show_entity_validation_warning;
///
/// fn example(ui: &mut egui::Ui, item_id: antares::domain::types::ItemId, items: &[Item]) {
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
