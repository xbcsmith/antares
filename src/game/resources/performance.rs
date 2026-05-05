// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance metrics and profiling resources
//!
//! This module provides global resources for tracking and optimizing
//! rendering performance, including frame time tracking, LOD metrics,
//! and instancing statistics.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default first tree LOD switch distance in world units.
pub const DEFAULT_TREE_LOD_DISTANCE_1: f32 = 18.0;

/// Default second tree LOD switch distance in world units.
pub const DEFAULT_TREE_LOD_DISTANCE_2: f32 = 34.0;

/// Default grass LOD switch distance in world units.
pub const DEFAULT_GRASS_LOD_DISTANCE: f32 = 15.0;

/// Default maximum vegetation draw distance in world units.
pub const DEFAULT_VEGETATION_CULL_DISTANCE: f32 = 45.0;

/// Default cap for cached procedural tree mesh variants per species.
pub const DEFAULT_MAX_TREE_MESH_VARIANTS_PER_SPECIES: usize = 8;

/// Default cap for cached grass material variants.
pub const DEFAULT_MAX_GRASS_MATERIAL_VARIANTS: usize = 64;

/// Vegetation-wide visual quality levels.
///
/// This render-only setting changes geometry detail, LOD distances, density
/// scaling, and cache budgets without mutating map data or gameplay state.
///
/// # Examples
///
/// ```
/// use antares::game::resources::performance::VegetationQualityLevel;
///
/// assert_eq!(VegetationQualityLevel::Medium.name(), "Medium");
/// assert!(VegetationQualityLevel::Low.density_multiplier() < 1.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VegetationQualityLevel {
    /// Low quality uses shorter draw distances, fewer variants, and lower density.
    Low,

    /// Medium quality is the balanced default.
    #[default]
    Medium,

    /// High quality uses longer draw distances and full near-field detail.
    High,
}

impl VegetationQualityLevel {
    /// Returns a human-readable quality-level name.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::performance::VegetationQualityLevel;
    ///
    /// assert_eq!(VegetationQualityLevel::High.name(), "High");
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }

    /// Returns the deterministic density multiplier for vegetation spawns.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::performance::VegetationQualityLevel;
    ///
    /// assert_eq!(VegetationQualityLevel::Medium.density_multiplier(), 1.0);
    /// ```
    pub fn density_multiplier(self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 1.0,
            Self::High => 1.5,
        }
    }

    /// Returns the matching grass performance level.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
    /// use antares::game::resources::performance::VegetationQualityLevel;
    ///
    /// assert_eq!(
    ///     VegetationQualityLevel::Low.grass_performance_level(),
    ///     GrassPerformanceLevel::Low
    /// );
    /// ```
    pub fn grass_performance_level(
        self,
    ) -> crate::game::resources::grass_quality_settings::GrassPerformanceLevel {
        match self {
            Self::Low => crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Low,
            Self::Medium => {
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium
            }
            Self::High => {
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::High
            }
        }
    }
}

/// Render-only quality, LOD, and cache-budget settings for vegetation.
///
/// The settings apply to procedural trees, shrubs, and grass. They deliberately
/// live in the game-resource layer so authors can tune runtime visual quality
/// without changing domain map data.
///
/// # Examples
///
/// ```
/// use antares::game::resources::performance::{
///     VegetationQualityLevel, VegetationQualitySettings,
/// };
///
/// let low = VegetationQualitySettings::for_level(VegetationQualityLevel::Low);
/// let high = VegetationQualitySettings::for_level(VegetationQualityLevel::High);
/// assert!(low.vegetation_cull_distance < high.vegetation_cull_distance);
/// assert!(low.max_tree_mesh_variants_per_species < high.max_tree_mesh_variants_per_species);
/// ```
#[derive(Resource, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VegetationQualitySettings {
    /// Vegetation-wide quality preset.
    pub quality_level: VegetationQualityLevel,

    /// Distance at which trees switch from LOD0 to LOD1.
    pub tree_lod_distance_1: f32,

    /// Distance at which trees switch from LOD1 to LOD2.
    pub tree_lod_distance_2: f32,

    /// Distance at which grass switches from near to mid/far LOD.
    pub grass_lod_distance: f32,

    /// Maximum distance at which vegetation remains visible.
    pub vegetation_cull_distance: f32,

    /// Maximum reusable mesh variant buckets per species and quality level.
    pub max_tree_mesh_variants_per_species: usize,

    /// Maximum reusable grass material buckets.
    pub max_grass_material_variants: usize,
}

impl Default for VegetationQualitySettings {
    fn default() -> Self {
        Self::for_level(VegetationQualityLevel::Medium)
    }
}

impl VegetationQualitySettings {
    /// Builds deterministic settings for a quality preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::performance::{
    ///     VegetationQualityLevel, VegetationQualitySettings,
    /// };
    ///
    /// let settings = VegetationQualitySettings::for_level(VegetationQualityLevel::Low);
    /// assert_eq!(settings.quality_level, VegetationQualityLevel::Low);
    /// ```
    pub fn for_level(quality_level: VegetationQualityLevel) -> Self {
        match quality_level {
            VegetationQualityLevel::Low => Self {
                quality_level,
                tree_lod_distance_1: 10.0,
                tree_lod_distance_2: 22.0,
                grass_lod_distance: 8.0,
                vegetation_cull_distance: 28.0,
                max_tree_mesh_variants_per_species: 2,
                max_grass_material_variants: 16,
            },
            VegetationQualityLevel::Medium => Self {
                quality_level,
                tree_lod_distance_1: DEFAULT_TREE_LOD_DISTANCE_1,
                tree_lod_distance_2: DEFAULT_TREE_LOD_DISTANCE_2,
                grass_lod_distance: DEFAULT_GRASS_LOD_DISTANCE,
                vegetation_cull_distance: DEFAULT_VEGETATION_CULL_DISTANCE,
                max_tree_mesh_variants_per_species: DEFAULT_MAX_TREE_MESH_VARIANTS_PER_SPECIES,
                max_grass_material_variants: DEFAULT_MAX_GRASS_MATERIAL_VARIANTS,
            },
            VegetationQualityLevel::High => Self {
                quality_level,
                tree_lod_distance_1: 28.0,
                tree_lod_distance_2: 52.0,
                grass_lod_distance: 24.0,
                vegetation_cull_distance: 72.0,
                max_tree_mesh_variants_per_species: 8,
                max_grass_material_variants: 64,
            },
        }
    }

    /// Returns grass quality settings derived from this vegetation preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
    /// use antares::game::resources::performance::{
    ///     VegetationQualityLevel, VegetationQualitySettings,
    /// };
    ///
    /// let settings = VegetationQualitySettings::for_level(VegetationQualityLevel::High);
    /// assert_eq!(
    ///     settings.grass_quality_settings().performance_level,
    ///     GrassPerformanceLevel::High
    /// );
    /// ```
    pub fn grass_quality_settings(
        self,
    ) -> crate::game::resources::grass_quality_settings::GrassQualitySettings {
        crate::game::resources::grass_quality_settings::GrassQualitySettings {
            performance_level: self.quality_level.grass_performance_level(),
        }
    }

    /// Returns whether tree LOD distances are strictly ordered and culling is farthest.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::performance::VegetationQualitySettings;
    ///
    /// assert!(VegetationQualitySettings::default().has_valid_lod_order());
    /// ```
    pub fn has_valid_lod_order(self) -> bool {
        self.tree_lod_distance_1 > 0.0
            && self.tree_lod_distance_2 > self.tree_lod_distance_1
            && self.vegetation_cull_distance > self.tree_lod_distance_2
            && self.grass_lod_distance > 0.0
            && self.vegetation_cull_distance > self.grass_lod_distance
    }
}

/// Global performance metrics resource
///
/// Tracks real-time performance data for optimization and debugging.
///
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::resources::performance::PerformanceMetrics;
///
/// fn update_metrics(mut metrics: ResMut<PerformanceMetrics>, time: Res<Time>) {
///     metrics.update_frame_time(time.delta_secs());
///
///     println!("Current FPS: {:.1}", metrics.current_fps());
/// }
/// ```
#[derive(Resource, Debug, Clone)]
pub struct PerformanceMetrics {
    /// Recent frame times (seconds) for averaging
    frame_times: Vec<f32>,

    /// Maximum samples to keep
    max_samples: usize,

    /// Total entities rendered this frame
    pub entities_rendered: usize,

    /// Total triangles rendered this frame
    pub triangles_rendered: usize,

    /// Total draw calls this frame
    pub draw_calls: usize,

    /// LOD statistics by level
    pub lod_stats: HashMap<usize, LodLevelStats>,

    /// Instancing statistics
    pub instancing_stats: InstancingStats,

    /// Memory usage estimate (bytes)
    pub memory_usage: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            frame_times: Vec::with_capacity(60),
            max_samples: 60,
            entities_rendered: 0,
            triangles_rendered: 0,
            draw_calls: 0,
            lod_stats: HashMap::new(),
            instancing_stats: InstancingStats::default(),
            memory_usage: 0,
        }
    }
}

impl PerformanceMetrics {
    /// Create new performance metrics tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with new frame time
    pub fn update_frame_time(&mut self, delta_seconds: f32) {
        self.frame_times.push(delta_seconds);

        // Keep only recent samples
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }

    /// Get current FPS (frames per second)
    pub fn current_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let avg_frame_time: f32 =
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;

        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Get average frame time in milliseconds
    pub fn avg_frame_time_ms(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let avg = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        avg * 1000.0
    }

    /// Get minimum frame time (best case)
    pub fn min_frame_time_ms(&self) -> f32 {
        self.frame_times
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .copied()
            .unwrap_or(0.0)
            * 1000.0
    }

    /// Get maximum frame time (worst case)
    pub fn max_frame_time_ms(&self) -> f32 {
        self.frame_times
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .copied()
            .unwrap_or(0.0)
            * 1000.0
    }

    /// Reset per-frame counters
    pub fn reset_frame_counters(&mut self) {
        self.entities_rendered = 0;
        self.triangles_rendered = 0;
        self.draw_calls = 0;
        self.lod_stats.clear();
        self.instancing_stats = InstancingStats::default();
    }

    /// Record LOD level usage
    pub fn record_lod_level(&mut self, level: usize, triangles: usize) {
        let stats = self.lod_stats.entry(level).or_default();
        stats.count += 1;
        stats.total_triangles += triangles;
    }

    /// Get total entities across all LOD levels
    pub fn total_lod_entities(&self) -> usize {
        self.lod_stats.values().map(|s| s.count).sum()
    }
}

/// Statistics for a single LOD level
#[derive(Debug, Clone, Default)]
pub struct LodLevelStats {
    /// Number of entities at this LOD level
    pub count: usize,

    /// Total triangles rendered at this level
    pub total_triangles: usize,
}

/// Instancing statistics
#[derive(Debug, Clone, Default)]
pub struct InstancingStats {
    /// Number of instanced batches
    pub batch_count: usize,

    /// Total instances rendered
    pub instance_count: usize,

    /// Draw calls saved by instancing
    pub draw_calls_saved: usize,
}

/// LOD auto-tuning configuration resource
///
/// Controls automatic adjustment of LOD distances based on performance.
#[derive(Resource, Debug, Clone)]
pub struct LodAutoTuning {
    /// Whether auto-tuning is enabled
    pub enabled: bool,

    /// Target FPS for auto-tuning
    pub target_fps: f32,

    /// Adjustment rate (0.0 to 1.0)
    pub adjustment_rate: f32,

    /// Minimum distance multiplier
    pub min_distance_scale: f32,

    /// Maximum distance multiplier
    pub max_distance_scale: f32,

    /// Current distance scale factor
    pub current_scale: f32,

    /// Time since last adjustment
    pub time_since_adjustment: f32,

    /// Minimum time between adjustments
    pub adjustment_interval: f32,
}

impl Default for LodAutoTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            target_fps: 60.0,
            adjustment_rate: 0.1,
            min_distance_scale: 0.5,
            max_distance_scale: 2.0,
            current_scale: 1.0,
            time_since_adjustment: 0.0,
            adjustment_interval: 1.0, // Adjust at most once per second
        }
    }
}

impl LodAutoTuning {
    /// Create new auto-tuning configuration
    pub fn new(target_fps: f32) -> Self {
        Self {
            target_fps,
            ..Default::default()
        }
    }

    /// Update auto-tuning based on current FPS
    ///
    /// Returns true if scale was adjusted
    pub fn update(&mut self, current_fps: f32, delta_seconds: f32) -> bool {
        if !self.enabled {
            return false;
        }

        self.time_since_adjustment += delta_seconds;

        if self.time_since_adjustment < self.adjustment_interval {
            return false;
        }

        self.time_since_adjustment = 0.0;

        let fps_ratio = current_fps / self.target_fps;
        let old_scale = self.current_scale;

        if fps_ratio < 0.9 {
            // Below target: reduce distances (show lower LOD sooner)
            self.current_scale *= 1.0 - self.adjustment_rate;
            self.current_scale = self.current_scale.max(self.min_distance_scale);
        } else if fps_ratio > 1.2 {
            // Well above target: increase distances (show higher LOD longer)
            self.current_scale *= 1.0 + self.adjustment_rate * 0.5;
            self.current_scale = self.current_scale.min(self.max_distance_scale);
        }

        (self.current_scale - old_scale).abs() > 0.001
    }

    /// Get scaled distance
    pub fn scale_distance(&self, base_distance: f32) -> f32 {
        base_distance * self.current_scale
    }
}

/// Mesh cache resource for avoiding redundant mesh generation
#[derive(Resource, Debug, Default)]
pub struct MeshCache {
    /// Cache of generated mesh handles by creature ID
    cache: HashMap<u32, Handle<Mesh>>,

    /// Cache hit/miss statistics
    pub hits: usize,
    pub misses: usize,
}

impl MeshCache {
    /// Create new mesh cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Get cached mesh handle if available
    pub fn get(&mut self, creature_id: u32) -> Option<Handle<Mesh>> {
        if let Some(handle) = self.cache.get(&creature_id) {
            self.hits += 1;
            Some(handle.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert mesh into cache
    pub fn insert(&mut self, creature_id: u32, mesh: Handle<Mesh>) {
        self.cache.insert(creature_id, mesh);
    }

    /// Clear all cached meshes
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vegetation_quality_level_names() {
        assert_eq!(VegetationQualityLevel::Low.name(), "Low");
        assert_eq!(VegetationQualityLevel::Medium.name(), "Medium");
        assert_eq!(VegetationQualityLevel::High.name(), "High");
    }

    #[test]
    fn test_vegetation_quality_level_density_multipliers_are_ordered() {
        assert!(
            VegetationQualityLevel::Low.density_multiplier()
                < VegetationQualityLevel::Medium.density_multiplier()
        );
        assert!(
            VegetationQualityLevel::Medium.density_multiplier()
                < VegetationQualityLevel::High.density_multiplier()
        );
    }

    #[test]
    fn test_vegetation_quality_level_maps_to_grass_performance_level() {
        use crate::game::resources::grass_quality_settings::GrassPerformanceLevel;

        assert_eq!(
            VegetationQualityLevel::Low.grass_performance_level(),
            GrassPerformanceLevel::Low
        );
        assert_eq!(
            VegetationQualityLevel::Medium.grass_performance_level(),
            GrassPerformanceLevel::Medium
        );
        assert_eq!(
            VegetationQualityLevel::High.grass_performance_level(),
            GrassPerformanceLevel::High
        );
    }

    #[test]
    fn test_vegetation_quality_settings_default_is_medium() {
        let settings = VegetationQualitySettings::default();

        assert_eq!(settings.quality_level, VegetationQualityLevel::Medium);
        assert_eq!(settings.tree_lod_distance_1, DEFAULT_TREE_LOD_DISTANCE_1);
        assert_eq!(settings.tree_lod_distance_2, DEFAULT_TREE_LOD_DISTANCE_2);
        assert_eq!(settings.grass_lod_distance, DEFAULT_GRASS_LOD_DISTANCE);
        assert_eq!(
            settings.vegetation_cull_distance,
            DEFAULT_VEGETATION_CULL_DISTANCE
        );
    }

    #[test]
    fn test_vegetation_quality_settings_lod_distances_are_valid_for_all_levels() {
        for level in [
            VegetationQualityLevel::Low,
            VegetationQualityLevel::Medium,
            VegetationQualityLevel::High,
        ] {
            let settings = VegetationQualitySettings::for_level(level);
            assert!(
                settings.has_valid_lod_order(),
                "{level:?} vegetation LOD distances must be ordered"
            );
        }
    }

    #[test]
    fn test_vegetation_quality_settings_low_reduces_budgets_vs_high() {
        let low = VegetationQualitySettings::for_level(VegetationQualityLevel::Low);
        let high = VegetationQualitySettings::for_level(VegetationQualityLevel::High);

        assert!(low.tree_lod_distance_1 < high.tree_lod_distance_1);
        assert!(low.tree_lod_distance_2 < high.tree_lod_distance_2);
        assert!(low.grass_lod_distance < high.grass_lod_distance);
        assert!(low.vegetation_cull_distance < high.vegetation_cull_distance);
        assert!(low.max_tree_mesh_variants_per_species < high.max_tree_mesh_variants_per_species);
        assert!(low.max_grass_material_variants < high.max_grass_material_variants);
    }

    #[test]
    fn test_vegetation_quality_settings_derives_grass_quality_settings() {
        use crate::game::resources::grass_quality_settings::GrassPerformanceLevel;

        let settings = VegetationQualitySettings::for_level(VegetationQualityLevel::Low);

        assert_eq!(
            settings.grass_quality_settings().performance_level,
            GrassPerformanceLevel::Low
        );
    }

    #[test]
    fn test_performance_metrics_fps() {
        let mut metrics = PerformanceMetrics::new();

        // Simulate 60 FPS (16.67ms per frame)
        for _ in 0..10 {
            metrics.update_frame_time(1.0 / 60.0);
        }

        let fps = metrics.current_fps();
        assert!((fps - 60.0).abs() < 1.0);
    }

    #[test]
    fn test_performance_metrics_frame_time() {
        let mut metrics = PerformanceMetrics::new();

        metrics.update_frame_time(0.016);
        metrics.update_frame_time(0.017);
        metrics.update_frame_time(0.015);

        let avg_ms = metrics.avg_frame_time_ms();
        assert!((avg_ms - 16.0).abs() < 1.0);
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut metrics = PerformanceMetrics::new();

        metrics.entities_rendered = 100;
        metrics.triangles_rendered = 5000;
        metrics.draw_calls = 50;

        metrics.reset_frame_counters();

        assert_eq!(metrics.entities_rendered, 0);
        assert_eq!(metrics.triangles_rendered, 0);
        assert_eq!(metrics.draw_calls, 0);
    }

    #[test]
    fn test_lod_auto_tuning_below_target() {
        let mut tuning = LodAutoTuning::new(60.0);

        let changed = tuning.update(45.0, 1.5);

        assert!(changed);
        assert!(tuning.current_scale < 1.0);
    }

    #[test]
    fn test_lod_auto_tuning_above_target() {
        let mut tuning = LodAutoTuning::new(60.0);

        let changed = tuning.update(80.0, 1.5);

        assert!(changed);
        assert!(tuning.current_scale > 1.0);
    }

    #[test]
    fn test_lod_auto_tuning_disabled() {
        let mut tuning = LodAutoTuning::new(60.0);
        tuning.enabled = false;

        let changed = tuning.update(30.0, 1.5);

        assert!(!changed);
        assert_eq!(tuning.current_scale, 1.0);
    }

    #[test]
    fn test_lod_auto_tuning_interval() {
        let mut tuning = LodAutoTuning::new(60.0);
        tuning.adjustment_interval = 2.0;

        let changed1 = tuning.update(45.0, 0.5);
        assert!(!changed1); // Too soon

        let changed2 = tuning.update(45.0, 2.0);
        assert!(changed2); // Enough time passed
    }

    #[test]
    fn test_mesh_cache() {
        let mut cache = MeshCache::new();
        let mesh_handle = Handle::default();

        assert_eq!(cache.get(1000), None);
        assert_eq!(cache.misses, 1);

        cache.insert(1000, mesh_handle.clone());

        assert!(cache.get(1000).is_some());
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_mesh_cache_hit_rate() {
        let mut cache = MeshCache::new();
        let mesh_handle = Handle::default();

        cache.insert(1000, mesh_handle);

        cache.get(1000); // Hit
        cache.get(1000); // Hit
        cache.get(2000); // Miss

        assert_eq!(cache.hit_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_lod_stats_recording() {
        let mut metrics = PerformanceMetrics::new();

        metrics.record_lod_level(0, 1000);
        metrics.record_lod_level(0, 1000);
        metrics.record_lod_level(1, 500);

        assert_eq!(metrics.lod_stats.get(&0).unwrap().count, 2);
        assert_eq!(metrics.lod_stats.get(&0).unwrap().total_triangles, 2000);
        assert_eq!(metrics.lod_stats.get(&1).unwrap().count, 1);
        assert_eq!(metrics.total_lod_entities(), 3);
    }

    #[test]
    fn test_min_frame_time_handles_nan() {
        let mut metrics = PerformanceMetrics::new();
        metrics.update_frame_time(f32::NAN);
        metrics.update_frame_time(0.016);
        // Should not panic — NaN sorts to one end with total_cmp
        let _min = metrics.min_frame_time_ms();
    }

    #[test]
    fn test_max_frame_time_handles_nan() {
        let mut metrics = PerformanceMetrics::new();
        metrics.update_frame_time(f32::NAN);
        metrics.update_frame_time(0.016);
        // Should not panic — NaN sorts to one end with total_cmp
        let _max = metrics.max_frame_time_ms();
    }
}
