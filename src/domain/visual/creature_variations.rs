// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature variation system for creating color/scale variants from base creatures
//!
//! This module provides functionality to create creature variations by applying
//! override parameters to base creature definitions. This allows creating
//! variants like "blue dragon" and "red dragon" from a single base dragon,
//! or "young dragon" and "ancient dragon" with different scales.
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
//! use antares::domain::visual::creature_variations::{CreatureVariation, apply_variation};
//! use std::collections::HashMap;
//!
//! // Create a base creature
//! let base = CreatureDefinition {
//!     id: 1,
//!     name: "Base Dragon".to_string(),
//!     meshes: vec![
//!         MeshDefinition {
//!             vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
//!             indices: vec![0, 1, 2],
//!             normals: None,
//!             uvs: None,
//!             color: [0.5, 0.5, 0.5, 1.0],
//!             lod_levels: None,
//!             lod_distances: None,
//!             material: None,
//!             texture_path: None,
//!         },
//!     ],
//!     mesh_transforms: vec![MeshTransform::identity()],
//!     scale: 1.0,
//!     color_tint: None,
//! };
//!
//! // Create a red color variation
//! let mut color_overrides = HashMap::new();
//! color_overrides.insert(0, [1.0, 0.0, 0.0, 1.0]); // Red
//!
//! let variation = CreatureVariation {
//!     base_creature_id: 1,
//!     name: "Red Dragon".to_string(),
//!     scale_override: None,
//!     mesh_color_overrides: color_overrides,
//!     mesh_scale_overrides: HashMap::new(),
//! };
//!
//! let red_dragon = apply_variation(&base, &variation);
//! assert_eq!(red_dragon.name, "Red Dragon");
//! assert_eq!(red_dragon.meshes[0].color, [1.0, 0.0, 0.0, 1.0]);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::types::CreatureId;
use crate::domain::visual::CreatureDefinition;

/// Defines override parameters for creating a creature variation
///
/// A variation references a base creature and specifies which properties
/// to override. This allows creating multiple variants from a single
/// base definition without duplicating geometry.
///
/// # Fields
///
/// * `base_creature_id` - ID of the base creature to derive from
/// * `name` - Display name for this variation
/// * `scale_override` - Optional global scale multiplier override
/// * `mesh_color_overrides` - Per-mesh color overrides (mesh index -> color)
/// * `mesh_scale_overrides` - Per-mesh scale overrides (mesh index -> scale)
///
/// # Examples
///
/// ```
/// use antares::domain::visual::creature_variations::CreatureVariation;
/// use std::collections::HashMap;
///
/// // Create a blue color variant
/// let mut color_overrides = HashMap::new();
/// color_overrides.insert(0, [0.0, 0.0, 1.0, 1.0]); // Blue
///
/// let variation = CreatureVariation {
///     base_creature_id: 1,
///     name: "Blue Dragon".to_string(),
///     scale_override: None,
///     mesh_color_overrides: color_overrides,
///     mesh_scale_overrides: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureVariation {
    /// ID of the base creature to derive from
    pub base_creature_id: CreatureId,

    /// Display name for this variation
    pub name: String,

    /// Optional override for the creature's global scale
    ///
    /// If Some, replaces the base creature's scale value.
    /// If None, uses the base creature's scale.
    #[serde(default)]
    pub scale_override: Option<f32>,

    /// Per-mesh color overrides as [r, g, b, a] in range 0.0-1.0
    ///
    /// Keys are mesh indices (0-based), values are RGBA colors.
    /// Only specified meshes will have their colors overridden.
    #[serde(default)]
    pub mesh_color_overrides: HashMap<usize, [f32; 4]>,

    /// Per-mesh scale overrides as [x, y, z]
    ///
    /// Keys are mesh indices (0-based), values are scale vectors.
    /// Only specified meshes will have their scales overridden.
    #[serde(default)]
    pub mesh_scale_overrides: HashMap<usize, [f32; 3]>,
}

impl CreatureVariation {
    /// Creates a new variation with just a name override
    ///
    /// # Arguments
    ///
    /// * `base_creature_id` - ID of the base creature
    /// * `name` - Name for this variation
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_variations::CreatureVariation;
    ///
    /// let variation = CreatureVariation::new(1, "Variant Dragon");
    /// assert_eq!(variation.base_creature_id, 1);
    /// assert_eq!(variation.name, "Variant Dragon");
    /// assert!(variation.scale_override.is_none());
    /// ```
    pub fn new(base_creature_id: CreatureId, name: impl Into<String>) -> Self {
        Self {
            base_creature_id,
            name: name.into(),
            scale_override: None,
            mesh_color_overrides: HashMap::new(),
            mesh_scale_overrides: HashMap::new(),
        }
    }

    /// Sets the global scale override
    ///
    /// # Arguments
    ///
    /// * `scale` - Global scale multiplier
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_variations::CreatureVariation;
    ///
    /// let mut variation = CreatureVariation::new(1, "Ancient Dragon");
    /// variation.with_scale(2.0);
    /// assert_eq!(variation.scale_override, Some(2.0));
    /// ```
    pub fn with_scale(&mut self, scale: f32) -> &mut Self {
        self.scale_override = Some(scale);
        self
    }

    /// Adds a color override for a specific mesh
    ///
    /// # Arguments
    ///
    /// * `mesh_index` - Index of the mesh to override
    /// * `color` - RGBA color as [r, g, b, a] in range 0.0-1.0
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_variations::CreatureVariation;
    ///
    /// let mut variation = CreatureVariation::new(1, "Red Dragon");
    /// variation.with_mesh_color(0, [1.0, 0.0, 0.0, 1.0]);
    /// assert_eq!(variation.mesh_color_overrides.get(&0), Some(&[1.0, 0.0, 0.0, 1.0]));
    /// ```
    pub fn with_mesh_color(&mut self, mesh_index: usize, color: [f32; 4]) -> &mut Self {
        self.mesh_color_overrides.insert(mesh_index, color);
        self
    }

    /// Adds a scale override for a specific mesh
    ///
    /// # Arguments
    ///
    /// * `mesh_index` - Index of the mesh to override
    /// * `scale` - Scale vector as [x, y, z]
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::creature_variations::CreatureVariation;
    ///
    /// let mut variation = CreatureVariation::new(1, "Long-necked Dragon");
    /// variation.with_mesh_scale(1, [1.0, 2.0, 1.0]); // Stretch neck mesh vertically
    /// assert_eq!(variation.mesh_scale_overrides.get(&1), Some(&[1.0, 2.0, 1.0]));
    /// ```
    pub fn with_mesh_scale(&mut self, mesh_index: usize, scale: [f32; 3]) -> &mut Self {
        self.mesh_scale_overrides.insert(mesh_index, scale);
        self
    }

    /// Validates that all mesh indices reference valid meshes in the base creature
    ///
    /// # Arguments
    ///
    /// * `base` - The base creature definition
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all mesh indices are valid
    ///
    /// # Errors
    ///
    /// Returns `Err` with a description if any mesh index is out of bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
    /// use antares::domain::visual::creature_variations::CreatureVariation;
    ///
    /// let creature = CreatureDefinition {
    ///     id: 1,
    ///     name: "Test".to_string(),
    ///     meshes: vec![MeshDefinition {
    ///         vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
    ///         indices: vec![0, 1, 2],
    ///         normals: None,
    ///         uvs: None,
    ///         color: [1.0, 1.0, 1.0, 1.0],
    ///         lod_levels: None,
    ///         lod_distances: None,
    ///         material: None,
    ///         texture_path: None,
    ///     }],
    ///     mesh_transforms: vec![MeshTransform::identity()],
    ///     scale: 1.0,
    ///     color_tint: None,
    /// };
    ///
    /// let variation = CreatureVariation::new(1, "Variant");
    /// assert!(variation.validate(&creature).is_ok());
    /// ```
    pub fn validate(&self, base: &CreatureDefinition) -> Result<(), String> {
        let mesh_count = base.meshes.len();

        // Check color overrides
        for &mesh_index in self.mesh_color_overrides.keys() {
            if mesh_index >= mesh_count {
                return Err(format!(
                    "Color override mesh index {} out of bounds (base has {} meshes)",
                    mesh_index, mesh_count
                ));
            }
        }

        // Check scale overrides
        for &mesh_index in self.mesh_scale_overrides.keys() {
            if mesh_index >= mesh_count {
                return Err(format!(
                    "Scale override mesh index {} out of bounds (base has {} meshes)",
                    mesh_index, mesh_count
                ));
            }
        }

        // Validate scale override is positive
        if let Some(scale) = self.scale_override {
            if scale <= 0.0 {
                return Err(format!("Scale override must be positive, got {}", scale));
            }
        }

        // Validate mesh scale overrides are positive
        for (mesh_index, scale) in &self.mesh_scale_overrides {
            if scale[0] <= 0.0 || scale[1] <= 0.0 || scale[2] <= 0.0 {
                return Err(format!(
                    "Mesh {} scale override must have all positive components, got [{}, {}, {}]",
                    mesh_index, scale[0], scale[1], scale[2]
                ));
            }
        }

        Ok(())
    }
}

/// Applies a variation to a base creature definition, creating a new creature
///
/// This function creates a new `CreatureDefinition` by cloning the base creature
/// and applying the variation's overrides. The base creature is not modified.
///
/// # Arguments
///
/// * `base` - The base creature definition to derive from
/// * `variation` - The variation parameters to apply
///
/// # Returns
///
/// Returns a new `CreatureDefinition` with the variation applied
///
/// # Panics
///
/// Panics if the variation references mesh indices that don't exist in the base.
/// Call `variation.validate(base)` first to check validity.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
/// use antares::domain::visual::creature_variations::{CreatureVariation, apply_variation};
///
/// let base = CreatureDefinition {
///     id: 1,
///     name: "Base".to_string(),
///     meshes: vec![
///         MeshDefinition {
///             vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
///             indices: vec![0, 1, 2],
///             normals: None,
///             uvs: None,
///             color: [0.5, 0.5, 0.5, 1.0],
///             lod_levels: None,
///             lod_distances: None,
///             material: None,
///             texture_path: None,
///         },
///     ],
///     mesh_transforms: vec![MeshTransform::identity()],
///     scale: 1.0,
///     color_tint: None,
/// };
///
/// let mut variation = CreatureVariation::new(1, "Varied");
/// variation.with_scale(2.0);
///
/// let varied = apply_variation(&base, &variation);
/// assert_eq!(varied.scale, 2.0);
/// ```
pub fn apply_variation(
    base: &CreatureDefinition,
    variation: &CreatureVariation,
) -> CreatureDefinition {
    // Clone the base creature
    let mut result = base.clone();

    // Apply name override
    result.name = variation.name.clone();

    // Apply global scale override
    if let Some(scale) = variation.scale_override {
        result.scale = scale;
    }

    // Apply color overrides to meshes
    for (mesh_index, color) in &variation.mesh_color_overrides {
        if let Some(mesh) = result.meshes.get_mut(*mesh_index) {
            mesh.color = *color;
        }
    }

    // Apply scale overrides to mesh transforms
    for (mesh_index, scale) in &variation.mesh_scale_overrides {
        if let Some(transform) = result.mesh_transforms.get_mut(*mesh_index) {
            transform.scale = *scale;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::{MeshDefinition, MeshTransform};

    fn create_test_base_creature() -> CreatureDefinition {
        CreatureDefinition {
            id: 1,
            name: "Base Dragon".to_string(),
            meshes: vec![
                MeshDefinition {
                    name: None,
                    vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
                    indices: vec![0, 1, 2],
                    normals: None,
                    uvs: None,
                    color: [0.5, 0.5, 0.5, 1.0],
                    lod_levels: None,
                    lod_distances: None,
                    material: None,
                    texture_path: None,
                },
                MeshDefinition {
                    name: None,
                    vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
                    indices: vec![0, 1, 2],
                    normals: None,
                    uvs: None,
                    color: [0.5, 0.5, 0.5, 1.0],
                    lod_levels: None,
                    lod_distances: None,
                    material: None,
                    texture_path: None,
                },
            ],
            mesh_transforms: vec![MeshTransform::identity(), MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        }
    }

    #[test]
    fn test_creature_variation_new() {
        let variation = CreatureVariation::new(1, "Test Variant");
        assert_eq!(variation.base_creature_id, 1);
        assert_eq!(variation.name, "Test Variant");
        assert!(variation.scale_override.is_none());
        assert!(variation.mesh_color_overrides.is_empty());
        assert!(variation.mesh_scale_overrides.is_empty());
    }

    #[test]
    fn test_creature_variation_with_scale() {
        let mut variation = CreatureVariation::new(1, "Large Variant");
        variation.with_scale(2.5);
        assert_eq!(variation.scale_override, Some(2.5));
    }

    #[test]
    fn test_creature_variation_with_mesh_color() {
        let mut variation = CreatureVariation::new(1, "Red Variant");
        variation.with_mesh_color(0, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(
            variation.mesh_color_overrides.get(&0),
            Some(&[1.0, 0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_creature_variation_with_mesh_scale() {
        let mut variation = CreatureVariation::new(1, "Stretched Variant");
        variation.with_mesh_scale(0, [1.0, 2.0, 1.0]);
        assert_eq!(
            variation.mesh_scale_overrides.get(&0),
            Some(&[1.0, 2.0, 1.0])
        );
    }

    #[test]
    fn test_apply_variation_color_override() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Red Dragon");
        variation.with_mesh_color(0, [1.0, 0.0, 0.0, 1.0]);

        let result = apply_variation(&base, &variation);

        assert_eq!(result.name, "Red Dragon");
        assert_eq!(result.meshes[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(result.meshes[1].color, [0.5, 0.5, 0.5, 1.0]); // Unchanged
    }

    #[test]
    fn test_apply_variation_scale_override() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Ancient Dragon");
        variation.with_scale(3.0);

        let result = apply_variation(&base, &variation);

        assert_eq!(result.name, "Ancient Dragon");
        assert_eq!(result.scale, 3.0);
    }

    #[test]
    fn test_apply_variation_mesh_scale_override() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Long-necked Dragon");
        variation.with_mesh_scale(1, [1.0, 2.0, 1.0]);

        let result = apply_variation(&base, &variation);

        assert_eq!(result.mesh_transforms[0].scale, [1.0, 1.0, 1.0]); // Unchanged
        assert_eq!(result.mesh_transforms[1].scale, [1.0, 2.0, 1.0]); // Modified
    }

    #[test]
    fn test_apply_variation_multiple_overrides() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Ancient Blue Dragon");
        variation.with_scale(2.0);
        variation.with_mesh_color(0, [0.0, 0.0, 1.0, 1.0]);
        variation.with_mesh_color(1, [0.2, 0.2, 0.8, 1.0]);
        variation.with_mesh_scale(0, [1.5, 1.5, 1.5]);

        let result = apply_variation(&base, &variation);

        assert_eq!(result.name, "Ancient Blue Dragon");
        assert_eq!(result.scale, 2.0);
        assert_eq!(result.meshes[0].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(result.meshes[1].color, [0.2, 0.2, 0.8, 1.0]);
        assert_eq!(result.mesh_transforms[0].scale, [1.5, 1.5, 1.5]);
    }

    #[test]
    fn test_variation_validate_success() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Valid Variant");
        variation.with_mesh_color(0, [1.0, 0.0, 0.0, 1.0]);
        variation.with_mesh_scale(1, [1.0, 2.0, 1.0]);

        assert!(variation.validate(&base).is_ok());
    }

    #[test]
    fn test_variation_validate_color_index_out_of_bounds() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_mesh_color(5, [1.0, 0.0, 0.0, 1.0]); // Out of bounds

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Color override mesh index 5 out of bounds"));
    }

    #[test]
    fn test_variation_validate_scale_index_out_of_bounds() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_mesh_scale(10, [1.0, 2.0, 1.0]); // Out of bounds

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Scale override mesh index 10 out of bounds"));
    }

    #[test]
    fn test_variation_validate_negative_global_scale() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_scale(-1.0);

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Scale override must be positive"));
    }

    #[test]
    fn test_variation_validate_zero_global_scale() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_scale(0.0);

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Scale override must be positive"));
    }

    #[test]
    fn test_variation_validate_negative_mesh_scale() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_mesh_scale(0, [-1.0, 1.0, 1.0]);

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must have all positive components"));
    }

    #[test]
    fn test_variation_validate_zero_mesh_scale_component() {
        let base = create_test_base_creature();
        let mut variation = CreatureVariation::new(1, "Invalid Variant");
        variation.with_mesh_scale(0, [1.0, 0.0, 1.0]);

        let result = variation.validate(&base);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must have all positive components"));
    }

    #[test]
    fn test_variation_serialization() {
        let mut variation = CreatureVariation::new(1, "Test Variant");
        variation.with_scale(2.0);
        variation.with_mesh_color(0, [1.0, 0.0, 0.0, 1.0]);

        let serialized = ron::to_string(&variation).unwrap();
        let deserialized: CreatureVariation = ron::from_str(&serialized).unwrap();

        assert_eq!(variation, deserialized);
    }

    #[test]
    fn test_apply_variation_preserves_base() {
        let base = create_test_base_creature();
        let base_clone = base.clone();
        let mut variation = CreatureVariation::new(1, "Modified");
        variation.with_scale(5.0);

        let _result = apply_variation(&base, &variation);

        // Base should be unchanged
        assert_eq!(base, base_clone);
    }
}
