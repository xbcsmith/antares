# Consumable Duration Effects Implementation Plan

## Overview

Antares already has the raw pieces for timed out-of-combat consumable effects: [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) stores party-wide minute countdowns, [`GameState::advance_time`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) ticks those counters, and [`ConditionDuration::Minutes`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/conditions.rs) exists for per-character effects. This plan extends `ConsumableData` with explicit durations, routes timed resistance potions through `GameState.active_spells`, adds reversible per-character timed attribute boosts, and finishes by exposing the duration field in the Campaign Builder and inventory-use flow.

## Current State Analysis

### Existing Infrastructure

- [`ConsumableData`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) currently contains only `effect` and `is_combat_usable`, and all existing combat and SDK code constructs it with those two fields.
- [`ConsumableEffect::BoostAttribute`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) and [`ConsumableEffect::BoostResistance`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) currently mutate `character.stats.*.current` and `character.resistances.*.current` directly inside [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs).
- [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) already stores minute countdowns for `fire_protection`, `cold_protection`, `electricity_protection`, `magic_protection`, `fear_protection`, `psychic_protection`, and related party-wide spell effects.
- [`GameState::advance_time`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) already advances the world clock and ticks `active_spells` once per minute.
- [`Character::tick_conditions_minute`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs) and [`ActiveCondition::tick_minute`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/conditions.rs) already support minute-based expiry for per-character timed state.
- [`rest_party_hour`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs) and [`rest_party`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs) already tick character minute conditions during long time jumps.
- [`sdk/campaign_builder/src/items_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs) already exposes consumable effect editing and preview text for `BoostAttribute` and `BoostResistance`.

### Identified Issues

- `ConsumableData` has no duration field, so timed and permanent consumables are indistinguishable in data files.
- Out-of-combat consumable execution cannot stay purely character-scoped if timed resistances must be stored on [`GameState.active_spells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs); a pure `Character` helper is insufficient for that branch.
- [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) is currently ticked but not obviously projected into effective resistance calculations elsewhere in the codebase, so simply writing a duration there is not enough to guarantee gameplay impact.
- Per-character timed attribute boosts need reversible expiry semantics; the current condition system stores timed condition identities, but it does not directly own an inline “attribute + delta” payload suitable for consumable-generated boosts.
- [`GameState::advance_time`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) does not currently tick every party member’s minute-based state, so any new per-character timed boost system would not expire from normal travel alone.
- [`docs/reference/architecture.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/reference/architecture.md) currently defines `ConsumableData` without `duration_minutes`, so this feature touches architecture-defined core data structures and needs explicit alignment during implementation.

## Implementation Phases

### Phase 1: Extend Consumable Data and Align Core Contracts

#### 1.1 Foundation Work

- Add `duration_minutes: Option<u16>` to [`ConsumableData`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) with `#[serde(default)]` so all existing RON files continue to deserialize as `None`.
- Update all `ConsumableData` struct literals across `src/`, `sdk/`, and tests to set `duration_minutes` explicitly or use a safe constructor/default pattern.
- Update public doc comments and doctests in [`src/domain/items/types.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs) to include duration-aware examples.

#### 1.2 Add Foundation Functionality

- Keep `None` as the approved “legacy / permanent” representation so existing content and combat behavior remain unchanged until later phases opt into timed handling.
- Define a small shared helper for clamping duration to `u8::MAX` when bridging `Option<u16>` into [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) fields.
- Normalize `duration_minutes = Some(0)` to `None` so the editor’s `0 = permanent` behavior and runtime semantics stay consistent.

#### 1.3 Integrate Foundation Work

- Audit all `ConsumableData` previews and serializers in [`sdk/campaign_builder/src/items_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs), [`src/sdk/templates.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/templates.rs), and related editor/test helpers so the added field does not create compile gaps.
- Reconcile the architecture definition in [`docs/reference/architecture.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/reference/architecture.md) before or alongside code changes because `ConsumableData` is architecture-defined.

#### 1.4 Testing Requirements

- Add serialization tests proving legacy consumable RON without `duration_minutes` deserializes as `None`.
- Add constructor/literal regression tests in `src/domain/items/types.rs`, `src/domain/items/database.rs`, and `sdk/campaign_builder` to ensure the new field is present and defaults as intended.

#### 1.5 Deliverables

- [ ] `ConsumableData` includes `duration_minutes: Option<u16>` with serde defaulting.
- [ ] All Rust struct literals compile with the new field.
- [ ] Architecture and doc comments reflect the new schema.

#### 1.6 Success Criteria

- Existing consumable data files and tests still deserialize cleanly.
- Timed consumables can now be expressed in data without changing legacy item behavior.

### Phase 2: Add Out-of-Combat Consumable Orchestration with Timed Resistance Routing

#### 2.1 Feature Work

- Introduce an exploration/menu consumable executor that operates at the application or game-system layer rather than only on `Character`, because timed resistance effects must mutate [`GameState.active_spells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs).
- Split responsibilities into two layers:
  - a pure character-focused helper for instant effects and combat-safe stat/resistance mutations;
  - an exploration-only orchestrator that can inspect `ConsumableData.duration_minutes` and choose `Character` mutation vs. `ActiveSpells` mutation.
- Route `BoostResistance` with `duration_minutes = Some(n)` to the mapped [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) field instead of directly mutating `character.resistances.*.current`.

#### 2.2 Integrate Feature

- Implement the resistance mapping exactly once in a dedicated helper, covering:
  - `Fire -> fire_protection`
  - `Cold -> cold_protection`
  - `Electricity -> electricity_protection`
  - `Energy -> magic_protection`
  - `Fear -> fear_protection`
  - `Physical -> magic_protection`
  - `Paralysis -> psychic_protection`
  - `Sleep -> psychic_protection`
- Preserve current combat behavior in [`execute_item_use_by_slot`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/item_usage.rs): combat may continue to write directly to `character.resistances` because combat state is ephemeral.
- Make permanent or duration-less resistance consumables follow an explicit rule: either keep direct resistance mutation for `None`, or forbid permanent out-of-combat resistance boosts until a stronger design is chosen.

#### 2.3 Configuration Updates

- Define repeat-use behavior explicitly: when a second resistance potion targets an already-active [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) field, overwrite the remaining duration with the new duration after `u8::MAX` clamping.
- Standardize the user-facing feedback text so timed protections clearly report both protection type and minutes applied.

#### 2.4 Testing Requirements

- Add exploration-use tests proving timed resistance consumables set the expected `active_spells` field and respect `u8::MAX` saturation.
- Add regression tests proving combat item use still mutates combatant resistances directly and ignores timed-expiry orchestration.
- Add tests covering repeated use on the same protection field according to the chosen stacking rule.

#### 2.5 Deliverables

- [ ] Exploration consumable execution can mutate `GameState.active_spells`.
- [ ] Timed resistance consumables no longer become permanent out of combat.
- [ ] Combat resistance consumables keep existing semantics.

#### 2.6 Success Criteria

- Out-of-combat resistance potions expire via time flow instead of permanently altering character state.
- The application layer, not the combat layer, owns timed protection routing.

### Phase 3: Make Timed Protections and Timed Attribute Boosts Expire Correctly

#### 3.1 Foundation Work

- Add a reversible per-character timed boost structure to [`Character`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs) as a dedicated `TimedStatBoost` list, rather than overloading [`ActiveCondition`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/conditions.rs) with synthetic consumable-only definitions.
- Each timed boost entry should store at minimum `attribute: AttributeType`, `amount: i8`, and `minutes_remaining: u16`.
- Add character methods that apply a timed boost, tick it once per minute, and reverse the `current` value mutation exactly once on expiry.

#### 3.2 Add Foundation Functionality

- Route `BoostAttribute` with `duration_minutes = Some(n)` through the new timed-boost list instead of applying a permanent change to `stats.*.current`.
- Keep `BoostAttribute` with `duration_minutes = None` as valid permanent behavior for backward compatibility, while making timed behavior opt-in via `Some(n)`.
- Add a projection/helper layer for [`ActiveSpells`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) so those countdowns are actually reflected in effective resistance checks, HUD state, or any other gameplay path that consumes protections.

#### 3.3 Integrate Foundation Work

- Update [`GameState::advance_time`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs) to tick every party member’s minute-based timed boosts in addition to ticking `active_spells`.
- Keep [`rest_party`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs) and [`rest_party_hour`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs) aligned with the same minute-expiry rules so long time jumps do not desynchronize timed consumables from normal travel.
- Audit combat-entry and combat-exit synchronization so temporary stat boosts on party members survive into combat and continue expiring only through time, not immediately on encounter start.

#### 3.4 Testing Requirements

- Add minute-tick tests proving timed stat boosts expire exactly when `minutes_remaining` reaches zero and restore the original `current` attribute value.
- Add `GameState::advance_time` tests proving both `active_spells` and per-character timed boosts tick during normal time advancement.
- Add rest/travel regression tests proving long time jumps expire temporary boosts and protections consistently.
- Add gameplay-path tests proving the effective resistance projection actually changes resistance-sensitive calculations while a timed protection is active.

#### 3.5 Deliverables

- [ ] `Character` owns reversible timed attribute boosts.
- [ ] `GameState::advance_time` ticks both party-wide and per-character timed consumable effects.
- [ ] `ActiveSpells` protections have an actual gameplay effect while active.

#### 3.6 Success Criteria

- Timed attribute potions expire cleanly without leaving permanent stat drift.
- Timed resistance potions matter during gameplay and expire automatically through time passage.

### Phase 4: Campaign Builder Support for Duration-Aware Consumables

#### 4.1 Feature Work

- Extend [`show_type_editor`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs) in the SDK item editor to show a `Duration (minutes)` `DragValue` for `BoostResistance` and `BoostAttribute` consumables only.
- Treat `0` in the editor as the chosen “permanent / None” representation and convert it consistently when saving back to [`ConsumableData`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/types.rs).
- Update the consumable preview text in [`sdk/campaign_builder/src/items_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs) so timed boosts render as `"(60 min)"` or equivalent beside the effect description.

#### 4.2 Integrate Feature

- Follow `sdk/AGENTS.md` ID rules: any new `ComboBox`, loop body, or duration-specific widgets in [`sdk/campaign_builder/src/items_editor.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs) must use stable `from_id_salt` and `push_id` patterns as required.
- Update SDK-side sample items, test fixtures, and template helpers that create consumables so the editor and preview paths exercise timed and permanent variants.

#### 4.3 Configuration Updates

- Keep duration editing hidden for instant consumables like `HealHp`, `RestoreSp`, and `CureCondition` to avoid implying unsupported semantics.
- Standardize preview wording across list, detail, and editor views so timed boosts display consistently.

#### 4.4 Testing Requirements

- Add SDK tests proving the item editor can load, edit, and preserve `duration_minutes`.
- Add preview-format tests covering both permanent and timed boost consumables.
- Add regression tests for widget-ID safety if new looped controls or combo boxes are introduced in the editor.

#### 4.5 Deliverables

- [ ] Campaign Builder can author timed attribute and resistance consumables.
- [ ] Editor preview clearly shows duration when present.
- [ ] SDK tests cover the new consumable duration field.

#### 4.6 Success Criteria

- Timed consumable authoring is available in the supported editor workflow.
- The SDK accurately round-trips permanent and timed consumable data.

### Phase 5: Integrate Inventory “Use” Flow with Duration-Aware Out-of-Combat Items

#### 5.1 Feature Work

- Reconcile this plan with the existing out-of-combat use plan in [`docs/explanation/consumables_outside_combat_implementation_plan.md`](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/consumables_outside_combat_implementation_plan.md).
- Update the inventory-side use flow in [`src/game/systems/inventory_ui.rs`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/inventory_ui.rs) to call the new exploration/application-level consumable executor rather than a pure `Character`-only helper.
- Ensure status/log messaging distinguishes instant recovery from timed protection or timed stat-boost application.

#### 5.2 Integrate Feature

- Consume charges and reset inventory selection state consistently whether the effect is instant, timed party-wide, or timed per-character.
- Ensure `is_combat_usable: false` remains blocked in combat and allowed in exploration, even when `duration_minutes` is present.

#### 5.3 Configuration Updates

- Standardize success messages for timed boosts, for example:
  - `"Fire Resistance active for 60 minutes."`
  - `"Might increased by 5 for 30 minutes."`
- Standardize failure messages for unsupported permanent timed-effect combinations if the implementation chooses to reject them.

#### 5.4 Testing Requirements

- Add end-to-end inventory-use tests covering:
  - instant healing consumables;
  - timed resistance consumables;
  - timed attribute consumables;
  - combat-only restriction boundaries.
- Add regression tests ensuring charge consumption and inventory-slot removal still work for timed items.

#### 5.5 Deliverables

- [ ] Inventory “Use” flow supports duration-aware consumables out of combat.
- [ ] Feedback strings clearly communicate timed effect application.
- [ ] Inventory regressions are covered for timed and non-timed consumables.

#### 5.6 Success Criteria

- Players can consume timed out-of-combat items from inventory and see the effect persist for the expected number of in-game minutes.
- The inventory flow uses the finalized timed-effect architecture rather than duplicating logic.

## Design Decisions

1. `duration_minutes = None` remains valid and preserves permanent out-of-combat boost behavior for backward compatibility.
2. Repeated use of the same timed resistance consumable overwrites the remaining duration after `u8::MAX` clamping.
3. Timed attribute consumables will use a dedicated `TimedStatBoost` field on [`Character`](/Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs), not the existing definition-driven condition system.
