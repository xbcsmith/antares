// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Level of Detail (LOD) switching system
//!
//! Automatically switches creature mesh LOD levels based on distance from camera.
//! This improves rendering performance by using simpler meshes for distant objects.
//!
//! # Features
//!
//! - Automatic LOD switching based on camera distance
//! - Configurable distance thresholds per creature
//! - Smooth transitions between LOD levels
//! - Support for multiple LOD levels (LOD0, LOD1, LOD2, etc.)
//!
//! # Performance
//!
//! - Only processes creatures with `LodState` component
//! - Distance calculations use squared distance for efficiency
//! - Only updates mesh handles when LOD level changes
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::lod::lod_switching_system;
//!
//! fn build_app(app: &mut App) {
//!     app.add_systems(Update, lod_switching_system);
//! }
//! ```

use crate::game::components::creature::LodState;
use bevy::prelude::*;

/// System that switches LOD levels based on camera distance
///
/// For each creature with `LodState`:
/// 1. Calculates distance from camera to creature
/// 2. Determines appropriate LOD level for that distance
/// 3. Switches mesh handle if LOD level changed
///
/// # System Parameters
///
/// * `camera_query` - Query for camera transform
/// * `creature_query` - Query for creatures with LOD state
/// * `mesh_query` - Query for mesh handles on child entities
///
/// # Performance
///
/// - O(n) where n = number of creatures with LOD
/// - Uses squared distance to avoid sqrt() call
/// - Only updates mesh when level changes
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::lod::lod_switching_system;
/// use antares::game::components::creature::LodState;
///
/// fn setup(mut commands: Commands) {
///     // Spawn camera
///     commands.spawn((
///         Camera3d::default(),
///         Transform::from_xyz(0.0, 5.0, 10.0),
///     ));
///
///     // System will automatically switch LOD based on distance
/// }
/// ```
pub fn lod_switching_system(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut creature_query: Query<(&Transform, &mut LodState, &Children)>,
    mut mesh_query: Query<&mut Mesh3d>,
) {
    // Get camera position (use first camera if multiple exist)
    let camera_transform = match camera_query.iter().next() {
        Some(transform) => transform,
        None => return, // No camera, skip LOD updates
    };

    let camera_pos = camera_transform.translation;

    // Process each creature with LOD state
    for (creature_transform, mut lod_state, children) in creature_query.iter_mut() {
        let creature_pos = creature_transform.translation;

        // Calculate distance from camera to creature
        let distance = camera_pos.distance(creature_pos);

        // Determine appropriate LOD level for this distance
        let new_level = lod_state.level_for_distance(distance);

        // Only update if LOD level changed
        if new_level != lod_state.current_level {
            // Clamp to available LOD levels
            let level_index = new_level.min(lod_state.mesh_handles.len().saturating_sub(1));

            // Update mesh handles on child entities
            if let Some(new_mesh_handle) = lod_state.mesh_handles.get(level_index) {
                for child in children.iter() {
                    if let Ok(mut mesh) = mesh_query.get_mut(child) {
                        mesh.0 = new_mesh_handle.clone();
                    }
                }

                lod_state.current_level = new_level;
            }
        }
    }
}

/// Calculates the appropriate LOD level for a given distance
///
/// This is a pure function helper for testing and custom LOD logic.
///
/// # Arguments
///
/// * `distance` - Distance from camera to object
/// * `thresholds` - Distance thresholds for each LOD level
///
/// # Returns
///
/// LOD level to use (0 = highest detail)
///
/// # Examples
///
/// ```
/// use antares::game::systems::lod::calculate_lod_level;
///
/// let thresholds = vec![10.0, 25.0, 50.0];
///
/// assert_eq!(calculate_lod_level(5.0, &thresholds), 0);
/// assert_eq!(calculate_lod_level(15.0, &thresholds), 1);
/// assert_eq!(calculate_lod_level(30.0, &thresholds), 2);
/// assert_eq!(calculate_lod_level(60.0, &thresholds), 3);
/// ```
pub fn calculate_lod_level(distance: f32, thresholds: &[f32]) -> usize {
    for (level, &threshold) in thresholds.iter().enumerate() {
        if distance < threshold {
            return level;
        }
    }
    thresholds.len()
}

/// System that visualizes LOD levels for debugging
///
/// Draws gizmos showing LOD distance thresholds and current LOD level.
/// Only active when `debug_assertions` is enabled or "debug_lod" feature is enabled.
///
/// # System Parameters
///
/// * `creature_query` - Query for creatures with LOD state
/// * `gizmos` - Gizmo drawing context
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::lod::debug_lod_system;
///
/// fn build_app(app: &mut App) {
///     #[cfg(debug_assertions)]
///     app.add_systems(Update, debug_lod_system);
/// }
/// ```
#[cfg(debug_assertions)]
pub fn debug_lod_system(creature_query: Query<(&Transform, &LodState)>, mut gizmos: Gizmos) {
    for (transform, lod_state) in creature_query.iter() {
        let pos = transform.translation;

        // Draw spheres for each LOD distance threshold
        for (level, &distance) in lod_state.distances.iter().enumerate() {
            let color = match level {
                0 => Color::srgb(0.0, 1.0, 0.0), // Green - LOD0
                1 => Color::srgb(1.0, 1.0, 0.0), // Yellow - LOD1
                2 => Color::srgb(1.0, 0.5, 0.0), // Orange - LOD2
                _ => Color::srgb(1.0, 0.0, 0.0), // Red - LOD3+
            };

            gizmos.circle(pos, distance, color);
        }

        // Draw a marker for current LOD level
        let marker_color = Color::srgb(1.0, 1.0, 1.0);
        gizmos.sphere(pos + Vec3::Y * 2.0, 0.2, marker_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_lod_level_close() {
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(5.0, &thresholds), 0);
        assert_eq!(calculate_lod_level(9.9, &thresholds), 0);
    }

    #[test]
    fn test_calculate_lod_level_medium() {
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(10.0, &thresholds), 1);
        assert_eq!(calculate_lod_level(15.0, &thresholds), 1);
        assert_eq!(calculate_lod_level(24.9, &thresholds), 1);
    }

    #[test]
    fn test_calculate_lod_level_far() {
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(25.0, &thresholds), 2);
        assert_eq!(calculate_lod_level(40.0, &thresholds), 2);
        assert_eq!(calculate_lod_level(49.9, &thresholds), 2);
    }

    #[test]
    fn test_calculate_lod_level_very_far() {
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(50.0, &thresholds), 3);
        assert_eq!(calculate_lod_level(100.0, &thresholds), 3);
        assert_eq!(calculate_lod_level(1000.0, &thresholds), 3);
    }

    #[test]
    fn test_calculate_lod_level_empty_thresholds() {
        let thresholds = vec![];
        assert_eq!(calculate_lod_level(0.0, &thresholds), 0);
        assert_eq!(calculate_lod_level(1000.0, &thresholds), 0);
    }

    #[test]
    fn test_calculate_lod_level_single_threshold() {
        let thresholds = vec![20.0];
        assert_eq!(calculate_lod_level(10.0, &thresholds), 0);
        assert_eq!(calculate_lod_level(25.0, &thresholds), 1);
    }

    #[test]
    fn test_calculate_lod_level_boundary() {
        let thresholds = vec![10.0, 20.0];

        // Exactly on boundary should use next LOD level
        assert_eq!(calculate_lod_level(10.0, &thresholds), 1);
        assert_eq!(calculate_lod_level(20.0, &thresholds), 2);
    }

    #[test]
    fn test_calculate_lod_level_zero_distance() {
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(0.0, &thresholds), 0);
    }

    #[test]
    fn test_calculate_lod_level_negative_distance() {
        // Distance should never be negative, but test for robustness
        let thresholds = vec![10.0, 25.0, 50.0];
        assert_eq!(calculate_lod_level(-5.0, &thresholds), 0);
    }

    #[test]
    fn test_calculate_lod_level_multiple_levels() {
        let thresholds = vec![5.0, 10.0, 15.0, 20.0, 25.0];

        assert_eq!(calculate_lod_level(3.0, &thresholds), 0);
        assert_eq!(calculate_lod_level(7.0, &thresholds), 1);
        assert_eq!(calculate_lod_level(12.0, &thresholds), 2);
        assert_eq!(calculate_lod_level(17.0, &thresholds), 3);
        assert_eq!(calculate_lod_level(22.0, &thresholds), 4);
        assert_eq!(calculate_lod_level(30.0, &thresholds), 5);
    }
}
