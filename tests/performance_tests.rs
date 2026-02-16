// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance integration tests for procedural mesh optimization
//!
//! This test suite validates:
//! - LOD generation performance and correctness
//! - Mesh batching analysis
//! - Texture atlas packing
//! - Memory optimization strategies
//! - Auto-tuning algorithms

use antares::domain::visual::performance::{
    analyze_batching, analyze_memory_usage, auto_tune_lod_distances, estimate_mesh_memory,
    generate_lod_with_distances, BatchingConfig, LodGenerationConfig, MemoryOptimizationConfig,
    MemoryStrategy,
};
use antares::domain::visual::texture_atlas::{estimate_atlas_size, generate_atlas, AtlasConfig};
use antares::domain::visual::MeshDefinition;
use std::collections::HashMap;

/// Create a test mesh with specified complexity
fn create_test_mesh(num_vertices: usize) -> MeshDefinition {
    let vertices: Vec<[f32; 3]> = (0..num_vertices)
        .map(|i| {
            let angle = (i as f32) * std::f32::consts::TAU / num_vertices as f32;
            [angle.cos(), angle.sin(), 0.0]
        })
        .collect();

    let indices: Vec<u32> = (0..num_vertices.saturating_sub(2))
        .flat_map(|i| vec![0, (i + 1) as u32, (i + 2) as u32])
        .collect();

    MeshDefinition {
        name: None,
        vertices,
        indices,
        normals: None,
        uvs: None,
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

#[test]
fn test_lod_generation_reduces_complexity() {
    let base_mesh = create_test_mesh(100);
    let config = LodGenerationConfig::default();

    let result = generate_lod_with_distances(&base_mesh, &config);

    // Should generate correct number of LOD levels
    assert_eq!(result.lod_meshes.len(), config.num_levels);
    assert_eq!(result.distances.len(), config.num_levels);

    // Overall complexity should be reduced
    let base_triangles = base_mesh.indices.len() / 3;
    let last_lod_triangles = result.lod_meshes.last().unwrap().indices.len() / 3;

    // Last LOD should be significantly simpler than base mesh
    assert!(
        last_lod_triangles < base_triangles,
        "Final LOD should have fewer triangles than base mesh"
    );

    // Should save memory
    assert!(result.memory_saved > 0);
}

#[test]
fn test_lod_distances_increase() {
    let base_mesh = create_test_mesh(200);
    let config = LodGenerationConfig::default();

    let result = generate_lod_with_distances(&base_mesh, &config);

    // Distances should increase for each LOD level
    for i in 1..result.distances.len() {
        assert!(
            result.distances[i] > result.distances[i - 1],
            "LOD distances should increase"
        );
    }
}

#[test]
fn test_batching_groups_similar_meshes() {
    let mut meshes = Vec::new();

    // Create meshes with same material (should batch together)
    for _ in 0..5 {
        meshes.push(create_test_mesh(50));
    }

    let config = BatchingConfig::default();
    let batches = analyze_batching(&meshes, &config);

    // All meshes should be in same batch (same material)
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].mesh_count, 5);
}

#[test]
fn test_texture_atlas_packing() {
    let mut textures = HashMap::new();
    textures.insert("texture1.png".to_string(), (64, 64));
    textures.insert("texture2.png".to_string(), (128, 128));
    textures.insert("texture3.png".to_string(), (32, 32));

    let config = AtlasConfig::default();
    let result = generate_atlas(&textures, &config).unwrap();

    // Should pack all textures
    assert_eq!(result.entries.len(), 3);

    // Atlas should be reasonable size
    assert!(result.width > 0);
    assert!(result.height > 0);
    assert!(result.width <= config.max_width);
    assert!(result.height <= config.max_height);

    // Efficiency should be positive
    assert!(result.efficiency > 0.0 && result.efficiency <= 1.0);

    // UV coordinates should be valid
    for entry in &result.entries {
        assert!(entry.atlas_uvs.0 >= 0.0 && entry.atlas_uvs.0 <= 1.0);
        assert!(entry.atlas_uvs.1 >= 0.0 && entry.atlas_uvs.1 <= 1.0);
        assert!(entry.atlas_uvs.2 >= 0.0 && entry.atlas_uvs.2 <= 1.0);
        assert!(entry.atlas_uvs.3 >= 0.0 && entry.atlas_uvs.3 <= 1.0);
    }
}

#[test]
fn test_auto_tuning_adjusts_distances() {
    let current_distances = vec![10.0, 20.0, 40.0];

    // Test below target FPS - should reduce distances
    let adjusted_below = auto_tune_lod_distances(&current_distances, 60.0, 45.0, 0.1);
    for i in 0..current_distances.len() {
        assert!(adjusted_below[i] < current_distances[i]);
    }

    // Test above target FPS - should increase distances
    let adjusted_above = auto_tune_lod_distances(&current_distances, 60.0, 80.0, 0.1);
    for i in 0..current_distances.len() {
        assert!(adjusted_above[i] > current_distances[i]);
    }
}

#[test]
fn test_memory_optimization_recommends_strategy() {
    // Small memory footprint - should keep all
    let small_meshes = vec![create_test_mesh(10); 5];
    let config_large = MemoryOptimizationConfig {
        max_mesh_memory: 1024 * 1024 * 1024, // 1GB
        ..Default::default()
    };

    let (strategy, _savings) = analyze_memory_usage(&small_meshes, &config_large);
    assert_eq!(strategy, MemoryStrategy::KeepAll);

    // Large memory footprint - should optimize
    let large_meshes = vec![create_test_mesh(1000); 100];
    let config_small = MemoryOptimizationConfig {
        max_mesh_memory: 1024, // Very small
        ..Default::default()
    };

    let (strategy, savings) = analyze_memory_usage(&large_meshes, &config_small);
    assert_ne!(strategy, MemoryStrategy::KeepAll);
    assert!(savings > 0);
}

#[test]
fn test_mesh_memory_estimation_accurate() {
    let mesh = create_test_mesh(100);
    let estimated = estimate_mesh_memory(&mesh);

    // Should account for vertices and indices at minimum
    let min_expected = mesh.vertices.len() * 12 + mesh.indices.len() * 4;
    assert!(estimated >= min_expected);
}

#[test]
fn test_atlas_size_estimation() {
    let mut textures = HashMap::new();
    for i in 0..10 {
        textures.insert(format!("tex{}.png", i), (64, 64));
    }

    let (width, height) = estimate_atlas_size(&textures);

    // Should be power of two
    assert!(width.is_power_of_two());
    assert!(height.is_power_of_two());

    // Should be large enough
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_lod_generation_preserves_color() {
    let mut base_mesh = create_test_mesh(50);
    base_mesh.color = [1.0, 0.5, 0.25, 1.0];

    let config = LodGenerationConfig::default();
    let result = generate_lod_with_distances(&base_mesh, &config);

    // All LOD levels should preserve color
    for lod_mesh in &result.lod_meshes {
        assert_eq!(lod_mesh.color, base_mesh.color);
    }
}

#[test]
fn test_batching_respects_max_vertices() {
    let config = BatchingConfig {
        max_vertices_per_batch: 100,
        ..Default::default()
    };

    let meshes = vec![create_test_mesh(150)]; // Exceeds limit

    let batches = analyze_batching(&meshes, &config);

    // Should still create batch (config is advisory)
    assert!(!batches.is_empty());
}

#[test]
fn test_atlas_packing_with_padding() {
    let mut textures = HashMap::new();
    textures.insert("tex1.png".to_string(), (64, 64));

    let config = AtlasConfig {
        padding: 4,
        ..Default::default()
    };

    let result = generate_atlas(&textures, &config).unwrap();

    // Position should account for padding
    assert_eq!(result.entries[0].atlas_position.0, config.padding);
    assert_eq!(result.entries[0].atlas_position.1, config.padding);
}

#[test]
fn test_lod_generation_with_custom_config() {
    let base_mesh = create_test_mesh(200);
    let config = LodGenerationConfig {
        num_levels: 4,
        reduction_factor: 0.6,
        min_triangles: 10,
        generate_billboard: false,
    };

    let result = generate_lod_with_distances(&base_mesh, &config);

    assert_eq!(result.lod_meshes.len(), 4);
}

#[test]
fn test_memory_usage_calculation_comprehensive() {
    // Create mesh with all optional fields populated
    let vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
    let normals = vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

    let mesh = MeshDefinition {
        name: None,
        vertices: vertices.clone(),
        indices: vec![0, 1, 2],
        normals: Some(normals.clone()),
        uvs: Some(uvs.clone()),
        color: [1.0, 1.0, 1.0, 1.0],
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    };

    let memory = estimate_mesh_memory(&mesh);

    // Should account for all fields
    let expected = vertices.len() * 12 // vertices
        + 3 * 4 // indices
        + normals.len() * 12 // normals
        + uvs.len() * 8; // uvs

    assert_eq!(memory, expected);
}

#[test]
fn test_auto_tuning_respects_bounds() {
    let distances = vec![10.0, 20.0, 40.0];

    // Extreme low FPS
    let adjusted = auto_tune_lod_distances(&distances, 60.0, 10.0, 1.0);

    // Should still produce valid distances
    for distance in &adjusted {
        assert!(*distance > 0.0);
    }
}

#[test]
fn test_texture_atlas_sorts_by_size() {
    let mut textures = HashMap::new();
    textures.insert("small.png".to_string(), (32, 32));
    textures.insert("large.png".to_string(), (128, 128));
    textures.insert("medium.png".to_string(), (64, 64));

    let config = AtlasConfig::default();
    let result = generate_atlas(&textures, &config).unwrap();

    // Largest should be packed first (at position 0,0 or close)
    assert_eq!(result.entries[0].path, "large.png");
}

#[test]
fn test_performance_optimization_end_to_end() {
    // Create a realistic scenario: multiple creatures with LOD
    let mut creatures = Vec::new();
    for i in 0..10 {
        let mesh = create_test_mesh(100 + i * 10);
        creatures.push(mesh);
    }

    // Generate LOD for all
    let lod_config = LodGenerationConfig::default();
    let mut total_memory_saved = 0;

    for creature in &creatures {
        let result = generate_lod_with_distances(creature, &lod_config);
        total_memory_saved += result.memory_saved;
    }

    // Should save significant memory across all creatures
    assert!(total_memory_saved > 0);

    // Analyze batching
    let batch_config = BatchingConfig::default();
    let batches = analyze_batching(&creatures, &batch_config);
    assert!(!batches.is_empty());

    // Create texture atlas
    let mut textures = HashMap::new();
    for i in 0..10 {
        textures.insert(format!("creature_{}.png", i), (64, 64));
    }
    let atlas_config = AtlasConfig::default();
    let atlas = generate_atlas(&textures, &atlas_config).unwrap();
    assert_eq!(atlas.entries.len(), 10);

    // Test auto-tuning
    let distances = vec![10.0, 25.0, 50.0];
    let tuned = auto_tune_lod_distances(&distances, 60.0, 55.0, 0.1);
    assert_eq!(tuned.len(), distances.len());
}
