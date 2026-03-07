# Consumables Outside Combat Implementation Plan

<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

## Overview

Antares already supports consumable execution in combat through
[`UseItemAction`](../../src/game/systems/combat.rs) and
[`execute_item_use_by_slot`](../../src/domain/combat/item_usage.rs), but the
inventory flow in [`GameMode::Inventory`](../../src/game/systems/inventory_ui.rs)
only exposes Drop and Transfer. This plan adds a parallel exploration/menu
item-use path that reuses a pure domain consumable executor, preserves the
combat-only restriction on `is_combat_usable: false`, consumes charges
correctly, and surfaces player-facing feedback through the existing
[`GameLog`](../../src/game/systems/ui.rs).

**Scope decision — self-target only.** Out-of-combat consumable use applies the
effect to the owning character only. Cross-party targeting (e.g. healing another
party member from the inventory screen) is explicitly out of scope for this plan
and belongs to a future targeting phase.

**Scope decision — `CureCondition` bitflags only.** `CureCondition(u8)` clears
only the matching bits in `character.conditions` (the `Condition` newtype) using
`character.conditions.remove(flags)`. It does **not** touch
`character.active_conditions`. This matches existing combat behavior in
`execute_item_use_by_slot`.

**Scope decision — all failures write to `GameLog`.** Every validation failure
produces a visible `GameLog` entry using `game_log.add(...)`. Low-level
diagnostic context may additionally use `warn!()`, but the `GameLog` message is
mandatory so the player is never silently blocked.

---

## Current State Analysis

### Existing Infrastructure

| Symbol                                                                                | File                                          | Notes                                                                                                                                            |
| ------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `ConsumableData { effect, is_combat_usable }`                                         | `src/domain/items/types.rs` L276–281          | Models all five effect variants and the combat restriction flag.                                                                                 |
| `ConsumableEffect`                                                                    | `src/domain/items/types.rs` L285–293          | Five variants: `HealHp(u16)`, `RestoreSp(u16)`, `CureCondition(u8)`, `BoostAttribute(AttributeType, i8)`, `BoostResistance(ResistanceType, i8)`. |
| `ResistanceType`                                                                      | `src/domain/items/types.rs` L339–386          | Eight variants. **Not** currently re-exported from `src/domain/items/mod.rs`.                                                                    |
| `validate_item_use_slot(character, slot_index, content, in_combat)`                   | `src/domain/combat/item_usage.rs` L153–226    | Central permission gate; passing `in_combat = false` correctly permits `is_combat_usable: false` items.                                          |
| `execute_item_use_by_slot(combat_state, user, inventory_index, target, content, rng)` | `src/domain/combat/item_usage.rs` L248–496    | Contains the authoritative match on all five `ConsumableEffect` variants embedded inside combat-state mutation.                                  |
| `ItemUseError`                                                                        | `src/domain/combat/item_usage.rs` L101–131    | Ten variants covering all validation failure cases. Reused as-is by the exploration path.                                                        |
| `InventoryPlugin::build()`                                                            | `src/game/systems/inventory_ui.rs` L74–88     | Chains `(inventory_input_system, inventory_ui_system, inventory_action_system)` in that exact order using `.chain()`.                            |
| `PanelAction`                                                                         | `src/game/systems/inventory_ui.rs` L187–205   | Currently has only `Drop { party_index, slot_index }` and `Transfer { from_party_index, from_slot_index, to_party_index }`.                      |
| `build_action_list(focused_party_index, panel_names)`                                 | `src/game/systems/inventory_ui.rs` L281–302   | Returns `Vec<PanelAction>` with Drop first and one Transfer per other open panel. Receives **no** item type or `GameContent` argument today.     |
| `inventory_input_system`                                                              | `src/game/systems/inventory_ui.rs` L324–564   | Parameters: `keyboard`, `global_state`, `nav_state`, `drop_writer`, `transfer_writer`. No `GameContent` and no use-writer today.                 |
| `inventory_ui_system`                                                                 | `src/game/systems/inventory_ui.rs` L576–744   | Renders egui panels and dispatches `PanelAction` via `drop_writer` / `transfer_writer`. No use-writer today.                                     |
| `inventory_action_system`                                                             | `src/game/systems/inventory_ui.rs` L1192–1369 | Reads `DropItemAction` and `TransferItemAction`; mutates `GlobalState`; resets `nav_state`.                                                      |
| `GameLog { messages: Vec<String> }`                                                   | `src/game/systems/ui.rs` L15–17               | `add(msg)` trims to 50 entries. Used via `Option<ResMut<GameLog>>` in `merchant_inventory_ui` and `rest`.                                        |

### Identified Issues

1. `inventory_input_system` cannot emit any use-item message, so consumables are
   unreachable outside combat regardless of their `is_combat_usable` flag.
2. The entire `ConsumableEffect` match lives inside `execute_item_use_by_slot`,
   which requires a live `CombatState`. There is no exploration-safe execution
   path, creating a risk of logic drift if a second implementation is written.
3. `build_action_list` has no access to item types, so it cannot conditionally
   expose a `Use` action for consumables.
4. `inventory_input_system` has no `GameContent` parameter, so the `U` shortcut
   cannot inspect whether the focused slot is a consumable.
5. `ResistanceType` is defined in `src/domain/items/types.rs` but is **not**
   re-exported from `src/domain/items/mod.rs`, so the new shared helper cannot
   be imported via the public items API without a fix.
6. Player feedback for out-of-combat item use is entirely absent.
7. Tests cover drop/transfer and combat item use, but not exploration usage,
   `BoostResistance` effects, or the `is_combat_usable` boundary from the
   inventory screen.

---

## Implementation Phases

---

### Phase 1: Extract Shared Consumable Domain Logic

Extract the `ConsumableEffect` match from `execute_item_use_by_slot` into a
standalone pure-domain function in a new `src/domain/items/consumable_usage.rs`
module, then re-export it and its result type from `src/domain/items/mod.rs`.
Route combat's existing executor through the new helper so there is one
authoritative implementation.

#### 1.1 Foundation Work

**File to create:** `src/domain/items/consumable_usage.rs`

Add the SPDX header block:

```text
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

Define the following public items in this file:

**`ConsumableApplyResult`** — a plain struct describing what changed so callers
do not need to re-derive deltas:

```text
pub struct ConsumableApplyResult {
    pub healing: i32,     // HP actually restored (0 if no heal)
    pub sp_restored: i32, // SP actually restored (0 if no SP restore)
    pub conditions_cleared: u8, // bitflags that were cleared (0 if none)
    pub attribute_delta: i16,   // stat change applied (0 if none)
    pub resistance_delta: i16,  // resistance change applied (0 if none)
}
```

**`apply_consumable_effect(character: &mut Character, effect: ConsumableEffect) -> ConsumableApplyResult`**
— applies one `ConsumableEffect` variant to `character` and returns a populated
`ConsumableApplyResult`. Rules per variant (all match existing combat behavior):

| Variant                             | Mutation                                                                                                                                                           | Cap       |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| `HealHp(amount)`                    | `character.hp.modify(amount as i32)` then clamp `character.hp.current` to `character.hp.base`                                                                      | `hp.base` |
| `RestoreSp(amount)`                 | `character.sp.modify(amount as i32)` then clamp `character.sp.current` to `character.sp.base`                                                                      | `sp.base` |
| `CureCondition(flags)`              | `character.conditions.remove(flags)` — bitflag removal only, do **not** touch `active_conditions`                                                                  | none      |
| `BoostAttribute(attr, amount)`      | call `character.stats.<field>.modify(amount as i16)` on the matching `AttributeType` arm                                                                           | none      |
| `BoostResistance(res_type, amount)` | call `character.resistances.<field>.modify(amount as i16)` on the matching `ResistanceType` arm using the same field mapping already in `execute_item_use_by_slot` | none      |

The `AttributeType` → field mapping is:
`Might → stats.might`, `Intellect → stats.intellect`, `Personality → stats.personality`,
`Endurance → stats.endurance`, `Speed → stats.speed`, `Accuracy → stats.accuracy`,
`Luck → stats.luck`.

The `ResistanceType` → field mapping (copied verbatim from `execute_item_use_by_slot`):
`Physical → resistances.magic`, `Fire → resistances.fire`, `Cold → resistances.cold`,
`Electricity → resistances.electricity`, `Energy → resistances.magic`,
`Paralysis → resistances.psychic`, `Fear → resistances.fear`, `Sleep → resistances.psychic`.

#### 1.2 Add Foundation Functionality

**File to edit:** `src/domain/items/mod.rs`

1. Add `pub mod consumable_usage;` in the module declarations block alongside
   `pub mod database;`, `pub mod equipment_validation;`, and `pub mod types;`.
2. Add the following two items to the `pub use` block so callers can import via
   `antares::domain::items::`:

   ```text
   pub use consumable_usage::{apply_consumable_effect, ConsumableApplyResult};
   pub use types::ResistanceType;   // currently missing from this re-export list
   ```

   The updated `pub use types::` line must now include `ResistanceType` alongside
   the existing exports (`AttributeType`, `ConsumableData`, `ConsumableEffect`, etc.).

**File to edit:** `src/domain/combat/item_usage.rs`

Replace the inline `ConsumableEffect` match arms inside `execute_item_use_by_slot`
(currently at approximately L370–455) with a call to
`apply_consumable_effect(pc_target, effect)`. Capture the returned
`ConsumableApplyResult` and use its fields to populate `total_healing`,
`total_damage`, and `applied_conditions` exactly as before. The public signature
of `execute_item_use_by_slot` does **not** change.

#### 1.3 Integrate Foundation Work

- `validate_item_use_slot` in `src/domain/combat/item_usage.rs` remains unchanged
  and continues to serve as the single permission gate. The exploration path will
  call it with `in_combat = false`.
- `execute_item_use_by_slot` retains all combat-only responsibilities: user
  identity check, `CombatState` borrow, charge consumption, turn advancement
  (`combat_state.advance_turn`), and `check_combat_end`. Only the effect match
  is delegated.
- Add `///` doc comments and a doctest to `apply_consumable_effect` and
  `ConsumableApplyResult` describing the self-mutation contract and cap behavior.

#### 1.4 Testing Requirements

Add a `#[cfg(test)] mod tests` block inside `src/domain/items/consumable_usage.rs`
with the following named tests. Each test constructs a `Character` directly
(no Bevy app required) and calls `apply_consumable_effect`.

| Test name                                              | What it verifies                                                                                                            |
| ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| `test_heal_hp_restores_up_to_base`                     | `HealHp(50)` on a character with `hp.base=30, hp.current=10` → `hp.current == 30` (capped), `result.healing == 20`          |
| `test_heal_hp_already_full_is_noop`                    | `HealHp(10)` on a full-health character → `hp.current` unchanged, `result.healing == 0`                                     |
| `test_restore_sp_capped_at_base`                       | `RestoreSp(100)` → `sp.current <= sp.base`                                                                                  |
| `test_cure_condition_clears_flags`                     | `CureCondition(Condition::POISONED)` removes only the poisoned bit; other bits unchanged                                    |
| `test_cure_condition_does_not_touch_active_conditions` | After `CureCondition`, `character.active_conditions` is unmodified                                                          |
| `test_boost_attribute_modifies_current_not_base`       | `BoostAttribute(AttributeType::Might, 5)` → `stats.might.current` increases by 5; `stats.might.base` unchanged              |
| `test_boost_resistance_modifies_current_not_base`      | `BoostResistance(ResistanceType::Fire, 10)` → `resistances.fire.current` increases by 10; `resistances.fire.base` unchanged |

Add the following regression tests to the existing `mod tests` block inside
`src/domain/combat/item_usage.rs`:

| Test name                                               | What it verifies                                                                                                                 |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `test_execute_item_use_delegates_to_shared_helper`      | After `execute_item_use_by_slot` heals a player, character HP matches what `apply_consumable_effect` would produce independently |
| `test_validate_out_of_combat_permits_non_combat_usable` | `validate_item_use_slot(..., false)` returns `Ok(())` for an item with `is_combat_usable: false`                                 |
| `test_validate_in_combat_rejects_non_combat_usable`     | `validate_item_use_slot(..., true)` returns `Err(ItemUseError::NotUsableInCombat)` for same item                                 |

#### 1.5 Deliverables

- [ ] `src/domain/items/consumable_usage.rs` created with SPDX header, `ConsumableApplyResult`, and `apply_consumable_effect` covering all five `ConsumableEffect` variants.
- [ ] `src/domain/items/mod.rs` updated: `pub mod consumable_usage;` added; `ResistanceType` added to `pub use types::...`; `apply_consumable_effect` and `ConsumableApplyResult` re-exported from `consumable_usage`.
- [ ] `execute_item_use_by_slot` in `src/domain/combat/item_usage.rs` delegates the `ConsumableEffect` match to `apply_consumable_effect`.
- [ ] All ten domain and regression tests listed above pass.
- [ ] `cargo fmt --all`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo nextest run --all-features` all pass with zero warnings.

#### 1.6 Success Criteria

- All five `ConsumableEffect` variants have a single authoritative implementation
  in `src/domain/items/consumable_usage.rs`.
- Combat and exploration share that implementation with no duplicated match logic.
- `ResistanceType` is importable via `antares::domain::items::ResistanceType`.
- The `is_combat_usable` permission gate is enforced solely by the `in_combat`
  parameter of `validate_item_use_slot`.

---

### Phase 2: Add Inventory-Level Use Actions and Input

Introduce the `UseItemExplorationAction` message, add `PanelAction::Use` to the
inventory action model, update `build_action_list` to expose `Use` for
consumable slots, and add keyboard routing in `inventory_input_system` and egui
dispatch in `inventory_ui_system`.

#### 2.1 Feature Work — New Message Type

**File to edit:** `src/game/systems/inventory_ui.rs`

Add a new `#[derive(Message)]` struct after the `TransferItemAction` struct
(currently ending at approximately L148):

```text
/// Emitted when the player uses a consumable item outside of combat.
///
/// The effect is applied to the owning character only (self-targeted).
/// `party_index` identifies which party member owns the item.
/// `slot_index` is the index within that character's `inventory.items`.
#[derive(Message)]
pub struct UseItemExplorationAction {
    /// Index of the party member (0-based) whose inventory contains the item.
    pub party_index: usize,
    /// Index of the slot within that character's inventory to use.
    pub slot_index: usize,
}
```

#### 2.2 Feature Work — `PanelAction::Use` Variant

**File to edit:** `src/game/systems/inventory_ui.rs`

Add a `Use` variant to the `PanelAction` enum (currently at L187–205) as the
**first** variant so it appears before `Drop` in the action strip when present:

```text
/// Use the consumable at `slot_index` owned by `party_index`.
Use {
    party_index: usize,
    slot_index: usize,
},
```

The `Drop` and `Transfer` variants are unchanged. Update the `PanelAction` doc
example to include a `Use` arm.

#### 2.3 Feature Work — Update `build_action_list`

**File to edit:** `src/game/systems/inventory_ui.rs`, function `build_action_list`
(currently L281–302).

Change the signature to:

```text
fn build_action_list(
    focused_party_index: usize,
    selected_slot_index: usize,
    panel_names: &[(usize, String)],
    character: &crate::domain::character::Character,
    game_content: Option<&crate::application::resources::GameContent>,
) -> Vec<PanelAction>
```

Logic change: before pushing `Drop`, check whether the slot at
`selected_slot_index` in `character.inventory.items` is a consumable item.
Do this by looking up `slot.item_id` in
`game_content.map(|gc| gc.db().items.get_item(slot_id))` and checking
`matches!(item.item_type, ItemType::Consumable(_))`. If it is a consumable,
push `PanelAction::Use { party_index: focused_party_index, slot_index: 0 }`
(slot placeholder, filled at execution time) as the **first** action before
`Drop`. Non-consumable slots or slots where `game_content` is `None` skip the
`Use` entry and return only `Drop` and `Transfer` as before.

Update every existing call site of `build_action_list` to pass the two new
arguments. There are two call sites:

1. Inside `inventory_input_system` (approximately L378) — pass
   `nav_state.selected_slot_index.unwrap_or(0)`, a reference to the focused
   character, and the `game_content` resource (added to the system parameters,
   see 2.5 below).
2. Inside `render_character_panel` (called from `inventory_ui_system`) — pass
   `selected_slot.unwrap_or(0)`, the `character` reference already in scope,
   and the `game_content` argument already passed to `render_character_panel`.

#### 2.4 Feature Work — Keyboard Routing

**File to edit:** `src/game/systems/inventory_ui.rs`, function
`inventory_input_system`.

**Parameter additions** — add to the function signature:

```text
game_content: Option<Res<GameContent>>,
mut use_writer: MessageWriter<UseItemExplorationAction>,
```

**`U` shortcut in `SlotNavigation` phase** — after the existing `Enter` handler
in the `SlotNavigation` section (approximately L478), add:

```text
// U key — use consumable in the highlighted slot directly (skip ActionNavigation)
if keyboard.just_pressed(KeyCode::KeyU) {
    if let Some(slot_idx) = nav_state.selected_slot_index {
        let is_consumable = global_state
            .0
            .party
            .members
            .get(focused_party_index)
            .and_then(|ch| ch.inventory.items.get(slot_idx))
            .and_then(|slot| {
                game_content
                    .as_deref()
                    .and_then(|gc| gc.db().items.get_item(slot.item_id))
            })
            .map(|item| matches!(item.item_type, ItemType::Consumable(_)))
            .unwrap_or(false);

        if is_consumable {
            use_writer.write(UseItemExplorationAction {
                party_index: focused_party_index,
                slot_index: slot_idx,
            });
            // Reset nav state
            if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                inv_state.selected_slot = None;
            }
            nav_state.selected_slot_index = None;
            nav_state.focused_action_index = 0;
            nav_state.phase = NavigationPhase::SlotNavigation;
        }
    }
    return;
}
```

**`ActionNavigation` phase — `Use` action dispatch** — inside the `Enter` branch
of the `ActionNavigation` section (approximately L408), add a match arm for
`PanelAction::Use`:

```text
PanelAction::Use { party_index, .. } => {
    use_writer.write(UseItemExplorationAction {
        party_index: *party_index,
        slot_index: slot_idx,
    });
}
```

Reset `nav_state` to `SlotNavigation` and clear `selected_slot` after dispatch
(same pattern as `Drop` and `Transfer`).

#### 2.5 Feature Work — egui Mouse Path

**File to edit:** `src/game/systems/inventory_ui.rs`, function
`inventory_ui_system`.

Add a `mut use_writer: MessageWriter<UseItemExplorationAction>` parameter to the
function signature.

In the `if let Some(action) = pending_action` block at the end of
`inventory_ui_system` (approximately L737), add:

```text
PanelAction::Use { party_index, slot_index } => {
    use_writer.write(UseItemExplorationAction { party_index, slot_index });
}
```

#### 2.6 Feature Work — UI Hint Text

**File to edit:** `src/game/systems/inventory_ui.rs`, function
`inventory_ui_system`.

Update the hint string for `NavigationPhase::SlotNavigation` (approximately L640)
to include the `U` shortcut:

```text
"Tab: cycle character   ←→↑↓: navigate slots   Enter: select item   U: use consumable   Esc/I: close"
```

Update the hint string for `NavigationPhase::ActionNavigation` to reflect that
`Use` is the first action for consumables:

```text
"←→: cycle actions   Enter: execute   Esc: cancel"
```

(This string does not need to change but add a note in the doc comment that
`Use` appears before `Drop` for consumable slots.)

Update the selected-item status line (approximately L632) to append
`"  [U: use]"` when the selected slot contains a consumable item. The check uses
the same `game_content` + `item_type` pattern as `build_action_list`.

#### 2.7 Plugin Registration

**File to edit:** `src/game/systems/inventory_ui.rs`, `InventoryPlugin::build()`
(L74–88).

Add `app.add_message::<UseItemExplorationAction>()` alongside the existing
`add_message` calls. The `.chain()` system set is updated in Phase 3 when the
handler system is added.

#### 2.8 Testing Requirements

Add tests to the `mod tests` block in `src/game/systems/inventory_ui.rs`:

| Test name                                          | What it verifies                                                                                    |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `test_build_action_list_use_first_for_consumable`  | `build_action_list` with a consumable slot returns `Use` as the first action                        |
| `test_build_action_list_no_use_for_non_consumable` | `build_action_list` with a weapon slot returns only `Drop` (and `Transfer` if applicable); no `Use` |
| `test_build_action_list_no_use_when_no_content`    | `build_action_list` with `game_content = None` returns no `Use` action                              |
| `test_panel_action_use_variant`                    | `PanelAction::Use { party_index: 0, slot_index: 2 }` round-trips through `Debug` and `PartialEq`    |
| `test_build_action_list_drop_transfer_unchanged`   | Existing Drop/Transfer behavior is intact after the signature change                                |

#### 2.9 Deliverables

- [ ] `UseItemExplorationAction { party_index: usize, slot_index: usize }` message struct added with SPDX-compatible doc comment and `#[derive(Message)]`.
- [ ] `PanelAction::Use { party_index, slot_index }` variant added as the first variant.
- [ ] `build_action_list` updated with `selected_slot_index`, `character`, and `game_content` parameters; returns `Use` as first action for consumable slots.
- [ ] `inventory_input_system` gains `Option<Res<GameContent>>` and `MessageWriter<UseItemExplorationAction>` parameters; `KeyCode::KeyU` shortcut dispatches `UseItemExplorationAction` from `SlotNavigation`; `Enter` in `ActionNavigation` dispatches `Use` when focused.
- [ ] `inventory_ui_system` gains `MessageWriter<UseItemExplorationAction>`; `PanelAction::Use` match arm dispatches via `use_writer`.
- [ ] `InventoryPlugin::build()` registers `UseItemExplorationAction`.
- [ ] UI hint text updated to mention `U: use consumable`.
- [ ] All Phase 2 tests pass.

#### 2.10 Success Criteria

- A player navigating to a consumable slot can press `U` or enter the action
  strip and select `Use` to emit `UseItemExplorationAction`.
- Non-consumable slots never show a `Use` action.
- The `Drop` and `Transfer` actions are unaffected in behavior, order (after
  `Use`), and test coverage.

---

### Phase 3: Handle Exploration/Menu Item Consumption and Feedback

Add `handle_use_item_action_exploration` — the Bevy system that reads
`UseItemExplorationAction`, validates via `validate_item_use_slot`, applies the
effect via `apply_consumable_effect`, consumes the charge or removes the slot,
resets navigation state, and writes `GameLog` feedback.

#### 3.1 Feature Work — Handler System

**File to edit:** `src/game/systems/inventory_ui.rs`

Add a new private system function after `inventory_action_system`:

```text
fn handle_use_item_action_exploration(
    mut reader: MessageReader<UseItemExplorationAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    game_content: Option<Res<GameContent>>,
    mut game_log: Option<ResMut<GameLog>>,
) {
```

**Required imports** to add at the top of the file:

```text
use crate::domain::combat::item_usage::{validate_item_use_slot, ItemUseError};
use crate::domain::items::consumable_usage::apply_consumable_effect;
use crate::domain::items::types::ItemType;
use crate::game::systems::ui::GameLog;
use crate::sdk::database::ContentDatabase;
```

**Logic per message** (execute the following steps in order; abort on any error
and write the appropriate `GameLog` message):

1. **Resolve `game_content`**: if `game_content` is `None`, write
   `"Cannot use item: game content not available."` to `game_log` and `return`.

2. **Bounds-check `party_index`**: if `party_index >= party.members.len()`,
   write `"Cannot use item: invalid character."` and `continue` to next message.

3. **Validate via `validate_item_use_slot`**: call
   `validate_item_use_slot(&character_snapshot, slot_index, content_db, false)`.
   On `Err(e)`, write the appropriate `GameLog` string from the table below and
   `continue`:

   | `ItemUseError` variant    | `GameLog` message                                                                              |
   | ------------------------- | ---------------------------------------------------------------------------------------------- |
   | `InventorySlotInvalid(_)` | `"Cannot use item: no item in that slot."`                                                     |
   | `ItemNotFound(_)`         | `"Cannot use item: item data not found."`                                                      |
   | `NotConsumable`           | `"Cannot use {item_name}: not a consumable."` (use item name if available, else `"that item"`) |
   | `NotUsableInCombat`       | `"Cannot use {item_name} outside of combat."`                                                  |
   | `NoCharges`               | `"Cannot use {item_name}: no charges remaining."`                                              |
   | `AlignmentRestriction`    | `"Cannot use {item_name}: alignment restriction."`                                             |
   | `ClassRestriction`        | `"Cannot use {item_name}: class restriction."`                                                 |
   | `RaceRestriction`         | `"Cannot use {item_name}: race restriction."`                                                  |
   | `InvalidTarget`           | `"Cannot use {item_name}: invalid target."`                                                    |
   | `Other(msg)`              | `"Cannot use item: {msg}."`                                                                    |

   Note: `NotUsableInCombat` is the error returned when `is_combat_usable: false`
   **and** `in_combat = true`. When calling with `in_combat = false`, this error
   will **not** be returned for `is_combat_usable: false` items. It would only
   appear if some future validation path is added; include the arm for
   completeness.

4. **Capture effect and item name**: borrow `character.inventory.items[slot_index]`,
   look up the item in `content_db.items`, extract `item.name` and
   `consumable_data.effect`. Do this in a short immutable borrow scope before
   mutating.

5. **Consume one charge**: in a mutable borrow of `character`:

   - If `slot.charges > 1`: `slot.charges -= 1`.
   - If `slot.charges == 1`: call `character.inventory.remove_item(slot_index)`.
   - If `slot.charges == 0`: write `"Cannot use {item_name}: no charges remaining."`
     and `continue` (this is a defensive check; `validate_item_use_slot` should
     have caught it).

6. **Apply effect**: call
   `let result = apply_consumable_effect(&mut character, effect);`

7. **Write success `GameLog` message** using the effect-specific template:

   | `ConsumableEffect` variant | `GameLog` message template                                                          |
   | -------------------------- | ----------------------------------------------------------------------------------- |
   | `HealHp(_)`                | `"{item_name} used. {character_name} recovered {result.healing} HP."`               |
   | `RestoreSp(_)`             | `"{item_name} used. {character_name} recovered {result.sp_restored} SP."`           |
   | `CureCondition(_)`         | `"{item_name} used. Conditions cleared."`                                           |
   | `BoostAttribute(attr, _)`  | `"{item_name} used. {character_name}'s {attr.display_name()} increased."`           |
   | `BoostResistance(res, _)`  | `"{item_name} used. {character_name}'s {res.display_name()} resistance increased."` |

   If `result.healing == 0` for `HealHp`, use `"{item_name} used. {character_name} was already at full health."`.
   If `result.sp_restored == 0` for `RestoreSp`, use `"{item_name} used. {character_name} was already at full SP."`.

8. **Reset navigation state** (exact fields in this order):

   ```text
   if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
       inv_state.selected_slot = None;
   }
   nav_state.selected_slot_index = None;
   nav_state.focused_action_index = 0;
   nav_state.phase = NavigationPhase::SlotNavigation;
   ```

#### 3.2 Plugin Registration

**File to edit:** `src/game/systems/inventory_ui.rs`, `InventoryPlugin::build()`
(L74–88).

Add `handle_use_item_action_exploration` to the **end** of the existing `.chain()`
system set so it runs after `inventory_action_system`:

```text
(
    inventory_input_system,
    inventory_ui_system,
    inventory_action_system,
    handle_use_item_action_exploration,
)
    .chain(),
```

This preserves the existing ordering guarantee while ensuring the new handler
sees messages written by `inventory_input_system` in the same frame.

#### 3.3 Configuration — `GameLog` Import

**File to edit:** `src/game/systems/inventory_ui.rs`

`GameLog` is defined in `src/game/systems/ui.rs`. It is already used in
`merchant_inventory_ui.rs` via:

```text
use crate::game::systems::ui::GameLog;
```

Add the same import to `inventory_ui.rs`.

#### 3.4 Testing Requirements

Add tests to `mod tests` in `src/game/systems/inventory_ui.rs`:

| Test name                                              | What it verifies                                                                                                                            |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_exploration_use_heals_character`                 | `UseItemExplorationAction` with a healing potion → `character.hp.current` increases, inventory slot removed, `GameLog` contains "recovered" |
| `test_exploration_use_restores_sp`                     | SP potion → `character.sp.current` increases, log contains "SP"                                                                             |
| `test_exploration_use_cures_condition`                 | Cure potion → condition bit cleared, log contains "Conditions cleared"                                                                      |
| `test_exploration_use_boosts_attribute`                | `BoostAttribute` potion → `stats.<attr>.current` increases                                                                                  |
| `test_exploration_use_boosts_resistance`               | `BoostResistance` potion → `resistances.<field>.current` increases                                                                          |
| `test_exploration_use_decrements_multi_charge_item`    | Item with `charges = 3` → after use `charges == 2`, slot still present                                                                      |
| `test_exploration_use_removes_last_charge`             | Item with `charges = 1` → slot removed entirely                                                                                             |
| `test_exploration_use_resets_nav_state`                | After successful use, `nav_state.phase == SlotNavigation` and `selected_slot_index == None`                                                 |
| `test_exploration_use_writes_game_log`                 | Successful use appends exactly one entry to `GameLog.messages`                                                                              |
| `test_exploration_use_invalid_slot_writes_log`         | `slot_index` beyond inventory length → `GameLog` contains "no item in that slot"                                                            |
| `test_exploration_use_non_consumable_writes_log`       | Weapon in slot → `GameLog` contains "not a consumable"                                                                                      |
| `test_exploration_use_zero_charges_writes_log`         | Item with `charges = 0` → `GameLog` contains "no charges"                                                                                   |
| `test_exploration_use_non_combat_usable_item_succeeds` | Item with `is_combat_usable: false` → use succeeds, effect applied, `GameLog` contains item name                                            |
| `test_exploration_use_invalid_party_index_writes_log`  | `party_index` beyond party size → `GameLog` contains "invalid character"                                                                    |

#### 3.5 Deliverables

- [ ] `handle_use_item_action_exploration` system added to `src/game/systems/inventory_ui.rs` with SPDX-compatible doc comment.
- [ ] System registered as the last entry in the `InventoryPlugin` `.chain()` set.
- [ ] `GameLog` import added to `src/game/systems/inventory_ui.rs`.
- [ ] All 14 Phase 3 tests pass.
- [ ] All four quality gates pass.

#### 3.6 Success Criteria

- A consumable can be used from the inventory screen in exploration and menu modes.
- Item charge consumption (decrement or remove) is correct for both single-charge
  and multi-charge items.
- Every outcome — success or failure — produces a visible `GameLog` entry.
- `is_combat_usable: false` items are correctly allowed in exploration.
- Navigation state is fully reset after every use attempt (success or failure).

---

### Phase 4: Harden Contracts, Docs, and Cross-Mode Regression Coverage

Finalize documentation, verify no regressions in the combat path, and run the
complete quality-gate matrix.

#### 4.1 Documentation Updates

**File to edit:** `src/domain/items/consumable_usage.rs`

Every `pub` symbol must have a `///` doc comment including:

- A one-line summary.
- An `# Arguments` section for functions.
- An `# Returns` section describing `ConsumableApplyResult` fields.
- A runnable `# Examples` doctest (these are compiled by `cargo nextest run`).

**File to edit:** `src/game/systems/inventory_ui.rs`

- Add doc comments to `UseItemExplorationAction` describing the self-target
  contract and the `party_index`/`slot_index` valid ranges.
- Update the `PanelAction` doc example to include the `Use` variant.
- Update the module-level doc comment key-routing table to include `U: use consumable`.

**File to edit:** `src/domain/combat/item_usage.rs`

- Update the module doc comment to note that effect application is now delegated
  to `apply_consumable_effect` in `src/domain/items/consumable_usage.rs`.
- Update the `execute_item_use_by_slot` doc comment to reference the shared helper.

#### 4.2 Finalize and Review

- Review `src/game/systems/combat.rs` `perform_use_item_action_with_rng` to
  confirm it still calls `execute_item_use_by_slot` (unchanged) and that there
  is no duplicated effect logic remaining.
- Search for any `ConsumableEffect` match arms outside `consumable_usage.rs` and
  `item_usage.rs` to confirm no stale copies exist.
- Update `docs/explanation/implementations.md` with a concise summary of the
  consumables-outside-combat feature once all phases are complete.

#### 4.3 Testing Requirements

Run all four quality gates after every phase and confirm clean output:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Add the following cross-mode regression tests in `src/domain/combat/item_usage.rs`
`mod tests`:

| Test name                                        | What it verifies                                                                                                                            |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_combat_still_rejects_non_combat_usable`    | After Phase 1 refactor, `execute_item_use_by_slot` with `is_combat_usable: false` item still returns `Err(ItemUseError::NotUsableInCombat)` |
| `test_combat_boost_attribute_via_shared_helper`  | `BoostAttribute` item used in combat → stat increase matches `apply_consumable_effect` output                                               |
| `test_combat_boost_resistance_via_shared_helper` | `BoostResistance` item used in combat → resistance increase matches `apply_consumable_effect` output                                        |

#### 4.4 Deliverables

- [ ] All `pub` symbols in `src/domain/items/consumable_usage.rs` have `///` doc comments and doctests.
- [ ] `UseItemExplorationAction` and `PanelAction::Use` have doc comments.
- [ ] Module doc comment in `src/domain/combat/item_usage.rs` references the shared helper.
- [ ] `docs/explanation/implementations.md` updated with feature summary.
- [ ] Zero stray `ConsumableEffect` match arms outside the two designated files.
- [ ] Three Phase 4 cross-mode regression tests pass.
- [ ] Full quality-gate run: zero errors, zero warnings, all tests pass.

#### 4.5 Success Criteria

- The feature is documented, testable, and architecture-consistent.
- Shared consumable logic has exactly one implementation (single source of truth).
- No regression in existing combat or inventory behavior.
- `cargo nextest run --all-features` reports all tests passed.

---

## Resolved Design Decisions

| Question                                                                  | Decision                                                                                                                                         |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Self-target only or cross-party targeting?                                | **Self-target only** for this plan. Cross-party targeting is a future phase.                                                                     |
| `CureCondition` — `conditions` bitflags only or also `active_conditions`? | **`conditions` bitflags only** via `character.conditions.remove(flags)`. `active_conditions` is not touched.                                     |
| Failure feedback — `GameLog` or `warn!()` only?                           | **`GameLog` always** for all validation failures. `warn!()` may be added for diagnostic context but is not a substitute for the `GameLog` entry. |

---

## File Change Summary

| File                                   | Change Type                                   | Phase |
| -------------------------------------- | --------------------------------------------- | ----- |
| `src/domain/items/consumable_usage.rs` | **Create**                                    | 1     |
| `src/domain/items/mod.rs`              | Edit — add `pub mod`, re-exports              | 1     |
| `src/domain/combat/item_usage.rs`      | Edit — delegate effect match                  | 1     |
| `src/game/systems/inventory_ui.rs`     | Edit — message, enum variants, systems, hints | 2, 3  |
| `docs/explanation/implementations.md`  | Edit — feature summary                        | 4     |
