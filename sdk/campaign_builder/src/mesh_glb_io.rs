// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! GLB (binary glTF) mesh import backend for the Campaign Builder importer workflow.
//!
//! Provides functionality for importing meshes from binary glTF (`.glb`) files into
//! Antares [`MeshDefinition`] values. This module is intentionally parser-only with no
//! UI dependencies, mirroring the structure of `mesh_obj_io.rs`.
//!
//! The `mesh_glb_io` module is a pure parser backend. Importer-tab state lives in
//! `obj_importer.rs` and egui rendering lives in `obj_importer_ui.rs`. This module
//! converts GLB data into [`ImportedGlbScene`] which the importer state consumes.
//!
//! # Supported Features
//!
//! - `TRIANGLES` primitive mode.
//! - `POSITION`, `NORMAL`, and `TEXCOORD_0` vertex attributes.
//! - Index buffers; sequential indices are generated for unindexed primitives.
//! - PBR metallic-roughness materials (base color, metallic, roughness, emissive, alpha mode).
//! - Embedded base-color textures (PNG, JPEG) extracted as raw encoded bytes from GLB buffer views.
//! - Per-primitive scale and UV-V-flip import options.
//!
//! # Unsupported Features
//!
//! The following return errors rather than being silently ignored:
//!
//! - External URI textures or buffers — all data must be embedded in the GLB binary chunk.
//! - `TRIANGLE_STRIP` and `TRIANGLE_FAN` primitives — triangulation is not implemented;
//!   callers should pre-process meshes into `TRIANGLES` before importing.
//! - `LINES`, `POINTS`, `LINE_STRIP`, `LINE_LOOP` — non-triangle primitives.
//!
//! # Node Transforms
//!
//! Node world transforms are **not** flattened into vertex positions. Each
//! [`ImportedGlbMesh`] preserves the node's local transform matrix in
//! `node_transform: Option<[[f32; 4]; 4]>` for future use by the export pipeline.
//! The existing `CreatureDefinition.mesh_transforms` mechanism applies transforms;
//! the parser must not duplicate that logic.
//!
//! # Texture Placeholder
//!
//! When a primitive has a base-color texture, [`MeshDefinition::texture_path`] is set
//! to the placeholder string `"__glb_texture_{mesh_doc_index}_{prim_index}"`.  The
//! importer export step (Phase 4) rewrites this placeholder to the final
//! campaign-relative path after copying the embedded image bytes into
//! `assets/textures/imported/`.

use antares::domain::visual::{AlphaMode, MaterialDefinition, MeshDefinition};
use std::path::{Path, PathBuf};
use thiserror::Error;

// ─── Public-visible option and error types ───────────────────────────────────

/// Import options controlling GLB parsing behaviour.
///
/// Construct with [`GlbImportOptions::default`] and override individual fields as
/// needed before passing to [`import_glb_scene_from_file`] or
/// [`import_glb_scene_from_bytes`].
#[derive(Debug, Clone)]
pub struct GlbImportOptions {
    /// Optional path to the source `.glb` file.
    ///
    /// Used for error messages and future relative-path resolution.  Not required
    /// for [`import_glb_scene_from_bytes`]; set automatically by
    /// [`import_glb_scene_from_file`].
    pub source_path: Option<PathBuf>,

    /// Uniform scale factor applied to every vertex position.
    ///
    /// Defaults to `1.0` (no scaling).  A value of `0.01` converts centimetre-space
    /// models into metre-space.
    pub scale: f32,

    /// When `true`, inverts the V component of every UV coordinate: `v = 1.0 - v`.
    ///
    /// Required when the exporter uses a top-left UV origin (DirectX convention)
    /// while the runtime expects a bottom-left origin (OpenGL convention).
    pub flip_uv_v: bool,

    /// Reserved: index of the scene to import when the document has multiple scenes.
    ///
    /// Currently unused.  The parser always imports
    /// [`gltf::Document::default_scene`] when present, or the first scene
    /// otherwise.  Multi-scene selection is deferred to a future phase.
    pub scene_index: usize,

    /// Fallback base color used when a primitive has no material.
    ///
    /// Defaults to opaque white `[1.0, 1.0, 1.0, 1.0]`.
    pub default_color: [f32; 4],
}

impl Default for GlbImportOptions {
    fn default() -> Self {
        Self {
            source_path: None,
            scale: 1.0,
            flip_uv_v: false,
            scene_index: 0,
            default_color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Errors that can occur during GLB import.
#[derive(Error, Debug)]
pub enum GlbImportError {
    /// IO error reading the `.glb` file from disk.
    #[error("IO error reading GLB file: {0}")]
    Io(#[from] std::io::Error),

    /// The glTF document failed to parse or an internal glTF operation failed.
    #[error("glTF error: {0}")]
    GltfError(String),

    /// A primitive uses a mode other than `TRIANGLES`.
    ///
    /// `mode` is the human-readable name of the unsupported mode, e.g. `"Lines"`.
    #[error("Unsupported primitive mode: {mode}")]
    UnsupportedPrimitive { mode: String },

    /// The required `POSITION` attribute is absent from a primitive.
    ///
    /// `mesh` is the glTF mesh name and `primitive` is the zero-based primitive
    /// index within that mesh.
    #[error("Missing POSITION attribute in mesh '{mesh}', primitive {primitive}")]
    MissingPositions { mesh: String, primitive: usize },

    /// A required buffer or image buffer view is missing or inaccessible.
    ///
    /// This error is returned for external URI textures or buffers, which are not
    /// supported — all data must be embedded in the GLB binary chunk.
    #[error("Missing buffer data: {detail}")]
    MissingBuffer { detail: String },

    /// The GLB document contains no scenes.
    #[error("GLB file contains no scenes")]
    EmptyScene,

    /// An accessor index is out of range.
    #[error("Invalid index in accessor")]
    InvalidIndex,
}

// ─── Result types ─────────────────────────────────────────────────────────────

/// Top-level result of a GLB import operation.
///
/// Contains all mesh rows produced from the imported scene plus metadata about
/// the source document's structure.
#[derive(Debug, Clone)]
pub struct ImportedGlbScene {
    /// One entry per `(node, mesh, primitive)` triple found in the selected scene.
    pub meshes: Vec<ImportedGlbMesh>,

    /// Number of images embedded in the GLB document.
    pub embedded_image_count: usize,

    /// Number of materials defined in the GLB document.
    pub material_count: usize,

    /// Number of scenes in the GLB document.
    pub scene_count: usize,

    /// `true` if the document contains any skin (skeletal animation rig) definitions.
    pub has_skinning: bool,

    /// `true` if the document contains any animation clip definitions.
    pub has_animations: bool,

    /// `true` if any material in the document uses PBR texture channels that
    /// Antares does not support: `normalTexture`, `occlusionTexture`, or
    /// `pbrMetallicRoughness.metallicRoughnessTexture`.
    ///
    /// These channels are detected but **not** imported — their texture data is
    /// silently ignored.  The importer UI surfaces this flag in the metadata
    /// status message so users know what was skipped.
    pub has_unsupported_pbr_channels: bool,
}

/// A single imported mesh row, corresponding to one glTF primitive.
///
/// Each primitive in the imported scene becomes one `ImportedGlbMesh` entry in
/// [`ImportedGlbScene::meshes`].
#[derive(Debug, Clone)]
pub struct ImportedGlbMesh {
    /// Converted mesh geometry and material ready for Antares use.
    pub mesh_def: MeshDefinition,

    /// Embedded base-color texture payload, or `None` when the primitive has no
    /// base-color texture.
    pub texture_payload: Option<ImportedGlbTexturePayload>,

    /// Name of the glTF node that owns this mesh, if present.
    pub node_name: Option<String>,

    /// Name of the glTF material applied to this primitive, if present.
    pub material_name: Option<String>,

    /// The node's local transform matrix as `[[f32; 4]; 4]`, preserved for future
    /// use by the export pipeline.  The parser does **not** flatten this transform
    /// into vertex positions.
    pub node_transform: Option<[[f32; 4]; 4]>,
}

/// Embedded base-color texture data extracted from a GLB buffer view.
///
/// Contains the raw encoded image bytes (PNG, JPEG, etc.) and metadata needed
/// to write the image to disk during the export step (Phase 4).
#[derive(Debug, Clone)]
pub struct ImportedGlbTexturePayload {
    /// Human-readable label: the glTF image `name` field when present, otherwise
    /// `"image_{index}"`.
    pub source_label: String,

    /// Sanitized export filename including extension (e.g., `"albedo_0.png"`).
    ///
    /// Derived from [`source_label`][Self::source_label] and
    /// [`mime_type`][Self::mime_type] by [`sanitize_glb_file_name`].
    pub file_name_hint: String,

    /// Raw encoded image bytes extracted from the GLB buffer view.
    ///
    /// These are the original PNG/JPEG encoded bytes, **not** decoded pixel data.
    /// The export step writes these bytes directly to disk.
    pub bytes: Vec<u8>,

    /// MIME type from the glTF image metadata, e.g. `"image/png"` or
    /// `"image/jpeg"`.
    pub mime_type: Option<String>,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read a `.glb` file from disk and parse it into an [`ImportedGlbScene`].
///
/// This is a thin wrapper around [`import_glb_scene_from_bytes`] that reads the
/// file and sets [`GlbImportOptions::source_path`] when it is not already set.
/// Prefer [`import_glb_scene_from_bytes`] in unit tests to avoid filesystem
/// dependencies.
///
/// # Errors
///
/// Returns [`GlbImportError::Io`] when the file cannot be read, plus any error
/// that [`import_glb_scene_from_bytes`] can produce.
///
/// # Example
///
/// ```no_run
/// use campaign_builder::mesh_glb_io::{import_glb_scene_from_file, GlbImportError};
/// use std::path::Path;
///
/// // GlbImportOptions is pub(crate); this example is illustrative only.
/// // See import_glb_scene_from_bytes for the testable entry point.
/// match import_glb_scene_from_file(Path::new("model.glb"), &Default::default()) {
///     Ok(scene) => println!("imported {} meshes", scene.meshes.len()),
///     Err(GlbImportError::EmptyScene) => eprintln!("GLB has no scenes"),
///     Err(e) => eprintln!("import failed: {e}"),
/// }
/// ```
pub fn import_glb_scene_from_file(
    path: &Path,
    options: &GlbImportOptions,
) -> Result<ImportedGlbScene, GlbImportError> {
    let bytes = std::fs::read(path)?;
    let mut opts = options.clone();
    if opts.source_path.is_none() {
        opts.source_path = Some(path.to_path_buf());
    }
    import_glb_scene_from_bytes(&bytes, &opts)
}

/// Parse raw GLB bytes into an [`ImportedGlbScene`].
///
/// This is the main testable entry point for the GLB parser.  All unit tests
/// call this function with in-memory GLB data built by the test helpers.
///
/// # Algorithm
///
/// 1. Parse the document with [`gltf::Gltf::from_slice`].
/// 2. Pre-validate: reject external URI buffers and images (all data must be
///    embedded).
/// 3. Select the scene: `default_scene()` if present, else the first scene.
///    Return [`GlbImportError::EmptyScene`] when no scenes exist.
/// 4. Traverse scene nodes depth-first and convert each primitive.
///
/// # Errors
///
/// - [`GlbImportError::GltfError`] — malformed glTF document.
/// - [`GlbImportError::MissingBuffer`] — external URI texture or buffer detected.
/// - [`GlbImportError::EmptyScene`] — document has no scenes.
/// - [`GlbImportError::MissingPositions`] — a primitive lacks a `POSITION` attribute.
/// - [`GlbImportError::UnsupportedPrimitive`] — a primitive uses a non-triangle mode.
pub(crate) fn import_glb_scene_from_bytes(
    bytes: &[u8],
    options: &GlbImportOptions,
) -> Result<ImportedGlbScene, GlbImportError> {
    // ── Step 1: parse document (no external resource loading) ─────────────
    let gltf =
        gltf::Gltf::from_slice(bytes).map_err(|e| GlbImportError::GltfError(e.to_string()))?;

    // ── Step 2: pre-validate — reject external URI buffers ────────────────
    for buffer in gltf.document.buffers() {
        if let gltf::buffer::Source::Uri(uri) = buffer.source() {
            if !uri.starts_with("data:") {
                return Err(GlbImportError::MissingBuffer {
                    detail: format!(
                        "external URI buffers are not supported; the GLB must be \
                         self-contained (found URI: {uri})"
                    ),
                });
            }
        }
    }

    // ── Step 2b: pre-validate — reject external URI images ────────────────
    for image in gltf.document.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            if !uri.starts_with("data:") {
                return Err(GlbImportError::MissingBuffer {
                    detail: format!(
                        "external URI textures are not supported; embed textures in \
                         the GLB file (found URI: {uri})"
                    ),
                });
            }
        }
    }

    // ── Step 3: blob is buffer-0 data for GLB files ───────────────────────
    // blob has the same lifetime as gltf.document, so &'gltf [u8] satisfies
    // the reader's Fn(Buffer<'a>) -> Option<&'a [u8]> constraint.
    let blob: &[u8] = gltf.blob.as_deref().unwrap_or(&[]);

    // ── Step 4: select scene ──────────────────────────────────────────────
    let scene = gltf
        .document
        .default_scene()
        .or_else(|| gltf.document.scenes().next())
        .ok_or(GlbImportError::EmptyScene)?;

    // ── Document-level metadata ───────────────────────────────────────────
    let scene_count = gltf.document.scenes().count();
    let material_count = gltf.document.materials().count();
    let embedded_image_count = gltf.document.images().count();
    let has_skinning = gltf.document.skins().count() > 0;
    let has_animations = gltf.document.animations().count() > 0;
    // Detect unsupported PBR texture channels across all materials.
    // normalTexture, occlusionTexture, and metallicRoughnessTexture are not
    // mapped to Antares domain fields; set a flag so the UI can warn the user.
    let has_unsupported_pbr_channels = gltf.document.materials().any(|mat| {
        let pbr = mat.pbr_metallic_roughness();
        mat.normal_texture().is_some()
            || mat.occlusion_texture().is_some()
            || pbr.metallic_roughness_texture().is_some()
    });

    // ── Step 5: depth-first node traversal ───────────────────────────────
    let mut imported_meshes: Vec<ImportedGlbMesh> = Vec::new();
    let mut node_stack: Vec<gltf::Node<'_>> = scene.nodes().collect();

    while let Some(node) = node_stack.pop() {
        // Push children for subsequent processing (depth-first order).
        for child in node.children() {
            node_stack.push(child);
        }

        let node_name = node.name().map(str::to_string);
        let node_transform = Some(node.transform().matrix());

        if let Some(mesh) = node.mesh() {
            let mesh_doc_index = mesh.index();
            let mesh_name = mesh.name().unwrap_or("unnamed");

            for (prim_index, primitive) in mesh.primitives().enumerate() {
                let material_name = primitive.material().name().map(str::to_string);

                let mesh_def = glb_primitive_to_mesh_definition(
                    &primitive,
                    blob,
                    options,
                    mesh_name,
                    mesh_doc_index,
                    prim_index,
                )?;

                let texture_payload = extract_base_color_texture_payload(&primitive, blob)?;

                imported_meshes.push(ImportedGlbMesh {
                    mesh_def,
                    texture_payload,
                    node_name: node_name.clone(),
                    material_name,
                    node_transform,
                });
            }
        }
    }

    Ok(ImportedGlbScene {
        meshes: imported_meshes,
        embedded_image_count,
        material_count,
        scene_count,
        has_skinning,
        has_animations,
        has_unsupported_pbr_channels,
    })
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Convert a single glTF primitive into a [`MeshDefinition`].
///
/// The `blob` slice must be buffer-0 data from the GLB binary chunk.  Its
/// lifetime `'doc` is tied to the enclosing [`gltf::Gltf`] so the primitive
/// reader's lifetime constraint (`Fn(Buffer<'doc>) -> Option<&'doc [u8]>`) is
/// satisfied without unsafe code.
///
/// # Errors
///
/// - [`GlbImportError::UnsupportedPrimitive`] for non-triangle modes.
/// - [`GlbImportError::MissingPositions`] when `POSITION` is absent.
fn glb_primitive_to_mesh_definition<'doc>(
    primitive: &gltf::Primitive<'doc>,
    blob: &'doc [u8],
    options: &GlbImportOptions,
    mesh_name: &str,
    mesh_doc_index: usize,
    prim_index: usize,
) -> Result<MeshDefinition, GlbImportError> {
    // ── Validate primitive mode ───────────────────────────────────────────
    let mode_name = match primitive.mode() {
        gltf::mesh::Mode::Triangles => None, // supported
        gltf::mesh::Mode::TriangleStrip => Some("TriangleStrip"),
        gltf::mesh::Mode::TriangleFan => Some("TriangleFan"),
        gltf::mesh::Mode::Points => Some("Points"),
        gltf::mesh::Mode::Lines => Some("Lines"),
        gltf::mesh::Mode::LineLoop => Some("LineLoop"),
        gltf::mesh::Mode::LineStrip => Some("LineStrip"),
    };
    if let Some(name) = mode_name {
        return Err(GlbImportError::UnsupportedPrimitive {
            mode: name.to_string(),
        });
    }

    // ── Reader — blob is buffer-0; returns None when blob is empty (no BIN chunk) ─
    let reader = primitive.reader(|buf| {
        if buf.index() == 0 && !blob.is_empty() {
            Some(blob)
        } else {
            None
        }
    });

    // ── Vertex positions (required) ───────────────────────────────────────
    let vertices: Vec<[f32; 3]> = reader
        .read_positions()
        .ok_or_else(|| GlbImportError::MissingPositions {
            mesh: mesh_name.to_string(),
            primitive: prim_index,
        })?
        .map(|[x, y, z]| [x * options.scale, y * options.scale, z * options.scale])
        .collect();

    // ── Indices (generate sequential when absent) ─────────────────────────
    let indices: Vec<u32> = if let Some(idx_reader) = reader.read_indices() {
        idx_reader.into_u32().collect()
    } else {
        (0u32..vertices.len() as u32).collect()
    };

    // ── Normals (optional) ────────────────────────────────────────────────
    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|iter| iter.collect());

    // ── UV coordinates (optional, with optional V-flip) ───────────────────
    let uvs: Option<Vec<[f32; 2]>> = reader.read_tex_coords(0).map(|tc| {
        let raw: Vec<[f32; 2]> = tc.into_f32().collect();
        if options.flip_uv_v {
            raw.into_iter().map(|[u, v]| [u, 1.0 - v]).collect()
        } else {
            raw
        }
    });

    // ── Material ──────────────────────────────────────────────────────────
    let material_ref = primitive.material();
    let pbr = material_ref.pbr_metallic_roughness();
    let base_color = pbr.base_color_factor();

    // Texture placeholder: rewritten to campaign-relative path in Phase 4.
    let texture_path: Option<String> = if pbr.base_color_texture().is_some() {
        Some(format!("__glb_texture_{mesh_doc_index}_{prim_index}"))
    } else {
        None
    };

    Ok(MeshDefinition {
        name: Some(format!("{mesh_name}_{prim_index}")),
        vertices,
        indices,
        normals,
        uvs,
        color: base_color,
        lod_levels: None,
        lod_distances: None,
        material: Some(convert_gltf_material(&material_ref)),
        texture_path,
    })
}

/// Extract the base-color texture payload from a primitive's material.
///
/// Returns `None` when the primitive has no material or the material has no
/// base-color texture.  Returns an error for external URI image sources (which
/// should already have been rejected by the pre-validation in
/// [`import_glb_scene_from_bytes`], but are handled defensively here too).
///
/// Image bytes are read directly from the GLB `blob` buffer view, giving the
/// original encoded PNG/JPEG bytes rather than decoded pixel data.
fn extract_base_color_texture_payload<'doc>(
    primitive: &gltf::Primitive<'doc>,
    blob: &'doc [u8],
) -> Result<Option<ImportedGlbTexturePayload>, GlbImportError> {
    let pbr = primitive.material().pbr_metallic_roughness();

    let texture_info = match pbr.base_color_texture() {
        Some(info) => info,
        None => return Ok(None),
    };

    // Traverse: texture info → texture → source image
    let image_gltf = texture_info.texture().source();
    let image_index = image_gltf.index();
    let source_label = image_gltf
        .name()
        .map(str::to_string)
        .unwrap_or_else(|| format!("image_{image_index}"));

    match image_gltf.source() {
        gltf::image::Source::View { view, mime_type } => {
            // Sanity-check: we only support buffer 0 (the GLB blob).
            if view.buffer().index() != 0 {
                return Err(GlbImportError::MissingBuffer {
                    detail: format!(
                        "image {image_index} references buffer {} but only buffer 0 \
                         (GLB blob) is supported",
                        view.buffer().index()
                    ),
                });
            }

            let start = view.offset();
            let end = start + view.length();
            let raw_bytes = blob
                .get(start..end)
                .ok_or_else(|| GlbImportError::MissingBuffer {
                    detail: format!(
                        "image {image_index} buffer view [{start}..{end}] is out of \
                         bounds (blob length {})",
                        blob.len()
                    ),
                })?;

            let file_name_hint = sanitize_glb_file_name(&source_label, Some(mime_type));

            Ok(Some(ImportedGlbTexturePayload {
                source_label,
                file_name_hint,
                bytes: raw_bytes.to_vec(),
                mime_type: Some(mime_type.to_string()),
            }))
        }

        gltf::image::Source::Uri { uri, .. } => {
            // Defensive: should have been caught in pre-validation.
            Err(GlbImportError::MissingBuffer {
                detail: format!(
                    "external URI textures are not supported; embed textures in the \
                     GLB file (found URI: {uri})"
                ),
            })
        }
    }
}

/// Map a glTF material's PBR fields to a [`MaterialDefinition`].
///
/// Mapped fields:
///
/// | glTF field                                    | `MaterialDefinition` field |
/// |-----------------------------------------------|---------------------------|
/// | `pbrMetallicRoughness.baseColorFactor`        | `base_color`              |
/// | `pbrMetallicRoughness.metallicFactor`         | `metallic`                |
/// | `pbrMetallicRoughness.roughnessFactor`        | `roughness`               |
/// | `emissiveFactor` (when non-zero)              | `emissive`                |
/// | `alphaMode`                                   | `alpha_mode`              |
///
/// Advanced PBR channels (normal map, metallic-roughness map, occlusion map,
/// emissive map textures) are deferred to a later compatibility phase.
fn convert_gltf_material(material: &gltf::Material<'_>) -> MaterialDefinition {
    let pbr = material.pbr_metallic_roughness();

    let emissive = material.emissive_factor();
    let emissive_opt = if emissive[0] > 0.0 || emissive[1] > 0.0 || emissive[2] > 0.0 {
        Some(emissive)
    } else {
        None
    };

    let alpha_mode = match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask,
    };

    MaterialDefinition {
        base_color: pbr.base_color_factor(),
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        emissive: emissive_opt,
        alpha_mode,
    }
}

/// Produce a sanitized export filename from an image label and MIME type.
///
/// The label is lower-cased and every non-alphanumeric, non-underscore character
/// is replaced with `_`.  Consecutive underscores are collapsed into one.
/// The file extension is derived from the MIME type.
///
/// | MIME type      | Extension |
/// |----------------|-----------|
/// | `image/png`    | `.png`    |
/// | `image/jpeg`   | `.jpg`    |
/// | `image/webp`   | `.webp`   |
/// | `image/gif`    | `.gif`    |
/// | `image/bmp`    | `.bmp`    |
/// | other `image/` | sub-type  |
/// | `None`         | `.bin`    |
fn sanitize_glb_file_name(label: &str, mime_type: Option<&str>) -> String {
    let ext = match mime_type {
        Some("image/png") => "png",
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/webp") => "webp",
        Some("image/gif") => "gif",
        Some("image/bmp") => "bmp",
        Some(other) => other.strip_prefix("image/").unwrap_or("bin"),
        None => "bin",
    };

    // Lower-case; replace non-alphanumeric/non-underscore with '_'.
    let sanitized: String = label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();

    // Collapse consecutive underscores and trim leading/trailing ones.
    let collapsed = sanitized
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    let name = if collapsed.is_empty() {
        "image".to_string()
    } else {
        collapsed
    };

    format!("{name}.{ext}")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GLB binary builder helpers ────────────────────────────────────────

    /// Build a GLB binary from a JSON chunk and an optional binary chunk.
    ///
    /// The JSON chunk is space-padded to a 4-byte boundary.
    /// The BIN chunk (when present) is zero-padded to a 4-byte boundary.
    fn build_glb(json: &str, bin: Option<&[u8]>) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }

        let bin_chunk_total = bin.map_or(0usize, |b| {
            let padded_len = (b.len() + 3) & !3;
            8 + padded_len // chunkLength(4) + chunkType(4) + padded data
        });

        let total_len = 12 + 8 + json_bytes.len() + bin_chunk_total;
        let mut out = Vec::with_capacity(total_len);

        // 12-byte GLB header
        out.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic "glTF"
        out.extend_from_slice(&2u32.to_le_bytes()); // version 2
        out.extend_from_slice(&(total_len as u32).to_le_bytes());

        // JSON chunk
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        out.extend_from_slice(&json_bytes);

        // Optional BIN chunk
        if let Some(bin_data) = bin {
            let padded_len = (bin_data.len() + 3) & !3;
            out.extend_from_slice(&(padded_len as u32).to_le_bytes());
            out.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            out.extend_from_slice(bin_data);
            let pad = padded_len - bin_data.len();
            out.resize(out.len() + pad, 0x00);
        }

        out
    }

    /// Default import options with all fields at baseline values.
    fn default_options() -> GlbImportOptions {
        GlbImportOptions {
            source_path: None,
            scale: 1.0,
            flip_uv_v: false,
            scene_index: 0,
            default_color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Append a `[f32; 3]` as three little-endian floats.
    fn push_vec3(buf: &mut Vec<u8>, v: [f32; 3]) {
        for f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }

    /// Append a `[f32; 2]` as two little-endian floats.
    fn push_vec2(buf: &mut Vec<u8>, v: [f32; 2]) {
        for f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }

    /// Append a `u16` as two little-endian bytes.
    fn push_u16(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    // ── GLB fixture factories ─────────────────────────────────────────────

    /// GLB with one triangle: positions `[(-1,0,0),(1,0,0),(0,1,0)]`,
    /// UVs `[(0,0),(1,0),(0.5,1)]`, indices `[0,1,2]`, no material.
    ///
    /// Binary layout (66 bytes):
    /// - offset   0: positions (3 × VEC3 FLOAT = 36 bytes)
    /// - offset  36: UVs       (3 × VEC2 FLOAT = 24 bytes)
    /// - offset  60: indices   (3 × UNSIGNED_SHORT = 6 bytes)
    fn build_triangle_uvs_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for uv in [[0.0f32, 0.0], [1.0, 0.0], [0.5, 1.0]] {
            push_vec2(&mut bin, uv);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        assert_eq!(bin.len(), 66);

        build_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":"TestNode"}],"meshes":[{"name":"TestMesh","primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"indices":2,"mode":4}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"},{"bufferView":2,"componentType":5123,"count":3,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24},{"buffer":0,"byteOffset":60,"byteLength":6}],"buffers":[{"byteLength":66}]}"#,
            Some(&bin),
        )
    }

    /// GLB where the primitive declares `POSITION` in attributes but has no binary
    /// chunk (the blob is absent).  When the reader gets `None` for buffer 0 it
    /// cannot read position data, triggering [`GlbImportError::MissingPositions`].
    ///
    /// The document is structurally valid JSON (POSITION is declared as required by
    /// the glTF spec); the _data_ is absent because there is no BIN chunk.  This is
    /// a "buffer data missing" scenario rather than a JSON-schema violation, so
    /// `gltf::Gltf::from_slice` succeeds and our reader catches the absence.
    fn build_no_position_glb() -> Vec<u8> {
        // No binary chunk passed — gltf.blob = None → blob = &[] → reader returns None
        // for buffer 0 → read_positions() returns None → MissingPositions.
        build_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"name":"NoPos","primitives":[{"attributes":{"POSITION":0},"mode":4}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],"buffers":[{"byteLength":36}]}"#,
            None, // deliberately absent: reader will return None for buffer 0
        )
    }

    /// GLB with a triangle, a PBR material with the given `base_color_factor`.
    ///
    /// Binary layout (42 bytes):
    /// - offset  0: positions (36 bytes)
    /// - offset 36: indices   (6 bytes)
    fn build_triangle_material_glb(base_color: [f32; 4]) -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        assert_eq!(bin.len(), 42);

        let [r, g, b, a] = base_color;
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[{r},{g},{b},{a}]}}}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}}],"buffers":[{{"byteLength":42}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    /// Fake encoded image bytes used as embedded texture data in tests.
    ///
    /// These are not valid image files; they are used only to verify that the
    /// parser reads the correct bytes from the GLB buffer view.
    const FAKE_IMAGE_BYTES: &[u8] = b"FAKE_GLB_TEXTURE_PAYLOAD";

    /// GLB with a triangle, a material that references a base-color texture, and
    /// a fake image embedded in a GLB buffer view.
    ///
    /// Binary layout (68 bytes):
    /// - offset  0: positions (36 bytes)
    /// - offset 36: indices   (6 bytes)
    /// - offset 42: padding   (2 bytes, aligns image to offset 44)
    /// - offset 44: image     (FAKE_IMAGE_BYTES.len() bytes)
    fn build_triangle_texture_glb() -> Vec<u8> {
        let img_len = FAKE_IMAGE_BYTES.len(); // 24 bytes
        let img_offset = 44usize;

        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        // Pad positions(36) + indices(6) = 42 bytes to 44 (next multiple of 4)
        bin.push(0x00);
        bin.push(0x00);
        bin.extend_from_slice(FAKE_IMAGE_BYTES);
        let total = img_offset + img_len;
        assert_eq!(bin.len(), total);

        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}}}}}],"textures":[{{"source":0}}],"images":[{{"bufferView":2,"mimeType":"image/png"}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}},{{"buffer":0,"byteOffset":{img_offset},"byteLength":{img_len}}}],"buffers":[{{"byteLength":{total}}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    /// Fake image bytes for the two-primitives test — first primitive.
    const FAKE_IMAGE_A: &[u8] = b"IMAGE_DATA_PRIM_A";
    /// Fake image bytes for the two-primitives test — second primitive.
    const FAKE_IMAGE_B: &[u8] = b"IMAGE_DATA_PRIM_B";

    /// GLB with one mesh containing two primitives, each referencing a distinct
    /// embedded image.
    ///
    /// Binary layout (118 bytes):
    /// - offset  0: positions A (36 bytes)
    /// - offset 36: positions B (36 bytes)
    /// - offset 72: indices A   (6 bytes)
    /// - offset 78: indices B   (6 bytes)
    /// - offset 84: image A     (17 bytes)
    /// - offset 101: image B    (17 bytes)
    fn build_two_primitives_glb() -> Vec<u8> {
        let img_a_len = FAKE_IMAGE_A.len(); // 17
        let img_b_len = FAKE_IMAGE_B.len(); // 17
        let img_a_offset = 84usize;
        let img_b_offset = img_a_offset + img_a_len; // 101
        let total = img_b_offset + img_b_len; // 118

        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for pos in [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        // 36+36+6+6 = 84 — already 4-byte aligned
        bin.extend_from_slice(FAKE_IMAGE_A);
        bin.extend_from_slice(FAKE_IMAGE_B);
        assert_eq!(bin.len(), total);

        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":2,"material":0,"mode":4}},{{"attributes":{{"POSITION":1}},"indices":3,"material":1,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}}}}},{{"pbrMetallicRoughness":{{"baseColorTexture":{{"index":1}}}}}}],"textures":[{{"source":0}},{{"source":1}}],"images":[{{"bufferView":4,"mimeType":"image/png"}},{{"bufferView":5,"mimeType":"image/png"}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5126,"count":3,"type":"VEC3","min":[0.0,0.0,0.0],"max":[2.0,1.0,0.0]}},{{"bufferView":2,"componentType":5123,"count":3,"type":"SCALAR"}},{{"bufferView":3,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":36}},{{"buffer":0,"byteOffset":72,"byteLength":6}},{{"buffer":0,"byteOffset":78,"byteLength":6}},{{"buffer":0,"byteOffset":{img_a_offset},"byteLength":{img_a_len}}},{{"buffer":0,"byteOffset":{img_b_offset},"byteLength":{img_b_len}}}],"buffers":[{{"byteLength":{total}}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    /// GLB with a Lines primitive (mode `1`).
    ///
    /// Binary layout (28 bytes):
    /// - offset  0: 2 positions (24 bytes)
    /// - offset 24: 2 indices   (4 bytes)
    fn build_lines_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1] {
            push_u16(&mut bin, idx);
        }
        assert_eq!(bin.len(), 28);

        build_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"mode":1}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":2,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,0.0,0.0]},{"bufferView":1,"componentType":5123,"count":2,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":24},{"buffer":0,"byteOffset":24,"byteLength":4}],"buffers":[{"byteLength":28}]}"#,
            Some(&bin),
        )
    }

    /// GLB with no scenes (only the required `asset` property).
    fn build_empty_scenes_glb() -> Vec<u8> {
        build_glb(r#"{"asset":{"version":"2.0"}}"#, None)
    }

    /// GLB whose material's base-color texture references an external URI image.
    ///
    /// Binary layout (42 bytes — mesh data only; no image bytes in the blob):
    /// - offset  0: positions (36 bytes)
    /// - offset 36: indices   (6 bytes)
    fn build_external_uri_texture_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        assert_eq!(bin.len(), 42);

        build_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0,"mode":4}]}],"materials":[{"pbrMetallicRoughness":{"baseColorTexture":{"index":0}}}],"textures":[{"source":0}],"images":[{"uri":"texture.png"}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],"buffers":[{"byteLength":42}]}"#,
            Some(&bin),
        )
    }

    /// GLB with one triangle and a full PBR material: `base_color_factor`,
    /// `metallicFactor`, and `roughnessFactor` all specified.
    ///
    /// Binary layout (42 bytes): same geometry as [`build_triangle_material_glb`].
    fn build_triangle_full_material_glb(
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
    ) -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        assert_eq!(bin.len(), 42);

        let [r, g, b, a] = base_color;
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[{r},{g},{b},{a}],"metallicFactor":{metallic},"roughnessFactor":{roughness}}}}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}}],"buffers":[{{"byteLength":42}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    /// GLB with one triangle and a material that has `normalTexture` set.
    ///
    /// The normal-map image is fake embedded bytes.  Used to test
    /// `ImportedGlbScene::has_unsupported_pbr_channels` detection.
    ///
    /// Binary layout:
    /// - offset  0: positions (36 bytes)
    /// - offset 36: indices   (6 bytes)
    /// - offset 42: padding   (2 bytes — aligns image to offset 44)
    /// - offset 44: fake normal-map image bytes
    fn build_normal_texture_glb() -> Vec<u8> {
        const FAKE_NORMAL: &[u8] = b"FAKE_NORMAL_MAP_DATA";
        let img_offset: usize = 44;
        let img_len = FAKE_NORMAL.len();
        let total = img_offset + img_len;

        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        bin.push(0x00);
        bin.push(0x00); // pad positions(36) + indices(6) to offset 44
        bin.extend_from_slice(FAKE_NORMAL);
        assert_eq!(bin.len(), total);

        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[1.0,1.0,1.0,1.0]}},"normalTexture":{{"index":0}}}}],"textures":[{{"source":0}}],"images":[{{"bufferView":2,"mimeType":"image/png"}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}},{{"buffer":0,"byteOffset":{img_offset},"byteLength":{img_len}}}],"buffers":[{{"byteLength":{total}}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    /// GLB with one triangle and a material that has `occlusionTexture` set.
    ///
    /// Used to test `has_unsupported_pbr_channels` detection for the occlusion
    /// channel variant.
    fn build_occlusion_texture_glb() -> Vec<u8> {
        const FAKE_OCC: &[u8] = b"FAKE_OCCLUSION_DATA";
        let img_offset: usize = 44;
        let img_len = FAKE_OCC.len();
        let total = img_offset + img_len;

        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            push_vec3(&mut bin, pos);
        }
        for idx in [0u16, 1, 2] {
            push_u16(&mut bin, idx);
        }
        bin.push(0x00);
        bin.push(0x00); // pad to offset 44
        bin.extend_from_slice(FAKE_OCC);
        assert_eq!(bin.len(), total);

        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[1.0,1.0,1.0,1.0]}},"occlusionTexture":{{"index":0}}}}],"textures":[{{"source":0}}],"images":[{{"bufferView":2,"mimeType":"image/png"}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1.0,0.0,0.0],"max":[1.0,1.0,0.0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}},{{"buffer":0,"byteOffset":{img_offset},"byteLength":{img_len}}}],"buffers":[{{"byteLength":{total}}}]}}"#
        );
        build_glb(&json, Some(&bin))
    }

    // ── Phase 5 test helpers end; file-based fixture helper below ─────────────────

    /// Path helper for binary GLB fixture files in `data/test_fixtures/`.
    ///
    /// Used when a test requires a real `.glb` file on disk rather than an
    /// in-memory fixture.  All Phase 1 tests use in-memory data; this helper
    /// is included for future phases.
    #[allow(dead_code)]
    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/test_fixtures")
            .join(name)
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_import_glb_rejects_missing_positions() {
        let glb = build_no_position_glb();
        let result = import_glb_scene_from_bytes(&glb, &default_options());
        assert!(
            matches!(result, Err(GlbImportError::MissingPositions { .. })),
            "expected MissingPositions, got {result:?}"
        );
    }

    #[test]
    fn test_import_glb_triangle_positions_indices_uvs() {
        let glb = build_triangle_uvs_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert_eq!(scene.meshes.len(), 1);
        let mesh = &scene.meshes[0].mesh_def;

        assert_eq!(mesh.vertices.len(), 3, "should have 3 vertices");
        assert_eq!(mesh.indices, vec![0u32, 1, 2], "indices should be [0,1,2]");

        let uvs = mesh.uvs.as_ref().expect("UVs must be present");
        assert_eq!(uvs.len(), 3, "should have 3 UV pairs");
        // First UV is (0.0, 0.0)
        assert!(
            (uvs[0][0] - 0.0_f32).abs() < f32::EPSILON,
            "UV[0].u should be 0.0"
        );
        assert!(
            (uvs[0][1] - 0.0_f32).abs() < f32::EPSILON,
            "UV[0].v should be 0.0"
        );
        // Third UV is (0.5, 1.0)
        assert!((uvs[2][0] - 0.5_f32).abs() < 1e-5, "UV[2].u should be 0.5");
        assert!(
            (uvs[2][1] - 1.0_f32).abs() < f32::EPSILON,
            "UV[2].v should be 1.0"
        );
    }

    #[test]
    fn test_import_glb_applies_scale_to_vertices() {
        let glb = build_triangle_uvs_glb();
        let opts = GlbImportOptions {
            scale: 2.0,
            ..default_options()
        };
        let scene = import_glb_scene_from_bytes(&glb, &opts).expect("valid GLB should parse");

        let verts = &scene.meshes[0].mesh_def.vertices;
        // Original vertex 0: (-1, 0, 0) → scaled by 2 → (-2, 0, 0)
        assert!(
            (verts[0][0] - (-2.0_f32)).abs() < f32::EPSILON,
            "vertex[0].x should be -2.0 after scale=2.0, got {}",
            verts[0][0]
        );
        assert!(
            (verts[0][1] - 0.0_f32).abs() < f32::EPSILON,
            "vertex[0].y should be 0.0"
        );
        // Original vertex 1: (1, 0, 0) → scaled by 2 → (2, 0, 0)
        assert!(
            (verts[1][0] - 2.0_f32).abs() < f32::EPSILON,
            "vertex[1].x should be 2.0 after scale=2.0, got {}",
            verts[1][0]
        );
        // Original vertex 2: (0, 1, 0) → scaled by 2 → (0, 2, 0)
        assert!(
            (verts[2][1] - 2.0_f32).abs() < f32::EPSILON,
            "vertex[2].y should be 2.0 after scale=2.0, got {}",
            verts[2][1]
        );
    }

    #[test]
    fn test_import_glb_material_base_color_maps_to_mesh_def() {
        let color = [0.5_f32, 0.5, 0.5, 1.0];
        let glb = build_triangle_material_glb(color);
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        let mesh = &scene.meshes[0].mesh_def;
        // Base color maps to both mesh_def.color and material.base_color
        for (i, (&expected, &actual)) in color.iter().zip(mesh.color.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < f32::EPSILON,
                "mesh_def.color[{i}] should be {expected}, got {actual}"
            );
        }
        let mat = mesh.material.as_ref().expect("material must be present");
        for (i, (&expected, &actual)) in color.iter().zip(mat.base_color.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < f32::EPSILON,
                "material.base_color[{i}] should be {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_import_glb_embedded_base_color_texture_payload() {
        let glb = build_triangle_texture_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert_eq!(scene.embedded_image_count, 1);
        let mesh = &scene.meshes[0];
        let payload = mesh
            .texture_payload
            .as_ref()
            .expect("texture_payload must be Some");

        assert_eq!(
            payload.bytes, FAKE_IMAGE_BYTES,
            "bytes should match the embedded image data"
        );
        assert!(
            payload.file_name_hint.ends_with(".png"),
            "file_name_hint should end with .png, got {:?}",
            payload.file_name_hint
        );
        assert_eq!(
            payload.mime_type.as_deref(),
            Some("image/png"),
            "mime_type should be image/png"
        );
        // Texture path is set to the placeholder
        assert!(
            mesh.mesh_def
                .texture_path
                .as_deref()
                .unwrap_or("")
                .starts_with("__glb_texture_"),
            "texture_path should start with __glb_texture_"
        );
    }

    #[test]
    fn test_import_glb_multiple_primitives_distinct_payloads() {
        let glb = build_two_primitives_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert_eq!(
            scene.meshes.len(),
            2,
            "should have 2 ImportedGlbMesh entries"
        );

        let label_0 = scene.meshes[0]
            .texture_payload
            .as_ref()
            .expect("primitive 0 must have texture_payload")
            .source_label
            .clone();
        let label_1 = scene.meshes[1]
            .texture_payload
            .as_ref()
            .expect("primitive 1 must have texture_payload")
            .source_label
            .clone();

        assert_ne!(
            label_0, label_1,
            "source_labels must be distinct: got {label_0:?} and {label_1:?}"
        );
        // Image names fall back to "image_N" since the JSON has no "name" fields.
        assert_eq!(label_0, "image_0");
        assert_eq!(label_1, "image_1");
    }

    #[test]
    fn test_import_glb_unsupported_primitive_mode_returns_error() {
        let glb = build_lines_glb();
        let result = import_glb_scene_from_bytes(&glb, &default_options());
        assert!(
            matches!(
                result,
                Err(GlbImportError::UnsupportedPrimitive { ref mode }) if mode == "Lines"
            ),
            "expected UnsupportedPrimitive {{ mode: \"Lines\" }}, got {result:?}"
        );
    }

    #[test]
    fn test_import_glb_no_texture_gives_none_payload() {
        // Triangle with UVs but no material → no base-color texture
        let glb = build_triangle_uvs_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        let mesh = &scene.meshes[0];
        assert!(
            mesh.texture_payload.is_none(),
            "texture_payload should be None when no base-color texture is present"
        );
        assert!(
            mesh.mesh_def.texture_path.is_none(),
            "texture_path should be None when no base-color texture is present"
        );
    }

    #[test]
    fn test_import_glb_empty_scene_returns_error() {
        let glb = build_empty_scenes_glb();
        let result = import_glb_scene_from_bytes(&glb, &default_options());
        assert!(
            matches!(result, Err(GlbImportError::EmptyScene)),
            "expected EmptyScene, got {result:?}"
        );
    }

    #[test]
    fn test_import_glb_external_uri_texture_returns_error() {
        let glb = build_external_uri_texture_glb();
        let result = import_glb_scene_from_bytes(&glb, &default_options());
        assert!(
            matches!(result, Err(GlbImportError::MissingBuffer { .. })),
            "expected MissingBuffer for external URI texture, got {result:?}"
        );
    }

    #[test]
    fn test_import_glb_flip_uv_v() {
        let glb = build_triangle_uvs_glb();
        let opts = GlbImportOptions {
            flip_uv_v: true,
            ..default_options()
        };
        let scene = import_glb_scene_from_bytes(&glb, &opts).expect("valid GLB should parse");

        let uvs = scene.meshes[0]
            .mesh_def
            .uvs
            .as_ref()
            .expect("UVs must be present");

        // Original UVs: [(0,0), (1,0), (0.5,1)]
        // After flip_uv_v: [(0,1), (1,1), (0.5,0)]
        assert!(
            (uvs[0][1] - 1.0_f32).abs() < f32::EPSILON,
            "UV[0].v should be 1.0 after flip (was 0.0), got {}",
            uvs[0][1]
        );
        assert!(
            (uvs[1][1] - 1.0_f32).abs() < f32::EPSILON,
            "UV[1].v should be 1.0 after flip (was 0.0), got {}",
            uvs[1][1]
        );
        assert!(
            (uvs[2][1] - 0.0_f32).abs() < f32::EPSILON,
            "UV[2].v should be 0.0 after flip (was 1.0), got {}",
            uvs[2][1]
        );
        // U coordinates must remain unchanged
        assert!(
            (uvs[0][0] - 0.0_f32).abs() < f32::EPSILON,
            "UV[0].u should be unchanged"
        );
        assert!(
            (uvs[2][0] - 0.5_f32).abs() < 1e-5,
            "UV[2].u should be unchanged"
        );
    }

    // ── Phase 5 tests: Runtime and Domain Compatibility ─────────────────────

    /// PBR `baseColorFactor: [0.8, 0.2, 0.1, 1.0]` must map to both
    /// `MeshDefinition.color` and `MaterialDefinition.base_color`.
    #[test]
    fn test_glb_material_base_color_maps_to_domain_material() {
        let color = [0.8_f32, 0.2, 0.1, 1.0];
        let glb = build_triangle_material_glb(color);
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert_eq!(scene.meshes.len(), 1, "expected exactly one mesh");
        let mesh = &scene.meshes[0].mesh_def;

        for (i, (&expected, &actual)) in color.iter().zip(mesh.color.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < f32::EPSILON,
                "mesh_def.color[{i}] should be {expected:.4}, got {actual:.4}"
            );
        }
        let mat = mesh.material.as_ref().expect("material must be present");
        for (i, (&expected, &actual)) in color.iter().zip(mat.base_color.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < f32::EPSILON,
                "material.base_color[{i}] should be {expected:.4}, got {actual:.4}"
            );
        }
    }

    /// `metallicFactor: 0.3` and `roughnessFactor: 0.7` must map to
    /// `MaterialDefinition.metallic` and `MaterialDefinition.roughness`.
    #[test]
    fn test_glb_material_metallic_roughness_maps_to_domain() {
        let glb = build_triangle_full_material_glb([1.0, 1.0, 1.0, 1.0], 0.3, 0.7);
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert_eq!(scene.meshes.len(), 1, "expected exactly one mesh");
        let mat = scene.meshes[0]
            .mesh_def
            .material
            .as_ref()
            .expect("material must be present");

        assert!(
            (mat.metallic - 0.3_f32).abs() < f32::EPSILON,
            "metallic should be 0.3, got {}",
            mat.metallic
        );
        assert!(
            (mat.roughness - 0.7_f32).abs() < f32::EPSILON,
            "roughness should be 0.7, got {}",
            mat.roughness
        );
    }

    /// A GLB whose material has `normalTexture` must set
    /// `ImportedGlbScene::has_unsupported_pbr_channels = true` and geometry
    /// must still import normally.
    #[test]
    fn test_glb_normal_texture_sets_unsupported_pbr_channels_flag() {
        let glb = build_normal_texture_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert!(
            scene.has_unsupported_pbr_channels,
            "normalTexture must set has_unsupported_pbr_channels=true"
        );
        assert_eq!(
            scene.meshes.len(),
            1,
            "geometry must still import with one mesh"
        );
    }

    /// A GLB whose material has `occlusionTexture` must also set the flag.
    #[test]
    fn test_glb_occlusion_texture_sets_unsupported_pbr_channels_flag() {
        let glb = build_occlusion_texture_glb();
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert!(
            scene.has_unsupported_pbr_channels,
            "occlusionTexture must set has_unsupported_pbr_channels=true"
        );
        assert_eq!(
            scene.meshes.len(),
            1,
            "geometry must still import with one mesh"
        );
    }

    /// A standard PBR material (color, metallic, roughness scalars only — no
    /// texture maps) must leave `has_unsupported_pbr_channels = false`.
    #[test]
    fn test_glb_standard_pbr_leaves_unsupported_channels_flag_false() {
        let glb = build_triangle_full_material_glb([0.8, 0.2, 0.1, 1.0], 0.3, 0.7);
        let scene =
            import_glb_scene_from_bytes(&glb, &default_options()).expect("valid GLB should parse");

        assert!(
            !scene.has_unsupported_pbr_channels,
            "a material with only color/metallic/roughness scalars must leave \
             has_unsupported_pbr_channels=false"
        );
    }
}
