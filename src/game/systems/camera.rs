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
    // Spawn a 3D camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.7, 0.0).looking_at(Vec3::NEG_Z, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // Add a light source attached to the camera (or just global for now)
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            range: 20.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 5.0, 0.0),
        ..default()
    });
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    global_state: Res<GlobalState>,
) {
    let game_state = &global_state.0;
    let party_pos = game_state.world.party_position;
    let party_facing = game_state.world.party_facing;

    if let Ok(mut transform) = camera_query.get_single_mut() {
        // Update position
        // Map (x, y) -> World (x, 0, y)
        // Add 0.5 to center in tile
        let target_pos = Vec3::new(
            party_pos.x as f32 + 0.5,
            1.7, // Eye height
            party_pos.y as f32 + 0.5,
        );

        // Smoothly interpolate position (optional, for now snap)
        transform.translation = target_pos;

        // Update rotation based on facing
        // Wait, let's verify direction mapping.
        // North (0, -1). World -Z.
        // South (0, 1). World +Z.
        // East (1, 0). World +X.
        // West (-1, 0). World -X.

        // Bevy Camera looking at -Z by default.
        // So:
        // North (-Z) -> 0
        // West (-X) -> PI/2
        // South (+Z) -> PI
        // East (+X) -> -PI/2 (or 3PI/2)

        let rotation = match party_facing {
            Direction::North => Quat::from_rotation_y(0.0),
            Direction::West => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Direction::South => Quat::from_rotation_y(std::f32::consts::PI),
            Direction::East => Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        };

        transform.rotation = rotation;
    }
}
