// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types;
use crate::domain::world;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

pub struct MapRenderingPlugin;

impl Plugin for MapRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_map);
    }
}

fn spawn_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_state: Res<GlobalState>,
) {
    let game_state = &global_state.0;

    if let Some(map) = game_state.world.get_current_map() {
        // Materials
        let floor_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.3),
            perceptual_roughness: 0.9,
            ..default()
        });

        let wall_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.6, 0.6),
            perceptual_roughness: 0.5,
            ..default()
        });

        let door_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.2, 0.1), // Brown
            perceptual_roughness: 0.5,
            ..default()
        });

        // Meshes
        let floor_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
        let wall_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let door_mesh = meshes.add(Cuboid::new(1.0, 1.0, 0.2)); // Thinner

        // Iterate over tiles
        for y in 0..map.height {
            for x in 0..map.width {
                let pos = types::Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    // Spawn floor
                    commands.spawn((
                        Mesh3d(floor_mesh.clone()),
                        MeshMaterial3d(floor_material.clone()),
                        Transform::from_xyz(x as f32, 0.0, y as f32),
                    ));

                    // Spawn wall/door
                    match tile.wall_type {
                        world::WallType::Normal => {
                            commands.spawn((
                                Mesh3d(wall_mesh.clone()),
                                MeshMaterial3d(wall_material.clone()),
                                Transform::from_xyz(x as f32, 0.5, y as f32),
                            ));
                        }
                        world::WallType::Door => {
                            commands.spawn((
                                Mesh3d(door_mesh.clone()),
                                MeshMaterial3d(door_material.clone()),
                                Transform::from_xyz(x as f32, 0.5, y as f32),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
