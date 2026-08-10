// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OBJ Import/Export and Mesh Validation Tests
//!
//! Tests the mesh validation system and OBJ import/export functionality
//! in isolation from any mesh-editing types.

use antares::domain::visual::MeshDefinition;
use campaign_builder::mesh_obj_io::{export_mesh_to_obj, import_mesh_from_obj, ObjExportOptions};
use campaign_builder::mesh_validation::{is_valid_mesh, validate_mesh, MeshError};

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
// Edge Case Tests
// ============================================================================

#[test]
fn test_edge_case_obj_import_malformed() {
    let obj = "invalid obj file content";
    let result = import_mesh_from_obj(obj);
    assert!(result.is_err());
}
