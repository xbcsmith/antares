// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Workflow state management for the Item Mesh Editor.
//!
//! This module tracks which "mode" the Item Mesh Editor is currently in
//! (browsing the registry vs. editing a specific asset) and provides
//! breadcrumb / mode-indicator strings for the UI header.
//!
//! # Design
//!
//! [`ItemMeshWorkflow`] is a lightweight companion to
//! [`crate::item_mesh_editor::ItemMeshEditorState`].  It owns the navigation
//! state (current file, unsaved-changes flag) so that the larger editor
//! struct stays focused on visual properties.
//!
//! [`ItemMeshEditorMode`] is defined here (not in the editor) because both
//! the workflow and the editor struct need it, and placing it here avoids a
//! circular import.
//!
//! # Examples
//!
//! ```
//! use campaign_builder::item_mesh_workflow::{ItemMeshWorkflow, ItemMeshEditorMode};
//!
//! let mut wf = ItemMeshWorkflow::new();
//! assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
//!
//! wf.enter_edit("my_sword.ron");
//! assert!(matches!(wf.mode(), ItemMeshEditorMode::Edit));
//! assert_eq!(wf.current_file(), Some("my_sword.ron"));
//! assert_eq!(wf.breadcrumb_string(), "Item Meshes > my_sword.ron");
//!
//! wf.return_to_registry();
//! assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
//! assert!(wf.current_file().is_none());
//! ```

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshEditorMode
// ─────────────────────────────────────────────────────────────────────────────

/// The high-level navigation mode of the Item Mesh Editor.
///
/// # Variants
///
/// - `Registry` — The editor is showing the list of all registered item mesh
///   assets. The user can search, filter, sort, and select an entry to edit.
/// - `Edit` — The editor has opened a specific asset for editing. Property
///   sliders and the preview panel are visible.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_workflow::ItemMeshEditorMode;
///
/// let mode = ItemMeshEditorMode::Registry;
/// assert!(matches!(mode, ItemMeshEditorMode::Registry));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemMeshEditorMode {
    /// Browsing the registry of registered item mesh assets.
    #[default]
    Registry,
    /// Editing a specific item mesh asset.
    Edit,
}

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshWorkflow
// ─────────────────────────────────────────────────────────────────────────────

/// Navigation and dirty-tracking state for the Item Mesh Editor.
///
/// Encapsulates:
/// - Which [`ItemMeshEditorMode`] is currently active.
/// - The file name of the asset currently being edited (if any).
/// - Whether there are unsaved changes to the current asset.
///
/// The workflow struct does **not** own the editor's data buffers; it only
/// tracks navigation and dirty state.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
///
/// let mut wf = ItemMeshWorkflow::new();
/// assert!(!wf.has_unsaved_changes());
///
/// wf.enter_edit("shield.ron");
/// wf.mark_dirty();
/// assert!(wf.has_unsaved_changes());
///
/// wf.return_to_registry();
/// assert!(!wf.has_unsaved_changes());
/// ```
#[derive(Debug)]
pub struct ItemMeshWorkflow {
    mode: ItemMeshEditorMode,
    current_file: Option<String>,
    unsaved_changes: bool,
}

impl Default for ItemMeshWorkflow {
    fn default() -> Self {
        Self {
            mode: ItemMeshEditorMode::Registry,
            current_file: None,
            unsaved_changes: false,
        }
    }
}

impl ItemMeshWorkflow {
    /// Creates a new [`ItemMeshWorkflow`] in [`ItemMeshEditorMode::Registry`]
    /// with no current file and no unsaved changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::{ItemMeshWorkflow, ItemMeshEditorMode};
    ///
    /// let wf = ItemMeshWorkflow::new();
    /// assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
    /// assert!(wf.current_file().is_none());
    /// assert!(!wf.has_unsaved_changes());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current editor mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::{ItemMeshWorkflow, ItemMeshEditorMode};
    ///
    /// let wf = ItemMeshWorkflow::new();
    /// assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
    /// ```
    pub fn mode(&self) -> ItemMeshEditorMode {
        self.mode
    }

    /// Returns a short human-readable string describing the current mode for
    /// display in the editor header / status bar.
    ///
    /// - Registry mode → `"Registry Mode"`
    /// - Edit mode with a current file → `"Asset Editor: <file>"`
    /// - Edit mode with no current file → `"Asset Editor"`
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// assert_eq!(wf.mode_indicator(), "Registry Mode");
    ///
    /// wf.enter_edit("dagger.ron");
    /// assert_eq!(wf.mode_indicator(), "Asset Editor: dagger.ron");
    /// ```
    pub fn mode_indicator(&self) -> String {
        match self.mode {
            ItemMeshEditorMode::Registry => "Registry Mode".to_string(),
            ItemMeshEditorMode::Edit => match &self.current_file {
                Some(file) => format!("Asset Editor: {}", file),
                None => "Asset Editor".to_string(),
            },
        }
    }

    /// Returns a breadcrumb navigation string for the editor header.
    ///
    /// - Registry mode → `"Item Meshes"`
    /// - Edit mode with a current file → `"Item Meshes > <file>"`
    /// - Edit mode with no current file → `"Item Meshes"`
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// assert_eq!(wf.breadcrumb_string(), "Item Meshes");
    ///
    /// wf.enter_edit("staff.ron");
    /// assert_eq!(wf.breadcrumb_string(), "Item Meshes > staff.ron");
    /// ```
    pub fn breadcrumb_string(&self) -> String {
        match (&self.mode, &self.current_file) {
            (ItemMeshEditorMode::Edit, Some(file)) => format!("Item Meshes > {}", file),
            _ => "Item Meshes".to_string(),
        }
    }

    /// Transitions to [`ItemMeshEditorMode::Edit`] for the given file.
    ///
    /// Sets `mode` to `Edit`, records `file_name` as the current file, and
    /// **clears** the unsaved-changes flag (loading a fresh asset starts clean).
    ///
    /// # Arguments
    ///
    /// * `file_name` — The file name (or relative path) of the asset being
    ///   opened for editing.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::{ItemMeshWorkflow, ItemMeshEditorMode};
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// wf.enter_edit("boots.ron");
    ///
    /// assert!(matches!(wf.mode(), ItemMeshEditorMode::Edit));
    /// assert_eq!(wf.current_file(), Some("boots.ron"));
    /// assert!(!wf.has_unsaved_changes());
    /// ```
    pub fn enter_edit(&mut self, file_name: impl Into<String>) {
        self.mode = ItemMeshEditorMode::Edit;
        self.current_file = Some(file_name.into());
        self.unsaved_changes = false;
    }

    /// Returns to [`ItemMeshEditorMode::Registry`], clearing the current file
    /// and the unsaved-changes flag.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::{ItemMeshWorkflow, ItemMeshEditorMode};
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// wf.enter_edit("ring.ron");
    /// wf.mark_dirty();
    ///
    /// wf.return_to_registry();
    /// assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
    /// assert!(wf.current_file().is_none());
    /// assert!(!wf.has_unsaved_changes());
    /// ```
    pub fn return_to_registry(&mut self) {
        self.mode = ItemMeshEditorMode::Registry;
        self.current_file = None;
        self.unsaved_changes = false;
    }

    /// Marks the current asset as having unsaved changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// wf.enter_edit("amulet.ron");
    /// assert!(!wf.has_unsaved_changes());
    ///
    /// wf.mark_dirty();
    /// assert!(wf.has_unsaved_changes());
    /// ```
    pub fn mark_dirty(&mut self) {
        self.unsaved_changes = true;
    }

    /// Clears the unsaved-changes flag.
    ///
    /// Call this after a successful save.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// wf.enter_edit("potion.ron");
    /// wf.mark_dirty();
    /// wf.mark_clean();
    /// assert!(!wf.has_unsaved_changes());
    /// ```
    pub fn mark_clean(&mut self) {
        self.unsaved_changes = false;
    }

    /// Returns `true` if the current asset has unsaved changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// assert!(!wf.has_unsaved_changes());
    /// wf.enter_edit("scroll.ron");
    /// wf.mark_dirty();
    /// assert!(wf.has_unsaved_changes());
    /// ```
    pub fn has_unsaved_changes(&self) -> bool {
        self.unsaved_changes
    }

    /// Returns the file name of the asset currently being edited, or `None`
    /// when in Registry mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_workflow::ItemMeshWorkflow;
    ///
    /// let mut wf = ItemMeshWorkflow::new();
    /// assert!(wf.current_file().is_none());
    ///
    /// wf.enter_edit("helmet.ron");
    /// assert_eq!(wf.current_file(), Some("helmet.ron"));
    /// ```
    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── default state ─────────────────────────────────────────────────────────

    /// A freshly-created workflow must start in Registry mode.
    #[test]
    fn test_workflow_default_is_registry() {
        let wf = ItemMeshWorkflow::new();
        assert!(
            matches!(wf.mode(), ItemMeshEditorMode::Registry),
            "default mode should be Registry"
        );
        assert!(wf.current_file().is_none(), "no file on creation");
        assert!(!wf.has_unsaved_changes(), "no unsaved changes on creation");
    }

    // ── mode_indicator ────────────────────────────────────────────────────────

    /// `mode_indicator` in Registry mode should return `"Registry Mode"`.
    #[test]
    fn test_item_mesh_editor_mode_indicator_registry() {
        let wf = ItemMeshWorkflow::new();
        assert_eq!(wf.mode_indicator(), "Registry Mode");
    }

    /// `mode_indicator` in Edit mode should include the file name.
    #[test]
    fn test_item_mesh_editor_mode_indicator_edit() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("iron_sword.ron");
        assert_eq!(wf.mode_indicator(), "Asset Editor: iron_sword.ron");
    }

    /// `mode_indicator` in Edit mode with no file set should return the
    /// generic "Asset Editor" string.
    #[test]
    fn test_item_mesh_editor_mode_indicator_edit_no_file() {
        let mut wf = ItemMeshWorkflow::new();
        // Manually force Edit mode without a file (edge case).
        wf.mode = ItemMeshEditorMode::Edit;
        assert_eq!(wf.mode_indicator(), "Asset Editor");
    }

    // ── breadcrumb_string ─────────────────────────────────────────────────────

    /// `breadcrumb_string` in Registry mode should return `"Item Meshes"`.
    #[test]
    fn test_item_mesh_editor_breadcrumb_registry() {
        let wf = ItemMeshWorkflow::new();
        assert_eq!(wf.breadcrumb_string(), "Item Meshes");
    }

    /// `breadcrumb_string` in Edit mode should return `"Item Meshes > <file>"`.
    #[test]
    fn test_item_mesh_editor_breadcrumb_edit() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("silver_amulet.ron");
        assert_eq!(wf.breadcrumb_string(), "Item Meshes > silver_amulet.ron");
    }

    /// `breadcrumb_string` in Edit mode with no file should fall back to
    /// `"Item Meshes"`.
    #[test]
    fn test_item_mesh_editor_breadcrumb_edit_no_file() {
        let mut wf = ItemMeshWorkflow::new();
        wf.mode = ItemMeshEditorMode::Edit;
        assert_eq!(wf.breadcrumb_string(), "Item Meshes");
    }

    // ── enter_edit ────────────────────────────────────────────────────────────

    /// `enter_edit` should switch to Edit mode and record the file name.
    #[test]
    fn test_workflow_enter_edit() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("great_sword.ron");

        assert!(matches!(wf.mode(), ItemMeshEditorMode::Edit));
        assert_eq!(wf.current_file(), Some("great_sword.ron"));
    }

    /// `enter_edit` should clear any pending unsaved-changes from a previous
    /// session.
    #[test]
    fn test_workflow_enter_edit_clears_unsaved_changes() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("old_file.ron");
        wf.mark_dirty();
        assert!(wf.has_unsaved_changes());

        wf.enter_edit("new_file.ron");
        assert!(
            !wf.has_unsaved_changes(),
            "enter_edit should reset unsaved changes"
        );
    }

    // ── return_to_registry ────────────────────────────────────────────────────

    /// `return_to_registry` should reset mode, clear file, and clear dirty flag.
    #[test]
    fn test_workflow_return_to_registry() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("staff_of_fire.ron");
        wf.mark_dirty();

        wf.return_to_registry();

        assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
        assert!(wf.current_file().is_none());
        assert!(!wf.has_unsaved_changes());
    }

    // ── mark_dirty / mark_clean ───────────────────────────────────────────────

    #[test]
    fn test_workflow_mark_dirty() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("ring_of_power.ron");
        assert!(!wf.has_unsaved_changes());
        wf.mark_dirty();
        assert!(wf.has_unsaved_changes());
    }

    #[test]
    fn test_workflow_mark_clean() {
        let mut wf = ItemMeshWorkflow::new();
        wf.enter_edit("cloak_of_shadows.ron");
        wf.mark_dirty();
        assert!(wf.has_unsaved_changes());
        wf.mark_clean();
        assert!(!wf.has_unsaved_changes());
    }

    // ── Default impl ──────────────────────────────────────────────────────────

    #[test]
    fn test_workflow_default_impl() {
        let wf = ItemMeshWorkflow::default();
        assert!(matches!(wf.mode(), ItemMeshEditorMode::Registry));
        assert!(wf.current_file().is_none());
        assert!(!wf.has_unsaved_changes());
    }
}
