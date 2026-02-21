// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature Editor Workflow Integration Tests
//!
//! Tests for the full creature editor workflow including:
//! - Undo/redo system
//! - Keyboard shortcuts
//! - Context menus
//! - Auto-save and recovery
//! - Preview features
//! - Integrated workflows

use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
use campaign_builder::auto_save::{AutoSaveConfig, AutoSaveManager};
use campaign_builder::context_menu::{ContextMenuManager, ContextType, MenuContext, MenuItemId};
use campaign_builder::creature_undo_redo::{
    AddMeshCommand, CreatureUndoRedoManager, ModifyCreaturePropertiesCommand, ModifyMeshCommand,
    ModifyTransformCommand, RemoveMeshCommand,
};
use campaign_builder::creatures_workflow::{CreatureWorkflowState, WorkflowMode};
use campaign_builder::keyboard_shortcuts::{Key, Shortcut, ShortcutAction, ShortcutManager};
use campaign_builder::preview_features::{
    CameraConfig, LightingConfig, PreviewOptions, PreviewState, PreviewStatistics,
};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// =============================================================================
// Helper Functions
// =============================================================================

fn make_creature(name: &str) -> CreatureDefinition {
    CreatureDefinition {
        id: 1,
        name: name.to_string(),
        meshes: vec![],
        mesh_transforms: vec![],
        scale: 1.0,
        color_tint: None,
    }
}

fn make_mesh(name: &str) -> MeshDefinition {
    MeshDefinition {
        name: Some(name.to_string()),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
        normals: None,
        uvs: None,
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

fn make_transform() -> MeshTransform {
    MeshTransform {
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    }
}

// =============================================================================
// Workflow Mode Tests
// =============================================================================

#[test]
fn test_full_creation_workflow() {
    let mut workflow = CreatureWorkflowState::new();
    assert_eq!(workflow.mode, WorkflowMode::Registry);
    assert!(workflow.current_file().is_none());

    workflow.enter_asset_editor("goblin.ron", "Goblin");
    assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
    assert_eq!(workflow.current_file(), Some("goblin.ron"));
    assert_eq!(workflow.current_creature_name(), Some("Goblin"));
    assert!(!workflow.has_unsaved_changes());

    let mut creature = make_creature("Goblin");
    let mut undo_manager = CreatureUndoRedoManager::new();

    for part in &["body", "head", "left_arm", "right_arm"] {
        undo_manager
            .execute(
                Box::new(AddMeshCommand::new(make_mesh(part), make_transform())),
                &mut creature,
            )
            .expect("AddMeshCommand should succeed");
        workflow.mark_dirty();
    }

    assert_eq!(creature.meshes.len(), 4);
    assert!(workflow.has_unsaved_changes());
    assert!(undo_manager.can_undo());

    let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
    assert_eq!(labels, ["Creatures", "Goblin"]);
    assert!(workflow.mode_indicator().starts_with("Asset Editor:"));

    workflow.mark_clean();
    assert!(!workflow.has_unsaved_changes());

    workflow.return_to_registry();
    assert_eq!(workflow.mode, WorkflowMode::Registry);
    assert!(workflow.current_file().is_none());
    let root_labels: Vec<&str> = workflow.breadcrumb_labels().collect();
    assert_eq!(root_labels, ["Creatures"]);
}

#[test]
fn test_full_editing_workflow() {
    let mut creature = make_creature("Troll");
    creature.meshes.push(make_mesh("body"));
    creature.mesh_transforms.push(make_transform());

    let mut undo_manager = CreatureUndoRedoManager::new();
    let mut workflow = CreatureWorkflowState::new();

    workflow.enter_asset_editor("troll.ron", "Troll");
    assert_eq!(workflow.mode, WorkflowMode::AssetEditor);

    let old_transform = creature.mesh_transforms[0];
    let new_transform = MeshTransform {
        translation: [0.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.5, 1.5, 1.5],
    };

    undo_manager
        .execute(
            Box::new(ModifyTransformCommand::new(0, old_transform, new_transform)),
            &mut creature,
        )
        .expect("ModifyTransformCommand should succeed");

    workflow.mark_dirty();

    assert_eq!(creature.mesh_transforms[0].translation, [0.0, 2.0, 0.0]);
    assert_eq!(creature.mesh_transforms[0].scale, [1.5, 1.5, 1.5]);
    assert!(workflow.has_unsaved_changes());

    let second_mesh = make_mesh("head");
    undo_manager
        .execute(
            Box::new(AddMeshCommand::new(second_mesh, make_transform())),
            &mut creature,
        )
        .expect("AddMeshCommand should succeed");
    assert_eq!(creature.meshes.len(), 2);

    undo_manager
        .undo(&mut creature)
        .expect("undo add-mesh should succeed");
    assert_eq!(creature.meshes.len(), 1);

    undo_manager
        .undo(&mut creature)
        .expect("undo transform should succeed");
    assert_eq!(
        creature.mesh_transforms[0].translation,
        old_transform.translation
    );

    undo_manager
        .redo(&mut creature)
        .expect("redo transform should succeed");
    assert_eq!(creature.mesh_transforms[0].translation, [0.0, 2.0, 0.0]);

    workflow.mark_clean();
    workflow.return_to_registry();
    assert_eq!(workflow.mode, WorkflowMode::Registry);
}

#[test]
fn test_registry_to_asset_navigation() {
    let mut workflow = CreatureWorkflowState::new();

    assert_eq!(workflow.mode, WorkflowMode::Registry);
    assert_eq!(workflow.mode_indicator(), "Registry Mode");

    workflow.enter_asset_editor("goblin.ron", "Goblin");
    assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
    assert_eq!(workflow.current_file(), Some("goblin.ron"));
    assert_eq!(workflow.breadcrumb_string(), "Creatures > Goblin");
    assert_eq!(workflow.mode_indicator(), "Asset Editor: goblin.ron");

    workflow.return_to_registry();
    assert_eq!(workflow.mode, WorkflowMode::Registry);
    assert!(workflow.current_file().is_none());
    assert_eq!(workflow.breadcrumb_string(), "Creatures");
    assert_eq!(workflow.mode_indicator(), "Registry Mode");

    workflow.enter_asset_editor("orc.ron", "Orc");
    assert_eq!(workflow.current_file(), Some("orc.ron"));
    assert_eq!(workflow.current_creature_name(), Some("Orc"));
    assert_eq!(workflow.breadcrumb_string(), "Creatures > Orc");

    workflow.enter_mesh_editor("orc.ron", "Orc", "club");
    let labels: Vec<&str> = workflow.breadcrumb_labels().collect();
    assert_eq!(labels, ["Creatures", "Orc", "club"]);

    workflow.return_to_registry();

    let mut creature = make_creature("Skeleton");
    {
        let mut undo_manager = CreatureUndoRedoManager::new();
        workflow.enter_asset_editor("skeleton.ron", "Skeleton");
        undo_manager
            .execute(
                Box::new(AddMeshCommand::new(make_mesh("ribcage"), make_transform())),
                &mut creature,
            )
            .unwrap();
        assert!(undo_manager.can_undo());
        workflow.return_to_registry();
    }

    workflow.enter_asset_editor("skeleton.ron", "Skeleton");
    assert!(!workflow.can_undo());
    assert_eq!(workflow.mode, WorkflowMode::AssetEditor);
}

// =============================================================================
// Undo/Redo Tests
// =============================================================================

#[test]
fn test_undo_redo_full_session() {
    let mut undo_manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("Dragon");

    let meshes = ["wings", "body", "tail"];
    for name in &meshes {
        undo_manager
            .execute(
                Box::new(AddMeshCommand::new(make_mesh(name), make_transform())),
                &mut creature,
            )
            .unwrap();
    }
    assert_eq!(creature.meshes.len(), 3);
    assert_eq!(undo_manager.undo_count(), 3);
    assert_eq!(undo_manager.redo_count(), 0);

    let old_t = creature.mesh_transforms[0];
    let new_t = MeshTransform {
        translation: [0.0, 5.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [2.0, 2.0, 2.0],
    };
    undo_manager
        .execute(
            Box::new(ModifyTransformCommand::new(0, old_t, new_t)),
            &mut creature,
        )
        .unwrap();
    assert_eq!(creature.mesh_transforms[0].translation, [0.0, 5.0, 0.0]);
    assert_eq!(undo_manager.undo_count(), 4);

    let removed_mesh = creature.meshes[1].clone();
    let removed_transform = creature.mesh_transforms[1];
    undo_manager
        .execute(
            Box::new(RemoveMeshCommand::new(1, removed_mesh, removed_transform)),
            &mut creature,
        )
        .unwrap();
    assert_eq!(creature.meshes.len(), 2);
    assert_eq!(undo_manager.undo_count(), 5);

    undo_manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 3);

    undo_manager.undo(&mut creature).unwrap();
    assert_eq!(creature.mesh_transforms[0].translation, old_t.translation);

    undo_manager.undo(&mut creature).unwrap();
    undo_manager.undo(&mut creature).unwrap();
    undo_manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 0);
    assert!(!undo_manager.can_undo());

    assert_eq!(undo_manager.redo_count(), 5);
    for _ in 0..5 {
        undo_manager.redo(&mut creature).unwrap();
    }
    assert_eq!(creature.meshes.len(), 2);
    assert!(!undo_manager.can_redo());

    undo_manager
        .execute(
            Box::new(AddMeshCommand::new(
                make_mesh("fire_breath"),
                make_transform(),
            )),
            &mut creature,
        )
        .unwrap();
    assert_eq!(undo_manager.redo_count(), 0);
    assert_eq!(creature.meshes.len(), 3);
}

#[test]
fn test_undo_redo_add_mesh_workflow() {
    let mut manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("TestCreature");

    for i in 0..3 {
        let mesh = make_mesh(&format!("Mesh{}", i));
        manager
            .execute(
                Box::new(AddMeshCommand::new(mesh, make_transform())),
                &mut creature,
            )
            .unwrap();
    }

    assert_eq!(creature.meshes.len(), 3);
    assert_eq!(manager.undo_count(), 3);

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 2);

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 1);

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 0);

    manager.redo(&mut creature).unwrap();
    manager.redo(&mut creature).unwrap();
    manager.redo(&mut creature).unwrap();

    assert_eq!(creature.meshes.len(), 3);
    assert_eq!(creature.meshes[0].name, Some("Mesh0".to_string()));
    assert_eq!(creature.meshes[1].name, Some("Mesh1".to_string()));
    assert_eq!(creature.meshes[2].name, Some("Mesh2".to_string()));
}

#[test]
fn test_undo_redo_mixed_operations() {
    let mut manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("TestCreature");

    let mesh = make_mesh("Mesh1");
    let transform = make_transform();
    manager
        .execute(
            Box::new(AddMeshCommand::new(mesh.clone(), transform)),
            &mut creature,
        )
        .unwrap();

    let mut new_transform = make_transform();
    new_transform.translation = [1.0, 2.0, 3.0];
    manager
        .execute(
            Box::new(ModifyTransformCommand::new(0, transform, new_transform)),
            &mut creature,
        )
        .unwrap();

    let new_mesh = make_mesh("Mesh1Modified");
    manager
        .execute(
            Box::new(ModifyMeshCommand::new(0, mesh.clone(), new_mesh.clone())),
            &mut creature,
        )
        .unwrap();

    manager
        .execute(
            Box::new(ModifyCreaturePropertiesCommand::new(
                "TestCreature".to_string(),
                "RenamedCreature".to_string(),
            )),
            &mut creature,
        )
        .unwrap();

    assert_eq!(creature.name, "RenamedCreature");
    assert_eq!(creature.meshes[0].name, Some("Mesh1Modified".to_string()));
    assert_eq!(creature.mesh_transforms[0].translation, [1.0, 2.0, 3.0]);

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.name, "TestCreature");

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes[0].name, Some("Mesh1".to_string()));

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.mesh_transforms[0].translation, [0.0, 0.0, 0.0]);

    manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 0);
}

#[test]
fn test_undo_redo_descriptions() {
    let mut manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("TestCreature");

    manager
        .execute(
            Box::new(AddMeshCommand::new(make_mesh("MyMesh"), make_transform())),
            &mut creature,
        )
        .unwrap();

    assert_eq!(
        manager.next_undo_description(),
        Some("Add mesh 'MyMesh'".to_string())
    );

    manager.undo(&mut creature).unwrap();
    assert_eq!(
        manager.next_redo_description(),
        Some("Add mesh 'MyMesh'".to_string())
    );
}

#[test]
fn test_undo_redo_new_action_clears_redo() {
    let mut manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("TestCreature");

    manager
        .execute(
            Box::new(AddMeshCommand::new(make_mesh("Mesh1"), make_transform())),
            &mut creature,
        )
        .unwrap();

    manager.undo(&mut creature).unwrap();
    assert!(manager.can_redo());

    manager
        .execute(
            Box::new(AddMeshCommand::new(make_mesh("Mesh2"), make_transform())),
            &mut creature,
        )
        .unwrap();

    assert!(!manager.can_redo());
}

#[test]
fn test_undo_redo_empty_stack() {
    let mut manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("EmptyTest");

    assert!(manager.undo(&mut creature).is_err());
    assert!(manager.redo(&mut creature).is_err());
}

#[test]
fn test_undo_redo_max_history() {
    let mut manager = CreatureUndoRedoManager::with_max_history(3);
    let mut creature = make_creature("HistoryTest");

    for i in 0..5 {
        manager
            .execute(
                Box::new(AddMeshCommand::new(
                    make_mesh(&format!("Mesh{}", i)),
                    make_transform(),
                )),
                &mut creature,
            )
            .unwrap();
    }

    assert_eq!(manager.undo_count(), 3);
    assert_eq!(creature.meshes.len(), 5);

    manager.undo(&mut creature).unwrap();
    manager.undo(&mut creature).unwrap();
    manager.undo(&mut creature).unwrap();

    assert_eq!(creature.meshes.len(), 2);
    assert!(!manager.can_undo());
}

// =============================================================================
// Keyboard Shortcut Tests
// =============================================================================

#[test]
fn test_keyboard_shortcuts_core_operations() {
    let shortcuts = ShortcutManager::new();

    let save = shortcuts.get_shortcut(ShortcutAction::Save);
    assert!(save.is_some(), "Save shortcut must be registered");
    assert_eq!(save.unwrap().key, Key::S);
    assert!(save.unwrap().modifiers.ctrl);

    let undo = shortcuts.get_shortcut(ShortcutAction::Undo);
    assert!(undo.is_some(), "Undo shortcut must be registered");
    assert_eq!(undo.unwrap().key, Key::Z);
    assert!(undo.unwrap().modifiers.ctrl);

    let redo = shortcuts.get_shortcut(ShortcutAction::Redo);
    assert!(redo.is_some(), "Redo shortcut must be registered");
    assert!(redo.unwrap().modifiers.ctrl);

    assert!(
        shortcuts.get_shortcut(ShortcutAction::Delete).is_some(),
        "Delete shortcut must be registered"
    );
}

#[test]
fn test_keyboard_shortcuts_default_registration() {
    let manager = ShortcutManager::new();

    assert_eq!(
        manager.get_action(&Shortcut::ctrl(Key::Z)),
        Some(ShortcutAction::Undo)
    );
    assert_eq!(
        manager.get_action(&Shortcut::ctrl_shift(Key::Z)),
        Some(ShortcutAction::Redo)
    );
    assert_eq!(
        manager.get_action(&Shortcut::ctrl(Key::S)),
        Some(ShortcutAction::Save)
    );
    assert_eq!(
        manager.get_action(&Shortcut::key_only(Key::T)),
        Some(ShortcutAction::TranslateTool)
    );
}

#[test]
fn test_keyboard_shortcuts_custom_registration() {
    let mut manager = ShortcutManager::new();

    let custom_shortcut = Shortcut::ctrl(Key::U);
    manager.register(custom_shortcut.clone(), ShortcutAction::Undo);

    assert_eq!(manager.get_action(&Shortcut::ctrl(Key::Z)), None);
    assert_eq!(
        manager.get_action(&custom_shortcut),
        Some(ShortcutAction::Undo)
    );
}

#[test]
fn test_keyboard_shortcuts_categories() {
    let manager = ShortcutManager::new();
    let categories = manager.shortcuts_by_category();

    assert!(categories.contains_key("Edit"));
    assert!(categories.contains_key("Tools"));
    assert!(categories.contains_key("View"));
    assert!(categories.contains_key("Mesh"));
    assert!(categories.contains_key("File"));

    let edit_shortcuts = &categories["Edit"];
    assert!(
        edit_shortcuts
            .iter()
            .any(|(_, action)| *action == ShortcutAction::Undo),
        "Edit category must contain Undo"
    );
    assert!(
        edit_shortcuts
            .iter()
            .any(|(_, action)| *action == ShortcutAction::Redo),
        "Edit category must contain Redo"
    );
}

#[test]
fn test_keyboard_shortcuts_modifiers() {
    let mut manager = ShortcutManager::new();

    manager.register(Shortcut::ctrl(Key::S), ShortcutAction::Save);
    manager.register(Shortcut::ctrl_shift(Key::S), ShortcutAction::SaveAs);
    manager.register(Shortcut::alt(Key::S), ShortcutAction::ShowHelp);

    assert_eq!(
        manager.get_action(&Shortcut::ctrl(Key::S)),
        Some(ShortcutAction::Save)
    );
    assert_eq!(
        manager.get_action(&Shortcut::ctrl_shift(Key::S)),
        Some(ShortcutAction::SaveAs)
    );
    assert_eq!(
        manager.get_action(&Shortcut::alt(Key::S)),
        Some(ShortcutAction::ShowHelp)
    );
}

#[test]
fn test_keyboard_shortcuts_description() {
    let manager = ShortcutManager::new();

    let desc = manager.describe(ShortcutAction::Undo);
    assert!(desc.is_some());
    let desc_str = desc.unwrap();
    assert!(desc_str.contains("Ctrl"));
    assert!(desc_str.contains("Z"));
}

// =============================================================================
// Context Menu Tests
// =============================================================================

#[test]
fn test_context_menu_responds_to_state() {
    let menus = ContextMenuManager::new();

    let ctx_no_sel = MenuContext::new();
    let mesh_menu = menus
        .get_menu_with_context(ContextType::MeshList, &ctx_no_sel)
        .expect("MeshList menu must exist");

    let delete_item = mesh_menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
    if let Some(item) = delete_item {
        assert!(
            !item.is_enabled(),
            "Delete should be disabled with no selection"
        );
    }

    let mut ctx_with_sel = MenuContext::new();
    ctx_with_sel.selected_meshes = 1;
    let mesh_menu_sel = menus
        .get_menu_with_context(ContextType::MeshList, &ctx_with_sel)
        .expect("MeshList menu must exist");

    let delete_item_sel = mesh_menu_sel
        .iter()
        .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
    if let Some(item) = delete_item_sel {
        assert!(
            item.is_enabled(),
            "Delete should be enabled when mesh is selected"
        );
    }

    let ctx_no_clipboard = MenuContext::new();
    let vp_menu = menus
        .get_menu_with_context(ContextType::Viewport, &ctx_no_clipboard)
        .expect("Viewport menu must exist");
    let paste = vp_menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::Paste));
    if let Some(item) = paste {
        assert!(
            !item.is_enabled(),
            "Paste should be disabled without clipboard content"
        );
    }
}

#[test]
fn test_context_menu_viewport() {
    let manager = ContextMenuManager::new();
    let menu = manager.get_menu(ContextType::Viewport);

    assert!(menu.is_some());
    assert!(!menu.unwrap().is_empty());
}

#[test]
fn test_context_menu_with_selection() {
    let manager = ContextMenuManager::new();
    let context = MenuContext::with_meshes(1);

    let menu = manager
        .get_menu_with_context(ContextType::Mesh, &context)
        .unwrap();

    let delete_item = menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
    assert!(delete_item.is_some());
    assert!(delete_item.unwrap().is_enabled());
}

#[test]
fn test_context_menu_without_selection() {
    let manager = ContextMenuManager::new();
    let context = MenuContext::new();

    let menu = manager
        .get_menu_with_context(ContextType::Mesh, &context)
        .unwrap();

    let delete_item = menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::DeleteMesh));
    assert!(delete_item.is_some());
    assert!(!delete_item.unwrap().is_enabled());
}

#[test]
fn test_context_menu_undo_redo_state() {
    let manager = ContextMenuManager::new();
    let mut context = MenuContext::new();

    context.can_undo = false;
    let menu = manager
        .get_menu_with_context(ContextType::Viewport, &context)
        .unwrap();
    let undo_item = menu.iter().find(|item| item.id() == Some(MenuItemId::Undo));
    assert!(!undo_item.unwrap().is_enabled());

    context.can_undo = true;
    let menu = manager
        .get_menu_with_context(ContextType::Viewport, &context)
        .unwrap();
    let undo_item = menu.iter().find(|item| item.id() == Some(MenuItemId::Undo));
    assert!(undo_item.unwrap().is_enabled());
}

#[test]
fn test_context_menu_merge_vertices_requires_multiple() {
    let manager = ContextMenuManager::new();

    let context_one = MenuContext::with_vertices(1);
    let menu_one = manager
        .get_menu_with_context(ContextType::Vertex, &context_one)
        .unwrap();
    let merge_one = menu_one
        .iter()
        .find(|item| item.id() == Some(MenuItemId::MergeVertices));
    assert!(!merge_one.unwrap().is_enabled());

    let context_multi = MenuContext::with_vertices(3);
    let menu_multi = manager
        .get_menu_with_context(ContextType::Vertex, &context_multi)
        .unwrap();
    let merge_multi = menu_multi
        .iter()
        .find(|item| item.id() == Some(MenuItemId::MergeVertices));
    assert!(merge_multi.unwrap().is_enabled());
}

#[test]
fn test_context_menu_clipboard_state() {
    let manager = ContextMenuManager::new();
    let mut context = MenuContext::new();

    context.has_clipboard = false;
    let menu = manager
        .get_menu_with_context(ContextType::Viewport, &context)
        .unwrap();
    let paste_item = menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::Paste));
    if let Some(item) = paste_item {
        assert!(!item.is_enabled());
    }

    context.has_clipboard = true;
    let menu = manager
        .get_menu_with_context(ContextType::Viewport, &context)
        .unwrap();
    let paste_item = menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::Paste));
    if let Some(item) = paste_item {
        assert!(item.is_enabled());
    }
}

// =============================================================================
// Auto-Save Tests
// =============================================================================

#[test]
fn test_autosave_recovery() {
    let dir = TempDir::new().expect("TempDir::new should succeed");
    let config = AutoSaveConfig::default()
        .with_directory(dir.path())
        .with_max_backups(5)
        .with_interval(1);

    let mut manager = AutoSaveManager::new(config).expect("AutoSaveManager::new should succeed");

    let mut creature = make_creature("Lich");
    creature.meshes.push(make_mesh("skull"));
    creature.meshes.push(make_mesh("robes"));
    creature.mesh_transforms.push(make_transform());
    creature.mesh_transforms.push(make_transform());

    manager.mark_dirty();
    manager
        .auto_save(&creature)
        .expect("auto_save should succeed");

    let backups = manager
        .list_backups("Lich")
        .expect("list_backups should succeed");
    assert!(!backups.is_empty(), "at least one backup must exist");

    let recovered = manager
        .load_recovery_file(&backups[0])
        .expect("load_recovery_file should succeed");

    assert_eq!(recovered.name, "Lich");
    assert_eq!(recovered.meshes.len(), 2);

    manager.mark_clean();
    assert!(!manager.is_dirty());
}

#[test]
fn test_auto_save_basic_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let config = AutoSaveConfig::default()
        .with_directory(temp_dir.path())
        .with_interval(1);

    let mut manager = AutoSaveManager::new(config).unwrap();
    let creature = make_creature("TestCreature");

    manager.mark_dirty();
    assert!(manager.should_auto_save());

    let path = manager.auto_save(&creature).unwrap();
    assert!(path.exists());
    assert!(!manager.is_dirty());
}

#[test]
fn test_auto_save_find_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let config = AutoSaveConfig::default()
        .with_directory(temp_dir.path())
        .with_interval(1);

    let mut manager = AutoSaveManager::new(config).unwrap();
    let creature = make_creature("RecoveryTest");

    manager.mark_dirty();
    manager.auto_save(&creature).unwrap();

    let recovery_files = manager.find_all_recovery_files().unwrap();
    assert_eq!(recovery_files.len(), 1);

    let loaded = manager.load_recovery_file(&recovery_files[0]).unwrap();
    assert_eq!(loaded.name, "RecoveryTest");
}

#[test]
fn test_auto_save_cleanup_old_backups() {
    let temp_dir = TempDir::new().unwrap();
    let config = AutoSaveConfig::default()
        .with_directory(temp_dir.path())
        .with_max_backups(2)
        .with_interval(1);

    let mut manager = AutoSaveManager::new(config).unwrap();
    let creature = make_creature("TestCreature");

    for _ in 0..4 {
        manager.mark_dirty();
        manager.auto_save(&creature).unwrap();
        thread::sleep(Duration::from_millis(500));
    }

    let backups = manager.list_backups("TestCreature").unwrap();
    assert!(
        backups.len() <= 3,
        "Expected at most 3 backups, got {}",
        backups.len()
    );
}

#[test]
fn test_auto_save_interval_check() {
    let temp_dir = TempDir::new().unwrap();
    let config = AutoSaveConfig::default()
        .with_directory(temp_dir.path())
        .with_interval(2);

    let mut manager = AutoSaveManager::new(config).unwrap();
    let creature = make_creature("TestCreature");

    manager.mark_dirty();
    manager.auto_save(&creature).unwrap();

    manager.mark_dirty();
    assert!(!manager.should_auto_save());

    thread::sleep(Duration::from_secs(3));
    assert!(manager.should_auto_save());
}

#[test]
fn test_auto_save_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = AutoSaveConfig::default().with_directory(temp_dir.path());
    config.enabled = false;

    let mut manager = AutoSaveManager::new(config).unwrap();
    let creature = make_creature("TestCreature");

    manager.mark_dirty();
    assert!(!manager.should_auto_save());
    assert!(manager.auto_save(&creature).is_err());
}

// =============================================================================
// Preview Feature Tests
// =============================================================================

#[test]
fn test_preview_state_updates_with_creature_edits() {
    let mut preview = PreviewState::new();
    let mut creature = make_creature("Bandit");

    let initial_stats = PreviewStatistics::new();
    preview.update_statistics(initial_stats);
    assert_eq!(preview.statistics.mesh_count, 0);
    assert_eq!(preview.statistics.vertex_count, 0);

    creature.meshes.push(make_mesh("body"));
    creature.meshes.push(make_mesh("sword"));

    let mut updated = PreviewStatistics::new();
    updated.mesh_count = creature.meshes.len();
    updated.vertex_count = creature.meshes.iter().map(|m| m.vertices.len()).sum();
    updated.triangle_count = creature.meshes.iter().map(|m| m.indices.len() / 3).sum();
    preview.update_statistics(updated);

    assert_eq!(preview.statistics.mesh_count, 2);
    assert_eq!(preview.statistics.vertex_count, 6);
    assert_eq!(preview.statistics.triangle_count, 2);

    preview.options.toggle_wireframe();
    assert!(preview.options.show_wireframe);

    preview.options.toggle_grid();
    assert!(!preview.options.show_grid);

    preview.camera.position = [99.0, 99.0, 99.0];
    preview.reset_camera();
    assert_eq!(preview.camera.position, [5.0, 5.0, 5.0]);
}

#[test]
fn test_preview_options_toggles() {
    let mut options = PreviewOptions::default();

    assert!(options.show_grid);
    options.toggle_grid();
    assert!(!options.show_grid);

    assert!(!options.show_wireframe);
    options.toggle_wireframe();
    assert!(options.show_wireframe);

    assert!(!options.show_normals);
    options.toggle_normals();
    assert!(options.show_normals);
}

#[test]
fn test_camera_views() {
    let mut camera = CameraConfig::default();

    camera.front_view();
    assert_eq!(camera.position, [0.0, 0.0, 10.0]);
    assert_eq!(camera.target, [0.0, 0.0, 0.0]);

    camera.top_view();
    assert_eq!(camera.position, [0.0, 10.0, 0.0]);

    camera.right_view();
    assert_eq!(camera.position, [10.0, 0.0, 0.0]);

    camera.isometric_view();
    assert_eq!(camera.position, [5.0, 5.0, 5.0]);
}

#[test]
fn test_preview_statistics() {
    let mut stats = PreviewStatistics::new();
    stats.mesh_count = 5;
    stats.vertex_count = 150;
    stats.triangle_count = 100;
    stats.fps = 60.0;

    let formatted = stats.format();
    assert!(formatted.contains("Meshes: 5"));
    assert!(formatted.contains("Vertices: 150"));
    assert!(formatted.contains("Triangles: 100"));
    assert!(formatted.contains("FPS: 60"));
}

#[test]
fn test_preview_statistics_bounds() {
    let mut stats = PreviewStatistics::new();
    stats.bounds_min = [-1.0, -2.0, -3.0];
    stats.bounds_max = [1.0, 2.0, 3.0];

    assert_eq!(stats.bounds_size(), [2.0, 4.0, 6.0]);
    assert_eq!(stats.bounds_center(), [0.0, 0.0, 0.0]);
}

#[test]
fn test_preview_state_management() {
    let mut state = PreviewState::new();

    state.options.show_grid = false;
    state.camera.position = [10.0, 10.0, 10.0];

    state.reset();
    assert!(state.options.show_grid);
    assert_eq!(state.camera.position, [5.0, 5.0, 5.0]);
}

#[test]
fn test_lighting_config() {
    let lighting = LightingConfig::default();

    assert_eq!(lighting.ambient_intensity, 0.3);
    assert_eq!(lighting.directional_lights.len(), 1);
    assert_eq!(lighting.point_lights.len(), 0);
}

// =============================================================================
// Integrated Workflow Tests
// =============================================================================

#[test]
fn test_full_session_undo_redo_with_autosave() {
    let dir = TempDir::new().unwrap();
    let config = AutoSaveConfig::default()
        .with_directory(dir.path())
        .with_interval(1);
    let mut auto_save = AutoSaveManager::new(config).unwrap();
    auto_save.set_file_path("dragon.ron".to_string());

    let mut undo_manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("Dragon");

    for name in &["wings", "body"] {
        undo_manager
            .execute(
                Box::new(AddMeshCommand::new(make_mesh(name), make_transform())),
                &mut creature,
            )
            .unwrap();
    }
    assert_eq!(creature.meshes.len(), 2);

    auto_save.mark_dirty();
    auto_save.auto_save(&creature).unwrap();

    undo_manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 1);

    let backups = auto_save.list_backups("Dragon").unwrap();
    assert!(!backups.is_empty());
    let recovered = auto_save.load_recovery_file(&backups[0]).unwrap();
    assert_eq!(recovered.meshes.len(), 2);

    undo_manager.redo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 2);
}

#[test]
fn test_complete_editing_workflow() {
    let mut undo_manager = CreatureUndoRedoManager::new();
    let mut creature = make_creature("CompleteTest");
    let shortcut_manager = ShortcutManager::new();
    let context_menu_manager = ContextMenuManager::new();

    undo_manager
        .execute(
            Box::new(AddMeshCommand::new(make_mesh("Body"), make_transform())),
            &mut creature,
        )
        .unwrap();
    assert_eq!(creature.meshes.len(), 1);

    let undo_shortcut = shortcut_manager.get_shortcut(ShortcutAction::Undo);
    assert!(undo_shortcut.is_some());
    assert_eq!(undo_shortcut.unwrap().key, Key::Z);

    undo_manager.undo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 0);

    undo_manager.redo(&mut creature).unwrap();
    assert_eq!(creature.meshes.len(), 1);

    let mut context = MenuContext::new();
    context.selected_meshes = 1;
    context.can_undo = undo_manager.can_undo();

    let menu = context_menu_manager
        .get_menu_with_context(ContextType::Mesh, &context)
        .unwrap();

    let delete_enabled = menu
        .iter()
        .find(|item| item.id() == Some(MenuItemId::DeleteMesh))
        .map(|item| item.is_enabled())
        .unwrap_or(false);
    assert!(delete_enabled);
}

#[test]
fn test_preview_update_with_editing() {
    let mut preview_state = PreviewState::new();
    let mut creature = make_creature("PreviewTest");

    let initial_stats = PreviewStatistics::new();
    preview_state.update_statistics(initial_stats);
    assert_eq!(preview_state.statistics.mesh_count, 0);

    creature.meshes.push(make_mesh("TestMesh"));

    let mut updated_stats = PreviewStatistics::new();
    updated_stats.mesh_count = creature.meshes.len();
    updated_stats.vertex_count = creature.meshes.iter().map(|m| m.vertices.len()).sum();
    updated_stats.triangle_count = creature.meshes.iter().map(|m| m.indices.len() / 3).sum();

    preview_state.update_statistics(updated_stats);

    assert_eq!(preview_state.statistics.mesh_count, 1);
    assert_eq!(preview_state.statistics.vertex_count, 3);
    assert_eq!(preview_state.statistics.triangle_count, 1);
}

#[test]
fn test_keyboard_shortcuts_with_context_menus() {
    let shortcut_manager = ShortcutManager::new();
    let context_menu_manager = ContextMenuManager::new();

    let menu = context_menu_manager
        .get_menu(ContextType::Viewport)
        .unwrap();

    let mut undo_found = false;
    for item in menu {
        if item.id() == Some(MenuItemId::Undo) {
            undo_found = true;
            assert!(shortcut_manager
                .get_shortcut(ShortcutAction::Undo)
                .is_some());
        }
    }

    assert!(undo_found);
}
