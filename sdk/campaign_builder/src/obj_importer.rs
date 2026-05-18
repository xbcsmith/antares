// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! State and persistence helpers for the OBJ and GLB importer workflow.
//!
//! This module owns the non-UI state needed by the Importer tab so later UI
//! phases can focus on interaction and rendering instead of file I/O and color
//! assignment mechanics.
//!
//! It is the seam between the parser backends and the importer UI:
//! `mesh_obj_io.rs` and `mesh_glb_io.rs` return domain `MeshDefinition` values,
//! this module turns them into editable importer rows plus campaign-scoped state,
//! and `obj_importer_ui.rs` renders and exports that state.
//!
//! Both OBJ and GLB sources converge into the same [`ObjImporterState`] and export
//! code paths.  The active source format is recorded in
//! [`ObjImporterState::source_format`] and each imported mesh carries a generalized
//! [`ImportedTexturePayload`] that covers both OBJ filesystem texture paths and GLB
//! embedded image bytes.
//!
//! # Examples
//!
//! ```
//! use campaign_builder::obj_importer::{ImporterMode, ObjImporterState};
//!
//! let state = ObjImporterState::new();
//! assert_eq!(state.mode, ImporterMode::Idle);
//! assert!(state.meshes.is_empty());
//! ```

use crate::color_palette::{suggest_color_for_mesh, CustomPalette, PaletteError};
use crate::mesh_glb_io::{
    import_glb_scene_from_file, GlbImportError, GlbImportOptions, ImportedGlbScene,
};
use crate::mesh_obj_io::{
    import_obj_scene_for_importer_from_obj_file_with_options, ImportedObjMaterialSwatch,
    ImportedObjMesh, ImportedObjMeshColorSource, ImportedObjMtlSourceKind, ImportedObjScene,
    ObjError, ObjImportOptions,
};
use antares::domain::types::{CreatureId, FurnitureMeshId};
use antares::domain::visual::{AlphaMode, MeshDefinition};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Importer UI mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImporterMode {
    /// No OBJ file is loaded.
    #[default]
    Idle,
    /// An OBJ file has been loaded and its meshes are ready for editing.
    Loaded,
    /// An export is currently in progress.
    Exporting,
}

/// Target asset type for the importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportType {
    /// Export as a creature asset.
    #[default]
    Creature,
    /// Export as an item asset.
    Item,
    /// Export as a furniture mesh asset.
    Furniture,
}

/// Records how the importer's current mesh color was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedMeshColorSource {
    /// The current color came from explicit MTL material color data.
    ImportedMaterial,
    /// The current color came from the built-in mesh-name heuristic.
    AutoAssigned,
    /// The current color was changed by the user after import.
    ManualOverride,
}

/// Records which MTL source path, if any, was used for the current importer session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportedMtlSourceKind {
    /// No MTL file was resolved for the current import.
    #[default]
    None,
    /// One or more MTL files were discovered from OBJ `mtllib` directives.
    AutoDetected,
    /// A manually selected MTL file override was used.
    ManualOverride,
}

/// Records whether the current importer session loaded an OBJ or GLB source.
///
/// Tracks format origin for the active importer session.  The field is reset to
/// [`ImportSourceFormat::Obj`] by [`ObjImporterState::clear`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportSourceFormat {
    /// Source was an OBJ/MTL file pair.
    #[default]
    Obj,
    /// Source was a binary glTF (`.glb`) file.
    Glb,
}

/// Temporary imported-material swatch surfaced in the importer UI for the current session.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMaterialSwatch {
    /// Material name used as the swatch label.
    pub label: String,
    /// Imported RGBA color derived from MTL diffuse and dissolve values.
    pub color: [f32; 4],
    /// Optional texture metadata preserved from `map_Kd` when portable.
    pub texture_path: Option<String>,
    /// Resolved source texture path used for export-time asset copying.
    pub texture_source_path: Option<PathBuf>,
}

/// Generalized texture source for export: covers both OBJ filesystem paths and
/// GLB embedded image bytes.
///
/// Replaces the OBJ-only `texture_source_path: Option<PathBuf>` field that was
/// previously stored directly on [`ImportedMesh`].  Use
/// `texture_payload.as_ref().and_then(|p| p.source_path.as_ref())` to get the
/// filesystem path when handling OBJ textures.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTexturePayload {
    /// Human-readable label for UI display (e.g., image name or material name).
    pub source_label: String,
    /// Sanitized export filename candidate including extension (e.g., `"albedo_0.png"`).
    pub file_name_hint: String,
    /// Embedded image bytes from a GLB file.  `None` for OBJ filesystem textures.
    pub bytes: Option<Vec<u8>>,
    /// Filesystem source path for OBJ/MTL textures.  `None` for embedded GLB images.
    pub source_path: Option<PathBuf>,
    /// MIME type when known from GLB metadata (e.g., `"image/png"`).
    pub mime_type: Option<String>,
}

/// A mesh loaded into the importer, along with editable per-mesh metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMesh {
    /// Display name shown in the mesh list.
    pub name: String,
    /// Number of vertices in the mesh.
    pub vertex_count: usize,
    /// Number of triangles in the mesh.
    pub triangle_count: usize,
    /// Editable RGBA color assigned to the mesh.
    pub color: [f32; 4],
    /// Tracks whether the current color came from MTL, fallback heuristics, or manual edits.
    pub color_source: ImportedMeshColorSource,
    /// Selection flag used for bulk operations.
    pub selected: bool,
    /// Backing mesh definition used for export.
    pub mesh_def: MeshDefinition,
    /// Generalized texture source used at export time.
    ///
    /// Replaces the OBJ-only `texture_source_path` field.  For OBJ imports the
    /// inner `source_path` field contains the filesystem path; for GLB imports
    /// the inner `bytes` field contains embedded image bytes.
    pub texture_payload: Option<ImportedTexturePayload>,
}

/// State owned by the OBJ importer tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjImporterState {
    /// Current importer lifecycle mode.
    pub mode: ImporterMode,
    /// Source OBJ file path, if any.
    pub source_path: Option<PathBuf>,
    /// Optional manual override for the MTL file path.
    pub manual_mtl_path: Option<PathBuf>,
    /// How the currently loaded importer session resolved its MTL source.
    pub active_mtl_source: ImportedMtlSourceKind,
    /// Which format was loaded in the current importer session.
    ///
    /// Reset to [`ImportSourceFormat::Obj`] by [`ObjImporterState::clear`].
    pub source_format: ImportSourceFormat,
    /// `mtllib` names declared by the current OBJ source, if any.
    pub declared_mtl_libraries: Vec<String>,
    /// Resolved MTL file paths that were actually used during the current import.
    pub resolved_mtl_paths: Vec<PathBuf>,
    /// Session-only imported material swatches surfaced in the importer palette.
    pub imported_material_palette: Vec<ImportedMaterialSwatch>,
    /// Parsed meshes currently loaded in the importer.
    pub meshes: Vec<ImportedMesh>,
    /// Whether the export target is a creature or an item.
    pub export_type: ExportType,
    /// Suggested next creature ID for export.
    pub creature_id: CreatureId,
    /// Suggested next furniture mesh ID for export.
    pub furniture_id: FurnitureMeshId,
    /// Name entered by the user for the export.
    pub creature_name: String,
    /// Optional category subfolder used when exporting item or furniture meshes.
    pub category: String,
    /// Uniform OBJ import scale.
    pub scale: f32,
    /// Status text shown in the importer UI.
    pub status_message: String,
    /// User-defined palette additions for the active campaign.
    pub custom_palette: CustomPalette,
    /// Mesh currently focused by the color editor, if any.
    pub active_mesh_index: Option<usize>,
    /// Draft label used by the custom-palette add form.
    pub new_custom_color_label: String,
    /// Draft color used by the custom-palette add form.
    pub new_custom_color: [f32; 4],
    /// When `true`, exporting a creature automatically switches the active tab
    /// to the Creatures editor. Defaults to `false` to keep the user in the
    /// Importer tab after export.
    pub open_after_export: bool,
    /// When `true`, the `status_message` describes a load or export error;
    /// the UI renders it in red. When `false`, it is an informational message
    /// rendered in italic text.
    pub is_error: bool,
}

/// Errors that can occur while preparing importer state.
#[derive(Debug, Error)]
pub enum ObjImporterError {
    /// OBJ loading failed.
    #[error("OBJ import failed: {0}")]
    Obj(#[from] ObjError),

    /// Custom palette load or save failed.
    #[error("Importer palette error: {0}")]
    Palette(#[from] PaletteError),

    /// GLB loading failed.
    #[error("GLB import failed: {0}")]
    Glb(#[from] GlbImportError),
}

impl ImportedMesh {
    /// Creates an importer mesh row from a domain mesh definition.
    pub fn from_mesh_definition(mesh_def: MeshDefinition) -> Self {
        let color_source = if has_imported_material_color(&mesh_def) {
            ImportedMeshColorSource::ImportedMaterial
        } else {
            ImportedMeshColorSource::AutoAssigned
        };

        Self::from_mesh_definition_with_color_source(mesh_def, color_source)
    }

    fn from_imported_obj_mesh(imported_mesh: ImportedObjMesh) -> Self {
        let color_source = match imported_mesh.color_source {
            ImportedObjMeshColorSource::ImportedMaterial => {
                ImportedMeshColorSource::ImportedMaterial
            }
            ImportedObjMeshColorSource::HeuristicFallback => ImportedMeshColorSource::AutoAssigned,
        };

        let texture_payload = imported_mesh.texture_source_path.map(|source_path| {
            let file_name_hint = source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("texture.png")
                .to_string();
            ImportedTexturePayload {
                source_label: file_name_hint.clone(),
                file_name_hint,
                bytes: None,
                source_path: Some(source_path),
                mime_type: None,
            }
        });

        Self::from_mesh_definition_with_color_source_and_texture_source(
            imported_mesh.mesh_def,
            texture_payload,
            color_source,
        )
    }

    fn from_mesh_definition_with_color_source(
        mesh_def: MeshDefinition,
        color_source: ImportedMeshColorSource,
    ) -> Self {
        Self::from_mesh_definition_with_color_source_and_texture_source(
            mesh_def,
            None::<ImportedTexturePayload>,
            color_source,
        )
    }

    fn from_mesh_definition_with_color_source_and_texture_source(
        mut mesh_def: MeshDefinition,
        texture_payload: Option<ImportedTexturePayload>,
        color_source: ImportedMeshColorSource,
    ) -> Self {
        let name = mesh_def.name.clone().unwrap_or_else(|| "mesh".to_string());
        let color = match color_source {
            ImportedMeshColorSource::ImportedMaterial | ImportedMeshColorSource::ManualOverride => {
                mesh_def.color
            }
            ImportedMeshColorSource::AutoAssigned => auto_assigned_color(&name, mesh_def.color[3]),
        };
        apply_color_to_mesh_definition(&mut mesh_def, color);

        Self {
            name,
            vertex_count: mesh_def.vertices.len(),
            triangle_count: mesh_def.indices.len() / 3,
            color,
            color_source,
            selected: true,
            mesh_def,
            texture_payload,
        }
    }

    /// Applies a new color to the mesh and its backing definition.
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
        self.color_source = ImportedMeshColorSource::ManualOverride;
        apply_color_to_mesh_definition(&mut self.mesh_def, color);
    }

    fn reapply_auto_color(&mut self) {
        let color = auto_assigned_color(&self.name, self.mesh_def.color[3]);
        self.color = color;
        self.color_source = ImportedMeshColorSource::AutoAssigned;
        apply_color_to_mesh_definition(&mut self.mesh_def, color);
    }

    /// Creates an importer mesh row from an imported GLB mesh definition.
    ///
    /// GLB meshes are not auto-color-assigned on load; they carry their PBR
    /// base-color factor directly.  The user can call
    /// [`ObjImporterState::auto_assign_colors`] to apply name-based heuristics.
    ///
    /// Meshes default to `selected: false`; the UI must opt-in to bulk selection.
    fn from_imported_glb_mesh(
        index: usize,
        mesh_def: MeshDefinition,
        payload: Option<ImportedTexturePayload>,
    ) -> Self {
        let color = mesh_def.color;
        let color_source = if color != [1.0, 1.0, 1.0, 1.0] {
            ImportedMeshColorSource::ImportedMaterial
        } else {
            ImportedMeshColorSource::AutoAssigned
        };
        Self {
            name: mesh_def
                .name
                .clone()
                .unwrap_or_else(|| format!("mesh_{}", index)),
            vertex_count: mesh_def.vertices.len(),
            triangle_count: mesh_def.indices.len() / 3,
            color,
            color_source,
            selected: false,
            mesh_def,
            texture_payload: payload,
        }
    }
}

impl ImportedMaterialSwatch {
    fn from_imported_obj_swatch(swatch: ImportedObjMaterialSwatch) -> Self {
        Self {
            label: swatch.label,
            color: swatch.color,
            texture_path: swatch.texture_path,
            texture_source_path: swatch.texture_source_path,
        }
    }
}

fn imported_mtl_source_kind_from_obj(kind: ImportedObjMtlSourceKind) -> ImportedMtlSourceKind {
    match kind {
        ImportedObjMtlSourceKind::None => ImportedMtlSourceKind::None,
        ImportedObjMtlSourceKind::AutoDetected => ImportedMtlSourceKind::AutoDetected,
        ImportedObjMtlSourceKind::ManualOverride => ImportedMtlSourceKind::ManualOverride,
    }
}

fn auto_assigned_color(name: &str, alpha: f32) -> [f32; 4] {
    let mut color = suggest_color_for_mesh(name);
    color[3] = alpha.clamp(0.0, 1.0);
    color
}

fn apply_color_to_mesh_definition(mesh_def: &mut MeshDefinition, color: [f32; 4]) {
    mesh_def.color = color;

    if let Some(material) = mesh_def.material.as_mut() {
        material.base_color = color;
        if color[3] < 1.0 {
            material.alpha_mode = AlphaMode::Blend;
        } else if material.alpha_mode == AlphaMode::Blend {
            material.alpha_mode = AlphaMode::Opaque;
        }
    }
}

fn has_imported_material_color(mesh_def: &MeshDefinition) -> bool {
    mesh_def.color != [1.0, 1.0, 1.0, 1.0]
}

impl Default for ObjImporterState {
    fn default() -> Self {
        Self {
            mode: ImporterMode::Idle,
            source_path: None,
            manual_mtl_path: None,
            active_mtl_source: ImportedMtlSourceKind::None,
            source_format: ImportSourceFormat::Obj,
            declared_mtl_libraries: Vec::new(),
            resolved_mtl_paths: Vec::new(),
            imported_material_palette: Vec::new(),
            meshes: Vec::new(),
            export_type: ExportType::Creature,
            creature_id: 4000,
            furniture_id: 10001,
            creature_name: String::new(),
            category: String::new(),
            scale: 0.01,
            status_message: String::new(),
            custom_palette: CustomPalette::default(),
            active_mesh_index: None,
            new_custom_color_label: String::new(),
            new_custom_color: [0.8, 0.8, 0.8, 1.0],
            open_after_export: false,
            is_error: false,
        }
    }
}

impl ObjImporterState {
    /// Creates a fresh importer state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears any loaded OBJ data and returns the importer to idle mode.
    ///
    /// The following fields survive a clear and are restored into the new
    /// default state: `scale`, `custom_palette`, `creature_id`,
    /// `furniture_id`, `export_type`, `category`, `new_custom_color`,
    /// `manual_mtl_path`, and `open_after_export`.
    pub fn clear(&mut self) {
        let scale = self.scale;
        let custom_palette = self.custom_palette.clone();
        let creature_id = self.creature_id;
        let furniture_id = self.furniture_id;
        let export_type = self.export_type;
        let category = self.category.clone();
        let new_custom_color = self.new_custom_color;
        let manual_mtl_path = self.manual_mtl_path.clone();
        let open_after_export = self.open_after_export;

        *self = Self {
            scale,
            custom_palette,
            creature_id,
            furniture_id,
            export_type,
            category,
            new_custom_color,
            manual_mtl_path,
            open_after_export,
            ..Self::default()
        };
    }

    /// Loads a campaign-specific custom palette from disk.
    pub fn load_custom_palette(&mut self, campaign_dir: &Path) -> Result<(), ObjImporterError> {
        self.custom_palette = CustomPalette::load_from_campaign_dir(campaign_dir)?;
        Ok(())
    }

    /// Saves the current custom palette to the active campaign.
    pub fn save_custom_palette(&self, campaign_dir: &Path) -> Result<(), ObjImporterError> {
        self.custom_palette.save_to_campaign_dir(campaign_dir)?;
        Ok(())
    }

    /// Adds or updates a custom palette entry.
    pub fn add_custom_color(&mut self, label: impl Into<String>, color: [f32; 4]) {
        self.custom_palette.add_color(label, color);
    }

    /// Removes a custom palette entry by label.
    pub fn remove_custom_color(&mut self, label: &str) -> bool {
        self.custom_palette.remove_color(label)
    }

    /// Loads importer meshes from already-parsed mesh definitions.
    pub fn load_mesh_definitions(
        &mut self,
        source_path: Option<PathBuf>,
        meshes: Vec<MeshDefinition>,
    ) {
        self.load_imported_mesh_rows(
            source_path,
            meshes
                .into_iter()
                .map(ImportedMesh::from_mesh_definition)
                .collect(),
            ImportedMtlSourceKind::None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
    }

    fn load_imported_scene(&mut self, source_path: Option<PathBuf>, scene: ImportedObjScene) {
        self.load_imported_mesh_rows(
            source_path,
            scene
                .meshes
                .into_iter()
                .map(ImportedMesh::from_imported_obj_mesh)
                .collect(),
            imported_mtl_source_kind_from_obj(scene.metadata.mtl_source_kind),
            scene.metadata.declared_material_libraries,
            scene.metadata.resolved_material_library_paths,
            scene
                .metadata
                .material_swatches
                .into_iter()
                .map(ImportedMaterialSwatch::from_imported_obj_swatch)
                .collect(),
        );
        self.source_format = ImportSourceFormat::Obj;
    }

    fn load_imported_mesh_rows(
        &mut self,
        source_path: Option<PathBuf>,
        meshes: Vec<ImportedMesh>,
        active_mtl_source: ImportedMtlSourceKind,
        declared_mtl_libraries: Vec<String>,
        resolved_mtl_paths: Vec<PathBuf>,
        imported_material_palette: Vec<ImportedMaterialSwatch>,
    ) {
        self.source_path = source_path;
        self.active_mtl_source = active_mtl_source;
        self.declared_mtl_libraries = declared_mtl_libraries;
        self.resolved_mtl_paths = resolved_mtl_paths;
        self.imported_material_palette = imported_material_palette;
        self.meshes = meshes;
        self.active_mesh_index = (!self.meshes.is_empty()).then_some(0);
        self.mode = if self.meshes.is_empty() {
            ImporterMode::Idle
        } else {
            ImporterMode::Loaded
        };
        self.status_message = if self.meshes.is_empty() {
            "No meshes loaded".to_string()
        } else {
            format!("Loaded {} meshes", self.meshes.len())
        };
    }

    /// Loads meshes directly from an OBJ file using the state's current scale.
    pub fn load_obj_file(&mut self, path: &Path) -> Result<(), ObjImporterError> {
        let mut options = self.obj_import_options();
        options.source_path = Some(path.to_path_buf());
        let path_string = path.to_string_lossy().to_string();
        let scene =
            import_obj_scene_for_importer_from_obj_file_with_options(&path_string, &options)?;
        self.load_imported_scene(Some(path.to_path_buf()), scene);
        Ok(())
    }

    /// Loads meshes from a GLB file and updates importer state.
    ///
    /// After a successful load `state.mode` will be [`ImporterMode::Loaded`] and
    /// `state.source_format` will be [`ImportSourceFormat::Glb`].
    ///
    /// Embedded base-color texture bytes are preserved in each mesh's
    /// [`ImportedMesh::texture_payload`] for export-time processing.
    ///
    /// # Errors
    ///
    /// Returns [`ObjImporterError::Glb`] when the file cannot be read or the GLB
    /// document is invalid (e.g., missing scene, unsupported primitive mode,
    /// external URI textures).
    pub fn load_glb_file(&mut self, path: &Path) -> Result<(), ObjImporterError> {
        let options = GlbImportOptions {
            source_path: Some(path.to_path_buf()),
            scale: self.scale,
            ..Default::default()
        };
        let scene = import_glb_scene_from_file(path, &options)?;
        self.load_imported_glb_scene(Some(path.to_path_buf()), scene);
        Ok(())
    }

    /// Populates importer state from a parsed [`ImportedGlbScene`].
    ///
    /// Converts each [`ImportedGlbMesh`] into an [`ImportedMesh`] row, sets
    /// `source_format` to [`ImportSourceFormat::Glb`], and writes a metadata
    /// summary into `status_message`.
    fn load_imported_glb_scene(&mut self, source_path: Option<PathBuf>, scene: ImportedGlbScene) {
        let meshes: Vec<ImportedMesh> = scene
            .meshes
            .into_iter()
            .enumerate()
            .map(|(i, glb_mesh)| {
                let payload = glb_mesh.texture_payload.map(|p| ImportedTexturePayload {
                    source_label: p.source_label,
                    file_name_hint: p.file_name_hint,
                    bytes: Some(p.bytes),
                    source_path: None,
                    mime_type: p.mime_type,
                });
                ImportedMesh::from_imported_glb_mesh(i, glb_mesh.mesh_def, payload)
            })
            .collect();

        let mut status_parts: Vec<String> = vec![format!(
            "GLB: {} mesh(es), {} embedded image(s), {} material(s)",
            meshes.len(),
            scene.embedded_image_count,
            scene.material_count,
        )];

        if scene.scene_count > 1 {
            status_parts.push(format!(
                "GLB contains {} scene(s); importing default scene.",
                scene.scene_count
            ));
        }

        if scene.has_skinning || scene.has_animations {
            status_parts.push("Skinning/animations present but not imported.".to_string());
        }

        if scene.has_unsupported_pbr_channels {
            status_parts.push(
                "Unsupported PBR channels (normal/occlusion/metallic-roughness) ignored."
                    .to_string(),
            );
        }

        let glb_metadata_summary = status_parts.join(" ");

        self.load_imported_mesh_rows(
            source_path,
            meshes,
            ImportedMtlSourceKind::None, // GLB has no MTL
            Vec::new(),                  // no declared_mtl_libraries
            Vec::new(),                  // no resolved_mtl_paths
            Vec::new(),                  // no imported_material_palette
        );
        self.source_format = ImportSourceFormat::Glb;
        self.is_error = false;
        self.status_message = glb_metadata_summary;
    }

    /// Re-runs automatic built-in color assignment for every loaded mesh.
    pub fn auto_assign_colors(&mut self) {
        for mesh in &mut self.meshes {
            mesh.reapply_auto_color();
        }
    }

    /// Updates the suggested creature ID shown by the importer.
    pub fn set_next_creature_id(&mut self, creature_id: CreatureId) {
        self.creature_id = creature_id;
    }

    /// Updates the suggested furniture mesh ID shown by the importer.
    pub fn set_next_furniture_id(&mut self, furniture_id: FurnitureMeshId) {
        self.furniture_id = furniture_id;
    }

    /// Sets the mesh currently targeted by the color editor.
    pub fn set_active_mesh(&mut self, index: Option<usize>) {
        self.active_mesh_index = index.filter(|idx| *idx < self.meshes.len());
    }

    /// Returns the mesh currently targeted by the color editor.
    pub fn active_mesh(&self) -> Option<&ImportedMesh> {
        self.active_mesh_index.and_then(|idx| self.meshes.get(idx))
    }

    /// Returns the mutable mesh currently targeted by the color editor.
    pub fn active_mesh_mut(&mut self) -> Option<&mut ImportedMesh> {
        self.active_mesh_index
            .and_then(move |idx| self.meshes.get_mut(idx))
    }

    /// Builds the parser options used by `mesh_obj_io` for the current state.
    ///
    /// This helper is the stable handoff between importer-tab state and the OBJ
    /// parser backend. Future parser-facing features, such as MTL resolution or
    /// source-path-aware import behavior, should be threaded through this method
    /// instead of assembled ad hoc in the UI layer.
    fn obj_import_options(&self) -> ObjImportOptions {
        ObjImportOptions {
            source_path: self.source_path.clone(),
            manual_mtl_path: self.manual_mtl_path.clone(),
            scale: self.scale,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExportType, ImportSourceFormat, ImportedMesh, ImportedMeshColorSource,
        ImportedMtlSourceKind, ImporterMode, ObjImporterState,
    };
    use antares::domain::visual::{AlphaMode, MaterialDefinition, MeshDefinition};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn named_triangle(name: &str) -> MeshDefinition {
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

    #[test]
    fn test_imported_mesh_from_mesh_definition_auto_assigns_color() {
        let mesh = ImportedMesh::from_mesh_definition(named_triangle("EM3D_Base_Body"));

        assert_eq!(mesh.color, [0.92, 0.85, 0.78, 1.0]);
        assert_eq!(mesh.mesh_def.color, [0.92, 0.85, 0.78, 1.0]);
        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.triangle_count, 1);
    }

    #[test]
    fn test_imported_mesh_from_mesh_definition_preserves_imported_material_color() {
        let mut mesh_def = named_triangle("EM3D_Base_Body");
        mesh_def.color = [0.2, 0.3, 0.4, 0.75];
        mesh_def.material = Some(MaterialDefinition {
            base_color: [0.2, 0.3, 0.4, 0.75],
            metallic: 0.0,
            roughness: 0.9,
            emissive: None,
            alpha_mode: AlphaMode::Blend,
        });

        let mesh = ImportedMesh::from_mesh_definition(mesh_def);

        assert_eq!(mesh.color, [0.2, 0.3, 0.4, 0.75]);
        assert_eq!(mesh.mesh_def.color, [0.2, 0.3, 0.4, 0.75]);
        assert_eq!(mesh.color_source, ImportedMeshColorSource::ImportedMaterial);
    }

    #[test]
    fn test_imported_mesh_set_color_marks_manual_override_and_updates_material() {
        let mut mesh_def = named_triangle("EM3D_Base_Body");
        mesh_def.material = Some(MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            emissive: None,
            alpha_mode: AlphaMode::Opaque,
        });

        let mut mesh = ImportedMesh::from_mesh_definition(mesh_def);
        mesh.set_color([0.1, 0.2, 0.3, 0.5]);

        assert_eq!(mesh.color_source, ImportedMeshColorSource::ManualOverride);
        assert_eq!(mesh.mesh_def.color, [0.1, 0.2, 0.3, 0.5]);
        assert_eq!(
            mesh.mesh_def.material.as_ref().unwrap().base_color,
            [0.1, 0.2, 0.3, 0.5]
        );
        assert_eq!(
            mesh.mesh_def.material.as_ref().unwrap().alpha_mode,
            AlphaMode::Blend
        );
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_preserves_explicit_white_mtl_color() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/white_hero.obj");
        let mtl_path = temp_dir.path().join("models/white_hero.mtl");
        fs::create_dir_all(obj_path.parent().unwrap()).unwrap();
        fs::write(
            &mtl_path,
            concat!("newmtl WhiteCloth\n", "Kd 1.0 1.0 1.0\n"),
        )
        .unwrap();
        fs::write(
            &obj_path,
            concat!(
                "mtllib white_hero.mtl\n",
                "o Cloth_Dress\n",
                "usemtl WhiteCloth\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.meshes[0].color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            state.meshes[0].color_source,
            ImportedMeshColorSource::ImportedMaterial
        );
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_uses_auto_color_when_mtl_has_no_diffuse() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let mtl_path = temp_dir.path().join("models/hero.mtl");
        fs::create_dir_all(obj_path.parent().unwrap()).unwrap();
        fs::write(
            &mtl_path,
            concat!("newmtl Textured\n", "d 0.5\n", "map_Kd textures/hero.png\n"),
        )
        .unwrap();
        fs::write(
            &obj_path,
            concat!(
                "mtllib hero.mtl\n",
                "o EM3D_Base_Body\n",
                "usemtl Textured\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.meshes[0].color, [0.92, 0.85, 0.78, 0.5]);
        assert_eq!(
            state.meshes[0].color_source,
            ImportedMeshColorSource::AutoAssigned
        );
        assert_eq!(
            state.meshes[0]
                .mesh_def
                .material
                .as_ref()
                .unwrap()
                .base_color,
            [0.92, 0.85, 0.78, 0.5]
        );
        assert!(state.imported_material_palette.is_empty());
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_tracks_imported_material_swatches_and_mtl_source() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let mtl_path = temp_dir.path().join("models/materials/hero.mtl");
        fs::create_dir_all(mtl_path.parent().unwrap()).unwrap();
        fs::write(
            &mtl_path,
            concat!(
                "newmtl HeroSkin\n",
                "Kd 0.7 0.6 0.5\n",
                "map_Kd textures/hero.png\n",
                "newmtl Cloth\n",
                "Kd 0.2 0.3 0.7\n"
            ),
        )
        .unwrap();
        fs::write(
            &obj_path,
            concat!(
                "mtllib materials/hero.mtl\n",
                "o Body\n",
                "usemtl HeroSkin\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.active_mtl_source, ImportedMtlSourceKind::AutoDetected);
        assert_eq!(
            state.declared_mtl_libraries,
            vec!["materials/hero.mtl".to_string()]
        );
        assert_eq!(state.resolved_mtl_paths, vec![mtl_path]);
        assert_eq!(state.imported_material_palette.len(), 2);
        assert_eq!(state.imported_material_palette[0].label, "Cloth");
        assert_eq!(
            state.imported_material_palette[0].color,
            [0.2, 0.3, 0.7, 1.0]
        );
        assert_eq!(state.imported_material_palette[1].label, "HeroSkin");
        assert_eq!(
            state.imported_material_palette[1].texture_path.as_deref(),
            Some("textures/hero.png")
        );
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_marks_manual_override_source() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("models/hero.obj");
        let auto_mtl_path = temp_dir.path().join("models/hero.mtl");
        let manual_mtl_path = temp_dir.path().join("overrides/manual.mtl");
        fs::create_dir_all(auto_mtl_path.parent().unwrap()).unwrap();
        fs::create_dir_all(manual_mtl_path.parent().unwrap()).unwrap();
        fs::write(&auto_mtl_path, "newmtl Auto\nKd 0.1 0.1 0.1\n").unwrap();
        fs::write(&manual_mtl_path, "newmtl Override\nKd 0.8 0.6 0.4\n").unwrap();
        fs::write(
            &obj_path,
            concat!(
                "mtllib hero.mtl\n",
                "o Body\n",
                "usemtl Override\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.manual_mtl_path = Some(manual_mtl_path.clone());
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(
            state.active_mtl_source,
            ImportedMtlSourceKind::ManualOverride
        );
        assert_eq!(state.resolved_mtl_paths, vec![manual_mtl_path]);
        assert_eq!(state.meshes[0].color, [0.8, 0.6, 0.4, 1.0]);
        assert_eq!(state.imported_material_palette.len(), 1);
        assert_eq!(state.imported_material_palette[0].label, "Override");
    }

    #[test]
    fn test_obj_importer_state_auto_assign_colors_marks_imported_meshes_as_auto_assigned() {
        let mut mesh_def = named_triangle("Hair_Pink");
        mesh_def.color = [0.3, 0.3, 0.3, 0.75];
        mesh_def.material = Some(MaterialDefinition {
            base_color: [0.3, 0.3, 0.3, 0.75],
            metallic: 0.0,
            roughness: 0.9,
            emissive: None,
            alpha_mode: AlphaMode::Blend,
        });
        let mut state = ObjImporterState::new();
        state.load_mesh_definitions(None, vec![mesh_def]);

        state.auto_assign_colors();

        assert_eq!(state.meshes[0].color, [0.92, 0.55, 0.70, 0.75]);
        assert_eq!(
            state.meshes[0].color_source,
            ImportedMeshColorSource::AutoAssigned
        );
    }

    #[test]
    fn test_obj_importer_state_load_mesh_definitions_transitions_to_loaded() {
        let mut state = ObjImporterState::new();
        state.export_type = ExportType::Item;
        state.load_mesh_definitions(
            None,
            vec![
                named_triangle("EM3D_Base_Body"),
                named_triangle("Hair_Pink"),
            ],
        );

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert_eq!(state.meshes.len(), 2);
        assert_eq!(state.export_type, ExportType::Item);
        assert_eq!(state.meshes[0].color, [0.92, 0.85, 0.78, 1.0]);
        assert_eq!(state.meshes[1].color, [0.92, 0.55, 0.70, 1.0]);
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_assigns_non_default_body_part_colors() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("female_1.obj");
        let obj_contents = concat!(
            "o EM3D_Base_Body\n",
            "v 0.0 0.0 0.0\n",
            "v 1.0 0.0 0.0\n",
            "v 0.0 1.0 0.0\n",
            "f 1 2 3\n",
            "o Hair_Pink\n",
            "v 0.0 0.0 1.0\n",
            "v 1.0 0.0 1.0\n",
            "v 0.0 1.0 1.0\n",
            "f 4 5 6\n",
            "o Cloth_Dress\n",
            "v 0.0 0.0 2.0\n",
            "v 1.0 0.0 2.0\n",
            "v 0.0 1.0 2.0\n",
            "f 7 8 9\n"
        );
        fs::write(&obj_path, obj_contents).unwrap();

        let mut state = ObjImporterState::new();
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert_eq!(state.meshes.len(), 3);
        assert_eq!(state.meshes[0].name, "EM3D_Base_Body");
        assert_eq!(state.meshes[0].color, [0.92, 0.85, 0.78, 1.0]);
        assert_eq!(state.meshes[1].color, [0.92, 0.55, 0.70, 1.0]);
        assert_eq!(state.meshes[2].color, [0.14, 0.12, 0.18, 1.0]);
    }

    #[test]
    fn test_obj_importer_state_clear_preserves_scale_palette_and_id() {
        let mut state = ObjImporterState::new();
        state.scale = 0.05;
        state.creature_id = 4012;
        state.furniture_id = 10042;
        state.export_type = ExportType::Furniture;
        state.category = "tables".to_string();
        state.manual_mtl_path = Some(PathBuf::from("materials/hero_override.mtl"));
        state.new_custom_color = [0.2, 0.4, 0.6, 1.0];
        state.add_custom_color("favorite_teal", [0.1, 0.7, 0.7, 1.0]);
        state.load_mesh_definitions(None, vec![named_triangle("EM3D_Base_Body")]);

        state.clear();

        assert_eq!(state.mode, ImporterMode::Idle);
        assert!(state.meshes.is_empty());
        assert_eq!(state.scale, 0.05);
        assert_eq!(state.creature_id, 4012);
        assert_eq!(state.furniture_id, 10042);
        assert_eq!(state.export_type, ExportType::Furniture);
        assert_eq!(state.category, "tables");
        assert_eq!(
            state.manual_mtl_path,
            Some(PathBuf::from("materials/hero_override.mtl"))
        );
        assert_eq!(state.new_custom_color, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(state.custom_palette.colors.len(), 1);
    }

    #[test]
    fn test_obj_importer_state_load_obj_file_uses_current_parser_options() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("scaled_triangle.obj");
        fs::write(
            &obj_path,
            concat!(
                "o Scaled\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "v 0.0 0.0 1.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.scale = 0.5;

        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert_eq!(state.meshes.len(), 1);
        assert_eq!(state.meshes[0].mesh_def.vertices[0], [0.5, 0.0, 0.0]);
        assert_eq!(state.meshes[0].mesh_def.vertices[1], [0.0, 0.5, 0.0]);
        assert_eq!(state.meshes[0].mesh_def.vertices[2], [0.0, 0.0, 0.5]);
    }

    #[test]
    fn test_obj_importer_options_forward_source_and_manual_mtl_paths() {
        let mut state = ObjImporterState::new();
        state.source_path = Some(PathBuf::from("models/hero.obj"));
        state.manual_mtl_path = Some(PathBuf::from("materials/hero_override.mtl"));
        state.scale = 0.25;

        let options = state.obj_import_options();

        assert_eq!(options.source_path, Some(PathBuf::from("models/hero.obj")));
        assert_eq!(
            options.manual_mtl_path,
            Some(PathBuf::from("materials/hero_override.mtl"))
        );
        assert_eq!(options.scale, 0.25);
    }

    #[test]
    fn test_obj_importer_state_save_custom_palette_writes_importer_palette_file() {
        let temp_dir = tempdir().unwrap();
        let mut state = ObjImporterState::new();
        state.add_custom_color("hero_skin", [0.7, 0.6, 0.5, 1.0]);

        state.save_custom_palette(temp_dir.path()).unwrap();

        let palette_path = temp_dir.path().join("config/importer_palette.ron");
        assert!(palette_path.exists());
        let saved = fs::read_to_string(&palette_path).unwrap();
        assert!(saved.contains("hero_skin"));

        let mut reloaded = ObjImporterState::new();
        reloaded.load_custom_palette(temp_dir.path()).unwrap();
        assert_eq!(reloaded.custom_palette, state.custom_palette);
    }

    // ─── GLB-specific helpers (Phase 2) ──────────────────────────────────────

    /// Build a minimal GLB binary from a JSON chunk and an optional binary chunk.
    /// JSON is space-padded; BIN is zero-padded to 4-byte alignment.
    fn build_test_glb(json: &str, bin: Option<&[u8]>) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let bin_chunk_total = bin.map_or(0usize, |b| {
            let padded = (b.len() + 3) & !3;
            8 + padded
        });
        let total_len = 12 + 8 + json_bytes.len() + bin_chunk_total;
        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic "glTF"
        out.extend_from_slice(&2u32.to_le_bytes()); // version
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        out.extend_from_slice(&json_bytes);
        if let Some(bin_data) = bin {
            let padded = (bin_data.len() + 3) & !3;
            out.extend_from_slice(&(padded as u32).to_le_bytes());
            out.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            out.extend_from_slice(bin_data);
            let pad = padded - bin_data.len();
            out.resize(out.len() + pad, 0x00);
        }
        out
    }

    /// A minimal GLB with one triangle mesh and no texture.
    fn build_minimal_triangle_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for f in pos {
                bin.extend_from_slice(&f.to_le_bytes());
            }
        }
        for idx in [0u16, 1, 2] {
            bin.extend_from_slice(&idx.to_le_bytes());
        }
        build_test_glb(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"name":"TestMesh","primitives":[{"attributes":{"POSITION":0},"indices":1,"mode":4}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],"buffers":[{"byteLength":42}]}"#,
            Some(&bin),
        )
    }

    /// A GLB with one triangle mesh that has an embedded base-color texture.
    fn build_textured_triangle_glb() -> Vec<u8> {
        const FAKE_BYTES: &[u8] = b"FAKE_PNG_TEXTURE_DATA";
        let img_offset = 44usize; // 36 pos + 6 idx + 2 padding
        let img_len = FAKE_BYTES.len();
        let total = img_offset + img_len;

        let mut bin = Vec::new();
        for pos in [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for f in pos {
                bin.extend_from_slice(&f.to_le_bytes());
            }
        }
        for idx in [0u16, 1, 2] {
            bin.extend_from_slice(&idx.to_le_bytes());
        }
        // pad positions(36) + indices(6) = 42 to 44
        bin.push(0x00);
        bin.push(0x00);
        bin.extend_from_slice(FAKE_BYTES);
        assert_eq!(bin.len(), total);

        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0,"mode":4}}]}}],"materials":[{{"pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}}}}}],"textures":[{{"source":0}}],"images":[{{"bufferView":2,"mimeType":"image/png"}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1,0,0],"max":[1,1,0]}},{{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}},{{"buffer":0,"byteOffset":{img_offset},"byteLength":{img_len}}}],"buffers":[{{"byteLength":{total}}}]}}"#
        );
        build_test_glb(&json, Some(&bin))
    }

    // ─── Phase 2 tests ────────────────────────────────────────────────────────

    #[test]
    fn test_obj_importer_state_load_glb_file_sets_loaded_mode() {
        let temp_dir = tempdir().unwrap();
        let glb_path = temp_dir.path().join("model.glb");
        fs::write(&glb_path, build_minimal_triangle_glb()).unwrap();

        let mut state = ObjImporterState::new();
        state.load_glb_file(&glb_path).unwrap();

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert_eq!(state.source_format, ImportSourceFormat::Glb);
        assert_eq!(state.meshes.len(), 1);
    }

    #[test]
    fn test_obj_importer_state_load_glb_preserves_texture_payload() {
        let temp_dir = tempdir().unwrap();
        let glb_path = temp_dir.path().join("textured.glb");
        fs::write(&glb_path, build_textured_triangle_glb()).unwrap();

        let mut state = ObjImporterState::new();
        state.load_glb_file(&glb_path).unwrap();

        assert!(!state.meshes.is_empty(), "expected at least one mesh");
        let mesh = &state.meshes[0];

        // texture_payload must be present and hold embedded bytes
        let payload = mesh
            .texture_payload
            .as_ref()
            .expect("texture_payload must be Some for textured GLB");
        assert!(
            payload.bytes.is_some(),
            "GLB texture payload must have embedded bytes"
        );

        // texture_path in the mesh_def must contain the GLB placeholder
        let texture_path = mesh.mesh_def.texture_path.as_deref().unwrap_or("");
        assert!(
            texture_path.contains("__glb_texture_"),
            "expected placeholder in texture_path, got: {texture_path}"
        );
    }

    #[test]
    fn test_obj_importer_state_load_glb_metadata_summary_in_status() {
        let temp_dir = tempdir().unwrap();
        let glb_path = temp_dir.path().join("textured.glb");
        fs::write(&glb_path, build_textured_triangle_glb()).unwrap();

        let mut state = ObjImporterState::new();
        state.load_glb_file(&glb_path).unwrap();

        assert!(
            state.status_message.contains("1 mesh(es)"),
            "expected '1 mesh(es)' in status: {}",
            state.status_message
        );
        assert!(
            state.status_message.contains("1 embedded image(s)"),
            "expected '1 embedded image(s)' in status: {}",
            state.status_message
        );
    }

    #[test]
    fn test_obj_importer_clear_resets_source_format_to_obj() {
        let temp_dir = tempdir().unwrap();
        let glb_path = temp_dir.path().join("model.glb");
        fs::write(&glb_path, build_minimal_triangle_glb()).unwrap();

        let mut state = ObjImporterState::new();
        state.load_glb_file(&glb_path).unwrap();
        assert_eq!(
            state.source_format,
            ImportSourceFormat::Glb,
            "source_format should be Glb after load_glb_file"
        );

        state.clear();

        assert_eq!(
            state.source_format,
            ImportSourceFormat::Obj,
            "source_format must reset to Obj after clear()"
        );
        assert_eq!(state.mode, ImporterMode::Idle);
    }

    #[test]
    fn test_importer_creature_export_stays_in_importer_by_default() {
        let state = ObjImporterState::default();
        assert!(
            !state.open_after_export,
            "open_after_export must default to false"
        );
    }

    #[test]
    fn test_open_after_export_preserved_across_clear() {
        let mut state = ObjImporterState {
            open_after_export: true,
            creature_name: "Test".to_string(),
            ..Default::default()
        };
        state.clear();
        assert!(
            state.open_after_export,
            "open_after_export must survive clear()"
        );
    }

    #[test]
    fn test_obj_importer_load_obj_still_works_after_glb_fields_added() {
        let temp_dir = tempdir().unwrap();
        let obj_path = temp_dir.path().join("triangle.obj");
        fs::write(
            &obj_path,
            concat!(
                "o Triangle\n",
                "v 0.0 0.0 0.0\n",
                "v 1.0 0.0 0.0\n",
                "v 0.0 1.0 0.0\n",
                "f 1 2 3\n"
            ),
        )
        .unwrap();

        let mut state = ObjImporterState::new();
        state.load_obj_file(&obj_path).unwrap();

        assert_eq!(state.mode, ImporterMode::Loaded);
        assert_eq!(
            state.source_format,
            ImportSourceFormat::Obj,
            "OBJ load must set source_format to Obj"
        );
        assert_eq!(state.meshes.len(), 1);
        assert_eq!(state.meshes[0].name, "Triangle");
        // texture_payload should be None for an OBJ with no material texture
        assert!(
            state.meshes[0].texture_payload.is_none(),
            "no MTL texture means texture_payload must be None"
        );
    }

    #[test]
    fn test_obj_importer_state_is_error_defaults_to_false() {
        let state = ObjImporterState::new();
        assert!(!state.is_error, "is_error should default to false");
    }

    #[test]
    fn test_obj_importer_state_is_error_resets_on_clear() {
        let mut state = ObjImporterState::new();
        state.is_error = true;
        state.clear();
        assert!(!state.is_error, "is_error should reset to false on clear");
    }

    #[test]
    fn test_obj_importer_state_load_glb_multi_scene_status() {
        use crate::mesh_glb_io::{ImportedGlbMesh, ImportedGlbScene};
        use antares::domain::visual::MeshDefinition;

        let scene = ImportedGlbScene {
            meshes: vec![ImportedGlbMesh {
                mesh_def: MeshDefinition {
                    name: Some("TestMesh".to_string()),
                    vertices: vec![],
                    indices: vec![],
                    normals: None,
                    uvs: None,
                    color: [1.0, 1.0, 1.0, 1.0],
                    lod_levels: None,
                    lod_distances: None,
                    material: None,
                    texture_path: None,
                },
                texture_payload: None,
                node_name: None,
                material_name: None,
                node_transform: None,
            }],
            embedded_image_count: 0,
            material_count: 0,
            scene_count: 3,
            has_skinning: false,
            has_animations: false,
            has_unsupported_pbr_channels: false,
        };

        let mut state = ObjImporterState::new();
        state.load_imported_glb_scene(None, scene);

        assert!(
            state.status_message.contains("GLB contains 3 scene(s)"),
            "status_message should mention multi-scene count, got: {}",
            state.status_message
        );
        assert!(
            state.status_message.contains("importing default scene"),
            "status_message should mention importing default scene, got: {}",
            state.status_message
        );
    }

    #[test]
    fn test_obj_importer_state_load_glb_skinning_status() {
        use crate::mesh_glb_io::{ImportedGlbMesh, ImportedGlbScene};
        use antares::domain::visual::MeshDefinition;

        let scene = ImportedGlbScene {
            meshes: vec![ImportedGlbMesh {
                mesh_def: MeshDefinition {
                    name: Some("SkinMesh".to_string()),
                    vertices: vec![],
                    indices: vec![],
                    normals: None,
                    uvs: None,
                    color: [1.0, 1.0, 1.0, 1.0],
                    lod_levels: None,
                    lod_distances: None,
                    material: None,
                    texture_path: None,
                },
                texture_payload: None,
                node_name: None,
                material_name: None,
                node_transform: None,
            }],
            embedded_image_count: 0,
            material_count: 0,
            scene_count: 1,
            has_skinning: true,
            has_animations: false,
            has_unsupported_pbr_channels: false,
        };

        let mut state = ObjImporterState::new();
        state.load_imported_glb_scene(None, scene);

        assert!(
            state
                .status_message
                .contains("Skinning/animations present but not imported"),
            "status_message should contain skinning warning, got: {}",
            state.status_message
        );
        assert!(
            !state.is_error,
            "is_error should be false for successful load"
        );
    }
}
