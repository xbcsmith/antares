// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Billboard update system for camera-facing sprites
//!
//! This module provides the system that updates all billboard entities
//! to face the active camera each frame.
//!
//! # Performance
//!
//! - Only entities with `Billboard` component are processed
//! - Y-locked billboards use optimized rotation calculation
//! - Skips update if no camera exists (early return)
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::billboard::update_billboards;
//!
//! fn build_app(app: &mut App) {
//!     app.add_systems(Update, update_billboards);
//! }
//! ```

use crate::game::components::billboard::Billboard;
use bevy::prelude::*;

/// Plugin that manages billboard entities (always face camera)
pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_billboards);
    }
}

/// System that updates billboard entities to face the camera
///
/// # Behavior
///
/// For each entity with `Billboard` component:
/// - If `lock_y: true` - Rotates only around Y-axis (stays upright)
/// - If `lock_y: false` - Rotates to fully face camera (all axes)
///
/// # Performance
///
/// - Early return if no camera found (no processing)
/// - Efficient Y-axis-only rotation for upright billboards
/// - Full look-at rotation for non-locked billboards
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::billboard::update_billboards;
/// use antares::game::components::billboard::Billboard;
///
/// fn setup(mut commands: Commands) {
///     // Spawn billboard entity
///     commands.spawn((
///         Transform::from_xyz(5.0, 1.0, 5.0),
///         Billboard { lock_y: true },
///     ));
/// }
///
/// fn build_app(app: &mut App) {
///     app.add_systems(Startup, setup)
///        .add_systems(Update, update_billboards);
/// }
/// ```
pub fn update_billboards(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut billboard_query: Query<(&mut Transform, &Billboard), Without<Camera3d>>,
) {
    // Get camera transform
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation;

    for (mut transform, billboard) in billboard_query.iter_mut() {
        let entity_pos = transform.translation;
        let direction = camera_pos - entity_pos;

        if billboard.lock_y {
            // Y-axis locked: Only rotate around Y to face camera (characters stay upright)
            let angle = direction.x.atan2(direction.z);
            transform.rotation = Quat::from_rotation_y(angle + std::f32::consts::PI);
        } else {
            // Full rotation: Billboard always faces camera (particles, effects)
            transform.look_at(camera_pos, Vec3::Y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billboard_system_no_camera() {
        // Setup: No camera, one billboard
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        let entity = app
            .world_mut()
            .spawn((Transform::default(), Billboard { lock_y: true }))
            .id();

        // Update should not crash with no camera
        app.update();

        // Verify entity still exists
        assert!(app.world().get_entity(entity).is_ok());
    }

    #[test]
    fn test_billboard_lock_y_true() {
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        // Spawn camera at (10, 5, 10)
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(10.0, 5.0, 10.0)));

        // Spawn billboard at origin with Y-lock
        let billboard = app
            .world_mut()
            .spawn((Transform::default(), Billboard { lock_y: true }))
            .id();

        app.update();

        // Verify billboard rotation (only Y-axis should change)
        let transform = app.world().get::<Transform>(billboard).unwrap();

        // Should have some rotation applied
        assert_ne!(
            transform.rotation,
            Quat::IDENTITY,
            "Billboard should be rotated to face camera"
        );
    }

    #[test]
    fn test_billboard_lock_y_false_full_rotation() {
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        // Spawn camera above and in front
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

        // Spawn billboard with full rotation
        let billboard = app
            .world_mut()
            .spawn((Transform::default(), Billboard { lock_y: false }))
            .id();

        app.update();

        // Verify billboard has been rotated
        let transform = app.world().get::<Transform>(billboard).unwrap();
        // Should not be identity (default) rotation anymore
        assert_ne!(transform.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_billboard_multiple_billboards() {
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        // Spawn camera
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)));

        // Spawn multiple billboards
        let billboard1 = app
            .world_mut()
            .spawn((
                Transform::from_xyz(5.0, 0.0, 0.0),
                Billboard { lock_y: true },
            ))
            .id();

        let billboard2 = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, 5.0),
                Billboard { lock_y: false },
            ))
            .id();

        app.update();

        // Both billboards should have rotations
        let t1 = app.world().get::<Transform>(billboard1).unwrap();
        let t2 = app.world().get::<Transform>(billboard2).unwrap();

        // At least one should have rotation (most likely both)
        assert!(t1.rotation != Quat::IDENTITY || t2.rotation != Quat::IDENTITY);
    }
}
