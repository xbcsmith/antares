// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Rest System Module
//!
//! Provides the Bevy event types, orchestration systems, plugin, and UI overlay
//! for the party rest sequence.
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
//! 3. Rolls for a random encounter, scaled by
//!    [`RestConfig::rest_encounter_rate_multiplier`](crate::sdk::game_config::RestConfig).
//! 4. Either interrupts the rest (encounter found) or continues until all
//!    requested hours are completed.
//!
//! While resting, [`update_rest_ui`] shows a centered [`RestProgressRoot`]
//! overlay that displays the current hour count and flavour text.  The overlay
//! is hidden automatically when the game returns to `Exploration` mode.
//!
//! When the sequence ends — by completion or encounter interruption —
//! [`RestCompleteEvent`] is written.  The [`handle_rest_complete`] system
//! reads that event:
//! - On completion: writes a "refreshed" message to the [`GameLog`] and leaves
//!   the game in `Exploration` mode (already set by `process_rest`).
//! - On encounter interruption: writes an "interrupted" message to the
//!   [`GameLog`] and calls
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
//! - **`RestConfig::rest_encounter_rate_multiplier`** — `0.0` disables all
//!   rest encounters; `1.0` uses the map's normal rate; values above `1.0`
//!   increase the chance beyond normal.
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
use crate::domain::resources::{
    consume_food, count_food_in_party, food_needed_to_rest, rest_party_hour, RestDuration,
};
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
/// let ev = InitiateRestEvent::from_duration(antares::domain::resources::RestDuration::Full);
/// assert_eq!(ev.hours, 12);
/// ```
#[derive(Message, Debug, Clone, PartialEq)]
pub struct InitiateRestEvent {
    /// Number of in-game hours to rest (4, 8, or 12).
    pub hours: u32,
    /// HP/SP fraction of each character's base to restore per hour tick.
    ///
    /// Computed from the chosen [`RestDuration`].  Stored in the event so
    /// `process_rest` can pass it straight into [`RestState`] without
    /// recomputing.
    pub restore_fraction_per_hour: f32,
}

impl InitiateRestEvent {
    /// Construct from a [`RestDuration`].
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::rest::InitiateRestEvent;
    /// use antares::domain::resources::RestDuration;
    ///
    /// let ev = InitiateRestEvent::from_duration(RestDuration::Full);
    /// assert_eq!(ev.hours, 12);
    /// ```
    pub fn from_duration(duration: RestDuration) -> Self {
        Self {
            hours: duration.hours(),
            restore_fraction_per_hour: duration.restore_fraction_per_hour(),
        }
    }
}

/// Sent when a rest sequence ends — either completed normally or interrupted
/// by a random encounter.
///
/// The [`handle_rest_complete`] system reads this event to initiate combat
/// when `interrupted_by_encounter` is `true`.  UI systems will also read it
/// to display completion / interruption messages.
///
/// Note: hunger can never interrupt a rest in progress.  The food check is
/// performed **before** rest begins; if the party lacks food the rest is
/// refused at initiation and this event is never emitted.
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestCompleteEvent;
/// use antares::domain::combat::types::CombatEventType;
///
/// // Completed rest
/// let done = RestCompleteEvent {
///     hours_completed: 12,
///     interrupted_by_encounter: false,
///     encounter_group: None,
///     encounter_combat_event_type: CombatEventType::Normal,
/// };
/// assert!(!done.interrupted_by_encounter);
///
/// // Interrupted by encounter
/// let interrupted = RestCompleteEvent {
///     hours_completed: 3,
///     interrupted_by_encounter: true,
///     encounter_group: Some(vec![1, 2]),
///     encounter_combat_event_type: CombatEventType::Ambush,
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
    /// The [`CombatEventType`] of the interrupting encounter.
    ///
    /// Defaults to [`CombatEventType::Normal`].  Set to
    /// [`CombatEventType::Ambush`] when the rest is interrupted (the party
    /// is caught off-guard while sleeping).  Forwarded to
    /// [`start_encounter`](crate::game::systems::combat::start_encounter).
    pub encounter_combat_event_type: crate::domain::combat::types::CombatEventType,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Marker component for the rest-progress overlay root node.
///
/// Spawned at startup by [`setup_rest_ui`] as an absolutely-positioned,
/// full-screen flex container that is hidden (`Display::None`) by default.
/// [`update_rest_ui`] toggles it to `Display::Flex` while the game is in
/// [`GameMode::Resting`](crate::application::GameMode::Resting).
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestProgressRoot;
/// use bevy::prelude::*;
///
/// // The component is a simple marker — no fields.
/// let _marker: RestProgressRoot = RestProgressRoot;
/// ```
#[derive(Component)]
pub struct RestProgressRoot;

/// Marker component for the rest-duration selection menu root node.
///
/// Spawned at startup by [`setup_rest_ui`] and shown (`Display::Flex`) while
/// the game is in [`GameMode::RestMenu`](crate::application::GameMode::RestMenu).
/// Hidden at all other times.  The player selects 4 / 8 / 12 hours using
/// keys `1` / `2` / `3`, or presses Escape to cancel back to Exploration.
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestMenuRoot;
///
/// let _marker: RestMenuRoot = RestMenuRoot;
/// ```
#[derive(Component)]
pub struct RestMenuRoot;

/// Marker component for the "Hour X / Y" progress label inside the overlay.
///
/// Updated every frame by [`update_rest_ui`] to reflect the current rest
/// progress stored in [`RestState`](crate::application::RestState).
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestProgressLabel;
///
/// let _marker: RestProgressLabel = RestProgressLabel;
/// ```
#[derive(Component)]
pub struct RestProgressLabel;

/// Marker component for the hint / flavour text label inside the overlay.
///
/// Updated every frame by [`update_rest_ui`] with cycling flavour messages
/// derived from the number of completed rest hours.
///
/// # Examples
///
/// ```
/// use antares::game::systems::rest::RestHintLabel;
///
/// let _marker: RestHintLabel = RestHintLabel;
/// ```
#[derive(Component)]
pub struct RestHintLabel;

// ---------------------------------------------------------------------------
// Flavour messages cycled in the overlay
// ---------------------------------------------------------------------------

/// Rest atmosphere messages displayed in the hint label.
///
/// The label cycles through these strings based on `hours_completed % len`.
pub const REST_FLAVOUR_MESSAGES: &[&str] = &[
    "The party settles in for the night.",
    "Distant sounds echo in the dark.",
    "A cool breeze drifts through the camp.",
    "The fire crackles softly.",
    "Stars wheel slowly overhead.",
    "Somewhere in the distance, an owl calls.",
    "The watch changes without incident.",
    "Dreams flicker at the edge of sleep.",
    "Silence wraps the camp like a blanket.",
    "First light begins to grey the horizon.",
    "The world holds its breath.",
    "Tired bones find welcome rest.",
];

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
            .insert_resource(RestTickTimer(Timer::from_seconds(
                REST_HOUR_REAL_SECONDS,
                TimerMode::Repeating,
            )))
            .add_systems(Startup, setup_rest_ui)
            .add_systems(
                Update,
                (
                    handle_rest_menu_input,
                    process_rest,
                    handle_rest_complete,
                    update_rest_ui,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Timer resource
// ---------------------------------------------------------------------------

/// How many real-time seconds each in-game rest hour takes.
///
/// At 1.0 s/hour a 12-hour rest takes 12 seconds — long enough to read the
/// flavour text but not tedious.  Adjust in [`RestPlugin`] if needed.
pub const REST_HOUR_REAL_SECONDS: f32 = 1.0;

/// Bevy resource that gates the per-hour rest tick.
///
/// Each Bevy frame [`process_rest`] calls `tick()` on this timer.  An hour
/// of healing is only applied when the timer fires (i.e. once every
/// [`REST_HOUR_REAL_SECONDS`] of real time).  This makes the rest sequence
/// visible to the player instead of blinking past in a fraction of a second.
#[derive(Resource)]
pub struct RestTickTimer(pub Timer);

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns all rest-related UI hierarchies (runs once at startup).
///
/// Spawns two hidden overlays:
///
/// 1. **Rest-duration menu** ([`RestMenuRoot`]) — shown in
///    [`GameMode::RestMenu`](crate::application::GameMode::RestMenu).
///    Displays three choices (4 / 8 / 12 hours) with their HP/SP restore
///    percentages.  The player presses `1`, `2`, or `3` to choose, or
///    `Escape` to cancel.
///
/// 2. **Rest-progress overlay** ([`RestProgressRoot`]) — shown while in
///    [`GameMode::Resting`](crate::application::GameMode::Resting).
///    Displays the current hour, a progress bar, and cycling flavour text.
///
/// # Node hierarchy (rest menu)
///
/// ```text
/// RestMenuRoot  (full-screen dim overlay)
///   └─ panel (column)
///        ├─ "Rest — choose duration"  (title)
///        ├─ "[1]  4 hours  — 50% HP/SP restored"
///        ├─ "[2]  8 hours  — 75% HP/SP restored"
///        ├─ "[3] 12 hours  — 100% HP/SP restored"
///        └─ "[Esc] Cancel"
/// ```
///
/// # Node hierarchy (rest progress)
///
/// ```text
/// RestProgressRoot  (full-screen dim overlay)
///   └─ panel (column)
///        ├─ "Resting…"              (title)
///        ├─ "Hour X / Y"            ← RestProgressLabel
///        └─ flavour text            ← RestHintLabel
/// ```
pub fn setup_rest_ui(mut commands: Commands) {
    // ── Rest-duration menu ────────────────────────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.80)),
            ZIndex(100),
            RestMenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(40.0)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Rest \u{2014} Choose Duration"),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.9, 0.6, 1.0)),
                    ));
                    panel.spawn((
                        Text::new("[1]  4 hours  \u{2014}  50% HP/SP restored"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.8, 1.0, 0.8, 1.0)),
                    ));
                    panel.spawn((
                        Text::new("[2]  8 hours  \u{2014}  75% HP/SP restored"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.8, 1.0, 0.8, 1.0)),
                    ));
                    panel.spawn((
                        Text::new("[3] 12 hours  \u{2014} 100% HP/SP restored"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.8, 1.0, 0.8, 1.0)),
                    ));
                    panel.spawn((
                        Text::new("[Esc]  Cancel"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
                    ));
                });
        });

    // ── Rest-progress overlay ─────────────────────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            ZIndex(100),
            RestProgressRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Resting\u{2026}"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.9, 0.6, 1.0)),
                    ));

                    panel.spawn((
                        Text::new("Hour 0 / 12"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.9, 0.9, 0.9, 1.0)),
                        RestProgressLabel,
                    ));

                    panel.spawn((
                        Text::new(REST_FLAVOUR_MESSAGES[0]),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.6, 0.8, 0.6, 1.0)),
                        RestHintLabel,
                    ));

                    panel.spawn((
                        Text::new("(encounter may interrupt)"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.7, 0.5, 0.5, 1.0)),
                    ));
                });
        });
}

/// Handles keyboard input while the rest-duration menu is open.
///
/// Runs every frame (before [`process_rest`]).  When the game is in
/// [`GameMode::RestMenu`](crate::application::GameMode::RestMenu):
///
/// - `1` → initiate a 4-hour (Short) rest
/// - `2` → initiate an 8-hour (Long) rest
/// - `3` → initiate a 12-hour (Full) rest
/// - `Escape` → cancel and return to Exploration
///
/// On a valid choice the system fires [`InitiateRestEvent`] and the game
/// transitions to `Resting` on the same frame via [`process_rest`].
pub fn handle_rest_menu_input(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: Option<ResMut<GlobalState>>,
    mut rest_writer: MessageWriter<InitiateRestEvent>,
) {
    let Some(ref mut gs) = global_state else {
        return;
    };
    if !matches!(gs.0.mode, GameMode::RestMenu) {
        return;
    }

    // If there is no keyboard resource (e.g. headless test apps that don't
    // register ButtonInput), do nothing — the menu stays open until the
    // caller sets the mode manually or a test fires InitiateRestEvent directly.
    let Some(keyboard) = keyboard else {
        return;
    };

    let chosen = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(RestDuration::Short)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(RestDuration::Long)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(RestDuration::Full)
    } else {
        None
    };

    if let Some(duration) = chosen {
        info!(
            "Rest menu: player chose {} hours ({:.0}% HP/SP)",
            duration.hours(),
            duration.total_restore_fraction() * 100.0
        );
        // Return to Exploration first — process_rest will re-enter Resting.
        gs.0.mode = GameMode::Exploration;
        rest_writer.write(InitiateRestEvent::from_duration(duration));
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        info!("Rest menu: cancelled");
        gs.0.mode = GameMode::Exploration;
    }
}

/// Shows or hides the rest-duration menu and rest-progress overlays.
///
/// Runs every frame (after [`process_rest`]).  Toggles `Display` on both
/// overlay roots based on the current [`GameMode`], and keeps the progress
/// labels up-to-date while resting.
pub fn update_rest_ui(
    global_state: Option<Res<GlobalState>>,
    mut menu_query: Query<&mut Node, (With<RestMenuRoot>, Without<RestProgressRoot>)>,
    mut overlay_query: Query<&mut Node, (With<RestProgressRoot>, Without<RestMenuRoot>)>,
    mut progress_query: Query<&mut Text, (With<RestProgressLabel>, Without<RestHintLabel>)>,
    mut hint_query: Query<&mut Text, (With<RestHintLabel>, Without<RestProgressLabel>)>,
) {
    let Some(global_state) = global_state else {
        return;
    };

    let is_rest_menu = matches!(global_state.0.mode, GameMode::RestMenu);
    let is_resting = matches!(global_state.0.mode, GameMode::Resting(_));

    for mut node in &mut menu_query {
        node.display = if is_rest_menu {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut node in &mut overlay_query {
        node.display = if is_resting {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !is_resting {
        return;
    }

    if let GameMode::Resting(ref rs) = global_state.0.mode {
        let completed = rs.hours_completed;
        let requested = rs.hours_requested;
        let progress_text = format!("Hour {} / {}", completed + 1, requested);

        for mut text in &mut progress_query {
            **text = progress_text.clone();
        }

        let flavour_idx = (completed as usize) % REST_FLAVOUR_MESSAGES.len();
        let flavour = REST_FLAVOUR_MESSAGES[flavour_idx];
        for mut text in &mut hint_query {
            **text = flavour.to_string();
        }
    }
}

/// Orchestrates the per-hour rest loop.
///
/// Each Bevy frame this system:
///
/// 1. If [`InitiateRestEvent`] is pending **and** the game is in
///    [`Exploration`](crate::application::GameMode::Exploration) mode,
///    performs the food check, consumes rations, and calls
///    [`GameState::enter_rest`] to begin the sequence.
/// 2. If the game is in [`GameMode::Resting`](crate::application::GameMode::Resting):
///    - Advances the [`RestTickTimer`].  An hour tick only fires when the
///      timer completes (once per [`REST_HOUR_REAL_SECONDS`] of real time).
///    - Calls [`rest_party_hour`] with the stored `restore_fraction_per_hour`
///      to heal one hour of HP/SP.
///    - Advances game time by 60 minutes.
///    - Rolls for a random encounter, scaled by
///      [`RestConfig::rest_encounter_rate_multiplier`].  A multiplier of
///      `0.0` skips the encounter roll entirely.
///    - On encounter: sets `interrupted`, writes [`RestCompleteEvent`], and
///      returns the game to `Exploration` mode (combat is started by
///      [`handle_rest_complete`]).
///    - On normal completion: writes [`RestCompleteEvent`] and returns to
///      `Exploration`.
pub fn process_rest(
    global_state: Option<ResMut<GlobalState>>,
    mut initiate_reader: MessageReader<InitiateRestEvent>,
    mut complete_writer: MessageWriter<RestCompleteEvent>,
    mut tick_timer: Option<ResMut<RestTickTimer>>,
    time: Option<Res<Time>>,
    content: Option<Res<crate::application::resources::GameContent>>,
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
    let mut initiate_event: Option<InitiateRestEvent> = None;
    for event in initiate_reader.read() {
        if initiate_event.is_none() {
            initiate_event = Some(event.clone());
        }
    }

    if let Some(event) = initiate_event {
        if matches!(game_state.mode, GameMode::Exploration) {
            // Check food upfront — 1 ration per party member required.
            // If the party can't pay, refuse rest entirely: no food consumed,
            // no HP/SP restored, no mode change.
            //
            // Food is tracked as IsFood inventory items.  We use an
            // empty ItemDatabase as a fallback so the food check passes
            // gracefully when no campaign content is loaded (e.g. unit tests
            // that don't set up GameContent).
            let empty_item_db = crate::domain::items::ItemDatabase::new();
            let item_db = content
                .as_ref()
                .map(|c| &c.db().items)
                .unwrap_or(&empty_item_db);

            let needed = food_needed_to_rest(&game_state.party);
            let available = count_food_in_party(&game_state.party, item_db);
            if available < needed {
                warn!(
                    "Rest refused: party needs {} food ration(s) but has {} in inventories — \
                     cannot rest while hungry",
                    needed, available
                );
                return;
            }

            // Consume food now (from inventories), before the first healing tick.
            // SAFETY: we already verified available >= needed above, so this
            // cannot fail.  The unwrap is intentional.
            consume_food(
                &mut game_state.party,
                item_db,
                crate::domain::resources::FOOD_PER_REST,
            )
            .expect("consume_food must succeed after food check passed");

            info!(
                "Rest initiated: {} hours requested ({:.0}% HP/SP)",
                event.hours,
                event.restore_fraction_per_hour * event.hours as f32 * 100.0
            );

            // Reset the tick timer so the first hour doesn't fire immediately.
            if let Some(ref mut timer) = tick_timer {
                timer.0.reset();
            }

            game_state.mode = GameMode::Resting(crate::application::RestState::with_fraction(
                event.hours,
                event.restore_fraction_per_hour,
            ));
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

    // ── Tick the real-time timer ────────────────────────────────────────────
    // Only advance one hour of in-game rest when the timer fires.
    // This paces the rest to REST_HOUR_REAL_SECONDS per hour so the player
    // can see the progress overlay and flavour text update.
    let timer_fired = if let Some(ref mut timer) = tick_timer {
        // Use the real delta when available; fall back to zero (timer won't
        // fire, but unit tests override by inserting a zero-duration timer).
        let delta = time.as_ref().map(|t| t.delta()).unwrap_or_default();
        timer.0.tick(delta).just_finished()
    } else {
        // No timer resource at all — fire every frame so legacy tests that
        // call enter_rest directly still complete quickly.
        true
    };

    if !timer_fired {
        return;
    }

    // All requested hours completed — emit completion event and exit.
    if rest_state_snapshot.is_complete() {
        let hours_done = rest_state_snapshot.hours_completed;
        info!("Rest complete after {} hour(s)", hours_done);
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: false,
            encounter_group: None,
            encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        });
        return;
    }

    // ── Heal one hour ───────────────────────────────────────────────────────
    // Food was already consumed upfront at initiation — rest_party_hour is a
    // pure healing tick and will not fail under normal circumstances.
    //
    // Pass hours_completed_after_tick = hours_completed + 1 so the function
    // can compute the cumulative healing target for this tick without
    // rounding-to-zero on low base HP values.
    let restore_fraction = rest_state_snapshot.restore_fraction_per_hour;
    let hours_after = rest_state_snapshot.hours_completed + 1;
    if let Err(e) = rest_party_hour(&mut game_state.party, restore_fraction, hours_after) {
        // Unexpected error (future condition variants, etc.).  Log and abort.
        warn!(
            "rest_party_hour failed unexpectedly: {} — ending rest early",
            e
        );
        let hours_done = rest_state_snapshot.hours_completed;
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: false,
            encounter_group: None,
            encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        });
        return;
    }

    // ── Advance time 60 minutes ─────────────────────────────────────────────
    // Pass `None` for stock templates: the rest system operates without
    // requiring campaign content to be loaded.  Merchant restocking during
    // rest is handled lazily when the player next opens a merchant screen.
    game_state.advance_time_minutes(60, None);

    // ── Increment the completed-hour counter ────────────────────────────────
    if let GameMode::Resting(ref mut rs) = game_state.mode {
        rs.hours_completed += 1;
        let completed = rs.hours_completed;
        let requested = rs.hours_requested;
        let pct = (restore_fraction * requested as f32 * 100.0).round() as u32;
        info!(
            "Resting: hour {}/{} complete ({}% HP/SP target)",
            completed, requested, pct
        );
    }

    // ── Random encounter check ──────────────────────────────────────────────
    // The encounter chance is scaled by `rest_encounter_rate_multiplier`.
    // A multiplier of 0.0 disables encounters completely; 1.0 is normal rate.
    let multiplier = game_state.config.rest.rest_encounter_rate_multiplier;

    let encounter_group = if multiplier <= 0.0 {
        // Multiplier is zero — skip the RNG roll entirely.
        None
    } else {
        let mut rng = rand::rng();
        if multiplier >= 1.0 {
            // Normal or elevated rate: roll once (or use the result directly
            // for multiplier == 1.0; for values > 1.0 we keep the single roll
            // since `random_encounter` already encodes terrain probability).
            random_encounter(&game_state.world, &mut rng)
        } else {
            // Reduced rate: roll and then apply the multiplier as an extra
            // probability gate.  If the base roll returns a group, accept it
            // only with probability == multiplier.
            random_encounter(&game_state.world, &mut rng).and_then(|group| {
                use rand::Rng as _;
                if rng.random::<f32>() < multiplier {
                    Some(group)
                } else {
                    None
                }
            })
        }
    };

    if let Some(encounter_group) = encounter_group {
        // Encounter! Interrupt the rest.
        info!("Random encounter during rest — interrupting rest");
        let hours_done = match &game_state.mode {
            GameMode::Resting(rs) => rs.hours_completed,
            _ => rest_state_snapshot.hours_completed + 1,
        };

        // Rest interruptions are always ambushes — the party is caught
        // off-guard while sleeping.
        let encounter_combat_event_type = crate::domain::combat::types::CombatEventType::Ambush;

        // Mark interrupted and return to Exploration so handle_rest_complete
        // can start combat without nesting mode transitions.
        game_state.mode = GameMode::Exploration;
        complete_writer.write(RestCompleteEvent {
            hours_completed: hours_done,
            interrupted_by_encounter: true,
            encounter_group: Some(encounter_group.monster_group),
            encounter_combat_event_type,
        });
    }
}

/// Handles the outcome of a completed or interrupted rest sequence.
///
/// Reads [`RestCompleteEvent`] and:
/// - On **completion**: writes `"The party rests for the night and awakens
///   refreshed."` to the [`GameLog`].
/// - On **encounter interruption**: writes `"Rest interrupted! Enemies
///   attack!"` to the [`GameLog`] and calls
///   [`start_encounter`](crate::game::systems::combat::start_encounter) to
///   initialise combat from the encounter group.
///
/// This mirrors the movement-encounter path in `move_party_and_handle_events`
/// and `start_encounter` in `combat.rs`, reusing existing combat
/// initialisation without duplication.
pub fn handle_rest_complete(
    mut reader: MessageReader<RestCompleteEvent>,
    global_state: Option<ResMut<GlobalState>>,
    content: Option<Res<crate::application::resources::GameContent>>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
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

                // Write interruption notice to the game log (UI notification).
                if let Some(ref mut log_res) = game_log {
                    log_res.add_combat("Rest interrupted! Enemies attack!".to_string());
                }

                if let Some(ref content_res) = content {
                    match crate::game::systems::combat::start_encounter(
                        &mut global_state.0,
                        content_res.as_ref(),
                        group,
                        event.encounter_combat_event_type,
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

            // Write completion notice to the game log (UI notification).
            if let Some(ref mut log_res) = game_log {
                log_res.add_exploration(
                    "The party rests for the night and awakens refreshed.".to_string(),
                );
            }
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
    use crate::domain::resources::{RestDuration, REST_DURATION_HOURS};
    use crate::sdk::game_config::RestConfig;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a minimal `ItemDatabase` containing a "Food Ration" item
    /// (id=1, `IsFood(1)`) for use in rest tests that require food.
    fn make_food_item_db() -> crate::domain::items::ItemDatabase {
        use crate::domain::items::types::{ConsumableData, ConsumableEffect};
        use crate::domain::items::{Item, ItemDatabase, ItemType};

        let mut db = ItemDatabase::new();
        db.add_item(Item {
            id: 1,
            name: "Food Ration".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::IsFood(1),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 5,
            sell_cost: 2,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        })
        .unwrap();
        db
    }

    /// Build a Bevy `GameContent` resource whose item DB contains a Food Ration.
    fn make_food_game_content() -> crate::application::resources::GameContent {
        let mut db = crate::sdk::database::ContentDatabase::new();
        db.items = make_food_item_db();
        crate::application::resources::GameContent::new(db)
    }

    /// Build a minimal Bevy app with `RestPlugin`, `GlobalState`, `GameLog`,
    /// and a `GameContent` resource that contains a Food Ration item (id=1).
    fn build_rest_app() -> App {
        let mut app = App::new();
        app.add_plugins(RestPlugin);
        app.init_resource::<crate::game::systems::ui::GameLog>();

        let game_state = crate::application::GameState::new();
        app.insert_resource(GlobalState(game_state));
        // Insert a GameContent resource so process_rest can resolve IsFood items.
        app.insert_resource(make_food_game_content());

        // Use a zero-duration timer so every frame fires a tick in tests.
        app.insert_resource(RestTickTimer(Timer::from_seconds(
            0.0,
            TimerMode::Repeating,
        )));
        app
    }

    /// Build a minimal Bevy app configured with a specific [`RestConfig`].
    fn build_rest_app_with_config(rest_config: RestConfig) -> App {
        let mut app = App::new();
        app.add_plugins(RestPlugin);
        app.init_resource::<crate::game::systems::ui::GameLog>();

        let mut game_state = crate::application::GameState::new();
        game_state.config.rest = rest_config;
        app.insert_resource(GlobalState(game_state));
        // Insert GameContent so food checks work in any test that needs it.
        app.insert_resource(make_food_game_content());

        // Zero-duration timer so every frame fires a tick in tests.
        app.insert_resource(RestTickTimer(Timer::from_seconds(
            0.0,
            TimerMode::Repeating,
        )));
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

    /// Add a character AND give them `rations` Food Ration items (id=1).
    fn add_character_with_hp_and_food(
        app: &mut App,
        hp_base: u16,
        hp_current: u16,
        rations: usize,
    ) {
        add_character_with_hp(app, hp_base, hp_current);
        // The member just added is the last one.
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        let member = gs.0.party.members.last_mut().unwrap();
        for _ in 0..rations {
            member.inventory.add_item(1, 0).unwrap();
        }
    }

    // ── RestConfig multiplier tests ───────────────────────────────────────────

    /// When `rest_encounter_rate_multiplier` is `0.0`, running many rest ticks
    /// must never produce a `RestCompleteEvent` with `interrupted_by_encounter`.
    ///
    /// We run enough ticks to complete a full rest; the absence of an
    /// interrupted event confirms that the zero-multiplier path is exercised.
    #[test]
    fn test_rest_config_zero_multiplier_prevents_encounters() {
        let config = RestConfig {
            full_rest_hours: 12,
            rest_encounter_rate_multiplier: 0.0,
            allow_partial_rest: false,
        };
        let mut app = build_rest_app_with_config(config);
        // Add character with enough food rations to survive the full rest.
        add_character_with_hp_and_food(&mut app, 120, 0, 5);

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(REST_DURATION_HOURS);
        }

        // Run enough frames to complete the full rest; any encounter
        // interruption would change the mode before all hours finish.
        for _ in 0..(REST_DURATION_HOURS + 5) {
            app.update();
        }

        // The game must have returned to Exploration normally (not interrupted).
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "zero multiplier: rest should complete without interruption; got {:?}",
            gs.0.mode
        );
    }

    // ── RestProgressRoot marker tests ─────────────────────────────────────────

    /// `RestProgressRoot` is a plain marker component with no fields.
    #[test]
    fn test_rest_progress_root_is_marker() {
        let _marker = RestProgressRoot;
    }

    /// `RestProgressLabel` is a plain marker component with no fields.
    #[test]
    fn test_rest_progress_label_is_marker() {
        let _marker = RestProgressLabel;
    }

    /// `RestHintLabel` is a plain marker component with no fields.
    #[test]
    fn test_rest_hint_label_is_marker() {
        let _marker = RestHintLabel;
    }

    // ── Flavour message tests ─────────────────────────────────────────────────

    /// The `REST_FLAVOUR_MESSAGES` slice must be non-empty so that the modulo
    /// index in `update_rest_ui` never panics.
    #[test]
    fn test_rest_flavour_messages_non_empty() {
        assert!(
            !REST_FLAVOUR_MESSAGES.is_empty(),
            "REST_FLAVOUR_MESSAGES must contain at least one entry"
        );
    }

    /// Every message in `REST_FLAVOUR_MESSAGES` must be non-empty.
    #[test]
    fn test_rest_flavour_messages_all_non_empty() {
        for (i, msg) in REST_FLAVOUR_MESSAGES.iter().enumerate() {
            assert!(!msg.is_empty(), "REST_FLAVOUR_MESSAGES[{}] is empty", i);
        }
    }

    /// The modulo index used in `update_rest_ui` must always be in-bounds.
    #[test]
    fn test_rest_flavour_index_never_out_of_bounds() {
        for hours_completed in 0u32..=100 {
            let idx = (hours_completed as usize) % REST_FLAVOUR_MESSAGES.len();
            assert!(
                idx < REST_FLAVOUR_MESSAGES.len(),
                "index {} out of bounds for hours_completed={}",
                idx,
                hours_completed
            );
        }
    }

    // ── RestPlugin UI system registration tests ───────────────────────────────

    /// `RestPlugin::build` must not panic when the app has no `GlobalState`.
    #[test]
    fn test_rest_plugin_builds_without_global_state() {
        let mut app = App::new();
        app.add_plugins(RestPlugin);
        // Running update without GlobalState must not panic.
        app.update();
    }

    // ── GameLog notification tests ────────────────────────────────────────────

    /// When a `RestCompleteEvent` with `interrupted_by_encounter = false` is
    /// processed, the `GameLog` must contain the completion message.
    #[test]
    fn test_rest_completion_message_emitted() {
        let mut app = build_rest_app();
        // Add character with food so the rest system has food items available.
        add_character_with_hp_and_food(&mut app, 120, 0, 5);

        // Enter a 1-hour rest directly (food already consumed upfront in enter_rest path).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(1);
        }

        // Frame 1: heals one hour.
        app.update();
        // Frame 2: rest completes → RestCompleteEvent written → handle_rest_complete fires.
        app.update();

        let log = app.world().resource::<crate::game::systems::ui::GameLog>();
        let has_complete_msg = log
            .entries()
            .iter()
            .any(|m| m.contains("awakens refreshed"));
        assert!(
            has_complete_msg,
            "GameLog must contain the rest-completion message; log={:?}",
            log.entries()
        );
    }

    /// When a `RestCompleteEvent` with `interrupted_by_encounter = true` is
    /// written directly to the message bus, `handle_rest_complete` must add
    /// the interruption notice to the `GameLog`.
    #[test]
    fn test_rest_interrupt_message_emitted() {
        let mut app = build_rest_app();

        // Write a pre-built interrupted event directly so we don't need to
        // seed the RNG or rely on the world having encounter tables.
        app.world_mut()
            .resource_mut::<Messages<RestCompleteEvent>>()
            .write(RestCompleteEvent {
                hours_completed: 3,
                interrupted_by_encounter: true,
                encounter_group: Some(vec![1, 2, 3]),
                encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Ambush,
            });

        app.update();

        let log = app.world().resource::<crate::game::systems::ui::GameLog>();
        let has_interrupt_msg = log.entries().iter().any(|m| m.contains("interrupted"));
        assert!(
            has_interrupt_msg,
            "GameLog must contain the rest-interruption message; log={:?}",
            log.entries()
        );
    }

    // ── update_rest_ui overlay visibility tests ───────────────────────────────

    /// While in `Resting` mode the `RestProgressRoot` node must have
    /// `Display::Flex`.
    #[test]
    fn test_rest_ui_shows_during_resting_mode() {
        let mut app = build_rest_app();
        add_character_with_hp_and_food(&mut app, 100, 50, 5);

        // Startup systems run on the first update — let them run first.
        app.update();

        // Now enter resting mode.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(12);
        }

        app.update();

        // The overlay must be visible.
        let mut found = false;
        let mut visible = false;
        let mut query = app.world_mut().query::<(&Node, &RestProgressRoot)>();
        for (node, _) in query.iter(app.world()) {
            found = true;
            visible = matches!(node.display, Display::Flex);
        }
        assert!(found, "RestProgressRoot entity must exist after startup");
        assert!(
            visible,
            "RestProgressRoot must have Display::Flex while resting"
        );
    }

    /// After the rest completes, the `RestProgressRoot` node must be hidden
    /// (`Display::None`).
    #[test]
    fn test_rest_ui_hides_after_completion() {
        let mut app = build_rest_app();
        add_character_with_hp_and_food(&mut app, 120, 0, 5);

        // Let startup systems run.
        app.update();

        // Enter and complete a 1-hour rest.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_rest(1);
        }
        // Frame 1: heals one hour.
        app.update();
        // Frame 2: rest completes, mode returns to Exploration.
        app.update();
        // Frame 3: update_rest_ui hides the overlay.
        app.update();

        let mut found = false;
        let mut hidden = false;
        let mut query = app.world_mut().query::<(&Node, &RestProgressRoot)>();
        for (node, _) in query.iter(app.world()) {
            found = true;
            hidden = matches!(node.display, Display::None);
        }
        assert!(found, "RestProgressRoot entity must exist");
        assert!(
            hidden,
            "RestProgressRoot must have Display::None after rest completes"
        );
    }

    // ── InitiateRestEvent tests ───────────────────────────────────────────────

    /// [`InitiateRestEvent`] stores the requested hour count correctly.
    #[test]
    fn test_initiate_rest_event_stores_hours() {
        let event = InitiateRestEvent::from_duration(RestDuration::Full);
        assert_eq!(event.hours, 12);
    }

    #[test]
    fn test_initiate_rest_event_from_duration_short() {
        let ev = InitiateRestEvent::from_duration(RestDuration::Short);
        assert_eq!(ev.hours, 4);
        assert!(
            (ev.restore_fraction_per_hour - RestDuration::Short.restore_fraction_per_hour()).abs()
                < 1e-6
        );
    }

    #[test]
    fn test_initiate_rest_event_from_duration_long() {
        let ev = InitiateRestEvent::from_duration(RestDuration::Long);
        assert_eq!(ev.hours, 8);
        assert!(
            (ev.restore_fraction_per_hour - RestDuration::Long.restore_fraction_per_hour()).abs()
                < 1e-6
        );
    }

    #[test]
    fn test_initiate_rest_event_clone_and_eq() {
        let a = InitiateRestEvent::from_duration(RestDuration::Long);
        let b = a.clone();
        assert_eq!(a.hours, b.hours);
    }

    #[test]
    fn test_initiate_rest_event_inequality() {
        let a = InitiateRestEvent::from_duration(RestDuration::Short);
        let b = InitiateRestEvent::from_duration(RestDuration::Full);
        assert_ne!(a.hours, b.hours);
    }

    // ── RestCompleteEvent tests ───────────────────────────────────────────────

    /// Pressing 1/2/3 in RestMenu mode fires InitiateRestEvent and exits RestMenu.
    #[test]
    fn test_rest_menu_key_1_fires_short_rest() {
        let mut app = build_rest_app();
        add_character_with_hp_and_food(&mut app, 100, 50, 5);

        app.insert_resource(ButtonInput::<KeyCode>::default());

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.mode = GameMode::RestMenu;
        }

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit1);
        app.update();

        let gs = app.world().resource::<GlobalState>();
        // Mode should have moved to Resting (process_rest consumed the event).
        assert!(
            matches!(gs.0.mode, GameMode::Resting(_)) || matches!(gs.0.mode, GameMode::Exploration),
            "after pressing 1 in RestMenu mode should be Resting or Exploration; got {:?}",
            gs.0.mode
        );
    }

    /// Pressing Escape in RestMenu mode cancels back to Exploration.
    #[test]
    fn test_rest_menu_escape_cancels() {
        let mut app = build_rest_app();

        app.insert_resource(ButtonInput::<KeyCode>::default());

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.mode = GameMode::RestMenu;
        }

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "Escape in RestMenu must return to Exploration; got {:?}",
            gs.0.mode
        );
    }

    /// [`RestCompleteEvent`] stores all fields correctly for a normal completion.
    #[test]
    fn test_rest_complete_event_normal() {
        let event = RestCompleteEvent {
            hours_completed: 12,
            interrupted_by_encounter: false,
            encounter_group: None,
            encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        };
        assert_eq!(event.hours_completed, 12);
        assert!(!event.interrupted_by_encounter);
        assert!(event.encounter_group.is_none());
        assert_eq!(
            event.encounter_combat_event_type,
            crate::domain::combat::types::CombatEventType::Normal
        );
    }

    /// [`RestCompleteEvent`] stores all fields correctly for an interruption.
    #[test]
    fn test_rest_complete_event_interrupted() {
        let event = RestCompleteEvent {
            hours_completed: 3,
            interrupted_by_encounter: true,
            encounter_group: Some(vec![1, 2, 3]),
            encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Ambush,
        };
        assert_eq!(event.hours_completed, 3);
        assert!(event.interrupted_by_encounter);
        assert_eq!(event.encounter_group, Some(vec![1, 2, 3]));
        assert_eq!(
            event.encounter_combat_event_type,
            crate::domain::combat::types::CombatEventType::Ambush
        );
    }

    /// [`RestCompleteEvent`] is `Clone` and `PartialEq`.
    #[test]
    fn test_rest_complete_event_clone_eq() {
        let a = RestCompleteEvent {
            hours_completed: 6,
            interrupted_by_encounter: false,
            encounter_group: None,
            encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
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
            .write(InitiateRestEvent::from_duration(RestDuration::Full));
        app.world_mut()
            .resource_mut::<Messages<RestCompleteEvent>>()
            .write(RestCompleteEvent {
                hours_completed: 0,
                interrupted_by_encounter: false,
                encounter_group: None,
                encounter_combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            });
        app.update();
    }

    // ── process_rest system tests ─────────────────────────────────────────────

    /// Sending [`InitiateRestEvent`] while in `Exploration` mode transitions
    /// the game state to `GameMode::Resting`.
    #[test]
    fn test_initiate_rest_enters_resting_mode() {
        let mut app = build_rest_app();
        // Add a member with food rations so the food check in process_rest passes.
        add_character_with_hp_and_food(&mut app, 100, 50, 5);

        // Verify we start in Exploration.
        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, GameMode::Exploration),
                "must start in Exploration"
            );
        }

        // Fire InitiateRestEvent for a Full rest.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent::from_duration(RestDuration::Full));

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
            assert!(
                (rs.restore_fraction_per_hour - RestDuration::Full.restore_fraction_per_hour())
                    .abs()
                    < 1e-6,
                "Full rest must store correct per-hour fraction"
            );
        }
    }

    /// When the party does not have enough food (< 1 ration per member),
    /// `process_rest` must refuse the rest: mode stays `Exploration`,
    /// no food is consumed, and the game never enters `Resting`.
    #[test]
    fn test_rest_refused_when_insufficient_food() {
        let mut app = build_rest_app();
        // Add 3 party members — they need 3 rations total.
        // Give only 2 rations across all members (one short).
        add_character_with_hp_and_food(&mut app, 100, 50, 1); // 1 ration
        add_character_with_hp_and_food(&mut app, 100, 50, 1); // 1 ration
        add_character_with_hp(&mut app, 100, 50); // 0 rations — total = 2, need 3

        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent::from_duration(RestDuration::Full));

        app.update();

        let gs = app.world().resource::<GlobalState>();

        // Mode must still be Exploration — rest was refused.
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "mode must remain Exploration when party lacks food; got {:?}",
            gs.0.mode
        );

        // Inventories must be untouched — no partial consumption on failure.
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            1,
            "food items must not be consumed when rest is refused"
        );
        assert_eq!(
            gs.0.party.members[1].inventory.items.len(),
            1,
            "food items must not be consumed when rest is refused"
        );
        assert_eq!(
            gs.0.party.members[2].inventory.items.len(),
            0,
            "member with no food must still have empty inventory"
        );

        // HP must be unchanged — no healing occurred.
        assert_eq!(
            gs.0.party.members[0].hp.current, 50,
            "HP must not change when rest is refused"
        );
    }

    /// When the party has exactly enough food (1 ration per member),
    /// rest must be accepted: mode transitions to `Resting` and food is consumed.
    #[test]
    fn test_rest_accepted_with_exact_food() {
        let mut app = build_rest_app();
        // Exactly 1 ration per member for 2 members — just enough.
        add_character_with_hp_and_food(&mut app, 100, 50, 1);
        add_character_with_hp_and_food(&mut app, 100, 50, 1);

        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent::from_duration(RestDuration::Full));

        app.update();

        let gs = app.world().resource::<GlobalState>();

        // Mode must have transitioned to Resting.
        assert!(
            matches!(gs.0.mode, GameMode::Resting(_)),
            "mode must be Resting when party has exactly enough food; got {:?}",
            gs.0.mode
        );

        // Both food rations must have been consumed upfront from inventories.
        let total_food: usize =
            gs.0.party
                .members
                .iter()
                .map(|m| m.inventory.items.len())
                .sum();
        assert_eq!(
            total_food, 0,
            "both rations must be consumed upfront when rest begins"
        );
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
            .write(InitiateRestEvent::from_duration(RestDuration::Full));

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
        add_character_with_hp_and_food(&mut app, 120, 0, 5);

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
        add_character_with_hp_and_food(&mut app, 120, 0, 5);

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
        // Food already consumed upfront when entering Resting mode directly;
        // adding rations here ensures inventory-based count is satisfied.
        add_character_with_hp_and_food(&mut app, 120, 0, 20);

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            // Use with_fraction so the correct heal rate is stored.
            gs.0.mode = GameMode::Resting(crate::application::RestState::with_fraction(
                REST_DURATION_HOURS,
                RestDuration::Full.restore_fraction_per_hour(),
            ));
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
            // enter_rest(12) should use Full rate
            assert!(
                (rs.restore_fraction_per_hour - RestDuration::Full.restore_fraction_per_hour())
                    .abs()
                    < 1e-6,
                "enter_rest(12) must store Full duration heal rate"
            );
        }
    }

    /// enter_rest_menu transitions to GameMode::RestMenu.
    #[test]
    fn test_game_state_enter_rest_menu() {
        let mut state = crate::application::GameState::new();
        state.enter_rest_menu();
        assert!(
            matches!(state.mode, GameMode::RestMenu),
            "enter_rest_menu must set mode to RestMenu"
        );
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
            gs.0.mode = GameMode::Resting(crate::application::RestState::with_fraction(
                12,
                RestDuration::Full.restore_fraction_per_hour(),
            ));
        }

        // Send a new InitiateRestEvent for a different duration — should be ignored
        // because we are already in Resting mode, not Exploration.
        app.world_mut()
            .resource_mut::<Messages<InitiateRestEvent>>()
            .write(InitiateRestEvent::from_duration(RestDuration::Short));

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
