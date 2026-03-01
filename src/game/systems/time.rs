// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Time-of-day ambient lighting system
//!
//! This module provides a Bevy system that reads the current [`TimeOfDay`] from
//! the game state and adjusts the scene's ambient light intensity accordingly.
//!
//! # Design
//!
//! - During bright periods (Dawn → Dusk) the ambient light is at full
//!   brightness (`AMBIENT_DAY_BRIGHTNESS`).
//! - During Evening the light is dimmed to `AMBIENT_EVENING_BRIGHTNESS`.
//! - During Night the light is reduced to `AMBIENT_NIGHT_BRIGHTNESS`.
//!
//! The system runs every frame in the `Update` schedule so that any time
//! advancement (step, rest, map transition) is reflected immediately.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/time_system_implementation_plan.md` Phase 2.3.

use crate::domain::types::TimeOfDay;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

// ===== Constants =====

/// Ambient brightness during Night (22:00–04:59).
/// At this level the world is very dark; a light source is required indoors.
pub const AMBIENT_NIGHT_BRIGHTNESS: f32 = 0.25;

/// Ambient brightness during Evening (19:00–21:59).
/// Noticeably darker than daytime but not pitch-black.
pub const AMBIENT_EVENING_BRIGHTNESS: f32 = 0.50;

/// Ambient brightness during Dawn (05:00–07:59).
/// Slightly dimmer than full day — pale early-morning light.
pub const AMBIENT_DAWN_BRIGHTNESS: f32 = 0.70;

/// Ambient brightness during Dusk (16:00–18:59).
/// Golden hour — slightly reduced from peak noon.
pub const AMBIENT_DUSK_BRIGHTNESS: f32 = 0.70;

/// Ambient brightness during Morning and Afternoon (08:00–15:59).
/// Full daytime illumination.
pub const AMBIENT_DAY_BRIGHTNESS: f32 = 1.00;

// ===== Plugin =====

/// Plugin that registers the ambient-light update system.
///
/// Add this plugin to the Bevy [`App`] alongside other game plugins so that
/// the ambient light is kept in sync with the in-game clock every frame.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::time::TimeOfDayPlugin;
///
/// App::new()
///     .add_plugins(TimeOfDayPlugin)
///     .run();
/// ```
pub struct TimeOfDayPlugin;

impl Plugin for TimeOfDayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_ambient_light);
    }
}

// ===== Systems =====

/// Updates the scene's [`AmbientLight`] intensity based on the current
/// [`TimeOfDay`] read from [`GlobalState`].
///
/// Called every frame so that any in-game time advancement is reflected
/// without delay.
///
/// | Period    | Brightness                      |
/// |-----------|---------------------------------|
/// | Night     | [`AMBIENT_NIGHT_BRIGHTNESS`]    |
/// | Evening   | [`AMBIENT_EVENING_BRIGHTNESS`]  |
/// | Dawn      | [`AMBIENT_DAWN_BRIGHTNESS`]     |
/// | Dusk      | [`AMBIENT_DUSK_BRIGHTNESS`]     |
/// | Morning   | [`AMBIENT_DAY_BRIGHTNESS`]      |
/// | Afternoon | [`AMBIENT_DAY_BRIGHTNESS`]      |
pub fn update_ambient_light(
    global_state: Res<GlobalState>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    let brightness = time_of_day_brightness(global_state.0.time_of_day());
    ambient_light.brightness = brightness;
}

/// Pure function mapping a [`TimeOfDay`] period to an ambient brightness value.
///
/// Extracted from the system so it can be called in unit tests without
/// requiring a full Bevy world.
///
/// # Examples
///
/// ```
/// use antares::domain::types::TimeOfDay;
/// use antares::game::systems::time::{
///     time_of_day_brightness, AMBIENT_DAY_BRIGHTNESS, AMBIENT_NIGHT_BRIGHTNESS,
///     AMBIENT_EVENING_BRIGHTNESS, AMBIENT_DAWN_BRIGHTNESS, AMBIENT_DUSK_BRIGHTNESS,
/// };
///
/// assert_eq!(time_of_day_brightness(TimeOfDay::Night),     AMBIENT_NIGHT_BRIGHTNESS);
/// assert_eq!(time_of_day_brightness(TimeOfDay::Evening),   AMBIENT_EVENING_BRIGHTNESS);
/// assert_eq!(time_of_day_brightness(TimeOfDay::Dawn),      AMBIENT_DAWN_BRIGHTNESS);
/// assert_eq!(time_of_day_brightness(TimeOfDay::Dusk),      AMBIENT_DUSK_BRIGHTNESS);
/// assert_eq!(time_of_day_brightness(TimeOfDay::Morning),   AMBIENT_DAY_BRIGHTNESS);
/// assert_eq!(time_of_day_brightness(TimeOfDay::Afternoon), AMBIENT_DAY_BRIGHTNESS);
/// ```
pub fn time_of_day_brightness(period: TimeOfDay) -> f32 {
    match period {
        TimeOfDay::Night => AMBIENT_NIGHT_BRIGHTNESS,
        TimeOfDay::Evening => AMBIENT_EVENING_BRIGHTNESS,
        TimeOfDay::Dawn => AMBIENT_DAWN_BRIGHTNESS,
        TimeOfDay::Dusk => AMBIENT_DUSK_BRIGHTNESS,
        TimeOfDay::Morning | TimeOfDay::Afternoon => AMBIENT_DAY_BRIGHTNESS,
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::GameTime;

    #[test]
    fn test_brightness_night() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Night),
            AMBIENT_NIGHT_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_evening() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Evening),
            AMBIENT_EVENING_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_dawn() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Dawn),
            AMBIENT_DAWN_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_dusk() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Dusk),
            AMBIENT_DUSK_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_morning() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Morning),
            AMBIENT_DAY_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_afternoon() {
        assert_eq!(
            time_of_day_brightness(TimeOfDay::Afternoon),
            AMBIENT_DAY_BRIGHTNESS
        );
    }

    #[test]
    fn test_brightness_is_darker_at_night_than_day() {
        let night = time_of_day_brightness(TimeOfDay::Night);
        let day = time_of_day_brightness(TimeOfDay::Afternoon);
        assert!(
            night < day,
            "Night brightness ({}) should be less than day brightness ({})",
            night,
            day
        );
    }

    #[test]
    fn test_brightness_ordering() {
        // Verify the intended brightness ordering: Night < Evening < Dawn/Dusk < Day
        let night = time_of_day_brightness(TimeOfDay::Night);
        let evening = time_of_day_brightness(TimeOfDay::Evening);
        let dawn = time_of_day_brightness(TimeOfDay::Dawn);
        let dusk = time_of_day_brightness(TimeOfDay::Dusk);
        let morning = time_of_day_brightness(TimeOfDay::Morning);
        let afternoon = time_of_day_brightness(TimeOfDay::Afternoon);

        assert!(night < evening, "Night should be darker than Evening");
        assert!(evening < dawn, "Evening should be darker than Dawn");
        assert_eq!(dawn, dusk, "Dawn and Dusk should have equal brightness");
        assert!(dawn < morning, "Dawn should be dimmer than Morning");
        assert_eq!(
            morning, afternoon,
            "Morning and Afternoon should have equal brightness"
        );
    }

    #[test]
    fn test_all_hours_produce_valid_brightness() {
        for hour in 0u8..24 {
            let time = GameTime::new(1, hour, 0);
            let brightness = time_of_day_brightness(time.time_of_day());
            assert!(
                (0.0..=1.0).contains(&brightness),
                "hour {} produced out-of-range brightness: {}",
                hour,
                brightness
            );
        }
    }

    #[test]
    fn test_dark_periods_below_threshold() {
        // Evening and Night must be strictly below 1.0 (noticeably dark)
        let evening = time_of_day_brightness(TimeOfDay::Evening);
        let night = time_of_day_brightness(TimeOfDay::Night);
        assert!(evening < 1.0, "Evening brightness must be below 1.0");
        assert!(night < 1.0, "Night brightness must be below 1.0");
    }

    #[test]
    fn test_time_of_day_is_dark_matches_brightness_reduction() {
        // Any period flagged as dark by TimeOfDay::is_dark() must have
        // brightness strictly below AMBIENT_DAY_BRIGHTNESS.
        for period in [
            TimeOfDay::Dawn,
            TimeOfDay::Morning,
            TimeOfDay::Afternoon,
            TimeOfDay::Dusk,
            TimeOfDay::Evening,
            TimeOfDay::Night,
        ] {
            let brightness = time_of_day_brightness(period);
            if period.is_dark() {
                assert!(
                    brightness < AMBIENT_DAY_BRIGHTNESS,
                    "{:?} is flagged as dark but brightness {} is not below day brightness {}",
                    period,
                    brightness,
                    AMBIENT_DAY_BRIGHTNESS
                );
            }
        }
    }
}
