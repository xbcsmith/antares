// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inverse Kinematics (IK) system for procedural bone positioning
//!
//! This module provides IK solvers for positioning skeletal chains to reach
//! target positions. IK is useful for foot placement, hand reaching, and
//! procedural adjustments to animations.
//!
//! # Overview
//!
//! The IK system supports:
//!
//! - Two-bone IK chains (e.g., upper arm + forearm, thigh + shin)
//! - Target position and optional pole vector for elbow/knee direction
//! - Joint angle limits and constraints
//! - Chain length preservation
//!
//! # Examples
//!
//! ```
//! use antares::game::systems::ik::{IkChain, solve_two_bone_ik, Vec3};
//! use antares::domain::visual::skeleton::BoneId;
//!
//! // Define an IK chain for an arm
//! let chain = IkChain {
//!     bones: [0, 1], // Upper arm, forearm
//!     target: Vec3::new(2.0, 1.0, 0.0),
//!     pole_target: Some(Vec3::new(0.0, 1.0, 1.0)),
//! };
//!
//! // Solve IK (returns rotation quaternions for the two bones)
//! // let rotations = solve_two_bone_ik(&chain, &skeleton);
//! ```

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

use crate::domain::visual::skeleton::BoneId;

/// 3D vector for positions and directions
///
/// # Examples
///
/// ```
/// use antares::game::systems::ik::Vec3;
///
/// let position = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(position.x, 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    /// X coordinate
    pub x: f32,

    /// Y coordinate
    pub y: f32,

    /// Z coordinate
    pub z: f32,
}

impl Vec3 {
    /// Creates a new 3D vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let vec = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec.x, 1.0);
    /// assert_eq!(vec.y, 2.0);
    /// assert_eq!(vec.z, 3.0);
    /// ```
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Returns a zero vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let zero = Vec3::zero();
    /// assert_eq!(zero.x, 0.0);
    /// assert_eq!(zero.y, 0.0);
    /// assert_eq!(zero.z, 0.0);
    /// ```
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Calculates the length of the vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let vec = Vec3::new(3.0, 4.0, 0.0);
    /// assert_eq!(vec.length(), 5.0);
    /// ```
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalizes the vector to unit length
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let vec = Vec3::new(3.0, 4.0, 0.0);
    /// let normalized = vec.normalize();
    /// assert!((normalized.length() - 1.0).abs() < 0.001);
    /// ```
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::zero()
        }
    }

    /// Calculates the dot product with another vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.0, 1.0, 0.0);
    /// assert_eq!(a.dot(b), 0.0);
    /// ```
    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Calculates the cross product with another vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.0, 1.0, 0.0);
    /// let c = a.cross(b);
    /// assert_eq!(c.z, 1.0);
    /// ```
    pub fn cross(self, other: Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Multiplies by a scalar
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::Vec3;
    ///
    /// let vec = Vec3::new(1.0, 2.0, 3.0);
    /// let scaled = vec.scale(2.0);
    /// assert_eq!(scaled, Vec3::new(2.0, 4.0, 6.0));
    /// ```
    pub fn scale(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

/// Quaternion for 3D rotations
///
/// Stored as [x, y, z, w] where w is the scalar component.
///
/// # Examples
///
/// ```
/// use antares::game::systems::ik::Quat;
///
/// let identity = Quat::identity();
/// assert_eq!(identity, [0.0, 0.0, 0.0, 1.0]);
/// ```
pub type Quat = [f32; 4];

/// Implement subtraction for Vec3
impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Implement addition for Vec3
impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

/// IK chain definition
///
/// Defines a two-bone IK chain with target position and optional pole vector.
/// The pole vector controls the direction of the middle joint (e.g., elbow or knee).
///
/// # Fields
///
/// * `bones` - Array of two bone IDs [parent, child]
/// * `target` - Target position for the end of the chain
/// * `pole_target` - Optional pole vector for joint direction control
///
/// # Examples
///
/// ```
/// use antares::game::systems::ik::{IkChain, Vec3};
///
/// let arm_chain = IkChain {
///     bones: [0, 1], // Upper arm, forearm
///     target: Vec3::new(2.0, 0.5, 0.0),
///     pole_target: Some(Vec3::new(0.0, 1.0, 0.0)),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IkChain {
    /// Two bone IDs forming the chain [parent, child]
    pub bones: [BoneId; 2],

    /// Target position for the end effector
    pub target: Vec3,

    /// Optional pole target for controlling joint bend direction
    pub pole_target: Option<Vec3>,
}

impl IkChain {
    /// Creates a new IK chain
    ///
    /// # Arguments
    ///
    /// * `bones` - Array of two bone IDs
    /// * `target` - Target position
    /// * `pole_target` - Optional pole vector
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ik::{IkChain, Vec3};
    ///
    /// let chain = IkChain::new([0, 1], Vec3::new(1.0, 2.0, 3.0), None);
    /// assert_eq!(chain.bones, [0, 1]);
    /// ```
    pub fn new(bones: [BoneId; 2], target: Vec3, pole_target: Option<Vec3>) -> Self {
        Self {
            bones,
            target,
            pole_target,
        }
    }
}

/// Solves two-bone IK to reach a target position
///
/// Calculates the rotation quaternions needed for two bones to position the
/// end effector at the target location while preserving bone lengths.
///
/// # Arguments
///
/// * `root_pos` - Position of the root bone
/// * `mid_pos` - Position of the middle joint
/// * `end_pos` - Position of the end effector
/// * `target` - Target position to reach
/// * `pole_target` - Optional pole vector for joint direction
///
/// # Returns
///
/// Returns `[root_rotation, mid_rotation]` as quaternions
///
/// # Algorithm
///
/// Uses the law of cosines to calculate joint angles, then constructs
/// quaternions to achieve those angles while respecting the pole vector.
///
/// # Examples
///
/// ```
/// use antares::game::systems::ik::{solve_two_bone_ik, Vec3};
///
/// let root = Vec3::new(0.0, 0.0, 0.0);
/// let mid = Vec3::new(1.0, 0.0, 0.0);
/// let end = Vec3::new(2.0, 0.0, 0.0);
/// let target = Vec3::new(1.5, 1.0, 0.0);
///
/// let rotations = solve_two_bone_ik(root, mid, end, target, None);
/// // Returns quaternions for both joints
/// ```
pub fn solve_two_bone_ik(
    root_pos: Vec3,
    mid_pos: Vec3,
    end_pos: Vec3,
    target: Vec3,
    pole_target: Option<Vec3>,
) -> [Quat; 2] {
    // Calculate bone lengths
    let upper_length = (mid_pos - root_pos).length();
    let lower_length = (end_pos - mid_pos).length();

    // Calculate distance to target
    let target_dir = target - root_pos;
    let target_distance = target_dir.length();

    // Clamp target distance to reachable range
    let max_reach = upper_length + lower_length;
    let min_reach = (upper_length - lower_length).abs();

    let clamped_distance = target_distance.clamp(min_reach + 0.001, max_reach - 0.001);

    // Calculate angles using law of cosines
    let cos_upper = (upper_length * upper_length + clamped_distance * clamped_distance
        - lower_length * lower_length)
        / (2.0 * upper_length * clamped_distance);

    let cos_mid = (upper_length * upper_length + lower_length * lower_length
        - clamped_distance * clamped_distance)
        / (2.0 * upper_length * lower_length);

    let upper_angle = cos_upper.clamp(-1.0, 1.0).acos();
    let mid_angle = std::f32::consts::PI - cos_mid.clamp(-1.0, 1.0).acos();

    // Calculate rotation axis based on pole target
    let target_normalized = if target_distance > 0.0 {
        target_dir.scale(1.0 / target_distance)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    };

    let pole_dir = if let Some(pole) = pole_target {
        (pole - root_pos).normalize()
    } else {
        Vec3::new(0.0, 1.0, 0.0) // Default pole direction (up)
    };

    // Calculate rotation axis perpendicular to target direction
    let rotation_axis = target_normalized.cross(pole_dir).normalize();

    // Create quaternions for the rotations
    let root_quat = quat_from_axis_angle(rotation_axis, upper_angle);
    let mid_quat = quat_from_axis_angle(rotation_axis, mid_angle);

    [root_quat, mid_quat]
}

/// Creates a quaternion from an axis and angle
///
/// # Arguments
///
/// * `axis` - Rotation axis (should be normalized)
/// * `angle` - Rotation angle in radians
///
/// # Returns
///
/// Quaternion [x, y, z, w]
fn quat_from_axis_angle(axis: Vec3, angle: f32) -> Quat {
    let half_angle = angle * 0.5;
    let sin_half = half_angle.sin();
    let cos_half = half_angle.cos();

    [
        axis.x * sin_half,
        axis.y * sin_half,
        axis.z * sin_half,
        cos_half,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_new() {
        let vec = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_vec3_zero() {
        let zero = Vec3::zero();
        assert_eq!(zero.x, 0.0);
        assert_eq!(zero.y, 0.0);
        assert_eq!(zero.z, 0.0);
    }

    #[test]
    fn test_vec3_length() {
        let vec = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(vec.length(), 5.0);
    }

    #[test]
    fn test_vec3_normalize() {
        let vec = Vec3::new(3.0, 4.0, 0.0);
        let normalized = vec.normalize();
        assert!((normalized.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a.dot(b), 32.0); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn test_vec3_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(b);
        assert_eq!(c.x, 0.0);
        assert_eq!(c.y, 0.0);
        assert_eq!(c.z, 1.0);
    }

    #[test]
    fn test_vec3_sub() {
        let a = Vec3::new(5.0, 3.0, 1.0);
        let b = Vec3::new(2.0, 1.0, 0.0);
        let c = a - b;
        assert_eq!(c, Vec3::new(3.0, 2.0, 1.0));
    }

    #[test]
    fn test_vec3_add() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let c = a + b;
        assert_eq!(c, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn test_vec3_scale() {
        let vec = Vec3::new(1.0, 2.0, 3.0);
        let scaled = vec.scale(2.0);
        assert_eq!(scaled, Vec3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn test_ik_chain_new() {
        let chain = IkChain::new([0, 1], Vec3::new(1.0, 2.0, 3.0), None);
        assert_eq!(chain.bones, [0, 1]);
        assert_eq!(chain.target, Vec3::new(1.0, 2.0, 3.0));
        assert!(chain.pole_target.is_none());
    }

    #[test]
    fn test_ik_chain_with_pole() {
        let chain = IkChain::new(
            [0, 1],
            Vec3::new(1.0, 2.0, 3.0),
            Some(Vec3::new(0.0, 1.0, 0.0)),
        );
        assert!(chain.pole_target.is_some());
    }

    #[test]
    fn test_solve_two_bone_ik_reachable() {
        let root = Vec3::new(0.0, 0.0, 0.0);
        let mid = Vec3::new(1.0, 0.0, 0.0);
        let end = Vec3::new(2.0, 0.0, 0.0);
        let target = Vec3::new(1.5, 0.5, 0.0);

        let rotations = solve_two_bone_ik(root, mid, end, target, None);

        // Should return two quaternions
        assert_eq!(rotations.len(), 2);

        // Quaternions should have unit length (approximately)
        let quat1_len = (rotations[0][0] * rotations[0][0]
            + rotations[0][1] * rotations[0][1]
            + rotations[0][2] * rotations[0][2]
            + rotations[0][3] * rotations[0][3])
            .sqrt();
        assert!((quat1_len - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_solve_two_bone_ik_with_pole() {
        let root = Vec3::new(0.0, 0.0, 0.0);
        let mid = Vec3::new(1.0, 0.0, 0.0);
        let end = Vec3::new(2.0, 0.0, 0.0);
        let target = Vec3::new(1.5, 0.5, 0.0);
        let pole = Vec3::new(0.0, 1.0, 0.0);

        let rotations = solve_two_bone_ik(root, mid, end, target, Some(pole));

        // Should return two quaternions
        assert_eq!(rotations.len(), 2);
    }

    #[test]
    fn test_quat_from_axis_angle_identity() {
        let axis = Vec3::new(0.0, 1.0, 0.0);
        let angle = 0.0;
        let quat = quat_from_axis_angle(axis, angle);

        // Identity quaternion
        assert_eq!(quat[0], 0.0);
        assert_eq!(quat[1], 0.0);
        assert_eq!(quat[2], 0.0);
        assert_eq!(quat[3], 1.0);
    }

    #[test]
    fn test_quat_from_axis_angle_90_degrees() {
        let axis = Vec3::new(0.0, 1.0, 0.0);
        let angle = std::f32::consts::FRAC_PI_2; // 90 degrees
        let quat = quat_from_axis_angle(axis, angle);

        // 90 degree rotation around Y axis
        assert!((quat[3] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.001);
    }

    #[test]
    fn test_ik_chain_serialization() {
        let chain = IkChain::new([0, 1], Vec3::new(1.0, 2.0, 3.0), None);
        let serialized = ron::to_string(&chain).unwrap();
        let deserialized: IkChain = ron::from_str(&serialized).unwrap();
        assert_eq!(chain, deserialized);
    }
}
