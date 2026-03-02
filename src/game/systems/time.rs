// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Time-of-day ambient lighting and time-advance event systems.
//!
//! This module provides two independent but complementary systems:
//!
//! ## Ambient Lighting ([`TimeOfDayPlugin`])
//!
//! Reads the current [`TimeOfDay`] from the game state every frame and adjusts
//! the scene's [`AmbientLight`] intensity:
//!
//! - During bright periods (Dawn → Dusk) the ambient light is at full
//!   brightness (`AMBIENT_DAY_BRIGHTNESS`).
//! - During Evening the light is dimmed to `AMBIENT_EVENING_BRIGHTNESS`.
//! - During Night the light is reduced to `AMBIENT_NIGHT_BRIGHTNESS`.
//!
//! ## Centralised Time Mutation ([`TimeAdvanceEvent`] / [`apply_time_advance`])
//!
//! Systems that need to advance the in-game clock — combat rounds, map
//! transitions, rest ticks, or future food/condition hooks — should **send**
//! a [`TimeAdvanceEvent`] rather than mutating `GlobalState` directly.
//! The single [`apply_time_advance`] system drains the event queue each frame
//! and calls [`GameState::advance_time`] once per event, keeping time mutation
//! centralised and easy to audit.
//!
//! ### Usage
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::time::TimeAdvanceEvent;
//!
//! fn my_system(mut writer: EventWriter<TimeAdvanceEvent>) {
//!     // Advance 5 minutes (e.g. one combat round)
//!     writer.write(TimeAdvanceEvent { minutes: 5 });
//! }
//! ```
//!
//! # Architecture Reference
//!
//! See `docs/explanation/time_system_implementation_plan.md` Phases 2.3 and 4.3.

use crate::domain::types::TimeOfDay;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

// ===== TimeAdvanceEvent =====

/// Bevy event that requests the in-game clock to advance by `minutes`.
///
/// Any system that causes time to pass (combat rounds, map transitions, rest
/// ticks, etc.) should **send** this event rather than mutating [`GlobalState`]
/// directly.  The [`apply_time_advance`] system drains the queue each frame and
/// applies all pending advances in order via [`crate::application::GameState::advance_time`].
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::time::TimeAdvanceEvent;
///
/// fn advance_one_round(mut writer: MessageWriter<TimeAdvanceEvent>) {
///     writer.write(TimeAdvanceEvent { minutes: 5 });
/// }
/// ```
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeAdvanceEvent {
    /// Number of in-game minutes to advance the clock.
    pub minutes: u32,
}

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
        app.add_message::<TimeAdvanceEvent>();
        app.add_systems(Update, apply_time_advance.before(update_ambient_light));
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

// ===== TimeAdvanceEvent system =====

/// Bevy system that drains all [`TimeAdvanceEvent`]s queued this frame and
/// applies them to the in-game clock via [`crate::application::GameState::advance_time`].
///
/// This system is registered by [`TimeOfDayPlugin`] and runs **before**
/// [`update_ambient_light`] so that the ambient light always reflects the
/// updated time within the same frame.
///
/// # Design
///
/// Centralising time mutation here means that future hooks (food consumption,
/// active-condition ticking, ambient-light updates) only need to be added in
/// one place rather than scattered across every system that advances time.
///
/// Systems that need to advance time should send a [`TimeAdvanceEvent`]
/// instead of mutating [`GlobalState`] directly:
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::time::TimeAdvanceEvent;
///
/// fn my_system(mut writer: MessageWriter<TimeAdvanceEvent>) {
///     writer.write(TimeAdvanceEvent { minutes: 5 });
/// }
/// ```
pub fn apply_time_advance(
    mut global_state: ResMut<GlobalState>,
    mut events: MessageReader<TimeAdvanceEvent>,
) {
    for ev in events.read() {
        global_state.0.advance_time(ev.minutes, None);
    }
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
    use crate::application::GameState;
    use crate::domain::types::GameTime;
    use crate::game::resources::GlobalState;

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

    // ── TimeAdvanceEvent tests ────────────────────────────────────────────────

    /// A single TimeAdvanceEvent advances the game clock by the requested minutes.
    #[test]
    fn test_time_advance_event_advances_clock() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Register the message and the apply_time_advance system manually so we
        // don't need the full TimeOfDayPlugin (which requires AmbientLight).
        app.add_message::<TimeAdvanceEvent>();
        app.add_systems(Update, apply_time_advance);

        let mut gs = GameState::new();
        gs.time = GameTime::new(1, 6, 0); // start at 06:00
        app.insert_resource(GlobalState(gs));

        // Send a 60-minute advance
        app.world_mut()
            .get_resource_mut::<Messages<TimeAdvanceEvent>>()
            .expect("TimeAdvanceEvent message queue must exist")
            .write(TimeAdvanceEvent { minutes: 60 });
        app.update();

        let state = app.world().resource::<GlobalState>();
        assert_eq!(
            state.0.time.hour, 7,
            "clock must advance to 07:00 after 60-minute event"
        );
        assert_eq!(state.0.time.minute, 0);
    }

    /// Multiple TimeAdvanceEvents in the same frame are all applied.
    #[test]
    fn test_multiple_time_advance_events_same_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TimeAdvanceEvent>();
        app.add_systems(Update, apply_time_advance);

        let mut gs = GameState::new();
        gs.time = GameTime::new(1, 0, 0); // midnight
        app.insert_resource(GlobalState(gs));

        // Send three separate 5-minute messages (15 minutes total)
        {
            let mut msgs = app
                .world_mut()
                .get_resource_mut::<Messages<TimeAdvanceEvent>>()
                .expect("TimeAdvanceEvent message queue must exist");
            msgs.write(TimeAdvanceEvent { minutes: 5 });
            msgs.write(TimeAdvanceEvent { minutes: 5 });
            msgs.write(TimeAdvanceEvent { minutes: 5 });
        }
        app.update();

        let state = app.world().resource::<GlobalState>();
        assert_eq!(
            state.0.time.minute, 15,
            "three 5-minute events must total 15 minutes"
        );
    }

    /// No event sent → clock must not move.
    #[test]
    fn test_no_time_advance_event_leaves_clock_unchanged() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TimeAdvanceEvent>();
        app.add_systems(Update, apply_time_advance);

        let mut gs = GameState::new();
        gs.time = GameTime::new(1, 12, 30);
        app.insert_resource(GlobalState(gs));

        app.update(); // no event sent

        let state = app.world().resource::<GlobalState>();
        assert_eq!(state.0.time.hour, 12);
        assert_eq!(state.0.time.minute, 30);
    }

    /// A large advance that crosses midnight must roll over the day counter.
    #[test]
    fn test_time_advance_event_rolls_over_midnight() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TimeAdvanceEvent>();
        app.add_systems(Update, apply_time_advance);

        let mut gs = GameState::new();
        gs.time = GameTime::new(1, 23, 0); // 23:00 day 1
        app.insert_resource(GlobalState(gs));

        // 2 hours → 01:00 day 2
        app.world_mut()
            .get_resource_mut::<Messages<TimeAdvanceEvent>>()
            .expect("TimeAdvanceEvent message queue must exist")
            .write(TimeAdvanceEvent { minutes: 120 });
        app.update();

        let state = app.world().resource::<GlobalState>();
        assert_eq!(state.0.time.day, 2, "day must roll over to 2");
        assert_eq!(state.0.time.hour, 1, "hour must be 01:00");
    }

    /// TimeAdvanceEvent is Clone + Copy + Debug + PartialEq (trait bounds check).
    #[test]
    fn test_time_advance_event_trait_bounds() {
        let ev = TimeAdvanceEvent { minutes: 30 };
        let copy = ev;
        let clone = ev;
        assert_eq!(ev, copy);
        assert_eq!(ev, clone);
        assert!(!format!("{:?}", ev).is_empty());
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
