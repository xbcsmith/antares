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
//!     )).set_parent(parent);
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
///         .set_parent(parent);
///
///     // Spawn second mesh (e.g., head)
///     commands.spawn(MeshPart::new(creature_id, 1))
///         .set_parent(parent);
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
}
