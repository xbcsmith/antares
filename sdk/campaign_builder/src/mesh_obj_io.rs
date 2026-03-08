// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OBJ mesh import/export backend for the Campaign Builder importer workflow.
//!
//! Provides functionality for importing and exporting meshes in Wavefront OBJ
//! format, a widely-used 3D model interchange format.
//!
//! The standalone `Importer` tab uses the multi-mesh import APIs in this module
//! as its parser backend. This module is intentionally focused on OBJ parsing,
//! serialization, and parser-facing options, while `obj_importer.rs` owns the
//! importer-tab state and `obj_importer_ui.rs` owns egui rendering and export
//! actions.
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
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
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

/// Options for OBJ import.
///
/// This struct is the parser-facing contract shared by direct OBJ import calls
/// and the standalone importer tab state in `obj_importer.rs`.
#[derive(Debug, Clone)]
pub struct ObjImportOptions {
    /// Name to assign to imported mesh
    pub mesh_name: Option<String>,

    /// Source OBJ path used for resolving sidecar assets such as `.mtl` files.
    pub source_path: Option<PathBuf>,

    /// Optional manual override for the material library path.
    ///
    /// When present, this path is preferred over any `mtllib` directives found
    /// inside the OBJ.
    pub manual_mtl_path: Option<PathBuf>,

    /// Default color to use if not specified
    pub default_color: [f32; 4],

    /// Whether to flip Y/Z axes (some tools use different conventions)
    pub flip_yz: bool,

    /// Whether to flip texture V coordinate
    pub flip_uv_v: bool,

    /// Uniform scale applied to imported vertex positions
    pub scale: f32,
}

impl Default for ObjImportOptions {
    fn default() -> Self {
        Self {
            mesh_name: None,
            source_path: None,
            manual_mtl_path: None,
            default_color: [1.0, 1.0, 1.0, 1.0],
            flip_yz: false,
            flip_uv_v: false,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ObjVertexRef {
    vertex_index: usize,
    texture_index: Option<usize>,
    normal_index: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ObjSegmentIdentity {
    object_name: Option<String>,
    group_name: Option<String>,
    material_name: Option<String>,
}

impl ObjSegmentIdentity {
    fn display_name(&self) -> Option<String> {
        match (&self.object_name, &self.group_name) {
            (Some(object_name), Some(group_name)) if object_name != group_name => {
                Some(format!("{}_{}", object_name, group_name))
            }
            (Some(object_name), _) => Some(object_name.clone()),
            (None, Some(group_name)) => Some(group_name.clone()),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Default)]
struct ParsedObjSegment {
    identity: ObjSegmentIdentity,
    faces: Vec<Vec<ObjVertexRef>>,
}

#[derive(Debug, Default)]
struct ParsedObjData {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    segments: Vec<ParsedObjSegment>,
    material_library_names: Vec<String>,
    material_library_paths: Vec<PathBuf>,
    materials: HashMap<String, ParsedMtlMaterial>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ParsedMtlMaterial {
    name: String,
    diffuse_color: Option<[f32; 3]>,
    specular_color: Option<[f32; 3]>,
    emissive_color: Option<[f32; 3]>,
    specular_exponent: Option<f32>,
    dissolve: Option<f32>,
    illumination_model: Option<u8>,
    diffuse_texture_path: Option<PathBuf>,
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
/// If the OBJ contains multiple object, group, or material segments, they are
/// combined into a single mesh for this API. Use
/// [`import_meshes_from_obj_with_options`] to preserve per-segment output.
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
    let parsed = parse_obj_meshes(obj_string, options)?;

    if parsed.segments.is_empty() {
        return Err(ObjError::MissingData("No faces found".to_string()));
    }

    let mesh_name = options.mesh_name.clone().or_else(|| {
        parsed
            .segments
            .iter()
            .find_map(|segment| segment.identity.display_name())
            .map(|name| sanitize_mesh_name(&name))
            .filter(|name| !name.is_empty())
    });

    build_mesh_from_faces(
        &parsed,
        parsed
            .segments
            .iter()
            .flat_map(|segment| segment.faces.iter()),
        options,
        mesh_name.unwrap_or_else(|| "mesh_0".to_string()),
    )
}

/// Imports multiple meshes from an OBJ format string.
///
/// Each object, group, or material segment is converted into a separate
/// [`MeshDefinition`] with per-mesh vertex remapping. If the OBJ does not
/// contain any naming directives, a single mesh named `mesh_0` is returned.
///
/// # Examples
///
/// ```
/// use campaign_builder::mesh_obj_io::import_meshes_from_obj;
///
/// let obj = "o Head\n\
///            v 0.0 0.0 0.0\n\
///            v 1.0 0.0 0.0\n\
///            v 0.0 1.0 0.0\n\
///            f 1 2 3\n\
///            o Arm\n\
///            v 0.0 0.0 1.0\n\
///            v 1.0 0.0 1.0\n\
///            v 0.0 1.0 1.0\n\
///            f 4 5 6\n";
///
/// let meshes = import_meshes_from_obj(obj).unwrap();
/// assert_eq!(meshes.len(), 2);
/// assert_eq!(meshes[0].name.as_deref(), Some("Head"));
/// assert_eq!(meshes[1].name.as_deref(), Some("Arm"));
/// ```
pub fn import_meshes_from_obj(obj_string: &str) -> Result<Vec<MeshDefinition>, ObjError> {
    import_meshes_from_obj_with_options(obj_string, &ObjImportOptions::default())
}

/// Imports multiple meshes from an OBJ format string with custom options.
///
/// This is the primary OBJ parsing entry point used by the standalone Importer
/// tab after `ObjImporterState` assembles parser options.
///
/// # Arguments
///
/// * `obj_string` - OBJ format string to parse
/// * `options` - Import options
///
/// # Returns
///
/// A list of imported meshes, or an error if import fails
pub fn import_meshes_from_obj_with_options(
    obj_string: &str,
    options: &ObjImportOptions,
) -> Result<Vec<MeshDefinition>, ObjError> {
    let parsed = parse_obj_meshes(obj_string, options)?;

    if parsed.segments.is_empty() {
        return Err(ObjError::MissingData("No faces found".to_string()));
    }

    let segment_names = resolve_segment_names(&parsed.segments);
    let mut meshes = Vec::with_capacity(parsed.segments.len());
    for (parsed_segment, mesh_name) in parsed.segments.iter().zip(segment_names) {
        meshes.push(build_mesh_from_faces(
            &parsed,
            parsed_segment.faces.iter(),
            options,
            mesh_name,
        )?);
    }

    Ok(meshes)
}

/// Imports multiple meshes from an OBJ file.
///
/// # Arguments
///
/// * `path` - File path to read from
///
/// # Returns
///
/// Imported meshes, or an error if import fails
pub fn import_meshes_from_obj_file(path: &str) -> Result<Vec<MeshDefinition>, ObjError> {
    import_meshes_from_obj_file_with_options(path, &ObjImportOptions::default())
}

/// Imports multiple meshes from an OBJ file with custom options.
///
/// The standalone Importer tab routes file-based OBJ loading through this API,
/// then converts the returned `MeshDefinition` values into editable importer
/// rows.
///
/// # Arguments
///
/// * `path` - File path to read from
/// * `options` - Import options
///
/// # Returns
///
/// Imported meshes, or an error if import fails
pub fn import_meshes_from_obj_file_with_options(
    path: &str,
    options: &ObjImportOptions,
) -> Result<Vec<MeshDefinition>, ObjError> {
    let obj_string = std::fs::read_to_string(path)?;
    let options = options_with_source_path(options, path);
    import_meshes_from_obj_with_options(&obj_string, &options)
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
    let options = options_with_source_path(options, path);
    import_mesh_from_obj_with_options(&obj_string, &options)
}

fn options_with_source_path(options: &ObjImportOptions, path: &str) -> ObjImportOptions {
    let mut resolved = options.clone();
    if resolved.source_path.is_none() {
        resolved.source_path = Some(PathBuf::from(path));
    }
    resolved
}

fn parse_obj_meshes(
    obj_string: &str,
    options: &ObjImportOptions,
) -> Result<ParsedObjData, ObjError> {
    let reader = BufReader::new(obj_string.as_bytes());

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut segments = Vec::new();
    let mut material_library_names = Vec::new();
    let mut current_identity = ObjSegmentIdentity::default();
    let mut current_faces: Vec<Vec<ObjVertexRef>> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "o" => {
                flush_parsed_segment(&mut segments, &current_identity, &mut current_faces);
                current_identity.object_name = parse_obj_label(parts.as_slice());
            }
            "g" => {
                flush_parsed_segment(&mut segments, &current_identity, &mut current_faces);
                current_identity.group_name = parse_obj_label(parts.as_slice());
            }
            "v" => vertices.push(parse_vertex(parts.as_slice(), options)?),
            "vn" => normals.push(parse_normal(parts.as_slice(), options)?),
            "vt" => uvs.push(parse_uv(parts.as_slice(), options)?),
            "f" => {
                if parts.len() < 4 {
                    return Err(ObjError::ParseError(format!(
                        "Invalid face definition: {}",
                        line
                    )));
                }

                let face = parts
                    .iter()
                    .skip(1)
                    .map(|part| parse_obj_face_vertex(part))
                    .collect::<Result<Vec<_>, _>>()?;

                validate_face_vertex_indices(&face, vertices.len(), uvs.len(), normals.len())?;
                current_faces.push(face);
            }
            "usemtl" => {
                flush_parsed_segment(&mut segments, &current_identity, &mut current_faces);
                current_identity.material_name = parse_obj_label(parts.as_slice());
            }
            "mtllib" => {
                material_library_names.extend(parse_mtllib_directive(parts.as_slice()));
            }
            "s" => {
                // Smoothing directives are intentionally ignored for now.
            }
            _ => {
                // Unknown directive - skip.
            }
        }
    }

    if vertices.is_empty() {
        return Err(ObjError::MissingData("No vertices found".to_string()));
    }

    flush_parsed_segment(&mut segments, &current_identity, &mut current_faces);
    let material_library_paths = resolve_material_library_paths(options, &material_library_names);
    let materials = load_resolved_material_libraries(&material_library_paths);

    Ok(ParsedObjData {
        vertices,
        normals,
        uvs,
        segments,
        material_library_names,
        material_library_paths,
        materials,
    })
}

fn parse_mtllib_directive(parts: &[&str]) -> Vec<String> {
    parts
        .iter()
        .skip(1)
        .filter_map(|name| {
            let trimmed = name.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

fn resolve_material_library_paths(
    options: &ObjImportOptions,
    material_library_names: &[String],
) -> Vec<PathBuf> {
    if let Some(manual_mtl_path) = options.manual_mtl_path.as_ref() {
        return manual_mtl_path
            .exists()
            .then(|| vec![manual_mtl_path.clone()])
            .unwrap_or_default();
    }

    let Some(source_path) = options.source_path.as_ref() else {
        return Vec::new();
    };

    let Some(source_dir) = source_path.parent() else {
        return Vec::new();
    };

    let mut resolved_paths = Vec::new();
    for material_library_name in material_library_names {
        let candidate_path = resolve_relative_path(source_dir, material_library_name);
        if candidate_path.exists() {
            resolved_paths.push(candidate_path);
        }
    }

    resolved_paths
}

fn resolve_relative_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let relative_path = Path::new(relative_path);
    if relative_path.is_absolute() {
        relative_path.to_path_buf()
    } else {
        base_dir.join(relative_path)
    }
}

fn load_resolved_material_libraries(
    material_library_paths: &[PathBuf],
) -> HashMap<String, ParsedMtlMaterial> {
    let mut materials = HashMap::new();

    for material_library_path in material_library_paths {
        let Ok(mtl_string) = std::fs::read_to_string(material_library_path) else {
            continue;
        };

        let parsed_materials = parse_mtl_materials(&mtl_string, Some(material_library_path));
        materials.extend(parsed_materials);
    }

    materials
}

fn parse_mtl_materials(
    mtl_string: &str,
    source_path: Option<&Path>,
) -> HashMap<String, ParsedMtlMaterial> {
    let reader = BufReader::new(mtl_string.as_bytes());
    let mut materials = HashMap::new();
    let mut current_material: Option<ParsedMtlMaterial> = None;

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "newmtl" => {
                if let Some(material) = current_material.take() {
                    materials.insert(material.name.clone(), material);
                }

                if let Some(name) = parse_obj_label(parts.as_slice()) {
                    current_material = Some(ParsedMtlMaterial {
                        name,
                        ..Default::default()
                    });
                }
            }
            "Kd" => {
                if let (Some(material), Some(color)) =
                    (current_material.as_mut(), parse_mtl_color(parts.as_slice()))
                {
                    material.diffuse_color = Some(color);
                }
            }
            "Ks" => {
                if let (Some(material), Some(color)) =
                    (current_material.as_mut(), parse_mtl_color(parts.as_slice()))
                {
                    material.specular_color = Some(color);
                }
            }
            "Ke" => {
                if let (Some(material), Some(color)) =
                    (current_material.as_mut(), parse_mtl_color(parts.as_slice()))
                {
                    material.emissive_color = Some(color);
                }
            }
            "Ns" => {
                if let (Some(material), Some(value)) = (
                    current_material.as_mut(),
                    parse_mtl_scalar::<f32>(parts.as_slice()),
                ) {
                    material.specular_exponent = Some(value);
                }
            }
            "d" => {
                if let (Some(material), Some(value)) = (
                    current_material.as_mut(),
                    parse_mtl_scalar::<f32>(parts.as_slice()),
                ) {
                    material.dissolve = Some(value);
                }
            }
            "illum" => {
                if let (Some(material), Some(value)) = (
                    current_material.as_mut(),
                    parse_mtl_scalar::<u8>(parts.as_slice()),
                ) {
                    material.illumination_model = Some(value);
                }
            }
            "map_Kd" => {
                if let Some(material) = current_material.as_mut() {
                    if let Some(texture_path) = parse_obj_label(parts.as_slice()) {
                        material.diffuse_texture_path =
                            resolve_texture_path(source_path, &texture_path);
                    }
                }
            }
            _ => {
                // Unsupported or malformed directives are ignored so geometry import still succeeds.
            }
        }
    }

    if let Some(material) = current_material {
        materials.insert(material.name.clone(), material);
    }

    materials
}

fn parse_mtl_color(parts: &[&str]) -> Option<[f32; 3]> {
    if parts.len() < 4 {
        return None;
    }

    Some([
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
    ])
}

fn parse_mtl_scalar<T>(parts: &[&str]) -> Option<T>
where
    T: std::str::FromStr,
{
    (parts.len() >= 2).then_some(())?;
    parts[1].parse().ok()
}

fn resolve_texture_path(source_path: Option<&Path>, texture_path: &str) -> Option<PathBuf> {
    let texture_path = Path::new(texture_path);
    if texture_path.is_absolute() {
        return Some(texture_path.to_path_buf());
    }

    let source_dir = source_path.and_then(Path::parent)?;
    Some(source_dir.join(texture_path))
}

fn flush_parsed_segment(
    segments: &mut Vec<ParsedObjSegment>,
    current_identity: &ObjSegmentIdentity,
    current_faces: &mut Vec<Vec<ObjVertexRef>>,
) {
    if current_faces.is_empty() {
        return;
    }

    segments.push(ParsedObjSegment {
        identity: current_identity.clone(),
        faces: std::mem::take(current_faces),
    });
}

fn build_mesh_from_faces<'a, I>(
    parsed: &ParsedObjData,
    faces: I,
    options: &ObjImportOptions,
    mesh_name: String,
) -> Result<MeshDefinition, ObjError>
where
    I: IntoIterator<Item = &'a Vec<ObjVertexRef>>,
{
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut local_indices = HashMap::<ObjVertexRef, u32>::new();
    let mut saw_normals = false;
    let mut saw_uvs = false;

    for face in faces {
        let mut face_indices = Vec::with_capacity(face.len());

        for vertex_ref in face {
            let local_index = if let Some(existing_index) = local_indices.get(vertex_ref) {
                *existing_index
            } else {
                let vertex = parsed.vertices[vertex_ref.vertex_index - 1];
                let next_index = vertices.len() as u32;
                vertices.push(vertex);

                if let Some(normal_index) = vertex_ref.normal_index {
                    normals.push(parsed.normals[normal_index - 1]);
                    saw_normals = true;
                } else {
                    normals.push([0.0, 0.0, 0.0]);
                }

                if let Some(texture_index) = vertex_ref.texture_index {
                    uvs.push(parsed.uvs[texture_index - 1]);
                    saw_uvs = true;
                } else {
                    uvs.push([0.0, 0.0]);
                }

                local_indices.insert(*vertex_ref, next_index);
                next_index
            };

            face_indices.push(local_index);
        }

        triangulate_face_indices(&face_indices, &mut indices);
    }

    Ok(MeshDefinition {
        name: Some(mesh_name),
        vertices,
        indices,
        normals: if saw_normals { Some(normals) } else { None },
        uvs: if saw_uvs { Some(uvs) } else { None },
        color: options.default_color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    })
}

fn resolve_segment_names(segments: &[ParsedObjSegment]) -> Vec<String> {
    let mut total_counts = HashMap::<String, usize>::new();
    let mut base_names = Vec::with_capacity(segments.len());

    for (segment_index, segment) in segments.iter().enumerate() {
        let base_name = segment
            .identity
            .display_name()
            .map(|name| sanitize_mesh_name(&name))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("mesh_{}", segment_index));
        *total_counts.entry(base_name.clone()).or_insert(0) += 1;
        base_names.push(base_name);
    }

    let mut seen_counts = HashMap::<String, usize>::new();
    base_names
        .into_iter()
        .map(|base_name| {
            let occurrence_index = seen_counts.entry(base_name.clone()).or_insert(0);
            let resolved_name = if total_counts.get(&base_name).copied().unwrap_or_default() > 1
                && *occurrence_index > 0
            {
                format!("{}_segment_{}", base_name, *occurrence_index)
            } else {
                base_name
            };
            *occurrence_index += 1;
            resolved_name
        })
        .collect()
}

fn parse_obj_label(parts: &[&str]) -> Option<String> {
    (parts.len() > 1)
        .then(|| parts[1..].join(" "))
        .filter(|label| !label.trim().is_empty())
}

fn triangulate_face_indices(face_indices: &[u32], indices: &mut Vec<u32>) {
    if face_indices.len() == 3 {
        indices.extend_from_slice(face_indices);
        return;
    }

    for triangle_index in 1..(face_indices.len() - 1) {
        indices.push(face_indices[0]);
        indices.push(face_indices[triangle_index]);
        indices.push(face_indices[triangle_index + 1]);
    }
}

fn parse_vertex(parts: &[&str], options: &ObjImportOptions) -> Result<[f32; 3], ObjError> {
    if parts.len() < 4 {
        return Err(ObjError::ParseError(format!(
            "Invalid vertex definition: {}",
            parts.join(" ")
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

    let x = x * options.scale;
    let y = y * options.scale;
    let z = z * options.scale;

    Ok(if options.flip_yz {
        [x, z, -y]
    } else {
        [x, y, z]
    })
}

fn parse_normal(parts: &[&str], options: &ObjImportOptions) -> Result<[f32; 3], ObjError> {
    if parts.len() < 4 {
        return Err(ObjError::ParseError(format!(
            "Invalid normal definition: {}",
            parts.join(" ")
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

    Ok(if options.flip_yz {
        [x, z, -y]
    } else {
        [x, y, z]
    })
}

fn parse_uv(parts: &[&str], options: &ObjImportOptions) -> Result<[f32; 2], ObjError> {
    if parts.len() < 3 {
        return Err(ObjError::ParseError(format!(
            "Invalid texture coordinate definition: {}",
            parts.join(" ")
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

    Ok([u, v])
}

fn parse_obj_face_vertex(s: &str) -> Result<ObjVertexRef, ObjError> {
    let (vertex_index, texture_index, normal_index) = parse_face_vertex(s)?;
    Ok(ObjVertexRef {
        vertex_index,
        texture_index,
        normal_index,
    })
}

fn validate_face_vertex_indices(
    face: &[ObjVertexRef],
    vertex_count: usize,
    uv_count: usize,
    normal_count: usize,
) -> Result<(), ObjError> {
    for vertex_ref in face {
        if vertex_ref.vertex_index == 0 || vertex_ref.vertex_index > vertex_count {
            return Err(ObjError::InvalidIndex(format!(
                "Vertex index {} out of range (1-{})",
                vertex_ref.vertex_index, vertex_count
            )));
        }

        if let Some(texture_index) = vertex_ref.texture_index {
            if texture_index == 0 || texture_index > uv_count {
                return Err(ObjError::InvalidIndex(format!(
                    "Texture index {} out of range (1-{})",
                    texture_index, uv_count
                )));
            }
        }

        if let Some(normal_index) = vertex_ref.normal_index {
            if normal_index == 0 || normal_index > normal_count {
                return Err(ObjError::InvalidIndex(format!(
                    "Normal index {} out of range (1-{})",
                    normal_index, normal_count
                )));
            }
        }
    }

    Ok(())
}

fn sanitize_mesh_name(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    let mut last_was_underscore = false;

    for ch in raw.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else {
            '_'
        };

        if mapped == '_' {
            if last_was_underscore {
                continue;
            }
            last_was_underscore = true;
        } else {
            last_was_underscore = false;
        }

        sanitized.push(mapped);
    }

    sanitized.trim_matches('_').to_string()
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
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    #[test]
    fn test_import_multi_mesh_without_groups_uses_mesh_0() {
        let obj = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let meshes = import_meshes_from_obj(obj).unwrap();

        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].name.as_deref(), Some("mesh_0"));
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_parse_obj_meshes_preserves_object_group_and_material_segments() {
        let obj = "o Body\n\
                   g Torso\n\
                   usemtl Skin\n\
                   v 0.0 0.0 0.0\n\
                   v 1.0 0.0 0.0\n\
                   v 0.0 1.0 0.0\n\
                   v 1.0 1.0 0.0\n\
                   f 1 2 3\n\
                   usemtl Armor\n\
                   f 2 4 3\n\
                   g Cape\n\
                   usemtl Cloth\n\
                   f 1 3 4\n";

        let parsed = parse_obj_meshes(obj, &ObjImportOptions::default()).unwrap();

        assert_eq!(parsed.segments.len(), 3);

        assert_eq!(
            parsed.segments[0].identity.object_name.as_deref(),
            Some("Body")
        );
        assert_eq!(
            parsed.segments[0].identity.group_name.as_deref(),
            Some("Torso")
        );
        assert_eq!(
            parsed.segments[0].identity.material_name.as_deref(),
            Some("Skin")
        );

        assert_eq!(
            parsed.segments[1].identity.object_name.as_deref(),
            Some("Body")
        );
        assert_eq!(
            parsed.segments[1].identity.group_name.as_deref(),
            Some("Torso")
        );
        assert_eq!(
            parsed.segments[1].identity.material_name.as_deref(),
            Some("Armor")
        );

        assert_eq!(
            parsed.segments[2].identity.object_name.as_deref(),
            Some("Body")
        );
        assert_eq!(
            parsed.segments[2].identity.group_name.as_deref(),
            Some("Cape")
        );
        assert_eq!(
            parsed.segments[2].identity.material_name.as_deref(),
            Some("Cloth")
        );
    }

    #[test]
    fn test_import_multi_mesh_splits_material_boundaries_without_losing_identity() {
        let obj = "o Body\n\
                   g Torso\n\
                   usemtl Skin\n\
                   v 0.0 0.0 0.0\n\
                   v 1.0 0.0 0.0\n\
                   v 0.0 1.0 0.0\n\
                   v 1.0 1.0 0.0\n\
                   f 1 2 3\n\
                   usemtl Armor\n\
                   f 2 4 3\n";

        let meshes = import_meshes_from_obj(obj).unwrap();

        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].name.as_deref(), Some("Body_Torso"));
        assert_eq!(meshes[1].name.as_deref(), Some("Body_Torso_segment_1"));
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
        assert_eq!(meshes[1].indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_import_single_mesh_combines_material_segments() {
        let obj = "o Body\n\
                   g Torso\n\
                   usemtl Skin\n\
                   v 0.0 0.0 0.0\n\
                   v 1.0 0.0 0.0\n\
                   v 0.0 1.0 0.0\n\
                   v 1.0 1.0 0.0\n\
                   f 1 2 3\n\
                   usemtl Armor\n\
                   f 2 4 3\n";

        let mesh = import_mesh_from_obj(obj).unwrap();

        assert_eq!(mesh.name.as_deref(), Some("Body_Torso"));
        assert_eq!(mesh.indices.len(), 6);
    }

    #[test]
    fn test_parse_obj_meshes_resolves_relative_mtllib_and_parses_material_fields() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let mtl_path = temp_dir.path().join("models/materials/hero.mtl");
        fs::create_dir_all(mtl_path.parent().unwrap()).unwrap();
        fs::write(
            &mtl_path,
            concat!(
                "newmtl HeroSkin\n",
                "Kd 0.1 0.2 0.3\n",
                "Ks 0.4 0.5 0.6\n",
                "Ke 0.7 0.8 0.9\n",
                "Ns 128.0\n",
                "d 0.75\n",
                "illum 2\n",
                "map_Kd textures/hero.png\n"
            ),
        )
        .unwrap();

        let obj = concat!(
            "mtllib materials/hero.mtl\n",
            "o Body\n",
            "usemtl HeroSkin\n",
            "v 0.0 0.0 0.0\n",
            "v 1.0 0.0 0.0\n",
            "v 0.0 1.0 0.0\n",
            "f 1 2 3\n"
        );
        let options = ObjImportOptions {
            source_path: Some(obj_path.clone()),
            ..Default::default()
        };

        let parsed = parse_obj_meshes(obj, &options).unwrap();

        assert_eq!(parsed.material_library_names, vec!["materials/hero.mtl"]);
        assert_eq!(parsed.material_library_paths, vec![mtl_path.clone()]);

        let material = parsed.materials.get("HeroSkin").unwrap();
        assert_eq!(material.diffuse_color, Some([0.1, 0.2, 0.3]));
        assert_eq!(material.specular_color, Some([0.4, 0.5, 0.6]));
        assert_eq!(material.emissive_color, Some([0.7, 0.8, 0.9]));
        assert_eq!(material.specular_exponent, Some(128.0));
        assert_eq!(material.dissolve, Some(0.75));
        assert_eq!(material.illumination_model, Some(2));
        assert_eq!(
            material.diffuse_texture_path,
            Some(mtl_path.parent().unwrap().join("textures/hero.png"))
        );
    }

    #[test]
    fn test_parse_obj_meshes_loads_multiple_mtllib_directives() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let base_mtl_path = temp_dir.path().join("models/base.mtl");
        let accent_mtl_path = temp_dir.path().join("models/accent.mtl");
        fs::create_dir_all(base_mtl_path.parent().unwrap()).unwrap();
        fs::write(&base_mtl_path, "newmtl Base\nKd 0.1 0.1 0.1\n").unwrap();
        fs::write(&accent_mtl_path, "newmtl Accent\nKd 0.9 0.2 0.2\n").unwrap();

        let obj = concat!(
            "mtllib base.mtl\n",
            "mtllib accent.mtl\n",
            "o Body\n",
            "usemtl Accent\n",
            "v 0.0 0.0 0.0\n",
            "v 1.0 0.0 0.0\n",
            "v 0.0 1.0 0.0\n",
            "f 1 2 3\n"
        );
        let options = ObjImportOptions {
            source_path: Some(obj_path),
            ..Default::default()
        };

        let parsed = parse_obj_meshes(obj, &options).unwrap();

        assert_eq!(
            parsed.material_library_paths,
            vec![base_mtl_path, accent_mtl_path]
        );
        assert!(parsed.materials.contains_key("Base"));
        assert!(parsed.materials.contains_key("Accent"));
    }

    #[test]
    fn test_parse_obj_meshes_prefers_manual_mtl_override() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let auto_mtl_path = temp_dir.path().join("models/auto.mtl");
        let override_mtl_path = temp_dir.path().join("overrides/manual.mtl");
        fs::create_dir_all(auto_mtl_path.parent().unwrap()).unwrap();
        fs::create_dir_all(override_mtl_path.parent().unwrap()).unwrap();
        fs::write(&auto_mtl_path, "newmtl Auto\nKd 0.1 0.1 0.1\n").unwrap();
        fs::write(&override_mtl_path, "newmtl Override\nKd 0.8 0.7 0.6\n").unwrap();

        let obj = concat!(
            "mtllib auto.mtl\n",
            "o Body\n",
            "usemtl Override\n",
            "v 0.0 0.0 0.0\n",
            "v 1.0 0.0 0.0\n",
            "v 0.0 1.0 0.0\n",
            "f 1 2 3\n"
        );
        let options = ObjImportOptions {
            source_path: Some(obj_path),
            manual_mtl_path: Some(override_mtl_path.clone()),
            ..Default::default()
        };

        let parsed = parse_obj_meshes(obj, &options).unwrap();

        assert_eq!(parsed.material_library_paths, vec![override_mtl_path]);
        assert!(!parsed.materials.contains_key("Auto"));
        assert!(parsed.materials.contains_key("Override"));
    }

    #[test]
    fn test_import_meshes_from_obj_file_missing_mtl_is_non_fatal() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("missing_material.obj");
        fs::write(
            &obj_path,
            concat!(
                "mtllib missing.mtl\n",
                "o Body\n",
                "usemtl Missing\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let meshes = import_meshes_from_obj_file_with_options(
            obj_path.to_str().unwrap(),
            &ObjImportOptions::default(),
        )
        .unwrap();

        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].name.as_deref(), Some("Body"));
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_parse_obj_meshes_tolerates_malformed_mtl_data() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let mtl_path = temp_dir.path().join("models/broken.mtl");
        fs::create_dir_all(mtl_path.parent().unwrap()).unwrap();
        fs::write(
            &mtl_path,
            concat!(
                "newmtl Broken\n",
                "Kd nope nope nope\n",
                "Ns ???\n",
                "d 0.5\n",
                "illum 1\n"
            ),
        )
        .unwrap();

        let obj = concat!(
            "mtllib broken.mtl\n",
            "o Body\n",
            "usemtl Broken\n",
            "v 0.0 0.0 0.0\n",
            "v 1.0 0.0 0.0\n",
            "v 0.0 1.0 0.0\n",
            "f 1 2 3\n"
        );
        let options = ObjImportOptions {
            source_path: Some(obj_path),
            ..Default::default()
        };

        let parsed = parse_obj_meshes(obj, &options).unwrap();
        let material = parsed.materials.get("Broken").unwrap();

        assert_eq!(material.diffuse_color, None);
        assert_eq!(material.specular_exponent, None);
        assert_eq!(material.dissolve, Some(0.5));
        assert_eq!(material.illumination_model, Some(1));

        let meshes = import_meshes_from_obj_with_options(obj, &options).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_sanitize_mesh_name_edge_cases() {
        assert_eq!(sanitize_mesh_name(""), "");
        assert_eq!(sanitize_mesh_name("***"), "");
        assert_eq!(sanitize_mesh_name(" Head / Torso !! "), "Head_Torso");
        assert_eq!(sanitize_mesh_name("___left__arm___"), "left_arm");
    }

    #[test]
    fn test_import_multi_mesh_scale_option_applies_to_vertices() {
        let obj = "o Scaled\nv 2.0 4.0 6.0\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nf 1 2 3\n";
        let options = ObjImportOptions {
            scale: 0.5,
            ..Default::default()
        };

        let meshes = import_meshes_from_obj_with_options(obj, &options).unwrap();
        assert_eq!(meshes[0].vertices[0], [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_import_meshes_from_obj_file_skeleton_fixture() {
        let path = fixture_path("examples/skeleton.obj");
        let obj_string = std::fs::read_to_string(&path).unwrap();
        let expected_meshes = obj_string
            .lines()
            .filter(|line| line.trim_start().starts_with("o "))
            .count();

        let meshes = import_meshes_from_obj_file(path.to_str().unwrap()).unwrap();

        assert_eq!(meshes.len(), expected_meshes);
        for mesh in meshes {
            assert!(mesh.name.as_deref().is_some_and(|name| !name.is_empty()));
            assert!(!mesh.vertices.is_empty());
            assert_eq!(mesh.indices.len() % 3, 0);
        }
    }

    #[test]
    fn test_import_meshes_from_obj_file_female_fixture() {
        let path = fixture_path("examples/female_1.obj");
        let obj_string = std::fs::read_to_string(&path).unwrap();
        let expected_meshes = obj_string
            .lines()
            .filter(|line| line.trim_start().starts_with("o "))
            .count();

        let meshes = import_meshes_from_obj_file(path.to_str().unwrap()).unwrap();

        assert_eq!(meshes.len(), expected_meshes);
        for mesh in meshes {
            assert!(mesh.name.as_deref().is_some_and(|name| !name.is_empty()));
            assert!(!mesh.vertices.is_empty());
            assert_eq!(mesh.indices.len() % 3, 0);
        }
    }

    fn fixture_path(relative_path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(relative_path)
    }
}
