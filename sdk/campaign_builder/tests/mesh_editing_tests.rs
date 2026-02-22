// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Advanced Mesh Editing Tools - Integration Tests
//!
//! Tests the complete mesh editing workflow including:
//! - Mesh validation system
//! - Vertex editing with manipulation
//! - Index/triangle editing
//! - Normal calculation and editing
//! - OBJ import/export

use antares::domain::visual::MeshDefinition;
use campaign_builder::mesh_index_editor::{MeshIndexEditor, Triangle};
use campaign_builder::mesh_normal_editor::{MeshNormalEditor, NormalMode};
use campaign_builder::mesh_obj_io::{export_mesh_to_obj, import_mesh_from_obj, ObjExportOptions};
use campaign_builder::mesh_validation::{is_valid_mesh, validate_mesh, MeshError};
use campaign_builder::mesh_vertex_editor::{MeshVertexEditor, SelectionMode};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_simple_triangle() -> MeshDefinition {
    MeshDefinition {
        name: Some("triangle".to_string()),
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

fn create_quad_mesh() -> MeshDefinition {
    MeshDefinition {
        name: Some("quad".to_string()),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        normals: None,
        uvs: None,
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

fn create_cube_mesh() -> MeshDefinition {
    MeshDefinition {
        name: Some("cube".to_string()),
        vertices: vec![
            // Front face
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
            // Back face
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
        ],
        indices: vec![
            // Front
            0, 1, 2, 0, 2, 3, // Back
            5, 4, 7, 5, 7, 6, // Top
            3, 2, 6, 3, 6, 7, // Bottom
            4, 5, 1, 4, 1, 0, // Right
            1, 5, 6, 1, 6, 2, // Left
            4, 0, 3, 4, 3, 7,
        ],
        normals: None,
        uvs: None,
        color: [0.8, 0.8, 0.8, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

// ============================================================================
// Mesh Validation Tests
// ============================================================================

#[test]
fn test_validation_valid_mesh() {
    let mesh = create_simple_triangle();
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
    assert_eq!(report.errors.len(), 0);
}

#[test]
fn test_validation_no_vertices() {
    let mut mesh = create_simple_triangle();
    mesh.vertices.clear();
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, MeshError::NoVertices)));
}

#[test]
fn test_validation_no_indices() {
    let mut mesh = create_simple_triangle();
    mesh.indices.clear();
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, MeshError::NoIndices)));
}

#[test]
fn test_validation_invalid_index() {
    let mut mesh = create_simple_triangle();
    mesh.indices = vec![0, 1, 99]; // Index 99 out of range
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, MeshError::InvalidIndex { .. })));
}

#[test]
fn test_validation_degenerate_triangle() {
    let mut mesh = create_simple_triangle();
    mesh.vertices = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, MeshError::DegenerateTriangle { .. })));
}

#[test]
fn test_validation_mismatched_normals() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![[0.0, 1.0, 0.0]]); // Only 1 normal for 3 vertices
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, MeshError::MismatchedNormalCount { .. })));
}

#[test]
fn test_validation_unnormalized_normal_warning() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![
        [0.0, 2.0, 0.0], // Not normalized
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ]);
    let report = validate_mesh(&mesh);
    assert!(report.is_valid()); // Still valid, just a warning
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_validation_info_contains_stats() {
    let mesh = create_quad_mesh();
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
    assert!(!report.info.is_empty());
    // Should contain vertex count, triangle count, etc.
}

#[test]
fn test_validation_helper_function() {
    let mesh = create_simple_triangle();
    assert!(is_valid_mesh(&mesh));

    let mut invalid_mesh = create_simple_triangle();
    invalid_mesh.vertices.clear();
    assert!(!is_valid_mesh(&invalid_mesh));
}

// ============================================================================
// Vertex Editor Tests
// ============================================================================

#[test]
fn test_vertex_editor_creation() {
    let mesh = create_quad_mesh();
    let editor = MeshVertexEditor::new(mesh);
    assert_eq!(editor.vertex_count(), 4);
    assert_eq!(editor.selected_vertices().len(), 0);
}

#[test]
fn test_vertex_selection_replace_mode() {
    let mesh = create_quad_mesh();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_vertex(0);
    assert!(editor.is_vertex_selected(0));

    editor.select_vertex(1);
    assert!(!editor.is_vertex_selected(0));
    assert!(editor.is_vertex_selected(1));
}

#[test]
fn test_vertex_selection_add_mode() {
    let mesh = create_quad_mesh();
    let mut editor = MeshVertexEditor::new(mesh);
    editor.set_selection_mode(SelectionMode::Add);

    editor.select_vertex(0);
    editor.select_vertex(1);
    editor.select_vertex(2);

    assert_eq!(editor.selected_vertices().len(), 3);
}

#[test]
fn test_vertex_translate() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_vertex(0);
    editor.translate_selected([1.0, 2.0, 3.0]);

    let vertex = editor.mesh().vertices[0];
    assert_eq!(vertex, [1.0, 2.0, 3.0]);
}

#[test]
fn test_vertex_scale() {
    let mesh = create_quad_mesh();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_all();
    editor.scale_selected([2.0, 2.0, 2.0]);

    // All vertices should be scaled by 2
    for vertex in &editor.mesh().vertices {
        assert!(vertex[0].abs() <= 2.0);
        assert!(vertex[1].abs() <= 2.0);
    }
}

#[test]
fn test_vertex_snap_to_grid() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.set_snap_to_grid(true);
    editor.set_grid_snap_size(0.5);
    editor.select_vertex(0);
    editor.translate_selected([0.23, 0.78, 0.0]);

    let vertex = editor.mesh().vertices[0];
    // Should snap to nearest 0.5
    assert!((vertex[0] - 0.0).abs() < 0.01);
    assert!((vertex[1] - 1.0).abs() < 0.01);
}

#[test]
fn test_vertex_add_and_delete() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    let new_idx = editor.add_vertex([5.0, 5.0, 5.0]);
    assert_eq!(new_idx, 3);
    assert_eq!(editor.vertex_count(), 4);

    editor.select_vertex(new_idx);
    editor.delete_selected();
    assert_eq!(editor.vertex_count(), 3);
}

#[test]
fn test_vertex_duplicate() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_vertex(0);
    let duplicates = editor.duplicate_selected();

    assert_eq!(duplicates.len(), 1);
    assert_eq!(editor.vertex_count(), 4);
    assert_eq!(editor.mesh().vertices[3], editor.mesh().vertices[0]);
}

#[test]
fn test_vertex_merge() {
    let mut mesh = create_simple_triangle();
    mesh.vertices.push([0.01, 0.01, 0.01]); // Very close to first vertex
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_vertex(0);
    editor.set_selection_mode(SelectionMode::Add);
    editor.select_vertex(3);
    editor.merge_selected(0.1);

    assert_eq!(editor.vertex_count(), 3);
}

#[test]
fn test_vertex_undo_redo() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    let original_pos = editor.mesh().vertices[0];

    editor.select_vertex(0);
    editor.translate_selected([1.0, 0.0, 0.0]);

    assert_ne!(editor.mesh().vertices[0], original_pos);
    assert!(editor.can_undo());

    editor.undo();
    assert_eq!(editor.mesh().vertices[0], original_pos);
    assert!(editor.can_redo());

    editor.redo();
    assert_ne!(editor.mesh().vertices[0], original_pos);
}

#[test]
fn test_vertex_selection_center_calculation() {
    let mesh = create_quad_mesh();
    let mut editor = MeshVertexEditor::new(mesh);

    editor.select_all();
    let center = editor.calculate_selection_center();

    // Center of unit square should be at (0.5, 0.5, 0.0)
    assert!((center[0] - 0.5).abs() < 0.01);
    assert!((center[1] - 0.5).abs() < 0.01);
    assert!(center[2].abs() < 0.01);
}

// ============================================================================
// Index Editor Tests
// ============================================================================

#[test]
fn test_index_editor_creation() {
    let mesh = create_quad_mesh();
    let editor = MeshIndexEditor::new(mesh);
    assert_eq!(editor.triangle_count(), 2);
}

#[test]
fn test_index_get_triangle() {
    let mesh = create_quad_mesh();
    let editor = MeshIndexEditor::new(mesh);

    let triangle = editor.get_triangle(0).unwrap();
    assert_eq!(triangle.vertices(), [0, 1, 2]);
}

#[test]
fn test_index_set_triangle() {
    let mesh = create_quad_mesh();
    let mut editor = MeshIndexEditor::new(mesh);

    let new_triangle = Triangle::new(1, 2, 3);
    editor.set_triangle(0, new_triangle);

    let triangle = editor.get_triangle(0).unwrap();
    assert_eq!(triangle.vertices(), [1, 2, 3]);
}

#[test]
fn test_index_add_triangle() {
    let mesh = create_quad_mesh();
    let mut editor = MeshIndexEditor::new(mesh);

    let triangle = Triangle::new(0, 1, 3);
    editor.add_triangle(triangle);

    assert_eq!(editor.triangle_count(), 3);
}

#[test]
fn test_index_delete_triangle() {
    let mesh = create_quad_mesh();
    let mut editor = MeshIndexEditor::new(mesh);

    editor.select_triangle(0);
    editor.delete_selected();

    assert_eq!(editor.triangle_count(), 1);
}

#[test]
fn test_index_flip_triangle() {
    let mesh = create_simple_triangle();
    let mut editor = MeshIndexEditor::new(mesh);

    let original = editor.get_triangle(0).unwrap();
    assert_eq!(original.vertices(), [0, 1, 2]);

    editor.select_triangle(0);
    editor.flip_selected();

    let flipped = editor.get_triangle(0).unwrap();
    assert_eq!(flipped.vertices(), [0, 2, 1]);
}

#[test]
fn test_index_remove_degenerate() {
    let mut mesh = create_quad_mesh();
    mesh.indices.extend_from_slice(&[0, 0, 1]); // Degenerate triangle
    let mut editor = MeshIndexEditor::new(mesh);

    assert_eq!(editor.triangle_count(), 3);

    let removed = editor.remove_degenerate_triangles();
    assert_eq!(removed, 1);
    assert_eq!(editor.triangle_count(), 2);
}

#[test]
fn test_index_validate() {
    let mut mesh = create_quad_mesh();
    mesh.indices.push(99); // Invalid
    mesh.indices.push(100);
    mesh.indices.push(101);
    let editor = MeshIndexEditor::new(mesh);

    let invalid = editor.validate_indices();
    assert_eq!(invalid.len(), 3);
}

#[test]
fn test_index_find_triangles_using_vertex() {
    let mesh = create_quad_mesh();
    let editor = MeshIndexEditor::new(mesh);

    let triangles = editor.find_triangles_using_vertex(0);
    assert_eq!(triangles.len(), 2); // Vertex 0 is shared
}

#[test]
fn test_index_find_adjacent_triangles() {
    let mesh = create_quad_mesh();
    let editor = MeshIndexEditor::new(mesh);

    let adjacent = editor.find_adjacent_triangles();
    assert!(!adjacent.is_empty()); // Quad's triangles share an edge
}

#[test]
fn test_index_grow_selection() {
    let mesh = create_quad_mesh();
    let mut editor = MeshIndexEditor::new(mesh);

    editor.select_triangle(0);
    editor.grow_selection(1);

    // After growing, both triangles should be selected
    assert_eq!(editor.selected_triangles().len(), 2);
}

#[test]
fn test_index_undo_redo() {
    let mesh = create_quad_mesh();
    let mut editor = MeshIndexEditor::new(mesh);

    let original_count = editor.triangle_count();

    editor.add_triangle(Triangle::new(0, 1, 2));
    assert_eq!(editor.triangle_count(), original_count + 1);

    editor.undo();
    assert_eq!(editor.triangle_count(), original_count);

    editor.redo();
    assert_eq!(editor.triangle_count(), original_count + 1);
}

// ============================================================================
// Normal Editor Tests
// ============================================================================

#[test]
fn test_normal_editor_creation() {
    let mesh = create_simple_triangle();
    let editor = MeshNormalEditor::new(mesh);
    assert!(editor.mesh().normals.is_none());
}

#[test]
fn test_normal_calculate_flat() {
    let mesh = create_simple_triangle();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.calculate_flat_normals();

    assert!(editor.mesh().normals.is_some());
    let normals = editor.mesh().normals.as_ref().unwrap();
    assert_eq!(normals.len(), 3);
}

#[test]
fn test_normal_calculate_smooth() {
    let mesh = create_quad_mesh();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.calculate_smooth_normals();

    assert!(editor.mesh().normals.is_some());
    let normals = editor.mesh().normals.as_ref().unwrap();
    assert_eq!(normals.len(), 4);
}

#[test]
fn test_normal_calculate_weighted_smooth() {
    let mesh = create_quad_mesh();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.calculate_weighted_smooth_normals();

    assert!(editor.mesh().normals.is_some());
    let normals = editor.mesh().normals.as_ref().unwrap();
    assert_eq!(normals.len(), 4);
}

#[test]
fn test_normal_modes() {
    let mesh = create_quad_mesh();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.calculate_normals(NormalMode::Flat);
    assert!(editor.mesh().normals.is_some());

    editor.calculate_normals(NormalMode::Smooth);
    assert!(editor.mesh().normals.is_some());

    editor.calculate_normals(NormalMode::WeightedSmooth);
    assert!(editor.mesh().normals.is_some());
}

#[test]
fn test_normal_set_and_get() {
    let mesh = create_simple_triangle();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.set_normal(0, [1.0, 0.0, 0.0]);
    let normal = editor.get_normal(0).unwrap();

    // Should be normalized
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    assert!((length - 1.0).abs() < 0.01);
}

#[test]
fn test_normal_flip_all() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);
    let mut editor = MeshNormalEditor::new(mesh);

    editor.flip_all_normals();

    let normals = editor.mesh().normals.as_ref().unwrap();
    for normal in normals {
        assert!((normal[2] + 1.0).abs() < 0.01); // Should be -1.0
    }
}

#[test]
fn test_normal_flip_specific() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);
    let mut editor = MeshNormalEditor::new(mesh);

    editor.flip_normals(vec![0, 2]);

    let normals = editor.mesh().normals.as_ref().unwrap();
    assert!((normals[0][2] + 1.0).abs() < 0.01);
    assert!((normals[1][2] - 1.0).abs() < 0.01);
    assert!((normals[2][2] + 1.0).abs() < 0.01);
}

#[test]
fn test_normal_remove() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![[0.0, 1.0, 0.0]; 3]);
    let mut editor = MeshNormalEditor::new(mesh);

    assert!(editor.mesh().normals.is_some());
    editor.remove_normals();
    assert!(editor.mesh().normals.is_none());
}

#[test]
fn test_normal_auto_normalize() {
    let mesh = create_simple_triangle();
    let mut editor = MeshNormalEditor::new(mesh);

    editor.set_auto_normalize(true);
    editor.set_normal(0, [2.0, 0.0, 0.0]); // Not normalized

    let normal = editor.get_normal(0).unwrap();
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    assert!((length - 1.0).abs() < 0.01);
}

// ============================================================================
// OBJ Import/Export Tests
// ============================================================================

#[test]
fn test_obj_export_simple() {
    let mesh = create_simple_triangle();
    let obj = export_mesh_to_obj(&mesh).unwrap();

    assert!(obj.contains("v 0.0"));
    assert!(obj.contains("v 1.0"));
    assert!(obj.contains("f "));
}

#[test]
fn test_obj_import_simple() {
    let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
    let mesh = import_mesh_from_obj(obj).unwrap();

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
}

#[test]
fn test_obj_export_import_roundtrip() {
    let original = create_quad_mesh();
    let obj = export_mesh_to_obj(&original).unwrap();
    let imported = import_mesh_from_obj(&obj).unwrap();

    assert_eq!(imported.vertices.len(), original.vertices.len());
    assert_eq!(imported.indices.len(), original.indices.len());
}

#[test]
fn test_obj_import_with_normals() {
    let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
               vn 0.0 0.0 1.0\nvn 0.0 0.0 1.0\nvn 0.0 0.0 1.0\n\
               f 1//1 2//2 3//3\n";
    let mesh = import_mesh_from_obj(obj).unwrap();

    assert!(mesh.normals.is_some());
    assert_eq!(mesh.normals.as_ref().unwrap().len(), 3);
}

#[test]
fn test_obj_import_quad_triangulation() {
    let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3 4\n";
    let mesh = import_mesh_from_obj(obj).unwrap();

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 6); // Quad -> 2 triangles
}

#[test]
fn test_obj_export_with_options() {
    let mesh = create_quad_mesh();
    let options = ObjExportOptions {
        include_normals: false,
        include_comments: false,
        ..Default::default()
    };
    let obj =
        campaign_builder::mesh_obj_io::export_mesh_to_obj_with_options(&mesh, &options).unwrap();

    assert!(!obj.contains("vn "));
    assert!(!obj.contains("#"));
}

// ============================================================================
// Integration Workflow Tests
// ============================================================================

#[test]
fn test_workflow_create_edit_validate() {
    // Create a mesh
    let mut mesh = create_simple_triangle();

    // Edit vertices - translate in Z to keep triangle valid
    let mut vertex_editor = MeshVertexEditor::new(mesh);
    vertex_editor.select_vertex(0);
    vertex_editor.translate_selected([0.0, 0.0, 0.5]);
    mesh = vertex_editor.into_mesh();

    // Add normals
    let mut normal_editor = MeshNormalEditor::new(mesh);
    normal_editor.calculate_flat_normals();
    mesh = normal_editor.into_mesh();

    // Validate
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
    assert!(mesh.normals.is_some());
}

#[test]
fn test_workflow_import_edit_export() {
    // Import from OBJ
    let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3 4\n";
    let mut mesh = import_mesh_from_obj(obj).unwrap();

    // Edit vertices
    let mut vertex_editor = MeshVertexEditor::new(mesh);
    vertex_editor.select_all();
    vertex_editor.scale_selected([2.0, 2.0, 2.0]);
    mesh = vertex_editor.into_mesh();

    // Calculate normals
    let mut normal_editor = MeshNormalEditor::new(mesh);
    normal_editor.calculate_smooth_normals();
    mesh = normal_editor.into_mesh();

    // Export to OBJ
    let exported_obj = export_mesh_to_obj(&mesh).unwrap();
    assert!(exported_obj.contains("vn "));
}

#[test]
fn test_workflow_complex_editing_sequence() {
    let mut mesh = create_cube_mesh();

    // Step 1: Validate initial mesh
    assert!(is_valid_mesh(&mesh));

    // Step 2: Edit some vertices
    let mut vertex_editor = MeshVertexEditor::new(mesh);
    vertex_editor.select_vertex(0);
    vertex_editor.set_selection_mode(SelectionMode::Add);
    vertex_editor.select_vertex(1);
    vertex_editor.translate_selected([0.0, 0.0, 0.2]);
    mesh = vertex_editor.into_mesh();

    // Step 3: Flip some triangles
    let mut index_editor = MeshIndexEditor::new(mesh);
    index_editor.select_triangle(0);
    index_editor.flip_selected();
    mesh = index_editor.into_mesh();

    // Step 4: Calculate normals
    let mut normal_editor = MeshNormalEditor::new(mesh);
    normal_editor.calculate_weighted_smooth_normals();
    mesh = normal_editor.into_mesh();

    // Step 5: Final validation
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
    assert!(mesh.normals.is_some());
}

#[test]
fn test_workflow_error_recovery() {
    let mut mesh = create_quad_mesh();

    // Introduce an error by adding invalid index
    mesh.indices.push(99);
    mesh.indices.push(100);
    mesh.indices.push(101);

    // Validate and detect error
    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());

    // Fix using index editor
    let mut index_editor = MeshIndexEditor::new(mesh);
    let invalid = index_editor.validate_indices();
    assert!(!invalid.is_empty());

    // Remove the bad triangle
    index_editor.select_triangle(2); // The invalid one
    index_editor.delete_selected();
    mesh = index_editor.into_mesh();

    // Re-validate
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
}

#[test]
fn test_workflow_undo_redo_across_operations() {
    let mesh = create_simple_triangle();
    let mut vertex_editor = MeshVertexEditor::new(mesh);

    let original_pos = vertex_editor.mesh().vertices[0];

    // Perform multiple operations
    vertex_editor.select_vertex(0);
    vertex_editor.translate_selected([1.0, 0.0, 0.0]);
    vertex_editor.translate_selected([0.0, 1.0, 0.0]);
    vertex_editor.translate_selected([0.0, 0.0, 1.0]);

    // Undo all
    vertex_editor.undo();
    vertex_editor.undo();
    vertex_editor.undo();

    assert_eq!(vertex_editor.mesh().vertices[0], original_pos);
}

#[test]
fn test_workflow_validation_messages() {
    let mut mesh = create_simple_triangle();
    mesh.normals = Some(vec![[0.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]]);

    let report = validate_mesh(&mesh);

    let error_msgs = report.error_messages();
    let warning_msgs = report.warning_messages();
    let info_msgs = report.info_messages();

    assert!(error_msgs.is_empty());
    assert!(!warning_msgs.is_empty());
    assert!(!info_msgs.is_empty());

    let summary = report.summary();
    assert!(summary.contains("Errors: 0"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_edge_case_empty_mesh_handling() {
    let mesh = MeshDefinition {
        name: Some("empty".to_string()),
        vertices: vec![],
        indices: vec![],
        normals: None,
        uvs: None,
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    };

    let report = validate_mesh(&mesh);
    assert!(!report.is_valid());
}

#[test]
fn test_edge_case_single_vertex() {
    let mesh = MeshDefinition {
        name: Some("single".to_string()),
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

    let report = validate_mesh(&mesh);
    assert!(!report.is_valid()); // No triangles
}

#[test]
fn test_edge_case_large_mesh_performance() {
    // Create a mesh with many vertices
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..100 {
        for j in 0..100 {
            vertices.push([i as f32, j as f32, 0.0]);
        }
    }

    // Create triangle strip
    for i in 0..99 {
        for j in 0..99 {
            let idx = i * 100 + j;
            indices.push(idx as u32);
            indices.push((idx + 1) as u32);
            indices.push((idx + 100) as u32);
            indices.push((idx + 1) as u32);
            indices.push((idx + 101) as u32);
            indices.push((idx + 100) as u32);
        }
    }

    let mesh = MeshDefinition {
        name: Some("large".to_string()),
        vertices,
        indices,
        normals: None,
        uvs: None,
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    };

    // Should handle large mesh efficiently
    let report = validate_mesh(&mesh);
    assert!(report.is_valid());
}

#[test]
fn test_edge_case_obj_import_malformed() {
    let obj = "invalid obj file content";
    let result = import_mesh_from_obj(obj);
    assert!(result.is_err());
}

#[test]
fn test_edge_case_vertex_editor_out_of_bounds() {
    let mesh = create_simple_triangle();
    let mut editor = MeshVertexEditor::new(mesh);

    // Try to select vertex that doesn't exist
    editor.select_vertex(999);
    assert!(!editor.is_vertex_selected(999));
}
