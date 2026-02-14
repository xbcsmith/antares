// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance optimization systems for Bevy
//!
//! This module provides systems for:
//! - LOD (Level of Detail) switching based on camera distance
//! - Mesh instancing for identical creatures
//! - Distance culling
//! - Performance metrics collection
//! - Auto-tuning of LOD distances

use bevy::prelude::*;

use crate::game::components::performance::{
    DistanceCulling, InstanceData, InstancedCreature, LodState, MeshStreaming, PerformanceMarker,
};
use crate::game::resources::performance::{LodAutoTuning, PerformanceMetrics};

/// System to update LOD levels based on camera distance
///
/// This system calculates the distance from each entity with a `LodState`
/// component to the camera and updates the LOD level accordingly.
///
/// # Examples
///
/// Add this system to your app:
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::performance::lod_switching_system;
///
/// fn setup_app(app: &mut App) {
///     app.add_systems(Update, lod_switching_system);
/// }
/// ```
pub fn lod_switching_system(
    camera_query: Query<&Transform, With<Camera>>,
    mut entity_query: Query<(&Transform, &mut LodState), Without<Camera>>,
    auto_tuning: Option<Res<LodAutoTuning>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let distance_scale = auto_tuning.map(|t| t.current_scale).unwrap_or(1.0);

    for (transform, mut lod_state) in entity_query.iter_mut() {
        let distance = camera_transform.translation.distance(transform.translation);

        // Apply auto-tuning scale
        let scaled_distance = distance / distance_scale;

        lod_state.update_for_distance(scaled_distance);
    }
}

/// System to cull entities beyond a certain distance
///
/// Disables rendering for entities that are too far from the camera
/// to improve performance.
pub fn distance_culling_system(
    camera_query: Query<&Transform, With<Camera>>,
    mut entity_query: Query<(&Transform, &mut DistanceCulling, &mut Visibility), Without<Camera>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    for (transform, mut culling, mut visibility) in entity_query.iter_mut() {
        let distance = camera_transform.translation.distance(transform.translation);

        let should_cull = distance > culling.max_distance;

        if should_cull != culling.culled {
            culling.culled = should_cull;
            *visibility = if should_cull {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
    }
}

/// System to collect performance metrics
///
/// Gathers statistics about rendering performance, LOD usage,
/// and entity counts.
pub fn performance_metrics_system(
    time: Res<Time>,
    mut metrics: ResMut<PerformanceMetrics>,
    lod_query: Query<&LodState>,
    marker_query: Query<&PerformanceMarker>,
) {
    // Update frame time
    metrics.update_frame_time(time.delta_secs());

    // Reset per-frame counters
    metrics.reset_frame_counters();

    // Count LOD levels
    for lod_state in lod_query.iter() {
        let triangles = 1000; // Placeholder - would get from actual mesh
        metrics.record_lod_level(lod_state.current_level, triangles);
    }

    // Count entities by category
    let mut category_counts = std::collections::HashMap::new();
    for marker in marker_query.iter() {
        *category_counts.entry(marker.category).or_insert(0) += 1;
    }

    metrics.entities_rendered = marker_query.iter().count();
}

/// System to auto-tune LOD distances based on performance
///
/// Dynamically adjusts LOD distance thresholds to maintain target FPS.
pub fn lod_auto_tuning_system(
    mut auto_tuning: ResMut<LodAutoTuning>,
    metrics: Res<PerformanceMetrics>,
    time: Res<Time>,
) {
    let current_fps = metrics.current_fps();
    auto_tuning.update(current_fps, time.delta_secs());
}

/// System to manage mesh streaming (loading/unloading)
///
/// Loads meshes when entities get close to camera and unloads
/// them when they're far away to save memory.
pub fn mesh_streaming_system(
    camera_query: Query<&Transform, With<Camera>>,
    mut entity_query: Query<(&Transform, &mut MeshStreaming), Without<Camera>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    for (transform, mut streaming) in entity_query.iter_mut() {
        let distance = camera_transform.translation.distance(transform.translation);

        if !streaming.loaded && distance <= streaming.load_distance {
            // TODO: Load mesh data
            streaming.loaded = true;
        } else if streaming.loaded && distance > streaming.unload_distance {
            // TODO: Unload mesh data
            streaming.loaded = false;
        }
    }
}

/// System to update instance data for instanced rendering
///
/// Synchronizes transform and color data for instanced entities.
pub fn instancing_update_system(
    mut instance_query: Query<(&Transform, &mut InstanceData, &InstancedCreature)>,
) {
    for (transform, mut instance_data, _instanced) in instance_query.iter_mut() {
        instance_data.transform = *transform;
    }
}

/// Plugin to register all performance systems
pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PerformanceMetrics>()
            .init_resource::<LodAutoTuning>()
            .add_systems(
                Update,
                (
                    lod_switching_system,
                    distance_culling_system,
                    performance_metrics_system,
                    lod_auto_tuning_system,
                    mesh_streaming_system,
                    instancing_update_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_plugin() {
        let mut app = App::new();
        app.add_plugins(PerformancePlugin);

        // Verify resources are initialized
        assert!(app.world().get_resource::<PerformanceMetrics>().is_some());
        assert!(app.world().get_resource::<LodAutoTuning>().is_some());
    }

    #[test]
    fn test_lod_switching_with_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Spawn camera
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)));

        // Spawn entity with LOD
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(15.0, 0.0, 0.0),
                LodState::new(vec![10.0, 20.0, 40.0]),
            ))
            .id();

        // Run system
        app.add_systems(Update, lod_switching_system);
        app.update();

        // Check LOD level was updated
        let lod_state = app.world().entity(entity).get::<LodState>().unwrap();
        assert_eq!(lod_state.current_level, 1); // Should be at medium distance LOD
    }

    #[test]
    fn test_distance_culling() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Spawn camera
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)));

        // Spawn far entity
        let far_entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(150.0, 0.0, 0.0),
                DistanceCulling {
                    max_distance: 100.0,
                    culled: false,
                },
                Visibility::Visible,
            ))
            .id();

        // Run system
        app.add_systems(Update, distance_culling_system);
        app.update();

        // Check entity was culled
        let culling = app
            .world()
            .entity(far_entity)
            .get::<DistanceCulling>()
            .unwrap();
        assert!(culling.culled);

        let visibility = app.world().entity(far_entity).get::<Visibility>().unwrap();
        assert_eq!(*visibility, Visibility::Hidden);
    }

    #[test]
    fn test_performance_metrics_collection() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PerformanceMetrics>();

        // Spawn entities with LOD
        for _ in 0..5 {
            app.world_mut().spawn(LodState::new(vec![10.0, 20.0, 40.0]));
        }

        // Run system
        app.add_systems(Update, performance_metrics_system);
        app.update();

        // Check metrics were collected
        let metrics = app.world().resource::<PerformanceMetrics>();
        assert_eq!(metrics.total_lod_entities(), 5);
    }

    #[test]
    fn test_mesh_streaming_load() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Spawn camera
        app.world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)));

        // Spawn close entity
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(30.0, 0.0, 0.0),
                MeshStreaming {
                    loaded: false,
                    load_distance: 50.0,
                    unload_distance: 100.0,
                    priority: 0,
                },
            ))
            .id();

        // Run system
        app.add_systems(Update, mesh_streaming_system);
        app.update();

        // Check mesh was marked for loading
        let streaming = app.world().entity(entity).get::<MeshStreaming>().unwrap();
        assert!(streaming.loaded);
    }

    #[test]
    fn test_instancing_update() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let new_transform = Transform::from_xyz(5.0, 10.0, 15.0);

        let entity = app
            .world_mut()
            .spawn((
                new_transform,
                InstanceData::default(),
                InstancedCreature {
                    creature_id: 1000,
                    instance_id: 0,
                },
            ))
            .id();

        // Run system
        app.add_systems(Update, instancing_update_system);
        app.update();

        // Check instance data was updated
        let instance_data = app.world().entity(entity).get::<InstanceData>().unwrap();
        assert_eq!(instance_data.transform, new_transform);
    }
}
