// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sky-related ECS components for celestial body entities.
//!
//! Provides marker components used by [`crate::game::systems::sky_bodies`] to
//! identify, query, toggle visibility of, and animate sun disc, star-field,
//! and cloud layer entities.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sky_system_implementation_plan.md` Phase 4 and Phase 5.

use bevy::prelude::*;

/// Marker component for a sun disc entity spawned by [`crate::game::systems::sky_bodies::SkyBodyPlugin`].
///
/// Each sun disc corresponds to one entry in `SkyConfig::sun_count`.
/// The sky body system queries all entities with this component to toggle
/// visibility based on the current `TimeOfDay`.
///
/// # Examples
///
/// ```
/// use antares::game::components::sky::SunMarker;
/// use bevy::prelude::Component;
///
/// fn assert_component<T: Component>() {}
/// assert_component::<SunMarker>();
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SunMarker;

/// Marker component for the star-field mesh entity spawned by [`crate::game::systems::sky_bodies::SkyBodyPlugin`].
///
/// There is at most one star-field entity per map. The sky body system queries
/// this component to toggle visibility when transitioning between day and night.
///
/// # Examples
///
/// ```
/// use antares::game::components::sky::StarFieldMarker;
/// use bevy::prelude::Component;
///
/// fn assert_component<T: Component>() {}
/// assert_component::<StarFieldMarker>();
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarFieldMarker;

/// Marker and animation-parameter component for the cloud layer entity
/// spawned by [`crate::game::systems::sky_bodies::SkyBodyPlugin`].
///
/// The [`crate::game::systems::sky_bodies::animate_clouds`] system reads
/// `cloud_speed` and `plane_half_width` from this component to animate the
/// cloud layer each frame via [`crate::game::systems::sky_bodies::wrap_cloud_position`].
///
/// # Examples
///
/// ```
/// use antares::game::components::sky::CloudLayerMarker;
///
/// let marker = CloudLayerMarker { cloud_speed: 1.0, plane_half_width: 100.0 };
/// assert_eq!(marker.cloud_speed, 1.0);
/// assert_eq!(marker.plane_half_width, 100.0);
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CloudLayerMarker {
    /// Horizontal scroll speed in world units per second.
    pub cloud_speed: f32,
    /// Half the total cloud plane width, used for X-position wrapping.
    pub plane_half_width: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sun_marker_is_copy() {
        let marker = SunMarker;
        let _copy = marker;
        // If Copy were not derived, the line above would move `marker`,
        // and using it here would be a compile error.
        let _original = marker;
    }

    #[test]
    fn test_star_field_marker_is_copy() {
        let marker = StarFieldMarker;
        let _copy = marker;
        let _original = marker;
    }

    #[test]
    fn test_sun_marker_debug() {
        let marker = SunMarker;
        assert_eq!(format!("{:?}", marker), "SunMarker");
    }

    #[test]
    fn test_star_field_marker_debug() {
        let marker = StarFieldMarker;
        assert_eq!(format!("{:?}", marker), "StarFieldMarker");
    }

    #[test]
    fn test_cloud_layer_marker_fields() {
        let marker = CloudLayerMarker {
            cloud_speed: 2.5,
            plane_half_width: 100.0,
        };
        assert!((marker.cloud_speed - 2.5).abs() < 1e-5);
        assert!((marker.plane_half_width - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_cloud_layer_marker_is_copy() {
        let marker = CloudLayerMarker {
            cloud_speed: 1.0,
            plane_half_width: 50.0,
        };
        let _copy = marker;
        let _original = marker;
    }
}
