// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sky-related ECS components for celestial body entities.
//!
//! Provides marker components used by [`crate::game::systems::sky_bodies`] to
//! identify, query, and toggle visibility of sun disc and star-field entities.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sky_system_implementation_plan.md` Phase 4.

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
}
