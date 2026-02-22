// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for Phase 2: Creature Asset Editor UI
//!
//! Tests cover:
//! - Loading creature assets
//! - Adding meshes to creatures
//! - Removing meshes from creatures
//! - Duplicating meshes
//! - Reordering meshes
//! - Updating mesh transforms
//! - Updating mesh colors
//! - Replacing meshes with primitives
//! - Creature scale multiplier
//! - Saving assets to files

use antares::domain::visual::{CreatureDefinition, CreatureReference, MeshTransform};
use campaign_builder::creature_assets::CreatureAssetManager;
use campaign_builder::creatures_editor::{CreaturesEditorState, PrimitiveType};
use campaign_builder::primitive_generators::*;

#[test]
fn test_load_creature_asset() {
    let mut state = CreaturesEditorState::new();

    // Create a test creature
    let creature = CreatureDefinition {
        id: 1,
        name: "Test Creature".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    state.edit_buffer = creature.clone();

    assert_eq!(state.edit_buffer.id, 1);
    assert_eq!(state.edit_buffer.name, "Test Creature");
    assert_eq!(state.edit_buffer.meshes.len(), 1);
    assert_eq!(state.edit_buffer.mesh_transforms.len(), 1);
}

#[test]
fn test_add_mesh_to_creature() {
    let mut state = CreaturesEditorState::new();

    // Start with a creature with one mesh
    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    let initial_count = state.edit_buffer.meshes.len();

    // Add a new mesh
    let new_mesh = generate_sphere(0.5, 8, 8, [0.0, 1.0, 0.0, 1.0]);
    state.edit_buffer.meshes.push(new_mesh);
    state
        .edit_buffer
        .mesh_transforms
        .push(MeshTransform::identity());

    assert_eq!(state.edit_buffer.meshes.len(), initial_count + 1);
    assert_eq!(
        state.edit_buffer.meshes.len(),
        state.edit_buffer.mesh_transforms.len()
    );
}

#[test]
fn test_remove_mesh_from_creature() {
    let mut state = CreaturesEditorState::new();

    // Create creature with multiple meshes
    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![
            generate_cube(1.0, [1.0, 0.0, 0.0, 1.0]),
            generate_sphere(0.5, 8, 8, [0.0, 1.0, 0.0, 1.0]),
            generate_cylinder(0.3, 1.0, 8, [0.0, 0.0, 1.0, 1.0]),
        ],
        mesh_transforms: vec![
            MeshTransform::identity(),
            MeshTransform::identity(),
            MeshTransform::identity(),
        ],
        scale: 1.0,
        color_tint: None,
    };

    let initial_count = state.edit_buffer.meshes.len();

    // Remove middle mesh
    state.edit_buffer.meshes.remove(1);
    state.edit_buffer.mesh_transforms.remove(1);

    assert_eq!(state.edit_buffer.meshes.len(), initial_count - 1);
    assert_eq!(
        state.edit_buffer.meshes.len(),
        state.edit_buffer.mesh_transforms.len()
    );
    assert_eq!(state.edit_buffer.meshes.len(), 2);
}

#[test]
fn test_duplicate_mesh() {
    let mut state = CreaturesEditorState::new();

    // Create creature with one mesh
    let original_mesh = generate_cube(1.0, [1.0, 0.0, 0.0, 1.0]);
    let mut transform = MeshTransform::identity();
    transform.translation = [1.0, 2.0, 3.0];

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![original_mesh.clone()],
        mesh_transforms: vec![transform],
        scale: 1.0,
        color_tint: None,
    };

    // Duplicate the mesh
    let mesh_clone = state.edit_buffer.meshes[0].clone();
    let transform_clone = state.edit_buffer.mesh_transforms[0];

    state.edit_buffer.meshes.push(mesh_clone);
    state.edit_buffer.mesh_transforms.push(transform_clone);

    assert_eq!(state.edit_buffer.meshes.len(), 2);
    assert_eq!(state.edit_buffer.mesh_transforms.len(), 2);

    // Verify duplicate has same properties
    assert_eq!(
        state.edit_buffer.meshes[0].color,
        state.edit_buffer.meshes[1].color
    );
    assert_eq!(
        state.edit_buffer.meshes[0].vertices.len(),
        state.edit_buffer.meshes[1].vertices.len()
    );
    assert_eq!(
        state.edit_buffer.mesh_transforms[0].translation,
        state.edit_buffer.mesh_transforms[1].translation
    );
}

#[test]
fn test_reorder_meshes() {
    let mut state = CreaturesEditorState::new();

    // Create creature with three distinct meshes
    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![
            generate_cube(1.0, [1.0, 0.0, 0.0, 1.0]),         // Red cube
            generate_sphere(0.5, 8, 8, [0.0, 1.0, 0.0, 1.0]), // Green sphere
            generate_cylinder(0.3, 1.0, 8, [0.0, 0.0, 1.0, 1.0]), // Blue cylinder
        ],
        mesh_transforms: vec![
            MeshTransform::identity(),
            MeshTransform::identity(),
            MeshTransform::identity(),
        ],
        scale: 1.0,
        color_tint: None,
    };

    // Swap first and last mesh
    state.edit_buffer.meshes.swap(0, 2);
    state.edit_buffer.mesh_transforms.swap(0, 2);

    // Verify order changed
    assert_eq!(state.edit_buffer.meshes[0].color, [0.0, 0.0, 1.0, 1.0]); // Blue now first
    assert_eq!(state.edit_buffer.meshes[2].color, [1.0, 0.0, 0.0, 1.0]); // Red now last
}

#[test]
fn test_update_mesh_transform() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 1.0, 1.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    // Update transform
    let mut transform = state.edit_buffer.mesh_transforms[0];
    transform.translation = [5.0, 10.0, 15.0];
    transform.rotation = [0.5, 1.0, 1.5];
    transform.scale = [2.0, 2.0, 2.0];

    state.edit_buffer.mesh_transforms[0] = transform;

    assert_eq!(
        state.edit_buffer.mesh_transforms[0].translation,
        [5.0, 10.0, 15.0]
    );
    assert_eq!(
        state.edit_buffer.mesh_transforms[0].rotation,
        [0.5, 1.0, 1.5]
    );
    assert_eq!(state.edit_buffer.mesh_transforms[0].scale, [2.0, 2.0, 2.0]);
}

#[test]
fn test_update_mesh_color() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    let original_color = state.edit_buffer.meshes[0].color;
    assert_eq!(original_color, [1.0, 0.0, 0.0, 1.0]);

    // Update color
    state.edit_buffer.meshes[0].color = [0.0, 1.0, 0.0, 0.5];

    assert_eq!(state.edit_buffer.meshes[0].color, [0.0, 1.0, 0.0, 0.5]);
    assert_ne!(state.edit_buffer.meshes[0].color, original_color);
}

#[test]
fn test_replace_mesh_with_primitive_cube() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_sphere(1.0, 8, 8, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    // Replace with cube
    state.primitive_type = PrimitiveType::Cube;
    state.primitive_size = 2.0;
    state.primitive_use_current_color = false;
    state.primitive_custom_color = [0.0, 1.0, 0.0, 1.0];
    state.selected_mesh_index = Some(0);

    let new_mesh = generate_cube(state.primitive_size, state.primitive_custom_color);
    state.edit_buffer.meshes[0] = new_mesh;

    assert_eq!(state.edit_buffer.meshes[0].color, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(state.edit_buffer.meshes[0].vertices.len(), 24); // Cube has 24 vertices
}

#[test]
fn test_replace_mesh_with_primitive_sphere() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    // Replace with sphere
    state.primitive_type = PrimitiveType::Sphere;
    state.primitive_size = 1.5;
    state.primitive_segments = 16;
    state.primitive_rings = 16;
    state.primitive_use_current_color = true;
    state.selected_mesh_index = Some(0);

    let current_color = state.edit_buffer.meshes[0].color;
    let new_mesh = generate_sphere(
        state.primitive_size,
        state.primitive_segments,
        state.primitive_rings,
        current_color,
    );
    state.edit_buffer.meshes[0] = new_mesh;

    assert_eq!(state.edit_buffer.meshes[0].color, [1.0, 0.0, 0.0, 1.0]);
    // Sphere with 16 segments and 16 rings has (16+1) * (16+1) = 289 vertices
    assert_eq!(state.edit_buffer.meshes[0].vertices.len(), 17 * 17);
}

#[test]
fn test_creature_scale_multiplier() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 1.0, 1.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    assert_eq!(state.edit_buffer.scale, 1.0);

    // Update scale
    state.edit_buffer.scale = 2.5;

    assert_eq!(state.edit_buffer.scale, 2.5);
}

#[test]
fn test_save_asset_to_file() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let campaign_dir = temp_dir.path().to_path_buf();

    let manager = CreatureAssetManager::new(campaign_dir.clone());

    // Create a test creature
    let creature = CreatureDefinition {
        id: 1,
        name: "Test Creature".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.5,
        color_tint: Some([1.0, 1.0, 1.0, 1.0]),
    };

    // Save creature
    let result = manager.save_creature(&creature);
    assert!(result.is_ok());

    // Verify registry exists and points to a per-creature asset file.
    let creatures_file = campaign_dir.join("data/creatures.ron");
    assert!(creatures_file.exists());

    let registry_text = std::fs::read_to_string(&creatures_file).unwrap();
    let registry = ron::from_str::<Vec<CreatureReference>>(&registry_text).unwrap();
    assert_eq!(registry.len(), 1);
    let asset_file = campaign_dir.join(&registry[0].filepath);
    assert!(
        asset_file.exists(),
        "asset file must exist: {}",
        registry[0].filepath
    );

    // Load and verify
    let loaded = manager.load_creature(1);
    assert!(loaded.is_ok());
    let loaded_creature = loaded.unwrap();

    assert_eq!(loaded_creature.id, creature.id);
    assert_eq!(loaded_creature.name, creature.name);
    assert_eq!(loaded_creature.scale, creature.scale);
    assert_eq!(loaded_creature.meshes.len(), creature.meshes.len());
}

#[test]
fn test_mesh_visibility_tracking() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![
            generate_cube(1.0, [1.0, 0.0, 0.0, 1.0]),
            generate_sphere(0.5, 8, 8, [0.0, 1.0, 0.0, 1.0]),
            generate_cylinder(0.3, 1.0, 8, [0.0, 0.0, 1.0, 1.0]),
        ],
        mesh_transforms: vec![
            MeshTransform::identity(),
            MeshTransform::identity(),
            MeshTransform::identity(),
        ],
        scale: 1.0,
        color_tint: None,
    };

    // Initialize visibility
    state.mesh_visibility = vec![true, true, true];

    assert_eq!(state.mesh_visibility.len(), 3);
    assert!(state.mesh_visibility[0]);
    assert!(state.mesh_visibility[1]);
    assert!(state.mesh_visibility[2]);

    // Hide middle mesh
    state.mesh_visibility[1] = false;

    assert!(state.mesh_visibility[0]);
    assert!(!state.mesh_visibility[1]);
    assert!(state.mesh_visibility[2]);
}

#[test]
fn test_primitive_type_enum() {
    let cube = PrimitiveType::Cube;
    let sphere = PrimitiveType::Sphere;
    let cylinder = PrimitiveType::Cylinder;
    let pyramid = PrimitiveType::Pyramid;
    let cone = PrimitiveType::Cone;

    assert_eq!(cube, PrimitiveType::Cube);
    assert_eq!(sphere, PrimitiveType::Sphere);
    assert_eq!(cylinder, PrimitiveType::Cylinder);
    assert_eq!(pyramid, PrimitiveType::Pyramid);
    assert_eq!(cone, PrimitiveType::Cone);

    assert_ne!(cube, sphere);
}

#[test]
fn test_uniform_scale_toggle() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 1.0, 1.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    assert!(state.uniform_scale); // Default should be true

    // When uniform scale is on, all axes should be equal
    let mut transform = state.edit_buffer.mesh_transforms[0];
    transform.scale = [2.0, 2.0, 2.0];
    state.edit_buffer.mesh_transforms[0] = transform;

    assert_eq!(transform.scale, [2.0, 2.0, 2.0]);

    // Disable uniform scale
    state.uniform_scale = false;

    // Now can set different scales
    transform.scale = [1.0, 2.0, 3.0];
    state.edit_buffer.mesh_transforms[0] = transform;

    assert_eq!(state.edit_buffer.mesh_transforms[0].scale, [1.0, 2.0, 3.0]);
}

#[test]
fn test_preview_dirty_flag() {
    let mut state = CreaturesEditorState::new();

    assert!(!state.preview_dirty); // Starts clean

    // Modifying creature should set dirty flag
    state.edit_buffer.scale = 2.0;
    state.preview_dirty = true;

    assert!(state.preview_dirty);

    // Can clear flag
    state.preview_dirty = false;

    assert!(!state.preview_dirty);
}

#[test]
fn test_mesh_transform_identity() {
    let transform = MeshTransform::identity();

    assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
    assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
}

#[test]
fn test_creature_color_tint_optional() {
    let mut state = CreaturesEditorState::new();

    state.edit_buffer = CreatureDefinition {
        id: 1,
        name: "Test".to_string(),
        meshes: vec![generate_cube(1.0, [1.0, 1.0, 1.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    };

    assert!(state.edit_buffer.color_tint.is_none());

    // Add tint
    state.edit_buffer.color_tint = Some([1.0, 0.5, 0.5, 1.0]);

    assert!(state.edit_buffer.color_tint.is_some());
    assert_eq!(state.edit_buffer.color_tint.unwrap(), [1.0, 0.5, 0.5, 1.0]);

    // Remove tint
    state.edit_buffer.color_tint = None;

    assert!(state.edit_buffer.color_tint.is_none());
}

#[test]
fn test_camera_distance_controls() {
    let mut state = CreaturesEditorState::new();

    assert_eq!(state.camera_distance, 5.0); // Default

    // Zoom in
    state.camera_distance = 2.0;
    assert_eq!(state.camera_distance, 2.0);

    // Zoom out
    state.camera_distance = 8.0;
    assert_eq!(state.camera_distance, 8.0);

    // Clamped in UI to 1.0..=10.0 range
    state.camera_distance = 1.0;
    assert_eq!(state.camera_distance, 1.0);

    state.camera_distance = 10.0;
    assert_eq!(state.camera_distance, 10.0);
}

#[test]
fn test_preview_options_defaults() {
    let state = CreaturesEditorState::new();

    assert!(state.show_grid);
    assert!(!state.show_wireframe);
    assert!(!state.show_normals);
    assert!(state.show_axes);
    assert_eq!(state.background_color, [0.2, 0.2, 0.25, 1.0]);
}

#[test]
fn test_mesh_name_optional() {
    let mut mesh = generate_cube(1.0, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.name.is_none());

    // Set name
    mesh.name = Some("TestCube".to_string());
    assert!(mesh.name.is_some());
    assert_eq!(mesh.name.unwrap(), "TestCube");

    // Clear name
    let mut mesh2 = generate_sphere(1.0, 8, 8, [1.0, 1.0, 1.0, 1.0]);
    mesh2.name = Some("TestSphere".to_string());
    mesh2.name = None;
    assert!(mesh2.name.is_none());
}
