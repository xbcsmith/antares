// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skeletal animation system for per-bone animations
//!
//! This module provides skeletal animation capabilities with per-bone animation tracks.
//! Unlike the simple keyframe system, skeletal animations apply transformations to
//! individual bones in a skeleton, allowing for complex character animations.
//!
//! # Overview
//!
//! The skeletal animation system supports:
//!
//! - Per-bone animation tracks with independent keyframes
//! - Quaternion-based rotations for smooth interpolation (SLERP)
//! - Position and scale with linear interpolation (LERP)
//! - Looping and one-shot animations
//! - Animation sampling at arbitrary time points
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
//! use std::collections::HashMap;
//!
//! // Create a simple arm wave animation
//! let mut bone_tracks = HashMap::new();
//! bone_tracks.insert(0, vec![
//!     BoneKeyframe {
//!         time: 0.0,
//!         position: [0.0, 0.0, 0.0],
//!         rotation: [0.0, 0.0, 0.0, 1.0],
//!         scale: [1.0, 1.0, 1.0],
//!     },
//!     BoneKeyframe {
//!         time: 0.5,
//!         position: [0.0, 0.0, 0.0],
//!         rotation: [0.0, 0.0, 0.707, 0.707], // 90 degree rotation
//!         scale: [1.0, 1.0, 1.0],
//!     },
//! ]);
//!
//! let animation = SkeletalAnimation {
//!     name: "Wave".to_string(),
//!     duration: 1.0,
//!     bone_tracks,
//!     looping: true,
//! };
//!
//! assert_eq!(animation.name, "Wave");
//! assert_eq!(animation.duration, 1.0);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::visual::skeleton::BoneId;

/// A single keyframe for a bone in a skeletal animation
///
/// Stores the transformation of a bone at a specific time. Uses quaternions
/// for rotation to enable smooth spherical linear interpolation (SLERP).
///
/// # Fields
///
/// * `time` - Time in seconds when this keyframe occurs
/// * `position` - Translation as [x, y, z]
/// * `rotation` - Rotation as quaternion [x, y, z, w]
/// * `scale` - Scale as [x, y, z]
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeletal_animation::BoneKeyframe;
///
/// let keyframe = BoneKeyframe {
///     time: 0.5,
///     position: [1.0, 2.0, 3.0],
///     rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
///     scale: [1.0, 1.0, 1.0],
/// };
///
/// assert_eq!(keyframe.time, 0.5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoneKeyframe {
    /// Time in seconds when this keyframe occurs
    pub time: f32,

    /// Position as [x, y, z]
    pub position: [f32; 3],

    /// Rotation as quaternion [x, y, z, w]
    pub rotation: [f32; 4],

    /// Scale as [x, y, z]
    pub scale: [f32; 3],
}

impl BoneKeyframe {
    /// Creates a new bone keyframe with identity transform
    ///
    /// # Arguments
    ///
    /// * `time` - Time in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::BoneKeyframe;
    ///
    /// let keyframe = BoneKeyframe::identity(0.5);
    /// assert_eq!(keyframe.time, 0.5);
    /// assert_eq!(keyframe.position, [0.0, 0.0, 0.0]);
    /// assert_eq!(keyframe.rotation, [0.0, 0.0, 0.0, 1.0]);
    /// assert_eq!(keyframe.scale, [1.0, 1.0, 1.0]);
    /// ```
    pub fn identity(time: f32) -> Self {
        Self {
            time,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Creates a new bone keyframe with specific values
    ///
    /// # Arguments
    ///
    /// * `time` - Time in seconds
    /// * `position` - Position as [x, y, z]
    /// * `rotation` - Rotation as quaternion [x, y, z, w]
    /// * `scale` - Scale as [x, y, z]
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::BoneKeyframe;
    ///
    /// let keyframe = BoneKeyframe::new(
    ///     0.5,
    ///     [1.0, 2.0, 3.0],
    ///     [0.0, 0.0, 0.707, 0.707],
    ///     [1.5, 1.5, 1.5],
    /// );
    ///
    /// assert_eq!(keyframe.time, 0.5);
    /// assert_eq!(keyframe.position, [1.0, 2.0, 3.0]);
    /// ```
    pub fn new(time: f32, position: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Self {
        Self {
            time,
            position,
            rotation,
            scale,
        }
    }
}

/// A complete skeletal animation with per-bone tracks
///
/// Defines an animation as a collection of bone animation tracks. Each bone
/// can have its own independent keyframes, allowing for complex coordinated
/// movements.
///
/// # Fields
///
/// * `name` - Display name for the animation
/// * `duration` - Total duration in seconds
/// * `bone_tracks` - Map of bone IDs to their keyframe sequences
/// * `looping` - Whether the animation loops
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
/// use std::collections::HashMap;
///
/// let mut bone_tracks = HashMap::new();
/// bone_tracks.insert(0, vec![
///     BoneKeyframe::identity(0.0),
///     BoneKeyframe::identity(1.0),
/// ]);
///
/// let animation = SkeletalAnimation {
///     name: "Idle".to_string(),
///     duration: 1.0,
///     bone_tracks,
///     looping: true,
/// };
///
/// assert_eq!(animation.name, "Idle");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletalAnimation {
    /// Display name for this animation
    pub name: String,

    /// Total duration in seconds
    pub duration: f32,

    /// Map of bone IDs to their keyframe sequences
    pub bone_tracks: HashMap<BoneId, Vec<BoneKeyframe>>,

    /// Whether the animation loops
    pub looping: bool,
}

impl SkeletalAnimation {
    /// Creates a new skeletal animation
    ///
    /// # Arguments
    ///
    /// * `name` - Animation name
    /// * `duration` - Total duration in seconds
    /// * `looping` - Whether to loop
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::SkeletalAnimation;
    ///
    /// let animation = SkeletalAnimation::new("Walk".to_string(), 2.0, true);
    /// assert_eq!(animation.name, "Walk");
    /// assert_eq!(animation.duration, 2.0);
    /// assert!(animation.looping);
    /// assert!(animation.bone_tracks.is_empty());
    /// ```
    pub fn new(name: String, duration: f32, looping: bool) -> Self {
        Self {
            name,
            duration,
            bone_tracks: HashMap::new(),
            looping,
        }
    }

    /// Adds a bone track to the animation
    ///
    /// # Arguments
    ///
    /// * `bone_id` - The bone to animate
    /// * `keyframes` - Keyframes for this bone
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
    ///
    /// let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
    /// animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);
    ///
    /// assert_eq!(animation.bone_tracks.len(), 1);
    /// ```
    pub fn add_bone_track(&mut self, bone_id: BoneId, keyframes: Vec<BoneKeyframe>) {
        self.bone_tracks.insert(bone_id, keyframes);
    }

    /// Samples the animation at a specific time for a specific bone
    ///
    /// # Arguments
    ///
    /// * `bone_id` - The bone to sample
    /// * `time` - Time in seconds (will be wrapped if looping)
    ///
    /// # Returns
    ///
    /// Returns `Some(BoneKeyframe)` with interpolated transform, or `None` if
    /// the bone has no track.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
    ///
    /// let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
    /// animation.add_bone_track(0, vec![
    ///     BoneKeyframe::new(0.0, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
    ///     BoneKeyframe::new(1.0, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
    /// ]);
    ///
    /// let sample = animation.sample_bone(0, 0.5);
    /// assert!(sample.is_some());
    /// ```
    pub fn sample_bone(&self, bone_id: BoneId, time: f32) -> Option<BoneKeyframe> {
        let track = self.bone_tracks.get(&bone_id)?;

        if track.is_empty() {
            return None;
        }

        // Wrap time if looping
        let sample_time = if self.looping && self.duration > 0.0 {
            time % self.duration
        } else {
            time.min(self.duration)
        };

        // Find keyframes to interpolate between
        if track.len() == 1 {
            return Some(track[0].clone());
        }

        // Find the two keyframes surrounding sample_time
        let mut prev_keyframe = &track[0];
        let mut next_keyframe = &track[0];

        for keyframe in track {
            if keyframe.time <= sample_time {
                prev_keyframe = keyframe;
            }
            if keyframe.time >= sample_time {
                next_keyframe = keyframe;
                break;
            }
        }

        // If we're before the first keyframe or after the last, return that keyframe
        if sample_time <= track[0].time {
            return Some(track[0].clone());
        }
        if sample_time >= track.last().unwrap().time {
            return Some(track.last().unwrap().clone());
        }

        // Interpolate between prev and next keyframes
        let duration = next_keyframe.time - prev_keyframe.time;
        if duration <= 0.0 {
            return Some(prev_keyframe.clone());
        }

        let t = (sample_time - prev_keyframe.time) / duration;

        Some(BoneKeyframe {
            time: sample_time,
            position: lerp_vec3(prev_keyframe.position, next_keyframe.position, t),
            rotation: slerp_quat(prev_keyframe.rotation, next_keyframe.rotation, t),
            scale: lerp_vec3(prev_keyframe.scale, next_keyframe.scale, t),
        })
    }

    /// Validates the skeletal animation
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `Err(String)` with error description
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Duration is negative or zero
    /// - Name is empty
    /// - Any keyframe time is negative or exceeds duration
    /// - Keyframes are not sorted by time
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
    ///
    /// let mut animation = SkeletalAnimation::new("Valid".to_string(), 1.0, false);
    /// animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);
    ///
    /// assert!(animation.validate().is_ok());
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

        // Validate each bone track
        for (bone_id, keyframes) in &self.bone_tracks {
            if keyframes.is_empty() {
                return Err(format!("Bone {} has empty keyframe track", bone_id));
            }

            // Check keyframe times are valid and sorted
            let mut prev_time = -1.0;
            for keyframe in keyframes {
                if keyframe.time < 0.0 {
                    return Err(format!(
                        "Bone {} has keyframe with negative time: {}",
                        bone_id, keyframe.time
                    ));
                }

                if keyframe.time > self.duration {
                    return Err(format!(
                        "Bone {} has keyframe at time {} which exceeds duration {}",
                        bone_id, keyframe.time, self.duration
                    ));
                }

                if keyframe.time < prev_time {
                    return Err(format!(
                        "Bone {} has unsorted keyframes: {} comes after {}",
                        bone_id, keyframe.time, prev_time
                    ));
                }

                prev_time = keyframe.time;
            }
        }

        Ok(())
    }

    /// Returns the number of bones with animation tracks
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeletal_animation::{SkeletalAnimation, BoneKeyframe};
    ///
    /// let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
    /// animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);
    /// animation.add_bone_track(1, vec![BoneKeyframe::identity(0.0)]);
    ///
    /// assert_eq!(animation.bone_count(), 2);
    /// ```
    pub fn bone_count(&self) -> usize {
        self.bone_tracks.len()
    }
}

/// Linear interpolation for 3D vectors
fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Spherical linear interpolation for quaternions
///
/// Provides smooth rotation interpolation between two quaternions.
/// Uses SLERP algorithm for shortest-path rotation.
fn slerp_quat(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    // Normalize quaternions
    let qa = normalize_quat(a);
    let mut qb = normalize_quat(b);

    // Calculate dot product
    let mut dot = qa[0] * qb[0] + qa[1] * qb[1] + qa[2] * qb[2] + qa[3] * qb[3];

    // If dot product is negative, negate one quaternion to take shorter path
    if dot < 0.0 {
        qb = [-qb[0], -qb[1], -qb[2], -qb[3]];
        dot = -dot;
    }

    // If quaternions are very close, use linear interpolation
    if dot > 0.9995 {
        return normalize_quat([
            qa[0] + (qb[0] - qa[0]) * t,
            qa[1] + (qb[1] - qa[1]) * t,
            qa[2] + (qb[2] - qa[2]) * t,
            qa[3] + (qb[3] - qa[3]) * t,
        ]);
    }

    // Calculate angle between quaternions
    let theta = dot.clamp(-1.0, 1.0).acos();
    let theta_t = theta * t;

    // Calculate SLERP result
    let sin_theta = theta.sin();
    let sin_theta_t = theta_t.sin();
    let sin_theta_1_t = (theta - theta_t).sin();

    let scale_a = sin_theta_1_t / sin_theta;
    let scale_b = sin_theta_t / sin_theta;

    [
        qa[0] * scale_a + qb[0] * scale_b,
        qa[1] * scale_a + qb[1] * scale_b,
        qa[2] * scale_a + qb[2] * scale_b,
        qa[3] * scale_a + qb[3] * scale_b,
    ]
}

/// Normalizes a quaternion to unit length
fn normalize_quat(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len > 0.0 {
        [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
    } else {
        [0.0, 0.0, 0.0, 1.0] // Return identity quaternion if zero length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bone_keyframe_identity() {
        let keyframe = BoneKeyframe::identity(0.5);
        assert_eq!(keyframe.time, 0.5);
        assert_eq!(keyframe.position, [0.0, 0.0, 0.0]);
        assert_eq!(keyframe.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(keyframe.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_bone_keyframe_new() {
        let keyframe = BoneKeyframe::new(
            1.0,
            [1.0, 2.0, 3.0],
            [0.0, 0.0, 0.707, 0.707],
            [2.0, 2.0, 2.0],
        );
        assert_eq!(keyframe.time, 1.0);
        assert_eq!(keyframe.position, [1.0, 2.0, 3.0]);
        assert_eq!(keyframe.rotation, [0.0, 0.0, 0.707, 0.707]);
        assert_eq!(keyframe.scale, [2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_skeletal_animation_new() {
        let animation = SkeletalAnimation::new("Walk".to_string(), 2.0, true);
        assert_eq!(animation.name, "Walk");
        assert_eq!(animation.duration, 2.0);
        assert!(animation.looping);
        assert!(animation.bone_tracks.is_empty());
    }

    #[test]
    fn test_skeletal_animation_add_bone_track() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);
        animation.add_bone_track(1, vec![BoneKeyframe::identity(0.0)]);

        assert_eq!(animation.bone_tracks.len(), 2);
        assert!(animation.bone_tracks.contains_key(&0));
        assert!(animation.bone_tracks.contains_key(&1));
    }

    #[test]
    fn test_skeletal_animation_sample_bone_single_keyframe() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);

        let sample = animation.sample_bone(0, 0.5);
        assert!(sample.is_some());
        let sample = sample.unwrap();
        assert_eq!(sample.position, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_skeletal_animation_sample_bone_interpolation() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(
            0,
            vec![
                BoneKeyframe::new(0.0, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
                BoneKeyframe::new(1.0, [2.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
            ],
        );

        let sample = animation.sample_bone(0, 0.5).unwrap();
        // Should interpolate to middle position
        assert!((sample.position[0] - 1.0).abs() < 0.01);
        assert_eq!(sample.position[1], 0.0);
        assert_eq!(sample.position[2], 0.0);
    }

    #[test]
    fn test_skeletal_animation_sample_bone_before_first_keyframe() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(
            0,
            vec![BoneKeyframe::new(
                0.5,
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            )],
        );

        let sample = animation.sample_bone(0, 0.0).unwrap();
        // Should return first keyframe
        assert_eq!(sample.position, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_skeletal_animation_sample_bone_after_last_keyframe() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(
            0,
            vec![BoneKeyframe::new(
                0.5,
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            )],
        );

        let sample = animation.sample_bone(0, 1.5).unwrap();
        // Should return last keyframe
        assert_eq!(sample.position, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_skeletal_animation_sample_bone_looping() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, true);
        animation.add_bone_track(
            0,
            vec![
                BoneKeyframe::new(0.0, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
                BoneKeyframe::new(1.0, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
            ],
        );

        // Time 1.5 should wrap to 0.5
        let sample = animation.sample_bone(0, 1.5).unwrap();
        assert!((sample.position[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_skeletal_animation_sample_bone_missing_track() {
        let animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        let sample = animation.sample_bone(0, 0.5);
        assert!(sample.is_none());
    }

    #[test]
    fn test_skeletal_animation_validate_success() {
        let mut animation = SkeletalAnimation::new("Valid".to_string(), 1.0, false);
        animation.add_bone_track(
            0,
            vec![BoneKeyframe::identity(0.0), BoneKeyframe::identity(1.0)],
        );

        assert!(animation.validate().is_ok());
    }

    #[test]
    fn test_skeletal_animation_validate_negative_duration() {
        let animation = SkeletalAnimation::new("Invalid".to_string(), -1.0, false);
        let result = animation.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_skeletal_animation_validate_empty_name() {
        let animation = SkeletalAnimation::new("".to_string(), 1.0, false);
        let result = animation.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_skeletal_animation_validate_keyframe_exceeds_duration() {
        let mut animation = SkeletalAnimation::new("Invalid".to_string(), 1.0, false);
        animation.add_bone_track(0, vec![BoneKeyframe::identity(2.0)]);

        let result = animation.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds duration"));
    }

    #[test]
    fn test_skeletal_animation_validate_unsorted_keyframes() {
        let mut animation = SkeletalAnimation::new("Invalid".to_string(), 2.0, false);
        animation.add_bone_track(
            0,
            vec![BoneKeyframe::identity(1.0), BoneKeyframe::identity(0.5)],
        );

        let result = animation.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsorted"));
    }

    #[test]
    fn test_skeletal_animation_bone_count() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);
        animation.add_bone_track(1, vec![BoneKeyframe::identity(0.0)]);
        animation.add_bone_track(5, vec![BoneKeyframe::identity(0.0)]);

        assert_eq!(animation.bone_count(), 3);
    }

    #[test]
    fn test_lerp_vec3() {
        let a = [0.0, 0.0, 0.0];
        let b = [2.0, 4.0, 6.0];

        let result = lerp_vec3(a, b, 0.5);
        assert_eq!(result, [1.0, 2.0, 3.0]);

        let result = lerp_vec3(a, b, 0.0);
        assert_eq!(result, [0.0, 0.0, 0.0]);

        let result = lerp_vec3(a, b, 1.0);
        assert_eq!(result, [2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_normalize_quat() {
        let q = [0.0, 0.0, 0.0, 2.0];
        let normalized = normalize_quat(q);
        assert_eq!(normalized, [0.0, 0.0, 0.0, 1.0]);

        let q = [1.0, 1.0, 1.0, 1.0];
        let normalized = normalize_quat(q);
        let len = (normalized[0] * normalized[0]
            + normalized[1] * normalized[1]
            + normalized[2] * normalized[2]
            + normalized[3] * normalized[3])
            .sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_slerp_quat_identity() {
        let identity = [0.0, 0.0, 0.0, 1.0];
        let result = slerp_quat(identity, identity, 0.5);
        assert_eq!(result, identity);
    }

    #[test]
    fn test_skeletal_animation_serialization() {
        let mut animation = SkeletalAnimation::new("Test".to_string(), 1.0, false);
        animation.add_bone_track(0, vec![BoneKeyframe::identity(0.0)]);

        let serialized = ron::to_string(&animation).unwrap();
        let deserialized: SkeletalAnimation = ron::from_str(&serialized).unwrap();

        assert_eq!(animation, deserialized);
    }
}
