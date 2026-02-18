// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Context Menu System - Phase 5.4
//!
//! Provides right-click context menu management for the creature editor.
//! Supports:
//! - Mesh context menus (add, delete, duplicate, etc.)
//! - Vertex context menus (move, delete, merge, etc.)
//! - View context menus (camera, grid, wireframe, etc.)
//! - Dynamic menu item enable/disable based on context
//! - Hierarchical menus with submenus
//! - Separator support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context menu item identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MenuItemId {
    // Mesh operations
    AddMesh,
    DeleteMesh,
    DuplicateMesh,
    RenameMesh,
    ImportMesh,
    ExportMesh,
    IsolateMesh,
    HideMesh,
    ShowAllMeshes,

    // Vertex operations
    AddVertex,
    DeleteVertex,
    DuplicateVertex,
    MergeVertices,
    SnapToGrid,
    SetVertexPosition,

    // Face operations
    AddFace,
    DeleteFace,
    FlipWinding,
    FlipNormals,
    TriangulateFaces,
    SubdivideFaces,

    // Normal operations
    RecalculateNormals,
    SmoothNormals,
    FlattenNormals,
    SetNormal,
    FlipNormal,

    // Transform operations
    ResetTransform,
    ResetPosition,
    ResetRotation,
    ResetScale,
    CenterPivot,
    SnapToOrigin,

    // View operations
    FocusSelected,
    FrameAll,
    ToggleWireframe,
    ToggleNormals,
    ToggleGrid,
    ToggleBoundingBox,
    ResetCamera,

    // Edit operations
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Duplicate,
    SelectAll,
    DeselectAll,
    InvertSelection,

    // Template operations
    ApplyTemplate,
    SaveAsTemplate,
    BrowseTemplates,

    // Validation
    ValidateMesh,
    FixErrors,
    ShowValidationReport,

    // Properties
    ShowProperties,
    EditMetadata,
}

/// Menu item type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MenuItem {
    /// Regular menu item with label and action
    Action {
        id: MenuItemId,
        label: String,
        enabled: bool,
        shortcut: Option<String>,
    },
    /// Separator line
    Separator,
    /// Submenu with children
    Submenu {
        label: String,
        enabled: bool,
        items: Vec<MenuItem>,
    },
}

impl MenuItem {
    /// Create a new action menu item
    pub fn action(id: MenuItemId, label: impl Into<String>) -> Self {
        Self::Action {
            id,
            label: label.into(),
            enabled: true,
            shortcut: None,
        }
    }

    /// Create a disabled action menu item
    pub fn disabled_action(id: MenuItemId, label: impl Into<String>) -> Self {
        Self::Action {
            id,
            label: label.into(),
            enabled: false,
            shortcut: None,
        }
    }

    /// Create an action with a keyboard shortcut hint
    pub fn action_with_shortcut(
        id: MenuItemId,
        label: impl Into<String>,
        shortcut: impl Into<String>,
    ) -> Self {
        Self::Action {
            id,
            label: label.into(),
            enabled: true,
            shortcut: Some(shortcut.into()),
        }
    }

    /// Create a separator
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Create a submenu
    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            enabled: true,
            items,
        }
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        match &mut self {
            Self::Action {
                enabled: ref mut e, ..
            } => *e = enabled,
            Self::Submenu {
                enabled: ref mut e, ..
            } => *e = enabled,
            Self::Separator => {}
        }
        self
    }

    /// Get the menu item ID if this is an action
    pub fn id(&self) -> Option<MenuItemId> {
        match self {
            Self::Action { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Action { enabled, .. } => *enabled,
            Self::Submenu { enabled, .. } => *enabled,
            Self::Separator => false,
        }
    }

    /// Check if this is a separator
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }
}

/// Context type determines which menu to show
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// Clicked on empty viewport
    Viewport,
    /// Clicked on a mesh
    Mesh,
    /// Clicked on a vertex
    Vertex,
    /// Clicked on a face/triangle
    Face,
    /// Clicked in mesh list
    MeshList,
    /// Clicked in vertex editor
    VertexEditor,
    /// Clicked in index editor
    IndexEditor,
}

/// Context menu state and selection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MenuContext {
    /// Number of meshes selected
    pub selected_meshes: usize,
    /// Number of vertices selected
    pub selected_vertices: usize,
    /// Number of faces selected
    pub selected_faces: usize,
    /// Whether undo is available
    pub can_undo: bool,
    /// Whether redo is available
    pub can_redo: bool,
    /// Whether clipboard has content
    pub has_clipboard: bool,
    /// Current mesh index (if applicable)
    pub current_mesh_index: Option<usize>,
    /// Total number of meshes
    pub total_meshes: usize,
}

impl MenuContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with selected mesh count
    pub fn with_meshes(count: usize) -> Self {
        Self {
            selected_meshes: count,
            ..Default::default()
        }
    }

    /// Create context with selected vertex count
    pub fn with_vertices(count: usize) -> Self {
        Self {
            selected_vertices: count,
            ..Default::default()
        }
    }

    /// Has any selection
    pub fn has_selection(&self) -> bool {
        self.selected_meshes > 0 || self.selected_vertices > 0 || self.selected_faces > 0
    }

    /// Has mesh selection
    pub fn has_mesh_selection(&self) -> bool {
        self.selected_meshes > 0
    }

    /// Has vertex selection
    pub fn has_vertex_selection(&self) -> bool {
        self.selected_vertices > 0
    }

    /// Has multiple vertices selected
    pub fn has_multiple_vertices(&self) -> bool {
        self.selected_vertices > 1
    }
}

/// Context menu manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuManager {
    menus: HashMap<ContextType, Vec<MenuItem>>,
}

impl Default for ContextMenuManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextMenuManager {
    /// Create a new context menu manager with default menus
    pub fn new() -> Self {
        let mut manager = Self {
            menus: HashMap::new(),
        };
        manager.register_defaults();
        manager
    }

    /// Register default context menus
    fn register_defaults(&mut self) {
        // Viewport context menu
        self.register(ContextType::Viewport, Self::create_viewport_menu());

        // Mesh context menu
        self.register(ContextType::Mesh, Self::create_mesh_menu());

        // Vertex context menu
        self.register(ContextType::Vertex, Self::create_vertex_menu());

        // Face context menu
        self.register(ContextType::Face, Self::create_face_menu());

        // Mesh list context menu
        self.register(ContextType::MeshList, Self::create_mesh_list_menu());

        // Vertex editor context menu
        self.register(ContextType::VertexEditor, Self::create_vertex_editor_menu());

        // Index editor context menu
        self.register(ContextType::IndexEditor, Self::create_index_editor_menu());
    }

    /// Create viewport context menu
    fn create_viewport_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action(MenuItemId::AddMesh, "Add Mesh"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::Undo, "Undo", "Ctrl+Z"),
            MenuItem::action_with_shortcut(MenuItemId::Redo, "Redo", "Ctrl+Y"),
            MenuItem::separator(),
            MenuItem::submenu(
                "View",
                vec![
                    MenuItem::action_with_shortcut(MenuItemId::ToggleGrid, "Toggle Grid", "G"),
                    MenuItem::action_with_shortcut(
                        MenuItemId::ToggleWireframe,
                        "Toggle Wireframe",
                        "W",
                    ),
                    MenuItem::action_with_shortcut(
                        MenuItemId::ToggleNormals,
                        "Toggle Normals",
                        "N",
                    ),
                    MenuItem::action_with_shortcut(
                        MenuItemId::ToggleBoundingBox,
                        "Toggle Bounding Box",
                        "B",
                    ),
                    MenuItem::separator(),
                    MenuItem::action_with_shortcut(MenuItemId::ResetCamera, "Reset Camera", "Home"),
                    MenuItem::action(MenuItemId::FrameAll, "Frame All"),
                ],
            ),
        ]
    }

    /// Create mesh context menu
    fn create_mesh_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action_with_shortcut(MenuItemId::DuplicateMesh, "Duplicate", "Ctrl+D"),
            MenuItem::action(MenuItemId::RenameMesh, "Rename"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::IsolateMesh, "Isolate"),
            MenuItem::action(MenuItemId::HideMesh, "Hide"),
            MenuItem::separator(),
            MenuItem::submenu(
                "Transform",
                vec![
                    MenuItem::action(MenuItemId::ResetTransform, "Reset All"),
                    MenuItem::action(MenuItemId::ResetPosition, "Reset Position"),
                    MenuItem::action(MenuItemId::ResetRotation, "Reset Rotation"),
                    MenuItem::action(MenuItemId::ResetScale, "Reset Scale"),
                    MenuItem::separator(),
                    MenuItem::action(MenuItemId::CenterPivot, "Center Pivot"),
                    MenuItem::action(MenuItemId::SnapToOrigin, "Snap to Origin"),
                ],
            ),
            MenuItem::submenu(
                "Normals",
                vec![
                    MenuItem::action_with_shortcut(
                        MenuItemId::RecalculateNormals,
                        "Recalculate",
                        "Shift+N",
                    ),
                    MenuItem::action(MenuItemId::SmoothNormals, "Smooth"),
                    MenuItem::action(MenuItemId::FlattenNormals, "Flatten"),
                    MenuItem::action_with_shortcut(MenuItemId::FlipNormals, "Flip", "Shift+F"),
                ],
            ),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::ValidateMesh, "Validate"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::ExportMesh, "Export", "Ctrl+E"),
            MenuItem::action_with_shortcut(MenuItemId::DeleteMesh, "Delete", "Delete"),
        ]
    }

    /// Create vertex context menu
    fn create_vertex_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action_with_shortcut(MenuItemId::DuplicateVertex, "Duplicate", "Ctrl+D"),
            MenuItem::action(MenuItemId::SetVertexPosition, "Set Position"),
            MenuItem::action(MenuItemId::SnapToGrid, "Snap to Grid"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::MergeVertices, "Merge Selected"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::SetNormal, "Set Normal"),
            MenuItem::action(MenuItemId::FlipNormal, "Flip Normal"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DeleteVertex, "Delete", "Delete"),
        ]
    }

    /// Create face context menu
    fn create_face_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action(MenuItemId::FlipWinding, "Flip Winding"),
            MenuItem::action_with_shortcut(MenuItemId::FlipNormals, "Flip Normals", "Shift+F"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::SubdivideFaces, "Subdivide"),
            MenuItem::action_with_shortcut(MenuItemId::TriangulateFaces, "Triangulate", "Shift+T"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DeleteFace, "Delete", "Delete"),
        ]
    }

    /// Create mesh list context menu
    fn create_mesh_list_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action(MenuItemId::AddMesh, "Add Mesh"),
            MenuItem::action_with_shortcut(MenuItemId::ImportMesh, "Import Mesh", "Ctrl+I"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DuplicateMesh, "Duplicate", "Ctrl+D"),
            MenuItem::action(MenuItemId::RenameMesh, "Rename"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::ShowAllMeshes, "Show All"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DeleteMesh, "Delete", "Delete"),
        ]
    }

    /// Create vertex editor context menu
    fn create_vertex_editor_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action_with_shortcut(MenuItemId::AddVertex, "Add Vertex", "Shift+A"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::Cut, "Cut", "Ctrl+X"),
            MenuItem::action_with_shortcut(MenuItemId::Copy, "Copy", "Ctrl+C"),
            MenuItem::action_with_shortcut(MenuItemId::Paste, "Paste", "Ctrl+V"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::MergeVertices, "Merge Selected"),
            MenuItem::action(MenuItemId::SnapToGrid, "Snap to Grid"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DeleteVertex, "Delete", "Delete"),
        ]
    }

    /// Create index editor context menu
    fn create_index_editor_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::action(MenuItemId::AddFace, "Add Face"),
            MenuItem::separator(),
            MenuItem::action(MenuItemId::FlipWinding, "Flip Winding"),
            MenuItem::action_with_shortcut(MenuItemId::TriangulateFaces, "Triangulate", "Shift+T"),
            MenuItem::separator(),
            MenuItem::action_with_shortcut(MenuItemId::DeleteFace, "Delete", "Delete"),
        ]
    }

    /// Register a context menu
    pub fn register(&mut self, context_type: ContextType, menu: Vec<MenuItem>) {
        self.menus.insert(context_type, menu);
    }

    /// Get context menu for a context type
    pub fn get_menu(&self, context_type: ContextType) -> Option<&Vec<MenuItem>> {
        self.menus.get(&context_type)
    }

    /// Get context menu with enabled/disabled states based on context
    pub fn get_menu_with_context(
        &self,
        context_type: ContextType,
        context: &MenuContext,
    ) -> Option<Vec<MenuItem>> {
        self.menus.get(&context_type).map(|items| {
            items
                .iter()
                .map(|item| self.apply_context_to_item(item.clone(), context))
                .collect()
        })
    }

    /// Apply context to a menu item to determine if it should be enabled
    fn apply_context_to_item(&self, mut item: MenuItem, context: &MenuContext) -> MenuItem {
        match &mut item {
            MenuItem::Action { id, enabled, .. } => {
                *enabled = self.is_item_enabled(*id, context);
            }
            MenuItem::Submenu { items, enabled, .. } => {
                *enabled = items.iter().any(|i| match i {
                    MenuItem::Action { id, .. } => self.is_item_enabled(*id, context),
                    _ => true,
                });
                *items = items
                    .iter()
                    .map(|i| self.apply_context_to_item(i.clone(), context))
                    .collect();
            }
            MenuItem::Separator => {}
        }
        item
    }

    /// Check if a menu item should be enabled based on context
    fn is_item_enabled(&self, id: MenuItemId, context: &MenuContext) -> bool {
        match id {
            // Edit operations
            MenuItemId::Undo => context.can_undo,
            MenuItemId::Redo => context.can_redo,
            MenuItemId::Cut | MenuItemId::Copy | MenuItemId::Delete | MenuItemId::Duplicate => {
                context.has_selection()
            }
            MenuItemId::Paste => context.has_clipboard,
            MenuItemId::DeselectAll | MenuItemId::InvertSelection => context.has_selection(),

            // Mesh operations requiring selection
            MenuItemId::DeleteMesh
            | MenuItemId::DuplicateMesh
            | MenuItemId::RenameMesh
            | MenuItemId::ExportMesh
            | MenuItemId::IsolateMesh
            | MenuItemId::HideMesh => context.has_mesh_selection(),

            // Vertex operations requiring selection
            MenuItemId::DeleteVertex
            | MenuItemId::DuplicateVertex
            | MenuItemId::SetVertexPosition
            | MenuItemId::SnapToGrid
            | MenuItemId::SetNormal
            | MenuItemId::FlipNormal => context.has_vertex_selection(),

            // Merge requires multiple vertices
            MenuItemId::MergeVertices => context.has_multiple_vertices(),

            // Face operations
            MenuItemId::DeleteFace
            | MenuItemId::FlipWinding
            | MenuItemId::SubdivideFaces
            | MenuItemId::TriangulateFaces => context.selected_faces > 0,

            // Transform operations
            MenuItemId::ResetTransform
            | MenuItemId::ResetPosition
            | MenuItemId::ResetRotation
            | MenuItemId::ResetScale
            | MenuItemId::CenterPivot
            | MenuItemId::SnapToOrigin => context.has_mesh_selection(),

            // Normal operations
            MenuItemId::RecalculateNormals
            | MenuItemId::SmoothNormals
            | MenuItemId::FlattenNormals
            | MenuItemId::FlipNormals => context.has_mesh_selection(),

            // View operations - always enabled
            MenuItemId::FocusSelected => context.has_selection(),
            MenuItemId::FrameAll
            | MenuItemId::ToggleWireframe
            | MenuItemId::ToggleNormals
            | MenuItemId::ToggleGrid
            | MenuItemId::ToggleBoundingBox
            | MenuItemId::ResetCamera => true,

            // Show all meshes enabled if there are meshes
            MenuItemId::ShowAllMeshes => context.total_meshes > 0,

            // Validate enabled if mesh selected
            MenuItemId::ValidateMesh | MenuItemId::ShowValidationReport => {
                context.has_mesh_selection()
            }

            // Always enabled
            MenuItemId::AddMesh
            | MenuItemId::AddVertex
            | MenuItemId::AddFace
            | MenuItemId::ImportMesh
            | MenuItemId::SelectAll
            | MenuItemId::ApplyTemplate
            | MenuItemId::SaveAsTemplate
            | MenuItemId::BrowseTemplates
            | MenuItemId::ShowProperties
            | MenuItemId::EditMetadata
            | MenuItemId::FixErrors => true,
        }
    }

    /// Clear all menus
    pub fn clear(&mut self) {
        self.menus.clear();
    }

    /// Reset to default menus
    pub fn reset_defaults(&mut self) {
        self.clear();
        self.register_defaults();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_item_action() {
        let item = MenuItem::action(MenuItemId::AddMesh, "Add Mesh");
        assert_eq!(item.id(), Some(MenuItemId::AddMesh));
        assert!(item.is_enabled());
        assert!(!item.is_separator());
    }

    #[test]
    fn test_menu_item_disabled() {
        let item = MenuItem::disabled_action(MenuItemId::DeleteMesh, "Delete");
        assert!(!item.is_enabled());
    }

    #[test]
    fn test_menu_item_with_shortcut() {
        let item = MenuItem::action_with_shortcut(MenuItemId::Undo, "Undo", "Ctrl+Z");
        match item {
            MenuItem::Action { shortcut, .. } => assert_eq!(shortcut, Some("Ctrl+Z".to_string())),
            _ => panic!("Expected action item"),
        }
    }

    #[test]
    fn test_menu_item_separator() {
        let item = MenuItem::separator();
        assert!(item.is_separator());
        assert_eq!(item.id(), None);
    }

    #[test]
    fn test_menu_item_submenu() {
        let submenu = MenuItem::submenu(
            "Transform",
            vec![MenuItem::action(MenuItemId::ResetTransform, "Reset")],
        );

        match submenu {
            MenuItem::Submenu { items, .. } => assert_eq!(items.len(), 1),
            _ => panic!("Expected submenu"),
        }
    }

    #[test]
    fn test_menu_context_has_selection() {
        let mut context = MenuContext::new();
        assert!(!context.has_selection());

        context.selected_meshes = 1;
        assert!(context.has_selection());
        assert!(context.has_mesh_selection());
    }

    #[test]
    fn test_menu_context_with_meshes() {
        let context = MenuContext::with_meshes(2);
        assert_eq!(context.selected_meshes, 2);
        assert!(context.has_mesh_selection());
    }

    #[test]
    fn test_menu_context_with_vertices() {
        let context = MenuContext::with_vertices(3);
        assert_eq!(context.selected_vertices, 3);
        assert!(context.has_vertex_selection());
        assert!(context.has_multiple_vertices());
    }

    #[test]
    fn test_context_menu_manager_creation() {
        let manager = ContextMenuManager::new();
        assert!(manager.get_menu(ContextType::Viewport).is_some());
        assert!(manager.get_menu(ContextType::Mesh).is_some());
        assert!(manager.get_menu(ContextType::Vertex).is_some());
    }

    #[test]
    fn test_viewport_menu() {
        let manager = ContextMenuManager::new();
        let menu = manager.get_menu(ContextType::Viewport);
        assert!(menu.is_some());
        assert!(!menu.unwrap().is_empty());
    }

    #[test]
    fn test_mesh_menu() {
        let manager = ContextMenuManager::new();
        let menu = manager.get_menu(ContextType::Mesh);
        assert!(menu.is_some());
        assert!(!menu.unwrap().is_empty());
    }

    #[test]
    fn test_context_menu_register() {
        let mut manager = ContextMenuManager::new();
        let custom_menu = vec![MenuItem::action(MenuItemId::AddMesh, "Custom Add")];

        manager.register(ContextType::Viewport, custom_menu);
        let menu = manager.get_menu(ContextType::Viewport).unwrap();
        assert_eq!(menu.len(), 1);
    }

    #[test]
    fn test_menu_with_context_undo() {
        let manager = ContextMenuManager::new();
        let mut context = MenuContext::new();
        context.can_undo = false;

        let menu = manager
            .get_menu_with_context(ContextType::Viewport, &context)
            .unwrap();

        // Find the undo item and check it's disabled
        let undo_item = menu.iter().find(|item| item.id() == Some(MenuItemId::Undo));
        assert!(undo_item.is_some());
        assert!(!undo_item.unwrap().is_enabled());
    }

    #[test]
    fn test_menu_with_context_delete_enabled() {
        let manager = ContextMenuManager::new();
        let context = MenuContext::with_meshes(1);

        let menu = manager
            .get_menu_with_context(ContextType::Mesh, &context)
            .unwrap();

        // Find the delete item and check it's enabled
        let delete_item = menu
            .iter()
            .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
        assert!(delete_item.is_some());
        assert!(delete_item.unwrap().is_enabled());
    }

    #[test]
    fn test_menu_with_context_delete_disabled() {
        let manager = ContextMenuManager::new();
        let context = MenuContext::new(); // No selection

        let menu = manager
            .get_menu_with_context(ContextType::Mesh, &context)
            .unwrap();

        // Find the delete item and check it's disabled
        let delete_item = menu
            .iter()
            .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
        assert!(delete_item.is_some());
        assert!(!delete_item.unwrap().is_enabled());
    }

    #[test]
    fn test_merge_vertices_requires_multiple() {
        let manager = ContextMenuManager::new();

        // One vertex - should be disabled
        let context_one = MenuContext::with_vertices(1);
        let menu_one = manager
            .get_menu_with_context(ContextType::Vertex, &context_one)
            .unwrap();
        let merge_one = menu_one
            .iter()
            .find(|item| item.id() == Some(MenuItemId::MergeVertices));
        assert!(!merge_one.unwrap().is_enabled());

        // Two vertices - should be enabled
        let context_two = MenuContext::with_vertices(2);
        let menu_two = manager
            .get_menu_with_context(ContextType::Vertex, &context_two)
            .unwrap();
        let merge_two = menu_two
            .iter()
            .find(|item| item.id() == Some(MenuItemId::MergeVertices));
        assert!(merge_two.unwrap().is_enabled());
    }

    #[test]
    fn test_clear_and_reset() {
        let mut manager = ContextMenuManager::new();
        assert!(manager.get_menu(ContextType::Viewport).is_some());

        manager.clear();
        assert!(manager.get_menu(ContextType::Viewport).is_none());

        manager.reset_defaults();
        assert!(manager.get_menu(ContextType::Viewport).is_some());
    }

    #[test]
    fn test_submenu_enabled_state() {
        let manager = ContextMenuManager::new();
        let context = MenuContext::with_meshes(1);

        let menu = manager
            .get_menu_with_context(ContextType::Mesh, &context)
            .unwrap();

        // Find Transform submenu
        let transform_submenu = menu.iter().find(|item| match item {
            MenuItem::Submenu { label, .. } => label == "Transform",
            _ => false,
        });

        assert!(transform_submenu.is_some());
        assert!(transform_submenu.unwrap().is_enabled());
    }
}
