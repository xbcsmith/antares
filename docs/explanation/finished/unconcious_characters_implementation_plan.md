# Unconscious and Dead Characters Implementation Plan

## Overview

Characters with 0 HP should become unconscious and be
unable to act in combat. Dead characters should require
resurrection. Both Unconscious and Dead need to be
data-driven campaign conditions so campaign creators
can customize behavior (e.g. permadeath). This plan
fixes the existing bug where 0 HP characters can still
attack, adds proper condition lifecycle management,
and integrates with healing, rest, and the SDK.

## Current State Analysis

### Existing Infrastructure

- **Bitflag `Condition` struct** in
  [character.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs#L484-L554):
  `UNCONSCIOUS = 64`, `DEAD = 128`, `STONE = 160`,
  `ERADICATED = 255`. Already has `is_unconscious()`,
  `is_fatal()`, `is_bad()` methods.
- **Data-driven conditions** in
  [conditions.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/conditions.rs):
  `ConditionDefinition`, `ActiveCondition`, `ConditionDuration`
  (including `Permanent`). 13 conditions exist in
  [conditions.ron](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/tutorial/data/conditions.ron)
  — **no unconscious or dead entries**.
- **Status string mapping** in
  [engine.rs `status_str_to_flag`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/engine.rs#L1267-L1282):
  already maps `"unconscious"` → `UNCONSCIOUS`, `"dead"` →
  `DEAD`. Wiring exists but no data triggers it.
- **`reconcile_character_conditions`** in
  [engine.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/engine.rs#L1176-L1231):
  syncs `ActiveCondition` → bitflags. Lists `UNCONSCIOUS`
  and `DEAD` in its flag_list. Ready to work once RON
  entries exist.
- **`apply_damage`** in
  [engine.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/engine.rs#L712-L731):
  drops HP to 0 but **never sets UNCONSCIOUS** — the core
  bug.
- **`apply_starvation_damage`** in
  [resources.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs#L855-L871):
  **does** set `Condition::DEAD` when HP hits 0 — shows
  the intended pattern.
- **Rest system** in
  [resources.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs#L645-L690):
  already skips `is_fatal() || is_unconscious()` characters
  — partially anticipating this feature.
- **`Character::is_alive()`**: checks `!conditions.is_fatal()`.
  Does **not** check HP. Monster `is_alive()` checks
  `hp.current > 0 && !conditions.is_dead()`.
- **HealHp consumable** in
  [consumable_usage.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/consumable_usage.rs#L229-L241):
  modifies `hp.current` but does **not** clear UNCONSCIOUS.
- **CureCondition consumable**: clears bitflags only. Does
  not touch `active_conditions` vec.

### Identified Issues

1. **Bug**: `apply_damage` doesn't set `UNCONSCIOUS` when
   HP reaches 0. Characters with 0 HP can still attack.
2. **Bug**: `Character::is_alive()` doesn't check HP,
   unlike the Monster equivalent.
3. **Missing data**: No `"unconscious"` or `"dead"` entries
   in `conditions.ron`.
4. **No revival mechanic**: Dead characters have no path
   back to life except editing saves.
5. **Healing doesn't clear unconscious**: `HealHp`
   consumable/spell doesn't remove the UNCONSCIOUS condition
   when HP is restored above 0.
6. **Rest skips unconscious**: By design, but needs a
   separate "revive unconscious via rest" path.
7. **Monster targeting**: Monsters can target unconscious
   characters (0 HP). They should skip them.
8. **SDK templates**: No default unconscious/dead condition
   definitions. Campaign creators must add them manually.

## Implementation Phases

### Phase 1: Core Unconscious Condition

Wire the unconscious condition end-to-end: HP → 0 sets
UNCONSCIOUS, healing above 0 clears it, combat respects it.

#### 1.1 Add Unconscious to `conditions.ron`

Add an `"unconscious"` entry to
[conditions.ron](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/tutorial/data/conditions.ron)
and
[data/test_campaign/data/conditions.ron](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/data/test_campaign/data/conditions.ron):

- `id: "unconscious"`
- `effects: [StatusEffect("unconscious")]`
- `default_duration: Permanent` (until healed/rested)
- `icon_id: Some("icon_unconscious")`

#### 1.2 Set UNCONSCIOUS on 0 HP in `apply_damage`

Modify `apply_damage` in
[engine.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/engine.rs#L712-L731):
when a player character's `hp.current` reaches 0, call
`character.conditions.add(Condition::UNCONSCIOUS)`.

#### 1.3 Fix `Character::is_alive()` to Check HP

Update `is_alive()` in
[character.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs#L1244-L1247)
to also check `self.hp.current > 0`. This is a safety net
matching the Monster pattern. `can_act()` already delegates
to `is_alive()` so this cascades properly.

#### 1.4 Clear UNCONSCIOUS When Healed Above 0 HP

Modify `apply_consumable_effect` in
[consumable_usage.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/items/consumable_usage.rs#L229-L241):
after the `HealHp` branch restores HP, if `character.hp.current > 0`
and `character.conditions.has(Condition::UNCONSCIOUS)`, remove
the UNCONSCIOUS bitflag **and** remove any matching
`active_conditions` entry with `condition_id == "unconscious"`.

Do the same in the spell healing path
([spell_casting.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/combat/spell_casting.rs))
and the rest system
([resources.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs#L645-L690)).

Extract a helper function `revive_from_unconscious(character)`
to avoid duplicating this logic.

#### 1.5 Prevent Monsters From Targeting Unconscious Players

In
[combat.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/combat.rs),
monster target selection must skip combatants where
`!combatant.is_alive()` (which now covers 0 HP). The
`resolve_attack` function already checks `target.is_alive()`
and returns `InvalidTarget` — verify this is sufficient
for the monster AI targeting path.

#### 1.6 Allow Rest to Revive Unconscious

Update `rest_party` and `rest_party_hour` in
[resources.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/resources.rs):
instead of skipping unconscious characters entirely, heal
them normally and call `revive_from_unconscious` once
`hp.current > 0`. Keep skipping fatal (dead) characters.

#### 1.7 Testing Requirements

- `test_apply_damage_sets_unconscious_at_zero_hp`
- `test_character_is_alive_false_at_zero_hp`
- `test_character_can_act_false_at_zero_hp`
- `test_heal_hp_clears_unconscious`
- `test_monster_skips_unconscious_target`
- `test_rest_revives_unconscious_character`
- `test_already_unconscious_not_doubled`
- `test_unconscious_condition_in_ron_loaded`
- All existing tests continue to pass.

#### 1.8 Deliverables

- [ ] `"unconscious"` entry in `conditions.ron` and
  `data/test_campaign/data/conditions.ron`
- [ ] `apply_damage` sets `UNCONSCIOUS` on 0 HP
- [ ] `Character::is_alive()` checks `hp.current > 0`
- [ ] `revive_from_unconscious` helper function
- [ ] `HealHp` / spell heal / rest clears UNCONSCIOUS
- [ ] Monster targeting skips unconscious players
- [ ] Rest heals unconscious characters
- [ ] Tests pass at >80% coverage

#### 1.9 Success Criteria

- A character reduced to 0 HP in combat becomes
  unconscious and cannot act.
- Monsters do not target unconscious characters.
- Healing a character above 0 HP revives them.
- Resting revives unconscious characters.
- All quality gates pass.

---

### Phase 2: Core Dead Condition

Add the Dead condition with resurrection mechanics.

#### 2.1 Add Dead to `conditions.ron`

Add a `"dead"` entry to `conditions.ron` and
`data/test_campaign/data/conditions.ron`:

- `id: "dead"`
- `effects: [StatusEffect("dead")]`
- `default_duration: Permanent` (until resurrected)
- `icon_id: Some("icon_dead")`

#### 2.2 Transition Unconscious → Dead

Decide on the death mechanic. Two options the campaign
creator should be able to configure:

**Option A — Instant death at 0 HP**: Skip unconscious,
go straight to dead. Simple but harsh.

**Option B — Unconscious first, dead on further damage**:
If an unconscious character (0 HP) takes additional damage,
they die. This is the classic RPG approach.

For implementation, add a `ConditionDefinition` field or
a campaign-level config flag:
`unconscious_before_death: bool` (default `true`).

When Option B is enabled: in `apply_damage`, if the target
is already unconscious and takes damage, set `DEAD` and
remove `UNCONSCIOUS`. When Option A is enabled: skip
`UNCONSCIOUS` entirely and set `DEAD` at 0 HP.

#### 2.3 Resurrection Mechanic

Add a new `ConsumableEffect::Resurrect` variant (or use
`CureCondition(Condition::DEAD)` + `HealHp`). Resurrection
should:

1. Remove `DEAD` condition (bitflag + active_conditions).
2. Restore HP to a configurable amount (e.g. 1 HP, or
   percentage of base).
3. Only work on dead characters, not eradicated.

Sources of resurrection:

- **Spells**: Add a resurrection spell effect variant to
  [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/magic/types.rs).
- **Scrolls/Items**: `ConsumableEffect::Resurrect` or
  `CureCondition(DEAD)` + `HealHp(1)`.
- **Temple/Priest NPC**: New NPC interaction type. A temple
  NPC service that charges gold to resurrect party members.
  This depends on the NPC/dialogue system.

#### 2.4 Campaign Creator Configurable Permadeath

Add a field to `ConditionDefinition` to support this:

```text
is_permanent: bool  // default false
```

When `is_permanent` is true on the `"dead"` condition,
resurrection spells and items fail with an appropriate
message. The campaign creator sets this in `conditions.ron`.

Alternatively, add to campaign config:

```text
permadeath: bool  // default false
```

#### 2.5 Update Rest System

Dead characters are **not** healed by rest. This is already
the case (`is_fatal()` check skips them). Verify this
remains correct after changes.

#### 2.6 Testing Requirements

- `test_unconscious_to_dead_on_further_damage`
- `test_instant_death_mode_skips_unconscious`
- `test_resurrect_removes_dead_condition`
- `test_resurrect_restores_hp`
- `test_resurrect_fails_on_eradicated`
- `test_permadeath_blocks_resurrection`
- `test_dead_character_skipped_in_rest`
- `test_dead_condition_in_ron_loaded`

#### 2.7 Deliverables

- [ ] `"dead"` entry in `conditions.ron` and test fixtures
- [ ] Unconscious → Dead transition on further damage
- [ ] `ConsumableEffect::Resurrect` variant
- [ ] Resurrection spell effect variant
- [ ] Campaign permadeath config option
- [ ] Tests at >80% coverage

#### 2.8 Success Criteria

- Dead characters cannot act, be targeted, or be healed
  by rest.
- Resurrection spells/items revive dead characters.
- Permadeath flag prevents resurrection.
- Campaign creators can choose instant-death or
  unconscious-first behavior.

---

### Phase 3: SDK Default Templates and Editor Integration

Ensure campaign creators get unconscious and dead
conditions by default and can customize them.

#### 3.1 Add Default Conditions to SDK Templates

Update
[templates.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/sdk/templates.rs)
to include `"unconscious"` and `"dead"` condition
definitions in the default template that new campaigns
generate. These should appear in every new campaign's
`conditions.ron` by default.

#### 3.2 Conditions Editor Integration

The SDK conditions editor (if it exists) should:

- Mark unconscious and dead as **system conditions**
  (cannot be deleted, only customized).
- Allow editing duration, icon, and description.
- Show a warning if these conditions are missing from
  a campaign.

#### 3.3 Spell/Item Editor Integration

- Expose `Resurrect` as a consumable effect type in the
  item editor.
- Expose resurrection as a spell effect in the spell editor.
- Allow campaign creators to create resurrection scrolls
  via the item editor.

#### 3.4 Testing Requirements

- `test_default_templates_include_unconscious`
- `test_default_templates_include_dead`
- `test_campaign_validation_warns_missing_conditions`

#### 3.5 Deliverables

- [ ] Default templates include unconscious and dead
- [ ] SDK warns if conditions are missing
- [ ] Resurrect effect available in item/spell editors
- [ ] Tests pass

#### 3.6 Success Criteria

- New campaigns created via SDK include unconscious and
  dead conditions by default.
- Campaign creators can customize but not delete them.
- Resurrection is available as a spell/item effect in
  the editors.

---

### Phase 4: NPC Temple/Priest Resurrection Service

Add an NPC service for resurrecting dead party members.

#### 4.1 Temple NPC Service Type

Add a new NPC interaction/service type for resurrection.
A temple NPC charges gold to resurrect dead party members.
The cost should be configurable per NPC (e.g. based on
character level or fixed cost).

#### 4.2 UI for Temple Service

Add a UI dialog (similar to inn/merchant) that:

- Lists dead party members.
- Shows cost per resurrection.
- Allows the player to select and pay for resurrection.

#### 4.3 Testing Requirements

- `test_temple_npc_lists_dead_members`
- `test_temple_npc_charges_gold`
- `test_temple_npc_resurrects_dead_character`

#### 4.4 Deliverables

- [ ] Temple NPC service type
- [ ] Resurrection service UI
- [ ] Cost configuration per NPC
- [ ] Tests pass

#### 4.5 Success Criteria

- Players can visit a temple NPC to resurrect dead
  party members for gold.
- The service is configurable by campaign creators.
