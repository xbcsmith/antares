// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Animation blend tree system for combining multiple animations
//!
//! This module provides a blend tree system that allows complex animation blending.
//! Blend trees enable smooth transitions between animations, additive layers, and
//! directional blending (e.g., walk/run based on speed, aim based on direction).
//!
//! # Overview
//!
//! The blend tree system supports:
//!
//! - Simple animation clips (single animation playback)
//! - 2D blend spaces (blend based on two parameters, e.g., speed and direction)
//! - Additive blending (add animation on top of base, e.g., hit reactions)
//! - Layered blending (multiple animations with weights, e.g., upper/lower body)
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::blend_tree::{BlendNode, AnimationClip};
//!
//! // Simple clip playback
//! let idle_clip = AnimationClip {
//!     animation_name: "Idle".to_string(),
//!     speed: 1.0,
//! };
//!
//! let node = BlendNode::Clip(idle_clip);
//! ```

use serde::{Deserialize, Serialize};

/// A reference to an animation clip
///
/// References an animation by name for playback in a blend tree.
/// The speed multiplier allows for slower or faster playback.
///
/// # Fields
///
/// * `animation_name` - Name of the animation to play
/// * `speed` - Playback speed multiplier (1.0 = normal speed)
///
/// # Examples
///
/// ```
/// use antares::domain::visual::blend_tree::AnimationClip;
///
/// let walk = AnimationClip {
///     animation_name: "Walk".to_string(),
///     speed: 1.0,
/// };
///
/// let run = AnimationClip {
///     animation_name: "Run".to_string(),
///     speed: 1.2, // 20% faster
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationClip {
    /// Name of the animation to play
    pub animation_name: String,

    /// Playback speed multiplier (1.0 = normal speed)
    pub speed: f32,
}

impl AnimationClip {
    /// Creates a new animation clip reference
    ///
    /// # Arguments
    ///
    /// * `animation_name` - Name of the animation
    /// * `speed` - Playback speed multiplier
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::AnimationClip;
    ///
    /// let clip = AnimationClip::new("Walk".to_string(), 1.0);
    /// assert_eq!(clip.animation_name, "Walk");
    /// assert_eq!(clip.speed, 1.0);
    /// ```
    pub fn new(animation_name: String, speed: f32) -> Self {
        Self {
            animation_name,
            speed,
        }
    }
}

/// A 2D position for blend sample placement
///
/// Used in 2D blend spaces to specify where a sample animation should be placed
/// in the blend space (e.g., [speed, direction]).
///
/// # Examples
///
/// ```
/// use antares::domain::visual::blend_tree::Vec2;
///
/// let position = Vec2 { x: 0.5, y: 0.8 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    /// X coordinate
    pub x: f32,

    /// Y coordinate
    pub y: f32,
}

impl Vec2 {
    /// Creates a new 2D vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::Vec2;
    ///
    /// let vec = Vec2::new(1.0, 2.0);
    /// assert_eq!(vec.x, 1.0);
    /// assert_eq!(vec.y, 2.0);
    /// ```
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculates the squared distance to another vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::Vec2;
    ///
    /// let a = Vec2::new(0.0, 0.0);
    /// let b = Vec2::new(3.0, 4.0);
    ///
    /// assert_eq!(a.distance_squared(b), 25.0);
    /// ```
    pub fn distance_squared(self, other: Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

/// A sample point in a 2D blend space
///
/// Associates an animation with a position in 2D blend space. The blend tree
/// will interpolate between nearby samples based on the current parameter values.
///
/// # Fields
///
/// * `position` - Position in 2D blend space
/// * `animation` - Animation clip at this position
///
/// # Examples
///
/// ```
/// use antares::domain::visual::blend_tree::{BlendSample, AnimationClip, Vec2};
///
/// let sample = BlendSample {
///     position: Vec2::new(0.5, 0.5),
///     animation: AnimationClip::new("WalkForward".to_string(), 1.0),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlendSample {
    /// Position in 2D blend space
    pub position: Vec2,

    /// Animation clip at this position
    pub animation: AnimationClip,
}

/// A node in an animation blend tree
///
/// Blend trees are hierarchical structures that define how animations are combined.
/// Different node types provide different blending behaviors.
///
/// # Variants
///
/// * `Clip` - Simple single animation playback
/// * `Blend2D` - 2D blend space (e.g., walk/run based on speed and direction)
/// * `Additive` - Additive blending (base + additive layer)
/// * `LayeredBlend` - Multiple animation layers with independent weights
///
/// # Examples
///
/// ```
/// use antares::domain::visual::blend_tree::{BlendNode, AnimationClip, BlendSample, Vec2};
///
/// // Simple clip
/// let idle = BlendNode::Clip(AnimationClip::new("Idle".to_string(), 1.0));
///
/// // 2D blend space for locomotion
/// let locomotion = BlendNode::Blend2D {
///     x_param: "speed".to_string(),
///     y_param: "direction".to_string(),
///     samples: vec![
///         BlendSample {
///             position: Vec2::new(0.0, 0.0),
///             animation: AnimationClip::new("Idle".to_string(), 1.0),
///         },
///         BlendSample {
///             position: Vec2::new(1.0, 0.0),
///             animation: AnimationClip::new("Walk".to_string(), 1.0),
///         },
///         BlendSample {
///             position: Vec2::new(2.0, 0.0),
///             animation: AnimationClip::new("Run".to_string(), 1.0),
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlendNode {
    /// Simple animation clip playback
    Clip(AnimationClip),

    /// 2D blend space
    ///
    /// Blends between multiple animation samples based on two parameters.
    /// For example, blending walk/run animations based on speed and direction.
    Blend2D {
        /// Name of X-axis parameter (e.g., "speed")
        x_param: String,

        /// Name of Y-axis parameter (e.g., "direction")
        y_param: String,

        /// Sample points in the blend space
        samples: Vec<BlendSample>,
    },

    /// Additive blending
    ///
    /// Adds an additive animation on top of a base animation.
    /// Weight controls how much of the additive layer to apply (0.0 to 1.0).
    Additive {
        /// Base animation node
        base: Box<BlendNode>,

        /// Additive animation node (difference animation)
        additive: Box<BlendNode>,

        /// Weight of additive layer (0.0 = none, 1.0 = full)
        weight: f32,
    },

    /// Layered blending
    ///
    /// Combines multiple animation layers, each with its own weight.
    /// For example, upper body and lower body animations can be layered.
    LayeredBlend {
        /// Animation layers with their weights
        layers: Vec<(Box<BlendNode>, f32)>,
    },
}

impl BlendNode {
    /// Creates a simple clip node
    ///
    /// # Arguments
    ///
    /// * `animation_name` - Name of the animation
    /// * `speed` - Playback speed multiplier
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let node = BlendNode::clip("Idle".to_string(), 1.0);
    /// ```
    pub fn clip(animation_name: String, speed: f32) -> Self {
        BlendNode::Clip(AnimationClip::new(animation_name, speed))
    }

    /// Creates a 2D blend space node
    ///
    /// # Arguments
    ///
    /// * `x_param` - X-axis parameter name
    /// * `y_param` - Y-axis parameter name
    /// * `samples` - Blend samples
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::{BlendNode, BlendSample, AnimationClip, Vec2};
    ///
    /// let samples = vec![
    ///     BlendSample {
    ///         position: Vec2::new(0.0, 0.0),
    ///         animation: AnimationClip::new("Idle".to_string(), 1.0),
    ///     },
    /// ];
    ///
    /// let node = BlendNode::blend_2d("speed".to_string(), "direction".to_string(), samples);
    /// ```
    pub fn blend_2d(x_param: String, y_param: String, samples: Vec<BlendSample>) -> Self {
        BlendNode::Blend2D {
            x_param,
            y_param,
            samples,
        }
    }

    /// Creates an additive blend node
    ///
    /// # Arguments
    ///
    /// * `base` - Base animation node
    /// * `additive` - Additive animation node
    /// * `weight` - Weight of additive layer (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let base = BlendNode::clip("Walk".to_string(), 1.0);
    /// let additive = BlendNode::clip("HitReaction".to_string(), 1.0);
    ///
    /// let node = BlendNode::additive(base, additive, 0.5);
    /// ```
    pub fn additive(base: BlendNode, additive: BlendNode, weight: f32) -> Self {
        BlendNode::Additive {
            base: Box::new(base),
            additive: Box::new(additive),
            weight: weight.clamp(0.0, 1.0),
        }
    }

    /// Creates a layered blend node
    ///
    /// # Arguments
    ///
    /// * `layers` - Vector of (node, weight) pairs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let upper_body = BlendNode::clip("UpperBodyIdle".to_string(), 1.0);
    /// let lower_body = BlendNode::clip("Walk".to_string(), 1.0);
    ///
    /// let node = BlendNode::layered(vec![
    ///     (upper_body, 1.0),
    ///     (lower_body, 1.0),
    /// ]);
    /// ```
    pub fn layered(layers: Vec<(BlendNode, f32)>) -> Self {
        BlendNode::LayeredBlend {
            layers: layers
                .into_iter()
                .map(|(node, weight)| (Box::new(node), weight.clamp(0.0, 1.0)))
                .collect(),
        }
    }

    /// Validates the blend tree structure
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `Err(String)` with error description
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Animation clip has empty name
    /// - Animation clip has invalid speed (<= 0.0)
    /// - Blend2D has no samples
    /// - Additive weight is out of range
    /// - Layered blend has no layers
    /// - Child nodes are invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let node = BlendNode::clip("Walk".to_string(), 1.0);
    /// assert!(node.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        match self {
            BlendNode::Clip(clip) => {
                if clip.animation_name.is_empty() {
                    return Err("Animation clip has empty name".to_string());
                }
                if clip.speed <= 0.0 {
                    return Err(format!(
                        "Animation clip '{}' has invalid speed: {}",
                        clip.animation_name, clip.speed
                    ));
                }
            }
            BlendNode::Blend2D {
                x_param,
                y_param,
                samples,
            } => {
                if x_param.is_empty() {
                    return Err("Blend2D x_param is empty".to_string());
                }
                if y_param.is_empty() {
                    return Err("Blend2D y_param is empty".to_string());
                }
                if samples.is_empty() {
                    return Err("Blend2D has no samples".to_string());
                }
                for sample in samples {
                    if sample.animation.animation_name.is_empty() {
                        return Err("Blend2D sample has empty animation name".to_string());
                    }
                    if sample.animation.speed <= 0.0 {
                        return Err(format!(
                            "Blend2D sample '{}' has invalid speed: {}",
                            sample.animation.animation_name, sample.animation.speed
                        ));
                    }
                }
            }
            BlendNode::Additive {
                base,
                additive,
                weight,
            } => {
                if *weight < 0.0 || *weight > 1.0 {
                    return Err(format!("Additive weight out of range [0,1]: {}", weight));
                }
                base.validate()?;
                additive.validate()?;
            }
            BlendNode::LayeredBlend { layers } => {
                if layers.is_empty() {
                    return Err("LayeredBlend has no layers".to_string());
                }
                for (layer, weight) in layers {
                    if *weight < 0.0 || *weight > 1.0 {
                        return Err(format!("Layer weight out of range [0,1]: {}", weight));
                    }
                    layer.validate()?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_clip_new() {
        let clip = AnimationClip::new("Walk".to_string(), 1.0);
        assert_eq!(clip.animation_name, "Walk");
        assert_eq!(clip.speed, 1.0);
    }

    #[test]
    fn test_vec2_new() {
        let vec = Vec2::new(1.0, 2.0);
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
    }

    #[test]
    fn test_vec2_distance_squared() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert_eq!(a.distance_squared(b), 25.0);

        let c = Vec2::new(1.0, 1.0);
        let d = Vec2::new(1.0, 1.0);
        assert_eq!(c.distance_squared(d), 0.0);
    }

    #[test]
    fn test_blend_node_clip() {
        let node = BlendNode::clip("Idle".to_string(), 1.0);
        match node {
            BlendNode::Clip(clip) => {
                assert_eq!(clip.animation_name, "Idle");
                assert_eq!(clip.speed, 1.0);
            }
            _ => panic!("Expected Clip variant"),
        }
    }

    #[test]
    fn test_blend_node_blend_2d() {
        let samples = vec![BlendSample {
            position: Vec2::new(0.0, 0.0),
            animation: AnimationClip::new("Idle".to_string(), 1.0),
        }];

        let node = BlendNode::blend_2d("speed".to_string(), "direction".to_string(), samples);

        match node {
            BlendNode::Blend2D {
                x_param,
                y_param,
                samples,
            } => {
                assert_eq!(x_param, "speed");
                assert_eq!(y_param, "direction");
                assert_eq!(samples.len(), 1);
            }
            _ => panic!("Expected Blend2D variant"),
        }
    }

    #[test]
    fn test_blend_node_additive() {
        let base = BlendNode::clip("Walk".to_string(), 1.0);
        let additive = BlendNode::clip("HitReaction".to_string(), 1.0);

        let node = BlendNode::additive(base, additive, 0.5);

        match node {
            BlendNode::Additive {
                base: _,
                additive: _,
                weight,
            } => {
                assert_eq!(weight, 0.5);
            }
            _ => panic!("Expected Additive variant"),
        }
    }

    #[test]
    fn test_blend_node_layered() {
        let upper = BlendNode::clip("UpperIdle".to_string(), 1.0);
        let lower = BlendNode::clip("Walk".to_string(), 1.0);

        let node = BlendNode::layered(vec![(upper, 1.0), (lower, 0.8)]);

        match node {
            BlendNode::LayeredBlend { layers } => {
                assert_eq!(layers.len(), 2);
                assert_eq!(layers[0].1, 1.0);
                assert_eq!(layers[1].1, 0.8);
            }
            _ => panic!("Expected LayeredBlend variant"),
        }
    }

    #[test]
    fn test_blend_node_validate_clip_success() {
        let node = BlendNode::clip("Walk".to_string(), 1.0);
        assert!(node.validate().is_ok());
    }

    #[test]
    fn test_blend_node_validate_clip_empty_name() {
        let node = BlendNode::clip("".to_string(), 1.0);
        let result = node.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty name"));
    }

    #[test]
    fn test_blend_node_validate_clip_invalid_speed() {
        let node = BlendNode::clip("Walk".to_string(), 0.0);
        let result = node.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid speed"));
    }

    #[test]
    fn test_blend_node_validate_blend_2d_success() {
        let samples = vec![BlendSample {
            position: Vec2::new(0.0, 0.0),
            animation: AnimationClip::new("Idle".to_string(), 1.0),
        }];

        let node = BlendNode::blend_2d("speed".to_string(), "direction".to_string(), samples);
        assert!(node.validate().is_ok());
    }

    #[test]
    fn test_blend_node_validate_blend_2d_no_samples() {
        let node = BlendNode::blend_2d("speed".to_string(), "direction".to_string(), vec![]);
        let result = node.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no samples"));
    }

    #[test]
    fn test_blend_node_validate_additive_success() {
        let base = BlendNode::clip("Walk".to_string(), 1.0);
        let additive = BlendNode::clip("Hit".to_string(), 1.0);
        let node = BlendNode::additive(base, additive, 0.5);

        assert!(node.validate().is_ok());
    }

    #[test]
    fn test_blend_node_validate_additive_invalid_weight() {
        let base = BlendNode::clip("Walk".to_string(), 1.0);
        let additive = BlendNode::clip("Hit".to_string(), 1.0);

        // Weight is clamped in constructor, so we need to manually create invalid node
        let node = BlendNode::Additive {
            base: Box::new(base),
            additive: Box::new(additive),
            weight: 1.5, // Invalid
        };

        let result = node.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    #[test]
    fn test_blend_node_validate_layered_success() {
        let upper = BlendNode::clip("Upper".to_string(), 1.0);
        let lower = BlendNode::clip("Lower".to_string(), 1.0);
        let node = BlendNode::layered(vec![(upper, 1.0), (lower, 0.5)]);

        assert!(node.validate().is_ok());
    }

    #[test]
    fn test_blend_node_validate_layered_no_layers() {
        let node = BlendNode::LayeredBlend { layers: vec![] };
        let result = node.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no layers"));
    }

    #[test]
    fn test_blend_node_serialization() {
        let node = BlendNode::clip("Walk".to_string(), 1.0);
        let serialized = ron::to_string(&node).unwrap();
        let deserialized: BlendNode = ron::from_str(&serialized).unwrap();
        assert_eq!(node, deserialized);
    }

    #[test]
    fn test_blend_sample_creation() {
        let sample = BlendSample {
            position: Vec2::new(1.0, 2.0),
            animation: AnimationClip::new("Test".to_string(), 1.5),
        };

        assert_eq!(sample.position.x, 1.0);
        assert_eq!(sample.position.y, 2.0);
        assert_eq!(sample.animation.animation_name, "Test");
        assert_eq!(sample.animation.speed, 1.5);
    }
}
