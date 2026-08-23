# NPC Combat Trigger — Generic Feature + Eonir Tutorial Example

## Overview

Add a generic `NpcCombatSwitch` configuration to `NpcDefinition` so any NPC
can be replaced by a combat encounter when a named global flag is set.  The
switch is entirely data-driven: a dialogue action sets a flag, a new exploration
system detects it, despawns the NPC entity, builds a `CombatState` from the
configured `monster_id`, and starts combat.  After the party wins, a
configurable `defeat_flag` is set automatically.

The Eonir fight is the canonical tutorial example: once the generic system
exists, Eonir's NPC entry is extended with one `combat_switch` block and his
dialogue tree receives a combat branch — no engine changes beyond what is
already implemented in the generic phases.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Notes |
|--------|----------|-------|
| `NpcDefinition` | `src/domain/world/npc.rs` L111 | 20+ fields; none for combat triggering; all optional fields use `#[serde(default)]` |
| `GlobalFlags` | `src/application/mod.rs` L1053 | `HashMap<String, bool>`; `get(flag)`; `set(flag, bool)` |
| `DialogueAction::SetFlag` | `src/domain/dialogue.rs` L1582 | Already sets `global_flags` in the dialogue executor |
| `DialogueCondition::FlagSet` | `src/domain/dialogue.rs` | Already reads `global_flags` for branch conditions |
| `CombatResource` | `src/game/systems/combat.rs` L449 | Holds `CombatState`, `combat_event_type`, participant mapping |
| `GameState::enter_combat_with_state` | `src/application/mod.rs` L1853 | Sets `mode = GameMode::Combat(cs)` |
| `CombatStarted` message | `src/game/systems/combat.rs` L89 | Emitted when combat starts; carries position, map id, combat type |
| `handle_combat_victory` | `src/game/systems/combat.rs` | Handles `CombatVictory`; plays victory fanfare; returns to exploration |
| `MapEvent::NpcDialogue` | `src/domain/world/types.rs` L2164 | Places an NPC on a tile; no combat-trigger variant |
| `tutorial_lich_eonir` NPC | `campaigns/tutorial/data/npcs.ron` L134 | `dialogue_id: Some(1004)`, `creature_id: Some(1021)` — no `combat_switch` yet |

### Identified Gaps

1. **No `combat_switch` on `NpcDefinition`** — there is no way to configure an
   NPC to become a monster encounter when a flag fires.
2. **No exploration system for NPC flag monitoring** — nothing polls whether a
   combat-switch NPC's trigger flag has been set.
3. **No NPC spawn guard** — if the trigger flag is already true on map load (e.g.
   save/reload after the dialogue), the NPC re-spawns as if nothing happened.
4. **No post-combat defeat flag** — `handle_combat_victory` does not set any
   flag when combat ends; the victory path is hardcoded to exploration return.
5. **No `DialogueAction::StartCombat`** — the current `DialogueAction` enum has
   `SetFlag`, `TriggerEvent`, etc., but no direct combat-start variant.

---

## Design: Flag-Triggered NPC → Monster Swap

```
Dialogue node (combat branch)
  └── DialogueAction::SetFlag { "eonir_combat_triggered", true }
        │
        │  (next frame, Exploration mode)
        ▼
npc_combat_switch_system
  ├── reads NpcDefinition::combat_switch for each live NPC entity
  ├── checks global_flags.get(trigger_flag) == true
  ├── despawns NPC entity
  └── writes PendingNpcCombat resource
        │
        ▼
start_npc_combat_system
  ├── builds CombatState from monster_id
  ├── calls enter_combat_with_state(cs)
  └── sends CombatStarted message
        │
        ▼
handle_combat_victory
  └── reads CombatResource::defeat_flag → sets global_flags.set(defeat_flag, true)
```

---

## Implementation Phases

### Phase 1: Domain — `NpcCombatSwitch` Data Structure

All changes in `src/domain/world/npc.rs`.

#### 1.1 Add `NpcCombatSwitch` Struct

Add `pub struct NpcCombatSwitch` immediately before `NpcDefinition`:

- `pub trigger_flag: String` — name of the `GlobalFlags` key that activates the
  swap (e.g. `"eonir_combat_triggered"`)
- `pub monster_id: MonsterId` — ID of the monster entry to fight (looked up in
  the `MonsterDatabase` at swap time)
- `pub defeat_flag: Option<String>` — if `Some`, the engine sets this global flag
  automatically when the party wins; `None` means no post-combat flag
- `pub combat_type: crate::domain::combat::types::CombatEventType` — `Normal`,
  `Boss`, `Ambush`, etc.; defaults to `Normal` via `#[serde(default)]`

Derive `Debug, Clone, Serialize, Deserialize, PartialEq`.  All fields except
`trigger_flag` and `monster_id` should use `#[serde(default)]`.

#### 1.2 Add `combat_switch` to `NpcDefinition`

At the end of `NpcDefinition` (after `skill_training_max_rank`), add:

```
/// Optional combat-trigger configuration.
///
/// When set, the exploration system monitors `combat_switch.trigger_flag` in
/// `GlobalFlags`. When the flag becomes true, this NPC entity is despawned and
/// the configured monster encounter is started at the same tile position.
/// After party victory, `combat_switch.defeat_flag` (if set) is written back to
/// `GlobalFlags`.
///
/// `#[serde(default)]` ensures all existing RON files without this field
/// continue to deserialize correctly.
#[serde(default)]
pub combat_switch: Option<NpcCombatSwitch>,
```

Update the existing `NpcDefinition` doctest to include `combat_switch: None`.

#### 1.3 Testing Requirements

- `test_npc_definition_combat_switch_defaults_to_none` — a minimal `NpcDefinition`
  without the field deserialises with `combat_switch == None`.
- `test_npc_combat_switch_round_trip` — a full `NpcCombatSwitch` value
  serialises to RON and back to an equal value.
- `test_npc_combat_switch_defaults_combat_type_to_normal` — a RON block with
  only `trigger_flag` and `monster_id` deserialises with `combat_type == Normal`.

#### 1.4 Deliverables

- [ ] `NpcCombatSwitch` struct in `src/domain/world/npc.rs`
- [ ] `NpcDefinition::combat_switch: Option<NpcCombatSwitch>` field added
- [ ] `NpcDefinition` doctest updated to include `combat_switch: None`
- [ ] All Phase 1 tests pass; `cargo check` clean

#### 1.5 Success Criteria

All existing `npcs.ron` files across `campaigns/tutorial` and `data/test_campaign`
continue to deserialise without change.  A new NPC entry with
`combat_switch: Some(...)` deserialises correctly.

---

### Phase 2: Exploration Engine — NPC Spawn Guard + Combat Switch System

New Bevy systems in `src/game/systems/` (new file `npc_combat_switch.rs` or
added to the existing NPC system file).

#### 2.1 NPC Spawn Guard

In the NPC entity spawning path (wherever NPC entities are created from
`npc_placements`), add a pre-spawn check:

Before spawning any NPC, look up its `NpcDefinition` in the content database.
If `npc.combat_switch.is_some()` AND
`global_flags.get(&npc.combat_switch.trigger_flag)` is `true`:
- Skip spawning (the NPC has already been defeated or the fight is already
  pending).
- Log at `debug!` level: `"Suppressing NPC spawn for '{}': combat trigger flag '{}' is set"`.

This prevents a dead NPC from re-appearing after a save/reload.

#### 2.2 `PendingNpcCombat` Bevy Resource

Add `pub struct PendingNpcCombat` to `src/game/systems/npc_combat_switch.rs`:

```rust
pub struct PendingNpcCombat {
    pub monster_id: MonsterId,
    pub position: Position,
    pub map_id: MapId,
    pub defeat_flag: Option<String>,
    pub combat_type: CombatEventType,
}
```

Derive `Resource`.  Default: `Option<PendingNpcCombat>` (the resource holds
`Option<PendingNpcCombat>` so "no pending combat" is encoded as `None`).

#### 2.3 `npc_combat_switch_system`

A new Bevy `Update` system that runs only in `Exploration` mode:

Parameters:
- `query: Query<(Entity, &NpcId, &Transform)>` — all live NPC entities with
  position (or whatever component carries the tile position)
- `content: Res<GameContent>` — to look up `NpcDefinition::combat_switch`
- `global_state: ResMut<GlobalState>` — to read `global_flags`
- `mut pending: ResMut<Option<PendingNpcCombat>>`
- `mut commands: Commands`

Logic:
1. If `pending.is_some()`, return early — a swap is already in progress.
2. For each NPC entity `(entity, npc_id, transform)`:
   - Look up `NpcDefinition` by `npc_id`.
   - If `combat_switch.is_none()`, skip.
   - If `!global_flags.get(&combat_switch.trigger_flag)`, skip.
   - Record position from `transform` (convert to `Position`).
   - Despawn the NPC entity: `commands.entity(entity).despawn_recursive()`.
   - Write `pending = Some(PendingNpcCombat { monster_id, position, map_id, defeat_flag, combat_type })`.
   - Break (handle one swap per frame to avoid reentrancy issues).

#### 2.4 `start_npc_combat_system`

A second Bevy `Update` system, ordered after `npc_combat_switch_system`:

Parameters:
- `mut pending: ResMut<Option<PendingNpcCombat>>`
- `mut global_state: ResMut<GlobalState>`
- `content: Res<GameContent>`
- `mut combat_started_writer: MessageWriter<CombatStarted>`

Logic:
1. If `pending.is_none()`, return early.
2. Take the pending combat: `let pending = pending.take().unwrap()`.
3. Look up the `MonsterDefinition` by `pending.monster_id`.
4. Build a `CombatState` with the monster as a participant (same pattern used
   by `move_party_and_handle_events` for `MapEvent::Encounter`).
5. Set `CombatResource::defeat_flag = pending.defeat_flag` (see Phase 3).
6. Call `global_state.0.enter_combat_with_state(cs)`.
7. Send `CombatStarted { encounter_position: Some(pending.position), encounter_map_id: Some(pending.map_id), combat_event_type: pending.combat_type }`.

#### 2.5 Register Systems in Plugin

Add `npc_combat_switch_system` and `start_npc_combat_system` to the appropriate
plugin's `build` method, ordered:
`npc_combat_switch_system` → `start_npc_combat_system`.  Both run only when
`in_exploration_mode` (matching how other exploration-only systems are gated).

Insert `Option::<PendingNpcCombat>::None` as a Bevy resource on startup.

#### 2.6 Testing Requirements

- `test_npc_combat_switch_system_skips_npc_without_combat_switch`
- `test_npc_combat_switch_system_skips_when_trigger_flag_not_set`
- `test_npc_combat_switch_system_despawns_npc_when_flag_set` — Bevy app test;
  asserts the NPC entity is despawned and `PendingNpcCombat` is `Some(...)`.
- `test_start_npc_combat_system_clears_pending_and_enters_combat` — asserts
  `GameState::mode` becomes `GameMode::Combat(...)` and `PendingNpcCombat` is `None`.
- `test_npc_spawn_guard_suppresses_npc_when_trigger_flag_already_set`.

#### 2.7 Deliverables

- [ ] `PendingNpcCombat` Bevy resource added and registered
- [ ] `npc_combat_switch_system` implemented (despawn NPC, write pending)
- [ ] `start_npc_combat_system` implemented (read pending, start combat)
- [ ] NPC spawn guard added to the NPC instantiation path
- [ ] Both systems registered and gated to `Exploration` mode
- [ ] All Phase 2 tests pass

#### 2.8 Success Criteria

With an NPC whose `combat_switch.trigger_flag = "test_trigger"`: setting
`global_flags.set("test_trigger", true)` on the next exploration frame causes
the NPC entity to be despawned and `GameState::mode` to become
`GameMode::Combat(...)`.  Reloading the map after the flag is set does not
re-spawn the NPC.

---

### Phase 3: Post-Combat Hook — Defeat Flag

Wire the `defeat_flag` from `PendingNpcCombat` through combat to `handle_combat_victory`.

#### 3.1 Add `defeat_flag` to `CombatResource`

In `src/game/systems/combat.rs`, add to `CombatResource`:

```rust
/// Optional global flag name to set when this combat is won.
/// Populated by `start_npc_combat_system` for NPC combat-switch encounters.
/// Cleared when `CombatResource::clear()` is called.
pub defeat_flag: Option<String>,
```

Update `CombatResource::clear()` to reset `defeat_flag = None`.

#### 3.2 Update `handle_combat_victory`

After the existing victory logic (playing fanfare, transitioning to exploration),
add:

```rust
if let Some(ref flag) = combat_res.defeat_flag {
    global_state.0.global_flags.set(flag, true);
    tracing::info!("NPC combat switch: defeat flag '{}' set", flag);
}
```

Call `combat_res.defeat_flag = None` to avoid double-setting across frames.

#### 3.3 Testing Requirements

- `test_combat_resource_defeat_flag_set_on_npc_combat_start`
- `test_handle_combat_victory_sets_defeat_flag_when_present` — Bevy app test;
  asserts `global_flags.get("eonir_defeated")` is `true` after victory when
  `combat_res.defeat_flag = Some("eonir_defeated".to_string())`.
- `test_handle_combat_victory_no_defeat_flag_does_not_panic` — victory without
  a defeat flag does not change any flags.
- `test_combat_resource_clear_resets_defeat_flag`

#### 3.4 Deliverables

- [ ] `CombatResource::defeat_flag: Option<String>` added
- [ ] `CombatResource::clear()` resets `defeat_flag`
- [ ] `handle_combat_victory` writes the defeat flag
- [ ] All Phase 3 tests pass

#### 3.5 Success Criteria

An NPC combat switch that sets `defeat_flag: Some("eonir_defeated")` causes
`global_flags.get("eonir_defeated")` to be `true` after the party wins the combat.
Normal (non-switch) combat is completely unaffected.

---

### Phase 4: Tutorial — Eonir the Still Boss Fight

Apply the generic system to wire up the Act IV Eonir encounter.  All test data
uses `data/test_campaign`, never `campaigns/tutorial` (Implementation Rule 5).

#### 4.1 Add `lich_king_eonir` Monster Entry

Add a new entry to `campaigns/tutorial/data/monsters.ron`:

- `id`: a new unused `MonsterId` (verify no collision with existing 1–136 range)
- `name: "Lich King Eonir"`
- Boss-tier stats drawn from `eonir.md` combat description:
  - High `hp` (befitting a centuries-old lich)
  - High `magic_resistance`
  - Low `speed` (rarely moves)
  - Regeneration flag / regen amount
  - Any special attacks (gravity, shadow barrier) defined in the monster's
    `attack_patterns` or `special_abilities`
- `visual_id: Some(1021)` (same creature visual as the NPC, per the original plan)
- `combat_type: Boss`

#### 4.2 Update `tutorial_lich_eonir` NPC Entry

In `campaigns/tutorial/data/npcs.ron`, add to the `tutorial_lich_eonir` entry:

```ron
combat_switch: Some((
    trigger_flag: "eonir_combat_triggered",
    monster_id: <lich_king_eonir_id>,
    defeat_flag: Some("eonir_defeated"),
    combat_type: Boss,
)),
```

No other NPC fields change.

#### 4.3 Update Dialogue 1004 — Add Combat Branch

In the tutorial dialogue file for dialogue `1004` (Eonir's dialogue), add:

- A new dialogue branch for "refuse to negotiate" (or "failed persuasion"):
  - Choice text: something like "I will not bargain with a lich."
  - Actions: `[SetFlag { flag_name: "eonir_combat_triggered", value: true }]`
  - No follow-up node (the dialogue ends and the combat switch fires)
- The existing peaceful resolution branch sets:
  - `SetFlag { flag_name: "eonir_relic_returned", value: true }`
  - Appropriate quest completion action
- Both branches must ensure `tutorial_lich_eonir` never reappears:
  - Peaceful path: set a separate flag like `"eonir_departed"` and add a spawn
    guard in the NPC definition for that flag, OR add `defeat_flag` equivalent
    for the NPC's peaceful exit — see §4.4 below.

#### 4.4 NPC Peaceful Removal (Supplemental)

For the peaceful path, Eonir must also not reappear after the party accepts
the deal.  Add a second `npc_suppress_flag` field to `NpcCombatSwitch` — OR
handle this via a simpler approach: add `#[serde(default)] pub suppress_flag:
Option<String>` to `NpcDefinition` directly (a flag that suppresses spawn
without starting combat).  When `global_flags.get(suppress_flag)` is true, the
NPC spawn guard skips spawning (same guard added in Phase 2.1).

Assign `suppress_flag: Some("eonir_relic_returned")` to `tutorial_lich_eonir`.

#### 4.5 Quest Integration

Update the tutorial quest data for Act IV/V so that:

- **Combat path** (`eonir_defeated = true`): advances the Act V "return the
  relic to Jyeshtha" quest step
- **Peaceful path** (`eonir_relic_returned = true`): advances the same Act V
  step via the existing quest stage completion action

Both paths should converge on the same quest step, so Act V is identical
regardless of how the Eonir encounter resolved.

#### 4.6 Minimal `data/test_campaign` Fixture

Add to `data/test_campaign/data/npcs.ron` a test NPC entry:

```ron
(
    id: "test_combat_switch_npc",
    name: "Test Switch NPC",
    portrait_id: "placeholder",
    combat_switch: Some((
        trigger_flag: "test_switch_trigger",
        monster_id: 1,   // Goblin — always present in test fixtures
        defeat_flag: Some("test_switch_defeated"),
        combat_type: Normal,
    )),
)
```

This NPC powers the integration tests in Phases 2 and 3 without depending on
tutorial campaign content.

#### 4.7 Testing Requirements

- `test_test_campaign_combat_switch_npc_deserialises` — loads
  `data/test_campaign/data/npcs.ron`; asserts `test_combat_switch_npc` has
  `combat_switch.is_some()`.
- `test_lich_king_eonir_monster_has_boss_stats` (doc-level check; no test
  required for raw stat values — these are data, not logic).

#### 4.8 Deliverables

- [ ] `lich_king_eonir` monster added to tutorial `monsters.ron`
- [ ] `tutorial_lich_eonir` NPC extended with `combat_switch` and `suppress_flag`
- [ ] Dialogue 1004 updated with combat and peaceful branches
- [ ] Quest data updated for both convergence paths
- [ ] `data/test_campaign` fixture NPC added
- [ ] All tests pass; `docs/explanation/implementations.md` updated

#### 4.9 Success Criteria

In the game: approaching Eonir opens dialogue 1004. Choosing the combat branch
sets `eonir_combat_triggered`. On the next frame Eonir's NPC entity is despawned
and the Lich King combat starts. Winning sets `eonir_defeated`. Choosing the
peaceful branch sets `eonir_relic_returned` and Eonir does not reappear on map
reload. Both paths enable the Act V quest step.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/world/npc.rs` | 1, 4 | Add `NpcCombatSwitch`; add `combat_switch` + `suppress_flag` to `NpcDefinition` |
| `src/game/systems/npc_combat_switch.rs` | 2 | **New** — `PendingNpcCombat`, `npc_combat_switch_system`, `start_npc_combat_system` |
| `src/game/systems/<npc_spawn>.rs` | 2 | NPC spawn guard checks `combat_switch.trigger_flag` and `suppress_flag` |
| `src/game/systems/combat.rs` | 3 | Add `defeat_flag` to `CombatResource`; update `handle_combat_victory` |
| `campaigns/tutorial/data/monsters.ron` | 4 | Add `lich_king_eonir` boss entry |
| `campaigns/tutorial/data/npcs.ron` | 4 | Add `combat_switch` + `suppress_flag` to `tutorial_lich_eonir` |
| `campaigns/tutorial/data/dialogues.ron` | 4 | Update dialogue 1004 with combat and peaceful branches |
| `campaigns/tutorial/data/quests.ron` | 4 | Act IV/V quest step convergence for both paths |
| `data/test_campaign/data/npcs.ron` | 4 | Add `test_combat_switch_npc` fixture |
| `docs/explanation/implementations.md` | 4 | Implementation summary |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| Flag-based vs. dialogue-direct? | Flag-based — decoupled, data-driven; any system can trigger the swap, not only dialogue |
| One swap per frame or all at once? | One per frame — simpler; multiple simultaneous NPC-to-combat switches in one location are not a realistic use case |
| Peaceful NPC removal mechanism? | Add `suppress_flag: Option<String>` to `NpcDefinition` (reuses the same spawn guard, no new logic) |
| `defeat_flag` storage? | On `CombatResource` (cleared on `clear()`) — avoids coupling `GlobalState` to the pending-combat resource |
| NPC visual during combat? | The monster entry (`lich_king_eonir`) gets `visual_id: Some(1021)` — same creature mesh as the NPC — so the visual remains identical |
| SDK editor changes? | Deferred — the `combat_switch` and `suppress_flag` fields are plain optional data; the SDK NPC editor can be updated separately once the game layer is validated |
