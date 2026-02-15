// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Animation system for creature keyframe-based animations
//!
//! This module provides foundation for animating creatures using keyframe transforms.
//! Animations are defined as a series of keyframes that specify mesh transforms at
//! specific times, with interpolation between keyframes during playback.
//!
//! # Overview
//!
//! The animation system supports:
//!
//! - Keyframe-based animation (position, rotation, scale per mesh)
//! - Linear interpolation between keyframes
//! - Looping and one-shot animations
//! - Multiple meshes animated independently
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
//! use antares::domain::visual::MeshTransform;
//!
//! // Create a simple bounce animation for mesh 0
//! let animation = AnimationDefinition {
//!     name: "Bounce".to_string(),
//!     duration: 1.0,
//!     keyframes: vec![
//!         Keyframe {
//!             time: 0.0,
//!             mesh_index: 0,
//!             transform: MeshTransform::translation(0.0, 0.0, 0.0),
//!         },
//!         Keyframe {
//!             time: 0.5,
//!             mesh_index: 0,
//!             transform: MeshTransform::translation(0.0, 1.0, 0.0),
//!         },
//!         Keyframe {
//!             time: 1.0,
//!             mesh_index: 0,
//!             transform: MeshTransform::translation(0.0, 0.0, 0.0),
//!         },
//!     ],
//!     looping: true,
//! };
//!
//! assert_eq!(animation.duration, 1.0);
//! assert_eq!(animation.keyframes.len(), 3);
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::visual::MeshTransform;

/// Animation definition with keyframes
///
/// Defines a complete animation as a sequence of keyframes. Each keyframe
/// specifies a transform for a specific mesh at a specific time.
///
/// # Fields
///
/// * `name` - Display name for the animation
/// * `duration` - Total duration in seconds
/// * `keyframes` - List of keyframes defining the animation
/// * `looping` - Whether the animation loops or plays once
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
/// use antares::domain::visual::MeshTransform;
///
/// let walk_cycle = AnimationDefinition {
///     name: "Walk".to_string(),
///     duration: 2.0,
///     keyframes: vec![
///         Keyframe {
///             time: 0.0,
///             mesh_index: 1, // Left leg
///             transform: MeshTransform::translation(0.0, 0.0, 0.5),
///         },
///         Keyframe {
///             time: 1.0,
///             mesh_index: 1,
///             transform: MeshTransform::translation(0.0, 0.0, -0.5),
///         },
///         Keyframe {
///             time: 2.0,
///             mesh_index: 1,
///             transform: MeshTransform::translation(0.0, 0.0, 0.5),
///         },
///     ],
///     looping: true,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationDefinition {
    /// Display name for this animation
    pub name: String,

    /// Total duration of the animation in seconds
    pub duration: f32,

    /// List of keyframes that define the animation
    pub keyframes: Vec<Keyframe>,

    /// Whether the animation loops (true) or plays once (false)
    #[serde(default)]
    pub looping: bool,
}

impl AnimationDefinition {
    /// Creates a new empty animation
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the animation
    /// * `duration` - Duration in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::AnimationDefinition;
    ///
    /// let anim = AnimationDefinition::new("Idle", 1.0);
    /// assert_eq!(anim.name, "Idle");
    /// assert_eq!(anim.duration, 1.0);
    /// assert!(anim.keyframes.is_empty());
    /// ```
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration,
            keyframes: Vec::new(),
            looping: false,
        }
    }

    /// Adds a keyframe to the animation
    ///
    /// # Arguments
    ///
    /// * `keyframe` - The keyframe to add
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let mut anim = AnimationDefinition::new("Test", 1.0);
    /// anim.add_keyframe(Keyframe {
    ///     time: 0.0,
    ///     mesh_index: 0,
    ///     transform: MeshTransform::identity(),
    /// });
    /// assert_eq!(anim.keyframes.len(), 1);
    /// ```
    pub fn add_keyframe(&mut self, keyframe: Keyframe) -> &mut Self {
        self.keyframes.push(keyframe);
        self
    }

    /// Sets whether the animation loops
    ///
    /// # Arguments
    ///
    /// * `looping` - Whether to loop
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::AnimationDefinition;
    ///
    /// let mut anim = AnimationDefinition::new("Walk", 2.0);
    /// anim.set_looping(true);
    /// assert!(anim.looping);
    /// ```
    pub fn set_looping(&mut self, looping: bool) -> &mut Self {
        self.looping = looping;
        self
    }

    /// Validates the animation definition
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `Err` with a description of the problem
    ///
    /// # Errors
    ///
    /// Returns errors if:
    /// - Duration is not positive
    /// - Keyframe times are outside [0, duration]
    /// - Keyframes are not sorted by time
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let mut anim = AnimationDefinition::new("Test", 1.0);
    /// anim.add_keyframe(Keyframe {
    ///     time: 0.0,
    ///     mesh_index: 0,
    ///     transform: MeshTransform::identity(),
    /// });
    /// assert!(anim.validate().is_ok());
    ///
    /// let mut bad_anim = AnimationDefinition::new("Bad", -1.0);
    /// assert!(bad_anim.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.duration <= 0.0 {
            return Err(format!(
                "Animation duration must be positive, got {}",
                self.duration
            ));
        }

        if self.name.is_empty() {
            return Err("Animation name cannot be empty".to_string());
        }

        // Check keyframe times
        let mut last_time = -f32::EPSILON;
        for (i, keyframe) in self.keyframes.iter().enumerate() {
            if keyframe.time < 0.0 || keyframe.time > self.duration {
                return Err(format!(
                    "Keyframe {} time {} is outside valid range [0, {}]",
                    i, keyframe.time, self.duration
                ));
            }

            if keyframe.time < last_time {
                return Err(format!(
                    "Keyframes must be sorted by time, but keyframe {} (time {}) comes after keyframe with time {}",
                    i, keyframe.time, last_time
                ));
            }

            last_time = keyframe.time;
        }

        Ok(())
    }

    /// Gets the transform for a mesh at a specific time
    ///
    /// Interpolates between keyframes to get the transform at the given time.
    /// If no keyframes exist for the mesh, returns None.
    ///
    /// # Arguments
    ///
    /// * `mesh_index` - Index of the mesh
    /// * `time` - Current time in the animation (will be wrapped if looping)
    ///
    /// # Returns
    ///
    /// Returns the interpolated transform, or None if no keyframes for this mesh
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::{AnimationDefinition, Keyframe};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let mut anim = AnimationDefinition::new("Test", 1.0);
    /// anim.add_keyframe(Keyframe {
    ///     time: 0.0,
    ///     mesh_index: 0,
    ///     transform: MeshTransform::translation(0.0, 0.0, 0.0),
    /// });
    /// anim.add_keyframe(Keyframe {
    ///     time: 1.0,
    ///     mesh_index: 0,
    ///     transform: MeshTransform::translation(0.0, 1.0, 0.0),
    /// });
    ///
    /// let transform = anim.sample(0, 0.5).unwrap();
    /// assert_eq!(transform.translation, [0.0, 0.5, 0.0]);
    /// ```
    pub fn sample(&self, mesh_index: usize, time: f32) -> Option<MeshTransform> {
        // Wrap time if looping
        let t = if self.looping && self.duration > 0.0 {
            time % self.duration
        } else {
            time.clamp(0.0, self.duration)
        };

        // Find keyframes for this mesh
        let mesh_keyframes: Vec<&Keyframe> = self
            .keyframes
            .iter()
            .filter(|kf| kf.mesh_index == mesh_index)
            .collect();

        if mesh_keyframes.is_empty() {
            return None;
        }

        // Find surrounding keyframes
        let mut prev: Option<&Keyframe> = None;
        let mut next: Option<&Keyframe> = None;

        for kf in &mesh_keyframes {
            if kf.time <= t {
                prev = Some(kf);
            }
            if kf.time >= t && next.is_none() {
                next = Some(kf);
            }
        }

        // Interpolate between keyframes
        match (prev, next) {
            (Some(p), Some(n)) if p.time != n.time => {
                let alpha = (t - p.time) / (n.time - p.time);
                Some(interpolate_transform(&p.transform, &n.transform, alpha))
            }
            (Some(p), _) => Some(p.transform),
            (_, Some(n)) => Some(n.transform),
            (None, None) => None,
        }
    }
}

/// A single keyframe in an animation
///
/// Defines the transform of a specific mesh at a specific time.
///
/// # Fields
///
/// * `time` - Time in seconds from animation start
/// * `mesh_index` - Index of the mesh to transform
/// * `transform` - Transform to apply at this keyframe
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation::Keyframe;
/// use antares::domain::visual::MeshTransform;
///
/// let keyframe = Keyframe {
///     time: 0.5,
///     mesh_index: 0,
///     transform: MeshTransform::translation(0.0, 1.0, 0.0),
/// };
///
/// assert_eq!(keyframe.time, 0.5);
/// assert_eq!(keyframe.mesh_index, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time in seconds from animation start
    pub time: f32,

    /// Index of the mesh to transform
    pub mesh_index: usize,

    /// Transform to apply at this time
    pub transform: MeshTransform,
}

impl Keyframe {
    /// Creates a new keyframe
    ///
    /// # Arguments
    ///
    /// * `time` - Time in seconds
    /// * `mesh_index` - Mesh index
    /// * `transform` - Transform to apply
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation::Keyframe;
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let kf = Keyframe::new(0.5, 0, MeshTransform::identity());
    /// assert_eq!(kf.time, 0.5);
    /// ```
    pub fn new(time: f32, mesh_index: usize, transform: MeshTransform) -> Self {
        Self {
            time,
            mesh_index,
            transform,
        }
    }
}

/// Interpolates between two transforms using linear interpolation
///
/// # Arguments
///
/// * `a` - Start transform
/// * `b` - End transform
/// * `t` - Interpolation factor [0, 1]
///
/// # Returns
///
/// Returns the interpolated transform
fn interpolate_transform(a: &MeshTransform, b: &MeshTransform, t: f32) -> MeshTransform {
    let t = t.clamp(0.0, 1.0);

    MeshTransform {
        translation: [
            lerp(a.translation[0], b.translation[0], t),
            lerp(a.translation[1], b.translation[1], t),
            lerp(a.translation[2], b.translation[2], t),
        ],
        rotation: [
            lerp(a.rotation[0], b.rotation[0], t),
            lerp(a.rotation[1], b.rotation[1], t),
            lerp(a.rotation[2], b.rotation[2], t),
        ],
        scale: [
            lerp(a.scale[0], b.scale[0], t),
            lerp(a.scale[1], b.scale[1], t),
            lerp(a.scale[2], b.scale[2], t),
        ],
    }
}

/// Linear interpolation between two values
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_definition_new() {
        let anim = AnimationDefinition::new("Test", 1.0);
        assert_eq!(anim.name, "Test");
        assert_eq!(anim.duration, 1.0);
        assert!(anim.keyframes.is_empty());
        assert!(!anim.looping);
    }

    #[test]
    fn test_animation_add_keyframe() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(0.0, 0, MeshTransform::identity()));
        assert_eq!(anim.keyframes.len(), 1);
    }

    #[test]
    fn test_animation_set_looping() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.set_looping(true);
        assert!(anim.looping);
    }

    #[test]
    fn test_animation_validate_success() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(0.0, 0, MeshTransform::identity()));
        anim.add_keyframe(Keyframe::new(1.0, 0, MeshTransform::identity()));
        assert!(anim.validate().is_ok());
    }

    #[test]
    fn test_animation_validate_negative_duration() {
        let anim = AnimationDefinition::new("Test", -1.0);
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_animation_validate_zero_duration() {
        let anim = AnimationDefinition::new("Test", 0.0);
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_animation_validate_empty_name() {
        let anim = AnimationDefinition::new("", 1.0);
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_animation_validate_keyframe_time_out_of_range() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(2.0, 0, MeshTransform::identity()));
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_animation_validate_keyframe_negative_time() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(-0.5, 0, MeshTransform::identity()));
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_animation_validate_keyframes_not_sorted() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(1.0, 0, MeshTransform::identity()));
        anim.add_keyframe(Keyframe::new(0.5, 0, MeshTransform::identity()));
        assert!(anim.validate().is_err());
    }

    #[test]
    fn test_keyframe_new() {
        let kf = Keyframe::new(0.5, 0, MeshTransform::identity());
        assert_eq!(kf.time, 0.5);
        assert_eq!(kf.mesh_index, 0);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 1.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 1.0, 0.5), 0.5);
        assert_eq!(lerp(0.0, 1.0, 1.0), 1.0);
        assert_eq!(lerp(10.0, 20.0, 0.5), 15.0);
    }

    #[test]
    fn test_interpolate_transform() {
        let a = MeshTransform::translation(0.0, 0.0, 0.0);
        let b = MeshTransform::translation(1.0, 2.0, 3.0);
        let result = interpolate_transform(&a, &b, 0.5);

        assert_eq!(result.translation, [0.5, 1.0, 1.5]);
    }

    #[test]
    fn test_animation_sample_no_keyframes() {
        let anim = AnimationDefinition::new("Test", 1.0);
        assert!(anim.sample(0, 0.5).is_none());
    }

    #[test]
    fn test_animation_sample_single_keyframe() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(
            0.5,
            0,
            MeshTransform::translation(1.0, 2.0, 3.0),
        ));

        let result = anim.sample(0, 0.5).unwrap();
        assert_eq!(result.translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_animation_sample_interpolation() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(
            0.0,
            0,
            MeshTransform::translation(0.0, 0.0, 0.0),
        ));
        anim.add_keyframe(Keyframe::new(
            1.0,
            0,
            MeshTransform::translation(0.0, 2.0, 0.0),
        ));

        let result = anim.sample(0, 0.5).unwrap();
        assert_eq!(result.translation, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_animation_sample_before_first_keyframe() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(
            0.5,
            0,
            MeshTransform::translation(1.0, 0.0, 0.0),
        ));

        let result = anim.sample(0, 0.0).unwrap();
        assert_eq!(result.translation, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_animation_sample_after_last_keyframe() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(
            0.5,
            0,
            MeshTransform::translation(1.0, 0.0, 0.0),
        ));

        let result = anim.sample(0, 1.0).unwrap();
        assert_eq!(result.translation, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_animation_sample_looping() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.set_looping(true);
        anim.add_keyframe(Keyframe::new(
            0.0,
            0,
            MeshTransform::translation(0.0, 0.0, 0.0),
        ));
        anim.add_keyframe(Keyframe::new(
            1.0,
            0,
            MeshTransform::translation(0.0, 2.0, 0.0),
        ));

        // Time 1.5 should wrap to 0.5
        let result = anim.sample(0, 1.5).unwrap();
        assert_eq!(result.translation, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_animation_sample_multiple_meshes() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.add_keyframe(Keyframe::new(
            0.0,
            0,
            MeshTransform::translation(0.0, 0.0, 0.0),
        ));
        anim.add_keyframe(Keyframe::new(
            1.0,
            0,
            MeshTransform::translation(1.0, 0.0, 0.0),
        ));
        anim.add_keyframe(Keyframe::new(
            0.0,
            1,
            MeshTransform::translation(0.0, 0.0, 0.0),
        ));
        anim.add_keyframe(Keyframe::new(
            1.0,
            1,
            MeshTransform::translation(0.0, 2.0, 0.0),
        ));

        let mesh0 = anim.sample(0, 0.5).unwrap();
        let mesh1 = anim.sample(1, 0.5).unwrap();

        assert_eq!(mesh0.translation, [0.5, 0.0, 0.0]);
        assert_eq!(mesh1.translation, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_animation_serialization() {
        let mut anim = AnimationDefinition::new("Test", 1.0);
        anim.set_looping(true);
        anim.add_keyframe(Keyframe::new(0.0, 0, MeshTransform::identity()));

        let serialized = ron::to_string(&anim).unwrap();
        let deserialized: AnimationDefinition = ron::from_str(&serialized).unwrap();

        assert_eq!(anim, deserialized);
    }
}
