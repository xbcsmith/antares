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
//! When a monster has a `creature_id`, the system:
//! 1. Looks up the `CreatureDefinition` from the game data resource
//! 2. Spawns the creature visual hierarchy
//! 3. Attaches a `MonsterMarker` to link the visual to the combat entity
//!
//! If no `creature_id` is present, a fallback representation is used (billboard/sprite).
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::monster_rendering::spawn_monster_with_visual;
//! use antares::game::resources::GameDataResource;
//! use antares::domain::combat::{Monster, LootTable};
//! use antares::domain::character::Stats;
//! use antares::domain::campaign_loader::GameData;
//! use bevy::prelude::*;
//!
//! fn spawn_example(
//!     mut commands: Commands,
//!     game_data: Res<GameDataResource>,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     let monster = Monster::new(
//!         1,
//!         "Goblin".to_string(),
//!         Stats::new(8, 6, 6, 8, 10, 8, 5),
//!         10,
//!         5,
//!         vec![],
//!         LootTable::new(5, 15, 0, 1, 25),
//!     );
//!     let entity = spawn_monster_with_visual(
//!         &mut commands,
//!         &monster,
//!         &game_data,
//!         &mut meshes,
//!         &mut materials,
//!         Vec3::new(5.0, 0.0, 10.0),
//!     );
//! }
//! ```

use crate::domain::combat::Monster;
use crate::game::components::creature::CreatureVisual;
use crate::game::resources::GameDataResource;
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
/// This function checks if the monster has a `creature_id`. If present, it spawns
/// the corresponding creature visual. If not, it spawns a fallback representation.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity creation
/// * `monster` - The monster to spawn a visual for
/// * `game_data` - Game data resource containing creature database
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
/// use antares::game::resources::GameDataResource;
/// use antares::domain::combat::{Monster, LootTable};
/// use antares::domain::character::Stats;
/// use antares::domain::campaign_loader::GameData;
/// use bevy::prelude::*;
///
/// fn example(
///     mut commands: Commands,
///     game_data: Res<GameDataResource>,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
/// ) {
///     let monster = Monster::new(
///         1,
///         "Goblin".to_string(),
///         Stats::new(8, 6, 6, 8, 10, 8, 5),
///         10,
///         5,
///         vec![],
///         LootTable::new(5, 15, 0, 1, 25),
///     );
///
///     let visual_entity = spawn_monster_with_visual(
///         &mut commands,
///         &monster,
///         &game_data,
///         &mut meshes,
///         &mut materials,
///         Vec3::new(10.0, 0.0, 5.0),
///     );
/// }
/// ```
pub fn spawn_monster_with_visual(
    commands: &mut Commands,
    monster: &Monster,
    game_data: &GameDataResource,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) -> Entity {
    if let Some(creature_id) = monster.creature_id {
        // Look up creature definition
        if let Some(creature_def) = game_data.get_creature(creature_id) {
            // Lift the spawn position so the creature's lowest vertex rests on
            // the floor rather than sinking through it.  The caller passes the
            // tile-centre position (Y = floor level); foot_ground_offset()
            // returns the additional Y needed based on the creature geometry.
            let grounded_position = bevy::math::Vec3::new(
                position.x,
                position.y + creature_def.foot_ground_offset(),
                position.z,
            );

            // Spawn creature visual
            let visual_entity = spawn_creature(
                commands,
                creature_def,
                meshes,
                materials,
                grounded_position,
                None,
                None,
                None, // facing: preserve existing behaviour (North default)
            );

            // Update CreatureVisual with correct ID
            commands.entity(visual_entity).insert(CreatureVisual {
                creature_id,
                scale_override: None,
            });

            visual_entity
        } else {
            // creature_id is invalid, spawn fallback
            warn!(
                "Monster '{}' has invalid creature_id {}, using fallback",
                monster.name, creature_id
            );
            spawn_fallback_visual(commands, monster, materials, meshes, position)
        }
    } else {
        // No creature_id, spawn fallback
        spawn_fallback_visual(commands, monster, materials, meshes, position)
    }
}

/// Returns the fallback visual color for a monster based on its might value.
///
/// Used to colour-code fallback billboard markers by difficulty tier.
///
/// | Might | Colour | Tier   |
/// |-------|--------|--------|
/// | 1–8   | green  | easy   |
/// | 9–15  | yellow | medium |
/// | 16–20 | orange | hard   |
/// | 21+   | purple | boss   |
///
/// # Examples
///
/// ```
/// use antares::game::systems::monster_rendering::fallback_monster_color;
/// use bevy::prelude::Color;
///
/// assert_eq!(fallback_monster_color(5),  Color::srgb(0.3, 0.8, 0.3));
/// assert_eq!(fallback_monster_color(12), Color::srgb(0.9, 0.7, 0.1));
/// assert_eq!(fallback_monster_color(18), Color::srgb(0.9, 0.3, 0.1));
/// assert_eq!(fallback_monster_color(25), Color::srgb(0.7, 0.1, 0.9));
/// ```
pub fn fallback_monster_color(might: u8) -> Color {
    match might {
        1..=8 => Color::srgb(0.3, 0.8, 0.3),
        9..=15 => Color::srgb(0.9, 0.7, 0.1),
        16..=20 => Color::srgb(0.9, 0.3, 0.1),
        _ => Color::srgb(0.7, 0.1, 0.9),
    }
}

/// Spawns an improved fallback visual for a monster without a creature definition.
///
/// Creates a vertically-oriented thin cuboid panel (billboard-style) with a
/// colour-coded material based on the monster's power level, plus a small sphere
/// "icon" at the top to distinguish it from world geometry.
///
/// Colour tiers (by `stats.might`):
/// - Green  (1–8):  easy monster
/// - Yellow (9–15): medium monster
/// - Orange (16–20): hard monster
/// - Purple (21+):  boss-tier monster
///
/// The material has an emissive component so fallback markers are visually
/// distinct from real creature meshes even in dim lighting.
///
/// # Arguments
///
/// * `commands`  - Bevy commands for entity creation
/// * `monster`   - The monster to create a fallback visual for
/// * `materials` - Material asset storage
/// * `meshes`    - Mesh asset storage
/// * `position`  - World position to spawn at
///
/// # Returns
///
/// Entity ID of the fallback visual parent
fn spawn_fallback_visual(
    commands: &mut Commands,
    monster: &Monster,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    position: Vec3,
) -> Entity {
    let base_color = fallback_monster_color(monster.stats.might.base);

    // Emissive values are 0.4× the base colour's sRGB components so the
    // billboard glows distinctly without blowing out.
    let emissive = match monster.stats.might.base {
        1..=8 => LinearRgba::new(0.12, 0.32, 0.12, 1.0),
        9..=15 => LinearRgba::new(0.36, 0.28, 0.04, 1.0),
        16..=20 => LinearRgba::new(0.36, 0.12, 0.04, 1.0),
        _ => LinearRgba::new(0.28, 0.04, 0.36, 1.0),
    };

    let panel_mat = materials.add(StandardMaterial {
        base_color,
        emissive,
        perceptual_roughness: 0.6,
        metallic: 0.0,
        double_sided: true,
        cull_mode: None,
        ..Default::default()
    });

    // Thin cuboid panel facing +Z (looks flat from the front).
    let panel_mesh = meshes.add(Cuboid::new(0.8, 1.4, 0.05));

    // Small sphere accent at the top to act as an "icon".
    let sphere_mesh = meshes.add(Sphere::new(0.15));
    let sphere_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(0.6, 0.6, 0.6, 1.0),
        ..Default::default()
    });

    // Parent entity positioned slightly above the floor so the panel base
    // sits at ground level.
    let parent = commands
        .spawn((
            Transform::from_translation(position + Vec3::new(0.0, 0.7, 0.0)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Panel child.
    let panel = commands
        .spawn((
            Mesh3d(panel_mesh),
            MeshMaterial3d(panel_mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Sphere child at the top of the panel.
    let sphere = commands
        .spawn((
            Mesh3d(sphere_mesh),
            MeshMaterial3d(sphere_mat),
            Transform::from_xyz(0.0, 0.85, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    commands.entity(parent).add_child(panel);
    commands.entity(parent).add_child(sphere);

    parent
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

    #[test]
    fn test_fallback_visual_color_easy_monster() {
        assert_eq!(fallback_monster_color(5), Color::srgb(0.3, 0.8, 0.3));
        assert_eq!(fallback_monster_color(12), Color::srgb(0.9, 0.7, 0.1));
        assert_eq!(fallback_monster_color(18), Color::srgb(0.9, 0.3, 0.1));
        assert_eq!(fallback_monster_color(25), Color::srgb(0.7, 0.1, 0.9));
    }

    #[test]
    fn test_fallback_color_boundary_values() {
        // Boundary between easy and medium
        assert_eq!(fallback_monster_color(8), Color::srgb(0.3, 0.8, 0.3));
        assert_eq!(fallback_monster_color(9), Color::srgb(0.9, 0.7, 0.1));
        // Boundary between medium and hard
        assert_eq!(fallback_monster_color(15), Color::srgb(0.9, 0.7, 0.1));
        assert_eq!(fallback_monster_color(16), Color::srgb(0.9, 0.3, 0.1));
        // Boundary between hard and boss
        assert_eq!(fallback_monster_color(20), Color::srgb(0.9, 0.3, 0.1));
        assert_eq!(fallback_monster_color(21), Color::srgb(0.7, 0.1, 0.9));
    }
}
