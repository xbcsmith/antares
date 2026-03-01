# Combat System Improvement Implementation Plan

## Overview

This plan upgrades combat input handling, action flow, and visual feedback so
that combat is readable and controllable from both keyboard and mouse. The
approach is phased to preserve existing encounter and combat-start logic while
improving action selection, target selection, and on-screen results. The primary
goals are: predictable `Tab`/`Enter` keyboard flow, reliable mouse activation,
complete action wiring for Cast and Item actions (both routes — spell/item
submenu panels already have marker components), and visible per-action outcomes
anchored to the correct combatant.

**Required control behaviour enforced by this plan:**

| # | Requirement |
|---|-------------|
| 1 | `A`, `D`, `F` are removed as combat action shortcuts because they are also movement/turn keys in `input.rs` (`GameAction::TurnLeft`, `GameAction::TurnRight`). |
| 2 | When the action menu appears, the `Attack` button is highlighted by default (index 0). |
| 3 | `Tab` cycles the active-index highlight forward through the five action buttons; wraps from index 4 back to 0. |
| 4 | `Enter` dispatches the action bound to the currently active index using the same code path as a mouse click. |
| 5 | Movement/rotation input during `GameMode::Combat(_)` is silently ignored at the `handle_input` system level in `src/game/systems/input.rs` — it never consumes a party action or advances turn order. |
| 6 | Enemy HP bars (both UI-panel cards and in-world hover bars above each monster) are visible immediately when combat starts. |

---

## Current State Analysis

### Existing Infrastructure

| Symbol | File | Purpose |
|--------|------|---------|
| `start_encounter` (line 442) | `src/game/systems/combat.rs` | Initialises `CombatState`, wires `GameMode::Combat`. |
| `setup_combat_ui` | `src/game/systems/combat.rs` | Spawns `CombatHudRoot`, enemy cards, turn order panel, `ActionMenuPanel` with five `ActionButton` nodes. |
| `cleanup_combat_ui` (line 860) | `src/game/systems/combat.rs` | Despawns HUD when `GameMode` leaves `Combat`. |
| `update_combat_ui` (line 884) | `src/game/systems/combat.rs` | Updates enemy HP bars/text/conditions label and action menu visibility per turn state. |
| `ActionButtonType` enum (line 275) | `src/game/systems/combat.rs` | Five variants: `Attack`, `Defend`, `Cast`, `Item`, `Flee`. |
| `ActionMenuPanel` (line 271) | `src/game/systems/combat.rs` | Marker component for the action menu UI node. |
| `EnemyCard`, `EnemyHpBarFill`, `EnemyHpText` (lines 228–245) | `src/game/systems/combat.rs` | Existing combat UI card components, updated each frame by `update_combat_ui`. |
| `combat_input_system` (line 1036) | `src/game/systems/combat.rs` | Current player input handler — reads `Interaction != None` for mouse and `KeyCode::KeyA/D/F/Escape` for keyboard. |
| `select_target` (line 1125) | `src/game/systems/combat.rs` | Mouse-only enemy-card click handler; sets `TargetSelection`, writes `AttackAction`. |
| `TargetSelection` resource (line 356) | `src/game/systems/combat.rs` | `Option<CombatantId>` indicating attacker in target-select mode. |
| `perform_attack_action_with_rng` (line 1150) | `src/game/systems/combat.rs` | Deterministic attack resolver; advances turn and updates `CombatTurnStateResource`. |
| `perform_cast_action_with_rng` (line 1272) | `src/game/systems/combat.rs` | Deterministic spell caster; SP validation done in domain layer. |
| `perform_use_item_action_with_rng` (line 1339) | `src/game/systems/combat.rs` | Deterministic item user; charge and restriction validation in domain layer. |
| `handle_cast_spell_action` (line 1779) | `src/game/systems/combat.rs` | System wrapper that reads `CastSpellAction` messages. |
| `handle_use_item_action` (line 1517) | `src/game/systems/combat.rs` | System wrapper that reads `UseItemAction` messages. |
| `SpellSelectionPanel` (line 295), `SpellButton` (line 304) | `src/game/systems/combat.rs` | Marker components for spell submenu — declared but UI spawn not registered. |
| `ItemSelectionPanel` (line 317), `ItemButton` (line 326) | `src/game/systems/combat.rs` | Marker components for item submenu — declared but UI spawn not registered. |
| `CombatTurnState` enum (line 119) | `src/game/systems/combat.rs` | `PlayerTurn`, `EnemyTurn`, `Animating`, `RoundEnd`. |
| `CombatTurnStateResource` (line 128) | `src/game/systems/combat.rs` | Wraps `CombatTurnState`; read by UI systems to gate visibility. |
| `FloatingDamage` (line 343), `DamageText` (line 350) | `src/game/systems/combat.rs` | Spawned on hits; currently placed at absolute position with no combatant anchor. |
| `spawn_turn_indicator`, `update_turn_indicator` | `src/game/systems/combat_visual.rs` | Manage `TurnIndicator` child node on the active actor's UI element. |
| `hide_indicator_during_animation` (line 271) | `src/game/systems/combat_visual.rs` | Hides `TurnIndicator` while `CombatTurnState::Animating`. |
| `handle_input` (line 405) | `src/game/systems/input.rs` | Movement/interaction system; has `GameMode::Menu` guard (line 463) but **no `GameMode::Combat` guard**. |
| `GameAction` enum (line 104) | `src/game/systems/input.rs` | `MoveForward`, `MoveBack`, `TurnLeft`, `TurnRight`, `Interact`, `Menu`. `A`/`D` default-bound to `TurnLeft`/`TurnRight`. |

### Identified Issues

| # | Issue | Location |
|---|-------|---------|
| I-1 | `A`/`D`/`F` are wired as combat shortcuts (lines 1089–1105 in `combat.rs`) AND are the configured movement/turn keys in `input.rs` default `ControlsConfig`. Pressing `A` during combat triggers both rotation via `handle_input` and `target_sel` activation via `combat_input_system`. | `src/game/systems/combat.rs` lines 1089–1105; `src/game/systems/input.rs` lines 800–815. |
| I-2 | `handle_input` has no `GameMode::Combat(_)` guard; movement input during combat passes through, moves the party, advances the `last_move_time` cooldown, and can trigger `MapEvent::Encounter` on new tiles. | `src/game/systems/input.rs` lines 455–645. |
| I-3 | `combat_input_system` uses `Interaction != None` (line 1062) rather than `Interaction::Pressed`, so hover alone can trigger an action. | `src/game/systems/combat.rs` line 1062. |
| I-4 | No unified action dispatch path; mouse and keyboard reach different code paths and have non-identical semantics. | `src/game/systems/combat.rs` lines 1061–1106. |
| I-5 | No `Tab`/`Enter` keyboard action traversal; no default `Attack` highlight on menu open. | `src/game/systems/combat.rs` lines 1087–1106. |
| I-6 | No action-selection-index resource; no `ActionButton` highlight component. | `src/game/systems/combat.rs` — absent. |
| I-7 | No blocked-turn feedback; when `CombatTurnState` is not `PlayerTurn`, input is silently dropped. | `src/game/systems/combat.rs` lines 1051–1053. |
| I-8 | No keyboard target cycling; `select_target` is mouse-only (`EnemyCard` click). `Tab`/`Enter` during target selection are unhandled. | `src/game/systems/combat.rs` lines 1125–1146. |
| I-9 | `ActionButtonType::Cast | ActionButtonType::Item` branch in `combat_input_system` is a no-op comment (line 1079). `SpellSelectionPanel` and `ItemSelectionPanel` marker components exist but no spawn system is registered. | `src/game/systems/combat.rs` lines 1079–1081, 295–331. |
| I-10 | `FloatingDamage` nodes are spawned at `PositionType::Absolute` with no anchor to the target's UI card; numbers float in an undefined position. | `src/game/systems/combat.rs` lines 1472–1492, 1601–1633, 1661–1663, 1860–1875. |
| I-11 | Monster-turn actions (AI) do not emit `FloatingDamage`; only player actions do. | `src/game/systems/combat.rs` — `execute_monster_turn` absent from audit lines. |
| I-12 | `CombatTurnState::Animating` and `RoundEnd` are declared (line 119) but never set at runtime; `hide_indicator_during_animation` always sees a non-`Animating` state. | `src/game/systems/combat.rs` — no `turn_state.0 = CombatTurnState::Animating` write anywhere; `combat_visual.rs` line 275. |
| I-13 | No in-world monster HP hover bars; only UI card bars are updated. | `src/game/systems/combat.rs` — absent. |
| I-14 | No cast/item failure feedback (insufficient SP, empty slot) shown to the player; failures are silent `Ok(())` returns. | `src/game/systems/combat.rs` lines 1306–1310, 1372–1374. |
| I-15 | No documentation updates for combat controls in player-facing docs. | `docs/how-to/using_game_menu.md` line 150; no combat controls how-to guide exists. |

---

## Implementation Phases

---

### Phase 1: Input Reliability and Action Selection

**Goal:** Fix movement key leakage, implement `Tab`/`Enter` keyboard navigation,
default `Attack` highlight, and unified mouse/keyboard dispatch. All five action
types route through one function.

#### 1.1 Foundation Work

**File: `src/game/systems/input.rs`**

Add a `GameMode::Combat(_)` guard inside `handle_input` (after the existing
`GameMode::Menu` guard at line 463). The guard must return early **before** any
movement or interaction processing. The current `GameMode::Menu` block is at
lines 461–465. Insert immediately after it:

```text
// Block all movement/interaction input when in Combat mode.
// Combat action input is handled exclusively by combat_input_system.
if matches!(game_state.mode, crate::application::GameMode::Combat(_)) {
    return;
}
```

This fix ensures `A`/`D` never reach the `TurnLeft`/`TurnRight` branches in
`handle_input` while the player is in combat.

**File: `src/game/systems/combat.rs`**

Introduce two new resources immediately after the `TargetSelection` resource
declaration (around line 356):

1. `ActionMenuState` resource:
   - Field `active_index: usize` — index (0–4) of the currently highlighted
     action button. Order is `[Attack, Defend, Cast, Item, Flee]` matching the
     spawn order at lines 824–830.
   - Field `confirmed: bool` — set to `true` when `Enter` or a `Pressed`
     mouse event fires; consumed by the unified dispatch function.
   - Default: `active_index = 0`, `confirmed = false`.

2. Register `ActionMenuState` in `CombatPlugin::build` alongside `TargetSelection`
   (near line 372): `.insert_resource(ActionMenuState::default())`.

Add a new `ActiveActionHighlight` marker component (derive `Component`,
`Debug`, `Clone`, `Copy`) to tag the currently highlighted `ActionButton`
entity. This component is used by a new visual update system to swap
`BackgroundColor`.

#### 1.2 Add Foundation Functionality

**File: `src/game/systems/combat.rs`**

**Step 1 — Remove old `A`/`D`/`F` keyboard bindings from `combat_input_system`.**

Delete lines 1089–1105 (the `KeyA`, `KeyD`, `KeyF`, `Escape` branches) in their
entirety from `combat_input_system`.

**Step 2 — Replace `Interaction != None` with `Interaction::Pressed`.**

Change line 1062:
- Before: `if *interaction != Interaction::None {`
- After: `if *interaction == Interaction::Pressed {`

**Step 3 — Add `Tab`/`Enter` keyboard action handling to `combat_input_system`.**

Inside `combat_input_system`, after the existing mouse interaction block,
add a keyboard block using `Option<Res<ButtonInput<KeyCode>>>`:

- `Tab` just-pressed: increment `action_menu_state.active_index` modulo 5;
  reset `action_menu_state.confirmed = false`.
- `Enter` just-pressed: set `action_menu_state.confirmed = true`.
- `Escape` just-pressed: if `target_sel.0.is_some()`, clear
  `target_sel.0 = None`; else no-op (target cancel is Phase 2).

**Step 4 — Implement `dispatch_combat_action` helper function.**

Add a new free function `dispatch_combat_action` in
`src/game/systems/combat.rs`:

```text
fn dispatch_combat_action(
    button_type: ActionButtonType,
    actor: CombatantId,
    target_sel: &mut TargetSelection,
    defend_writer: &mut Option<MessageWriter<DefendAction>>,
    flee_writer: &mut Option<MessageWriter<FleeAction>>,
) {
    match button_type {
        ActionButtonType::Attack => { target_sel.0 = Some(actor); }
        ActionButtonType::Defend => {
            if let Some(w) = defend_writer { w.write(DefendAction { combatant: actor }); }
        }
        ActionButtonType::Flee => {
            if let Some(w) = flee_writer { w.write(FleeAction); }
        }
        ActionButtonType::Cast | ActionButtonType::Item => {
            // Phase 4: submenu open — handled by separate systems
        }
    }
}
```

**Step 5 — Route both mouse and keyboard through `dispatch_combat_action`.**

Rewrite `combat_input_system` so that:
- A `Pressed` mouse interaction on an `ActionButton` calls
  `dispatch_combat_action(button.button_type, actor, ...)`.
- An `Enter` press (confirmed) reads
  `action_menu_state.active_index` to select the `ActionButtonType` from the
  ordered array `[Attack, Defend, Cast, Item, Flee]` and calls
  `dispatch_combat_action(selected_type, actor, ...)`.
- After either dispatch, `action_menu_state.confirmed = false`.

**Step 6 — Add `update_action_highlight` system.**

Add a new system `update_action_highlight` in
`src/game/systems/combat.rs`:

```text
fn update_action_highlight(
    action_menu_state: Res<ActionMenuState>,
    mut buttons: Query<(&ActionButton, &mut BackgroundColor)>,
) {
    let ordered = [
        ActionButtonType::Attack,
        ActionButtonType::Defend,
        ActionButtonType::Cast,
        ActionButtonType::Item,
        ActionButtonType::Flee,
    ];
    let active_type = ordered[action_menu_state.active_index];
    for (btn, mut bg) in buttons.iter_mut() {
        *bg = if btn.button_type == active_type {
            BackgroundColor(ACTION_BUTTON_HOVER_COLOR)   // existing constant, line 178
        } else {
            BackgroundColor(ACTION_BUTTON_COLOR)          // existing constant, line 175
        };
    }
}
```

Register `update_action_highlight` in `CombatPlugin::build` as an `Update`
system ordered after `combat_input_system`.

**Step 7 — Reset `ActionMenuState` on menu open.**

In `update_combat_ui` (line 1018), where the action menu becomes `Visible`,
also reset `action_menu_state.active_index = 0` and
`action_menu_state.confirmed = false`.  Add `ResMut<ActionMenuState>` as a
parameter to `update_combat_ui`.

**Step 8 — Add blocked-turn feedback.**

In `combat_input_system`, if `turn_state.0` is not `PlayerTurn` and any input
event (mouse `Pressed` or `Tab`/`Enter` key) occurs, log a visible info message:
`info!("Combat: input blocked — not player turn")`. (Phase 3 will surface this
in the UI; the log is the Phase 1 minimum.)

#### 1.3 Integrate Foundation Work

- Verify that `handle_input` in `src/game/systems/input.rs` now exits before
  any movement logic when `GameMode::Combat(_)` is active. Run
  `cargo test -p antares -- input` to confirm no regression in movement tests.
- Verify `update_action_highlight` reads `ACTION_BUTTON_COLOR` (line 175) and
  `ACTION_BUTTON_HOVER_COLOR` (line 178) — both constants already exist in
  `src/game/systems/combat.rs`.
- Verify `ActionMenuState::default()` is registered before `combat_input_system`
  runs by confirming the resource insert in `CombatPlugin::build`.

#### 1.4 Testing Requirements

Write all new tests as `#[test]` functions in `#[cfg(test)]` modules inside
their respective source files.

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T1-1 | `test_tab_cycles_through_actions` | `combat.rs` | Set `active_index=0`, simulate `Tab` press exactly 4 times, verify `active_index` ends at 4. |
| T1-2 | `test_tab_wraps_at_end` | `combat.rs` | Set `active_index=4`, simulate `Tab`, verify `active_index` becomes 0. |
| T1-3 | `test_default_highlight_is_attack_on_menu_open` | `combat.rs` | Call the action-menu-open path; assert `action_menu_state.active_index == 0`. |
| T1-4 | `test_enter_dispatches_active_action` | `combat.rs` | Set `active_index=1` (Defend), set `confirmed=true`, run system, assert `DefendAction` was written and `confirmed==false`. |
| T1-5 | `test_mouse_pressed_dispatches_action` | `combat.rs` | Simulate `Interaction::Pressed` on `Attack` button, assert `TargetSelection` becomes `Some`. |
| T1-6 | `test_mouse_hover_does_not_dispatch` | `combat.rs` | Simulate `Interaction::Hovered` on an `ActionButton`, assert no `TargetSelection` or `DefendAction`. |
| T1-7 | `test_key_a_does_not_dispatch_in_combat` | `combat.rs` | Press `KeyA` during `PlayerTurn`; assert `TargetSelection` remains `None` and no `DefendAction` or `FleeAction` written. |
| T1-8 | `test_movement_blocked_in_combat_mode` | `input.rs` | Insert `GlobalState` with `GameMode::Combat`, run `handle_input` with `MoveForward` pressed, assert party position unchanged. |
| T1-9 | `test_blocked_input_logs_feedback` | `combat.rs` | Set `CombatTurnState::EnemyTurn`, simulate `Tab` press, verify `action_menu_state` unchanged (no crash, no dispatch). |

#### 1.5 Deliverables

- [ ] `ActionMenuState` resource declared and registered in `CombatPlugin::build`.
- [ ] `update_action_highlight` system implemented and registered.
- [ ] `combat_input_system`: `A`/`D`/`F` shortcuts removed; `Interaction::Pressed` used; `Tab`/`Enter` keyboard traversal implemented; unified dispatch through `dispatch_combat_action`.
- [ ] `handle_input` in `src/game/systems/input.rs`: `GameMode::Combat(_)` guard added before movement processing.
- [ ] Default `Attack` highlight applied when action menu becomes `Visible` (via `update_combat_ui` reset).
- [ ] Blocked-turn info log emitted when input arrives during non-`PlayerTurn` state.
- [ ] All 9 tests in T1-1 through T1-9 pass under `cargo test -p antares`.

#### 1.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- `Tab` cycles the highlighted action; `Enter` dispatches it — both verified by automated tests.
- `Attack` button is highlighted on every menu open — verified by T1-3.
- `Interaction::Pressed` is the sole mouse activation event — verified by T1-5 and T1-6.
- `A` / `D` / `F` no longer trigger any combat action — verified by T1-7.
- Movement and rotation input during combat has no effect on party position or turn order — verified by T1-8.

---

### Phase 2: Target Selection and Action Completeness

**Goal:** Add keyboard target cycling as a complement to existing mouse
`select_target` click handling. Both paths use the same dispatch code.

#### 2.1 Feature Work

**File: `src/game/systems/combat.rs`**

Add a field `active_target_index: Option<usize>` to `ActionMenuState` (or add a
separate `TargetSelectionState` resource — prefer adding to `ActionMenuState` to
keep state in one place):

- `active_target_index: Option<usize>` — set to `Some(0)` when target-select
  mode is entered (i.e., when `TargetSelection.0` becomes `Some`); `None` when
  not selecting.

When `TargetSelection.0` is `Some` (target-select mode is active):
- `Tab` just-pressed: advance `active_target_index` modulo
  `[count of alive monster participants in CombatResource]`.
- `Enter` just-pressed: read `active_target_index`, resolve it to the
  corresponding `CombatantId::Monster(idx)` from alive participants in
  `combat_res.state.participants`, write `AttackAction { attacker, target }`,
  clear `TargetSelection.0 = None`, reset `active_target_index = None`.
- `Escape` just-pressed: clear `TargetSelection.0 = None`, reset
  `active_target_index = None`.

#### 2.2 Integrate Feature

**File: `src/game/systems/combat.rs`**

Refactor `select_target` (line 1125) to extract a helper function
`confirm_attack_target(attacker, target_monster_idx, ...)` that:
1. Writes `AttackAction { attacker, target: CombatantId::Monster(target_monster_idx) }`.
2. Clears `TargetSelection.0 = None`.

Both mouse (`select_target`) and keyboard (`Tab`/`Enter` in target mode) call
`confirm_attack_target`.

Add a new `update_target_highlight` system that sets all `EnemyCard`
`BackgroundColor` values and additionally marks the card at
`active_target_index` with `TURN_INDICATOR_COLOR` to distinguish keyboard
selection from the generic `ENEMY_CARD_HIGHLIGHT_COLOR`.

Ensure `enter_target_selection` (line 1110) remains as-is for general
highlight-all-enemies behaviour; `update_target_highlight` adds the
specific-card highlight on top.

#### 2.3 Configuration Updates

**File: `src/game/systems/combat.rs`**

Add the following public constants near the existing colour constants (around
line 184):

```text
/// Number of top-level action buttons (Attack, Defend, Cast, Item, Flee)
pub const COMBAT_ACTION_COUNT: usize = 5;
/// Canonical order of action buttons (matches spawn order in setup_combat_ui)
pub const COMBAT_ACTION_ORDER: [ActionButtonType; 5] = [
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Cast,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];
```

Update `combat_input_system` and `update_action_highlight` to reference
`COMBAT_ACTION_COUNT` and `COMBAT_ACTION_ORDER` instead of inline literals.

#### 2.4 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T2-1 | `test_tab_cycles_targets` | `combat.rs` | Enter target-select mode with 3 monsters; press `Tab` 3 times; verify index wraps 0→1→2→0. |
| T2-2 | `test_enter_confirms_target` | `combat.rs` | Enter target-select mode with index=1; press `Enter`; assert `AttackAction.target == CombatantId::Monster(1)` and `TargetSelection.0 == None`. |
| T2-3 | `test_escape_cancels_target_selection` | `combat.rs` | Enter target-select mode; press `Escape`; assert `TargetSelection.0 == None` and `active_target_index == None`. |
| T2-4 | `test_mouse_click_target_matches_keyboard_confirm` | `combat.rs` | Click `EnemyCard` at index 0 via `Interaction::Pressed`; assert same `AttackAction` as keyboard confirm would produce. |
| T2-5 | `test_combat_action_order_constant_matches_spawn_order` | `combat.rs` | Assert `COMBAT_ACTION_ORDER[0] == ActionButtonType::Attack` and that all 5 variants are covered. |

#### 2.5 Deliverables

- [ ] `active_target_index: Option<usize>` field added to `ActionMenuState`.
- [ ] Keyboard target cycling (`Tab`) and confirmation (`Enter`) implemented in `combat_input_system` when `TargetSelection.0.is_some()`.
- [ ] `Escape` cancels target selection correctly.
- [ ] `confirm_attack_target` helper extracted; mouse and keyboard both call it.
- [ ] `update_target_highlight` system implemented and registered.
- [ ] `COMBAT_ACTION_COUNT` and `COMBAT_ACTION_ORDER` constants defined and used.
- [ ] All 5 tests T2-1 through T2-5 pass under `cargo test -p antares`.

#### 2.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- Full keyboard-only attack flow works: `Tab` to Attack → `Enter` → `Tab` to target → `Enter` executes attack.
- Mouse target click and keyboard target confirm produce identical `AttackAction` messages — verified by T2-4.
- `Escape` during target selection cleanly resets state — verified by T2-3.

---

### Phase 3: Visual Combat Feedback and Animation State

**Goal:** Anchor floating damage to target UI cards, add effect-type visual
differentiation, surface monster-turn damage numbers, integrate
`CombatTurnState::Animating` as a real runtime transition, and add in-world
monster HP hover bars.

#### 3.1 Foundation Work

**File: `src/game/systems/combat.rs`**

Define a `CombatFeedbackEvent` struct:

```text
#[derive(Message, Debug, Clone)]
pub struct CombatFeedbackEvent {
    pub target: CombatantId,
    pub effect: CombatFeedbackEffect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatFeedbackEffect {
    Damage(u32),
    Heal(u32),
    Miss,
    Status(String),   // condition name
}
```

Register `CombatFeedbackEvent` with `.add_message::<CombatFeedbackEvent>()` in
`CombatPlugin::build`.

Define a `MonsterHpHoverBar` marker component:

```text
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterHpHoverBar {
    pub participant_index: usize,
}
```

#### 3.2 Add Foundation Functionality

**Anchored floating numbers:**

Replace the current absolute-positioned `FloatingDamage` spawn pattern (e.g.,
lines 1472–1492) with an `emit_combat_feedback` helper that:

1. Writes a `CombatFeedbackEvent` message.
2. A new `spawn_combat_feedback` system reads the message and spawns the
   `FloatingDamage` node as a **child of the target's `EnemyCard` entity**
   (for `CombatantId::Monster`) or the target's HUD slot (for
   `CombatantId::Player`). Use `commands.entity(card_entity).with_children(...)`.

Call `emit_combat_feedback` from `handle_attack_action`, `handle_cast_spell_action`,
and `handle_use_item_action` instead of the inline spawn code.

**Effect type colours:**

Add constants:

```text
pub const FEEDBACK_COLOR_DAMAGE: Color = Color::srgb(1.0, 0.3, 0.3);  // red
pub const FEEDBACK_COLOR_HEAL: Color   = Color::srgb(0.3, 1.0, 0.3);  // green
pub const FEEDBACK_COLOR_MISS: Color   = Color::srgb(0.8, 0.8, 0.8);  // grey
pub const FEEDBACK_COLOR_STATUS: Color = Color::srgb(1.0, 0.8, 0.0);  // yellow
```

`spawn_combat_feedback` picks the colour based on `CombatFeedbackEffect` variant.

**Monster-turn feedback:**

In `execute_monster_turn` (the existing system at around line 2320), after
damage is resolved, write a `CombatFeedbackEvent` with the same path as player
attacks.

**`CombatTurnState::Animating` integration:**

In `handle_attack_action`, `handle_cast_spell_action`, and
`handle_use_item_action` (ECS system wrappers), set
`turn_state.0 = CombatTurnState::Animating` **before** calling the
`perform_*_with_rng` helper, and restore it to `PlayerTurn` or `EnemyTurn`
**after** the call returns. Use a 0-frame delay (just set the state; visual
systems already read it every frame).

**In-world monster HP hover bars:**

Add a new system `spawn_monster_hp_hover_bars` in
`src/game/systems/combat.rs` (or `combat_visual.rs`):

- Runs once when `GameMode::Combat(_)` becomes active (check: no
  `MonsterHpHoverBar` entities exist but `CombatResource` has monsters).
- For each alive monster in `combat_res.state.participants`, spawns a
  billboard-style UI world-space node (or a 2D screen-space node above the
  monster entity) tagged with `MonsterHpHoverBar { participant_index }`.

Add a system `update_monster_hp_hover_bars` that reads `MonsterHpHoverBar`
components and updates bar widths/colours from `CombatResource`, identical to
`EnemyHpBarFill` updates in `update_combat_ui`.

Add a system `cleanup_monster_hp_hover_bars` that despawns all
`MonsterHpHoverBar` entities when `GameMode` leaves `Combat`.

Register all three systems in `CombatPlugin::build`.

#### 3.3 Integrate Foundation Work

- Remove all four inline `FloatingDamage` spawn blocks from
  `handle_attack_action` (lines ~1472–1492), `handle_cast_spell_action`
  (lines ~1853–1875), `handle_use_item_action`, and monster-turn handler.
  Replace each with a call to `emit_combat_feedback`.
- Register `spawn_combat_feedback` in `CombatPlugin::build` ordered after
  the action handlers.
- Verify `hide_indicator_during_animation` in `combat_visual.rs` (line 275)
  correctly hides during the now-active `Animating` state and restores after.
- Verify `update_combat_ui` action-menu visibility (line 1019) works correctly
  during `Animating` (should remain hidden for monsters, and temporarily hidden
  for players during animation window).

#### 3.4 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T3-1 | `test_feedback_event_emitted_on_hit` | `combat.rs` | Fire `AttackAction` that hits; assert `CombatFeedbackEvent { effect: Damage(_) }` was written. |
| T3-2 | `test_feedback_event_emitted_on_miss` | `combat.rs` | Fire `AttackAction` that misses (mock RNG); assert `CombatFeedbackEvent { effect: Miss }`. |
| T3-3 | `test_monster_turn_emits_feedback` | `combat.rs` | Run `execute_monster_turn`; assert `CombatFeedbackEvent` was written for the attacked player. |
| T3-4 | `test_animating_state_set_during_action` | `combat.rs` | After firing `AttackAction` and before `perform_attack_action_with_rng` returns, assert `CombatTurnStateResource.0 == Animating`. |
| T3-5 | `test_indicator_hidden_during_animating` | `combat_visual.rs` | Already covered by existing `test_turn_indicator_hidden_during_animation` — confirm it still passes with real Animating transitions. |
| T3-6 | `test_hover_bars_spawned_on_combat_start` | `combat.rs` | Enter combat with 2 monsters; run 1 frame; assert 2 `MonsterHpHoverBar` entities exist. |
| T3-7 | `test_hover_bars_removed_on_combat_exit` | `combat.rs` | Enter then exit combat; run 1 frame; assert 0 `MonsterHpHoverBar` entities exist. |
| T3-8 | `test_hover_bar_hp_updated_after_damage` | `combat.rs` | Damage a monster; run 1 frame; assert corresponding `MonsterHpHoverBar` bar reflects reduced HP. |

#### 3.5 Deliverables

- [ ] `CombatFeedbackEvent` and `CombatFeedbackEffect` declared and registered.
- [ ] `emit_combat_feedback` helper implemented; `spawn_combat_feedback` system registered.
- [ ] All four inline `FloatingDamage` spawn blocks replaced with `emit_combat_feedback` calls.
- [ ] Floating numbers anchored to target's `EnemyCard` or player HUD slot.
- [ ] Effect-type colour constants defined; `spawn_combat_feedback` uses them.
- [ ] `execute_monster_turn` writes `CombatFeedbackEvent` for player targets.
- [ ] `CombatTurnState::Animating` set and cleared during action resolution in all three action system wrappers.
- [ ] `MonsterHpHoverBar` spawn, update, and cleanup systems implemented and registered.
- [ ] All 8 tests T3-1 through T3-8 pass under `cargo test -p antares`.

#### 3.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- Every resolved combat action (player and monster) produces a visible,
  target-anchored feedback floating number.
- Feedback numbers are colour-coded: red for damage, green for heal, grey for
  miss, yellow for status.
- `CombatTurnState::Animating` is a real runtime state — verified by T3-4 and
  the existing T3-5.
- Monster HP hover bars are present immediately when combat starts — verified
  by T3-6.
- HP hover bars are removed cleanly when combat ends — verified by T3-7.

---

### Phase 4: Defeated Monster World-Mesh Removal

**Goal:** Remove the monster's 3D mesh from the game world and clear its
`MapEvent::Encounter` entry from the map data after combat is won. Currently
neither action happens: the creature visual persists in the world and the
encounter event remains live, causing the player to re-enter the same fight if
they walk back to the tile.

#### 4.1 Analysis

| Item | Detail |
|------|--------|
| Encounter mesh spawn | `src/game/systems/map.rs` lines 1111–1147 inside `spawn_map`. Each `MapEvent::Encounter` spawns a creature entity tagged `MapEntity(map.id)` + `TileCoord(position)`. No `MonsterMarker` or encounter-specific component is attached. |
| After combat victory | `handle_combat_victory` (line 2542, `combat.rs`) calls `global_state.0.exit_combat()` and spawns the victory UI. It does **not** clear the `MapEvent` or despawn any world entity. |
| Encounter re-trigger | `events.rs` `handle_map_event_triggered` still finds the live `MapEvent::Encounter` on the tile, so walking over it again starts another fight. |
| Existing precedent | `cleanup_recruitable_visuals` (line 127, `map.rs`) despawns recruitable NPC meshes when their backing `MapEvent::RecruitableCharacter` disappears. The same pattern applies here. |

#### 4.2 Foundation Work

**File: `src/game/systems/combat.rs`**

Add a field `encounter_position: Option<crate::domain::types::Position>` and
`encounter_map_id: Option<crate::domain::types::MapId>` to `CombatResource`
(around line 190):

```text
pub struct CombatResource {
    pub state: CombatState,
    pub player_orig_indices: Vec<Option<usize>>,
    pub resolution_handled: bool,
    /// World position of the encounter tile that started this combat.
    pub encounter_position: Option<crate::domain::types::Position>,
    /// Map ID of the map containing the encounter tile.
    pub encounter_map_id: Option<crate::domain::types::MapId>,
}
```

Update `CombatResource::new()` and `CombatResource::clear()` to initialise/reset
both new fields to `None`.

**File: `src/game/systems/events.rs`**

In `handle_map_event_triggered` (the function that processes
`MapEvent::Encounter` and calls `start_encounter`), after `start_encounter`
succeeds, write the encounter position and map ID into `CombatResource`:

```text
combat_res.encounter_position = Some(triggered_event.position);
combat_res.encounter_map_id = Some(global_state.0.world.current_map);
```

This makes the position available to the victory handler without requiring a
separate message.

#### 4.3 Add Feature Functionality

**File: `src/game/systems/combat.rs`**

In `handle_combat_victory` (line 2542), **before** calling
`global_state.0.exit_combat()`, add:

```text
// Remove the encounter event from the map so the tile no longer re-triggers.
if let (Some(pos), Some(map_id)) =
    (combat_res.encounter_position, combat_res.encounter_map_id)
{
    if let Some(map) = global_state.0.world.get_map_mut(map_id) {
        map.remove_event(pos);
        info!("Removed encounter event at {:?} on map {}", pos, map_id);
    }
}
// Clear stored position so it doesn't accidentally affect a later combat.
combat_res.encounter_position = None;
combat_res.encounter_map_id = None;
```

**File: `src/game/systems/map.rs`**

Add a new marker component to tag encounter creature visuals distinctly from
other `MapEntity` entities. Insert after the `RecruitableVisualMarker`
declaration (around line 58):

```text
/// Component tagging an entity as a visual marker for a map encounter.
/// Despawned by `cleanup_encounter_visuals` when the backing MapEvent is removed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterVisualMarker {
    /// Map ID this entity belongs to.
    pub map_id: types::MapId,
    /// Tile position of the originating MapEvent::Encounter.
    pub position: types::Position,
}
```

In `spawn_map` (line 1129), attach `EncounterVisualMarker` to the entity
spawned for each `MapEvent::Encounter`:

```text
commands.entity(entity).insert((
    CreatureVisual { creature_id, scale_override: None },
    MapEntity(map.id),
    TileCoord(*position),
    EncounterVisualMarker { map_id: map.id, position: *position },
    Visibility::default(),
));
```

Add a new system `cleanup_encounter_visuals` in `src/game/systems/map.rs`,
mirroring `cleanup_recruitable_visuals` (line 127):

```text
fn cleanup_encounter_visuals(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    query: Query<(Entity, &EncounterVisualMarker)>,
) {
    let game_state = &global_state.0;
    for (entity, marker) in query.iter() {
        let Some(map) = game_state.world.get_map(marker.map_id) else {
            // Map no longer loaded — despawn the visual
            commands.entity(entity).despawn();
            continue;
        };
        // Despawn if the backing encounter event is gone
        let event_present = matches!(
            map.get_event(marker.position),
            Some(world::MapEvent::Encounter { .. })
        );
        if !event_present {
            commands.entity(entity).despawn();
        }
    }
}
```

Register `cleanup_encounter_visuals` in `MapManagerPlugin::build` (around
line 121) alongside the existing cleanup systems:

```text
.add_systems(
    Update,
    (
        map_change_handler,
        handle_door_opened,
        spawn_map_markers,
        cleanup_recruitable_visuals,
        cleanup_encounter_visuals,   // ← add
    ),
)
```

#### 4.4 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T4-E1 | `test_encounter_position_stored_on_combat_start` | `events.rs` or `combat.rs` | After `handle_map_event_triggered` fires an encounter, assert `combat_res.encounter_position == Some(tile_pos)` and `combat_res.encounter_map_id == Some(map_id)`. |
| T4-E2 | `test_encounter_event_removed_on_victory` | `combat.rs` | Run `handle_combat_victory`; assert `map.get_event(encounter_position)` returns `None` afterwards. |
| T4-E3 | `test_encounter_position_cleared_after_victory` | `combat.rs` | After victory, assert `combat_res.encounter_position == None` and `combat_res.encounter_map_id == None`. |
| T4-E4 | `test_encounter_visual_despawned_when_event_removed` | `map.rs` | Spawn an `EncounterVisualMarker` entity; remove the backing event; run `cleanup_encounter_visuals`; assert entity no longer exists. |
| T4-E5 | `test_encounter_visual_kept_when_event_present` | `map.rs` | Spawn an `EncounterVisualMarker`; leave the backing event intact; run `cleanup_encounter_visuals`; assert entity still exists. |

#### 4.5 Deliverables

- [ ] `encounter_position` and `encounter_map_id` fields added to `CombatResource`; `new()` and `clear()` updated.
- [ ] `events.rs` populates `CombatResource.encounter_position/map_id` when encounter combat starts.
- [ ] `handle_combat_victory` removes `MapEvent::Encounter` from map data and clears stored position.
- [ ] `EncounterVisualMarker` component declared in `src/game/systems/map.rs` and attached during `spawn_map`.
- [ ] `cleanup_encounter_visuals` system implemented and registered in `MapManagerPlugin`.
- [ ] All 5 tests T4-E1 through T4-E5 pass under `cargo test -p antares`.

#### 4.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- After winning combat the monster mesh disappears from the world on the next frame — verified by T4-E4.
- Walking back to the encounter tile after victory does not restart combat — verified by T4-E2.
- `CombatResource` encounter fields are reset cleanly so a subsequent combat on a different tile is unaffected — verified by T4-E3.

---

### Phase 5: Cast/Item Action Completion and UX Hardening

**Goal:** Wire `ActionButtonType::Cast` and `ActionButtonType::Item` all the way
from the action menu through submenu panel spawn, user selection, and target
dispatch. Complete the existing `SpellSelectionPanel` / `ItemSelectionPanel`
marker component stubs.

#### 4.1 Feature Work

**File: `src/game/systems/combat.rs`**

**Spell submenu:**

Add a system `spawn_spell_selection_panel` that:
- Is triggered when `ActionMenuState.active_index == CAST_INDEX (2)` and
  `confirmed == true` (or when a `Pressed` mouse click on `ActionButtonType::Cast`
  occurs).
- Spawns a `SpellSelectionPanel { caster: actor }` UI node as a child of the
  `CombatHudRoot`.
- Populates the panel with one `SpellButton { spell_id, sp_cost }` per spell in
  the active caster's spell list (read from `CombatResource`).
- Hides the `ActionMenuPanel` while the submenu is open.

Add a system `handle_spell_selection_input` that:
- Reads `Interaction::Pressed` on `SpellButton` entities.
- Also handles `Tab` (cycle among spell buttons) and `Enter` (confirm).
- On confirm: writes `CastSpellAction { caster, spell_id, target }` where
  target is either the already-selected `TargetSelection` or enters target-
  select mode if the spell requires a single target.
- Despawns the `SpellSelectionPanel` and restores `ActionMenuPanel` visibility.

**Item submenu:**

Implement `spawn_item_selection_panel` and `handle_item_selection_input` using
the same pattern as the spell submenu but reading the caster's inventory from
`CombatResource` and populating `ItemButton { item_id, charges }` nodes.

Add failure feedback: if the caster has no usable spells (or no items), spawn a
temporary `FloatingDamage`-style text node with the message `"No spells"` /
`"No items"` using `FEEDBACK_COLOR_STATUS` colour and a 1.5-second lifetime
instead of opening an empty panel.

#### 4.2 Integrate Feature

- Register all four new systems (`spawn_spell_selection_panel`,
  `handle_spell_selection_input`, `spawn_item_selection_panel`,
  `handle_item_selection_input`) in `CombatPlugin::build`.
- Remove the `ActionButtonType::Cast | ActionButtonType::Item => { // Not
  implemented }` comment from `dispatch_combat_action`; replace with a call
  to the submenu-open logic.
- Connect `CastSpellAction` and `UseItemAction` through the existing
  `handle_cast_spell_action` and `handle_use_item_action` system wrappers.
- Wire failure-reason feedback for failed casts/item uses from
  `perform_cast_action_with_rng` (currently a silent `Ok(())` at line 1306)
  and `perform_use_item_action_with_rng` (line 1372): convert the `Err` branches
  to emit a `CombatFeedbackEvent { effect: Status("No SP".to_string()) }` before
  returning `Ok(())`.

#### 4.3 Configuration Updates

**File: `src/game/systems/combat.rs`**

Add constants for submenu sizing consistent with the existing UI constant block
(around line 139):

```text
pub const SPELL_PANEL_WIDTH: Val = Val::Px(200.0);
pub const SPELL_PANEL_HEIGHT: Val = Val::Px(240.0);
pub const SPELL_BUTTON_HEIGHT: Val = Val::Px(32.0);
pub const ITEM_PANEL_WIDTH: Val = Val::Px(200.0);
pub const ITEM_PANEL_HEIGHT: Val = Val::Px(240.0);
pub const ITEM_BUTTON_HEIGHT: Val = Val::Px(32.0);
```

#### 4.4 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T4-1 | `test_cast_button_mouse_opens_spell_panel` | `combat.rs` | Simulate `Pressed` on `ActionButtonType::Cast` button; assert a `SpellSelectionPanel` entity exists. |
| T4-2 | `test_item_button_mouse_opens_item_panel` | `combat.rs` | Simulate `Pressed` on `ActionButtonType::Item` button; assert `ItemSelectionPanel` entity exists. |
| T4-3 | `test_spell_confirm_writes_cast_action` | `combat.rs` | Open spell panel; simulate `Pressed` on a `SpellButton`; assert `CastSpellAction` written with correct `spell_id`. |
| T4-4 | `test_item_confirm_writes_use_item_action` | `combat.rs` | Open item panel; simulate `Pressed` on an `ItemButton`; assert `UseItemAction` written with correct `inventory_index`. |
| T4-5 | `test_empty_spell_list_shows_no_spell_feedback` | `combat.rs` | Player has empty spell list; activate Cast; assert no `SpellSelectionPanel` spawned and `CombatFeedbackEvent { Status("No spells") }` written. |
| T4-6 | `test_cast_failure_emits_feedback` | `combat.rs` | Fire `CastSpellAction` with insufficient SP; assert `CombatFeedbackEvent { Status(_) }` written instead of silent no-op. |
| T4-7 | `test_keyboard_navigates_spell_panel` | `combat.rs` | Open spell panel with 3 spells; press `Tab` twice; press `Enter`; assert `CastSpellAction` with third spell's `spell_id`. |
| T4-8 | `test_all_five_actions_are_dispatched_via_enter` | `combat.rs` | For each of the 5 action indices, set `active_index` and `confirmed=true`, run system, assert appropriate message written or submenu opened. |

#### 4.5 Deliverables

- [ ] `spawn_spell_selection_panel` and `handle_spell_selection_input` systems implemented and registered.
- [ ] `spawn_item_selection_panel` and `handle_item_selection_input` systems implemented and registered.
- [ ] `dispatch_combat_action` Cast/Item branch opens the appropriate submenu instead of being a no-op.
- [ ] Failure feedback emitted for: no spells, no items, insufficient SP, invalid item slot.
- [ ] `SPELL_PANEL_*` and `ITEM_PANEL_*` constants defined.
- [ ] End-to-end parity: all 5 top-level actions reachable by keyboard (`Tab`/`Enter`) and mouse (`Pressed`).
- [ ] All 8 tests T4-1 through T4-8 pass under `cargo test -p antares`.

#### 4.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- All five action menu entries (Attack, Defend, Cast, Item, Flee) are fully
  interactive and produce a non-silent result — verified by T4-8.
- Spell and item submenus are keyboard-navigable — verified by T4-7.
- Failure paths produce visible feedback rather than silent returns — verified
  by T4-5 and T4-6.

---

### Phase 5: Victory Screen Lifecycle and Level-Up Notification

**Goal:** Close the two reward UX gaps that exist in the already-working reward
distribution code. XP, gold, gems, and items are all correctly applied by
`process_combat_victory_with_rng` (line 2405) and verified by existing tests at
lines 3337 and 3415. The gaps are: (1) the `VictorySummaryRoot` overlay is
never removed so it persists on screen forever, and (2) level-up events from
`award_experience` are silently discarded.

#### 5.1 Analysis of Existing Reward System

| Component | Status | Location |
|-----------|--------|---------|
| XP summed from dead monsters | ✅ Implemented | `process_combat_victory_with_rng` line 2411 |
| XP split evenly, remainder distributed | ✅ Implemented | Lines 2444–2461 |
| `award_experience` called per recipient | ✅ Implemented, return value discarded (`let _ =`) | Line 2456 |
| Gold rolled and added to `party.gold` | ✅ Implemented | Lines 2464–2480, 2503 |
| Gems rolled and added to `party.gems` | ✅ Implemented | Lines 2482–2490, 2504 |
| Items probabilistic drop, placed in inventory | ✅ Implemented | Lines 2492–2527 |
| HP/SP/conditions synced back to party | ✅ Implemented | `sync_combat_to_party_on_exit` line 537 |
| `VictorySummaryRoot` overlay despawned | ❌ **Missing** | No cleanup system registered |
| Level-up detection and notification | ❌ **Missing** | `let _ =` at line 2456 discards result |

#### 5.2 Feature Work

**File: `src/game/systems/combat.rs`**

**Fix 1 — Victory screen dismiss lifecycle.**

Add a `VictoryScreenTimer` resource:

```text
/// Resource tracking how long the victory summary screen has been shown.
/// Cleared when the screen is dismissed.
#[derive(Resource, Debug, Clone)]
pub struct VictoryScreenTimer {
    /// Seconds elapsed since the victory screen appeared. `None` = screen not shown.
    pub elapsed: Option<f32>,
}
impl Default for VictoryScreenTimer {
    fn default() -> Self { Self { elapsed: None } }
}
```

Register `VictoryScreenTimer` in `CombatPlugin::build` with
`.insert_resource(VictoryScreenTimer::default())`.

In `handle_combat_victory` (line 2542), after spawning the `VictorySummaryRoot`
overlay, set `victory_timer.elapsed = Some(0.0)`.

Replace the raw text-only `VictorySummaryRoot` overlay spawn with a layout
that includes:
- A text node showing `"Victory!\nXP: N  Gold: N  Gems: N  Items: [...]"`.
- A `"Continue"` `Button` node tagged with a new `VictoryContinueButton` marker
  component (derive `Component`, `Debug`, `Clone`, `Copy`).

Add a new system `handle_victory_screen`:

```text
fn handle_victory_screen(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<VictoryScreenTimer>,
    mut interactions: Query<
        &Interaction,
        (Changed<Interaction>, With<VictoryContinueButton>),
    >,
    roots: Query<Entity, With<VictorySummaryRoot>>,
) {
    let Some(ref mut elapsed) = timer.elapsed else { return; };
    *elapsed += time.delta_secs();

    let dismissed = *elapsed >= 8.0   // auto-dismiss after 8 seconds
        || interactions.iter().any(|i| *i == Interaction::Pressed);

    if dismissed {
        for entity in roots.iter() {
            commands.entity(entity).despawn();
        }
        timer.elapsed = None;
    }
}
```

Register `handle_victory_screen` in `CombatPlugin::build` as an `Update` system.

**Fix 2 — Level-up detection and notification.**

In `process_combat_victory_with_rng` (line 2456), replace `let _ =` with:

```text
let level_ups = crate::domain::progression::award_experience(
    &mut global_state.0.party.members[party_idx],
    award,
)
.unwrap_or(0);
if level_ups > 0 {
    // Store for UI display — collected into VictorySummary
    xp_awarded.push((party_idx, award));
    // level_up_counts tracks levels gained per party member
    level_up_counts.push((party_idx, level_ups));
}
```

Add field `pub level_ups: Vec<(usize, u32)>` to `VictorySummary` (line 2393)
where `(party_index, levels_gained)`.

Update `handle_combat_victory` to extend the summary text with level-up lines:
one line per party member who levelled up, e.g.
`"{name} reached level {new_level}!"`.

#### 5.3 Configuration Updates

**File: `src/game/systems/combat.rs`**

Add a constant near the existing UI constants (around line 154):

```text
/// Seconds before the victory summary screen auto-dismisses
pub const VICTORY_SCREEN_AUTO_DISMISS_SECS: f32 = 8.0;
```

Update `handle_victory_screen` to use `VICTORY_SCREEN_AUTO_DISMISS_SECS`
instead of the inline literal `8.0`.

#### 5.4 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T5-1 | `test_victory_screen_auto_dismisses_after_timeout` | `combat.rs` | Set `elapsed = Some(VICTORY_SCREEN_AUTO_DISMISS_SECS - 0.01)`, advance one frame by delta 0.02s, run `handle_victory_screen`, assert `VictorySummaryRoot` entities are despawned and `timer.elapsed == None`. |
| T5-2 | `test_victory_screen_dismissed_by_continue_button` | `combat.rs` | Set `elapsed = Some(1.0)`, simulate `Interaction::Pressed` on `VictoryContinueButton`, run system, assert zero `VictorySummaryRoot` entities remain. |
| T5-3 | `test_victory_screen_not_dismissed_before_timeout` | `combat.rs` | Set `elapsed = Some(1.0)`, no interaction, run system, assert `VictorySummaryRoot` entity still exists. |
| T5-4 | `test_level_up_captured_in_victory_summary` | `combat.rs` | Build a party member 1 XP below level-up threshold; award that XP via `process_combat_victory_with_rng`; assert `summary.level_ups` contains `(party_idx, 1)`. |
| T5-5 | `test_no_level_up_when_xp_insufficient` | `combat.rs` | Award 0 XP; assert `summary.level_ups` is empty. |
| T5-6 | `test_victory_timer_reset_after_dismiss` | `combat.rs` | Run through dismiss; assert `VictoryScreenTimer.elapsed == None`. |

#### 5.5 Deliverables

- [ ] `VictoryScreenTimer` resource declared and registered in `CombatPlugin::build`.
- [ ] `VictoryContinueButton` marker component declared.
- [ ] `VictorySummaryRoot` overlay updated to include `VictoryContinueButton`.
- [ ] `handle_victory_screen` system implemented and registered.
- [ ] `VICTORY_SCREEN_AUTO_DISMISS_SECS` constant defined and used.
- [ ] `VictorySummary.level_ups` field added; `process_combat_victory_with_rng` populates it.
- [ ] `handle_combat_victory` displays level-up lines in the victory overlay text.
- [ ] All 6 tests T5-1 through T5-6 pass under `cargo test -p antares`.

#### 5.6 Success Criteria

- `cargo test -p antares` passes with no regressions.
- Victory screen despawns after `VICTORY_SCREEN_AUTO_DISMISS_SECS` even if
  the player does not interact — verified by T5-1.
- Victory screen despawns immediately when the player presses Continue —
  verified by T5-2.
- Party members who levelled up see their name and new level in the victory
  text — verified by T5-4.
- `VictoryScreenTimer.elapsed` resets to `None` after dismiss so a subsequent
  combat does not inherit stale state — verified by T5-6.

---

### Phase 6: Initiative and Dynamic Turn Order

**Goal:** Modify the turn order system so that combat initiative is not purely
deterministic based on raw speed stats, incorporates equipment bonuses, and
reacts dynamically to speed changes mid-combat (like Haste or Slow spells)
rather than remaining frozen for the entire encounter.

#### 6.1 Analysis of Existing Turn Order System

| Component | Status | Location |
|-----------|--------|---------|
| Speed property extraction | ✅ Implemented | `Combatant::get_speed` reads `stats.speed.current` (`engine.rs` line 71) |
| Hardcoded priority | ✅ Implemented | `Handicap` enum shifts priority based on ambush state (`engine.rs`) |
| Equipment bonuses on speed | ❌ **Missing** | No system reads equipped items to alter `speed.current` |
| Initiative Roll | ❌ **Missing** | Turn order is a pure sort by speed (`engine.rs` line 446). No 1d10 + Speed mechanic. |
| Dynamic Turn Order | ❌ **Missing** | `calculate_turn_order` runs exactly once at `start_combat`. If a character is paralyzed, their turn is skipped, but the list isn't resorted. If a character dies, they remain in the array but their turn is skipped. |

#### 6.2 Feature Work

**File: `src/domain/combat/engine.rs`**

**Fix 1 — Add an initiative roll to `calculate_turn_order`.**

Currently, `calculate_turn_order` takes a `&CombatState` only, which means it
cannot use `rng` to roll initiative.
Modify `calculate_turn_order` to accept a `&mut impl Rng`.
Update `start_combat` to also take `rng: &mut impl Rng` and pass it through.
Update all calls to `start_combat` and `calculate_turn_order` in `events.rs`
and tests to pass an RNG.

In `calculate_turn_order` (line 403), instead of purely mapping to
`(id, speed)`, map to `(id, initiative_score)` where:
`initiative_score = speed as i32 + rng.random_range(1..=10)` (a 1d10 roll).

Modify the sorting logic to use `initiative_score` descending.

**Fix 2 — Dynamic Re-sort per Round.**

Currently, `turn_order` is calculated once in `start_combat`.
In `CombatState::advance_round` (line 229), after all condition effects are
ticked and DoT is applied, re-run `turn_order = calculate_turn_order(self, rng)`.
Because `advance_round` does not currently take an RNG, you will need to add
`rng: &mut impl Rng` to `advance_round` and cascade that to `advance_turn`.

Wait, `calculate_turn_order` needs to look at `speed.current`, not `speed.base`.
When conditions like Haste/Slow affect `speed.current` during `reconcile_conditions`,
the next round's re-sort will naturally pick up the altered speed.

**File: `src/domain/character.rs` (or a higher-level system)**

**Fix 3 — Apply equipment bonuses before combat.**

`character.inventory` and `character.equipment` only store `ItemId`s. To read
the `speed` stat off an item, the domain needs to look up the item in the
`ContentDatabase`. The pure domain module `character.rs` does not have access
to the `ContentDatabase`.

Add a system to `src/game/systems/inventory.rs` or `combat.rs` called
`apply_equipment_stats` that recalculates a character's `stats.current` values.
It should:
1. Reset all `stats.current` to `stats.base`.
2. Lookup `equipment.weapon` in `ContentDatabase.weapons`. If it has a
   `speed_bonus` (requires adding this to `WeaponData` if it doesn't exist already,
   but currently `stats.speed` exists on `ItemData`/`ArmorData`), apply it.
3. Lookup `equipment.armor` in `ContentDatabase.armors`. Apply stat modifiers.
4. Lookup active conditions and apply those modifiers.

This `apply_equipment_stats` system must run outside combat (e.g. `Update`
schedule) so the player's Speed is correct *before* the encounter starts, ensuring
`start_combat` reads the correct enhanced Speed.

#### 6.3 Testing Requirements

| Test ID | Test Name | File | Description |
|---------|-----------|------|-------------|
| T6-1 | `test_turn_order_uses_initiative_roll` | `engine.rs` | Test that identical speed combatants can sort in different orders across multiple `calculate_turn_order` calls depending on the RNG. |
| T6-2 | `test_turn_order_recalculated_each_round` | `engine.rs` | Start combat; `advance_turn` until a new round. Mock RNG so a slow character rolls high. Assert the `turn_order` array has changed. |
| T6-3 | `test_equipment_modifies_speed` | `inventory.rs` | Equip an item with a speed bonus; run `apply_equipment_stats`; assert `speed.current == speed.base + bonus`. |

#### 6.4 Deliverables

- [ ] `calculate_turn_order` updated to take `rng` and use `speed + 1d10`.
- [ ] `advance_round` updated to re-roll and re-sort `turn_order` every round.
- [ ] `apply_equipment_stats` function built to refresh `stats.current` based on `base` + equipped gear + conditions.
- [ ] Automated tests T6-1 through T6-3 implemented and passing.

---

### Phase 7: Documentation Updates

**Goal:** Update and create player-facing and developer-facing documentation to
reflect all changes introduced in Phases 1–4.

#### 5.1 Documentation Work

**File: `docs/how-to/using_game_menu.md`**

Update lines 142–171 ("Menu During Combat" section) to reflect:
- `ESC` during combat opens the pause menu.
- Remove any claim that combat controls use `A` / `D` / `F`.
- Add current controls: `Tab` cycles actions, `Enter` activates, `Escape`
  cancels target selection.

**New file: `docs/how-to/playing_combat.md`**

Create a new how-to guide with the following sections:

| Section | Content |
|---------|---------|
| Overview | What combat mode is; when it triggers. |
| Action Menu Controls | Table: `Tab` (cycle), `Enter` (activate), mouse click (activate). |
| Target Selection | `Tab` (cycle targets), `Enter` (confirm), `Escape` (cancel). |
| Reading the UI | Enemy HP bars in the combat panel; HP hover bars above monsters; floating damage numbers; turn order display. |
| Action Types | Attack, Defend, Cast (spell submenu), Item (item submenu), Flee — one paragraph each. |
| Feedback Colours | Red = damage, green = heal, grey = miss, yellow = status. |

**File: `docs/explanation/combat_system_improvement_implementation.md`**

Mark each deliverable checkbox as completed `[x]` as phases are completed.

#### 5.2 Testing Requirements

No automated tests for documentation. Manual review checklist:

| Check | Criterion |
|-------|-----------|
| D-1 | `docs/how-to/playing_combat.md` exists and renders without broken links. |
| D-2 | `docs/how-to/using_game_menu.md` no longer references `A`/`D`/`F` as combat shortcuts. |
| D-3 | All five action types are described in `playing_combat.md`. |
| D-4 | Control table in `playing_combat.md` matches the implementation in Phase 1/2. |

#### 5.3 Deliverables

- [ ] `docs/how-to/using_game_menu.md` updated: combat controls section corrected.
- [ ] `docs/how-to/playing_combat.md` created with all sections listed in 5.1.
- [ ] All deliverable checkboxes in this plan file updated to `[x]` upon phase completion.

#### 5.4 Success Criteria

- `docs/how-to/playing_combat.md` exists as a valid Markdown file.
- `docs/how-to/using_game_menu.md` contains no references to `A`/`D`/`F` as combat action keys.
- The control description in documentation exactly matches the implementation verified in Phases 1–4.
