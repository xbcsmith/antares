// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Grass density quality settings for performance tuning
//!
//! This module provides configurable grass blade density levels to support
//! a range of hardware capabilities, from older integrated graphics to
//! modern gaming systems.
//!
//! # Architecture
//!
//! The `GrassQualitySettings` resource is initialized with a default density
//! level (Medium) and can be modified at runtime to adapt to frame rate or
//! user preferences.
//!
//! # Examples
//!
//! ```text
//! use antares::game::resources::grass_quality_settings::{GrassQualitySettings, GrassDensity};
//!
//! let settings = GrassQualitySettings::default();
//! assert_eq!(settings.density, GrassDensity::Medium);
//!
//! let (min, max) = settings.density.blade_count_range();
//! println!("Grass blades per tile: {}-{}", min, max);
//! ```

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ==================== Resource ====================

/// Resource controlling grass blade density for performance tuning
///
/// Grass density is configurable to support a range of hardware capabilities.
/// The density setting determines how many grass blade billboards are spawned
/// per grass terrain tile, directly affecting visual quality and performance.
///
/// # Fields
///
/// * `density` - Current grass density level (Low, Medium, or High)
///
/// # Examples
///
/// ```text
/// use antares::game::resources::grass_quality_settings::{GrassQualitySettings, GrassDensity};
///
/// let mut settings = GrassQualitySettings::default();
/// assert_eq!(settings.density, GrassDensity::Medium);
///
/// // Change to high density for better visuals
/// settings.density = GrassDensity::High;
/// let (min, max) = settings.density.blade_count_range();
/// assert_eq!((min, max), (12, 20));
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrassQualitySettings {
    /// Current grass blade density level
    pub density: GrassDensity,
}

impl Default for GrassQualitySettings {
    /// Creates default grass quality settings with Medium density
    ///
    /// Medium density (6-10 blades per tile) provides a balanced visual
    /// quality suitable for standard desktop hardware.
    fn default() -> Self {
        Self {
            density: GrassDensity::Medium,
        }
    }
}

// ==================== Density Enum ====================

/// Grass blade density levels
///
/// Controls how many grass blade billboards spawn per terrain tile.
/// Each level targets a specific hardware capability tier:
///
/// | Level  | Blades/Tile | Target Hardware                     |
/// |--------|-------------|-------------------------------------|
/// | Low    | 2-4         | Older hardware, integrated graphics |
/// | Medium | 6-10        | Standard desktop                    |
/// | High   | 12-20       | Modern gaming hardware              |
///
/// # Examples
///
/// ```text
/// use antares::game::resources::grass_quality_settings::GrassDensity;
///
/// let low = GrassDensity::Low;
/// let (min, max) = low.blade_count_range();
/// assert_eq!((min, max), (2, 4));
/// assert_eq!(low.name(), "Low (2-4 blades)");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassDensity {
    /// 2-4 blades per tile (older hardware, integrated graphics)
    ///
    /// Minimum grass coverage for systems with limited GPU memory
    /// or older integrated graphics. Visual quality is reduced but
    /// frame rate remains stable on older hardware.
    Low,

    /// 6-10 blades per tile (standard desktop)
    ///
    /// Balanced default providing adequate grass coverage without
    /// significant performance impact on typical modern systems.
    /// Recommended for most use cases.
    Medium,

    /// 12-20 blades per tile (modern gaming hardware)
    ///
    /// Dense grass coverage for maximum visual fidelity on modern
    /// gaming GPUs. Requires sufficient GPU memory and bandwidth.
    High,
}

impl GrassDensity {
    /// Returns the range of grass blades to spawn per tile
    ///
    /// # Returns
    ///
    /// A tuple `(min, max)` representing the range of grass blades
    /// that should be spawned for this density setting.
    ///
    /// # Examples
    ///
    /// ```text
    /// use antares::game::resources::grass_quality_settings::GrassDensity;
    ///
    /// assert_eq!(GrassDensity::Low.blade_count_range(), (2, 4));
    /// assert_eq!(GrassDensity::Medium.blade_count_range(), (6, 10));
    /// assert_eq!(GrassDensity::High.blade_count_range(), (12, 20));
    /// ```
    pub fn blade_count_range(&self) -> (u32, u32) {
        match self {
            Self::Low => (2, 4),
            Self::Medium => (6, 10),
            Self::High => (12, 20),
        }
    }

    /// Returns display name for UI and debugging
    ///
    /// # Returns
    ///
    /// A human-readable string describing this density level and
    /// its blade count range.
    ///
    /// # Examples
    ///
    /// ```text
    /// use antares::game::resources::grass_quality_settings::GrassDensity;
    ///
    /// assert_eq!(GrassDensity::Low.name(), "Low (2-4 blades)");
    /// assert_eq!(GrassDensity::Medium.name(), "Medium (6-10 blades)");
    /// assert_eq!(GrassDensity::High.name(), "High (12-20 blades)");
    /// ```
    pub fn name(&self) -> &str {
        match self {
            Self::Low => "Low (2-4 blades)",
            Self::Medium => "Medium (6-10 blades)",
            Self::High => "High (12-20 blades)",
        }
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grass_quality_settings_default_is_medium() {
        let settings = GrassQualitySettings::default();
        assert_eq!(settings.density, GrassDensity::Medium);
    }

    #[test]
    fn test_grass_density_low_blade_count_range() {
        let (min, max) = GrassDensity::Low.blade_count_range();
        assert_eq!(min, 2);
        assert_eq!(max, 4);
    }

    #[test]
    fn test_grass_density_medium_blade_count_range() {
        let (min, max) = GrassDensity::Medium.blade_count_range();
        assert_eq!(min, 6);
        assert_eq!(max, 10);
    }

    #[test]
    fn test_grass_density_high_blade_count_range() {
        let (min, max) = GrassDensity::High.blade_count_range();
        assert_eq!(min, 12);
        assert_eq!(max, 20);
    }

    #[test]
    fn test_grass_density_low_name() {
        assert_eq!(GrassDensity::Low.name(), "Low (2-4 blades)");
    }

    #[test]
    fn test_grass_density_medium_name() {
        assert_eq!(GrassDensity::Medium.name(), "Medium (6-10 blades)");
    }

    #[test]
    fn test_grass_density_high_name() {
        assert_eq!(GrassDensity::High.name(), "High (12-20 blades)");
    }

    #[test]
    fn test_grass_quality_settings_clone() {
        let settings = GrassQualitySettings::default();
        let cloned = settings.clone();
        assert_eq!(settings, cloned);
    }

    #[test]
    fn test_grass_quality_settings_custom_density() {
        let settings = GrassQualitySettings {
            density: GrassDensity::High,
        };
        assert_eq!(settings.density, GrassDensity::High);
    }
}
