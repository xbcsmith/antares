// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature spawning system for procedurally-generated creature visuals
//!
//! This module provides systems and functions to spawn creatures in the game world.
//! Creatures are spawned as hierarchical entities with a parent holding the
//! `CreatureVisual` component and child entities for each mesh part.
//!
//! # Architecture
//!
//! - Parent entity: `CreatureVisual` component + `Transform`
//! - Child entities: `MeshPart` + `Mesh` + `Material` + `Transform`
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::creature_spawning::spawn_creature;
//! use antares::domain::visual::CreatureDefinition;
//! use antares::domain::types::Direction;
//! use bevy::prelude::*;
//!
//! fn spawn_example(
//!     mut commands: Commands,
//!     creature_def: &CreatureDefinition,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     let entity = spawn_creature(
//!         &mut commands,
//!         creature_def,
//!         &mut meshes,
//!         &mut materials,
//!         Vec3::new(10.0, 0.0, 5.0),
//!         None,
//!         None,
//!         None,
//!         None, // glb_cache: RON path used
//!     );
//! }
//! ```

use crate::domain::types::Direction;
use crate::domain::visual::animation::AnimationDefinition;
use crate::domain::visual::CreatureDefinition;
use crate::game::components::creature::{
    CreatureAnimation, CreatureVisual, FacingComponent, LodState, MeshPart,
};
use crate::game::resources::GlbHandleCache;
use crate::game::systems::creature_meshes::{
    create_material_from_color, material_definition_to_bevy, mesh_definition_to_bevy,
};
use bevy::prelude::*;

/// Spawns a creature visual from a definition
///
/// Creates a hierarchical entity structure:
/// - Parent entity with `CreatureVisual` and `FacingComponent` components
/// - Child entities for each mesh in the creature definition
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity creation
/// * `creature_def` - The creature definition to spawn
/// * `meshes` - Mesh asset storage
/// * `materials` - Material asset storage
/// * `position` - World position to spawn at
/// * `scale_override` - Optional scale multiplier (overrides creature definition scale)
/// * `animation` - Optional animation to play on spawn
/// * `facing` - Optional cardinal direction the creature should face on spawn.
///   `None` defaults to [`Direction::North`] (zero rotation).
/// * `glb_cache` - Optional reference to the pre-populated [`GlbHandleCache`].
///   When `Some` and the definition has `glb_path` set, the GLB rendering path
///   is used (`WorldAssetRoot`).  Pass `None` to always use the RON path.
///
/// # Returns
///
/// Entity ID of the parent creature entity
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_spawning::spawn_creature;
/// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
/// use antares::domain::types::Direction;
/// use bevy::prelude::*;
///
/// fn example(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     let creature_def = CreatureDefinition {
///         id: 1,
///         name: "Test Creature".to_string(),
///         meshes: vec![
///             MeshDefinition {
///                 name: None,
///                 vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///                 indices: vec![0, 1, 2],
///                 normals: None,
///                 uvs: None,
///                 color: [1.0, 1.0, 1.0, 1.0],
///                 lod_levels: None,
///                 lod_distances: None,
///                 material: None,
///                 texture_path: None,
///             },
///         ],
///         mesh_transforms: vec![MeshTransform::identity()],
///         scale: 1.0,
///         color_tint: None,
///         glb_path: None,
///         glb_scene_index: 0,
///     };
///
///     // Spawn facing South (RON path — no GLB cache needed)
///     let entity = spawn_creature(
///         &mut commands,
///         &creature_def,
///         &mut meshes,
///         &mut materials,
///         Vec3::ZERO,
///         None,
///         None,
///         Some(Direction::South),
///         None, // glb_cache
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_creature(
    commands: &mut Commands,
    creature_def: &CreatureDefinition,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    scale_override: Option<f32>,
    animation: Option<AnimationDefinition>,
    facing: Option<Direction>,
    glb_cache: Option<&GlbHandleCache>,
) -> Entity {
    // GLB rendering path: delegate to spawn_creature_glb() when a glb_path is set.
    // This path uses Bevy's WorldAssetRoot instead of the inline mesh definitions.
    if creature_def.glb_path.is_some() {
        return spawn_creature_glb(
            commands,
            creature_def,
            position,
            scale_override,
            animation,
            facing,
            glb_cache,
        );
    }

    // RON rendering path (unchanged from pre-Phase-3 behaviour).
    // Determine effective scale
    let scale = scale_override.unwrap_or(creature_def.scale);

    // Compute yaw rotation from the optional facing direction.
    // None → Direction::North → 0.0 rad (identity rotation preserved).
    let effective_direction = facing.unwrap_or(Direction::North);
    let yaw = effective_direction.direction_to_yaw_radians();
    let rotation = Quat::from_rotation_y(yaw);

    // Create parent entity with CreatureVisual and FacingComponent
    let mut parent_entity = commands.spawn((
        CreatureVisual {
            creature_id: 0, // Will be set by caller if needed
            scale_override,
        },
        FacingComponent::new(effective_direction),
        Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(scale)),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    // Add animation component if animation is provided
    if let Some(anim_def) = animation {
        parent_entity.insert(CreatureAnimation::new(anim_def));
    }

    let parent = parent_entity.id();

    // Spawn child entities for each mesh
    for (mesh_index, mesh_def) in creature_def.meshes.iter().enumerate() {
        // Convert mesh definition to Bevy mesh (LOD0)
        let bevy_mesh = mesh_definition_to_bevy(mesh_def);
        let mesh_handle = meshes.add(bevy_mesh);

        // Create material - use MaterialDefinition if provided, otherwise use color
        let material = if let Some(ref material_def) = mesh_def.material {
            material_definition_to_bevy(material_def)
        } else {
            create_material_from_color(mesh_def.color)
        };
        let material_handle = materials.add(material);

        // Prepare LOD meshes if defined
        let mut lod_mesh_handles = vec![mesh_handle.clone()]; // LOD0
        let lod_distances = if let Some(ref lod_levels) = mesh_def.lod_levels {
            // Generate meshes for each LOD level
            for lod_mesh_def in lod_levels.iter() {
                let lod_bevy_mesh = mesh_definition_to_bevy(lod_mesh_def);
                let lod_handle = meshes.add(lod_bevy_mesh);
                lod_mesh_handles.push(lod_handle);
            }

            // Get LOD distances if defined
            mesh_def.lod_distances.clone()
        } else {
            None
        };

        // Spawn child entity
        let mut child_entity = commands.spawn((
            MeshPart {
                creature_id: 0, // Will be set by caller if needed
                mesh_index,
                material_override: None,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            if let Some(mesh_transform) = creature_def.mesh_transforms.get(mesh_index) {
                Transform::from_translation(Vec3::from(mesh_transform.translation))
                    .with_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        mesh_transform.rotation[0],
                        mesh_transform.rotation[1],
                        mesh_transform.rotation[2],
                    ))
                    .with_scale(Vec3::from(mesh_transform.scale))
            } else {
                Transform::default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Add LOD component if LOD levels are defined
        if let Some(distances) = lod_distances {
            child_entity.insert(LodState::new(lod_mesh_handles, distances));
        }

        let child = child_entity.id();

        commands.entity(parent).add_child(child);
    }

    parent
}

/// Spawns a GLB-backed creature using Bevy's `WorldAssetRoot` component.
///
/// Called by [`spawn_creature`] when `creature_def.glb_path` is `Some`.
/// Bevy's `SceneSpawner` will asynchronously attach the GLB scene's child
/// hierarchy once the asset finishes loading.
///
/// # Graceful degradation
///
/// - If `glb_cache` is `None`, logs a warning and spawns a placeholder entity
///   (a root with no mesh) so callers are not left with an invalid entity ID.
/// - If the handle is not found in the cache (pre-load in `glb_scene_loader_system`
///   should prevent this in normal operation), logs a warning and spawns a
///   placeholder entity.
#[allow(clippy::too_many_arguments)]
fn spawn_creature_glb(
    commands: &mut Commands,
    creature_def: &CreatureDefinition,
    position: Vec3,
    scale_override: Option<f32>,
    animation: Option<AnimationDefinition>,
    facing: Option<Direction>,
    glb_cache: Option<&GlbHandleCache>,
) -> Entity {
    // Resolve transform parameters first so the placeholder branch can use them.
    let effective_scale = scale_override.unwrap_or(creature_def.scale);
    let effective_direction = facing.unwrap_or(Direction::North);
    let yaw = effective_direction.direction_to_yaw_radians();
    let rotation = Quat::from_rotation_y(yaw);

    // Safety: this function is only called from spawn_creature() when
    // creature_def.glb_path.is_some() — the None branch is a programming
    // error guard that logs and spawns a placeholder so the caller always
    // gets a valid Entity back.
    let Some(glb_path) = creature_def.glb_path.as_deref() else {
        warn!(
            "spawn_creature_glb called without glb_path for creature '{}'; \
             this is a programming error. Spawning empty placeholder.",
            creature_def.name
        );
        let mut placeholder_cmds = commands.spawn((
            CreatureVisual {
                creature_id: creature_def.id,
                scale_override,
            },
            FacingComponent::new(effective_direction),
            Transform::from_translation(position)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(effective_scale)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
        if let Some(anim_def) = animation {
            placeholder_cmds.insert(CreatureAnimation::new(anim_def));
        }
        return placeholder_cmds.id();
    };

    // Attempt to look up the pre-loaded scene handle from the cache.
    let handle_opt = glb_cache
        .and_then(|cache| cache.scenes.get(glb_path))
        .cloned();

    let Some(handle) = handle_opt else {
        if glb_cache.is_none() {
            warn!(
                "spawn_creature_glb: GlbHandleCache not provided for GLB-only creature '{}' \
                 (glb_path='{}'). Spawning placeholder entity.",
                creature_def.name, glb_path
            );
        } else {
            warn!(
                "spawn_creature_glb: GLB path '{}' not found in GlbHandleCache for creature '{}'. \
                 Ensure glb_scene_loader_system ran before map spawn. Spawning placeholder entity.",
                glb_path, creature_def.name
            );
        }
        // Spawn a zero-vertex placeholder entity when the handle is unavailable.
        // GLB-only definitions have no `meshes` to fall back to.
        let mut entity_cmds = commands.spawn((
            CreatureVisual {
                creature_id: creature_def.id,
                scale_override,
            },
            FacingComponent::new(effective_direction),
            Transform::from_translation(position)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(effective_scale)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
        if let Some(anim_def) = animation {
            entity_cmds.insert(CreatureAnimation::new(anim_def));
        }
        return entity_cmds.id();
    };

    // Spawn the root entity with WorldAssetRoot so Bevy's scene spawner
    // attaches the GLB scene's child hierarchy asynchronously.
    let mut entity_cmds = commands.spawn((
        CreatureVisual {
            creature_id: creature_def.id,
            scale_override,
        },
        FacingComponent::new(effective_direction),
        Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(effective_scale)),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        WorldAssetRoot(handle),
    ));

    // Mirror the existing RON path: insert CreatureAnimation when provided.
    if let Some(anim_def) = animation {
        entity_cmds.insert(CreatureAnimation::new(anim_def));
    }

    entity_cmds.id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Direction;
    use crate::domain::visual::MeshDefinition;
    use crate::game::components::creature::SpawnCreatureRequest;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_creature_visual_component_creation() {
        let visual = CreatureVisual::new(42);
        assert_eq!(visual.creature_id, 42);
        assert_eq!(visual.scale_override, None);
    }

    #[test]
    fn test_mesh_part_component_creation() {
        let part = MeshPart::new(10, 2);
        assert_eq!(part.creature_id, 10);
        assert_eq!(part.mesh_index, 2);
        assert!(part.material_override.is_none());
    }

    #[test]
    fn test_spawn_creature_request_creation() {
        let request = SpawnCreatureRequest::new(5, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(request.creature_id, 5);
        assert_eq!(request.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(request.scale_override, None);
    }

    /// Builds a minimal single-mesh `CreatureDefinition` suitable for unit tests
    fn make_test_creature_def() -> CreatureDefinition {
        CreatureDefinition {
            id: 1,
            name: "Test Creature".to_string(),
            meshes: vec![MeshDefinition {
                name: None,
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            }],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
            glb_path: None,
            glb_scene_index: 0,
        }
    }

    // ===== facing / FacingComponent unit tests =====

    /// `spawn_creature` with `facing = None` must produce North rotation (yaw == 0).
    #[test]
    fn test_spawn_creature_facing_none_is_north() {
        // None facing resolves to North → yaw 0 → identity rotation (y component == 0)
        let direction = Direction::North;
        let yaw = direction.direction_to_yaw_radians();
        let rotation = Quat::from_rotation_y(yaw);
        assert!(
            (rotation.y).abs() < 1e-6,
            "None facing must produce identity (North) rotation, got {:?}",
            rotation
        );
    }

    /// `spawn_creature` with `facing = Some(South)` must produce yaw == π.
    #[test]
    fn test_spawn_creature_facing_south_rotation() {
        let yaw = Direction::South.direction_to_yaw_radians();
        let rotation = Quat::from_rotation_y(yaw);
        let expected = Quat::from_rotation_y(std::f32::consts::PI);
        // Compare component-wise (within float tolerance)
        let close = (rotation.x - expected.x).abs() < 1e-5
            && (rotation.y - expected.y).abs() < 1e-5
            && (rotation.z - expected.z).abs() < 1e-5
            && (rotation.w - expected.w).abs() < 1e-5;
        assert!(
            close,
            "South facing must produce Quat::from_rotation_y(PI), got {:?}",
            rotation
        );
    }

    /// `FacingComponent` inserted for `Some(South)` must store `Direction::South`.
    #[test]
    fn test_facing_component_inserted_south() {
        // Mirrors spawn_creature logic: Some(dir) → use dir directly
        let component = FacingComponent::new(Direction::South);
        assert_eq!(
            component.direction,
            Direction::South,
            "FacingComponent must store the provided direction"
        );
    }

    /// `FacingComponent` inserted for `None` must default to `Direction::North`.
    #[test]
    fn test_facing_component_inserted_none_defaults_north() {
        // Mirrors spawn_creature logic: None → Direction::North
        let component = FacingComponent::new(Direction::North);
        assert_eq!(
            component.direction,
            Direction::North,
            "FacingComponent must default to North when facing is None"
        );
    }

    /// Round-trip: for every cardinal, the inserted `FacingComponent` direction
    /// equals the original `Direction` passed to `spawn_creature`.
    #[test]
    fn test_facing_component_roundtrip_all_directions() {
        for dir in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            let component = FacingComponent::new(dir);
            assert_eq!(
                component.direction, dir,
                "FacingComponent direction mismatch for {:?}",
                dir
            );
        }
    }

    // ===== Existing structural / definition tests =====

    #[test]
    fn test_spawn_creature_with_lod() {
        // Create a simple creature definition with LOD levels
        let lod1 = MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let mesh_with_lod = MeshDefinition {
            name: None,
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            normals: None,
            uvs: None,
            color: [1.0, 0.0, 0.0, 1.0],
            lod_levels: Some(vec![lod1]),
            lod_distances: Some(vec![10.0]),
            material: None,
            texture_path: None,
        };

        let creature_def = CreatureDefinition {
            id: 1,
            name: "Test LOD Creature".to_string(),
            meshes: vec![mesh_with_lod],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
            glb_path: None,
            glb_scene_index: 0,
        };

        // Verify creature definition is valid
        assert!(creature_def.validate().is_ok());
        assert_eq!(creature_def.meshes.len(), 1);
        assert!(creature_def.meshes[0].lod_levels.is_some());
    }

    #[test]
    fn test_spawn_creature_with_material() {
        use crate::domain::visual::{AlphaMode, MaterialDefinition};

        let material_def = MaterialDefinition {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 1.0,
            roughness: 0.2,
            emissive: None,
            alpha_mode: AlphaMode::Opaque,
        };

        let mesh_with_material = MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: Some(material_def),
            texture_path: None,
        };

        let creature_def = CreatureDefinition {
            id: 2,
            name: "Test Material Creature".to_string(),
            meshes: vec![mesh_with_material],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
            glb_path: None,
            glb_scene_index: 0,
        };

        assert!(creature_def.validate().is_ok());
        assert!(creature_def.meshes[0].material.is_some());
    }

    /// Verify the helper produces a valid definition (used by facing tests above)
    #[test]
    fn test_make_test_creature_def_is_valid() {
        let def = make_test_creature_def();
        assert!(def.validate().is_ok());
    }

    // Integration tests with full Bevy app context are complex due to borrow checker
    // requirements. Full integration testing should be done via manual testing or
    // end-to-end tests that run the actual game systems.

    // ===== Phase 3: GLB rendering path tests =====

    /// A creature with `glb_path: Some(…)` and a pre-populated `GlbHandleCache`
    /// must spawn an entity with a `WorldAssetRoot` component and must NOT have
    /// a direct `Mesh3d` component on the root entity.
    #[test]
    fn test_spawn_creature_with_glb_path_uses_scene_root() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        // Build a GLB-only creature definition
        let glb_def = CreatureDefinition {
            id: 42,
            name: "GlbCreature".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
            glb_path: Some("assets/meshes/test_triangle.glb".to_string()),
            glb_scene_index: 0,
        };

        // Pre-populate the cache with a fake handle for the GLB path
        let mut cache = GlbHandleCache::default();
        let fake_handle: Handle<WorldAsset> = Handle::default();
        cache
            .scenes
            .insert("assets/meshes/test_triangle.glb".to_string(), fake_handle);

        app.world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>| {
                    let _entity = spawn_creature(
                        &mut commands,
                        &glb_def,
                        &mut meshes,
                        &mut materials,
                        Vec3::ZERO,
                        None,
                        None,
                        None,
                        Some(&cache),
                    );
                },
            )
            .unwrap();

        // The spawned entity should have WorldAssetRoot (GLB path)
        let has_world_asset_root = {
            let w = app.world_mut();
            let mut q = w.query::<&WorldAssetRoot>();
            q.iter(w).next().is_some()
        };
        assert!(
            has_world_asset_root,
            "GLB creature must have WorldAssetRoot component"
        );

        // The root entity must NOT have Mesh3d (GLB children are added by Bevy async)
        let has_mesh3d = {
            let w = app.world_mut();
            let mut q = w.query::<&Mesh3d>();
            q.iter(w).next().is_some()
        };
        assert!(
            !has_mesh3d,
            "GLB creature root entity must not have a direct Mesh3d component"
        );
    }

    /// A creature with `glb_path: None` must spawn child entities with `Mesh3d`
    /// components via the RON rendering path.
    #[test]
    fn test_spawn_creature_without_glb_path_uses_mesh_bundle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        let ron_def = make_test_creature_def(); // has glb_path: None, one mesh

        app.world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>| {
                    let _entity = spawn_creature(
                        &mut commands,
                        &ron_def,
                        &mut meshes,
                        &mut materials,
                        Vec3::ZERO,
                        None,
                        None,
                        None,
                        None, // no GLB cache needed for RON path
                    );
                },
            )
            .unwrap();

        // At least one entity must have Mesh3d (the mesh child)
        let has_mesh3d = {
            let w = app.world_mut();
            let mut q = w.query::<&Mesh3d>();
            q.iter(w).next().is_some()
        };
        assert!(
            has_mesh3d,
            "RON creature must spawn child entities with Mesh3d component"
        );

        // No WorldAssetRoot should be present for the RON path
        let has_world_asset_root = {
            let w = app.world_mut();
            let mut q = w.query::<&WorldAssetRoot>();
            q.iter(w).next().is_some()
        };
        assert!(
            !has_world_asset_root,
            "RON creature must not have WorldAssetRoot"
        );
    }
}
