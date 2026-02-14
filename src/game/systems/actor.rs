// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Actor sprite spawning and management
//!
//! Handles spawning of NPCs, Monsters, and Recruitable characters with sprite visuals.
//!
//! # Components
//!
//! - `ActorSprite` - Component marking an entity as an actor with sprite
//! - `Billboard` - Component making actor face camera (Y-axis locked)
//! - `AnimatedSprite` - Optional animation for actor sprites
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::actor::spawn_actor_sprite;
//! use antares::game::components::sprite::ActorType;
//! use antares::domain::world::SpriteReference;
//! use antares::game::resources::sprite_assets::SpriteAssets;
//!
//! fn spawn_npc(
//!     mut commands: Commands,
//!     mut sprite_assets: ResMut<SpriteAssets>,
//!     asset_server: Res<AssetServer>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     mut meshes: ResMut<Assets<Mesh>>,
//! ) {
//!     let sprite_ref = SpriteReference {
//!         sheet_path: "sprites/npcs_town.png".to_string(),
//!         sprite_index: 3,
//!         animation: None,
//!         material_properties: None,
//!     };
//!     spawn_actor_sprite(
//!         &mut commands,
//!         &mut sprite_assets,
//!         &asset_server,
//!         &mut materials,
//!         &mut meshes,
//!         &sprite_ref,
//!         Vec3::new(10.0, 0.5, 10.0),
//!         ActorType::Npc,
//!     );
//! }
//! ```

use crate::domain::world::SpriteReference;
use crate::game::components::billboard::Billboard;
use crate::game::components::sprite::{ActorSprite, ActorType, AnimatedSprite};
use crate::game::resources::sprite_assets::SpriteAssets;
use bevy::prelude::*;

/// Spawns an actor (NPC/Monster/Recruitable) with sprite visual and billboard
///
/// # Arguments
///
/// * `commands` - Bevy command buffer
/// * `sprite_assets` - Sprite asset registry (mutable for caching)
/// * `asset_server` - Asset server for loading textures
/// * `materials` - Material asset storage (mutable)
/// * `meshes` - Mesh asset storage (mutable)
/// * `sprite_ref` - Sprite reference with sheet path and index
/// * `position` - World position for the actor
/// * `actor_type` - Type of actor (NPC, Monster, Recruitable)
///
/// # Returns
///
/// Entity ID of spawned actor
///
/// # Behavior
///
/// - Loads sprite texture from `sprite_ref.sheet_path`
/// - Creates entity with Mesh and StandardMaterial components
/// - Attaches `ActorSprite` component with sheet path and index
/// - Attaches `Billboard` component (Y-locked, faces camera)
/// - If animation specified, attaches `AnimatedSprite` component
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::actor::spawn_actor_sprite;
/// use antares::game::components::sprite::ActorType;
/// use antares::domain::world::SpriteReference;
/// use antares::game::resources::sprite_assets::SpriteAssets;
///
/// fn spawn_npc(
///     mut commands: Commands,
///     mut sprite_assets: ResMut<SpriteAssets>,
///     asset_server: Res<AssetServer>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     mut meshes: ResMut<Assets<Mesh>>,
/// ) {
///     let sprite_ref = SpriteReference {
///         sheet_path: "sprites/npcs_town.png".to_string(),
///         sprite_index: 3,
///         animation: None,
///         material_properties: None,
///     };
///     spawn_actor_sprite(
///         &mut commands,
///         &mut sprite_assets,
///         &asset_server,
///         &mut materials,
///         &mut meshes,
///         &sprite_ref,
///         Vec3::new(10.0, 0.5, 10.0),
///         ActorType::Npc,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_actor_sprite(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    sprite_ref: &SpriteReference,
    position: Vec3,
    actor_type: ActorType,
) -> Entity {
    // Get or load material for sprite sheet (caches per sheet path)
    let material = sprite_assets.get_or_load_material(
        &sprite_ref.sheet_path,
        asset_server,
        materials,
        sprite_ref.material_properties.as_ref(),
    );

    // Get or load mesh for actor sprites (1.0 x 2.0 tall quad)
    let mesh = sprite_assets.get_or_load_mesh((1.0, 2.0), meshes);

    // Spawn actor with individual components
    let mut entity_commands = commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        GlobalTransform::default(),
        ActorSprite {
            sheet_path: sprite_ref.sheet_path.clone(),
            sprite_index: sprite_ref.sprite_index,
            actor_type,
        },
        Billboard { lock_y: true }, // Actors stay upright and face camera
    ));

    // Add animation if specified
    if let Some(anim) = &sprite_ref.animation {
        entity_commands.insert(AnimatedSprite::new(
            anim.frames.clone(),
            anim.fps,
            anim.looping,
        ));
    }

    entity_commands.id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_sprite_component() {
        let sprite = ActorSprite {
            sheet_path: "sprites/npcs.png".to_string(),
            sprite_index: 2,
            actor_type: ActorType::Npc,
        };
        assert_eq!(sprite.sheet_path, "sprites/npcs.png");
        assert_eq!(sprite.sprite_index, 2);
        assert_eq!(sprite.actor_type, ActorType::Npc);
    }

    #[test]
    fn test_actor_type_npc() {
        let actor_type = ActorType::Npc;
        assert_eq!(actor_type, ActorType::Npc);
    }

    #[test]
    fn test_actor_type_monster() {
        let actor_type = ActorType::Monster;
        assert_eq!(actor_type, ActorType::Monster);
    }

    #[test]
    fn test_actor_type_recruitable() {
        let actor_type = ActorType::Recruitable;
        assert_eq!(actor_type, ActorType::Recruitable);
    }

    #[test]
    fn test_billboard_component_for_actors() {
        let billboard = Billboard { lock_y: true };
        assert!(billboard.lock_y, "Actors should have Y-axis locked");
    }

    #[test]
    fn test_animated_sprite_creation_for_actors() {
        let anim = AnimatedSprite::new(vec![0, 1, 2, 3], 8.0, true);
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
        assert_eq!(anim.current_frame, 0);
    }
}
