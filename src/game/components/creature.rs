// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Bevy ECS components for creature visual rendering
//!
//! This module provides components that link game entities to procedurally-generated
//! creature meshes defined in the domain layer. These components integrate the
//! domain-level `CreatureDefinition` with Bevy's ECS rendering pipeline.
//!
//! # Component Hierarchy
//!
//! Creatures are spawned as hierarchical entities:
//! - Parent entity: `CreatureVisual` component (references CreatureDefinition)
//! - Child entities: `MeshPart` components (one per mesh in creature definition)
//!
//! # Examples
//!
//! ```
//! use antares::game::components::creature::{CreatureVisual, MeshPart};
//! use antares::domain::types::CreatureId;
//! use bevy::prelude::*;
//!
//! fn spawn_example(mut commands: Commands) {
//!     let creature_id: CreatureId = 42;
//!
//!     // Spawn parent entity with CreatureVisual
//!     let parent = commands.spawn(CreatureVisual {
//!         creature_id,
//!         scale_override: None,
//!     }).id();
//!
//!     // Spawn child entities for each mesh part
//!     commands.spawn((
//!         MeshPart {
//!             creature_id,
//!             mesh_index: 0,
//!             material_override: None,
//!         },
//!         Transform::default(),
//!     )).set_parent_in_place(parent);
//! }
//! ```

use crate::domain::types::CreatureId;
use bevy::prelude::*;

/// Component linking an entity to a creature visual definition
///
/// This component is attached to the parent entity of a creature visual.
/// Child entities contain the actual mesh parts (see `MeshPart`).
///
/// # Fields
///
/// * `creature_id` - References a `CreatureDefinition` in the content database
/// * `scale_override` - Optional scale multiplier (overrides creature definition scale)
///
/// # Examples
///
/// ```
/// use antares::game::components::creature::CreatureVisual;
/// use antares::domain::types::CreatureId;
/// use bevy::prelude::*;
///
/// fn spawn_scaled_creature(mut commands: Commands) {
///     let creature_id: CreatureId = 10;
///
///     commands.spawn(CreatureVisual {
///         creature_id,
///         scale_override: Some(2.0), // Double the size
///     });
/// }
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct CreatureVisual {
    /// ID of the creature definition in the content database
    pub creature_id: CreatureId,

    /// Optional scale multiplier (1.0 = normal size)
    ///
    /// If `Some(scale)`, this overrides the scale defined in `CreatureDefinition`.
    /// If `None`, uses the scale from the creature definition.
    pub scale_override: Option<f32>,
}

impl CreatureVisual {
    /// Creates a new creature visual reference
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature definition to render
    ///
    /// # Returns
    ///
    /// `CreatureVisual` component with no scale override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureVisual;
    /// use antares::domain::types::CreatureId;
    ///
    /// let visual = CreatureVisual::new(42);
    /// assert_eq!(visual.creature_id, 42);
    /// assert_eq!(visual.scale_override, None);
    /// ```
    pub fn new(creature_id: CreatureId) -> Self {
        Self {
            creature_id,
            scale_override: None,
        }
    }

    /// Creates a creature visual with a scale override
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature definition to render
    /// * `scale` - Scale multiplier (1.0 = normal size)
    ///
    /// # Returns
    ///
    /// `CreatureVisual` component with the specified scale
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureVisual;
    ///
    /// let visual = CreatureVisual::with_scale(10, 1.5);
    /// assert_eq!(visual.creature_id, 10);
    /// assert_eq!(visual.scale_override, Some(1.5));
    /// ```
    pub fn with_scale(creature_id: CreatureId, scale: f32) -> Self {
        Self {
            creature_id,
            scale_override: Some(scale),
        }
    }

    /// Gets the effective scale value
    ///
    /// # Arguments
    ///
    /// * `definition_scale` - The scale defined in the creature definition
    ///
    /// # Returns
    ///
    /// The scale to use (override if present, otherwise definition scale)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureVisual;
    ///
    /// let visual_no_override = CreatureVisual::new(1);
    /// assert_eq!(visual_no_override.effective_scale(2.0), 2.0);
    ///
    /// let visual_with_override = CreatureVisual::with_scale(1, 3.0);
    /// assert_eq!(visual_with_override.effective_scale(2.0), 3.0);
    /// ```
    pub fn effective_scale(&self, definition_scale: f32) -> f32 {
        self.scale_override.unwrap_or(definition_scale)
    }
}

/// Component representing one mesh part of a multi-mesh creature
///
/// Creatures can have multiple meshes (e.g., body, head, limbs). Each mesh
/// is spawned as a separate child entity with this component.
///
/// # Fields
///
/// * `creature_id` - References the parent creature definition
/// * `mesh_index` - Index into the creature's `meshes` array
/// * `material_override` - Optional material handle (overrides mesh color)
///
/// # Examples
///
/// ```
/// use antares::game::components::creature::MeshPart;
/// use antares::domain::types::CreatureId;
/// use bevy::prelude::*;
///
/// fn spawn_multi_mesh_creature(mut commands: Commands) {
///     let creature_id: CreatureId = 20;
///
///     // Spawn parent
///     let parent = commands.spawn_empty().id();
///
///     // Spawn first mesh (e.g., body)
///     commands.spawn(MeshPart::new(creature_id, 0))
///         .set_parent_in_place(parent);
///
///     // Spawn second mesh (e.g., head)
///     commands.spawn(MeshPart::new(creature_id, 1))
///         .set_parent_in_place(parent);
/// }
/// ```
#[derive(Component, Debug, Clone)]
pub struct MeshPart {
    /// ID of the creature definition
    pub creature_id: CreatureId,

    /// Index into the creature's meshes array
    pub mesh_index: usize,

    /// Optional material override
    ///
    /// If `Some(handle)`, this material is used instead of the color
    /// defined in the mesh definition. Useful for damage effects,
    /// status indicators, or dynamic recoloring.
    pub material_override: Option<Handle<StandardMaterial>>,
}

impl MeshPart {
    /// Creates a new mesh part reference
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature definition
    /// * `mesh_index` - Index of the mesh within the creature definition
    ///
    /// # Returns
    ///
    /// `MeshPart` component with no material override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::MeshPart;
    ///
    /// let part = MeshPart::new(42, 0);
    /// assert_eq!(part.creature_id, 42);
    /// assert_eq!(part.mesh_index, 0);
    /// assert!(part.material_override.is_none());
    /// ```
    pub fn new(creature_id: CreatureId, mesh_index: usize) -> Self {
        Self {
            creature_id,
            mesh_index,
            material_override: None,
        }
    }

    /// Creates a mesh part with a material override
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature definition
    /// * `mesh_index` - Index of the mesh within the creature definition
    /// * `material` - Material handle to override default color
    ///
    /// # Returns
    ///
    /// `MeshPart` component with the specified material
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::MeshPart;
    /// use bevy::prelude::*;
    ///
    /// fn spawn_with_override(mut commands: Commands, material: Handle<StandardMaterial>) {
    ///     let part = MeshPart::with_material(10, 0, material.clone());
    ///     assert_eq!(part.creature_id, 10);
    ///     assert_eq!(part.mesh_index, 0);
    ///     assert!(part.material_override.is_some());
    /// }
    /// ```
    pub fn with_material(
        creature_id: CreatureId,
        mesh_index: usize,
        material: Handle<StandardMaterial>,
    ) -> Self {
        Self {
            creature_id,
            mesh_index,
            material_override: Some(material),
        }
    }
}

/// Marker component for spawning a creature
///
/// This is a request component that triggers the creature spawning system.
/// Once processed, the system removes this component and creates the actual
/// creature visual hierarchy.
///
/// # Fields
///
/// * `creature_id` - ID of the creature to spawn
/// * `position` - World position to spawn at
/// * `scale_override` - Optional scale multiplier
///
/// # Examples
///
/// ```
/// use antares::game::components::creature::SpawnCreatureRequest;
/// use bevy::prelude::*;
///
/// fn request_spawn(mut commands: Commands) {
///     commands.spawn(SpawnCreatureRequest {
///         creature_id: 42,
///         position: Vec3::new(10.0, 0.0, 5.0),
///         scale_override: None,
///     });
/// }
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpawnCreatureRequest {
    /// ID of the creature to spawn
    pub creature_id: CreatureId,

    /// World position to spawn the creature at
    pub position: Vec3,

    /// Optional scale multiplier
    pub scale_override: Option<f32>,
}

impl SpawnCreatureRequest {
    /// Creates a new spawn request
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature to spawn
    /// * `position` - World position
    ///
    /// # Returns
    ///
    /// Spawn request with no scale override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::SpawnCreatureRequest;
    /// use bevy::prelude::*;
    ///
    /// let request = SpawnCreatureRequest::new(42, Vec3::ZERO);
    /// assert_eq!(request.creature_id, 42);
    /// assert_eq!(request.position, Vec3::ZERO);
    /// ```
    pub fn new(creature_id: CreatureId, position: Vec3) -> Self {
        Self {
            creature_id,
            position,
            scale_override: None,
        }
    }

    /// Creates a spawn request with scale override
    ///
    /// # Arguments
    ///
    /// * `creature_id` - ID of the creature to spawn
    /// * `position` - World position
    /// * `scale` - Scale multiplier
    ///
    /// # Returns
    ///
    /// Spawn request with the specified scale
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::SpawnCreatureRequest;
    /// use bevy::prelude::*;
    ///
    /// let request = SpawnCreatureRequest::with_scale(10, Vec3::ZERO, 2.0);
    /// assert_eq!(request.scale_override, Some(2.0));
    /// ```
    pub fn with_scale(creature_id: CreatureId, position: Vec3, scale: f32) -> Self {
        Self {
            creature_id,
            position,
            scale_override: Some(scale),
        }
    }
}

/// Component for tracking LOD (Level of Detail) state
///
/// This component manages automatic LOD switching based on camera distance.
/// Each creature can have multiple LOD levels with progressively simpler meshes.
///
/// # Fields
///
/// * `current_level` - Current LOD level (0 = highest detail)
/// * `mesh_handles` - Mesh handles for each LOD level
/// * `distances` - Distance thresholds for switching LOD levels
///
/// # Examples
///
/// ```
/// use antares::game::components::creature::LodState;
/// use bevy::prelude::*;
///
/// fn example_lod() {
///     let lod_state = LodState {
///         current_level: 0,
///         mesh_handles: vec![],
///         distances: vec![10.0, 25.0, 50.0],
///     };
///     assert_eq!(lod_state.current_level, 0);
/// }
/// ```
#[derive(Component, Debug, Clone)]
pub struct LodState {
    /// Current active LOD level (0 = highest detail)
    pub current_level: usize,

    /// Mesh handles for each LOD level (LOD0, LOD1, LOD2, etc.)
    pub mesh_handles: Vec<Handle<Mesh>>,

    /// Distance thresholds for switching to each LOD level
    ///
    /// Example: [10.0, 25.0, 50.0] means:
    /// - Distance < 10.0: Use LOD0 (highest detail)
    /// - Distance 10.0-25.0: Use LOD1
    /// - Distance 25.0-50.0: Use LOD2
    /// - Distance > 50.0: Use LOD3 (or billboard/culled)
    pub distances: Vec<f32>,
}

impl LodState {
    /// Creates a new LOD state
    ///
    /// # Arguments
    ///
    /// * `mesh_handles` - Mesh handles for each LOD level
    /// * `distances` - Distance thresholds for LOD switching
    ///
    /// # Returns
    ///
    /// `LodState` with current level set to 0 (highest detail)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::LodState;
    /// use bevy::prelude::*;
    ///
    /// let lod = LodState::new(vec![], vec![10.0, 25.0]);
    /// assert_eq!(lod.current_level, 0);
    /// ```
    pub fn new(mesh_handles: Vec<Handle<Mesh>>, distances: Vec<f32>) -> Self {
        Self {
            current_level: 0,
            mesh_handles,
            distances,
        }
    }

    /// Determines the appropriate LOD level for a given distance
    ///
    /// # Arguments
    ///
    /// * `distance` - Distance from camera to creature
    ///
    /// # Returns
    ///
    /// LOD level to use (0 = highest detail)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::LodState;
    /// use bevy::prelude::*;
    ///
    /// let lod = LodState::new(vec![], vec![10.0, 25.0, 50.0]);
    /// assert_eq!(lod.level_for_distance(5.0), 0);
    /// assert_eq!(lod.level_for_distance(15.0), 1);
    /// assert_eq!(lod.level_for_distance(30.0), 2);
    /// assert_eq!(lod.level_for_distance(60.0), 3);
    /// ```
    pub fn level_for_distance(&self, distance: f32) -> usize {
        for (level, &threshold) in self.distances.iter().enumerate() {
            if distance < threshold {
                return level;
            }
        }
        // Beyond all thresholds - use highest LOD level available
        self.distances.len()
    }
}

/// Component for creature keyframe animation playback
///
/// This component tracks the state of a currently playing animation,
/// including playback time, speed, and looping behavior.
///
/// # Fields
///
/// * `definition` - The animation definition with keyframes
/// * `current_time` - Current playback time in seconds
/// * `playing` - Whether animation is currently playing
/// * `speed` - Playback speed multiplier (1.0 = normal speed)
/// * `looping` - Whether animation loops when finished
///
/// # Examples
///
/// ```
/// use antares::game::components::creature::CreatureAnimation;
/// use antares::domain::visual::animation::AnimationDefinition;
///
/// fn example_animation() {
///     let anim_def = AnimationDefinition {
///         name: "walk".to_string(),
///         duration: 1.0,
///         keyframes: vec![],
///         looping: true,
///     };
///
///     let anim = CreatureAnimation::new(anim_def);
///     assert_eq!(anim.current_time, 0.0);
///     assert_eq!(anim.playing, true);
/// }
/// ```
#[derive(Component, Debug, Clone)]
pub struct CreatureAnimation {
    /// The animation definition with keyframes
    pub definition: crate::domain::visual::animation::AnimationDefinition,

    /// Current playback time in seconds
    pub current_time: f32,

    /// Whether animation is currently playing
    pub playing: bool,

    /// Playback speed multiplier (1.0 = normal, 2.0 = double speed, 0.5 = half speed)
    pub speed: f32,

    /// Whether animation loops when it reaches the end
    pub looping: bool,
}

impl CreatureAnimation {
    /// Creates a new animation in the playing state
    ///
    /// # Arguments
    ///
    /// * `definition` - The animation definition to play
    ///
    /// # Returns
    ///
    /// `CreatureAnimation` starting at time 0.0 with normal speed
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureAnimation;
    /// use antares::domain::visual::animation::AnimationDefinition;
    ///
    /// let def = AnimationDefinition {
    ///     name: "idle".to_string(),
    ///     duration: 2.0,
    ///     keyframes: vec![],
    ///     looping: true,
    /// };
    ///
    /// let anim = CreatureAnimation::new(def);
    /// assert!(anim.playing);
    /// assert_eq!(anim.speed, 1.0);
    /// ```
    pub fn new(definition: crate::domain::visual::animation::AnimationDefinition) -> Self {
        let looping = definition.looping;
        Self {
            definition,
            current_time: 0.0,
            playing: true,
            speed: 1.0,
            looping,
        }
    }

    /// Advances animation time by delta seconds
    ///
    /// # Arguments
    ///
    /// * `delta_seconds` - Time to advance (will be multiplied by speed)
    ///
    /// # Returns
    ///
    /// `true` if animation finished (non-looping only), `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureAnimation;
    /// use antares::domain::visual::animation::AnimationDefinition;
    ///
    /// let mut anim = CreatureAnimation::new(AnimationDefinition {
    ///     name: "idle".to_string(),
    ///     duration: 1.0,
    ///     keyframes: vec![],
    ///     looping: false,
    /// });
    ///
    /// let finished = anim.advance(0.5);
    /// assert!(!finished);
    /// assert_eq!(anim.current_time, 0.5);
    ///
    /// let finished = anim.advance(0.6);
    /// assert!(finished);
    /// ```
    pub fn advance(&mut self, delta_seconds: f32) -> bool {
        if !self.playing {
            return false;
        }

        self.current_time += delta_seconds * self.speed;

        if self.current_time >= self.definition.duration {
            if self.looping {
                self.current_time %= self.definition.duration;
                false
            } else {
                self.current_time = self.definition.duration;
                self.playing = false;
                true
            }
        } else {
            false
        }
    }

    /// Resets animation to the beginning
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::creature::CreatureAnimation;
    /// use antares::domain::visual::animation::AnimationDefinition;
    ///
    /// let mut anim = CreatureAnimation::new(AnimationDefinition {
    ///     name: "jump".to_string(),
    ///     duration: 0.5,
    ///     keyframes: vec![],
    ///     looping: false,
    /// });
    ///
    /// anim.advance(0.3);
    /// anim.reset();
    /// assert_eq!(anim.current_time, 0.0);
    /// assert!(anim.playing);
    /// ```
    pub fn reset(&mut self) {
        self.current_time = 0.0;
        self.playing = true;
    }

    /// Pauses animation playback
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resumes animation playback
    pub fn resume(&mut self) {
        self.playing = true;
    }
}

/// Marker component indicating texture has been loaded for this creature
///
/// Prevents re-loading textures that have already been loaded.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TextureLoaded;

/// Placeholder component for future animation support
///
/// This component will eventually track animation state for creatures
/// with keyframe animations defined. Currently a placeholder for Phase 5.
///
/// # Future Fields (Phase 5)
///
/// * `current_animation` - Name of the currently playing animation
/// * `animation_time` - Current playback time
/// * `looping` - Whether animation should loop
#[derive(Component, Debug, Clone, Default)]
pub struct CreatureAnimationState {
    // Placeholder for Phase 5: Animation Keyframes
    // Will include:
    // - current_animation: String
    // - animation_time: f32
    // - looping: bool
    // - keyframe_index: usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_visual_new() {
        let visual = CreatureVisual::new(42);
        assert_eq!(visual.creature_id, 42);
        assert_eq!(visual.scale_override, None);
    }

    #[test]
    fn test_creature_visual_with_scale() {
        let visual = CreatureVisual::with_scale(10, 1.5);
        assert_eq!(visual.creature_id, 10);
        assert_eq!(visual.scale_override, Some(1.5));
    }

    #[test]
    fn test_creature_visual_effective_scale_no_override() {
        let visual = CreatureVisual::new(1);
        assert_eq!(visual.effective_scale(2.0), 2.0);
        assert_eq!(visual.effective_scale(0.5), 0.5);
    }

    #[test]
    fn test_creature_visual_effective_scale_with_override() {
        let visual = CreatureVisual::with_scale(1, 3.0);
        assert_eq!(visual.effective_scale(2.0), 3.0);
        assert_eq!(visual.effective_scale(0.5), 3.0);
    }

    #[test]
    fn test_mesh_part_new() {
        let part = MeshPart::new(42, 3);
        assert_eq!(part.creature_id, 42);
        assert_eq!(part.mesh_index, 3);
        assert!(part.material_override.is_none());
    }

    #[test]
    fn test_spawn_creature_request_new() {
        let request = SpawnCreatureRequest::new(42, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(request.creature_id, 42);
        assert_eq!(request.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(request.scale_override, None);
    }

    #[test]
    fn test_spawn_creature_request_with_scale() {
        let request = SpawnCreatureRequest::with_scale(10, Vec3::ZERO, 2.5);
        assert_eq!(request.creature_id, 10);
        assert_eq!(request.position, Vec3::ZERO);
        assert_eq!(request.scale_override, Some(2.5));
    }

    #[test]
    fn test_creature_animation_state_default() {
        let state = CreatureAnimationState::default();
        // Currently just a placeholder, ensure it can be constructed
        assert!(format!("{:?}", state).contains("CreatureAnimationState"));
    }

    #[test]
    fn test_creature_visual_clone() {
        let visual = CreatureVisual::with_scale(42, 1.5);
        let cloned = visual;
        assert_eq!(cloned.creature_id, visual.creature_id);
        assert_eq!(cloned.scale_override, visual.scale_override);
    }

    #[test]
    fn test_mesh_part_clone() {
        let part = MeshPart::new(42, 2);
        let cloned = part.clone();
        assert_eq!(cloned.creature_id, part.creature_id);
        assert_eq!(cloned.mesh_index, part.mesh_index);
    }

    #[test]
    fn test_spawn_request_clone() {
        let request = SpawnCreatureRequest::new(10, Vec3::ONE);
        let cloned = request;
        assert_eq!(cloned.creature_id, request.creature_id);
        assert_eq!(cloned.position, request.position);
    }

    #[test]
    fn test_lod_state_new() {
        let lod = LodState::new(vec![], vec![10.0, 25.0, 50.0]);
        assert_eq!(lod.current_level, 0);
        assert_eq!(lod.distances.len(), 3);
    }

    #[test]
    fn test_lod_state_level_for_distance() {
        let lod = LodState::new(vec![], vec![10.0, 25.0, 50.0]);

        // Close range - LOD0
        assert_eq!(lod.level_for_distance(5.0), 0);
        assert_eq!(lod.level_for_distance(9.9), 0);

        // Medium range - LOD1
        assert_eq!(lod.level_for_distance(10.0), 1);
        assert_eq!(lod.level_for_distance(15.0), 1);
        assert_eq!(lod.level_for_distance(24.9), 1);

        // Far range - LOD2
        assert_eq!(lod.level_for_distance(25.0), 2);
        assert_eq!(lod.level_for_distance(40.0), 2);
        assert_eq!(lod.level_for_distance(49.9), 2);

        // Very far range - LOD3
        assert_eq!(lod.level_for_distance(50.0), 3);
        assert_eq!(lod.level_for_distance(100.0), 3);
    }

    #[test]
    fn test_lod_state_empty_distances() {
        let lod = LodState::new(vec![], vec![]);
        assert_eq!(lod.level_for_distance(0.0), 0);
        assert_eq!(lod.level_for_distance(1000.0), 0);
    }

    #[test]
    fn test_creature_animation_new() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "walk".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: true,
        };

        let anim = CreatureAnimation::new(def);
        assert_eq!(anim.current_time, 0.0);
        assert!(anim.playing);
        assert_eq!(anim.speed, 1.0);
        assert!(anim.looping);
    }

    #[test]
    fn test_creature_animation_advance() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "idle".to_string(),
            duration: 2.0,
            keyframes: vec![],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(def);

        // Advance partway
        let finished = anim.advance(0.5);
        assert!(!finished);
        assert_eq!(anim.current_time, 0.5);

        // Advance to end
        let finished = anim.advance(1.5);
        assert!(finished);
        assert_eq!(anim.current_time, 2.0);
        assert!(!anim.playing);
    }

    #[test]
    fn test_creature_animation_looping() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "run".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: true,
        };

        let mut anim = CreatureAnimation::new(def);

        // Advance past duration
        let finished = anim.advance(1.5);
        assert!(!finished);
        assert_eq!(anim.current_time, 0.5);
        assert!(anim.playing);
    }

    #[test]
    fn test_creature_animation_reset() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "jump".to_string(),
            duration: 0.5,
            keyframes: vec![],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(def);
        anim.advance(0.3);
        anim.reset();

        assert_eq!(anim.current_time, 0.0);
        assert!(anim.playing);
    }

    #[test]
    fn test_creature_animation_pause_resume() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "attack".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(def);
        anim.pause();
        assert!(!anim.playing);

        let finished = anim.advance(0.5);
        assert!(!finished);
        assert_eq!(anim.current_time, 0.0); // No advance when paused

        anim.resume();
        assert!(anim.playing);
        anim.advance(0.5);
        assert_eq!(anim.current_time, 0.5); // Advances when playing
    }

    #[test]
    fn test_creature_animation_speed() {
        use crate::domain::visual::animation::AnimationDefinition;

        let def = AnimationDefinition {
            name: "walk".to_string(),
            duration: 1.0,
            keyframes: vec![],
            looping: false,
        };

        let mut anim = CreatureAnimation::new(def);
        anim.speed = 2.0; // Double speed

        anim.advance(0.5);
        assert_eq!(anim.current_time, 1.0); // 0.5 * 2.0
    }

    #[test]
    fn test_texture_loaded_default() {
        let marker = TextureLoaded;
        assert!(format!("{:?}", marker).contains("TextureLoaded"));
    }
}
