# Equipped Weapon Damage in Combat Implementation Plan

## Overview

Player characters in combat always deal 1d4 physical damage regardless of their
equipped weapon, because `perform_attack_action_with_rng` in
`src/game/systems/combat.rs` hardcodes
`Attack::physical(DiceRoll::new(1, 4, 0))` for every `CombatantId::Player`
turn. Monster attacks already read from their `attacks` list via
`choose_monster_attack`. This plan repairs that asymmetry by introducing a
pure-domain helper `get_character_attack` in the combat engine, wiring it into
the game system, and covering all edge cases with tests.

**Ranged weapons are explicitly out of scope for the melee attack path.**
`WeaponClassification::MartialRanged` weapons (bows, crossbows) require ammo,
consume it on use, and are resolved through a separate
`perform_ranged_attack_action_with_rng()` function specified in
`combat_events_implementation_plan.md` Phase 3. This plan ensures
`get_character_attack` correctly identifies ranged weapons and that the melee
path in `perform_attack_action_with_rng` refuses to fire them, preventing
silent misuse.

## Current State Analysis

### Existing Infrastructure

- `perform_attack_action_with_rng` ([src/game/systems/combat.rs](../../src/game/systems/combat.rs))
  already receives `content: &GameContent`, whose `.db().items` field is a live
  `ItemDatabase`. No additional resource access is needed.
- `Equipment` ([src/domain/character.rs](../../src/domain/character.rs)) has a
  `weapon: Option<ItemId>` field that is set when the player equips a weapon.
- `WeaponData` ([src/domain/items/types.rs](../../src/domain/items/types.rs))
  carries `damage: DiceRoll`, `bonus: i8`, and
  `classification: WeaponClassification`.
- `Attack::physical(DiceRoll)` in `src/domain/combat/types.rs` is the
  constructor used today and sufficient for the new code.
- `WeaponClassification::Unarmed` already exists and the `"unarmed"` proficiency
  is present in `data/proficiencies.ron`.
- `choose_monster_attack` lives in `src/domain/combat/engine.rs` alongside
  `resolve_attack` and `apply_damage` — the natural home for
  `get_character_attack`.

### Identified Issues

1. `CombatantId::Player` branch in `perform_attack_action_with_rng` ignores
   `equipment.weapon` entirely.
2. No `UNARMED_DAMAGE` constant — the magic literal `DiceRoll::new(1, 4, 0)` is
   used in multiple places and does not reflect unarmed damage (1d2).
3. `WeaponData::bonus` (i16 per the task spec, `i8` as found) is not applied to
   the resolved attack at all.
4. No pure-domain function exists to derive an `Attack` from a character's
   equipped weapon — the lookup is done only in game-system code.
5. `get_character_attack` (as originally sketched) treats all equipped weapons
   identically regardless of `WeaponClassification` — a `MartialRanged` bow
   would be used as a melee weapon with no ammo check, no ammo consumption, and
   `is_ranged` left `false`, silently producing wrong behaviour.
6. `Attack` has no `is_ranged: bool` field (added by
   `combat_events_implementation_plan.md` §3.2.1); without it the engine cannot
   distinguish a ranged weapon's attack from a melee one.

---

## Implementation Phases

### Phase 1: Domain Combat Engine Changes

Add the `UNARMED_DAMAGE` constant and `get_character_attack` function to
[src/domain/combat/engine.rs](../../src/domain/combat/engine.rs). This is a
pure-domain change: no Bevy dependencies, no I/O.

#### 1.1 Add `UNARMED_DAMAGE` Constant

At module scope near the top of `src/domain/combat/engine.rs`, add:

```
pub const UNARMED_DAMAGE: DiceRoll = DiceRoll { count: 1, sides: 2, bonus: 0 };
```

This replaces the current scattered `DiceRoll::new(1, 4, 0)` literal used as the
player fallback (1d4 is wrong — unarmed is 1d2 per the spec).

#### 1.2 Add `MeleeAttackResult` Return Type

Because `get_character_attack` must now communicate weapon classification to
its callers, define a small result enum at module scope in `engine.rs`:

```rust
/// Outcome of resolving a character's melee attack.
///
/// The `Ranged` variant is returned when the character has a
/// `MartialRanged` weapon equipped but the caller is operating in
/// the melee path.  Callers must treat this as an error / early-return
/// and direct the player through `perform_ranged_attack_action_with_rng`
/// instead.
pub enum MeleeAttackResult {
    /// A valid melee `Attack` ready for `resolve_attack`.
    Melee(Attack),
    /// The equipped weapon is ranged; melee path must not proceed.
    /// The inner `Attack` is provided so callers can log or display
    /// the weapon stats without a second lookup, but must NOT apply
    /// damage with it through the melee pipeline.
    Ranged(Attack),
}
```

#### 1.3 Add `get_character_attack` Function

Add to `src/domain/combat/engine.rs`:

```rust
pub fn get_character_attack(
    character: &Character,
    item_db: &ItemDatabase,
) -> MeleeAttackResult
```

Logic (in order):

1. If `character.equipment.weapon` is `None`, return
   `MeleeAttackResult::Melee(Attack::physical(UNARMED_DAMAGE))`.
2. Look up the item via `item_db.get_item(item_id)`.
3. If not found or the `item_type` is not `ItemType::Weapon(weapon_data)`,
   fall back to `MeleeAttackResult::Melee(Attack::physical(UNARMED_DAMAGE))`.
4. Build a `DiceRoll` from `weapon_data.damage`. Apply `weapon_data.bonus` to
   the `bonus` field of `DiceRoll` via `saturating_add`.
5. **If `weapon_data.classification == WeaponClassification::MartialRanged`**,
   construct the `Attack` with `is_ranged: true` (see §1.5) and return
   `MeleeAttackResult::Ranged(attack)`. Do **not** return
   `MeleeAttackResult::Melee` for ranged weapons under any circumstances.
6. Otherwise return `MeleeAttackResult::Melee(Attack::physical(adjusted_dice_roll))`.

The function is infallible — all failure paths return a valid `MeleeAttackResult`
rather than propagating an error.

#### 1.4 Update Required Imports

`get_character_attack` needs `ItemDatabase`, `ItemType`, and
`WeaponClassification` in scope. Add the necessary `use` lines to `engine.rs`:

```rust
use crate::domain::items::{ItemDatabase, ItemType, WeaponClassification};
```

#### 1.5 Add `is_ranged: bool` to `Attack`

> **Dependency**: `combat_events_implementation_plan.md` §3.2.1 specifies this
> field. If that plan is implemented first, skip this step. If this plan is
> implemented first, add the field here and remove it from the combat-events
> plan's deliverables to avoid a conflict.

In `src/domain/combat/types.rs`, add to the `Attack` struct:

```rust
/// True when this attack is ranged (bow, crossbow, thrown).
///
/// Used by `perform_attack_action_with_rng` to reject ranged weapons
/// in the melee path, and by `perform_monster_turn_with_rng` to prefer
/// ranged monster attacks in Ranged combat events.
#[serde(default)]
pub is_ranged: bool,
```

Update `Attack::physical` to keep `is_ranged: false`. Add a constructor:

```rust
pub fn ranged(damage: DiceRoll) -> Self {
    Self {
        damage,
        attack_type: AttackType::Physical,
        special_effect: None,
        is_ranged: true,
    }
}
```

The `#[serde(default)]` attribute ensures all existing RON monster data that
lacks the field deserialises correctly (defaults to `false`).

#### 1.6 Add `has_ranged_weapon` Helper

> **Dependency**: `combat_events_implementation_plan.md` §3.2 specifies this
> helper. If that plan is implemented first, skip this step.

Add to `src/domain/combat/engine.rs`:

```rust
/// Returns `true` if `character` has a `MartialRanged` weapon equipped
/// **and** at least one compatible ammo item in their inventory.
///
/// A character with a bow but no arrows returns `false`.
pub fn has_ranged_weapon(character: &Character, item_db: &ItemDatabase) -> bool {
    let Some(weapon_slot) = &character.equipment.weapon else {
        return false;
    };
    let Some(item) = item_db.get_item(weapon_slot) else {
        return false;
    };
    let ItemType::Weapon(data) = &item.item_type else {
        return false;
    };
    if data.classification != WeaponClassification::MartialRanged {
        return false;
    }
    // Must also have at least one ammo item in inventory
    character.inventory.items.iter().any(|slot| {
        item_db
            .get_item(&slot.item_id)
            .map(|i| matches!(i.item_type, ItemType::Ammo(_)))
            .unwrap_or(false)
    })
}
```

#### 1.7 Testing Requirements

Unit tests inside `src/domain/combat/engine.rs` (under `#[cfg(test)]`):

**`get_character_attack` tests:**

- `test_get_character_attack_no_weapon_returns_unarmed` — `equipment.weapon =
None`; assert result is `MeleeAttackResult::Melee` and `attack.damage ==
UNARMED_DAMAGE`.
- `test_get_character_attack_melee_weapon_returns_melee` — equip a `Simple`
  longsword (1d8, bonus 0); assert result is `MeleeAttackResult::Melee` and
  `attack.damage == DiceRoll::new(1, 8, 0)`.
- `test_get_character_attack_weapon_bonus_applied` — equip a `+2` sword;
  assert result is `MeleeAttackResult::Melee` and `attack.damage.bonus == 2`.
- `test_get_character_attack_unknown_item_id_falls_back` — equip item_id 99
  (not in db); assert `MeleeAttackResult::Melee` with `UNARMED_DAMAGE` (no
  panic).
- `test_get_character_attack_non_weapon_item_falls_back` — equip a Consumable
  item; assert `MeleeAttackResult::Melee` with `UNARMED_DAMAGE`.
- `test_get_character_attack_ranged_weapon_returns_ranged_variant` — equip a
  `MartialRanged` bow (1d6, bonus 0); assert result is
  `MeleeAttackResult::Ranged` and `attack.is_ranged == true`.
- `test_get_character_attack_ranged_weapon_damage_correct` — equip a
  `MartialRanged` crossbow (1d8, bonus 1); assert the inner `Attack` has
  `damage == DiceRoll { count: 1, sides: 8, bonus: 1 }`.

**`has_ranged_weapon` tests:**

- `test_has_ranged_weapon_false_no_weapon` — no weapon equipped; assert
  `false`.
- `test_has_ranged_weapon_false_melee_weapon` — melee weapon equipped; assert
  `false`.
- `test_has_ranged_weapon_false_no_ammo` — `MartialRanged` bow equipped but
  inventory has no ammo items; assert `false`.
- `test_has_ranged_weapon_true_with_bow_and_arrows` — `MartialRanged` bow
  equipped and arrows in inventory; assert `true`.

**`Attack::ranged` constructor test:**

- `test_attack_ranged_constructor_sets_is_ranged_true` — `Attack::ranged(DiceRoll::new(1,6,0)).is_ranged == true`.
- `test_attack_physical_constructor_is_ranged_false` — `Attack::physical(DiceRoll::new(1,4,0)).is_ranged == false`.

#### 1.8 Deliverables

- [ ] `UNARMED_DAMAGE` constant in `src/domain/combat/engine.rs`
- [ ] `MeleeAttackResult` enum in `src/domain/combat/engine.rs`
- [ ] `get_character_attack(character, item_db) -> MeleeAttackResult` in
      `src/domain/combat/engine.rs`
- [ ] `has_ranged_weapon(character, item_db) -> bool` in
      `src/domain/combat/engine.rs`
- [ ] `is_ranged: bool` field on `Attack` with `#[serde(default)]`
- [ ] `Attack::ranged(damage)` constructor
- [ ] Required `use` imports added
- [ ] All thirteen unit tests pass

#### 1.9 Success Criteria

`cargo check --all-targets --all-features` and `cargo nextest run --all-features`
pass with zero errors/warnings. `get_character_attack` is callable from outside
the module. Equipping a bow returns `MeleeAttackResult::Ranged`; equipping a
sword returns `MeleeAttackResult::Melee`; no weapon returns unarmed.

---

### Phase 2: Game System Integration

Replace the hardcoded `DiceRoll::new(1, 4, 0)` player attack in
`perform_attack_action_with_rng` with a call to `get_character_attack`, and
add a guard that refuses to fire ranged weapons through the melee path.

#### 2.1 Locate the Hardcoded Player Attack

In [src/game/systems/combat.rs](../../src/game/systems/combat.rs), line ~1992:

```rust
CombatantId::Player(_) => {
    crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
}
```

#### 2.2 Extract Character Reference and Dispatch on Result

Replace the block above with:

```rust
CombatantId::Player(idx) => {
    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
        match get_character_attack(pc, &content.db().items) {
            MeleeAttackResult::Melee(attack) => attack,
            MeleeAttackResult::Ranged(_) => {
                // Ranged weapons must be used via TurnAction::RangedAttack /
                // perform_ranged_attack_action_with_rng, not the melee path.
                // Log a warning and skip the turn rather than dealing wrong damage.
                warn!(
                    "Player {:?} attempted melee attack with ranged weapon; \
                     use TurnAction::RangedAttack instead. Turn skipped.",
                    action.attacker
                );
                return Ok(());
            }
        }
    } else {
        return Err(CombatError::CombatantNotFound(action.attacker));
    }
}
```

Add the imports at the top of `combat.rs`:

```rust
use crate::domain::combat::engine::{get_character_attack, MeleeAttackResult};
```

#### 2.3 Leave Monster Path Unchanged

`perform_monster_turn_with_rng` and the `CombatantId::Monster` branch of
`perform_attack_action_with_rng` must not be modified by this plan. Ranged
monster attack preference in Ranged combat is specified in
`combat_events_implementation_plan.md` §3.6.

#### 2.4 Testing Requirements

Integration-level tests (pure function tests with a constructed
`CombatResource`). All tests use `data/test_campaign` fixtures — no reference
to `campaigns/tutorial`.

- `test_player_attack_uses_equipped_melee_weapon_damage` — equip a longsword
  (1d8, bonus 0); run `perform_attack_action_with_rng` with seeded RNG; assert
  applied damage is in range [1, 8], not the old [1, 4].
- `test_player_attack_unarmed_when_no_weapon` — `equipment.weapon = None`; run
  attack; assert damage is in range [1, 2] (before might modifier).
- `test_player_attack_bonus_weapon_floor_at_one` — equip a -3 cursed dagger
  (1d4, bonus -3); assert damage is at least 1, never 0 or negative.
- `test_player_melee_attack_with_ranged_weapon_skips_turn` — equip a
  `MartialRanged` bow; call `perform_attack_action_with_rng`; assert the
  function returns `Ok(())` without applying any damage, and that the target's
  HP is unchanged.

#### 2.5 Deliverables

- [ ] Hardcoded `DiceRoll::new(1, 4, 0)` removed from player attack branch
- [ ] `get_character_attack` + `MeleeAttackResult` dispatch wired into
      `perform_attack_action_with_rng`
- [ ] Ranged weapon guard logs a warning and returns `Ok(())` without damage
- [ ] `use` imports added in `combat.rs`
- [ ] All four integration tests pass

#### 2.6 Success Criteria

All four quality gates pass. A character with a melee weapon deals damage in
that weapon's stated range. An unarmed character deals 1d2. A character with a
bow who somehow triggers the melee action has their turn skipped with a warning
— no damage is dealt, no panic occurs.

---

### Phase 2.5: Ranged Attack Path — Sequencing Note

This plan does **not** implement `perform_ranged_attack_action_with_rng` or the
`TurnAction::RangedAttack` UI button. Those are specified in full in
`combat_events_implementation_plan.md` Phase 3 (§3.1–§3.10). The two plans
interact as follows:

| Concern                                      | Owner plan                                                    |
| -------------------------------------------- | ------------------------------------------------------------- |
| `is_ranged: bool` on `Attack`                | This plan §1.5 (or combat-events §3.2.1 — first to land wins) |
| `has_ranged_weapon()` helper                 | This plan §1.6 (or combat-events §3.2 — first to land wins)   |
| `MeleeAttackResult` / `get_character_attack` | This plan §1.2–§1.3                                           |
| `TurnAction::RangedAttack` variant           | `combat_events_implementation_plan.md` §3.1                   |
| `perform_ranged_attack_action_with_rng()`    | `combat_events_implementation_plan.md` §3.4                   |
| Ammo consumption                             | `combat_events_implementation_plan.md` §3.4                   |
| Ranged Attack UI button                      | `combat_events_implementation_plan.md` §3.3                   |
| Monster ranged attack preference             | `combat_events_implementation_plan.md` §3.6                   |

**Implementation order recommendation**: implement this plan first (it is a
prerequisite fix) then implement `combat_events_implementation_plan.md` Phase 3.
If the combat-events plan lands first, the `is_ranged` field and
`has_ranged_weapon` helper from §1.5–§1.6 of this plan should be removed from
this plan's deliverables to avoid a double-definition conflict.

---

### Phase 3: Damage Floor and Bonus Application Verification

Ensure the `WeaponData::bonus` value is applied correctly and the results are
floored at 1, consistent with the spec.

#### 3.1 Bonus Integration Detail

`DiceRoll` already has a `bonus: i8` (or equivalent signed field). In
`get_character_attack`, when building the `DiceRoll` from `weapon_data`:

```
DiceRoll {
    count: weapon_data.damage.count,
    sides: weapon_data.damage.sides,
    bonus: weapon_data.damage.bonus.saturating_add(weapon_data.bonus),
}
```

This propagates the bonus into `resolve_attack`'s existing damage roll. The
`resolve_attack` function handles the actual floor-at-1 clamping on the result.
If `resolve_attack` does not yet floor at 1, add `damage = damage.max(1)` after
the roll there — do not add it in `get_character_attack`.

#### 3.2 Verify `DiceRoll::bonus` Field Type

Confirm the field type of `DiceRoll::bonus` in `src/domain/types.rs`. If it is
`i8` (same as `WeaponData::bonus`), use `saturating_add`. If it is a different
type (e.g. `i16`), cast appropriately with no silent truncation.

#### 3.3 Testing Requirements

- `test_cursed_weapon_damage_floor_at_one` — equip a cursed weapon with bonus
  -10 on a 1d4 weapon; assert the minimum resolved damage is 1.
- `test_positive_bonus_adds_to_roll` — equip a +3 sword (1d6 base); verify
  minimum possible outcome is `1 + 3 = 4` (i.e. `DiceRoll::bonus` is 3,
  minimum roll of 1 gives 4).

#### 3.4 Deliverables

- [ ] `DiceRoll::bonus` field type verified and bonus applied via `saturating_add`
- [ ] `resolve_attack` floors damage at 1 (add if missing)
- [ ] Two boundary tests pass

#### 3.5 Success Criteria

A cursed weapon with a high negative bonus never reduces damage below 1. A
magical weapon's bonus raises the minimum and average damage correctly.

---

### Phase 4: Documentation and Final Validation

#### 4.1 Update `docs/explanation/implementations.md`

Add a section summarising what was built, which files changed, and what the
expected player-facing behaviour is.

#### 4.2 Final Quality Gate Run

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All four must pass with zero errors and zero warnings.

#### 4.3 Deliverables

- [ ] `docs/explanation/implementations.md` updated
- [ ] All quality gates pass

---

## Sequence Summary

| Phase | Core Output                                                                                                 | Key Files                                                   |
| ----- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| 1     | `UNARMED_DAMAGE`, `MeleeAttackResult`, `get_character_attack`, `has_ranged_weapon`, `is_ranged` on `Attack` | `src/domain/combat/engine.rs`, `src/domain/combat/types.rs` |
| 2     | Melee path wired + ranged-weapon guard                                                                      | `src/game/systems/combat.rs`                                |
| 2.5   | Sequencing note — ranged attack path deferred to combat-events plan                                         | _(no code)_                                                 |
| 3     | Bonus application and damage floor-at-1 verified                                                            | `src/domain/combat/engine.rs`, `src/domain/types.rs`        |
| 4     | Documentation + quality gates                                                                               | `docs/explanation/implementations.md`                       |

## Architecture Compliance Checklist

- [ ] `get_character_attack` is in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [ ] `MeleeAttackResult` is in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [ ] `has_ranged_weapon` is in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [ ] `UNARMED_DAMAGE` is a named constant, not a magic literal
- [ ] `is_ranged: bool` field added to `Attack` with `#[serde(default)]`
- [ ] `Attack::ranged(damage)` constructor sets `is_ranged = true`
- [ ] `Attack::physical(damage)` constructor keeps `is_ranged = false`
- [ ] Melee path in `perform_attack_action_with_rng` returns `Ok(())` (no damage, with warning) when `MeleeAttackResult::Ranged` is returned
- [ ] `ItemId`, `DiceRoll` type aliases used, not raw primitives
- [ ] All public functions have `///` doc comments with runnable doctests
- [ ] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [ ] SPDX header present in all modified `.rs` files
- [ ] Coordination note checked: no double-definition of `is_ranged` / `has_ranged_weapon` if `combat_events_implementation_plan.md` Phase 3 landed first
- [ ] `docs/explanation/implementations.md` updated after completion
