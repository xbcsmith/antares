// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sky background colour system.
//!
//! Updates Bevy's [`ClearColor`] each frame based on the current map's
//! [`SkyConfig`] and the current [`TimeOfDay`].
//!
//! # Design
//!
//! The core logic lives in the pure function [`sky_color_for_time`], which is
//! unit-testable without a full Bevy world.  The Bevy system
//! [`update_sky_background`] wraps it: reading [`GlobalState`] for the current
//! map and time, then writing the resulting colour to [`ClearColor`].
//!
//! ## Ordering
//!
//! Registered in [`SkyPlugin`] **after** `apply_time_advance` (so time is
//! always up-to-date) and **before** `update_ambient_light` (so both sky
//! colour and ambient brightness are driven by the same time value in the same
//! frame).
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sky_system_implementation_plan.md` Phase 2.

use crate::domain::types::TimeOfDay;
use crate::domain::world::SkyConfig;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

// ===== Constants =====

/// Sky colour for indoor maps (dungeons, buildings, caves).
///
/// Near-black warm grey that simulates a dimly lit cave ceiling.  Applied
/// whenever the current map's `is_outdoor` flag is `false`, regardless of the
/// time of day.
pub const INDOOR_SKY_COLOR: [f32; 4] = [0.05, 0.04, 0.03, 1.0];

/// Default outdoor sky colour during Morning and Afternoon.
pub const DEFAULT_OUTDOOR_DAY_SKY_COLOR: [f32; 4] = [0.53, 0.81, 0.98, 1.0];

/// Default outdoor sky colour during Evening and Night.
pub const DEFAULT_OUTDOOR_NIGHT_SKY_COLOR: [f32; 4] = [0.02, 0.02, 0.08, 1.0];

/// Default outdoor sky colour during Dawn and Dusk.
pub const DEFAULT_OUTDOOR_DUSK_DAWN_SKY_COLOR: [f32; 4] = [0.98, 0.60, 0.20, 1.0];

// ===== Plugin =====

/// Plugin that registers the sky background colour update system.
///
/// Add this plugin **after** [`TimeOfDayPlugin`](crate::game::systems::time::TimeOfDayPlugin)
/// so that time always advances before the sky colour is computed in the same
/// frame.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::sky::SkyPlugin;
///
/// App::new().add_plugins(SkyPlugin).run();
/// ```
pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_sky_background
                .after(crate::game::systems::time::apply_time_advance)
                .before(crate::game::systems::time::update_ambient_light),
        );
    }
}

// ===== Helpers =====

/// Linearly interpolates two RGBA colours by factor `t` (clamped to `0.0`–`1.0`).
fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

// ===== Pure sky colour function =====

/// Returns the sky RGBA colour for the given [`TimeOfDay`] and [`SkyConfig`].
///
/// This is a pure function with no Bevy dependencies, making it
/// straightforwardly unit-testable without a full Bevy world.
///
/// | `TimeOfDay` | Colour                                              |
/// |-------------|-----------------------------------------------------|
/// | `Night`     | `night_sky_color`                                   |
/// | `Evening`   | lerp(`night_sky_color`, `dusk_dawn_sky_color`, 0.3) |
/// | `Dawn`      | `dusk_dawn_sky_color`                               |
/// | `Morning`   | lerp(`dusk_dawn_sky_color`, `day_sky_color`, 0.7)   |
/// | `Afternoon` | `day_sky_color`                                     |
/// | `Dusk`      | `dusk_dawn_sky_color`                               |
///
/// # Examples
///
/// ```
/// use antares::domain::types::TimeOfDay;
/// use antares::domain::world::SkyConfig;
/// use antares::game::systems::sky::sky_color_for_time;
///
/// let config = SkyConfig::default();
/// let color = sky_color_for_time(&config, TimeOfDay::Afternoon);
/// assert_eq!(color, config.day_sky_color);
/// ```
pub fn sky_color_for_time(config: &SkyConfig, tod: TimeOfDay) -> [f32; 4] {
    match tod {
        TimeOfDay::Night => config.night_sky_color,
        TimeOfDay::Evening => lerp_color(config.night_sky_color, config.dusk_dawn_sky_color, 0.3),
        TimeOfDay::Dawn => config.dusk_dawn_sky_color,
        TimeOfDay::Morning => lerp_color(config.dusk_dawn_sky_color, config.day_sky_color, 0.7),
        TimeOfDay::Afternoon => config.day_sky_color,
        TimeOfDay::Dusk => config.dusk_dawn_sky_color,
    }
}

// ===== Bevy system =====

/// Bevy system that updates [`ClearColor`] each frame based on the current
/// map's sky configuration and the current time of day.
///
/// ### Selection logic
///
/// - `is_outdoor == false` → [`INDOOR_SKY_COLOR`]
/// - `is_outdoor == true`, `sky == None` → [`SkyConfig::default()`] fed into
///   [`sky_color_for_time`]
/// - `is_outdoor == true`, `sky == Some(cfg)` → per-map `cfg` fed into
///   [`sky_color_for_time`]
///
/// Falls back to [`INDOOR_SKY_COLOR`] when no map is currently active.
///
/// Ordered **after** `apply_time_advance` and **before** `update_ambient_light`
/// so that time is always up-to-date before the sky colour is computed.
pub fn update_sky_background(global_state: Res<GlobalState>, mut clear_color: ResMut<ClearColor>) {
    let game_state = &global_state.0;
    let tod = game_state.time_of_day();

    let rgba = if let Some(map) = game_state.world.get_current_map() {
        if !map.is_outdoor {
            INDOOR_SKY_COLOR
        } else {
            let default_sky = SkyConfig::default();
            let cfg = map.sky.as_ref().unwrap_or(&default_sky);
            sky_color_for_time(cfg, tod)
        }
    } else {
        // No active map — fall back to the dark indoor colour.
        INDOOR_SKY_COLOR
    };

    clear_color.0 = Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3]);
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience: a `SkyConfig` with distinct, well-separated values for
    /// every colour field so blend assertions are unambiguous.
    fn custom_config() -> SkyConfig {
        SkyConfig {
            day_sky_color: [0.53, 0.81, 0.98, 1.0],
            dusk_dawn_sky_color: [0.98, 0.60, 0.20, 1.0],
            night_sky_color: [0.02, 0.02, 0.08, 1.0],
            ..SkyConfig::default()
        }
    }

    #[test]
    fn test_sky_color_for_time_night() {
        let config = custom_config();
        let color = sky_color_for_time(&config, TimeOfDay::Night);
        assert_eq!(
            color, config.night_sky_color,
            "Night should return night_sky_color exactly"
        );
    }

    #[test]
    fn test_sky_color_for_time_afternoon() {
        let config = custom_config();
        let color = sky_color_for_time(&config, TimeOfDay::Afternoon);
        assert_eq!(
            color, config.day_sky_color,
            "Afternoon should return day_sky_color exactly"
        );
    }

    #[test]
    fn test_sky_color_for_time_dusk() {
        let config = custom_config();
        let color = sky_color_for_time(&config, TimeOfDay::Dusk);
        assert_eq!(
            color, config.dusk_dawn_sky_color,
            "Dusk should return dusk_dawn_sky_color exactly"
        );
    }

    #[test]
    fn test_sky_color_for_time_evening_is_blend() {
        let config = custom_config();
        let color = sky_color_for_time(&config, TimeOfDay::Evening);

        // Evening is lerp(night → dusk_dawn, t=0.3).
        // Every RGB component must be between the night and dusk/dawn values.
        for (i, &color_val) in color.iter().enumerate().take(3) {
            let night = config.night_sky_color[i];
            let dusk = config.dusk_dawn_sky_color[i];
            let lo = night.min(dusk);
            let hi = night.max(dusk);
            assert!(
                color_val >= lo && color_val <= hi,
                "Evening component {i} ({color_val}) is outside [{lo}, {hi}]",
            );
        }
        // Must not equal either pure endpoint.
        assert_ne!(
            color, config.night_sky_color,
            "Evening must not equal Night"
        );
        assert_ne!(
            color, config.dusk_dawn_sky_color,
            "Evening must not equal Dusk/Dawn"
        );
    }

    #[test]
    fn test_sky_color_for_time_morning_is_blend() {
        let config = custom_config();
        let color = sky_color_for_time(&config, TimeOfDay::Morning);

        // Morning is lerp(dusk_dawn → day, t=0.7).
        for (i, &color_val) in color.iter().enumerate().take(3) {
            let dusk = config.dusk_dawn_sky_color[i];
            let day = config.day_sky_color[i];
            let lo = dusk.min(day);
            let hi = dusk.max(day);
            assert!(
                color_val >= lo && color_val <= hi,
                "Morning component {i} ({color_val}) is outside [{lo}, {hi}]",
            );
        }
        assert_ne!(
            color, config.dusk_dawn_sky_color,
            "Morning must not equal Dusk/Dawn"
        );
        assert_ne!(color, config.day_sky_color, "Morning must not equal Day");
    }

    #[test]
    fn test_sky_color_all_periods_produce_valid_rgba() {
        let config = SkyConfig::default();
        let periods = [
            TimeOfDay::Night,
            TimeOfDay::Evening,
            TimeOfDay::Dawn,
            TimeOfDay::Morning,
            TimeOfDay::Afternoon,
            TimeOfDay::Dusk,
        ];
        for tod in periods {
            let color = sky_color_for_time(&config, tod);
            for (i, &component) in color.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(&component),
                    "{:?} sky color component {i} ({component}) is outside [0.0, 1.0]",
                    tod,
                );
            }
        }
    }
}
