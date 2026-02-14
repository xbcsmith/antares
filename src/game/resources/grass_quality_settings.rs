// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Grass performance settings and content-density conversion
//!
//! This module separates **content density** (domain-level grass density on tiles)
//! from **performance level** (render-time quality settings). The conversion is:
//!
//! `content_density_range × performance_multiplier = blade_count_range`

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::world::GrassDensity;

// ==================== Resource ====================

/// Resource controlling grass performance level for rendering
///
/// This setting scales domain-level grass density into an actual blade count
/// range used at render time.
///
/// # Fields
///
/// * `performance_level` - Current performance setting (Low/Medium/High)
///
/// # Examples
///
/// ```rust
/// use antares::game::resources::grass_quality_settings::{
///     GrassPerformanceLevel, GrassQualitySettings,
/// };
///
/// let settings = GrassQualitySettings::default();
/// assert_eq!(settings.performance_level, GrassPerformanceLevel::Medium);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrassQualitySettings {
    /// Current grass performance level
    pub performance_level: GrassPerformanceLevel,
}

impl Default for GrassQualitySettings {
    /// Creates default grass quality settings with Medium performance
    ///
    /// Medium performance maintains a 1.0x multiplier on content density.
    fn default() -> Self {
        Self {
            performance_level: GrassPerformanceLevel::Medium,
        }
    }
}

impl GrassQualitySettings {
    /// Computes the blade-count range for the given content density
    ///
    /// Applies the current performance multiplier to the domain-level density.
    ///
    /// # Arguments
    ///
    /// * `content_density` - Domain grass density from tile metadata
    ///
    /// # Returns
    ///
    /// Tuple `(min, max)` for blades per tile after scaling.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::domain::world::GrassDensity;
    /// use antares::game::resources::grass_quality_settings::{
    ///     GrassPerformanceLevel, GrassQualitySettings,
    /// };
    ///
    /// let settings = GrassQualitySettings {
    ///     performance_level: GrassPerformanceLevel::Low,
    /// };
    ///
    /// let (min, max) = settings.blade_count_range_for_content(GrassDensity::High);
    /// assert!(min <= max);
    /// assert!(max > 0);
    /// ```
    pub fn blade_count_range_for_content(&self, content_density: GrassDensity) -> (u32, u32) {
        self.performance_level
            .apply_to_content_density(content_density)
    }
}

// ==================== Performance Level Enum ====================

/// Grass performance levels for rendering
///
/// Performance settings scale the content density configured in map data.
/// These are **not** the same as the domain `GrassDensity` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassPerformanceLevel {
    /// Low performance setting: 0.25x content density
    Low,
    /// Medium performance setting: 1.0x content density
    Medium,
    /// High performance setting: 1.5x content density
    High,
}

impl GrassPerformanceLevel {
    /// Returns the multiplier applied to content density
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
    ///
    /// assert_eq!(GrassPerformanceLevel::Low.multiplier(), 0.25);
    /// assert_eq!(GrassPerformanceLevel::Medium.multiplier(), 1.0);
    /// assert_eq!(GrassPerformanceLevel::High.multiplier(), 1.5);
    /// ```
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 1.0,
            Self::High => 1.5,
        }
    }

    /// Applies performance scaling to a domain-level content density
    ///
    /// # Arguments
    ///
    /// * `content_density` - Domain grass density from tile metadata
    ///
    /// # Returns
    ///
    /// Tuple `(min, max)` of scaled blade counts. If content density is `None`,
    /// returns `(0, 0)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::domain::world::GrassDensity;
    /// use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
    ///
    /// let (min, max) = GrassPerformanceLevel::Low.apply_to_content_density(GrassDensity::Medium);
    /// assert!(min <= max);
    /// assert!(max > 0);
    /// ```
    pub fn apply_to_content_density(self, content_density: GrassDensity) -> (u32, u32) {
        let (base_min, base_max) = base_density_range(content_density);
        if base_min == 0 && base_max == 0 {
            return (0, 0);
        }

        let multiplier = self.multiplier();
        let scaled_min = scale_blade_count(base_min, multiplier);
        let scaled_max = scale_blade_count(base_max, multiplier);

        let min = scaled_min.min(scaled_max);
        let max = scaled_min.max(scaled_max);

        (min, max)
    }

    /// Returns a display name for UI and debugging
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::resources::grass_quality_settings::GrassPerformanceLevel;
    ///
    /// assert_eq!(GrassPerformanceLevel::Low.name(), "Low (0.25x)");
    /// assert_eq!(GrassPerformanceLevel::Medium.name(), "Medium (1.0x)");
    /// assert_eq!(GrassPerformanceLevel::High.name(), "High (1.5x)");
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "Low (0.25x)",
            Self::Medium => "Medium (1.0x)",
            Self::High => "High (1.5x)",
        }
    }
}

// ==================== Helpers ====================

fn base_density_range(density: GrassDensity) -> (u32, u32) {
    match density {
        GrassDensity::None => (0, 0),
        GrassDensity::Low => (10, 20),
        GrassDensity::Medium => (40, 60),
        GrassDensity::High => (80, 120),
        GrassDensity::VeryHigh => (150, 200),
    }
}

fn scale_blade_count(value: u32, multiplier: f32) -> u32 {
    let scaled = (value as f32 * multiplier).round();
    if value > 0 && scaled < 1.0 {
        1
    } else {
        scaled as u32
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_performance_level_is_medium() {
        let settings = GrassQualitySettings::default();
        assert_eq!(settings.performance_level, GrassPerformanceLevel::Medium);
    }

    #[test]
    fn test_performance_level_multiplier_values() {
        assert_eq!(GrassPerformanceLevel::Low.multiplier(), 0.25);
        assert_eq!(GrassPerformanceLevel::Medium.multiplier(), 1.0);
        assert_eq!(GrassPerformanceLevel::High.multiplier(), 1.5);
    }

    #[test]
    fn test_performance_level_names() {
        assert_eq!(GrassPerformanceLevel::Low.name(), "Low (0.25x)");
        assert_eq!(GrassPerformanceLevel::Medium.name(), "Medium (1.0x)");
        assert_eq!(GrassPerformanceLevel::High.name(), "High (1.5x)");
    }

    #[test]
    fn test_apply_to_content_density_none_returns_zero() {
        let (min, max) = GrassPerformanceLevel::Medium.apply_to_content_density(GrassDensity::None);
        assert_eq!((min, max), (0, 0));
    }

    #[test]
    fn test_apply_to_content_density_scales_medium_density_low_performance() {
        let (min, max) = GrassPerformanceLevel::Low.apply_to_content_density(GrassDensity::Medium);
        assert!(min <= max);
        assert!(min >= 1);
        assert!(max >= min);
    }

    #[test]
    fn test_apply_to_content_density_scales_high_density_high_performance() {
        let (min, max) = GrassPerformanceLevel::High.apply_to_content_density(GrassDensity::High);
        assert!(min <= max);
        assert!(min > 0);
        assert!(max > min);
    }

    #[test]
    fn test_blade_count_range_for_content_uses_performance_level() {
        let settings = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::Low,
        };
        let (min, max) = settings.blade_count_range_for_content(GrassDensity::Low);
        assert!(min <= max);
        assert!(max > 0);
    }

    #[test]
    fn test_scale_blade_count_rounds_and_clamps() {
        assert_eq!(scale_blade_count(10, 0.25), 3);
        assert_eq!(scale_blade_count(1, 0.1), 1);
    }

    #[test]
    fn test_base_density_range_values() {
        assert_eq!(base_density_range(GrassDensity::Low), (10, 20));
        assert_eq!(base_density_range(GrassDensity::Medium), (40, 60));
        assert_eq!(base_density_range(GrassDensity::High), (80, 120));
        assert_eq!(base_density_range(GrassDensity::VeryHigh), (150, 200));
        assert_eq!(base_density_range(GrassDensity::None), (0, 0));
    }
}
