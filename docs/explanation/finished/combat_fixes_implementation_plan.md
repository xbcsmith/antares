i!--
SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
-->

# Combat Fixes Implementation Plan

## Overview

Three combat-adjacent systems require completion: the Defense action is only
half-implemented (it applies a permanent AC bonus that is never reset), the
In-Combat Item Use button exists in the UI but dispatches nothing (no item
selection panel is ever spawned), and Out-of-Combat Item Use works for
consumable items but silently fails for non-consumable charged items (wands,
staves) because neither the action list builder nor the exploration handler
cover that item type. The plan is ordered so each phase is independently
testable and shippable.

---

## Current State Analysis

### Existing Infrastructure

| Area | File(s) | Notes |
|---|---|---|
| Defense action domain | `src/domain/combat/types.rs` | `TurnAction::Defend` variant defined |
| Defense action handler | `src/game/systems/combat.rs` | `perform_defend_action` applies `pc.ac.modify(2)` — **never reset** |
| Defending state tracking | `src/domain/combat/engine.rs` `CombatState` | **No `defending_combatants` field** — no per-combatant defending flag |
| Damage pipeline | `src/domain/combat/engine.rs` `resolve_attack` | AC is consulted but there is **no damage-reduction path for defenders** |
| Active spell defence buffs | `src/application/mod.rs` `ActiveSpells` | `shield`, `power_shield`, `leather_skin` fields exist — **not consulted in `resolve_attack`** |
| Combat item use dispatch | `src/game/systems/combat.rs` `dispatch_combat_action` | `ActionButtonType::Item` branch is a comment: `// Item submenu — handled by separate systems` — **no dispatch code** |
| Item selection panel | `src/game/systems/combat.rs` | `ItemSelectionPanel`, `ItemButton` components defined — **never spawned; no `update_item_selection_panel` system registered** |
| Item use domain | `src/domain/combat/item_usage.rs` | `validate_item_use_slot`, `execute_item_use_by_slot` fully implemented and tested |
| Combat item use handler | `src/game/systems/combat.rs` | `handle_use_item_action`, `perform_use_item_action_with_rng` implemented and registered |
| Exploration item use | `src/game/systems/inventory_ui.rs` | `handle_use_item_action_exploration` implemented and registered for `ItemType::Consumable` |
| Exploration action list | `src/game/systems/inventory_ui.rs` `build_action_list` | Use button added only when `ItemType::Consumable` — **charged non-consumable items (wands, staves) never get a Use button** |
| Exploration handler charged | `src/game/systems/inventory_ui.rs` `handle_use_item_action_exploration` | Calls `resolve_consumable_for_use` which returns `Err` for non-consumables — **wands/staves silently fail** |

### Identified Issues

1. **Defense never resets and has no damage reduction** — `perform_defend_action`
   calls `pc.ac.modify(2)` but `CombatState` has no flag to remember which
   combatants are defending, and `advance_round` does not undo the bonus. The
   AC bonus stacks infinitely across multiple Defend turns. There is no
   percentage-based damage reduction path in `resolve_attack`, and the
   `shield` / `power_shield` / `leather_skin` fields in `ActiveSpells` are
   never consulted during attack resolution.

2. **In-combat Item button is non-functional** — `dispatch_combat_action`
   contains only a comment for `ActionButtonType::Item`. No
   `update_item_selection_panel` system exists; the `ItemSelectionPanel` and
   `ItemButton` components are unreachable dead code. Clicking the Item button
   during combat produces no visible effect.

3. **Out-of-combat item use is incomplete for charged magical items** —
   `build_action_list` only adds a Use action when the item is
   `ItemType::Consumable`. Wands, staves, and other non-consumable items that
   carry `spell_effect` and charges are never offered a Use option in the
   exploration inventory. If a `UseItemExplorationAction` were somehow
   dispatched for one of these items, `resolve_consumable_for_use` would return
   an error and the player would see a "not a consumable" log message.

---

## Implementation Phases

---

### Phase 1: Defense System — Complete Implementation

Fix the defense action so the AC bonus is properly bounded to a single round,
add per-combatant defending state tracking, introduce percentage-based damage
reduction when a defender is attacked, and wire the existing `ActiveSpells`
defence buffs (`shield`, `power_shield`, `leather_skin`) into the damage
pipeline.

#### 1.1 Add Defending State to `CombatState`

**File:** `src/domain/combat/engine.rs`

Add a `defending_combatants: std::collections::HashSet<usize>` field to
`CombatState`. The `usize` key is the participant index (position in
`participants`). Player combatants are eligible; monster combatants are not
(monsters do not defend in the current design).

Initialise to an empty `HashSet` in `CombatState::new`. Derive nothing extra —
the type already derives `Default` indirectly through its fields.

#### 1.2 Fix `perform_defend_action` — Set Flag, Keep AC Bonus

**File:** `src/game/systems/combat.rs`

In `perform_defend_action`, after applying `pc.ac.modify(2)`, resolve the
participant index from `CombatantId::Player(idx)` and insert `idx` into
`combat_res.state.defending_combatants`. The AC bonus signals a lighter-weight
evasion improvement; the defending flag gates the damage-reduction path added
in Phase 1.3.

Remove the AC bonus application for `CombatantId::Monster` — monsters do not
defend; the branch should return `Err(CombatError::CombatantNotFound)` or be
handled as a no-op to future-proof against erroneous dispatches.

#### 1.3 Reset Defending State at Round End

**File:** `src/domain/combat/engine.rs`

In `advance_round` (called at the end of each full round from `advance_turn`),
after condition ticks and DOT applications, clear `defending_combatants` and
undo the +2 AC bonus for each defending player:

```
for idx in self.defending_combatants.drain() {
    if let Some(Combatant::Player(pc)) = self.participants.get_mut(idx) {
        // Remove the defend bonus by reversing the modify.
        // ac.current is clamped to [ac.base, AC_MAX] so a simple
        // subtraction with floor-at-base is safe.
        pc.ac.current = pc.ac.current.saturating_sub(2).max(pc.ac.base);
    }
}
```

This ensures the defending bonus lasts exactly until the start of the next
round, mirroring the design intent from architecture Section 4.4.

#### 1.4 Add Damage Reduction in `resolve_attack`

**File:** `src/domain/combat/engine.rs`

In `resolve_attack`, after the hit/miss determination succeeds and before
`apply_damage` is called, check whether the target is defending and whether
any `ActiveSpells` defence buffs are active, then compute a damage reduction
multiplier.

The reduction rules (implement in a small private helper
`compute_defense_reduction`):

| Condition | Damage multiplier |
|---|---|
| `power_shield` active | `0.0` (immune, no damage) |
| Defending + `shield` active | `0.35` (65 % reduction) |
| Defending only | `0.5` (50 % reduction), modified by Endurance: each full 10 points of `endurance.current` above 10 adds an additional `−0.02` (capped at `0.25` minimum multiplier) |
| `shield` active (not defending) | `0.80` (20 % reduction) |
| `leather_skin` active (not defending) | `0.90` (10 % reduction) |
| None of the above | `1.0` (no reduction) |

Apply the multiplier to the raw damage roll before calling `apply_damage`.
Cast via `(damage as f32 * multiplier).ceil() as i32` with a minimum of `1` so
attacks always deal at least 1 damage (no reduction grants immunity except
`power_shield`).

`ActiveSpells` lives on `GameState`, not `CombatState`. Pass an
`Option<&ActiveSpells>` into `resolve_attack` from the Bevy system wrapper
(`handle_attack_action`, `perform_attack_action_with_rng`) so pure domain
tests can pass `None` to get the old behaviour. The signature change requires
updating all call sites.

#### 1.5 Combat Log Entry for Defending

In `format_combat_log_line` (called from `collect_combat_feedback_log_lines`)
add a `CombatFeedbackEffect::Defend` variant for the log and a corresponding
branch in `format_combat_log_line` that emits `"{name} takes a defensive
stance."` This matches the existing pattern for Miss / Status feedback.

#### 1.6 Testing Requirements

**New unit tests** in `src/game/systems/combat.rs` `mod tests`:

- `test_defend_bonus_resets_after_round_end` — after defending, advance one
  full round and assert `pc.ac.current == pc.ac.base`.
- `test_defend_reduces_incoming_damage` — defending character receives an
  attack; assert final HP reflects ~50 % damage, not full damage.
- `test_power_shield_grants_immunity` — character with `power_shield` active
  receives an attack; assert HP is unchanged.
- `test_shield_reduces_damage_without_defending` — character with `shield`
  active (not defending) receives 20 % less damage.
- `test_defend_and_shield_combo_reduces_damage_65_percent` — character with
  `shield` active AND defending receives 65 % reduction.
- `test_defend_bonus_does_not_stack` — calling `perform_defend_action` twice in
  the same round does not add the +2 AC bonus twice (guard the insertion with
  a `contains` check before modifying AC).
- `test_monster_defend_action_returns_error` — dispatching `DefendAction` with
  a `CombatantId::Monster` returns an appropriate error.

Update `test_defend_action_improves_ac` to also assert `defending_combatants`
contains the player index after the action.

Run: `cargo nextest run --all-features -p antares`.

#### 1.7 Deliverables

- [ ] `defending_combatants: HashSet<usize>` added to `CombatState`
- [ ] `perform_defend_action` inserts into `defending_combatants`
- [ ] `advance_round` clears `defending_combatants` and removes +2 AC bonus
- [ ] `compute_defense_reduction` helper implemented and used in `resolve_attack`
- [ ] `power_shield`, `shield`, `leather_skin` from `ActiveSpells` consulted in damage calculation
- [ ] `CombatFeedbackEffect::Defend` variant + log line added
- [ ] All call sites of `resolve_attack` updated to pass `Option<&ActiveSpells>`
- [ ] New and updated unit tests passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 1.8 Success Criteria

- A character that Defends sees the +2 AC bonus for exactly one round; it
  returns to base at the start of the next round.
- Defending reduces damage taken by ~50 % (Endurance-adjusted), with further
  reductions when `shield` is active.
- `power_shield` (spell or item) grants full immunity for the round.
- The combat log shows `"{name} takes a defensive stance."` when Defend is
  chosen.
- Choosing Defend twice in the same round does not stack the AC bonus.

---

### Phase 2: In-Combat Item Use — Item Selection Panel

Wire the existing `UseItemAction` / `handle_use_item_action` infrastructure
to the UI by implementing an item selection panel that mirrors the spell
selection panel pattern already in the codebase.

#### 2.1 Add `ItemPanelState` Resource

**File:** `src/game/systems/combat.rs`

Add a new resource immediately after `SpellPanelState`:

```rust
/// Tracks which combatant's item inventory is open in the item selection panel.
///
/// `None` means the panel is closed.
#[derive(Resource, Default)]
pub struct ItemPanelState {
    pub user: Option<CombatantId>,
}
```

Register it in `CombatPlugin::build` with `.insert_resource(ItemPanelState::default())`.

#### 2.2 Wire `ActionButtonType::Item` in `dispatch_combat_action`

**File:** `src/game/systems/combat.rs`

Replace the no-op comment in `dispatch_combat_action` for `ActionButtonType::Item`:

```rust
ActionButtonType::Item => {
    item_panel_state.user = Some(actor);
}
```

Add `item_panel_state: &mut ItemPanelState` to the `dispatch_combat_action`
parameter list and thread it through every call site (two: `combat_input_system`
and the mouse-click path).

#### 2.3 Implement `update_item_selection_panel`

**File:** `src/game/systems/combat.rs`

Model this system directly after `update_spell_selection_panel`. It:

1. Despawns the existing panel when `item_panel_state.user` is `None`.
2. Returns early (does nothing) when the panel is already open.
3. Resolves the user's character from `CombatResource`.
4. Iterates the character's inventory items, filtering to those where
   `validate_item_use_slot(character, idx, content, true)` returns `Ok`.
5. Spawns a `Node` panel entity with `ItemSelectionPanel { user }` and one
   `ItemButton { item_id, charges }` child per usable slot, styled to match
   the spell panel layout constants (`SPELL_PANEL_LEFT`, `SPELL_PANEL_TOP`).
6. Each `ItemButton` row renders: icon (use `🧪` as default for consumables,
   `✨` for charged non-consumables), item name, current charges if > 0.

Register in `CombatPlugin::build` immediately after `update_spell_selection_panel`:

```rust
.add_systems(
    Update,
    update_item_selection_panel.after(combat_input_system),
)
```

#### 2.4 Implement `handle_item_button_interaction`

**File:** `src/game/systems/combat.rs`

Model after `handle_spell_button_interaction`. When an `ItemButton` is clicked
or keyboard-confirmed:

1. Read `item_panel_state.user` for the user combatant.
2. Determine target: for consumables that self-target (HealHp, RestoreSp,
   CureCondition, BoostAttribute, BoostResistance) default target is the user.
   For offensive items (damage spells via `spell_effect`) enter target
   selection the same way Attack does.
3. Write `UseItemAction { user, inventory_index, target }`.
4. Set `item_panel_state.user = None` to close the panel.

A cancel button (labelled `✖ Cancel`, same as the spell panel's cancel
button pattern) sets `item_panel_state.user = None` without dispatching.

Register in `CombatPlugin::build`:

```rust
.add_systems(
    Update,
    handle_item_button_interaction.after(update_item_selection_panel),
)
```

#### 2.5 Add Cleanup System

**File:** `src/game/systems/combat.rs`

Add `cleanup_item_panel_on_combat_exit` (model after
`cleanup_spell_panel_on_combat_exit`) that:

1. Despawns all `ItemSelectionPanel` entities.
2. Resets `item_panel_state.user = None`.

Runs on combat exit. Register it in `CombatPlugin::build`.

#### 2.6 Keyboard Navigation for Item Panel

In `combat_input_system`, add handling for the item panel that mirrors the
spell panel:

- `Escape` while item panel is open → close panel (set `user = None`).
- `↑` / `↓` cycle between item buttons (track `focused_item_index` in a new
  field on `ItemPanelState` or reuse `ActionMenuState.active_index`).
- `Enter` confirms the focused item.

#### 2.7 Testing Requirements

**New unit tests** in `src/game/systems/combat.rs` `mod tests` and
`mod perform_use_item_tests`:

- `test_dispatch_item_sets_item_panel_user` — `dispatch_combat_action` with
  `ActionButtonType::Item` sets `item_panel_state.user = Some(actor)`.
- `test_item_panel_closes_on_cancel` — a cancel action clears
  `item_panel_state.user`.
- `test_item_panel_dispatches_use_item_action` — selecting an item from the
  panel writes a `UseItemAction` message with correct fields.
- `test_item_panel_not_open_when_user_is_none` — with `user = None`, no
  `ItemSelectionPanel` entity is spawned.
- `test_item_panel_escape_closes_panel` — pressing Escape while the panel is
  open clears `user`.
- `test_combat_item_use_heals_party_member` — full Bevy integration test: party
  member with a healing potion in inventory clicks Item, selects the potion,
  asserts HP increased and inventory slot consumed.

Run: `cargo nextest run --all-features -p antares`.

#### 2.8 Deliverables

- [ ] `ItemPanelState` resource added and registered
- [ ] `dispatch_combat_action` `ActionButtonType::Item` sets `item_panel_state.user`
- [ ] `update_item_selection_panel` system implemented and registered
- [ ] `handle_item_button_interaction` system implemented and registered
- [ ] `cleanup_item_panel_on_combat_exit` system implemented and registered
- [ ] Keyboard navigation (Escape / Up / Down / Enter) for item panel
- [ ] New unit and integration tests passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 2.9 Success Criteria

- Clicking the Item button in the combat action menu opens an item selection
  panel listing only combat-usable items in the current actor's inventory.
- Selecting a consumable from the panel applies its effect, consumes one
  charge, and advances the turn.
- The panel closes when cancel is clicked or Escape is pressed.
- Items with `is_combat_usable: false` do not appear in the combat item panel.
- After the last charge is consumed, the item is removed from inventory.

---

### Phase 3: Out-of-Combat Item Use — Charged Magical Items

Extend the exploration item use flow to support non-consumable items that
carry `spell_effect` and charges (wands, staves, enchanted accessories). The
consumable path is already complete and must not be regressed.

#### 3.1 Extend `build_action_list` for Charged Items

**File:** `src/game/systems/inventory_ui.rs`

After the existing `is_consumable` check, add a second eligibility check:

```rust
// Also offer Use for non-consumable items that have a spell_effect and charges.
let is_usable_charged_item = item_opt
    .map(|item| {
        item.spell_effect.is_some()
            && item.max_charges > 0
            && !matches!(item.item_type, ItemType::Consumable(_))
    })
    .unwrap_or(false);

// Slot charges must be > 0 for the Use button to appear.
let slot_has_charges = character
    .inventory
    .items
    .get(selected_slot_index)
    .map(|s| s.charges > 0)
    .unwrap_or(false);

if is_consumable || (is_usable_charged_item && slot_has_charges) {
    actions.push(PanelAction::Use {
        party_index: focused_party_index,
        slot_index: 0,
    });
}
```

Remove the old `if is_consumable` block and replace with this combined guard.

#### 3.2 Extend `handle_use_item_action_exploration`

**File:** `src/game/systems/inventory_ui.rs`

After calling `validate_item_use_slot` (which already passes for charged
non-consumables with `in_combat = false`), branch on item type:

**Branch A — Consumable (existing path):** Call `resolve_consumable_for_use`,
apply effect, write game log. No change.

**Branch B — Charged non-consumable with `spell_effect`:**

1. Retrieve the item from the database.
2. Verify `slot.charges > 0` (defensive check).
3. Decrement the slot's charge count (or remove the slot if charges reach 0),
   using the same charge-consume logic as the combat path.
4. Retrieve the `SpellId` from `item.spell_effect`.
5. Apply the spell's effect using
   `apply_consumable_effect_exploration` after constructing a synthetic
   `ConsumableData { effect: ConsumableEffect::CastSpell(spell_id), is_combat_usable: true, duration_minutes: None }`.
   This reuses the existing CastSpell exploration path already handled by
   `apply_consumable_effect_exploration`.
6. Write a game log line: `"{item_name} used. Casting {spell_name}."`.
7. Reset `InventoryNavigationState` to `SlotNavigation` phase.

#### 3.3 Ensure `ActiveSpells` Write-Back for Buff Spells

When a wand casts a buff spell out of combat (fire shield, haste, etc.), the
effect needs to propagate into `GameState.active_spells` the same way spell
casting does during exploration. Verify that the `CastSpell` branch in
`apply_consumable_effect_exploration` already writes to `active_spells`; if
not, thread `&mut ActiveSpells` into the handler and apply the buff there.

#### 3.4 Render Charged Items Distinctly in Inventory

**File:** `src/game/systems/inventory_ui.rs` `render_item_grid` / `paint_item_silhouette`

When rendering an item slot that is a charged non-consumable (wand/staff),
display the remaining charges as a small numeric annotation overlaid on the
item silhouette (e.g., `"✨3"` in the bottom-right corner). This matches how
ammo quantities are displayed elsewhere. Use the same `egui::Color32` overlay
pattern used for the existing charge indicator (search for `charges` in
`render_item_grid`).

#### 3.5 Testing Requirements

**New unit tests** in `src/game/systems/inventory_ui.rs` `mod tests`:

- `test_build_action_list_use_present_for_charged_wand` — a character holding
  a wand (non-consumable, `spell_effect = Some(1)`, `charges = 3`) gets a Use
  action in the list.
- `test_build_action_list_no_use_for_charged_wand_with_zero_charges` —
  same wand but `charges = 0` does not get a Use action.
- `test_build_action_list_no_use_for_non_consumable_without_spell_effect` —
  a plain weapon with no `spell_effect` does not get a Use action.
- `test_exploration_use_wand_applies_spell_effect` — full system test:
  character holds a wand with a healing spell effect, `UseItemExplorationAction`
  is dispatched, assert HP increased and wand charge decremented.
- `test_exploration_use_wand_removes_slot_on_last_charge` — wand with 1
  charge: after use, slot is removed from inventory.
- `test_exploration_use_wand_writes_game_log` — assert game log contains
  `"{item_name} used."` after wand use.
- `test_exploration_use_wand_zero_charges_writes_error_log` — if a charged
  item somehow reaches the handler with 0 charges, the player sees a
  `"Cannot use {name}: no charges left."` message.

Run: `cargo nextest run --all-features -p antares`.

#### 3.6 Deliverables

- [ ] `build_action_list` adds Use button for non-consumable items with
  `spell_effect` and `charges > 0`
- [ ] `handle_use_item_action_exploration` handles the charged item path
- [ ] Charge decrement / slot removal for wands on use
- [ ] Spell effect applied via `apply_consumable_effect_exploration` `CastSpell` branch
- [ ] `ActiveSpells` buff write-back verified / implemented
- [ ] Charged item visual charge counter in inventory grid
- [ ] New unit and integration tests passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean
- [ ] All existing `test_exploration_use_*` tests continue to pass

#### 3.7 Success Criteria

- Opening the exploration inventory with a wand (non-consumable, charged) shows
  a Use button.
- Selecting Use on a wand applies its spell effect to the owning character,
  decrements charges by 1, and writes a game log entry.
- When the last charge is used, the wand is removed from inventory.
- Consumable item use (all existing paths) is unaffected.
- Items with 0 charges never show a Use button.
- The game log entry for wand use names both the item and the spell.

---

## Implementation Order Summary

| Phase | Scope | Primary Files |
|---|---|---|
| 1 | Defense reset + damage reduction + `ActiveSpells` buffs | `engine.rs`, `combat.rs` |
| 2 | In-combat item selection panel + keyboard/mouse interaction | `combat.rs` |
| 3 | Out-of-combat charged item use + Use button extension | `inventory_ui.rs` |

Each phase must pass all four quality gates before the next phase begins:

```text
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

## Notes

- All test data lives in `data/test_campaign`, never in `campaigns/tutorial`
  (AGENTS.md Implementation Rule 5).
- The `perform_defend_action` public signature change (adding
  `Option<&ActiveSpells>`) is intentional. We do not care about backwards
  compatibility.
- Phase 1 defence reduction percentages are game-design values, not
  architecture-mandated constants. Extract them as named constants at the top
  of `engine.rs` (e.g., `DEFEND_DAMAGE_MULTIPLIER`, `SHIELD_DAMAGE_MULTIPLIER`)
  rather than embedding magic numbers in the code, per AGENTS.md
  Implementation Rule.
- No git operations; all commits are left to the user.
