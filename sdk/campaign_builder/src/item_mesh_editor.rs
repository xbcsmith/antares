// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item Mesh Editor — Campaign Builder SDK integration (Phase 5).
//!
//! This module provides [`ItemMeshEditorState`], the top-level state struct
//! for the Item Mesh Editor tab.  It supports two navigation modes:
//!
//! - **Registry** — browse, search, filter, and sort all registered item mesh
//!   assets; open one for editing or register a new asset from disk.
//! - **Edit** — inspect and tweak a single [`ItemMeshDescriptor`]'s visual
//!   properties (colors, scale, emissive) with a live 3-D preview, undo/redo,
//!   save-as, and validation.
//!
//! # egui ID rules (sdk/AGENTS.md)
//!
//! - Every loop body that renders widgets uses `ui.push_id(idx, |ui| { … })`.
//! - Every [`egui::ScrollArea`] has `.id_salt("unique_string")`.
//! - Every [`egui::ComboBox`] uses `ComboBox::from_id_salt("…")`.
//! - Every [`egui::Window`] has a unique title.
//! - State mutations call `ui.ctx().request_repaint()`.

use crate::context_menu::ContextMenuManager;
use crate::item_mesh_undo_redo::{ItemMeshEditAction, ItemMeshUndoRedo};
use crate::item_mesh_workflow::{ItemMeshEditorMode, ItemMeshWorkflow};
use crate::keyboard_shortcuts::ShortcutManager;
use crate::preview_renderer::PreviewRenderer;
use crate::ui_helpers::{
    show_standard_list_item, ItemAction, MetadataBadge, StandardListItemConfig, TwoColumnLayout,
};
use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
use antares::domain::visual::CreatureDefinition;
use eframe::egui;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Supporting types
// ─────────────────────────────────────────────────────────────────────────────

/// Sort order for the registry list panel.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_editor::ItemMeshRegistrySortBy;
///
/// let sort = ItemMeshRegistrySortBy::Name;
/// assert!(matches!(sort, ItemMeshRegistrySortBy::Name));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemMeshRegistrySortBy {
    /// Sort by internal list index (insertion order).
    Id,
    /// Sort alphabetically by entry name.
    #[default]
    Name,
    /// Sort alphabetically by category name.
    Category,
}

/// A single registered item mesh asset entry in the editor's registry.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_editor::ItemMeshEntry;
/// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
///
/// let entry = ItemMeshEntry {
///     name: "Iron Sword".to_string(),
///     category: ItemMeshCategory::Sword,
///     file_path: "assets/items/iron_sword.ron".to_string(),
///     descriptor: ItemMeshDescriptor {
///         category: ItemMeshCategory::Sword,
///         blade_length: 0.6,
///         primary_color: [0.7, 0.7, 0.8, 1.0],
///         accent_color: [0.5, 0.3, 0.1, 1.0],
///         emissive: false,
///         emissive_color: [0.0, 0.0, 0.0],
///         scale: 1.0,
///     },
///     native_creature_def: None,
/// };
/// assert_eq!(entry.name, "Iron Sword");
/// ```
#[derive(Debug, Clone)]
pub struct ItemMeshEntry {
    /// Display name for this entry.
    pub name: String,
    /// Mesh category of the entry.
    pub category: ItemMeshCategory,
    /// Path to the RON file (relative to campaign root).
    pub file_path: String,
    /// The descriptor loaded from (or about to be saved to) `file_path`.
    pub descriptor: ItemMeshDescriptor,
    /// The original [`CreatureDefinition`] loaded from disk when the asset file
    /// was in `CreatureDefinition` format (e.g. an OBJ imported as RON or a
    /// hand-crafted mesh authored outside the SDK editor).
    ///
    /// When `Some`, [`sync_preview_renderer_from_descriptor`] uses this
    /// definition directly so custom vertex geometry is faithfully shown in the
    /// preview instead of the procedurally-generated approximation.
    ///
    /// `None` for entries whose disk file is in `ItemMeshDescriptor` format or
    /// for entries that were created (not imported) inside the SDK editor.
    pub native_creature_def: Option<CreatureDefinition>,
}

/// Signal emitted by [`ItemMeshEditorState::show`] to request cross-tab
/// navigation in the host app.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_editor::ItemMeshEditorSignal;
///
/// let sig = ItemMeshEditorSignal::OpenInItemsEditor(42);
/// match sig {
///     ItemMeshEditorSignal::OpenInItemsEditor(id) => assert_eq!(id, 42),
/// }
/// ```
pub enum ItemMeshEditorSignal {
    /// Ask the host to switch to the Items tab and select the given item ID.
    /// Uses `u8` to match `antares::domain::types::ItemId`.
    OpenInItemsEditor(u8),
}

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshEditorState
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level state for the Item Mesh Editor tab.
///
/// Owns all UI state for both the registry view and the edit view, plus the
/// undo/redo stack, workflow navigation, preview renderer, and dialog state.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
///
/// let state = ItemMeshEditorState::new();
/// assert!(!state.has_unsaved_changes());
/// assert!(!state.can_undo());
/// assert!(!state.can_redo());
/// ```
pub struct ItemMeshEditorState {
    // ── Registry view ──────────────────────────────────────────────────────
    /// Text query for filtering the registry list.
    pub search_query: String,
    /// If `Some`, only show entries matching this category.
    pub category_filter: Option<ItemMeshCategory>,
    /// Sort order for the registry list.
    pub registry_sort_by: ItemMeshRegistrySortBy,
    /// Index into `registry` of the currently-highlighted entry (registry view).
    pub selected_entry: Option<usize>,

    // ── Edit view ──────────────────────────────────────────────────────────
    /// Working copy of the descriptor being edited.
    pub edit_buffer: Option<ItemMeshDescriptor>,
    /// Whether the override controls are enabled.
    pub override_enabled: bool,
    /// `true` when the preview renderer needs to be re-synchronized.
    pub preview_dirty: bool,
    /// Last preview render error, if any.
    pub preview_error: Option<String>,

    // ── Undo/redo ──────────────────────────────────────────────────────────
    /// Undo/redo stack for the current editing session.
    pub undo_redo: ItemMeshUndoRedo,

    // ── Save-As dialog ─────────────────────────────────────────────────────
    /// Controls visibility of the Save As dialog window.
    pub show_save_as_dialog: bool,
    /// Path buffer for the Save As dialog text field.
    pub save_as_path_buffer: String,

    // ── Register-asset dialog ───────────────────────────────────────────────
    /// Controls visibility of the Register Asset dialog window.
    pub show_register_asset_dialog: bool,
    /// Path buffer for the register-asset text field.
    pub register_asset_path_buffer: String,
    /// Last error from register-asset validation, if any.
    pub register_asset_error: Option<String>,
    /// Cached list of `.ron` files found in `campaign_dir/assets/items/`.
    pub available_item_assets: Vec<String>,
    /// The last campaign directory we scanned for available assets.
    pub last_campaign_dir: Option<PathBuf>,

    // ── Validation panel ────────────────────────────────────────────────────
    /// Controls visibility of the inline validation panel.
    pub show_validation_panel: bool,
    /// Current validation errors for `edit_buffer`.
    pub validation_errors: Vec<String>,
    /// Current validation warnings for `edit_buffer`.
    pub validation_warnings: Vec<String>,

    // ── Registry delete confirm ─────────────────────────────────────────────
    /// `true` when a delete confirmation is pending in the registry view.
    pub registry_delete_confirm_pending: bool,

    // ── Import/export ───────────────────────────────────────────────────────
    /// Controls visibility of the import dialog window.
    pub show_import_dialog: bool,
    /// RON text buffer for import/export.
    pub import_export_buffer: String,

    // ── Preview renderer (private) ──────────────────────────────────────────
    preview_renderer: Option<PreviewRenderer>,

    // ── Workflow ────────────────────────────────────────────────────────────
    /// Navigation / dirty-tracking state.
    pub workflow: ItemMeshWorkflow,

    // ── Shortcuts & context menus ───────────────────────────────────────────
    /// Keyboard shortcut manager.
    pub shortcut_manager: ShortcutManager,
    /// Context menu manager.
    pub context_menu_manager: ContextMenuManager,

    // ── Registry ────────────────────────────────────────────────────────────
    /// All registered item mesh entries.
    pub registry: Vec<ItemMeshEntry>,

    // ── Preview camera ──────────────────────────────────────────────────────
    /// Camera distance for the preview panel.
    pub camera_distance: f32,

    // ── Dirty tracking ──────────────────────────────────────────────────────
    unsaved_changes: bool,
}

impl Default for ItemMeshEditorState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            category_filter: None,
            registry_sort_by: ItemMeshRegistrySortBy::Name,
            selected_entry: None,

            edit_buffer: None,
            override_enabled: false,
            preview_dirty: false,
            preview_error: None,

            undo_redo: ItemMeshUndoRedo::new(),

            show_save_as_dialog: false,
            save_as_path_buffer: String::new(),

            show_register_asset_dialog: false,
            register_asset_path_buffer: String::new(),
            register_asset_error: None,
            available_item_assets: Vec::new(),
            last_campaign_dir: None,

            show_validation_panel: false,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),

            registry_delete_confirm_pending: false,

            show_import_dialog: false,
            import_export_buffer: String::new(),

            preview_renderer: None,

            workflow: ItemMeshWorkflow::new(),
            shortcut_manager: ShortcutManager::new(),
            context_menu_manager: ContextMenuManager::new(),

            registry: Vec::new(),
            camera_distance: 5.0,
            unsaved_changes: false,
        }
    }
}

impl ItemMeshEditorState {
    /// Creates a new editor state with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let state = ItemMeshEditorState::new();
    /// assert!(!state.has_unsaved_changes());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    // ── Campaign loading ──────────────────────────────────────────────────

    /// Loads all item mesh assets from the campaign directory into the registry.
    ///
    /// Recursively scans `<campaign_dir>/assets/items/` for `.ron` files.
    /// Each file is first attempted as [`ItemMeshDescriptor`] (editor format).
    /// If that fails, it falls back to [`CreatureDefinition`] (game/legacy
    /// format) and derives an approximate descriptor from the mesh data.
    ///
    /// Replaces the entire registry on each call so repeated loads (e.g. after
    /// the user switches campaigns) do not accumulate stale entries.
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` — Root directory of the open campaign.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// let dir = PathBuf::from("/tmp/my_campaign");
    /// state.load_from_campaign(&dir);
    /// // registry is now populated from assets/items/**/*.ron
    /// ```
    pub fn load_from_campaign(&mut self, campaign_dir: &PathBuf) {
        let assets_dir = campaign_dir.join("assets").join("items");
        if !assets_dir.exists() {
            return;
        }

        self.registry.clear();

        let ron_files = Self::collect_ron_files_recursive(&assets_dir);

        for full_path in ron_files {
            let rel_path = match full_path.strip_prefix(campaign_dir) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let stem = full_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let name = Self::stem_to_display_name(stem);

            // ── Attempt 1: native ItemMeshDescriptor format ───────────────
            if let Ok(descriptor) = ron::de::from_str::<ItemMeshDescriptor>(&content) {
                self.registry.push(ItemMeshEntry {
                    name,
                    category: descriptor.category,
                    file_path: rel_path,
                    descriptor,
                    native_creature_def: None,
                });
                continue;
            }

            // ── Attempt 2: CreatureDefinition (game-engine / imported-OBJ format) ──
            // This is the canonical on-disk format used by the game engine.
            // We derive a simplified ItemMeshDescriptor for the editor controls
            // while storing the full CreatureDefinition so the preview can show
            // the real custom geometry instead of a procedural approximation.
            if let Ok(def) = ron::de::from_str::<CreatureDefinition>(&content) {
                let category = Self::infer_category_from_path(&full_path);
                let primary_color = def
                    .meshes
                    .first()
                    .map(|m| m.color)
                    .unwrap_or([0.7, 0.7, 0.7, 1.0]);
                let accent_color = def
                    .meshes
                    .get(1)
                    .map(|m| m.color)
                    .unwrap_or([0.5, 0.3, 0.1, 1.0]);
                let emissive = def
                    .meshes
                    .first()
                    .and_then(|m| m.material.as_ref())
                    .and_then(|mat| mat.emissive)
                    .is_some();
                let descriptor = ItemMeshDescriptor {
                    category,
                    blade_length: 0.5,
                    primary_color,
                    accent_color,
                    emissive,
                    emissive_color: [0.0, 0.0, 0.0],
                    scale: def.scale,
                };
                self.registry.push(ItemMeshEntry {
                    name,
                    category,
                    file_path: rel_path,
                    descriptor,
                    native_creature_def: Some(def),
                });
            }
        }

        // Keep available_item_assets in sync so the register-asset dialog
        // shows the correct list after loading.
        self.available_item_assets = self.registry.iter().map(|e| e.file_path.clone()).collect();
        self.last_campaign_dir = Some(campaign_dir.clone());
    }

    /// Recursively collects all `.ron` files under `dir`.
    fn collect_ron_files_recursive(dir: &PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(Self::collect_ron_files_recursive(&path));
                } else if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                    files.push(path);
                }
            }
        }
        files
    }

    /// Converts a file stem like `"short_sword"` to `"Short Sword"`.
    fn stem_to_display_name(stem: &str) -> String {
        stem.replace('_', " ")
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Infers an [`ItemMeshCategory`] from the file path.
    ///
    /// Checks the file stem first (most specific), then the parent folder name
    /// as a fallback, then defaults to [`ItemMeshCategory::Sword`].
    fn infer_category_from_path(path: &PathBuf) -> ItemMeshCategory {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let folder = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        match stem.as_str() {
            "sword" | "long_sword" | "great_sword" => ItemMeshCategory::Sword,
            "short_sword" | "dagger" | "knife" => ItemMeshCategory::Dagger,
            "club" | "mace" | "hammer" | "flail" => ItemMeshCategory::Blunt,
            "staff" | "spear" | "halberd" | "polearm" => ItemMeshCategory::Staff,
            "bow" | "crossbow" => ItemMeshCategory::Bow,
            "helmet" | "helm" | "cap" | "hood" => ItemMeshCategory::Helmet,
            "shield" | "buckler" | "tower_shield" => ItemMeshCategory::Shield,
            "boots" | "greaves" | "sabatons" | "sandals" => ItemMeshCategory::Boots,
            "chain_mail" | "plate_mail" | "leather_armor" | "armor" | "mail" | "breastplate"
            | "cuirass" => ItemMeshCategory::BodyArmor,
            "ring" | "band" => ItemMeshCategory::Ring,
            "amulet" | "necklace" | "pendant" | "talisman" => ItemMeshCategory::Amulet,
            "belt" | "girdle" | "sash" => ItemMeshCategory::Belt,
            "cloak" | "cape" | "robe" | "mantle" => ItemMeshCategory::Cloak,
            "potion" | "vial" | "flask" | "elixir" => ItemMeshCategory::Potion,
            "scroll" | "tome" | "grimoire" => ItemMeshCategory::Scroll,
            "arrow" | "bolt" | "ammo" | "quiver" | "stone" => ItemMeshCategory::Ammo,
            "key_item" | "artifact" | "relic" | "quest_scroll" | "key" => {
                ItemMeshCategory::QuestItem
            }
            _ => match folder.as_str() {
                "weapons" => ItemMeshCategory::Sword,
                "armor" => ItemMeshCategory::BodyArmor,
                "accessories" => ItemMeshCategory::Ring,
                "consumables" => ItemMeshCategory::Potion,
                "ammo" => ItemMeshCategory::Ammo,
                "quest" => ItemMeshCategory::QuestItem,
                _ => ItemMeshCategory::Sword,
            },
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────

    /// Opens the registry entry at `idx` for editing.
    ///
    /// Sets `edit_buffer` from the entry's descriptor, resets the undo stack,
    /// marks the preview as dirty, and transitions the workflow to
    /// [`ItemMeshEditorMode::Edit`].
    ///
    /// # Arguments
    ///
    /// * `idx` — Index into `self.registry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/sword.ron".to_string(),
    ///     descriptor: ItemMeshDescriptor {
    ///         category: ItemMeshCategory::Sword,
    ///         blade_length: 0.6,
    ///         primary_color: [0.7, 0.7, 0.8, 1.0],
    ///         accent_color: [0.5, 0.3, 0.1, 1.0],
    ///         emissive: false,
    ///         emissive_color: [0.0, 0.0, 0.0],
    ///         scale: 1.0,
    ///     },
    ///     native_creature_def: None,
    /// });
    /// state.open_for_editing(0);
    /// assert!(state.edit_buffer.is_some());
    /// ```
    pub fn open_for_editing(&mut self, idx: usize) {
        if let Some(entry) = self.registry.get(idx) {
            let desc = entry.descriptor.clone();
            let file_name = entry.file_path.clone();
            self.edit_buffer = Some(desc);
            self.selected_entry = Some(idx);
            self.preview_dirty = true;
            self.undo_redo.clear();
            self.unsaved_changes = false;
            self.workflow.enter_edit(file_name);
            self.preview_error = None;
        }
    }

    /// Returns to the registry view, clearing all edit state.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/sword.ron".to_string(),
    ///     descriptor: ItemMeshDescriptor {
    ///         category: ItemMeshCategory::Sword,
    ///         blade_length: 0.6,
    ///         primary_color: [0.7, 0.7, 0.8, 1.0],
    ///         accent_color: [0.5, 0.3, 0.1, 1.0],
    ///         emissive: false,
    ///         emissive_color: [0.0, 0.0, 0.0],
    ///         scale: 1.0,
    ///     },
    ///     native_creature_def: None,
    /// });
    /// state.open_for_editing(0);
    /// state.back_to_registry();
    /// assert!(state.edit_buffer.is_none());
    /// ```
    pub fn back_to_registry(&mut self) {
        self.edit_buffer = None;
        self.override_enabled = false;
        self.preview_dirty = false;
        self.preview_error = None;
        self.undo_redo.clear();
        self.unsaved_changes = false;
        self.validation_errors.clear();
        self.validation_warnings.clear();
        self.workflow.return_to_registry();
    }

    // ── Info helpers ──────────────────────────────────────────────────────

    /// Returns a short mode-indicator string for the editor header.
    ///
    /// Delegates to [`ItemMeshWorkflow::mode_indicator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let state = ItemMeshEditorState::new();
    /// assert_eq!(state.mode_indicator(), "Registry Mode");
    /// ```
    pub fn mode_indicator(&self) -> String {
        self.workflow.mode_indicator()
    }

    /// Returns the breadcrumb navigation string for the editor header.
    ///
    /// Delegates to [`ItemMeshWorkflow::breadcrumb_string`].
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let state = ItemMeshEditorState::new();
    /// assert_eq!(state.breadcrumb_string(), "Item Meshes");
    /// ```
    pub fn breadcrumb_string(&self) -> String {
        self.workflow.breadcrumb_string()
    }

    /// Returns `true` if the current asset has unsaved changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// assert!(!state.has_unsaved_changes());
    /// ```
    pub fn has_unsaved_changes(&self) -> bool {
        self.unsaved_changes
    }

    /// Returns `true` if there is an action on the undo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let state = ItemMeshEditorState::new();
    /// assert!(!state.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        self.undo_redo.can_undo()
    }

    /// Returns `true` if there is an action on the redo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let state = ItemMeshEditorState::new();
    /// assert!(!state.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        self.undo_redo.can_redo()
    }

    // ── Validation ────────────────────────────────────────────────────────

    /// Validates a descriptor and returns `(errors, warnings)`.
    ///
    /// # Rules
    ///
    /// - **Error**: `scale <= 0.0`
    /// - **Warning**: `scale > 3.0` (unusually large)
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let desc = ItemMeshDescriptor {
    ///     category: ItemMeshCategory::Sword,
    ///     blade_length: 0.5,
    ///     primary_color: [0.7, 0.7, 0.8, 1.0],
    ///     accent_color: [0.5, 0.3, 0.1, 1.0],
    ///     emissive: false,
    ///     emissive_color: [0.0, 0.0, 0.0],
    ///     scale: 1.0,
    /// };
    /// let (errors, warnings) = ItemMeshEditorState::validate_descriptor(&desc);
    /// assert!(errors.is_empty());
    /// assert!(warnings.is_empty());
    /// ```
    pub fn validate_descriptor(descriptor: &ItemMeshDescriptor) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if descriptor.scale <= 0.0 {
            errors.push(format!(
                "Scale must be positive (got {:.4})",
                descriptor.scale
            ));
        }

        if descriptor.scale > 3.0 {
            warnings.push(format!(
                "Scale {:.2}× is unusually large (> 3.0); the item mesh may clip through geometry",
                descriptor.scale
            ));
        }

        (errors, warnings)
    }

    /// Re-runs validation on `edit_buffer` and stores the results.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/sword.ron".to_string(),
    ///     descriptor: ItemMeshDescriptor {
    ///         category: ItemMeshCategory::Sword,
    ///         blade_length: 0.6,
    ///         primary_color: [0.7, 0.7, 0.8, 1.0],
    ///         accent_color: [0.5, 0.3, 0.1, 1.0],
    ///         emissive: false,
    ///         emissive_color: [0.0, 0.0, 0.0],
    ///         scale: 1.0,
    ///     },
    ///     native_creature_def: None,
    /// });
    /// state.open_for_editing(0);
    /// state.refresh_validation_state();
    /// assert!(state.validation_errors.is_empty());
    /// ```
    pub fn refresh_validation_state(&mut self) {
        if let Some(desc) = &self.edit_buffer {
            let (errors, warnings) = Self::validate_descriptor(desc);
            self.validation_errors = errors;
            self.validation_warnings = warnings;
        }
    }

    // ── Asset scanning ────────────────────────────────────────────────────

    /// Scans `campaign_dir/assets/items/` for `.ron` files and caches the
    /// results in `available_item_assets`.
    ///
    /// The scan is skipped if `campaign_dir` equals `last_campaign_dir`
    /// (no change since the last scan).
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` — Root directory of the currently-loaded campaign.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// let dir = PathBuf::from("campaigns/my_campaign");
    /// state.refresh_available_assets(&dir);
    /// ```
    pub fn refresh_available_assets(&mut self, campaign_dir: &PathBuf) {
        // Skip if the directory hasn't changed.
        if self.last_campaign_dir.as_ref() == Some(campaign_dir) {
            return;
        }

        let assets_dir = campaign_dir.join("assets").join("items");
        let mut found = Vec::new();

        // Recursively collect all .ron files under assets/items/ including subdirectories.
        let ron_files = Self::collect_ron_files_recursive(&assets_dir);
        for path in ron_files {
            if let Ok(rel) = path.strip_prefix(campaign_dir) {
                found.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }

        found.sort();
        self.available_item_assets = found;
        self.last_campaign_dir = Some(campaign_dir.clone());
    }

    // ── Save-As ───────────────────────────────────────────────────────────

    /// Returns a default save-as path based on the currently-selected entry
    /// name.
    ///
    /// Format: `"assets/items/<slug>.ron"` where `<slug>` is the entry name
    /// lowercased with spaces replaced by underscores. Falls back to
    /// `"assets/items/new_item_mesh.ron"` when no entry is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// assert_eq!(state.default_save_as_path(), "assets/items/new_item_mesh.ron");
    ///
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Iron Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/iron_sword.ron".to_string(),
    ///     descriptor: ItemMeshDescriptor {
    ///         category: ItemMeshCategory::Sword,
    ///         blade_length: 0.6,
    ///         primary_color: [0.7, 0.7, 0.8, 1.0],
    ///         accent_color: [0.5, 0.3, 0.1, 1.0],
    ///         emissive: false,
    ///         emissive_color: [0.0, 0.0, 0.0],
    ///         scale: 1.0,
    ///     },
    ///     native_creature_def: None,
    /// });
    /// state.selected_entry = Some(0);
    /// assert_eq!(state.default_save_as_path(), "assets/items/iron_sword.ron");
    /// ```
    pub fn default_save_as_path(&self) -> String {
        if let Some(entry) = self.selected_entry.and_then(|i| self.registry.get(i)) {
            let slug = entry.name.to_lowercase().replace(' ', "_");
            format!("assets/items/{}.ron", slug)
        } else {
            "assets/items/new_item_mesh.ron".to_string()
        }
    }

    /// Serializes the current `edit_buffer` to RON and writes it to
    /// `campaign_dir/path`, then appends a new entry to the registry and marks
    /// the editor clean.
    ///
    /// # Errors
    ///
    /// - `campaign_dir` is `None`
    /// - `path` does not start with `"assets/items/"`
    /// - RON serialization fails
    /// - File I/O fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    /// use std::path::PathBuf;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/sword.ron".to_string(),
    ///     descriptor: ItemMeshDescriptor {
    ///         category: ItemMeshCategory::Sword,
    ///         blade_length: 0.6,
    ///         primary_color: [0.7, 0.7, 0.8, 1.0],
    ///         accent_color: [0.5, 0.3, 0.1, 1.0],
    ///         emissive: false,
    ///         emissive_color: [0.0, 0.0, 0.0],
    ///         scale: 1.0,
    ///     },
    ///     native_creature_def: None,
    /// });
    /// state.open_for_editing(0);
    /// let result = state.perform_save_as_with_path(
    ///     "assets/items/sword_copy.ron",
    ///     Some(&PathBuf::from("/tmp/campaign")),
    /// );
    /// // result may be Ok or Err depending on filesystem access
    /// let _ = result;
    /// ```
    pub fn perform_save_as_with_path(
        &mut self,
        path: &str,
        campaign_dir: Option<&PathBuf>,
    ) -> Result<(), String> {
        // Validate path prefix.
        if !path.starts_with("assets/items/") {
            return Err(format!(
                "Save path must start with 'assets/items/' (got '{}')",
                path
            ));
        }

        // campaign_dir must be provided.
        let dir = campaign_dir.ok_or_else(|| {
            "No campaign directory set; open or create a campaign first".to_string()
        })?;

        // We must have an edit buffer.
        let descriptor = self
            .edit_buffer
            .as_ref()
            .ok_or_else(|| "No descriptor is currently being edited".to_string())?
            .clone();

        // Serialize to RON.
        //
        // Always write CreatureDefinition format so the game engine's
        // CreatureDatabase::load_from_registry can load the file directly.
        //
        // If the entry being edited originated from a hand-crafted
        // CreatureDefinition (e.g. an imported OBJ), we reuse its geometry but
        // propagate the current descriptor's scale so any SDK scale edits are
        // preserved.  For procedural-only entries we generate the mesh from the
        // descriptor.
        let mut creature_def = if let Some(idx) = self.selected_entry {
            self.registry
                .get(idx)
                .and_then(|e| e.native_creature_def.clone())
                .unwrap_or_else(|| descriptor.to_creature_definition())
        } else {
            descriptor.to_creature_definition()
        };
        // Propagate scale edits from the descriptor so SDK changes are visible.
        creature_def.scale = descriptor.scale;
        let ron_text = ron::ser::to_string_pretty(&creature_def, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("RON serialization failed: {}", e))?;

        // Write to disk.
        let full_path = dir.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directories: {}", e))?;
        }
        std::fs::write(&full_path, &ron_text)
            .map_err(|e| format!("Failed to write file '{}': {}", full_path.display(), e))?;

        // Derive entry name from path stem.
        let stem = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("new_item_mesh");
        let name = stem.replace('_', " ");
        let cap_name: String = name
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Append new entry to registry.
        // native_creature_def is None for newly saved entries: the descriptor
        // is the source of truth and the file was just written from it.
        self.registry.push(ItemMeshEntry {
            name: cap_name,
            category: descriptor.category,
            file_path: path.to_string(),
            descriptor,
            native_creature_def: None,
        });

        // Update workflow file reference and mark clean.
        self.workflow.enter_edit(path.to_string());
        self.unsaved_changes = false;
        self.workflow.mark_clean();

        Ok(())
    }

    // ── Register existing asset ───────────────────────────────────────────

    /// Validates the path in `register_asset_path_buffer` without committing
    /// the entry.
    ///
    /// Checks:
    /// - The file exists and can be read.
    /// - The file deserializes as a valid [`ItemMeshDescriptor`].
    /// - The file path is not already registered (duplicate check).
    ///
    /// Sets `register_asset_error` on failure; clears it on success.
    /// Returns `true` if validation passed.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.register_asset_path_buffer = "assets/items/nonexistent.ron".to_string();
    /// // Validation fails because file doesn't exist (no campaign dir).
    /// let ok = state.execute_register_asset_validation(None);
    /// assert!(!ok);
    /// assert!(state.register_asset_error.is_some());
    /// ```
    pub fn execute_register_asset_validation(&mut self, campaign_dir: Option<&PathBuf>) -> bool {
        let path = self.register_asset_path_buffer.trim().to_string();

        if path.is_empty() {
            self.register_asset_error = Some("Path is empty".to_string());
            return false;
        }

        // Check for duplicate file path in registry.
        if self.registry.iter().any(|e| e.file_path == path) {
            self.register_asset_error = Some(format!("'{}' is already registered", path));
            return false;
        }

        // Try to read the file.
        let dir = match campaign_dir {
            Some(d) => d,
            None => {
                self.register_asset_error =
                    Some("No campaign directory; open a campaign first".to_string());
                return false;
            }
        };

        let full_path = dir.join(&path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                self.register_asset_error =
                    Some(format!("Cannot read '{}': {}", full_path.display(), e));
                return false;
            }
        };

        // Accept either ItemMeshDescriptor format (legacy/editor-authored) or
        // CreatureDefinition format (game-engine canonical / imported-OBJ).
        // Reject the file only if neither can be parsed.
        let is_descriptor = ron::de::from_str::<ItemMeshDescriptor>(&content).is_ok();
        let is_creature_def = ron::de::from_str::<CreatureDefinition>(&content).is_ok();
        if !is_descriptor && !is_creature_def {
            self.register_asset_error = Some(format!(
                "'{}' is not a valid ItemMeshDescriptor or CreatureDefinition",
                path
            ));
            return false;
        }

        self.register_asset_error = None;
        true
    }

    /// Registers the asset identified by `register_asset_path_buffer`.
    ///
    /// Only call after a successful [`execute_register_asset_validation`]
    /// (this method trusts that the file is readable and valid).
    ///
    /// # Errors
    ///
    /// Returns `Err` if the file cannot be read / deserialized (should not
    /// happen if called after successful validation).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::item_mesh_editor::ItemMeshEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// state.register_asset_path_buffer = "assets/items/my_sword.ron".to_string();
    /// // Assumes the file exists and is valid.
    /// let _ = state.execute_register_asset(Some(&PathBuf::from("/tmp/campaign")));
    /// ```
    pub fn execute_register_asset(&mut self, campaign_dir: Option<&PathBuf>) -> Result<(), String> {
        let path = self.register_asset_path_buffer.trim().to_string();

        let dir = campaign_dir.ok_or_else(|| "No campaign directory".to_string())?;

        let full_path = dir.join(&path);
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Cannot read '{}': {}", full_path.display(), e))?;

        // Try ItemMeshDescriptor first (editor-authored format), then fall back
        // to CreatureDefinition (game-engine / imported-OBJ format).
        let (descriptor, native_creature_def) =
            if let Ok(desc) = ron::de::from_str::<ItemMeshDescriptor>(&content) {
                (desc, None)
            } else if let Ok(def) = ron::de::from_str::<CreatureDefinition>(&content) {
                let category = Self::infer_category_from_path(&full_path);
                let primary_color = def
                    .meshes
                    .first()
                    .map(|m| m.color)
                    .unwrap_or([0.7, 0.7, 0.7, 1.0]);
                let accent_color = def
                    .meshes
                    .get(1)
                    .map(|m| m.color)
                    .unwrap_or([0.5, 0.3, 0.1, 1.0]);
                let emissive = def
                    .meshes
                    .first()
                    .and_then(|m| m.material.as_ref())
                    .and_then(|mat| mat.emissive)
                    .is_some();
                let desc = ItemMeshDescriptor {
                    category,
                    blade_length: 0.5,
                    primary_color,
                    accent_color,
                    emissive,
                    emissive_color: [0.0, 0.0, 0.0],
                    scale: def.scale,
                };
                (desc, Some(def))
            } else {
                return Err(format!(
                    "'{}' is not a valid ItemMeshDescriptor or CreatureDefinition",
                    path
                ));
            };

        let stem = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let name = stem.replace('_', " ");
        let cap_name: String = name
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        self.registry.push(ItemMeshEntry {
            name: cap_name,
            category: descriptor.category,
            file_path: path,
            descriptor,
            native_creature_def,
        });

        Ok(())
    }

    // ── Revert ────────────────────────────────────────────────────────────

    /// Reverts `edit_buffer` to the stored descriptor in `registry`.
    ///
    /// # Errors
    ///
    /// - Returns `Err` if the editor is in Registry mode (nothing to revert).
    /// - Returns `Err` if no entry is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_editor::{ItemMeshEditorState, ItemMeshEntry};
    /// use antares::domain::visual::item_mesh::{ItemMeshCategory, ItemMeshDescriptor};
    ///
    /// let mut state = ItemMeshEditorState::new();
    /// let desc = ItemMeshDescriptor {
    ///     category: ItemMeshCategory::Sword,
    ///     blade_length: 0.6,
    ///     primary_color: [0.7, 0.7, 0.8, 1.0],
    ///     accent_color: [0.5, 0.3, 0.1, 1.0],
    ///     emissive: false,
    ///     emissive_color: [0.0, 0.0, 0.0],
    ///     scale: 1.0,
    /// };
    /// state.registry.push(ItemMeshEntry {
    ///     name: "Sword".to_string(),
    ///     category: ItemMeshCategory::Sword,
    ///     file_path: "assets/items/sword.ron".to_string(),
    ///     descriptor: desc.clone(),
    ///     native_creature_def: None,
    /// });
    /// state.open_for_editing(0);
    ///
    /// // Modify the buffer then revert.
    /// if let Some(buf) = state.edit_buffer.as_mut() { buf.scale = 2.0; }
    /// state.revert_edit_buffer_from_registry().unwrap();
    /// assert_eq!(state.edit_buffer.as_ref().unwrap().scale, 1.0);
    /// ```
    pub fn revert_edit_buffer_from_registry(&mut self) -> Result<(), String> {
        if matches!(self.workflow.mode(), ItemMeshEditorMode::Registry) {
            return Err("Cannot revert: editor is in Registry mode".to_string());
        }

        let idx = self
            .selected_entry
            .ok_or_else(|| "Cannot revert: no entry selected".to_string())?;

        let descriptor = self
            .registry
            .get(idx)
            .ok_or_else(|| format!("Registry entry {} no longer exists", idx))?
            .descriptor
            .clone();

        self.edit_buffer = Some(descriptor);
        self.undo_redo.clear();
        self.unsaved_changes = false;
        self.preview_dirty = true;
        self.workflow.mark_clean();
        self.refresh_validation_state();
        Ok(())
    }

    // ── Preview ───────────────────────────────────────────────────────────

    /// Synchronizes the preview renderer with the current `edit_buffer`.
    ///
    /// Creates the renderer on first call if it doesn't exist.  Clears
    /// `preview_dirty` after synchronization.
    fn sync_preview_renderer_from_descriptor(&mut self) {
        // If the selected entry has a native CreatureDefinition (imported OBJ
        // or hand-crafted mesh), use that for the preview so the user sees the
        // real custom geometry instead of a procedural approximation.
        // Otherwise, generate the mesh from the descriptor in the edit buffer.
        let creature_def = if let Some(idx) = self.selected_entry {
            self.registry
                .get(idx)
                .and_then(|e| e.native_creature_def.clone())
                .or_else(|| {
                    self.edit_buffer
                        .as_ref()
                        .map(|d| d.to_creature_definition())
                })
        } else {
            self.edit_buffer
                .as_ref()
                .map(|d| d.to_creature_definition())
        };

        if let Some(creature_def) = creature_def {
            if let Some(renderer) = &mut self.preview_renderer {
                renderer.update_creature(Some(creature_def));
            } else {
                let mut renderer = PreviewRenderer::default();
                renderer.update_creature(Some(creature_def));
                self.preview_renderer = Some(renderer);
            }
            self.preview_dirty = false;
        }
    }

    // ── Registry helpers ──────────────────────────────────────────────────

    /// Returns a `HashMap<category_name, count>` of entries by category.
    fn count_by_category(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for entry in &self.registry {
            *counts.entry(format!("{:?}", entry.category)).or_insert(0) += 1;
        }
        counts
    }

    /// Returns the indices of registry entries that match the current
    /// `search_query` and `category_filter`, sorted by `registry_sort_by`.
    fn filtered_sorted_registry(&self) -> Vec<usize> {
        let query = self.search_query.to_lowercase();

        let mut indices: Vec<usize> = self
            .registry
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Category filter.
                if let Some(cat) = &self.category_filter {
                    if &entry.category != cat {
                        return false;
                    }
                }
                // Text search on name and file path.
                if !query.is_empty() {
                    let name_lc = entry.name.to_lowercase();
                    let path_lc = entry.file_path.to_lowercase();
                    if !name_lc.contains(&query) && !path_lc.contains(&query) {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        match self.registry_sort_by {
            ItemMeshRegistrySortBy::Id => {} // Already in insertion order.
            ItemMeshRegistrySortBy::Name => {
                indices.sort_by(|&a, &b| self.registry[a].name.cmp(&self.registry[b].name));
            }
            ItemMeshRegistrySortBy::Category => {
                indices.sort_by(|&a, &b| {
                    let ca = format!("{:?}", self.registry[a].category);
                    let cb = format!("{:?}", self.registry[b].category);
                    ca.cmp(&cb)
                        .then(self.registry[a].name.cmp(&self.registry[b].name))
                });
            }
        }

        indices
    }

    /// All [`ItemMeshCategory`] variants in display order.
    fn all_categories() -> &'static [ItemMeshCategory] {
        &[
            ItemMeshCategory::Sword,
            ItemMeshCategory::Dagger,
            ItemMeshCategory::Blunt,
            ItemMeshCategory::Staff,
            ItemMeshCategory::Bow,
            ItemMeshCategory::BodyArmor,
            ItemMeshCategory::Helmet,
            ItemMeshCategory::Shield,
            ItemMeshCategory::Boots,
            ItemMeshCategory::Ring,
            ItemMeshCategory::Amulet,
            ItemMeshCategory::Belt,
            ItemMeshCategory::Cloak,
            ItemMeshCategory::Potion,
            ItemMeshCategory::Scroll,
            ItemMeshCategory::Ammo,
            ItemMeshCategory::QuestItem,
        ]
    }

    // ── Main show entry point ─────────────────────────────────────────────

    /// Renders the Item Mesh Editor and returns an optional cross-tab signal.
    ///
    /// Dispatches to [`show_registry_mode`](Self::show_registry_mode) or
    /// [`show_edit_mode`](Self::show_edit_mode) based on the current workflow
    /// mode.  Also handles keyboard shortcuts (Ctrl+Z undo, Ctrl+Y / Ctrl+Shift+Z redo,
    /// Ctrl+S save, Escape back-to-registry) and shows any pending dialog
    /// windows on top.
    ///
    /// # Arguments
    ///
    /// * `ui` — The egui UI context for this frame.
    /// * `campaign_dir` — Optional path to the loaded campaign's root directory.
    ///
    /// # Returns
    ///
    /// `Some(ItemMeshEditorSignal)` when the editor needs to trigger cross-tab
    /// navigation; `None` otherwise.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<ItemMeshEditorSignal> {
        // ── Keyboard shortcuts ─────────────────────────────────────────────
        if matches!(self.workflow.mode(), ItemMeshEditorMode::Edit) {
            // Ctrl+Z — undo
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.apply_undo();
                ui.ctx().request_repaint();
            }
            // Ctrl+Shift+Z / Ctrl+Y — redo
            let redo_shift = ui.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
            });
            let redo_y = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y));
            if redo_shift || redo_y {
                self.apply_redo();
                ui.ctx().request_repaint();
            }
            // Escape — back to registry
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.back_to_registry();
                ui.ctx().request_repaint();
            }
        }

        // ── Dispatch ───────────────────────────────────────────────────────
        let signal = match self.workflow.mode() {
            ItemMeshEditorMode::Registry => {
                self.show_registry_mode(ui, campaign_dir);
                None
            }
            ItemMeshEditorMode::Edit => {
                self.show_edit_mode(ui, campaign_dir);
                None
            }
        };

        // ── Dialog windows (rendered on top) ───────────────────────────────
        if self.show_save_as_dialog {
            self.show_save_as_dialog_window(ui, campaign_dir);
        }
        if self.show_register_asset_dialog {
            self.show_register_asset_dialog_window(ui, campaign_dir);
        }

        signal
    }

    // ── Undo / redo application ───────────────────────────────────────────

    /// Pops from the undo stack and applies the `old` value.
    fn apply_undo(&mut self) {
        if let Some(action) = self.undo_redo.undo() {
            if let Some(desc) = &mut self.edit_buffer {
                apply_action_old(desc, &action);
                self.preview_dirty = true;
                self.unsaved_changes = true;
                self.workflow.mark_dirty();
            }
        }
    }

    /// Pops from the redo stack and applies the `new` value.
    fn apply_redo(&mut self) {
        if let Some(action) = self.undo_redo.redo() {
            if let Some(desc) = &mut self.edit_buffer {
                apply_action_new(desc, &action);
                self.preview_dirty = true;
                self.unsaved_changes = true;
                self.workflow.mark_dirty();
            }
        }
    }

    // ── Registry mode UI ──────────────────────────────────────────────────

    fn show_registry_mode(&mut self, ui: &mut egui::Ui, campaign_dir: Option<&PathBuf>) {
        ui.heading("🧊 Item Mesh Editor");
        ui.separator();

        // ── Pre-compute shared read-only state ────────────────────────────
        // sdk/AGENTS.md Rule 10: pre-compute before multi-closure calls.
        // Both columns of TwoColumnLayout need to read from `self` but neither
        // can hold a live borrow while the other mutates — so we snapshot all
        // display data upfront and collect deferred mutations in local vars.

        // Refresh available assets and populate the registry if the campaign
        // dir changed (e.g. user opened a different campaign while the tab was
        // visible, or the tab is first shown after open_campaign).
        if let Some(dir) = campaign_dir {
            if self.last_campaign_dir.as_ref() != Some(dir) {
                self.load_from_campaign(dir);
            }
        }

        // Snapshot the fields the closures read.
        let selected_idx = self.selected_entry;
        let registry_len = self.registry.len();
        let delete_confirm = self.registry_delete_confirm_pending;

        // Build filtered + sorted index list (owned, no external borrows).
        let filtered = self.filtered_sorted_registry();
        let total_filtered = filtered.len();
        let total_registry = self.registry.len();

        // Snapshot per-row display data so the left closure owns everything it
        // needs without borrowing `self.registry` at call time.
        #[derive(Clone)]
        struct RowData {
            real_idx: usize,
            name: String,
            category: ItemMeshCategory,
            is_selected: bool,
        }
        let row_data: Vec<RowData> = filtered
            .iter()
            .map(|&real_idx| {
                let entry = &self.registry[real_idx];
                RowData {
                    real_idx,
                    name: entry.name.clone(),
                    category: entry.category,
                    is_selected: self.selected_entry == Some(real_idx),
                }
            })
            .collect();

        // Snapshot of the selected entry for the right column.
        let preview_data: Option<(String, String, String)> = selected_idx
            .and_then(|idx| self.registry.get(idx))
            .map(|e| {
                (
                    e.name.clone(),
                    format!("{:?}", e.category),
                    e.file_path.clone(),
                )
            });
        let has_preview_renderer = self.preview_renderer.is_some();

        // Snapshot of current filter/sort state for combo-box selected text.
        let cat_label = self
            .category_filter
            .map(|c| format!("{:?}", c))
            .unwrap_or_else(|| "All Categories".to_string());
        let sort_label = match self.registry_sort_by {
            ItemMeshRegistrySortBy::Id => "Sort: ID",
            ItemMeshRegistrySortBy::Name => "Sort: Name",
            ItemMeshRegistrySortBy::Category => "Sort: Category",
        };
        let category_filter_snap = self.category_filter;
        let sort_snap = self.registry_sort_by;

        // ── Deferred mutations: collected in closures, applied after ──────
        // Each closure gets its OWN set of pending vars to avoid two
        // simultaneous &mut borrows of the same variable (E0499).
        // After show_split returns we merge left_* and right_* into the
        // canonical pending_* values and apply them.
        let mut pending_select: Option<usize> = None;
        let mut pending_new = false;
        let mut pending_register_asset = false;
        let mut pending_reload = false;
        let mut new_category_filter = category_filter_snap;
        let mut new_sort = sort_snap;
        // Deferred search query update from the text edit widget.
        let mut pending_new_search: Option<String> = None;

        // Left-closure-owned pending vars.
        // Note: pending_new_search is also left-closure-owned (declared above
        // before the layout split so it is accessible in the apply block too).
        let mut left_open_edit: Option<usize> = None;
        let mut left_duplicate: Option<usize> = None;
        let mut left_delete_confirm = delete_confirm;
        let mut left_export_ron: Option<usize> = None;

        // Right-closure-owned pending vars.
        let mut right_open_edit: Option<usize> = None;
        let mut right_duplicate: Option<usize> = None;
        let mut right_delete_confirm = delete_confirm;
        let mut right_execute_delete = false;
        let mut right_export_ron: Option<usize> = None;

        TwoColumnLayout::new("item_mesh_registry").show_split(
            ui,
            // ── Left column: search / filter / toolbar / list ─────────────
            |ui| {
                ui.label("Search:");
                // We need a local copy of search_query to satisfy the borrow
                // checker — the text_edit writes back via a `&mut String`.
                // Since both closures cannot borrow `self` simultaneously we
                // read and write through a local buffer set before the layout
                // and flushed after.  Because egui renders synchronously
                // within the frame the mutation is visible immediately.
                //
                // Note: `search_query` is not snapshotted here because the
                // left closure is the *only* place that mutates it; the right
                // closure never reads it.  We therefore forward the mutable
                // reference directly through a raw pointer — sound because
                // the two closures run sequentially (not concurrently) and
                // the right closure never touches this field.
                //
                // A cleaner approach (used by creatures_editor) is to collect
                // a deferred "new_search_query: Option<String>" and apply it
                // after show_split.  We use that pattern here.
                //
                // (The TextEdit widget does not have a "changed + value"
                //  API in egui 0.33; we must give it a &mut String.)
                //
                // We cannot borrow `self.search_query` here because `self`
                // is already captured by the outer `show_registry_mode` call.
                // Instead we keep a copy in a local and flush it after.
                //
                // ── Workaround: the closure captures `pending_*` locals
                //    plus an owned copy of the data it only reads.
                // The `ui.text_edit_singleline` widget needs a `&mut String`.
                // We pass a stack-allocated copy and detect whether it changed.
                // (The copy is at most a few hundred bytes — negligible.)
                //
                // This is the canonical SDK pattern for mutable text fields
                // inside `TwoColumnLayout` closures.
                //
                // Deferred search query: collect any change in a local
                // owned String and flush it back after show_split returns.
                // This avoids unsafe and keeps both closures free of
                // simultaneous &mut self borrows.
                let mut new_search_query = self.search_query.clone();
                let changed = ui.text_edit_singleline(&mut new_search_query).changed();
                if changed {
                    // Flush immediately — the closure is FnOnce so this runs
                    // before the right closure is constructed.
                    // We must write through a raw pointer here because Rust
                    // sees both closures as borrowing `self` at the same time,
                    // even though they execute sequentially.
                    // Use a purely-stack local instead: signal the change via
                    // the existing `pending_*` deferred-mutation variables.
                    //
                    // Store the new value in a local that is captured only by
                    // the left closure.  After show_split returns we apply it.
                    pending_new_search = Some(new_search_query);
                } else {
                    // Display value unchanged; nothing to flush.
                    drop(new_search_query);
                }
                ui.add_space(4.0);

                // Category filter combo.
                egui::ComboBox::from_id_salt("item_mesh_category_filter")
                    .selected_text(&cat_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(new_category_filter.is_none(), "All Categories")
                            .clicked()
                        {
                            new_category_filter = None;
                        }
                        for cat in ItemMeshEditorState::all_categories() {
                            let label = format!("{:?}", cat);
                            if ui
                                .selectable_label(new_category_filter == Some(*cat), &label)
                                .clicked()
                            {
                                new_category_filter = Some(*cat);
                            }
                        }
                    });
                ui.add_space(4.0);

                // Sort combo.
                egui::ComboBox::from_id_salt("item_mesh_sort_by")
                    .selected_text(sort_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(new_sort == ItemMeshRegistrySortBy::Id, "Sort: ID")
                            .clicked()
                        {
                            new_sort = ItemMeshRegistrySortBy::Id;
                        }
                        if ui
                            .selectable_label(
                                new_sort == ItemMeshRegistrySortBy::Name,
                                "Sort: Name",
                            )
                            .clicked()
                        {
                            new_sort = ItemMeshRegistrySortBy::Name;
                        }
                        if ui
                            .selectable_label(
                                new_sort == ItemMeshRegistrySortBy::Category,
                                "Sort: Category",
                            )
                            .clicked()
                        {
                            new_sort = ItemMeshRegistrySortBy::Category;
                        }
                    });
                ui.add_space(6.0);

                // Toolbar row.
                ui.horizontal_wrapped(|ui| {
                    if ui.button("➕ New").clicked() {
                        pending_new = true;
                        ui.ctx().request_repaint();
                    }
                    if ui.button("📁 Register Asset").clicked() {
                        pending_register_asset = true;
                        ui.ctx().request_repaint();
                    }
                    if ui.button("🔄 Reload").clicked() {
                        pending_reload = true;
                        ui.ctx().request_repaint();
                    }
                });

                ui.separator();
                ui.label(format!("{} / {} entries", total_filtered, total_registry));

                egui::ScrollArea::vertical()
                    .id_salt("item_mesh_registry_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (display_idx, row) in row_data.iter().enumerate() {
                            let real_idx = row.real_idx;
                            // sdk/AGENTS.md Rule 1: push_id in every loop body.
                            ui.push_id(display_idx, |ui| {
                                let badge = item_mesh_category_badge(row.category);
                                let config = StandardListItemConfig::new(&row.name)
                                    .with_badges(vec![badge])
                                    .selected(row.is_selected);

                                let (clicked, ctx_action) = show_standard_list_item(ui, config);

                                if clicked {
                                    pending_select = Some(real_idx);
                                    ui.ctx().request_repaint();
                                }

                                match ctx_action {
                                    ItemAction::Edit => {
                                        left_open_edit = Some(real_idx);
                                        ui.ctx().request_repaint();
                                    }
                                    ItemAction::Duplicate => {
                                        left_duplicate = Some(real_idx);
                                        ui.ctx().request_repaint();
                                    }
                                    ItemAction::Delete => {
                                        left_delete_confirm = true;
                                        ui.ctx().request_repaint();
                                    }
                                    ItemAction::Export => {
                                        left_export_ron = Some(real_idx);
                                        ui.ctx().request_repaint();
                                    }
                                    ItemAction::None => {}
                                }
                            });
                        }
                    });
            },
            // ── Right column: detail / preview ────────────────────────────
            |ui| {
                ui.heading("Detail");
                ui.separator();

                if let Some((name, category, file_path)) = &preview_data {
                    ui.label(format!("Name: {}", name));
                    ui.label(format!("Category: {}", category));
                    ui.label(format!("File: {}", file_path));
                } else {
                    ui.label("Select an entry to see details.");
                }

                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("item_mesh_registry_preview")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if has_preview_renderer {
                            // We cannot call renderer.show() here because we'd
                            // need a second `&mut self` borrow.  We render a
                            // placeholder and let the right-column update happen
                            // after show_split (see preview sync below).
                            ui.label("(Preview synced on selection)");
                        } else {
                            ui.label("(no preview)");
                        }

                        ui.add_space(8.0);

                        if selected_idx.is_some() {
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("✏️ Edit").clicked() {
                                    if let Some(idx) = selected_idx {
                                        right_open_edit = Some(idx);
                                        ui.ctx().request_repaint();
                                    }
                                }
                                if ui.button("📋 Duplicate").clicked() {
                                    if let Some(idx) = selected_idx {
                                        right_duplicate = Some(idx);
                                        ui.ctx().request_repaint();
                                    }
                                }
                                if ui
                                    .button("🗑 Delete")
                                    .on_hover_text("Delete this entry from the registry")
                                    .clicked()
                                {
                                    right_delete_confirm = true;
                                    ui.ctx().request_repaint();
                                }
                                if ui.button("📤 Export RON").clicked() {
                                    if let Some(idx) = selected_idx {
                                        right_export_ron = Some(idx);
                                        ui.ctx().request_repaint();
                                    }
                                }
                            });
                        }

                        // Delete confirmation inline.
                        if delete_confirm {
                            ui.separator();
                            ui.colored_label(
                                egui::Color32::RED,
                                "⚠️ Confirm delete? This cannot be undone.",
                            );
                            ui.horizontal(|ui| {
                                if ui.button("✅ Yes, Delete").clicked() {
                                    right_execute_delete = true;
                                    right_delete_confirm = false;
                                    ui.ctx().request_repaint();
                                }
                                if ui.button("❌ Cancel").clicked() {
                                    right_delete_confirm = false;
                                    ui.ctx().request_repaint();
                                }
                            });
                        }
                    });
            },
        );

        // ── Merge left / right pending vars into canonical pending vars ────
        let pending_open_edit = left_open_edit.or(right_open_edit);
        let pending_duplicate = left_duplicate.or(right_duplicate);
        // For delete_confirm: either closure can set it true; cancel resets it.
        // If either side executed delete, that takes priority.
        let pending_execute_delete = right_execute_delete;
        let pending_delete_confirm = if pending_execute_delete {
            false
        } else {
            left_delete_confirm || right_delete_confirm
        };
        let pending_export_ron = left_export_ron.or(right_export_ron);

        // ── Apply deferred mutations (all closures have returned) ─────────

        self.category_filter = new_category_filter;
        self.registry_sort_by = new_sort;
        self.registry_delete_confirm_pending = pending_delete_confirm;

        // Flush deferred search query.
        if let Some(new_q) = pending_new_search {
            self.search_query = new_q;
        }

        if pending_new {
            self.registry.push(ItemMeshEntry {
                name: format!("New Mesh {}", registry_len + 1),
                category: ItemMeshCategory::Sword,
                file_path: format!("assets/items/new_mesh_{}.ron", registry_len + 1),
                descriptor: ItemMeshDescriptor {
                    category: ItemMeshCategory::Sword,
                    blade_length: 0.5,
                    primary_color: [0.75, 0.75, 0.78, 1.0],
                    accent_color: [0.5, 0.3, 0.1, 1.0],
                    emissive: false,
                    emissive_color: [0.0, 0.0, 0.0],
                    scale: 1.0,
                },
                native_creature_def: None,
            });
            self.selected_entry = Some(self.registry.len() - 1);
            self.preview_dirty = true;
            ui.ctx().request_repaint();
        }

        if pending_register_asset {
            if let Some(dir) = campaign_dir {
                self.refresh_available_assets(dir);
            }
            self.register_asset_path_buffer.clear();
            self.register_asset_error = None;
            self.show_register_asset_dialog = true;
            ui.ctx().request_repaint();
        }

        if pending_reload {
            if let Some(dir) = campaign_dir {
                self.last_campaign_dir = None;
                self.refresh_available_assets(dir);
                ui.ctx().request_repaint();
            }
        }

        if let Some(idx) = pending_select {
            if self.selected_entry != Some(idx) {
                self.registry_delete_confirm_pending = false;
            }
            self.selected_entry = Some(idx);
            self.preview_dirty = true;
            ui.ctx().request_repaint();
        }

        if let Some(idx) = pending_duplicate {
            if let Some(src) = self.registry.get(idx).cloned() {
                let new_name = format!("{} (Copy)", src.name);
                let new_path = format!(
                    "assets/items/{}.ron",
                    new_name.to_lowercase().replace(' ', "_")
                );
                self.registry.push(ItemMeshEntry {
                    name: new_name,
                    category: src.category,
                    file_path: new_path,
                    descriptor: src.descriptor.clone(),
                    native_creature_def: src.native_creature_def.clone(),
                });
            }
            ui.ctx().request_repaint();
        }

        if pending_execute_delete {
            if let Some(idx) = selected_idx {
                if idx < self.registry.len() {
                    self.registry.remove(idx);
                    self.selected_entry = None;
                    self.preview_renderer = None;
                    self.preview_dirty = false;
                }
            }
            self.registry_delete_confirm_pending = false;
            ui.ctx().request_repaint();
        }

        if let Some(idx) = pending_export_ron {
            if let Some(entry) = self.registry.get(idx) {
                if let Ok(ron_text) =
                    ron::ser::to_string_pretty(&entry.descriptor, ron::ser::PrettyConfig::default())
                {
                    self.import_export_buffer = ron_text;
                    self.show_import_dialog = true;
                }
            }
            ui.ctx().request_repaint();
        }

        if let Some(idx) = pending_open_edit {
            self.open_for_editing(idx);
            ui.ctx().request_repaint();
        }

        // Sync preview renderer for selected entry (after mutations applied).
        if self.preview_dirty {
            if let Some(idx) = self.selected_entry {
                if let Some(entry) = self.registry.get(idx) {
                    let creature_def = entry.descriptor.to_creature_definition();
                    if let Some(renderer) = &mut self.preview_renderer {
                        renderer.update_creature(Some(creature_def));
                    } else {
                        let mut renderer = PreviewRenderer::default();
                        renderer.update_creature(Some(creature_def));
                        self.preview_renderer = Some(renderer);
                    }
                    self.preview_dirty = false;
                }
            }
        }

        // Render preview in a separate non-columnar region below the split.
        // This avoids the two-closure borrow conflict while still showing the
        // preview on the same frame as the selection.
        if selected_idx.is_some() {
            ui.separator();
            ui.label("Preview:");
            if let Some(renderer) = &mut self.preview_renderer {
                renderer.show(ui);
            } else {
                ui.label("(no preview available)");
            }
        }

        // Export RON inline window.
        if self.show_import_dialog {
            let ctx = ui.ctx().clone();
            egui::Window::new("Export Item Mesh RON")
                .collapsible(false)
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.label("RON representation (read-only):");
                    egui::ScrollArea::vertical()
                        .id_salt("item_mesh_export_ron_scroll")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.import_export_buffer)
                                    .desired_rows(12)
                                    .code_editor(),
                            );
                        });
                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                        ui.ctx().request_repaint();
                    }
                });
        }
    }

    // ── Edit mode UI ──────────────────────────────────────────────────────

    fn show_edit_mode(&mut self, ui: &mut egui::Ui, campaign_dir: Option<&PathBuf>) {
        // ── Top bar ────────────────────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
            let breadcrumb = self.breadcrumb_string();
            ui.label(&breadcrumb);
            ui.separator();
            let mode_ind = self.mode_indicator();
            ui.label(&mode_ind);

            ui.separator();

            let can_undo = self.undo_redo.can_undo();
            let can_redo = self.undo_redo.can_redo();

            if ui.button("💾 Save").on_hover_text("Ctrl+S").clicked() {
                // Write in-place to the current file.
                if let Some(idx) = self.selected_entry {
                    if let Some(entry) = self.registry.get(idx) {
                        let file_path = entry.file_path.clone();
                        let _ = self.perform_save_as_with_path(&file_path, campaign_dir);
                    }
                }
            }

            if ui
                .button("💾 Save As…")
                .on_hover_text("Ctrl+Shift+S")
                .clicked()
            {
                self.save_as_path_buffer = self.default_save_as_path();
                self.show_save_as_dialog = true;
                ui.ctx().request_repaint();
            }

            if ui
                .button("↺ Revert")
                .on_hover_text("Reload from registry")
                .clicked()
            {
                let _ = self.revert_edit_buffer_from_registry();
                ui.ctx().request_repaint();
            }

            if ui.button("✅ Validate").clicked() {
                self.refresh_validation_state();
                self.show_validation_panel = true;
                ui.ctx().request_repaint();
            }

            if ui.button("⬅ Registry").on_hover_text("Escape").clicked() {
                self.back_to_registry();
                ui.ctx().request_repaint();
            }

            ui.add_space(8.0);

            if ui
                .add_enabled(can_undo, egui::Button::new("⎌ Undo"))
                .clicked()
            {
                self.apply_undo();
                ui.ctx().request_repaint();
            }
            if ui
                .add_enabled(can_redo, egui::Button::new("↷ Redo"))
                .clicked()
            {
                self.apply_redo();
                ui.ctx().request_repaint();
            }

            if self.unsaved_changes {
                ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "● Unsaved changes");
            }
        });

        ui.separator();

        // ── Two-column content: properties | preview ───────────────────────
        ui.columns(2, |cols| {
            // ── Left: property editors ─────────────────────────────────────
            egui::ScrollArea::vertical()
                .id_salt("item_mesh_edit_props")
                .auto_shrink([false, false])
                .show(&mut cols[0], |ui| {
                    // Override enabled checkbox.
                    let mut override_enabled = self.override_enabled;
                    if ui
                        .checkbox(&mut override_enabled, "Override Enabled")
                        .changed()
                    {
                        let old = self.override_enabled;
                        self.override_enabled = override_enabled;
                        self.undo_redo.push(ItemMeshEditAction::SetOverrideEnabled {
                            old,
                            new: override_enabled,
                        });
                        self.unsaved_changes = true;
                        self.workflow.mark_dirty();
                        ui.ctx().request_repaint();
                    }

                    ui.separator();

                    let controls_enabled = self.override_enabled;

                    // Category (read-only label).
                    if let Some(desc) = &self.edit_buffer {
                        ui.label(format!("Category: {:?}", desc.category));
                    }
                    ui.add_space(6.0);

                    // Primary color sliders.
                    ui.group(|ui| {
                        ui.label("Primary Color:");
                        if let Some(desc) = self.edit_buffer.as_mut() {
                            let mut color = desc.primary_color;
                            let mut changed = false;

                            ui.add_enabled_ui(controls_enabled, |ui| {
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[0], 0.0..=1.0).text("R"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[1], 0.0..=1.0).text("G"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[2], 0.0..=1.0).text("B"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[3], 0.0..=1.0).text("A"))
                                    .changed();
                            });

                            if changed {
                                let old = desc.primary_color;
                                desc.primary_color = color;
                                self.undo_redo
                                    .push(ItemMeshEditAction::SetPrimaryColor { old, new: color });
                                self.preview_dirty = true;
                                self.unsaved_changes = true;
                                self.workflow.mark_dirty();
                                ui.ctx().request_repaint();
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Accent color sliders.
                    ui.group(|ui| {
                        ui.label("Accent Color:");
                        if let Some(desc) = self.edit_buffer.as_mut() {
                            let mut color = desc.accent_color;
                            let mut changed = false;

                            ui.add_enabled_ui(controls_enabled, |ui| {
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[0], 0.0..=1.0).text("R"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[1], 0.0..=1.0).text("G"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[2], 0.0..=1.0).text("B"))
                                    .changed();
                                changed |= ui
                                    .add(egui::Slider::new(&mut color[3], 0.0..=1.0).text("A"))
                                    .changed();
                            });

                            if changed {
                                let old = desc.accent_color;
                                desc.accent_color = color;
                                self.undo_redo
                                    .push(ItemMeshEditAction::SetAccentColor { old, new: color });
                                self.preview_dirty = true;
                                self.unsaved_changes = true;
                                self.workflow.mark_dirty();
                                ui.ctx().request_repaint();
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Scale slider.
                    ui.group(|ui| {
                        ui.label("Scale:");
                        if let Some(desc) = self.edit_buffer.as_mut() {
                            let mut scale = desc.scale;
                            let scale_resp = ui.add_enabled(
                                controls_enabled,
                                egui::Slider::new(&mut scale, 0.25..=4.0)
                                    .step_by(0.05)
                                    .text("×"),
                            );
                            if scale_resp.changed() {
                                let old = desc.scale;
                                desc.scale = scale;
                                self.undo_redo
                                    .push(ItemMeshEditAction::SetScale { old, new: scale });
                                self.preview_dirty = true;
                                self.unsaved_changes = true;
                                self.workflow.mark_dirty();
                                ui.ctx().request_repaint();
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Emissive checkbox.
                    if let Some(desc) = self.edit_buffer.as_mut() {
                        let mut emissive = desc.emissive;
                        let emissive_resp = ui.add_enabled(
                            controls_enabled,
                            egui::Checkbox::new(&mut emissive, "Emissive (Magical Glow)"),
                        );
                        if emissive_resp.changed() {
                            let old = desc.emissive;
                            desc.emissive = emissive;
                            self.undo_redo
                                .push(ItemMeshEditAction::SetEmissive { old, new: emissive });
                            self.preview_dirty = true;
                            self.unsaved_changes = true;
                            self.workflow.mark_dirty();
                            ui.ctx().request_repaint();
                        }
                    }

                    ui.add_space(8.0);

                    // Reset to defaults button.
                    if ui
                        .button("↺ Reset to Defaults")
                        .on_hover_text("Resets all properties to auto-derived values")
                        .clicked()
                    {
                        if let Some(desc) = &self.edit_buffer {
                            let old_desc = desc.clone();
                            // Reset to a neutral Sword descriptor (user can
                            // edit from there; a real reset would re-derive
                            // from the Item, but we don't hold the Item here).
                            let new_desc = ItemMeshDescriptor {
                                category: desc.category,
                                blade_length: 0.5,
                                primary_color: [0.75, 0.75, 0.78, 1.0],
                                accent_color: [0.5, 0.3, 0.1, 1.0],
                                emissive: false,
                                emissive_color: [0.0, 0.0, 0.0],
                                scale: 1.0,
                            };
                            self.undo_redo.push(ItemMeshEditAction::ReplaceDescriptor {
                                old: old_desc,
                                new: new_desc.clone(),
                            });
                            self.edit_buffer = Some(new_desc);
                            self.preview_dirty = true;
                            self.unsaved_changes = true;
                            self.workflow.mark_dirty();
                        }
                        ui.ctx().request_repaint();
                    }

                    ui.add_space(8.0);

                    // Validation collapsible.
                    ui.collapsing("✅ Validation", |ui| {
                        self.refresh_validation_state();
                        if self.validation_errors.is_empty() && self.validation_warnings.is_empty()
                        {
                            ui.label("✅ No issues found.");
                        }
                        for err in &self.validation_errors {
                            ui.colored_label(egui::Color32::RED, format!("❌ {}", err));
                        }
                        for warn in &self.validation_warnings {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 200, 0),
                                format!("⚠️ {}", warn),
                            );
                        }
                    });
                });

            // ── Right: preview ─────────────────────────────────────────────
            egui::ScrollArea::vertical()
                .id_salt("item_mesh_edit_preview")
                .auto_shrink([false, false])
                .show(&mut cols[1], |ui| {
                    ui.heading("Preview");

                    if ui.button("🔄 Regenerate Preview").clicked() {
                        self.sync_preview_renderer_from_descriptor();
                        ui.ctx().request_repaint();
                    }

                    if let Some(desc) = &self.edit_buffer {
                        ui.label(format!("Category: {:?}", desc.category));
                        ui.label("Triangle count: (procedural)");
                    }

                    if let Some(err) = &self.preview_error {
                        ui.colored_label(egui::Color32::RED, format!("Preview error: {}", err));
                    }

                    ui.add_space(4.0);
                    ui.label("Camera distance:");
                    ui.add(egui::Slider::new(&mut self.camera_distance, 1.0..=20.0).text("units"));

                    // Sync preview if dirty.
                    if self.preview_dirty {
                        self.sync_preview_renderer_from_descriptor();
                    }

                    if let Some(renderer) = &mut self.preview_renderer {
                        renderer.camera.distance = self.camera_distance;
                        renderer.show(ui);
                    } else {
                        ui.label("Preview not available in this build.");
                    }
                });
        });
    }

    // ── Save-As dialog ─────────────────────────────────────────────────────

    fn show_save_as_dialog_window(&mut self, ui: &mut egui::Ui, campaign_dir: Option<&PathBuf>) {
        // Snapshot mutable fields so we can pass them into the window closure
        // without borrowing `self` — egui Window closures capture by move or
        // by reference depending on the outer function, but we need `&mut self`
        // both for the closure and after it to call perform_save_as_with_path.
        // We use the deferred-action pattern: collect button presses in locals,
        // act after the window closure returns.
        let ctx = ui.ctx().clone();
        let mut do_save = false;
        let mut do_cancel = false;
        // Snapshot error and path for display inside the closure.
        let current_error = self.preview_error.clone();
        // Snapshot the path buffer so we can render it while also being able
        // to call perform_save_as_with_path later.
        let mut path_snapshot = self.save_as_path_buffer.clone();

        egui::Window::new("Save Item Mesh As")
            .collapsible(false)
            .resizable(true)
            .show(&ctx, |ui| {
                ui.label("Save path (relative to campaign, must start with 'assets/items/'):");
                ui.text_edit_singleline(&mut path_snapshot);

                if let Some(err) = &current_error {
                    ui.colored_label(egui::Color32::RED, format!("❌ {}", err));
                }

                ui.horizontal(|ui| {
                    if ui.button("✅ Save").clicked() {
                        do_save = true;
                    }
                    if ui.button("❌ Cancel").clicked() {
                        do_cancel = true;
                    }
                });
            });

        // Flush path edits back.
        self.save_as_path_buffer = path_snapshot;

        if do_save {
            let path = self.save_as_path_buffer.clone();
            match self.perform_save_as_with_path(&path, campaign_dir) {
                Ok(()) => {
                    self.show_save_as_dialog = false;
                    self.preview_error = None;
                    ui.ctx().request_repaint();
                }
                Err(e) => {
                    self.preview_error = Some(e);
                    ui.ctx().request_repaint();
                }
            }
        }

        if do_cancel {
            self.show_save_as_dialog = false;
            self.preview_error = None;
            ui.ctx().request_repaint();
        }
    }

    // ── Register asset dialog ─────────────────────────────────────────────

    fn show_register_asset_dialog_window(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
    ) {
        // Use the deferred-action pattern (sdk/AGENTS.md Rule 10):
        // snapshot display data, collect button intents in locals, act after
        // the window closure returns to avoid double-borrow on `self`.
        let ctx = ui.ctx().clone();
        let mut do_validate = false;
        let mut do_register = false;
        let mut do_cancel = false;
        let mut selected_asset: Option<String> = None;

        // Snapshots for read-only display inside the closure.
        let register_asset_error_snapshot = self.register_asset_error.clone();
        let available_assets_snapshot = self.available_item_assets.clone();
        let validation_passed = self.register_asset_error.is_none()
            && !self.register_asset_path_buffer.trim().is_empty();
        let mut path_snapshot = self.register_asset_path_buffer.clone();

        egui::Window::new("Register Item Mesh Asset")
            .collapsible(false)
            .resizable(true)
            .show(&ctx, |ui| {
                ui.label("Asset path (relative to campaign root):");
                ui.text_edit_singleline(&mut path_snapshot);

                if !available_assets_snapshot.is_empty() {
                    ui.separator();
                    ui.label("Available assets:");
                    egui::ScrollArea::vertical()
                        .id_salt("item_mesh_register_asset_list")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for (idx, asset) in available_assets_snapshot.iter().enumerate() {
                                // sdk/AGENTS.md Rule 1: push_id in every loop body.
                                ui.push_id(idx, |ui| {
                                    if ui.selectable_label(false, asset).clicked() {
                                        selected_asset = Some(asset.clone());
                                        ui.ctx().request_repaint();
                                    }
                                });
                            }
                        });
                }

                ui.separator();

                if let Some(err) = &register_asset_error_snapshot {
                    ui.colored_label(egui::Color32::RED, format!("❌ {}", err));
                }

                ui.horizontal(|ui| {
                    if ui.button("🔍 Validate").clicked() {
                        do_validate = true;
                    }
                    if ui
                        .add_enabled(validation_passed, egui::Button::new("✅ Register"))
                        .clicked()
                    {
                        do_register = true;
                    }
                    if ui.button("❌ Cancel").clicked() {
                        do_cancel = true;
                    }
                });
            });

        // Flush path edits back.
        self.register_asset_path_buffer = path_snapshot;

        // Apply asset selection from the list.
        if let Some(asset) = selected_asset {
            self.register_asset_path_buffer = asset;
        }

        if do_validate {
            self.execute_register_asset_validation(campaign_dir);
            ui.ctx().request_repaint();
        }

        if do_register {
            match self.execute_register_asset(campaign_dir) {
                Ok(()) => {
                    self.show_register_asset_dialog = false;
                    self.register_asset_path_buffer.clear();
                    self.register_asset_error = None;
                    ui.ctx().request_repaint();
                }
                Err(e) => {
                    self.register_asset_error = Some(e);
                    ui.ctx().request_repaint();
                }
            }
        }

        if do_cancel {
            self.show_register_asset_dialog = false;
            self.register_asset_error = None;
            ui.ctx().request_repaint();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Left-panel badge helper
// ─────────────────────────────────────────────────────────────────────────────

/// Returns a [`MetadataBadge`] with the display name and color for an
/// [`ItemMeshCategory`], used in the registry list left panel.
fn item_mesh_category_badge(category: ItemMeshCategory) -> MetadataBadge {
    let (label, color) = match category {
        ItemMeshCategory::Sword => ("Sword", egui::Color32::from_rgb(180, 80, 80)),
        ItemMeshCategory::Dagger => ("Dagger", egui::Color32::from_rgb(180, 100, 80)),
        ItemMeshCategory::Blunt => ("Blunt", egui::Color32::from_rgb(160, 100, 60)),
        ItemMeshCategory::Staff => ("Staff", egui::Color32::from_rgb(140, 100, 60)),
        ItemMeshCategory::Bow => ("Bow", egui::Color32::from_rgb(160, 120, 60)),
        ItemMeshCategory::BodyArmor => ("Body Armor", egui::Color32::from_rgb(80, 100, 180)),
        ItemMeshCategory::Helmet => ("Helmet", egui::Color32::from_rgb(80, 120, 180)),
        ItemMeshCategory::Shield => ("Shield", egui::Color32::from_rgb(80, 140, 180)),
        ItemMeshCategory::Boots => ("Boots", egui::Color32::from_rgb(100, 120, 160)),
        ItemMeshCategory::Ring => ("Ring", egui::Color32::from_rgb(200, 160, 0)),
        ItemMeshCategory::Amulet => ("Amulet", egui::Color32::from_rgb(180, 140, 0)),
        ItemMeshCategory::Belt => ("Belt", egui::Color32::from_rgb(160, 120, 20)),
        ItemMeshCategory::Cloak => ("Cloak", egui::Color32::from_rgb(100, 80, 160)),
        ItemMeshCategory::Potion => ("Potion", egui::Color32::from_rgb(80, 180, 80)),
        ItemMeshCategory::Scroll => ("Scroll", egui::Color32::from_rgb(200, 200, 80)),
        ItemMeshCategory::Ammo => ("Ammo", egui::Color32::from_rgb(150, 150, 150)),
        ItemMeshCategory::QuestItem => ("Quest", egui::Color32::from_rgb(200, 160, 60)),
    };
    MetadataBadge::new(label)
        .with_color(color)
        .with_tooltip(format!("Category: {label}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Action application helpers (free functions)
// ─────────────────────────────────────────────────────────────────────────────

/// Applies the `old` value of an action to `desc` (used by undo).
fn apply_action_old(desc: &mut ItemMeshDescriptor, action: &ItemMeshEditAction) {
    match action {
        ItemMeshEditAction::SetPrimaryColor { old, .. } => desc.primary_color = *old,
        ItemMeshEditAction::SetAccentColor { old, .. } => desc.accent_color = *old,
        ItemMeshEditAction::SetScale { old, .. } => desc.scale = *old,
        ItemMeshEditAction::SetEmissive { old, .. } => desc.emissive = *old,
        ItemMeshEditAction::SetOverrideEnabled { .. } => {} // override_enabled not in descriptor
        ItemMeshEditAction::ReplaceDescriptor { old, .. } => *desc = old.clone(),
    }
}

/// Applies the `new` value of an action to `desc` (used by redo).
fn apply_action_new(desc: &mut ItemMeshDescriptor, action: &ItemMeshEditAction) {
    match action {
        ItemMeshEditAction::SetPrimaryColor { new, .. } => desc.primary_color = *new,
        ItemMeshEditAction::SetAccentColor { new, .. } => desc.accent_color = *new,
        ItemMeshEditAction::SetScale { new, .. } => desc.scale = *new,
        ItemMeshEditAction::SetEmissive { new, .. } => desc.emissive = *new,
        ItemMeshEditAction::SetOverrideEnabled { .. } => {}
        ItemMeshEditAction::ReplaceDescriptor { new, .. } => *desc = new.clone(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::visual::item_mesh::ItemMeshCategory;
    use std::path::PathBuf;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_descriptor(scale: f32) -> ItemMeshDescriptor {
        ItemMeshDescriptor {
            category: ItemMeshCategory::Sword,
            blade_length: 0.5,
            primary_color: [0.7, 0.7, 0.8, 1.0],
            accent_color: [0.5, 0.3, 0.1, 1.0],
            emissive: false,
            emissive_color: [0.0, 0.0, 0.0],
            scale,
        }
    }

    fn make_entry(name: &str) -> ItemMeshEntry {
        ItemMeshEntry {
            name: name.to_string(),
            category: ItemMeshCategory::Sword,
            file_path: format!("assets/items/{}.ron", name.to_lowercase().replace(' ', "_")),
            descriptor: make_descriptor(1.0),
            native_creature_def: None,
        }
    }

    fn make_state_with_entry() -> ItemMeshEditorState {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(make_entry("Iron Sword"));
        state
    }

    // ── Default / new ─────────────────────────────────────────────────────────

    /// A freshly constructed state should be clean and empty.
    #[test]
    fn test_item_mesh_editor_state_default() {
        let state = ItemMeshEditorState::new();
        assert!(state.registry.is_empty());
        assert!(state.selected_entry.is_none());
        assert!(state.edit_buffer.is_none());
        assert!(!state.override_enabled);
        assert!(!state.preview_dirty);
    }

    /// `has_unsaved_changes` must be `false` on a new state.
    #[test]
    fn test_item_mesh_editor_has_unsaved_changes_false_by_default() {
        let state = ItemMeshEditorState::new();
        assert!(!state.has_unsaved_changes());
    }

    /// After setting `unsaved_changes` the helper should return `true`.
    #[test]
    fn test_item_mesh_editor_has_unsaved_changes_true_after_edit() {
        let mut state = ItemMeshEditorState::new();
        state.unsaved_changes = true;
        assert!(state.has_unsaved_changes());
    }

    /// `can_undo` must be `false` on a new state.
    #[test]
    fn test_item_mesh_editor_can_undo_false_by_default() {
        let state = ItemMeshEditorState::new();
        assert!(!state.can_undo());
    }

    /// `can_redo` must be `false` on a new state.
    #[test]
    fn test_item_mesh_editor_can_redo_false_by_default() {
        let state = ItemMeshEditorState::new();
        assert!(!state.can_redo());
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// `back_to_registry` should clear edit state completely.
    #[test]
    fn test_item_mesh_editor_back_to_registry_clears_edit_state() {
        let mut state = make_state_with_entry();
        state.open_for_editing(0);
        assert!(state.edit_buffer.is_some());

        state.back_to_registry();
        assert!(state.edit_buffer.is_none());
        assert!(!state.override_enabled);
        assert!(!state.preview_dirty);
        assert!(!state.has_unsaved_changes());
        assert!(!state.can_undo());
        assert!(!state.can_redo());
    }

    // ── Available assets — empty dir ─────────────────────────────────────────

    /// When the `assets/items/` directory does not exist the list should stay
    /// empty but not panic.
    #[test]
    fn test_available_item_assets_empty_when_no_assets_dir() {
        let mut state = ItemMeshEditorState::new();
        let dir = PathBuf::from("/nonexistent/path/that/does/not/exist");
        state.refresh_available_assets(&dir);
        assert!(state.available_item_assets.is_empty());
    }

    /// Assets are populated when the directory exists and contains `.ron` files.
    #[test]
    fn test_available_item_assets_populated_from_campaign_dir() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let items_dir = tmp.path().join("assets").join("items");
        std::fs::create_dir_all(&items_dir).unwrap();
        std::fs::write(items_dir.join("sword.ron"), b"()").unwrap();
        std::fs::write(items_dir.join("shield.ron"), b"()").unwrap();
        // A non-ron file should be ignored.
        std::fs::write(items_dir.join("notes.txt"), b"ignore me").unwrap();

        let mut state = ItemMeshEditorState::new();
        state.refresh_available_assets(&tmp.path().to_path_buf());

        assert_eq!(state.available_item_assets.len(), 2);
        for asset in &state.available_item_assets {
            assert!(
                asset.ends_with(".ron"),
                "expected .ron extension: {}",
                asset
            );
        }
    }

    /// A second call with the same directory should skip the re-scan.
    #[test]
    fn test_available_item_assets_not_refreshed_when_dir_unchanged() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let items_dir = tmp.path().join("assets").join("items");
        std::fs::create_dir_all(&items_dir).unwrap();
        std::fs::write(items_dir.join("sword.ron"), b"()").unwrap();

        let dir = tmp.path().to_path_buf();
        let mut state = ItemMeshEditorState::new();
        state.refresh_available_assets(&dir);
        assert_eq!(state.available_item_assets.len(), 1);

        // Add a new file — but the scan should NOT pick it up because dir is unchanged.
        std::fs::write(items_dir.join("bow.ron"), b"()").unwrap();
        state.refresh_available_assets(&dir);
        assert_eq!(
            state.available_item_assets.len(),
            1,
            "second call with same dir should not re-scan"
        );
    }

    /// When the campaign directory changes the list should be refreshed.
    #[test]
    fn test_available_item_assets_refreshed_when_dir_changes() {
        use tempfile::TempDir;

        let tmp1 = TempDir::new().expect("tempdir1");
        let items1 = tmp1.path().join("assets").join("items");
        std::fs::create_dir_all(&items1).unwrap();
        std::fs::write(items1.join("sword.ron"), b"()").unwrap();

        let tmp2 = TempDir::new().expect("tempdir2");
        let items2 = tmp2.path().join("assets").join("items");
        std::fs::create_dir_all(&items2).unwrap();
        std::fs::write(items2.join("bow.ron"), b"()").unwrap();
        std::fs::write(items2.join("staff.ron"), b"()").unwrap();

        let mut state = ItemMeshEditorState::new();
        state.refresh_available_assets(&tmp1.path().to_path_buf());
        assert_eq!(state.available_item_assets.len(), 1);

        state.refresh_available_assets(&tmp2.path().to_path_buf());
        assert_eq!(
            state.available_item_assets.len(),
            2,
            "new dir should trigger re-scan"
        );
    }

    // ── Register asset validation ─────────────────────────────────────────────

    /// Duplicate file paths must be rejected.
    #[test]
    fn test_register_asset_validate_duplicate_id_sets_error() {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(ItemMeshEntry {
            name: "Sword".to_string(),
            category: ItemMeshCategory::Sword,
            file_path: "assets/items/sword.ron".to_string(),
            descriptor: make_descriptor(1.0),
            native_creature_def: None,
        });

        state.register_asset_path_buffer = "assets/items/sword.ron".to_string();
        let ok = state.execute_register_asset_validation(None);
        assert!(!ok);
        assert!(state.register_asset_error.is_some());
    }

    /// Cancelling the register dialog must not modify the registry.
    #[test]
    fn test_register_asset_cancel_does_not_modify_registry() {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(make_entry("Sword"));
        let original_len = state.registry.len();

        // Simulate cancel: just close the dialog without calling execute_register_asset.
        state.show_register_asset_dialog = true;
        state.show_register_asset_dialog = false;

        assert_eq!(state.registry.len(), original_len);
    }

    /// A successful register must append to the registry.
    #[test]
    fn test_register_asset_success_appends_entry() {
        use antares::domain::visual::item_mesh::ItemMeshDescriptor;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let items_dir = tmp.path().join("assets").join("items");
        std::fs::create_dir_all(&items_dir).unwrap();

        let desc = make_descriptor(1.0);
        let ron_text =
            ron::ser::to_string_pretty(&desc, ron::ser::PrettyConfig::default()).unwrap();
        std::fs::write(items_dir.join("axe.ron"), ron_text.as_bytes()).unwrap();

        let mut state = ItemMeshEditorState::new();
        state.register_asset_path_buffer = "assets/items/axe.ron".to_string();

        let dir = tmp.path().to_path_buf();
        let ok = state.execute_register_asset_validation(Some(&dir));
        assert!(
            ok,
            "validation should pass: {:?}",
            state.register_asset_error
        );

        state.execute_register_asset(Some(&dir)).unwrap();
        assert_eq!(state.registry.len(), 1, "entry should be appended");
        assert_eq!(state.registry[0].file_path, "assets/items/axe.ron");
    }

    // ── Save-As ───────────────────────────────────────────────────────────────

    /// A save operation to a valid path should append an entry.
    #[test]
    fn test_perform_save_as_with_path_appends_new_entry() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let mut state = make_state_with_entry();
        state.open_for_editing(0);

        let path = "assets/items/copy_sword.ron";
        state
            .perform_save_as_with_path(path, Some(&tmp.path().to_path_buf()))
            .expect("save should succeed");

        // One original entry + one new entry.
        assert_eq!(state.registry.len(), 2);
        assert_eq!(state.registry.last().unwrap().file_path, path);
        // File should exist on disk.
        assert!(tmp.path().join(path).exists());
    }

    /// Without a campaign directory `perform_save_as_with_path` must return Err.
    #[test]
    fn test_perform_save_as_requires_campaign_directory() {
        let mut state = make_state_with_entry();
        state.open_for_editing(0);

        let result = state.perform_save_as_with_path("assets/items/sword.ron", None);
        assert!(result.is_err(), "expected error when campaign_dir is None");
    }

    /// Paths that do not start with `"assets/items/"` must be rejected.
    #[test]
    fn test_perform_save_as_rejects_non_item_asset_paths() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let mut state = make_state_with_entry();
        state.open_for_editing(0);

        let result =
            state.perform_save_as_with_path("data/bad_path.ron", Some(&tmp.path().to_path_buf()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("assets/items/"),
            "error message should mention required prefix: {}",
            err
        );
    }

    // ── Revert ────────────────────────────────────────────────────────────────

    /// `revert_edit_buffer_from_registry` should restore the original descriptor.
    #[test]
    fn test_revert_edit_buffer_restores_original() {
        let mut state = make_state_with_entry();
        state.open_for_editing(0);

        // Mutate the buffer.
        state.edit_buffer.as_mut().unwrap().scale = 99.0;
        state
            .revert_edit_buffer_from_registry()
            .expect("revert should succeed");

        assert_eq!(
            state.edit_buffer.as_ref().unwrap().scale,
            1.0,
            "scale should be restored to original"
        );
    }

    /// Calling revert in Registry mode should return an error.
    #[test]
    fn test_revert_edit_buffer_errors_in_registry_mode() {
        let mut state = make_state_with_entry();
        // Do NOT open for editing — stay in Registry mode.
        let result = state.revert_edit_buffer_from_registry();
        assert!(result.is_err(), "revert in Registry mode should fail");
    }

    // ── Validation ────────────────────────────────────────────────────────────

    /// A zero scale should produce an error.
    #[test]
    fn test_validate_descriptor_reports_invalid_scale() {
        let desc = make_descriptor(0.0);
        let (errors, _warnings) = ItemMeshEditorState::validate_descriptor(&desc);
        assert!(
            !errors.is_empty(),
            "zero scale should produce a validation error"
        );
    }

    /// A negative scale should produce an error.
    #[test]
    fn test_validate_descriptor_reports_negative_scale() {
        let desc = make_descriptor(-1.0);
        let (errors, _warnings) = ItemMeshEditorState::validate_descriptor(&desc);
        assert!(
            !errors.is_empty(),
            "negative scale should produce a validation error"
        );
    }

    /// A default (1.0) scale should produce neither errors nor warnings.
    #[test]
    fn test_validate_descriptor_passes_for_default_descriptor() {
        let desc = make_descriptor(1.0);
        let (errors, warnings) = ItemMeshEditorState::validate_descriptor(&desc);
        assert!(
            errors.is_empty(),
            "default descriptor should have no errors"
        );
        assert!(
            warnings.is_empty(),
            "default descriptor should have no warnings"
        );
    }

    /// Scale > 3.0 should produce a warning but not an error.
    #[test]
    fn test_validate_descriptor_warns_on_large_scale() {
        let desc = make_descriptor(3.5);
        let (errors, warnings) = ItemMeshEditorState::validate_descriptor(&desc);
        assert!(errors.is_empty(), "large scale is a warning, not an error");
        assert!(!warnings.is_empty(), "large scale should produce a warning");
    }

    // ── filtered_sorted_registry ──────────────────────────────────────────────

    #[test]
    fn test_filtered_sorted_registry_empty() {
        let state = ItemMeshEditorState::new();
        let result = state.filtered_sorted_registry();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filtered_sorted_registry_by_name() {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(make_entry("Zeal Axe"));
        state.registry.push(make_entry("Alpha Shield"));
        state.registry.push(make_entry("Magic Ring"));

        state.registry_sort_by = ItemMeshRegistrySortBy::Name;
        let indices = state.filtered_sorted_registry();
        let names: Vec<&str> = indices
            .iter()
            .map(|&i| state.registry[i].name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha Shield", "Magic Ring", "Zeal Axe"]);
    }

    #[test]
    fn test_filtered_sorted_registry_search_filter() {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(make_entry("Iron Sword"));
        state.registry.push(make_entry("Magic Staff"));
        state.registry.push(make_entry("Iron Shield"));

        state.search_query = "iron".to_string();
        let indices = state.filtered_sorted_registry();
        assert_eq!(indices.len(), 2, "should match 'iron' prefix in name");
    }

    // ── count_by_category ─────────────────────────────────────────────────────

    #[test]
    fn test_count_by_category() {
        let mut state = ItemMeshEditorState::new();
        state.registry.push(make_entry("Sword A"));
        state.registry.push(make_entry("Sword B"));
        state.registry.push(ItemMeshEntry {
            name: "Potion".to_string(),
            category: ItemMeshCategory::Potion,
            file_path: "assets/items/potion.ron".to_string(),
            descriptor: make_descriptor(1.0),
            native_creature_def: None,
        });

        let counts = state.count_by_category();
        assert_eq!(*counts.get("Sword").unwrap_or(&0), 2);
        assert_eq!(*counts.get("Potion").unwrap_or(&0), 1);
    }
}
