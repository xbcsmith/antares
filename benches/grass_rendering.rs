// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance measurements for grass rendering systems (Phase 2)
//!
//! Simple benchmark using standard library timing.
//! Run with: `cargo bench --bench grass_rendering`

use std::time::Instant;

use antares::game::systems::advanced_grass::{
    grass_distance_culling_system, grass_lod_system, GrassBlade, GrassCluster, GrassRenderConfig,
};
use bevy::prelude::*;

fn main() {
    println!("Grass Rendering Performance Measurements");
    println!("==========================================\n");

    run_culling_benchmark("100 tiles", 100, 8);
    run_culling_benchmark("400 tiles", 400, 8);

    println!("==========================================");
    println!("All benchmarks completed successfully!");
}

fn run_culling_benchmark(label: &str, cluster_count: u32, blades_per_cluster: u32) {
    println!(
        "Benchmark: {} ({} clusters, {} blades/cluster)",
        label, cluster_count, blades_per_cluster
    );

    let mut app = App::new();
    app.insert_resource(GrassRenderConfig::default());
    app.add_systems(Update, (grass_distance_culling_system, grass_lod_system));

    app.world_mut().spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 0.0),
        GlobalTransform::from(Transform::from_xyz(0.0, 2.0, 0.0)),
    ));

    for i in 0..cluster_count {
        let cluster_entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(i as f32, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(i as f32, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        for blade_index in 0..blades_per_cluster {
            let blade = app
                .world_mut()
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    GrassBlade {
                        lod_index: blade_index,
                    },
                ))
                .id();
            app.world_mut().entity_mut(cluster_entity).add_child(blade);
        }
    }

    let start = Instant::now();
    app.update();
    let duration = start.elapsed();

    println!("  Time: {:?}\n", duration);
}
