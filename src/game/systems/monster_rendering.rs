// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Monster rendering system for integrating combat monsters with creature visuals
//!
//! This module provides systems and functions to spawn visual representations
//! for monsters in combat. It links the domain-level `Monster` entities to
//! the procedurally-generated creature meshes.
//!
//! # Architecture
//!
//! When a monster has a `visual_id`, the system:
//! 1. Looks up the `CreatureDefinition` from the content database
//! 2. Spawns the creature visual hierarchy
//! 3. Attaches a `MonsterMarker` to link the visual to the combat entity
//!
//! If no `visual_id` is present, a fallback representation is used (billboard/sprite).
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::monster_rendering::spawn_monster_with_visual;
//! use antares::domain::combat::Monster;
//! use antares::domain::visual::CreatureDatabase;
//! use bevy::prelude::*;
//!
//! fn spawn_example(
//!     mut commands: Commands,
//!     monster: &Monster,
//!     creature_db: &CreatureDatabase,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     let entity = spawn_monster_with_visual(
//!         &mut commands,
//!         monster,
//!         creature_db,
//!         &mut meshes,
//!         &mut materials,
//!         Vec3::new(5.0, 0.0, 10.0),
//!     );
//! }
//! ```

use crate::domain::combat::Monster;
use crate::domain::CreatureDatabase;
use crate::game::components::creature::CreatureVisual;
use crate::game::systems::creature_spawning::spawn_creature;
use bevy::prelude::*;

/// Marker component linking a visual entity to a combat monster
///
/// This component is attached to the creature visual's parent entity
/// to establish the connection between the visual representation and
/// the game logic monster entity.
///
/// # Fields
///
/// * `monster_entity` - The entity ID of the monster in the combat system
///
/// # Examples
///
/// ```
/// use antares::game::systems::monster_rendering::MonsterMarker;
/// use bevy::prelude::*;
///
/// fn mark_monster_visual(mut commands: Commands, monster_entity: Entity) {
///     commands.spawn((
///         MonsterMarker { monster_entity },
///         Transform::default(),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterMarker {
    /// Entity ID of the associated monster in combat system
    pub monster_entity: Entity,
}

/// Spawns a visual representation for a monster
///
/// This function checks if the monster has a `visual_id`. If present, it spawns
/// the corresponding creature visual. If not, it spawns a fallback representation.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity creation
/// * `monster` - The monster to spawn a visual for
/// * `creature_db` - Database of creature definitions
/// * `meshes` - Mesh asset storage
/// * `materials` - Material asset storage
/// * `position` - World position to spawn at
///
/// # Returns
///
/// Entity ID of the spawned visual (either creature or fallback)
///
/// # Examples
///
/// ```
/// use antares::game::systems::monster_rendering::spawn_monster_with_visual;
/// use antares::domain::combat::Monster;
/// use antares::domain::visual::CreatureDatabase;
/// use bevy::prelude::*;
///
/// fn example(
///     mut commands: Commands,
///     creature_db: CreatureDatabase,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     let monster = Monster::new(
///         "Goblin".to_string(),
///         1,
///         10,
///         5,
///         5,
///         5,
///     );
///
///     let visual_entity = spawn_monster_with_visual(
///         &mut commands,
///         &monster,
///         &creature_db,
///         &mut meshes,
///         &mut materials,
///         Vec3::new(10.0, 0.0, 5.0),
///     );
/// }
/// ```
pub fn spawn_monster_with_visual(
    commands: &mut Commands,
    monster: &Monster,
    creature_db: &CreatureDatabase,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) -> Entity {
    if let Some(visual_id) = monster.visual_id {
        // Look up creature definition
        if let Some(creature_def) = creature_db.get_creature(visual_id) {
            // Spawn creature visual
            let visual_entity =
                spawn_creature(commands, creature_def, meshes, materials, position, None);

            // Update CreatureVisual with correct ID
            commands.entity(visual_entity).insert(CreatureVisual {
                creature_id: visual_id,
                scale_override: None,
            });

            visual_entity
        } else {
            // Visual ID is invalid, spawn fallback
            warn!(
                "Monster '{}' has invalid visual_id {}, using fallback",
                monster.name, visual_id
            );
            spawn_fallback_visual(commands, monster, materials, meshes, position)
        }
    } else {
        // No visual_id, spawn fallback
        spawn_fallback_visual(commands, monster, materials, meshes, position)
    }
}

/// Spawns a fallback visual representation for a monster
///
/// Used when a monster has no `visual_id` or the visual_id is invalid.
/// Creates a simple colored cube as a placeholder.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity creation
/// * `monster` - The monster to create a fallback visual for
/// * `materials` - Material asset storage
/// * `position` - World position to spawn at
///
/// # Returns
///
/// Entity ID of the fallback visual
fn spawn_fallback_visual(
    commands: &mut Commands,
    monster: &Monster,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    position: Vec3,
) -> Entity {
    // Create a simple colored material based on monster stats
    // Use stats.might as a proxy for level/danger
    let color = match monster.stats.might.base {
        1..=8 => Color::srgb(0.5, 0.5, 0.5),   // Gray for low-level
        9..=15 => Color::srgb(0.8, 0.6, 0.2),  // Orange for mid-level
        16..=20 => Color::srgb(0.8, 0.2, 0.2), // Red for high-level
        _ => Color::srgb(0.5, 0.2, 0.8),       // Purple for very high-level
    };

    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.8,
        metallic: 0.0,
        ..Default::default()
    });

    // Create a simple cube mesh
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Spawn a simple cube as placeholder
    commands
        .spawn((
            Mesh3d(cube_mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monster_marker_creation() {
        let marker = MonsterMarker {
            monster_entity: Entity::PLACEHOLDER,
        };
        assert_eq!(marker.monster_entity, Entity::PLACEHOLDER);
    }

    #[test]
    fn test_monster_marker_component_is_copy() {
        let marker1 = MonsterMarker {
            monster_entity: Entity::PLACEHOLDER,
        };
        let marker2 = marker1; // Copy
        assert_eq!(marker1.monster_entity, marker2.monster_entity);
    }

    // Integration tests with full Bevy app context are complex due to borrow checker
    // requirements. Full integration testing should be done via manual testing or
    // end-to-end tests that run the actual game systems.
}
