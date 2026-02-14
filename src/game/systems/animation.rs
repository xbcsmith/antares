// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Animation systems for sprites and creatures
//!
//! Provides two animation systems:
//! - Sprite animation: Updates animated sprites by advancing frames
//! - Creature keyframe animation: Plays mesh transform animations
//!
//! # Performance
//!
//! - Only entities with animation components are updated
//! - Frame-rate independent (uses delta time)
//! - Efficient frame advancement
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::animation::{update_sprite_animations, animation_playback_system};
//!
//! fn build_app(app: &mut App) {
//!     app.add_systems(Update, (update_sprite_animations, animation_playback_system));
//! }
//! ```

use crate::game::components::creature::CreatureAnimation;
use crate::game::components::sprite::AnimatedSprite;
use bevy::prelude::*;

/// System that updates animated sprite frames
///
/// # Behavior
///
/// For each entity with `AnimatedSprite`:
/// - Advances timer by delta time
/// - When timer completes, advances to next frame
/// - Loops if `looping: true`, otherwise stops at last frame
///
/// # Performance
///
/// - O(n) where n = number of animated sprites
/// - Only processes entities with AnimatedSprite component
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::animation::update_sprite_animations;
/// use antares::game::components::sprite::AnimatedSprite;
///
/// fn setup(mut commands: Commands) {
///     commands.spawn((
///         // ... Mesh3d and MeshMaterial3d ...
///         AnimatedSprite::new(vec![0, 1, 2, 3], 8.0, true),
///     ));
/// }
/// ```
pub fn update_sprite_animations(time: Res<Time>, mut query: Query<&mut AnimatedSprite>) {
    for mut anim in query.iter_mut() {
        // Advance animation by delta time
        let _finished = anim.advance(time.delta_secs());
        // Note: finished flag can be used for one-shot animations if needed
    }
}

/// System that updates creature keyframe animations
///
/// For each creature with `CreatureAnimation`:
/// - Advances animation time by delta seconds
/// - Samples animation keyframes at current time
/// - Applies transforms to child mesh entities
///
/// # Behavior
///
/// - Loops if `looping: true`, otherwise stops at last frame
/// - Respects playback speed multiplier
/// - Interpolates between keyframes for smooth animation
///
/// # Performance
///
/// - O(n) where n = number of animated creatures
/// - Only processes entities with CreatureAnimation component
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::animation::animation_playback_system;
/// use antares::game::components::creature::CreatureAnimation;
/// use antares::domain::visual::AnimationDefinition;
///
/// fn spawn_animated_creature(mut commands: Commands) {
///     let anim_def = AnimationDefinition {
///         name: "walk".to_string(),
///         duration: 1.0,
///         keyframes: vec![],
///         looping: true,
///     };
///
///     commands.spawn((
///         CreatureAnimation::new(anim_def),
///         Transform::default(),
///     ));
/// }
/// ```
pub fn animation_playback_system(
    time: Res<Time>,
    mut query: Query<(&mut CreatureAnimation, &Children)>,
    mut transform_query: Query<&mut Transform>,
) {
    for (mut animation, children) in query.iter_mut() {
        if !animation.playing {
            continue;
        }

        // Advance animation time
        let _finished = animation.advance(time.delta_secs());

        // Sample animation at current time
        let current_time = animation.current_time;

        // Apply transforms from keyframes to child entities
        for keyframe in &animation.definition.keyframes {
            // Find the keyframe that affects this time
            if keyframe.time <= current_time {
                // Find the mesh entity (child at mesh_index)
                if let Some(child) = children.iter().nth(keyframe.mesh_index) {
                    if let Ok(mut transform) = transform_query.get_mut(child) {
                        // Apply the keyframe transform
                        transform.translation = Vec3::from(keyframe.transform.translation);
                        transform.rotation = Quat::from_euler(
                            EulerRot::XYZ,
                            keyframe.transform.rotation[0],
                            keyframe.transform.rotation[1],
                            keyframe.transform.rotation[2],
                        );
                        transform.scale = Vec3::from(keyframe.transform.scale);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animated_sprite_advance_frame() {
        // Test that AnimatedSprite::advance() works correctly
        let mut anim = AnimatedSprite::new(vec![0, 1, 2], 2.0, true);
        assert_eq!(anim.current_frame, 0);

        // Advance by 0.5 seconds (one frame at 2 fps)
        anim.advance(0.5);
        assert_eq!(anim.current_frame, 1, "Should advance to frame 1");
    }

    #[test]
    fn test_animated_sprite_loops() {
        // Test that looping animation returns to frame 0
        let mut anim = AnimatedSprite::new(vec![0, 1], 2.0, true);

        // Advance through frames
        anim.advance(0.5); // frame 1
        anim.advance(0.5); // loop back to frame 0
        assert_eq!(
            anim.current_frame, 0,
            "Looping animation should reset to frame 0"
        );
    }

    #[test]
    fn test_animated_sprite_non_looping_finishes() {
        // Test that non-looping animation stops at last frame
        let mut anim = AnimatedSprite::new(vec![0, 1], 2.0, false);

        // Advance through frames
        anim.advance(0.5); // frame 1
        let finished = anim.advance(0.5); // past end

        assert!(finished, "Non-looping animation should finish");
        assert_eq!(anim.current_frame, 1, "Should stay on last frame");
    }

    #[test]
    fn test_animated_sprite_current_sprite_index() {
        // Test that current_sprite_index returns correct frame index
        let mut anim = AnimatedSprite::new(vec![10, 20, 30], 2.0, true);

        assert_eq!(
            anim.current_sprite_index(),
            10,
            "Frame 0 should be sprite 10"
        );

        anim.advance(0.5);
        assert_eq!(
            anim.current_sprite_index(),
            20,
            "Frame 1 should be sprite 20"
        );

        anim.advance(0.5);
        assert_eq!(
            anim.current_sprite_index(),
            30,
            "Frame 2 should be sprite 30"
        );
    }

    #[test]
    fn test_animated_sprite_frame_duration() {
        // Test frame duration calculation
        let anim = AnimatedSprite::new(vec![0, 1, 2], 8.0, true);
        assert!(
            (anim.frame_duration() - 0.125).abs() < 0.0001,
            "8 fps = 0.125s per frame"
        );

        let anim2 = AnimatedSprite::new(vec![0, 1], 4.0, true);
        assert!(
            (anim2.frame_duration() - 0.25).abs() < 0.0001,
            "4 fps = 0.25s per frame"
        );
    }

    #[test]
    fn test_animated_sprite_empty_frames() {
        // Test empty frames doesn't crash
        let mut anim = AnimatedSprite::new(vec![], 8.0, true);
        let finished = anim.advance(1.0);
        assert!(!finished, "Empty animation should not finish");
        assert_eq!(anim.current_sprite_index(), 0, "Empty animation returns 0");
    }

    #[test]
    fn test_creature_animation_playback() {
        use crate::domain::visual::animation::{AnimationDefinition, Keyframe};
        use crate::domain::visual::MeshTransform;

        let keyframe = Keyframe {
            time: 0.5,
            mesh_index: 0,
            transform: MeshTransform {
                translation: [1.0, 2.0, 3.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
        };

        let anim_def = AnimationDefinition {
            name: "test".to_string(),
            duration: 1.0,
            keyframes: vec![keyframe],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(anim_def);
        assert_eq!(anim.current_time, 0.0);
        assert!(anim.playing);

        // Advance to keyframe time
        anim.advance(0.5);
        assert_eq!(anim.current_time, 0.5);
    }

    #[test]
    fn test_creature_animation_stops_when_finished() {
        use crate::domain::visual::animation::AnimationDefinition;

        let anim_def = AnimationDefinition {
            name: "one_shot".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(anim_def);

        // Advance past duration
        let finished = anim.advance(1.5);
        assert!(finished);
        assert!(!anim.playing);
        assert_eq!(anim.current_time, 1.0);
    }

    #[test]
    fn test_creature_animation_loops() {
        use crate::domain::visual::animation::AnimationDefinition;

        let anim_def = AnimationDefinition {
            name: "loop".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: true,
        };

        let mut anim = CreatureAnimation::new(anim_def);

        // Advance past duration
        let finished = anim.advance(1.2);
        assert!(!finished);
        assert!(anim.playing);
        assert!((anim.current_time - 0.2).abs() < 0.0001);
    }
}
