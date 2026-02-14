// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance measurements for grass instance batching (Phase 4)
//!
//! Simple benchmark using standard library timing.
//! Run with: `cargo bench --bench grass_instancing`

use std::time::Instant;

use antares::game::systems::advanced_grass::{
    build_grass_instance_batches_system, GrassBladeInstance, GrassCluster, GrassInstanceConfig,
};
use antares::game::systems::map::MapEntity;
use bevy::prelude::*;

fn main() {
    println!("Grass Instance Batching Measurements");
    println!("======================================\n");

    run_instance_bench("100 clusters", 100, 8);
    run_instance_bench("400 clusters", 400, 8);

    println!("======================================");
    println!("All benchmarks completed successfully!");
}

fn run_instance_bench(label: &str, cluster_count: u32, blades_per_cluster: u32) {
    println!(
        "Benchmark: {} ({} clusters, {} blades/cluster)",
        label, cluster_count, blades_per_cluster
    );

    let mut app = App::new();
    app.insert_resource(GrassInstanceConfig {
        enabled: true,
        max_instances_per_batch: 1024,
    });
    app.add_systems(Update, build_grass_instance_batches_system);

    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());

    let mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mut test_mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::all(),
        );
        test_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
        );
        test_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
        test_mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        );
        test_mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
        meshes.add(test_mesh)
    };

    let material = {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        materials.add(StandardMaterial::default())
    };

    for i in 0..cluster_count {
        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(i as f32, 0.0, 0.0)),
                GlobalTransform::default(),
                GrassCluster::default(),
                MapEntity(1),
            ))
            .id();

        for blade_index in 0..blades_per_cluster {
            let blade = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(0.1 * blade_index as f32, 0.0, 0.0)),
                    GlobalTransform::default(),
                    GrassBladeInstance {
                        mesh: mesh.clone(),
                        material: material.clone(),
                    },
                ))
                .id();
            app.world_mut().entity_mut(cluster).add_child(blade);
        }
    }

    let start = Instant::now();
    app.update();
    let duration = start.elapsed();

    println!("  Time: {:?}\n", duration);
}
