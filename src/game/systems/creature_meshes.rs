// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Mesh generation system for converting domain MeshDefinitions to Bevy meshes
//!
//! This module provides pure functions to convert domain-level `MeshDefinition`
//! structures into Bevy's `Mesh` format for rendering. It handles vertex data,
//! normals, UVs, colors, and indices.
//!
//! # Features
//!
//! - Converts `MeshDefinition` to Bevy `Mesh`
//! - Auto-generates normals (flat or smooth) when not provided
//! - Creates materials from mesh colors and material definitions
//! - Texture loading and application
//! - Material conversion from domain MaterialDefinition to Bevy StandardMaterial
//! - Helper functions for normal calculation
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::creature_meshes::mesh_definition_to_bevy;
//! use antares::domain::visual::MeshDefinition;
//! use bevy::prelude::*;
//!
//! fn convert_mesh(mesh_def: &MeshDefinition) -> Mesh {
//!     mesh_definition_to_bevy(mesh_def)
//! }
//! ```

use crate::domain::visual::{AlphaMode as DomainAlphaMode, MaterialDefinition, MeshDefinition};
use crate::game::components::creature::{CreatureVisual, MeshPart, TextureLoaded};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Converts a domain MeshDefinition to a Bevy Mesh
///
/// This function performs the core conversion from the domain-level mesh
/// representation to Bevy's rendering format. It:
/// - Inserts vertex positions
/// - Generates or inserts normals
/// - Inserts UVs if provided
/// - Inserts vertex colors if provided
/// - Sets up triangle indices
///
/// # Arguments
///
/// * `mesh_def` - The domain mesh definition to convert
///
/// # Returns
///
/// A Bevy `Mesh` ready for rendering
///
/// # Panics
///
/// Panics if the mesh definition is invalid (should be validated before calling)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::mesh_definition_to_bevy;
/// use antares::domain::visual::MeshDefinition;
/// use bevy::prelude::*;
///
/// let mesh_def = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
/// };
///
/// let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
/// ```
pub fn mesh_definition_to_bevy(mesh_def: &MeshDefinition) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Insert vertex positions
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_def.vertices.clone());

    // Insert or generate normals
    let normals = if let Some(ref provided_normals) = mesh_def.normals {
        provided_normals.clone()
    } else {
        // Auto-generate flat normals
        calculate_flat_normals(&mesh_def.vertices, &mesh_def.indices)
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    // Insert UVs if provided
    if let Some(ref uvs) = mesh_def.uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());
    }

    // Insert vertex colors (replicate mesh color for all vertices)
    let vertex_colors: Vec<[f32; 4]> = vec![mesh_def.color; mesh_def.vertices.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);

    // Insert indices
    mesh.insert_indices(Indices::U32(mesh_def.indices.clone()));

    mesh
}

/// Calculates flat normals for a mesh
///
/// Flat normals give a faceted appearance where each triangle has a uniform normal.
/// The normal for each vertex of a triangle is the face normal of that triangle.
///
/// # Arguments
///
/// * `vertices` - Vertex positions
/// * `indices` - Triangle indices (must be divisible by 3)
///
/// # Returns
///
/// Vector of normals (one per vertex, matching vertex array length)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::calculate_flat_normals;
///
/// let vertices = vec![
///     [0.0, 0.0, 0.0],
///     [1.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0],
/// ];
/// let indices = vec![0, 1, 2];
///
/// let normals = calculate_flat_normals(&vertices, &indices);
/// assert_eq!(normals.len(), 3);
/// ```
pub fn calculate_flat_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 0.0, 0.0]; vertices.len()];

    // Process each triangle
    for triangle_indices in indices.chunks(3) {
        if triangle_indices.len() != 3 {
            continue;
        }

        let i0 = triangle_indices[0] as usize;
        let i1 = triangle_indices[1] as usize;
        let i2 = triangle_indices[2] as usize;

        let v0 = Vec3::from(vertices[i0]);
        let v1 = Vec3::from(vertices[i1]);
        let v2 = Vec3::from(vertices[i2]);

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        // Assign face normal to all three vertices of the triangle
        normals[i0] = face_normal.to_array();
        normals[i1] = face_normal.to_array();
        normals[i2] = face_normal.to_array();
    }

    normals
}

/// Calculates smooth normals for a mesh
///
/// Smooth normals give a rounded appearance by averaging normals of adjacent
/// triangles that share a vertex. This creates smooth shading across the surface.
///
/// # Arguments
///
/// * `vertices` - Vertex positions
/// * `indices` - Triangle indices (must be divisible by 3)
///
/// # Returns
///
/// Vector of normals (one per vertex, matching vertex array length)
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::calculate_smooth_normals;
///
/// let vertices = vec![
///     [0.0, 0.0, 0.0],
///     [1.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0],
///     [1.0, 1.0, 0.0],
/// ];
/// let indices = vec![0, 1, 2, 1, 3, 2];
///
/// let normals = calculate_smooth_normals(&vertices, &indices);
/// assert_eq!(normals.len(), 4);
/// ```
pub fn calculate_smooth_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; vertices.len()];

    // Accumulate face normals for each vertex
    for triangle_indices in indices.chunks(3) {
        if triangle_indices.len() != 3 {
            continue;
        }

        let i0 = triangle_indices[0] as usize;
        let i1 = triangle_indices[1] as usize;
        let i2 = triangle_indices[2] as usize;

        let v0 = Vec3::from(vertices[i0]);
        let v1 = Vec3::from(vertices[i1]);
        let v2 = Vec3::from(vertices[i2]);

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        // Accumulate face normal to each vertex
        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // Normalize accumulated normals
    normals
        .into_iter()
        .map(|n| n.normalize_or_zero().to_array())
        .collect()
}

/// Creates a StandardMaterial from a color array
///
/// Converts a `[f32; 4]` RGBA color to a Bevy `StandardMaterial`.
/// This is used to create materials for creature meshes.
///
/// # Arguments
///
/// * `color` - RGBA color values in range [0.0, 1.0]
///
/// # Returns
///
/// A `StandardMaterial` with the specified base color
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::create_material_from_color;
///
/// let red = [1.0, 0.0, 0.0, 1.0];
/// let material = create_material_from_color(red);
/// ```
pub fn create_material_from_color(color: [f32; 4]) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
        perceptual_roughness: 0.8,
        metallic: 0.0,
        reflectance: 0.3,
        ..Default::default()
    }
}

/// Converts a domain MaterialDefinition to Bevy StandardMaterial
///
/// This function converts the domain-level material representation
/// (used in RON files and the content database) to Bevy's rendering
/// material format with PBR parameters.
///
/// # Arguments
///
/// * `material_def` - The domain material definition to convert
///
/// # Returns
///
/// A Bevy `StandardMaterial` with PBR parameters applied
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::material_definition_to_bevy;
/// use antares::domain::visual::{MaterialDefinition, AlphaMode};
///
/// let material_def = MaterialDefinition {
///     base_color: [0.8, 0.8, 0.8, 1.0],
///     metallic: 1.0,
///     roughness: 0.2,
///     emissive: None,
///     alpha_mode: AlphaMode::Opaque,
/// };
///
/// let bevy_material = material_definition_to_bevy(&material_def);
/// ```
pub fn material_definition_to_bevy(material_def: &MaterialDefinition) -> StandardMaterial {
    let base_color = Color::srgba(
        material_def.base_color[0],
        material_def.base_color[1],
        material_def.base_color[2],
        material_def.base_color[3],
    );

    let emissive = if let Some(emissive_rgb) = material_def.emissive {
        LinearRgba::rgb(emissive_rgb[0], emissive_rgb[1], emissive_rgb[2])
    } else {
        LinearRgba::BLACK
    };

    let alpha_mode = match material_def.alpha_mode {
        DomainAlphaMode::Opaque => bevy::prelude::AlphaMode::Opaque,
        DomainAlphaMode::Blend => bevy::prelude::AlphaMode::Blend,
        DomainAlphaMode::Mask => bevy::prelude::AlphaMode::Mask(0.5),
    };

    StandardMaterial {
        base_color,
        metallic: material_def.metallic,
        perceptual_roughness: material_def.roughness,
        emissive,
        alpha_mode,
        ..Default::default()
    }
}

/// Creates a material with texture applied
///
/// Combines material properties with a texture for the base color.
/// If material_def is provided, uses those PBR parameters.
/// Otherwise, creates a default material with the texture.
///
/// # Arguments
///
/// * `texture` - Texture handle for base color
/// * `material_def` - Optional material definition for PBR parameters
///
/// # Returns
///
/// A `StandardMaterial` with texture and PBR parameters
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::create_material_with_texture;
/// use antares::domain::visual::MaterialDefinition;
/// use bevy::prelude::*;
///
/// fn apply_texture(
///     texture_handle: Handle<Image>,
///     material_def: Option<&MaterialDefinition>,
/// ) -> StandardMaterial {
///     create_material_with_texture(texture_handle, material_def)
/// }
/// ```
pub fn create_material_with_texture(
    texture: Handle<Image>,
    material_def: Option<&MaterialDefinition>,
) -> StandardMaterial {
    let mut material = if let Some(def) = material_def {
        material_definition_to_bevy(def)
    } else {
        StandardMaterial {
            perceptual_roughness: 0.8,
            metallic: 0.0,
            ..Default::default()
        }
    };

    material.base_color_texture = Some(texture);
    material
}

/// Loads a texture from path using the asset server
///
/// This is a helper function for loading textures referenced in
/// creature mesh definitions. Handles relative paths from the
/// campaign directory.
///
/// # Arguments
///
/// * `asset_server` - Bevy asset server resource
/// * `texture_path` - Relative path to texture file
///
/// # Returns
///
/// Handle to the loading/loaded texture
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_meshes::load_texture;
/// use bevy::prelude::*;
///
/// fn load_dragon_texture(asset_server: &AssetServer) -> Handle<Image> {
///     load_texture(asset_server, "textures/dragon_scales.png")
/// }
/// ```
pub fn load_texture(asset_server: &AssetServer, texture_path: &str) -> Handle<Image> {
    asset_server.load(texture_path.to_string())
}

/// System that loads textures for creatures that need them
///
/// This system:
/// 1. Queries creatures without `TextureLoaded` marker
/// 2. Checks if creature meshes have texture_path defined
/// 3. Loads textures using the asset server
/// 4. Applies textures to mesh materials
/// 5. Marks creature as `TextureLoaded` to prevent re-loading
///
/// # System Parameters
///
/// * `commands` - Entity commands
/// * `asset_server` - Asset server for loading textures
/// * `materials` - Material asset storage
/// * `query` - Query for creatures needing texture loading
/// * `content` - Game content database
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::creature_meshes::texture_loading_system;
///
/// fn build_app(app: &mut App) {
///     app.add_systems(Update, texture_loading_system);
/// }
/// ```
#[allow(clippy::type_complexity)]
pub fn texture_loading_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    creatures: Res<crate::application::resources::GameContent>,
    query: Query<
        (Entity, &CreatureVisual, &Children),
        (Without<TextureLoaded>, With<CreatureVisual>),
    >,
    mut mesh_parts: Query<(&MeshPart, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, creature_visual, children) in query.iter() {
        // Look up creature definition
        let Some(creature_def) = creatures
            .0
            .creatures
            .get_creature(creature_visual.creature_id)
        else {
            // Creature not found, mark as loaded to avoid repeated lookups
            commands.entity(entity).insert(TextureLoaded);
            continue;
        };

        let mut textures_loaded = true;

        // Process each child mesh part
        for child in children.iter() {
            if let Ok((mesh_part, mut material_handle)) = mesh_parts.get_mut(child) {
                // Get the mesh definition for this part
                if let Some(mesh_def) = creature_def.meshes.get(mesh_part.mesh_index) {
                    // Check if this mesh has a texture path
                    if let Some(ref texture_path) = mesh_def.texture_path {
                        // Load the texture
                        let texture_handle = load_texture(&asset_server, texture_path);

                        // Create new material with texture
                        let material = create_material_with_texture(
                            texture_handle,
                            mesh_def.material.as_ref(),
                        );

                        // Update the material handle
                        material_handle.0 = materials.add(material);
                    } else if let Some(ref material_def) = mesh_def.material {
                        // No texture, but has material definition - apply PBR properties
                        let material = material_definition_to_bevy(material_def);
                        material_handle.0 = materials.add(material);
                    }
                } else {
                    // Mesh index out of bounds
                    warn!(
                        "MeshPart has invalid mesh_index {} for creature {}",
                        mesh_part.mesh_index, creature_visual.creature_id
                    );
                    textures_loaded = false;
                }
            }
        }

        // Mark as loaded if all textures were processed
        if textures_loaded {
            commands.entity(entity).insert(TextureLoaded);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_definition_to_bevy_vertices() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);

        // Verify mesh has expected attributes
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        assert!(bevy_mesh.indices().is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_normals_auto() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None, // Should auto-generate
            uvs: None,
            color: [1.0, 0.0, 0.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_normals_provided() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_uvs() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]),
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_mesh_definition_to_bevy_color() {
        let mesh_def = MeshDefinition {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 0.0, 0.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let bevy_mesh = mesh_definition_to_bevy(&mesh_def);
        assert!(bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
    }

    #[test]
    fn test_calculate_flat_normals_triangle() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0, 1, 2];

        let normals = calculate_flat_normals(&vertices, &indices);

        assert_eq!(normals.len(), 3);
        // All vertices of the triangle should have the same normal
        assert_eq!(normals[0], normals[1]);
        assert_eq!(normals[1], normals[2]);
    }

    #[test]
    fn test_calculate_flat_normals_cube() {
        // Simple cube vertices (8 corners)
        let vertices = vec![
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ];

        // Front face indices
        let indices = vec![0, 1, 2];

        let normals = calculate_flat_normals(&vertices, &indices);
        assert_eq!(normals.len(), 8);
    }

    #[test]
    fn test_calculate_smooth_normals_triangle() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0, 1, 2];

        let normals = calculate_smooth_normals(&vertices, &indices);

        assert_eq!(normals.len(), 3);
        // For a single triangle, smooth normals should equal flat normals
        assert_eq!(normals[0], normals[1]);
        assert_eq!(normals[1], normals[2]);
    }

    #[test]
    fn test_calculate_smooth_normals_shared_vertex() {
        // Two triangles sharing vertices
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let indices = vec![
            0, 1, 2, // First triangle
            1, 3, 2, // Second triangle
        ];

        let normals = calculate_smooth_normals(&vertices, &indices);
        assert_eq!(normals.len(), 4);

        // Shared vertices (1 and 2) should have averaged normals
        // (different from pure flat normals)
    }

    #[test]
    fn test_create_material_from_color_red() {
        let color = [1.0, 0.0, 0.0, 1.0];
        let material = create_material_from_color(color);

        // Material should have red base color
        assert_eq!(material.base_color.to_srgba().red, 1.0);
        assert_eq!(material.base_color.to_srgba().green, 0.0);
        assert_eq!(material.base_color.to_srgba().blue, 0.0);
    }

    #[test]
    fn test_create_material_from_color_green() {
        let color = [0.0, 1.0, 0.0, 1.0];
        let material = create_material_from_color(color);

        assert_eq!(material.base_color.to_srgba().green, 1.0);
    }

    #[test]
    fn test_create_material_from_color_alpha() {
        let color = [1.0, 1.0, 1.0, 0.5];
        let material = create_material_from_color(color);

        assert_eq!(material.base_color.to_srgba().alpha, 0.5);
    }

    #[test]
    fn test_material_definition_to_bevy_basic() {
        let material_def = MaterialDefinition {
            base_color: [0.5, 0.5, 0.5, 1.0],
            metallic: 0.8,
            roughness: 0.3,
            emissive: None,
            alpha_mode: DomainAlphaMode::Opaque,
        };

        let bevy_material = material_definition_to_bevy(&material_def);

        assert_eq!(bevy_material.metallic, 0.8);
        assert_eq!(bevy_material.perceptual_roughness, 0.3);
    }

    #[test]
    fn test_material_definition_to_bevy_with_emissive() {
        let material_def = MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            emissive: Some([1.0, 0.5, 0.0]),
            alpha_mode: DomainAlphaMode::Opaque,
        };

        let bevy_material = material_definition_to_bevy(&material_def);

        assert_eq!(bevy_material.emissive.red, 1.0);
        assert_eq!(bevy_material.emissive.green, 0.5);
        assert_eq!(bevy_material.emissive.blue, 0.0);
    }

    #[test]
    fn test_material_definition_to_bevy_alpha_modes() {
        let opaque = MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: None,
            alpha_mode: DomainAlphaMode::Opaque,
        };

        let blend = MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 0.5],
            metallic: 0.0,
            roughness: 0.5,
            emissive: None,
            alpha_mode: DomainAlphaMode::Blend,
        };

        let mask = MaterialDefinition {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: None,
            alpha_mode: DomainAlphaMode::Mask,
        };

        assert!(matches!(
            material_definition_to_bevy(&opaque).alpha_mode,
            bevy::prelude::AlphaMode::Opaque
        ));
        assert!(matches!(
            material_definition_to_bevy(&blend).alpha_mode,
            bevy::prelude::AlphaMode::Blend
        ));
        assert!(matches!(
            material_definition_to_bevy(&mask).alpha_mode,
            bevy::prelude::AlphaMode::Mask(_)
        ));
    }

    #[test]
    fn test_material_definition_to_bevy_base_color() {
        let material_def = MaterialDefinition {
            base_color: [0.2, 0.4, 0.6, 0.8],
            metallic: 0.0,
            roughness: 0.5,
            emissive: None,
            alpha_mode: DomainAlphaMode::Opaque,
        };

        let bevy_material = material_definition_to_bevy(&material_def);
        let color = bevy_material.base_color.to_srgba();

        assert!((color.red - 0.2).abs() < 0.01);
        assert!((color.green - 0.4).abs() < 0.01);
        assert!((color.blue - 0.6).abs() < 0.01);
        assert!((color.alpha - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_flat_normals_empty_indices() {
        let vertices = vec![[0.0, 0.0, 0.0]];
        let indices = vec![];

        let normals = calculate_flat_normals(&vertices, &indices);
        assert_eq!(normals.len(), 1);
        assert_eq!(normals[0], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_smooth_normals_empty_indices() {
        let vertices = vec![[0.0, 0.0, 0.0]];
        let indices = vec![];

        let normals = calculate_smooth_normals(&vertices, &indices);
        assert_eq!(normals.len(), 1);
        assert_eq!(normals[0], [0.0, 0.0, 0.0]);
    }
}
