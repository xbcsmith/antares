# Rest System Implementation Plan

## Overview

Antares has foundational rest infrastructure (`rest_party()` in
`src/domain/resources.rs`) but it is not wired to any input, has no UI, does not
check for random encounters during rest, and uses the wrong rate constants (8-hour
full heal instead of the required 12-hour). This plan adds a complete, configurable
party rest system bound to the `R` key: the player presses `R` in exploration mode,
the party rests one hour at a time, HP/SP are healed proportionally, time advances,
food is consumed, and each rest-hour has a chance to trigger a random encounter that
interrupts sleep.

---

## Current State Analysis

### Existing Infrastructure

- `rest_party()` in [src/domain/resources.rs](../../src/domain/resources.rs) — full
  rest logic: HP/SP restoration, food consumption, direct `game_time.advance_hours()`.
- `HP_RESTORE_RATE = 0.125` and `REST_DURATION_HOURS = 8` in the same file — wrong
  for the 12-hour requirement.
- `random_encounter()` in
  [src/domain/world/events.rs](../../src/domain/world/events.rs) — accepts `&World`
  and `rng`, returns `Option<Vec<u8>>` (a monster group). Reusable as-is.
- `GameState::advance_time(minutes)` in
  [src/application/mod.rs](../../src/application/mod.rs) — correctly ticks
  active-spell durations; `rest_party()` bypasses it by calling
  `game_time.advance_hours()` directly.
- `ControlsConfig` in [src/sdk/game_config.rs](../../src/sdk/game_config.rs) — has
  `move_forward`, `move_back`, `interact`, `menu`, `inventory`; **no `rest` key**.
- `GameAction` enum in
  [src/game/systems/input.rs](../../src/game/systems/input.rs) — has `MoveForward`,
  `MoveBack`, `TurnLeft`, `TurnRight`, `Interact`, `Menu`, `Inventory`; **no `Rest`
  variant**.
- `handle_input` system in
  [src/game/systems/input.rs](../../src/game/systems/input.rs) — handles all current
  actions; rest key not yet present.
- `GameMode` enum in [src/application/mod.rs](../../src/application/mod.rs) —
  `Exploration`, `Combat`, `Inventory`, `InnManagement`, `Menu`, `Dialogue`; **no
  `Resting` variant needed** (rest is processed as a fast-forward sequence in
  exploration mode, interrupted by encounter).

### Identified Issues

- `HP_RESTORE_RATE` and `REST_DURATION_HOURS` reflect 8-hour full heal; plan requires
  12-hour full heal.
- `rest_party()` calls `game_time.advance_hours()` directly, bypassing
  `GameState::advance_time()` and preventing active-spell ticking during rest.
- No per-hour random-encounter check during rest.
- No `rest` field in `ControlsConfig`; no `Rest` action in `GameAction`; `KeyMap`
  does not register an `R` binding.
- `handle_input` has no branch that triggers rest.
- No `RestState` to track progress across a multi-hour rest sequence.
- No HUD feedback during rest (progress text, interruption notice).
- No `RestConfig` block in `GameConfig` for tuning rest duration and encounter
  probability.

---

## Implementation Phases

### Phase 1: Core Domain Corrections

Fixes the existing `rest_party()` domain function to match the 12-hour requirement,
removes the time-advancement side-effect, and adds a per-hour encounter-capable rest
helper.

#### 1.1 Update Rest Constants

In [src/domain/resources.rs](../../src/domain/resources.rs), update:

| Constant | Old | New | Reason |
|---|---|---|---|
| `REST_DURATION_HOURS` | `8` | `12` | Full heal requires 12 hours per spec |
| `HP_RESTORE_RATE` | `0.125` (1/8) | `0.0833` (1/12) | Rate must match 12-hour full heal |
| `SP_RESTORE_RATE` | `0.125` (1/8) | `0.0833` (1/12) | Same |

Add a new constant:

```
/// Clamp fraction: HP/SP per-hour restore is 1 / REST_DURATION_HOURS
pub const HP_RESTORE_RATE: f32 = 1.0 / REST_DURATION_HOURS as f32;
pub const SP_RESTORE_RATE: f32 = 1.0 / REST_DURATION_HOURS as f32;
```

#### 1.2 Fix `rest_party()` Time Advancement

Remove the `game_time.advance_hours(hours)` call at the end of `rest_party()`.
Change the function signature to accept `party: &mut Party` and `hours: u32` only
(drop the `game_time` parameter). The caller (`GameState`) will call
`self.advance_time(hours * 60)` after `rest_party()` returns, ensuring
active-spell durations are ticked correctly.

Update the doc-comment example accordingly.

#### 1.3 Add `rest_party_hour()` Helper

Add a new function `rest_party_hour(party: &mut Party) -> Result<(), ResourceError>`
that restores one hour of HP/SP (using `HP_RESTORE_RATE`/`SP_RESTORE_RATE`) and
consumes a proportional food fraction. This is used by the per-hour rest loop in
Phase 2 so the caller can interleave encounter checks between each hour without
re-entering `rest_party()`.

No time advancement inside this function; callers own time via
`GameState::advance_time(60)`.

#### 1.4 Add `RestError` Variants

Extend `ResourceError` with:

- `CannotRestWithActiveEncounter` — rest interrupted by incoming encounter.
- `RestInterrupted { hours_completed: u32 }` — rest ended early (encounter or
  explicit cancel).

#### 1.5 Testing Requirements

- `test_full_rest_heals_in_12_hours` — `rest_party_hour()` × 12; assert all members
  are at full HP/SP.
- `test_partial_rest_heals_proportionally` — rest 6 hours; assert HP ≈ 50% of max.
- `test_rest_party_no_longer_advances_time` — call `rest_party()`; assert
  `game_time` unchanged (time control is now caller responsibility).
- `test_rest_consumes_food` — food count decreases after `rest_party()`.
- `test_rest_party_fails_without_food` — `party.food = 0`; assert
  `ResourceError::TooHungryToRest`.
- `test_rest_skips_dead_characters` — character with `conditions.is_fatal()` keeps
  `hp.current = 0` after rest.

#### 1.6 Deliverables

- [ ] `REST_DURATION_HOURS = 12` in `src/domain/resources.rs`
- [ ] `HP_RESTORE_RATE` and `SP_RESTORE_RATE` derived from `1.0 / REST_DURATION_HOURS as f32`
- [ ] `rest_party()` no longer takes `game_time` parameter
- [ ] `rest_party_hour()` added and tested
- [ ] `ResourceError::RestInterrupted` and `CannotRestWithActiveEncounter` variants
- [ ] All phase-1 tests pass

#### 1.7 Success Criteria

`rest_party_hour()` × 12 restores all party members to full HP/SP. The time
advancement removed from `rest_party()` is confirmed by tests. All  `cargo
clippy` gates pass with zero warnings.

---

### Phase 2: Rest Configuration and Input Binding

Adds the `rest` key binding to `ControlsConfig`, registers it in `KeyMap`, and wires
the `R` key in `handle_input`.

#### 2.1 Add `rest` to `ControlsConfig`

In [src/sdk/game_config.rs](../../src/sdk/game_config.rs), add to `ControlsConfig`:

```rust
/// Keys for party rest
#[serde(default = "default_rest_keys")]
pub rest: Vec<String>,
```

```rust
fn default_rest_keys() -> Vec<String> {
    vec!["R".to_string()]
}
```

Update `Default for ControlsConfig` to include `rest: default_rest_keys()`.

Update `ControlsConfig::validate()` to check that `rest` is non-empty.

#### 2.2 Add `GameAction::Rest`

In [src/game/systems/input.rs](../../src/game/systems/input.rs), add:

```rust
/// Begin a party rest sequence
Rest,
```

to the `GameAction` enum.

#### 2.3 Wire `Rest` in `KeyMap`

In `KeyMap::from_controls_config()`, add a loop that reads `config.rest` and inserts
each parsed `KeyCode → GameAction::Rest` into `bindings`, following the same pattern
as `inventory`.

#### 2.4 Add Rest Handler in `handle_input`

After the `Inventory` toggle block and before the movement-cooldown check, add a
block that:

1. Guards on `GameAction::Rest` just-pressed.
2. Only acts when `game_state.mode == GameMode::Exploration` (no rest during Combat,
   Menu, Dialogue, InnManagement, Inventory).
3. Writes a new Bevy event `InitiateRestEvent` (defined in Phase 3) to the event
   bus; the per-frame rest orchestration system will read it.
4. Returns early from `handle_input`.

#### 2.5 Update `config.template.ron`

In [campaigns/config.template.ron](../../campaigns/config.template.ron), add
`rest: ["R"]` inside the `controls:` block so campaign authors can see it.

#### 2.6 Testing Requirements

- `test_controls_config_rest_default` — `ControlsConfig::default().rest ==
  vec!["R"]`.
- `test_key_map_rest_action` — default `KeyMap` maps `KeyCode::KeyR` to
  `GameAction::Rest`.
- `test_controls_config_validates_empty_rest_list` — empty `rest` vec fails
  `validate()`.
- `test_custom_rest_key` — set `rest: ["F5"]`; `KeyMap` maps `F5 → Rest`.

#### 2.7 Deliverables

- [ ] `rest: Vec<String>` field in `ControlsConfig` with `serde(default)`
- [ ] `default_rest_keys()` returning `["R"]`
- [ ] `GameAction::Rest` variant
- [ ] `KeyMap` wires `rest` keys to `GameAction::Rest`
- [ ] `handle_input` writes `InitiateRestEvent` on `R` press in Exploration mode
- [ ] `config.template.ron` updated
- [ ] All phase-2 tests pass

#### 2.8 Success Criteria

Pressing `R` during exploration fires `InitiateRestEvent`. Pressing `R` during
Combat, Menu, Dialogue, or InnManagement does nothing. `ControlsConfig` round-trips
correctly through RON serialization.

---

### Phase 3: Rest Orchestration System

Adds the per-hour rest loop with encounter interruption and time/food accounting.

#### 3.1 Define `RestState`

Add `RestState` to [src/application/mod.rs](../../src/application/mod.rs):

```rust
/// Tracks progress of an in-progress party rest sequence
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestState {
    /// Total hours of rest requested (e.g. 12 for a full rest)
    pub hours_requested: u32,
    /// Hours of rest completed so far
    pub hours_completed: u32,
    /// Set when an encounter interrupts the rest
    pub interrupted: bool,
}
```

Add `GameState::enter_rest(hours: u32)` that sets `self.mode =
GameMode::Resting(RestState::new(hours))`.

Add `GameMode::Resting(RestState)` variant to the `GameMode` enum. This mode blocks
all movement/interaction input (handled in `handle_input` via the existing mode
guard), which ensures the rest system runs uninterrupted until an encounter or
completion.

#### 3.2 Define `InitiateRestEvent` and `RestCompleteEvent`

In a new file [src/game/systems/rest.rs](../../src/game/systems/rest.rs):

```rust
/// Sent by the input handler to begin a rest sequence
#[derive(Event)]
pub struct InitiateRestEvent {
    /// Hours to rest (default REST_DURATION_HOURS = 12)
    pub hours: u32,
}

/// Sent when a rest sequence ends (completed or interrupted)
#[derive(Event)]
pub struct RestCompleteEvent {
    pub hours_completed: u32,
    pub interrupted_by_encounter: bool,
    /// Monster group that interrupted rest (if any)
    pub encounter_group: Option<Vec<u8>>,
}
```

#### 3.3 Add `RestPlugin` and `process_rest` System

In `src/game/systems/rest.rs`, implement a `RestPlugin`:

- Registers `InitiateRestEvent` and `RestCompleteEvent` as Bevy events.
- Adds startup system `register_rest_systems`.
- Adds `Update` system `process_rest` that:
  1. If `InitiateRestEvent` is received and mode is `Exploration`, call
     `game_state.enter_rest(hours)`.
  2. If mode is `Resting(rest_state)`:
     - If `rest_state.hours_completed < rest_state.hours_requested`:
       - Call `rest_party_hour(&mut game_state.party)`.
       - Call `game_state.advance_time(60)` (one hour).
       - Roll `random_encounter(&game_state.world, &mut rng)`.
       - If encounter: set `rest_state.interrupted = true`, write
         `RestCompleteEvent { interrupted_by_encounter: true, encounter_group: Some(group) }`,
         and return to `GameMode::Exploration` (combat is initiated separately by a
         `RestCompleteEvent` handler in the encounter system).
     - Else (all hours completed): write `RestCompleteEvent { interrupted:
       false, encounter_group: None }` and set `game_state.mode =
       GameMode::Exploration`.

The system advances **one hour per Bevy frame** to keep the rest fast without being
instant (the HUD will show each tick — see Phase 4).

> **Design note**: One hour per frame at 60 fps means a full 12-hour rest completes
> in 12 frames (~0.2 s). This is intentional "fast-forward" behaviour. A future
> enhancement could add a configurable fast-forward speed.

#### 3.4 Handle `RestCompleteEvent` for Encounter Interruption

Add a second `Update` system `handle_rest_complete` that reads `RestCompleteEvent`.
When `interrupted_by_encounter == true` and `encounter_group` is `Some`, build a
`CombatState` via `initialize_combat_from_group()` (the same pattern used in
`move_party_and_handle_events`) and call `game_state.enter_combat_with_state(cs)`.
This reuses the existing combat initialization path without duplication.

#### 3.5 Update `handle_input` Mode Guard

Add `GameMode::Resting(_)` to the early-return guard that blocks movement and
interaction, so the player cannot walk away mid-rest:

```rust
if matches!(
    game_state.mode,
    GameMode::Menu(_) | GameMode::Inventory(_) | GameMode::Resting(_)
) {
    return;
}
```

`R` pressed during `Resting` mode is also a no-op (same guard).

#### 3.6 Register `RestPlugin` in the Game Application

In the Bevy `App::new()` setup (wherever `InputPlugin`, `HudPlugin`, etc. are added),
add `.add_plugins(RestPlugin)`.

#### 3.7 Testing Requirements

- `test_initiate_rest_enters_resting_mode` — send `InitiateRestEvent { hours: 12 }`;
  assert `game_state.mode == GameMode::Resting(_)`.
- `test_rest_completes_after_requested_hours` — process 12 `process_rest` ticks;
  assert mode returns to `Exploration` and all members at full HP.
- `test_rest_encounter_interrupts` — seed RNG to guarantee encounter on hour 3;
  assert mode transitions to `Combat` after 3 `process_rest` ticks.
- `test_rest_advances_time_per_hour` — process 6 ticks; assert
  `game_state.time` advanced by exactly 360 minutes.
- `test_rest_blocked_in_combat_mode` — mode is `Combat`; send
  `InitiateRestEvent`; assert mode remains `Combat`.
- `test_movement_blocked_during_rest` — mode is `Resting`; simulate movement
  key; assert party position unchanged.

#### 3.8 Deliverables

- [ ] `RestState` struct in `src/application/mod.rs`
- [ ] `GameMode::Resting(RestState)` variant
- [ ] `GameState::enter_rest(hours)` method
- [ ] `src/game/systems/rest.rs` with `InitiateRestEvent`, `RestCompleteEvent`,
  `process_rest`, `handle_rest_complete`, and `RestPlugin`
- [ ] `RestPlugin` registered in the game application
- [ ] `handle_input` mode guard updated to block input during `Resting`
- [ ] All phase-3 tests pass

#### 3.9 Success Criteria

Pressing `R` in exploration starts a rest sequence. Each Bevy tick advances one
rest-hour, heals proportionally, and checks for a random encounter. A successful
encounter terminates rest and starts combat. A completed 12-hour rest returns the
party to full HP/SP and exploration mode.

---

### Phase 4: Rest UI Feedback

Adds a visible rest-progress overlay and interruption/completion notices to the HUD.

#### 4.1 Add Rest Progress Overlay

In [src/game/systems/hud.rs](../../src/game/systems/hud.rs) (or a dedicated
`setup_rest_ui` in `src/game/systems/rest.rs`), spawn a centered overlay node with
marker component `RestProgressRoot` that:

- Is `display: None` by default.
- Becomes `display: Flex` when `game_state.mode == GameMode::Resting(_)`.
- Contains:
  - Title text: `"Resting…"`
  - Progress text: `"Hour X / 12"` (updated each frame).
  - Flavour text cycling through rest-atmosphere messages (e.g. `"The party settles
    in for the night."`, `"Distant sounds echo in the dark."`).
  - Hint text: `"(encounter may interrupt)"`.

#### 4.2 Add `update_rest_ui` System

Add a Bevy `Update` system `update_rest_ui` that:

1. Reads `global_state.0.mode`.
2. If `Resting(rest_state)`: show `RestProgressRoot`, update progress label to
   `format!("Hour {} / {}", rest_state.hours_completed + 1, rest_state.hours_requested)`.
3. Otherwise: hide `RestProgressRoot`.

Register it in `HudPlugin` (or `RestPlugin`).

#### 4.3 Rest Notification Messages

When `RestCompleteEvent` fires, write a game-notification message visible in the
existing log/notification layer:

- Completion: `"The party rests for the night and awakens refreshed."` (or
  equivalent message surface already used by combat/dialogue systems).
- Interruption: `"Rest interrupted! Enemies attack!"`.

Check what notification surface already exists (e.g. `CombatLogEntry`, HUD status
text) and reuse it rather than creating a parallel system.

#### 4.4 Rest Config Block (Optional Extension)

Optionally add a `RestConfig` block to `GameConfig` (or `ControlsConfig`) for
campaign-level tuning:

```rust
#[serde(default)]
pub rest: RestConfig,
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RestConfig {
    /// Full-rest duration in hours (default: 12)
    pub full_rest_hours: u32,
    /// Encounter check probability multiplier during rest (0.0 = never, 1.0 = normal rate)
    pub rest_encounter_rate_multiplier: f32,
    /// Allow partial rest (press R again to rest for fewer hours) — not yet implemented
    pub allow_partial_rest: bool,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            full_rest_hours: 12,
            rest_encounter_rate_multiplier: 1.0,
            allow_partial_rest: false,
        }
    }
}
```

`process_rest` reads `rest_encounter_rate_multiplier` and scales the encounter roll
accordingly (multiplied into `encounter_rate` before the RNG check). This allows
town-area maps to reduce or eliminate rest encounters by setting
`rest_encounter_rate_multiplier: 0.0` in `config.ron`.

#### 4.5 Testing Requirements

- `test_rest_ui_shows_during_resting_mode` — mode transitions to `Resting`; assert
  `RestProgressRoot` `display == Flex`.
- `test_rest_ui_hides_after_completion` — rest completes; assert `display == None`.
- `test_rest_completion_message_emitted` — `RestCompleteEvent` with
  `interrupted: false` fires; message surface contains appropriate text.
- `test_rest_interrupt_message_emitted` — `RestCompleteEvent` with
  `interrupted: true`; message contains encounter-interrupt text.
- `test_rest_config_zero_multiplier_prevents_encounters` — set
  `rest_encounter_rate_multiplier: 0.0`; run 100 hours of rest ticks; assert
  `RestCompleteEvent.interrupted == false` always.

#### 4.6 Deliverables

- [ ] `RestProgressRoot` marker component and overlay node
- [ ] `update_rest_ui` system updating progress label each frame
- [ ] Overlay shown/hidden based on `GameMode::Resting`
- [ ] Completion and interruption notification messages
- [ ] `RestConfig` struct (with `rest_encounter_rate_multiplier`)
- [ ] `process_rest` reads `rest_encounter_rate_multiplier`
- [ ] `config.template.ron` updated with `rest:` block example
- [ ] All phase-4 tests pass

#### 4.7 Success Criteria

The player sees a clear rest progress overlay when resting. The overlay updates each
frame. An encounter-interrupted rest immediately exits the overlay and begins combat.
Campaign authors can disable rest encounters by setting the multiplier to `0.0`.

---

## Decisions

1. **12-hour full heal** — `HP_RESTORE_RATE = 1.0 / 12.0`. All constants are derived
   from `REST_DURATION_HOURS` so changing the constant is sufficient.

2. **One hour per Bevy frame** — fast-forward model. The rest completes in ~0.2 s at
   60 fps on a standard system. No "wait for real time" tick is introduced.

3. **Encounter per hour** — the existing `random_encounter()` is called once per
   rest-hour tick using the current map's encounter table and terrain modifiers.
   `rest_encounter_rate_multiplier` in `RestConfig` scales the chance.

4. **No separate `Resting` game mode required for serialisation** — `RestState` is
   stored in `GameMode::Resting(RestState)` which is already `Serialize`/`Deserialize`.
   A save made mid-rest will resume the rest sequence correctly on load.

5. **No partial-rest UI in Phase 4** — `allow_partial_rest: false` by default.
   Partial-rest support (choose hours) is deferred to a future plan.

6. **`rest_party()` signature break** — removing the `game_time` parameter is an
   intentional backwards-incompatible change (AGENTS.md: "WE DO NOT CARE ABOUT
   BACKWARDS COMPATIBILITY RIGHT NOW"). All callers must be updated in Phase 1.
