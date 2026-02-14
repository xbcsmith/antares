// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance metrics and profiling resources
//!
//! This module provides global resources for tracking and optimizing
//! rendering performance, including frame time tracking, LOD metrics,
//! and instancing statistics.

use bevy::prelude::*;
use std::collections::HashMap;

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
///     metrics.update_frame_time(time.delta_seconds());
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
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0)
            * 1000.0
    }

    /// Get maximum frame time (worst case)
    pub fn max_frame_time_ms(&self) -> f32 {
        self.frame_times
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
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
}
