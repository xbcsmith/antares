// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skeletal hierarchy system for advanced creature animation
//!
//! This module provides the foundation for skeletal animation by defining
//! bone hierarchies and skeletal structures. Bones can be organized in parent-child
//! relationships, allowing for hierarchical transformations.
//!
//! # Overview
//!
//! The skeletal system supports:
//!
//! - Hierarchical bone structures with parent-child relationships
//! - Rest pose and inverse bind pose matrices for skinning
//! - Bone traversal utilities for animation application
//! - Validation of skeletal hierarchies
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::skeleton::{Skeleton, Bone, BoneId};
//! use antares::domain::visual::MeshTransform;
//!
//! // Create a simple two-bone skeleton (upper arm -> forearm)
//! let upper_arm = Bone {
//!     id: 0,
//!     name: "upper_arm".to_string(),
//!     parent: None,
//!     rest_transform: MeshTransform::translation(0.0, 1.0, 0.0),
//!     inverse_bind_pose: [[1.0, 0.0, 0.0, 0.0],
//!                         [0.0, 1.0, 0.0, -1.0],
//!                         [0.0, 0.0, 1.0, 0.0],
//!                         [0.0, 0.0, 0.0, 1.0]],
//! };
//!
//! let forearm = Bone {
//!     id: 1,
//!     name: "forearm".to_string(),
//!     parent: Some(0),
//!     rest_transform: MeshTransform::translation(0.0, 1.0, 0.0),
//!     inverse_bind_pose: [[1.0, 0.0, 0.0, 0.0],
//!                         [0.0, 1.0, 0.0, -2.0],
//!                         [0.0, 0.0, 1.0, 0.0],
//!                         [0.0, 0.0, 0.0, 1.0]],
//! };
//!
//! let skeleton = Skeleton {
//!     bones: vec![upper_arm, forearm],
//!     root_bone: 0,
//! };
//!
//! assert_eq!(skeleton.bones.len(), 2);
//! assert!(skeleton.validate().is_ok());
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::visual::MeshTransform;

/// Unique identifier for a bone in a skeleton
///
/// BoneId is an index into the skeleton's bone array. It's used to reference
/// bones in parent-child relationships and animation tracks.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeleton::BoneId;
///
/// let root_bone: BoneId = 0;
/// let child_bone: BoneId = 1;
/// ```
pub type BoneId = usize;

/// 4x4 matrix for skeletal transformations
///
/// Column-major order matrix used for skinning calculations.
/// Format: `[column0, column1, column2, column3]` where each column is `[x, y, z, w]`
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeleton::Mat4;
///
/// // Identity matrix
/// let identity: Mat4 = [
///     [1.0, 0.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0, 0.0],
///     [0.0, 0.0, 1.0, 0.0],
///     [0.0, 0.0, 0.0, 1.0],
/// ];
/// ```
pub type Mat4 = [[f32; 4]; 4];

/// A single bone in a skeletal hierarchy
///
/// Bones are the fundamental building blocks of a skeleton. Each bone has:
/// - A unique identifier within its skeleton
/// - A name for reference and debugging
/// - An optional parent bone (None for root bones)
/// - A rest transform (default position/rotation/scale)
/// - An inverse bind pose matrix for skinning
///
/// # Fields
///
/// * `id` - Unique identifier for this bone
/// * `name` - Human-readable name (e.g., "left_forearm", "spine_2")
/// * `parent` - Parent bone ID, or None if this is a root bone
/// * `rest_transform` - Transform in rest pose (relative to parent)
/// * `inverse_bind_pose` - Matrix for converting mesh-space to bone-space
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeleton::{Bone, Mat4};
/// use antares::domain::visual::MeshTransform;
///
/// let root_bone = Bone {
///     id: 0,
///     name: "root".to_string(),
///     parent: None,
///     rest_transform: MeshTransform::identity(),
///     inverse_bind_pose: [[1.0, 0.0, 0.0, 0.0],
///                         [0.0, 1.0, 0.0, 0.0],
///                         [0.0, 0.0, 1.0, 0.0],
///                         [0.0, 0.0, 0.0, 1.0]],
/// };
///
/// assert_eq!(root_bone.id, 0);
/// assert_eq!(root_bone.name, "root");
/// assert!(root_bone.parent.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bone {
    /// Unique identifier for this bone
    pub id: BoneId,

    /// Human-readable name for this bone
    pub name: String,

    /// Parent bone ID (None for root bones)
    pub parent: Option<BoneId>,

    /// Rest pose transform (relative to parent)
    pub rest_transform: MeshTransform,

    /// Inverse bind pose matrix for skinning calculations
    pub inverse_bind_pose: Mat4,
}

impl Bone {
    /// Creates a new bone with the given parameters
    ///
    /// # Arguments
    ///
    /// * `id` - Unique bone identifier
    /// * `name` - Human-readable bone name
    /// * `parent` - Optional parent bone ID
    /// * `rest_transform` - Rest pose transform
    /// * `inverse_bind_pose` - Inverse bind pose matrix
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Bone, Mat4};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix: Mat4 = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let bone = Bone::new(
    ///     0,
    ///     "pelvis".to_string(),
    ///     None,
    ///     MeshTransform::identity(),
    ///     identity_matrix,
    /// );
    ///
    /// assert_eq!(bone.id, 0);
    /// assert_eq!(bone.name, "pelvis");
    /// ```
    pub fn new(
        id: BoneId,
        name: String,
        parent: Option<BoneId>,
        rest_transform: MeshTransform,
        inverse_bind_pose: Mat4,
    ) -> Self {
        Self {
            id,
            name,
            parent,
            rest_transform,
            inverse_bind_pose,
        }
    }

    /// Checks if this bone is a root bone (has no parent)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::Bone;
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    /// let child = Bone::new(1, "child".to_string(), Some(0),
    ///                       MeshTransform::identity(), identity_matrix);
    ///
    /// assert!(root.is_root());
    /// assert!(!child.is_root());
    /// ```
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

/// A complete skeletal hierarchy
///
/// A skeleton is a collection of bones organized in a hierarchical structure.
/// It has one or more root bones and maintains the complete bone hierarchy
/// for animation and skinning.
///
/// # Fields
///
/// * `bones` - All bones in the skeleton
/// * `root_bone` - The primary root bone ID (for traversal starting point)
///
/// # Examples
///
/// ```
/// use antares::domain::visual::skeleton::{Skeleton, Bone};
/// use antares::domain::visual::MeshTransform;
///
/// let identity_matrix = [
///     [1.0, 0.0, 0.0, 0.0],
///     [0.0, 1.0, 0.0, 0.0],
///     [0.0, 0.0, 1.0, 0.0],
///     [0.0, 0.0, 0.0, 1.0],
/// ];
///
/// let root = Bone::new(0, "root".to_string(), None,
///                      MeshTransform::identity(), identity_matrix);
/// let child = Bone::new(1, "child".to_string(), Some(0),
///                       MeshTransform::identity(), identity_matrix);
///
/// let skeleton = Skeleton {
///     bones: vec![root, child],
///     root_bone: 0,
/// };
///
/// assert_eq!(skeleton.bones.len(), 2);
/// assert_eq!(skeleton.root_bone, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    /// All bones in the skeleton
    pub bones: Vec<Bone>,

    /// The primary root bone ID
    pub root_bone: BoneId,
}

impl Skeleton {
    /// Creates a new skeleton with the given bones and root
    ///
    /// # Arguments
    ///
    /// * `bones` - Vector of all bones in the skeleton
    /// * `root_bone` - ID of the primary root bone
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    ///
    /// let skeleton = Skeleton::new(vec![root], 0);
    /// assert_eq!(skeleton.bones.len(), 1);
    /// ```
    pub fn new(bones: Vec<Bone>, root_bone: BoneId) -> Self {
        Self { bones, root_bone }
    }

    /// Gets a bone by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The bone ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&Bone)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    /// let skeleton = Skeleton::new(vec![root], 0);
    ///
    /// let bone = skeleton.get_bone(0);
    /// assert!(bone.is_some());
    /// assert_eq!(bone.unwrap().name, "root");
    /// ```
    pub fn get_bone(&self, id: BoneId) -> Option<&Bone> {
        self.bones.iter().find(|b| b.id == id)
    }

    /// Gets a mutable reference to a bone by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The bone ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&mut Bone)` if found, `None` otherwise
    pub fn get_bone_mut(&mut self, id: BoneId) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.id == id)
    }

    /// Finds a bone by name
    ///
    /// # Arguments
    ///
    /// * `name` - The bone name to search for
    ///
    /// # Returns
    ///
    /// Returns `Some(&Bone)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let pelvis = Bone::new(0, "pelvis".to_string(), None,
    ///                        MeshTransform::identity(), identity_matrix);
    /// let skeleton = Skeleton::new(vec![pelvis], 0);
    ///
    /// let bone = skeleton.find_bone_by_name("pelvis");
    /// assert!(bone.is_some());
    /// assert_eq!(bone.unwrap().id, 0);
    /// ```
    pub fn find_bone_by_name(&self, name: &str) -> Option<&Bone> {
        self.bones.iter().find(|b| b.name == name)
    }

    /// Gets all child bones of a given parent bone
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The parent bone ID
    ///
    /// # Returns
    ///
    /// Returns a vector of references to all child bones
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    /// let child1 = Bone::new(1, "child1".to_string(), Some(0),
    ///                        MeshTransform::identity(), identity_matrix);
    /// let child2 = Bone::new(2, "child2".to_string(), Some(0),
    ///                        MeshTransform::identity(), identity_matrix);
    ///
    /// let skeleton = Skeleton::new(vec![root, child1, child2], 0);
    /// let children = skeleton.get_children(0);
    ///
    /// assert_eq!(children.len(), 2);
    /// ```
    pub fn get_children(&self, parent_id: BoneId) -> Vec<&Bone> {
        self.bones
            .iter()
            .filter(|b| b.parent == Some(parent_id))
            .collect()
    }

    /// Validates the skeletal hierarchy
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `Err(String)` with error description
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Skeleton has no bones
    /// - Root bone ID is out of bounds
    /// - Root bone has a parent
    /// - Any bone references a non-existent parent
    /// - Bone IDs are not unique
    /// - Circular parent references exist
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    /// let skeleton = Skeleton::new(vec![root], 0);
    ///
    /// assert!(skeleton.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // Check skeleton has bones
        if self.bones.is_empty() {
            return Err("Skeleton has no bones".to_string());
        }

        // Check root bone exists
        if self.root_bone >= self.bones.len() {
            return Err(format!(
                "Root bone ID {} is out of bounds (skeleton has {} bones)",
                self.root_bone,
                self.bones.len()
            ));
        }

        // Check root bone has no parent
        if let Some(root) = self.get_bone(self.root_bone) {
            if root.parent.is_some() {
                return Err(format!(
                    "Root bone '{}' (ID {}) has a parent",
                    root.name, root.id
                ));
            }
        }

        // Check all bone IDs are unique and match their index
        for (index, bone) in self.bones.iter().enumerate() {
            if bone.id != index {
                return Err(format!(
                    "Bone '{}' has ID {} but is at index {}",
                    bone.name, bone.id, index
                ));
            }
        }

        // Check all parent references are valid
        for bone in &self.bones {
            if let Some(parent_id) = bone.parent {
                if parent_id >= self.bones.len() {
                    return Err(format!(
                        "Bone '{}' references non-existent parent ID {}",
                        bone.name, parent_id
                    ));
                }

                // Check for self-reference
                if parent_id == bone.id {
                    return Err(format!("Bone '{}' has itself as parent", bone.name));
                }
            }
        }

        // Check for circular references
        for bone in &self.bones {
            if self.has_circular_reference(bone.id) {
                return Err(format!(
                    "Bone '{}' has circular parent reference",
                    bone.name
                ));
            }
        }

        Ok(())
    }

    /// Checks if a bone has a circular parent reference
    fn has_circular_reference(&self, bone_id: BoneId) -> bool {
        let mut visited = vec![false; self.bones.len()];
        let mut current_id = bone_id;

        loop {
            if visited[current_id] {
                return true; // Circular reference detected
            }

            visited[current_id] = true;

            if let Some(bone) = self.get_bone(current_id) {
                if let Some(parent_id) = bone.parent {
                    current_id = parent_id;
                } else {
                    return false; // Reached root, no circular reference
                }
            } else {
                return false; // Invalid bone ID
            }
        }
    }

    /// Returns the number of bones in the skeleton
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::skeleton::{Skeleton, Bone};
    /// use antares::domain::visual::MeshTransform;
    ///
    /// let identity_matrix = [
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0],
    /// ];
    ///
    /// let root = Bone::new(0, "root".to_string(), None,
    ///                      MeshTransform::identity(), identity_matrix);
    /// let skeleton = Skeleton::new(vec![root], 0);
    ///
    /// assert_eq!(skeleton.bone_count(), 1);
    /// ```
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_identity_matrix() -> Mat4 {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    #[test]
    fn test_bone_new() {
        let bone = Bone::new(
            0,
            "test_bone".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        assert_eq!(bone.id, 0);
        assert_eq!(bone.name, "test_bone");
        assert!(bone.parent.is_none());
    }

    #[test]
    fn test_bone_is_root() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child = Bone::new(
            1,
            "child".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        assert!(root.is_root());
        assert!(!child.is_root());
    }

    #[test]
    fn test_skeleton_new() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root], 0);

        assert_eq!(skeleton.bones.len(), 1);
        assert_eq!(skeleton.root_bone, 0);
    }

    #[test]
    fn test_skeleton_get_bone() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root], 0);

        let bone = skeleton.get_bone(0);
        assert!(bone.is_some());
        assert_eq!(bone.unwrap().name, "root");

        let missing = skeleton.get_bone(99);
        assert!(missing.is_none());
    }

    #[test]
    fn test_skeleton_find_bone_by_name() {
        let root = Bone::new(
            0,
            "pelvis".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child = Bone::new(
            1,
            "spine".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root, child], 0);

        let bone = skeleton.find_bone_by_name("spine");
        assert!(bone.is_some());
        assert_eq!(bone.unwrap().id, 1);

        let missing = skeleton.find_bone_by_name("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_skeleton_get_children() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child1 = Bone::new(
            1,
            "child1".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child2 = Bone::new(
            2,
            "child2".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let grandchild = Bone::new(
            3,
            "grandchild".to_string(),
            Some(1),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root, child1, child2, grandchild], 0);

        let root_children = skeleton.get_children(0);
        assert_eq!(root_children.len(), 2);

        let child1_children = skeleton.get_children(1);
        assert_eq!(child1_children.len(), 1);
        assert_eq!(child1_children[0].name, "grandchild");
    }

    #[test]
    fn test_skeleton_validate_success() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child = Bone::new(
            1,
            "child".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root, child], 0);

        assert!(skeleton.validate().is_ok());
    }

    #[test]
    fn test_skeleton_validate_empty() {
        let skeleton = Skeleton::new(vec![], 0);

        let result = skeleton.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no bones"));
    }

    #[test]
    fn test_skeleton_validate_root_out_of_bounds() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root], 99);

        let result = skeleton.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_skeleton_validate_root_has_parent() {
        let fake_root = Bone::new(
            0,
            "fake_root".to_string(),
            Some(1),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![fake_root], 0);

        let result = skeleton.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has a parent"));
    }

    #[test]
    fn test_skeleton_validate_invalid_parent_reference() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child = Bone::new(
            1,
            "child".to_string(),
            Some(99),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root, child], 0);

        let result = skeleton.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent parent"));
    }

    #[test]
    fn test_skeleton_bone_count() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let child = Bone::new(
            1,
            "child".to_string(),
            Some(0),
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root, child], 0);

        assert_eq!(skeleton.bone_count(), 2);
    }

    #[test]
    fn test_skeleton_serialization() {
        let root = Bone::new(
            0,
            "root".to_string(),
            None,
            MeshTransform::identity(),
            create_identity_matrix(),
        );

        let skeleton = Skeleton::new(vec![root], 0);

        // Test RON serialization
        let serialized = ron::to_string(&skeleton).unwrap();
        let deserialized: Skeleton = ron::from_str(&serialized).unwrap();

        assert_eq!(skeleton, deserialized);
    }
}
