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

#### 1.2 Add `get_character_attack` Function

Add to `src/domain/combat/engine.rs`:

```
pub fn get_character_attack(
    character: &Character,
    item_db: &ItemDatabase,
) -> Attack
```

Logic (in order):

1. If `character.equipment.weapon` is `None`, return
   `Attack::physical(UNARMED_DAMAGE)`.
2. Look up the item via `item_db.get_item(item_id)`.
3. If not found or the `item_type` is not `ItemType::Weapon(weapon_data)`,
   fall back to `Attack::physical(UNARMED_DAMAGE)`.
4. Build a `DiceRoll` from `weapon_data.damage`. Apply `weapon_data.bonus` to
   the `bonus` field of `DiceRoll` (clamping so the combined bonus is at most
   `i8::MAX` to stay in range).
5. Return `Attack::physical(adjusted_dice_roll)`.

The bonus application rule: `damage = roll(weapon_data.damage) + weapon_data.bonus`,
floored at 1. Because `Attack::physical` stores a `DiceRoll`, embed the bonus
directly in `DiceRoll::bonus` so `resolve_attack`'s existing damage
calculation picks it up automatically.

The function is infallible (`-> Attack`, never `Result`) — all failure paths
return the unarmed fallback rather than propagating an error.

#### 1.3 Update Required Imports

`get_character_attack` needs `ItemDatabase` and `ItemType` in scope. Add the
necessary `use` lines to `engine.rs`:

```
use crate::domain::items::{ItemDatabase, ItemType};
```

#### 1.4 Testing Requirements

Unit tests inside `src/domain/combat/engine.rs` (under `#[cfg(test)]`):

- `test_get_character_attack_no_weapon_returns_unarmed` — `equipment.weapon = None`; assert `attack.damage == UNARMED_DAMAGE`.
- `test_get_character_attack_with_weapon_returns_weapon_damage` — equip a
  known weapon (1d8 longsword, bonus 0); assert `attack.damage ==
  DiceRoll::new(1, 8, 0)`.
- `test_get_character_attack_weapon_bonus_applied` — equip a +2 sword; assert
  `attack.damage.bonus == 2`.
- `test_get_character_attack_unknown_item_id_falls_back` — equip item_id 99
  (not in db); assert `attack.damage == UNARMED_DAMAGE` (no panic).
- `test_get_character_attack_non_weapon_item_falls_back` — equip a Consumable
  item; assert fallback to `UNARMED_DAMAGE`.

#### 1.5 Deliverables

- [ ] `UNARMED_DAMAGE` constant in `src/domain/combat/engine.rs`
- [ ] `get_character_attack(character, item_db) -> Attack` in `src/domain/combat/engine.rs`
- [ ] Required `use` imports added
- [ ] All five unit tests pass

#### 1.6 Success Criteria

`cargo check --all-targets --all-features` and
`cargo nextest run --all-features` pass with zero errors/warnings. The function
is callable from outside the module by importing
`antares::domain::combat::engine::get_character_attack`.

---

### Phase 2: Game System Integration

Replace the hardcoded `DiceRoll::new(1, 4, 0)` player attack in
`perform_attack_action_with_rng` with a call to `get_character_attack`.

#### 2.1 Locate the Hardcoded Player Attack

In [src/game/systems/combat.rs](../../src/game/systems/combat.rs), line ~1992:

```rust
CombatantId::Player(_) => {
    crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
}
```

#### 2.2 Extract Character Reference

Before the `attack_data` match block, extract the attacker character when the
attacker is a player:

1. When `action.attacker` is `CombatantId::Player(idx)`, get the `Combatant`
   from `combat_res.state.participants.get(idx)`.
2. Extract the inner `Character` reference from `Combatant::Player(pc)`.
3. Call `get_character_attack(pc, &content.db().items)`.

#### 2.3 Replace Hardcoded Attack

Replace:

```rust
CombatantId::Player(_) => {
    crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
}
```

With:

```rust
CombatantId::Player(idx) => {
    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
        get_character_attack(pc, &content.db().items)
    } else {
        return Err(CombatError::CombatantNotFound(action.attacker));
    }
}
```

Add the import at the top of `combat.rs`:

```rust
use crate::domain::combat::engine::get_character_attack;
```

#### 2.4 Leave Monster Path Unchanged

`perform_monster_turn_with_rng` and the `CombatantId::Monster` branch of
`perform_attack_action_with_rng` must not be modified.

#### 2.5 Testing Requirements

Integration-level tests (Bevy `App` tests or pure function tests with a
constructed `CombatResource`):

- `test_player_attack_uses_equipped_weapon_damage` — equip a longsword
  (1d8, bonus 0) on the attacker; run `perform_attack_action_with_rng` with
  seeded RNG; assert applied damage is in range [1, 8], not the old [1, 4].
- `test_player_attack_unarmed_when_no_weapon` — `equipment.weapon = None`; run
  attack; assert damage is in range [1, 2] (before might modifier).
- `test_player_attack_bonus_weapon_floor_at_one` — equip a -3 cursed dagger
  (1d4, bonus -3); assert damage is at least 1, never 0 or negative.

All tests use `data/test_campaign` fixtures — no reference to
`campaigns/tutorial`.

#### 2.6 Deliverables

- [ ] Hardcoded `DiceRoll::new(1, 4, 0)` removed from player attack branch
- [ ] `get_character_attack` wired into `perform_attack_action_with_rng`
- [ ] `use` import added in `combat.rs`
- [ ] Three integration tests pass

#### 2.7 Success Criteria

All four quality gates pass. In the running game, a character equipped with a
named weapon deals damage in that weapon's stated range, not the old 1d4 range.
An unarmed character deals 1d2 damage.

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

| Phase | Core Output | Key Files |
|-------|-------------|-----------|
| 1 | `UNARMED_DAMAGE` constant + `get_character_attack` function | `src/domain/combat/engine.rs` |
| 2 | `perform_attack_action_with_rng` wired to use equipped weapon | `src/game/systems/combat.rs` |
| 3 | Bonus application and damage floor-at-1 verified | `src/domain/combat/engine.rs`, `src/domain/types.rs` |
| 4 | Documentation + quality gates | `docs/explanation/implementations.md` |

## Architecture Compliance Checklist

- [ ] `get_character_attack` is in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [ ] `UNARMED_DAMAGE` is a named constant, not a magic literal
- [ ] `ItemId`, `DiceRoll` type aliases used, not raw primitives
- [ ] All public functions have `///` doc comments with runnable doctests
- [ ] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [ ] SPDX header present in all modified `.rs` files
- [ ] `docs/explanation/implementations.md` updated after completion
