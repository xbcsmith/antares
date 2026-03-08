// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! State and persistence helpers for the OBJ importer workflow.
//!
//! This module owns the non-UI state needed by the Importer tab so later UI
//! phases can focus on interaction and rendering instead of file I/O and color
//! assignment mechanics.
//!
//! It is also the seam between the parser backend and the importer UI:
//! `mesh_obj_io.rs` returns domain `MeshDefinition` values, this module turns
//! them into editable importer rows plus campaign-scoped state, and
//! `obj_importer_ui.rs` renders and exports that state.
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
use crate::mesh_obj_io::{
    import_obj_scene_for_importer_from_obj_file_with_options, ImportedObjMaterialSwatch,
    ImportedObjMesh, ImportedObjMeshColorSource, ImportedObjMtlSourceKind, ImportedObjScene,
    ObjError, ObjImportOptions,
};
use antares::domain::types::CreatureId;
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

/// Temporary imported-material swatch surfaced in the importer UI for the current session.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMaterialSwatch {
    /// Material name used as the swatch label.
    pub label: String,
    /// Imported RGBA color derived from MTL diffuse and dissolve values.
    pub color: [f32; 4],
    /// Optional texture metadata preserved from `map_Kd` when portable.
    pub texture_path: Option<String>,
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
    /// Name entered by the user for the export.
    pub creature_name: String,
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

        Self::from_mesh_definition_with_color_source(imported_mesh.mesh_def, color_source)
    }

    fn from_mesh_definition_with_color_source(
        mut mesh_def: MeshDefinition,
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
}

impl ImportedMaterialSwatch {
    fn from_imported_obj_swatch(swatch: ImportedObjMaterialSwatch) -> Self {
        Self {
            label: swatch.label,
            color: swatch.color,
            texture_path: swatch.texture_path,
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
            declared_mtl_libraries: Vec::new(),
            resolved_mtl_paths: Vec::new(),
            imported_material_palette: Vec::new(),
            meshes: Vec::new(),
            export_type: ExportType::Creature,
            creature_id: 4000,
            creature_name: String::new(),
            scale: 0.01,
            status_message: String::new(),
            custom_palette: CustomPalette::default(),
            active_mesh_index: None,
            new_custom_color_label: String::new(),
            new_custom_color: [0.8, 0.8, 0.8, 1.0],
        }
    }
}

impl ObjImporterState {
    /// Creates a fresh importer state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears any loaded OBJ data and returns the importer to idle mode.
    pub fn clear(&mut self) {
        let scale = self.scale;
        let custom_palette = self.custom_palette.clone();
        let creature_id = self.creature_id;
        let export_type = self.export_type;
        let new_custom_color = self.new_custom_color;
        let manual_mtl_path = self.manual_mtl_path.clone();

        *self = Self {
            scale,
            custom_palette,
            creature_id,
            export_type,
            new_custom_color,
            manual_mtl_path,
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
        ExportType, ImportedMesh, ImportedMeshColorSource, ImportedMtlSourceKind, ImporterMode,
        ObjImporterState,
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
        state.manual_mtl_path = Some(PathBuf::from("materials/hero_override.mtl"));
        state.new_custom_color = [0.2, 0.4, 0.6, 1.0];
        state.add_custom_color("favorite_teal", [0.1, 0.7, 0.7, 1.0]);
        state.load_mesh_definitions(None, vec![named_triangle("EM3D_Base_Body")]);

        state.clear();

        assert_eq!(state.mode, ImporterMode::Idle);
        assert_eq!(state.scale, 0.05);
        assert_eq!(state.creature_id, 4012);
        assert_eq!(
            state.manual_mtl_path,
            Some(PathBuf::from("materials/hero_override.mtl"))
        );
        assert_eq!(state.new_custom_color, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(state.custom_palette.colors.len(), 1);
        assert!(state.meshes.is_empty());
        assert!(state.imported_material_palette.is_empty());
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
}
