// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Billboard component for camera-facing sprites
//!
//! Billboards are entities that always face the camera, useful for:
//! - Character sprites (NPCs, Monsters, Recruitables)
//! - Decorative sprites that should be visible from any angle
//! - Particle effects and UI elements in 3D space
//!
//! # Y-Axis Locking
//!
//! - `lock_y: true` - Entity stays upright (rotates only around Y-axis)
//! - `lock_y: false` - Entity always faces camera (rotates on all axes)
//!
//! # Examples
//!
//! ```
//! use antares::game::components::billboard::Billboard;
//!
//! let character_billboard = Billboard { lock_y: true };
//! let particle_billboard = Billboard { lock_y: false };
//! ```

use bevy::prelude::*;

/// Component that makes an entity face the camera (billboard effect)
///
/// # Fields
///
/// * `lock_y` - If true, only rotates around Y-axis (stays upright)
///
/// # Behavior
///
/// Entities with this component will be rotated by `update_billboards` system
/// to face the active camera. This is updated every frame.
///
/// # Examples
///
/// ```
/// use antares::game::components::billboard::Billboard;
///
/// // Character sprite (stays upright)
/// let character_billboard = Billboard { lock_y: true };
///
/// // Particle effect (full rotation)
/// let particle_billboard = Billboard { lock_y: false };
/// ```
#[derive(Component)]
pub struct Billboard {
    /// Lock Y-axis rotation (true for characters standing upright)
    pub lock_y: bool,
}

impl Default for Billboard {
    /// Creates a billboard with Y-axis locked (stays upright)
    fn default() -> Self {
        Self { lock_y: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billboard_default_lock_y_true() {
        let billboard = Billboard::default();
        assert!(billboard.lock_y);
    }

    #[test]
    fn test_billboard_lock_y_explicit_true() {
        let billboard = Billboard { lock_y: true };
        assert!(billboard.lock_y);
    }

    #[test]
    fn test_billboard_lock_y_explicit_false() {
        let billboard = Billboard { lock_y: false };
        assert!(!billboard.lock_y);
    }
}
