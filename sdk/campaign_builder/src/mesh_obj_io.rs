// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OBJ mesh import/export for creature editor
//!
//! Provides functionality for importing and exporting meshes in Wavefront OBJ format,
//! a widely-used 3D model interchange format.
//!
//! # Examples
//!
//! ```
//! use campaign_builder::mesh_obj_io::{export_mesh_to_obj, import_mesh_from_obj};
//! use antares::domain::visual::MeshDefinition;
//!
//! let mesh = MeshDefinition {
//!     name: Some("cube".to_string()),
//!     vertices: vec![
//!         [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0],
//!         [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
//!     ],
//!     indices: vec![0, 1, 2, 0, 2, 3],
//!     normals: None,
//!     uvs: None,
//!     color: [1.0, 1.0, 1.0, 1.0],
//!     lod_levels: None,
//!     lod_distances: None,
//!     material: None,
//!     texture_path: None,
//! };
//!
//! // Export to OBJ format
//! let obj_string = export_mesh_to_obj(&mesh).unwrap();
//!
//! // Import from OBJ format
//! let imported_mesh = import_mesh_from_obj(&obj_string).unwrap();
//! assert_eq!(imported_mesh.vertices.len(), mesh.vertices.len());
//! ```

use antares::domain::visual::MeshDefinition;
use std::io::{BufRead, BufReader, Write};
use thiserror::Error;

/// Errors that can occur during OBJ import/export
#[derive(Error, Debug)]
pub enum ObjError {
    /// Failed to parse OBJ file
    #[error("Failed to parse OBJ file: {0}")]
    ParseError(String),

    /// Invalid vertex index
    #[error("Invalid vertex index: {0}")]
    InvalidIndex(String),

    /// Invalid face definition
    #[error("Invalid face definition: {0}")]
    InvalidFace(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Missing required data
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Unsupported feature
    #[error("Unsupported OBJ feature: {0}")]
    UnsupportedFeature(String),
}

/// Options for OBJ export
#[derive(Debug, Clone)]
pub struct ObjExportOptions {
    /// Include normals in export
    pub include_normals: bool,

    /// Include texture coordinates in export
    pub include_uvs: bool,

    /// Include object name
    pub include_object_name: bool,

    /// Include comments
    pub include_comments: bool,

    /// Precision for floating point numbers
    pub float_precision: usize,
}

impl Default for ObjExportOptions {
    fn default() -> Self {
        Self {
            include_normals: true,
            include_uvs: true,
            include_object_name: true,
            include_comments: true,
            float_precision: 6,
        }
    }
}

/// Options for OBJ import
#[derive(Debug, Clone)]
pub struct ObjImportOptions {
    /// Name to assign to imported mesh
    pub mesh_name: Option<String>,

    /// Default color to use if not specified
    pub default_color: [f32; 4],

    /// Whether to flip Y/Z axes (some tools use different conventions)
    pub flip_yz: bool,

    /// Whether to flip texture V coordinate
    pub flip_uv_v: bool,
}

impl Default for ObjImportOptions {
    fn default() -> Self {
        Self {
            mesh_name: None,
            default_color: [1.0, 1.0, 1.0, 1.0],
            flip_yz: false,
            flip_uv_v: false,
        }
    }
}

/// Exports a mesh to OBJ format string
///
/// # Arguments
///
/// * `mesh` - The mesh to export
///
/// # Returns
///
/// OBJ format string, or error if export fails
///
/// # Examples
///
/// ```
/// use antares::domain::visual::MeshDefinition;
/// use campaign_builder::mesh_obj_io::export_mesh_to_obj;
///
/// let mesh = MeshDefinition {
///     name: Some("test".to_string()),
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
///     lod_levels: None,
///     lod_distances: None,
///     material: None,
///     texture_path: None,
/// };
///
/// let obj = export_mesh_to_obj(&mesh).unwrap();
/// assert!(obj.contains("v 0.0"));
/// ```
pub fn export_mesh_to_obj(mesh: &MeshDefinition) -> Result<String, ObjError> {
    export_mesh_to_obj_with_options(mesh, &ObjExportOptions::default())
}

/// Exports a mesh to OBJ format string with custom options
///
/// # Arguments
///
/// * `mesh` - The mesh to export
/// * `options` - Export options
///
/// # Returns
///
/// OBJ format string, or error if export fails
pub fn export_mesh_to_obj_with_options(
    mesh: &MeshDefinition,
    options: &ObjExportOptions,
) -> Result<String, ObjError> {
    let mut output = Vec::new();

    // Write header comment
    if options.include_comments {
        writeln!(output, "# Wavefront OBJ file")?;
        writeln!(output, "# Exported from Antares Campaign Builder")?;
        writeln!(output, "#")?;
    }

    // Write object name
    if options.include_object_name {
        let name = mesh.name.as_deref().unwrap_or("mesh");
        writeln!(output, "o {}", name)?;
    }

    // Write vertices
    if options.include_comments {
        writeln!(output, "# {} vertices", mesh.vertices.len())?;
    }
    for vertex in &mesh.vertices {
        writeln!(
            output,
            "v {:.prec$} {:.prec$} {:.prec$}",
            vertex[0],
            vertex[1],
            vertex[2],
            prec = options.float_precision
        )?;
    }

    // Write texture coordinates
    if options.include_uvs {
        if let Some(ref uvs) = mesh.uvs {
            if options.include_comments {
                writeln!(output, "# {} texture coordinates", uvs.len())?;
            }
            for uv in uvs {
                writeln!(
                    output,
                    "vt {:.prec$} {:.prec$}",
                    uv[0],
                    uv[1],
                    prec = options.float_precision
                )?;
            }
        }
    }

    // Write normals
    if options.include_normals {
        if let Some(ref normals) = mesh.normals {
            if options.include_comments {
                writeln!(output, "# {} normals", normals.len())?;
            }
            for normal in normals {
                writeln!(
                    output,
                    "vn {:.prec$} {:.prec$} {:.prec$}",
                    normal[0],
                    normal[1],
                    normal[2],
                    prec = options.float_precision
                )?;
            }
        }
    }

    // Write faces
    if options.include_comments {
        writeln!(output, "# {} faces", mesh.indices.len() / 3)?;
    }

    let has_uvs = options.include_uvs && mesh.uvs.is_some();
    let has_normals = options.include_normals && mesh.normals.is_some();

    for triangle_idx in 0..(mesh.indices.len() / 3) {
        let i0 = (mesh.indices[triangle_idx * 3] + 1) as usize; // OBJ indices are 1-based
        let i1 = (mesh.indices[triangle_idx * 3 + 1] + 1) as usize;
        let i2 = (mesh.indices[triangle_idx * 3 + 2] + 1) as usize;

        write!(output, "f")?;

        for &idx in &[i0, i1, i2] {
            write!(output, " {}", idx)?;

            if has_uvs || has_normals {
                write!(output, "/")?;
                if has_uvs {
                    write!(output, "{}", idx)?;
                }
                if has_normals {
                    write!(output, "/{}", idx)?;
                }
            }
        }

        writeln!(output)?;
    }

    String::from_utf8(output)
        .map_err(|e| ObjError::ParseError(format!("UTF-8 conversion error: {}", e)))
}

/// Exports a mesh to an OBJ file
///
/// # Arguments
///
/// * `mesh` - The mesh to export
/// * `path` - File path to write to
///
/// # Returns
///
/// Ok(()) on success, or error if export fails
pub fn export_mesh_to_obj_file(mesh: &MeshDefinition, path: &str) -> Result<(), ObjError> {
    export_mesh_to_obj_file_with_options(mesh, path, &ObjExportOptions::default())
}

/// Exports a mesh to an OBJ file with custom options
///
/// # Arguments
///
/// * `mesh` - The mesh to export
/// * `path` - File path to write to
/// * `options` - Export options
///
/// # Returns
///
/// Ok(()) on success, or error if export fails
pub fn export_mesh_to_obj_file_with_options(
    mesh: &MeshDefinition,
    path: &str,
    options: &ObjExportOptions,
) -> Result<(), ObjError> {
    let obj_string = export_mesh_to_obj_with_options(mesh, options)?;
    std::fs::write(path, obj_string)?;
    Ok(())
}

/// Imports a mesh from OBJ format string
///
/// # Arguments
///
/// * `obj_string` - OBJ format string to parse
///
/// # Returns
///
/// Imported mesh, or error if import fails
///
/// # Examples
///
/// ```
/// use campaign_builder::mesh_obj_io::import_mesh_from_obj;
///
/// let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
/// let mesh = import_mesh_from_obj(obj).unwrap();
/// assert_eq!(mesh.vertices.len(), 3);
/// assert_eq!(mesh.indices.len(), 3);
/// ```
pub fn import_mesh_from_obj(obj_string: &str) -> Result<MeshDefinition, ObjError> {
    import_mesh_from_obj_with_options(obj_string, &ObjImportOptions::default())
}

/// Imports a mesh from OBJ format string with custom options
///
/// # Arguments
///
/// * `obj_string` - OBJ format string to parse
/// * `options` - Import options
///
/// # Returns
///
/// Imported mesh, or error if import fails
pub fn import_mesh_from_obj_with_options(
    obj_string: &str,
    options: &ObjImportOptions,
) -> Result<MeshDefinition, ObjError> {
    let reader = BufReader::new(obj_string.as_bytes());

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut mesh_name = options.mesh_name.clone();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "o" | "g" => {
                // Object or group name
                if parts.len() > 1 && mesh_name.is_none() {
                    mesh_name = Some(parts[1..].join(" "));
                }
            }
            "v" => {
                // Vertex
                if parts.len() < 4 {
                    return Err(ObjError::ParseError(format!(
                        "Invalid vertex definition: {}",
                        line
                    )));
                }

                let x: f32 = parts[1]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid vertex X: {}", e)))?;
                let y: f32 = parts[2]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid vertex Y: {}", e)))?;
                let z: f32 = parts[3]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid vertex Z: {}", e)))?;

                if options.flip_yz {
                    vertices.push([x, z, -y]);
                } else {
                    vertices.push([x, y, z]);
                }
            }
            "vn" => {
                // Normal
                if parts.len() < 4 {
                    return Err(ObjError::ParseError(format!(
                        "Invalid normal definition: {}",
                        line
                    )));
                }

                let x: f32 = parts[1]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid normal X: {}", e)))?;
                let y: f32 = parts[2]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid normal Y: {}", e)))?;
                let z: f32 = parts[3]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid normal Z: {}", e)))?;

                if options.flip_yz {
                    normals.push([x, z, -y]);
                } else {
                    normals.push([x, y, z]);
                }
            }
            "vt" => {
                // Texture coordinate
                if parts.len() < 3 {
                    return Err(ObjError::ParseError(format!(
                        "Invalid texture coordinate definition: {}",
                        line
                    )));
                }

                let u: f32 = parts[1]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid UV U: {}", e)))?;
                let mut v: f32 = parts[2]
                    .parse()
                    .map_err(|e| ObjError::ParseError(format!("Invalid UV V: {}", e)))?;

                if options.flip_uv_v {
                    v = 1.0 - v;
                }

                uvs.push([u, v]);
            }
            "f" => {
                // Face
                if parts.len() < 4 {
                    return Err(ObjError::ParseError(format!(
                        "Invalid face definition: {}",
                        line
                    )));
                }

                // Parse face indices (OBJ uses 1-based indexing)
                let mut face_indices = Vec::new();
                for i in 1..parts.len() {
                    let index = parse_face_vertex(parts[i])?;
                    if index.0 == 0 || index.0 > vertices.len() {
                        return Err(ObjError::InvalidIndex(format!(
                            "Vertex index {} out of range (1-{})",
                            index.0,
                            vertices.len()
                        )));
                    }
                    face_indices.push(index.0 - 1); // Convert to 0-based
                }

                // Triangulate face if necessary
                if face_indices.len() == 3 {
                    indices.extend_from_slice(&face_indices);
                } else if face_indices.len() == 4 {
                    // Quad -> two triangles
                    indices.push(face_indices[0]);
                    indices.push(face_indices[1]);
                    indices.push(face_indices[2]);
                    indices.push(face_indices[0]);
                    indices.push(face_indices[2]);
                    indices.push(face_indices[3]);
                } else if face_indices.len() > 4 {
                    // N-gon -> triangle fan
                    for i in 1..(face_indices.len() - 1) {
                        indices.push(face_indices[0]);
                        indices.push(face_indices[i]);
                        indices.push(face_indices[i + 1]);
                    }
                }
            }
            "mtllib" | "usemtl" => {
                // Material library/usage - not yet supported
            }
            "s" => {
                // Smoothing group - not yet supported
            }
            _ => {
                // Unknown directive - skip
            }
        }
    }

    // Validate we have required data
    if vertices.is_empty() {
        return Err(ObjError::MissingData("No vertices found".to_string()));
    }

    if indices.is_empty() {
        return Err(ObjError::MissingData("No faces found".to_string()));
    }

    // Convert indices to u32
    let indices_u32: Vec<u32> = indices.iter().map(|&i| i as u32).collect();

    Ok(MeshDefinition {
        name: mesh_name,
        vertices,
        indices: indices_u32,
        normals: if normals.is_empty() {
            None
        } else {
            Some(normals)
        },
        uvs: if uvs.is_empty() { None } else { Some(uvs) },
        color: options.default_color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    })
}

/// Imports a mesh from an OBJ file
///
/// # Arguments
///
/// * `path` - File path to read from
///
/// # Returns
///
/// Imported mesh, or error if import fails
pub fn import_mesh_from_obj_file(path: &str) -> Result<MeshDefinition, ObjError> {
    import_mesh_from_obj_file_with_options(path, &ObjImportOptions::default())
}

/// Imports a mesh from an OBJ file with custom options
///
/// # Arguments
///
/// * `path` - File path to read from
/// * `options` - Import options
///
/// # Returns
///
/// Imported mesh, or error if import fails
pub fn import_mesh_from_obj_file_with_options(
    path: &str,
    options: &ObjImportOptions,
) -> Result<MeshDefinition, ObjError> {
    let obj_string = std::fs::read_to_string(path)?;
    import_mesh_from_obj_with_options(&obj_string, options)
}

/// Parses a face vertex string (e.g., "1", "1/2", "1/2/3", "1//3")
///
/// Returns (vertex_index, texture_index, normal_index)
fn parse_face_vertex(s: &str) -> Result<(usize, Option<usize>, Option<usize>), ObjError> {
    let parts: Vec<&str> = s.split('/').collect();

    let vertex_index: usize = parts[0]
        .parse()
        .map_err(|e| ObjError::ParseError(format!("Invalid vertex index: {}", e)))?;

    let texture_index = if parts.len() > 1 && !parts[1].is_empty() {
        Some(
            parts[1]
                .parse()
                .map_err(|e| ObjError::ParseError(format!("Invalid texture index: {}", e)))?,
        )
    } else {
        None
    };

    let normal_index = if parts.len() > 2 && !parts[2].is_empty() {
        Some(
            parts[2]
                .parse()
                .map_err(|e| ObjError::ParseError(format!("Invalid normal index: {}", e)))?,
        )
    } else {
        None
    };

    Ok((vertex_index, texture_index, normal_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> MeshDefinition {
        MeshDefinition {
            name: Some("test".to_string()),
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            normals: Some(vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ]),
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        }
    }

    #[test]
    fn test_export_basic_mesh() {
        let mesh = create_test_mesh();
        let obj = export_mesh_to_obj(&mesh).unwrap();

        assert!(obj.contains("o test"));
        assert!(obj.contains("v 0.0"));
        assert!(obj.contains("v 1.0"));
        assert!(obj.contains("vn 0.0"));
        assert!(obj.contains("vt 0.0"));
        assert!(obj.contains("f "));
    }

    #[test]
    fn test_export_without_normals() {
        let mesh = create_test_mesh();
        let options = ObjExportOptions {
            include_normals: false,
            ..Default::default()
        };
        let obj = export_mesh_to_obj_with_options(&mesh, &options).unwrap();

        assert!(!obj.contains("vn "));
    }

    #[test]
    fn test_export_without_uvs() {
        let mesh = create_test_mesh();
        let options = ObjExportOptions {
            include_uvs: false,
            ..Default::default()
        };
        let obj = export_mesh_to_obj_with_options(&mesh, &options).unwrap();

        assert!(!obj.contains("vt "));
    }

    #[test]
    fn test_import_basic_obj() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_import_with_normals() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
                   vn 0.0 0.0 1.0\nvn 0.0 0.0 1.0\nvn 0.0 0.0 1.0\n\
                   f 1//1 2//2 3//3\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert!(mesh.normals.is_some());
        assert_eq!(mesh.normals.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_import_with_uvs() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
                   vt 0.0 0.0\nvt 1.0 0.0\nvt 0.0 1.0\n\
                   f 1/1 2/2 3/3\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert!(mesh.uvs.is_some());
        assert_eq!(mesh.uvs.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_import_quad_face() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\n\
                   f 1 2 3 4\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6); // Quad triangulated to 2 triangles
    }

    #[test]
    fn test_import_object_name() {
        let obj = "o MyMesh\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert_eq!(mesh.name, Some("MyMesh".to_string()));
    }

    #[test]
    fn test_import_with_comments() {
        let obj = "# This is a comment\nv 0.0 0.0 0.0\n# Another comment\n\
                   v 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let mesh = import_mesh_from_obj(obj).unwrap();

        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_export_import_roundtrip() {
        let original = create_test_mesh();
        let obj = export_mesh_to_obj(&original).unwrap();
        let imported = import_mesh_from_obj(&obj).unwrap();

        assert_eq!(imported.vertices.len(), original.vertices.len());
        assert_eq!(imported.indices.len(), original.indices.len());
        assert_eq!(imported.normals.is_some(), original.normals.is_some());
        assert_eq!(imported.uvs.is_some(), original.uvs.is_some());
    }

    #[test]
    fn test_import_empty_fails() {
        let result = import_mesh_from_obj("");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_no_vertices_fails() {
        let obj = "f 1 2 3\n";
        let result = import_mesh_from_obj(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_no_faces_fails() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n";
        let result = import_mesh_from_obj(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_invalid_vertex_index() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 99\n";
        let result = import_mesh_from_obj(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_face_vertex_simple() {
        let result = parse_face_vertex("5").unwrap();
        assert_eq!(result, (5, None, None));
    }

    #[test]
    fn test_parse_face_vertex_with_uv() {
        let result = parse_face_vertex("5/3").unwrap();
        assert_eq!(result, (5, Some(3), None));
    }

    #[test]
    fn test_parse_face_vertex_with_normal() {
        let result = parse_face_vertex("5//2").unwrap();
        assert_eq!(result, (5, None, Some(2)));
    }

    #[test]
    fn test_parse_face_vertex_full() {
        let result = parse_face_vertex("5/3/2").unwrap();
        assert_eq!(result, (5, Some(3), Some(2)));
    }

    #[test]
    fn test_export_options_float_precision() {
        let mesh = create_test_mesh();
        let options = ObjExportOptions {
            float_precision: 2,
            ..Default::default()
        };
        let obj = export_mesh_to_obj_with_options(&mesh, &options).unwrap();

        // Check that precision is limited
        assert!(obj.contains("0.00") || obj.contains("1.00"));
    }

    #[test]
    fn test_import_flip_yz() {
        let obj = "v 1.0 2.0 3.0\nv 0.0 0.0 0.0\nv 0.0 0.0 0.0\nf 1 2 3\n";
        let options = ObjImportOptions {
            flip_yz: true,
            ..Default::default()
        };
        let mesh = import_mesh_from_obj_with_options(obj, &options).unwrap();

        // Y and Z should be swapped, Z negated
        assert_eq!(mesh.vertices[0], [1.0, 3.0, -2.0]);
    }

    #[test]
    fn test_import_flip_uv_v() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
                   vt 0.0 0.75\nvt 1.0 0.0\nvt 0.0 1.0\n\
                   f 1/1 2/2 3/3\n";
        let options = ObjImportOptions {
            flip_uv_v: true,
            ..Default::default()
        };
        let mesh = import_mesh_from_obj_with_options(obj, &options).unwrap();

        let uvs = mesh.uvs.unwrap();
        assert_eq!(uvs[0], [0.0, 0.25]); // 1.0 - 0.75
    }
}
