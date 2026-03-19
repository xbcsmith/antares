# Unconscious and Dead Characters Implementation Plan

## Overview

Characters reduced to 0 HP must become unconscious and be unable to act in
combat. Dead characters must require resurrection. Both states must be
data-driven campaign conditions so campaign creators can customize behavior
(e.g. permadeath, instant death). This plan fixes the existing bug where 0
HP characters can still attack, adds the `ActiveCondition` lifecycle for
unconscious and dead states, wires healing and rest to revive unconscious
characters, implements a resurrection mechanic (items, spells, and NPC
services), and adds campaign-level permadeath configuration.

## Current State Analysis

### Existing Infrastructure

- **`Condition` bitflag struct** in
  `src/domain/character.rs` (L484–554): constants `UNCONSCIOUS = 64`,
  `DEAD = 128`, `STONE = 160`, `ERADICATED = 255`. Has `is_unconscious()`,
  `is_fatal()`, `is_bad()`, and `has()` methods. No `is_dead()` method.

- **`Character::is_alive()`** in `src/domain/character.rs` (L1245–1247):
  returns `!self.conditions.is_fatal()`. Does **not** check `hp.current > 0`.
  Monster `is_alive()` checks `hp.current > 0 && !conditions.is_dead()` — the
  two are inconsistent.

- **`Character::can_act()`** in `src/domain/character.rs` (L1250–1252):
  delegates to `is_alive() && !conditions.is_bad()`. Fixing `is_alive()`
  cascades here automatically.

- **Data-driven condition system** in `src/domain/conditions.rs`:
  `ConditionDefinition`, `ActiveCondition`, `ConditionDuration::Permanent`.
  13 conditions exist in `campaigns/tutorial/data/conditions.ron`. **No
  `"unconscious"` or `"dead"` entries exist in either that file or in
  `data/test_campaign/data/conditions.ron`.**

- **`status_str_to_flag`** in `src/domain/combat/engine.rs` (L1267–1282):
  already maps `"unconscious"` → `UNCONSCIOUS`, `"dead"` → `DEAD`. Wiring
  exists, but nothing fires it.

- **`reconcile_character_conditions`** in `src/domain/combat/engine.rs`
  (L1176–1231): syncs `ActiveCondition` entries → bitflags. It lists
  `UNCONSCIOUS` and `DEAD` in its `flag_list`. **This is the authoritative
  sync path. If an `ActiveCondition("unconscious")` exists, the bitflag is
  set; if not, the bitflag is cleared.** Setting the bitflag directly without
  a matching `ActiveCondition` will be undone the next time
  `reconcile_character_conditions` runs.

- **`apply_condition_to_character`** in `src/domain/combat/engine.rs`
  (L1000–1050): handles `StatusEffect("unconscious")` by calling
  `target.conditions.add(Condition::UNCONSCIOUS)`. This path works once the
  RON entries exist and conditions are applied properly.

- **`apply_damage`** in `src/domain/combat/engine.rs` (L712–731): reduces
  `hp.current` to 0 but **never sets `UNCONSCIOUS`** — the core bug.

- **`apply_starvation_damage`** in `src/domain/resources.rs` (L855–871):
  sets `Condition::DEAD` bitflag when HP hits 0 but does **not** push an
  `ActiveCondition`. This is inconsistent with the data-driven approach and
  will be fixed in Phase 2.

- **`rest_party` and `rest_party_hour`** in `src/domain/resources.rs`
  (L645–690 and L645–697): both skip characters where
  `conditions.is_fatal() || conditions.is_unconscious()`. The skip-unconscious
  logic must change so resting heals unconscious characters back above 0 HP.

- **`select_monster_target`** in `src/game/systems/combat.rs` (L3869–3912):
  already filters candidates on `pc.is_alive()`. **No change is needed here.**
  Fixing `Character::is_alive()` to check `hp.current > 0` automatically
  prevents monsters from targeting unconscious players.

- **`CombatState::check_combat_end`** in `src/domain/combat/engine.rs`
  (L284–289): calls `alive_party_count()`, which uses `is_alive()`. After
  fixing `is_alive()`, a party where all members have 0 HP will correctly
  trigger `CombatStatus::Defeat`.

- **`sync_combat_to_party_on_exit`** in `src/game/systems/combat.rs`
  (L1320–1414): already copies `conditions` bitflag and `active_conditions`
  vec from combat participants back to `party.members` on exit. No changes
  needed here.

- **HUD** in `src/game/systems/hud.rs`: already defines `PRIORITY_UNCONSCIOUS
= 90`, `PRIORITY_DEAD = 100`, and `get_priority_condition` returns
  `"💤 Unconscious"` and `"💀 Dead"`. **No HUD changes are required.**

- **`HealHp` in `apply_consumable_effect`** in
  `src/domain/items/consumable_usage.rs` (L229–241): modifies `hp.current`
  but does **not** clear `UNCONSCIOUS`. The `CureCondition` handler (L258–262)
  clears bitflags only and explicitly leaves `active_conditions` untouched.

- **`apply_consumable_effect_exploration`** in
  `src/domain/items/consumable_usage.rs` (L382–415): delegates to
  `apply_consumable_effect` for all effects except timed resistance boosts.
  Healing revive logic added to `apply_consumable_effect` will propagate here
  automatically.

- **`ConsumableEffect` enum** in `src/domain/items/types.rs` (L354–392):
  variants are `HealHp`, `RestoreSp`, `CureCondition`, `BoostAttribute`,
  `BoostResistance`, `IsFood`. No `Resurrect` variant exists.

- **`Spell` struct** in `src/domain/magic/types.rs`: has `damage`,
  `applied_conditions`, and `duration` fields. Has no explicit healing or
  resurrection effect field.

- **`ServiceEntry` and `ServiceCatalog`** in `src/domain/inventory.rs`
  (L333–514): fully implemented. `ServiceEntry::with_gem_cost("raise_dead",
...)` appears in existing tests. `NpcDatabase::priests()` exists in
  `src/sdk/database.rs` (L930–932). The data infrastructure for NPC
  resurrection services already exists.

- **`CampaignConfig`** in `src/domain/campaign.rs` (L104–155): fields are
  `max_party_level`, `difficulty_multiplier`, `experience_rate`, `gold_rate`,
  `random_encounter_rate`, `rest_healing_rate`, `custom_rules`. No permadeath
  or death-mode flags exist yet.

### Identified Issues

1. **Bug**: `apply_damage` in `src/domain/combat/engine.rs` (L712–731)
   reduces HP to 0 but does not set `UNCONSCIOUS` bitflag or push an
   `ActiveCondition`. Characters at 0 HP can still act.

2. **Bug**: `Character::is_alive()` does not check `hp.current > 0`,
   unlike `Monster::is_alive()`. Characters at 0 HP with no condition set
   are considered alive.

3. **Missing data**: No `"unconscious"` or `"dead"` entries in
   `campaigns/tutorial/data/conditions.ron` or
   `data/test_campaign/data/conditions.ron`.

4. **Reconcile will undo direct bitflag writes**: Setting `UNCONSCIOUS`
   directly without pushing a matching `ActiveCondition("unconscious",
Permanent)` will be cleared the next time `reconcile_character_conditions`
   runs. Both must be written together.

5. **`CureCondition` does not clear `active_conditions`**: The handler in
   `apply_consumable_effect` (L258–262) is explicitly marked "intentionally
   untouched". The revive and resurrection paths require clearing both the
   bitflag and the `active_conditions` entry.

6. **No revival mechanic**: Dead characters have no code path back to life
   except save-file editing.

7. **Healing does not clear `UNCONSCIOUS`**: `HealHp` items and spells
   restore HP but do not remove the condition.

8. **Rest skips unconscious characters**: `rest_party` and `rest_party_hour`
   skip `is_unconscious()` characters entirely; they should be healed and
   revived when HP rises above 0.

9. **`apply_starvation_damage` inconsistency**: Sets `Condition::DEAD`
   bitflag without pushing an `ActiveCondition`. Must push
   `ActiveCondition("dead", Permanent)` after Phase 2 data exists.

10. **No permadeath configuration**: `CampaignConfig` has no flag for
    permadeath or for choosing instant-death vs. unconscious-first mode.

11. **No `Condition::is_dead()` helper**: Distinguishing dead from stone
    or eradicated requires calling `has(Condition::DEAD)` explicitly.
    A named helper improves readability and prevents logic errors.

12. **No `ConsumableEffect::Resurrect` variant**: Resurrection via items
    requires a dedicated effect that clears `DEAD`, removes the
    `active_conditions` entry, and restores HP in one atomic operation.

13. **No resurrection spell effect field in `Spell`**: The `Spell` struct
    has no field for healing or resurrection effects, only `damage` and
    `applied_conditions`.

14. **No application-layer resurrection service function**: The
    `ServiceCatalog` infrastructure exists, but there is no function to
    execute a `"raise_dead"` service transaction (charge gold/gems, call
    the domain revive helper, validate permadeath).

15. **SDK `ContentDatabase::validate()` does not warn on missing system
    conditions**: A campaign without `"unconscious"` or `"dead"` in its
    conditions will silently misbehave.

---

## Implementation Phases

---

### Phase 1: Core Unconscious Condition

Wire the unconscious condition end-to-end: HP reaching 0 sets
`UNCONSCIOUS`, healing above 0 clears it, rest revives unconscious
characters, and combat respects the condition.

---

#### 1.1 Add `"unconscious"` to Both `conditions.ron` Files

**Files to modify:**

- `campaigns/tutorial/data/conditions.ron`
- `data/test_campaign/data/conditions.ron`

**Action:** Append the following entry to both files:

```ron
(
    id: "unconscious",
    name: "Unconscious",
    description: "Character is at 0 HP and cannot act. Revived by healing above 0 HP or by resting.",
    effects: [
        StatusEffect("unconscious"),
    ],
    default_duration: Permanent,
    icon_id: Some("icon_unconscious"),
),
```

**Validation:** Load both files with `ConditionDatabase::load_from_file` in
a test and assert `db.has_condition("unconscious") == true`.

---

#### 1.2 Add `Condition::is_dead()` Helper

**File:** `src/domain/character.rs`

**Location:** `impl Condition` block after the `is_unconscious()` method
(currently at L536–538).

**Action:** Add the following public method with a `///` doc comment:

```rust
/// Returns true if the character is dead (DEAD bit is set but not STONE
/// or ERADICATED, which set higher bits).
pub fn is_dead(&self) -> bool {
    self.has(Self::DEAD) && self.0 < Self::STONE
}
```

**Why:** `is_fatal()` is `>= DEAD` and also catches `STONE` and
`ERADICATED`. Resurrection must only target characters where `is_dead()`
is true, not stone or eradicated. Every caller that needs to distinguish
dead from stone must use this new helper.

---

#### 1.3 Fix `Character::is_alive()` to Check `hp.current`

**File:** `src/domain/character.rs`

**Location:** `pub fn is_alive` at L1245–1247.

**Current implementation:**

```rust
pub fn is_alive(&self) -> bool {
    !self.conditions.is_fatal()
}
```

**New implementation:**

```rust
/// Returns true if the character is alive (HP above 0 and not dead,
/// stoned, or eradicated).
pub fn is_alive(&self) -> bool {
    self.hp.current > 0 && !self.conditions.is_fatal()
}
```

**Cascading effects that require no further changes:**

- `Character::can_act()` delegates to `is_alive()` — automatically fixed.
- `Combatant::is_alive()` in `src/domain/combat/engine.rs` delegates to
  `character.is_alive()` — automatically fixed.
- `CombatState::alive_party_count()` uses `Combatant::is_alive()` —
  automatically fixed. A party where all members have 0 HP will now
  correctly produce `alive_party_count() == 0` and trigger
  `CombatStatus::Defeat` via `check_combat_end()`.
- `select_monster_target` in `src/game/systems/combat.rs` (L3869) filters
  on `pc.is_alive()` — automatically fixed. No separate monster-targeting
  task is required.
- `calculate_turn_order` filters on `is_alive()` — automatically fixed.

---

#### 1.4 Extract `revive_from_unconscious` Helper

**File:** `src/domain/resources.rs`

**Action:** Add a new public helper function **before** `rest_party_hour`.
This function is the single authoritative place to clear the unconscious
state. All callers (rest, healing items, healing spells) must use it.

```rust
/// Revives a character from unconscious if their HP is above 0.
///
/// Clears the `UNCONSCIOUS` bitflag AND removes any `ActiveCondition`
/// with `condition_id == "unconscious"` from `active_conditions`.
/// Both must be cleared together to prevent `reconcile_character_conditions`
/// from re-setting the bitflag on the next turn tick.
///
/// This function is a no-op if the character is not unconscious or if
/// `hp.current == 0`.
///
/// # Arguments
///
/// * `character` - The character to potentially revive
pub fn revive_from_unconscious(character: &mut crate::domain::character::Character) {
    use crate::domain::character::Condition;
    if character.conditions.has(Condition::UNCONSCIOUS) && character.hp.current > 0 {
        character.conditions.remove(Condition::UNCONSCIOUS);
        character.remove_condition("unconscious");
    }
}
```

**Note:** `character.remove_condition` is defined in `character.rs`
(L1269–1272) and removes from `active_conditions` by `condition_id`.

---

#### 1.5 Set `UNCONSCIOUS` in `apply_damage`

**File:** `src/domain/combat/engine.rs`

**Location:** `pub fn apply_damage` at L712–731.

**Current `Combatant::Player` branch:**

```rust
Combatant::Player(character) => {
    let old_hp = character.hp.current;
    character.hp.modify(-(damage as i32));
    character.hp.current == 0 && old_hp > 0
}
```

**New `Combatant::Player` branch:**

```rust
Combatant::Player(character) => {
    use crate::domain::conditions::{ActiveCondition, ConditionDuration};
    let old_hp = character.hp.current;
    character.hp.modify(-(damage as i32));
    let just_downed = character.hp.current == 0 && old_hp > 0;
    if just_downed && !character.conditions.has(crate::domain::character::Condition::UNCONSCIOUS) {
        // Set both the bitflag AND the ActiveCondition so that
        // reconcile_character_conditions does not clear the flag on
        // the next turn tick.
        character.conditions.add(crate::domain::character::Condition::UNCONSCIOUS);
        character.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
    }
    just_downed
}
```

**Key constraint:** Only set `UNCONSCIOUS` when `just_downed` is true
(HP crossed 0 this hit). Characters already at 0 HP must not have the
condition pushed again.

---

#### 1.6 Clear `UNCONSCIOUS` in `apply_consumable_effect` (`HealHp` branch)

**File:** `src/domain/items/consumable_usage.rs`

**Location:** `pub fn apply_consumable_effect`, `ConsumableEffect::HealHp`
match arm at L229–241.

**Action:** After the existing HP modification and clamp, call
`revive_from_unconscious`:

```rust
ConsumableEffect::HealHp(amount) => {
    let pre = character.hp.current as i32;
    character.hp.modify(amount as i32);
    if character.hp.current > character.hp.base {
        character.hp.current = character.hp.base;
    }
    let post = character.hp.current as i32;
    let healed = post - pre;
    if healed > 0 {
        result.healing = healed;
    }
    // Revive from unconscious if HP is now above 0.
    crate::domain::resources::revive_from_unconscious(character);
}
```

**Note:** `apply_consumable_effect_exploration` delegates to
`apply_consumable_effect` for `HealHp`, so the exploration (out-of-combat)
path is covered automatically.

---

#### 1.7 Clear `UNCONSCIOUS` in Spell Healing Path

**File:** `src/domain/combat/spell_casting.rs`

**Location:** `execute_spell_cast_with_spell`, inside the
`if let Some(dice) = &spell.damage` block, `SpellTarget::SingleCharacter`
branch and `SpellTarget::AllCharacters` branch.

**Action:** After applying damage to player HP via `pc.hp.modify(-dmg)`,
call `crate::domain::resources::revive_from_unconscious(pc)` when `dmg`
is negative (healing). The current spell path only handles damage; healing
spells use negative damage values. Add the revive call after any HP
modification on a `Combatant::Player` combatant.

---

#### 1.8 Update `rest_party` and `rest_party_hour` to Revive Unconscious

**File:** `src/domain/resources.rs`

**Location:** Both `rest_party` (L645) and `rest_party_hour` (L645–697).

**Current guard in both functions:**

```rust
if character.conditions.is_fatal() || character.conditions.is_unconscious() {
    continue;
}
```

**New guard — skip only fatal (dead/stone/eradicated):**

```rust
if character.conditions.is_fatal() {
    continue;
}
```

**After the HP restoration block in both functions**, call revive:

```rust
// After HP restoration for this character:
crate::domain::resources::revive_from_unconscious(character);
```

**Why:** Unconscious characters have 0 HP. Rest restores HP. Once HP is
above 0, `revive_from_unconscious` clears the condition. If the full rest
does not restore any HP (e.g., a character with `base == 0`, which is
invalid but defensive), the guard keeps them unconscious.

---

#### 1.9 Testing Requirements

All tests must be placed in the relevant module's `#[cfg(test)]` block.
Test data must use `data/test_campaign`, never `campaigns/tutorial`.

| Test name                                          | File                  | What it verifies                                                                                                                                                               |
| -------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_apply_damage_sets_unconscious_at_zero_hp`    | `engine.rs`           | After `apply_damage` reduces HP to 0, `character.conditions.has(UNCONSCIOUS) == true` and `character.active_conditions` contains an entry with `condition_id == "unconscious"` |
| `test_apply_damage_already_at_zero_no_double_push` | `engine.rs`           | Calling `apply_damage` on a character already at 0 HP does not push a second `ActiveCondition`                                                                                 |
| `test_character_is_alive_false_at_zero_hp`         | `character.rs`        | `Character::is_alive()` returns `false` when `hp.current == 0` regardless of conditions                                                                                        |
| `test_character_can_act_false_at_zero_hp`          | `character.rs`        | `Character::can_act()` returns `false` when `hp.current == 0`                                                                                                                  |
| `test_condition_is_dead_helper`                    | `character.rs`        | `is_dead()` is `true` for `DEAD` bit alone, `false` for `STONE` and `ERADICATED`                                                                                               |
| `test_heal_hp_clears_unconscious`                  | `consumable_usage.rs` | `HealHp` on an unconscious character above 0 HP removes `UNCONSCIOUS` bitflag and `active_conditions` entry                                                                    |
| `test_heal_hp_does_not_clear_when_still_zero`      | `consumable_usage.rs` | `HealHp(0)` or healing that does not raise HP above 0 leaves `UNCONSCIOUS` set                                                                                                 |
| `test_revive_from_unconscious_noop_when_fine`      | `resources.rs`        | `revive_from_unconscious` is a no-op on a character with `FINE` condition                                                                                                      |
| `test_revive_from_unconscious_clears_both`         | `resources.rs`        | Clears bitflag and `active_conditions` entry                                                                                                                                   |
| `test_monster_skips_unconscious_target`            | `combat.rs`           | A monster with `is_alive() == false` (0 HP) is not returned by `select_monster_target`                                                                                         |
| `test_rest_revives_unconscious_character`          | `resources.rs`        | After `rest_party`, an unconscious character with 0 HP and non-zero base HP has `UNCONSCIOUS` cleared and HP above 0                                                           |
| `test_rest_does_not_heal_fatal_character`          | `resources.rs`        | A character with `DEAD` bit set is skipped entirely by rest                                                                                                                    |
| `test_unconscious_party_triggers_defeat`           | `combat.rs`           | When all player combatants have 0 HP, `check_combat_end` sets `CombatStatus::Defeat`                                                                                           |
| `test_unconscious_condition_in_ron_loaded`         | `database.rs`         | `ConditionDatabase::load_from_file("data/test_campaign/data/conditions.ron")` contains `"unconscious"`                                                                         |

---

#### 1.10 Deliverables

- [ ] `"unconscious"` entry added to `campaigns/tutorial/data/conditions.ron`
- [ ] `"unconscious"` entry added to `data/test_campaign/data/conditions.ron`
- [ ] `Condition::is_dead()` method added to `src/domain/character.rs`
- [ ] `Character::is_alive()` checks `hp.current > 0`
- [ ] `revive_from_unconscious(character)` helper in `src/domain/resources.rs`
- [ ] `apply_damage` sets `UNCONSCIOUS` bitflag and `ActiveCondition` on 0 HP
- [ ] `HealHp` consumable calls `revive_from_unconscious` after HP restoration
- [ ] Spell healing path calls `revive_from_unconscious` after HP modification
- [ ] `rest_party` and `rest_party_hour` remove the `is_unconscious()` skip
      and call `revive_from_unconscious` after per-character HP restoration
- [ ] All tests listed in 1.9 pass
- [ ] All four quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`

#### 1.11 Success Criteria

- A character reduced to 0 HP in combat is immediately marked unconscious
  (bitflag + `ActiveCondition`) and cannot act.
- Monsters do not target unconscious (0 HP) characters.
- A party where all members have 0 HP triggers a combat defeat.
- Healing a character above 0 HP removes the unconscious condition.
- Resting heals unconscious characters and revives them when HP > 0.
- All quality gates pass with zero errors and zero warnings.

---

### Phase 2: Core Dead Condition

Add the Dead condition, the transition from unconscious to dead, a
resurrection mechanic for items and spells, campaign-level permadeath
configuration, and consistent data-driven `DEAD` handling.

---

#### 2.1 Add `"dead"` to Both `conditions.ron` Files

**Files to modify:**

- `campaigns/tutorial/data/conditions.ron`
- `data/test_campaign/data/conditions.ron`

**Action:** Append the following entry to both files:

```ron
(
    id: "dead",
    name: "Dead",
    description: "Character is dead. Requires resurrection to revive. Cannot act, be targeted, or be healed by rest.",
    effects: [
        StatusEffect("dead"),
    ],
    default_duration: Permanent,
    icon_id: Some("icon_dead"),
),
```

**Validation:** `db.has_condition("dead") == true` in a test that loads
`data/test_campaign/data/conditions.ron`.

---

#### 2.2 Add Permadeath and Death Mode Flags to `CampaignConfig`

**File:** `src/domain/campaign.rs`

**Location:** `pub struct CampaignConfig` at L104.

**Action:** Add two new fields with `#[serde(default)]` for backward
compatibility with existing `campaign.ron` files:

```rust
/// If true, dead characters cannot be resurrected by any means.
/// Campaign creators set this for permadeath runs. Default: false.
#[serde(default)]
pub permadeath: bool,

/// If true, a character that reaches 0 HP becomes unconscious first and
/// only dies if they receive further damage while unconscious.
/// If false, reaching 0 HP sets DEAD immediately (instant death mode).
/// Default: true (unconscious before death, classic RPG behavior).
#[serde(default = "default_true")]
pub unconscious_before_death: bool,
```

**Add the `default_true` helper function** (private, before `impl Default`):

```rust
fn default_true() -> bool { true }
```

**Update `impl Default for CampaignConfig`** to include:

```rust
permadeath: false,
unconscious_before_death: true,
```

**Note:** `CampaignConfig` is accessible at runtime via
`game_state.config` (a `CampaignConfig` field in `GameState`). Callers
needing the permadeath flag read `game_state.config.permadeath`. Callers
needing the death mode read `game_state.config.unconscious_before_death`.

---

#### 2.3 Implement Unconscious → Dead Transition in `apply_damage`

**File:** `src/domain/combat/engine.rs`

**Location:** `pub fn apply_damage`, `Combatant::Player` branch (modified in
Phase 1.5).

**Action:** After the existing 0 HP / unconscious logic, add a second check:
if the target is **already unconscious** (0 HP) and receives additional
damage, transition to dead.

The `apply_damage` function requires a `death_mode` parameter, or it must
read the mode from the `CombatState`. The cleanest approach is to add an
optional `death_mode: bool` to the function signature:

```rust
pub fn apply_damage(
    combat: &mut CombatState,
    target_id: CombatantId,
    damage: u16,
) -> Result<bool, CombatError>
```

This signature is unchanged. Instead, the death mode is checked inside the
function by reading `combat.unconscious_before_death` — a new field added
to `CombatState` (see 2.3a below).

**2.3a Add `unconscious_before_death` to `CombatState`:**

File: `src/domain/combat/engine.rs`, `pub struct CombatState` at L188.

Add:

```rust
/// Controls whether characters become unconscious before dying.
/// Copied from `CampaignConfig::unconscious_before_death` at combat start.
/// Default: true.
#[serde(default = "default_true")]
pub unconscious_before_death: bool,
```

Update `CombatState::new()` and `start_encounter` in
`src/game/systems/combat.rs` to copy the value from `CampaignConfig`.

**2.3b Update `apply_damage` player branch** with full dead/unconscious logic:

```rust
Combatant::Player(character) => {
    use crate::domain::conditions::{ActiveCondition, ConditionDuration};
    use crate::domain::character::Condition;

    let old_hp = character.hp.current;
    let already_unconscious = character.conditions.has(Condition::UNCONSCIOUS);

    character.hp.modify(-(damage as i32));

    if character.hp.current == 0 {
        if already_unconscious {
            // Further damage to an unconscious character — they die.
            // In instant-death mode (unconscious_before_death == false),
            // this path also runs on the first hit that reaches 0 HP.
            if !character.conditions.has(Condition::DEAD) {
                character.conditions.remove(Condition::UNCONSCIOUS);
                character.remove_condition("unconscious");
                character.conditions.add(Condition::DEAD);
                character.add_condition(ActiveCondition::new(
                    "dead".to_string(),
                    ConditionDuration::Permanent,
                ));
            }
        } else if combat.unconscious_before_death {
            // First hit to 0 HP: become unconscious.
            if !character.conditions.has(Condition::UNCONSCIOUS) {
                character.conditions.add(Condition::UNCONSCIOUS);
                character.add_condition(ActiveCondition::new(
                    "unconscious".to_string(),
                    ConditionDuration::Permanent,
                ));
            }
        } else {
            // Instant-death mode: skip unconscious, go straight to dead.
            character.conditions.add(Condition::DEAD);
            character.add_condition(ActiveCondition::new(
                "dead".to_string(),
                ConditionDuration::Permanent,
            ));
        }
    }
    character.hp.current == 0 && old_hp > 0
}
```

---

#### 2.4 Fix `apply_starvation_damage` for Data-Driven Consistency

**File:** `src/domain/resources.rs`

**Location:** `pub fn apply_starvation_damage` at L855–871.

**Current:** Sets `Condition::DEAD` bitflag directly without an
`ActiveCondition`.

**Action:** Replace direct bitflag write with the same pattern used in
`apply_damage`:

```rust
if character.hp.current == 0 {
    use crate::domain::conditions::{ActiveCondition, ConditionDuration};
    character.conditions.add(crate::domain::character::Condition::DEAD);
    character.add_condition(ActiveCondition::new(
        "dead".to_string(),
        ConditionDuration::Permanent,
    ));
}
```

**Dependency:** Requires the `"dead"` `conditions.ron` entry from 2.1 to
exist before this path is exercised. Add a note in the function doc comment.

---

#### 2.5 Add `ConsumableEffect::Resurrect` Variant

**File:** `src/domain/items/types.rs`

**Location:** `pub enum ConsumableEffect` at L354.

**Action:** Add a new variant after `CureCondition`:

```rust
/// Resurrect a dead character, restoring them to `hp` hit points.
///
/// - Clears the `DEAD` bitflag and removes the `"dead"` ActiveCondition.
/// - Restores HP to `hp` (clamped to `hp.base`).
/// - Fails silently (no-op) if the character is not dead (`is_dead() == false`)
///   or if the character is STONE or ERADICATED.
/// - Respects campaign permadeath: callers must check before calling.
Resurrect(u16),
```

---

#### 2.6 Extract `revive_from_dead` Helper

**File:** `src/domain/resources.rs`

**Action:** Add alongside `revive_from_unconscious`:

```rust
/// Revives a dead character, restoring them to `hp` hit points.
///
/// Clears the `DEAD` bitflag, removes the `"dead"` `ActiveCondition` entry,
/// and sets `hp.current` to `hp.min(character.hp.base)`.
///
/// This function is a no-op if `character.conditions.is_dead()` is `false`
/// (e.g. character is Stone or Eradicated — those cannot be revived here).
///
/// The caller is responsible for checking campaign permadeath before calling
/// this function.
pub fn revive_from_dead(character: &mut crate::domain::character::Character, hp: u16) {
    use crate::domain::character::Condition;
    if !character.conditions.is_dead() {
        return; // Not dead (could be Stone/Eradicated), no-op.
    }
    character.conditions.remove(Condition::DEAD);
    character.remove_condition("dead");
    let restored = hp.min(character.hp.base);
    character.hp.current = restored;
}
```

---

#### 2.7 Handle `ConsumableEffect::Resurrect` in `apply_consumable_effect`

**File:** `src/domain/items/consumable_usage.rs`

**Location:** `pub fn apply_consumable_effect` match block.

**Action:** Add a new match arm:

```rust
ConsumableEffect::Resurrect(hp) => {
    // Caller is responsible for permadeath validation.
    // This function only performs the domain operation.
    if character.conditions.is_dead() {
        crate::domain::resources::revive_from_dead(character, hp);
        result.healing = hp as i32;
    }
}
```

**Note:** `apply_consumable_effect_exploration` delegates to
`apply_consumable_effect` and will handle `Resurrect` automatically.

---

#### 2.8 Add Resurrection Spell Effect to `Spell` Struct

**File:** `src/domain/magic/types.rs`

**Location:** `pub struct Spell`.

**Action:** Add an optional resurrection field:

```rust
/// Optional resurrection effect: restores a dead character to this many HP.
/// When `Some(hp)`, the spell acts as a resurrection spell targeting a
/// `SingleCharacter`. When `None`, normal damage / condition logic applies.
#[serde(default)]
pub resurrect_hp: Option<u16>,
```

Update `Spell::new()` to initialize `resurrect_hp: None`.

**File:** `src/domain/combat/spell_casting.rs`

**Location:** `execute_spell_cast_with_spell`, after the `if let Some(dice)`
damage block.

**Action:** Add a resurrection handling block after the damage block:

```rust
// Resurrection spell handling
if let Some(hp) = spell.resurrect_hp {
    if let CombatantId::Player(idx) = target {
        if let Some(Combatant::Player(pc)) = combat_state.get_combatant_mut(&target) {
            if pc.conditions.is_dead() {
                crate::domain::resources::revive_from_dead(pc, hp);
                result = result.with_healing(hp as i32, vec![idx]);
            }
        }
    }
}
```

**Note:** Permadeath validation is the caller's responsibility. The domain
layer does not read `CampaignConfig`; the application/game layer enforces it.

---

#### 2.9 Enforce Permadeath in Application Layer

**File:** `src/application/resources.rs`

**Action:** Add a helper function used by both item use and spell casting
paths before any resurrection is attempted:

```rust
/// Returns an error if the campaign has permadeath enabled.
///
/// # Errors
///
/// Returns a descriptive error string when `game_state.config.permadeath == true`.
pub fn check_permadeath_allows_resurrection(
    config: &crate::domain::campaign::CampaignConfig,
) -> Result<(), String> {
    if config.permadeath {
        Err("Resurrection is not allowed in this campaign (permadeath enabled).".to_string())
    } else {
        Ok(())
    }
}
```

Callers in the game systems layer (`perform_use_item_action_with_rng` in
`src/game/systems/combat.rs` and spell casting) must call this before
applying a `Resurrect` effect or `resurrect_hp` spell.

---

#### 2.10 Testing Requirements

| Test name                                            | File                  | What it verifies                                                                                 |
| ---------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------ |
| `test_unconscious_to_dead_on_further_damage`         | `engine.rs`           | An unconscious (0 HP) player that takes further damage gets `DEAD` set and `UNCONSCIOUS` cleared |
| `test_instant_death_mode_skips_unconscious`          | `engine.rs`           | With `unconscious_before_death == false`, 0 HP immediately sets `DEAD`, never `UNCONSCIOUS`      |
| `test_unconscious_before_death_mode_default`         | `campaign.rs`         | `CampaignConfig::default()` has `unconscious_before_death == true` and `permadeath == false`     |
| `test_apply_starvation_damage_sets_dead_condition`   | `resources.rs`        | `apply_starvation_damage` at 0 HP sets `DEAD` bitflag AND pushes `ActiveCondition("dead")`       |
| `test_revive_from_dead_clears_both`                  | `resources.rs`        | `revive_from_dead` removes `DEAD` bitflag, removes `active_conditions` entry, sets HP            |
| `test_revive_from_dead_noop_on_stone`                | `resources.rs`        | `revive_from_dead` is a no-op when `STONE` is set                                                |
| `test_revive_from_dead_noop_on_eradicated`           | `resources.rs`        | `revive_from_dead` is a no-op when `ERADICATED` is set                                           |
| `test_resurrect_consumable_clears_dead`              | `consumable_usage.rs` | `ConsumableEffect::Resurrect(5)` on a dead character clears `DEAD` and sets HP to 5              |
| `test_resurrect_consumable_noop_on_alive`            | `consumable_usage.rs` | `ConsumableEffect::Resurrect(5)` on a living character is a no-op                                |
| `test_resurrect_consumable_noop_on_eradicated`       | `consumable_usage.rs` | `ConsumableEffect::Resurrect(5)` is a no-op for `ERADICATED` characters                          |
| `test_permadeath_blocks_resurrection`                | `resources.rs`        | `check_permadeath_allows_resurrection` returns `Err` when `permadeath == true`                   |
| `test_dead_character_skipped_in_rest`                | `resources.rs`        | A character with `DEAD` bitflag is not healed by `rest_party`                                    |
| `test_dead_condition_in_ron_loaded`                  | `database.rs`         | `ConditionDatabase::load_from_file("data/test_campaign/data/conditions.ron")` contains `"dead"`  |
| `test_resurrect_spell_revives_dead_player`           | `spell_casting.rs`    | A spell with `resurrect_hp: Some(1)` removes `DEAD` and sets HP to 1 on the target               |
| `test_resurrect_spell_noop_on_alive`                 | `spell_casting.rs`    | A resurrection spell targeting a living player is a no-op                                        |
| `test_combat_state_unconscious_before_death_default` | `engine.rs`           | `CombatState::new()` sets `unconscious_before_death == true`                                     |

---

#### 2.11 Deliverables

- [ ] `"dead"` entry added to `campaigns/tutorial/data/conditions.ron`
- [ ] `"dead"` entry added to `data/test_campaign/data/conditions.ron`
- [ ] `permadeath: bool` and `unconscious_before_death: bool` added to
      `CampaignConfig` in `src/domain/campaign.rs`
- [ ] `unconscious_before_death: bool` added to `CombatState`; copied from
      `CampaignConfig` in `start_encounter`
- [ ] `apply_damage` transitions unconscious → dead on further damage
      (respects `unconscious_before_death` flag)
- [ ] `apply_starvation_damage` pushes `ActiveCondition("dead", Permanent)`
- [ ] `revive_from_dead(character, hp)` helper in `src/domain/resources.rs`
- [ ] `ConsumableEffect::Resurrect(u16)` variant in
      `src/domain/items/types.rs`
- [ ] `apply_consumable_effect` handles `Resurrect`
- [ ] `Spell::resurrect_hp: Option<u16>` field in
      `src/domain/magic/types.rs`
- [ ] `execute_spell_cast_with_spell` handles `resurrect_hp`
- [ ] `check_permadeath_allows_resurrection` in
      `src/application/resources.rs`
- [ ] Game-layer callers enforce permadeath before applying resurrection
- [ ] All tests listed in 2.10 pass
- [ ] All four quality gates pass

#### 2.12 Success Criteria

- A character killed via `apply_damage` while already unconscious becomes
  dead (DEAD bitflag + `ActiveCondition`).
- In instant-death mode (`unconscious_before_death == false`), reaching 0 HP
  sets DEAD immediately with no unconscious step.
- Dead characters are not healed by rest.
- `ConsumableEffect::Resurrect(hp)` on a dead character restores them to
  `hp` HP.
- A spell with `resurrect_hp: Some(1)` revives a dead combat participant.
- Permadeath flag prevents resurrection at the application layer.
- All quality gates pass.

---

### Phase 3: SDK Validation and Content Database Defaults

Ensure that missing `"unconscious"` and `"dead"` conditions are detected
early, and that new campaigns created via the SDK include them by default.

---

#### 3.1 Add Validation Warnings for Missing System Conditions

**File:** `src/sdk/database.rs`

**Location:** `impl ContentDatabase`, `pub fn validate` at L1430–1534.

**Action:** Add checks for the two required system conditions. If either
is absent, add a `ValidationError` (or `Warning`) to the result:

```rust
// System conditions: "unconscious" and "dead" must be present.
for required_id in &["unconscious", "dead"] {
    if !self.conditions.has_condition(required_id) {
        errors.push(format!(
            "Campaign is missing required system condition '{}'. \
             Characters at 0 HP will not behave correctly.",
            required_id
        ));
    }
}
```

**Note:** Use the existing `errors` accumulation pattern already present in
`validate()`. Treat missing system conditions as errors, not warnings, since
their absence causes silent bugs.

---

#### 3.2 Add Default System Conditions to `ConditionDatabase::new()`

**File:** `src/sdk/database.rs`

**Location:** `impl ConditionDatabase`, `pub fn new()` at L580–584.

**Action:** Pre-populate the empty database with the two system conditions
so that any code path creating a `ContentDatabase` programmatically (tests,
SDK tooling) has them by default:

```rust
pub fn new() -> Self {
    use crate::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};
    let mut db = Self { conditions: HashMap::new() };
    // System conditions: always present in every campaign.
    let system_conditions = vec![
        ConditionDefinition {
            id: "unconscious".to_string(),
            name: "Unconscious".to_string(),
            description: "Character is at 0 HP and cannot act.".to_string(),
            effects: vec![ConditionEffect::StatusEffect("unconscious".to_string())],
            default_duration: ConditionDuration::Permanent,
            icon_id: Some("icon_unconscious".to_string()),
        },
        ConditionDefinition {
            id: "dead".to_string(),
            name: "Dead".to_string(),
            description: "Character is dead. Requires resurrection.".to_string(),
            effects: vec![ConditionEffect::StatusEffect("dead".to_string())],
            default_duration: ConditionDuration::Permanent,
            icon_id: Some("icon_dead".to_string()),
        },
    ];
    for c in system_conditions {
        db.conditions.insert(c.id.clone(), c);
    }
    db
}
```

**Why `ConditionDatabase::new()` and not `templates.rs`:** `templates.rs`
contains item templates only (weapons, armor, potions, maps). Condition
defaults belong in the database layer. `ContentDatabase::new()` calls
`ConditionDatabase::new()`, so pre-populating there ensures all code paths
that create a fresh database get the system conditions.

---

#### 3.3 Add Default Resurrection Item Template to `templates.rs`

**File:** `src/sdk/templates.rs`

**Action:** Add a `resurrection_scroll()` template function using the new
`ConsumableEffect::Resurrect(1)` variant, following the exact same pattern
as `healing_potion()`:

```rust
/// Creates a default resurrection scroll template
///
/// A single-use item that revives a dead character to 1 HP.
/// Campaign creators can modify the HP amount and cost.
pub fn resurrection_scroll() -> Item { ... }
```

Add a corresponding test `test_resurrection_scroll` that asserts the
effect is `ConsumableEffect::Resurrect(1)` and `is_combat_usable == false`.

---

#### 3.4 Add Default Resurrection Spell to `data/test_campaign`

**File:** `data/test_campaign/data/spells.ron`

**Action:** Add a `"raise_dead"` spell entry using the `resurrect_hp` field:

```ron
(
    id: 0x0105,
    name: "Raise Dead",
    school: Cleric,
    level: 5,
    sp_cost: 15,
    gem_cost: 2,
    context: Anytime,
    target: SingleCharacter,
    description: "Resurrects a dead character to 1 HP.",
    damage: None,
    duration: 0,
    saving_throw: false,
    applied_conditions: [],
    resurrect_hp: Some(1),
),
```

**Note:** Also add this spell to `campaigns/tutorial/data/spells.ron`.

---

#### 3.5 Testing Requirements

| Test name                                              | File           | What it verifies                                                                           |
| ------------------------------------------------------ | -------------- | ------------------------------------------------------------------------------------------ |
| `test_default_condition_database_includes_unconscious` | `database.rs`  | `ConditionDatabase::new()` contains `"unconscious"`                                        |
| `test_default_condition_database_includes_dead`        | `database.rs`  | `ConditionDatabase::new()` contains `"dead"`                                               |
| `test_validate_warns_missing_unconscious`              | `database.rs`  | `ContentDatabase::validate()` returns an error when `"unconscious"` is absent              |
| `test_validate_warns_missing_dead`                     | `database.rs`  | `ContentDatabase::validate()` returns an error when `"dead"` is absent                     |
| `test_resurrection_scroll_template`                    | `templates.rs` | `resurrection_scroll()` returns an item with `ConsumableEffect::Resurrect(1)`              |
| `test_raise_dead_spell_loads_from_test_campaign`       | `database.rs`  | Loading `data/test_campaign/data/spells.ron` includes a spell with `resurrect_hp: Some(1)` |

---

#### 3.6 Deliverables

- [ ] `ContentDatabase::validate()` errors on missing `"unconscious"` or
      `"dead"` conditions
- [ ] `ConditionDatabase::new()` pre-populates system conditions
- [ ] `resurrection_scroll()` template function and test in `templates.rs`
- [ ] `"raise_dead"` spell entry in `data/test_campaign/data/spells.ron`
      and `campaigns/tutorial/data/spells.ron`
- [ ] All tests in 3.5 pass
- [ ] All four quality gates pass

#### 3.7 Success Criteria

- A `ContentDatabase` built with `new()` always has `"unconscious"` and
  `"dead"` conditions.
- `validate()` errors on any campaign missing these system conditions.
- Campaign creators can use `resurrection_scroll()` from `templates.rs`
  as a starting point.
- The `"raise_dead"` spell is available in the test campaign fixture.

---

### Phase 4: NPC Temple/Priest Resurrection Service

Add an application-layer transaction for priest NPC resurrection services
and the corresponding UI.

---

#### 4.1 Add `"raise_dead"` Service to Priest NPC Data

**Files to modify:**

- `campaigns/tutorial/data/npcs.ron` — add or update the existing priest
  NPC entry to include a `service_catalog` with a `"raise_dead"` entry.
- `data/test_campaign/data/npcs.ron` — same change for the fixture NPC.

**Service catalog entry format (RON):**

```ron
service_catalog: Some((
    services: [
        (
            service_id: "raise_dead",
            cost: 500,
            gem_cost: 1,
            description: "Resurrect a dead party member for 500 gold and 1 gem.",
        ),
    ],
)),
```

The NPC must have `is_priest: true`. A typical tutorial priest NPC ID is
`"temple_priest"`.

---

#### 4.2 Add `perform_resurrection_service` to Application Layer

**File:** `src/application/resources.rs`

**Action:** Add a new public function:

```rust
/// Performs a priest resurrection service for the party member at `character_index`.
///
/// Steps performed:
/// 1. Looks up the NPC by `npc_id` in `content.npcs`.
/// 2. Verifies the NPC has a `"raise_dead"` service entry.
/// 3. Checks campaign permadeath via `check_permadeath_allows_resurrection`.
/// 4. Verifies the target character is dead (`conditions.is_dead() == true`).
/// 5. Verifies the party has enough gold and gems.
/// 6. Deducts gold and gems from the party.
/// 7. Calls `revive_from_dead(character, 1)`.
///
/// # Arguments
///
/// * `game_state` - Mutable game state (party gold/gems and character conditions)
/// * `npc_id` - String ID of the priest NPC (e.g. `"temple_priest"`)
/// * `character_index` - Index into `game_state.party.members`
/// * `content` - Content database for NPC and service lookup
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a descriptive `String` error for: NPC not found, service not found,
/// permadeath enabled, target not dead, insufficient gold, insufficient gems.
pub fn perform_resurrection_service(
    game_state: &mut crate::application::GameState,
    npc_id: &str,
    character_index: usize,
    content: &crate::sdk::database::ContentDatabase,
) -> Result<(), String> { ... }
```

This function lives in the application layer (not domain) because it reads
`CampaignConfig` for permadeath and touches both party resources and character
conditions.

---

#### 4.3 Resurrection Service UI

**File:** `src/game/systems/inn_ui.rs` (extend) OR new file
`src/game/systems/temple_ui.rs`.

Follow the exact structural pattern of `src/game/systems/inn_ui.rs`:

- A `TempleUiRoot` marker component.
- A `TemplePlugin` that registers systems.
- `setup_temple_ui`: spawns the UI on NPC interaction when the NPC is a
  priest with `service_catalog.has_service("raise_dead") == true`.
- `update_temple_ui`: displays each dead party member's name and the cost
  in gold and gems. Living and unconscious members are not shown.
- `handle_temple_input`: on confirm, calls
  `perform_resurrection_service(game_state, npc_id, idx, content)` and
  refreshes the display. Shows "Party has insufficient funds" if the call
  returns an error about gold/gems. Shows "Permadeath is enabled" if
  permadeath error is returned.
- `cleanup_temple_ui`: despawns on dialogue close.

The UI must show only dead party members (those where
`character.conditions.is_dead() == true`). Members with `STONE` or
`ERADICATED` must not appear.

---

#### 4.4 Testing Requirements

| Test name                                                 | File           | What it verifies                                                   |
| --------------------------------------------------------- | -------------- | ------------------------------------------------------------------ |
| `test_perform_resurrection_service_success`               | `resources.rs` | Gold/gems are deducted and dead character is revived to 1 HP       |
| `test_perform_resurrection_service_insufficient_gold`     | `resources.rs` | Returns `Err` when party gold < service cost                       |
| `test_perform_resurrection_service_insufficient_gems`     | `resources.rs` | Returns `Err` when party gems < gem cost                           |
| `test_perform_resurrection_service_target_not_dead`       | `resources.rs` | Returns `Err` when target character is alive                       |
| `test_perform_resurrection_service_npc_not_found`         | `resources.rs` | Returns `Err` when NPC ID is not in database                       |
| `test_perform_resurrection_service_no_raise_dead_service` | `resources.rs` | Returns `Err` when NPC lacks `"raise_dead"` service                |
| `test_perform_resurrection_service_permadeath`            | `resources.rs` | Returns `Err` when `campaign_config.permadeath == true`            |
| `test_temple_npc_has_raise_dead_service`                  | `database.rs`  | Test campaign priest NPC has `service_catalog` with `"raise_dead"` |
| `test_temple_ui_shows_dead_members_only`                  | `temple_ui.rs` | UI lists only members with `is_dead() == true`                     |
| `test_temple_ui_does_not_show_eradicated`                 | `temple_ui.rs` | Eradicated characters do not appear in the temple list             |

---

#### 4.5 Deliverables

- [ ] Tutorial and test campaign priest NPC has `"raise_dead"` service in
      `service_catalog`
- [ ] `perform_resurrection_service` function in `src/application/resources.rs`
- [ ] `TemplePlugin` / temple UI system registered in
      `src/game/systems/` (new file `temple_ui.rs` or extended `inn_ui.rs`)
- [ ] Temple UI shows dead party members with cost; hides living/stone/eradicated
- [ ] Insufficient funds and permadeath errors shown in UI
- [ ] All tests in 4.4 pass
- [ ] All four quality gates pass

#### 4.6 Success Criteria

- Players can interact with a `is_priest: true` NPC that has the
  `"raise_dead"` service to revive dead party members for gold and gems.
- The service fails gracefully when the party lacks funds, when permadeath
  is enabled, or when no party member is dead.
- Eradicated characters do not appear in the list.
- All quality gates pass.

---

## Quality Gate Reference

Run these commands in order after completing each phase. All must produce
zero errors and zero warnings before proceeding to the next phase.

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

## Post-Implementation Checklist

- [ ] No test references `campaigns/tutorial` (all fixture data uses `data/test_campaign`)
- [ ] All new `.rs` files begin with SPDX copyright and license headers
- [ ] All public functions have `///` doc comments with `# Examples`
- [ ] `docs/explanation/implementations.md` updated with a summary of this implementation
- [ ] Architecture deviations (if any) are documented with rationale
