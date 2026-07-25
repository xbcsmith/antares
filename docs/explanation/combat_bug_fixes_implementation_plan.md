# Combat Bug Fixes Implementation Plan

## Overview

Fixes four related combat bugs reported in `docs/explanation/next_plans.md`
(lines 220–230). All four root causes live in the Bevy game layer
(`src/game/systems/combat.rs`); the domain combat engine
(`src/domain/combat/engine.rs`) turn logic is correct and needs no changes.

The bugs and their root causes:

| # | Symptom | Root Cause | Location |
|---|---------|------------|----------|
| 1 | Dead monster card shows the literal word "Condition" in yellow instead of "Dead"/"Unconscious" | Placeholder string: any non-`Normal` `MonsterCondition` renders `"Condition"` | `update_combat_ui`, `src/game/systems/combat.rs:3579-3586` |
| 2 | With multiple monsters, only the first monster ever attacks; combat stalls once it cannot act | `Local<bool>` latch `was_enemy_turn` is never reset after a monster successfully acts, and `MonsterTurnTimer` is `TimerMode::Once` — a finished one-shot timer never reports `just_finished()` again, so consecutive monster turns deadlock | `execute_monster_turn`, `src/game/systems/combat.rs:6262-6281` |
| 3 | Ambush with 2+ monsters: first monster attacks, second never does; combat stuck, ESC dead, log spams `input blocked — not player turn` | Same root cause as #2 — in an ambush round every slot is a monster turn, so the deadlock always triggers after the first monster acts | Same as #2 |
| 4 | Opening the Player Screen `[p]` (or clicking a HUD portrait) during combat and closing it resets all participants' HP to combat-start values | While the character sheet is open, `mode != GameMode::Combat`, so `sync_combat_to_party_on_exit` fires and calls `combat_res.clear()`; on close, mode resumes to the **stale `CombatState` snapshot** stored inside `GameMode::Combat(..)` at encounter start, and `sync_party_to_combat` re-clones it into the emptied `CombatResource` | `sync_combat_to_party_on_exit` (`src/game/systems/combat.rs:1675-1785`), `sync_party_to_combat` (`src/game/systems/combat.rs:1529-1621`), `GameState::enter_character_sheet` (`src/application/mod.rs:2326`) |

Phases are ordered by severity: the turn-deadlock (bugs 2/3) makes combat
unplayable, the HP reset (bug 4) is an exploit/corruption bug, and the
condition label (bug 1) is cosmetic.

## Current State Analysis

### Existing Infrastructure

- **Domain engine** (`src/domain/combat/engine.rs`): `CombatState::advance_turn`
  (line 327) correctly skips incapacitated combatants and wraps rounds;
  `advance_round` (line 385) recalculates `turn_order` excluding dead
  combatants and resets monster `has_acted` flags via `Monster::reset_turn`.
  `calculate_turn_order` (line 625) includes **all** living combatants — the
  turn order itself is never wrong.
- **Monster state** (`src/domain/combat/monster.rs`): `Monster::take_damage`
  (line 440) sets `MonsterCondition::Dead` when HP reaches 0.
  `MonsterCondition` (line 117) is a 10-variant enum (`Normal`, `Paralyzed`,
  `Webbed`, `Held`, `Asleep`, `Mindless`, `Silenced`, `Blinded`, `Afraid`,
  `Dead`) with no display/name method.
- **Monster turn pacing** (`src/game/systems/combat.rs`): `MonsterTurnTimer`
  resource (line 1105) wraps a `Timer::from_seconds(MONSTER_TURN_DELAY_SECS,
  TimerMode::Once)`, inserted pre-finished by `CombatPlugin` (lines
  1136–1140). `execute_monster_turn` (line 6126) arms it on the first
  EnemyTurn frame using the `Local<bool>` `was_enemy_turn` latch. The
  incapacitated-monster skip path (line 6249) already resets the latch with an
  explanatory comment; the successful-action path does not.
- **Party/combat sync systems** (`src/game/systems/combat.rs`):
  `sync_party_to_combat` (line 1529) initialises `CombatResource` from the
  `CombatState` embedded in `GameMode::Combat(..)` only when the resource has
  no player participants; `sync_party_hp_during_combat` (line 1638) mirrors
  live HP/SP/conditions to `party.members` every combat frame;
  `sync_combat_to_party_on_exit` (line 1675) copies combat results back to the
  party and clears `CombatResource` whenever `mode != Combat` and the
  participant mapping is non-empty.
- **Combat termination**: `CombatStatus` is one of `InProgress`, `Victory`,
  `Defeat`, `Fled`. All legitimate combat exits set a terminal status before
  `GlobalState::exit_combat()` is called (flee: lines 5784, 6060; victory /
  defeat handled via `check_combat_resolution`, line 6387).
- **Character sheet overlay**: `GameState::enter_character_sheet` /
  `enter_character_sheet_at` (`src/application/mod.rs:2326`, `2371`) stash the
  current mode (including a `GameMode::Combat` value) as the resume mode and
  restore it verbatim on close. HUD portrait clicks call this during combat
  (`src/game/systems/hud.rs:886`).

### Identified Issues

1. `update_combat_ui` renders a hardcoded `"Condition"` placeholder for any
   non-`Normal` monster condition (bug 1).
2. `execute_monster_turn` never re-arms the one-shot `MonsterTurnTimer` /
   `was_enemy_turn` latch after a monster successfully acts, deadlocking
   consecutive monster turns (bugs 2 and 3).
3. `sync_combat_to_party_on_exit` cannot distinguish "combat ended" from
   "combat suspended behind a UI overlay", so it destroys live combat state
   whenever the character sheet (or any overlay) opens mid-combat (bug 4).
4. The `CombatState` snapshot embedded in `GameMode::Combat(..)` is never
   updated during combat, making it dangerous as a re-initialisation source
   (contributing cause of bug 4).
5. Existing `execute_monster_turn` tests omit the `MonsterTurnTimer` resource
   (the documented test path acts immediately), so the timer/latch interaction
   was never exercised — which is why bugs 2/3 shipped undetected.

## Implementation Phases

### Phase 1: Monster Turn Deadlock Fix (Bugs 2 & 3)

#### 1.1 Reset the EnemyTurn latch after a successful monster action

In `execute_monster_turn` (`src/game/systems/combat.rs`), immediately after the
`perform_monster_turn_with_rng` call and its follow-up logging/feedback block
(after line 6331), set `*was_enemy_turn = false`. This mirrors the existing
reset in the incapacitated-skip path (line 6249) and guarantees the next
EnemyTurn frame re-arms `MonsterTurnTimer` via `timer.0.reset()` for the next
monster, restoring the per-monster action delay instead of deadlocking.

#### 1.2 Reset the latch on the ambush player-skip path

The ambush skip branch (lines 6146–6190) returns early without touching the
latch. Add `*was_enemy_turn = false` before its `return` so that a
monster→player-skip→monster sequence during an ambush round also re-arms the
timer. (`Local<bool>` defaults to `false`, but the latch may be `true` from a
monster action earlier in the same ambush round.)

#### 1.3 Audit remaining early returns

Verify every code path in `execute_monster_turn` that transitions the current
actor leaves the latch in a state consistent with the next frame's expectation:
the stale-state correction branch (the `else if !turn_order.is_empty()` arm,
lines 6332–6346) should also reset the latch when forcing `PlayerTurn`. Note:
the two early returns at lines 6195 and 6200 (not-in-combat-mode check and
not-EnemyTurn check) already reset the latch correctly — do not modify them;
they simply never fire for the ambush branch, which is why 1.2 is needed.

#### 1.4 Testing Requirements

- New Bevy integration test: 2-monster, 1-player combat with
  `MonsterTurnTimer` **present** (zero-duration `Timer::from_seconds(0.0,
  TimerMode::Once)`); pump `app.update()` and assert both monsters act and
  `turn_state` returns to `PlayerTurn` within a bounded number of frames.
- New test: ambush encounter (`CombatEventType::Ambush`) with 2 monsters;
  assert round 1 completes (both monsters act, player slots auto-skipped) and
  round 2 begins in `PlayerTurn` with `ambush_round_active == false`.
- Regression: existing `execute_monster_turn` tests without the timer resource
  must still pass unchanged.

#### 1.5 Deliverables

- [ ] Latch reset after successful monster action in `execute_monster_turn`
- [ ] Latch reset on ambush player-skip early return
- [ ] Latch reset in stale-state correction branch
- [ ] Two-monster consecutive-turn integration test (timer present)
- [ ] Two-monster ambush round-completion integration test

#### 1.6 Success Criteria

- In a 2+ monster encounter every living, able monster acts each round with
  the configured delay between actions.
- Ambush encounters with 2+ monsters complete round 1 and hand control to the
  player in round 2; ESC and action input work normally.
- No `input blocked — not player turn` log spam after the enemy round ends.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass (full doctest run excluded per project
  convention).

### Phase 2: Player Screen HP Reset Fix (Bug 4)

#### 2.1 Gate the exit-sync on combat resolution

In `sync_combat_to_party_on_exit` (`src/game/systems/combat.rs:1675`), add an
early return when `combat_res.state.status == CombatStatus::InProgress`. The
system then only syncs-and-clears once combat has genuinely resolved
(`Victory`, `Defeat`, or `Fled`), never while combat is merely suspended
behind the character sheet or another overlay.

#### 2.2 Audit combat exit paths

Confirm every path that leaves `GameMode::Combat` permanently sets a terminal
`CombatStatus` first, so the new gate cannot strand party data:

- Flee: `perform_flee_action` sets `CombatStatus::Fled` before
  `exit_combat()` (lines 5784, 6060).
- Victory/Defeat: `check_combat_end` sets status; `check_combat_resolution`
  (line 6387) emits `CombatVictory`/`CombatDefeat`; their handlers call
  `exit_combat()` (lines 6689, 6699).
- Surrender/bribe and any menu-driven exits: verify each call site of
  `exit_combat()` in `src/game/systems/combat.rs` (lines 4575, 4772, 4859,
  4959) sets a terminal status; if any does not, set `CombatStatus::Fled`
  there as part of this phase. (Pre-audit note: all four sites are already
  gated by `status == CombatStatus::Fled` checks, so this audit is expected
  to pass without changes.)
- **Non-`exit_combat()` mode transitions**: the new gate also changes behavior
  for paths that leave `GameMode::Combat` without calling `exit_combat()` —
  quitting to the main menu, starting a new game, or loading a save
  mid-combat. With the gate, `CombatResource` would remain populated with
  `InProgress` state, and `sync_party_to_combat`'s `existing_players > 0`
  guard (line 1548) would then feed that stale state into the **next**
  encounter. Audit these paths and ensure `CombatResource::clear()` is called
  on new-game and load-game transitions (and any quit-to-menu path) if it is
  not already; add this reset as part of this phase where missing.

#### 2.3 Verify overlay round-trip preserves live state

With the gate in place, opening the character sheet mid-combat leaves
`CombatResource` untouched; on close, `sync_party_to_combat`'s existing
`existing_players > 0` early return (line 1548) preserves the live state, and
the stale snapshot inside `GameMode::Combat(..)` is never re-copied. No
changes to `enter_character_sheet` or `CharacterSheetState` are required.

#### 2.4 Testing Requirements

- New Bevy integration test: start combat, apply damage to one monster and one
  player participant in `CombatResource`, switch `GlobalState` mode to
  `CharacterSheet` (resume mode = the combat mode value), pump frames, restore
  the resume mode, pump frames; assert `CombatResource` participant HP,
  `round`, `current_turn`, and `turn_order` are unchanged.
- New unit test: `sync_combat_to_party_on_exit` is a no-op (no clear, no party
  writes) while `status == InProgress` and mode is not `Combat`.
- Regression: existing victory/defeat/flee sync tests must still pass —
  terminal-status exits still copy HP/SP/conditions/stats back to the party
  and clear the resource.

#### 2.5 Deliverables

- [ ] `InProgress` guard in `sync_combat_to_party_on_exit`
- [ ] Audit (and fix if needed) of all `exit_combat()` call sites for terminal
      status
- [ ] Audit (and fix if needed) of non-`exit_combat()` combat-mode exits
      (main menu, new game, load game) — `CombatResource::clear()` on each
- [ ] Character-sheet round-trip integration test
- [ ] No-op-while-suspended unit test

#### 2.6 Success Criteria

- Opening and closing the Player Screen (`[p]` or HUD portrait click) during
  combat leaves all participant HP, conditions, round, and turn order exactly
  as they were.
- Victory, defeat, and flee still propagate final HP/SP/conditions/stat
  currents to `party.members` and clear `CombatResource`.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass.

### Phase 3: Real Condition Names on Enemy Cards (Bug 1)

#### 3.1 Add a display name to `MonsterCondition`

Implement `std::fmt::Display` for `MonsterCondition` in
`src/domain/combat/monster.rs` mapping each variant to its player-facing
label: `Normal` → empty string, `Paralyzed` → "Paralyzed", `Webbed` →
"Webbed", `Held` → "Held", `Asleep` → "Asleep", `Mindless` → "Mindless",
`Silenced` → "Silenced", `Blinded` → "Blinded", `Afraid` → "Afraid", `Dead` →
"Dead". Include a doctest per project convention.

#### 3.2 Render the real condition in the combat UI

In `update_combat_ui` (`src/game/systems/combat.rs:3570-3589`), replace the
hardcoded `"Condition"` placeholder with the `Display` output of
`monster.conditions`. A monster at 0 HP then shows "Dead" (monsters have no
unconscious state — `Monster::take_damage` goes straight to
`MonsterCondition::Dead`; "Unconscious" applies only to player characters).

#### 3.3 Colour differentiation (decided)

Render "Dead" in a **transparent red** and all non-fatal conditions in a
**transparent yellow** — the current opaque yellow is too bright. Use the
shared condition-colour constants defined in Phase 4.2 (single source of
truth); if Phase 3 lands before Phase 4, introduce the two constants
(`CONDITION_FATAL_COLOR` ≈ `Color::srgba(0.85, 0.2, 0.2, 0.85)`,
`CONDITION_STATUS_COLOR` ≈ `Color::srgba(0.9, 0.85, 0.3, 0.85)`) in this phase
and have Phase 4 consume them.

#### 3.4 Testing Requirements

- Unit test (or doctest) covering the `Display` mapping for every
  `MonsterCondition` variant.
- Integration test: reduce a monster to 0 HP in `CombatResource`, run
  `update_combat_ui`, assert the condition `Text` for that participant equals
  "Dead" and the HP text equals "0/{base}".

#### 3.5 Deliverables

- [ ] `Display` impl for `MonsterCondition` with doctest
- [ ] `update_combat_ui` renders real condition names
- [ ] Display-mapping unit test
- [ ] Dead-monster card integration test

#### 3.6 Success Criteria

- A monster reduced to 0 HP shows `0/{base}` HP and the word "Dead" in
  transparent red on its card; paralyzed/asleep/etc. monsters show the
  matching condition name in transparent yellow.
- The word "Condition" no longer appears anywhere in combat UI output.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass.

### Phase 4: Combat UX Polish (Turn Highlighting, Action Buttons, HUD Card Colors)

Addresses the combat UX issues in `docs/explanation/next_plans.md` lines
212–214: it is hard to tell whose turn it is, the action menu does not reset
to Attack between party members' turns, and the action buttons are
near-identical shades of grey with no mouse feedback.

#### 4.1 Highlight the active character's HUD card

- Extend `CombatResource`/`update_hud` wiring so the HUD knows which party
  member is the current combatant: map
  `combat_res.state.turn_order[current_turn]` → `CombatantId::Player(idx)` →
  party slot via `combat_res.player_orig_indices`
  (`src/game/systems/combat.rs`).
- In `src/game/systems/hud.rs`, add a system (or extend `update_hud`,
  line 915) that sets the `CharacterCard` background (currently the flat
  `Color::srgba(0.2, 0.2, 0.2, 0.7)` spawned at line 586) to a new
  `ACTIVE_TURN_CARD_COLOR` — a transparent green,
  `Color::srgba(0.15, 0.45, 0.15, 0.7)` — for the active member during
  `CombatTurnState::PlayerTurn`, and restores the default otherwise.
- Define the constant alongside the existing HUD color constants in
  `src/game/systems/hud.rs`.

#### 4.2 Universal condition-based tinting

Condition tints are **universal**: they apply to every UI element representing
an entity that has a condition — party HUD cards, enemy combat cards, and any
future card/panel (see the new reference doc in 4.3).

- Define a single condition→colour mapping in one module (e.g. a
  `condition_color(...)` helper co-located with `get_priority_condition` in
  `src/game/systems/hud.rs`, or a shared `ui_helpers` location) returning
  **transparent** tints: fatal/dead → transparent red
  (`CONDITION_FATAL_COLOR`), non-fatal statuses → transparent yellow
  (`CONDITION_STATUS_COLOR`), with per-condition variants (poisoned → green
  tint, unconscious → grey tint, etc.) where `get_priority_condition` already
  distinguishes them. All alphas < 1.0 — no opaque, high-saturation colours.
- Apply the tint to party HUD card backgrounds in `update_hud`
  (`src/game/systems/hud.rs:915`) and to enemy card backgrounds in
  `update_combat_ui` / `update_target_highlight`
  (`src/game/systems/combat.rs`), replacing per-site ad-hoc colours.
- Precedence (uniform everywhere): active-turn highlight (4.1) > condition
  tint > default background. Encode this in one small pure function
  (card state → `Color`) so it is unit-testable without Bevy.

#### 4.3 Condition colour reference document

Add `docs/reference/condition_color_scheme.md` specifying, for future
features: the canonical condition→colour table (name, hex/srgba value, alpha),
the transparency rule (tints are translucent overlays, never opaque), the
precedence order (active-turn > condition > default), and the constants'
source location. Follow the existing `docs/reference/` style (cf.
`stat_ranges.md`) and include the SPDX copyright footer.

#### 4.4 Action menu reset between party members' turns

- Today `update_combat_ui` (`src/game/systems/combat.rs:3643-3648`) resets
  `ActionMenuState::active_index` to 0 (Attack) only on the menu's
  Hidden→Visible transition, so when two players act back-to-back the previous
  selection persists.
- Track the last player combatant index (new field on `ActionMenuState` or a
  `Local` in `update_combat_ui`); when the current player combatant changes —
  even without a visibility transition — reset `active_index = 0` and
  `confirmed = false`.

#### 4.5 Action button colors and mouse feedback

- Replace the near-grey palette (`src/game/systems/combat.rs:225-234`) with
  clearly distinct states using the Spell Panel scheme, whose highlight color
  is `ACTION_BUTTON_CONFIRMED_COLOR` (gold `0.65, 0.55, 0.25`, shared by
  `update_spell_focus_highlight`, line 2644):
  - selected/active: the gold scheme (visibly brighter than idle);
  - hover: a lightened variant;
  - pressed: a distinct pressed shade;
  - idle: keep a neutral dark base; disabled: keep
    `ACTION_BUTTON_DISABLED_COLOR`.
- `update_action_highlight` (`src/game/systems/combat.rs:4307-4335`) currently
  colors buttons from keyboard state only. Extend it (or add a companion
  system) to also read the `Interaction` component on `ActionButton` entities
  (the query pattern already exists at line 1862) so mouse hover and press
  produce visible feedback, with keyboard selection and mouse hover composing
  sensibly (selected state wins; hover modulates non-selected buttons).

#### 4.6 Testing Requirements

- Unit test for the card-color precedence function: active turn > condition
  tint > default; dead/poisoned/unconscious map to their expected tints; all
  condition tints have alpha < 1.0.
- Integration test: two-player combat; after player 1 acts, assert
  `ActionMenuState::active_index == 0` when player 2's turn begins without the
  menu ever hiding.
- Integration test: during `PlayerTurn`, assert the active member's
  `CharacterCard` background equals `ACTIVE_TURN_CARD_COLOR` and other cards
  keep their default/condition color; assert it reverts on `EnemyTurn`.
- Color-constant sanity tests (project pattern, cf. the existing
  `SPELL_PANEL_*` layout tests): selected, hover, idle, and disabled action
  button colors are pairwise distinct.

#### 4.7 Deliverables

- [ ] Active-turn transparent green highlight on the current character's HUD
      card
- [ ] Universal condition tinting (party HUD cards and enemy cards) with
      documented precedence and shared colour constants
- [ ] `docs/reference/condition_color_scheme.md` reference document
- [ ] Action menu resets to Attack whenever the acting party member changes
- [ ] Action button palette aligned with the Spell Panel gold scheme
- [ ] Mouse hover/press feedback on action buttons
- [ ] Unit + integration tests listed in 4.6

#### 4.8 Success Criteria

- At any moment in combat a player can identify whose turn it is from the HUD
  alone (green card behind the active portrait).
- HUD cards and enemy cards visibly reflect the entity's worst condition
  (translucent tint) whenever the card is not the active-turn card; the same
  colour constants drive both, and
  `docs/reference/condition_color_scheme.md` documents the scheme.
- Each party member's turn starts with Attack highlighted.
- Hovering or clicking an action button with the mouse produces an obvious
  color change; the selected button uses the Spell Panel gold scheme.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass.

## Copyright

SPDX-License-Identifier: Apache-2.0

This document follows the [SPDX Spec](https://spdx.github.io/spdx-spec/) for
copyright and licensing information.
