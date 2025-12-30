// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Camera System
//!
//! Provides camera setup and update logic for the game, supporting multiple camera modes
//! and configuration-driven behavior.
//!
//! # Camera Modes
//!
//! - **FirstPerson**: Classic Might and Magic 1 style first-person view
//! - **Tactical**: Overhead tactical view (future implementation)
//! - **Isometric**: Isometric view (future implementation)
//!
//! # Configuration
//!
//! Camera behavior is driven by `CameraConfig` loaded from campaign configuration.
//! See [`CameraConfigResource`] for available settings.

use crate::domain::types::Direction;
use crate::game::resources::GlobalState;
use crate::sdk::game_config::{CameraConfig, CameraMode};
use bevy::prelude::*;

/// Camera plugin that sets up and manages the game camera
///
/// This plugin must be initialized with a [`CameraConfig`] that controls
/// camera behavior, field of view, clipping planes, and lighting.
pub struct CameraPlugin {
    /// Camera configuration from campaign
    pub config: CameraConfig,
}

impl CameraPlugin {
    /// Create a new camera plugin with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Camera configuration from campaign
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::camera::CameraPlugin;
    /// use antares::sdk::game_config::CameraConfig;
    ///
    /// let config = CameraConfig::default();
    /// let plugin = CameraPlugin::new(config);
    /// ```
    pub fn new(config: CameraConfig) -> Self {
        Self { config }
    }
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // Insert camera configuration as a resource
        app.insert_resource(CameraConfigResource(self.config.clone()));

        app.add_systems(Startup, setup_camera)
            .add_systems(Update, update_camera);
    }
}

/// Bevy resource wrapping camera configuration
///
/// This resource is inserted by [`CameraPlugin`] and used by camera systems
/// to control camera behavior.
#[derive(Resource, Clone)]
pub struct CameraConfigResource(pub CameraConfig);

/// Marker component for the main game camera
#[derive(Component)]
pub struct MainCamera;

/// Marker component for the main game light source
#[derive(Component)]
pub struct MainLight;

/// Sets up the camera and lighting based on configuration
///
/// Creates a 3D camera at the appropriate position and a point light source.
/// The camera mode determines the initial setup behavior.
fn setup_camera(mut commands: Commands, config: Res<CameraConfigResource>) {
    let camera_config = &config.0;

    debug!(
        "Setting up camera: mode={:?}, eye_height={}, fov={}",
        camera_config.mode, camera_config.eye_height, camera_config.fov
    );

    // Setup camera based on mode
    match camera_config.mode {
        CameraMode::FirstPerson => {
            setup_first_person_camera(&mut commands, camera_config);
        }
        CameraMode::Tactical => {
            // Future implementation: tactical overhead view
            warn!("Tactical camera mode not yet implemented, falling back to first-person");
            setup_first_person_camera(&mut commands, camera_config);
        }
        CameraMode::Isometric => {
            // Future implementation: isometric view
            warn!("Isometric camera mode not yet implemented, falling back to first-person");
            setup_first_person_camera(&mut commands, camera_config);
        }
    }

    // Add a light source - positioned above to illuminate dungeon walls
    // Bevy 0.17 uses lumen-based physically-based rendering
    debug!(
        "Adding point light at (0, {}, 0) with intensity {} lumens, range {}, shadows={}",
        camera_config.light_height,
        camera_config.light_intensity,
        camera_config.light_range,
        camera_config.shadows_enabled
    );

    commands.spawn((
        PointLight {
            intensity: camera_config.light_intensity,
            shadows_enabled: camera_config.shadows_enabled,
            range: camera_config.light_range,
            ..default()
        },
        Transform::from_xyz(0.0, camera_config.light_height, 0.0),
        MainLight,
    ));

    debug!("Camera setup complete");
}

/// Sets up a first-person camera (Might and Magic 1 style)
///
/// Camera is positioned at party location at eye level, looking in the facing direction.
fn setup_first_person_camera(commands: &mut Commands, config: &CameraConfig) {
    // First-person view like Might and Magic 1:
    // - Camera at party position at eye level (configurable)
    // - Looking straight ahead in facing direction
    // - No offset - camera IS the party's viewpoint
    // - FOV and clipping planes are configurable

    let projection = Projection::Perspective(PerspectiveProjection {
        fov: config.fov.to_radians(),
        near: config.near_clip,
        far: config.far_clip,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        projection,
        Transform::from_xyz(0.0, config.eye_height, 0.0).looking_at(Vec3::NEG_Z, Vec3::Y),
        MainCamera,
    ));
}

/// Updates camera position and rotation based on party state
///
/// Synchronizes camera with party position and facing direction from game state.
/// Supports smooth rotation if enabled in configuration.
fn update_camera(
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    mut light_query: Query<&mut Transform, (With<MainLight>, Without<MainCamera>)>,
    global_state: Res<GlobalState>,
    config: Res<CameraConfigResource>,
) {
    let game_state = &global_state.0;
    let party_pos = game_state.world.party_position;
    let party_facing = game_state.world.party_facing;
    let camera_config = &config.0;

    if let Ok(mut camera_transform) = camera_query.single_mut() {
        match camera_config.mode {
            CameraMode::FirstPerson => {
                update_first_person_camera(
                    &mut camera_transform,
                    party_pos,
                    party_facing,
                    camera_config,
                    time.delta_secs(),
                );
            }
            CameraMode::Tactical => {
                // Future: tactical camera update
                update_first_person_camera(
                    &mut camera_transform,
                    party_pos,
                    party_facing,
                    camera_config,
                    time.delta_secs(),
                );
            }
            CameraMode::Isometric => {
                // Future: isometric camera update
                update_first_person_camera(
                    &mut camera_transform,
                    party_pos,
                    party_facing,
                    camera_config,
                    time.delta_secs(),
                );
            }
        }

        // Update light to follow camera
        if let Ok(mut light_transform) = light_query.single_mut() {
            light_transform.translation = Vec3::new(
                camera_transform.translation.x,
                camera_config.light_height,
                camera_transform.translation.z,
            );
        }
    }
}

/// Updates first-person camera position and rotation
///
/// Positions camera at party location at configured eye height, facing the party's direction.
/// Optionally applies smooth rotation based on configuration.
fn update_first_person_camera(
    transform: &mut Transform,
    party_pos: crate::domain::types::Position,
    party_facing: Direction,
    config: &CameraConfig,
    delta_time: f32,
) {
    // First-person camera: positioned AT party location at eye level
    // Party center position in world space
    // Add 0.5 to center camera in the tile
    let camera_pos = Vec3::new(
        party_pos.x as f32 + 0.5,
        config.eye_height,
        party_pos.y as f32 + 0.5,
    );

    // Camera rotation based on party facing direction
    // Looking straight ahead (horizontal) in the facing direction
    // Direction mapping:
    // North (0, -1) -> World -Z -> Y-rotation 0
    // South (0, 1) -> World +Z -> Y-rotation PI
    // East (1, 0) -> World +X -> Y-rotation -PI/2
    // West (-1, 0) -> World -X -> Y-rotation PI/2
    let target_y_rotation = match party_facing {
        Direction::North => 0.0,
        Direction::South => std::f32::consts::PI,
        Direction::East => -std::f32::consts::FRAC_PI_2,
        Direction::West => std::f32::consts::FRAC_PI_2,
    };

    transform.translation = camera_pos;

    // Apply rotation (smooth or instant based on config)
    if config.smooth_rotation {
        // Smooth rotation using configured rotation speed
        let current_rotation = transform.rotation;
        let target_rotation = Quat::from_rotation_y(target_y_rotation);

        // Interpolate rotation
        let max_rotation_delta = config.rotation_speed.to_radians() * delta_time;
        let angle_diff = current_rotation.angle_between(target_rotation);

        if angle_diff > max_rotation_delta {
            // Partial rotation
            let t = max_rotation_delta / angle_diff;
            transform.rotation = current_rotation.slerp(target_rotation, t);
        } else {
            // Snap to target if close enough
            transform.rotation = target_rotation;
        }
    } else {
        // Instant rotation
        transform.rotation = Quat::from_rotation_y(target_y_rotation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Position;

    #[test]
    fn test_camera_plugin_creation() {
        let config = CameraConfig::default();
        let plugin = CameraPlugin::new(config.clone());
        assert_eq!(plugin.config.mode, config.mode);
        assert_eq!(plugin.config.eye_height, config.eye_height);
    }

    #[test]
    fn test_camera_config_resource() {
        let config = CameraConfig::default();
        let resource = CameraConfigResource(config.clone());
        assert_eq!(resource.0.fov, config.fov);
        assert_eq!(resource.0.near_clip, config.near_clip);
        assert_eq!(resource.0.far_clip, config.far_clip);
    }

    #[test]
    fn test_first_person_camera_rotation_north() {
        let mut transform = Transform::default();
        let config = CameraConfig {
            smooth_rotation: false,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 5, y: 5 },
            Direction::North,
            &config,
            0.016,
        );

        // North should have 0 rotation
        assert_eq!(transform.rotation, Quat::from_rotation_y(0.0));
        assert_eq!(transform.translation.x, 5.5);
        assert_eq!(transform.translation.z, 5.5);
        assert_eq!(transform.translation.y, config.eye_height);
    }

    #[test]
    fn test_first_person_camera_rotation_south() {
        let mut transform = Transform::default();
        let config = CameraConfig {
            smooth_rotation: false,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 3, y: 7 },
            Direction::South,
            &config,
            0.016,
        );

        // South should have PI rotation
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_y(std::f32::consts::PI)
        );
    }

    #[test]
    fn test_first_person_camera_rotation_east() {
        let mut transform = Transform::default();
        let config = CameraConfig {
            smooth_rotation: false,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 2, y: 2 },
            Direction::East,
            &config,
            0.016,
        );

        // East should have -PI/2 rotation
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
        );
    }

    #[test]
    fn test_first_person_camera_rotation_west() {
        let mut transform = Transform::default();
        let config = CameraConfig {
            smooth_rotation: false,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 8, y: 1 },
            Direction::West,
            &config,
            0.016,
        );

        // West should have PI/2 rotation
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        );
    }

    #[test]
    fn test_camera_position_centering() {
        let mut transform = Transform::default();
        let config = CameraConfig::default();

        update_first_person_camera(
            &mut transform,
            Position { x: 10, y: 20 },
            Direction::North,
            &config,
            0.016,
        );

        // Position should be centered in tile (+0.5)
        assert_eq!(transform.translation.x, 10.5);
        assert_eq!(transform.translation.z, 20.5);
        assert_eq!(transform.translation.y, config.eye_height);
    }

    #[test]
    fn test_smooth_rotation_enabled() {
        let start_rotation = Quat::from_rotation_y(0.0);
        let mut transform = Transform::from_rotation(start_rotation);
        let config = CameraConfig {
            smooth_rotation: true,
            rotation_speed: 90.0, // 90 degrees per second
            ..Default::default()
        };

        let target_rotation = Quat::from_rotation_y(std::f32::consts::PI);

        // Target is South (PI radians), current is North (0)
        // Delta time is 0.1 seconds, so max rotation is 90 * 0.1 = 9 degrees = 0.157 radians
        update_first_person_camera(
            &mut transform,
            Position { x: 0, y: 0 },
            Direction::South,
            &config,
            0.1,
        );

        // Should have rotated partially, not snapped to target
        // Check that rotation has changed from start
        let angle_diff_from_start = start_rotation.angle_between(transform.rotation);
        let angle_diff_from_target = target_rotation.angle_between(transform.rotation);

        // Verify rotation occurred (moved away from start)
        assert!(
            angle_diff_from_start > 0.01,
            "Rotation should have moved from start"
        );
        // Verify we haven't reached target (still some distance from target)
        assert!(
            angle_diff_from_target > 0.01,
            "Rotation should not have reached target yet"
        );
    }

    #[test]
    fn test_smooth_rotation_disabled() {
        let mut transform = Transform::from_rotation(Quat::from_rotation_y(0.0));
        let config = CameraConfig {
            smooth_rotation: false,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 0, y: 0 },
            Direction::South,
            &config,
            0.1,
        );

        // Should snap immediately to target rotation
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_y(std::f32::consts::PI)
        );
    }

    #[test]
    fn test_camera_eye_height_configuration() {
        let mut transform = Transform::default();
        let config = CameraConfig {
            eye_height: 1.5,
            ..Default::default()
        };

        update_first_person_camera(
            &mut transform,
            Position { x: 0, y: 0 },
            Direction::North,
            &config,
            0.016,
        );

        assert_eq!(transform.translation.y, 1.5);
    }

    #[test]
    fn test_default_camera_config_values() {
        let config = CameraConfig::default();

        // Verify default values match Phase 1 implementation
        assert_eq!(config.mode, CameraMode::FirstPerson);
        assert_eq!(config.eye_height, 0.6);
        assert_eq!(config.fov, 70.0);
        assert_eq!(config.near_clip, 0.1);
        assert_eq!(config.far_clip, 1000.0);
        assert!(!config.smooth_rotation);
        assert_eq!(config.rotation_speed, 180.0);
        assert_eq!(config.light_height, 5.0);
        assert_eq!(config.light_intensity, 2_000_000.0);
        assert_eq!(config.light_range, 60.0);
        assert!(config.shadows_enabled);
    }
}
