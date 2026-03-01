# Time System Implementation Plan

## Overview

Antares already has a foundational `GameTime` struct (`day`, `hour`, `minute`) stored
in `GameState.time`, and `GameState::advance_time()` already ticks active-spell
durations. However, time is not wired to any player actions outside of resting.
Movement, combat, and map transitions all happen without advancing the clock.
There is also no in-game clock visible in the HUD, and no mechanism for events to
fire at specific times of day.

This plan extends the existing infrastructure in four sequential phases:

1. Wire time advancement to all player actions (movement, combat, travel).
2. Add a `TimeOfDay` enum and time-aware game state helpers.
3. Add a clock widget to the HUD beneath the compass.
4. Add time-conditional events and scheduled event triggers.

---

## Current State Analysis

### Existing Infrastructure

- `GameTime` in [src/domain/types.rs](../../src/domain/types.rs) — `day`, `hour`,
  `minute`; `advance_minutes()`, `advance_hours()`, `advance_days()`, `is_night()`,
  `is_day()`.
- `GameState.time: GameTime` in [src/application/mod.rs](../../src/application/mod.rs).
- `GameState::advance_time(minutes)` — advances `GameTime` and ticks all active-spell
  durations. Already present and correct.
- `rest_party()` in [src/domain/resources.rs](../../src/domain/resources.rs) — calls
  `game_time.advance_hours(hours)` directly. Should be replaced with a call to
  `GameState::advance_time()` for consistency.
- Compass HUD in [src/game/systems/hud.rs](../../src/game/systems/hud.rs) —
  `CompassRoot` / `CompassText` at the top-right of the screen; layout space below the
  compass is currently unused.
- Map-travel event handling in
  [src/game/systems/map.rs](../../src/game/systems/map.rs) via `MapChangeMessage` and
  `handle_map_change`.
- Movement in `GameState::move_party_and_handle_events()` in
  [src/application/mod.rs](../../src/application/mod.rs).

### Identified Issues

- No time cost for each step in exploration mode.
- No time cost for map transitions (teleport, enter dungeon, town portal, etc.).
- No time cost per combat round.
- `rest_party()` bypasses `GameState::advance_time()`, so active-spell ticking after
  rest is inconsistent.
- No visual clock in the HUD.
- No `TimeOfDay` category (dawn, day, dusk, night) for branching logic.
- No mechanism for map events to be gated by the time of day.

---

## Implementation Phases

### Phase 1: Time Advancement Hooks

#### 1.1 Define Time-Cost Constants

Add a `TIME_COST_*` constant block to `src/domain/resources.rs` (alongside the
existing `REST_DURATION_HOURS`, `HP_RESTORE_RATE`, etc. constants). Define:

```rust
/// Minutes of game time consumed per tile stepped in exploration mode.
pub const TIME_COST_STEP_MINUTES: u32 = 5;

/// Minutes of game time consumed per combat round.
pub const TIME_COST_COMBAT_ROUND_MINUTES: u32 = 5;

/// Minutes of game time consumed when transitioning between maps (same-world).
pub const TIME_COST_MAP_TRANSITION_MINUTES: u32 = 30;

/// Full hours of rest required for a full heal.
pub const REST_DURATION_HOURS: u32 = 12; // already exists, leave unchanged
```

#### 1.2 Wire Time to Exploration Movement

In `GameState::move_party_and_handle_events()` in
[src/application/mod.rs](../../src/application/mod.rs), call
`self.advance_time(TIME_COST_STEP_MINUTES)` immediately after a successful step (after
the call to `move_party()` succeeds, before event resolution). This ensures every
step — whether it triggers an event or not — costs time.

#### 1.3 Wire Time to Map Transitions

In the Bevy `handle_map_change` system in
[src/game/systems/map.rs](../../src/game/systems/map.rs), send a `TimeAdvanceEvent`
(Bevy event carrying a `u32` minutes field) whenever a `MapChangeMessage` is
processed and the map actually changes. A new Bevy system
`apply_time_advance_events` in the same file (or in a new
`src/game/systems/time.rs`) reads those events and calls
`global_state.0.advance_time(minutes)`.

Alternatively, if the application-layer `GameState` is updated synchronously during
map loading (as it currently is), call `state.advance_time(TIME_COST_MAP_TRANSITION_MINUTES)`
directly in the map-loading code path.

#### 1.4 Wire Time to Combat Rounds

In `src/game/systems/combat.rs`, at the **start** of `process_combat_round` (or
equivalent), before any combatant acts, call
`global_state.0.advance_time(TIME_COST_COMBAT_ROUND_MINUTES)`. Advancing at the start
of the round means time passes even if the round ends early (flee, wipe). This keeps
combat-round advancement in the game layer where combat itself lives.

#### 1.5 Fix `rest_party()` to Use `GameState::advance_time()`

Change `rest_party()` in [src/domain/resources.rs](../../src/domain/resources.rs) so
it no longer directly calls `game_time.advance_hours(hours)`. Instead, make it take
only `party: &mut Party` and `hours: u32` and return the number of minutes consumed;
call `state.advance_time(minutes_consumed)` in the callers (the game systems and any
tests that construct a `GameState`).

If that signature change is too disruptive, keep the existing signature but update the
callers to also call `state.advance_time()` so active-spell ticking is not missed.

#### 1.6 Testing Requirements

- `test_step_advances_time` — call `move_party_and_handle_events` once on a clear
  tile; assert `game_state.time` advanced by exactly `TIME_COST_STEP_MINUTES`.
- `test_blocked_step_does_not_advance_time` — attempt to walk into a wall; assert time
  unchanged.
- `test_combat_round_advances_time` — process one combat round; assert time advanced
  by `TIME_COST_COMBAT_ROUND_MINUTES`.
- `test_map_transition_advances_time` — trigger a `Teleport` event; assert time
  advanced by `TIME_COST_MAP_TRANSITION_MINUTES`.
- `test_rest_advances_time` — rest 8 hours; assert time is exactly 8 hours ahead and
  active spells were ticked.

#### 1.7 Deliverables

- [ ] `TIME_COST_*` constants in `src/domain/resources.rs`
- [ ] Time advance on successful movement step
- [ ] Time advance on map transition
- [ ] Time advance per combat round
- [ ] `rest_party()` callers consistently use `GameState::advance_time()`
- [ ] All phase-1 tests pass

#### 1.8 Success Criteria

Each player action that should cost time does so. Time never goes backward. All four
`cargo` quality gates pass with zero warnings.

---

### Phase 2: Time-of-Day System

#### 2.1 Add `TimeOfDay` Enum

Add `TimeOfDay` to `src/domain/types.rs` alongside `GameTime`:

```rust
/// Categorised period of the day for event gating and visual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeOfDay {
    /// 05:00–07:59 — pale light, roosters crow
    Dawn,
    /// 08:00–11:59 — full daylight
    Morning,
    /// 12:00–15:59 — peak brightness
    Afternoon,
    /// 16:00–18:59 — golden light, shadows lengthen
    Dusk,
    /// 19:00–21:59 — dark but not full night
    Evening,
    /// 22:00–04:59 — pitch black without a light source
    Night,
}
```

Add `GameTime::time_of_day(&self) -> TimeOfDay` using the hour boundaries above.
Replace the coarse `is_night()` / `is_day()` booleans with methods that delegate to
`time_of_day()`:

```rust
pub fn is_night(&self) -> bool {
    matches!(self.time_of_day(), TimeOfDay::Evening | TimeOfDay::Night)
}
```

(Keep the existing `is_night()` and `is_day()` signatures for backward compatibility.)

#### 2.2 Expose Time-of-Day on `GameState`

Add a convenience helper to `GameState` in `src/application/mod.rs`:

```rust
pub fn time_of_day(&self) -> TimeOfDay {
    self.time.time_of_day()
}
```

#### 2.3 Ambient Darkness Flag

In the exploration map-rendering system (`src/game/systems/map.rs` or
`src/game/systems/rendering.rs`), read `global_state.0.time_of_day()`. When the
result is `TimeOfDay::Night`, reduce ambient light intensity (or apply a dark post-
process tint). When dawn arrives, restore the default. This is a hook for future
visual polish; the flag itself must be correct from day one.

The actual ambient-light parameters are up to the renderer; add a
`night_ambient_brightness: f32` constant (e.g. `0.25`) to
`src/game/systems/hud.rs` or a new `src/game/systems/time.rs`.

#### 2.4 Testing Requirements

- `test_time_of_day_dawn` — `GameTime::new(1, 5, 0).time_of_day() == TimeOfDay::Dawn`
- `test_time_of_day_night` — `GameTime::new(1, 22, 0).time_of_day() == TimeOfDay::Night`
- `test_is_night_delegates_to_time_of_day` — verify both `Evening` and `Night`
  return `true` for `is_night()`.
- Boundary test for each `TimeOfDay` variant transition hour.

#### 2.5 Deliverables

- [ ] `TimeOfDay` enum in `src/domain/types.rs`
- [ ] `GameTime::time_of_day()` with all six periods
- [ ] `GameState::time_of_day()` convenience helper
- [ ] Ambient-light hook in map renderer reading `time_of_day()`
- [ ] All phase-2 tests pass

#### 2.6 Success Criteria

Any system can call `game_state.time_of_day()` and get the correct period. The map
renders noticeably darker at night.

---

### Phase 3: Clock UI in the HUD

#### 3.1 Add Clock Marker Components

Add to `src/game/systems/hud.rs`:

```rust
/// Marker for the clock container (sits below the compass)
#[derive(Component)]
pub struct ClockRoot;

/// Text node displaying the current time (HH:MM)
#[derive(Component)]
pub struct ClockTimeText;

/// Text node displaying the current day ("Day N")
#[derive(Component)]
pub struct ClockDayText;
```

Add clock display constants adjacent to the compass constants:

```rust
pub const CLOCK_FONT_SIZE: f32 = 14.0;
pub const CLOCK_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);
pub const CLOCK_BORDER_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 1.0);
pub const CLOCK_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
pub const CLOCK_NIGHT_TEXT_COLOR: Color = Color::srgba(0.6, 0.6, 1.0, 1.0);
pub const CLOCK_DAY_TEXT_COLOR: Color = Color::srgba(1.0, 0.9, 0.5, 1.0);
```

#### 3.2 Spawn the Clock Widget

In `setup_hud` (or in a separate `setup_clock` startup system registered by
`HudPlugin`), spawn a vertical flex node anchored `position_type: Absolute`,
`top: Val::Px(...)`, `right: Val::Px(...)` immediately below the compass widget.
The clock contains two `Text` children: a time line (`"00:00"`) and a day line
(`"Day 1"`).

Link it to `CompassRoot` either by positioning it relative to the compass size, or by
placing both compass and clock inside a single right-panel column container.

#### 3.3 Add `update_clock` System

Add a Bevy system `update_clock` that runs in the same `Update` set as
`update_compass`, guarded by `not_in_combat`:

```rust
fn update_clock(
    global_state: Res<GlobalState>,
    mut time_query: Query<&mut Text, With<ClockTimeText>>,
    mut day_query: Query<&mut Text, With<ClockDayText>>,
) {
    let game_time = &global_state.0.time;
    for mut text in &mut time_query {
        **text = format!("{:02}:{:02}", game_time.hour, game_time.minute);
    }
    for mut text in &mut day_query {
        **text = format!("Day {}", game_time.day);
    }
}
```

Text color can optionally vary with `time_of_day()` (warmer during day, cooler at
night) using `CLOCK_DAY_TEXT_COLOR` / `CLOCK_NIGHT_TEXT_COLOR` constants.

The clock is hidden during combat, consistent with the rest of the exploration HUD.

#### 3.4 Register the System in `HudPlugin`

```rust
.add_systems(Update, update_clock.run_if(not_in_combat))
```

#### 3.5 Testing Requirements

- Visual inspection test: run the game, confirm the clock appears below the compass.
- `test_clock_format_midnight` — hour=0, minute=0 → `"00:00"`
- `test_clock_format_noon` — hour=12, minute=5 → `"12:05"`
- `test_clock_day_display` — day=42 → `"Day 42"`

#### 3.6 Deliverables

- [ ] `ClockRoot`, `ClockTimeText`, `ClockDayText` marker components
- [ ] Clock widget spawned below compass in `setup_hud`
- [ ] `update_clock` system registered in `HudPlugin`
- [ ] Clock visible and updating in-game
- [ ] All phase-3 tests pass

#### 3.7 Success Criteria

The clock is visible under the compass and displays the correct time after every
player action that advances time.

---

### Phase 4: Time-Triggered Events

#### 4.1 Add `TimeCondition` to Map Events

In `src/domain/world/types.rs`, add an optional `time_condition` field to `MapEvent`
variants that support it (primarily `Encounter`, `Sign`, `NpcDialogue`, and
`RecruitableCharacter`):

```rust
/// Optional condition that must be true for the event to fire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeCondition {
    /// Event fires only during these time-of-day periods.
    DuringPeriods(Vec<TimeOfDay>),
    /// Event fires only after this many in-game days have elapsed.
    AfterDay(u32),
    /// Event fires only before this many in-game days have elapsed.
    BeforeDay(u32),
    /// Event fires only between these hours (inclusive, 0-23).
    BetweenHours { from: u8, to: u8 },
}
```

A `TimeCondition` is `Option<TimeCondition>` on the event; `None` means always fires
(backward compatible with all existing RON data).

#### 4.2 Evaluate Time Conditions in `trigger_event`

In `src/domain/world/events.rs`, before returning the `EventResult`, check the event's
`time_condition` against the current `GameTime`. If the condition is not met, return
`EventResult::None`. This keeps the domain layer pure — the caller passes the current
`GameTime` as an argument.

Signature change to `trigger_event`:

```rust
pub fn trigger_event(
    world: &mut World,
    position: Position,
    game_time: &GameTime,
) -> Result<EventResult, EventError>
```

Update all callers of `trigger_event` to pass `&game_state.time`.

#### 4.3 Add `TimeAdvanceEvent` Bevy Event

Add a Bevy event `TimeAdvanceEvent { minutes: u32 }` to `src/game/systems/time.rs`
(new file). Systems that advance time (combat, map transitions, input handler) send
this event. A single `apply_time_advance` system reads it and calls
`global_state.0.advance_time(minutes)`. This centralises time mutation in the game
layer and avoids scattered direct mutations of `GlobalState`.

This is the correct place to hook future effects tied to time passage (e.g., food
consumption, active condition ticking, ambient light updates).

#### 4.4 Example RON Usage

Show campaign data authors how to write a time-gated event in a map file:

```ron
MapEvent::Encounter(
    name: "Night Ambush",
    monster_group: [3, 3, 4],
    time_condition: Some(DuringPeriods([Night, Evening])),
)
```

Add this example to `docs/how-to/authoring_time_events.md`.

#### 4.5 Testing Requirements

- `test_time_condition_night_fires_at_night` — set `game_time.hour = 23`; event with
  `DuringPeriods([Night])` returns a non-`None` result.
- `test_time_condition_night_skips_at_noon` — set `game_time.hour = 12`; same event
  returns `EventResult::None`.
- `test_time_condition_after_day_fires` — set `game_state.time.day = 10`; event with
  `AfterDay(5)` fires; `AfterDay(15)` does not.
- `test_time_condition_between_hours` — boundary tests at from/to edges.
- `test_no_time_condition_always_fires` — event with `time_condition: None` fires at
  any time.

#### 4.6 Deliverables

- [ ] `TimeCondition` enum in `src/domain/world/types.rs`
- [ ] `time_condition: Option<TimeCondition>` on applicable `MapEvent` variants
- [ ] `trigger_event` accepts `&GameTime` and evaluates conditions
- [ ] `TimeAdvanceEvent` Bevy event and `apply_time_advance` system in
      `src/game/systems/time.rs`
- [ ] `docs/how-to/authoring_time_events.md` with example RON snippets
- [ ] All phase-4 tests pass

#### 4.7 Success Criteria

A campaign author can write a night-only encounter or a day-only merchant that the
engine correctly gates by the current `GameTime`. The event system remains pure-
function at the domain layer.

---

## Phase 5: Campaign Builder — Starting Date/Time

Adds a **Starting Date/Time** row to the Campaign Builder's Campaign Editor
(**Gameplay** section) so campaign authors can configure the day, hour, and minute
at which a new game begins, rather than always starting at Day 1, 00:00.

---

### 5.1 Add `starting_time` to `CampaignConfig`

In [src/sdk/campaign_loader.rs](../../src/sdk/campaign_loader.rs), add a field to
`CampaignConfig`:

```rust
/// Starting game time for a new campaign (day, hour, minute).
/// Defaults to Day 1, 08:00 (morning) if not specified.
#[serde(default = "default_starting_time")]
pub starting_time: GameTime,
```

Add the default function immediately after the other `default_*` helpers:

```rust
fn default_starting_time() -> GameTime {
    GameTime::new(1, 8, 0) // Day 1, 08:00 — campaign starts in the morning
}
```

Update `Default for CampaignConfig` to include `starting_time:
default_starting_time()`.

The `#[serde(default)]` attribute ensures all existing `config.ron` files that lack
this field continue to deserialize correctly (backward compatible by design).

---

### 5.2 Wire `starting_time` into `GameState` at Campaign Load

In [src/bin/antares.rs](../../src/bin/antares.rs), in `AntaresPlugin::build()`,
after the starting-map and starting-position setup, set the game clock:

```rust
// Initialise the game clock from the campaign's configured starting time
game_state.time = self.campaign.config.starting_time.clone();
```

This ensures that when a new game starts the clock reflects the campaign author's
intent (e.g. a horror campaign can start at midnight, a cheerful campaign at dawn).

---

### 5.3 Add `starting_time` to `CampaignMetadataEditBuffer`

In [sdk/campaign_builder/src/campaign_editor.rs](../../sdk/campaign_builder/src/campaign_editor.rs),
extend `CampaignMetadataEditBuffer` with three separate integer fields (split from
`GameTime` for ergonomic drag-value editing):

```rust
/// Starting day (1-based)
pub starting_day: u32,
/// Starting hour (0–23)
pub starting_hour: u8,
/// Starting minute (0–59)
pub starting_minute: u8,
```

Update `CampaignMetadataEditBuffer::from_metadata()`:

```rust
starting_day:    m.starting_time.day,
starting_hour:   m.starting_time.hour,
starting_minute: m.starting_time.minute,
```

Update `CampaignMetadataEditBuffer::apply_to()`:

```rust
dest.starting_time = GameTime::new(
    self.starting_day.max(1),
    self.starting_hour.min(23),
    self.starting_minute.min(59),
);
```

Update `Default for CampaignMetadataEditBuffer`:

```rust
starting_day:    1,
starting_hour:   8,
starting_minute: 0,
```

---

### 5.4 Add Starting Date/Time UI Row to the Gameplay Section

In `render_ui()` inside `CampaignSection::Gameplay`, add a row immediately **after**
the **Starting Direction** row and **before** the **Starting Gold** row:

```rust
ui.label("Starting Date/Time:")
    .on_hover_text("Day, hour (0–23), and minute (0–59) at which the campaign begins");
ui.horizontal(|ui| {
    ui.label("Day");
    let mut day = self.buffer.starting_day as i32;
    if ui
        .add(egui::DragValue::new(&mut day).range(1..=9999))
        .changed()
    {
        self.buffer.starting_day = day.max(1) as u32;
        self.has_unsaved_changes = true;
        *unsaved_changes = true;
    }

    ui.label("Hour");
    let mut hour = self.buffer.starting_hour as i32;
    if ui
        .add(egui::DragValue::new(&mut hour).range(0..=23))
        .changed()
    {
        self.buffer.starting_hour = hour.clamp(0, 23) as u8;
        self.has_unsaved_changes = true;
        *unsaved_changes = true;
    }

    ui.label("Min");
    let mut minute = self.buffer.starting_minute as i32;
    if ui
        .add(egui::DragValue::new(&mut minute).range(0..=59))
        .changed()
    {
        self.buffer.starting_minute = minute.clamp(0, 59) as u8;
        self.has_unsaved_changes = true;
        *unsaved_changes = true;
    }

    // Preview the time-of-day period next to the spinners
    let preview_time = antares::domain::types::GameTime::new(
        self.buffer.starting_day.max(1),
        self.buffer.starting_hour.min(23),
        self.buffer.starting_minute.min(59),
    );
    ui.colored_label(
        egui::Color32::GRAY,
        format!("({})", period_label(preview_time.time_of_day())),
    );
});
ui.end_row();
```

Add a private helper function at the bottom of `campaign_editor.rs` (outside the
`impl` block):

```rust
/// Returns a short display label for a `TimeOfDay` period used in the
/// starting-time preview hint.
fn period_label(tod: antares::domain::types::TimeOfDay) -> &'static str {
    match tod {
        antares::domain::types::TimeOfDay::Dawn      => "Dawn",
        antares::domain::types::TimeOfDay::Morning   => "Morning",
        antares::domain::types::TimeOfDay::Afternoon => "Afternoon",
        antares::domain::types::TimeOfDay::Dusk      => "Dusk",
        antares::domain::types::TimeOfDay::Evening   => "Evening",
        antares::domain::types::TimeOfDay::Night     => "Night",
    }
}
```

Note: `period_label` depends on `TimeOfDay`, which is defined in **Phase 2** of this
plan. The Campaign Builder starting-time UI therefore depends on Phase 2 being
complete. If implemented before Phase 2, replace the preview hint with a plain
`format!("{:02}:{:02}", hour, minute)` label and add `TimeOfDay` integration in a
follow-up once Phase 2 lands.

---

### 5.5 Update `campaigns/tutorial/config.ron`

Add `starting_time: (day: 1, hour: 8, minute: 0)` inside the `config:` block so
the tutorial campaign has an explicit starting time and serves as the canonical
example for campaign authors:

```ron
controls: (
    // ... existing keys ...
),
starting_time: (day: 1, hour: 8, minute: 0),
```

---

### 5.6 Update `data/test_campaign/config.ron`

Add the same field to the test-campaign fixture so the test suite does not rely on
the `serde(default)` fallback:

```ron
starting_time: (day: 1, hour: 8, minute: 0),
```

---

### 5.7 Testing Requirements

**Domain / loader tests** (in `src/sdk/campaign_loader.rs`):

- `test_campaign_config_starting_time_default` — `CampaignConfig::default()` has
  `starting_time == GameTime::new(1, 8, 0)`.
- `test_campaign_config_starting_time_roundtrip` — serialize a `CampaignConfig` with
  `starting_time: GameTime::new(3, 22, 30)` to RON and deserialize it; assert the
  field survives the round-trip unchanged.
- `test_campaign_config_missing_starting_time_uses_default` — deserialize a RON
  string that lacks the `starting_time` key; assert it defaults to Day 1, 08:00.

**Campaign Editor buffer tests** (in `sdk/campaign_builder/src/campaign_editor.rs`):

- `test_buffer_from_metadata_copies_starting_time` — build a `CampaignMetadata`
  with `starting_time: GameTime::new(2, 20, 45)`; call `from_metadata()`; assert
  `buffer.starting_day == 2`, `starting_hour == 20`, `starting_minute == 45`.
- `test_buffer_apply_to_writes_starting_time` — set buffer fields to day=5, hour=6,
  minute=30; call `apply_to()`; assert `dest.starting_time == GameTime::new(5, 6, 30)`.
- `test_buffer_starting_time_clamps_hour` — set `starting_hour = 25`; call
  `apply_to()`; assert `dest.starting_time.hour == 23`.
- `test_buffer_starting_time_clamps_minute` — set `starting_minute = 75`; call
  `apply_to()`; assert `dest.starting_time.minute == 59`.
- `test_buffer_starting_time_clamps_day_zero` — set `starting_day = 0`; call
  `apply_to()`; assert `dest.starting_time.day == 1`.

---

### 5.8 Deliverables

- [ ] `starting_time: GameTime` field on `CampaignConfig` with `serde(default)`
- [ ] `default_starting_time()` returning `GameTime::new(1, 8, 0)`
- [ ] `AntaresPlugin::build()` sets `game_state.time` from `starting_time`
- [ ] `starting_day`, `starting_hour`, `starting_minute` fields on
      `CampaignMetadataEditBuffer`
- [ ] `from_metadata()` and `apply_to()` updated to round-trip `starting_time`
- [ ] **Starting Date/Time** row in Campaign Builder → Campaign Editor → Gameplay
- [ ] `period_label()` helper (or plain time preview if Phase 2 not yet complete)
- [ ] `campaigns/tutorial/config.ron` updated with explicit `starting_time`
- [ ] `data/test_campaign/config.ron` updated with explicit `starting_time`
- [ ] All phase-5 tests pass

### 5.9 Success Criteria

A campaign author can open the Campaign Builder, navigate to Campaign Editor →
Gameplay, and set Day 3, 22:00 as the starting time. After saving and launching the
game, the HUD clock (Phase 3) shows `22:00` and `Day 3` from the first frame of
exploration. A campaign whose `config.ron` lacks the field starts at Day 1, 08:00
without error. All four `cargo` quality gates pass with zero warnings.

---

## Decisions

1. **Step time cost** — Flat cost regardless of terrain or map type. All steps cost
   `TIME_COST_STEP_MINUTES` (5 min) whether indoors or outdoors.

2. **Combat time granularity** — Time advances once per full round, **at the start of
   the round** (before any combatant acts). Use `TIME_COST_COMBAT_ROUND_MINUTES` (5 min).

3. **Clock visibility in combat** — The clock is hidden during combat mode, consistent
   with the exploration HUD. `update_clock` runs under the existing `not_in_combat`
   run condition.
