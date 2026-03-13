# Consumable Duration Effects Implementation Plan

## Overview

Antares already has the raw pieces for timed out-of-combat consumable effects:
[`ActiveSpells`](../../src/application/mod.rs) stores party-wide `u8` minute
countdowns, [`GameState::advance_time`](../../src/application/mod.rs) ticks
those counters once per minute, and
[`ConditionDuration::Minutes`](../../src/domain/conditions.rs) exists for
per-character effects. However `ConsumableData` has no duration field, the
`BoostAttribute` / `BoostResistance` paths permanently mutate character state
instead of routing through the timed infrastructure, and neither `Character`
nor `GameState` expose the hooks needed to let consumable effects expire.

This plan adds `duration_minutes: Option<u16>` to `ConsumableData`, introduces
a dedicated `TimedStatBoost` structure on `Character`, routes timed resistance
potions through `GameState.active_spells`, wires expiry into
`GameState::advance_time`, and finishes by exposing duration authoring in the
Campaign Builder and the inventory-use flow.

**Prerequisite:** The
[`consumables_outside_combat_implementation_plan.md`](./consumables_outside_combat_implementation_plan.md)
plan must be **fully implemented** before this plan begins. That plan creates
`src/domain/items/consumable_usage.rs` (with `apply_consumable_effect` and
`ConsumableApplyResult`), the `UseItemExplorationAction` Bevy message,
`PanelAction::Use`, and `handle_use_item_action_exploration`. Phases 2–5 of
this plan build directly on those deliverables.

---

## Current State Analysis

### Existing Infrastructure

| Symbol                                                                              | File                                                  | Notes                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ConsumableData { effect, is_combat_usable }`                                       | `src/domain/items/types.rs` L276–281                  | Two fields only; no duration                                                                                                                                                                                                                                                                                                          |
| `ConsumableEffect::{HealHp,RestoreSp,CureCondition,BoostAttribute,BoostResistance}` | `src/domain/items/types.rs` L285–293                  | All five variants already exist                                                                                                                                                                                                                                                                                                       |
| `ResistanceType::{Physical,Fire,Cold,Electricity,Energy,Paralysis,Fear,Sleep}`      | `src/domain/items/types.rs` L339–356                  | All eight variants present                                                                                                                                                                                                                                                                                                            |
| `AttributeType::{Might,Intellect,Personality,Endurance,Speed,Accuracy,Luck}`        | `src/domain/items/types.rs` L297–305                  | All seven variants present                                                                                                                                                                                                                                                                                                            |
| `apply_consumable_effect`                                                           | `src/domain/items/consumable_usage.rs`                | Created by prerequisite plan; permanent mutation only                                                                                                                                                                                                                                                                                 |
| `ConsumableApplyResult`                                                             | `src/domain/items/consumable_usage.rs`                | Created by prerequisite plan                                                                                                                                                                                                                                                                                                          |
| `ActiveSpells` (18 `u8` fields)                                                     | `src/application/mod.rs` L266–303                     | Ticked by `advance_time`; fields: `fear_protection`, `cold_protection`, `fire_protection`, `poison_protection`, `acid_protection`, `electricity_protection`, `magic_protection`, `light`, `leather_skin`, `levitate`, `walk_on_water`, `guard_dog`, `psychic_protection`, `bless`, `invisibility`, `shield`, `power_shield`, `cursed` |
| `ActiveSpells::tick`                                                                | `src/application/mod.rs` L331–350                     | Decrements all fields by 1 via `saturating_sub` each minute                                                                                                                                                                                                                                                                           |
| `GameState::advance_time`                                                           | `src/application/mod.rs` L1462–1476                   | Calls `active_spells.tick()` once per minute; does **not** tick per-character timed boosts                                                                                                                                                                                                                                            |
| `Character::tick_conditions_minute`                                                 | `src/domain/character.rs` L1109–1111                  | Ticks `active_conditions` minute duration; no timed stat boost support                                                                                                                                                                                                                                                                |
| `ActiveCondition::tick_minute`                                                      | `src/domain/conditions.rs`                            | Returns `true` (expired) when `ConditionDuration::Minutes` reaches 0                                                                                                                                                                                                                                                                  |
| `rest_party_hour`                                                                   | `src/domain/resources.rs`                             | Advances time per hour but does **not** call per-character timed boost expiry                                                                                                                                                                                                                                                         |
| `execute_item_use_by_slot`                                                          | `src/domain/combat/item_usage.rs` L248–496            | Combat executor; delegates to `apply_consumable_effect` after prerequisite plan                                                                                                                                                                                                                                                       |
| `handle_use_item_action_exploration`                                                | `src/game/systems/inventory_ui.rs`                    | Exploration executor added by prerequisite plan; calls `apply_consumable_effect`                                                                                                                                                                                                                                                      |
| `show_type_editor` / `ItemsEditorState`                                             | `sdk/campaign_builder/src/items_editor.rs` L1092–1450 | Renders consumable effect editor; no duration widget                                                                                                                                                                                                                                                                                  |
| `data/items.ron`, `data/test_campaign/data/items.ron`                               | RON data files                                        | Consumable entries use two-field `ConsumableData`; must remain deserializable after schema change                                                                                                                                                                                                                                     |
| `ConsumableData` in `docs/reference/architecture.md`                                | L803–806                                              | Architecture-defined struct; must be updated to include `duration_minutes`                                                                                                                                                                                                                                                            |

### Identified Issues

1. **No duration field.** `ConsumableData` has no `duration_minutes` field; timed
   and permanent consumables are indistinguishable in data files.

2. **Permanent stat mutation.** `apply_consumable_effect` for `BoostAttribute` and
   `BoostResistance` calls `character.stats.<field>.modify()` /
   `character.resistances.<field>.modify()` directly. These mutations are never
   reversed, making all attribute/resistance boosts permanent regardless of intent.

3. **`ActiveSpells` not projected into gameplay.** `active_spells.*_protection`
   fields are ticked by `advance_time` but their non-zero values are never
   consulted in damage, resistance, or combat calculations anywhere in the
   codebase. Writing a duration there provides countdown semantics but no actual
   gameplay effect until a projection layer is added.

4. **`advance_time` does not tick per-character state.** `GameState::advance_time`
   ticks `active_spells` but never calls `tick_conditions_minute` (or any future
   equivalent) on party members. New per-character timed boosts would silently
   fail to expire during normal exploration travel.

5. **No reversible per-character timed boost structure.** `Character` has no field
   for tracking inline `(attribute, delta, minutes_remaining)` triples that can
   be applied and reversed. Overloading `active_conditions` with synthetic
   condition definitions is possible but adds fragility (condition IDs must be
   invented, and the condition database would need to be extended).

6. **`rest_party_hour` gap.** `rest_party_hour` in `src/domain/resources.rs`
   advances time but does not call per-character timed boost expiry, creating a
   desync risk during long rests.

7. **`ConsumableData` struct literals are widespread.** Adding a required field
   without `#[serde(default)]` would break every RON file and every Rust test
   helper that constructs `ConsumableData` with struct literal syntax. There are
   approximately 15–20 call sites across `src/`, `sdk/`, `data/*.ron`, and
   `data/test_campaign/`.

8. **Architecture document out of date.** `docs/reference/architecture.md` L803–806
   defines `ConsumableData` without `duration_minutes`. The AGENTS.md rule
   requires architecture compliance; the document must be updated alongside the
   code change.

---

## Implementation Phases

### Phase 1: Extend `ConsumableData` and Align Core Contracts

Add `duration_minutes: Option<u16>` to `ConsumableData` with `#[serde(default)]`
so all existing RON files and Rust struct literals continue to compile and
deserialize correctly. Update every affected struct literal to include the new
field. Update architecture documentation. No behavior changes in this phase.

#### 1.1 Foundation Work

**File to edit:** `src/domain/items/types.rs`

1. Replace the `ConsumableData` struct (L276–281) with the following. Preserve
   the SPDX header and all existing doc comments; add only the new field:

   ```text
   pub struct ConsumableData {
       /// Effect when consumed
       pub effect: ConsumableEffect,
       /// Whether usable during combat
       pub is_combat_usable: bool,
       /// Optional duration in in-game minutes.
       ///
       /// - `None` — effect is permanent (legacy behavior; backward compatible).
       /// - `Some(0)` — normalized to `None` at application time; treat as permanent.
       /// - `Some(n)` — effect expires after `n` in-game minutes.
       ///
       /// Only meaningful for `BoostAttribute` and `BoostResistance` effects.
       /// `HealHp`, `RestoreSp`, and `CureCondition` are instant and ignore this field.
       #[serde(default)]
       pub duration_minutes: Option<u16>,
   }
   ```

2. Update the existing doc example for `ConsumableData` (around L268–275) to
   include `duration_minutes: None` so the doctest still compiles:

   ```text
   /// let healing_potion = ConsumableData {
   ///     effect: ConsumableEffect::HealHp(20),
   ///     is_combat_usable: true,
   ///     duration_minutes: None,
   /// };
   ```

3. Add a second doc example showing a timed boost consumable:

   ```text
   /// let fire_resist_potion = ConsumableData {
   ///     effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
   ///     is_combat_usable: false,
   ///     duration_minutes: Some(60),
   /// };
   ```

**File to edit:** `docs/reference/architecture.md`

Update the `ConsumableData` definition (L803–806) to add `duration_minutes`:

```text
pub struct ConsumableData {
    pub effect: ConsumableEffect,      // What the consumable does
    pub is_combat_usable: bool,        // Can be used during combat
    pub duration_minutes: Option<u16>, // None = permanent; Some(n) = expires after n minutes
}
```

#### 1.2 Update All Struct Literal Call Sites

Search for every `ConsumableData {` construction across the codebase and add
`duration_minutes: None` to each. The complete list of files containing struct
literals (verified by `grep -r "ConsumableData {" src/ sdk/ data/`):

| File                                       | Approximate lines                                            | Action                       |
| ------------------------------------------ | ------------------------------------------------------------ | ---------------------------- |
| `src/domain/items/types.rs`                | doc examples                                                 | Updated in §1.1              |
| `src/domain/combat/item_usage.rs`          | test helpers ~L507–550                                       | Add `duration_minutes: None` |
| `src/domain/combat/engine.rs`              | `make_consumable_item` ~L1323–1333                           | Add `duration_minutes: None` |
| `src/domain/items/database.rs`             | test helpers ~L955–965                                       | Add `duration_minutes: None` |
| `src/domain/items/equipment_validation.rs` | test helper ~L459–469                                        | Add `duration_minutes: None` |
| `src/domain/visual/item_mesh.rs`           | `make_consumable` ~L1859–1869                                | Add `duration_minutes: None` |
| `src/game/systems/combat.rs`               | `test_perform_use_item_action_*` ~L3383–3393                 | Add `duration_minutes: None` |
| `src/sdk/templates.rs`                     | `healing_potion` ~L374–384, `sp_potion` ~L407–417            | Add `duration_minutes: None` |
| `src/bin/item_editor.rs`                   | `create_consumable` ~L346–350                                | Add `duration_minutes: None` |
| `sdk/campaign_builder/src/items_editor.rs` | `default_item` ~L121–143, `show_type_editor` effect defaults | Add `duration_minutes: None` |

**RON data files** — these use `#[serde(default)]` and do **not** need manual
edits. Verify deserialization still succeeds in the Phase 1 tests.

#### 1.3 Normalize `Some(0)` to `None`

Add a standalone pure function to `src/domain/items/types.rs` (placed after
the `ConsumableData` struct definition):

````text
/// Normalizes a raw `duration_minutes` value.
///
/// `Some(0)` is treated as permanent (`None`) so that editor inputs of `0`
/// and omitted RON fields both produce identical runtime semantics.
///
/// # Examples
///
/// ```
/// use antares::domain::items::types::normalize_duration;
/// assert_eq!(normalize_duration(Some(0)), None);
/// assert_eq!(normalize_duration(Some(60)), Some(60));
/// assert_eq!(normalize_duration(None), None);
/// ```
pub fn normalize_duration(raw: Option<u16>) -> Option<u16> {
    match raw {
        Some(0) | None => None,
        other => other,
    }
}
````

Re-export `normalize_duration` from `src/domain/items/mod.rs` alongside the
existing `pub use types::` block.

#### 1.4 Testing Requirements

Add the following tests inside the `mod tests` block in
`src/domain/items/types.rs`:

| Test name                                                     | What it verifies                                                                                                      |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `test_consumable_data_duration_defaults_none_in_ron`          | Deserializing `"Consumable((effect: HealHp(20), is_combat_usable: true))"` from RON produces `duration_minutes: None` |
| `test_consumable_data_duration_some_round_trips`              | Serializing then deserializing `duration_minutes: Some(60)` preserves the value                                       |
| `test_normalize_duration_zero_becomes_none`                   | `normalize_duration(Some(0)) == None`                                                                                 |
| `test_normalize_duration_none_stays_none`                     | `normalize_duration(None) == None`                                                                                    |
| `test_normalize_duration_positive_unchanged`                  | `normalize_duration(Some(30)) == Some(30)`                                                                            |
| `test_consumable_data_struct_literal_compiles_with_new_field` | Construct `ConsumableData` with all three fields; assert `duration_minutes == None`                                   |

#### 1.5 Deliverables

- [ ] `ConsumableData` in `src/domain/items/types.rs` includes `duration_minutes: Option<u16>` with `#[serde(default)]`.
- [ ] `normalize_duration` pure function added to `src/domain/items/types.rs` and re-exported from `src/domain/items/mod.rs`.
- [ ] All struct literals in `src/`, `sdk/`, and `src/bin/` compile with the new field.
- [ ] `docs/reference/architecture.md` updated at L803–806 to include `duration_minutes`.
- [ ] All 6 Phase 1 tests pass.
- [ ] All four quality gates pass with zero errors and zero warnings:
  ```bash
  cargo fmt --all
  cargo check --all-targets --all-features
  cargo clippy --all-targets --all-features -- -D warnings
  cargo nextest run --all-features
  ```

#### 1.6 Success Criteria

- Existing RON consumable data files (`data/items.ron`,
  `data/test_campaign/data/items.ron`, `campaigns/tutorial/data/items.ron` if
  present) deserialize without modification with `duration_minutes == None`.
- Timed consumables can now be expressed in data files as
  `duration_minutes: Some(60)` without changing any legacy item behavior.
- `normalize_duration` is importable as `antares::domain::items::normalize_duration`.

---

### Phase 2: Add `TimedStatBoost` to `Character` and Wire Expiry

Introduce a reversible per-character timed boost structure on `Character` so
that `BoostAttribute` consumables with `duration_minutes: Some(n)` can be
applied, tracked, and automatically reversed. Update `GameState::advance_time`
and `rest_party_hour` to tick per-character boosts during normal time passage.

#### 2.1 Foundation Work — New `TimedStatBoost` type

**File to edit:** `src/domain/character.rs`

1. Add the following new public struct **before** the `Character` struct
   definition (around L953). Place it after the `QuestFlags` impl block:

   ````text
   /// A reversible timed attribute boost applied by a consumable item.
   ///
   /// When `minutes_remaining` reaches zero the boost is reversed by subtracting
   /// `amount` from `stats.<attribute>.current` (or `resistances.<field>.current`
   /// for resistance boosts).
   ///
   /// # Examples
   ///
   /// ```
   /// use antares::domain::character::TimedStatBoost;
   /// use antares::domain::items::types::AttributeType;
   ///
   /// let boost = TimedStatBoost {
   ///     attribute: AttributeType::Might,
   ///     amount: 5,
   ///     minutes_remaining: 30,
   /// };
   /// assert_eq!(boost.minutes_remaining, 30);
   /// ```
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct TimedStatBoost {
       /// Which attribute this boost modifies.
       pub attribute: AttributeType,
       /// Signed delta applied to `current` (positive = boost, negative = penalty).
       pub amount: i8,
       /// Minutes remaining before the boost expires and is reversed.
       pub minutes_remaining: u16,
   }
   ````

2. Add `timed_stat_boosts: Vec<TimedStatBoost>` to the `Character` struct, with
   `#[serde(default)]` so existing save files deserialize without the field:

   ```text
   /// Active timed attribute boosts from consumable items.
   /// Each entry is reversed when `minutes_remaining` reaches zero.
   #[serde(default)]
   pub timed_stat_boosts: Vec<TimedStatBoost>,
   ```

   Place this field after `active_conditions` (currently at L990) to keep
   related timed-effect fields adjacent.

3. Initialize the field in `Character::new` (L1035–1071):

   ```text
   timed_stat_boosts: Vec::new(),
   ```

4. Add the following methods to `impl Character` (after `tick_conditions_minute`
   at L1109–1111):

   ```text
   /// Applies a timed attribute boost and records it for later reversal.
   ///
   /// Calls `normalize_duration` on `duration_minutes` before storing, so
   /// `Some(0)` behaves identically to `None` (no boost is applied).
   ///
   /// # Arguments
   ///
   /// * `attr` — the attribute to boost
   /// * `amount` — signed delta (positive = increase, negative = decrease)
   /// * `duration_minutes` — `Some(n)` for a timed boost; `None` or `Some(0)` = permanent (no entry stored)
   pub fn apply_timed_stat_boost(
       &mut self,
       attr: AttributeType,
       amount: i8,
       duration_minutes: Option<u16>,
   ) {
       use crate::domain::items::types::normalize_duration;
       let Some(minutes) = normalize_duration(duration_minutes) else { return; };
       // Apply the delta to current value only
       self.apply_attribute_delta(attr, amount as i16);
       self.timed_stat_boosts.push(TimedStatBoost {
           attribute: attr,
           amount,
           minutes_remaining: minutes,
       });
   }

   /// Ticks all timed stat boosts by one minute.
   ///
   /// Boosts whose `minutes_remaining` reaches zero are reversed: the `amount`
   /// is subtracted from the corresponding `current` attribute value.
   pub fn tick_timed_stat_boosts_minute(&mut self) {
       let mut expired: Vec<TimedStatBoost> = Vec::new();
       self.timed_stat_boosts.retain_mut(|boost| {
           if boost.minutes_remaining > 0 {
               boost.minutes_remaining -= 1;
           }
           if boost.minutes_remaining == 0 {
               expired.push(boost.clone());
               false
           } else {
               true
           }
       });
       for boost in expired {
           // Reverse: subtract the original delta
           self.apply_attribute_delta(boost.attribute, -(boost.amount as i16));
       }
   }
   ```

5. Add the private helper `apply_attribute_delta` to `impl Character`. This
   centralizes the `AttributeType`-to-field mapping used by both
   `apply_timed_stat_boost` and the reversal logic:

   ```text
   /// Applies a signed delta to the `current` value of the named attribute.
   ///
   /// This is the single authoritative mapping from `AttributeType` to a
   /// `Character` field. Used by timed-boost apply and reversal.
   fn apply_attribute_delta(&mut self, attr: AttributeType, delta: i16) {
       match attr {
           AttributeType::Might      => self.stats.might.modify(delta),
           AttributeType::Intellect  => self.stats.intellect.modify(delta),
           AttributeType::Personality => self.stats.personality.modify(delta),
           AttributeType::Endurance  => self.stats.endurance.modify(delta),
           AttributeType::Speed      => self.stats.speed.modify(delta),
           AttributeType::Accuracy   => self.stats.accuracy.modify(delta),
           AttributeType::Luck       => self.stats.luck.modify(delta),
       }
   }
   ```

#### 2.2 Wire Expiry into `GameState::advance_time`

**File to edit:** `src/application/mod.rs`

In `GameState::advance_time` (L1462–1476), extend the per-minute loop to also
tick each party member's timed stat boosts:

```text
pub fn advance_time(&mut self, minutes: u32, templates: Option<...>) {
    self.time.advance_minutes(minutes);
    for _ in 0..minutes {
        self.active_spells.tick();
        // Phase 2: tick per-character timed stat boosts
        for member in &mut self.party.members {
            member.tick_timed_stat_boosts_minute();
        }
    }
    if let Some(tmpl) = templates {
        self.npc_runtime.tick_restock(&self.time, tmpl);
    }
}
```

#### 2.3 Wire Expiry into `rest_party_hour`

**File to edit:** `src/domain/resources.rs`

`rest_party_hour` advances time in hour-sized chunks. It must tick per-character
timed stat boosts in sync with `active_spells`. Locate `rest_party_hour` (symbol
at approximately L432–476) and add a per-minute tick loop for timed boosts in
the same location where `active_spells.tick()` is called, using the same
60-iteration pattern already used for spell durations.

If `rest_party_hour` advances time through `GameState::advance_time`, this
wiring is automatic. If it calls `active_spells.tick()` directly, add:

```text
for member in &mut party.members {
    member.tick_timed_stat_boosts_minute();
}
```

in the same loop.

#### 2.4 Testing Requirements

Add the following tests inside `mod tests` in `src/domain/character.rs`:

| Test name                                                 | What it verifies                                                                                                                       |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `test_apply_timed_stat_boost_modifies_current_not_base`   | After `apply_timed_stat_boost(Might, 5, Some(30))`, `stats.might.current` increased by 5; `stats.might.base` unchanged                 |
| `test_apply_timed_stat_boost_none_duration_is_noop`       | `apply_timed_stat_boost(Might, 5, None)` leaves `timed_stat_boosts` empty and `stats.might.current` unchanged                          |
| `test_apply_timed_stat_boost_zero_duration_is_noop`       | `apply_timed_stat_boost(Might, 5, Some(0))` leaves `timed_stat_boosts` empty                                                           |
| `test_tick_timed_stat_boosts_decrements_counter`          | After 1 tick, `minutes_remaining` decreases by 1; stat unchanged                                                                       |
| `test_tick_timed_stat_boosts_reverses_on_expiry`          | After N ticks where N == initial `minutes_remaining`, `stats.might.current` returns to original value and `timed_stat_boosts` is empty |
| `test_tick_timed_stat_boosts_multiple_boosts_independent` | Two boosts with different durations expire at different times; each is reversed independently                                          |
| `test_timed_stat_boosts_defaults_empty_on_new_character`  | `Character::new(...)` produces `timed_stat_boosts == []`                                                                               |
| `test_timed_stat_boost_serde_default_deserializes`        | Deserializing a `Character` RON without `timed_stat_boosts` produces `timed_stat_boosts == []`                                         |

Add the following tests inside `mod tests` in `src/application/mod.rs`:

| Test name                                        | What it verifies                                                                                                              |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `test_advance_time_ticks_timed_stat_boosts`      | After `advance_time(N, None)`, a party member's timed boost with `minutes_remaining = N` has expired and the stat is restored |
| `test_advance_time_ticks_both_spells_and_boosts` | Both `active_spells.light` and a member's timed boost decrement together during the same `advance_time` call                  |

#### 2.5 Deliverables

- [ ] `TimedStatBoost` struct added to `src/domain/character.rs` with SPDX-compatible doc comment and doctest.
- [ ] `Character.timed_stat_boosts: Vec<TimedStatBoost>` field added with `#[serde(default)]`.
- [ ] `Character::apply_timed_stat_boost`, `tick_timed_stat_boosts_minute`, and `apply_attribute_delta` methods implemented.
- [ ] `GameState::advance_time` ticks `timed_stat_boosts` for every party member.
- [ ] `rest_party_hour` ticks `timed_stat_boosts` in sync with active spells.
- [ ] All 10 Phase 2 tests pass.
- [ ] All four quality gates pass.

#### 2.6 Success Criteria

- Timed attribute boosts applied to a `Character` expire exactly when
  `minutes_remaining` reaches zero, restoring the `current` attribute value.
- Normal exploration travel (`advance_time`) and long rests (`rest_party_hour`)
  both trigger expiry.
- Existing save files without `timed_stat_boosts` load without error.

---

### Phase 3: Route Timed Consumable Effects to the Correct Backend

Update `apply_consumable_effect` (and the exploration executor) to inspect
`ConsumableData.duration_minutes` and branch between permanent mutation and the
timed infrastructure added in Phase 2. Route `BoostResistance` with a duration
through `GameState.active_spells`; route `BoostAttribute` with a duration
through `Character::apply_timed_stat_boost`. Preserve permanent (duration-less)
behavior for both effects.

#### 3.1 Extend `ConsumableApplyResult`

**File to edit:** `src/domain/items/consumable_usage.rs`

Add two boolean fields to `ConsumableApplyResult` to allow callers to
distinguish instant mutations from timed registrations:

```text
pub struct ConsumableApplyResult {
    pub healing: i32,
    pub sp_restored: i32,
    pub conditions_cleared: u8,
    pub attribute_delta: i16,
    pub resistance_delta: i16,
    /// True when a `BoostAttribute` was registered as a timed boost
    /// (i.e., `duration_minutes` was `Some(n > 0)`).
    pub attribute_boost_is_timed: bool,
    /// True when a `BoostResistance` was handled by the caller's timed layer
    /// (populated by `apply_consumable_effect_exploration`; always false in
    /// the combat path which handles resistance directly).
    pub resistance_boost_is_timed: bool,
}
```

Initialize both new fields to `false` in every place that constructs
`ConsumableApplyResult` (all existing call sites return the struct via
`ConsumableApplyResult { ..., attribute_boost_is_timed: false, resistance_boost_is_timed: false }`).

#### 3.2 Update `apply_consumable_effect` for Timed Attribute Boosts

**File to edit:** `src/domain/items/consumable_usage.rs`

Change the signature of `apply_consumable_effect` to accept the full
`ConsumableData` instead of only `ConsumableEffect`, so the duration is
available:

```text
pub fn apply_consumable_effect(
    character: &mut Character,
    data: &ConsumableData,
) -> ConsumableApplyResult
```

Update the single call site in `src/domain/combat/item_usage.rs`
(`execute_item_use_by_slot`) to pass `consumable_data` (the full struct) instead
of `consumable_data.effect`.

Update the `BoostAttribute` arm to use `Character::apply_timed_stat_boost` when
`normalize_duration(data.duration_minutes)` is `Some(n)`, and fall back to the
original permanent `stats.<field>.modify` when it is `None`:

```text
ConsumableEffect::BoostAttribute(attr, amount) => {
    use crate::domain::items::types::normalize_duration;
    if normalize_duration(data.duration_minutes).is_some() {
        character.apply_timed_stat_boost(attr, amount, data.duration_minutes);
        ConsumableApplyResult {
            attribute_delta: amount as i16,
            attribute_boost_is_timed: true,
            ..Default::default()
        }
    } else {
        // permanent path (legacy behavior)
        character.apply_attribute_delta(attr, amount as i16);
        ConsumableApplyResult {
            attribute_delta: amount as i16,
            ..Default::default()
        }
    }
}
```

`BoostResistance` in `apply_consumable_effect` keeps its existing permanent
behavior (direct `resistances.<field>.modify`). The timed resistance path is
handled in the exploration executor (§3.3) so that it can access
`GameState.active_spells`. The combat path must **never** route through
`active_spells`.

#### 3.3 Add `apply_consumable_effect_exploration` for Timed Resistance Routing

**File to edit:** `src/domain/items/consumable_usage.rs`

Add a new public function that wraps `apply_consumable_effect` and additionally
handles timed resistance routing at the application layer. This function is
called by the exploration executor instead of `apply_consumable_effect` directly.

```text
/// Applies a consumable effect in the exploration context.
///
/// Identical to `apply_consumable_effect` except that `BoostResistance` with
/// a `duration_minutes` is written to `active_spells` instead of directly
/// mutating `character.resistances`, so the effect expires automatically via
/// `GameState::advance_time`.
///
/// # Arguments
///
/// * `character` — mutable reference to the consuming character
/// * `active_spells` — mutable reference to the party-wide active spells
/// * `data` — the full `ConsumableData` including optional duration
///
/// # Returns
///
/// A `ConsumableApplyResult` with `resistance_boost_is_timed = true` when
/// the resistance effect was routed to `active_spells`.
pub fn apply_consumable_effect_exploration(
    character: &mut Character,
    active_spells: &mut ActiveSpells,
    data: &ConsumableData,
) -> ConsumableApplyResult
```

**Timed resistance mapping** (implement this exactly; covers all eight
`ResistanceType` variants):

| `ResistanceType` variant | `ActiveSpells` field                                |
| ------------------------ | --------------------------------------------------- |
| `Fire`                   | `active_spells.fire_protection`                     |
| `Cold`                   | `active_spells.cold_protection`                     |
| `Electricity`            | `active_spells.electricity_protection`              |
| `Energy`                 | `active_spells.magic_protection`                    |
| `Fear`                   | `active_spells.fear_protection`                     |
| `Physical`               | `active_spells.magic_protection` (closest analogue) |
| `Paralysis`              | `active_spells.psychic_protection`                  |
| `Sleep`                  | `active_spells.psychic_protection`                  |

**Stacking rule:** overwrite — when a second potion is used while the field is
already non-zero, set it to the new duration (after `u8` clamping). Do not add
durations together.

**`u8` clamping:** `active_spells` fields are `u8`. Clamp via
`u16::min(duration, u8::MAX as u16) as u8`.

**Logic for `BoostResistance` arm:**

```text
ConsumableEffect::BoostResistance(res_type, _amount) => {
    use crate::domain::items::types::normalize_duration;
    if let Some(minutes) = normalize_duration(data.duration_minutes) {
        let clamped = u16::min(minutes, u8::MAX as u16) as u8;
        // write to the mapped active_spells field
        match res_type { ... }
        ConsumableApplyResult {
            resistance_delta: _amount as i16,
            resistance_boost_is_timed: true,
            ..Default::default()
        }
    } else {
        // permanent path: delegate to apply_consumable_effect
        apply_consumable_effect(character, data)
    }
}
```

For all other `ConsumableEffect` variants, delegate to `apply_consumable_effect`.

Re-export `apply_consumable_effect_exploration` from `src/domain/items/mod.rs`.

#### 3.4 Update the Exploration Executor

**File to edit:** `src/game/systems/inventory_ui.rs`

In `handle_use_item_action_exploration` (added by the prerequisite plan), replace
the call to `apply_consumable_effect(&mut character, effect)` with:

```text
let result = apply_consumable_effect_exploration(
    &mut character,
    &mut global_state.0.active_spells,
    consumable_data,
);
```

Update the import at the top of the file:

```text
use crate::domain::items::consumable_usage::apply_consumable_effect_exploration;
```

#### 3.5 Preserve Combat Behavior

**File to edit:** `src/domain/combat/item_usage.rs`

`execute_item_use_by_slot` calls `apply_consumable_effect`. Update this call to
pass the full `ConsumableData` reference (changed in §3.2). Verify by
inspection that:

- The combat path never touches `active_spells`.
- `BoostResistance` in combat still calls `character.resistances.<field>.modify`
  directly (via the permanent path inside `apply_consumable_effect` when
  `duration_minutes` is `None`).
- A timed `BoostAttribute` in combat **does** call `character.apply_timed_stat_boost`,
  because timed attribute boosts are character-scoped and safe in both contexts.

#### 3.6 Update `GameLog` Feedback Text in the Exploration Executor

**File to edit:** `src/game/systems/inventory_ui.rs`

In `handle_use_item_action_exploration`, update the `BoostResistance` and
`BoostAttribute` success messages to reflect whether the effect is timed:

| Condition                                                | `GameLog` message                                                                   |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `BoostResistance` + `resistance_boost_is_timed == true`  | `"{item_name} used. {res.display_name()} resistance active for {minutes} minutes."` |
| `BoostResistance` + `resistance_boost_is_timed == false` | `"{item_name} used. {character_name}'s {res.display_name()} resistance increased."` |
| `BoostAttribute` + `attribute_boost_is_timed == true`    | `"{item_name} used. {attr.display_name()} increased for {minutes} minutes."`        |
| `BoostAttribute` + `attribute_boost_is_timed == false`   | `"{item_name} used. {character_name}'s {attr.display_name()} increased."`           |

Retrieve `minutes` from `normalize_duration(consumable_data.duration_minutes)`.

#### 3.7 Testing Requirements

Add the following tests inside `mod tests` in `src/domain/items/consumable_usage.rs`:

| Test name                                                    | What it verifies                                                                                                                                                                                                                    |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_apply_timed_attribute_boost_registers_in_timed_list`   | `apply_consumable_effect` with `BoostAttribute(Might, 5)` and `duration_minutes: Some(30)` → `character.timed_stat_boosts.len() == 1`; `result.attribute_boost_is_timed == true`                                                    |
| `test_apply_permanent_attribute_boost_mutates_directly`      | `apply_consumable_effect` with `BoostAttribute(Might, 5)` and `duration_minutes: None` → `timed_stat_boosts` is empty; `stats.might.current` increased                                                                              |
| `test_apply_timed_resistance_routes_to_active_spells`        | `apply_consumable_effect_exploration` with `BoostResistance(Fire, 25)` and `duration_minutes: Some(60)` → `active_spells.fire_protection == 60`; `character.resistances.fire` unchanged; `result.resistance_boost_is_timed == true` |
| `test_apply_permanent_resistance_mutates_character_directly` | `apply_consumable_effect_exploration` with `BoostResistance(Fire, 25)` and `duration_minutes: None` → `character.resistances.fire.current` increased; `active_spells.fire_protection` unchanged                                     |
| `test_resistance_stacking_overwrites_duration`               | Two successive calls for the same resistance type; second call's duration overwrites first                                                                                                                                          |
| `test_resistance_u8_clamping`                                | `duration_minutes: Some(300)` clamps to `u8::MAX (255)` in `active_spells.fire_protection`                                                                                                                                          |
| `test_timed_resistance_all_eight_types_map_correctly`        | Each of the eight `ResistanceType` variants sets the expected `ActiveSpells` field                                                                                                                                                  |
| `test_combat_resistance_boost_still_permanent`               | `apply_consumable_effect` (not exploration variant) with `BoostResistance(Fire, 25)` and `duration_minutes: Some(60)` still mutates `character.resistances.fire` directly (combat path is permanent)                                |

Add these regression tests in `src/domain/combat/item_usage.rs` `mod tests`:

| Test name                                                       | What it verifies                                                                                                                    |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `test_execute_item_use_timed_attribute_in_combat_applies_boost` | Combat use of a `BoostAttribute` item with `duration_minutes: Some(30)` registers a `TimedStatBoost` on the character               |
| `test_execute_item_use_resistance_in_combat_is_permanent`       | Combat use of a `BoostResistance` item with `duration_minutes: Some(60)` still mutates `resistances` directly (not `active_spells`) |

#### 3.8 Deliverables

- [ ] `ConsumableApplyResult` extended with `attribute_boost_is_timed` and `resistance_boost_is_timed` fields.
- [ ] `apply_consumable_effect` signature accepts `&ConsumableData`; `BoostAttribute` arm branches on duration.
- [ ] `apply_consumable_effect_exploration` added with timed resistance routing via `ActiveSpells`.
- [ ] Both functions re-exported from `src/domain/items/mod.rs`.
- [ ] `handle_use_item_action_exploration` calls `apply_consumable_effect_exploration`.
- [ ] `execute_item_use_by_slot` passes full `ConsumableData` to `apply_consumable_effect`.
- [ ] Feedback text in exploration executor reflects timed vs. permanent application.
- [ ] All 10 Phase 3 tests pass.
- [ ] All four quality gates pass.

#### 3.9 Success Criteria

- Out-of-combat timed resistance potions set the correct `ActiveSpells` field
  and expire via `advance_time`; they do not permanently alter `character.resistances`.
- Out-of-combat timed attribute potions register a `TimedStatBoost` and auto-reverse.
- Combat item use remains entirely character-scoped; `active_spells` is never
  mutated from the combat path.
- All existing combat item use tests continue to pass unchanged.

---

### Phase 4: Project `ActiveSpells` into Effective Resistance Calculations

`active_spells.*_protection` fields are currently ticked but never read in any
damage, combat, or resistance check. This phase adds a projection helper that
converts active protection counts into effective resistance values, integrating
`active_spells` into the spell-resistance and damage paths that already consult
`character.resistances`.

#### 4.1 Add `effective_resistance` helper to `ActiveSpells`

**File to edit:** `src/application/mod.rs`

Add a new method to `impl ActiveSpells`:

````text
/// Returns the effective bonus resistance for a given `ResistanceType`
/// contributed by active spell protections.
///
/// Non-zero values indicate that a time-limited protection potion or spell
/// is active.  Callers add this to `character.resistances.<field>.current`
/// to obtain the total effective resistance.
///
/// # Examples
///
/// ```
/// use antares::application::ActiveSpells;
/// use antares::domain::items::types::ResistanceType;
///
/// let mut spells = ActiveSpells::new();
/// spells.fire_protection = 30;
/// // A non-zero protection count maps to a fixed gameplay bonus.
/// assert!(spells.effective_resistance(ResistanceType::Fire) > 0);
/// assert_eq!(spells.effective_resistance(ResistanceType::Cold), 0);
/// ```
pub fn effective_resistance(&self, res_type: ResistanceType) -> i16 {
    use crate::domain::items::types::ResistanceType;
    // Bonus per-minute-of-protection: 25 points while active, 0 when expired.
    const ACTIVE_BONUS: i16 = 25;
    let active = match res_type {
        ResistanceType::Fire        => self.fire_protection > 0,
        ResistanceType::Cold        => self.cold_protection > 0,
        ResistanceType::Electricity => self.electricity_protection > 0,
        ResistanceType::Energy      => self.magic_protection > 0,
        ResistanceType::Fear        => self.fear_protection > 0,
        ResistanceType::Physical    => self.magic_protection > 0,
        ResistanceType::Paralysis   => self.psychic_protection > 0,
        ResistanceType::Sleep       => self.psychic_protection > 0,
    };
    if active { ACTIVE_BONUS } else { 0 }
}
````

Define the constant `pub const ACTIVE_PROTECTION_BONUS: i16 = 25;` at module
scope in `src/application/mod.rs` and reference it from `effective_resistance`.

#### 4.2 Integrate into Combat Damage / Resistance Checks

**File to edit:** `src/domain/combat/engine.rs`

Locate `resolve_attack` (symbol at approximately L581–639). Currently this
function may use `character.resistances.<field>.current` to determine if an
attack is resisted. After this phase it must also add the `active_spells`
projection bonus.

Because `resolve_attack` does not currently have access to `ActiveSpells`
(it operates on `&CombatState` which does not store `active_spells`), there are
two valid approaches:

**Chosen approach: pass `active_spells` as an optional parameter**

Change `resolve_attack` signature to accept `active_spells: Option<&ActiveSpells>`:

```text
pub fn resolve_attack(
    combat_state: &CombatState,
    attacker: CombatantId,
    target: CombatantId,
    attack: &Attack,
    active_spells: Option<&ActiveSpells>,
    rng: &mut impl Rng,
) -> Result<(u32, Option<SpecialEffect>), CombatError>
```

When computing whether a resistance-based attack is resisted, add
`active_spells.map_or(0, |s| s.effective_resistance(res_type))` to the
character's current resistance value.

Update all call sites of `resolve_attack` in `src/game/systems/combat.rs` to
pass `Some(&combat_res.active_spells)` if available, or `None` in tests
that do not wire up `GameState`.

**Note:** `CombatResource` in `src/game/systems/combat.rs` does not currently
hold `ActiveSpells`. For the game system call site, read it from
`global_state.0.active_spells` and pass a reference.

#### 4.3 Testing Requirements

Add the following tests to `src/application/mod.rs` `mod tests`:

| Test name                                           | What it verifies                                                                            |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `test_effective_resistance_zero_when_no_protection` | All eight types return 0 when all `active_spells` fields are 0                              |
| `test_effective_resistance_nonzero_when_active`     | Each of the eight types returns `ACTIVE_PROTECTION_BONUS` when its mapped field is non-zero |
| `test_effective_resistance_zero_when_expired`       | After `active_spells.fire_protection` ticks to 0, `effective_resistance(Fire) == 0`         |

Add to `src/domain/combat/engine.rs` `mod tests`:

| Test name                                           | What it verifies                                                                                                                                    |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_resistance_check_without_active_spells`       | `resolve_attack` with `active_spells: None` behaves identically to current behavior                                                                 |
| `test_resistance_check_with_active_fire_protection` | A fire attack against a target with `active_spells.fire_protection > 0` has reduced (or zero) damage compared to the same attack without protection |

#### 4.4 Deliverables

- [ ] `effective_resistance` method and `ACTIVE_PROTECTION_BONUS` constant added to `ActiveSpells` in `src/application/mod.rs`.
- [ ] `resolve_attack` accepts `active_spells: Option<&ActiveSpells>` parameter.
- [ ] All call sites of `resolve_attack` in `src/game/systems/combat.rs` updated.
- [ ] All 5 Phase 4 tests pass.
- [ ] All four quality gates pass.

#### 4.5 Deliverables Checklist

- [ ] `ACTIVE_PROTECTION_BONUS: i16 = 25` constant defined in `src/application/mod.rs`.
- [ ] `ActiveSpells::effective_resistance` covers all eight `ResistanceType` variants.
- [ ] `resolve_attack` signature updated and all call sites pass `active_spells`.
- [ ] Resistance projection applies during combat damage calculation.
- [ ] No existing tests broken.

#### 4.6 Success Criteria

- A party member with an active `fire_protection` potion takes reduced fire
  damage in combat while the duration remains non-zero.
- The protection expires correctly when `advance_time` ticks the field to zero.
- Passing `None` for `active_spells` preserves all current test behavior.

---

### Phase 5: Campaign Builder Support for Duration-Aware Consumables

Expose `duration_minutes` in the `ItemsEditorState` consumable editor so
campaign authors can author timed attribute and resistance consumables.

#### 5.1 Feature Work — Duration Widget in `show_type_editor`

**File to edit:** `sdk/campaign_builder/src/items_editor.rs`

Inside `show_type_editor` (L1092–1450), in the `ItemType::Consumable(data)`
arm (around L1298), add a `Duration (minutes)` row **after** the existing
effect-value editors, but only for `BoostAttribute` and `BoostResistance`
effects. Do not show a duration widget for `HealHp`, `RestoreSp`, or
`CureCondition` (these are instant and do not use `duration_minutes`).

```text
// Duration row — shown only for timed-capable effects
if matches!(
    data.effect,
    ConsumableEffect::BoostAttribute(_, _) | ConsumableEffect::BoostResistance(_, _)
) {
    ui.horizontal(|ui| {
        ui.label("Duration (minutes):");
        let mut raw: u16 = data.duration_minutes.unwrap_or(0);
        ui.add(egui::DragValue::new(&mut raw).range(0..=u16::MAX));
        ui.label("(0 = permanent)");
        data.duration_minutes = if raw == 0 { None } else { Some(raw) };
    });
}
```

This widget uses no new `egui` ID contexts — it is a simple `DragValue` inside
an existing `ui.horizontal` without a `ComboBox` or loop, so no `push_id` or
`from_id_salt` is required.

#### 5.2 Update Preview Text in `show_preview_static`

**File to edit:** `sdk/campaign_builder/src/items_editor.rs`

Locate `show_preview_static` (L596–738) where consumable effect text is
rendered. Add a duration suffix to `BoostAttribute` and `BoostResistance`
preview lines:

```text
// In the BoostAttribute arm:
let duration_str = match item.item_type {
    ItemType::Consumable(ref d) =>
        d.duration_minutes.map(|m| format!(" ({} min)", m)).unwrap_or_default(),
    _ => String::new(),
};
// append duration_str to existing attribute boost preview text

// In the BoostResistance arm:
// same pattern
```

#### 5.3 SDK Fixtures and Template Updates

**File to edit:** `src/sdk/templates.rs`

Add two new template functions alongside `healing_potion` and `sp_potion`:

````text
/// Creates a timed fire resistance potion item with the given duration.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::timed_fire_resist_potion;
/// let item = timed_fire_resist_potion(60, 90, "Fire Resist Potion");
/// ```
pub fn timed_fire_resist_potion(id: ItemId, duration_minutes: u16, name: &str) -> Item

/// Creates a timed might boost potion item with the given duration.
///
/// # Examples
///
/// ```
/// use antares::sdk::templates::timed_might_potion;
/// let item = timed_might_potion(61, 30, "Might Potion");
/// ```
pub fn timed_might_potion(id: ItemId, duration_minutes: u16, name: &str) -> Item
````

Both functions use `normalize_duration(Some(duration_minutes))` so callers
passing `0` get a permanent item.

**File to edit:** `data/test_campaign/data/items.ron`

Add two new consumable entries (using IDs 60 and 61 to avoid conflicts) for
use by Phase 5 tests:

```ron
(
    id: 60,
    name: "Fire Resist Potion",
    item_type: Consumable((
        effect: BoostResistance(Fire, 25),
        is_combat_usable: false,
        duration_minutes: Some(60),
    )),
    base_cost: 100,
    sell_cost: 50,
    ...
),
(
    id: 61,
    name: "Might Potion",
    item_type: Consumable((
        effect: BoostAttribute(Might, 5),
        is_combat_usable: false,
        duration_minutes: Some(30),
    )),
    base_cost: 80,
    sell_cost: 40,
    ...
),
```

Follow all existing field conventions in the file (copy the full field list from
item 50 as a template; omit only the `id` and `name` substitutions).

#### 5.4 Testing Requirements

Add the following tests inside `mod tests` in
`sdk/campaign_builder/src/items_editor.rs`:

| Test name                                                  | What it verifies                                                                                                                         |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `test_duration_field_round_trips_through_editor`           | Setting `data.duration_minutes = Some(60)` in the editor buffer, then saving and reloading, preserves `Some(60)`                         |
| `test_duration_hidden_for_instant_effects`                 | For `HealHp` and `RestoreSp` effects the `duration_minutes` field in `edit_buffer.item_type` is `None` after `default_item` construction |
| `test_duration_zero_normalizes_to_none_on_save`            | Writing `raw = 0` in the DragValue produces `duration_minutes: None` in the saved `ConsumableData`                                       |
| `test_preview_text_includes_duration_for_timed_boost`      | Preview string for a `BoostAttribute` item with `duration_minutes: Some(60)` contains "60"                                               |
| `test_preview_text_no_duration_suffix_for_permanent_boost` | Preview string for a `BoostAttribute` item with `duration_minutes: None` does not contain "min"                                          |

Add to `src/sdk/templates.rs` `mod tests` (or inline doctests):

| Test name                                            | What it verifies                                                                                                  |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_timed_fire_resist_potion_has_correct_duration` | `timed_fire_resist_potion(60, 90, "X").item_type` unwraps to `ConsumableData { duration_minutes: Some(90), ... }` |
| `test_timed_might_potion_zero_duration_is_none`      | `timed_might_potion(61, 0, "X")` produces `duration_minutes: None`                                                |

#### 5.5 Deliverables

- [ ] `show_type_editor` in `sdk/campaign_builder/src/items_editor.rs` shows a `Duration (minutes)` `DragValue` for `BoostAttribute` and `BoostResistance` only.
- [ ] Preview text in `show_preview_static` appends `" (N min)"` for timed boosts.
- [ ] `timed_fire_resist_potion` and `timed_might_potion` template functions added to `src/sdk/templates.rs`.
- [ ] Test-campaign fixture `data/test_campaign/data/items.ron` includes items 60 and 61.
- [ ] All 7 Phase 5 tests pass.
- [ ] All four quality gates pass.
- [ ] `sdk/AGENTS.md` egui ID audit confirmed: no new `ComboBox`, loop, `SidePanel`, or `ScrollArea` was introduced without required ID annotations.

#### 5.6 Success Criteria

- A campaign author can open an existing `BoostResistance` or `BoostAttribute`
  consumable in the Campaign Builder, set a duration, save the file, reload, and
  see the correct `duration_minutes` value.
- Duration `0` and omitted duration are both treated as permanent.
- Instant consumables (`HealHp`, `RestoreSp`, `CureCondition`) never show the
  duration widget.

---

### Phase 6: End-to-End Integration Tests and Documentation

Harden the complete feature with cross-layer integration tests and update all
affected documentation.

#### 6.1 End-to-End Integration Tests

Add the following integration tests to `src/application/mod.rs` `mod tests`.
Each test exercises the full path from inventory use through time expiry:

| Test name                                                 | What it verifies                                                                                                                                                        |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_timed_resistance_potion_expires_after_advance_time` | Use a timed fire resist potion in exploration → `active_spells.fire_protection > 0` → `advance_time(60, None)` → `active_spells.fire_protection == 0`                   |
| `test_timed_attribute_potion_expires_after_advance_time`  | Use a timed might potion → `timed_stat_boosts.len() == 1` → `advance_time(30, None)` → `timed_stat_boosts.len() == 0`; `stats.might.current` restored to original value |
| `test_timed_potion_expires_during_rest`                   | Use a timed potion with `duration = 60` → `rest_party(REST_DURATION_HOURS, None)` → effect expired                                                                      |
| `test_permanent_attribute_potion_survives_advance_time`   | Use a permanent `BoostAttribute` potion (`duration_minutes: None`) → `advance_time(999, None)` → stat still boosted                                                     |
| `test_second_resistance_potion_overwrites_duration`       | Use fire resist potion (60 min) → `advance_time(30, None)` → use second fire resist potion (60 min) → `active_spells.fire_protection == 60` (not 90)                    |

#### 6.2 Documentation Updates

**File to edit:** `docs/reference/architecture.md`

Verify the `ConsumableData` definition at L803–806 already reflects `duration_minutes`
(updated in Phase 1). Additionally update the prose section on consumable effects
(if any) to describe the timed vs. permanent distinction.

**File to edit:** `src/domain/items/consumable_usage.rs`

- Ensure every `pub` function has a `///` doc comment, `# Arguments`, `# Returns`,
  and a runnable `# Examples` doctest.
- Add a module-level doc comment describing the timed vs. permanent split and
  the two entry points (`apply_consumable_effect` for combat,
  `apply_consumable_effect_exploration` for exploration).

**File to edit:** `src/domain/character.rs`

- Ensure `TimedStatBoost`, `apply_timed_stat_boost`, `tick_timed_stat_boosts_minute`,
  and `apply_attribute_delta` all have `///` doc comments matching AGENTS.md standards.

**File to edit:** `src/application/mod.rs`

- Update the `ActiveSpells` doc comment to describe `effective_resistance` and
  the `ACTIVE_PROTECTION_BONUS` constant.
- Update the `GameState::advance_time` doc comment to note that per-character
  timed stat boosts are also ticked.

**File to edit:** `docs/explanation/implementations.md`

Prepend a new section summarizing the consumable duration effects feature,
following the existing phase-summary format already present in the file.

#### 6.3 Testing Requirements

Run the complete quality gate matrix after every phase commit. All four commands
must produce zero errors and zero warnings:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Additionally verify test count increases by at least the number of new tests
defined across all phases.

#### 6.4 Deliverables

- [ ] All 5 end-to-end integration tests in `src/application/mod.rs` pass.
- [ ] All `pub` symbols in `src/domain/items/consumable_usage.rs` have `///` doc comments and doctests.
- [ ] `TimedStatBoost`, `apply_timed_stat_boost`, and `tick_timed_stat_boosts_minute` have `///` doc comments.
- [ ] `ActiveSpells::effective_resistance` has doc comment and doctest.
- [ ] `GameState::advance_time` doc comment updated to mention per-character boost ticking.
- [ ] `docs/reference/architecture.md` `ConsumableData` definition includes `duration_minutes`.
- [ ] `docs/explanation/implementations.md` includes a consumable duration effects summary section.
- [ ] All four quality gates pass.

#### 6.5 Success Criteria

- The complete feature passes all quality gates with zero warnings.
- A timed resistance potion and a timed attribute potion can be authored in RON,
  loaded, used from inventory, observed to affect gameplay (resistance during
  combat for fire potions; stat visible on HUD for attribute potions), and
  expire cleanly after the correct number of in-game minutes.
- All pre-existing tests continue to pass.

---

## Design Decisions

1. **`duration_minutes = None` is permanent.** `None` preserves full backward
   compatibility for all existing consumable RON data and Rust test helpers.
   `Some(0)` is normalized to `None` at apply time via `normalize_duration` so
   editor inputs of `0` and omitted RON fields produce identical behavior.

2. **Repeated use overwrites — no stacking.** When a second timed resistance
   potion is used while the same `ActiveSpells` field is still active, the new
   duration overwrites the remaining duration after `u8` clamping. This avoids
   integer overflow and matches the Might and Magic 1 design philosophy of simple,
   predictable consumable semantics.

3. **Timed attribute boosts use `TimedStatBoost` on `Character`, not
   `ActiveCondition`.** Overloading `active_conditions` would require inventing
   synthetic condition definition IDs in the condition RON database and would
   couple consumable expiry to the condition loader. `TimedStatBoost` is
   self-contained, serializable with `#[serde(default)]`, and trivially reversible.

4. **Timed resistance boosts use `ActiveSpells` at the application layer.**
   Combat already has ephemeral character state; routing resistance potions
   through `active_spells` during exploration keeps the combat path unmodified
   and ensures protections survive across multiple combat encounters during their
   active window.

5. **`ACTIVE_PROTECTION_BONUS = 25` is a fixed gameplay constant, not
   proportional to minutes remaining.** A flat bonus is simpler to balance and
   test than a decaying value. Campaign authors control item potency via the
   `amount` field in `BoostResistance`; the `active_spells` field stores only
   the remaining duration, not the magnitude.

6. **`resolve_attack` takes `active_spells: Option<&ActiveSpells>` rather than
   reading from a global.** This keeps `resolve_attack` a pure function testable
   without a full `GameState`, matching the existing architecture pattern.
