// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Rest System Module
//!
//! Provides the Bevy event types and plugin for the party rest sequence.
//!
//! # Overview
//!
//! The rest system coordinates party healing during exploration by wiring
//! together input, game state, and the domain-level `rest_party_hour()` helper.
//!
//! ## Phase 2 (current)
//!
//! - [`InitiateRestEvent`] — fired by the input handler when the player presses
//!   the rest key (`R` by default) while in [`Exploration`](crate::application::GameMode::Exploration)
//!   mode.
//! - [`RestPlugin`] — registers the event type with Bevy.
//!
//! ## Phase 3 (upcoming)
//!
//! Phase 3 will add:
//! - [`RestCompleteEvent`] — emitted when a rest sequence finishes or is
//!   interrupted by a random encounter.
//! - `process_rest` system — drives the per-hour rest loop, calls
//!   `rest_party_hour()`, advances game time, and rolls for encounters.
//! - `handle_rest_complete` system — triggers combat when an encounter
//!   interrupts rest.
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::rest::RestPlugin;
//!
//! # fn setup() {
//! let mut app = App::new();
//! app.add_plugins(RestPlugin);
//! # }
//! ```

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Sent by the input handler to begin a rest sequence.
///
/// When the player presses the rest key (default `R`) while in
/// [`Exploration`](crate::application::GameMode::Exploration) mode, the input
/// system writes this event.  The rest orchestration system (Phase 3) reads it
/// and transitions `game_state.mode` to `GameMode::Resting(…)`.
///
/// # Fields
///
/// * `hours` — how many in-game hours to rest.  Defaults to
///   [`REST_DURATION_HOURS`](crate::domain::resources::REST_DURATION_HOURS)
///   (12 hours) for a full rest.
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::InitiateRestEvent;
///
/// let event = InitiateRestEvent { hours: 12 };
/// assert_eq!(event.hours, 12);
/// ```
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct InitiateRestEvent {
    /// Number of in-game hours to rest.
    pub hours: u32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Bevy plugin that registers rest-related event types.
///
/// Add this plugin to your Bevy [`App`] alongside the other game plugins.
/// Phase 3 will extend this plugin with the `process_rest` and
/// `handle_rest_complete` systems.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::rest::RestPlugin;
///
/// # fn setup() {
/// let mut app = App::new();
/// app.add_plugins(RestPlugin);
/// # }
/// ```
pub struct RestPlugin;

impl Plugin for RestPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InitiateRestEvent>();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// [`InitiateRestEvent`] stores the requested hour count correctly.
    #[test]
    fn test_initiate_rest_event_stores_hours() {
        let event = InitiateRestEvent { hours: 12 };
        assert_eq!(event.hours, 12);
    }

    /// [`InitiateRestEvent`] is `Clone` and `PartialEq`.
    #[test]
    fn test_initiate_rest_event_clone_and_eq() {
        let a = InitiateRestEvent { hours: 8 };
        let b = a.clone();
        assert_eq!(a, b);
    }

    /// [`InitiateRestEvent`] with different hour counts are not equal.
    #[test]
    fn test_initiate_rest_event_inequality() {
        let a = InitiateRestEvent { hours: 6 };
        let b = InitiateRestEvent { hours: 12 };
        assert_ne!(a, b);
    }

    /// [`RestPlugin`] registers [`InitiateRestEvent`] so that Bevy can write and
    /// read it without panicking.
    #[test]
    fn test_rest_plugin_registers_initiate_rest_event() {
        let mut app = App::new();
        app.add_plugins(RestPlugin);

        // Writing a message after plugin registration must not panic.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent { hours: 12 });
        app.update();
    }
}
