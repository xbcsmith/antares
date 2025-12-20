// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types::Direction;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, update_camera);
    }
}

/// Marker component for the main game camera
#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    debug!("Setting up first-person camera at eye level (0.6 units)");

    // Spawn a 3D camera - Bevy 0.17 uses required components pattern
    // First-person view like Might and Magic 1:
    // - Camera at party position at eye level (0.6 units = 6 feet)
    // - Looking straight ahead in facing direction
    // - No offset - camera IS the party's viewpoint
    // Scale: 1 unit ≈ 10 feet, so eye level is 6 feet above ground
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.6, 0.0).looking_at(Vec3::NEG_Z, Vec3::Y),
        MainCamera,
    ));

    debug!("Adding point light at (0, 5, 0) with intensity 2M lumens");

    // Add a light source - positioned above to illuminate tall dungeon walls
    // Bevy 0.17 uses lumen-based physically-based rendering
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0, // Bright enough for tall walls in first-person
            shadows_enabled: true,
            range: 60.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));

    debug!("Camera setup complete");
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    global_state: Res<GlobalState>,
) {
    let game_state = &global_state.0;
    let party_pos = game_state.world.party_position;
    let party_facing = game_state.world.party_facing;

    if let Ok(mut transform) = camera_query.single_mut() {
        // First-person camera: positioned AT party location at eye level
        // Eye height constant
        let eye_height = 0.6; // 6 feet above ground (1 unit = 10 feet)

        // Party center position in world space
        // Add 0.5 to center camera in the tile
        let camera_pos = Vec3::new(
            party_pos.x as f32 + 0.5,
            eye_height,
            party_pos.y as f32 + 0.5,
        );

        // Camera rotation based on party facing direction
        // Looking straight ahead (horizontal) in the facing direction
        // Direction mapping:
        // North (0, -1) -> World -Z -> Y-rotation 0
        // South (0, 1) -> World +Z -> Y-rotation PI
        // East (1, 0) -> World +X -> Y-rotation -PI/2
        // West (-1, 0) -> World -X -> Y-rotation PI/2
        let y_rotation = match party_facing {
            Direction::North => 0.0,
            Direction::South => std::f32::consts::PI,
            Direction::East => -std::f32::consts::FRAC_PI_2,
            Direction::West => std::f32::consts::FRAC_PI_2,
        };

        let rotation = Quat::from_rotation_y(y_rotation);

        transform.translation = camera_pos;
        transform.rotation = rotation;
    }
}
