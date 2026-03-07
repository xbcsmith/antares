# Consumables Outside Combat Implementation Plan

## Overview

Antares already supports consumable execution in combat through [`UseItemAction`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/combat.rs) and [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs), but the inventory flow in [`GameMode::Inventory`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) only exposes Drop and Transfer. This plan adds a parallel exploration/menu item-use path that reuses a pure domain consumable executor, preserves the combat-only restriction on `is_combat_usable: false`, consumes charges correctly, and surfaces player-facing feedback through the existing [`GameLog`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/ui.rs).

## Current State Analysis

### Existing Infrastructure

- [`ConsumableData`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) already models both `effect: ConsumableEffect` and `is_combat_usable: bool`, matching the architecture definition in [`docs/reference/architecture.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/reference/architecture.md).
- [`validate_item_use_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) already accepts `in_combat: bool`, so the restriction rule for `is_combat_usable` is centralized and exploration can intentionally pass `false`.
- [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) already contains the authoritative effect logic for `HealHp`, `RestoreSp`, `CureCondition`, `BoostAttribute`, and `BoostResistance`, but that logic is embedded inside combat-state mutation.
- [`InventoryPlugin`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) already registers inventory messages and routes keyboard input through [`inventory_input_system`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) and [`inventory_action_system`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs).
- [`PanelAction`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) and `build_action_list()` currently model only `Drop` and `Transfer`, so the action row has no concept of a context-sensitive Use action.
- [`GameLog`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/ui.rs) is already used by non-combat systems such as [`merchant_inventory_ui`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/merchant_inventory_ui.rs), [`rest`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/rest.rs), and [`dialogue`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue.rs) for visible out-of-combat feedback.

### Identified Issues

- The current inventory action list cannot emit any use-item message from [`inventory_input_system`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs), so consumables are unreachable outside combat.
- Combat owns the only consumable-effect executor, which risks logic drift if exploration adds a second handwritten implementation.
- Charge consumption currently happens inside combat-specific code, so there is no exploration-safe path that both mutates the target character and removes or decrements the inventory slot.
- Player feedback for out-of-combat item use is absent; relying on `info!()` would be invisible in normal UI flows.
- The current tests cover drop/transfer inventory behavior and combat item use, but they do not verify exploration usage, combat restriction boundaries, or charge-removal behavior from the inventory screen.

## Implementation Phases

### Phase 1: Extract Shared Consumable Domain Logic

#### 1.1 Foundation Work

- Create a shared pure domain helper for consumable application in the items domain, preferably as [`src/domain/items/consumable_usage.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/mod.rs), and re-export it from [`src/domain/items/mod.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/mod.rs).
- Define an explicit result type for out-of-combat effect application so callers can distinguish healing, SP restoration, cured conditions, and pure stat/resistance changes without re-deriving deltas from side effects.
- Move the effect-switching logic now embedded in [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) behind `apply_consumable_effect(character: &mut Character, effect: ConsumableEffect)` so combat and exploration share one effect implementation.

#### 1.2 Add Foundation Functionality

- Make `apply_consumable_effect` mutate only [`Character`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs) fields, preserving the AttributePair pattern for `hp`, `sp`, `stats.*`, and `resistances.*`.
- Keep healing and SP restoration capped at `base`, matching the current combat behavior in [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs).
- Keep `CureCondition` limited to bitflag removal on `character.conditions` for parity with the existing combat behavior; do not silently expand scope to `active_conditions` unless a separate design decision is made.
- Route combat’s item executor through the new helper instead of keeping a private copy of the effect match.

#### 1.3 Integrate Foundation Work

- Leave [`validate_item_use_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) as the central permission gate and continue using its `in_combat` parameter to enforce `is_combat_usable`.
- Add domain-level documentation and doctests for the new helper and result type so later exploration and combat callers share one documented contract.
- Keep combat-only responsibilities in [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs): target resolution, charge consumption on the acting combatant, turn advancement, and combat-end checks.

#### 1.4 Testing Requirements

- Add pure domain tests covering all `ConsumableEffect` variants against [`Character`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs): heal capped at base, SP capped at base, condition flags removed, attribute current values changed without mutating base, and resistance current values changed without mutating base.
- Add a combat regression test proving [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) still behaves identically after delegating to the shared helper.
- Add an explicit validation test showing `validate_item_use_slot(..., false)` permits `is_combat_usable: false` consumables while `validate_item_use_slot(..., true)` rejects them.

#### 1.5 Deliverables

- [ ] Shared `apply_consumable_effect` domain helper exists outside the combat-only API.
- [ ] Combat item-use code delegates effect application to the shared helper.
- [ ] Domain tests cover all consumable variants and `is_combat_usable` mode boundaries.

#### 1.6 Success Criteria

- All consumable effects have a single authoritative domain implementation.
- Combat and exploration can share consumable semantics without duplicated effect logic.
- The `is_combat_usable` rule is enforced by mode, not by item type.

### Phase 2: Add Inventory-Level Use Actions and Input

#### 2.1 Feature Work

- Introduce an exploration/menu item-use message in [`src/game/systems/inventory_ui.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs), either a new message dedicated to exploration or a mode-specific sibling to combat’s [`UseItemAction`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/combat.rs).
- Extend [`PanelAction`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) with a `Use` variant that carries `party_index` and `slot_index`.
- Update `build_action_list()` so consumable slots include `Use` before Drop/Transfer, while non-consumables preserve the current Drop/Transfer-only behavior.

#### 2.2 Integrate Feature

- Update [`inventory_input_system`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) so `Enter` in action mode can dispatch the new use action when the focused action is `Use`.
- Add direct keybind support for `U` and `Enter` from slot-navigation mode when the focused slot contains a consumable, without requiring the player to cycle through actions first.
- Update the inventory hint text and selected-item status line in [`inventory_ui_system`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) so the new Use affordance is discoverable.

#### 2.3 Configuration Updates

- Preserve the existing close/toggle behavior owned by [`src/game/systems/input.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/input.rs); do not duplicate the inventory-open key handling in the inventory plugin.
- Keep keyboard handling scoped to [`GameMode::Inventory`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) so exploration movement input and combat input remain isolated.

#### 2.4 Testing Requirements

- Add inventory UI tests verifying `build_action_list()` exposes `Use` only for consumables.
- Add input-system tests covering `U` and `Enter` dispatch for consumables, and confirming non-consumables still do not emit a use action.
- Add a regression test confirming Drop and Transfer behavior remain intact after the action-row expansion.

#### 2.5 Deliverables

- [ ] Inventory action model includes a consumable `Use` action.
- [ ] Keyboard routing supports out-of-combat item use from the inventory screen.
- [ ] Inventory UI text reflects the new action and keybinds.

#### 2.6 Success Criteria

- A player can choose a consumable from the inventory UI without entering combat.
- Non-consumables still expose only valid actions.
- Existing inventory navigation and transfer/drop semantics remain unchanged.

### Phase 3: Handle Exploration/Menu Item Consumption and Feedback

#### 3.1 Feature Work

- Add a new Bevy system, for example `handle_use_item_action_exploration`, in [`src/game/systems/inventory_ui.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) or a small adjacent inventory-actions module.
- In that system, resolve the acting party member from `party_index`, validate the selected slot through [`validate_item_use_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs) with `in_combat = false`, then consume one charge or remove the slot exactly once.
- Apply the selected consumable’s effect to the same character by calling the shared `apply_consumable_effect` helper, because the current inventory UI is character-centric and does not yet support target selection.

#### 3.2 Integrate Feature

- Register the new message and system in [`InventoryPlugin`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) alongside Drop and Transfer.
- Reset [`InventoryNavigationState`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) and `InventoryState.selected_slot` consistently after a successful use so keyboard focus does not remain on a removed slot.
- Write visible feedback to [`GameLog`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/ui.rs), following the same resource pattern used by [`merchant_inventory_ui`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/merchant_inventory_ui.rs) and [`rest`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/rest.rs).

#### 3.3 Configuration Updates

- Standardize the log strings for all outcome classes: successful consumption, invalid selection, blocked use, and no charges left.
- Keep failures non-fatal and user-visible; exploration use should not panic or silently do nothing when validation fails.

#### 3.4 Testing Requirements

- Add system tests proving a successful exploration use mutates character state, decrements or removes the inventory slot, resets UI selection, and appends a `GameLog` message.
- Add failure-path tests for invalid slot indices, non-consumables, zero charges, and combat-only restrictions when called from exploration.
- Add a targeted test for an `is_combat_usable: false` item showing the exploration system allows it and consumes it successfully.

#### 3.5 Deliverables

- [ ] Exploration-side item-use system exists and is registered in the inventory plugin.
- [ ] Inventory charge consumption/removal works outside combat.
- [ ] Out-of-combat use writes visible `GameLog` feedback.

#### 3.6 Success Criteria

- Consumables can be used from the inventory screen in menu/exploration flows.
- Item consumption, state mutation, and player feedback all happen in one frame.
- Combat-only rules are enforced correctly without blocking exploration-only consumables.

### Phase 4: Harden Contracts, Docs, and Cross-Mode Regression Coverage

#### 4.1 Foundation Work

- Add or update `///` documentation on any new public message types, result structs, and helper functions introduced in Phases 1 through 3.
- Document the mode-specific contract explicitly: combat uses targetable item execution plus turn advancement; exploration uses self-targeted item execution with no combat-state dependency.

#### 4.2 Integrate Feature

- Update [`docs/explanation/implementations.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/implementations.md) with a concise summary of the finished consumables-outside-combat work once implementation is complete.
- Review [`src/game/systems/combat.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/combat.rs) and [`src/game/systems/inventory_ui.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) for any duplicated item-use text or assumptions that should instead reference the shared helper.

#### 4.3 Testing Requirements

- Run the required quality gates after implementation: `cargo fmt --all`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo nextest run --all-features`.
- Add a cross-mode regression test matrix covering at least one example for each consumable effect variant in exploration or menu flow.
- Add a regression test ensuring combat still rejects `is_combat_usable: false` consumables even after the shared helper is introduced.

#### 4.4 Deliverables

- [ ] Public APIs and messages are documented with doc comments and examples.
- [ ] Implementation summary is recorded in `docs/explanation/implementations.md`.
- [ ] Full quality-gate and regression coverage exists for combat and exploration item use.

#### 4.5 Success Criteria

- The feature is documented, testable, and architecture-consistent.
- Shared consumable logic remains single-source across combat and exploration.
- No regression is introduced in existing combat or inventory behavior.

## Open Questions

1. Should out-of-combat use remain self-target only in Phase 1, or should the inventory UI also support choosing another party member as the target for healing and cure consumables?
2. Should `CureCondition(u8)` remove only `character.conditions` bitflags for parity with combat, or should implementation also clear matching entries in `active_conditions` when a mapping exists?
3. Should exploration-use failures always write to [`GameLog`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/ui.rs), or should some low-level validation failures stay as `warn!()` only to avoid log spam?
