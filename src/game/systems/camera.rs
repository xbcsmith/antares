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
    debug!("Setting up angled tactical camera at (0, 3.0, 2.0) tilted down 35°");

    // Spawn a 3D camera - Bevy 0.17 uses required components pattern
    // Angled tactical view: higher position (y=3.0), offset back (z=2.0), tilted down ~35°
    // This creates perspective where walls appear tall and buildings look imposing
    // Scale: 1 unit ≈ 10 feet, so camera is ~30 feet high, party at ground level
    let camera_offset_back = 2.0; // Units behind party position
    let camera_height = 3.0; // Units above ground

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, camera_height, camera_offset_back)
            .looking_at(Vec3::new(0.0, 0.0, -2.0), Vec3::Y), // Look ahead and down
        MainCamera,
    ));

    debug!("Adding point light at (0, 8, 0) with intensity 2M lumens");

    // Add a light source higher up to illuminate taller walls
    // Bevy 0.17 uses lumen-based physically-based rendering
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0, // Brighter to illuminate 20+ foot tall walls
            shadows_enabled: true,
            range: 60.0, // Larger range for taller geometry
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 0.0), // Higher light source
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
        // Angled tactical camera: position above and behind party, looking down
        // Camera offset and height constants
        let camera_offset_back = 2.0; // Units behind party (in facing direction)
        let camera_height = 3.0; // Units above ground
        let tilt_angle = -35.0_f32.to_radians(); // Tilt down angle

        // Party center position in world space
        let party_center = Vec3::new(
            party_pos.x as f32 + 0.5,
            0.0, // Party at ground level
            party_pos.y as f32 + 0.5,
        );

        // Calculate offset direction based on party facing
        // Camera should be BEHIND the party (opposite of facing direction)
        // Direction mapping:
        // North (0, -1) -> World -Z -> Camera offset +Z (behind = south)
        // South (0, 1) -> World +Z -> Camera offset -Z (behind = north)
        // East (1, 0) -> World +X -> Camera offset -X (behind = west)
        // West (-1, 0) -> World -X -> Camera offset +X (behind = east)
        let (offset_x, offset_z, y_rotation) = match party_facing {
            Direction::North => (0.0, camera_offset_back, 0.0),
            Direction::South => (0.0, -camera_offset_back, std::f32::consts::PI),
            Direction::East => (-camera_offset_back, 0.0, -std::f32::consts::FRAC_PI_2),
            Direction::West => (camera_offset_back, 0.0, std::f32::consts::FRAC_PI_2),
        };

        // Position camera behind and above party
        let camera_pos = Vec3::new(
            party_center.x + offset_x,
            camera_height,
            party_center.z + offset_z,
        );

        // Create rotation: Y-rotation for facing, then X-rotation (tilt) for downward angle
        let y_rot = Quat::from_rotation_y(y_rotation);
        let x_tilt = Quat::from_rotation_x(tilt_angle);
        let combined_rotation = y_rot * x_tilt;

        transform.translation = camera_pos;
        transform.rotation = combined_rotation;
    }
}
