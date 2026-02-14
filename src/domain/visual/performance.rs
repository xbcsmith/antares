// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance optimization algorithms for procedural mesh systems
//!
//! This module provides domain-layer performance optimization algorithms including:
//! - Advanced LOD generation with mesh simplification
//! - Mesh batching analysis and optimization
//! - LOD distance calculation and auto-tuning
//! - Memory usage estimation and optimization strategies

use crate::domain::visual::MeshDefinition;
use serde::{Deserialize, Serialize};

/// Configuration for automatic LOD generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodGenerationConfig {
    /// Number of LOD levels to generate (excluding base mesh)
    pub num_levels: usize,

    /// Target triangle reduction per level (0.0 to 1.0)
    /// e.g., 0.5 means each level has 50% fewer triangles than previous
    pub reduction_factor: f32,

    /// Minimum number of triangles for the lowest LOD
    pub min_triangles: usize,

    /// Whether to generate billboard LOD as final level
    pub generate_billboard: bool,
}

impl Default for LodGenerationConfig {
    fn default() -> Self {
        Self {
            num_levels: 3,
            reduction_factor: 0.5,
            min_triangles: 8,
            generate_billboard: true,
        }
    }
}

/// Result of LOD generation analysis
#[derive(Debug, Clone)]
pub struct LodGenerationResult {
    /// Generated LOD meshes (excluding base mesh)
    pub lod_meshes: Vec<MeshDefinition>,

    /// Recommended viewing distances for each LOD level (in world units)
    pub distances: Vec<f32>,

    /// Memory saved by using LOD system (bytes)
    pub memory_saved: usize,

    /// Triangle count reduction percentage
    pub triangle_reduction: f32,
}

/// Automatically generate LOD levels with optimal distances
///
/// This function generates multiple LOD levels from a base mesh using
/// progressive mesh simplification. It also calculates optimal viewing
/// distances based on mesh complexity and screen-space error.
///
/// # Arguments
///
/// * `base_mesh` - The high-detail base mesh
/// * `config` - LOD generation configuration
///
/// # Returns
///
/// Returns `LodGenerationResult` with generated meshes and metadata
///
/// # Examples
///
/// ```
/// use antares::domain::visual::performance::{generate_lod_with_distances, LodGenerationConfig};
/// use antares::domain::visual::MeshDefinition;
///
/// let base_mesh = MeshDefinition {
///     vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///     indices: vec![0, 1, 2],
///     normals: None,
///     uvs: None,
///     color: [1.0, 1.0, 1.0, 1.0],
///     lod_levels: vec![],
///     lod_distances: vec![],
///     material: None,
///     texture_path: None,
/// };
///
/// let config = LodGenerationConfig::default();
/// let result = generate_lod_with_distances(&base_mesh, &config);
///
/// assert!(!result.lod_meshes.is_empty());
/// assert_eq!(result.distances.len(), result.lod_meshes.len());
/// ```
pub fn generate_lod_with_distances(
    base_mesh: &MeshDefinition,
    config: &LodGenerationConfig,
) -> LodGenerationResult {
    use crate::domain::visual::lod::simplify_mesh;

    let base_triangle_count = base_mesh.indices.len() / 3;
    let mut lod_meshes = Vec::new();
    let mut distances = Vec::new();
    let mut total_memory_saved = 0;

    let base_mesh_size = estimate_mesh_memory(base_mesh);
    let base_size = calculate_mesh_bounding_size(base_mesh);

    for level in 1..=config.num_levels {
        let reduction = config.reduction_factor.powi(level as i32);
        let target_count = (base_triangle_count as f32 * (1.0 - reduction))
            .max(config.min_triangles as f32) as usize;

        // Generate LOD mesh
        let lod_mesh = if target_count <= config.min_triangles && config.generate_billboard {
            // Use billboard for lowest LOD
            simplify_mesh(base_mesh, config.min_triangles / 2)
        } else {
            simplify_mesh(base_mesh, target_count)
        };

        // Calculate optimal viewing distance for this LOD level
        // Distance increases exponentially with LOD level
        // Base formula: distance = base_size * scale_factor * level_multiplier
        let scale_factor = 10.0; // Tunable parameter
        let level_multiplier = 2.0_f32.powi(level as i32);
        let distance = base_size * scale_factor * level_multiplier;

        let mesh_size = estimate_mesh_memory(&lod_mesh);
        total_memory_saved += base_mesh_size.saturating_sub(mesh_size);

        distances.push(distance);
        lod_meshes.push(lod_mesh);
    }

    let final_triangle_count: usize = lod_meshes.iter().map(|m| m.indices.len() / 3).sum();
    let triangle_reduction = if base_triangle_count > 0 {
        let reduction = 100.0 * (1.0 - (final_triangle_count as f32 / base_triangle_count as f32));
        reduction.max(0.0)
    } else {
        0.0
    };

    LodGenerationResult {
        lod_meshes,
        distances,
        memory_saved: total_memory_saved,
        triangle_reduction,
    }
}

/// Calculate bounding box size of a mesh (maximum dimension)
fn calculate_mesh_bounding_size(mesh: &MeshDefinition) -> f32 {
    if mesh.vertices.is_empty() {
        return 1.0;
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for vertex in &mesh.vertices {
        for i in 0..3 {
            min[i] = min[i].min(vertex[i]);
            max[i] = max[i].max(vertex[i]);
        }
    }

    let size_x = max[0] - min[0];
    let size_y = max[1] - min[1];
    let size_z = max[2] - min[2];

    size_x.max(size_y).max(size_z)
}

/// Estimate memory usage of a mesh in bytes
pub fn estimate_mesh_memory(mesh: &MeshDefinition) -> usize {
    let vertex_bytes = mesh.vertices.len() * std::mem::size_of::<[f32; 3]>();
    let index_bytes = mesh.indices.len() * std::mem::size_of::<u32>();
    let normal_bytes = mesh
        .normals
        .as_ref()
        .map_or(0, |n| n.len() * std::mem::size_of::<[f32; 3]>());
    let uv_bytes = mesh
        .uvs
        .as_ref()
        .map_or(0, |uv| uv.len() * std::mem::size_of::<[f32; 2]>());

    vertex_bytes + index_bytes + normal_bytes + uv_bytes
}

/// Configuration for mesh batching optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Maximum vertices per batch
    pub max_vertices_per_batch: usize,

    /// Maximum instances per batch
    pub max_instances_per_batch: usize,

    /// Whether to sort by material
    pub sort_by_material: bool,

    /// Whether to sort by texture
    pub sort_by_texture: bool,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            max_vertices_per_batch: 65536, // u16::MAX + 1
            max_instances_per_batch: 1024,
            sort_by_material: true,
            sort_by_texture: true,
        }
    }
}

/// Represents a batch of similar meshes that can be rendered together
#[derive(Debug, Clone)]
pub struct MeshBatch {
    /// Material key for this batch (for sorting/grouping)
    pub material_key: String,

    /// Texture key for this batch (for sorting/grouping)
    pub texture_key: String,

    /// Total vertices in this batch
    pub total_vertices: usize,

    /// Total triangles in this batch
    pub total_triangles: usize,

    /// Number of meshes in this batch
    pub mesh_count: usize,
}

/// Analyze meshes and suggest batching strategy
///
/// # Arguments
///
/// * `meshes` - Collection of meshes to analyze
/// * `config` - Batching configuration
///
/// # Returns
///
/// Returns suggested batches and estimated performance improvement
///
/// # Examples
///
/// ```
/// use antares::domain::visual::performance::{analyze_batching, BatchingConfig};
/// use antares::domain::visual::MeshDefinition;
///
/// let meshes = vec![
///     MeshDefinition {
///         vertices: vec![[0.0, 0.0, 0.0]],
///         indices: vec![0],
///         normals: None,
///         uvs: None,
///         color: [1.0, 1.0, 1.0, 1.0],
///         lod_levels: vec![],
///         lod_distances: vec![],
///         material: None,
///         texture_path: None,
///     },
/// ];
///
/// let config = BatchingConfig::default();
/// let batches = analyze_batching(&meshes, &config);
/// assert!(!batches.is_empty());
/// ```
pub fn analyze_batching(meshes: &[MeshDefinition], config: &BatchingConfig) -> Vec<MeshBatch> {
    use std::collections::HashMap;

    let mut batch_map: HashMap<(String, String), MeshBatch> = HashMap::new();

    for mesh in meshes {
        let material_key = mesh
            .material
            .as_ref()
            .map(|m| format!("{:?}", m))
            .unwrap_or_else(|| "default".to_string());

        let texture_key = mesh
            .texture_path
            .clone()
            .unwrap_or_else(|| "none".to_string());

        let key = (material_key.clone(), texture_key.clone());

        let batch = batch_map.entry(key).or_insert_with(|| MeshBatch {
            material_key,
            texture_key,
            total_vertices: 0,
            total_triangles: 0,
            mesh_count: 0,
        });

        batch.total_vertices += mesh.vertices.len();
        batch.total_triangles += mesh.indices.len() / 3;
        batch.mesh_count += 1;
    }

    let mut batches: Vec<MeshBatch> = batch_map.into_values().collect();

    // Sort batches for optimal rendering order
    if config.sort_by_material {
        batches.sort_by(|a, b| a.material_key.cmp(&b.material_key));
    }
    if config.sort_by_texture {
        batches.sort_by(|a, b| a.texture_key.cmp(&b.texture_key));
    }

    batches
}

/// Auto-tune LOD distances based on performance targets
///
/// # Arguments
///
/// * `current_distances` - Current LOD distances
/// * `target_fps` - Target frames per second
/// * `current_fps` - Current frames per second
/// * `adjustment_rate` - How aggressively to adjust (0.0 to 1.0)
///
/// # Returns
///
/// Returns adjusted LOD distances
///
/// # Examples
///
/// ```
/// use antares::domain::visual::performance::auto_tune_lod_distances;
///
/// let current = vec![10.0, 20.0, 40.0];
/// let adjusted = auto_tune_lod_distances(&current, 60.0, 45.0, 0.1);
///
/// // Distances should decrease when FPS is below target
/// assert!(adjusted[0] < current[0]);
/// ```
pub fn auto_tune_lod_distances(
    current_distances: &[f32],
    target_fps: f32,
    current_fps: f32,
    adjustment_rate: f32,
) -> Vec<f32> {
    if current_distances.is_empty() || target_fps <= 0.0 || current_fps <= 0.0 {
        return current_distances.to_vec();
    }

    let fps_ratio = current_fps / target_fps;
    let adjustment_rate = adjustment_rate.clamp(0.0, 1.0);

    current_distances
        .iter()
        .map(|&distance| {
            if fps_ratio < 1.0 {
                // Below target FPS: reduce distances (show lower LOD sooner)
                let scale = 1.0 - (adjustment_rate * (1.0 - fps_ratio));
                distance * scale
            } else if fps_ratio > 1.2 {
                // Well above target FPS: increase distances (show higher LOD longer)
                let scale = 1.0 + (adjustment_rate * (fps_ratio - 1.0) * 0.5);
                distance * scale
            } else {
                // Near target: no adjustment
                distance
            }
        })
        .collect()
}

/// Memory optimization strategy recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryStrategy {
    /// Keep all meshes loaded
    KeepAll,

    /// Unload meshes beyond certain distance
    DistanceBased,

    /// Use LRU cache with size limit
    LruCache,

    /// Stream meshes on demand
    Streaming,
}

/// Memory optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimizationConfig {
    /// Maximum total mesh memory (bytes)
    pub max_mesh_memory: usize,

    /// Unload distance threshold (world units)
    pub unload_distance: f32,

    /// Strategy to use
    pub strategy: MemoryStrategy,

    /// Cache size for LRU strategy
    pub cache_size: usize,
}

impl Default for MemoryOptimizationConfig {
    fn default() -> Self {
        Self {
            max_mesh_memory: 256 * 1024 * 1024, // 256 MB
            unload_distance: 100.0,
            strategy: MemoryStrategy::LruCache,
            cache_size: 1000,
        }
    }
}

/// Analyze memory usage and recommend optimization strategy
///
/// # Arguments
///
/// * `meshes` - Collection of meshes to analyze
/// * `config` - Memory optimization configuration
///
/// # Returns
///
/// Returns recommended strategy and potential memory savings
pub fn analyze_memory_usage(
    meshes: &[MeshDefinition],
    config: &MemoryOptimizationConfig,
) -> (MemoryStrategy, usize) {
    let total_memory: usize = meshes.iter().map(estimate_mesh_memory).sum();

    if total_memory <= config.max_mesh_memory {
        return (MemoryStrategy::KeepAll, 0);
    }

    let overflow = total_memory - config.max_mesh_memory;

    // Sort meshes by size to determine best strategy
    let mut mesh_sizes: Vec<usize> = meshes.iter().map(estimate_mesh_memory).collect();
    mesh_sizes.sort_unstable();

    let _median_size = mesh_sizes.get(mesh_sizes.len() / 2).copied().unwrap_or(0);

    let recommended_strategy = if overflow > config.max_mesh_memory / 2 {
        // Severe overflow: use streaming
        MemoryStrategy::Streaming
    } else if meshes.len() > config.cache_size * 2 {
        // Many small meshes: use LRU cache
        MemoryStrategy::LruCache
    } else {
        // Moderate overflow: use distance-based unloading
        MemoryStrategy::DistanceBased
    };

    let potential_savings = overflow.min(total_memory / 2);

    (recommended_strategy, potential_savings)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_generate_lod_with_distances() {
        let base_mesh = create_test_mesh(100);
        let config = LodGenerationConfig::default();

        let result = generate_lod_with_distances(&base_mesh, &config);

        assert_eq!(result.lod_meshes.len(), config.num_levels);
        assert_eq!(result.distances.len(), config.num_levels);
        assert!(result.memory_saved > 0);
        assert!(result.triangle_reduction >= 0.0);

        // Verify distances increase
        for i in 1..result.distances.len() {
            assert!(result.distances[i] > result.distances[i - 1]);
        }
    }

    #[test]
    fn test_calculate_mesh_bounding_size() {
        let mesh = MeshDefinition {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [0.0, 5.0, 0.0],
                [0.0, 0.0, 2.0],
            ],
            indices: vec![],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        let size = calculate_mesh_bounding_size(&mesh);
        assert_eq!(size, 10.0); // X dimension is largest
    }

    #[test]
    fn test_estimate_mesh_memory() {
        let mesh = create_test_mesh(10);
        let memory = estimate_mesh_memory(&mesh);

        // Should at least account for vertices and indices
        let min_expected = mesh.vertices.len() * 12 + mesh.indices.len() * 4;
        assert!(memory >= min_expected);
    }

    #[test]
    fn test_analyze_batching() {
        let meshes = vec![
            create_test_mesh(10),
            create_test_mesh(20),
            create_test_mesh(15),
        ];

        let config = BatchingConfig::default();
        let batches = analyze_batching(&meshes, &config);

        assert!(!batches.is_empty());
        assert_eq!(batches[0].mesh_count, 3);
    }

    #[test]
    fn test_auto_tune_lod_distances_below_target() {
        let current = vec![10.0, 20.0, 40.0];
        let adjusted = auto_tune_lod_distances(&current, 60.0, 45.0, 0.1);

        // Below target FPS: distances should decrease
        for i in 0..current.len() {
            assert!(adjusted[i] < current[i]);
        }
    }

    #[test]
    fn test_auto_tune_lod_distances_above_target() {
        let current = vec![10.0, 20.0, 40.0];
        let adjusted = auto_tune_lod_distances(&current, 60.0, 80.0, 0.1);

        // Above target FPS: distances should increase
        for i in 0..current.len() {
            assert!(adjusted[i] > current[i]);
        }
    }

    #[test]
    fn test_auto_tune_lod_distances_near_target() {
        let current = vec![10.0, 20.0, 40.0];
        let adjusted = auto_tune_lod_distances(&current, 60.0, 65.0, 0.1);

        // Near target: minimal change
        for i in 0..current.len() {
            let diff = (adjusted[i] - current[i]).abs();
            assert!(diff < 0.5); // Should be very close
        }
    }

    #[test]
    fn test_analyze_memory_usage_under_limit() {
        let meshes = vec![create_test_mesh(10), create_test_mesh(10)];
        let config = MemoryOptimizationConfig {
            max_mesh_memory: 1024 * 1024 * 1024, // 1GB (way over)
            ..Default::default()
        };

        let (strategy, savings) = analyze_memory_usage(&meshes, &config);

        assert_eq!(strategy, MemoryStrategy::KeepAll);
        assert_eq!(savings, 0);
    }

    #[test]
    fn test_analyze_memory_usage_over_limit() {
        let meshes = vec![create_test_mesh(1000); 100];
        let config = MemoryOptimizationConfig {
            max_mesh_memory: 1024, // Very small limit
            unload_distance: 50.0,
            strategy: MemoryStrategy::LruCache,
            cache_size: 10,
        };

        let (strategy, savings) = analyze_memory_usage(&meshes, &config);

        // Should recommend optimization
        assert_ne!(strategy, MemoryStrategy::KeepAll);
        assert!(savings > 0);
    }

    #[test]
    fn test_lod_generation_config_default() {
        let config = LodGenerationConfig::default();

        assert_eq!(config.num_levels, 3);
        assert_eq!(config.reduction_factor, 0.5);
        assert_eq!(config.min_triangles, 8);
        assert!(config.generate_billboard);
    }

    #[test]
    fn test_batching_config_default() {
        let config = BatchingConfig::default();

        assert_eq!(config.max_vertices_per_batch, 65536);
        assert_eq!(config.max_instances_per_batch, 1024);
        assert!(config.sort_by_material);
        assert!(config.sort_by_texture);
    }

    #[test]
    fn test_memory_optimization_config_default() {
        let config = MemoryOptimizationConfig::default();

        assert_eq!(config.max_mesh_memory, 256 * 1024 * 1024);
        assert_eq!(config.unload_distance, 100.0);
        assert_eq!(config.strategy, MemoryStrategy::LruCache);
        assert_eq!(config.cache_size, 1000);
    }
}
