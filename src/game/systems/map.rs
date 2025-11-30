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
        // Materials (base colors)
        // RGB tuples are kept to allow per-tile tinting of walls based on terrain
        let floor_rgb = (0.3_f32, 0.3_f32, 0.3_f32);
        let wall_base_rgb = (0.6_f32, 0.6_f32, 0.6_f32);
        let door_rgb = (0.4_f32, 0.2_f32, 0.1_f32); // Brown
        let water_rgb = (0.2_f32, 0.4_f32, 0.8_f32); // Blue
        let mountain_rgb = (0.5_f32, 0.5_f32, 0.5_f32); // Gray rock
        let forest_rgb = (0.2_f32, 0.6_f32, 0.2_f32); // Green
        let grass_rgb = (0.3_f32, 0.5_f32, 0.2_f32); // Darker green floor
        let stone_rgb = (0.5_f32, 0.5_f32, 0.55_f32);
        let dirt_rgb = (0.4_f32, 0.3_f32, 0.2_f32);

        let floor_color = Color::srgb(floor_rgb.0, floor_rgb.1, floor_rgb.2);
        let wall_base_color = Color::srgb(wall_base_rgb.0, wall_base_rgb.1, wall_base_rgb.2);
        let door_color = Color::srgb(door_rgb.0, door_rgb.1, door_rgb.2);
        let water_color = Color::srgb(water_rgb.0, water_rgb.1, water_rgb.2);
        let mountain_color = Color::srgb(mountain_rgb.0, mountain_rgb.1, mountain_rgb.2);
        let forest_color = Color::srgb(forest_rgb.0, forest_rgb.1, forest_rgb.2);
        let grass_color = Color::srgb(grass_rgb.0, grass_rgb.1, grass_rgb.2);


        let floor_material = materials.add(StandardMaterial {
            base_color: floor_color,
            perceptual_roughness: 0.9,
            ..default()
        });

        let wall_material = materials.add(StandardMaterial {
            base_color: wall_base_color,
            perceptual_roughness: 0.5,
            ..default()
        });

        let door_material = materials.add(StandardMaterial {
            base_color: door_color, // Brown
            perceptual_roughness: 0.5,
            ..default()
        });

        let water_material = materials.add(StandardMaterial {
            base_color: water_color, // Blue
            perceptual_roughness: 0.3,
            ..default()
        });

        let mountain_material = materials.add(StandardMaterial {
            base_color: mountain_color, // Gray rock
            perceptual_roughness: 0.8,
            ..default()
        });

        let forest_material = materials.add(StandardMaterial {
            base_color: forest_color, // Green
            perceptual_roughness: 0.7,
            ..default()
        });

        let grass_material = materials.add(StandardMaterial {
            base_color: grass_color, // Darker green floor
            perceptual_roughness: 0.9,
            ..default()
        });

        // Meshes
        let floor_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
        let wall_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let door_mesh = meshes.add(Cuboid::new(1.0, 1.0, 0.2)); // Thinner
        let water_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
        let mountain_mesh = meshes.add(Cuboid::new(1.0, 1.5, 1.0)); // Taller
        let forest_mesh = meshes.add(Cuboid::new(0.8, 1.2, 0.8)); // Tree-like

        // Iterate over tiles
        for y in 0..map.height {
            for x in 0..map.width {
                let pos = types::Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    // Determine if this is a perimeter tile
                    let is_perimeter =
                        x == 0 || y == 0 || x == map.width - 1 || y == map.height - 1;

                    // Render based on terrain type
                    match tile.terrain {
                        world::TerrainType::Water => {
                            // Render water slightly below at y = -0.1
                            commands.spawn((
                                Mesh3d(water_mesh.clone()),
                                MeshMaterial3d(water_material.clone()),
                                Transform::from_xyz(x as f32, -0.1, y as f32),
                            ));
                        }
                        world::TerrainType::Mountain => {
                            // Render mountain as tall block
                            commands.spawn((
                                Mesh3d(mountain_mesh.clone()),
                                MeshMaterial3d(mountain_material.clone()),
                                Transform::from_xyz(x as f32, 0.75, y as f32),
                            ));
                        }
                        world::TerrainType::Forest => {
                            // Render floor first
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(grass_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                            ));
                            // Then tree on top
                            commands.spawn((
                                Mesh3d(forest_mesh.clone()),
                                MeshMaterial3d(forest_material.clone()),
                                Transform::from_xyz(x as f32, 0.6, y as f32),
                            ));
                        }
                        world::TerrainType::Grass => {
                            // Grass floor
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(grass_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                            ));
                        }
                        _ => {
                            // Spawn regular floor for Ground, Stone, etc.
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(floor_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                            ));
                        }
                    }

                    // Spawn wall/door based on wall_type or perimeter
                    if is_perimeter && tile.wall_type == world::WallType::None {
                        // Add perimeter walls where none exist using the default wall material
                        commands.spawn((
                            Mesh3d(wall_mesh.clone()),
                            MeshMaterial3d(wall_material.clone()),
                            Transform::from_xyz(x as f32, 0.5, y as f32),
                        ));
                    } else {
                        match tile.wall_type {
                            world::WallType::Normal => {
                                // Tint/darken the wall material to match the underlying terrain color
                                // so a Forest Normal wall appears greenish while a Stone Normal wall remains grey.
                                let (tr, tg, tb) = match tile.terrain {
                                    world::TerrainType::Ground => floor_rgb,
                                    world::TerrainType::Grass => grass_rgb,
                                    world::TerrainType::Water => water_rgb,
                                    world::TerrainType::Lava => (0.8_f32, 0.3_f32, 0.2_f32),
                                    world::TerrainType::Swamp => (0.35_f32, 0.3_f32, 0.2_f32),
                                    world::TerrainType::Stone => stone_rgb,
                                    world::TerrainType::Dirt => dirt_rgb,
                                    world::TerrainType::Forest => forest_rgb,
                                    world::TerrainType::Mountain => mountain_rgb,
                                };
                                // Darken a bit to make the wall distinct from the floor
                                let darken = 0.6_f32;
                                let wall_color = Color::srgb(tr * darken, tg * darken, tb * darken);
                                let tile_wall_material = materials.add(StandardMaterial {
                                    base_color: wall_color,
                                    perceptual_roughness: 0.5,
                                    ..default()
                                });
                                commands.spawn((
                                    Mesh3d(wall_mesh.clone()),
                                    MeshMaterial3d(tile_wall_material.clone()),
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
}
