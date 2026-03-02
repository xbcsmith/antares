// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Rest System Module
//!
//! Provides the Bevy event types, orchestration systems, and plugin for the
//! party rest sequence.
//!
//! # Overview
//!
//! When the player presses the rest key (`R` by default) while in
//! [`Exploration`](crate::application::GameMode::Exploration) mode, the input
//! handler fires an [`InitiateRestEvent`].  The [`process_rest`] system reads
//! that event and transitions the game to
//! [`GameMode::Resting`](crate::application::GameMode::Resting).
//!
//! Each Bevy frame while in `Resting` mode, `process_rest`:
//!
//! 1. Calls [`rest_party_hour`](crate::domain::resources::rest_party_hour) to
//!    heal the party by one hour's worth of HP/SP and consume food.
//! 2. Advances game time by 60 minutes via
//!    [`GameState::advance_time`](crate::application::GameState::advance_time).
//! 3. Rolls for a random encounter via
//!    [`random_encounter`](crate::domain::world::random_encounter).
//! 4. Either interrupts the rest (encounter found) or continues until all
//!    requested hours are completed.
//!
//! When the sequence ends — by completion or encounter interruption —
//! [`RestCompleteEvent`] is written.  The [`handle_rest_complete`] system
//! reads that event:
//! - On completion: leaves the game in `Exploration` mode (already set by
//!   `process_rest`).
//! - On encounter interruption: calls
//!   [`start_encounter`](crate::game::systems::combat::start_encounter) to
//!   initialise combat, exactly mirroring the movement-encounter path.
//!
//! # Design Notes
//!
//! - **One hour per Bevy frame** — fast-forward model.  A 12-hour rest
//!   completes in 12 frames (~0.2 s at 60 fps).
//! - **No `GameContent` required during pure rest** — `advance_time` is called
//!   with `None` for the stock-template parameter so the rest system does not
//!   depend on campaign content being loaded.  If content is available, pass it
//!   through for merchant restocking.
//! - **Encounter initialisation reuses `start_encounter`** — no duplication of
//!   combat initialisation logic.
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

use crate::application::{GameMode, GameState};
use crate::domain::resources::rest_party_hour;
use crate::domain::world::random_encounter;
use crate::game::resources::GlobalState;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Sent by the input handler to begin a rest sequence.
///
/// When the player presses the rest key (default `R`) while in
/// [`Exploration`](crate::application::GameMode::Exploration) mode, the input
/// system writes this event.  [`process_rest`] reads it and transitions
/// `game_state.mode` to `GameMode::Resting(…)`.
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

/// Sent when a rest sequence ends — either completed or interrupted.
///
/// The [`handle_rest_complete`] system reads this event to initiate combat
/// when `interrupted_by_encounter` is `true`.  UI systems (Phase 4) will also
/// read it to display completion / interruption messages.
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestCompleteEvent;
///
/// // Completed rest
/// let done = RestCompleteEvent {
///     hours_completed: 12,
///     interrupted_by_encounter: false,
///     encounter_group: None,
/// };
/// assert!(!done.interrupted_by_encounter);
///
/// // Interrupted rest
/// let interrupted = RestCompleteEvent {
///     hours_completed: 3,
///     interrupted_by_encounter: true,
///     encounter_group: Some(vec![1, 2]),
/// };
/// assert!(interrupted.interrupted_by_encounter);
/// ```
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct RestCompleteEvent {
    /// Number of rest hours that were fully processed before the sequence ended.
    pub hours_completed: u32,
    /// `true` when a random encounter fired and cut the rest short.
    pub interrupted_by_encounter: bool,
    /// The monster group that interrupted the rest, if any.
    ///
    /// Each `u8` is a monster ID from the encounter table.  `None` when the
    /// rest completed normally.
    pub encounter_group: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Bevy plugin that registers rest-related event types and orchestration
/// systems.
///
/// Add this plugin to your Bevy [`App`] alongside the other game plugins.
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
        app.add_message::<InitiateRestEvent>()
            .add_message::<RestCompleteEvent>()
            .add_systems(Update, (process_rest, handle_rest_complete).chain());
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Orchestrates the per-hour rest loop.
///
/// Each Bevy frame this system:
///
/// 1. If [`InitiateRestEvent`] is pending **and** the game is in
///    [`Exploration`](crate::application::GameMode::Exploration) mode,
///    calls [`GameState::enter_rest`] to begin the sequence.
/// 2. If the game is in [`GameMode::Resting`](crate::application::GameMode::Resting):
///    - Calls [`rest_party_hour`] to heal one hour of HP/SP and consume food.
///    - Advances game time by 60 minutes.
///    - Rolls for a random encounter.
///    - On encounter: sets `interrupted`, writes [`RestCompleteEvent`], and
///      returns the game to `Exploration` mode (combat is started by
///      [`handle_rest_complete`]).
///    - On normal completion: writes [`RestCompleteEvent`] and returns to
///      `Exploration`.
pub fn process_rest(
    global_state: Option<ResMut<GlobalState>>,
    mut initiate_reader: MessageReader<InitiateRestEvent>,
    mut complete_writer: MessageWriter<RestCompleteEvent>,
) {
    let mut global_state = match global_state {
        Some(gs) => gs,
        None => {
            // Drain events so they don't accumulate when the resource is absent
            // (e.g. in plugin-registration unit tests).
            for _ in initiate_reader.read() {}
            return;
        }
    };
    let game_state: &mut GameState = &mut global_state.0;

    // ── Step 1: consume any pending InitiateRestEvent ──────────────────────
    // Drain all pending events; only the first one in Exploration mode takes
    // effect (extras are silently discarded to avoid double-starts).
    let mut initiate_hours: Option<u32> = None;
    for event in initiate_reader.read() {
        if initiate_hours.is_none() {
            initiate_hours = Some(event.hours);
        }
    }

    if let Some(hours) = initiate_hours {
        if matches!(game_state.mode, GameMode::Exploration) {
            info!("Rest initiated: {} hours requested", hours);
            game_state.enter_rest(hours);
            // The rest begins next frame so that the UI has one frame to show
            // the overlay before healing starts.
            return;
        }
    }

    // ── Step 2: advance the rest one hour if currently resting ─────────────
    // We take a clone of the rest_state to avoid holding a simultaneous
    // mutable borrow on game_state.mode while also mutating game_state.party
    // and game_state.time.
    let rest_state_snapshot = match &game_state.mode {
        GameMode::Resting(rs) => rs.clone(),
        _ => return,
    };

    // All requested hours completed — emit completion event and exit.
    if rest_state_snapshot.is_complete() {
        let hours_done = rest_state_snapshot.hours_completed;
        info!("Rest complete after {} hour(s)", hours_done);
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: false,
            encounter_group: None,
        });
        return;
    }

    // ── Heal one hour ───────────────────────────────────────────────────────
    if let Err(e) = rest_party_hour(&mut game_state.party) {
        // Cannot rest (no food). Treat as an interruption so the player is
        // notified and returned to exploration rather than being silently
        // stuck in Resting mode.
        warn!("rest_party_hour failed: {} — ending rest early", e);
        let hours_done = rest_state_snapshot.hours_completed;
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: false,
            encounter_group: None,
        });
        return;
    }

    // ── Advance time 60 minutes ─────────────────────────────────────────────
    // Pass `None` for stock templates: the rest system operates without
    // requiring campaign content to be loaded.  Merchant restocking during
    // rest is handled lazily when the player next opens a merchant screen.
    game_state.advance_time(60, None);

    // ── Increment the completed-hour counter ────────────────────────────────
    if let GameMode::Resting(ref mut rs) = game_state.mode {
        rs.hours_completed += 1;
        let completed = rs.hours_completed;
        let requested = rs.hours_requested;
        info!("Resting: hour {}/{} complete", completed, requested);
    }

    // ── Random encounter check ──────────────────────────────────────────────
    let mut rng = rand::rng();
    if let Some(monster_group) = random_encounter(&game_state.world, &mut rng) {
        // Encounter! Interrupt the rest.
        info!("Random encounter during rest — interrupting rest");
        let hours_done = match &game_state.mode {
            GameMode::Resting(rs) => rs.hours_completed,
            _ => rest_state_snapshot.hours_completed + 1,
        };

        // Mark interrupted and return to Exploration so handle_rest_complete
        // can start combat without nesting mode transitions.
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: true,
            encounter_group: Some(monster_group),
        });
    }
}

/// Handles the outcome of a completed or interrupted rest sequence.
///
/// Reads [`RestCompleteEvent`] and, when `interrupted_by_encounter` is
/// `true`, calls [`start_encounter`](crate::game::systems::combat::start_encounter)
/// to initialise combat from the encounter group.
///
/// This mirrors the movement-encounter path in `move_party_and_handle_events`
/// and `start_encounter` in `combat.rs`, reusing existing combat
/// initialisation without duplication.
pub fn handle_rest_complete(
    mut reader: MessageReader<RestCompleteEvent>,
    global_state: Option<ResMut<GlobalState>>,
    content: Option<Res<crate::application::resources::GameContent>>,
) {
    let mut global_state = match global_state {
        Some(gs) => gs,
        None => {
            for _ in reader.read() {}
            return;
        }
    };
    for event in reader.read() {
        if event.interrupted_by_encounter {
            if let Some(ref group) = event.encounter_group {
                info!(
                    "Rest interrupted by encounter after {} hour(s) — starting combat",
                    event.hours_completed
                );
                if let Some(ref content_res) = content {
                    match crate::game::systems::combat::start_encounter(
                        &mut global_state.0,
                        content_res.as_ref(),
                        group,
                    ) {
                        Ok(()) => {
                            info!("Combat started from rest interruption");
                        }
                        Err(e) => {
                            error!(
                                "Failed to initialise combat from rest encounter: {} — \
                                 returning to exploration",
                                e
                            );
                            global_state.0.mode = GameMode::Exploration;
                        }
                    }
                } else {
                    // No content database available (e.g. headless tests).
                    // Log and remain in Exploration (already set by process_rest).
                    warn!(
                        "Rest encounter fired but GameContent is not loaded — \
                         skipping combat initialisation"
                    );
                }
            }
        } else {
            info!(
                "Rest complete: {} hour(s) rested, party fully refreshed",
                event.hours_completed
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::RestState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::resources::REST_DURATION_HOURS;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a minimal Bevy app with `RestPlugin` and `GlobalState`.
    fn build_rest_app() -> App {
        let mut app = App::new();
        app.add_plugins(RestPlugin);

        let mut game_state = crate::application::GameState::new();
        // Give the party some food so rest doesn't fail immediately.
        game_state.party.food = 100;
        app.insert_resource(GlobalState(game_state));
        app
    }

    /// Add a single character with the given HP values to the party in `app`.
    fn add_character_with_hp(app: &mut App, hp_base: u16, hp_current: u16) {
        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.hp.base = hp_base;
        character.hp.current = hp_current;
        character.sp.base = 40;
        character.sp.current = 0;
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.party.add_member(character).unwrap();
    }

    // ── InitiateRestEvent tests ───────────────────────────────────────────────

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

    // ── RestCompleteEvent tests ───────────────────────────────────────────────

    /// [`RestCompleteEvent`] stores all fields correctly for a normal completion.
    #[test]
    fn test_rest_complete_event_normal() {
        let event = RestCompleteEvent {
            hours_completed: 12,
            interrupted_by_encounter: false,
            encounter_group: None,
        };
        assert_eq!(event.hours_completed, 12);
        assert!(!event.interrupted_by_encounter);
        assert!(event.encounter_group.is_none());
    }

    /// [`RestCompleteEvent`] stores all fields correctly for an interruption.
    #[test]
    fn test_rest_complete_event_interrupted() {
        let event = RestCompleteEvent {
            hours_completed: 3,
            interrupted_by_encounter: true,
            encounter_group: Some(vec![1, 2, 3]),
        };
        assert_eq!(event.hours_completed, 3);
        assert!(event.interrupted_by_encounter);
        assert_eq!(event.encounter_group, Some(vec![1, 2, 3]));
    }

    /// [`RestCompleteEvent`] is `Clone` and `PartialEq`.
    #[test]
    fn test_rest_complete_event_clone_eq() {
        let a = RestCompleteEvent {
            hours_completed: 6,
            interrupted_by_encounter: false,
            encounter_group: None,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ── RestPlugin registration tests ─────────────────────────────────────────

    /// [`RestPlugin`] registers both event types without panicking.
    #[test]
    fn test_rest_plugin_registers_both_events() {
        let mut app = App::new();
        app.add_plugins(RestPlugin);

        // Writing both messages must not panic.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent { hours: 12 });
        app.world_mut()
            .resource_mut::<Messages<RestCompleteEvent>>()
            .write(RestCompleteEvent {
                hours_completed: 0,
                interrupted_by_encounter: false,
                encounter_group: None,
            });
        app.update();
    }

    // ── process_rest system tests ─────────────────────────────────────────────

    /// Sending [`InitiateRestEvent`] while in `Exploration` mode transitions
    /// the game state to `GameMode::Resting`.
    #[test]
    fn test_initiate_rest_enters_resting_mode() {
        let mut app = build_rest_app();
        add_character_with_hp(&mut app, 100, 50);

        // Verify we start in Exploration.
        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, GameMode::Exploration),
                "must start in Exploration"
            );
        }

        // Fire InitiateRestEvent.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent {
                hours: REST_DURATION_HOURS,
            });

        app.update();

        // After one update the game should be in Resting mode.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Resting(_)),
            "mode should be Resting after InitiateRestEvent; got {:?}",
            gs.0.mode
        );
        if let GameMode::Resting(ref rs) = gs.0.mode {
            assert_eq!(rs.hours_requested, REST_DURATION_HOURS);
            assert_eq!(rs.hours_completed, 0);
            assert!(!rs.interrupted);
        }
    }

    /// Sending [`InitiateRestEvent`] while in `Combat` mode must NOT change
    /// the mode (rest is blocked during combat).
    #[test]
    fn test_rest_blocked_in_combat_mode() {
        let mut app = build_rest_app();

        // Put the game into Combat mode.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_combat();
        }

        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent { hours: 12 });

        app.update();

        // Mode must still be Combat.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Combat(_)),
            "Combat mode must not be overridden by rest; got {:?}",
            gs.0.mode
        );
    }

    /// Each call to `process_rest` while in `Resting` mode advances
    /// `hours_completed` by one and advances game time by 60 minutes.
    #[test]
    fn test_rest_advances_time_per_hour() {
        let mut app = build_rest_app();
        add_character_with_hp(&mut app, 120, 0);

        // Enter resting mode directly (skip initiation frame).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(6);
        }

        let initial_hour = {
            let gs = app.world().resource::<GlobalState>();
            gs.0.time.hour
        };

        // Run 6 frames.
        for _ in 0..6 {
            app.update();
        }

        let gs = app.world().resource::<GlobalState>();
        // After 6 frames with one-hour-per-frame, time must have advanced 6 hours.
        let expected_hour = (initial_hour + 6) % 24;
        assert_eq!(
            gs.0.time.hour, expected_hour,
            "time should have advanced by 6 hours; hour={} expected={}",
            gs.0.time.hour, expected_hour
        );
    }

    /// After processing all requested hours the game returns to `Exploration`
    /// and a `RestCompleteEvent` with `interrupted_by_encounter = false` is
    /// emitted.
    #[test]
    fn test_rest_completes_after_requested_hours() {
        let mut app = build_rest_app();
        add_character_with_hp(&mut app, 120, 0);

        // Enter resting mode for 3 hours (fast test).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(3);
        }

        // Run enough frames to complete (3 heal frames + 1 completion frame).
        for _ in 0..5 {
            app.update();
        }

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "mode should return to Exploration after rest completes; got {:?}",
            gs.0.mode
        );
    }

    /// After a full 12-hour rest, the party member's HP should be fully
    /// restored.
    #[test]
    fn test_rest_heals_party_after_full_rest() {
        let mut app = build_rest_app();
        add_character_with_hp(&mut app, 120, 0);

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.party.food = 200; // Plenty of food.
            gs.0.enter_rest(REST_DURATION_HOURS);
        }

        // Run enough frames to complete the rest:
        // REST_DURATION_HOURS heal frames + 1 completion frame + 1 safety.
        for _ in 0..(REST_DURATION_HOURS + 2) {
            app.update();
        }

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "mode should be Exploration after full rest"
        );
        // The party member should be fully healed.
        let member = &gs.0.party.members[0];
        assert_eq!(
            member.hp.current, member.hp.base,
            "HP should be fully restored after 12 hours of rest"
        );
    }

    /// `RestState::new` creates a state with zero completed hours and
    /// `interrupted = false`.
    #[test]
    fn test_rest_state_new() {
        let state = RestState::new(12);
        assert_eq!(state.hours_requested, 12);
        assert_eq!(state.hours_completed, 0);
        assert!(!state.interrupted);
    }

    /// `RestState::is_complete` returns `true` only when all hours are done.
    #[test]
    fn test_rest_state_is_complete() {
        let mut state = RestState::new(3);
        assert!(!state.is_complete());
        state.hours_completed = 2;
        assert!(!state.is_complete());
        state.hours_completed = 3;
        assert!(state.is_complete());
    }

    /// `GameState::enter_rest` sets the mode to `Resting` with the correct
    /// hours_requested.
    #[test]
    fn test_game_state_enter_rest() {
        let mut state = crate::application::GameState::new();
        state.enter_rest(12);
        assert!(
            matches!(state.mode, GameMode::Resting(_)),
            "enter_rest must set mode to Resting"
        );
        if let GameMode::Resting(ref rs) = state.mode {
            assert_eq!(rs.hours_requested, 12);
            assert_eq!(rs.hours_completed, 0);
        }
    }

    /// `InitiateRestEvent` is ignored when the game is already in `Resting`
    /// mode (prevent double-initiation).
    #[test]
    fn test_initiate_rest_ignored_when_already_resting() {
        let mut app = build_rest_app();
        add_character_with_hp(&mut app, 100, 50);

        // Manually enter resting mode for 12 hours.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(12);
        }

        // Send a new InitiateRestEvent for a different duration.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent { hours: 6 });

        app.update();

        // The mode should still be Resting with hours_requested == 12 (first
        // rest takes precedence; the new event is discarded because we are not
        // in Exploration mode).
        let gs = app.world().resource::<GlobalState>();
        // After one frame the rest advanced by one hour.
        if let GameMode::Resting(ref rs) = gs.0.mode {
            // hours_requested must still be 12 — the 6-hour event was ignored.
            assert_eq!(
                rs.hours_requested, 12,
                "hours_requested must not change while already resting"
            );
        } else {
            // Could be Exploration if the rest somehow completed in one tick —
            // that would be a bug for a 12-hour rest.
            panic!("Expected Resting mode, got {:?}", gs.0.mode);
        }
    }
}
