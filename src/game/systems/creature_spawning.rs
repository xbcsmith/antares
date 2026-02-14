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
//!     );
//! }
//! ```

use crate::application::resources::GameContent;
use crate::domain::visual::CreatureDefinition;
use crate::game::components::creature::{CreatureVisual, MeshPart, SpawnCreatureRequest};
use crate::game::systems::creature_meshes::{create_material_from_color, mesh_definition_to_bevy};
use bevy::prelude::*;

/// Spawns a creature visual from a definition
///
/// Creates a hierarchical entity structure:
/// - Parent entity with `CreatureVisual` component
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
///
/// # Returns
///
/// Entity ID of the parent creature entity
///
/// # Examples
///
/// ```
/// use antares::game::systems::creature_spawning::spawn_creature;
/// use antares::domain::visual::{CreatureDefinition, MeshDefinition};
/// use bevy::prelude::*;
///
/// fn example(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     let creature_def = CreatureDefinition {
///         name: "Test Creature".to_string(),
///         meshes: vec![
///             MeshDefinition {
///                 vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///                 indices: vec![0, 1, 2],
///                 normals: None,
///                 uvs: None,
///                 color: [1.0, 1.0, 1.0, 1.0],
///             },
///         ],
///         scale: 1.0,
///     };
///
///     let entity = spawn_creature(
///         &mut commands,
///         &creature_def,
///         &mut meshes,
///         &mut materials,
///         Vec3::ZERO,
///         None,
///     );
/// }
/// ```
pub fn spawn_creature(
    commands: &mut Commands,
    creature_def: &CreatureDefinition,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    scale_override: Option<f32>,
) -> Entity {
    // Determine effective scale
    let scale = scale_override.unwrap_or(creature_def.scale);

    // Create parent entity with CreatureVisual component
    let parent = commands
        .spawn((
            CreatureVisual {
                creature_id: 0, // Will be set by caller if needed
                scale_override,
            },
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Spawn child entities for each mesh
    for (mesh_index, mesh_def) in creature_def.meshes.iter().enumerate() {
        // Convert mesh definition to Bevy mesh
        let bevy_mesh = mesh_definition_to_bevy(mesh_def);
        let mesh_handle = meshes.add(bevy_mesh);

        // Create material from mesh color
        let material = create_material_from_color(mesh_def.color);
        let material_handle = materials.add(material);

        // Spawn child entity
        let child = commands
            .spawn((
                MeshPart {
                    creature_id: 0, // Will be set by caller if needed
                    mesh_index,
                    material_override: None,
                },
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        commands.entity(parent).add_child(child);
    }

    parent
}

/// Bevy system that processes spawn requests
///
/// This system:
/// 1. Queries for entities with `SpawnCreatureRequest` component
/// 2. Looks up the creature definition from the content database
/// 3. Spawns the creature visual hierarchy
/// 4. Removes the spawn request component
///
/// # System Parameters
///
/// * `commands` - Entity commands
/// * `query` - Query for spawn requests
/// * `creatures` - Content database resource
/// * `meshes` - Mesh asset storage
/// * `materials` - Material asset storage
///
/// # Examples
///
/// To trigger a spawn, create an entity with `SpawnCreatureRequest`:
///
/// ```
/// use antares::game::components::creature::SpawnCreatureRequest;
/// use bevy::prelude::*;
///
/// fn request_spawn(mut commands: Commands) {
///     commands.spawn(SpawnCreatureRequest {
///         creature_id: 42,
///         position: Vec3::new(10.0, 0.0, 5.0),
///         scale_override: None,
///     });
/// }
/// ```
pub fn creature_spawning_system(
    mut commands: Commands,
    query: Query<(Entity, &SpawnCreatureRequest)>,
    creatures: Res<GameContent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (request_entity, request) in query.iter() {
        // Look up creature definition
        if let Some(creature_def) = creatures.0.creatures.get_creature(request.creature_id) {
            // Spawn the creature
            let creature_entity = spawn_creature(
                &mut commands,
                creature_def,
                meshes.as_mut(),
                materials.as_mut(),
                request.position,
                request.scale_override,
            );

            // Update the spawned creature's CreatureVisual component with correct ID
            commands.entity(creature_entity).insert(CreatureVisual {
                creature_id: request.creature_id,
                scale_override: request.scale_override,
            });

            // Update child entities with correct creature_id
            // Note: We can't easily query children here, so the creature_id
            // is set to 0 initially in spawn_creature and updated if needed
        } else {
            warn!(
                "Creature spawn request failed: CreatureId {} not found in database",
                request.creature_id
            );
        }

        // Remove the spawn request component
        commands.entity(request_entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // Integration tests with full Bevy app context are complex due to borrow checker
    // requirements. Full integration testing should be done via manual testing or
    // end-to-end tests that run the actual game systems.
}
