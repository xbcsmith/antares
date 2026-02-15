// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance optimization components for Bevy ECS
//!
//! This module provides components for:
//! - Mesh instancing (rendering many identical creatures efficiently)
//! - LOD (Level of Detail) state management
//! - Performance profiling markers

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component for instanced rendering of identical creatures
///
/// This component marks an entity as part of an instanced batch,
/// allowing many identical creatures to be rendered with a single draw call.
///
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::components::performance::InstancedCreature;
///
/// fn spawn_instanced_creatures(mut commands: Commands) {
///     // Spawn multiple instances with individual transforms
///     for i in 0..100 {
///         commands.spawn((
///             InstancedCreature {
///                 creature_id: 1000,
///                 instance_id: i,
///             },
///             Transform::from_xyz(i as f32 * 2.0, 0.0, 0.0),
///         ));
///     }
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InstancedCreature {
    /// Creature definition ID (for grouping)
    pub creature_id: u32,

    /// Unique instance ID within the batch
    pub instance_id: u32,
}

/// Per-instance data for batch rendering
///
/// This component stores instance-specific rendering data that can
/// vary per instance (transform, color tint, etc.)
#[derive(Component, Debug, Clone)]
pub struct InstanceData {
    /// Instance transform (position, rotation, scale)
    pub transform: Transform,

    /// Optional color tint for this instance
    pub color_tint: Option<Color>,

    /// Visibility flag
    pub visible: bool,
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            color_tint: None,
            visible: true,
        }
    }
}

/// LOD (Level of Detail) state component
///
/// Tracks the current LOD level for an entity and manages LOD transitions.
///
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::components::performance::LodState;
///
/// fn create_creature_with_lod(mut commands: Commands) {
///     commands.spawn((
///         LodState {
///             current_level: 0,
///             num_levels: 3,
///             distances: vec![10.0, 25.0, 50.0],
///             auto_switch: true,
///         },
///         Transform::default(),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone)]
pub struct LodState {
    /// Current LOD level (0 = highest detail)
    pub current_level: usize,

    /// Total number of LOD levels
    pub num_levels: usize,

    /// Distance thresholds for each LOD level
    pub distances: Vec<f32>,

    /// Whether to automatically switch LOD based on camera distance
    pub auto_switch: bool,
}

impl Default for LodState {
    fn default() -> Self {
        Self {
            current_level: 0,
            num_levels: 1,
            distances: vec![],
            auto_switch: true,
        }
    }
}

impl LodState {
    /// Create a new LOD state with the given distances
    pub fn new(distances: Vec<f32>) -> Self {
        Self {
            current_level: 0,
            num_levels: distances.len() + 1,
            distances,
            auto_switch: true,
        }
    }

    /// Update LOD level based on distance to camera
    ///
    /// Returns true if the LOD level changed
    pub fn update_for_distance(&mut self, distance: f32) -> bool {
        if !self.auto_switch || self.distances.is_empty() {
            return false;
        }

        let new_level = self
            .distances
            .iter()
            .position(|&d| distance < d)
            .unwrap_or(self.num_levels - 1);

        let changed = new_level != self.current_level;
        self.current_level = new_level;
        changed
    }

    /// Get the current LOD distance threshold
    pub fn current_distance(&self) -> Option<f32> {
        self.distances.get(self.current_level).copied()
    }
}

/// Marker component for entities that should be culled when far from camera
#[derive(Component, Debug, Clone, Copy)]
pub struct DistanceCulling {
    /// Maximum distance before culling (world units)
    pub max_distance: f32,

    /// Whether entity is currently culled
    pub culled: bool,
}

impl Default for DistanceCulling {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            culled: false,
        }
    }
}

/// Performance profiling marker component
///
/// Marks entities that should be included in performance profiling
#[derive(Component, Debug, Clone, Copy)]
pub struct PerformanceMarker {
    /// Category for grouping in profiling reports
    pub category: PerformanceCategory,

    /// Whether to include in detailed profiling
    pub detailed: bool,
}

/// Performance profiling categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerformanceCategory {
    /// Creature/monster entities
    Creature,

    /// Static environment meshes
    Environment,

    /// UI elements
    UI,

    /// Particle effects
    Particles,

    /// Other
    Other,
}

/// Mesh streaming component for memory optimization
///
/// Manages loading/unloading of mesh data based on distance
#[derive(Component, Debug, Clone)]
pub struct MeshStreaming {
    /// Whether mesh data is currently loaded
    pub loaded: bool,

    /// Distance threshold for loading
    pub load_distance: f32,

    /// Distance threshold for unloading
    pub unload_distance: f32,

    /// Priority (higher = load first)
    pub priority: i32,
}

impl Default for MeshStreaming {
    fn default() -> Self {
        Self {
            loaded: false,
            load_distance: 50.0,
            unload_distance: 100.0,
            priority: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_state_new() {
        let distances = vec![10.0, 20.0, 40.0];
        let state = LodState::new(distances.clone());

        assert_eq!(state.current_level, 0);
        assert_eq!(state.num_levels, 4); // distances.len() + 1
        assert_eq!(state.distances, distances);
        assert!(state.auto_switch);
    }

    #[test]
    fn test_lod_state_update_close() {
        let mut state = LodState::new(vec![10.0, 20.0, 40.0]);
        // Start at a different level
        state.current_level = 1;

        let changed = state.update_for_distance(5.0);

        assert!(changed);
        assert_eq!(state.current_level, 0);
    }

    #[test]
    fn test_lod_state_update_medium() {
        let mut state = LodState::new(vec![10.0, 20.0, 40.0]);
        // Already at level 0, should change to 1

        let changed = state.update_for_distance(15.0);

        assert!(changed);
        assert_eq!(state.current_level, 1);
    }

    #[test]
    fn test_lod_state_update_far() {
        let mut state = LodState::new(vec![10.0, 20.0, 40.0]);
        // Already at level 0, should change to 3

        let changed = state.update_for_distance(50.0);

        assert!(changed);
        assert_eq!(state.current_level, 3); // Furthest LOD
    }

    #[test]
    fn test_lod_state_no_change() {
        let mut state = LodState::new(vec![10.0, 20.0, 40.0]);
        state.update_for_distance(5.0);

        let changed = state.update_for_distance(7.0);

        assert!(!changed); // Still at level 0
        assert_eq!(state.current_level, 0);
    }

    #[test]
    fn test_lod_state_auto_switch_disabled() {
        let mut state = LodState::new(vec![10.0, 20.0, 40.0]);
        state.auto_switch = false;

        let changed = state.update_for_distance(50.0);

        assert!(!changed);
        assert_eq!(state.current_level, 0); // Unchanged
    }

    #[test]
    fn test_lod_state_current_distance() {
        let state = LodState::new(vec![10.0, 20.0, 40.0]);

        assert_eq!(state.current_distance(), Some(10.0));
    }

    #[test]
    fn test_distance_culling_default() {
        let culling = DistanceCulling::default();

        assert_eq!(culling.max_distance, 100.0);
        assert!(!culling.culled);
    }

    #[test]
    fn test_mesh_streaming_default() {
        let streaming = MeshStreaming::default();

        assert!(!streaming.loaded);
        assert_eq!(streaming.load_distance, 50.0);
        assert_eq!(streaming.unload_distance, 100.0);
        assert_eq!(streaming.priority, 0);
    }

    #[test]
    fn test_instance_data_default() {
        let data = InstanceData::default();

        assert!(data.visible);
        assert!(data.color_tint.is_none());
    }
}
