// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature Editor Unified Workflow - Phase 5.1
//!
//! Provides the unified workflow state and logic for the creature editor,
//! integrating all Phase 5 components:
//! - Undo/redo history via `CreatureUndoRedoManager`
//! - Keyboard shortcuts via `ShortcutManager`
//! - Context menus via `ContextMenuManager`
//! - Auto-save and crash recovery via `AutoSaveManager`
//! - Enhanced preview via `PreviewState`
//!
//! # Workflow Modes
//!
//! The editor operates in two top-level modes:
//! - **Registry Mode**: Browse, search, and manage all registered creatures.
//! - **Asset Editor Mode**: Edit a single creature's mesh geometry, transforms,
//!   colors, and properties with real-time 3D preview.
//!
//! # Workflow Example
//!
//! ```
//! use campaign_builder::creatures_workflow::{CreatureWorkflowState, WorkflowMode};
//!
//! let mut workflow = CreatureWorkflowState::new();
//! assert_eq!(workflow.mode, WorkflowMode::Registry);
//!
//! // Transition to editing a creature named "Goblin"
//! workflow.enter_asset_editor("goblin.ron", "Goblin");
//! assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
//! assert_eq!(workflow.current_file(), Some("goblin.ron"));
//!
//! // Return to the registry
//! workflow.return_to_registry();
//! assert_eq!(workflow.mode, WorkflowMode::Registry);
//! ```

use crate::auto_save::{AutoSaveConfig, AutoSaveManager};
use crate::context_menu::ContextMenuManager;
use crate::creature_undo_redo::CreatureUndoRedoManager;
use crate::keyboard_shortcuts::ShortcutManager;
use crate::preview_features::PreviewState;

// ---------------------------------------------------------------------------
// WorkflowMode
// ---------------------------------------------------------------------------

/// Top-level operating mode of the creature editor.
///
/// # Examples
///
/// ```
/// use campaign_builder::creatures_workflow::WorkflowMode;
///
/// let mode = WorkflowMode::Registry;
/// assert_eq!(mode.display_name(), "Registry Mode");
///
/// let mode = WorkflowMode::AssetEditor;
/// assert_eq!(mode.display_name(), "Asset Editor");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowMode {
    /// The registry list view – browse and manage creature entries.
    Registry,
    /// The per-creature asset editor – edit meshes, transforms, and properties.
    AssetEditor,
}

impl WorkflowMode {
    /// Human-readable label shown in the top-bar mode indicator.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::WorkflowMode;
    ///
    /// assert_eq!(WorkflowMode::Registry.display_name(), "Registry Mode");
    /// assert_eq!(WorkflowMode::AssetEditor.display_name(), "Asset Editor");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            WorkflowMode::Registry => "Registry Mode",
            WorkflowMode::AssetEditor => "Asset Editor",
        }
    }

    /// Returns `true` when the editor is in asset-editing mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::WorkflowMode;
    ///
    /// assert!(!WorkflowMode::Registry.is_asset_editor());
    /// assert!(WorkflowMode::AssetEditor.is_asset_editor());
    /// ```
    pub fn is_asset_editor(&self) -> bool {
        *self == WorkflowMode::AssetEditor
    }
}

impl Default for WorkflowMode {
    fn default() -> Self {
        WorkflowMode::Registry
    }
}

// ---------------------------------------------------------------------------
// EditorBreadcrumb
// ---------------------------------------------------------------------------

/// A single breadcrumb segment shown in the navigation bar.
///
/// For example:  `Creatures > Monsters > Goblin > left_leg`
///
/// # Examples
///
/// ```
/// use campaign_builder::creatures_workflow::EditorBreadcrumb;
///
/// let crumb = EditorBreadcrumb::new("Goblin", Some("goblin.ron".to_string()));
/// assert_eq!(crumb.label, "Goblin");
/// assert_eq!(crumb.file_path.as_deref(), Some("goblin.ron"));
/// ```
#[derive(Debug, Clone)]
pub struct EditorBreadcrumb {
    /// Display label of this segment.
    pub label: String,
    /// Optional file path associated with this segment.
    pub file_path: Option<String>,
}

impl EditorBreadcrumb {
    /// Create a new breadcrumb segment.
    ///
    /// # Arguments
    ///
    /// * `label` - Human-readable name for this segment.
    /// * `file_path` - Optional path to the file being edited at this level.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::EditorBreadcrumb;
    ///
    /// let crumb = EditorBreadcrumb::new("Goblin", Some("goblin.ron".to_string()));
    /// assert_eq!(crumb.label, "Goblin");
    /// ```
    pub fn new(label: impl Into<String>, file_path: Option<String>) -> Self {
        Self {
            label: label.into(),
            file_path,
        }
    }

    /// Create a breadcrumb without an associated file path.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::EditorBreadcrumb;
    ///
    /// let crumb = EditorBreadcrumb::label_only("Creatures");
    /// assert!(crumb.file_path.is_none());
    /// ```
    pub fn label_only(label: impl Into<String>) -> Self {
        Self::new(label, None)
    }
}

// ---------------------------------------------------------------------------
// CreatureWorkflowState
// ---------------------------------------------------------------------------

/// Integrated workflow state for the creature editor (Phase 5).
///
/// Owns all Phase 5 subsystems: undo/redo, shortcuts, context menus,
/// auto-save, and preview.  The creature editor embeds this struct and
/// delegates to it for every operation that requires history, input
/// handling, or persistence.
///
/// # Examples
///
/// ```
/// use campaign_builder::creatures_workflow::CreatureWorkflowState;
/// use campaign_builder::creatures_workflow::WorkflowMode;
///
/// let mut workflow = CreatureWorkflowState::new();
/// assert_eq!(workflow.mode, WorkflowMode::Registry);
/// assert!(!workflow.has_unsaved_changes());
/// ```
pub struct CreatureWorkflowState {
    /// Current top-level editor mode.
    pub mode: WorkflowMode,

    /// Breadcrumb path reflecting the current navigation state.
    pub breadcrumbs: Vec<EditorBreadcrumb>,

    /// Undo/redo command history for the currently-edited creature.
    pub undo_redo: CreatureUndoRedoManager,

    /// Keyboard shortcut registry.
    pub shortcuts: ShortcutManager,

    /// Context menu registry.
    pub context_menus: ContextMenuManager,

    /// Auto-save and crash-recovery manager.
    /// `None` until a file path is established.
    pub auto_save: Option<AutoSaveManager>,

    /// Enhanced 3D preview state (options, camera, lighting, statistics).
    pub preview: PreviewState,

    /// Name of the creature currently being edited, if any.
    current_creature_name: Option<String>,

    /// File name of the asset currently being edited, if any.
    current_file: Option<String>,

    /// Whether there are unsaved changes in the current editing session.
    unsaved_changes: bool,
}

impl Default for CreatureWorkflowState {
    fn default() -> Self {
        Self::new()
    }
}

impl CreatureWorkflowState {
    /// Create a new workflow state with all Phase 5 subsystems initialised.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(!workflow.has_unsaved_changes());
    /// ```
    pub fn new() -> Self {
        Self {
            mode: WorkflowMode::Registry,
            breadcrumbs: vec![EditorBreadcrumb::label_only("Creatures")],
            undo_redo: CreatureUndoRedoManager::new(),
            shortcuts: ShortcutManager::new(),
            context_menus: ContextMenuManager::new(),
            auto_save: None,
            preview: PreviewState::new(),
            current_creature_name: None,
            current_file: None,
            unsaved_changes: false,
        }
    }

    /// Create a workflow state with a custom auto-save configuration.
    ///
    /// # Arguments
    ///
    /// * `auto_save_config` - Configuration for the auto-save manager.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` on success, or an error if the auto-save directory
    /// cannot be created.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the auto-save directory cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    /// use campaign_builder::auto_save::AutoSaveConfig;
    /// use std::path::Path;
    ///
    /// let config = AutoSaveConfig::default().with_directory(Path::new("/tmp/autosave"));
    /// let workflow = CreatureWorkflowState::with_auto_save(config).unwrap();
    /// assert!(workflow.auto_save.is_some());
    /// ```
    pub fn with_auto_save(config: AutoSaveConfig) -> Result<Self, crate::auto_save::AutoSaveError> {
        let manager = AutoSaveManager::new(config)?;
        let mut state = Self::new();
        state.auto_save = Some(manager);
        Ok(state)
    }

    // -----------------------------------------------------------------------
    // Mode transitions
    // -----------------------------------------------------------------------

    /// Transition into asset-editor mode for the specified creature.
    ///
    /// Resets the undo/redo history, clears unsaved-changes flag, and rebuilds
    /// the breadcrumb trail.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The `.ron` asset file name (e.g. `"goblin.ron"`).
    /// * `creature_name` - Display name of the creature being edited.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::{CreatureWorkflowState, WorkflowMode};
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    ///
    /// assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
    /// assert_eq!(workflow.current_creature_name(), Some("Goblin"));
    /// ```
    pub fn enter_asset_editor(
        &mut self,
        file_name: impl Into<String>,
        creature_name: impl Into<String>,
    ) {
        let file_name = file_name.into();
        let creature_name = creature_name.into();

        self.mode = WorkflowMode::AssetEditor;
        self.current_file = Some(file_name.clone());
        self.current_creature_name = Some(creature_name.clone());
        self.unsaved_changes = false;
        self.undo_redo.clear();

        // Rebuild breadcrumbs: Creatures > <Name>
        self.breadcrumbs = vec![
            EditorBreadcrumb::label_only("Creatures"),
            EditorBreadcrumb::new(creature_name, Some(file_name)),
        ];

        // Notify auto-save of the new file path
        if let Some(ref mut auto_save) = self.auto_save {
            auto_save.set_file_path(
                self.current_file
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_string(),
            );
        }
    }

    /// Transition into asset-editor mode and navigate into a specific mesh.
    ///
    /// Extends the breadcrumb trail with the mesh name.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The `.ron` asset file name.
    /// * `creature_name` - Display name of the creature.
    /// * `mesh_name` - Display name of the selected mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.enter_mesh_editor("goblin.ron", "Goblin", "left_leg");
    ///
    /// let crumbs: Vec<&str> = workflow.breadcrumb_labels().collect();
    /// assert_eq!(crumbs, ["Creatures", "Goblin", "left_leg"]);
    /// ```
    pub fn enter_mesh_editor(
        &mut self,
        file_name: impl Into<String>,
        creature_name: impl Into<String>,
        mesh_name: impl Into<String>,
    ) {
        self.enter_asset_editor(file_name, creature_name);
        self.breadcrumbs
            .push(EditorBreadcrumb::label_only(mesh_name.into()));
    }

    /// Return to registry mode, discarding any unsaved changes silently.
    ///
    /// Callers are responsible for prompting the user before calling this
    /// if `has_unsaved_changes()` is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::{CreatureWorkflowState, WorkflowMode};
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    /// workflow.return_to_registry();
    ///
    /// assert_eq!(workflow.mode, WorkflowMode::Registry);
    /// assert!(workflow.current_file().is_none());
    /// ```
    pub fn return_to_registry(&mut self) {
        self.mode = WorkflowMode::Registry;
        self.current_file = None;
        self.current_creature_name = None;
        self.unsaved_changes = false;
        self.undo_redo.clear();
        self.breadcrumbs = vec![EditorBreadcrumb::label_only("Creatures")];
    }

    // -----------------------------------------------------------------------
    // Unsaved-changes tracking
    // -----------------------------------------------------------------------

    /// Mark the current session as having unsaved changes.
    ///
    /// Also notifies the auto-save manager so a timed auto-save can be
    /// triggered on the next `tick`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// assert!(!workflow.has_unsaved_changes());
    ///
    /// workflow.mark_dirty();
    /// assert!(workflow.has_unsaved_changes());
    /// ```
    pub fn mark_dirty(&mut self) {
        self.unsaved_changes = true;
        if let Some(ref mut auto_save) = self.auto_save {
            auto_save.mark_dirty();
        }
    }

    /// Mark the current session as clean (all changes saved).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.mark_dirty();
    /// workflow.mark_clean();
    /// assert!(!workflow.has_unsaved_changes());
    /// ```
    pub fn mark_clean(&mut self) {
        self.unsaved_changes = false;
        if let Some(ref mut auto_save) = self.auto_save {
            auto_save.mark_clean();
        }
    }

    /// Returns `true` if there are unsaved changes in the current session.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(!workflow.has_unsaved_changes());
    /// ```
    pub fn has_unsaved_changes(&self) -> bool {
        self.unsaved_changes
    }

    // -----------------------------------------------------------------------
    // Undo / redo convenience wrappers
    // -----------------------------------------------------------------------

    /// Returns `true` if there is at least one action to undo.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(!workflow.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        self.undo_redo.can_undo()
    }

    /// Returns `true` if there is at least one action to redo.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(!workflow.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        self.undo_redo.can_redo()
    }

    /// Human-readable description of the next undoable action, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(workflow.undo_description().is_none());
    /// ```
    pub fn undo_description(&self) -> Option<String> {
        self.undo_redo.next_undo_description()
    }

    /// Human-readable description of the next redoable action, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// assert!(workflow.redo_description().is_none());
    /// ```
    pub fn redo_description(&self) -> Option<String> {
        self.undo_redo.next_redo_description()
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Returns the file name of the creature asset currently being edited.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// assert!(workflow.current_file().is_none());
    ///
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    /// assert_eq!(workflow.current_file(), Some("goblin.ron"));
    /// ```
    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    /// Returns the display name of the creature currently being edited.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    /// assert_eq!(workflow.current_creature_name(), Some("Goblin"));
    /// ```
    pub fn current_creature_name(&self) -> Option<&str> {
        self.current_creature_name.as_deref()
    }

    /// Returns an iterator over the breadcrumb labels for the current path.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let workflow = CreatureWorkflowState::new();
    /// let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
    /// assert_eq!(labels, ["Creatures"]);
    /// ```
    pub fn breadcrumb_labels(&self) -> impl Iterator<Item = &str> {
        self.breadcrumbs.iter().map(|c| c.label.as_str())
    }

    /// Formats the breadcrumb trail as a `>` separated string.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    ///
    /// assert_eq!(workflow.breadcrumb_string(), "Creatures > Goblin");
    /// ```
    pub fn breadcrumb_string(&self) -> String {
        self.breadcrumbs
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>()
            .join(" > ")
    }

    /// Builds the mode indicator string shown in the top bar.
    ///
    /// Format:  `"Registry Mode"` or `"Asset Editor: goblin.ron"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::creatures_workflow::CreatureWorkflowState;
    ///
    /// let mut workflow = CreatureWorkflowState::new();
    /// assert_eq!(workflow.mode_indicator(), "Registry Mode");
    ///
    /// workflow.enter_asset_editor("goblin.ron", "Goblin");
    /// assert_eq!(workflow.mode_indicator(), "Asset Editor: goblin.ron");
    /// ```
    pub fn mode_indicator(&self) -> String {
        match self.mode {
            WorkflowMode::Registry => "Registry Mode".to_string(),
            WorkflowMode::AssetEditor => {
                let file = self.current_file.as_deref().unwrap_or("unknown");
                format!("Asset Editor: {}", file)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workflow() -> CreatureWorkflowState {
        CreatureWorkflowState::new()
    }

    // --- WorkflowMode ---

    #[test]
    fn test_workflow_mode_display_names() {
        assert_eq!(WorkflowMode::Registry.display_name(), "Registry Mode");
        assert_eq!(WorkflowMode::AssetEditor.display_name(), "Asset Editor");
    }

    #[test]
    fn test_workflow_mode_is_asset_editor() {
        assert!(!WorkflowMode::Registry.is_asset_editor());
        assert!(WorkflowMode::AssetEditor.is_asset_editor());
    }

    #[test]
    fn test_workflow_mode_default_is_registry() {
        assert_eq!(WorkflowMode::default(), WorkflowMode::Registry);
    }

    // --- EditorBreadcrumb ---

    #[test]
    fn test_breadcrumb_new() {
        let crumb = EditorBreadcrumb::new("Goblin", Some("goblin.ron".to_string()));
        assert_eq!(crumb.label, "Goblin");
        assert_eq!(crumb.file_path.as_deref(), Some("goblin.ron"));
    }

    #[test]
    fn test_breadcrumb_label_only() {
        let crumb = EditorBreadcrumb::label_only("Creatures");
        assert_eq!(crumb.label, "Creatures");
        assert!(crumb.file_path.is_none());
    }

    // --- CreatureWorkflowState construction ---

    #[test]
    fn test_new_starts_in_registry_mode() {
        let workflow = make_workflow();
        assert_eq!(workflow.mode, WorkflowMode::Registry);
    }

    #[test]
    fn test_new_has_no_unsaved_changes() {
        let workflow = make_workflow();
        assert!(!workflow.has_unsaved_changes());
    }

    #[test]
    fn test_new_no_current_file() {
        let workflow = make_workflow();
        assert!(workflow.current_file().is_none());
    }

    #[test]
    fn test_new_no_current_creature_name() {
        let workflow = make_workflow();
        assert!(workflow.current_creature_name().is_none());
    }

    #[test]
    fn test_new_initial_breadcrumb() {
        let workflow = make_workflow();
        let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
        assert_eq!(labels, ["Creatures"]);
    }

    #[test]
    fn test_new_cannot_undo_or_redo() {
        let workflow = make_workflow();
        assert!(!workflow.can_undo());
        assert!(!workflow.can_redo());
    }

    // --- Mode transitions ---

    #[test]
    fn test_enter_asset_editor_sets_mode() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
    }

    #[test]
    fn test_enter_asset_editor_sets_file() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert_eq!(workflow.current_file(), Some("goblin.ron"));
    }

    #[test]
    fn test_enter_asset_editor_sets_creature_name() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert_eq!(workflow.current_creature_name(), Some("Goblin"));
    }

    #[test]
    fn test_enter_asset_editor_builds_breadcrumbs() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
        assert_eq!(labels, ["Creatures", "Goblin"]);
    }

    #[test]
    fn test_enter_asset_editor_clears_unsaved_changes() {
        let mut workflow = make_workflow();
        workflow.mark_dirty();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert!(!workflow.has_unsaved_changes());
    }

    #[test]
    fn test_enter_mesh_editor_extends_breadcrumbs() {
        let mut workflow = make_workflow();
        workflow.enter_mesh_editor("goblin.ron", "Goblin", "left_leg");
        let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
        assert_eq!(labels, ["Creatures", "Goblin", "left_leg"]);
    }

    #[test]
    fn test_return_to_registry_resets_mode() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        workflow.return_to_registry();
        assert_eq!(workflow.mode, WorkflowMode::Registry);
    }

    #[test]
    fn test_return_to_registry_clears_file() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        workflow.return_to_registry();
        assert!(workflow.current_file().is_none());
    }

    #[test]
    fn test_return_to_registry_clears_breadcrumbs() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        workflow.return_to_registry();
        let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
        assert_eq!(labels, ["Creatures"]);
    }

    #[test]
    fn test_return_to_registry_clears_unsaved_changes() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        workflow.mark_dirty();
        workflow.return_to_registry();
        assert!(!workflow.has_unsaved_changes());
    }

    // --- Unsaved-changes tracking ---

    #[test]
    fn test_mark_dirty_sets_flag() {
        let mut workflow = make_workflow();
        workflow.mark_dirty();
        assert!(workflow.has_unsaved_changes());
    }

    #[test]
    fn test_mark_clean_clears_flag() {
        let mut workflow = make_workflow();
        workflow.mark_dirty();
        workflow.mark_clean();
        assert!(!workflow.has_unsaved_changes());
    }

    // --- Breadcrumb and mode indicator ---

    #[test]
    fn test_breadcrumb_string_registry() {
        let workflow = make_workflow();
        assert_eq!(workflow.breadcrumb_string(), "Creatures");
    }

    #[test]
    fn test_breadcrumb_string_asset_editor() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert_eq!(workflow.breadcrumb_string(), "Creatures > Goblin");
    }

    #[test]
    fn test_breadcrumb_string_mesh_editor() {
        let mut workflow = make_workflow();
        workflow.enter_mesh_editor("goblin.ron", "Goblin", "left_leg");
        assert_eq!(
            workflow.breadcrumb_string(),
            "Creatures > Goblin > left_leg"
        );
    }

    #[test]
    fn test_mode_indicator_registry() {
        let workflow = make_workflow();
        assert_eq!(workflow.mode_indicator(), "Registry Mode");
    }

    #[test]
    fn test_mode_indicator_asset_editor() {
        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert_eq!(workflow.mode_indicator(), "Asset Editor: goblin.ron");
    }

    // --- Undo/redo delegation ---

    #[test]
    fn test_undo_description_empty_is_none() {
        let workflow = make_workflow();
        assert!(workflow.undo_description().is_none());
    }

    #[test]
    fn test_redo_description_empty_is_none() {
        let workflow = make_workflow();
        assert!(workflow.redo_description().is_none());
    }

    #[test]
    fn test_enter_asset_editor_clears_undo_history() {
        use crate::creature_undo_redo::AddMeshCommand;
        use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};

        let mut workflow = make_workflow();
        workflow.enter_asset_editor("goblin.ron", "Goblin");

        let mut creature = CreatureDefinition {
            id: 1,
            name: "Goblin".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        };

        let mesh = MeshDefinition {
            name: Some("body".to_string()),
            vertices: vec![[0.0, 0.0, 0.0]],
            indices: vec![],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let transform = MeshTransform {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        };

        workflow
            .undo_redo
            .execute(
                Box::new(AddMeshCommand::new(mesh, transform)),
                &mut creature,
            )
            .unwrap();

        assert!(workflow.can_undo());

        // Re-entering asset editor should reset history
        workflow.enter_asset_editor("goblin.ron", "Goblin");
        assert!(!workflow.can_undo());
    }

    // --- Auto-save integration ---

    #[test]
    fn test_mark_dirty_notifies_auto_save() {
        use crate::auto_save::AutoSaveConfig;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let config = AutoSaveConfig::default().with_directory(dir.path());
        let mut workflow = CreatureWorkflowState::with_auto_save(config).unwrap();

        workflow.enter_asset_editor("goblin.ron", "Goblin");
        workflow.mark_dirty();

        let auto_save = workflow.auto_save.as_ref().unwrap();
        assert!(auto_save.is_dirty());
    }

    #[test]
    fn test_mark_clean_notifies_auto_save() {
        use crate::auto_save::AutoSaveConfig;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let config = AutoSaveConfig::default().with_directory(dir.path());
        let mut workflow = CreatureWorkflowState::with_auto_save(config).unwrap();

        workflow.mark_dirty();
        workflow.mark_clean();

        let auto_save = workflow.auto_save.as_ref().unwrap();
        assert!(!auto_save.is_dirty());
    }

    // --- Preview state access ---

    #[test]
    fn test_preview_state_accessible() {
        let workflow = make_workflow();
        assert!(workflow.preview.options.show_grid);
    }

    // --- Multiple round-trips ---

    #[test]
    fn test_multiple_mode_transitions() {
        let mut workflow = make_workflow();

        for i in 0..3 {
            let file = format!("creature_{}.ron", i);
            let name = format!("Creature {}", i);
            workflow.enter_asset_editor(&file, &name);
            assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
            assert_eq!(workflow.current_file(), Some(file.as_str()));

            workflow.return_to_registry();
            assert_eq!(workflow.mode, WorkflowMode::Registry);
            assert!(workflow.current_file().is_none());
        }
    }
}
